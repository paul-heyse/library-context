//! Issuance is fenced independently of intent arrival. A retry carries this exact request.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuanceEra {
    pub generation: ContentHash,
    pub era: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRequest {
    pub issuance: IssuanceEra,
    pub operation: ContentHash,
    pub request: ContentHash,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectDisposition {
    Committed,
    FencedUncommitted,
    Pending,
    CompactedTerminal,
}
#[derive(Debug)]
pub struct NativeRequestError {
    pub request: NativeRequest,
    pub source: ModelError,
}
#[derive(Debug)]
pub struct NativeAttemptError {
    pub attempt: ContentHash,
    pub issuance: IssuanceEra,
    pub source: ModelError,
}
impl std::fmt::Display for NativeAttemptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "native attempt {} era {} generation {}: {}",
            self.attempt.hex(),
            self.issuance.era,
            self.issuance.generation.hex(),
            self.source
        )
    }
}
impl std::error::Error for NativeAttemptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
#[derive(Debug)]
pub struct NativeLifecycleError {
    pub kind: &'static str,
    pub identity: ContentHash,
    pub issuance: IssuanceEra,
    pub source: ModelError,
}
impl std::fmt::Display for NativeLifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "native {} {} era {} generation {}: {}",
            self.kind,
            self.identity.hex(),
            self.issuance.era,
            self.issuance.generation.hex(),
            self.source
        )
    }
}
impl std::error::Error for NativeLifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
pub(super) fn lifecycle_error(
    kind: &'static str,
    identity: ContentHash,
    issuance: IssuanceEra,
    source: ModelError,
) -> ModelError {
    ModelError::Cause(Box::new(NativeLifecycleError {
        kind,
        identity,
        issuance,
        source,
    }))
}
impl std::fmt::Display for NativeRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "native request {} era {} generation {} digest {}: {}",
            self.request.operation.hex(),
            self.request.issuance.era,
            self.request.issuance.generation.hex(),
            self.request.request.hex(),
            self.source
        )
    }
}
impl std::error::Error for NativeRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
impl NativeRequest {
    fn validate(self) -> Result<(), ModelError> {
        if self.issuance.era <= 0 {
            return Err(self.error(ModelError::Conflict(
                "native request requires a positive issuance era",
            )));
        }
        Ok(())
    }
    pub async fn issue(client: &Surreal<Client>, request: ContentHash) -> Result<Self, ModelError> {
        let issuance = IssuanceEra::capture(client).await?;
        // Never allocate the operation before capturing the authorization era.
        Ok(Self {
            issuance,
            operation: fresh_identity("effect")?,
            request,
        })
    }
    pub(super) fn bind(&self, bindings: &mut Variables) {
        bindings.insert(
            "__effect",
            RecordId::new("native_effect", self.operation.hex()),
        );
        bindings.insert("__request", self.request.hex());
        bindings.insert("__generation", self.issuance.generation.hex());
        bindings.insert("__era", self.issuance.era);
        coordination::bind_stripe(bindings, self.operation);
    }
    pub(super) fn error(self, source: ModelError) -> ModelError {
        ModelError::Cause(Box::new(NativeRequestError {
            request: self,
            source,
        }))
    }
}
impl IssuanceEra {
    pub async fn capture(client: &Surreal<Client>) -> Result<Self, ModelError> {
        let mut response = client
            .query("SELECT generation,era,closed_through FROM native_installation:current")
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        // A missing statement is not an observed empty installation. The SDK permits
        // empty envelopes for skipped control flow, but this fixed SELECT must return one.
        if response.num_statements() != 1 {
            tracing::warn!(target: "lctx_phase", observation="native_issuance_statement_inventory",
                expected=1, observed=response.num_statements(), "native issuance response inventory refused");
            return Err(ModelError::Schema("native issuance response inventory"));
        }
        let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("native issuance installation"));
        };
        let generation = hash_field(row, "generation")?;
        let era = int_field(row, "era")?;
        if era <= int_field(row, "closed_through")? || era <= 0 {
            return Err(ModelError::Conflict("native issuance era closed"));
        }
        Ok(Self { generation, era })
    }
}
pub(super) fn int_field(row: &Object, field: &str) -> Result<i64, ModelError> {
    match row.get(field) {
        Some(Value::Number(surrealdb::types::Number::Int(value))) => Ok(*value),
        _ => Err(ModelError::Schema("native control integer field")),
    }
}
pub(super) fn hash_field(row: &Object, field: &str) -> Result<ContentHash, ModelError> {
    match row.get(field) {
        Some(Value::String(value)) => Ok(ContentHash(
            hex::decode(value)
                .map_err(ModelError::codec)?
                .try_into()
                .map_err(|_| ModelError::Schema("native control hash width"))?,
        )),
        _ => Err(ModelError::Schema("native control hash field")),
    }
}
// Each effect writes one fixed stripe; only explicit admission/cut owners write all64.
use super::coordination::ERA_GUARD;
pub async fn issue_attempt(
    client: &Surreal<Client>,
    generation: ContentHash,
) -> Result<ContentHash, ModelError> {
    let era = IssuanceEra::capture(client).await?;
    if era.generation != generation {
        return Err(ModelError::Conflict("native attempt generation"));
    }
    let attempt = fresh_identity("compiler-attempt")?;
    begin_attempt_in(client, attempt, era).await?;
    Ok(attempt)
}
/// Exact lifecycle witness held from allocation commit until durable source terminal knowledge.
pub fn source_attempt_reference(attempt: ContentHash) -> RecordId {
    let mut bytes = b"native-attempt-source-reference/v1".to_vec();
    bytes.extend_from_slice(&attempt.0);
    RecordId::new("native_outcome_ref", ContentHash::of(&bytes).hex())
}
pub fn request_outcome_reference(
    request: NativeRequest,
    policy: RecoveryReferencePolicy,
) -> Result<RecordId, ModelError> {
    Ok(RecordId::new(
        "native_outcome_ref",
        ContentHash::of(
            &serde_json::to_vec(&(request.operation, request.operation.hex(), policy.kind()))
                .map_err(ModelError::codec)?,
        )
        .hex(),
    ))
}
pub fn allocation_outcome_reference(request: NativeRequest) -> Result<RecordId, ModelError> {
    request_outcome_reference(request, RecoveryReferencePolicy::RetainCaller)
}
pub fn source_pin_reference(pin: ContentHash) -> RecordId {
    let mut bytes = b"native-pin-source-reference/v1".to_vec();
    bytes.extend_from_slice(&pin.0);
    RecordId::new("native_outcome_ref", ContentHash::of(&bytes).hex())
}
/// One exact pin allocation followed by its bounded view holds. The factory
/// registers the identity/request before either effect is submitted.
pub struct PreparedPinAllocation {
    pub pin: ContentHash,
    pub request: NativeRequest,
    payload: PreparedNativePayload<'static>,
    objects: Vec<RecordId>,
}
impl PreparedPinAllocation {
    pub async fn issue(self, client: &Surreal<Client>) -> Result<(), ModelError> {
        let result = async {
            execute_recorded(
                client,
                self.request,
                EffectOwner::Installation,
                self.payload,
                true,
            )
            .await?;
            super::hold(
                client,
                None,
                RecordId::new("native_pin", self.pin.hex()),
                self.objects,
            )
            .await
        }
        .await;
        result.map_err(|source| {
            lifecycle_error("reader pin", self.pin, self.request.issuance, source)
        })
    }
}
pub fn prepare_pin_allocation(
    pin: ContentHash,
    issuance: IssuanceEra,
    views: &[ContentHash],
) -> Result<PreparedPinAllocation, ModelError> {
    let owner = RecordId::new("native_pin", pin.hex());
    let objects = views
        .iter()
        .map(|view| RecordId::new("compiler_view", view.hex()))
        .collect::<Vec<_>>();
    let mut bindings = Variables::new();
    bindings.insert("pin", owner.clone());
    bindings.insert("issuance", issuance.era);
    bindings.insert("views", objects.clone());
    bindings.insert("guard", super::guard_id(&Value::RecordId(owner))?);
    bindings.insert("source_reference", source_pin_reference(pin));
    bindings.insert("source_consumer", pin.hex());
    let payload = PreparedNativePayload::new(
        "IF !$__admission.admission_open OR $__era!=$issuance { THROW 'native reader admission closed'; }; fn::native_touch($pin); LET $prior=SELECT * FROM ONLY $pin FOR UPDATE; IF $prior!=NONE { THROW 'native pin identity collision'; }; CREATE $guard SET revision=1,retired=false,phase='active',incarnation=1,tombstone_revision=0 RETURN NONE; CREATE $pin SET views=$views,released=false,epoch=$__epoch,generation=$__generation,era=$__era RETURN NONE; fn::native_touch($source_reference); IF (SELECT VALUE id FROM ONLY $source_reference)!=NONE { THROW 'native pin source retention collision'; }; CREATE $source_reference SET object=$pin,owner=$source_consumer,kind='caller' RETURN NONE;",
        bindings,
    )?;
    let request = NativeRequest {
        issuance,
        operation: fresh_identity("pin-allocation")?,
        request: payload.digest,
    };
    request.validate()?;
    Ok(PreparedPinAllocation {
        pin,
        request,
        payload,
        objects,
    })
}
/// Producer-owned prepared allocation; durable recovery stores only request and source identities.
pub struct PreparedAttemptAllocation {
    pub attempt: ContentHash,
    pub request: NativeRequest,
    payload: PreparedNativePayload<'static>,
}
impl PreparedAttemptAllocation {
    pub async fn issue(self, client: &Surreal<Client>) -> Result<(), ModelError> {
        execute_recorded(
            client,
            self.request,
            EffectOwner::Installation,
            self.payload,
            true,
        )
        .await
        .map_err(|source| {
            ModelError::Cause(Box::new(NativeAttemptError {
                attempt: self.attempt,
                issuance: self.request.issuance,
                source,
            }))
        })
    }
}
pub fn prepare_attempt_allocation(
    attempt: ContentHash,
    era: IssuanceEra,
) -> Result<PreparedAttemptAllocation, ModelError> {
    let (query, mut bindings) = attempt_allocation_payload(attempt, era)?;
    bindings.insert("source_reference", source_attempt_reference(attempt));
    let payload = PreparedNativePayload::new(query, bindings)?;
    let request = NativeRequest {
        issuance: era,
        operation: fresh_identity("attempt-allocation")?,
        request: payload.digest,
    };
    request.validate()?;
    Ok(PreparedAttemptAllocation {
        attempt,
        request,
        payload,
    })
}
fn attempt_allocation_payload(
    attempt: ContentHash,
    era: IssuanceEra,
) -> Result<(&'static str, Variables), ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("attempt", RecordId::new("native_attempt", attempt.hex()));
    bindings.insert(
        "attempt_guard",
        guard_id(&Value::RecordId(RecordId::new(
            "native_attempt",
            attempt.hex(),
        )))?,
    );
    bindings.insert("attempt_generation", era.generation.hex());
    bindings.insert("attempt_era", era.era);
    bindings.insert("source_reference", Value::None);
    bindings.insert("source_consumer", attempt.hex());
    Ok((
        "IF $__generation!=$attempt_generation OR $__era!=$attempt_era { THROW 'native attempt original issuance changed'; }; LET $existing=SELECT * FROM ONLY $attempt FOR UPDATE; IF $existing!=NONE { IF $existing.generation!=$__generation OR $existing.era!=$__era OR $existing.epoch<=0 { THROW 'native attempt identity collision'; }; fn::native_attempt_validate($attempt,$__generation,$__era,$existing.epoch); } ELSE { IF !$__admission.admission_open { THROW 'native installation admission closed'; }; IF (SELECT VALUE id FROM ONLY $attempt_guard)!=NONE { THROW 'native attempt guard identity collision'; }; LET $predicate=type::record('native_predicate',[$attempt]); IF (SELECT VALUE id FROM ONLY $predicate)!=NONE { THROW 'native attempt predicate identity collision'; }; CREATE $predicate SET revision=1 RETURN NONE; CREATE $attempt_guard SET phase='active',incarnation=1,tombstone_revision=0,revision=1,retired=false,retiring_item=NONE RETURN NONE; CREATE $attempt SET generation=$__generation,era=$__era,state='open',revision=0,admitted=false,epoch=1 RETURN NONE; fn::native_attempt_initialize($attempt,$__generation,$__era,1); }; IF $source_reference!=NONE { fn::native_touch($source_reference); LET $prior_source=SELECT * FROM ONLY $source_reference FOR UPDATE; IF $prior_source!=NONE AND ($prior_source.object!=$attempt OR $prior_source.owner!=$source_consumer OR $prior_source.kind!='caller') { THROW 'native source retention collision'; }; UPSERT $source_reference SET object=$attempt,owner=$source_consumer,kind='caller' RETURN NONE; };",
        bindings,
    ))
}
pub async fn begin_attempt_in(
    client: &Surreal<Client>,
    attempt: ContentHash,
    era: IssuanceEra,
) -> Result<(), ModelError> {
    let (query, bindings) = attempt_allocation_payload(attempt, era)?;
    effect_for_owner(client, EffectOwner::Installation, query, bindings)
        .await
        .map_err(|source| {
            ModelError::Cause(Box::new(NativeAttemptError {
                attempt,
                issuance: era,
                source,
            }))
        })
}

#[derive(Clone, Copy)]
pub(super) struct TerminalAttempt {
    pub identity: ContentHash,
    pub epoch: i64,
}
pub(super) enum EffectOwner {
    Installation,
    OpenAttempt(ContentHash),
    FrozenAttempt(ContentHash),
    Pin(RecordId),
    TerminalAttempt(TerminalAttempt),
    Retirement(RecordId),
    RetirementSetup(RecordId),
    Cleanup(RecordId),
}
impl EffectOwner {
    fn record(&self) -> Option<RecordId> {
        match self {
            Self::OpenAttempt(id) | Self::FrozenAttempt(id) => {
                Some(RecordId::new("native_attempt", id.hex()))
            }
            Self::TerminalAttempt(owner) => {
                Some(RecordId::new("native_attempt", owner.identity.hex()))
            }
            Self::Pin(id)
            | Self::Retirement(id)
            | Self::RetirementSetup(id)
            | Self::Cleanup(id) => Some(id.clone()),
            Self::Installation => None,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Self::Installation => "installation",
            Self::OpenAttempt(_) => "attempt",
            Self::FrozenAttempt(_) => "frozen_attempt",
            Self::Pin(_) => "pin",
            Self::TerminalAttempt(_) => "terminal_attempt",
            Self::Retirement(_) => "retirement",
            Self::RetirementSetup(_) => "retirement_setup",
            Self::Cleanup(_) => "cleanup",
        }
    }
    fn predicate(&self) -> &'static str {
        match self {
            Self::OpenAttempt(_) => "$__owner.state='open'",
            Self::FrozenAttempt(_) => "$__owner.state='frozen'",
            Self::Pin(_) => "!$__owner.released",
            Self::TerminalAttempt(_) => {
                "$__owner.state IN ['closing','closed','abandoned','maintenance_fenced'] AND $__owner.epoch=$__expected_epoch"
            }
            Self::Retirement(_) | Self::Cleanup(_) => {
                "$__owner.state='open' AND $__owner.successor=NONE"
            }
            Self::RetirementSetup(_) => {
                "$__owner.state IN ['registering','recovering','open'] AND $__owner.successor=NONE"
            }
            Self::Installation => "true",
        }
    }
}
pub async fn effect(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    sql: &str,
    bindings: Variables,
) -> Result<(), ModelError> {
    effect_for_owner(
        client,
        attempt
            .map(EffectOwner::OpenAttempt)
            .unwrap_or(EffectOwner::Installation),
        sql,
        bindings,
    )
    .await
}
pub async fn effect_frozen_attempt(
    client: &Surreal<Client>,
    attempt: ContentHash,
    sql: &str,
    bindings: Variables,
) -> Result<(), ModelError> {
    effect_for_owner(client, EffectOwner::FrozenAttempt(attempt), sql, bindings).await
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OriginalCompletionOutcome {
    Applied,
    PremiseChanged,
}
fn original_completion_outcome(row: &Object) -> Result<OriginalCompletionOutcome, ModelError> {
    match row.get("original_completion_outcome") {
        Some(Value::String(code)) if code == "applied" => Ok(OriginalCompletionOutcome::Applied),
        Some(Value::String(code)) if code == "premise_changed" => {
            Ok(OriginalCompletionOutcome::PremiseChanged)
        }
        _ => Err(ModelError::Conflict(
            "original completion exact typed outcome missing",
        )),
    }
}
/// The result belongs to the original committed request, not a subsequent blob
/// observation. Keep its exact outcome reference until acquisition is confirmed.
pub(crate) async fn original_completion_effect(
    client: &Surreal<Client>,
    attempt: Option<ContentHash>,
    sql: &str,
    mut bindings: Variables,
) -> Result<OriginalCompletionOutcome, ModelError> {
    coordination::prepare_expectations(client, &mut bindings).await?;
    let payload = PreparedNativePayload::new(sql, bindings)?;
    let owner = attempt
        .map(EffectOwner::OpenAttempt)
        .unwrap_or(EffectOwner::Installation);
    let issuance = if let Some(id) = owner.record() {
        let mut response = client
            .query("SELECT generation,era FROM $owner")
            .bind(("owner", id))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("native effect original owner missing"));
        };
        IssuanceEra {
            generation: hash_field(row, "generation")?,
            era: int_field(row, "era")?,
        }
    } else {
        IssuanceEra::capture(client).await?
    };
    let request = NativeRequest {
        issuance,
        operation: fresh_identity("original-completion")?,
        request: payload.digest,
    };
    execute_recorded(client, request, owner, payload, true).await?;
    let (disposition, row) = observe_request(client, request)
        .await
        .map_err(|error| request.error(error))?;
    if disposition != EffectDisposition::Committed {
        return Err(request.error(ModelError::Conflict(
            "original completion request not committed",
        )));
    }
    let result = original_completion_outcome(&row.ok_or_else(|| {
        request.error(ModelError::Conflict("original completion receipt missing"))
    })?)
    .map_err(|error| request.error(error))?;
    if let Err(error) =
        super::history::release_outcome(client, request, &request.operation.hex(), "caller").await
    {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.committed(
            "original completion outcome acquired",
            request.operation.hex(),
        );
        return lctx_model::domain::completion::complete(Err(request.error(error)), completion);
    }
    Ok(result)
}
/// Only this constructor establishes correspondence between an internal payload and
/// its request digest. Public retry input earns the same check independently.
struct PreparedNativePayload<'a> {
    sql: &'a str,
    bindings: Variables,
    digest: ContentHash,
}
fn intent_bindings(payload: &Variables) -> Result<Variables, ModelError> {
    let mut controls = Variables::new();
    for key in [
        "__effect",
        "__request",
        "__generation",
        "__era",
        "__outcome_ref",
        "__reference_owner",
        "__reference_kind",
        "__attempt",
        "__owner_id",
        "__owner_kind",
        "__attempt_slot",
        "__stripe",
    ] {
        controls.insert(
            key,
            payload
                .get(key)
                .ok_or(ModelError::Schema("native intent control"))?
                .clone(),
        );
    }
    if let Some(epoch) = payload.get("__expected_epoch") {
        controls.insert("__expected_epoch", epoch.clone());
    }
    Ok(controls)
}
impl<'a> PreparedNativePayload<'a> {
    fn new(sql: &'a str, bindings: Variables) -> Result<Self, ModelError> {
        let digest =
            ContentHash::of(&serde_json::to_vec(&(sql, &bindings)).map_err(ModelError::codec)?);
        Ok(Self {
            sql,
            bindings,
            digest,
        })
    }
    fn check_request(&self, request: NativeRequest) -> Result<(), ModelError> {
        if self.digest != request.request {
            return Err(request.error(ModelError::Conflict("native retry payload changed")));
        }
        Ok(())
    }
}
pub(super) async fn effect_for_owner(
    client: &Surreal<Client>,
    owner: EffectOwner,
    sql: &str,
    bindings: Variables,
) -> Result<(), ModelError> {
    let mut bindings = bindings;
    coordination::prepare_expectations(client, &mut bindings).await?;
    let payload = PreparedNativePayload::new(sql, bindings)?;
    let issuance = if let Some(id) = owner.record() {
        let mut response = client
            .query("SELECT generation,era FROM $owner")
            .bind(("owner", id))
            .await
            .map_err(ModelError::codec)?
            .check()
            .map_err(ModelError::codec)?;
        let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("native effect original owner missing"));
        };
        IssuanceEra {
            generation: hash_field(row, "generation")?,
            era: int_field(row, "era")?,
        }
    } else {
        IssuanceEra::capture(client).await?
    };
    let request = NativeRequest {
        issuance,
        operation: fresh_identity("effect")?,
        request: payload.digest,
    };
    execute_recorded(client, request, owner, payload, false).await
}
pub(super) struct PreparedOwnerEffect {
    request: NativeRequest,
    plan: OwnerEffectPlan,
}
impl PreparedOwnerEffect {
    pub(super) fn bytes(&self) -> Result<usize, ModelError> {
        Ok(
            super::immutable_window::plan_bytes(&self.plan.query, &self.plan.bindings)?.max(
                super::immutable_window::plan_bytes(&self.plan.intent, &self.plan.intent_bindings)?,
            ),
        )
    }
}
pub(super) async fn owner_issuance(
    client: &Surreal<Client>,
    owner: &EffectOwner,
) -> Result<IssuanceEra, ModelError> {
    let id = owner.record().ok_or(ModelError::Conflict(
        "prepared witness requires an exact owner",
    ))?;
    let mut response = client
        .query("SELECT generation,era FROM $owner")
        .bind(("owner", id))
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict("native effect original owner missing"));
    };
    Ok(IssuanceEra {
        generation: hash_field(row, "generation")?,
        era: int_field(row, "era")?,
    })
}
pub(super) async fn prepare_owner_effect(
    client: &Surreal<Client>,
    owner: &EffectOwner,
    issuance: IssuanceEra,
    sql: &str,
    mut bindings: Variables,
) -> Result<PreparedOwnerEffect, ModelError> {
    coordination::prepare_expectations(client, &mut bindings).await?;
    let payload = PreparedNativePayload::new(sql, bindings)?;
    let request = NativeRequest {
        issuance,
        operation: fresh_identity("effect")?,
        request: payload.digest,
    };
    request.validate()?;
    let plan = owner_effect_plan(request, owner, payload.sql, payload.bindings, false)?;
    Ok(PreparedOwnerEffect { request, plan })
}
pub(super) async fn execute_prepared_owner_effect(
    client: &Surreal<Client>,
    prepared: PreparedOwnerEffect,
) -> Result<(), ModelError> {
    let PreparedOwnerEffect { request, plan } = prepared;
    // Sizing did not accept a receipt. Register only the exact already prepared request.
    super::receipts::accepted(request).map_err(|error| request.error(error))?;
    let result = match execute_owner_plan(client, request, plan) {
        Ok(run) => run.await,
        Err(error) => Err(error),
    };
    let disposition = if result.is_ok() {
        Some(EffectDisposition::Committed)
    } else if super::receipts::active() {
        request_disposition(client, request).await.ok()
    } else {
        None
    };
    if let Some(disposition) = disposition
        && let Err(error) = super::receipts::observed(request, disposition)
    {
        let mut terminal = lctx_model::domain::completion::Completion::default();
        terminal.step("native original-request receipt", Err(error));
        return lctx_model::domain::completion::complete(result, terminal);
    }
    result
}
/// Reissue only the original operation, bytes and authority. This cannot refresh a closed era.
pub async fn execute_request(
    client: &Surreal<Client>,
    request: NativeRequest,
    attempt: Option<ContentHash>,
    sql: &str,
    bindings: Variables,
) -> Result<(), ModelError> {
    let payload = PreparedNativePayload::new(sql, bindings)?;
    payload.check_request(request)?;
    execute_recorded(
        client,
        request,
        attempt
            .map(EffectOwner::OpenAttempt)
            .unwrap_or(EffectOwner::Installation),
        payload,
        true,
    )
    .await
}
async fn execute_recorded(
    client: &Surreal<Client>,
    request: NativeRequest,
    owner: EffectOwner,
    payload: PreparedNativePayload<'_>,
    retain: bool,
) -> Result<(), ModelError> {
    request.validate()?;
    // A recorded public request can outlive a format/era cut. Observation of its exact
    // retained committed receipt replays only the outcome, never old payload authority.
    // Internal freshly issued effects keep their ordinary single intent/payload path.
    let replay = if retain {
        Some(observe_request(client, request).await)
    } else {
        None
    };
    // Historical guarded bindings are valid evidence of their exact original digest,
    // but never authorize a new payload without captured incarnation expectations.
    // Refuse before publishing an accepted-request receipt for an unissued payload.
    if matches!(&replay, None | Some(Ok((EffectDisposition::Pending, _))))
        && ["guards", "hold_guards", "__write_guards"]
            .iter()
            .any(|key| payload.bindings.get(key).is_some())
        && payload.bindings.get("__object_expectations").is_none()
    {
        return Err(request.error(ModelError::Conflict(
            "native retry expected identities missing",
        )));
    }
    super::receipts::accepted(request).map_err(|error| request.error(error))?;
    let result = match replay {
        Some(Ok((EffectDisposition::Committed, Some(row)))) => {
            validate_replay_owner(request, &owner, &row)
        }
        Some(Ok((
            EffectDisposition::FencedUncommitted | EffectDisposition::CompactedTerminal,
            _,
        ))) => Err(request.error(ModelError::Conflict(
            "native original request terminal; payload fenced",
        ))),
        Some(Err(error)) => Err(request.error(error)),
        _ => execute_owner(client, request, owner, payload, retain).await,
    };
    let disposition = if result.is_ok() {
        Some(EffectDisposition::Committed)
    } else if super::receipts::active() {
        request_disposition(client, request).await.ok()
    } else {
        None
    };
    if let Some(disposition) = disposition
        && let Err(error) = super::receipts::observed(request, disposition)
    {
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("native original-request receipt", Err(error));
        return lctx_model::domain::completion::complete(result, completion);
    }
    result
}
pub(super) struct OwnerEffectPlan {
    pub(super) query: String,
    pub(super) intent: String,
    pub(super) intent_bindings: Variables,
    pub(super) bindings: Variables,
    reference: RecordId,
    retain: bool,
    recovery_owner: RecoveryOwner,
}
/// Pure shared protocol lowering: sizing and execution use precisely the same wrappers.
pub(super) fn owner_effect_plan(
    request: NativeRequest,
    owner: &EffectOwner,
    sql: &str,
    bindings: Variables,
    retain: bool,
) -> Result<OwnerEffectPlan, ModelError> {
    let mut bindings = bindings;
    request.bind(&mut bindings);
    let reference_owner = request.operation.hex();
    let reference_kind = if retain { "caller" } else { "recovery" };
    let reference = request_outcome_reference(
        request,
        if retain {
            RecoveryReferencePolicy::RetainCaller
        } else {
            RecoveryReferencePolicy::ReleaseRecovery
        },
    )?;
    bindings.insert("__outcome_ref", reference.clone());
    bindings.insert("__reference_owner", reference_owner);
    bindings.insert("__reference_kind", reference_kind);
    let installation_owner = owner.record().is_none();
    let owner_id = Some(
        owner
            .record()
            .unwrap_or_else(|| coordination::operation_owner(request)),
    );
    let attempt = match &owner {
        EffectOwner::OpenAttempt(id) | EffectOwner::FrozenAttempt(id) => Some(*id),
        EffectOwner::TerminalAttempt(owner) => Some(owner.identity),
        _ => None,
    };
    bindings.insert(
        "__attempt",
        attempt
            .map(|id| Value::RecordId(RecordId::new("native_attempt", id.hex())))
            .unwrap_or(Value::None),
    );
    bindings.insert(
        "__owner_id",
        owner_id.clone().map(Value::RecordId).unwrap_or(Value::None),
    );
    bindings.insert("__owner_kind", owner.kind());
    bindings.insert(
        "__attempt_slot",
        attempt
            .map(|_| {
                surrealdb::types::SurrealValue::into_value(coordination::attempt_slot(
                    request.operation,
                ))
            })
            .unwrap_or(Value::None),
    );
    if let EffectOwner::TerminalAttempt(owner) = &owner {
        bindings.insert("__expected_epoch", owner.epoch);
    }
    let predicate = owner.predicate();
    let authorization = if attempt.is_some() {
        format!(
            "LET $__owner=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__owner=NONE OR $__owner.generation!=$__generation OR $__owner.era!=$__era OR !({predicate}) {{ THROW 'native owner fenced'; }}; LET $__epoch=$__owner.epoch;"
        )
    } else if !installation_owner {
        format!(
            "fn::native_touch($__owner_id); LET $__owner=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__owner=NONE OR $__owner.generation!=$__generation OR $__owner.era!=$__era OR !({predicate}) {{ THROW 'native owner fenced'; }}; fn::native_incidence($__owner); LET $__epoch=$__owner.epoch;"
        )
    } else {
        "fn::native_touch($__owner_id); LET $__local=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__local=NONE { CREATE $__owner_id SET generation=$__generation,era=$__era,epoch=1,operation=$__reference_owner,state='open' RETURN NONE; } ELSE IF $__local.generation!=$__generation OR $__local.era!=$__era OR $__local.operation!=$__reference_owner OR $__local.epoch!=1 { THROW 'native installation operation owner collision'; }; LET $__epoch=1;".into()
    };
    let context = "{attempt:$__attempt,effect:$__effect,generation:$__generation,era:$__era,epoch:$__epoch,request:$__request,slot:$__attempt_slot}";
    let participation = if attempt.is_some() {
        format!(
            "LET $__attempt_context={context}; fn::native_attempt_participate($__attempt_context);"
        )
    } else {
        "LET $__attempt_context=NONE;".to_owned()
    };
    let intent = format!(
        "BEGIN; {ERA_GUARD} LET $__prior=SELECT * FROM ONLY $__effect FOR UPDATE; IF $__prior=NONE {{ {authorization} IF $__epoch=NONE OR $__epoch<=0 {{ THROW 'native authorization epoch missing'; }}; fn::native_touch($__effect); CREATE $__effect SET attempt=$__attempt,owner=$__owner_id,owner_kind=$__owner_kind,generation=$__generation,era=$__era,request=$__request,committed=false,resolved=false,revision=0,epoch=$__epoch,attempt_slot=$__attempt_slot RETURN NONE; {participation} CREATE $__outcome_ref SET effect=$__effect,owner=$__reference_owner,kind=$__reference_kind RETURN NONE; }} ELSE {{ IF $__prior.request!=$__request OR $__prior.era!=$__era OR $__prior.generation!=$__generation OR $__prior.owner!=$__owner_id OR $__prior.owner_kind!=$__owner_kind OR $__prior.attempt!=$__attempt OR $__prior.attempt_slot!=$__attempt_slot {{ THROW 'native operation identity collision'; }}; }}; COMMIT;"
    );
    // Intent persists authorization and outcome identity, never the payload's row
    // bindings. The full payload remains owned for its one later execution.
    let intent_bindings = intent_bindings(&bindings)?;
    let fence = if attempt.is_some() {
        format!(
            "LET $__owner=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__owner=NONE OR $__owner.epoch!=$__epoch OR $__owner.era!=$__era OR $__owner.generation!=$__generation OR !({predicate}) {{ THROW 'native owner fenced'; }};"
        )
    } else if !installation_owner {
        format!(
            "fn::native_touch($__owner_id); LET $__owner=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__owner=NONE OR $__owner.epoch!=$__epoch OR $__owner.era!=$__era OR $__owner.generation!=$__generation OR !({predicate}) {{ THROW 'native owner fenced'; }}; fn::native_incidence($__owner);"
        )
    } else {
        "fn::native_touch($__owner_id); LET $__local=SELECT * FROM ONLY $__owner_id FOR UPDATE; IF $__local=NONE OR $__local.state!='open' OR $__local.epoch!=$__epoch OR $__local.generation!=$__generation OR $__local.era!=$__era { THROW 'native local operation owner fenced'; };".into()
    };
    // A payload RETURN belongs to that payload, never the outer transaction.
    // The closure catches RETURN while errors still abort payload and receipt together.
    let query = format!(
        "BEGIN; {ERA_GUARD} LET $__operation=SELECT * FROM ONLY $__effect FOR UPDATE; IF $__operation=NONE OR $__operation.request!=$__request OR $__operation.era!=$__era OR $__operation.generation!=$__generation {{ THROW 'native operation fenced'; }}; IF $__operation.resolved {{ IF !$__operation.committed {{ THROW 'native operation fenced uncommitted'; }}; }} ELSE {{ IF $__operation.epoch<=0 {{ THROW 'native operation epoch missing'; }}; LET $__epoch=$__operation.epoch; fn::native_touch($__effect); {fence} {participation} {} LET $__payload=|| {{ {sql}; }}; LET $__payload_result=$__payload(); UPDATE $__effect SET committed=true,resolved=true,revision+=1 RETURN NONE; IF $__operation.owner_kind='installation' {{ UPDATE $__owner_id SET state='done' RETURN NONE; }}; }}; COMMIT;",
        coordination::EXPECTED_OBJECT_GUARD
    );
    coordination::preflight(&query, &bindings)?;
    coordination::preflight(&intent, &intent_bindings)?;
    Ok(OwnerEffectPlan {
        query,
        intent,
        intent_bindings,
        bindings,
        reference,
        retain,
        recovery_owner: RecoveryOwner::from_effect(owner),
    })
}
async fn execute_owner(
    client: &Surreal<Client>,
    request: NativeRequest,
    owner: EffectOwner,
    payload: PreparedNativePayload<'_>,
    retain: bool,
) -> Result<(), ModelError> {
    let plan = owner_effect_plan(request, &owner, payload.sql, payload.bindings, retain)?;
    execute_owner_plan(client, request, plan)?.await
}
fn execute_owner_plan<'a>(
    client: &'a Surreal<Client>,
    request: NativeRequest,
    plan: OwnerEffectPlan,
) -> Result<crate::continuation::Continuation<'a, Result<(), ModelError>>, ModelError> {
    use lctx_model::domain::completion::{Completion, LocalState, RemoteState, complete};
    let scope = capture_recovery_scope().ok_or_else(|| {
        request.error(ModelError::Conflict(
            "native effect requires its owning recovery scope",
        ))
    })?;
    if !std::ptr::eq(scope.client().as_ref(), client) {
        return Err(request.error(ModelError::Conflict(
            "native recovery session correspondence",
        )));
    }
    let budget = scope.budget().clone();
    let body = async move {
        let OwnerEffectPlan {
            query,
            intent,
            intent_bindings,
            bindings,
            reference,
            retain,
            recovery_owner,
        } = plan;
        let pending = scope
            .prepare(
                request,
                recovery_owner,
                reference,
                if retain {
                    RecoveryReferencePolicy::RetainCaller
                } else {
                    RecoveryReferencePolicy::ReleaseRecovery
                },
            )
            .map_err(|error| request.error(error))?;
        let intent_result =
            super::retry::transaction_named(client, &intent, intent_bindings, false, "intent")
                .await;
        let (issued_payload, result) = match intent_result {
            Ok(()) => (
                true,
                super::retry::transaction_named(client, &query, bindings, false, "payload").await,
            ),
            Err(error) => (false, Err(error)),
        };
        let primary = match result {
            Ok(()) => {
                // Durable knowledge precedes the last native witness release.
                let settled = match pending.committed() {
                    Ok(()) => settle_pending_recovery(&scope, &pending).await.map(|_| ()),
                    Err(error) => Err(error),
                };
                return settled.map_err(|error| {
                    let mut completion = Completion {
                        local: LocalState::Outstanding,
                        remote: RemoteState::Unknown,
                        ..Completion::default()
                    };
                    completion.committed("native effect committed", request.operation.hex());
                    complete::<()>(Err(request.error(error)), completion).unwrap_err()
                });
            }
            Err(error) => request.error(error),
        };
        match settle_pending_recovery(&scope, &pending).await {
            Ok(EffectDisposition::Committed) if issued_payload => Ok(()),
            Ok(disposition) => {
                let mut completion = Completion::default();
                if disposition == EffectDisposition::Committed {
                    completion.committed("native effect committed", request.operation.hex());
                }
                complete(Err(primary), completion)
            }
            Err(error) => {
                let mut completion = Completion {
                    local: LocalState::Outstanding,
                    remote: RemoteState::Unknown,
                    ..Completion::default()
                };
                if pending.inspect(|entry| entry.observed_disposition).ok()
                    == Some(EffectDisposition::Committed)
                {
                    completion.committed("native effect committed", request.operation.hex());
                }
                completion.step("native protected recovery handback", Err(error));
                complete(Err(primary), completion)
            }
        }
    };
    crate::continuation::admit(&budget, "native effect continuation", body)
}

/// Settle only exact original request metadata, never original production SQL.
async fn settle_pending_recovery(
    scope: &Arc<RecoveryScope>,
    pending: &PendingRecovery,
) -> Result<EffectDisposition, ModelError> {
    settle_pending_recovery_admitted(scope, pending, scope.cancellation()).await
}
async fn settle_pending_recovery_admitted(
    scope: &Arc<RecoveryScope>,
    pending: &PendingRecovery,
    admission: &RetryCancellation,
) -> Result<EffectDisposition, ModelError> {
    let body = recovery_settlement_body(scope, pending, admission)?;
    with_recovery_scope(
        scope.clone(),
        with_retry_context(super::retry::recovery_admission_context(admission), body),
    )
    .await
}
// Erase the logical settlement body before composing task-local policy wrappers.
// This keeps each caller's concrete future independent of the nested native stack.
fn recovery_settlement_body<'a>(
    scope: &'a Arc<RecoveryScope>,
    pending: &'a PendingRecovery,
    admission: &'a RetryCancellation,
) -> Result<crate::continuation::Continuation<'a, Result<EffectDisposition, ModelError>>, ModelError>
{
    use futures::FutureExt;
    let body = async move {
        if admission.stopped() {
            return Err(ModelError::Conflict(
                "native recovery issuance stopped; owner retained",
            ));
        }
        let envelope = pending.inspect(Clone::clone)?;
        let disposition = match envelope.action {
            RecoveryAction::ReconcileOriginalRequest => {
                let reconcile = reconcile_pending_envelope(scope.client(), &envelope, admission);
                let _reconcile_charge = scope.budget().reserve(
                    "native original request reconciliation future",
                    std::mem::size_of_val(&reconcile),
                )?;
                let disposition = reconcile.boxed().await?;
                pending.observed(disposition)?;
                disposition
            }
            RecoveryAction::ReleaseTerminalReference => envelope.observed_disposition,
        };
        let terminal = pending.inspect(Clone::clone)?;
        let release = settle_reference(scope.client(), &terminal, admission);
        let _release_charge = scope.budget().reserve(
            "native exact reference settlement future",
            std::mem::size_of_val(&release),
        )?;
        release.boxed().await?;
        pending.settle(disposition)?;
        Ok(disposition)
    };
    crate::continuation::admit(
        scope.budget(),
        "native exact request settlement future",
        body,
    )
}
// Only this protocol's still-pending owner can discharge a compacted/never-issued
// request. Permanent era fencing plus atomically retained witnesses is the proof;
// a missing outcome alone still means CompactedTerminal to ordinary observers.
pub(super) async fn prove_compacted_owner_absent(
    client: &Surreal<Client>,
    request: NativeRequest,
    reference: &RecordId,
    objects: &[RecordId],
    admission: &RetryCancellation,
) -> Result<(), ModelError> {
    request.validate()?;
    if admission.stopped() {
        return Err(ModelError::Conflict(
            "native compacted proof issuance stopped",
        ));
    }
    let scope = capture_recovery_scope().ok_or(ModelError::Conflict(
        "native compacted proof requires owning scope",
    ))?;
    if !std::ptr::eq(scope.client().as_ref(), client) {
        return Err(ModelError::Conflict(
            "native compacted proof session correspondence",
        ));
    }
    let reader = scope.finalization_reader(admission).await?;
    let mut records = Vec::with_capacity(2 + objects.len());
    records.push(RecordId::new("native_effect", request.operation.hex()));
    records.push(reference.clone());
    records.extend_from_slice(objects);
    let proof: Object = reader.query_native("RETURN {installation:(SELECT generation,closed_through FROM ONLY native_installation:current),records:(SELECT VALUE id FROM $records)}",
        Variables::from_iter([("records".into(), surrealdb::types::SurrealValue::into_value(records))])).await?;
    let installation = proof
        .get("installation")
        .and_then(|value| value.as_object())
        .ok_or(ModelError::Conflict(
            "native compacted owner installation cardinality",
        ))?;
    let records = proof
        .get("records")
        .and_then(|value| value.as_array())
        .ok_or(ModelError::Conflict(
            "native compacted owner record inventory",
        ))?;
    if hash_field(installation, "generation")? != request.issuance.generation
        || int_field(installation, "closed_through")? < request.issuance.era
        || !records.is_empty()
    {
        return Err(ModelError::Conflict(
            "native compacted owner lacks permanent cut/absent retained witnesses",
        ));
    }
    Ok(())
}
pub(super) async fn reconcile_pending_envelope(
    client: &Surreal<Client>,
    envelope: &PendingRecoveryEnvelope,
    admission: &RetryCancellation,
) -> Result<EffectDisposition, ModelError> {
    envelope.validate(&envelope.target)?;
    if envelope.action != RecoveryAction::ReconcileOriginalRequest
        || envelope.observed_disposition != EffectDisposition::Pending
    {
        return Err(ModelError::Conflict(
            "native pending owner reconciliation phase",
        ));
    }
    let disposition = reconcile_request(client, envelope.request).await?;
    if disposition == EffectDisposition::CompactedTerminal {
        prove_compacted_owner_absent(
            client,
            envelope.request,
            &envelope.outcome_reference,
            &[],
            admission,
        )
        .await?;
    }
    Ok(disposition)
}
pub(super) async fn settle_reference(
    client: &Surreal<Client>,
    envelope: &PendingRecoveryEnvelope,
    admission: &RetryCancellation,
) -> Result<(), ModelError> {
    envelope.validate(&envelope.target)?;
    let request = envelope.request;
    if envelope.observed_disposition == EffectDisposition::CompactedTerminal {
        if envelope.action != RecoveryAction::ReleaseTerminalReference {
            return Err(ModelError::Conflict(
                "native compacted settlement requires durable terminal knowledge",
            ));
        }
        return prove_compacted_owner_absent(
            client,
            request,
            &envelope.outcome_reference,
            &[],
            admission,
        )
        .await;
    }
    let mut bindings = Variables::new();
    bindings.insert("reference", envelope.outcome_reference.clone());
    bindings.insert(
        "effect",
        RecordId::new("native_effect", request.operation.hex()),
    );
    bindings.insert("owner", request.operation.hex());
    bindings.insert("kind", envelope.reference_policy.kind());
    bindings.insert("original_era", request.issuance.era);
    bindings.insert(
        "known_committed",
        envelope.observed_disposition == EffectDisposition::Committed,
    );
    if !matches!(
        envelope.observed_disposition,
        EffectDisposition::Committed | EffectDisposition::FencedUncommitted
    ) {
        return Err(ModelError::Conflict(
            "native recovery terminal knowledge missing",
        ));
    }
    request.bind(&mut bindings);
    let current = IssuanceEra::capture(client).await?;
    if current.generation != request.issuance.generation {
        return Err(ModelError::Conflict("native recovery reference generation"));
    }
    bindings.insert("__era", current.era);
    coordination::bind_stripe(&mut bindings, request.operation);
    let check = "LET $prior=SELECT * FROM ONLY $reference FOR UPDATE; IF $prior!=NONE AND ($prior.effect!=$effect OR $prior.owner!=$owner OR $prior.kind!=$kind) { THROW 'native recovery reference correspondence'; }; LET $outcome=SELECT * FROM ONLY $effect FOR UPDATE; IF $outcome!=NONE { IF $outcome.request!=$__request OR $outcome.generation!=$__generation OR $outcome.era!=$original_era OR !$outcome.resolved OR $outcome.committed!=$known_committed { THROW 'native recovery terminal outcome correspondence'; }; } ELSE IF $original_era>$__installation.closed_through { THROW 'native recovery terminal witness unexpectedly absent'; };";
    let action = if envelope.reference_policy == RecoveryReferencePolicy::RetainCaller {
        // An absent-intent fence can establish the explicitly requested caller
        // reference now. Never recreate absent/compacted operation history.
        "IF $outcome=NONE { THROW 'native caller outcome unavailable'; }; UPSERT $reference SET effect=$effect,owner=$owner,kind=$kind RETURN NONE;"
    } else {
        "DELETE $reference RETURN NONE;"
    };
    let sql = format!(
        "BEGIN; {ERA_GUARD} fn::native_touch($effect); fn::native_touch($reference); {check} {action} COMMIT;"
    );
    coordination::preflight(&sql, &bindings)?;
    super::retry::transaction_named(client, &sql, bindings, true, "outcome_reference_settlement")
        .await
}
/// The actual parent finalization owner calls this after joining submitted work.
/// Failed admission/settlement leaves both durable metadata and the owning scope.
pub async fn resume_recovery_scope(scope: &Arc<RecoveryScope>) -> Result<(), ModelError> {
    resume_recovery_scope_admitted(scope, scope.cancellation()).await
}
/// An explicit parent handback recipient may admit a separate recovery operation
/// after original producers have joined. This never reopens production admission.
pub async fn resume_recovery_scope_admitted(
    scope: &Arc<RecoveryScope>,
    admission: &RetryCancellation,
) -> Result<(), ModelError> {
    scope.join_finalization_reads().await?;
    while let Some(pending) = scope.first_pending()? {
        settle_pending_recovery_admitted(scope, &pending, admission).await?;
    }
    Ok(())
}
/// Settle allocation and earn exact source terminal knowledge while both native
/// witnesses remain held. The factory persists that knowledge before release.
pub fn finalize_factory_source_admitted<'a>(
    scope: &'a Arc<RecoveryScope>,
    marker: &'a super::SourceFinalizationMarker,
    admission: &'a RetryCancellation,
    abandon: bool,
) -> futures::future::BoxFuture<'a, Result<EffectDisposition, ModelError>> {
    use futures::FutureExt;
    async move {
        scope.join_finalization_reads().await?;
        if marker.phase == super::SourceFinalizationPhase::SourceTerminal {
            return match marker.allocation_disposition {
                EffectDisposition::Committed
                | EffectDisposition::FencedUncommitted
                | EffectDisposition::CompactedTerminal => Ok(marker.allocation_disposition),
                _ => Err(ModelError::Conflict(
                    "native source terminal allocation knowledge",
                )),
            };
        }
        with_retry_context(
            super::retry::recovery_admission_context(admission),
            with_recovery_scope(
                scope.clone(),
                source_finalization_body(scope, marker, admission, abandon)?,
            ),
        )
        .await
    }
    .boxed()
}
/// A durable SourceTerminal marker is the surviving witness throughout these
/// separately recorded releases; neither action replays the allocation payload.
fn source_finalization_body<'a>(
    scope: &'a Arc<RecoveryScope>,
    marker: &'a super::SourceFinalizationMarker,
    admission: &'a RetryCancellation,
    abandon: bool,
) -> Result<crate::continuation::Continuation<'a, Result<EffectDisposition, ModelError>>, ModelError>
{
    use futures::FutureExt;
    let body = async move {
        if admission.stopped() {
            return Err(ModelError::Conflict(
                "native source recovery issuance stopped",
            ));
        }
        if marker.original_epoch != 1
            || marker.target != *scope.target()
            || marker.allocation.issuance.generation != marker.target.generation
        {
            return Err(ModelError::Conflict(
                "native source finalization correspondence",
            ));
        }
        let disposition = reconcile_request(scope.client(), marker.allocation).await?;
        if disposition == EffectDisposition::CompactedTerminal {
            if marker.phase != super::SourceFinalizationPhase::AllocationPending {
                return Err(ModelError::Conflict(
                    "native active source lost its allocation witness",
                ));
            }
            prove_compacted_owner_absent(
                scope.client(),
                marker.allocation,
                &allocation_outcome_reference(marker.allocation)?,
                &[
                    RecordId::new("native_attempt", marker.attempt.hex()),
                    source_attempt_reference(marker.attempt),
                ],
                admission,
            )
            .await?;
            return Ok(disposition);
        }
        let mut vars = Variables::new();
        vars.insert(
            "attempt",
            RecordId::new("native_attempt", marker.attempt.hex()),
        );
        let reader = scope.finalization_reader(admission).await?;
        let rows: Vec<Object> = reader.query_native("SELECT * FROM $attempt", vars).await?;
        if disposition == EffectDisposition::FencedUncommitted {
            if !rows.is_empty() {
                return Err(ModelError::Conflict(
                    "fenced source allocation has an attempt",
                ));
            }
            return Ok(disposition);
        }
        if disposition != EffectDisposition::Committed {
            return Err(ModelError::Conflict(
                "native source allocation remains uncertain",
            ));
        }
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("native source attempt cardinality"));
        };
        if hash_field(row, "generation")? != marker.target.generation
            || int_field(row, "era")? != marker.allocation.issuance.era
            || int_field(row, "epoch")? != marker.original_epoch
        {
            return Err(ModelError::Conflict(
                "native source attempt identity changed",
            ));
        }
        let terminal = matches!(row.get("state"), Some(Value::String(state)) if matches!(state.as_str(), "closed" | "abandoned"));
        if terminal {
            let holds: Vec<RecordId> = reader.query_native(
                    "SELECT VALUE id FROM native_hold WITH INDEX owner_holds WHERE owner=$attempt LIMIT 1",
                    Variables::from_iter([("attempt".into(), Value::RecordId(RecordId::new("native_attempt", marker.attempt.hex())))]),
                ).await?;
            if !holds.is_empty() {
                return Err(ModelError::Conflict(
                    "native terminal source still holds objects",
                ));
            }
        }
        if abandon && !terminal {
            let close = super::close_attempt(scope.client(), marker.attempt, "abandoned");
            let _close_charge = scope
                .budget()
                .reserve("native source close future", std::mem::size_of_val(&close))?;
            close.boxed().await?;
        } else if !terminal {
            return Err(ModelError::Conflict(
                "native source attempt is not terminal",
            ));
        }
        Ok(disposition)
    };
    crate::continuation::admit(scope.budget(), "native source finalization future", body)
}
pub async fn release_factory_source_references_admitted(
    scope: &Arc<RecoveryScope>,
    marker: &super::SourceFinalizationMarker,
    admission: &RetryCancellation,
) -> Result<(), ModelError> {
    scope.join_finalization_reads().await?;
    if admission.stopped() {
        return Err(ModelError::Conflict(
            "native reference release issuance stopped",
        ));
    }
    let body =
        release_factory_references_body(scope, FactoryReferenceRelease::Source(marker), admission)?;
    if marker.phase == super::SourceFinalizationPhase::SourceTerminal
        && marker.allocation_disposition == EffectDisposition::CompactedTerminal
    {
        return with_recovery_scope(scope.clone(), body).await;
    }
    with_retry_context(
        super::retry::recovery_admission_context(admission),
        with_recovery_scope(scope.clone(), body),
    )
    .await
}
// Ordinary release and the permanently compacted absence proof are alternatives
// of the same witness-release action. Both cross the erased boundary before any
// task-local policy wrappers; neither can expose an unbounded proof future there.
enum FactoryReferenceRelease<'a> {
    Source(&'a super::SourceFinalizationMarker),
    Pin(&'a super::PinFinalizationMarker),
}
fn release_factory_references_body<'a>(
    scope: &'a Arc<RecoveryScope>,
    release: FactoryReferenceRelease<'a>,
    admission: &'a RetryCancellation,
) -> Result<crate::continuation::Continuation<'a, Result<(), ModelError>>, ModelError> {
    let body = async move {
        let (allocation, phase, disposition, identity, table, reference, terminal_error) =
            match release {
                FactoryReferenceRelease::Source(marker) => (
                    marker.allocation,
                    marker.phase,
                    marker.allocation_disposition,
                    marker.attempt,
                    "native_attempt",
                    source_attempt_reference(marker.attempt),
                    "native source durable terminal marker required",
                ),
                FactoryReferenceRelease::Pin(marker) => (
                    marker.allocation,
                    marker.phase,
                    marker.allocation_disposition,
                    marker.pin,
                    "native_pin",
                    source_pin_reference(marker.pin),
                    "native pin durable terminal marker required",
                ),
            };
        if phase == super::SourceFinalizationPhase::SourceTerminal
            && disposition == EffectDisposition::CompactedTerminal
        {
            return prove_compacted_owner_absent(
                scope.client(),
                allocation,
                &allocation_outcome_reference(allocation)?,
                &[RecordId::new(table, identity.hex()), reference],
                admission,
            )
            .await;
        }
        if phase != super::SourceFinalizationPhase::SourceTerminal
            || !matches!(
                disposition,
                EffectDisposition::Committed | EffectDisposition::FencedUncommitted
            )
        {
            return Err(ModelError::Conflict(terminal_error));
        }
        if admission.stopped() {
            return Err(ModelError::Conflict(
                "native source/pin reference release stopped",
            ));
        }
        super::history::release_outcome_reference(
            scope.client(),
            reference,
            &identity.hex(),
            "caller",
        )
        .await?;
        super::history::release_outcome(
            scope.client(),
            allocation,
            &allocation.operation.hex(),
            "caller",
        )
        .await
    };
    crate::continuation::admit(
        scope.budget(),
        "native source/pin witness release continuation",
        body,
    )
}

#[derive(Clone, Copy)]
pub enum PinFinalizationAuthority {
    VerifyReleased,
    /// The actual factory has joined query owners and checked pin borrowers/certainty.
    OwnedDrained,
    /// The explicit resumer authenticated the original host's loss under closed maintenance.
    HostGoneMaintenance,
}
pub fn finalize_factory_pin_admitted<'a>(
    scope: &'a Arc<RecoveryScope>,
    marker: &'a super::PinFinalizationMarker,
    admission: &'a RetryCancellation,
    authority: PinFinalizationAuthority,
) -> futures::future::BoxFuture<'a, Result<EffectDisposition, ModelError>> {
    use futures::FutureExt;
    async move {
        scope.join_finalization_reads().await?;
        if marker.phase == super::SourceFinalizationPhase::SourceTerminal {
            return match marker.allocation_disposition {
                EffectDisposition::Committed
                | EffectDisposition::FencedUncommitted
                | EffectDisposition::CompactedTerminal => Ok(marker.allocation_disposition),
                _ => Err(ModelError::Conflict(
                    "native pin terminal allocation knowledge",
                )),
            };
        }
        with_retry_context(
            super::retry::recovery_admission_context(admission),
            with_recovery_scope(
                scope.clone(),
                pin_finalization_body(scope, marker, admission, authority)?,
            ),
        )
        .await
    }
    .boxed()
}
fn pin_finalization_body<'a>(
    scope: &'a Arc<RecoveryScope>,
    marker: &'a super::PinFinalizationMarker,
    admission: &'a RetryCancellation,
    authority: PinFinalizationAuthority,
) -> Result<crate::continuation::Continuation<'a, Result<EffectDisposition, ModelError>>, ModelError>
{
    let body = async move {
        if admission.stopped() {
            return Err(ModelError::Conflict("native pin recovery issuance stopped"));
        }
        if marker.original_epoch != 1
            || marker.target != *scope.target()
            || marker.allocation.issuance.generation != marker.target.generation
        {
            return Err(ModelError::Conflict(
                "native pin finalization correspondence",
            ));
        }
        let disposition = reconcile_request(scope.client(), marker.allocation).await?;
        if disposition == EffectDisposition::CompactedTerminal {
            if marker.phase != super::SourceFinalizationPhase::AllocationPending {
                return Err(ModelError::Conflict(
                    "native active pin lost its allocation witness",
                ));
            }
            prove_compacted_owner_absent(
                scope.client(),
                marker.allocation,
                &allocation_outcome_reference(marker.allocation)?,
                &[
                    RecordId::new("native_pin", marker.pin.hex()),
                    source_pin_reference(marker.pin),
                ],
                admission,
            )
            .await?;
            return Ok(disposition);
        }
        let vars = Variables::from_iter([(
            "pin".into(),
            Value::RecordId(RecordId::new("native_pin", marker.pin.hex())),
        )]);
        let reader = scope.finalization_reader(admission).await?;
        let rows: Vec<Object> = reader.query_native("SELECT * FROM $pin", vars).await?;
        if disposition == EffectDisposition::FencedUncommitted {
            if !rows.is_empty() {
                return Err(ModelError::Conflict("fenced pin allocation has a pin"));
            }
            return Ok(disposition);
        }
        if disposition != EffectDisposition::Committed {
            return Err(ModelError::Conflict(
                "native pin allocation remains uncertain",
            ));
        }
        let [row] = rows.as_slice() else {
            return Err(ModelError::Conflict("native pin finalization cardinality"));
        };
        if hash_field(row, "generation")? != marker.target.generation
            || int_field(row, "era")? != marker.allocation.issuance.era
            || int_field(row, "epoch")? != marker.original_epoch
        {
            return Err(ModelError::Conflict(
                "native pin allocation identity changed",
            ));
        }
        match authority {
            PinFinalizationAuthority::VerifyReleased => {
                if row.get("released") != Some(&Value::Bool(true)) {
                    return Err(ModelError::Conflict("native pin is not released"));
                }
            }
            PinFinalizationAuthority::OwnedDrained => {
                super::release_pin_checked(scope.client(), marker.pin, false).await?
            }
            PinFinalizationAuthority::HostGoneMaintenance => {
                super::release_pin_checked(scope.client(), marker.pin, true).await?
            }
        }
        Ok(disposition)
    };
    crate::continuation::admit(scope.budget(), "native pin finalization future", body)
}
pub async fn release_factory_pin_references_admitted(
    scope: &Arc<RecoveryScope>,
    marker: &super::PinFinalizationMarker,
    admission: &RetryCancellation,
) -> Result<(), ModelError> {
    scope.join_finalization_reads().await?;
    if admission.stopped() {
        return Err(ModelError::Conflict(
            "native reference release issuance stopped",
        ));
    }
    let body =
        release_factory_references_body(scope, FactoryReferenceRelease::Pin(marker), admission)?;
    if marker.phase == super::SourceFinalizationPhase::SourceTerminal
        && marker.allocation_disposition == EffectDisposition::CompactedTerminal
    {
        return with_recovery_scope(scope.clone(), body).await;
    }
    with_retry_context(
        super::retry::recovery_admission_context(admission),
        with_recovery_scope(scope.clone(), body),
    )
    .await
}
// Observation retains the original row for owner validation without acquiring fresh
// owner authority or performing a second racy receipt lookup after an era cut.
pub async fn request_disposition(
    client: &Surreal<Client>,
    request: NativeRequest,
) -> Result<EffectDisposition, ModelError> {
    observe_request(client, request)
        .await
        .map(|(disposition, _)| disposition)
}
async fn observe_request(
    client: &Surreal<Client>,
    request: NativeRequest,
) -> Result<(EffectDisposition, Option<Object>), ModelError> {
    request.validate()?;
    let mut bindings = Variables::new();
    request.bind(&mut bindings);
    let mut response=client.query("SELECT * FROM $__effect; SELECT generation,closed_through FROM native_installation:current")
        .bind(bindings).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let mut rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let installations: Vec<Object> = response.take(1).map_err(ModelError::codec)?;
    if rows.len() == 1 {
        let row = rows.pop().unwrap();
        if row.get("id")
            != Some(&Value::RecordId(RecordId::new(
                "native_effect",
                request.operation.hex(),
            )))
            || hash_field(&row, "request")? != request.request
            || int_field(&row, "era")? != request.issuance.era
            || hash_field(&row, "generation")? != request.issuance.generation
        {
            return Err(ModelError::Conflict("native receipt identity"));
        }
        let disposition = if row.get("committed") == Some(&Value::Bool(true)) {
            EffectDisposition::Committed
        } else if row.get("resolved") == Some(&Value::Bool(true)) {
            EffectDisposition::FencedUncommitted
        } else {
            EffectDisposition::Pending
        };
        return Ok((disposition, Some(row)));
    }
    if !rows.is_empty() {
        return Err(ModelError::Schema("native receipt inventory"));
    }
    let [installation] = installations.as_slice() else {
        return Err(ModelError::Schema("native receipt installation"));
    };
    if hash_field(installation, "generation")? != request.issuance.generation {
        return Err(ModelError::Conflict("native receipt generation"));
    }
    Ok((
        if request.issuance.era <= int_field(installation, "closed_through")? {
            EffectDisposition::CompactedTerminal
        } else {
            EffectDisposition::Pending
        },
        None,
    ))
}
fn validate_replay_owner(
    request: NativeRequest,
    owner: &EffectOwner,
    row: &Object,
) -> Result<(), ModelError> {
    let expected_owner = owner
        .record()
        .unwrap_or_else(|| coordination::operation_owner(request));
    let expected_attempt = match owner {
        EffectOwner::OpenAttempt(attempt) | EffectOwner::FrozenAttempt(attempt) => {
            Value::RecordId(RecordId::new("native_attempt", attempt.hex()))
        }
        EffectOwner::TerminalAttempt(attempt) => {
            Value::RecordId(RecordId::new("native_attempt", attempt.identity.hex()))
        }
        _ => Value::None,
    };
    if row.get("owner_kind") != Some(&Value::String(owner.kind().into()))
        || row.get("owner") != Some(&Value::RecordId(expected_owner))
        || row.get("attempt").cloned().unwrap_or(Value::None) != expected_attempt
        || row.get("committed") != Some(&Value::Bool(true))
        || row.get("resolved") != Some(&Value::Bool(true))
        || int_field(row, "epoch")? <= 0
    {
        return Err(request.error(ModelError::Conflict(
            "native committed replay original owner changed",
        )));
    }
    Ok(())
}
pub async fn reconcile_request(
    client: &Surreal<Client>,
    request: NativeRequest,
) -> Result<EffectDisposition, ModelError> {
    request.validate()?;
    let current = IssuanceEra::capture(client).await?;
    if current.generation != request.issuance.generation {
        return Err(request.error(ModelError::Conflict("native reconciliation generation")));
    }
    let mut bindings = Variables::new();
    request.bind(&mut bindings);
    bindings.insert("original_era", request.issuance.era);
    bindings.insert("original_operation", request.operation.hex());
    bindings.insert("__era", current.era);
    // Fresh maintenance stripe authority fences or observes ONLY the exact original
    // request. No predecessor SQL or expected object identity is refreshed/executed.
    let sql = format!(
        "BEGIN; {ERA_GUARD} fn::native_touch($__effect); LET $state=SELECT * FROM ONLY $__effect FOR UPDATE; IF $state=NONE {{ IF $original_era>$__installation.closed_through {{ IF $original_era!=$__era {{ THROW 'native reconciliation era'; }}; CREATE $__effect SET attempt=NONE,owner=NONE,owner_kind='fence',generation=$__generation,era=$original_era,request=$__request,committed=false,resolved=true,revision=1,epoch=0 RETURN NONE; }}; }} ELSE {{ IF $state.request!=$__request OR $state.era!=$original_era OR $state.generation!=$__generation {{ THROW 'native reconciliation identity'; }}; fn::native_incidence($state); IF !$state.resolved {{ UPDATE $__effect SET resolved=true,revision+=1 RETURN NONE; IF $state.owner_kind='installation' AND $state.owner!=NONE {{ IF record::table($state.owner)!='native_operation_owner' {{ THROW 'native reconciliation local owner family'; }}; LET $local_owner=$state.owner; LET $local=SELECT * FROM ONLY $local_owner FOR UPDATE; IF $local=NONE OR $local.generation!=$__generation OR $local.era!=$original_era OR $local.epoch!=$state.epoch OR $local.operation!=$original_operation {{ THROW 'native reconciliation local owner identity'; }}; UPDATE $state.owner SET state='done' RETURN NONE; }}; }}; }}; COMMIT;"
    );
    coordination::preflight(&sql, &bindings)?;
    super::run_recovery_transaction(client, &sql, bindings)
        .await
        .map_err(|error| request.error(error))?;
    request_disposition(client, request).await
}
/// ID-only observation cannot invent missing authorization. Callers that may retry
/// use reconcile_request and retain NativeRequest, including its pre-intent era.
pub async fn reconcile_effect(
    client: &Surreal<Client>,
    operation: ContentHash,
) -> Result<bool, ModelError> {
    let mut bindings = Variables::new();
    bindings.insert("effect", RecordId::new("native_effect", operation.hex()));
    let mut response = client
        .query("SELECT * FROM $effect")
        .bind(bindings)
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
    let [row] = rows.as_slice() else {
        return Err(ModelError::Conflict(
            "native reconciliation requires original request",
        ));
    };
    let request = NativeRequest {
        issuance: IssuanceEra {
            generation: hash_field(row, "generation")?,
            era: int_field(row, "era")?,
        },
        operation,
        request: hash_field(row, "request")?,
    };
    Ok(reconcile_request(client, request).await? == EffectDisposition::Committed)
}

#[cfg(test)]
mod payload_controls {
    use super::*;
    #[tokio::test(flavor = "multi_thread")]
    async fn original_completion_replay_reads_same_retained_result_without_reexecuting_payload() {
        use futures::FutureExt;
        use lctx_compiler_store::compiler::NativeCompilerStore;
        use lctx_model::domain::completion;
        let config = lctx_compiler_store::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("fresh owned native campaign"),
        ))
        .unwrap();
        let store = NativeCompilerStore::begin(
            &config,
            lctx_model::domain::admission::Frontier::Facts,
            &lctx_compiler_store::compiler::recovery_factory(
                &config,
                (lctx_model::domain::resources::ResourceBudget::fixed(
                    lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
                )
                .expect("owned native control recovery budget"))
                .clone(),
            )
            .expect("owned native control factory"),
        )
        .await
        .unwrap();
        let client = lctx_compiler_store::reader::connect(
            &config.endpoint,
            &config.writer_credentials(),
            store.namespace().as_str(),
            store.database().as_str(),
        )
        .await
        .unwrap();
        let mut retained = None;
        let result=std::panic::AssertUnwindSafe(async{
            let payload=PreparedNativePayload::new("IF (SELECT VALUE original_completion_outcome FROM ONLY $__effect)!=NONE { THROW 'original completion replay reexecuted'; }; UPDATE $__effect SET original_completion_outcome='premise_changed' RETURN NONE;",Variables::new()).unwrap();
            let request=NativeRequest{issuance:IssuanceEra::capture(&client).await.unwrap(),operation:fresh_identity("completion-result-replay-control").unwrap(),request:payload.digest};
            retained=Some(request);
            execute_recorded(&client,request,EffectOwner::OpenAttempt(store.attempt()),payload,true).await.unwrap();
            let (disposition,row)=observe_request(&client,request).await.unwrap();assert_eq!(disposition,EffectDisposition::Committed);assert_eq!(original_completion_outcome(&row.unwrap()).unwrap(),OriginalCompletionOutcome::PremiseChanged);
            let same=PreparedNativePayload::new("IF (SELECT VALUE original_completion_outcome FROM ONLY $__effect)!=NONE { THROW 'original completion replay reexecuted'; }; UPDATE $__effect SET original_completion_outcome='premise_changed' RETURN NONE;",Variables::new()).unwrap();
            execute_recorded(&client,request,EffectOwner::OpenAttempt(store.attempt()),same,true).await.unwrap();
            let (disposition,row)=observe_request(&client,request).await.unwrap();assert_eq!(disposition,EffectDisposition::Committed);assert_eq!(original_completion_outcome(&row.unwrap()).unwrap(),OriginalCompletionOutcome::PremiseChanged);
        }).catch_unwind().await;
        let mut cleanup = completion::Completion::default();
        if let Some(request) = retained {
            cleanup.step(
                "completion replay exact outcome release",
                super::super::history::release_outcome(
                    &client,
                    request,
                    &request.operation.hex(),
                    "caller",
                )
                .await,
            );
        }
        cleanup.step("completion replay attempt cleanup", store.abandon().await);
        cleanup.step(
            "completion replay observer invalidation",
            client.invalidate().await.map_err(ModelError::codec),
        );
        let cleaned = completion::complete(Ok(()), cleanup);
        if let Err(panic) = result {
            if let Err(error) = cleaned {
                eprintln!("completion replay cleanup: {error:?}");
            }
            std::panic::resume_unwind(panic);
        }
        cleaned.unwrap();
    }
    #[test]
    fn original_completion_only_known_persisted_codes_allow_recapture() {
        let mut row = Object::new();
        assert!(original_completion_outcome(&row).is_err());
        for value in [
            Value::None,
            Value::Bool(true),
            Value::String("unknown".into()),
        ] {
            row.insert("original_completion_outcome", value);
            assert!(original_completion_outcome(&row).is_err());
        }
        row.insert("original_completion_outcome", "applied");
        assert_eq!(
            original_completion_outcome(&row).unwrap(),
            OriginalCompletionOutcome::Applied
        );
        row.insert("original_completion_outcome", "premise_changed");
        assert_eq!(
            original_completion_outcome(&row).unwrap(),
            OriginalCompletionOutcome::PremiseChanged
        );
    }
    #[tokio::test]
    async fn nonpositive_request_eras_refuse_before_observation_reconciliation_or_execution() {
        let client = Surreal::init();
        let payload = PreparedNativePayload::new("RETURN []", Variables::new()).unwrap();
        for era in [0, -1] {
            let request = NativeRequest {
                issuance: IssuanceEra {
                    generation: ContentHash::of(b"generation"),
                    era,
                },
                operation: ContentHash::of(b"operation"),
                request: payload.digest,
            };
            let failures = [
                request_disposition(&client, request).await.unwrap_err(),
                reconcile_request(&client, request).await.unwrap_err(),
                execute_request(&client, request, None, "RETURN []", Variables::new())
                    .await
                    .unwrap_err(),
            ];
            for error in failures {
                let ModelError::Cause(cause) = error else {
                    panic!("original request refusal missing");
                };
                let error = cause.downcast_ref::<NativeRequestError>().unwrap();
                assert_eq!(error.request, request);
                assert!(matches!(
                    error.source,
                    ModelError::Conflict("native request requires a positive issuance era")
                ));
            }
        }
    }
    #[tokio::test]
    async fn changed_retry_bytes_refuse_before_any_native_request() {
        let mut original = Variables::new();
        original.insert("rows", vec![1i64, 2]);
        let payload = PreparedNativePayload::new("RETURN $rows", original.clone()).unwrap();
        let request = NativeRequest {
            issuance: IssuanceEra {
                generation: ContentHash::of(b"generation"),
                era: 1,
            },
            operation: ContentHash::of(b"operation"),
            request: payload.digest,
        };
        let client = Surreal::init();
        let mut changed = original.clone();
        changed.insert("rows", vec![1i64, 3]);
        for (sql, bindings) in [("RETURN $rows", changed), ("RETURN []", original)] {
            let error = execute_request(&client, request, None, sql, bindings)
                .await
                .unwrap_err();
            let ModelError::Cause(cause) = error else {
                panic!("request error");
            };
            let error = cause.downcast_ref::<NativeRequestError>().unwrap();
            assert_eq!(error.request, request);
            assert!(matches!(
                error.source,
                ModelError::Conflict("native retry payload changed")
            ));
        }
    }
    #[test]
    fn intent_contains_only_authorization_controls_even_for_large_payloads() {
        let mut payload = Variables::new();
        let request = NativeRequest {
            issuance: IssuanceEra {
                generation: ContentHash::of(b"generation"),
                era: 3,
            },
            operation: ContentHash::of(b"operation"),
            request: ContentHash::of(b"payload"),
        };
        request.bind(&mut payload);
        for key in [
            "__outcome_ref",
            "__reference_owner",
            "__reference_kind",
            "__attempt",
            "__owner_id",
            "__owner_kind",
            "__attempt_slot",
            "__stripe",
        ] {
            payload.insert(key, "control");
        }
        payload.insert("rows", vec!["payload".repeat(1 << 20)]);
        payload.insert("__expected_epoch", 7i64);
        let controls = intent_bindings(&payload).unwrap();
        assert!(controls.get("rows").is_none());
        assert_eq!(
            controls.get("__expected_epoch"),
            payload.get("__expected_epoch")
        );
        assert!(serde_json::to_vec(&controls).unwrap().len() < 2048);
    }
}

#[cfg(test)]
mod owner_effect_packet_controls {
    use super::*;
    #[test]
    fn complete_owner_plan_refuses_payload_that_fits_without_protocol_wrappers() {
        let request = NativeRequest {
            issuance: IssuanceEra {
                generation: ContentHash([1; 32]),
                era: 1,
            },
            operation: ContentHash([2; 32]),
            request: ContentHash([3; 32]),
        };
        let owner = EffectOwner::OpenAttempt(ContentHash([4; 32]));
        let bindings = |length: usize| {
            let mut values = Variables::new();
            values.insert("payload", "x".repeat(length));
            values.insert("__object_expectations", Vec::<Value>::new());
            values
        };
        let mut lower = 0;
        let mut upper = lctx_model::domain::resources::TRANSFER_BYTES;
        while lower + 1 < upper {
            let middle = (lower + upper) / 2;
            if coordination::preflight("RETURN $payload;", &bindings(middle)).is_ok() {
                lower = middle;
            } else {
                upper = middle;
            }
        }
        let values = bindings(lower);
        coordination::preflight("RETURN $payload;", &values).unwrap();
        assert!(matches!(
            owner_effect_plan(request, &owner, "RETURN $payload;", values, false),
            Err(ModelError::Limit {
                owner: "native transaction",
                limit: "complete control and payload bytes",
                ..
            })
        ));
        let plan =
            owner_effect_plan(request, &owner, "RETURN $payload;", bindings(16), false).unwrap();
        assert_eq!(
            plan.bindings.get("__effect"),
            Some(&Value::RecordId(RecordId::new(
                "native_effect",
                request.operation.hex()
            )))
        );
        assert_eq!(
            plan.bindings.get("__object_expectations"),
            Some(&Value::Array(Vec::<Value>::new().into()))
        );
    }
}

#[cfg(test)]
mod compacted_owner_controls {
    use super::*;
    use lctx_model::domain::resources::{DEFAULT_MEMORY_BYTES, ResourceBudget};

    // Owns the installation era cut. Select only on a disposable, otherwise idle
    // RocksDB campaign; ordinary fixture verification must never select it.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "exclusive task-owned RocksDB campaign only"]
    async fn native_never_issued_owners_settle_after_permanent_cut_without_inventing_outcomes() {
        let config = crate::RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned native campaign"),
        ))
        .unwrap();
        let budget = ResourceBudget::fixed(DEFAULT_MEMORY_BYTES).unwrap();
        let factory = RecoveryFactoryOwner::new(config.receipt_binding().unwrap(), budget).unwrap();
        let (client, lease) = crate::reader::connect_owned(
            &config.endpoint,
            &config.writer_credentials(),
            config.namespace.as_str(),
            config.database.as_str(),
            &factory,
        )
        .await
        .unwrap();
        let scope = lease.recovery_scope().clone();
        with_recovery_scope(scope.clone(), async {
            let era = IssuanceEra::capture(&client).await.unwrap();
            let allocation =
                prepare_attempt_allocation(fresh_identity("never-issued-source").unwrap(), era)
                    .unwrap();
            lease
                .register_attempt(allocation.attempt, allocation.request)
                .unwrap();
            let request = NativeRequest::issue(&client, ContentHash::of(b"never-issued-request"))
                .await
                .unwrap();
            let reference =
                request_outcome_reference(request, RecoveryReferencePolicy::ReleaseRecovery)
                    .unwrap();
            let pending = scope
                .prepare(
                    request,
                    RecoveryOwner::Installation,
                    reference.clone(),
                    RecoveryReferencePolicy::ReleaseRecovery,
                )
                .unwrap();
            super::super::set_admission(&client, false).await.unwrap();
            super::super::history::cut_era(&client).await.unwrap();
            assert_eq!(
                reconcile_request(&client, request).await.unwrap(),
                EffectDisposition::CompactedTerminal
            );
            // A retained witness makes the purported absent-owner proof invalid.
            client
                .query(
                    "CREATE $reference SET effect=$effect,owner=$owner,kind='recovery' RETURN NONE",
                )
                .bind(("reference", reference.clone()))
                .bind((
                    "effect",
                    RecordId::new("native_effect", request.operation.hex()),
                ))
                .bind(("owner", request.operation.hex()))
                .await
                .unwrap()
                .check()
                .unwrap();
            assert!(
                prove_compacted_owner_absent(
                    &client,
                    request,
                    &reference,
                    &[],
                    scope.cancellation()
                )
                .await
                .is_err()
            );
            client
                .query("DELETE $reference RETURN NONE")
                .bind(("reference", reference.clone()))
                .await
                .unwrap()
                .check()
                .unwrap();
            assert_eq!(
                settle_pending_recovery_admitted(&scope, &pending, scope.cancellation())
                    .await
                    .unwrap(),
                EffectDisposition::CompactedTerminal
            );
            assert_eq!(scope.pending_count().unwrap(), 0);
            let count: i64 = scope
                .finalization_reader(scope.cancellation())
                .await
                .unwrap()
                .query_native(
                    "RETURN array::len(SELECT VALUE id FROM $records)",
                    Variables::from_iter([(
                        "records".into(),
                        surrealdb::types::SurrealValue::into_value(vec![
                            reference,
                            RecordId::new("native_effect", request.operation.hex()),
                        ]),
                    )]),
                )
                .await
                .unwrap();
            assert_eq!(
                count, 0,
                "settlement must not synthesize an old outcome or caller reference"
            );
            super::super::set_admission(&client, true).await.unwrap();
        })
        .await;
        lease.complete().await.unwrap();
        assert_eq!(factory.pending_count().unwrap(), 0);
    }
}

#[cfg(test)]
#[path = "authorization/recovery_controls.rs"]
mod recovery_native_controls;
