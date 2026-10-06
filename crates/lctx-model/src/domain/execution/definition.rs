//! Definition-time header/default evaluation is independent of body entry and call binding.
use super::{
    completion::CompletionRequest,
    evaluation::{self, ExpressionRequest, ReleaseSafety, boundary},
    source_call_records::SourceCallData,
};
use crate::domain::{
    analysis::{self, enriched_execution as publication, policy::EvidenceStatus},
    lexical::SyntaxField,
    normalized::Rows,
    resources::ResourceBudget,
    source::{Occurrence, SyntaxKind},
    *,
};
use crate::{Domain, DomainSum};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="definition_evaluations",rule="definition_evaluation",invariant_refs=super::enriched_production::definition_invariants_refs)]
pub struct DefinitionEvaluation {
    #[model(key, premise)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub statement: Id<Occurrence>,
    pub owner: Id<normalized::entities::EntityRef>,
    pub qualification: Id<assertion::AssertionQualification>,
    pub status: EvidenceStatus,
    pub defaults: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(
    name = "definition_evaluation_sources",
    rule = "definition_evaluation_source"
)]
pub enum DefinitionSource {
    #[model(code = 0)]
    Native {
        #[model(premise)]
        premise: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Default {
        #[model(premise)]
        evaluation: Id<super::records::ExpressionEvaluation>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="definition_evaluation_members",rule="definition_evaluation_member",conclusion=definition)]
pub struct DefinitionMember {
    #[model(key)]
    pub definition: Id<DefinitionEvaluation>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<DefinitionSource>,
}
impl analysis::support::sealed::DerivedEvidence for DefinitionEvaluation {}
impl analysis::support::DerivedEvidence for DefinitionEvaluation {
    fn source_facts(&self) -> analysis::support::SourceFacts {
        analysis::support::SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
pub struct CheckedDefinition {
    record: DefinitionEvaluation,
    sources: Rows<DefinitionSource>,
    members: Rows<DefinitionMember>,
    defaults: Vec<Id<Occurrence>>,
    _charge: charged::StateCharge,
    _native_charge: charged::StateCharge,
}
impl CheckedDefinition {
    pub fn record(&self) -> &DefinitionEvaluation {
        &self.record
    }
    pub fn sources(&self) -> &Rows<DefinitionSource> {
        &self.sources
    }
    pub fn members(&self) -> &Rows<DefinitionMember> {
        &self.members
    }
    pub(crate) fn defaults(&self) -> &[Id<Occurrence>] {
        &self.defaults
    }
    pub fn derive(
        data: &SourceCallData,
        request: CompletionRequest,
        invocation: &publication::AnalysisInvocation,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, obligation::ObligationKind>, ModelError> {
        Self::derive_with_values(data, request, invocation, budget, None)
    }
    /// Borrow the actual Base owner; ordered argument evidence is hydrated without evaluator replay.
    pub fn derive_produced(
        data: &SourceCallData,
        request: CompletionRequest,
        invocation: &publication::AnalysisInvocation,
        budget: &ResourceBudget,
        values: &super::production::ProducedEvaluations,
    ) -> Result<Result<Self, obligation::ObligationKind>, ModelError> {
        Self::derive_with_values(data, request, invocation, budget, Some(values))
    }
    pub(super) fn derive_with_values(
        data: &SourceCallData,
        request: CompletionRequest,
        invocation: &publication::AnalysisInvocation,
        budget: &ResourceBudget,
        values: Option<&super::production::ProducedEvaluations>,
    ) -> Result<Result<Self, obligation::ObligationKind>, ModelError> {
        use obligation::ObligationKind as K;
        if invocation.subject.is_some()
            || (invocation.input, invocation.context) != (request.input, request.context)
        {
            return Ok(Err(K::IncompatibleContexts));
        }
        let facts = &data.evaluation;
        let bindings = &data.bindings;
        let Some(site) = facts.occurrences.get(request.statement) else {
            return Ok(Err(K::MissingEvidence));
        };
        if site.syntax_kind != SyntaxKind::StmtFunctionDef {
            return Ok(Err(K::UnsupportedControlFlow));
        }
        let mut declarations = facts.declarations.iter().filter(|d| {
            d.declaration == site.id()
                && facts
                    .qualifications
                    .get(d.qualification)
                    .is_some_and(|q| q.context == request.context)
        });
        let Some(declaration) = declarations.next() else {
            return Ok(Err(K::MissingEvidence));
        };
        if declarations.next().is_some() || declaration.kind != syntax::DeclarationKind::Function {
            return Ok(Err(K::MissingEvidence));
        }
        let Some(normalized::entities::EntityRef::Callable { callable }) =
            facts.refs.get(request.owner)
        else {
            return Ok(Err(K::ScopeBoundary));
        };
        let Some(normalized::entities::CallableEntity::Source {
            declaration: parent,
            kind: normalized::entities::CallableKind::Function,
        }) = facts.callables.get(*callable)
        else {
            return Ok(Err(K::ScopeBoundary));
        };
        if declaration.parent != Some(*parent)
            || bindings
                .decorators
                .iter()
                .any(|d| d.declaration == site.id())
        {
            return Ok(Err(K::DefaultUnavailable));
        }
        let mut placements = facts.placements.iter().filter(|p| {
            p.occurrence == site.id() && p.parent == Some(*parent) && p.field == SyntaxField::Body
        });
        let Some(placement) = placements.next() else {
            return Ok(Err(K::MissingEvidence));
        };
        if placements.next().is_some() || placement.ordinal != 0 {
            return Ok(Err(K::EntryValueUnknown));
        }
        // A captured cell or prior replacement is an independent allocation/release obligation.
        for resolution in bindings.lexical_resolutions.iter().filter(|r| {
            r.captured
                && facts
                    .qualifications
                    .get(r.qualification)
                    .is_some_and(|q| q.context == request.context)
        }) {
            let read = bindings
                .occurrences
                .get(resolution.read)
                .ok_or_else(|| ModelError::Invalid("definition captured read absent".into()))?;
            if read.source == site.source
                && read.start >= site.start
                && read.end <= site.end
                && read.structural_path.starts_with(&site.structural_path)
            {
                return Ok(Err(K::CapturedStateUnavailable));
            }
        }
        let expression = ExpressionRequest {
            input: request.input,
            context: request.context,
            owner: request.owner,
            expression: request.statement,
        };
        evaluation::with_completion_syntax(facts, expression, budget, |syntax| {
            syntax.observe(site.id())?;
            syntax.support(declaration, declaration.qualification, site.id())?;
            let children = syntax.children(site.id())?;
            if children.iter().any(|p| {
                p.field != SyntaxField::Body
                    && facts.occurrences.get(p.occurrence).is_none_or(|o| {
                        !matches!(
                            o.syntax_kind,
                            SyntaxKind::Identifier | SyntaxKind::Parameters
                        )
                    })
            }) {
                return Err(boundary(K::DefaultUnavailable));
            }
            let mut charge = charged::StateCharge::new(budget, "definition_defaults");
            charge.grow(
                data.parameter_syntax.len() * size_of::<&syntax::ParameterSyntaxObservation>() * 2,
            )?;
            let parameters = data
                .parameter_syntax
                .iter()
                .filter(|p| {
                    p.function == site.id()
                        && facts
                            .qualifications
                            .get(p.qualification)
                            .is_some_and(|q| q.context == request.context)
                })
                .collect::<Vec<_>>();
            charge.grow(
                parameters.len()
                    * (size_of::<&syntax::ParameterSyntaxObservation>()
                        + size_of::<Id<Occurrence>>())
                    * 2,
            )?;
            let mut parameters = parameters;
            parameters.sort_by_key(|p| p.ordinal);
            if parameters
                .iter()
                .enumerate()
                .any(|(i, p)| p.ordinal != i as i64 || p.annotation.is_some())
            {
                return Err(boundary(K::DefaultUnavailable));
            }
            let mut syntax_parameters = Vec::new();
            for container in children.iter().filter(|p| {
                facts
                    .occurrences
                    .get(p.occurrence)
                    .is_some_and(|o| o.syntax_kind == SyntaxKind::Parameters)
            }) {
                for parameter in syntax.children(container.occurrence)? {
                    if !matches!(
                        facts
                            .occurrences
                            .get(parameter.occurrence)
                            .map(|o| o.syntax_kind),
                        Some(SyntaxKind::ParameterWithDefault | SyntaxKind::Parameter)
                    ) {
                        return Err(boundary(K::DefaultUnavailable));
                    }
                    charge.grow(size_of::<Id<Occurrence>>() * 2)?;
                    syntax_parameters.push(parameter.occurrence);
                }
            }
            if syntax_parameters.len() != parameters.len()
                || syntax_parameters
                    .iter()
                    .any(|site| parameters.iter().filter(|p| p.parameter == *site).count() != 1)
            {
                return Err(boundary(K::MissingEvidence));
            }
            for parameter in &parameters {
                let nodes = syntax.children(parameter.parameter)?;
                let default = nodes
                    .iter()
                    .find(|p| p.field == SyntaxField::Default)
                    .map(|p| p.occurrence);
                if default != parameter.default
                    || nodes
                        .iter()
                        .filter(|p| p.field == SyntaxField::Default)
                        .count()
                        > 1
                {
                    return Err(boundary(K::MissingEvidence));
                }
            }
            let mut defaults = Vec::new();
            let mut sources = Rows::new(budget);
            let base = data.completed.earlier();
            let mut status = syntax.status();
            let mut digest = KeySink::new("definition-default-evaluations");
            for parameter in parameters {
                syntax.support(parameter, parameter.qualification, site.id())?;
                if let Some(default) = parameter.default {
                    let mut rows = base.evaluations.iter().filter(|row| {
                        row.expression == default
                            && row.owner == request.owner
                            && base.invocations.get(row.invocation).is_some_and(|p| {
                                (p.input, p.context) == (request.input, request.context)
                            })
                    });
                    let row = rows.next().ok_or_else(|| boundary(K::DefaultUnavailable))?;
                    if rows.next().is_some() {
                        return Err(boundary(K::AmbiguousBinding));
                    }
                    let checked = super::source_invocation::checked_value(base, row, values)?;
                    if checked.release() != ReleaseSafety::Closed || checked.exception().is_some() {
                        return Err(boundary(K::DefaultUnavailable));
                    }
                    if facts.qualifications.get(checked.qualification())
                        != facts.qualifications.get(declaration.qualification)
                    {
                        return Err(boundary(K::IncompatibleContexts));
                    }
                    defaults.push(default);
                    default.encode(&mut digest);
                    row.id().encode(&mut digest);
                    sources.insert(DefinitionSource::Default {
                        evaluation: row.id(),
                    })?;
                    status = analysis::support::inferred_status(
                        analysis::Interpretation::Structural,
                        [status, checked.status()],
                    );
                }
            }
            let qualification = syntax.qualification(site.id()).map_err(boundary)?;
            status = analysis::support::inferred_status(
                analysis::Interpretation::Structural,
                [status, syntax.status()],
            );
            let (native, native_charge) = syntax.take_admission();
            for premise in native {
                sources.insert(DefinitionSource::Native { premise })?;
            }
            let record = DefinitionEvaluation {
                invocation: invocation.id(),
                statement: site.id(),
                owner: request.owner,
                qualification,
                status,
                defaults: digest.finish(),
            };
            let mut members = Rows::new(budget);
            for (ordinal, source) in sources.iter().enumerate() {
                members.insert(DefinitionMember {
                    definition: record.id(),
                    ordinal: ordinal as i64,
                    source: source.id(),
                })?;
            }
            Ok(Self {
                record,
                sources,
                members,
                defaults,
                _charge: charge,
                _native_charge: native_charge,
            })
        })
    }
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<DefinitionEvaluation>(),
        Relation::of::<DefinitionSource>(),
        Relation::of::<DefinitionMember>(),
    ]
}
