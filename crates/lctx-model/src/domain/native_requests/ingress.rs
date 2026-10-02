//! Proof ingress reuses canonical Entry, callable and rebase owners. Stored rows must first
//! pass their declared generation invariants; opaque Entry/rebase results are replayed here.
use super::*;
use crate::domain::{
    ModelError, Record, HeapSize, KeySink,
    analysis::local,
    assertion::AssertionQualification,
    conditions::{Diagram, EvaluationAtom, entry::{DerivedEntryValue, EntryAccessSource, EntryData}, rebase::RebasedGuards},
    local_theory::{CheckedBuiltinOperand, TheoryData},
    normalized::{Rows, callables::*, entities::*},
    resources::{ResourceBudget, Reservation},
    transfer::{TransferDescriptor, TransferKind, TransferKeyRecord},
    value::*,
};

/// Preparation supplies the confirmed canonical callable/variant/slot domain. This constructor
/// checks its relationship to the independently replayed Entry proof; no validated boolean.
pub struct NativeContext {
    pub(super) owner: Id<EntityRef>,
    pub(super) formal: Id<ParameterEntity>,
    pub(super) context: Id<crate::domain::attribution::AnalysisContext>,
    pub(super) place: Id<Place>,
    pub(super) proof: [RowRef; 4],
}
impl NativeContext {
    pub fn from_entry(
        entry: &DerivedEntryValue,
        data: &EntryData,
        assessment: &EffectiveCallableAssessment,
        variant: &SignatureVariant,
        slot: &SignatureSlot,
        decorators: &Rows<EffectiveDecoratorMember>,
    ) -> Result<Self, ObligationKind> {
        let witness = entry.witness();
        let entity = data.refs.get(witness.owner).ok_or(ObligationKind::MissingEvidence)?;
        let EntityRef::Callable { callable } = entity else { return Err(ObligationKind::EntryValueUnknown); };
        let Some(CallableEntity::Source { declaration, kind: CallableKind::Function }) = data.callables.get(*callable)
        else { return Err(ObligationKind::EntryValueUnknown); };
        let source = data.occurrences.get(*declaration).and_then(|o| data.artifacts.get(o.source))
            .ok_or(ObligationKind::MissingEvidence)?;
        let empty_decorators = KeySink::new("effective-decorator-chain").finish();
        if source.path.ends_with(".pyi") || assessment.validate().is_err()
            || assessment.callable != *callable || assessment.context != witness.context
            || assessment.identity != Knowledge::Known || assessment.signatures != Knowledge::Known
            || !assessment.body_admitted || assessment.body != Knowledge::Known
            || assessment.decorators != empty_decorators
            || decorators.iter().any(|d| d.assessment == assessment.id())
            || !matches!(assessment.descriptor_kind, Some(DescriptorKind::Function | DescriptorKind::InstanceMethod)) {
            return Err(ObligationKind::EntryValueUnknown);
        }
        if slot.default != DefaultSlot::Required { return Err(ObligationKind::DefaultStabilityUnknown); }
        if variant.context != witness.context || variant.callable != Some(*callable)
            || variant.assessment != Some(assessment.id()) || slot.variant != variant.id()
            || slot.parameter != entry.parameter() || data.parameters.get(slot.parameter)
                .is_none_or(|p| p.signature != variant.signature)
            || data.links.get(witness.link).is_none_or(|l| l.parameter != slot.parameter || l.entity != witness.formal) {
            return Err(ObligationKind::IncompatibleContexts);
        }
        Ok(Self { owner: witness.owner, formal: witness.formal, context: witness.context,
            place: entry.place().id(), proof: [RowRef::of(assessment.id()), RowRef::of(variant.id()),
                RowRef::of(slot.id()), RowRef::of(witness.link)] })
    }
    pub fn owner(&self) -> Id<EntityRef> { self.owner }
    pub fn formal(&self) -> Id<ParameterEntity> { self.formal }
}

/// An admitted finite path has original identity, condition and proof, independent of a query.
pub struct NativePath {
    pub(super) identity: RowRef,
    pub(super) owner: Id<EntityRef>,
    pub(super) input: Id<Place>,
    pub(super) condition: Diagram,
    pub(super) qualification: AssertionQualification,
    pub(super) kind: TransferKind,
    pub(super) proof: Vec<RowRef>,
    pub(super) _charge: Box<dyn Reservation>,
}
impl NativePath {
    /// Prepared generation consumption uses the current finite WitnessBranch after the declared
    /// Summary proof invariants have admitted its canonical rows. It is not an ID-only bridge.
    pub fn summary(branch: &crate::domain::transfer::summary::WitnessBranch,
        budget: &ResourceBudget) -> Result<Self, ModelError> {
        use crate::domain::composition::CompositionOperand;
        let crate::domain::transfer::summary::SummaryPremise::Witness { witness } = branch.premise()
        else { return Err(ModelError::Invalid("native summary path requires a finite witness".into())); };
        Self::from_branch(branch.descriptor(), branch.qualification(), branch.condition(), RowRef::of(witness), budget)
    }
    pub fn local(emission: &crate::domain::local_semantics::LocalEmission, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let branch = emission.branch();
        let mut path = Self::from_branch(branch.descriptor(), branch.qualification(), branch.condition(),
            RowRef::of(emission.contribution().id()), budget)?;
        path.proof.extend([RowRef::of(emission.contribution().id()), RowRef::of(emission.entry().witness().id()),
            RowRef::of(emission.contribution().support)]);
        Ok(path)
    }
    pub fn composed(transfer: &crate::domain::composition::ComposedTransfer,
        emission: &crate::domain::composition::SummaryEmission, budget: &ResourceBudget) -> Result<Self, ModelError> {
        if emission.alternative.qualification != transfer.qualification.id()
            || emission.contribution.witness != emission.witness.id()
            || emission.contribution.alternative != emission.alternative.id()
            || emission.key.descriptor() != transfer.descriptor
            || emission.witness.caller != transfer.witness.caller.id()
            || emission.witness.callee != transfer.witness.callee.id() {
            return Err(ModelError::Invalid("native composed proof differs from finite emission".into()));
        }
        let mut path = Self::from_branch(transfer.descriptor.clone(), &transfer.qualification,
            &transfer.condition, RowRef::of(emission.contribution.id()), budget)?;
        path.proof.extend([RowRef::of(emission.witness.id()), RowRef::of(emission.witness.caller),
            RowRef::of(emission.witness.callee)]);
        Ok(path)
    }
    pub fn continuation(emission: &crate::domain::execution::summary_path::PathEmission,
        budget: &ResourceBudget) -> Result<Self, ModelError> {
        let branch = &emission.branch;
        let mut path = Self::from_branch(branch.descriptor(), branch.qualification(), branch.condition(),
            RowRef::of(emission.witness.id()), budget)?;
        path.proof.extend([RowRef::of(emission.witness.id()), RowRef::of(emission.route.id()), RowRef::of(emission.source.id())]);
        Ok(path)
    }
    /// The exact native leaf path is independently admitted by its Guard Entry replay. It is
    /// a guard path, not a claim of return/raise execution or a summary transfer.
    pub fn guard(entry: &DerivedEntryValue, data: &EntryData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let EntryAccessSource::Guard { observation, support, .. } = entry.source()
        else { return Err(ModelError::Invalid("native guard path requires Guard Entry proof".into())); };
        let leaf = data.leaves.get(*observation).ok_or_else(|| ModelError::Invalid("native leaf absent".into()))?;
        let q = data.qualifications.get(leaf.qualification).ok_or_else(|| ModelError::Invalid("native leaf qualification absent".into()))?;
        let condition = data.conditions.get(q.condition).ok_or_else(|| ModelError::Invalid("native guard condition absent".into()))?;
        let allowance = data.condition_nodes.len().saturating_mul(2048).saturating_add(8192);
        let decode = budget.reserve("native-guard-condition-decode", allowance)?;
        let nodes = data.condition_nodes.iter().cloned().collect::<Vec<_>>();
        let diagram = Diagram::from_records(condition, &nodes)?;
        if q.context != entry.witness().context || !diagram.support().contains(&leaf.atom) {
            return Err(ModelError::Invalid("native guard path crosses Entry frame".into()));
        }
        let charge = budget.reserve("native-guard-path", diagram.allocation_allowance().saturating_add(8192))?;
        drop(nodes); drop(decode);
        let mut proof = entry_premises(entry);
        proof.extend([RowRef::of(leaf.id()), RowRef::of(*support)]);
        Ok(Self { identity: RowRef::of(leaf.id()), owner: entry.witness().owner, input: entry.place().id(),
            condition: diagram, qualification: q.clone(), kind: TransferKind::Identity, proof, _charge: charge })
    }
    fn from_branch(descriptor: TransferDescriptor, q: &AssertionQualification, condition: &Diagram,
        identity: RowRef, budget: &ResourceBudget) -> Result<Self, ModelError> {
        descriptor.check(q)?;
        if q.condition != condition.id() {
            return Err(ModelError::Invalid("native finite path differs from original condition".into()));
        }
        let charge = budget.reserve("native-finite-path", condition.allocation_allowance().saturating_add(8192))?;
        Ok(Self { identity, owner: descriptor.owner, input: descriptor.input, condition: condition.clone(),
            qualification: q.clone(), kind: descriptor.kind, proof: vec![identity], _charge: charge })
    }
    pub fn identity(&self) -> RowRef { self.identity }
    pub fn original_condition(&self) -> Id<crate::domain::conditions::Condition> { self.condition.id() }
}

/// The operation alone constructs atom assignment authority. Guard Entry is an opaque replay;
/// a stored witness, operand spelling or atom ID cannot construct this type.
pub struct CheckedAtom {
    pub(super) atom: Id<EvaluationAtom>,
    pub(super) owner: Id<EntityRef>,
    pub(super) formal: Id<ParameterEntity>,
    pub(super) context: Id<crate::domain::attribution::AnalysisContext>,
    pub(super) coverage: Diagram,
    pub(super) predicate: Predicate,
    pub(super) literals: Vec<Literal>,
    pub(super) builtin: Option<String>,
    pub(super) proof: Vec<RowRef>,
    pub(super) _charge: Box<dyn Reservation>,
}
impl CheckedAtom {
    pub fn derive(entry: &DerivedEntryValue, data: &TheoryData<'_>,
        invocation: &local::AnalysisInvocation, budget: &ResourceBudget)
        -> Result<Result<Self, ObligationKind>, ModelError> {
        let result = (|| -> Result<_, ObligationKind> {
            let EntryAccessSource::Guard { observation, support, .. } = entry.source()
            else { return Err(ObligationKind::EntryValueUnknown); };
            let leaf = data.entry.leaves.get(*observation).ok_or(ObligationKind::MissingEvidence)?;
            let atom = data.entry.atoms.get(leaf.atom).ok_or(ObligationKind::MissingEvidence)?;
            let predicate = data.entry.predicates.get(atom.predicate).ok_or(ObligationKind::MissingEvidence)?;
            if invocation.context != entry.witness().context || atom.context != invocation.context
                || data.entry.runs.get(entry.witness().run).is_none_or(|r| r.input != invocation.input) {
                return Err(ObligationKind::IncompatibleContexts);
            }
            Ok((leaf, atom, predicate, *support))
        })();
        let (leaf, atom, predicate, support) = match result { Ok(v) => v, Err(e) => return Ok(Err(e)) };
        // Admit temporary selection, proof and literal cloning before constructing any buffers.
        let allowance = data.literals.iter().try_fold(16384usize + entry.condition().allocation_allowance(), |sum, row|
            sum.checked_add(row.heap_bytes().saturating_mul(3).saturating_add(512)))
            .and_then(|n| n.checked_add(data.set_members.len().saturating_mul(128)))
            .ok_or_else(|| ModelError::Invalid("native literal admission overflow".into()))?;
        let charge = budget.reserve("native-exact-atom", allowance)?;
        let mut proof = entry_premises(entry);
        proof.extend([RowRef::of(leaf.id()), RowRef::of(support)]);
        let builtin = if matches!(predicate, Predicate::TypeIs { .. }) {
            match CheckedBuiltinOperand::derive(data, invocation, leaf.id(), support, budget)? {
                Ok(builtin) => {
                    // The request-only replay need not have emitted a canonical builtin witness.
                    // Cite original premises that can be hydrated from the generation instead.
                    let w = builtin.witness();
                    proof.extend_from_slice(builtin.operand_premises());
                    proof.extend([RowRef::of(w.call), RowRef::of(w.syntax_support), RowRef::of(w.callee_resolution),
                        RowRef::of(w.callee_support), RowRef::of(w.class_observation), RowRef::of(w.class_support),
                        RowRef::of(w.class_resolution), RowRef::of(w.class_resolution_support)]);
                    if let Some(row) = w.call_placement_support { proof.push(RowRef::of(row)); }
                    if let Some(row) = w.placement_support { proof.push(RowRef::of(row)); }
                    Some(builtin.class_name().to_owned())
                },
                Err(_) => return Ok(Err(ObligationKind::ConditionTransferUnsupported)),
            }
        } else { None };
        let selected = match predicate {
            Predicate::Equals { value } | Predicate::IsValue { value } => vec![*value],
            Predicate::MemberOf { values } => {
                let Some(set) = data.sets.get(*values) else { return Ok(Err(ObligationKind::MissingEvidence)); };
                if data.set_members.iter().filter(|m| m.set == *values).take(4097).count() > 4096 {
                    return Ok(Err(ObligationKind::ConditionWorkLimit));
                }
                let ids = data.set_members.iter().filter(|m| m.set == *values).map(|m| m.value).collect::<Vec<_>>();
                if LiteralSet::of(ids.iter().copied()).0 != *set { return Ok(Err(ObligationKind::MissingEvidence)); }
                proof.push(RowRef::of(set.id()));
                proof.extend(data.set_members.iter().filter(|m| m.set == *values).map(|m| RowRef::of(m.id())));
                ids
            },
            _ => vec![],
        };
        let mut literals = Vec::with_capacity(selected.len());
        for id in selected { let Some(row) = data.literals.get(id) else { return Ok(Err(ObligationKind::MissingEvidence)); }; row.validate()?;
            proof.push(RowRef::of(id)); literals.push(row.clone()); }
        Ok(Ok(Self { atom: atom.id(), owner: entry.witness().owner, formal: entry.witness().formal,
            context: entry.witness().context, coverage: entry.condition().clone(), predicate: predicate.clone(),
            literals, builtin, proof, _charge: charge }))
    }
    /// Follow the existing checked stability/binding substitution, then prove that the bound
    /// whole actual is the caller's entry formal. An opaque InvokedGuard never supplies this link.
    pub fn rebase(&self, output: &RebasedGuards, actual: &DerivedEntryValue,
        binding: &crate::domain::conditions::stability::CheckedGuardBinding,
        stability: &crate::domain::conditions::stability::CheckedStability,
        budget: &ResourceBudget) -> Result<Result<Self, ObligationKind>, ModelError> {
        if !self.coverage.is_true() { return Ok(Err(ObligationKind::ConditionTransferUnsupported)); }
        let actual_root = PlaceRoot::Occurrence { occurrence: actual.witness().access };
        let actual_place = Place { root: actual_root.id(), path: AccessPath::empty().id() };
        let mut substitutions = output.substitutions.iter().filter(|s| s.source_atom == self.atom);
        let Some(substitution) = substitutions.next() else { return Ok(Err(ObligationKind::ConditionTransferUnsupported)); };
        if substitutions.next().is_some() || substitution.actual_place != actual_place.id() {
            return Ok(Err(ObligationKind::EntryValueUnknown));
        }
        let Some(atom) = output.atoms.iter().find(|a| a.id() == substitution.atom) else { return Ok(Err(ObligationKind::MissingEvidence)); };
        if atom.context != actual.witness().context || atom.operand != Some(actual_place.id())
            || stability.witness().atom != self.atom || substitution.witness != stability.witness().id()
            || substitution.binding != binding.binding() || substitution.event != binding.event()
            || substitution.source != binding.source() || binding.root() != actual_root
            || !binding.agrees(stability, atom.evaluation, atom.context)
            || !output.predicates.iter().any(|p| p.id() == atom.predicate && *p == Predicate::BoundGuard { source: self.atom }) {
            return Ok(Err(ObligationKind::IncompatibleContexts));
        }
        let charge = budget.reserve("native-rebased-exact-atom", actual.condition().allocation_allowance()
            .saturating_add(self.literals.iter().map(|r| r.heap_bytes() + 256).sum::<usize>()).saturating_add(8192))?;
        let mut proof = self.proof.clone();
        proof.extend(entry_premises(actual));
        proof.extend([RowRef::of(substitution.id()), RowRef::of(substitution.witness),
            RowRef::of(substitution.binding), RowRef::of(substitution.event)]);
        Ok(Ok(Self { atom: atom.id(), owner: actual.witness().owner, formal: actual.witness().formal,
            context: atom.context, coverage: actual.condition().clone(), predicate: self.predicate.clone(),
            literals: self.literals.clone(), builtin: self.builtin.clone(), proof, _charge: charge }))
    }
}

// A request may replay Guard Entry for predicates outside the canonical stability envelope.
// Such replay does not publish a new Entry witness: cite its original recorded premises.
pub(super) fn entry_premises(entry: &DerivedEntryValue) -> Vec<RowRef> {
    let w = entry.witness();
    let mut proof = vec![RowRef::of(w.owner), RowRef::of(w.formal), RowRef::of(w.access),
        RowRef::of(w.run), RowRef::of(w.link), RowRef::of(w.declaration_support), RowRef::of(w.owner_support),
        RowRef::of(w.use_observation), RowRef::of(w.use_support), RowRef::of(w.reaching),
        RowRef::of(w.reaching_support), RowRef::of(w.definition), RowRef::of(w.definition_support), RowRef::of(w.coverage)];
    if let Some(row) = w.parameter_placement { proof.push(RowRef::of(row)); }
    if let Some(row) = w.parameter_placement_support { proof.push(RowRef::of(row)); }
    proof
}
