//! Executed validation premises. Serializable data is evidence only; the opaque owner
//! is created by this fixed driver after actual typed visits, checked ordering and EOF.
use super::{
    self as d, ContentHash, Key, KeySink, ModelError, Relation, ValidatedModel, ValidationIdentity,
    ValidationInput,
};
use arrow_array::{Array, RecordBatch};
use arrow_row::{RowConverter, SortField};
use arrow_schema::SortOptions;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPremise {
    pub contract: ContentHash,
    pub requested: Option<d::stages::PublicationBoundary>,
    pub resolved: Option<d::stages::PublicationBoundary>,
    pub source: d::analysis::sources::SourceSnapshot,
    pub rows: u64,
    /// Declared input order, nominal identity and full typed content. Batch framing is absent.
    pub ordered_content: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniformOwnerPremise {
    pub contribution: ContentHash,
    pub descriptor: [u8; 16],
    pub content: ContentHash,
    pub profile: d::stages::Profile,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationPremise {
    pub version: u32,
    pub kind: u8,
    pub name: String,
    pub revision: u32,
    pub model: ContentHash,
    pub factory: ContentHash,
    pub definition: ContentHash,
    pub inputs: Vec<InputPremise>,
    pub uniform_owners: Vec<UniformOwnerPremise>,
    pub profile: Option<d::stages::Profile>,
}
impl ValidationPremise {
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        if self.version != 1 || self.kind > 1 || self.name.is_empty() || self.revision == 0 {
            return Err(ModelError::Conflict(
                "executed validation premise structure",
            ));
        }
        let mut sink = KeySink::new("executed-validation-premise/v1");
        sink.part(
            b"canonical",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
    /// Current executable eligibility, not proof that these deserialized visits happened.
    pub fn eligible(&self, model: &ValidatedModel) -> Result<bool, ModelError> {
        if self.model != model.digest()
            || (self.kind == 0 && self.profile.is_some())
            || (self.kind == 1 && self.profile.is_none())
        {
            return Ok(false);
        }
        let identity = match self.kind {
            0 => model.invariant(&self.name)?.identity(),
            1 => model.publication_check(&self.name)?.identity(),
            _ => return Err(ModelError::Conflict("executed premise validation kind")),
        };
        if identity.revision != self.revision {
            return Ok(false);
        }
        let Some(factory) = model.factory_contract(identity) else {
            return Ok(false);
        };
        Ok(factory.identity()? == self.factory
            && factory.definition == self.definition
            && factory
                .inputs
                .iter()
                .copied()
                .eq(self.inputs.iter().map(|input| input.contract)))
    }
}
/// Pure executed semantic evidence. It grants no persistence or selected-view authority.
/// Native admission independently matches every input fingerprint/current physical witness.
pub struct CheckedValidationPremise {
    data: ValidationPremise,
    _charge: d::charged::StateCharge,
}
impl CheckedValidationPremise {
    pub fn data(&self) -> &ValidationPremise {
        &self.data
    }
}

struct InputState {
    declared: ValidationInput,
    relation: Relation,
    source: d::analysis::sources::SourceSnapshot,
    requested: Option<d::stages::PublicationBoundary>,
    resolved: Option<d::stages::PublicationBoundary>,
    rows: u64,
    sink: KeySink,
    previous: Option<Vec<u8>>,
    ended: bool,
}

enum ExecutedCheck {
    Invariant(Box<dyn d::InvariantCheck>),
    Publication(Box<dyn d::PublicationCheck>, d::stages::Profile),
}
impl ExecutedCheck {
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &RecordBatch,
    ) -> Result<(), ModelError> {
        match self {
            Self::Invariant(check) => check.visit_input(input, batch),
            Self::Publication(check, _) => check.visit_input(input, batch),
        }
    }
    fn finish(self, inputs: &[InputState]) -> Result<(), ModelError> {
        match self {
            Self::Invariant(check) => check.finish(),
            Self::Publication(check, profile) => {
                let sources = inputs
                    .iter()
                    .map(|input| input.source.clone())
                    .collect::<Vec<_>>();
                check.finish(&sources, profile)
            }
        }
    }
}

/// A fixed whole-input validator driver. A scoped compiler kernel must earn its own
/// exhaustive root ledger before it can use an equivalent executed-premise constructor;
/// a successful callback, a memo hit or a deserialized premise cannot construct this owner.
pub struct FixedValidationDriver {
    check: ExecutedCheck,
    identity: ValidationIdentity,
    model: ContentHash,
    factory: ContentHash,
    definition: ContentHash,
    inputs: Vec<InputState>,
    uniform_owners: Vec<UniformOwnerPremise>,
    current: usize,
    profile: Option<d::stages::Profile>,
    budget: d::resources::ResourceBudget,
    charge: d::charged::StateCharge,
    failed: bool,
}
impl FixedValidationDriver {
    pub fn invariant(
        model: &ValidatedModel,
        identity: ValidationIdentity,
        sources: Vec<(
            d::analysis::sources::SourceSnapshot,
            Option<d::stages::PublicationBoundary>,
            Option<d::stages::PublicationBoundary>,
        )>,
        uniform: &[d::uniform_outcome::ValidatedUniformOwner],
        budget: &d::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        if identity.kind != d::ValidationKind::Invariant {
            return Err(ModelError::Conflict("invariant fixed driver kind"));
        }
        let invariant = model.invariant(identity.id)?;
        if invariant.identity() != identity {
            return Err(ModelError::Conflict("invariant fixed driver revision"));
        }
        let contract = model
            .factory_contract(identity)
            .ok_or(ModelError::Conflict(
                "undeclared executable validation factory",
            ))?;
        let mut check = (invariant.create)(budget);
        // These scopes require grouped/exhaustive root controls. Running their factory as
        // an ordinary whole-relation checker would silently change their semantics.
        if check.retrieval_scope().is_some()
            || check.projection_scope().is_some()
            || check.normalization_scope().is_some()
            || check.execution_scope().is_some()
            || check.aspect_scope().is_some()
            || check.inventory_scope().is_some()
            || check.support_scope().is_some()
        {
            return Err(ModelError::Conflict(
                "scoped validation requires executed root ledger",
            ));
        }
        let _uniform_scratch = budget.reserve(
            "fixed driver uniform premise construction",
            uniform.len().saturating_mul(512),
        )?;
        let mut uniform_owners = Vec::with_capacity(uniform.len());
        for owner in uniform {
            check.admit_uniform_owner(owner)?;
            uniform_owners.push(UniformOwnerPremise {
                contribution: owner.contribution(),
                descriptor: *d::Record::id(owner.descriptor()).bytes(),
                content: d::Record::content_digest(owner.descriptor()),
                profile: owner.profile(),
            });
        }
        uniform_owners.sort_by_key(|owner| (owner.contribution, owner.descriptor));
        if uniform_owners.windows(2).any(|pair| {
            (pair[0].contribution, pair[0].descriptor) == (pair[1].contribution, pair[1].descriptor)
        }) {
            return Err(ModelError::Conflict(
                "fixed driver duplicate uniform premise owner",
            ));
        }
        if sources.len() != invariant.inputs.len() {
            return Err(ModelError::Conflict(
                "fixed driver ordered input cardinality",
            ));
        }
        let mut charge = d::charged::StateCharge::new(budget, "executed validation premise");
        charge.grow(
            uniform_owners
                .capacity()
                .saturating_mul(std::mem::size_of::<UniformOwnerPremise>()),
        )?;
        charge.grow(
            sources
                .iter()
                .map(|(source, _, _)| {
                    d::HeapSize::heap_bytes(source)
                        .saturating_mul(4)
                        .saturating_add(1024)
                })
                .sum(),
        )?;
        let mut inputs = Vec::with_capacity(sources.len());
        for (declared, (source, requested, resolved)) in invariant.inputs.iter().zip(sources) {
            if source.relation() != declared.name()
                || source.model() != model.digest()
                || source.rows() < 0
                || requested != declared.prefix()
            {
                return Err(ModelError::Conflict("fixed driver selected input binding"));
            }
            let relation = model
                .relation(declared.name())
                .ok_or(ModelError::Schema(declared.name()))?
                .clone();
            let mut sink = KeySink::new("validation-ordered-typed-input/v1");
            declared.encode_contract(&mut sink);
            inputs.push(InputState {
                declared: declared.clone(),
                relation,
                source,
                requested,
                resolved,
                rows: 0,
                sink,
                previous: None,
                ended: false,
            });
        }
        Ok(Self {
            check: ExecutedCheck::Invariant(check),
            identity,
            model: model.digest(),
            factory: contract.identity()?,
            definition: contract.definition,
            inputs,
            uniform_owners,
            current: 0,
            profile: None,
            budget: budget.clone(),
            charge,
            failed: false,
        })
    }
    pub fn publication(
        model: &ValidatedModel,
        identity: ValidationIdentity,
        sources: Vec<(
            d::analysis::sources::SourceSnapshot,
            Option<d::stages::PublicationBoundary>,
            Option<d::stages::PublicationBoundary>,
        )>,
        uniform: &[d::uniform_outcome::ValidatedUniformOwner],
        profile: d::stages::Profile,
        budget: &d::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        if identity.kind != d::ValidationKind::Publication {
            return Err(ModelError::Conflict("publication fixed driver kind"));
        }
        let declared = model.publication_check(identity.id)?;
        if declared.identity() != identity {
            return Err(ModelError::Conflict("publication fixed driver revision"));
        }
        let contract = model
            .factory_contract(identity)
            .ok_or(ModelError::Conflict(
                "undeclared executable publication factory",
            ))?;
        let mut check = (declared.create)(budget);
        let mut charge = d::charged::StateCharge::new(budget, "executed publication premise");
        charge.grow(
            uniform
                .len()
                .saturating_mul(512)
                .saturating_add(
                    sources
                        .len()
                        .saturating_mul(std::mem::size_of::<InputState>()),
                )
                .saturating_add(
                    sources
                        .iter()
                        .map(|(source, _, _)| {
                            d::HeapSize::heap_bytes(source)
                                .saturating_mul(4)
                                .saturating_add(1024)
                        })
                        .sum::<usize>(),
                ),
        )?;
        let mut uniform_owners = Vec::with_capacity(uniform.len());
        for owner in uniform {
            check.admit_uniform_owner(owner)?;
            uniform_owners.push(UniformOwnerPremise {
                contribution: owner.contribution(),
                descriptor: *d::Record::id(owner.descriptor()).bytes(),
                content: d::Record::content_digest(owner.descriptor()),
                profile: owner.profile(),
            });
        }
        uniform_owners.sort_by_key(|owner| (owner.contribution, owner.descriptor));
        if uniform_owners.windows(2).any(|pair| {
            (pair[0].contribution, pair[0].descriptor) == (pair[1].contribution, pair[1].descriptor)
        }) {
            return Err(ModelError::Conflict(
                "publication duplicate uniform premise",
            ));
        }
        if sources.len() != declared.inputs.len() {
            return Err(ModelError::Conflict(
                "publication ordered input cardinality",
            ));
        }
        let mut inputs = Vec::with_capacity(sources.len());
        for (declared, (source, requested, resolved)) in declared.inputs.iter().zip(sources) {
            if source.relation() != declared.name()
                || source.model() != model.digest()
                || source.rows() < 0
                || requested != declared.prefix()
            {
                return Err(ModelError::Conflict("publication selected input binding"));
            }
            let relation = model
                .relation(declared.name())
                .ok_or(ModelError::Schema(declared.name()))?
                .clone();
            let mut sink = KeySink::new("validation-ordered-typed-input/v1");
            declared.encode_contract(&mut sink);
            inputs.push(InputState {
                declared: declared.clone(),
                relation,
                source,
                requested,
                resolved,
                rows: 0,
                sink,
                previous: None,
                ended: false,
            });
        }
        Ok(Self {
            check: ExecutedCheck::Publication(check, profile),
            identity,
            model: model.digest(),
            factory: contract.identity()?,
            definition: contract.definition,
            inputs,
            uniform_owners,
            current: 0,
            profile: Some(profile),
            budget: budget.clone(),
            charge,
            failed: false,
        })
    }
    pub fn visit(&mut self, index: usize, batch: &RecordBatch) -> Result<(), ModelError> {
        if self.failed {
            return Err(ModelError::Conflict("failed fixed validation driver"));
        }
        let result = self.visit_inner(index, batch);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn visit_inner(&mut self, index: usize, batch: &RecordBatch) -> Result<(), ModelError> {
        if index != self.current {
            return Err(ModelError::Conflict("fixed driver input role order"));
        }
        let input = self
            .inputs
            .get_mut(index)
            .ok_or(ModelError::Conflict("fixed driver input index"))?;
        if input.ended {
            return Err(ModelError::Conflict("fixed driver input already ended"));
        }
        let bytes = d::logical_batch_bytes(batch)?;
        let _scratch = self.budget.reserve(
            "fixed validator typed input and order scratch",
            bytes.saturating_mul(32).saturating_add(65536),
        )?;
        let canonical = input.relation.ordered_canonical(batch)?;
        let arrays = input
            .declared
            .order()
            .iter()
            .map(|name| {
                canonical
                    .column_by_name(name)
                    .cloned()
                    .ok_or(ModelError::Schema("fixed driver declared order field"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let options = SortOptions {
            descending: false,
            nulls_first: false,
        };
        let converter = RowConverter::new(
            arrays
                .iter()
                .map(|array| SortField::new_with_options(array.data_type().clone(), options))
                .collect(),
        )
        .map_err(ModelError::codec)?;
        let order = converter
            .convert_columns(&arrays)
            .map_err(ModelError::codec)?;
        let fingerprints = input.relation.row_fingerprints(&canonical)?;
        for (position, fingerprint) in fingerprints.into_iter().enumerate() {
            let next = order.row(position);
            if input
                .previous
                .as_ref()
                .is_some_and(|previous| previous.as_slice() >= next.as_ref())
            {
                return Err(ModelError::Conflict("fixed driver input order/duplicate"));
            }
            // Retained last ordering tuple stays charged across batch boundaries.
            self.charge.grow(next.as_ref().len())?;
            let owned = next.as_ref().to_vec();
            let old = input.previous.replace(owned);
            if let Some(old) = old {
                self.charge.release(old.len());
            }
            input.sink.part(b"nominal", &fingerprint.nominal);
            fingerprint.content.encode(&mut input.sink);
            input.rows = input
                .rows
                .checked_add(1)
                .ok_or(ModelError::Schema("fixed driver input count"))?;
        }
        self.check.visit_input(&input.declared, &canonical)
    }
    pub fn end_input(&mut self, index: usize) -> Result<(), ModelError> {
        if self.failed || index != self.current {
            self.failed = true;
            return Err(ModelError::Conflict("fixed driver input EOF order"));
        }
        let Some(input) = self.inputs.get_mut(index) else {
            self.failed = true;
            return Err(ModelError::Conflict("fixed driver input EOF index"));
        };
        if input.ended || input.rows != input.source.rows() as u64 {
            self.failed = true;
            return Err(ModelError::Conflict("fixed driver actual input EOF count"));
        }
        input.ended = true;
        self.current += 1;
        Ok(())
    }
    pub fn finish(mut self) -> Result<CheckedValidationPremise, ModelError> {
        if self.failed || self.current != self.inputs.len() {
            return Err(ModelError::Conflict(
                "fixed validator incomplete/failed input ledger",
            ));
        }
        let _finish_scratch = self.budget.reserve(
            "fixed publication finish source metadata",
            self.inputs
                .iter()
                .map(|input| {
                    d::HeapSize::heap_bytes(&input.source)
                        .saturating_mul(4)
                        .saturating_add(std::mem::size_of::<d::analysis::sources::SourceSnapshot>())
                })
                .sum(),
        )?;
        self.check.finish(&self.inputs)?;
        // InputState source/order allocations coexist with the final premise vector.
        // Admit that vector separately before allocation and retain this same charge in
        // the result; no ownership claim depends on dropping the driver's old fields.
        self.charge.grow(
            self.inputs
                .len()
                .saturating_mul(std::mem::size_of::<InputPremise>())
                .saturating_add(self.identity.id.len())
                .saturating_add(std::mem::size_of::<ValidationPremise>()),
        )?;
        let mut inputs = Vec::with_capacity(self.inputs.len());
        for mut input in self.inputs {
            let mut sink = KeySink::new("validation-factory-input/v1");
            input.declared.encode_contract(&mut sink);
            input.sink.part(b"rows", &input.rows.to_le_bytes());
            inputs.push(InputPremise {
                contract: sink.finish(),
                requested: input.requested,
                resolved: input.resolved,
                source: input.source,
                rows: input.rows,
                ordered_content: input.sink.finish(),
            });
        }
        let profile = self.profile;
        let data = ValidationPremise {
            version: 1,
            profile,
            kind: self.identity.kind.code(),
            name: self.identity.id.into(),
            revision: self.identity.revision,
            model: self.model,
            factory: self.factory,
            definition: self.definition,
            inputs,
            uniform_owners: self.uniform_owners,
        };
        data.identity()?;
        Ok(CheckedValidationPremise {
            data,
            _charge: self.charge,
        })
    }
}

/// Persisted evidence is data. Only `CheckedAdmissionPremises::full` can wrap actual
/// executed owners, and native storage must independently match all exact selected inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionPremises {
    pub version: u32,
    pub model: ContentHash,
    pub profile: d::stages::Profile,
    pub validations: Vec<ValidationPremise>,
}
impl AdmissionPremises {
    pub fn validate_current(&self, model: &ValidatedModel) -> Result<(), ModelError> {
        if self.version != 1 || self.model != model.digest() {
            return Err(ModelError::Conflict(
                "admission premise current model/version",
            ));
        }
        let mut required = model
            .invariants()
            .iter()
            .filter(|check| check.purpose == d::InvariantPurpose::Admission)
            .map(|check| (0u8, check.name, check.revision))
            .chain(
                model
                    .publication_checks()
                    .iter()
                    .map(|check| (1u8, check.name, check.revision)),
            )
            .collect::<Vec<_>>();
        required.sort_unstable();
        if required.len() != self.validations.len()
            || !required
                .iter()
                .zip(&self.validations)
                .all(|((kind, name, revision), proof)| {
                    *kind == proof.kind && *name == proof.name && *revision == proof.revision
                })
        {
            return Err(ModelError::Conflict(
                "admission exact required check ledger (diagnostics cannot substitute)",
            ));
        }
        for proof in &self.validations {
            proof.identity()?;
            if !proof.eligible(model)? || (proof.kind == 1 && proof.profile != Some(self.profile)) {
                return Err(ModelError::Conflict(
                    "admission executable premise ineligible",
                ));
            }
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<ContentHash, ModelError> {
        let mut sink = KeySink::new("executed-full-admission-premises/v1");
        sink.part(
            b"canonical",
            &serde_json::to_vec(self).map_err(ModelError::codec)?,
        );
        Ok(sink.finish())
    }
}
pub struct CheckedAdmissionPremises {
    data: AdmissionPremises,
    _proofs: Vec<CheckedValidationPremise>,
    _charge: d::charged::StateCharge,
}
impl CheckedAdmissionPremises {
    pub fn data(&self) -> &AdmissionPremises {
        &self.data
    }
    pub fn retained_bytes(&self) -> usize {
        self._charge.reserved()
    }
    /// No serializable assertion, diagnostic replay, callback or registry entry fills a
    /// missing required Admission-purpose invariant or publication obligation.
    pub fn full(
        model: &ValidatedModel,
        profile: d::stages::Profile,
        mut proofs: Vec<CheckedValidationPremise>,
        budget: &d::resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut charge = d::charged::StateCharge::new(budget, "executed full admission ledger");
        charge.grow(
            proofs
                .iter()
                .map(|proof| {
                    proof
                        ._charge
                        .reserved()
                        .saturating_mul(2)
                        .saturating_add(2048)
                })
                .sum::<usize>()
                .saturating_add(
                    proofs
                        .capacity()
                        .saturating_mul(std::mem::size_of::<CheckedValidationPremise>()),
                ),
        )?;
        proofs.sort_by(|left, right| {
            (left.data.kind, &left.data.name, left.data.revision).cmp(&(
                right.data.kind,
                &right.data.name,
                right.data.revision,
            ))
        });
        let data = AdmissionPremises {
            version: 1,
            model: model.digest(),
            profile,
            validations: proofs.iter().map(|proof| proof.data.clone()).collect(),
        };
        data.validate_current(model)?;
        Ok(Self {
            data,
            _proofs: proofs,
            _charge: charge,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use d::Record;
    use std::sync::Arc;
    struct RejectForbidden;
    impl d::InvariantCheck for RejectForbidden {
        fn visit(&mut self, name: &str, batch: &RecordBatch) -> Result<(), ModelError> {
            if name != d::input::Package::NAME {
                return Err(ModelError::Conflict("test validator relation"));
            }
            for row in d::input::Package::decode(batch)? {
                if row.name == "forbidden" {
                    return Err(ModelError::Conflict("forbidden typed row"));
                }
            }
            Ok(())
        }
        fn finish(self: Box<Self>) -> Result<(), ModelError> {
            Ok(())
        }
    }
    fn fixture() -> (
        ValidatedModel,
        ValidationIdentity,
        d::analysis::sources::SourceSnapshot,
        Vec<d::input::Package>,
    ) {
        use d::Record;
        let invariant = d::Invariant {
            purpose: d::InvariantPurpose::Admission,
            revision: 1,
            name: "fixed_driver_control",
            inputs: vec![ValidationInput::of::<d::input::Package>(&["id"])],
            create: Arc::new(|_| Box::new(RejectForbidden)),
        };
        let identity = invariant.identity();
        let relation = Relation::of::<d::input::Package>();
        let mut model = ValidatedModel::validate(
            vec![relation.clone()],
            d::ValidationDefinitions {
                invariants: vec![invariant],
                publication_checks: vec![],
            },
        )
        .unwrap();
        model
            .register_factory(
                identity,
                ContentHash::of(b"controlled implementation"),
                vec![],
            )
            .unwrap();
        let view = d::completed::CompletedView::new(
            d::input::Package::NAME.into(),
            [ContentHash::of(b"completed owner")].into(),
            2,
        )
        .unwrap();
        let source = d::analysis::sources::SourceSnapshot::of_completed_view(
            &relation,
            model.digest(),
            &view,
        )
        .unwrap();
        let mut rows = vec![
            d::input::Package {
                name: "allowed-one".into(),
            },
            d::input::Package {
                name: "allowed-two".into(),
            },
        ];
        rows.sort_by_key(|row| row.id());
        (model, identity, source, rows)
    }
    fn driver(
        model: &ValidatedModel,
        identity: ValidationIdentity,
        source: &d::analysis::sources::SourceSnapshot,
        budget: &d::resources::ResourceBudget,
    ) -> FixedValidationDriver {
        FixedValidationDriver::invariant(
            model,
            identity,
            vec![(source.clone(), None, None)],
            &[],
            budget,
        )
        .unwrap()
    }
    #[test]
    fn fixed_driver_earns_same_full_typed_premise_across_batch_frames_and_keeps_charge() {
        use d::Record;
        let (model, identity, source, rows) = fixture();
        let budget = d::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let mut one = driver(&model, identity, &source, &budget);
        one.visit(0, &d::input::Package::encode(&rows).unwrap())
            .unwrap();
        one.end_input(0).unwrap();
        let one = one.finish().unwrap();
        let mut split = driver(&model, identity, &source, &budget);
        for row in &rows {
            split
                .visit(
                    0,
                    &d::input::Package::encode(std::slice::from_ref(row)).unwrap(),
                )
                .unwrap();
        }
        split.end_input(0).unwrap();
        let split = split.finish().unwrap();
        assert_eq!(
            one.data(),
            split.data(),
            "input framing cannot change executed semantic premise"
        );
        assert!(one.data().eligible(&model).unwrap());
        assert!(
            budget.reserved() > 0,
            "opaque results retain their actual premise allocation authority"
        );
        drop(one);
        assert!(
            budget.reserved() > 0,
            "another result still owns its independent lease"
        );
        drop(split);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn fixed_driver_refuses_missing_eof_duplicate_reordered_and_semantic_failure() {
        use d::Record;
        let (model, identity, source, rows) = fixture();
        let budget = d::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let mut missing = driver(&model, identity, &source, &budget);
        missing
            .visit(0, &d::input::Package::encode(&rows[..1]).unwrap())
            .unwrap();
        assert!(missing.end_input(0).is_err());
        assert!(missing.finish().is_err());
        let mut unfinished = driver(&model, identity, &source, &budget);
        unfinished
            .visit(0, &d::input::Package::encode(&rows).unwrap())
            .unwrap();
        assert!(
            unfinished.finish().is_err(),
            "full observed count is insufficient without EOF"
        );
        for invalid in [
            vec![rows[0].clone(), rows[0].clone()],
            vec![rows[1].clone(), rows[0].clone()],
            vec![d::input::Package {
                name: "forbidden".into(),
            }],
        ] {
            let mut actual = driver(&model, identity, &source, &budget);
            assert!(
                actual
                    .visit(0, &d::input::Package::encode(&invalid).unwrap())
                    .is_err()
            );
            assert!(actual.end_input(0).is_err());
            assert!(
                actual.finish().is_err(),
                "an ignored visit error cannot mint later capability"
            );
        }
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn fixed_driver_preserves_declared_non_id_order_instead_of_repairing_to_storage_order() {
        use d::Record;
        let (old, identity, _, _) = fixture();
        let mut definitions = old.definitions();
        definitions.invariants[0].inputs =
            vec![ValidationInput::of::<d::input::Package>(&["name"])];
        let relation = Relation::of::<d::input::Package>();
        let mut model = ValidatedModel::validate(vec![relation.clone()], definitions).unwrap();
        model
            .register_factory(
                identity,
                ContentHash::of(b"controlled name-ordered implementation"),
                vec![],
            )
            .unwrap();
        let rows = (0..20)
            .map(|number| d::input::Package {
                name: format!("name-{number:02}"),
            })
            .collect::<Vec<_>>();
        assert!(
            rows.windows(2).any(|pair| pair[0].id() > pair[1].id()),
            "control requires name order to differ from hashed-ID storage order"
        );
        let view = d::completed::CompletedView::new(
            d::input::Package::NAME.into(),
            [ContentHash::of(b"name-ordered completed owner")].into(),
            rows.len() as u64,
        )
        .unwrap();
        let source = d::analysis::sources::SourceSnapshot::of_completed_view(
            &relation,
            model.digest(),
            &view,
        )
        .unwrap();
        let budget = d::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let mut actual = driver(&model, identity, &source, &budget);
        for rows in rows.chunks(3) {
            actual
                .visit(0, &d::input::Package::encode(rows).unwrap())
                .unwrap();
        }
        actual.end_input(0).unwrap();
        actual.finish().unwrap();
    }
    #[test]
    fn data_only_replay_and_equal_semantic_model_do_not_grant_current_factory_eligibility() {
        use d::Record;
        let (model, identity, source, rows) = fixture();
        let budget = d::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let mut actual = driver(&model, identity, &source, &budget);
        actual
            .visit(0, &d::input::Package::encode(&rows).unwrap())
            .unwrap();
        actual.end_input(0).unwrap();
        let actual = actual.finish().unwrap();
        let replay: ValidationPremise =
            serde_json::from_slice(&serde_json::to_vec(actual.data()).unwrap()).unwrap();
        assert_eq!(&replay, actual.data());
        let mut opaque =
            ValidatedModel::validate(model.relations().to_vec(), model.definitions()).unwrap();
        assert_eq!(opaque.digest(), model.digest());
        assert!(!replay.eligible(&opaque).unwrap());
        assert!(
            FixedValidationDriver::invariant(
                &opaque,
                identity,
                vec![(source, None, None)],
                &[],
                &budget
            )
            .is_err()
        );
        opaque
            .register_factory(
                identity,
                ContentHash::of(b"replacement implementation"),
                vec![],
            )
            .unwrap();
        assert!(
            !replay.eligible(&opaque).unwrap(),
            "same definition cannot admit stale executable proof"
        );
    }
    #[test]
    fn full_admission_requires_exact_executed_obligations_and_keeps_result_charge() {
        let (model, identity, source, rows) = fixture();
        let budget = d::resources::ResourceBudget::fixed(16 << 20).unwrap();
        let run = || {
            let mut check = driver(&model, identity, &source, &budget);
            check
                .visit(0, &d::input::Package::encode(&rows).unwrap())
                .unwrap();
            check.end_input(0).unwrap();
            check.finish().unwrap()
        };
        assert!(
            CheckedAdmissionPremises::full(&model, d::stages::Profile::Catalog, vec![], &budget)
                .is_err(),
            "missing required admission is not filled by registry membership"
        );
        assert!(
            CheckedAdmissionPremises::full(
                &model,
                d::stages::Profile::Catalog,
                vec![run(), run()],
                &budget
            )
            .is_err(),
            "duplicate executed receipts cannot stand in for the exact ledger"
        );
        let admitted = CheckedAdmissionPremises::full(
            &model,
            d::stages::Profile::Catalog,
            vec![run()],
            &budget,
        )
        .unwrap();
        let replay: AdmissionPremises =
            serde_json::from_slice(&serde_json::to_vec(admitted.data()).unwrap()).unwrap();
        replay.validate_current(&model).unwrap();
        assert_eq!(
            replay,
            *admitted.data(),
            "serialized evidence keeps meaning but has no opaque owner constructor"
        );
        assert!(
            budget.reserved() > 0,
            "aggregate retains executed premises and its separately allocated ledger"
        );
        drop(admitted);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn diagnostic_execution_never_substitutes_for_admission() {
        let (_, _, source, rows) = fixture();
        let relation = Relation::of::<d::input::Package>();
        let diagnostic = d::Invariant {
            purpose: d::InvariantPurpose::DiagnosticReplay,
            revision: 1,
            name: "fixed_driver_control",
            inputs: vec![ValidationInput::of::<d::input::Package>(&["id"])],
            create: Arc::new(|_| Box::new(RejectForbidden)),
        };
        let identity = diagnostic.identity();
        let mut model = ValidatedModel::validate(
            vec![relation.clone()],
            d::ValidationDefinitions {
                invariants: vec![diagnostic],
                publication_checks: vec![],
            },
        )
        .unwrap();
        model
            .register_factory(identity, ContentHash::of(b"diagnostic executable"), vec![])
            .unwrap();
        let view = d::completed::CompletedView::new(
            relation.name().into(),
            [ContentHash::of(b"completed owner")].into(),
            source.rows() as u64,
        )
        .unwrap();
        let source = d::analysis::sources::SourceSnapshot::of_completed_view(
            &relation,
            model.digest(),
            &view,
        )
        .unwrap();
        let budget = d::resources::ResourceBudget::fixed(8 << 20).unwrap();
        let mut run = driver(&model, identity, &source, &budget);
        run.visit(0, &d::input::Package::encode(&rows).unwrap())
            .unwrap();
        run.end_input(0).unwrap();
        let evidence = run.finish().unwrap();
        assert!(
            CheckedAdmissionPremises::full(
                &model,
                d::stages::Profile::Catalog,
                vec![evidence],
                &budget
            )
            .is_err(),
            "successful diagnostic is extra evidence, never an Admission-purpose obligation"
        );
    }
}
