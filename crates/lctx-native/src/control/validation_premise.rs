//! Installation-local executed whole-input premises. Portable records never grant reuse.
//! The fixed model driver earns semantic success; this owner independently observes
//! exactly those typed rows beneath held installed namespaces before persisting it.
use super::*;
use crate::installed_view::{HeldInstalledView, InstalledViewCompletion};
use arrow_array::RecordBatch;
use lctx_model::domain::{
    self as d, Record, Relation, ValidatedModel,
    analysis::sources::SourceSnapshot,
    charged::StateCharge,
    checked_admission::{CheckedValidationPremise, UniformOwnerPremise, ValidationPremise},
    resources::ResourceBudget,
    stages::{Profile, PublicationBoundary},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedInput {
    pub source: SourceSnapshot,
    pub requested: Option<PublicationBoundary>,
    pub resolved: Option<PublicationBoundary>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum PhysicalInput {
    Internal {
        view: d::completed::CompletedView,
        mappings: Vec<crate::installed_view::ContributorRealization>,
    },
    Retained {
        completion: InstalledViewCompletion,
        incarnation: i64,
    },
}
impl PhysicalInput {
    fn retained(&self) -> Result<(&InstalledViewCompletion, i64), ModelError> {
        match self {
            Self::Retained {
                completion,
                incarnation,
            } => Ok((completion, *incarnation)),
            Self::Internal { .. } => Err(ModelError::Conflict(
                "internal selection has no durable validation receipt",
            )),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeKey {
    version: u32,
    model: ContentHash,
    profile: Profile,
    name: String,
    revision: u32,
    definition: ContentHash,
    factory: ContentHash,
    schema: ContentHash,
    contracts: Vec<ContentHash>,
    inputs: Vec<SelectedInput>,
    uniform: Vec<UniformOwnerPremise>,
    physical: Vec<PhysicalInput>,
}
impl ScopeKey {
    fn identity(&self) -> Result<ContentHash, ModelError> {
        let mut sink = KeySink::new("native-executed-validation-premise/v3");
        sink.part(
            b"canonical",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
    fn record(&self) -> Result<RecordId, ModelError> {
        Ok(RecordId::new(
            "native_validation_premise",
            self.identity()?.hex(),
        ))
    }
    fn check_proof(
        &self,
        proof: &ValidationPremise,
        model: &ValidatedModel,
    ) -> Result<(), ModelError> {
        proof.identity()?;
        if !proof.eligible(model)?
            || proof.kind != 0
            || proof.profile.is_some()
            || proof.name != self.name
            || proof.revision != self.revision
            || proof.definition != self.definition
            || proof.factory != self.factory
            || proof.model != self.model
            || proof.uniform_owners != self.uniform
            || proof.inputs.len() != self.inputs.len()
        {
            return Err(ModelError::Conflict(
                "native premise executable proof differs",
            ));
        }
        for ((actual, selected), contract) in
            proof.inputs.iter().zip(&self.inputs).zip(&self.contracts)
        {
            if actual.source != selected.source
                || actual.requested != selected.requested
                || actual.resolved != selected.resolved
                || actual.contract != *contract
                || actual.rows != selected.source.rows() as u64
            {
                return Err(ModelError::Conflict(
                    "native premise exact selected role differs",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    key: ScopeKey,
    proof: ValidationPremise,
}
impl Receipt {
    fn value(&self) -> Result<Value, ModelError> {
        let mut row = Object::new();
        row.insert("id", self.key.record()?);
        row.insert(
            "descriptor",
            surrealdb::types::Bytes::from(serde_json::to_vec(self).map_err(ModelError::codec)?),
        );
        Ok(Value::Object(row))
    }
    fn decode(value: &Value) -> Result<Self, ModelError> {
        let Some(Value::Bytes(bytes)) = value.as_object().and_then(|row| row.get("descriptor"))
        else {
            return Err(ModelError::Schema("native premise descriptor"));
        };
        let result: Self = serde_json::from_slice(bytes).map_err(ModelError::codec)?;
        if result.value()? != *value {
            return Err(ModelError::Conflict(
                "native premise exact immutable envelope",
            ));
        }
        Ok(result)
    }
}
fn serialization_workspace<T: Serialize>(
    value: &T,
    budget: &ResourceBudget,
    owner: &'static str,
) -> Result<Box<dyn d::resources::Reservation>, ModelError> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Counter(0);
    serde_json::to_writer(&mut count, value).map_err(ModelError::codec)?;
    budget.reserve(owner, count.0.saturating_mul(32).saturating_add(65536))
}

struct ObservedInput {
    relation: Relation,
    sink: Option<KeySink>,
    rows: u64,
    ended: bool,
}
/// A real native session and physical pin remain owned through lookup, observation,
/// persistence and cleanup. No public DTO can construct a successful observed ledger.
pub struct PendingValidation {
    key: ScopeKey,
    model: Arc<ValidatedModel>,
    caps: Vec<Arc<HeldInstalledView>>,
    reader: crate::NativeReader<()>,
    observed: Vec<ObservedInput>,
    current: usize,
    failed: bool,
    closed: bool,
    budget: ResourceBudget,
    _charge: StateCharge,
}
impl Drop for PendingValidation {
    fn drop(&mut self) {
        if !self.closed {
            self.reader.retain_unknown();
            for cap in &self.caps {
                cap.retain_unknown();
            }
            std::mem::forget(self.reader.shared_client());
        }
    }
}
impl PendingValidation {
    pub async fn prepare(
        model: Arc<ValidatedModel>,
        invariant: &d::Invariant,
        profile: Profile,
        inputs: Vec<SelectedInput>,
        mut caps: Vec<Arc<HeldInstalledView>>,
        uniform: &[d::uniform_outcome::ValidatedUniformOwner],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let contract = model
            .factory_contract(invariant.identity())
            .ok_or(ModelError::Conflict("native premise undeclared factory"))?;
        // Enforce the same closed whole-input eligibility at the native boundary.
        let _source_copy = budget.reserve(
            "native premise eligibility source copies",
            inputs
                .iter()
                .map(|input| {
                    std::mem::size_of::<(
                        SourceSnapshot,
                        Option<PublicationBoundary>,
                        Option<PublicationBoundary>,
                    )>()
                    .saturating_add(d::HeapSize::heap_bytes(&input.source))
                })
                .fold(0usize, usize::saturating_add),
        )?;
        let sources = inputs
            .iter()
            .map(|input| (input.source.clone(), input.requested, input.resolved))
            .collect();
        let _whole = d::checked_admission::FixedValidationDriver::invariant(
            &model,
            invariant.identity(),
            sources,
            uniform,
            budget,
        )?;
        drop(_whole);
        drop(_source_copy);
        if inputs.is_empty() || caps.is_empty() {
            return Err(ModelError::Conflict(
                "native premise requires protected declared inputs",
            ));
        }
        caps.sort_unstable_by_key(|cap| cap.completion().view.identity);
        caps.dedup_by(|left, right| left.completion() == right.completion());
        let mut charge = StateCharge::new(budget, "native durable premise metadata");
        charge.grow(
            std::mem::size_of::<Self>()
                .saturating_add(invariant.name.len())
                .saturating_add(
                    contract
                        .inputs
                        .len()
                        .saturating_mul(std::mem::size_of::<ContentHash>()),
                )
                .saturating_add(
                    inputs
                        .capacity()
                        .saturating_mul(std::mem::size_of::<SelectedInput>()),
                )
                .saturating_add(
                    inputs
                        .iter()
                        .map(|input| d::HeapSize::heap_bytes(&input.source))
                        .sum::<usize>(),
                )
                .saturating_add(
                    caps.capacity()
                        .saturating_mul(std::mem::size_of::<Arc<HeldInstalledView>>()),
                )
                .saturating_add(
                    caps.len()
                        .saturating_mul(std::mem::size_of::<PhysicalInput>()),
                )
                .saturating_add(
                    caps.iter()
                        .filter(|cap| cap.is_internal())
                        .map(|cap| {
                            crate::installed_view::view_heap(&cap.completion().view).saturating_add(
                                cap.completion().mappings.len().saturating_mul(
                                    std::mem::size_of::<
                                        crate::installed_view::ContributorRealization,
                                    >(),
                                ),
                            )
                        })
                        .sum::<usize>(),
                )
                .saturating_add(
                    uniform
                        .len()
                        .saturating_mul(std::mem::size_of::<UniformOwnerPremise>()),
                )
                .saturating_add(
                    invariant
                        .inputs
                        .len()
                        .saturating_mul(std::mem::size_of::<ObservedInput>()),
                )
                .saturating_add(4096),
        )?;
        for input in &inputs {
            if input.source.model() != model.digest()
                || input.source.relation() == d::artifact::ArtifactChunk::NAME
                || !caps.iter().any(|cap| {
                    cap.completion().view.identity == input.source.view()
                        && cap.completion().view.relation == input.source.relation()
                        && cap.completion().view.rows == input.source.rows() as u64
                })
            {
                return Err(ModelError::Conflict(
                    "native premise selected physical input missing/unsupported",
                ));
            }
        }
        let mut owners = uniform
            .iter()
            .map(|owner| UniformOwnerPremise {
                contribution: owner.contribution(),
                descriptor: *d::Record::id(owner.descriptor()).bytes(),
                content: d::Record::content_digest(owner.descriptor()),
                profile: owner.profile(),
            })
            .collect::<Vec<_>>();
        let owner_order = budget.reserve(
            "native premise uniform owner ordering",
            owners
                .len()
                .saturating_mul(std::mem::size_of::<UniformOwnerPremise>()),
        )?;
        owners.sort_by_key(|owner| (owner.contribution, owner.descriptor));
        drop(owner_order);
        if owners.iter().any(|owner| {
            !caps.iter().any(|cap| {
                cap.completion()
                    .mappings
                    .iter()
                    .any(|mapping| mapping.logical == owner.contribution)
            })
        }) {
            return Err(ModelError::Conflict(
                "native premise uniform owner outside protected closure",
            ));
        }
        let key = ScopeKey {
            version: 3,
            model: model.digest(),
            profile,
            name: invariant.name.into(),
            revision: invariant.revision,
            definition: invariant.digest(),
            factory: contract.identity()?,
            schema: crate::installation::base_schema_identity(),
            contracts: contract.inputs.clone(),
            inputs,
            uniform: owners,
            physical: caps
                .iter()
                .map(|cap| {
                    if cap.is_internal() {
                        PhysicalInput::Internal {
                            view: cap.completion().view.clone(),
                            mappings: cap.completion().mappings.clone(),
                        }
                    } else {
                        PhysicalInput::Retained {
                            completion: cap
                                .retained_completion()
                                .expect("retained capability variant")
                                .clone(),
                            incarnation: cap.completion_incarnation(),
                        }
                    }
                })
                .collect(),
        };
        let observed = invariant
            .inputs
            .iter()
            .map(|declared| {
                let mut sink = KeySink::new("validation-ordered-typed-input/v1");
                declared.encode_contract(&mut sink);
                Ok(ObservedInput {
                    relation: model
                        .relation(declared.name())
                        .ok_or(ModelError::Schema(declared.name()))?
                        .clone(),
                    sink: Some(sink),
                    rows: 0,
                    ended: false,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        let client = caps[0].validation_client();
        let reader = crate::NativeReader::for_installed_views(client, &caps, budget).await?;
        Ok(Self {
            key,
            model,
            caps,
            reader,
            observed,
            current: 0,
            failed: false,
            closed: false,
            budget: budget.clone(),
            _charge: charge,
        })
    }
    async fn current(&self) -> Result<(), ModelError> {
        self.reader.ensure_scope_live()?;
        for cap in &self.caps {
            cap.members(&self.reader, &[], &self.budget).await?;
        }
        Ok(())
    }
    /// Only an exact current record can hit. Different keys are misses; matching
    /// corrupt bytes/guards/holds are errors, never an excuse to trust fresh labels.
    pub async fn lookup(&self) -> Result<bool, ModelError> {
        self.current().await?;
        // Internal validation remains independently observed, but its operation-local
        // selector cannot own a durable whole-input receipt.
        if self.caps.iter().any(|cap| cap.is_internal()) {
            return Ok(false);
        }
        let address = {
            let _identity = serialization_workspace(
                &self.key,
                &self.budget,
                "native premise identity workspace",
            )?;
            self.key.record()?
        };
        let Some(expected) = inspect_expected_read(&self.reader, address, &self.budget).await?
        else {
            return Ok(false);
        };
        let _decode = self.budget.reserve(
            "native premise receipt decoding",
            crate::loader::native_bytes(expected.row())
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let receipt = Receipt::decode(expected.row())?;
        if receipt.key != self.key {
            return Err(ModelError::Conflict(
                "native premise key collision/corruption",
            ));
        }
        self.key.check_proof(&receipt.proof, &self.model)?;
        for physical in &self.key.physical {
            crate::completed_state::require_hold(
                &self.reader,
                &physical.retained()?.0.record()?,
                expected.record(),
            )
            .await?;
        }
        // The scope already protects every source. Hold the exact receipt too,
        // using an expected-incarnation transaction before it grants reuse.
        let pin = self
            .reader
            .scope_pin()
            .ok_or(ModelError::Conflict("native premise receiving pin missing"))?;
        super::live_view::transfer_expected(&pin, &expected, &self.budget).await?;
        self.current().await?;
        Ok(true)
    }
    /// Independently verify the exact typed rows delivered to the fixed driver.
    pub async fn visit(&mut self, index: usize, batch: &RecordBatch) -> Result<(), ModelError> {
        let result = self.visit_inner(index, batch).await;
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    async fn visit_inner(&mut self, index: usize, batch: &RecordBatch) -> Result<(), ModelError> {
        if self.failed || index != self.current {
            return Err(ModelError::Conflict("native observed premise input order"));
        }
        let state = self
            .observed
            .get(index)
            .ok_or(ModelError::Schema("native observed premise input index"))?;
        let _scratch = self.budget.reserve(
            "native observed premise typed window",
            d::logical_batch_bytes(batch)?
                .saturating_mul(32)
                .saturating_add(65536),
        )?;
        let canonical = state.relation.ordered_canonical(batch)?;
        let fingerprints = state.relation.row_fingerprints(&canonical)?;
        let bodies = crate::codec::batch_bodies(&state.relation, &canonical)?;
        let cap = self
            .caps
            .iter()
            .find(|cap| cap.completion().view.identity == self.key.inputs[index].source.view())
            .ok_or(ModelError::Conflict("native observed exact cap missing"))?;
        for (keys, bodies) in fingerprints
            .chunks(crate::loader::NATIVE_WINDOW_ROWS)
            .zip(bodies.chunks(crate::loader::NATIVE_WINDOW_ROWS))
        {
            let mut demand = keys
                .iter()
                .map(|fingerprint| fingerprint.nominal)
                .collect::<Vec<_>>();
            demand.sort_unstable();
            demand.dedup();
            if demand.len() != keys.len() {
                return Err(ModelError::Conflict("native observed duplicate typed key"));
            }
            let members = cap.members(&self.reader, &demand, &self.budget).await?;
            if members.rows().len() != keys.len() {
                return Err(ModelError::Conflict(
                    "native observed typed row absent from exact view",
                ));
            }
            let mut variables = Variables::new();
            variables.insert(
                "nodes",
                members
                    .rows()
                    .iter()
                    .map(|member| member.node.clone())
                    .collect::<Vec<_>>(),
            );
            let actual: Vec<Object> = self
                .reader
                .query_native(
                    "SELECT id,semantic_type,semantic_key,body,content FROM $nodes",
                    variables,
                )
                .await?;
            let _backing = self.budget.reserve(
                "native observed premise backing retention",
                actual
                    .iter()
                    .flat_map(|row| row.iter())
                    .fold(0usize, |bytes, (key, value)| {
                        bytes
                            .saturating_add(key.len())
                            .saturating_add(crate::loader::native_bytes(value))
                    })
                    .saturating_mul(4),
            )?;
            if actual.len() != keys.len() {
                return Err(ModelError::Conflict(
                    "native observed exact backing missing/extra",
                ));
            }
            for (fingerprint, body) in keys.iter().zip(bodies) {
                let key = &fingerprint.nominal;
                let member = members
                    .rows()
                    .iter()
                    .find(|member| member.key == *key)
                    .ok_or(ModelError::Conflict(
                        "native observed selected member missing",
                    ))?;
                let rows = actual
                    .iter()
                    .filter(|row| row.get("id") == Some(&Value::RecordId(member.node.clone())))
                    .collect::<Vec<_>>();
                let [row] = rows.as_slice() else {
                    return Err(ModelError::Conflict(
                        "native observed selected backing ambiguity",
                    ));
                };
                if row.get("semantic_type") != Some(&Value::String(state.relation.name().into()))
                    || row.get("semantic_key") != Some(&Value::String(hex::encode(key)))
                    || row.get("content") != Some(&Value::String(member.content.hex()))
                    || row.get("body") != Some(body)
                {
                    return Err(ModelError::Conflict(
                        "native observed typed payload differs from protected backing",
                    ));
                }
            }
        }
        let state = &mut self.observed[index];
        for fingerprint in fingerprints {
            let key = fingerprint.nominal;
            let content = fingerprint.content;
            let sink = state.sink.as_mut().ok_or(ModelError::Conflict(
                "native observed premise already consumed",
            ))?;
            sink.part(b"nominal", &key);
            content.encode(sink);
            state.rows = state
                .rows
                .checked_add(1)
                .ok_or(ModelError::Schema("native observed premise row count"))?;
        }
        if state.rows > self.key.inputs[index].source.rows() as u64 {
            return Err(ModelError::Conflict(
                "native observed premise exceeds selected count",
            ));
        }
        Ok(())
    }
    pub fn end_input(&mut self, index: usize) -> Result<(), ModelError> {
        if self.failed
            || index != self.current
            || self.observed.get(index).is_none_or(|state| {
                state.ended || state.rows != self.key.inputs[index].source.rows() as u64
            })
        {
            self.failed = true;
            return Err(ModelError::Conflict(
                "native observed premise incomplete EOF/count",
            ));
        }
        self.observed[index].ended = true;
        self.current += 1;
        Ok(())
    }
    pub async fn persist(&mut self, proof: &CheckedValidationPremise) -> Result<(), ModelError> {
        if self.failed || self.current != self.observed.len() {
            return Err(ModelError::Conflict(
                "native premise unexecuted/incomplete physical ledger",
            ));
        }
        let _proof_validation = serialization_workspace(
            proof.data(),
            &self.budget,
            "native premise proof validation workspace",
        )?;
        self.key.check_proof(proof.data(), &self.model)?;
        for (state, input) in self.observed.iter_mut().zip(&proof.data().inputs) {
            let mut sink = state.sink.take().ok_or(ModelError::Conflict(
                "native premise ledger already consumed",
            ))?;
            sink.part(b"rows", &state.rows.to_le_bytes());
            if sink.finish() != input.ordered_content || state.rows != input.rows {
                return Err(ModelError::Conflict(
                    "native premise semantic/physical observed ledger mismatch",
                ));
            }
        }
        drop(_proof_validation);
        self.current().await?;
        if self.caps.iter().any(|cap| cap.is_internal()) {
            return Ok(());
        }
        let _construction = serialization_workspace(
            &(&self.key, proof.data()),
            &self.budget,
            "native premise publication",
        )?;
        let receipt = Receipt {
            key: self.key.clone(),
            proof: proof.data().clone(),
        };
        let row = receipt.value()?;
        let (insert, mut variables) = immutable_row_plan(None, None, vec![row])?
            .ok_or(ModelError::Schema("native premise immutable plan"))?;
        let references = self
            .key
            .physical
            .iter()
            .map(|physical| Ok((physical.retained()?.0.record()?, self.key.record()?)))
            .collect::<Result<Vec<_>, ModelError>>()?;
        let (_, holds, bindings) =
            hold_plan(None, references)?.ok_or(ModelError::Schema("native premise owner plan"))?;
        merge_plan_bindings(&mut variables, bindings)?;
        let mut expected = Vec::new();
        for cap in &self.caps {
            let mut row = Object::new();
            row.insert("record", cap.retained_completion()?.record()?);
            row.insert(
                "guard",
                guard_id(&Value::RecordId(cap.retained_completion()?.record()?))?,
            );
            row.insert("incarnation", cap.completion_incarnation());
            row.insert("bytes", cap.retained_completion()?.value()?);
            expected.push(Value::Object(row));
        }
        variables.insert("premise_inputs", expected);
        variables.insert(
            "premise_generation",
            self.key.physical[0].retained()?.0.generation.hex(),
        );
        variables.insert(
            "premise_validity",
            self.key.physical[0].retained()?.0.validity_epoch,
        );
        let sql = format!(
            "IF $__generation!=$premise_generation OR $__installation.validity_epoch!=$premise_validity {{ THROW 'native premise installation changed'; }}; FOR $input IN $premise_inputs {{ LET $guard_id=$input.guard; LET $guard=SELECT * FROM ONLY $guard_id FOR UPDATE; IF $guard.phase!='active' OR $guard.incarnation!=$input.incarnation OR (SELECT * FROM ONLY $input.record)!=$input.bytes {{ THROW 'native premise protected input changed'; }}; }}; {insert}; {holds}"
        );
        with_recovery_scope(
            self.reader.recovery_scope()?,
            effect_for_owner(
                self.reader.client(),
                EffectOwner::Installation,
                &sql,
                variables,
            ),
        )
        .await?;
        self.current().await?;
        Ok(())
    }
    pub async fn close(&mut self) -> Result<(), ModelError> {
        let result = self.reader.close().await;
        self.closed = result
            .as_ref()
            .err()
            .is_none_or(ModelError::permits_storage_cleanup);
        result
    }
}
pub const fn schema_target() -> &'static str {
    "DEFINE TABLE native_validation_premise SCHEMAFULL; DEFINE FIELD descriptor ON native_validation_premise TYPE bytes;"
}

#[cfg(test)]
mod controls {
    use super::*;
    use d::{
        Invariant, InvariantCheck, InvariantPurpose, ValidationDefinitions, ValidationInput,
        checked_admission::FixedValidationDriver, completed::CompletedView, input::Package,
    };
    struct Check;
    impl InvariantCheck for Check {
        fn visit(&mut self, _: &str, batch: &RecordBatch) -> Result<(), ModelError> {
            Package::decode(batch)?;
            Ok(())
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> {
            Ok(())
        }
    }
    fn fixture() -> (ScopeKey, ValidatedModel, CheckedValidationPremise) {
        let invariant = Invariant {
            purpose: InvariantPurpose::Admission,
            revision: 1,
            name: "native_durable_key_control",
            inputs: vec![ValidationInput::of::<Package>(&["id"])],
            create: Arc::new(|_| Box::new(Check)),
        };
        let relation = Relation::of::<Package>();
        let mut model = ValidatedModel::validate(
            vec![relation.clone()],
            ValidationDefinitions {
                invariants: vec![invariant.clone()],
                publication_checks: vec![],
            },
        )
        .unwrap();
        model
            .register_factory(
                invariant.identity(),
                ContentHash::of(b"key-control executable"),
                vec![],
            )
            .unwrap();
        let view = CompletedView::new(
            Package::NAME.into(),
            [ContentHash::of(
                b"logical producer implementation and config",
            )]
            .into(),
            0,
        )
        .unwrap();
        let source = SourceSnapshot::of_completed_view(&relation, model.digest(), &view).unwrap();
        let mappings = vec![crate::installed_view::ContributorRealization {
            view: view.identity,
            logical: *view.contributions.first().unwrap(),
            physical: ContentHash::of(b"physical copy"),
            incarnation: 2,
        }];
        let completion = InstalledViewCompletion {
            generation: ContentHash::of(b"installation"),
            validity_epoch: 3,
            version: 2,
            parent_completion: ContentHash::of(b"compound boundary"),
            selection: std::sync::Arc::new(crate::installed_view::SelectedViewDescriptor {
                view,
                mappings,
            }),
            member_codec: crate::installed_view::MEMBER_CODEC,
            membership_rows: 0,
            membership_content: ContentHash::of(b"explicit empty members"),
        };
        let contract = model.factory_contract(invariant.identity()).unwrap();
        let key = ScopeKey {
            version: 3,
            model: model.digest(),
            profile: Profile::Catalog,
            name: invariant.name.into(),
            revision: 1,
            definition: invariant.digest(),
            factory: contract.identity().unwrap(),
            schema: ContentHash::of(b"physical schema"),
            contracts: contract.inputs.clone(),
            inputs: vec![SelectedInput {
                source: source.clone(),
                requested: None,
                resolved: None,
            }],
            uniform: vec![],
            physical: vec![PhysicalInput::Retained {
                completion,
                incarnation: 6,
            }],
        };
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut driver = FixedValidationDriver::invariant(
            &model,
            invariant.identity(),
            vec![(source, None, None)],
            &[],
            &budget,
        )
        .unwrap();
        driver.end_input(0).unwrap();
        let proof = driver.finish().unwrap();
        (key, model, proof)
    }
    #[test]
    fn durable_scope_separates_executable_selectors_profile_and_physical_validity() {
        let (key, _, _) = fixture();
        let identity = key.identity().unwrap();
        for field in [
            "factory",
            "definition",
            "schema",
            "model",
            "profile",
            "requested",
            "resolved",
            "logical",
            "generation",
            "epoch",
            "completion_incarnation",
            "parent_completion",
            "member_codec",
            "copy_incarnation",
            "copy",
        ] {
            let mut changed = key.clone();
            let other = ContentHash::of(b"different current premise");
            let PhysicalInput::Retained {
                completion,
                incarnation,
            } = &mut changed.physical[0]
            else {
                unreachable!()
            };
            match field {
                "factory" => changed.factory = other,
                "definition" => changed.definition = other,
                "schema" => changed.schema = other,
                "model" => changed.model = other,
                "profile" => changed.profile = Profile::Behavioral,
                "requested" => changed.inputs[0].requested = Some(PublicationBoundary::Local),
                "resolved" => changed.inputs[0].resolved = Some(PublicationBoundary::Local),
                "logical" => {
                    Arc::make_mut(&mut completion.selection).view.contributions = [other].into()
                }
                "generation" => completion.generation = other,
                "epoch" => completion.validity_epoch += 1,
                "completion_incarnation" => *incarnation += 1,
                "parent_completion" => completion.parent_completion = other,
                "member_codec" => completion.member_codec += 1,
                "copy_incarnation" => {
                    Arc::make_mut(&mut completion.selection).mappings[0].incarnation += 1
                }
                _ => Arc::make_mut(&mut completion.selection).mappings[0].physical = other,
            }
            assert_ne!(
                identity,
                changed.identity().unwrap(),
                "old durable success cannot match {field}"
            );
        }
    }
    #[test]
    fn durable_receipt_rejects_corrupt_envelope_proof_version_and_role() {
        let (key, model, proof) = fixture();
        key.check_proof(proof.data(), &model).unwrap();
        let receipt = Receipt {
            key: key.clone(),
            proof: proof.data().clone(),
        };
        let row = receipt.value().unwrap();
        assert_eq!(Receipt::decode(&row).unwrap(), receipt);
        let Value::Object(mut corrupt) = row else {
            unreachable!()
        };
        corrupt.insert("unexpected", true);
        assert!(Receipt::decode(&Value::Object(corrupt)).is_err());
        let mut wrong = proof.data().clone();
        wrong.version += 1;
        assert!(key.check_proof(&wrong, &model).is_err());
        let mut wrong = proof.data().clone();
        wrong.inputs[0].resolved = Some(PublicationBoundary::Local);
        assert!(key.check_proof(&wrong, &model).is_err());
        let mut wrong = proof.data().clone();
        wrong.inputs[0].rows += 1;
        assert!(key.check_proof(&wrong, &model).is_err());
    }
}
