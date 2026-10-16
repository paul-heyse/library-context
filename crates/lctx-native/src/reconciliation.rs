//! Full canonical and physical readback before a database can become a published snapshot.
use crate::{Loader, codec};
use lctx_model::domain::{
    ModelError,
    graph::{Assertion, Entity, FamilyHasher, GraphFamily, Manifest, Target},
};
use surrealdb::types::{Bytes, RecordId, SurrealValue, Value, Variables};
fn canonical_size(actual: &Value) -> Result<usize, ModelError> {
    match actual {
        Value::Object(object) => match object.get("canonical") {
            Some(Value::Bytes(bytes)) => Ok(bytes.len()),
            _ => Err(ModelError::Schema("native canonical payload")),
        },
        _ => Err(ModelError::Schema("native canonical object")),
    }
}
fn decode_entity(actual: &Value) -> Result<Entity, ModelError> {
    let node = Node::from_value(actual.clone())
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))?;
    serde_json::from_slice(&node.canonical)
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))
}
fn decode_assertion(actual: &Value) -> Result<Assertion, ModelError> {
    let node = Node::from_value(actual.clone())
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))?;
    serde_json::from_slice(&node.canonical)
        .map_err(|_| ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct Node {
    id: RecordId,
    canonical: Bytes,
}
impl Loader {
    #[allow(
        clippy::mutable_key_type,
        reason = "SDK RecordId includes regex caches; these generated string IDs are never mutated"
    )]
    pub async fn reconcile(&self, manifest: &Manifest) -> Result<(), ModelError> {
        self.reconcile_owned(manifest, None).await
    }
    pub(crate) async fn reconcile_owned(
        &self,
        manifest: &Manifest,
        inventory: Option<&mut crate::ordered_rows::SortedRows>,
    ) -> Result<(), ModelError> {
        manifest.validate()?;
        let budget = self.read_budget()?;
        let mut expected = crate::ordered_rows::SortedRows::with_budget(&budget)?;
        let reader = self.reader();
        for (table, family) in [
            ("entity", GraphFamily::Entities),
            ("assertion", GraphFamily::Assertions),
        ] {
            let mut rows = reader.selected_payload_rows(
                table,
                "true",
                Variables::new(),
                vec![],
                "anchor,id",
                None,
            )?;
            let mut hasher = FamilyHasher::new(family);
            let limits = lctx_model::domain::batching::TransferLimits::default();
            let mut pending = rows.next().await?;
            while let Some(first) = pending.take() {
                let mut window = vec![first];
                let mut bytes = canonical_size(&window[0])?;
                while window.len() < limits.rows && bytes < limits.bytes {
                    let Some(next) = rows.next().await? else {
                        break;
                    };
                    let next_bytes = canonical_size(&next)?;
                    if bytes.saturating_add(next_bytes) > limits.bytes {
                        pending = Some(next);
                        break;
                    }
                    bytes += next_bytes;
                    window.push(next);
                }
                let entities = if family == GraphFamily::Entities {
                    window
                        .iter()
                        .map(decode_entity)
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    vec![]
                };
                let assertions = if family == GraphFamily::Assertions {
                    window
                        .iter()
                        .map(decode_assertion)
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    vec![]
                };
                let views = if family == GraphFamily::Entities {
                    codec::entity_views(&entities)?
                } else {
                    codec::assertion_views(&assertions)?
                };
                for (index, (actual, view)) in window.into_iter().zip(views).enumerate() {
                    let row = Node::from_value(actual.clone()).map_err(|_| {
                        ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)
                    })?;
                    let (key, content, kind, subtype, view, canonical) = if family
                        == GraphFamily::Entities
                    {
                        let entity = &entities[index];
                        entity.validate().map_err(crate::reader::canonical_error)?;
                        for (position, reference) in
                            codec::entity_references(entity).into_iter().enumerate()
                        {
                            let target = lctx_model::domain::graph::reference_target(&reference)?.0;
                            remember_external(&mut expected, &target)?;
                            expected.push(self.reference_value(crate::loader::edge(
                                "reference",
                                row.id.clone(),
                                target,
                                reference.field,
                                0,
                                Some(position as u32),
                            )?)?)?;
                        }
                        (
                            entity.id().0,
                            entity.content(),
                            entity.kind() as i64,
                            entity.subtype(),
                            view,
                            serde_json::to_vec(&entity).map_err(ModelError::codec)?,
                        )
                    } else {
                        let assertion = &assertions[index];
                        assertion
                            .validate()
                            .map_err(crate::reader::canonical_error)?;
                        for p in &assertion.participants {
                            remember_external(&mut expected, &p.target)?;
                            expected.push(self.reference_value(crate::loader::edge(
                                "participant",
                                row.id.clone(),
                                p.target.clone(),
                                p.field.as_deref().unwrap_or(""),
                                p.role as i64,
                                p.position,
                            )?)?)?;
                        }
                        for (position, (target, _)) in
                            assertion.references()?.into_iter().enumerate()
                        {
                            if assertion.participants.iter().any(|p| p.target == target) {
                                continue;
                            }
                            remember_external(&mut expected, &target)?;
                            expected.push(self.reference_value(crate::loader::edge(
                                "participant",
                                row.id.clone(),
                                target,
                                "__reference",
                                -1,
                                Some(position as u32),
                            )?)?)?;
                        }
                        (
                            assertion.id().0,
                            assertion.content(),
                            assertion.kind as i64,
                            None,
                            view,
                            serde_json::to_vec(&assertion).map_err(ModelError::codec)?,
                        )
                    };
                    let physical = crate::adapter::physical_row(
                        crate::loader::payload_id(table, &view.semantic_type, &key.0, content)?,
                        content,
                        canonical,
                        Some((kind, subtype)),
                        view,
                    )?;
                    if serde_json::to_vec(&actual).map_err(ModelError::codec)?
                        != serde_json::to_vec(&physical).map_err(ModelError::codec)?
                    {
                        return Err(ModelError::Serving(
                            lctx_model::domain::serving::FailureKind::Corrupt,
                        ));
                    }
                    if !hasher.push(key, content)? {
                        return Err(ModelError::Conflict(
                            "duplicate native canonical graph element",
                        ));
                    }
                }
                if pending.is_none() {
                    pending = rows.next().await?;
                }
            }
            if !manifest.families.contains(&hasher.finish()) {
                return Err(ModelError::Conflict(
                    "native canonical family reconciliation",
                ));
            }
        }
        let mut actual = actual_roles(&reader, &budget)?;
        let mut expected = expected.finish()?;
        if let Some(inventory) = inventory {
            while let Some(row) = expected.next_row()? {
                let Value::Object(object) = &row else {
                    return Err(ModelError::Schema("expected derived role inventory"));
                };
                if matches!(object.get("id"),Some(Value::RecordId(id)) if crate::derivation::FAMILIES.contains(&id.table.as_str()))
                {
                    crate::control::staging::remember_expected_child(inventory, &row)?;
                }
            }
            expected.rewind()?;
        }
        expected.reconcile(&mut actual).await?;
        if self.view_ids().is_none()
            && table_count(self, "original").await? != manifest.originals.len() as u64
        {
            return Err(ModelError::Conflict("native original inventory"));
        }
        let mut blobs = std::collections::BTreeSet::new();
        let mut original_chunks = 0u64;
        for original in &manifest.originals {
            self.check_read_admission()?;
            let id = RecordId::new("original", original.source.0.hex());
            let layout = crate::originals::layout(&reader, &id)
                .await?
                .ok_or(ModelError::Conflict("native original layout"))?;
            crate::originals::validate_layout(&reader, &layout).await?;
            if layout.blob.content != original.content || layout.blob.byte_len != original.byte_len
            {
                return Err(ModelError::Conflict("native original declaration"));
            }
            if !crate::originals::header(&reader, &layout.blob).await? {
                return Err(ModelError::Conflict("native original blob incomplete"));
            }
            if blobs.insert(layout.blob.id()) {
                // Reconciliation independently replays actual bytes. A producer completion
                // marker never substitutes for this admission boundary.
                crate::originals::validate_blob(&reader, &layout.blob, &budget).await?;
                original_chunks = original_chunks
                    .checked_add(layout.blob.piece_count())
                    .ok_or(ModelError::Schema("native original piece inventory"))?;
            }
        }
        if self.view_ids().is_none()
            && (table_count(self, "original_blob").await? != blobs.len() as u64
                || table_count(self, "original_chunk").await? != original_chunks)
        {
            return Err(ModelError::Conflict("native original blob inventory"));
        }
        Ok(())
    }
}
/// Actual outgoing rows are nominated by selected sources, never by expected role IDs.
pub fn actual_roles<Context>(
    reader: &crate::NativeReader<Context>,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<crate::reader::NativeRows, ModelError> {
    let namespace = reader.derivation()?;
    let selection = reader.selection_preparation();
    let client = reader.shared_client();
    let cancellation = reader.read_cancellation();
    let budget = budget.clone();
    crate::reader::NativeRows::owned(budget.clone(), move |sender| async move {
        let selection = selection.await?;
        let mut actual = crate::ordered_rows::SortedRows::with_budget(&budget)?;
        let mut external = crate::ordered_rows::SortedRows::with_budget(&budget)?;
        for table in ["participant", "reference"] {
            let mut rows = if let Some(selection) = &selection {
                selection.outgoing(table)?
            } else {
                crate::prepared::PreparedQuery::new(
                    {
                        let mut vars = Variables::new();
                        vars.insert("lctx_derivation", namespace.hex());
                        vars
                    },
                    vec![],
                    vec![format!(
                        "SELECT * FROM {table} WHERE derivation=$lctx_derivation"
                    )],
                )?
                .stream_cancellable(&client, cancellation.as_ref(), &budget)?
            };
            let result = async {
                while let Some(row) = rows.next().await? {
                    let Value::Object(object) = &row else {
                        return Err(ModelError::Schema("actual native role"));
                    };
                    crate::derivation::require_namespace(object, namespace)?;
                    if let Some(Value::RecordId(out)) = object.get("out")
                        && out.table.as_str() == "external"
                    {
                        let mut pointer = surrealdb::types::Object::new();
                        pointer.insert("id", out.clone());
                        external.push(Value::Object(pointer))?;
                    }
                    actual.push(row)?;
                }
                Ok(())
            }
            .await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step("actual role drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut external = external.finish()?;
        loop {
            let mut ids = Vec::new();
            while ids.len() < 128 {
                let Some(row) = external.next_row()? else {
                    break;
                };
                let Value::Object(row) = row else {
                    return Err(ModelError::Schema("actual external pointer"));
                };
                ids.push(
                    RecordId::from_value(
                        row.get("id")
                            .cloned()
                            .ok_or(ModelError::Schema("actual external id"))?,
                    )
                    .map_err(ModelError::codec)?,
                );
            }
            if ids.is_empty() {
                break;
            }
            let mut vars = Variables::new();
            vars.insert("ids", ids);
            let mut rows = crate::prepared::PreparedQuery::new(
                vars,
                vec![],
                vec!["SELECT * FROM $ids".into()],
            )?
            .stream_cancellable(&client, cancellation.as_ref(), &budget)?;
            let result = async {
                while let Some(row) = rows.next().await? {
                    actual.push(row)?;
                }
                Ok(())
            }
            .await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step("actual external drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)?;
        }
        let mut actual = actual.finish()?;
        while let Some(row) = actual.next_row()? {
            if !sender.send(row).await? {
                break;
            }
        }
        Ok(())
    })
}
impl<Context> crate::NativeReader<Context> {
    /// One indexed union query for a bounded set of exact original byte ranges.
    /// Results preserve request order, including duplicates; source/chunk integrity is checked.
    pub async fn original_bytes_batch(
        &self,
        ranges: &[(lctx_model::domain::graph::EntityId, u64, usize)],
    ) -> Result<crate::originals::OriginalRanges, ModelError> {
        if ranges.len() > 32
            || ranges
                .iter()
                .try_fold(0usize, |sum, (_, _, n)| sum.checked_add(*n))
                .is_none_or(|n| n > 256 * 1024)
        {
            return Err(ModelError::Invalid("original union byte bound".into()));
        }
        if ranges.is_empty() {
            return Ok(crate::originals::OriginalRanges::empty());
        }
        self.authorize_original_ranges(ranges)?;
        let mut eligible = self.view_bindings();
        if eligible.get("lctx_views").is_some() {
            let anchors = ranges
                .iter()
                .map(|(source, _, _)| crate::reader::target_id(Target::Entity(*source)))
                .collect::<Vec<_>>();
            eligible.insert("anchors", anchors.clone());
            let mut candidates = self.stream_prepared(crate::prepared::PreparedQuery::new(eligible, vec![], vec!["SELECT id,anchor FROM entity WITH INDEX anchor_payload WHERE anchor IN $anchors".into()])?)?;
            let mut found = Vec::new();
            let result = async {
                while let Some(row) = candidates.next().await? {
                    let Value::Object(row) = row else {
                        return Err(ModelError::Schema("original candidate object"));
                    };
                    let node = RecordId::from_value(
                        row.get("id")
                            .cloned()
                            .ok_or(ModelError::Schema("original candidate identity"))?,
                    )
                    .map_err(ModelError::codec)?;
                    if !self
                        .selected_candidate_ids(&[node], &self.resource_budget()?)
                        .await?
                        .is_empty()
                    {
                        found.push(
                            RecordId::from_value(
                                row.get("anchor")
                                    .cloned()
                                    .ok_or(ModelError::Schema("original candidate anchor"))?,
                            )
                            .map_err(ModelError::codec)?,
                        );
                    }
                }
                Ok(())
            }
            .await;
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "original candidate drainage",
                candidates.drain_transport().await,
            );
            lctx_model::domain::completion::complete(result, completion)?;
            if anchors.iter().any(|anchor| !found.contains(anchor)) {
                return Err(ModelError::Conflict("original source outside exact view"));
            }
        }
        let requests = ranges
            .iter()
            .map(|(source, start, len)| (RecordId::new("original", source.0.hex()), *start, *len))
            .collect::<Vec<_>>();
        crate::originals::read_ranges(self, &requests, &self.resource_budget()?).await
    }

    pub async fn original_bytes(
        &self,
        source: lctx_model::domain::graph::EntityId,
        start: u64,
        length: usize,
    ) -> Result<crate::originals::OriginalBytes, ModelError> {
        if length > 256 << 10 {
            return Err(ModelError::Invalid(
                "original byte page exceeds response bound".into(),
            ));
        }
        self.original_bytes_batch(&[(source, start, length)])
            .await?
            .pop()
            .ok_or(ModelError::Schema("original logical result inventory"))
    }
}

fn remember_external(
    rows: &mut crate::ordered_rows::SortedRows,
    target: &Target,
) -> Result<(), ModelError> {
    if matches!(target, Target::External { .. }) {
        let mut row = surrealdb::types::Object::new();
        row.insert("id", crate::reader::target_id(target.clone()));
        row.insert(
            "canonical",
            Bytes::from(serde_json::to_vec(target).map_err(ModelError::codec)?),
        );
        rows.push(Value::Object(row))?;
    }
    Ok(())
}
pub fn add_scope_fields(
    object: &mut surrealdb::types::Object,
    body: &Value,
    semantic_type: &str,
    table: crate::schema::ScopeTable,
) -> Result<(), ModelError> {
    crate::adapter::add_scope_fields(object, body, semantic_type, table)
}

pub async fn table_count(loader: &Loader, table: &str) -> Result<u64, ModelError> {
    loader.check_read_admission()?;
    let mut response = loader
        .client()
        .query(format!("SELECT count() AS total FROM {table} GROUP ALL"))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let counts: Vec<InventoryCount> = response.take(0).map_err(ModelError::codec)?;
    Ok(counts.first().map_or(0, |row| row.total))
}
#[derive(SurrealValue)]
#[surreal(crate = "surrealdb::types")]
struct InventoryCount {
    total: u64,
}
