use futures::StreamExt;
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
}
impl<Context> NativeReader<Context> {
    /// Rows are provisional. Only exhaustion after every declared statement end and the outer
    /// transport completion establishes success. Private publishers discard their target on error.
    pub fn query_stream(
        &self,
        sql: impl Into<String>,
        bindings: Variables,
        statements: usize,
    ) -> Result<NativeRows, ModelError> {
        self.stream_prepared(crate::prepared::PreparedQuery::from_sql(
            sql.into(),
            bindings,
            statements,
            (0..statements).collect(),
        )?)
    }

    pub fn stream_prepared(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<NativeRows, ModelError> {
        query.stream(&self.client)
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
    async fn checked_native_value(
        &self,
        query: crate::prepared::PreparedQuery,
    ) -> Result<Value, ModelError> {
        if query.result_positions().len() != 1 {
            return Err(ModelError::Schema("native single result demand"));
        }
        let position = query.result_positions()[0];
        let terminals = query.expected_terminals();
        let (sql, bindings) = query.into_request();
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
        let query = crate::prepared::PreparedQuery::new(
            bindings,
            preparation,
            vec![
                format!(
                    "SELECT canonical, {order} FROM entity WHERE semantic_type=$type AND ({predicate}) ORDER BY {order}"
                ),
                format!(
                    "SELECT canonical, {order} FROM assertion WHERE semantic_type=$type AND ({predicate}) ORDER BY {order}"
                ),
            ],
        )?;
        Ok(CanonicalRecords {
            rows: self.stream_prepared(query)?,
            failed: false,
            marker: std::marker::PhantomData,
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
        T::from_value(self.checked_native_value(query).await?).map_err(ModelError::codec)
    }
    pub async fn records<R: Record + DeserializeOwned>(
        &self,
        selection: RecordSelection,
    ) -> Result<Vec<R>, ModelError> {
        let mut bindings = Variables::new();
        bindings.insert("type", R::NAME.to_string());
        let mut preparation = Vec::new();
        let predicate = match selection {
            RecordSelection::Keys(keys) => {
                if keys.is_empty() {
                    return Ok(vec![]);
                }
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
                if targets.is_empty() {
                    return Ok(vec![]);
                }
                bindings.insert(
                    "targets",
                    targets.into_iter().map(target_id).collect::<Vec<_>>(),
                );
                bindings.insert("fields", fields);
                "id IN (SELECT VALUE in FROM participant WHERE out IN $targets AND (array::len($fields)=0 OR field IN $fields)) OR id IN (SELECT VALUE in FROM reference WHERE out IN $targets AND (array::len($fields)=0 OR field IN $fields))".into()
            }
        };
        let query = crate::prepared::PreparedQuery::new(
            bindings,
            preparation,
            vec![
                format!(
                    "SELECT VALUE canonical FROM entity WHERE semantic_type=$type AND ({predicate}) ORDER BY semantic_key"
                ),
                format!(
                    "SELECT VALUE canonical FROM assertion WHERE semantic_type=$type AND ({predicate}) ORDER BY semantic_key"
                ),
            ],
        )?;
        let positions = query.result_positions().to_vec();
        let terminals = query.expected_terminals();
        let (sql, bindings) = query.into_request();
        let mut response = self
            .client
            .query(sql)
            .bind(bindings)
            .await
            .map_err(sdk_error)?
            .check()
            .map_err(sdk_error)?;
        if response.num_statements() != terminals {
            return Err(ModelError::Schema("native record response inventory"));
        }
        let entities: Vec<Bytes> = response.take(positions[0]).map_err(|_| corrupt())?;
        let assertions: Vec<Bytes> = response.take(positions[1]).map_err(|_| corrupt())?;
        let mut result = Vec::with_capacity(entities.len() + assertions.len());
        for payload in entities {
            result.push(canonical_entity::<R>(&payload)?);
        }
        for payload in assertions {
            result.push(canonical_assertion::<R>(&payload)?);
        }
        Ok(result)
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
        let query = crate::prepared::PreparedQuery::new(
            bindings,
            vec![],
            vec![candidate_records_sql().into()],
        )?;
        let position = query.result_positions()[0];
        let terminals = query.expected_terminals();
        let (sql, bindings) = query.into_request();
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
    pub fn new(client: Arc<Surreal<Client>>, handle: SnapshotHandle) -> Self {
        Self { client, handle }
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
        let reader = Self::new(client, handle);
        let markers: Vec<String> = reader
            .query(
                "SELECT VALUE handle FROM publication:current",
                Variables::new(),
            )
            .await?;
        let expected = hex::encode(serde_json::to_vec(reader.handle()).map_err(ModelError::codec)?);
        if markers != vec![expected] {
            return Err(ModelError::Conflict("snapshot publication handle"));
        }
        Ok(reader)
    }
    pub fn handle(&self) -> &SnapshotHandle {
        &self.handle
    }
}
impl NativeReader<()> {
    /// Owner-supplied private native access, with no published snapshot capability.
    /// The owner must drain/discard its private target after any terminal failure.
    pub fn private(client: Arc<Surreal<Client>>) -> Self {
        Self { client, handle: () }
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

pub struct NativeRows {
    stream: futures::stream::BoxStream<'static, surrealdb::Result<surrealdb::method::StreamItem>>,
    statements: usize,
    ended: usize,
    exhausted: bool,
    failure: Option<Arc<ModelError>>,
    drain_errors: Vec<Arc<ModelError>>,
    drain_row_order_error: bool,
    row_bytes: usize,
}
impl NativeRows {
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
            statements,
            ended: 0,
            exhausted: false,
            failure: None,
            drain_errors: Vec::new(),
            drain_row_order_error: false,
            row_bytes: 1024 * 1024,
        })
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
                    Err(error) => self.drain_errors.push(Arc::new(sdk_error(error))),
                    Ok(surrealdb::method::StreamItem::Row { statement, .. }) => {
                        if (statement != self.ended || statement >= self.statements)
                            && !self.drain_row_order_error
                        {
                            self.drain_row_order_error = true;
                            self.drain_errors.push(Arc::new(ModelError::Schema(
                                "native stream drainage row order",
                            )));
                        }
                    }
                    Ok(surrealdb::method::StreamItem::StatementEnd {
                        statement, result, ..
                    }) => {
                        if statement != self.ended || statement >= self.statements {
                            self.drain_errors.push(Arc::new(ModelError::Schema(
                                "native stream drainage terminal order",
                            )));
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
                self.drain_errors.push(Arc::new(ModelError::Schema(
                    "native stream drainage missing terminal",
                )));
            }
        }
        let mut completion = lctx_model::domain::completion::Completion::default();
        for error in &self.drain_errors {
            completion.step(
                "native stream finalization",
                Err(ModelError::SharedCause(error.clone())),
            );
        }
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
        if self.exhausted {
            return Ok(None);
        }
        while let Some(item) = self.stream.next().await {
            match item.map_err(sdk_error)? {
                surrealdb::method::StreamItem::Row { statement, value } => {
                    if statement != self.ended || statement >= self.statements {
                        return Err(ModelError::Schema("native stream row order"));
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
                        return Err(ModelError::Schema("native stream terminal order"));
                    }
                    self.ended += 1;
                    result.map_err(sdk_error)?;
                }
            }
        }
        self.exhausted = true;
        if self.ended != self.statements {
            return Err(ModelError::Schema("native stream missing terminal success"));
        }
        Ok(None)
    }
}

pub struct CanonicalRecords<R> {
    rows: NativeRows,
    failed: bool,
    marker: std::marker::PhantomData<R>,
}
impl<R: Record + DeserializeOwned> CanonicalRecords<R> {
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
        let row = match self.rows.ended {
            0 => canonical_entity::<R>(&canonical)?,
            1 => canonical_assertion::<R>(&canonical)?,
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
        target_id(Target::Entity(entity.id())),
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
        target_id(Target::Assertion(assertion.id())),
    ))
}
pub fn target_id(target: Target) -> RecordId {
    match target {
        Target::Entity(id) => RecordId::new("entity", id.0.hex()),
        Target::Assertion(id) => RecordId::new("assertion", id.0.hex()),
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
