//! Canonical key continuations over exact request, representation and snapshot identities.
use lctx_model::domain::{ContentHash, serving::*};

/// Some semantic page identities hash the complete result. Visit every result once,
/// but retain only the requested key range instead of a rich complete-domain array.
pub(crate) struct PageCollector<T> {
    position: Result<(CursorBinding, Option<ContentHash>), WireError>,
    size: usize,
    total: usize,
    ordinal: usize,
    values: std::collections::BTreeMap<(ContentHash, usize), (T, usize)>,
    charge: lctx_model::domain::charged::StateCharge,
}
impl<T: serde::Serialize> PageCollector<T> {
    #[allow(clippy::too_many_arguments, reason = "The same exact cursor binding as page is required")]
    pub(crate) fn new(request: &Request, snapshot: &SnapshotHandle, channels: &ChannelState,
        group: &str, section: &str,
        member: Option<lctx_model::domain::Id<lctx_model::domain::catalog::CatalogMember>>,
        budget: &lctx_model::domain::resources::ResourceBudget) -> Self {
        let position = (|| {
            let expected = binding(request, snapshot, channels, group, section, member)?;
            let mut after = None;
            if let Some(token) = &request.page().cursor.0 {
                let bytes = hex::decode(token.as_str()).map_err(|_| WireError::Invalid("cursor encoding".into()))?;
                let cursor: Cursor = serde_json::from_slice(&bytes)?;
                if cursor.binding.group == expected.group && cursor.binding.section == expected.section {
                    let cursor = Cursor::decode(token, &expected)?;
                    let CursorPosition::Key { key } = cursor.after else {
                        return Err(WireError::Continuation("key continuation required".into()));
                    };
                    after = Some(key);
                }
            }
            Ok((expected, after))
        })();
        Self { position, size: request.page().size as usize, total: 0, ordinal: 0,
            values: Default::default(), charge: lctx_model::domain::charged::StateCharge::new(budget, "bounded-section-page") }
    }
    pub(crate) fn push(&mut self, key: ContentHash, value: T) -> Result<(), lctx_model::domain::ModelError> {
        // Cursor refusals are reported only after the exhaustive semantic/validation
        // traversal, just as in the full-array page implementation.
        let Ok((_, after)) = &self.position else { return Ok(()); };
        let ordinal = self.ordinal;
        self.ordinal = self.ordinal.checked_add(1).ok_or(lctx_model::domain::ModelError::Schema("page ordinal overflow"))?;
        if after.is_some_and(|after| key <= after) { return Ok(()); }
        self.total = self.total.checked_add(1).ok_or(lctx_model::domain::ModelError::Schema("page count overflow"))?;
        let position = (key, ordinal);
        if self.size == 0 || (self.values.len() == self.size && self.values.last_key_value().is_some_and(|(last, _)| position >= *last)) {
            return Ok(());
        }
        let bytes = crate::ranked_results::allocation_allowance(&value).map_err(crate::records::wire)?
            .checked_add(size_of::<T>() + size_of::<((ContentHash, usize), (T, usize))>() + 64)
            .ok_or(lctx_model::domain::ModelError::Schema("page retention overflow"))?;
        self.charge.grow(bytes)?;
        self.values.insert(position, (value, bytes));
        if self.values.len() > self.size {
            let (_, (value, bytes)) = self.values.pop_last().expect("bounded page victim");
            drop(value); self.charge.release(bytes);
        }
        Ok(())
    }
    pub(crate) fn finish(mut self, availability: Availability) -> Result<(SectionPage<T>, lctx_model::domain::charged::StateCharge), lctx_model::domain::ModelError> {
        let (expected, _) = self.position.map_err(crate::records::wire)?;
        let omitted = self.total.saturating_sub(self.size) as u64;
        let continuation = if omitted > 0 {
            let key = self.values.last_key_value().ok_or(lctx_model::domain::ModelError::Schema("positive page size"))?.0.0;
            Optional(Some(Cursor { binding: expected, after: CursorPosition::Key { key } }.encode().map_err(crate::records::wire)?))
        } else { Optional::default() };
        self.charge.grow(self.values.len().saturating_mul(size_of::<T>()))?;
        let items = self.values.into_values().map(|(value, _)| value).collect();
        Ok((SectionPage { availability, items, continuation, omitted, truncated: omitted > 0 }, self.charge))
    }
}
pub(crate) fn packet_key(value: &impl serde::Serialize) -> Result<ContentHash, lctx_model::domain::ModelError> {
    struct Writer(lctx_model::domain::ContentHasher);
    impl std::io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> { self.0.update(bytes); Ok(bytes.len()) }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    let mut writer = Writer(Default::default());
    serde_json::to_writer(&mut writer, value).map_err(lctx_model::domain::ModelError::codec)?;
    Ok(writer.0.finish())
}

pub fn binding(
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
    group: &str,
    section: &str,
    member: Option<lctx_model::domain::Id<lctx_model::domain::catalog::CatalogMember>>,
) -> Result<CursorBinding, WireError> {
    let constants = crate::executable_constants();
    let ordering = if request.page().expanded {
        constants.expanded_ordering
    } else {
        constants.default_ordering
    };
    Ok(CursorBinding {
        snapshot: snapshot.clone(),
        request: request.canonical_identity()?,
        policy: *constants.policy.as_ref().map_err(|error| WireError::Invalid(error.clone()))?,
        wire: constants.wire,
        channels: channels.identity(),
        group: Name::new(group)?,
        section: Name::new(section)?,
        member,
        ordering,
    })
}
/// A response may expose several independently resumable sections. A cursor for one
/// section leaves the other sections on their first page; its full binding is still checked.
#[allow(
    clippy::too_many_arguments,
    reason = "Pagination binds snapshot, channel state, group, section, member and availability explicitly"
)]
pub fn page<T>(
    mut values: Vec<(ContentHash, T)>,
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
    group: &str,
    section: &str,
    member: Option<lctx_model::domain::Id<lctx_model::domain::catalog::CatalogMember>>,
    availability: Availability,
) -> Result<SectionPage<T>, WireError> {
    values.sort_by_key(|(key, _)| *key);
    let expected = binding(request, snapshot, channels, group, section, member)?;
    let mut after = None;
    if let Some(token) = &request.page().cursor.0 {
        let bytes = hex::decode(token.as_str())
            .map_err(|_| WireError::Invalid("cursor encoding".into()))?;
        let cursor: Cursor = serde_json::from_slice(&bytes)?;
        // Inspecting the public group never authorizes reading. The selected group validates
        // every bound identity before applying its position.
        if cursor.binding.group == expected.group && cursor.binding.section == expected.section {
            let cursor = Cursor::decode(token, &expected)?;
            match cursor.after {
                CursorPosition::Key { key } => after = Some(key),
                _ => return Err(WireError::Continuation("key continuation required".into())),
            }
        }
    }
    values.retain(|(key, _)| after.is_none_or(|after| *key > after));
    let omitted = values.len().saturating_sub(request.page().size as usize) as u64;
    values.truncate(request.page().size as usize);
    let continuation = if omitted > 0 {
        Optional(Some(
            Cursor {
                binding: expected,
                after: CursorPosition::Key {
                    key: values.last().expect("positive requested page size").0,
                },
            }
            .encode()?,
        ))
    } else {
        Optional::default()
    };
    Ok(SectionPage {
        availability,
        items: values.into_iter().map(|(_, value)| value).collect(),
        continuation,
        omitted,
        truncated: omitted > 0,
    })
}

/// Select outer parents first. A nested continuation addresses exactly its original association,
/// independently of the outer page's key range and the ordering of supplied rows.
pub fn scenario_parents<T>(
    values: Vec<(
        ContentHash,
        (
            lctx_model::domain::Id<lctx_model::domain::catalog::evidence::ScenarioAssociation>,
            T,
        ),
    )>,
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
    member: lctx_model::domain::Id<lctx_model::domain::catalog::CatalogMember>,
) -> Result<
    SectionPage<(
        lctx_model::domain::Id<lctx_model::domain::catalog::evidence::ScenarioAssociation>,
        T,
    )>,
    WireError,
> {
    if let Some(token) = &request.page().cursor.0 {
        let expected = binding(
            request,
            snapshot,
            channels,
            request.tool().name(),
            "scenario_diagnostics",
            Some(member),
        )?;
        let bytes = hex::decode(token.as_str())
            .map_err(|_| WireError::Invalid("cursor encoding".into()))?;
        let cursor: Cursor = serde_json::from_slice(&bytes)?;
        if cursor.binding.section.as_str() == "scenario_diagnostics" {
            let cursor = Cursor::decode(token, &expected)?;
            let CursorPosition::ScenarioDiagnostic { association, .. } = cursor.after else {
                return Err(WireError::Continuation(
                    "scenario diagnostic continuation required".into(),
                ));
            };
            let mut selected = values.into_iter().filter(|(_, (id, _))| *id == association);
            let (_, item) = selected.next().ok_or_else(|| {
                WireError::Continuation("scenario association absent from member".into())
            })?;
            if selected.next().is_some() {
                return Err(WireError::Continuation(
                    "scenario association duplicated".into(),
                ));
            }
            return Ok(SectionPage {
                availability: Availability::Available {},
                items: vec![item],
                continuation: Optional::default(),
                omitted: 0,
                truncated: false,
            });
        }
    }
    page(
        values,
        request,
        snapshot,
        channels,
        request.tool().name(),
        "scenarios",
        Some(member),
        Availability::Available {},
    )
}
/// Child positions retain the owning association as well as the last diagnostic key.
pub fn scenario_diagnostics<T>(
    mut values: Vec<(ContentHash, T)>,
    association: lctx_model::domain::Id<lctx_model::domain::catalog::evidence::ScenarioAssociation>,
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
    member: lctx_model::domain::Id<lctx_model::domain::catalog::CatalogMember>,
) -> Result<SectionPage<T>, WireError> {
    let expected = binding(
        request,
        snapshot,
        channels,
        request.tool().name(),
        "scenario_diagnostics",
        Some(member),
    )?;
    let mut after = None;
    if let Some(token) = &request.page().cursor.0 {
        let bytes = hex::decode(token.as_str())
            .map_err(|_| WireError::Invalid("cursor encoding".into()))?;
        let cursor: Cursor = serde_json::from_slice(&bytes)?;
        if cursor.binding.section == expected.section {
            let cursor = Cursor::decode(token, &expected)?;
            let CursorPosition::ScenarioDiagnostic {
                association: parent,
                key,
            } = cursor.after
            else {
                return Err(WireError::Continuation(
                    "scenario diagnostic continuation required".into(),
                ));
            };
            if parent != association {
                return Err(WireError::Continuation(
                    "scenario diagnostic parent changed".into(),
                ));
            }
            after = Some(key);
        }
    }
    if let Some(after) = after
        && !values.iter().any(|(key, _)| *key == after)
    {
        return Err(WireError::Continuation(
            "scenario diagnostic position absent".into(),
        ));
    }
    values.sort_by_key(|(key, _)| *key);
    values.retain(|(key, _)| after.is_none_or(|after| *key > after));
    let omitted = values.len().saturating_sub(request.page().size as usize) as u64;
    values.truncate(request.page().size as usize);
    let continuation = if omitted > 0 {
        Optional(Some(
            Cursor {
                binding: expected,
                after: CursorPosition::ScenarioDiagnostic {
                    association,
                    key: values.last().expect("positive page size").0,
                },
            }
            .encode()?,
        ))
    } else {
        Optional::default()
    };
    Ok(SectionPage {
        availability: Availability::Available {},
        items: values.into_iter().map(|(_, value)| value).collect(),
        continuation,
        omitted,
        truncated: omitted > 0,
    })
}

pub fn validate(
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
) -> Result<(), WireError> {
    let Some(token) = &request.page().cursor.0 else {
        return Ok(());
    };
    let bytes =
        hex::decode(token.as_str()).map_err(|_| WireError::Invalid("cursor encoding".into()))?;
    let cursor: Cursor = serde_json::from_slice(&bytes)?;
    let tool = request.tool().name();
    let group = cursor.binding.group.as_str();
    let section = cursor.binding.section.as_str();
    let permitted = match request {
        Request::FindOperations(_) => {
            ["supported", "unresolved", "conflicting"].contains(&group) && section == "results"
        }
        Request::GetOperation(r) => {
            group == tool
                && match section {
                    "access_routes" => r.sections.contains(&OperationSection::AccessRoutes),
                    "callable_comparison" => {
                        r.sections.contains(&OperationSection::CallableComparison)
                    }
                    "contextual_typing" => r.sections.contains(&OperationSection::ContextualTyping),
                    "incoming_references" => {
                        r.sections.contains(&OperationSection::IncomingReferences)
                    }
                    "scenarios" => r.sections.contains(&OperationSection::Scenarios),
                    "scenario_diagnostics" => {
                        r.sections.contains(&OperationSection::Scenarios)
                            && matches!(cursor.after, CursorPosition::ScenarioDiagnostic { .. })
                    }
                    "deployment" => r.sections.contains(&OperationSection::Deployment),
                    "relationships" => r.sections.contains(&OperationSection::Relationships),
                    "conflicts" => r.sections.contains(&OperationSection::Conflicts),
                    "briefs" => r.sections.contains(&OperationSection::Briefs),
                    "behavior" => r.sections.contains(&OperationSection::Behavior),
                    _ => false,
                }
        }
        Request::GetEvidence(_) => {
            group == tool
                && [
                    "body",
                    "flow_inventory",
                    "source_characterization",
                    "derivation",
                ]
                .contains(&section)
        }
        Request::BrowseLibrary(_) => group == tool && section == "entries",
        Request::InspectValuePaths(r) => {
            group == tool && section == "paths" && cursor.binding.member == Some(r.member)
        }
        Request::SearchOperations(_)
        | Request::SearchEvidence(_)
        | Request::SearchCapabilities(_) => group == tool && section == "results",
        Request::CompareOperations(_) | Request::GetCapability(_) => false,
    };
    if !permitted {
        return Err(WireError::Continuation(
            "cursor group or section is not part of this request".into(),
        ));
    }
    let expected = binding(
        request,
        snapshot,
        channels,
        group,
        section,
        cursor.binding.member,
    )?;
    Cursor::decode(token, &expected)?;
    Ok(())
}

pub fn target_key(target: lctx_model::domain::serving::ranking::Target) -> ContentHash {
    use lctx_model::domain::graph;
    match target {
        ranking::Target::Member { member } => graph::EntityId::of(member).0,
        ranking::Target::Unit { unit } => graph::EntityId::of(unit).0,
    }
}
pub fn ranked<T: serde::Serialize>(
    values: Vec<(ranking::RankedHit, ContentHash, T)>,
    request: &Request,
    snapshot: &SnapshotHandle,
    channels: &ChannelState,
    retained: &mut crate::ranked_results::RankedRequest<'_>,
) -> Result<(SectionPage<T>, Vec<ranking::RankedHit>), WireError> {
    retained.page(values, request, snapshot, channels)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn handle() -> SnapshotHandle {
        SnapshotHandle {
            publication: lctx_model::domain::ContentHash::of(b"fixture-publication"),
            view: lctx_model::domain::ContentHash::of(b"fixture-view"),
            service_generation: lctx_model::domain::ContentHash::of(b"fixture-service_generation"),
            definition_epoch: lctx_model::domain::ContentHash::of(b"fixture-definition_epoch"),
            semantic: ContentHash::of(b"s"),
            realization: ContentHash::of(b"r"),
            database: DatabaseIdentity {
                namespace: Name::new("control").unwrap(),
                database: Name::new("snapshot").unwrap(),
            },
        }
    }
    fn request() -> Request {
        Request::FindOperations(FindOperationsRequest {
            library: Name::new("fixture").unwrap(),
            selection: SelectionInput::default(),
            page: PageRequest {
                size: 1,
                ..PageRequest::default()
            },
        })
    }
    fn channels() -> ChannelState {
        ChannelState {
            lexical: false,
            vector: VectorChannel::Disabled {},
        }
    }
    #[test]
    fn key_pages_resume_by_key_and_refuse_changed_handle_or_representation() {
        let h = handle();
        let c = channels();
        let r = request();
        let mut keys = [ContentHash::of(b"one"), ContentHash::of(b"two")];
        keys.sort();
        let first = page(
            vec![(keys[1], 2), (keys[0], 1)],
            &r,
            &h,
            &c,
            "supported",
            "results",
            None,
            Availability::Available {},
        )
        .unwrap();
        assert_eq!(first.items, vec![1]);
        assert_eq!(first.omitted, 1);
        let mut next = r.clone();
        let Request::FindOperations(v) = &mut next else {
            unreachable!()
        };
        v.page.cursor = first.continuation;
        validate(&next, &h, &c).unwrap();
        let second = page(
            vec![(keys[1], 2), (keys[0], 1)],
            &next,
            &h,
            &c,
            "supported",
            "results",
            None,
            Availability::Available {},
        )
        .unwrap();
        assert_eq!(second.items, vec![2]);
        assert!(!second.truncated);
        let mut changed = h.clone();
        changed.realization = ContentHash::of(b"different policy");
        assert!(validate(&next, &changed, &c).is_err());
        changed = h.clone();
        changed.semantic = ContentHash::of(b"different content");
        assert!(validate(&next, &changed, &c).is_err());
        changed = h.clone();
        changed.definition_epoch = ContentHash::of(b"different installed definition epoch");
        assert!(validate(&next, &changed, &c).is_err());
        changed = h.clone();
        changed.database.database = Name::new("different_database").unwrap();
        assert!(validate(&next, &changed, &c).is_err());
        let Request::FindOperations(v) = &mut next else {
            unreachable!()
        };
        v.page.expanded = true;
        assert!(validate(&next, &h, &c).is_err());
    }
    #[test]
    fn foreign_section_and_degraded_channels_are_bound_before_reading() {
        let h = handle();
        let c = channels();
        let mut r = request();
        let foreign_binding = binding(&r, &h, &c, "get_evidence", "body", None).unwrap();
        let token = Cursor {
            binding: foreign_binding,
            after: CursorPosition::Key {
                key: ContentHash::of(b"row"),
            },
        }
        .encode()
        .unwrap();
        let Request::FindOperations(v) = &mut r else {
            unreachable!()
        };
        v.page.cursor = Optional(Some(token));
        assert!(validate(&r, &h, &c).is_err());
        let mut search = Request::SearchEvidence(SearchEvidenceRequest {
            library: Optional::default(),
            query: QueryText::new("fixture").unwrap(),
            families: vec![],
            page: PageRequest::default(),
        });
        let disabled = ChannelState {
            lexical: true,
            vector: VectorChannel::Disabled {},
        };
        let binding = binding(&search, &h, &disabled, "search_evidence", "results", None).unwrap();
        let token = Cursor {
            binding,
            after: CursorPosition::Ranked {
                session: ContentHash::of(b"session"),
                result: ContentHash::of(b"result"),
                digest: ContentHash::of(b"rows"),
                offset: 1,
            },
        }
        .encode()
        .unwrap();
        let Request::SearchEvidence(v) = &mut search else {
            unreachable!()
        };
        v.page.cursor = Optional(Some(token));
        validate(&search, &h, &disabled).unwrap();
        let degraded = ChannelState {
            lexical: true,
            vector: VectorChannel::Degraded {
                reason: Name::new("embedding_service_unavailable").unwrap(),
            },
        };
        assert!(validate(&search, &h, &degraded).is_err());
    }
    #[test]
    fn remediation_second_scenario_child_cursor_keeps_its_parent_and_exact_member() {
        use lctx_model::domain::{
            Id,
            catalog::{CatalogMember, evidence::ScenarioAssociation},
        };
        fn id<T>(byte: u8) -> Id<T> {
            serde_json::from_value(serde_json::json!(vec![byte; 16])).unwrap()
        }
        let member: Id<CatalogMember> = id(3);
        let first: Id<ScenarioAssociation> = id(1);
        let second: Id<ScenarioAssociation> = id(2);
        let h = handle();
        let c = channels();
        let mut request = Request::GetOperation(GetOperationRequest {
            library: Name::new("fixture").unwrap(),
            operation: OperationSelector::Member { member },
            comparison: Optional::default(),
            reference_parameter: Optional::default(),
            sections: vec![OperationSection::Scenarios],
            page: PageRequest {
                size: 1,
                ..Default::default()
            },
        });
        let parents = || {
            vec![
                (ContentHash([2; 32]), (second, "second")),
                (ContentHash([1; 32]), (first, "first")),
            ]
        };
        let first_page = scenario_parents(parents(), &request, &h, &c, member).unwrap();
        assert_eq!(first_page.items, vec![(first, "first")]);
        let Request::GetOperation(r) = &mut request else {
            unreachable!()
        };
        r.page.cursor = first_page.continuation;
        let second_page = scenario_parents(parents(), &request, &h, &c, member).unwrap();
        assert_eq!(second_page.items, vec![(second, "second")]);
        let children = || {
            vec![
                (ContentHash([8; 32]), "last"),
                (ContentHash([7; 32]), "next"),
                (ContentHash([6; 32]), "start"),
            ]
        };
        let child_page =
            scenario_diagnostics(children(), second, &request, &h, &c, member).unwrap();
        assert_eq!(child_page.items, vec!["start"]);
        let Request::GetOperation(r) = &mut request else {
            unreachable!()
        };
        r.page.cursor = child_page.continuation;
        validate(&request, &h, &c).unwrap();
        let parent = scenario_parents(parents(), &request, &h, &c, member).unwrap();
        assert_eq!(parent.items, vec![(second, "second")]);
        assert!(scenario_parents(parents(), &request, &h, &c, id(4)).is_err());
        assert!(
            scenario_parents(
                vec![(ContentHash([1; 32]), (first, "first"))],
                &request,
                &h,
                &c,
                member
            )
            .is_err()
        );
        assert!(scenario_diagnostics(children(), first, &request, &h, &c, member).is_err());
        let next = scenario_diagnostics(children(), second, &request, &h, &c, member).unwrap();
        assert_eq!(next.items, vec!["next"]);
        let Request::GetOperation(r) = &mut request else {
            unreachable!()
        };
        r.page.cursor = next.continuation;
        let last = scenario_diagnostics(children(), second, &request, &h, &c, member).unwrap();
        assert_eq!(last.items, vec!["last"]);
        assert!(last.continuation.0.is_none());
    }
    #[test]
    fn bounded_collector_matches_full_stable_pages_with_duplicate_keys() {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let values = (0..4096u32).rev().map(|ordinal| (ContentHash::of(&(ordinal % 23).to_le_bytes()), format!("value-{ordinal:04}"))).collect::<Vec<_>>();
        let mut request = request();
        let handle = handle();
        let channels = ChannelState { lexical: false, vector: VectorChannel::Disabled {} };
        loop {
            let expected = page(values.clone(), &request, &handle, &channels, "supported", "results", None, Availability::Available {}).unwrap();
            let mut collector = PageCollector::new(&request, &handle, &channels, "supported", "results", None, &budget);
            for (key, value) in &values { collector.push(*key, value.clone()).unwrap(); }
            let (actual, charge) = collector.finish(Availability::Available {}).unwrap();
            assert_eq!(actual, expected, "stable duplicate-key order, omission and cursor must agree");
            assert_eq!(charge.reserved() > 0, !actual.items.is_empty(),
                "only retained values require page storage, including an empty strict-key continuation");
            drop(charge);
            assert_eq!(budget.reserved(), 0);
            let Some(cursor) = actual.continuation.0 else { break; };
            let Request::FindOperations(r) = &mut request else { unreachable!() };
            r.page.cursor = Optional(Some(cursor));
        }
    }
    #[test]
    fn packet_key_streams_exact_json_and_collector_defers_invalid_cursor() {
        let value = serde_json::json!({"escaped":"quote\" slash\\ snow☃", "nested":[null, false, 1.25]});
        assert_eq!(packet_key(&value).unwrap(), ContentHash::of(&serde_json::to_vec(&value).unwrap()));
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut request = request();
        let Request::FindOperations(r) = &mut request else { unreachable!() };
        r.page.cursor = Optional(Some(CursorToken::new("zz").unwrap()));
        let mut collector = PageCollector::new(&request, &handle(), &ChannelState { lexical:false, vector:VectorChannel::Disabled {} }, "supported", "results", None, &budget);
        collector.push(ContentHash::of(b"value"), value).unwrap();
        assert!(collector.finish(Availability::Available {}).is_err());
        assert_eq!(budget.reserved(), 0);
    }

}
