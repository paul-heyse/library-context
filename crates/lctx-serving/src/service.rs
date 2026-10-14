//! One fixed native realization, bounded admission, and closed Rust operation transport.
use lctx_model::domain::{resources::ResourceBudget, serving::*, *};
use lctx_native::{NativeReader, RecordSelection};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{OnceCell, Semaphore};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryVector {
    pub spec: ContentHash,
    pub input: ContentHash,
    pub vector: Vec<f32>,
    pub recipe: embedding::QueryRecipe,
    pub projection: Id<embedding::projection::ProjectionDefinition>,
}
/// The encoded invocation keeps its tool name and raw request together.
#[derive(Clone, Copy)]
pub struct ToolInput<'a> {
    pub tool: &'a str,
    pub raw: &'a str,
}
/// Retained transport/preparation state shares the native service's byte budget,
/// without occupying CPU/query demand while waiting for an embedding response.
pub struct TransportRequest {
    _request: crate::preparation::RequestLease,
    owner: Arc<crate::preparation::PreparedCache>,
    charge: Mutex<lctx_model::domain::charged::StateCharge>,
}
impl TransportRequest {
    /// Reserve actual additional retained bytes before keeping a buffer or copy.
    pub fn retain(&self, bytes: usize) -> Result<(), ModelError> {
        self.charge
            .lock()
            .map_err(|_| ModelError::Conflict("transport request charge"))?
            .grow(bytes)
    }
}
pub struct NativeService {
    reader: NativeReader,
    limits: ResourceLimits,
    shared: ResourceBudget,
    queries: Arc<Semaphore>,
    cpu: Arc<Semaphore>,
    definition: OnceCell<()>,
    prepared: Arc<crate::preparation::PreparedCache>,
    retained: crate::ranked_results::RankedResults,
}
impl NativeService {
    pub fn new(reader: NativeReader, limits: ResourceLimits) -> Result<Self, ModelError> {
        limits.validate()?;
        let shared = ResourceBudget::fixed(limits.shared_bytes as usize)?;
        let queries = Arc::new(Semaphore::new(limits.query_connections as usize));
        let cpu = Arc::new(Semaphore::new(limits.cpu_jobs as usize));
        let prepared = crate::preparation::PreparedCache::new(
            reader.handle().clone(),
            &shared,
            &limits,
            queries.clone(),
            cpu.clone(),
        )?;
        let reader = reader.with_budget(&shared);
        let retained = crate::ranked_results::RankedResults::new(&shared);
        Ok(Self {
            shared,
            queries,
            cpu,
            prepared,
            reader,
            limits,
            definition: OnceCell::new(),
            retained,
        })
    }
    /// Admit retained request state before asynchronous preparation or worker creation.
    /// Native demand is acquired later by execute_encoded_for, after preparation.
    pub fn admit_transport(&self, bytes: usize) -> Result<TransportRequest, ModelError> {
        let request = self.prepared.admit_request()?;
        let budget = ResourceBudget::scoped(&self.shared, self.limits.request_bytes as usize)?;
        let mut charge =
            lctx_model::domain::charged::StateCharge::new(&budget, "native-transport-request");
        charge.grow(bytes)?;
        Ok(TransportRequest {
            _request: request,
            owner: self.prepared.clone(),
            charge: Mutex::new(charge),
        })
    }
    /// Fence requests, drain initializer owners and release preparation before client invalidation.
    pub async fn close(&self) {
        self.prepared.close().await;
        self.retained.clear();
    }
    pub fn request_deadline_ms(&self) -> u64 {
        self.limits.request_deadline_ms
    }
    pub fn handle(&self) -> &SnapshotHandle {
        self.reader.handle()
    }
    pub async fn execute(&self, tool: &str, raw: &str) -> Result<String, WireError> {
        self.run(
            tool,
            raw,
            None,
            false,
            self.limits.request_deadline_ms,
            None,
        )
        .await
        .map(EncodedJson::into_string)
    }
    pub async fn execute_unavailable(&self, tool: &str, raw: &str) -> Result<String, WireError> {
        self.run(tool, raw, None, true, self.limits.request_deadline_ms, None)
            .await
            .map(EncodedJson::into_string)
    }
    pub async fn execute_with_vector(
        &self,
        tool: &str,
        raw: &str,
        vector: Option<QueryVector>,
    ) -> Result<String, WireError> {
        self.run(
            tool,
            raw,
            vector,
            false,
            self.limits.request_deadline_ms,
            None,
        )
        .await
        .map(EncodedJson::into_string)
    }
    /// Execute within the transport's remaining request deadline, including native admission.
    pub async fn execute_for(
        &self,
        tool: &str,
        raw: &str,
        vector: Option<QueryVector>,
        unavailable: bool,
        remaining_ms: u64,
    ) -> Result<String, WireError> {
        self.execute_encoded_for(tool, raw, vector, unavailable, remaining_ms)
            .await
            .map(EncodedJson::into_string)
    }
    /// The actual bridge keeps the final-byte reservation live through its transport copy.
    pub async fn execute_encoded_for(
        &self,
        tool: &str,
        raw: &str,
        vector: Option<QueryVector>,
        unavailable: bool,
        remaining_ms: u64,
    ) -> Result<EncodedJson, WireError> {
        self.run(
            tool,
            raw,
            vector,
            unavailable,
            remaining_ms.min(self.limits.request_deadline_ms),
            None,
        )
        .await
    }
    /// Preparation and native execution retain one actual per-request byte pool.
    /// Cooperative cancellation fences subsequent reads and still awaits the
    /// original execution owner/NativeRows terminal and drain paths.
    pub fn execute_admitted_encoded_for<'a>(
        &'a self,
        transport: &'a TransportRequest,
        cancellation: lctx_native::control::RetryCancellation,
        input: ToolInput<'a>,
        vector: Option<QueryVector>,
        unavailable: bool,
        remaining_ms: u64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<EncodedJson, WireError>> + Send + 'a>,
    > {
        Box::pin(async move {
            if !Arc::ptr_eq(&transport.owner, &self.prepared) {
                return Err(WireError::Invalid(
                    "transport request belongs to another session".into(),
                ));
            }
            let budget = transport
                .charge
                .lock()
                .map_err(|_| failure(ModelError::Conflict("transport request charge")))?
                .budget()
                .cloned()
                .ok_or_else(|| failure(ModelError::Conflict("transport request budget")))?;
            let stopped = Arc::new(std::sync::atomic::AtomicBool::new(cancellation.stopped()));
            let execution = self.run(
                input.tool,
                input.raw,
                vector,
                unavailable,
                remaining_ms.min(self.limits.request_deadline_ms),
                Some((budget, stopped.clone())),
            );
            tokio::pin!(execution);
            tokio::select! {
                biased;
                () = cancellation.cancelled() => {
                    stopped.store(true, std::sync::atomic::Ordering::Release);
                    execution.await
                }
                result = &mut execution => result,
            }
        })
    }
    async fn run(
        &self,
        tool: &str,
        raw: &str,
        vector: Option<QueryVector>,
        unavailable: bool,
        remaining_ms: u64,
        context: Option<(ResourceBudget, Arc<std::sync::atomic::AtomicBool>)>,
    ) -> Result<EncodedJson, WireError> {
        lctx_native::prepared::check_read_cancellation(
            context.as_ref().map(|(_, stopped)| stopped),
        )
        .map_err(failure)?;
        let _request_lease = self.prepared.admit_request().map_err(failure)?;
        if remaining_ms == 0 {
            return Err(WireError::ResourceRefused("request deadline".into()));
        }
        let request = decode_request(tool, raw, &self.limits)?;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(remaining_ms);
        let request_admission = crate::preparation::RequestAdmission::new(
            self.queries.clone(),
            self.cpu.clone(),
            deadline,
            Duration::from_millis(self.limits.admission_wait_ms),
        )
        .await
        .map_err(failure)?;
        let preparation = crate::preparation::Preparation {
            cache: &self.prepared,
            admission: &request_admission,
        };
        let budget = match &context {
            Some((budget, _)) => budget.clone(),
            None => ResourceBudget::scoped(&self.shared, self.limits.request_bytes as usize)
                .map_err(failure)?,
        };
        let _request_charge = budget
            .reserve("native-request-wire", raw.len().saturating_mul(2))
            .map_err(failure)?;
        let reader = self.reader.clone().with_request_budget(&budget);
        let reader = match context {
            Some((_, stopped)) => reader.with_read_cancellation(stopped),
            None => reader,
        };
        let mut ranked = self.retained.request(&budget);
        tokio::time::timeout_at(deadline, async {
            self.definition
                .get_or_try_init(|| async {
                    let actual: String = reader
                        .query(
                            format!(
                                "RETURN {}();",
                                self.reader.handle().operation_definition_function()
                            ),
                            Default::default(),
                        )
                        .await
                        .map_err(failure)?;
                    if actual != crate::operation_definition().hex() {
                        return Err(WireError::Failure(PublicFailure::new(
                            FailureKind::Incompatible,
                        )));
                    }
                    Ok(())
                })
                .await?;
            if let Some(mut response) =
                ranked.resume(&request, self.handle(), vector.as_ref(), &self.limits)?
            {
                crate::delivery::finalize(&request, &mut response, &self.limits)?;
                ranked.complete(&mut response);
                let bytes = response.encode_json(
                    &budget,
                    self.limits.response_bytes(request.page().expanded) as usize,
                )?;
                if tokio::time::Instant::now() >= deadline {
                    return Err(WireError::ResourceRefused("request deadline".into()));
                }
                return Ok(bytes);
            }
            let query = match &request {
                Request::SearchOperations(r) => Some(r.query.as_str()),
                Request::SearchEvidence(r) => Some(r.query.as_str()),
                Request::SearchCapabilities(r) => Some(r.query.as_str()),
                _ => None,
            };
            if query.is_none() && vector.is_some() {
                return Err(WireError::Invalid(
                    "query vector supplied to a non-search operation".into(),
                ));
            }
            let _vector_charge = budget
                .reserve(
                    "native-query-vector",
                    vector.as_ref().map_or(0, |v| v.vector.len() * 4),
                )
                .map_err(failure)?;
            let vector_state = if let Some(v) = &vector {
                let specs = reader
                    .records::<embedding::EmbeddingSpec>(RecordSelection::Keys(vec![
                        *Id::<embedding::EmbeddingSpec>::of(&embedding::EmbeddingSpecKey {
                            service_hash: v.spec,
                        })
                        .bytes(),
                    ]))
                    .await
                    .map_err(failure)?;
                let spec = specs
                    .first()
                    .ok_or_else(|| {
                        WireError::Invalid("query embedding specification is not admitted".into())
                    })?
                    .configuration()
                    .map_err(failure)?;
                embedding::check_vector(&v.vector, spec.dimensions).map_err(WireError::Invalid)?;
                v.recipe.validate().map_err(WireError::Invalid)?;
                if spec.dimensions != 4096
                    || embedding::value::input_hash(
                        &v.recipe.text(query.expect("validated search vector")),
                    ) != v.input
                {
                    return Err(WireError::Invalid(
                        "query embedding specification or rendered input mismatch".into(),
                    ));
                }
                let policies = reader
                    .records::<embedding::projection::ProjectionDefinition>(RecordSelection::Keys(
                        vec![*v.projection.bytes()],
                    ))
                    .await
                    .map_err(failure)?;
                let policy = policies
                    .first()
                    .ok_or_else(|| WireError::Invalid("query projection is not admitted".into()))?;
                policy.validate().map_err(failure)?;
                if policy.dimensions != 1024 || v.recipe.max_tokens > 8192 {
                    return Err(WireError::Invalid(
                        "unsupported query projection/admission policy".into(),
                    ));
                }
                VectorChannel::Available {
                    spec: v.spec,
                    query_vector: embedding::value::value_digest(&v.vector),
                    query_recipe: v.recipe.identity(),
                    projection: v.projection,
                }
            } else if unavailable && query.is_some() {
                VectorChannel::Degraded {
                    reason: Name::new("embedding_service_unavailable")?,
                }
            } else {
                VectorChannel::Disabled {}
            };
            let channels = ChannelState {
                lexical: query.is_some(),
                vector: vector_state,
            };
            crate::pagination::validate(&request, self.handle(), &channels)?;
            let mut response = crate::operations::dispatch(
                crate::preparation::OperationRead::new(&reader, &preparation).map_err(failure)?,
                &request,
                &channels,
                vector.as_ref(),
                &self.limits,
                &mut ranked,
                &budget,
            )
            .await
            .map_err(failure)?;
            crate::delivery::finalize(&request, &mut response, &self.limits)?;
            ranked.complete(&mut response);
            let bytes = response.encode_json(
                &budget,
                self.limits.response_bytes(request.page().expanded) as usize,
            )?;
            ranked.attach(&bytes)?;
            if tokio::time::Instant::now() >= deadline {
                return Err(WireError::ResourceRefused("request deadline".into()));
            }
            ranked.publish()?;
            Ok(bytes)
        })
        .await
        .map_err(|_| WireError::ResourceRefused("request deadline".into()))?
    }
}
fn failure(error: ModelError) -> WireError {
    let kind = match error.primary() {
        Some(ModelError::Serving(kind)) => *kind,
        Some(ModelError::Resource { .. } | ModelError::Limit { .. }) => {
            FailureKind::ResourceRefused
        }
        Some(ModelError::Infrastructure {
            class: Infrastructure::Contract,
            ..
        }) => FailureKind::Incompatible,
        Some(
            ModelError::Schema(_)
            | ModelError::Identity(_)
            | ModelError::Conflict(_)
            | ModelError::Invalid(_)
            | ModelError::Frontier(_),
        ) => FailureKind::Corrupt,
        _ => FailureKind::Unavailable,
    };
    WireError::Failure(PublicFailure::new(kind))
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    use lctx_model::domain::completion::{Completion, complete};
    #[test]
    fn completion_failure_preserves_primary_public_category() {
        for (primary, expected) in [
            (
                ModelError::Limit {
                    owner: "test",
                    limit: "rows",
                    observed: 2,
                    bound: 1,
                },
                FailureKind::ResourceRefused,
            ),
            (
                ModelError::infrastructure(Infrastructure::Contract, "private contract detail"),
                FailureKind::Incompatible,
            ),
            (
                ModelError::Schema("private corrupt detail"),
                FailureKind::Corrupt,
            ),
        ] {
            let mut completion = Completion::default();
            completion.cleanup(
                "secret database",
                Err(ModelError::Codec("secret cleanup detail".into())),
            );
            let error = complete::<()>(Err(primary), completion).unwrap_err();
            let WireError::Failure(public) = failure(error) else {
                panic!()
            };
            assert_eq!(public.kind, expected);
        }
        let mut completion = Completion::default();
        completion.step("cleanup", Err(ModelError::Schema("secret detail")));
        let error = complete(Ok(()), completion).unwrap_err();
        let WireError::Failure(public) = failure(error) else {
            panic!()
        };
        assert_eq!(public.kind, FailureKind::Unavailable);
    }
}
