//! Continuation through an independently attributed native value path. A call result must
//! already have finite proof; the native through-call dependency cannot supply that proof.
use crate::domain::{
    analysis::{
        self,
        native::{NativeAssertionPremise, NativeQualification},
        summary as owner,
        support::{DerivedEvidence, SourceFacts},
    },
    assertion::*,
    attribution::*,
    composition::CompositionOperand,
    conditions::{entry::EntryData, *},
    flow::*,
    normalized::{Rows, entities::*},
    obligation::ObligationKind,
    resources::{Reservation, ResourceBudget},
    transfer::{summary::*, *},
    value::*,
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name = "summary_path_witnesses", rule = "continue_proven_call_value")]
pub struct SummaryPathWitness {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key)]
    pub transfer: Id<TransferKey>,
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key, premise)]
    pub source: Id<SummaryPremise>,
    #[model(key, premise)]
    pub route: Id<SummaryPathRoute>,
    pub status: analysis::policy::EvidenceStatus,
    pub heuristic: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum,serde::Serialize,serde::Deserialize)]
#[model(name = "summary_path_routes", rule = "summary_native_path_route")]
pub enum SummaryPathRoute {
    #[model(code = 0)]
    Call {
        #[model(premise)]
        value: Id<FlowValueObservation>,
        #[model(premise)]
        value_support: Id<FlowValueSupport>,
        #[model(premise)]
        path: Id<FlowValuePathObservation>,
        #[model(premise)]
        path_support: Id<FlowValuePathSupport>,
        #[model(premise)]
        step: Id<FlowCallStep>,
    },
    #[model(code = 1)]
    Alias {
        #[model(premise)]
        value: Id<FlowValueObservation>,
        #[model(premise)]
        value_support: Id<FlowValueSupport>,
        #[model(premise)]
        use_observation: Id<FlowUseObservation>,
        #[model(premise)]
        use_support: Id<FlowUseSupport>,
        #[model(premise)]
        reaching: Id<FlowReachingObservation>,
        #[model(premise)]
        reaching_support: Id<FlowReachingSupport>,
        #[model(premise)]
        definition: Id<FlowDefinitionObservation>,
        #[model(premise)]
        definition_support: Id<FlowDefinitionSupport>,
    },
    #[model(code = 2)]
    EscapingRaise {
        #[model(premise)]
        route: Id<SummaryPathRoute>,
        #[model(premise)]
        outcome: Id<super::summary_exceptions::SummaryExceptionOutcome>,
    },
}
impl analysis::support::sealed::DerivedEvidence for SummaryPathWitness {}
impl DerivedEvidence for SummaryPathWitness {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: self.heuristic,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="summary_path_contributions",rule="summary_path_contribution",conclusion=alternative)]
pub struct SummaryPathContribution {
    #[model(key)]
    pub alternative: Id<TransferAlternative>,
    #[model(key, premise)]
    pub witness: Id<SummaryPathWitness>,
}
#[macro_export]
macro_rules! summary_path_inputs{($apply:ident)=>{$apply!{
 call_paths:$crate::domain::flow::FlowCallPath,call_steps:$crate::domain::flow::FlowCallStep,value_paths:$crate::domain::flow::FlowValuePathObservation,value_path_supports:$crate::domain::flow::FlowValuePathSupport,
}};}
macro_rules! data{($($field:ident:$ty:ty,)*)=>{pub struct PathData{$(pub $field:Rows<$ty>,)*}impl PathData{pub fn new(b:&ResourceBudget)->Self{Self{$($field:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<(),ModelError>{$(if n==<$ty>::NAME{self.$field.decode(b)?;})*Ok(())}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}}};}
crate::summary_path_inputs!(data);
pub struct PathEmission {
    pub route: SummaryPathRoute,
    pub witness: SummaryPathWitness,
    pub source: SummaryPremise,
    pub branch: TransferBranch<TransferKey>,
    pub root: PlaceRoot,
    pub place: Place,
    pub(super) _charge: Box<dyn Reservation>,
}
impl PathEmission {
    /// Assemble canonical recorded continuation rows after the declared Summary replay has
    /// admitted them. The earlier finite operand remains opaque; IDs alone grant no source.
    #[expect(
        clippy::too_many_arguments,
        reason = "Canonical continuation keeps witness, route, source, branch frame and output vocabulary explicit"
    )]
    pub fn from_records(
        witness: &SummaryPathWitness,
        route: &SummaryPathRoute,
        source: &SummaryPremise,
        predecessor: &dyn CompositionOperand,
        key: TransferKey,
        q: AssertionQualification,
        condition: Diagram,
        root: PlaceRoot,
        place: Place,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let earlier = predecessor.descriptor();
        if witness.transfer != key.id()
            || witness.qualification != q.id()
            || witness.route != route.id()
            || witness.source != source.id()
            || predecessor.premise() != *source
            || key.kind != TransferKind::Derived
            || (earlier.owner, earlier.input, earlier.context, earlier.scope)
                != (key.owner, key.input, key.context, key.scope)
            || key.output != place.id()
            || place.root != root.id()
            || q.condition != condition.id()
        {
            return Err(invalid(
                "recorded continuation differs from its finite source/frame",
            ));
        }
        let charge = budget.reserve(
            "recorded-summary-path",
            condition.allocation_allowance().saturating_add(8192),
        )?;
        let branch = TransferBranch::new(key, q, condition, budget)?;
        Ok(Self {
            witness: witness.clone(),
            route: route.clone(),
            source: source.clone(),
            branch,
            root,
            place,
            _charge: charge,
        })
    }
}
impl CompositionOperand for PathEmission {
    fn descriptor(&self) -> TransferDescriptor {
        self.branch.descriptor()
    }
    fn qualification(&self) -> &AssertionQualification {
        self.branch.qualification()
    }
    fn condition(&self) -> &Diagram {
        self.branch.condition()
    }
    fn premise(&self) -> SummaryPremise {
        SummaryPremise::Path {
            witness: self.witness.id(),
        }
    }
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ObligationKind> {
    r.get(id).ok_or(ObligationKind::MissingEvidence)
}
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
/// Every selected native support remains an explicit proof member. The structural native path
/// carries a dependency; continuation conservatively retains Derived, never invents identity.
#[allow(
    clippy::too_many_arguments,
    reason = "Public summary-path proof keeps independent source, native, vocabulary and path witness authorities explicit."
)]
pub fn derive(
    data: &EntryData,
    paths: &PathData,
    native: &Rows<NativeQualification>,
    vocabulary: &crate::domain::conditions::rebase::GuardCatalog<'_>,
    invocation: &owner::AnalysisInvocation,
    source: &dyn CompositionOperand,
    facts: SourceFacts,
    value_support: Id<FlowValueSupport>,
    path_support: Id<FlowValuePathSupport>,
    step: Id<FlowCallStep>,
    budget: &ResourceBudget,
) -> Result<Result<PathEmission, ObligationKind>, ModelError> {
    let descriptor = source.descriptor();
    let setup = (|| {
        let vs = need(&data.value_supports, value_support)?;
        let value = need(&data.values, vs.assertion)?;
        let ps = need(&paths.value_path_supports, path_support)?;
        let path = need(&paths.value_paths, ps.assertion)?;
        let step = need(&paths.call_steps, step)?;
        if !value.through_call
            || path.value != value.id()
            || path.qualification != value.qualification
            || step.path != path.path
        {
            return Err(ObligationKind::MissingEvidence);
        }
        let va = vs.attribution().ok_or(ObligationKind::MissingEvidence)?;
        let pa = ps.attribution().ok_or(ObligationKind::MissingEvidence)?;
        if va.run != pa.run {
            return Err(ObligationKind::IncompatibleContexts);
        }
        let run = need(&data.runs, va.run)?;
        if (run.input, run.context) != (invocation.input, invocation.context) {
            return Err(ObligationKind::IncompatibleContexts);
        }
        let q = need(&data.qualifications, value.qualification)?;
        if q.context != descriptor.context
            || q.scope != descriptor.scope
            || q.context != invocation.context
        {
            return Err(ObligationKind::IncompatibleContexts);
        }
        if q.modality != Modality::Definite
            || q.approximation != Approximation::Exact
            || facts.qualification != source.qualification().id()
        {
            return Err(ObligationKind::Approximation);
        }
        let output = vocabulary
            .places
            .get(&descriptor.output)
            .ok_or(ObligationKind::MissingEvidence)?;
        if output.path != AccessPath::empty().id()
            || vocabulary.roots.get(&output.root)
                != Some(&PlaceRoot::Occurrence {
                    occurrence: step.call,
                })
        {
            return Err(ObligationKind::CallTransfer);
        }
        let use_ = need(&data.uses, value.use_)?;
        let input = vocabulary
            .places
            .get(&descriptor.input)
            .ok_or(ObligationKind::MissingEvidence)?;
        let original = need(&data.places, use_.place)?;
        let PlaceRoot::Formal { declaration } = need(&data.roots, original.root)? else {
            return Err(ObligationKind::EntryValueUnknown);
        };
        if input.path != AccessPath::empty().id()
            || original.path != AccessPath::empty().id()
            || vocabulary.roots.get(&input.root)
                != Some(&PlaceRoot::Entry {
                    declaration: *declaration,
                })
        {
            return Err(ObligationKind::EntryValueUnknown);
        }
        if !data
            .owners
            .iter()
            .any(|o| o.occurrence == use_.occurrence && o.entity == descriptor.owner)
            || !data
                .owners
                .iter()
                .any(|o| o.occurrence == value.sink && o.entity == descriptor.owner)
        {
            return Err(ObligationKind::ScopeBoundary);
        }
        let vp = NativeAssertionPremise::Value {
            assertion: value.id(),
            support: vs.id(),
        };
        let pp = NativeAssertionPremise::FlowValuePathObservation {
            assertion: path.id(),
            support: ps.id(),
        };
        let vq = native
            .iter()
            .find(|n| {
                n.premise == vp.id()
                    && n.qualification == q.id()
                    && n.family == FactFamily::Flow
                    && n.fidelity == Fidelity::NativeStructural
            })
            .ok_or(ObligationKind::MissingEvidence)?;
        let pq = native
            .iter()
            .find(|n| {
                n.premise == pp.id()
                    && n.qualification == q.id()
                    && n.family == FactFamily::Flow
                    && n.fidelity == Fidelity::NativeStructural
            })
            .ok_or(ObligationKind::MissingEvidence)?;
        let root = if step.ordinal > 0 {
            let outer = paths
                .call_steps
                .iter()
                .find(|s| s.path == step.path && s.ordinal == step.ordinal - 1)
                .ok_or(ObligationKind::MissingEvidence)?;
            if outer.role != FlowCallOperandRole::Argument {
                return Err(ObligationKind::CallTransfer);
            }
            PlaceRoot::Occurrence {
                occurrence: outer.operand,
            }
        } else {
            let EntityRef::Callable { callable } = need(&data.refs, descriptor.owner)? else {
                return Err(ObligationKind::ScopeBoundary);
            };
            let CallableEntity::Source { declaration, .. } = need(&data.callables, *callable)?
            else {
                return Err(ObligationKind::NoSourceDeclaration);
            };
            match value.kind {
                FlowSinkKind::Return => PlaceRoot::Return {
                    callable: *declaration,
                },
                FlowSinkKind::Yield => PlaceRoot::Yield {
                    callable: *declaration,
                },
                FlowSinkKind::Raise => PlaceRoot::Raise {
                    callable: *declaration,
                },
                FlowSinkKind::Argument => PlaceRoot::Occurrence {
                    occurrence: value.sink,
                },
                FlowSinkKind::Definition => PlaceRoot::Occurrence {
                    occurrence: value.sink,
                },
            }
        };
        Ok((value, path, step, q, root, vq.status, pq.status))
    })();
    let (value, path, step, q, root, vs, ps) = match setup {
        Ok(r) => r,
        Err(r) => return Ok(Err(r)),
    };
    let _decode = budget.reserve(
        "summary-path-condition",
        data.condition_nodes
            .len()
            .saturating_mul(2048)
            .saturating_add(4096),
    )?;
    let nodes = data.condition_nodes.iter().cloned().collect::<Vec<_>>();
    let condition = Diagram::from_records(
        data.conditions
            .get(q.condition)
            .ok_or_else(|| invalid("path condition absent"))?,
        &nodes,
    )?;
    let result =
        match source
            .condition()
            .admitted_binary(&condition, BooleanOperation::Conjunction, budget)
        {
            Ok(r) => r,
            Err(DiagramAdmissionError::Boundary(r)) => {
                return Ok(Err(crate::domain::obligation::from_kernel(r)));
            }
            Err(DiagramAdmissionError::Resource(e)) => return Err(e),
        };
    let (condition, charge) = result.into_parts();
    let q = AssertionQualification {
        condition: condition.id(),
        ..source.qualification().clone()
    };
    let place = Place {
        root: root.id(),
        path: AccessPath::empty().id(),
    };
    let key = TransferKey::from_descriptor(TransferDescriptor {
        output: place.id(),
        kind: TransferKind::Derived,
        provenance: ProvenanceClass::DerivedSummary,
        ..descriptor
    });
    let branch = TransferBranch::new(key, q.clone(), condition, budget)?;
    let source = source.premise();
    let status = analysis::support::inferred_status(
        analysis::Interpretation::Structural,
        [facts.status, vs, ps],
    );
    let route = SummaryPathRoute::Call {
        value: value.id(),
        value_support,
        path: path.id(),
        path_support,
        step: step.id(),
    };
    let witness = SummaryPathWitness {
        invocation: invocation.id(),
        transfer: branch.key().id(),
        qualification: q.id(),
        source: source.id(),
        route: route.id(),
        status,
        heuristic: facts.heuristic,
    };
    Ok(Ok(PathEmission {
        route,
        witness,
        source,
        branch,
        root,
        place,
        _charge: charge,
    }))
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<SummaryPathRoute>(),
        Relation::of::<SummaryPathWitness>(),
        Relation::of::<SummaryPathContribution>(),
    ]
}
