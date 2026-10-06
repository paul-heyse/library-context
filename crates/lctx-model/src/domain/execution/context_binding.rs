//! A context entry target read is distinct from parameter entry, mutation stability and
//! runtime heap identity. Only a replayed WithTarget reaching the exact read admits it.
use super::{
    evaluation::{self, CheckedEvaluation, ExpressionRequest, ReleaseSafety, boundary},
    model_application::ModelApplicationData,
    model_construction::CheckedContextConstruction,
    model_context::{CheckedContextProtocol, ContextValue},
    source_call_records::SourceCallData,
};
use crate::domain::{
    analysis::{self, enriched_execution as publication, policy::EvidenceStatus},
    lexical::SyntaxField,
    normalized::Rows,
    obligation::ObligationKind,
    resources::ResourceBudget,
    source::{Occurrence, SyntaxKind},
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="context_entry_bindings",rule="context_entry_binding",invariant_refs=super::enriched_production::binding_invariants_refs)]
pub struct ContextEntryBinding {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub access: Id<Occurrence>,
    pub owner: Id<normalized::entities::EntityRef>,
    pub item: Id<Occurrence>,
    pub site: Id<Occurrence>,
    pub target: Id<Occurrence>,
    #[model(premise)]
    pub protocol: Id<models::AuthoredContextProtocol>,
    pub class: Id<calls::ProviderSymbol>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
    pub entry_actual: Option<Id<Occurrence>>,
    pub release: ReleaseSafety,
    pub sources: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(
    name = "context_entry_binding_sources",
    rule = "context_entry_binding_source"
)]
pub enum BindingSource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Actual {
        #[model(premise)]
        evaluation: Id<super::records::ExpressionEvaluation>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="context_entry_binding_members",rule="context_entry_binding_member",conclusion=binding)]
pub struct BindingMember {
    #[model(key)]
    pub binding: Id<ContextEntryBinding>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<BindingSource>,
}
impl analysis::support::sealed::DerivedEvidence for ContextEntryBinding {}
impl analysis::support::DerivedEvidence for ContextEntryBinding {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct CheckedContextBinding {
    record: ContextEntryBinding,
    sources: Rows<BindingSource>,
    members: Rows<BindingMember>,
    evaluation: CheckedEvaluation,
    _charge: charged::StateCharge,
    _native_charge: charged::StateCharge,
}
impl CheckedContextBinding {
    pub fn record(&self) -> &ContextEntryBinding {
        &self.record
    }
    pub fn sources(&self) -> &Rows<BindingSource> {
        &self.sources
    }
    pub fn members(&self) -> &Rows<BindingMember> {
        &self.members
    }
    pub(crate) fn into_evaluation(self) -> CheckedEvaluation {
        self.evaluation
    }
    pub fn derive(
        catalog: &models::Catalog,
        application: &ModelApplicationData,
        data: &SourceCallData,
        invocation: &publication::AnalysisInvocation,
        request: ExpressionRequest,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        Self::derive_with_values(catalog,application,data,invocation,request,budget,None,None)
    }
    /// Borrow the actual Base owner; ordered argument evidence is hydrated without evaluator replay.
    pub fn derive_produced(
        catalog: &models::Catalog,
        application: &ModelApplicationData,
        data: &SourceCallData,
        invocation: &publication::AnalysisInvocation,
        request: ExpressionRequest,
        budget: &ResourceBudget,
        values:&super::production::ProducedEvaluations,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        Self::derive_with_values(catalog,application,data,invocation,request,budget,Some(values),None)
    }
    pub(super) fn derive_with_values(
        catalog: &models::Catalog,
        application: &ModelApplicationData,
        data: &SourceCallData,
        invocation: &publication::AnalysisInvocation,
        request: ExpressionRequest,
        budget: &ResourceBudget,
        values:Option<&super::production::ProducedEvaluations>,
        construction:Option<&super::model_construction::PreparedConstructionInputs<'_>>,
    ) -> Result<Result<Self, ObligationKind>, ModelError> {
        use ObligationKind as K;
        if invocation.subject.is_some()
            || (invocation.input, invocation.context) != (request.input, request.context)
        {
            return Ok(Err(K::IncompatibleContexts));
        }
        let facts = &data.evaluation;
        let flow = &data.flow;
        evaluation::with_completion_syntax(facts, request, budget, |syntax| {
            syntax.observe(request.expression)?;
            let read = facts
                .occurrences
                .get(request.expression)
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            if read.syntax_kind != SyntaxKind::ExprName {
                return Err(boundary(K::UnsupportedControlFlow));
            }
            let mut uses = flow.uses.iter().filter(|row| row.occurrence == read.id());
            let use_ = uses.next().ok_or_else(|| boundary(K::EntryValueUnknown))?;
            if uses.next().is_some() {
                return Err(boundary(K::AmbiguousBinding));
            }
            let mut reaching = flow.reaching.iter().filter(|row| {
                row.use_ == use_.id()
                    && flow
                        .qualifications
                        .get(row.qualification)
                        .is_some_and(|q| q.context == request.context)
            });
            let reach = reaching
                .next()
                .ok_or_else(|| boundary(K::EntryValueUnknown))?;
            if reaching.next().is_some() || reach.loop_carried {
                return Err(boundary(K::EntryValueUnknown));
            }
            syntax.support(reach, reach.qualification, read.id())?;
            let Some(flow::ReachingDefinition::Bound { definition }) =
                flow.targets.get(reach.target)
            else {
                return Err(boundary(K::EntryValueUnknown));
            };
            let definition = flow
                .definitions
                .get(*definition)
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            if definition.place != use_.place {
                return Err(boundary(K::MissingEvidence));
            }
            let mut observed = flow.definition_observations.iter().filter(|row| {
                row.definition == definition.id()
                    && flow
                        .qualifications
                        .get(row.qualification)
                        .is_some_and(|q| q.context == request.context)
            });
            let row = observed
                .next()
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            if observed.next().is_some() {
                return Err(boundary(K::AmbiguousBinding));
            }
            let observed = row;
            if observed.kind != lexical::BindingEventKind::WithTarget {
                return Err(boundary(K::EntryValueUnknown));
            }
            syntax.support(observed, observed.qualification, read.id())?;
            let mut use_rows = flow.use_observations.iter().filter(|row| {
                row.use_ == use_.id()
                    && flow
                        .qualifications
                        .get(row.qualification)
                        .is_some_and(|q| q.context == request.context)
            });
            let use_row = use_rows
                .next()
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            if use_rows.next().is_some() || use_row.annotation || use_row.scope != observed.scope {
                return Err(boundary(K::EntryValueUnknown));
            }
            syntax.support(use_row, use_row.qualification, read.id())?;
            let scope = flow
                .lexical_scopes
                .get(use_row.scope)
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            let Some(normalized::entities::EntityRef::Callable { callable }) =
                facts.refs.get(request.owner)
            else {
                return Err(boundary(K::ScopeBoundary));
            };
            if !matches!(facts.callables.get(*callable),Some(normalized::entities::CallableEntity::Source{declaration,..})if *declaration==scope.owner)
            {
                return Err(boundary(K::ScopeBoundary));
            }
            let target = flow
                .occurrences
                .get(definition.occurrence)
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            let mut targets = facts.placements.iter().filter(|p| {
                p.field == SyntaxField::Target
                    && facts.occurrences.get(p.occurrence).is_some_and(|o| {
                        o.source == target.source
                            && o.start == target.start
                            && o.end == target.end
                            && o.structural_path == target.structural_path
                    })
            });
            let target_placement = targets.next().ok_or_else(|| boundary(K::MissingEvidence))?;
            if targets.next().is_some() {
                return Err(boundary(K::AmbiguousBinding));
            }
            let item = target_placement
                .parent
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            syntax.observe(item)?;
            let nodes = syntax.children(item)?;
            if nodes.len() != 2
                || facts
                    .occurrences
                    .get(target_placement.occurrence)
                    .is_none_or(|o| o.syntax_kind != SyntaxKind::ExprName)
            {
                return Err(boundary(K::UnsupportedControlFlow));
            }
            let site = nodes
                .iter()
                .find(|p| p.field == SyntaxField::Value)
                .ok_or_else(|| boundary(K::MissingEvidence))?
                .occurrence;
            if observed.value != Some(site) {
                return Err(boundary(K::MissingEvidence));
            }
            let mut parents = facts
                .placements
                .iter()
                .filter(|p| p.occurrence == item && p.field == SyntaxField::Item);
            let parent = parents
                .next()
                .ok_or_else(|| boundary(K::MissingEvidence))?
                .parent
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            if parents.next().is_some() {
                return Err(boundary(K::AmbiguousBinding));
            }
            syntax.observe(parent)?;
            if syntax.detail(parent)? != Some(syntax::SyntaxDetail::WithMode { is_async: false }) {
                return Err(boundary(K::UnsupportedControlFlow));
            }
            // Entry target identity cannot escape the exact body or be silently reused after rebinding.
            let parent_row = facts
                .occurrences
                .get(parent)
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            if read.source != parent_row.source
                || !read
                    .structural_path
                    .starts_with(&parent_row.structural_path)
                || read.start < target.end
            {
                return Err(boundary(K::ScopeBoundary));
            }
            if flow.definition_observations.iter().any(|row| {
                row.scope == observed.scope
                    && row.id() != observed.id()
                    && flow
                        .definitions
                        .get(row.definition)
                        .is_some_and(|d| d.place == use_.place)
            }) {
                return Err(boundary(K::EntryValueUnknown));
            }
            let mut coverage = false;
            for row in flow.coverage.iter().filter(|row| {
                row.family == attribution::FactFamily::Flow && row.context == request.context
            }) {
                let Some(run) = row.run.and_then(|id| flow.runs.get(id)) else {
                    continue;
                };
                if run.input == request.input
                    && match flow.scopes.get(row.scope) {
                        Some(source::CoverageScope::Artifact { artifact }) => {
                            *artifact == read.source
                        }
                        Some(source::CoverageScope::Input { input }) => *input == request.input,
                        Some(source::CoverageScope::Module { module }) => flow
                            .modules
                            .get(*module)
                            .is_some_and(|m| m.source == read.source),
                        _ => false,
                    }
                {
                    if row.status != attribution::CoverageStatus::CompleteUnderStatedModel {
                        return Err(boundary(K::IncompleteCoverage));
                    }
                    coverage = true;
                }
            }
            if !coverage {
                return Err(boundary(K::IncompleteCoverage));
            }
            let mut initializers = application.bindings.targets.iter().filter(|t| {
                t.site == site
                    && t.phase == calls::CallPhase::Init
                    && application
                        .bindings
                        .qualifications
                        .get(t.qualification)
                        .is_some_and(|q| q.context == request.context)
            });
            let init = initializers
                .next()
                .ok_or_else(|| boundary(K::UnresolvedTarget))?;
            if initializers.next().is_some() {
                return Err(boundary(K::AmbiguousBinding));
            }
            let class = init
                .receiver_class
                .ok_or_else(|| boundary(K::MissingEvidence))?;
            let protocol = CheckedContextProtocol::derive(
                catalog,
                application,
                class,
                request.input,
                request.context,
                budget,
            )?
            .map_err(boundary)?;
            let construction = CheckedContextConstruction::derive_with_inputs(
                &protocol,
                application,
                site,
                request.owner,
                request.input,
                request.context,
                budget,
                construction,
            )?
            .map_err(boundary)?;
            let mut charge = charged::StateCharge::new(budget, "context_entry_binding");
            charge.grow(4096)?;
            let mut sources = Rows::new(budget);
            for premise in construction.premises().iter() {
                sources.insert(BindingSource::Native {
                    premise: premise.id(),
                })?;
            }
            let mut status = analysis::support::inferred_status(
                analysis::Interpretation::Structural,
                [syntax.status(), construction.status()],
            );
            let base = data.completed.earlier();
            let mut returned = None;
            // Every constructor actual must be independently normal and disposable in native order.
            for (ordinal, argument) in construction.arguments().iter().enumerate() {
                if argument.ordinal != ordinal as i64
                    || !matches!(
                        argument.kind,
                        calls::ArgumentKind::Positional | calls::ArgumentKind::Keyword
                    )
                {
                    return Err(boundary(K::UnsupportedUnpacking));
                }
                let mut rows = base.evaluations.iter().filter(|row| {
                    row.expression == argument.value
                        && row.owner == request.owner
                        && base.invocations.get(row.invocation).is_some_and(|p| {
                            (p.input, p.context) == (request.input, request.context)
                        })
                });
                let row = rows.next().ok_or_else(|| boundary(K::MissingEvidence))?;
                if rows.next().is_some() {
                    return Err(boundary(K::AmbiguousBinding));
                }
                let checked = super::source_invocation::checked_value(base,row,values)?;
                if checked.exception().is_some()
                    || checked.release() != ReleaseSafety::Closed
                        && super::source_invocation::held_formal(base,row,values)?.is_none()
                        && !super::builtin_read::CheckedBuiltinRead::derive(
                            facts,
                            checked.request(),
                            budget,
                        )?
                        .is_ok()
                {
                    return Err(boundary(K::FrameExitCleanup));
                }
                sources.insert(BindingSource::Actual {
                    evaluation: row.id(),
                })?;
                status = analysis::support::inferred_status(
                    analysis::Interpretation::Structural,
                    [status, checked.status()],
                );
                if construction.entry_value() == ContextValue::Actual(argument.value) {
                    returned = Some(checked);
                }
            }
            let entry_actual = match construction.entry_value() {
                ContextValue::None => None,
                ContextValue::Actual(actual) => Some(actual),
            };
            if entry_actual.is_some() && returned.is_none() {
                return Err(boundary(K::MissingEvidence));
            }
            let qualification = syntax.qualification(read.id()).map_err(boundary)?;
            let release = returned
                .as_ref()
                .map_or(ReleaseSafety::Closed, |proof|proof.release());
            let (native, native_charge) = syntax.take_admission();
            for premise in native {
                sources.insert(BindingSource::Native { premise })?;
            }
            let record = ContextEntryBinding {
                invocation: invocation.id(),
                access: read.id(),
                owner: request.owner,
                item,
                site,
                target: target_placement.occurrence,
                protocol: protocol.compiled().declaration().id(),
                class,
                qualification,
                status,
                entry_actual,
                release,
                sources: super::records::ordered_digest(
                    "context-binding-sources",
                    sources.iter().map(Record::id),
                ),
            };
            let evaluation = evaluation::context_binding_evaluation(
                facts,
                request,
                returned.as_deref(),
                record.id(),
                status,
                budget,
            )?
            .map_err(boundary)?;
            let mut members = Rows::new(budget);
            for (ordinal, source) in sources.iter().enumerate() {
                members.insert(BindingMember {
                    binding: record.id(),
                    ordinal: ordinal as i64,
                    source: source.id(),
                })?;
            }
            Ok(Self {
                record,
                sources,
                members,
                evaluation,
                _charge: charge,
                _native_charge: native_charge,
            })
        })
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<ContextEntryBinding>(),
        Relation::of::<BindingSource>(),
        Relation::of::<BindingMember>(),
    ]
}
