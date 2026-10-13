//! Attempt-owned publication preparation. Child effects retain one discoverable root,
//! which publication can subsequently retain without copying its entire child fanout.
use super::*;
use crate::completed_state::HeldFrozenCompletedState;
use lctx_model::domain::resources::ResourceBudget;
/// Neither a root ID nor deserialized completion data constructs this capability.
/// The native factory first earns a held, independently checked completed state.
pub struct DerivedStaging {
    root: RecordId,
    incarnation: i64,
    completed: HeldFrozenCompletedState,
    derivation: ContentHash,
}
impl DerivedStaging {
    pub fn root(&self) -> &RecordId {
        &self.root
    }
    pub fn derivation(&self) -> ContentHash {
        self.derivation
    }
    pub fn attempt(&self) -> ContentHash {
        self.completed.completion().attempt
    }
    pub fn completed(&self) -> &crate::completed_state::FrozenCompletion {
        self.completed.completion()
    }
    pub fn completed_owner(&self) -> &HeldFrozenCompletedState {
        &self.completed
    }

    /// Creates attempt→root→completed-state atomically before any child is installed.
    /// The caller retains the attempt's existing completion hold throughout construction.
    pub async fn begin(
        client: &Surreal<Client>,
        completed: HeldFrozenCompletedState,
        derivation: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = budget.reserve("publication staging root construction", 262144)?;
        let data = completed.completion();
        let attempt = data.attempt;
        if completed.owner() != &RecordId::new("native_attempt", attempt.hex()) {
            return Err(ModelError::Conflict("publication staging completion owner"));
        }
        let root = RecordId::new(
            "native_publication_staging",
            fresh_identity("publication-staging")?.hex(),
        );
        let mut row = Object::new();
        row.insert("id", root.clone());
        row.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
        row.insert("completion", data.record()?);
        row.insert("admission", completed.admission_record().clone());
        row.insert("derivation", derivation.hex());
        row.insert("state", "open");
        row.insert("generation", data.generation.hex());
        row.insert("epoch", data.attempt_epoch);
        row.insert("validity_epoch", data.validity_epoch);
        let (row_sql, mut variables) = immutable_row_plan(None, None, vec![Value::Object(row)])?
            .ok_or(ModelError::Schema("publication staging root plan"))?;
        let (_, hold_sql, holds) = hold_plan(
            Some(attempt),
            vec![
                (RecordId::new("native_attempt", attempt.hex()), root.clone()),
                (root.clone(), data.record()?),
                (root.clone(), completed.admission_record().clone()),
            ],
        )?
        .ok_or(ModelError::Schema("publication staging root holds"))?;
        merge_plan_bindings(&mut variables, holds)?;
        variables.insert("staging_completion", data.record()?);
        variables.insert(
            "staging_completion_guard",
            guard_id(&Value::RecordId(data.record()?))?,
        );
        variables.insert("staging_completion_incarnation", completed.incarnation());
        variables.insert("staging_completion_bytes", data.value(true)?);
        variables.insert(
            "staging_admission_guard",
            guard_id(&Value::RecordId(completed.admission_record().clone()))?,
        );
        variables.insert(
            "staging_admission_incarnation",
            completed.admission_incarnation(),
        );
        variables.insert("staging_completed_identity", data.identity()?.hex());
        variables.insert("staging_completed_revision", data.completed_revision);
        variables.insert("staging_validity_epoch", data.validity_epoch);
        variables.insert("staging_attempt_epoch", data.attempt_epoch);
        let sql = format!(
            "{FROZEN_COMPLETION_GUARD} LET $completion_guard=SELECT * FROM ONLY $staging_completion_guard FOR UPDATE; LET $completion_row=SELECT * FROM ONLY $staging_completion FOR UPDATE; IF $completion_guard.phase!='active' OR $completion_guard.incarnation!=$staging_completion_incarnation OR $completion_row!=$staging_completion_bytes {{ THROW 'publication staging completion changed'; }}; {row_sql}; {hold_sql}"
        );
        let bytes = immutable_window::plan_bytes(&sql, &variables)?;
        if bytes > 262144 {
            return Err(ModelError::Schema(
                "publication staging root construction bound",
            ));
        }
        charge.try_resize(bytes)?;
        effect_for_owner(client, EffectOwner::FrozenAttempt(attempt), &sql, variables).await?;
        let observed = inspect_expected(client, root.clone(), budget)
            .await?
            .ok_or(ModelError::Conflict("publication staging root readback"))?;
        let incarnation = observed.incarnation();
        // The attempt's native hold remains authoritative after this inspection is dropped.
        Ok(Self {
            root,
            incarnation,
            completed,
            derivation,
        })
    }

    /// Only derived physical families are writable after completed content is frozen.
    /// External canonical endpoints must already exist with exactly these bytes.
    pub async fn insert(
        &self,
        client: &Surreal<Client>,
        rows: Vec<Value>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        for row in &rows {
            validate_derived_row(row)?;
        }
        for window in
            immutable_window::ImmutableWindows::for_loading(rows, None, Some(self.root.clone()))
                .with_budget(Some(budget))
        {
            immutable_window::install_staging_window(client, self, window?, budget).await?;
        }
        Ok(())
    }

    /// Parent-private: no arbitrary SQL escapes this capability's fixed production callers.
    pub(super) async fn effect(
        &self,
        client: &Surreal<Client>,
        sql: &str,
        mut bindings: Variables,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let mut charge = budget.reserve(
            "publication frozen effect construction",
            immutable_window::plan_bytes(sql, &bindings)?.saturating_add(131072),
        )?;
        let data = self.completed.completion();
        bindings.insert("staging_root", self.root.clone());
        bindings.insert(
            "staging_guard",
            guard_id(&Value::RecordId(self.root.clone()))?,
        );
        bindings.insert("staging_incarnation", self.incarnation);
        bindings.insert("staging_completion", data.record()?);
        bindings.insert(
            "staging_admission",
            self.completed.admission_record().clone(),
        );
        bindings.insert(
            "staging_admission_guard",
            guard_id(&Value::RecordId(self.completed.admission_record().clone()))?,
        );
        bindings.insert(
            "staging_admission_incarnation",
            self.completed.admission_incarnation(),
        );
        bindings.insert("staging_derivation", self.derivation.hex());
        bindings.insert("staging_completed_identity", data.identity()?.hex());
        bindings.insert("staging_completed_revision", data.completed_revision);
        bindings.insert("staging_validity_epoch", data.validity_epoch);
        bindings.insert("staging_attempt_epoch", data.attempt_epoch);
        let guarded = format!(
            "{FROZEN_COMPLETION_GUARD} fn::native_touch($staging_root); LET $staging=SELECT * FROM ONLY $staging_root FOR UPDATE; LET $staging_object_guard=SELECT * FROM ONLY $staging_guard FOR UPDATE; IF $staging=NONE OR $staging.state!='open' OR $staging.attempt!=$__attempt OR $staging.completion!=$staging_completion OR $staging.admission!=$staging_admission OR $staging.derivation!=$staging_derivation OR $staging.validity_epoch!=$staging_validity_epoch OR $staging_object_guard.phase!='active' OR $staging_object_guard.incarnation!=$staging_incarnation {{ THROW 'publication staging capability fenced'; }}; {sql}"
        );
        charge.try_resize(immutable_window::plan_bytes(&guarded, &bindings)?)?;
        effect_for_owner(
            client,
            EffectOwner::FrozenAttempt(data.attempt),
            &guarded,
            bindings,
        )
        .await
    }
}
const FROZEN_COMPLETION_GUARD: &str = "LET $staging_installation=SELECT * FROM ONLY native_installation:current; IF !$__owner.admitted OR $__owner.epoch!=$staging_attempt_epoch OR $__owner.completed_identity!=$staging_completed_identity OR $__owner.completed_revision!=$staging_completed_revision OR $__owner.validity_epoch!=$staging_validity_epoch OR $staging_installation.validity_epoch!=$staging_validity_epoch { THROW 'publication staging frozen completion changed'; }; LET $staging_semantic_guard=SELECT * FROM ONLY $staging_admission_guard FOR UPDATE; IF $staging_semantic_guard=NONE OR $staging_semantic_guard.phase!='active' OR $staging_semantic_guard.incarnation!=$staging_admission_incarnation { THROW 'publication staging semantic admission changed'; };";

fn validate_derived_row(row: &Value) -> Result<(), ModelError> {
    let Value::Object(row) = row else {
        return Err(ModelError::Schema("publication derived row"));
    };
    let Some(Value::RecordId(id)) = row.get("id") else {
        return Err(ModelError::Schema("publication derived address"));
    };
    if !matches!(
        id.table.as_str(),
        "entity_anchor"
            | "assertion_anchor"
            | "participant"
            | "reference"
            | "external"
            | "search_api_options"
            | "search_documentation_deployment"
            | "search_scenario"
            | "search_source"
            | "vector"
            | "lex_occurs"
            | "vec_occurs"
            | "lexical_document"
            | "lexical_member"
            | "lexical_term"
            | "lexical_corpus"
    ) {
        return Err(ModelError::Conflict(
            "canonical content is not derived staging",
        ));
    }
    Ok(())
}

/// Format coordinator installs this only as part of the complete format-six transition.
pub fn schema_target() -> &'static str {
    "DEFINE TABLE native_publication_staging SCHEMAFULL; DEFINE FIELD id ON native_publication_staging TYPE string; DEFINE FIELD attempt ON native_publication_staging TYPE record<native_attempt>; DEFINE FIELD completion ON native_publication_staging TYPE record<native_completed_state>; DEFINE FIELD admission ON native_publication_staging TYPE record<native_completed_admission>; DEFINE FIELD derivation ON native_publication_staging TYPE string; DEFINE FIELD state ON native_publication_staging TYPE string; DEFINE FIELD derived_children ON native_publication_staging TYPE option<int>; DEFINE FIELD derived_inventory ON native_publication_staging TYPE option<string>; DEFINE FIELD generation ON native_publication_staging TYPE string; DEFINE FIELD epoch ON native_publication_staging TYPE int; DEFINE FIELD validity_epoch ON native_publication_staging TYPE int; DEFINE INDEX attempt_staging ON native_publication_staging FIELDS attempt,id;"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_frozen_derived_capability_cannot_install_canonical_or_control_content() {
        let row = |table: &str| {
            let mut row = Object::new();
            row.insert("id", RecordId::new(table, "candidate"));
            Value::Object(row)
        };
        for table in [
            "entity",
            "assertion",
            "compiler_record",
            "compiler_alias",
            "compiler_binding",
            "compiler_view",
            "original",
            "original_blob",
            "native_attempt",
            "native_effect",
            "publication",
        ] {
            assert!(
                validate_derived_row(&row(table)).is_err(),
                "frozen canonical mutation: {table}"
            );
        }
        for table in [
            "participant",
            "reference",
            "vector",
            "lex_occurs",
            "lexical_document",
            "lexical_corpus",
        ] {
            validate_derived_row(&row(table)).unwrap();
        }
    }
}

/// Called after the exact terminal attempt epoch/state fence and before either
/// admitted or unadmitted attempt-hold release. Winning publication-held roots remain
/// retained by their independent publication owner; registration itself never deletes.
pub(super) async fn nominate_attempt_roots(
    client: &Surreal<Client>,
    authorization: TerminalAttempt,
) -> Result<(), ModelError> {
    let attempt = RecordId::new("native_attempt", authorization.identity.hex());
    let mut bindings = Variables::new();
    bindings.insert("attempt", attempt);
    bindings.insert("after", Value::None);
    loop {
        let mut response=client.query("SELECT * FROM native_publication_staging WITH INDEX attempt_staging WHERE attempt=$attempt AND ($after=NONE OR id>$after) ORDER BY id LIMIT 32").bind(bindings.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let values: Vec<Value> = response.take(0).map_err(ModelError::codec)?;
        if values.is_empty() {
            return Ok(());
        }
        let mut after = None;
        for value in &values {
            let Value::Object(row) = value else {
                return Err(ModelError::Schema("publication staging terminal envelope"));
            };
            let Some(Value::RecordId(id)) = row.get("id") else {
                return Err(ModelError::Schema("publication staging terminal identity"));
            };
            if id.table.as_str() != "native_publication_staging"
                || row.get("attempt") != bindings.get("attempt")
                || authorization::int_field(row, "epoch")? != authorization.epoch
                || !matches!(row.get("completion"),Some(Value::RecordId(id)) if id.table.as_str()=="native_completed_state")
                || !matches!(row.get("admission"),Some(Value::RecordId(id)) if id.table.as_str()=="native_completed_admission")
                || !matches!(row.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"open"|"published"))
            {
                return Err(ModelError::Conflict(
                    "publication staging terminal original identity",
                ));
            }
            authorization::hash_field(row, "generation")?;
            authorization::hash_field(row, "derivation")?;
            if authorization::int_field(row, "validity_epoch")? <= 0 {
                return Err(ModelError::Conflict(
                    "publication staging terminal validity epoch",
                ));
            }
            after = Some(id.clone());
        }
        super::nominate_terminal_roots(client, authorization, values).await?;
        bindings.insert(
            "after",
            after.ok_or(ModelError::Schema("publication staging terminal page"))?,
        );
    }
}

/// Exact acknowledged marker identity. Its reachability is the publication's one root;
/// no child fanout is copied into the marker owner.
pub struct PublishedMarker {
    pub handle: lctx_model::domain::serving::SnapshotHandle,
}
impl DerivedStaging {
    /// Independent canonical/derived readback precedes one atomic root/marker publication.
    /// Caller-supplied hashes cannot skip these validators or produce a completed root.
    pub async fn publish(
        &self,
        loader: &crate::Loader,
        handle: &lctx_model::domain::serving::SnapshotHandle,
        manifest: &lctx_model::domain::graph::Manifest,
        views: &[lctx_model::domain::completed::CompletedBinding],
        budget: &ResourceBudget,
    ) -> Result<PublishedMarker, ModelError> {
        let data = self.completed();
        handle.validate_identity()?;
        manifest.validate()?;
        let eligibility = crate::completed_transport::eligibility_frozen(
            loader.shared_client(),
            &self.completed,
            budget,
        )
        .await?;
        if loader.staging_root() != Some(&self.root)
            || loader.derivation()? != self.derivation
            || handle.realization != self.derivation
            || handle.service_generation != data.generation
            || handle.semantic != manifest.content()
            || manifest.completed_state
                != lctx_model::domain::completed::CompletedStateIdentity::from_roots(&data.roots)?
            || handle.eligibility != eligibility
            || handle.view != data.bindings
            || lctx_model::domain::completed::binding_inventory_identity(views)? != data.bindings
        {
            return Err(ModelError::Conflict("publication marker completed scope"));
        }
        let _readback_charge = budget.reserve(
            "publication marker construction",
            marker_construction_bytes(handle, manifest, views)?,
        )?;
        let mut audit_bindings = Variables::new();
        audit_bindings.insert("root", self.root.clone());
        let audit_revision: i64 = loader
            .reader()
            .query_native(
                "RETURN (SELECT VALUE revision FROM ONLY type::record('native_predicate',[$root]))",
                audit_bindings,
            )
            .await?;
        if audit_revision <= 0 || audit_revision == i64::MAX {
            return Err(ModelError::Conflict("publication derived audit revision"));
        }
        let mut expected = crate::ordered_rows::SortedRows::with_budget(budget)?;
        loader
            .reconcile_owned(manifest, Some(&mut expected))
            .await?;
        crate::derived_search::reconcile_search_owned(loader, Some(&mut expected)).await?;
        for id in [data.record()?, self.completed.admission_record().clone()] {
            let mut row = Object::new();
            row.insert("id", id);
            expected.push(Value::Object(row))?;
        }
        let inventory = reconcile_root_inventory(loader, &self.root, expected, budget).await?;
        let marker = marker_value(handle, manifest, views)?;
        let id = RecordId::new("publication", handle.publication.hex());
        // A separately acknowledged winner may use another physical realization of the
        // same logical input. Its core must match exactly; the protected reader checks
        // its independently earned local completion and root before convergence.
        if existing_marker(loader, &id, &marker).await? {
            check_published(loader, handle, budget).await?;
            return Ok(PublishedMarker {
                handle: handle.clone(),
            });
        }
        let Value::Object(mut marker) = marker else {
            unreachable!()
        };
        marker.insert("staging_root", self.root.clone());
        marker.insert("completion", data.record()?);
        marker.insert("admission", self.completed.admission_record().clone());
        marker.insert("staging_incarnation", self.incarnation);
        marker.insert("completion_incarnation", self.completed.incarnation());
        marker.insert(
            "admission_incarnation",
            self.completed.admission_incarnation(),
        );
        marker.insert("derivation", self.derivation.hex());
        marker.insert("validity_epoch", data.validity_epoch);
        let (row_sql, mut bindings) = immutable_row_plan(None, None, vec![Value::Object(marker)])?
            .ok_or(ModelError::Schema("publication marker plan"))?;
        let (_, hold_sql, holds) = hold_plan(Some(data.attempt), vec![(id, self.root.clone())])?
            .ok_or(ModelError::Schema("publication marker root hold"))?;
        merge_plan_bindings(&mut bindings, holds)?;
        bindings.insert("staging_audit_revision", audit_revision);
        bindings.insert("staging_inventory_rows", inventory.0);
        bindings.insert("staging_inventory_digest", inventory.1.hex());
        let sql = format!(
            "LET $staging_predicate=type::record('native_predicate',[$staging_root]); IF (SELECT VALUE revision FROM ONLY $staging_predicate)!=$staging_audit_revision+1 {{ THROW 'publication derived audit changed'; }}; {row_sql}; {hold_sql}; UPDATE $staging_root SET state='published',derived_children=$staging_inventory_rows,derived_inventory=$staging_inventory_digest RETURN NONE;"
        );
        match self.effect(loader.client(), &sql, bindings, budget).await {
            Ok(()) => Ok(PublishedMarker {
                handle: handle.clone(),
            }),
            Err(error) => {
                // Unknown effects retain their original recovery obligation. Only a
                // confirmed refusal permits a separately issued winner observation.
                if !error.permits_storage_cleanup() {
                    return Err(error);
                }
                if existing_marker(
                    loader,
                    &RecordId::new("publication", handle.publication.hex()),
                    &marker_value(handle, manifest, views)?,
                )
                .await?
                {
                    check_published(loader, handle, budget).await?;
                    Ok(PublishedMarker {
                        handle: handle.clone(),
                    })
                } else {
                    Err(error)
                }
            }
        }
    }
}
fn marker_value(
    handle: &lctx_model::domain::serving::SnapshotHandle,
    manifest: &lctx_model::domain::graph::Manifest,
    views: &[lctx_model::domain::completed::CompletedBinding],
) -> Result<Value, ModelError> {
    let mut row = Object::new();
    row.insert("id", RecordId::new("publication", handle.publication.hex()));
    row.insert(
        "handle",
        hex::encode(serde_json::to_vec(handle).map_err(ModelError::codec)?),
    );
    row.insert(
        "manifest",
        surrealdb::types::Bytes::from(serde_json::to_vec(manifest).map_err(ModelError::codec)?),
    );
    row.insert(
        "views",
        surrealdb::types::Bytes::from(serde_json::to_vec(views).map_err(ModelError::codec)?),
    );
    row.insert("definition_epoch", handle.definition_epoch.hex());
    Ok(Value::Object(row))
}
async fn existing_marker(
    loader: &crate::Loader,
    id: &RecordId,
    expected: &Value,
) -> Result<bool, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("publication", id.clone());
    let rows: Vec<Value> = loader
        .reader()
        .query_native("SELECT * FROM $publication", bindings)
        .await?;
    if rows.is_empty() {
        return Ok(false);
    }
    let [Value::Object(actual)] = rows.as_slice() else {
        return Err(ModelError::Schema("publication marker cardinality"));
    };
    let Value::Object(expected) = expected else {
        unreachable!()
    };
    if expected
        .iter()
        .any(|(field, value)| actual.get(field) != Some(value))
    {
        return Err(ModelError::Conflict("publication immutable core collision"));
    }
    Ok(true)
}

fn marker_construction_bytes(
    handle: &lctx_model::domain::serving::SnapshotHandle,
    manifest: &lctx_model::domain::graph::Manifest,
    views: &[lctx_model::domain::completed::CompletedBinding],
) -> Result<usize, ModelError> {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("publication marker size overflow"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, handle).map_err(ModelError::codec)?;
    serde_json::to_writer(&mut count, manifest).map_err(ModelError::codec)?;
    serde_json::to_writer(&mut count, views).map_err(ModelError::codec)?;
    count
        .0
        .checked_mul(40)
        .and_then(|bytes| bytes.checked_add(262144))
        .ok_or(ModelError::Schema("publication marker construction size"))
}
async fn check_published(
    loader: &crate::Loader,
    handle: &lctx_model::domain::serving::SnapshotHandle,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let reader = crate::NativeReader::open(loader.shared_client(), handle.clone())
        .await?
        .with_budget(budget);
    let result = crate::completed_state::published_completed_state(&reader, budget)
        .await
        .map(drop);
    if result
        .as_ref()
        .err()
        .is_some_and(|error| !error.permits_storage_cleanup())
    {
        reader.retain_unknown();
    }
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step("converged publication reader close", reader.close().await);
    lctx_model::domain::completion::complete(result, completion)
}

/// Unchanged publication reuses only an independently checked, pinned winning root.
/// Caller-provided marker fields alone confer no eligibility.
pub async fn find_existing_publication(
    loader: &crate::Loader,
    handle: &lctx_model::domain::serving::SnapshotHandle,
    manifest: &lctx_model::domain::graph::Manifest,
    views: &[lctx_model::domain::completed::CompletedBinding],
    incoming: &HeldFrozenCompletedState,
    budget: &ResourceBudget,
) -> Result<Option<PublishedMarker>, ModelError> {
    handle.validate_identity()?;
    manifest.validate()?;
    let eligibility =
        crate::completed_transport::eligibility_frozen(loader.shared_client(), incoming, budget)
            .await?;
    if loader.derivation()? != handle.realization
        || handle.semantic != manifest.content()
        || handle.eligibility != eligibility
        || lctx_model::domain::completed::binding_inventory_identity(views)? != handle.view
    {
        return Err(ModelError::Conflict(
            "unchanged publication requested scope",
        ));
    }
    let _charge = budget.reserve(
        "unchanged publication core construction",
        marker_construction_bytes(handle, manifest, views)?,
    )?;
    if !existing_marker(
        loader,
        &RecordId::new("publication", handle.publication.hex()),
        &marker_value(handle, manifest, views)?,
    )
    .await?
    {
        return Ok(None);
    }
    check_published(loader, handle, budget).await?;
    Ok(Some(PublishedMarker {
        handle: handle.clone(),
    }))
}

/// Collect independently lowered child identities without retaining full row payloads.
pub(crate) fn remember_expected_child(
    inventory: &mut crate::ordered_rows::SortedRows,
    row: &Value,
) -> Result<(), ModelError> {
    let Value::Object(row) = row else {
        return Err(ModelError::Schema("expected staging child object"));
    };
    let Some(Value::RecordId(id)) = row.get("id") else {
        return Err(ModelError::Schema("expected staging child identity"));
    };
    let mut pointer = Object::new();
    pointer.insert("id", id.clone());
    inventory.push(Value::Object(pointer))
}
async fn reconcile_root_inventory(
    loader: &crate::Loader,
    root: &RecordId,
    expected: crate::ordered_rows::SortedRows,
    budget: &ResourceBudget,
) -> Result<(i64, ContentHash), ModelError> {
    let reader = loader.reader();
    let mut bindings = Variables::new();
    bindings.insert("root", root.clone());
    let mut rows = reader.query_stream(
        "SELECT object AS id FROM native_hold WITH INDEX owner_holds WHERE owner=$root ORDER BY id",
        bindings,
        1,
    )?;
    let mut actual = crate::ordered_rows::SortedRows::with_budget(budget)?;
    let result = async {
        while let Some(row) = rows.next().await? {
            remember_expected_child(&mut actual, &row)?;
        }
        Ok(())
    }
    .await;
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.step(
        "publication owned child inventory drainage",
        rows.drain_transport().await,
    );
    lctx_model::domain::completion::complete(result, completion)?;
    let mut actual = actual.finish()?;
    let mut expected = expected.finish()?;
    let mut count = 0i64;
    let mut digest = KeySink::new("native-publication-derived-closure/v1");
    loop {
        let a = actual.next_row()?;
        let e = expected.next_row()?;
        if a != e {
            return Err(ModelError::Conflict(
                "publication staging exact child closure differs",
            ));
        }
        let Some(row) = e else {
            break;
        };
        count = count
            .checked_add(1)
            .ok_or(ModelError::Schema("publication child inventory count"))?;
        digest.part(
            b"child",
            &serde_json::to_vec(&row).map_err(ModelError::codec)?,
        );
    }
    digest.part(b"children", &count.to_le_bytes());
    Ok((count, digest.finish()))
}
