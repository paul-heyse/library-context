use futures::{StreamExt, FutureExt};
use lctx_model::domain::{
    Key, KeySink, ModelError, Record,
    graph::{Assertion, Entity, Target},
    serving::SnapshotHandle,
};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;
use surrealdb::{
    Surreal,
    engine::remote::grpc::{Client, Grpc},
    opt::auth::{Database, Root},
    types::{Bytes, RecordId, SerdeWrapper, SurrealValue, Value, Variables},
};

/// Secrets are supplied by the operator; this type deliberately has no Debug or Serialize.
pub enum Credentials {
    Root { username: String, password: String },
    Database { username: String, password: String },
}
/// Finite inputs supplied by a model-owned operation, never a request-controlled SQL predicate.
pub enum RecordSelection {
    Keys(Vec<[u8; 16]>),
    Scope {
        field: String,
        values: Vec<serde_json::Value>,
    },
    Connected {
        targets: Vec<Target>,
        fields: Vec<String>,
    },
}
#[derive(Clone)]
pub struct NativeReader<Context = SnapshotHandle> {
    client: Arc<Surreal<Client>>,
    handle: Context,
    scope: Option<Arc<ReaderScope>>,
    cancellation: Option<Arc<std::sync::atomic::AtomicBool>>,
    budget: Option<lctx_model::domain::resources::ResourceBudget>,
    request_budget: Option<lctx_model::domain::resources::ResourceBudget>,
}
/// Protect a publication pin through unexpected unwind and async finalizer abandonment.
/// This holds the pin alone, so it never counts as another reader/scope borrower.
pub struct ReadFinalizationGuard { pin: Option<Arc<crate::control::ReaderPin>>, closed: bool }
impl ReadFinalizationGuard {
    pub async fn close<Context>(&mut self, reader: &NativeReader<Context>) -> Result<(), ModelError> {
        let actual = reader.scope.as_ref().and_then(|scope| scope._pin.as_ref());
        if !match (&self.pin, actual) { (Some(pin), Some(actual)) => Arc::ptr_eq(pin, actual), (None, None) => true, _ => false } {
            return Err(ModelError::Conflict("reader finalization guard owner"));
        }
        reader.close().await?; self.closed = true; Ok(())
    }
}
impl Drop for ReadFinalizationGuard {
    fn drop(&mut self) { if !self.closed { if let Some(pin) = &self.pin { pin.retain_unknown(); } } }
}
struct ReaderScope {
    originals: Option<Vec<lctx_model::domain::graph::Original>>,
    views: Vec<lctx_model::domain::ContentHash>,
    _pin: Option<Arc<crate::control::ReaderPin>>,
    selection: std::sync::Mutex<Option<(lctx_model::domain::resources::ResourceBudget, futures::future::Shared<futures::future::BoxFuture<'static, Result<crate::selection::SelectedPayloads, Arc<ModelError>>>>)>>,
}
struct SelectionLease { scope: Arc<ReaderScope>, confirmed: bool }
impl Drop for SelectionLease {
    fn drop(&mut self) {
        if !self.confirmed { if let Some(pin) = &self.scope._pin { pin.retain_unknown(); } }
    }
}
impl<Context> NativeReader<Context> {
    pub(crate) fn transport_reader(&self) -> NativeReader<()> { NativeReader { client: self.client.clone(), handle: (), scope: self.scope.clone(), cancellation: self.cancellation.clone(), budget: self.budget.clone(), request_budget: self.request_budget.clone() } }
    /// Retained selection and default scratch share the caller's allocation authority.
    pub fn with_budget(mut self, budget: &lctx_model::domain::resources::ResourceBudget) -> Self { self.budget = Some(budget.clone()); self.request_budget = Some(budget.clone()); self }
    pub fn with_request_budget_clone(&self, budget: &lctx_model::domain::resources::ResourceBudget) -> NativeReader<()> { self.transport_reader().with_request_budget(budget) }
    pub fn with_request_budget(mut self, budget: &lctx_model::domain::resources::ResourceBudget) -> Self { self.request_budget = Some(budget.clone()); self }
    pub fn resource_budget(&self) -> Result<lctx_model::domain::resources::ResourceBudget, ModelError> { self.budget.clone().ok_or(ModelError::Conflict("native reader requires caller budget")) }
    fn scratch_budget(&self) -> Result<lctx_model::domain::resources::ResourceBudget, ModelError> { self.request_budget.clone().ok_or(ModelError::Conflict("native selected read requires caller budget")) }
    pub fn protect_terminal_close(&self) -> ReadFinalizationGuard { ReadFinalizationGuard { pin: self.scope.as_ref().and_then(|scope| scope._pin.clone()), closed: false } }
    pub fn with_read_cancellation(mut self, flag: Arc<std::sync::atomic::AtomicBool>) -> Self { self.cancellation = Some(flag); self }
    pub(crate) fn read_cancellation(&self) -> Option<Arc<std::sync::atomic::AtomicBool>> { self.cancellation.clone() }
    pub fn check_read_admission(&self) -> Result<(), ModelError> { crate::prepared::check_read_cancellation(self.cancellation.as_ref()) }
    /// The owner drains reader clones and streams before explicit session invalidation.
    pub async fn close(&self) -> Result<(), ModelError> {
        if let Some(scope) = &self.scope {
            if Arc::strong_count(scope) != 1 {
                return Err(ModelError::Conflict("native reader clones remain live"));
            }
            if let Some(pin) = &scope._pin {
                pin.release().await?;
            }
            scope.selection.lock().map_err(|_| ModelError::Conflict("reader selection owner"))?.take();
        }
        Ok(())
    }
    /// Conservative protection when a finalizer cannot establish remote terminality.
    pub fn retain_unknown(&self) {
        if let Some(pin) = self.scope.as_ref().and_then(|scope| scope._pin.as_ref()) { pin.retain_unknown(); }
    }
    pub(crate) fn authorize_original_ranges(
        &self,
        ranges: &[(lctx_model::domain::graph::EntityId, u64, usize)],
    ) -> Result<(), ModelError> {
        if let Some(originals) = self
            .scope
            .as_ref()
            .and_then(|scope| scope.originals.as_ref())
        {
            for (source, start, length) in ranges {
                let original = originals
                    .iter()
                    .find(|original| original.source == *source)
                    .ok_or(ModelError::Conflict("original source outside publication"))?;
                if start
                    .checked_add(*length as u64)
                    .is_none_or(|end| end > original.byte_len)
                {
                    return Err(ModelError::Conflict("original range outside publication"));
                }
            }
        }
        Ok(())
    }
    pub fn view_bindings(&self) -> Variables {
        let mut bindings = Variables::new();
        if let Some(scope) = &self.scope {
            bindings.insert(
                "lctx_views",
                scope
                    .views
                    .iter()
                    .map(|view| RecordId::new("compiler_view", view.hex()))
                    .collect::<Vec<_>>(),
            );
        }
        bindings
    }
    /// One operation-local immutable, disk-backed selection; this carries no acceptance result.
    pub async fn prepare_selection(&self) -> Result<Option<crate::selection::SelectedPayloads>, ModelError> {
        self.selection_preparation().await
    }
    pub(crate) fn selection_preparation(&self) -> futures::future::BoxFuture<'static, Result<Option<crate::selection::SelectedPayloads>, ModelError>> {
        let scope = self.scope.clone(); let client = self.client.clone(); let cancellation = self.cancellation.clone(); let budget = self.budget.clone();
        async move {
            let Some(scope) = scope else { return Ok(None); };
            let budget = budget.ok_or(ModelError::Conflict("native selection requires caller budget"))?;
            let pending = {
                let mut selection = scope.selection.lock().map_err(|_| ModelError::Conflict("reader selection owner"))?;
                if let Some((owner, pending)) = selection.as_ref() { if !owner.shares_pool(&budget) { return Err(ModelError::Conflict("foreign native selection budget")); } pending.clone() } else {
                    let retained_scope = scope.clone(); let preparation_cancellation = cancellation.clone();
                    let retained_budget = budget.clone();
                    let task = tokio::spawn(async move {
                        let mut lease = SelectionLease { scope: retained_scope, confirmed: false };
                        let result = crate::selection::SelectedPayloads::for_cancellable_views(client, &lease.scope.views, preparation_cancellation, &retained_budget).await;
                        lease.confirmed = result.as_ref().err().is_none_or(|error| error.permits_storage_cleanup());
                        drop(lease); result
                    });
                    let pending = async move { task.await.map_err(|error| Arc::new(ModelError::codec(error)))?.map_err(Arc::new) }.boxed().shared();
                    *selection = Some((budget, pending.clone())); pending
                }
            };
            pending.await.map(|selected| Some(selected.with_read_cancellation(cancellation))).map_err(ModelError::SharedCause)
        }.boxed()
    }
    pub fn selected_payload_rows(
        &self, table: &str, predicate: &str, bindings: Variables,
        preparation: Vec<String>, order: &str, limit: Option<usize>,
    ) -> Result<NativeRows, ModelError> {
        if self.scope.is_none() {
            let limit = limit.map(|limit| format!(" LIMIT {limit}")).unwrap_or_default();
            return self.stream_prepared(crate::prepared::PreparedQuery::new(bindings, preparation,
                vec![format!("SELECT * FROM {table} WHERE ({predicate}) ORDER BY {order}{limit}")])?);
        }
        let selection = self.selection_preparation(); let scope = self.scope.clone().expect("scope"); let budget = self.scratch_budget()?;
        let table = table.to_owned(); let predicate = predicate.to_owned(); let order = order.to_owned();
        let mut rows = NativeRows::owned(move |sender| async move {
            let selection = selection.await?.ok_or(ModelError::Conflict("selected reader scope"))?;
            if sender.is_closed() { return Ok(()); }
            let mut rows = selection.rows_with_budget(&table, &predicate, bindings, preparation, &order, limit, &budget)?;
            let result = async { loop {
                let row = tokio::select! { biased; _ = sender.closed() => { rows.cancel_delivery(); break; }, row = rows.next() => row? };
                let Some(row) = row else { break; };
                if sender.send(row).await.is_err() { rows.cancel_delivery(); break; }
            } Ok(()) }.await;
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step("selected reader payload drainage", rows.drain_transport().await);
            lctx_model::domain::completion::complete(result, completion)
        })?;
        rows.scope = Some(scope); rows.client = Some(self.client.clone()); Ok(rows)
    }
    /// A model-owned indexed query returns compact `id` nominations. Exact selected
    /// membership is checked before fetching their complete canonical payloads.
    pub fn candidate_payload_rows(&self, candidates: crate::prepared::PreparedQuery, table: &str, order: &str) -> Result<NativeRows, ModelError> {
        self.check_read_admission()?;
        let budget = self.scratch_budget()?;
        let scope = self.scope.as_ref().ok_or(ModelError::Conflict("candidate read requires exact selected scope"))?;
        let mut rows = crate::selection::SelectedPayloads::sparse_candidate_rows(self.client.clone(), scope.views.clone(), self.cancellation.clone(), candidates, table, order, &budget)?;
        rows.scope = Some(scope.clone()); rows.client = Some(self.client.clone()); Ok(rows)
    }
    /// One answer for each distinct requested physical identity, with exact view and
    /// reverse-alias provenance. Absence is local to that identity, never an any-hit flag.
    pub async fn selected_membership(&self, requested: &[RecordId], budget: &lctx_model::domain::resources::ResourceBudget) -> Result<crate::selection::MembershipAnswers, ModelError> {
        self.check_read_admission()?;
        let Some(scope) = self.scope.clone() else { return crate::selection::MembershipAnswers::unrestricted(requested, budget); };
        let requested_charge = budget.reserve("selected-membership-task-input", requested.iter().map(crate::selection::node_bytes).sum::<usize>())?;
        let requested = requested.to_vec(); let budget = budget.clone(); let client = self.client.clone(); let cancelled = self.cancellation.clone();
        // Caller cancellation detaches delivery, not the admitted physical operation.
        // The task retains its input charge and reader pin until every query is drained.
        tokio::spawn(async move {
            let _requested_charge = requested_charge;
            let mut lease = SelectionLease { scope, confirmed: false };
            let result = crate::selection::SelectedPayloads::view_membership(client, &lease.scope.views, cancelled, &requested, &budget).await;
            lease.confirmed = result.as_ref().err().is_none_or(|error| error.permits_storage_cleanup());
            result
        }).await.map_err(ModelError::codec)?
    }
    pub(crate) fn retain_rows(&self, mut rows: NativeRows) -> NativeRows {
        rows.scope = self.scope.clone(); rows.client = Some(self.client.clone()); rows
    }
    pub async fn selected_candidate_ids(&self, requested: &[RecordId], budget: &lctx_model::domain::resources::ResourceBudget) -> Result<Vec<RecordId>, ModelError> {
        Ok(self.selected_membership(requested, budget).await?.answers.into_iter().filter(|answer| answer.selected).map(|answer| answer.requested).collect())
    }
    pub fn record_stream_candidates<R: Record + DeserializeOwned>(&self, candidates: crate::prepared::PreparedQuery, order: &str) -> Result<CanonicalRecords<R>, ModelError> {
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
        if table == "compiler_record" { return Err(ModelError::Schema("canonical native record family")); }
        Ok(CanonicalRecords { rows: self.candidate_payload_rows(candidates, table, order)?, positions: vec![0], physical_kind: true, failed: false, marker: std::marker::PhantomData })
    }
    /// Release retained immutable preparation after the owning service has drained requests.
    pub async fn release_preparation(&self) -> Result<(), ModelError> {
        if let Some(scope) = &self.scope {
            let pending = scope.selection.lock().map_err(|_| ModelError::Conflict("reader selection owner"))?.take();
            if let Some((_, pending)) = pending { pending.await.map_err(ModelError::SharedCause)?; }
        } Ok(())
    }
    pub fn relation_rows(&self, relation: &str) -> Result<NativeRows, ModelError> {
        if self.scope.is_none() { return Err(ModelError::Conflict("relation read requires exact published view")); }
        let mut vars = Variables::new(); vars.insert("type", relation.to_owned());
        self.selected_payload_rows(crate::schema::ScopeTable::for_relation(relation)?.name(), "semantic_type=$type", vars, vec![], "semantic_key", None)
    }
    pub fn relation_bodies(&self, relation: &str, limit: usize) -> Result<NativeRows, ModelError> {
        self.check_read_admission()?;
        let budget = self.scratch_budget()?;
        let scope = self.scope.as_ref().ok_or(ModelError::Conflict("relation read requires exact published view"))?.clone();
        let table = crate::schema::ScopeTable::for_relation(relation)?.name();
        if limit == 0 || scope.views.is_empty() { return Ok(self.retain_rows(NativeRows::owned(|_| async { Ok(()) })?)); }
        let mut input = crate::selection::SelectedPayloads::sparse_relation_rows(self.client.clone(), scope.views.clone(), self.cancellation.clone(), relation, table, &budget)?;
        let mut rows = NativeRows::owned(move |sender| async move {
            let result = async {
                let mut emitted = 0usize;
                while let Some(row) = input.next().await? {
                    if sender.is_closed() { input.cancel_delivery(); break; }
                    if emitted < limit { if sender.send(row).await.is_err() { input.cancel_delivery(); break; } emitted += 1; }
                }
                Ok(())
            }.await;
            let mut terminal = lctx_model::domain::completion::Completion::default();
            terminal.step("relation candidate drainage", input.drain_transport().await);
            lctx_model::domain::completion::complete(result, terminal)
        })?;
        rows.scope = Some(scope); rows.client = Some(self.client.clone()); Ok(rows)
    }
    pub fn query_stream(&self, sql: impl Into<String>, bindings: Variables, statements: usize) -> Result<NativeRows, ModelError> {
        self.stream_prepared(crate::prepared::PreparedQuery::from_sql(sql.into(), bindings, statements, (0..statements).collect())?)
    }
    pub fn stream_prepared(&self, query: crate::prepared::PreparedQuery) -> Result<NativeRows, ModelError> {
        query.with_bindings(self.view_bindings()).stream_cancellable(&self.client, self.cancellation.as_ref()).map(|mut rows| { rows.scope = self.scope.clone(); rows })
    }
    pub async fn query_prepared<T: Serialize + DeserializeOwned + 'static>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<T, ModelError> {
        let value = self.checked_native_value(query).await?;
        SerdeWrapper::<T>::from_value(value)
            .map(|value| value.0)
            .map_err(ModelError::codec)
    }
    /// Prepared SDK-native results retain record IDs and nested Object/Array variants.
    /// Exposure uses the same complete response and terminal checks as typed serde results.
    pub async fn query_prepared_native<T: SurrealValue>(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<T, ModelError> {
        T::from_value(self.checked_native_value(query).await?).map_err(ModelError::codec)
    }
    async fn checked_native_value(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<Value, ModelError> {
        self.check_read_admission()?;
        if query.result_positions().len() != 1 {
            return Err(ModelError::Schema("native single result demand"));
        }
        let position = query.result_positions()[0];
        let terminals = query.expected_terminals();
        let (sql, bindings) = query.with_bindings(self.view_bindings()).into_request();
        self.check_read_admission()?;
        let mut response = self
            .client
            .query(sql)
            .bind(bindings)
            .await
            .map_err(sdk_error)?
            .check()
            .map_err(sdk_error)?;
        if response.num_statements() != terminals {
            return Err(ModelError::Schema("native response terminal inventory"));
        }
        response.take(position).map_err(sdk_error)
    }

    /// Finite owner-selected predicates; source payloads are decoded one at a time.
    pub fn record_stream<R: Record + DeserializeOwned>(
        &self,
        predicate: &str,
        bindings: Variables,
        order: &str,
    ) -> Result<CanonicalRecords<R>, ModelError> {
        self.record_stream_prepared(predicate, bindings, vec![], order)
    }
    pub fn record_stream_prepared<R: Record + DeserializeOwned>(
        &self,
        predicate: &str,
        mut bindings: Variables,
        preparation: Vec<String>,
        order: &str,
    ) -> Result<CanonicalRecords<R>, ModelError> {
        bindings.insert("type", R::NAME.to_string());
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
        if table == "compiler_record" { return Err(ModelError::Schema("canonical native record family")); }
        Ok(CanonicalRecords {
            rows: self.selected_payload_rows(table, &format!("semantic_type=$type AND ({predicate})"), bindings, preparation, order, None)?,
            positions: vec![0], physical_kind: true, failed: false, marker: std::marker::PhantomData,
        })
    }
    pub fn client(&self) -> &Surreal<Client> {
        &self.client
    }
    pub fn shared_client(&self) -> Arc<Surreal<Client>> {
        self.client.clone()
    }
    /// Query results become visible only after the SDK has received the complete checked response.
    pub async fn query<T: Serialize + DeserializeOwned + 'static>(
        &self,
        sql: impl Into<String>,
        bindings: Variables,
    ) -> Result<T, ModelError> {
        self.query_prepared(crate::prepared::PreparedQuery::new(
            bindings,
            vec![],
            vec![sql.into()],
        )?)
        .await
    }
    /// Decode through the SDK's native value contract, preserving nested value variants.
    /// Results become visible only after the complete checked response and terminal inventory.
    pub async fn query_native<T: SurrealValue>(
        &self,
        sql: impl Into<String>,
        bindings: Variables,
    ) -> Result<T, ModelError> {
        let query = crate::prepared::PreparedQuery::new(bindings, vec![], vec![sql.into()])?;
        self.query_prepared_native(query).await
    }
    pub async fn records<R: Record + DeserializeOwned>(
        &self,
        selection: RecordSelection,
    ) -> Result<Vec<R>, ModelError> {
        let mut bindings = Variables::new();
        bindings.insert("type", R::NAME.to_string());
        let mut preparation = Vec::new();
        let mut index = ""; let mut connected = false;
        let predicate = match selection {
            RecordSelection::Keys(keys) => {
                if keys.is_empty() {
                    return Ok(vec![]);
                }
                index = " WITH INDEX semantic_key";
                bindings.insert("keys", keys.iter().map(hex::encode).collect::<Vec<_>>());
                "semantic_key IN $keys".to_owned()
            }
            RecordSelection::Scope { field, values } => {
                if values.is_empty() {
                    return Ok(vec![]);
                }
                if !R::fields().iter().any(|f| f.name() == field) {
                    return Err(ModelError::Invalid("undeclared native scope field".into()));
                }
                let sparse = crate::schema::atomic_scope_field(R::NAME, field.as_str())
                    && !values.iter().any(serde_json::Value::is_null);
                bindings.insert(
                    "values",
                    crate::loader::json_value(serde_json::Value::Array(values))?,
                );
                if sparse {
                    index = " WITH INDEX by_scope";
                    crate::prepared::prepare_scope(
                        &mut preparation,
                        "record_scope",
                        "$type",
                        &field,
                        "$values",
                    )
                } else {
                    format!("body.`{field}` IN $values")
                }
            }
            RecordSelection::Connected { targets, fields } => {
                connected = true;
                if targets.is_empty() {
                    return Ok(vec![]);
                }
                bindings.insert(
                    "targets",
                    targets.into_iter().map(target_id).collect::<Vec<_>>(),
                );
                bindings.insert("fields", fields);
                "array::len((SELECT VALUE id FROM participant WITH INDEX outgoing WHERE in=$parent.id AND out IN $targets AND (array::len($fields)=0 OR field IN $fields) LIMIT 1))>0 OR array::len((SELECT VALUE id FROM reference WITH INDEX outgoing WHERE in=$parent.id AND out IN $targets AND (array::len($fields)=0 OR field IN $fields) LIMIT 1))>0".into()
            }
        };
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?.name();
        let mut rows = if self.scope.is_none() {
            self.record_stream_prepared::<R>(&predicate, bindings, preparation, "semantic_key")?
        } else {
            let statements = if connected {
                ["participant", "reference"].map(|role| format!("SELECT in AS id FROM {role} WITH INDEX incoming WHERE out IN $targets AND (array::len($fields)=0 OR field IN $fields) AND in.semantic_type=$type")).to_vec()
            } else { vec![format!("SELECT id FROM {table}{index} WHERE semantic_type=$type AND ({predicate})")] };
            CanonicalRecords { rows: self.candidate_payload_rows(crate::prepared::PreparedQuery::new(bindings, preparation, statements)?, table, "semantic_key")?, positions: vec![0], physical_kind: true, failed: false, marker: std::marker::PhantomData }
        };
        let result = async { let mut result = Vec::new(); while let Some(row) = rows.next().await? { result.push(row); } Ok(result) }.await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("native typed record drainage", rows.drain_transport().await);
        lctx_model::domain::completion::complete(result, completion)
    }
    /// Hydrate one ordered window through its exact physical record pointers. A nominal
    /// family predicate is never a substitute for the already selected backing records.
    pub async fn records_from_candidates<R: Record + DeserializeOwned>(
        &self,
        candidates: &[crate::ordered_rows::Candidate],
    ) -> Result<Vec<R>, ModelError> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        if candidates.len() > 128 {
            return Err(ModelError::Limit {
                owner: "native-candidate-hydration",
                limit: "window rows",
                observed: candidates.len(),
                bound: 128,
            });
        }
        let table = crate::schema::ScopeTable::for_relation(R::NAME)?;
        if table == crate::schema::ScopeTable::CompilerRecord {
            return Err(ModelError::Schema("native candidate canonical family"));
        }
        if candidates.iter().any(|candidate| {
            candidate.relation != R::NAME || candidate.node.table.as_str() != table.name()
        }) || candidates.windows(2).any(|pair| pair[0].key >= pair[1].key)
        {
            return Err(ModelError::Conflict("native candidate identity/order"));
        }
        let mut bindings = Variables::new();
        bindings.insert(
            "nodes",
            candidates
                .iter()
                .map(|candidate| candidate.node.clone())
                .collect::<Vec<_>>(),
        );
        let requested = candidates.iter().map(|candidate| candidate.node.clone()).collect::<Vec<_>>();
        if self.selected_candidate_ids(&requested, &self.scratch_budget()?).await?.len() != requested.len() { return Err(ModelError::Conflict("native candidate outside exact view")); }
        let query = crate::prepared::PreparedQuery::new(bindings, vec![], vec![candidate_records_sql().into()])?;
        let position = query.result_positions()[0];
        let terminals = query.expected_terminals();
        let (sql, bindings) = query.with_bindings(self.view_bindings()).into_request();
        self.check_read_admission()?;
        let mut response = self
            .client
            .query(sql)
            .bind(bindings)
            .await
            .map_err(sdk_error)?
            .check()
            .map_err(sdk_error)?;
        if response.num_statements() != terminals {
            return Err(ModelError::Schema("native candidate response inventory"));
        }
        let rows: Vec<surrealdb::types::Object> = response.take(position).map_err(sdk_error)?;
        if rows.len() != candidates.len() {
            return Err(ModelError::Conflict("native candidate backing missing"));
        }
        let mut result = Vec::with_capacity(rows.len());
        for (projection, candidate) in rows.into_iter().zip(candidates) {
            if projection.get("id") != Some(&Value::RecordId(candidate.node.clone()))
                || projection.get("semantic_type") != Some(&Value::String(R::NAME.into()))
                || projection.get("semantic_key")
                    != Some(&Value::String(hex::encode(candidate.key)))
            {
                return Err(ModelError::Conflict("native candidate backing identity"));
            }
            let Some(Value::Bytes(canonical)) = projection.get("canonical") else {
                return Err(corrupt());
            };
            let (row, node) = match table {
                crate::schema::ScopeTable::Entity => canonical_entity_projection::<R>(canonical)?,
                crate::schema::ScopeTable::Assertion => {
                    canonical_assertion_projection::<R>(canonical)?
                }
                crate::schema::ScopeTable::CompilerRecord => {
                    return Err(ModelError::Schema("native candidate canonical family"));
                }
            };
            if *row.id().bytes() != candidate.key || node != candidate.node {
                return Err(ModelError::Conflict("native candidate canonical identity"));
            }
            result.push(row);
        }
        Ok(result)
    }
}

/// Ordering state is bounded by the caller's physical pointer window, never a family scan.
pub fn candidate_records_sql() -> &'static str {
    "SELECT id,semantic_type,semantic_key,canonical FROM $nodes ORDER BY semantic_key"
}

impl NativeReader<SnapshotHandle> {
    pub async fn open(
        client: Arc<Surreal<Client>>,
        handle: SnapshotHandle,
    ) -> Result<Self, ModelError> {
        crate::control::check_installation(&client, handle.service_generation).await?;
        let mut bindings = Variables::new();
        bindings.insert(
            "publication",
            RecordId::new("publication", handle.publication.hex()),
        );
        let mut response = client
            .query("SELECT handle,views,manifest,definition_epoch FROM $publication")
            .bind(bindings)
            .await
            .map_err(sdk_error)?
            .check()
            .map_err(sdk_error)?;
        let rows: Vec<surrealdb::types::Object> = response.take(0).map_err(sdk_error)?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("snapshot publication missing"));
        };
        let expected = hex::encode(serde_json::to_vec(&handle).map_err(ModelError::codec)?);
        if row.get("handle") != Some(&Value::String(expected))
            || row.get("definition_epoch") != Some(&Value::String(handle.definition_epoch.hex()))
        {
            return Err(ModelError::Conflict("snapshot publication handle"));
        }
        let Some(Value::Bytes(manifest_bytes)) = row.get("manifest") else {
            return Err(ModelError::Schema("publication manifest"));
        };
        let manifest = lctx_model::domain::graph::Manifest::decode(manifest_bytes)?;
        if manifest.content() != handle.semantic {
            return Err(ModelError::Conflict("publication semantic manifest"));
        }
        handle.validate_identity()?;
        let Some(Value::Bytes(bytes)) = row.get("views") else {
            return Err(ModelError::Schema("publication exact views"));
        };
        let completed: Vec<lctx_model::domain::completed::CompletedBinding> =
            serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if lctx_model::domain::completed::binding_inventory_identity(&completed)? != handle.view {
            return Err(ModelError::Conflict("publication exact view inventory"));
        }
        let mut all_views = completed
            .iter()
            .map(|binding| binding.view.identity)
            .collect::<Vec<_>>();
        all_views.sort();
        all_views.dedup();
        let mut views = completed
            .into_iter()
            .filter(|binding| binding.boundary.is_none())
            .map(|binding| binding.view.identity)
            .collect::<Vec<_>>();
        views.sort();
        views.dedup();
        let pin = crate::control::ReaderPin::acquire(client.clone(), &all_views).await?;
        if let Err(error) = pin.protect(RecordId::new("publication", handle.publication.hex())).await {
            if !error.permits_storage_cleanup() { pin.retain_unknown(); }
            let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("failed reader setup pin release", pin.release().await);
            return lctx_model::domain::completion::complete(Err(error), completion);
        }
        Ok(Self {
            client,
            handle,
            cancellation: None, budget: None, request_budget: None,
            scope: Some(Arc::new(ReaderScope {
                originals: Some(manifest.originals),
                views,
                _pin: Some(pin),
                selection: std::sync::Mutex::new(None),
            })),
        })
    }
    pub async fn connect(
        endpoint: &str,
        credentials: &Credentials,
        handle: SnapshotHandle,
    ) -> Result<Self, ModelError> {
        let client = connect(
            endpoint,
            credentials,
            handle.database.namespace.as_str(),
            handle.database.database.as_str(),
        )
        .await?;
        let result = Self::open(client.clone(), handle).await;
        if result.is_err() {
            let mut completion = lctx_model::domain::completion::Completion::default(); completion.step("failed reader setup session invalidation", client.invalidate().await.map_err(ModelError::codec));
            return lctx_model::domain::completion::complete(result, completion);
        }
        result
    }
    pub fn handle(&self) -> &SnapshotHandle {
        &self.handle
    }
}
impl NativeReader<()> {
    pub fn for_views(
        client: Arc<Surreal<Client>>,
        views: Vec<lctx_model::domain::ContentHash>,
    ) -> Self {
        Self {
            client,
            handle: (),
            cancellation: None, budget: None, request_budget: None,
            scope: Some(Arc::new(ReaderScope {
                originals: None,
                views,
                _pin: None,
                selection: std::sync::Mutex::new(None),
            })),
        }
    }

    /// Owner-supplied private native access, with no published snapshot capability.
    /// The owner must drain/discard its private target after any terminal failure.
    pub fn private(client: Arc<Surreal<Client>>) -> Self {
        Self {
            client,
            handle: (),
            cancellation: None, budget: None, request_budget: None,
            scope: None,
        }
    }
}

/// Preserve recognized work ceilings and the original typed SDK cause.
pub(crate) fn sdk_error(error: surrealdb::Error) -> ModelError {
    if matches!(
        error.query_details(),
        Some(surrealdb::types::QueryError::TimedOut { .. })
    ) {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step(
            "native SDK refusal",
            Err(ModelError::Cause(Box::new(error))),
        );
        lctx_model::domain::completion::complete::<()>(
            Err(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::ResourceRefused,
            )),
            completion,
        )
        .unwrap_err()
    } else {
        ModelError::Cause(Box::new(error))
    }
}
fn unconfirmed_stream(error: ModelError) -> ModelError {
    let mut completion = lctx_model::domain::completion::Completion::default();
    completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
    lctx_model::domain::completion::complete::<()>(Err(error), completion).unwrap_err()
}

pub struct NativeRows {
    stream: futures::stream::BoxStream<'static, surrealdb::Result<surrealdb::method::StreamItem>>,
    // Declaration order matters: dropping the stream signals cancellation before the final
    // session handle can be released. Explicit drainage retains both through completion.
    client: Option<Arc<Surreal<Client>>>,
    transaction: Option<Arc<surrealdb::method::Transaction<Client>>>,
    scope: Option<Arc<ReaderScope>>,
    statements: usize,
    ended: usize,
    exhausted: bool,
    failure: Option<Arc<ModelError>>,
    drain_errors: Vec<Arc<ModelError>>,
    drain_row_order_error: bool,
    row_bytes: usize,
    owned_result: Option<futures::future::Shared<futures::future::BoxFuture<'static, Result<(), Arc<ModelError>>>>>,
}
impl NativeRows {
    /// The driver owns all native preparation through its checked terminal. Receiver loss
    /// stops delivery; admitted native reads still drain before the synthetic terminal.
    pub(crate) fn owned<F, Fut>(driver: F) -> Result<Self, ModelError>
    where F: FnOnce(tokio::sync::mpsc::Sender<Value>) -> Fut + Send + 'static,
          Fut: std::future::Future<Output = Result<(), ModelError>> + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (terminal_sender, terminal_receiver) = tokio::sync::oneshot::channel();
        tokio::spawn(async move { let result = driver(sender).await; let _ = terminal_sender.send(result); });
        let rows = futures::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.map(|value| (Ok(surrealdb::method::StreamItem::Row { statement: 0, value }), receiver))
        });
        let result = async move {
            terminal_receiver.await.map_err(|error| Arc::new(ModelError::infrastructure(
                lctx_model::domain::Infrastructure::Unconfirmed, format!("selected driver lost terminal: {error}"))))?
                .map_err(Arc::new)
        }.boxed().shared();
        let terminal_result = result.clone();
        let terminal = futures::stream::once(async move {
            let _ = terminal_result.await;
            Ok(surrealdb::method::StreamItem::StatementEnd { statement: 0, stats: Default::default(), result: Ok(()) })
        });
        let mut rows = Self::new(rows.chain(terminal), 1)?;
        rows.owned_result = Some(result);
        Ok(rows)
    }
    pub(crate) fn new(
        stream: impl futures::Stream<Item = surrealdb::Result<surrealdb::method::StreamItem>>
        + Send
        + 'static,
        statements: usize,
    ) -> Result<Self, ModelError> {
        if statements == 0 {
            return Err(ModelError::Schema("native stream statement inventory"));
        }
        Ok(Self {
            stream: stream.boxed(),
            client: None,
            transaction: None,
            scope: None,
            statements,
            ended: 0,
            exhausted: false,
            failure: None,
            drain_errors: Vec::new(),
            drain_row_order_error: false,
            row_bytes: 1024 * 1024,
            owned_result: None,
        })
    }
    /// Close synthetic delivery while keeping its independent driver terminal alive.
    /// Raw transport streams remain owned until checked drainage.
    pub(crate) fn cancel_delivery(&mut self) {
        if self.owned_result.is_some() {
            self.stream = futures::stream::empty().boxed();
            self.exhausted = true; self.ended = self.statements;
        }
    }
    pub(crate) fn with_client(mut self, client: Arc<Surreal<Client>>) -> Self {
        self.client = Some(client);
        self
    }
    pub(crate) fn with_transaction(mut self, transaction: Arc<surrealdb::method::Transaction<Client>>) -> Self {
        self.transaction = Some(transaction); self
    }
    pub(crate) fn with_row_bytes(mut self, row_bytes: usize) -> Self {
        self.row_bytes = row_bytes;
        self
    }
    /// Finish the SDK transport after a semantic or envelope failure. The original failure
    /// remains sticky; draining does not turn the failed query into an accepted result.
    pub(crate) fn remember_failure(&mut self, error: ModelError) -> ModelError {
        let cause = self.failure.get_or_insert_with(|| Arc::new(error)).clone();
        ModelError::SharedCause(cause)
    }
    /// Consume every remaining statement and the outer transport, retaining late errors.
    /// Interrupted drainage can be retried on this same owner; it never admits failed rows.
    pub async fn drain_transport(&mut self) -> Result<(), ModelError> {
        if !self.exhausted {
            while let Some(item) = self.stream.next().await {
                match item {
                    Err(error) => self.drain_errors.push(Arc::new(unconfirmed_stream(sdk_error(error)))),
                    Ok(surrealdb::method::StreamItem::Row { statement, .. }) => {
                        if (statement != self.ended || statement >= self.statements)
                            && !self.drain_row_order_error
                        {
                            self.drain_row_order_error = true;
                            self.drain_errors.push(Arc::new(unconfirmed_stream(ModelError::Schema(
                                "native stream drainage row order",
                            ))));
                        }
                    }
                    Ok(surrealdb::method::StreamItem::StatementEnd {
                        statement, result, ..
                    }) => {
                        if statement != self.ended || statement >= self.statements {
                            self.drain_errors.push(Arc::new(unconfirmed_stream(ModelError::Schema(
                                "native stream drainage terminal order",
                            ))));
                        } else {
                            self.ended += 1;
                        }
                        if let Err(error) = result {
                            self.drain_errors.push(Arc::new(sdk_error(error)));
                        }
                    }
                }
            }
            self.exhausted = true;
            if self.ended != self.statements {
                self.drain_errors.push(Arc::new(ModelError::infrastructure(
                    lctx_model::domain::Infrastructure::Unconfirmed, "native stream drainage missing terminal")));
            }
        }
        if self.ended != self.statements {
            if let Some(scope) = &self.scope {
                if let Some(pin) = &scope._pin {
                    pin.retain_unknown();
                }
            }
        }
        if let Some(result) = self.owned_result.as_ref().cloned() {
            let result = result.await; self.owned_result.take();
            if let Err(error) = result { self.drain_errors.push(error); }
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        if self.ended != self.statements || self.failure.as_ref().is_some_and(|error| !error.permits_storage_cleanup()) {
            completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
        }
        for error in &self.drain_errors {
            completion.step(
                "native stream finalization",
                Err(ModelError::SharedCause(error.clone())),
            );
        }
        if completion.remote == lctx_model::domain::completion::RemoteState::Unknown {
            if let Some(pin) = self.scope.as_ref().and_then(|scope| scope._pin.as_ref()) { pin.retain_unknown(); }
        }
        self.scope.take();
        lctx_model::domain::completion::complete(Ok(()), completion)
    }
    pub async fn next(&mut self) -> Result<Option<Value>, ModelError> {
        if let Some(error) = &self.failure {
            return Err(ModelError::SharedCause(error.clone()));
        }
        match self.read_next().await {
            Ok(row) => Ok(row),
            Err(error) => Err(self.remember_failure(error)),
        }
    }
    async fn read_next(&mut self) -> Result<Option<Value>, ModelError> {
        if self.exhausted && self.owned_result.is_none() { return Ok(None); }
        while let Some(item) = self.stream.next().await {
            match item.map_err(|error| unconfirmed_stream(sdk_error(error)))? {
                surrealdb::method::StreamItem::Row { statement, value } => {
                    if statement != self.ended || statement >= self.statements {
                        return Err(unconfirmed_stream(ModelError::Schema("native stream row order")));
                    }
                    // The SDK queue is row bounded, not byte bounded. Refuse an oversized source
                    // value before decoding or expansion; this is not a server RSS guarantee.
                    let bytes = crate::loader::native_bytes(&value);
                    if bytes > self.row_bytes {
                        return Err(ModelError::Limit {
                            owner: "native-stream",
                            limit: "row bytes",
                            observed: bytes,
                            bound: self.row_bytes,
                        });
                    }
                    return Ok(Some(value));
                }
                surrealdb::method::StreamItem::StatementEnd {
                    statement, result, ..
                } => {
                    if statement != self.ended || statement >= self.statements {
                        return Err(unconfirmed_stream(ModelError::Schema("native stream terminal order")));
                    }
                    self.ended += 1;
                    result.map_err(sdk_error)?;
                }
            }
        }
        self.exhausted = true;
        if self.ended != self.statements {
            if let Some(scope) = &self.scope {
                if let Some(pin) = &scope._pin {
                    pin.retain_unknown();
                }
            }
            return Err(ModelError::infrastructure(lctx_model::domain::Infrastructure::Unconfirmed, "native stream missing terminal success"));
        }
        if let Some(result) = self.owned_result.as_ref().cloned() {
            let result = result.await; self.owned_result.take();
            if let Err(error) = result {
                if !error.permits_storage_cleanup() { if let Some(pin) = self.scope.as_ref().and_then(|scope| scope._pin.as_ref()) { pin.retain_unknown(); } }
                return Err(ModelError::SharedCause(error));
            }
        }
        self.scope.take();
        Ok(None)
    }
}

impl Drop for NativeRows {
    fn drop(&mut self) {
        let Some(scope) = self.scope.take() else {
            return;
        };
        if self.exhausted && self.ended == self.statements && self.owned_result.is_none() {
            if self.failure.as_ref().is_some_and(|error| !error.permits_storage_cleanup()) {
                if let Some(pin) = &scope._pin { pin.retain_unknown(); }
            }
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            if let Some(pin) = &scope._pin {
                pin.retain_unknown();
            }
            return;
        };
        self.cancel_delivery();
        let mut drain = NativeRows {
            stream: std::mem::replace(&mut self.stream, futures::stream::empty().boxed()),
            client: self.client.take(), transaction: self.transaction.take(), scope: Some(scope),
            statements: self.statements, ended: self.ended, exhausted: self.exhausted,
            failure: self.failure.take(), drain_errors: std::mem::take(&mut self.drain_errors),
            drain_row_order_error: self.drain_row_order_error, row_bytes: self.row_bytes,
            owned_result: self.owned_result.take(),
        };
        runtime.spawn(async move {
            let result = drain.drain_transport().await;
            if result.is_err() || drain.failure.is_some() {
                tracing::error!(primary = ?drain.failure, completion = ?result, "cancelled native read terminal outcome");
            }
        });
    }
}

pub struct CanonicalRecords<R> {
    rows: NativeRows,
    positions: Vec<usize>,
    physical_kind: bool,
    failed: bool,
    marker: std::marker::PhantomData<R>,
}
impl<R: Record + DeserializeOwned> CanonicalRecords<R> {
    /// Drain the owning transport without accepting a previously failed typed stream.
    /// Late statement/transport errors remain structured finalization failures.
    pub async fn drain_transport(&mut self) -> Result<(), ModelError> {
        self.rows.drain_transport().await
    }
    pub async fn next(&mut self) -> Result<Option<R>, ModelError> {
        if self.failed {
            return Err(ModelError::Conflict("failed canonical stream"));
        }
        let result = self.read_next().await;
        self.failed |= result.is_err();
        result
    }
    async fn read_next(&mut self) -> Result<Option<R>, ModelError> {
        let Some(value) = self.rows.next().await? else {
            return Ok(None);
        };
        let Value::Object(projection) = value else {
            return Err(corrupt());
        };
        let canonical = Bytes::from_value(projection.get("canonical").ok_or_else(corrupt)?.clone())
            .map_err(|_| corrupt())?;
        // Statement identity is supplied by the checked SDK envelope, not a string projection.
        // ORDER BY fields are selected for SurrealQL but never interpreted as canonical authority.
        if self.physical_kind {
            let id = RecordId::from_value(projection.get("id").cloned().ok_or_else(corrupt)?).map_err(|_| corrupt())?;
            return match id.table.as_str() {
                "entity" => canonical_entity::<R>(&canonical).map(Some),
                "assertion" => canonical_assertion::<R>(&canonical).map(Some),
                _ => Err(ModelError::Schema("native canonical family")),
            };
        }
        let row = match self
            .positions
            .iter()
            .position(|position| *position == self.rows.ended)
        {
            Some(0) => canonical_entity::<R>(&canonical)?,
            Some(1) => canonical_assertion::<R>(&canonical)?,
            _ => return Err(ModelError::Schema("native stream canonical statement")),
        };
        Ok(Some(row))
    }
}
fn corrupt() -> ModelError {
    ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt)
}
pub(crate) fn canonical_error(error: ModelError) -> ModelError {
    match error {
        refusal @ (ModelError::Resource { .. }
        | ModelError::Limit { .. }
        | ModelError::Serving(_)
        | ModelError::Completion(_)
        | ModelError::SharedCause(_)
        | ModelError::Cause(_)) => refusal,
        _ => corrupt(),
    }
}
fn canonical_entity<R: Record + DeserializeOwned>(payload: &[u8]) -> Result<R, ModelError> {
    canonical_entity_projection(payload).map(|(row, _)| row)
}
fn canonical_entity_projection<R: Record + DeserializeOwned>(
    payload: &[u8],
) -> Result<(R, RecordId), ModelError> {
    let entity: Entity = serde_json::from_slice(payload).map_err(|_| corrupt())?;
    entity.validate().map_err(canonical_error)?;
    Ok((
        crate::codec::entity_record::<R>(&entity).map_err(canonical_error)?,
        crate::loader::entity_payload_id(&entity)?,
    ))
}
fn canonical_assertion<R: Record + DeserializeOwned>(payload: &[u8]) -> Result<R, ModelError> {
    canonical_assertion_projection(payload).map(|(row, _)| row)
}
fn canonical_assertion_projection<R: Record + DeserializeOwned>(
    payload: &[u8],
) -> Result<(R, RecordId), ModelError> {
    let assertion: Assertion = serde_json::from_slice(payload).map_err(|_| corrupt())?;
    assertion.validate().map_err(canonical_error)?;
    Ok((
        crate::codec::assertion_record::<R>(&assertion).map_err(canonical_error)?,
        crate::loader::assertion_payload_id(&assertion)?,
    ))
}
pub fn target_id(target: Target) -> RecordId {
    match target {
        Target::Entity(id) => RecordId::new("entity_anchor", id.0.hex()),
        Target::Assertion(id) => RecordId::new("assertion_anchor", id.0.hex()),
        external @ Target::External { .. } => {
            let mut sink = KeySink::new("graph-external-endpoint/v1");
            external.encode(&mut sink);
            RecordId::new("external", sink.finish().hex())
        }
    }
}
pub async fn connect(
    endpoint: &str,
    credentials: &Credentials,
    namespace: &str,
    database: &str,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    let client = authenticated(endpoint, credentials, Some((namespace, database))).await?;
    client
        .use_ns(namespace)
        .use_db(database)
        .await
        .map_err(ModelError::codec)?;
    Ok(client)
}
/// Authenticate without selecting or implicitly creating a database. Private compiler creation
/// can therefore establish STRICT before selecting its first storage session.
pub async fn authenticated(
    endpoint: &str,
    credentials: &Credentials,
    scope: Option<(&str, &str)>,
) -> Result<Arc<Surreal<Client>>, ModelError> {
    let client = Surreal::new::<Grpc>(
        endpoint
            .strip_prefix("grpc://")
            .ok_or_else(|| ModelError::Invalid("native gRPC endpoint required".into()))?,
    )
    .await
    .map_err(ModelError::codec)?;
    match credentials {
        Credentials::Root { username, password } => {
            client
                .signin(Root {
                    username: username.clone(),
                    password: password.clone(),
                })
                .await
                .map_err(ModelError::codec)?;
        }
        Credentials::Database { username, password } => {
            let (namespace, database) =
                scope.ok_or(ModelError::Schema("database authentication scope"))?;
            client
                .signin(Database {
                    namespace: namespace.into(),
                    database: database.into(),
                    username: username.clone(),
                    password: password.clone(),
                })
                .await
                .map_err(ModelError::codec)?;
        }
    }
    Ok(Arc::new(client))
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    use surrealdb::method::StreamItem;
    #[tokio::test]
    async fn empty_relation_scope_preserves_admission_and_budget_checks_without_native_queries() {
        let budget = lctx_model::domain::resources::ResourceBudget::fixed(1 << 20).unwrap();
        let reader = NativeReader::for_views(Arc::new(Surreal::init()), vec![]).with_budget(&budget);
        let mut rows = reader.relation_bodies("packages", 0).unwrap();
        assert!(rows.next().await.unwrap().is_none()); rows.drain_transport().await.unwrap(); drop(rows);
        let nonempty = NativeReader::for_views(Arc::new(Surreal::init()), vec![lctx_model::domain::ContentHash::of(b"zero-limit-view")]).with_budget(&budget);
        let mut rows = nonempty.relation_bodies("packages", 0).unwrap();
        assert!(rows.next().await.unwrap().is_none()); rows.drain_transport().await.unwrap(); drop(rows);
        assert!(nonempty.relation_bodies("not-a-model-relation", 0).is_err(), "zero demand still validates the relation schema");
        nonempty.close().await.unwrap();
        assert_eq!(budget.reserved(), 0);
        let cancelled = reader.with_read_cancellation(Arc::new(std::sync::atomic::AtomicBool::new(true)));
        assert!(cancelled.relation_bodies("packages", 0).is_err());
        let unadmitted = NativeReader::for_views(Arc::new(Surreal::init()), vec![]);
        assert!(unadmitted.relation_bodies("packages", 0).is_err());
    }
    #[tokio::test]
    async fn reader_terminal_guard_retains_actual_pin_on_unwind_without_an_extra_scope_borrower() {
        use futures::FutureExt;
        let pin = crate::control::ReaderPin::test_detached(false);
        let mut reader = NativeReader::for_views(Arc::new(Surreal::init()), vec![]);
        Arc::get_mut(reader.scope.as_mut().unwrap()).unwrap()._pin = Some(pin.clone());
        let scope_borrowers = Arc::strong_count(reader.scope.as_ref().unwrap());
        let guard = reader.protect_terminal_close();
        assert_eq!(Arc::strong_count(reader.scope.as_ref().unwrap()), scope_borrowers);
        let panic = std::panic::AssertUnwindSafe(async move {
            let _guard = guard;
            panic!("read owner unwind before terminal close");
        }).catch_unwind().await;
        assert!(panic.is_err()); assert!(pin.test_uncertain());
        assert!(!reader.close().await.unwrap_err().permits_storage_cleanup());
        let pin = crate::control::ReaderPin::test_detached(true);
        Arc::get_mut(reader.scope.as_mut().unwrap()).unwrap()._pin = Some(pin.clone());
        let mut guard = reader.protect_terminal_close();
        guard.close(&reader).await.unwrap(); drop(guard);
        assert!(!pin.test_uncertain(), "successful explicit close disarms protection");
    }
    #[tokio::test]
    async fn cancelled_read_owner_fences_reader_loader_and_selected_native_admission() {
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let reader = NativeReader::private(Arc::new(Surreal::init())).with_read_cancellation(flag.clone());
        let error = reader.query_native::<Value>("RETURN 1", Variables::new()).await.unwrap_err();
        assert!(error.to_string().contains("delivery cancelled"), "must refuse before unconnected SDK client: {error}");
        let loader = crate::Loader::for_views(reader.shared_client(), vec![]).with_read_cancellation(flag);
        assert!(loader.check_read_admission().is_err());
        let error = match loader.reader().query_stream("RETURN 1", Variables::new(), 1) { Err(error) => error, Ok(_) => panic!("cancelled stream admitted") };
        assert!(error.to_string().contains("delivery cancelled"));
    }
    #[test]
    fn typed_sdk_timeout_is_resource_refused_without_message_classification() {
        let error = surrealdb::Error::query(
            "opaque engine reason".into(),
            surrealdb::types::QueryError::TimedOut {
                duration: std::time::Duration::from_secs(10),
            },
        );
        assert!(matches!(
            sdk_error(error).primary(),
            Some(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::ResourceRefused
            ))
        ));
    }
    #[test]
    fn other_sdk_errors_preserve_typed_payload_even_with_timeout_wording() {
        for error in [
            surrealdb::Error::query("timeout without typed details".into(), None),
            surrealdb::Error::query("cancelled".into(), surrealdb::types::QueryError::Cancelled),
            surrealdb::Error::query(
                "conflict".into(),
                surrealdb::types::QueryError::TransactionConflict,
            ),
            surrealdb::Error::internal("exceeded the timeout: 10s".into()),
            surrealdb::Error::serialization("serialization failure".into(), None),
        ] {
            let expected = error.to_string();
            assert!(
                matches!(sdk_error(error).primary(), Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some() && cause.to_string()==expected)
            );
        }
    }
    fn row() -> StreamItem {
        let mut value = surrealdb::types::Object::new();
        value.insert("id", RecordId::new("entity", "row"));
        StreamItem::Row {
            statement: 0,
            value: Value::Object(value),
        }
    }
    fn end(result: surrealdb::Result<()>) -> StreamItem {
        StreamItem::StatementEnd {
            statement: 0,
            stats: Default::default(),
            result,
        }
    }
    fn failure() -> surrealdb::Error {
        surrealdb::Error::internal("injected late stream failure".into())
    }
    #[tokio::test]
    async fn missing_terminal_and_late_physical_failure_forbid_pin_cleanup() {
        let mut missing = NativeRows::new(futures::stream::iter([Ok(row())]), 1).unwrap();
        assert!(missing.next().await.unwrap().is_some());
        let primary = missing.next().await.unwrap_err();
        assert!(!primary.permits_storage_cleanup());
        assert!(!missing.drain_transport().await.unwrap_err().permits_storage_cleanup());
        let mut late = NativeRows::new(futures::stream::iter([Ok(end(Ok(()))), Err(failure())]), 1).unwrap();
        let primary = late.next().await.unwrap_err();
        assert!(matches!(primary.primary(), Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some()));
        assert!(!primary.permits_storage_cleanup());
        assert!(!late.drain_transport().await.unwrap_err().permits_storage_cleanup());
        let mut semantic = NativeRows::new(futures::stream::iter([Ok(end(Err(surrealdb::Error::query("confirmed statement failure".into(), None))))]), 1).unwrap();
        assert!(semantic.next().await.unwrap_err().permits_storage_cleanup());
        semantic.drain_transport().await.unwrap();
    }
    #[tokio::test]
    async fn interrupted_transport_drain_retains_primary_and_both_late_errors() {
        let (release, gate) = tokio::sync::oneshot::channel::<()>();
        let initial = futures::stream::iter(vec![
            Ok(end(Err(surrealdb::Error::query(
                "primary statement failure".into(),
                None,
            )))),
            Err(surrealdb::Error::internal("late transport failure".into())),
        ]);
        let tail = futures::stream::once(async move {
            gate.await.unwrap();
            Ok(StreamItem::StatementEnd {
                statement: 1,
                stats: Default::default(),
                result: Err(surrealdb::Error::query(
                    "later statement failure".into(),
                    None,
                )),
            })
        });
        let mut rows = NativeRows::new(initial.chain(tail), 2).unwrap();
        let primary = rows.next().await.unwrap_err();
        assert!(
            matches!(primary.primary(),Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some() && cause.to_string().contains("primary statement failure"))
        );
        let original = rows.failure.as_ref().unwrap().clone();
        let mut interrupted = Box::pin(rows.drain_transport());
        assert!(futures::poll!(&mut interrupted).is_pending());
        drop(interrupted);
        assert_eq!(rows.drain_errors.len(), 1);
        assert!(Arc::ptr_eq(&original, rows.failure.as_ref().unwrap()));
        assert_eq!(
            rows.next().await.unwrap_err().to_string(),
            primary.to_string()
        );
        release.send(()).unwrap();
        let error = rows.drain_transport().await.unwrap_err();
        assert!(rows.exhausted);
        assert_eq!(rows.drain_errors.len(), 2);
        let ModelError::Completion(outcome) = error else {
            panic!("structured drain outcome");
        };
        assert!(outcome.primary.is_none());
        assert_eq!(outcome.completion.failures.len(), 2);
        for failure in outcome.completion.failures {
            assert!(
                matches!(failure.error.primary(),Some(ModelError::Cause(cause)) if cause.downcast_ref::<surrealdb::Error>().is_some())
            );
        }
        assert!(Arc::ptr_eq(&original, rows.failure.as_ref().unwrap()));
        assert!(rows.drain_transport().await.is_err());
    }
    #[tokio::test]
    async fn canonical_projection_decodes_one_valid_record_then_checks_completion() {
        let package = lctx_model::domain::input::Package {
            name: "projection".into(),
        };
        let mut node = surrealdb::types::Object::new();
        node.insert("node_kind", "entity");
        node.insert(
            "canonical",
            Bytes::from(serde_json::to_vec(&Entity::from(package.clone())).unwrap()),
        );
        let rows = NativeRows::new(
            futures::stream::iter(vec![
                Ok(StreamItem::Row {
                    statement: 0,
                    value: Value::Object(node),
                }),
                Ok(end(Ok(()))),
            ]),
            1,
        )
        .unwrap();
        let mut records = CanonicalRecords::<lctx_model::domain::input::Package> {
            rows,
            positions: vec![0],
            physical_kind: false,
            failed: false,
            marker: std::marker::PhantomData,
        };
        assert_eq!(records.next().await.unwrap(), Some(package));
        assert!(records.next().await.unwrap().is_none());
    }
    #[tokio::test]
    async fn malformed_canonical_stream_cannot_later_report_completion() {
        let mut node = surrealdb::types::Object::new();
        node.insert("node_kind", "entity");
        node.insert("canonical", Bytes::from(b"malformed".to_vec()));
        let rows = NativeRows::new(
            futures::stream::iter(vec![
                Ok(StreamItem::Row {
                    statement: 0,
                    value: Value::Object(node),
                }),
                Ok(end(Ok(()))),
            ]),
            1,
        )
        .unwrap();
        let mut records = CanonicalRecords::<lctx_model::domain::input::Package> {
            rows,
            positions: vec![0],
            physical_kind: false,
            failed: false,
            marker: std::marker::PhantomData,
        };
        assert!(matches!(
            records.next().await,
            Err(ModelError::Serving(
                lctx_model::domain::serving::FailureKind::Corrupt
            ))
        ));
        assert!(records.next().await.is_err());
    }
    #[tokio::test]
    async fn stream_rows_are_provisional_until_statement_and_outer_completion() {
        for items in [
            vec![Ok(row()), Ok(end(Err(failure())))],
            vec![Ok(row()), Ok(end(Ok(()))), Err(failure())],
            vec![Ok(row())],
        ] {
            let mut rows = NativeRows::new(futures::stream::iter(items), 1).unwrap();
            assert!(rows.next().await.unwrap().is_some());
            assert!(rows.next().await.is_err());
            assert!(rows.next().await.is_err());
        }
        let mut actual = NativeRows::new(
            futures::stream::iter(vec![Ok(row()), Ok(end(Ok(()))), Err(failure())]),
            1,
        )
        .unwrap();
        let StreamItem::Row { value, .. } = row() else {
            unreachable!()
        };
        let mut expected = crate::ordered_rows::SortedRows::new().unwrap();
        expected.push(value).unwrap();
        assert!(
            expected
                .finish()
                .unwrap()
                .reconcile(&mut actual)
                .await
                .is_err(),
            "full expected row equality cannot hide an outer completion error"
        );
    }
}
