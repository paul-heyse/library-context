//! Qualified source associations remain uncertain alternatives, never finite transfer proofs.
use super::summary_production::{SummaryData, SummaryRecords};
use crate::Domain;
use crate::domain::{
    analysis::summary as owner,
    assertion::*,
    attribution::*,
    calls::*,
    flow::*,
    normalized::{entities::*, symbolic_fields::*},
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::*,
    transfer::TransferKind,
    *,
};

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(
    name = "summary_symbolic_field_alternatives",
    rule = "qualified_symbolic_field_association"
)]
pub struct SymbolicFieldAlternative {
    #[model(key)]
    pub invocation: Id<owner::AnalysisInvocation>,
    #[model(key, premise)]
    pub link: Id<SourceFieldReaderLink>,
    #[model(key, premise)]
    pub value: Id<FlowValueObservation>,
    #[model(key, premise)]
    pub support: Id<FlowValueSupport>,
    #[model(premise)]
    pub store: Option<Id<crate::domain::local_symbolic::SymbolicFieldStore>>,
    pub parameter: Id<SignatureParameter>,
    pub constructor: Id<EntityRef>,
    pub reader: Id<EntityRef>,
    pub constructor_qualification: Id<AssertionQualification>,
    pub reader_qualification: Id<AssertionQualification>,
    pub sink: Id<Occurrence>,
    pub kind: FlowSinkKind,
    pub transfer: TransferKind,
    pub through_call: bool,
    /// The two source links constructor→field→reader. This is association depth, independent
    /// of the finite call-path proof limit: this row carries Unknown and no transfer proof.
    pub depth: i64,
    pub reason: ObligationKind,
}

pub fn derive(
    data: &SummaryData,
    invocation: &owner::AnalysisInvocation,
    budget: &ResourceBudget,
) -> Result<crate::domain::normalized::Rows<SymbolicFieldAlternative>, ModelError> {
    let mut alternatives = crate::domain::normalized::Rows::new(budget);
    let invalid = |s: &str| ModelError::Invalid(s.into());
    let e = &data.entry;
    for link in data.symbolic_links.iter() {
        let association = data
            .symbolic_associations
            .get(link.association)
            .ok_or_else(|| invalid("symbolic association missing"))?;
        let class = data
            .symbolic_classes
            .get(association.class)
            .ok_or_else(|| invalid("symbolic class assessment missing"))?;
        let reader = data
            .symbolic_readers
            .get(link.reader)
            .ok_or_else(|| invalid("symbolic reader missing"))?;
        if !class.supported_record {
            continue;
        }
        if reader.class != class.class {
            return Err(invalid("symbolic link crosses class"));
        }
        let cq = e
            .qualifications
            .get(association.qualification)
            .ok_or_else(|| invalid("symbolic constructor qualification missing"))?;
        if cq.context != invocation.context
            || cq.modality != Modality::Definite
            || cq.approximation != Approximation::Exact
            || cq.condition != conditions::Diagram::always().id()
        {
            continue;
        }
        let parameter = e
            .parameters
            .get(association.parameter)
            .ok_or_else(|| invalid("symbolic formal missing"))?;
        let signature = e
            .signatures
            .get(parameter.signature)
            .ok_or_else(|| invalid("symbolic signature missing"))?;
        // Declaration evaluation belongs to the enclosing class; the initializer body owns
        // the store. Resolve its existing callable identity, never the header's evaluator.
        let mut constructors = e
            .callables
            .iter()
            .filter(|c| match c {
                CallableEntity::Source { declaration, .. } => {
                    e.symbol_declarations.iter().any(|d| {
                        d.symbol == signature.symbol
                            && data.same_occurrence(*declaration, d.declaration)
                    })
                }
                CallableEntity::Synthetic { symbol } => *symbol == signature.symbol,
                _ => false,
            })
            .map(|c| EntityRef::Callable { callable: c.id() }.id())
            .filter(|id| e.refs.get(*id).is_some());
        let Some(constructor) = constructors.next() else {
            continue;
        };
        if constructors.next().is_some() {
            return Err(invalid("symbolic constructor body owner is ambiguous"));
        }
        let mut reader_owners = e
            .owners
            .iter()
            .filter(|o| data.same_occurrence(o.occurrence, reader.access));
        let reader_owner = reader_owners
            .next()
            .ok_or_else(|| invalid("symbolic reader owner missing"))?;
        if reader_owners.any(|o| o.entity != reader_owner.entity) {
            return Err(invalid("symbolic reader owner ambiguous"));
        }
        let store = if let Some(store) = association.store {
            data.symbolic_local_stores.iter().find(|s| {
                s.store == store
                    && s.constructor == constructor
                    && data.local_invocations.get(s.invocation).is_some_and(|i| {
                        (i.input, i.context) == (invocation.input, invocation.context)
                    })
            })
        } else {
            None
        };
        let reason = if association.kind == SourceStorageKind::PlainInitializer && store.is_none() {
            ObligationKind::MissingEvidence
        } else {
            ObligationKind::ScopeBoundary
        };
        for support in e.value_supports.iter() {
            let Some(run) = e.runs.get(support.run) else {
                return Err(invalid("symbolic flow run missing"));
            };
            if (run.input, run.context) != (invocation.input, invocation.context) {
                continue;
            }
            let value = e
                .values
                .get(support.assertion)
                .ok_or_else(|| invalid("symbolic flow observation missing"))?;
            let use_ = e
                .uses
                .get(value.use_)
                .ok_or_else(|| invalid("symbolic reader use missing"))?;
            if !data.same_occurrence(use_.occurrence, reader.access) {
                continue;
            }
            let rq = e
                .qualifications
                .get(value.qualification)
                .ok_or_else(|| invalid("symbolic reader qualification missing"))?;
            if rq.context != invocation.context
                || rq.modality != Modality::Definite
                || rq.approximation != Approximation::Exact
            {
                continue;
            }
            let premise = analysis::native::NativeAssertionPremise::Value {
                assertion: value.id(),
                support: support.id(),
            };
            let Some(native) = data.native.iter().find(|n| n.premise == premise.id()) else {
                continue;
            };
            let Some(attribution) = support.attribution() else {
                continue;
            };
            let Some(surface) = e.surfaces.get(attribution.surface) else {
                continue;
            };
            if native.qualification != rq.id()
                || native.family != FactFamily::Flow
                || native.fidelity != Fidelity::NativeStructural
                || native.status != analysis::policy::EvidenceStatus::StructurallyObserved
                || attribution.fidelity != native.fidelity
                || surface.provider != run.provider
                || surface.family != FactFamily::Flow
            {
                continue;
            }
            // Reader atoms keep their original owner and condition; no constructor rebase occurs.
            let _allocation = budget.reserve(
                "symbolic-field-alternative",
                size_of::<SymbolicFieldAlternative>(),
            )?;
            alternatives.insert(SymbolicFieldAlternative {
                invocation: invocation.id(),
                link: link.id(),
                value: value.id(),
                support: support.id(),
                store: store.map(Record::id),
                parameter: parameter.id(),
                constructor,
                reader: reader_owner.entity,
                constructor_qualification: cq.id(),
                reader_qualification: rq.id(),
                sink: value.sink,
                kind: value.kind,
                transfer: value.transfer,
                through_call: value.through_call,
                depth: 2,
                reason,
            })?;
        }
    }
    Ok(alternatives)
}

pub(super) fn produce(
    data: &SummaryData,
    invocation: &owner::AnalysisInvocation,
    out: &mut SummaryRecords,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    for row in derive(data, invocation, budget)?.iter() {
        out.symbolic_alternatives.insert(row.clone())?;
    }
    Ok(())
}
