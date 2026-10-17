//! Operation-local exact selectors over independently audited protected contributors.
use super::*;
use d::{
    charged::StateCharge,
    completion::{Completion, complete},
    resources::ResourceBudget,
};
use std::{collections::BTreeMap, sync::Arc};
use surrealdb::{
    Surreal,
    engine::remote::grpc::Client,
    types::{Object, Variables},
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SelectedViewDescriptor {
    pub view: d::completed::CompletedView,
    pub mappings: Vec<ContributorRealization>,
}
/// Actual immutable rows and their pin, never a serialized grant. Exact descriptors
/// are retained in ExpectedRecord bytes, with normalized preparation owned alongside.
pub struct EarnedContributionSet {
    pin: Arc<crate::control::ReaderPin>,
    context_lifetime: Option<super::audit::CheckedContextLifetime>,
    owner: RecordId,
    generation: ContentHash,
    validity_epoch: i64,
    contributors: BTreeMap<ContentHash, Arc<crate::control::ExpectedRecord>>,
    selector: ContentHash,
    members: crate::ordered_rows::PreparedPointRows,
    _prepared: Vec<super::audit::LogicalContributorRows>,
    pub(super) _charge: StateCharge,
}
impl EarnedContributionSet {
    #[cfg(test)]
    pub(super) fn has_checked_context_lifetime(&self) -> bool {
        self.context_lifetime.is_some()
    }
    pub fn owner(&self) -> &RecordId {
        &self.owner
    }
    pub(crate) fn contributors(
        &self,
    ) -> &BTreeMap<ContentHash, Arc<crate::control::ExpectedRecord>> {
        &self.contributors
    }
    pub(crate) fn pin(&self) -> &Arc<crate::control::ReaderPin> {
        &self.pin
    }
    pub(crate) fn context(&self) -> (ContentHash, i64) {
        (self.generation, self.validity_epoch)
    }
    pub fn retain_unknown(&self) {
        self.pin.retain_unknown();
    }
    /// Borrow already audited immutable rows beneath their actual source pin.
    /// Independent receiving owners still perform full exact transfer/current checks.
    pub(crate) async fn ensure_borrowed_current(
        &self,
        reader: &crate::NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if let Some(context) = &self.context_lifetime {
            return context.check(reader, budget);
        }
        self.pin.ensure_live()?;
        let _context = budget.reserve("earned borrowed current installation observation", 8192)?;
        let installations: Vec<Object> = reader
            .query_native(
                "SELECT generation,validity_epoch FROM native_installation:current",
                Variables::new(),
            )
            .await?;
        let [installation] = installations.as_slice() else {
            return Err(ModelError::Conflict(
                "earned current installation cardinality",
            ));
        };
        if installation.get("generation") != Some(&Value::String(self.generation.hex()))
            || installation.get("validity_epoch")
                != Some(&Value::Number(surrealdb::types::Number::Int(
                    self.validity_epoch,
                )))
        {
            return Err(ModelError::Conflict("earned current installation changed"));
        }
        self.pin.ensure_live()
    }
    pub(crate) async fn ensure_current(
        &self,
        reader: &crate::NativeReader<()>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.pin.ensure_live()?;
        if let Some(context) = &self.context_lifetime {
            context.check(reader, budget)?;
        } else {
            let installations: Vec<Object> = reader
                .query_native(
                    "SELECT generation,validity_epoch FROM native_installation:current",
                    Variables::new(),
                )
                .await?;
            let [installation] = installations.as_slice() else {
                return Err(ModelError::Conflict("earned current installation cardinality"));
            };
            if installation.get("generation") != Some(&Value::String(self.generation.hex()))
                || installation.get("validity_epoch") != Some(&Value::Number(surrealdb::types::Number::Int(self.validity_epoch)))
            {
                return Err(ModelError::Conflict("earned current installation changed"));
            }
        }
        let _requests = budget.reserve(
            "earned exact current contributor windows",
            self.contributors
                .len()
                .saturating_mul(2048)
                .saturating_add(4096),
        )?;
        let requested = self
            .contributors
            .values()
            .map(|record| (record.record().clone(), record))
            .collect::<BTreeMap<_, _>>();
        let ids = requested.keys().cloned().collect::<Vec<_>>();
        for ids in ids.chunks(crate::loader::NATIVE_WINDOW_ROWS) {
            let mut actual =
                crate::control::inspect_expected_window_read(reader, ids, budget).await?;
            for id in ids {
                let row = actual.take_required(id)?;
                let expected = requested
                    .get(id)
                    .ok_or(ModelError::Conflict("earned exact contributor absent"))?;
                if row.incarnation() != expected.incarnation() || row.row() != expected.row() {
                    return Err(ModelError::Conflict(
                        "earned contributor incarnation/full bytes changed",
                    ));
                }
            }
            actual.finish()?;
            let pairs = ids
                .iter()
                .map(|id| (self.owner.clone(), id.clone()))
                .collect::<Vec<_>>();
            let mut holds =
                crate::control::inspect_hold_window_read(reader, &pairs, budget).await?;
            for (owner, object) in &pairs {
                holds.take_required(owner, object)?;
            }
            holds.finish()?;
        }
        if let Some(context) = &self.context_lifetime {
            context.check(reader, budget)
        } else {
            self.pin.ensure_live()
        }
    }
}
impl HeldInstalledView {
    /// Rebind only immutable, already protected preparation to a completely earned
    /// receiving pin. No source session, lease or pin owner is retained by the copy.
    pub(super) fn receiving_copy(
        &self,
        pin: &Arc<crate::control::ReaderPin>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        self.require_pool(budget)?;
        pin.ensure_live()?;
        let owner = RecordId::new("native_pin", pin.identity.hex());
        let mut charge = StateCharge::new(budget, "receiving independent exact selector");
        charge.grow(std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>())?;
        let (selector, owner) = match &self.owner {
            InstalledViewOwner::Earned { set } => {
                charge.grow(super::view_heap(&self.selector.view).saturating_add(
                    self.selector.mappings.len().saturating_mul(std::mem::size_of::<ContributorRealization>())))?;
                let mut state = StateCharge::new(budget, "receiving immutable contributor preparation");
                state.grow(set.contributors.len().saturating_mul(1024)
                    .saturating_add(set._prepared.len().saturating_mul(std::mem::size_of::<super::audit::LogicalContributorRows>()))
                    .saturating_add(std::mem::size_of::<EarnedContributionSet>() + 512))?;
                let copied = EarnedContributionSet {
                    pin: pin.clone(), owner,
                    // A new receiving pin cannot inherit the source's checked
                    // context lifetime. Its existing fresh remote route remains.
                    context_lifetime: None,
                    generation: set.generation, validity_epoch: set.validity_epoch,
                    contributors: set.contributors.clone(), selector: set.selector,
                    members: set.members.clone(), _prepared: set._prepared.clone(), _charge: state,
                };
                (HeldSelector::Earned(self.completion().clone()), InstalledViewOwner::Earned { set: Arc::new(copied) })
            }
            InstalledViewOwner::Physical { expected, completed, .. } => (
                HeldSelector::Retained(self.retained_completion()?.clone()),
                InstalledViewOwner::Recovery { expected: expected.clone(), inventory: completed.inventory.clone(), pin: pin.clone(), owner },
            ),
            InstalledViewOwner::Frozen { expected, completed, .. } => (
                HeldSelector::Retained(self.retained_completion()?.clone()),
                InstalledViewOwner::Recovery { expected: expected.clone(), inventory: completed.inventory.clone(), pin: pin.clone(), owner },
            ),
            InstalledViewOwner::Published { expected, completed, .. } => (
                HeldSelector::Retained(self.retained_completion()?.clone()),
                InstalledViewOwner::Recovery { expected: expected.clone(), inventory: completed.shared_inventory(), pin: pin.clone(), owner },
            ),
            InstalledViewOwner::Recovery { expected, inventory, .. } => (
                HeldSelector::Retained(self.retained_completion()?.clone()),
                InstalledViewOwner::Recovery { expected: expected.clone(), inventory: inventory.clone(), pin: pin.clone(), owner },
            ),
        };
        Ok(Self { client: pin.client().clone(), selector, owner, _charge: charge })
    }
    pub fn is_internal(&self) -> bool {
        matches!(&self.owner, InstalledViewOwner::Earned { .. })
    }
}

pub async fn select_live_view_scoped(
    client: Arc<Surreal<Client>>,
    attempt: ContentHash,
    view: &d::completed::CompletedView,
    mappings: &[ContributorRealization],
    budget: &ResourceBudget,
    audit: &InstallationAudit,
) -> Result<HeldInstalledView, ModelError> {
    let reader = crate::NativeReader::private(client.clone()).with_budget(budget);
    let result = async {
        audit.ensure_context(&reader, budget).await?;
        let owner = crate::control::live_view::capture(&reader, attempt, budget).await?;
        let closure =
            super::audit::inspect_closure_in_audit(&reader, view, mappings, budget, audit).await?;
        super::audit::audit_memberships_scoped(&reader, &closure, budget, audit).await?;
        // Contributor-local alias integrity is reusable; composed exact-set reachability
        // is checked before selector authority is published, including cross-owner paths.
        super::audit::audit_aliases_scoped(&reader, &closure, budget, audit).await?;
        let prepared =
            super::audit::prepare_closure_scoped(&reader, &closure, budget, audit).await?;
        let members =
            super::audit::selected_prepared_members(&closure, view, budget, audit).await?;
        let selector = super::realization_id(view, mappings)?;
        let members = members.point_index(budget)?;
        let mut charge = StateCharge::new(budget, "earned exact contributor capability");
        charge.grow(
                closure
                    .contributors
                    .len()
                    .saturating_mul(1024)
                    .saturating_add(
                        prepared.len().saturating_mul(std::mem::size_of::<
                            super::audit::LogicalContributorRows,
                        >()),
                    )
                    .saturating_add(4096),
            )?;
        let set = Arc::new(EarnedContributionSet {
            pin: audit.borrow_pin()?,
            context_lifetime: Some(audit.checked_context_lifetime(&reader, budget)?),
            owner: RecordId::new("native_pin", audit.pin_identity().hex()),
            generation: owner.generation(),
            validity_epoch: owner.validity_epoch(),
            contributors: super::audit::protected_contributors(&closure, audit)?,
            selector,
            members,
            _prepared: prepared,
            _charge: charge,
        });
        // Native metadata, whole outputs and selected membership were independently
        // checked under this actual operation pin. Receiving readers earn their own
        // exact current authority; publishing another selector retains this pin.
        audit.ensure_context(&reader, budget).await?;
        set.pin.ensure_live()?;
        let mut charge = StateCharge::new(budget, "internal exact view selector");
        charge.grow(
            std::mem::size_of::<HeldInstalledView>()
                .saturating_add(2 * std::mem::size_of::<usize>())
                .saturating_add(super::view_heap(view))
                .saturating_add(
                    mappings
                        .len()
                        .saturating_mul(std::mem::size_of::<ContributorRealization>()),
                ),
        )?;
        Ok(HeldInstalledView {
            client: client.clone(),
            selector: HeldSelector::Earned(SelectedViewDescriptor {
                view: view.clone(),
                mappings: mappings.to_vec(),
            }),
            owner: InstalledViewOwner::Earned { set },
            _charge: charge,
        })
    }
    .await;
    let mut terminal = Completion::default();
    terminal.step("internal selected view reader close", reader.close().await);
    let result = complete(result, terminal);
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        audit.retain_unknown();
    }
    result
}

pub(super) async fn selected_members(
    cap: &HeldInstalledView,
    reader: &crate::NativeReader<()>,
    keys: &[[u8; 16]],
    budget: &ResourceBudget,
) -> Result<super::live::SelectedMembers, ModelError> {
    if let InstalledViewOwner::Earned { set } = &cap.owner {
        return prepared_selected_members(cap, set, reader, keys, budget).await;
    }
    let mut charge = StateCharge::new(budget, "canonical selected membership result");
    let mut members = BTreeMap::<[u8; 16], super::live::InstalledMember>::new();
    let mut mapping_index = 0usize;
    let mut key_index = 0usize;
    loop {
        let mut window_charge =
            StateCharge::new(budget, "selected canonical membership exact request window");
        let mut expected = BTreeMap::new();
        while expected.len() < crate::loader::NATIVE_WINDOW_ROWS {
            if keys.is_empty() {
                break;
            }
            while mapping_index < cap.selector.mappings.len()
                && cap.selector.mappings[mapping_index].view != cap.selector.view.identity
            {
                mapping_index += 1;
            }
            let Some(mapping) = cap.selector.mappings.get(mapping_index) else {
                break;
            };
            let physical = mapping.physical;
            let key = keys[key_index];
            key_index += 1;
            if key_index == keys.len() {
                key_index = 0;
                mapping_index += 1;
            }
            window_charge.grow(2048)?;
            let id =
                crate::completed_rows::membership_id(physical, &cap.selector.view.relation, &key);
            if expected.insert(id, (physical, key)).is_some() {
                return Err(ModelError::Conflict(
                    "selected duplicate physical membership demand",
                ));
            }
        }
        if expected.is_empty() {
            break;
        }
        let ids = expected.keys().cloned().collect::<Vec<_>>();
        let mut actual = crate::control::inspect_expected_window_read(reader, &ids, budget).await?;
        for (id, (physical, key)) in expected {
            let Some(row) = actual.take_optional(&id)? else {
                continue;
            };
            let member =
                decode_selected_member(row.row(), physical, &cap.selector.view.relation, key)?;
            if let Some(previous) = members.get(&key) {
                if previous != &member {
                    return Err(ModelError::Conflict(
                        "canonical selector conflicting nominal membership",
                    ));
                }
            } else {
                charge.grow(
                    crate::selection::node_bytes(&member.node)
                        .saturating_mul(2)
                        .saturating_add(512),
                )?;
                members.insert(key, member);
            }
        }
        actual.finish()?;
    }
    cap.ensure_borrowed_current(reader, budget).await?;
    charge.grow(
        members
            .len()
            .saturating_mul(std::mem::size_of::<super::live::InstalledMember>()),
    )?;
    Ok(super::live::SelectedMembers {
        rows: members.into_values().collect(),
        _charge: charge,
    })
}
async fn prepared_selected_members(
    cap: &HeldInstalledView,
    set: &EarnedContributionSet,
    reader: &crate::NativeReader<()>,
    keys: &[[u8; 16]],
    budget: &ResourceBudget,
) -> Result<super::live::SelectedMembers, ModelError> {
    cap.ensure_borrowed_current(reader, budget).await?;
    let mut charge = StateCharge::new(budget, "earned prepared selected member result");
    charge.grow(
        keys.len()
            .saturating_mul(std::mem::size_of::<super::live::InstalledMember>()),
    )?;
    let mut rows = Vec::with_capacity(keys.len());
    let mut points = set.members.cursor(budget)?;
    for key in keys {
        if !reader.holds_installed_premise(cap, budget)? {
            set.pin.ensure_live()?;
        }
        let id = super::member_id(set.selector, &cap.selector.view.relation, key);
        let Some(row) = points.get(&id)? else {
            continue;
        };
        // Admission precedes extracting owned node/content fields. The borrowed
        // decoded row remains in the point cursor until this iteration finishes.
        charge.grow(
            crate::loader::native_bytes(row)
                .saturating_mul(2)
                .saturating_add(512),
        )?;
        rows.push(decode_prepared_member(
            row,
            set.selector,
            &cap.selector.view.relation,
            *key,
        )?);
    }
    cap.ensure_borrowed_current(reader, budget).await?;
    Ok(super::live::SelectedMembers {
        rows,
        _charge: charge,
    })
}
fn decode_prepared_member(
    row: &Value,
    selector: ContentHash,
    relation: &str,
    key: [u8; 16],
) -> Result<super::live::InstalledMember, ModelError> {
    let object = row
        .as_object()
        .ok_or(ModelError::Schema("prepared selected membership object"))?;
    let Some(Value::RecordId(node)) = object.get("node") else {
        return Err(ModelError::Schema("prepared selected membership node"));
    };
    let Some(Value::String(encoded_content)) = object.get("content") else {
        return Err(ModelError::Schema("prepared selected membership content"));
    };
    if object.len() != 6
        || object.get("id") != Some(&Value::RecordId(super::member_id(selector, relation, &key)))
        || object.get("selector") != Some(&Value::String(selector.hex()))
        || object.get("relation") != Some(&Value::String(relation.into()))
        || object.get("semantic_key") != Some(&Value::String(hex::encode(key)))
    {
        return Err(ModelError::Conflict(
            "prepared selected exact selector/relation/key/envelope",
        ));
    }
    let content = ContentHash(
        hex::decode(encoded_content)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("prepared selected content width"))?,
    );
    if content.hex() != *encoded_content {
        return Err(ModelError::Conflict("prepared selected canonical content"));
    }
    Ok(super::live::InstalledMember {
        key,
        node: node.clone(),
        content,
    })
}
fn decode_selected_member(
    row: &Value,
    physical: ContentHash,
    relation: &str,
    key: [u8; 16],
) -> Result<super::live::InstalledMember, ModelError> {
    crate::completed_rows::validate_state_row("compiler_membership", row)?;
    let object = row
        .as_object()
        .ok_or(ModelError::Schema("canonical selected membership object"))?;
    if object.get("contribution")
        != Some(&Value::RecordId(RecordId::new(
            "compiler_contribution",
            physical.hex(),
        )))
        || object.get("relation") != Some(&Value::String(relation.to_owned()))
        || object.get("semantic_key") != Some(&Value::String(hex::encode(key)))
        || object.get("id")
            != Some(&Value::RecordId(crate::completed_rows::membership_id(
                physical, relation, &key,
            )))
    {
        return Err(ModelError::Conflict(
            "canonical selected exact owner/relation/key",
        ));
    }
    let Some(Value::RecordId(node)) = object.get("node") else {
        return Err(ModelError::Schema("canonical selected node"));
    };
    let Some(Value::String(content)) = object.get("content") else {
        return Err(ModelError::Schema("canonical selected content"));
    };
    let content = ContentHash(
        hex::decode(content)
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("canonical selected content width"))?,
    );
    let mut canonical = Object::new();
    canonical.insert(
        "id",
        crate::completed_rows::membership_id(physical, relation, &key),
    );
    canonical.insert(
        "contribution",
        RecordId::new("compiler_contribution", physical.hex()),
    );
    canonical.insert("relation", relation.to_owned());
    canonical.insert("semantic_key", hex::encode(key));
    canonical.insert("node", node.clone());
    canonical.insert("content", content.hex());
    if row != Value::Object(canonical) {
        return Err(ModelError::Conflict(
            "canonical selected exact membership envelope",
        ));
    }
    Ok(super::live::InstalledMember {
        key,
        node: node.clone(),
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receiving_copy_shares_immutable_preparation_without_retaining_source_pin() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let source_pin = crate::control::ReaderPin::test_detached(false);
        let source_state = source_pin.factory_state();
        let receiver = crate::control::ReaderPin::test_detached(false);
        let prepared = crate::ordered_rows::SortedRows::with_budget(&budget).unwrap().finish().unwrap().into_prepared();
        let mut state = StateCharge::new(&budget, "receiver copy control state");
        state.grow(4096).unwrap();
        let mut handle = StateCharge::new(&budget, "receiver copy control handle");
        handle.grow(4096).unwrap();
        let source = HeldInstalledView {
            client: source_pin.client().clone(),
            selector: HeldSelector::Earned(SelectedViewDescriptor {
                view: d::completed::CompletedView::new("packages".into(), [ContentHash([1; 32])].into(), 0).unwrap(),
                mappings: vec![],
            }),
            owner: InstalledViewOwner::Earned { set: Arc::new(EarnedContributionSet {
                pin: source_pin.clone(), owner: RecordId::new("native_pin", source_pin.identity.hex()),
                context_lifetime: None,
                generation: ContentHash([2; 32]), validity_epoch: 1, contributors: BTreeMap::new(),
                selector: ContentHash([3; 32]), members: prepared.point_index(&budget).unwrap(),
                _prepared: vec![], _charge: state,
            }) },
            _charge: handle,
        };
        let received = source.receiving_copy(&receiver, &budget).unwrap();
        source_pin.retain_unknown();
        received.check_borrowed_local(receiver.client(), &budget).unwrap();
        assert!(received.check_borrowed_local(source_pin.client(), &budget).is_err());
        let foreign_pool = ResourceBudget::fixed(1 << 20).unwrap();
        assert!(received.check_borrowed_local(receiver.client(), &foreign_pool).is_err());
        assert!(source.receiving_copy(&crate::control::ReaderPin::test_detached(true), &budget).is_err());
        drop(source);
        drop(source_pin);
        assert!(!source_state.has_borrower(), "receiving data must not retain the source factory borrower");
        received.check_borrowed_local(receiver.client(), &budget).unwrap();
        receiver.retain_unknown();
        assert!(received.check_borrowed_local(receiver.client(), &budget).is_err());
    }
    fn member(owner: ContentHash, key: [u8; 16]) -> Value {
        let mut row = Object::new();
        row.insert(
            "id",
            crate::completed_rows::membership_id(owner, "packages", &key),
        );
        row.insert(
            "contribution",
            RecordId::new("compiler_contribution", owner.hex()),
        );
        row.insert("relation", "packages");
        row.insert("semantic_key", hex::encode(key));
        row.insert("node", RecordId::new("entity", ContentHash([3; 32]).hex()));
        row.insert("content", ContentHash([4; 32]).hex());
        Value::Object(row)
    }
    #[test]
    fn selected_canonical_members_refuse_foreign_owner_relation_key_and_extra_payload() {
        let owner = ContentHash([1; 32]);
        let key = [2; 16];
        let row = member(owner, key);
        assert_eq!(
            decode_selected_member(&row, owner, "packages", key)
                .unwrap()
                .key,
            key
        );
        assert!(decode_selected_member(&row, ContentHash([9; 32]), "packages", key).is_err());
        assert!(decode_selected_member(&row, owner, "other", key).is_err());
        assert!(decode_selected_member(&row, owner, "packages", [8; 16]).is_err());
        let mut extra = row.clone();
        if let Value::Object(row) = &mut extra {
            row.insert("unexpected", true);
        }
        assert!(decode_selected_member(&extra, owner, "packages", key).is_err());
        let mut corrupt = row;
        if let Value::Object(row) = &mut corrupt {
            row.insert("content", "broken");
        }
        assert!(decode_selected_member(&corrupt, owner, "packages", key).is_err());
    }
    #[test]
    fn prepared_members_preserve_exact_selector_key_content_and_envelope() {
        let selector = ContentHash([8; 32]);
        let key = [2; 16];
        let row =
            crate::completed_state::earning::member(selector, &member(ContentHash([1; 32]), key))
                .unwrap();
        let decoded = decode_prepared_member(&row, selector, "packages", key).unwrap();
        assert_eq!(decoded.key, key);
        assert_eq!(
            decoded.node,
            RecordId::new("entity", ContentHash([3; 32]).hex())
        );
        assert_eq!(decoded.content, ContentHash([4; 32]));
        assert!(decode_prepared_member(&row, ContentHash([9; 32]), "packages", key).is_err());
        assert!(decode_prepared_member(&row, selector, "other", key).is_err());
        assert!(decode_prepared_member(&row, selector, "packages", [7; 16]).is_err());
        let mut extra = row.clone();
        if let Value::Object(row) = &mut extra {
            row.insert("extra", true);
        }
        assert!(decode_prepared_member(&extra, selector, "packages", key).is_err());
        let mut corrupt = row;
        if let Value::Object(row) = &mut corrupt {
            row.insert("content", "broken");
        }
        assert!(decode_prepared_member(&corrupt, selector, "packages", key).is_err());
    }
}
