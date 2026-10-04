//! Fresh source binding is independent of body completion. The first finite contract accepts
//! a synchronous nondefault nested definition read through its sole native reaching definition.
//! Defaults, caller-held arguments and imported/global callable availability have separate owners.
use super::evaluation::EvaluationData;
use crate::domain::{
    analysis::{native::NativeAssertionPremise, policy::EvidenceStatus},
    assertion::{Approximation, AssertionQualification},
    attribution::{FactFamily, Modality},
    conditions::entry::EntryData,
    flow::*,
    lexical::{BindingEventKind, LexicalScopeKind},
    normalized::{
        Rows,
        binding_normalization::{BindingData, CompositionAdmission, ValidatedBoundCall},
        entities::{CallableEntity, CallableKind, EntityRef},
    },
    resources::ResourceBudget,
    source::*,
    syntax::DeclarationKind,
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCallRequest {
    pub input: Id<input::InputRevision>,
    pub context: Id<attribution::AnalysisContext>,
    pub event: Id<normalized::events::NormalizedCallEvent>,
}
/// This receipt proves fresh binding only, not normal invocation, body outcome or frame release.
pub struct CheckedSourceBinding {
    request: SourceCallRequest,
    attempt: Id<normalized::bindings::CallBindingAttempt>,
    caller: Id<EntityRef>,
    callee: Id<EntityRef>,
    declaration: Id<Occurrence>,
    qualification: Id<AssertionQualification>,
    status: EvidenceStatus,
    premises: Rows<NativeAssertionPremise>,
    arguments: Vec<(Id<calls::SignatureParameter>, Id<Occurrence>)>,
    captures: Vec<super::capture_bridge::CheckedCaptureOrigin>,
    _charge: charged::StateCharge,
}
impl CheckedSourceBinding {
    pub(super) fn captures(&self) -> &[super::capture_bridge::CheckedCaptureOrigin] {
        &self.captures
    }
    pub fn arguments(&self) -> &[(Id<calls::SignatureParameter>, Id<Occurrence>)] {
        &self.arguments
    }
    pub fn request(&self) -> SourceCallRequest {
        self.request
    }
    pub fn attempt(&self) -> Id<normalized::bindings::CallBindingAttempt> {
        self.attempt
    }
    pub fn caller(&self) -> Id<EntityRef> {
        self.caller
    }
    pub fn callee(&self) -> Id<EntityRef> {
        self.callee
    }
    pub fn declaration(&self) -> Id<Occurrence> {
        self.declaration
    }
    pub fn qualification(&self) -> Id<AssertionQualification> {
        self.qualification
    }
    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn premises(&self) -> &Rows<NativeAssertionPremise> {
        &self.premises
    }
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, obligation::ObligationKind> {
    rows.get(id)
        .ok_or(obligation::ObligationKind::MissingEvidence)
}
fn same(left: &Occurrence, right: &Occurrence) -> bool {
    (
        left.source,
        left.start,
        left.end,
        left.syntax_kind,
        &left.structural_path,
    ) == (
        right.source,
        right.start,
        right.end,
        right.syntax_kind,
        &right.structural_path,
    )
}
pub(super) struct Evidence<'a> {
    data: &'a EvaluationData,
    request: SourceCallRequest,
    source: Id<SourceArtifact>,
    rows: Rows<NativeAssertionPremise>,
    status: EvidenceStatus,
}
impl Evidence<'_> {
    pub(super) fn include<R: Record>(
        &mut self,
        row: &R,
        qualification: Id<AssertionQualification>,
        selected: Option<&NativeAssertionPremise>,
    ) -> Result<(), super::evaluation::EvaluationError> {
        use super::evaluation::{EvaluationError, boundary};
        let q = need(&self.data.qualifications, qualification).map_err(boundary)?;
        let artifact = need(&self.data.artifacts, self.source).map_err(boundary)?;
        if artifact.input != self.request.input || q.context != self.request.context {
            return Err(boundary(obligation::ObligationKind::IncompatibleContexts));
        }
        if q.condition != conditions::Diagram::always().id()
            || q.modality != Modality::Definite
            || q.approximation != Approximation::Exact
            || q.assumptions != assumptions::AssumptionSet::empty_id()
        {
            return Err(boundary(obligation::ObligationKind::Approximation));
        }
        let covered = match need(&self.data.scopes, q.scope).map_err(boundary)? {
            CoverageScope::Input { input } => *input == self.request.input,
            CoverageScope::Artifact { artifact } => *artifact == self.source,
            CoverageScope::Module { module } => {
                need(&self.data.modules, *module).map_err(boundary)?.source == self.source
            }
            _ => false,
        };
        if !covered {
            return Err(boundary(obligation::ObligationKind::MissingEvidence));
        }
        let mut found = false;
        for native in self.data.native.iter() {
            let premise = need(&self.data.premises, native.premise).map_err(boundary)?;
            if premise.assertion_and_support().0 != derivation::RowRef::of(row.id())
                || selected.is_some_and(|selected| premise != selected)
            {
                continue;
            }
            if native.qualification != qualification {
                return Err(boundary(obligation::ObligationKind::MissingEvidence));
            }
            analysis::policy::behavioral_support(native.status, false)
                .map_err(|_| boundary(obligation::ObligationKind::MissingEvidence))?;
            self.rows
                .insert(premise.clone())
                .map_err(EvaluationError::Model)?;
            self.status = analysis::support::inferred_status(
                analysis::Interpretation::Structural,
                [self.status, native.status],
            );
            found = true;
        }
        if !found {
            return Err(boundary(obligation::ObligationKind::MissingEvidence));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) enum HeaderError {
    Boundary(obligation::ObligationKind),
    Model(ModelError),
}
impl From<obligation::ObligationKind> for HeaderError {
    fn from(value: obligation::ObligationKind) -> Self {
        Self::Boundary(value)
    }
}
impl From<ModelError> for HeaderError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}
impl From<super::evaluation::EvaluationError> for HeaderError {
    fn from(value: super::evaluation::EvaluationError) -> Self {
        match value {
            super::evaluation::EvaluationError::Boundary(r) => Self::Boundary(r),
            super::evaluation::EvaluationError::Model(e) => Self::Model(e),
        }
    }
}
pub(super) fn unique<'a, T>(
    mut values: impl Iterator<Item = &'a T>,
) -> Result<&'a T, obligation::ObligationKind> {
    let first = values
        .next()
        .ok_or(obligation::ObligationKind::MissingEvidence)?;
    if values.next().is_some() {
        return Err(obligation::ObligationKind::AmbiguousBinding);
    }
    Ok(first)
}
fn supported<S: assertion::Support>(
    rows: &Rows<S>,
    assertion: Id<S::Assertion>,
    run: Id<attribution::ProviderRun>,
) -> Result<&S, obligation::ObligationKind> {
    unique(
        rows.iter().filter(|s| {
            s.assertion() == assertion && s.attribution().is_some_and(|a| a.run == run)
        }),
    )
}
impl CheckedSourceBinding {
    /// Replayed normalized admission is necessary but does not establish fresh runtime availability.
    #[allow(
        clippy::too_many_arguments,
        reason = "Public checked source-call proof keeps independent evidence owners, request, completion authority and budget explicit."
    )]
    pub fn derive(
        data: &EvaluationData,
        flow: &EntryData,
        bindings: &BindingData,
        output: &normalized::binding_normalization::BindingOutput,
        checked: &ValidatedBoundCall,
        admission: &CompositionAdmission,
        request: SourceCallRequest,
        budget: &ResourceBudget,
    ) -> Result<Result<Self, obligation::ObligationKind>, ModelError> {
        use lexical::SyntaxField;
        use obligation::ObligationKind as K;
        let result = (|| -> Result<Self, HeaderError> {
            let mut charge = charged::StateCharge::new(budget, "fresh-source-binding");
            charge.grow(size_of::<Self>())?;
            if checked.attempt() != admission.attempt()
                || checked.event() != request.event
                || admission.event() != request.event
                || checked.context() != request.context
                || admission.phase() != calls::CallPhase::Call
            {
                return Err(K::IncompatibleContexts.into());
            }
            let event = need(&bindings.event_events, request.event)?;
            if event.context != request.context
                || event.site != checked.bound().site()
                || event.owner != admission.owner()
            {
                return Err(K::IncompatibleContexts.into());
            }
            let attempt = need(&output.attempts, checked.attempt())?;
            if attempt.event != request.event || checked.bound().target() != admission.target() {
                return Err(K::IncompatibleContexts.into());
            }
            let owner = need(&bindings.owners, admission.owner())?;
            if owner.entity != admission.owner_entity() {
                return Err(K::IncompatibleContexts.into());
            }
            let declaration = match need(&bindings.refs, admission.callee())? {
                EntityRef::Callable { callable } => match need(&bindings.callables, *callable)? {
                    CallableEntity::Source {
                        declaration,
                        kind: CallableKind::Function,
                    } => *declaration,
                    _ => return Err(K::NoSourceDeclaration.into()),
                },
                _ => return Err(K::NoSourceDeclaration.into()),
            };
            let caller = match need(&bindings.refs, admission.owner_entity())? {
                EntityRef::Callable { callable } => match need(&bindings.callables, *callable)? {
                    CallableEntity::Source {
                        declaration,
                        kind: CallableKind::Function,
                    } => *declaration,
                    _ => return Err(K::NoSourceDeclaration.into()),
                },
                _ => return Err(K::NoSourceDeclaration.into()),
            };
            if caller != admission.owner_declaration() || declaration == caller {
                return Err(K::NoSourceDeclaration.into());
            }
            let site = need(&bindings.occurrences, event.site)?;
            let declared = need(&bindings.occurrences, declaration)?;
            if site.source != declared.source
                || need(&bindings.artifacts, site.source)?.input != request.input
            {
                return Err(K::IncompatibleContexts.into());
            }
            let native_declaration = unique(bindings.declarations.iter().filter(|d| {
                d.declaration == declaration
                    && bindings
                        .qualifications
                        .get(d.qualification)
                        .is_some_and(|q| q.context == request.context)
            }))?;
            if native_declaration.kind != DeclarationKind::Function
                || native_declaration.parent != Some(caller)
            {
                return Err(K::NoSourceDeclaration.into());
            }
            // Retain the complete checked mapping; default/projection/receiver availability is separate.
            let mut arguments = Vec::new();
            charge.grow(
                checked.bound().bindings().len()
                    * size_of::<(Id<calls::SignatureParameter>, Id<Occurrence>)>()
                    * 2,
            )?;
            for binding in checked.bound().bindings() {
                match (&binding.source, &binding.projection) {
                    (
                        calls::BindingSource::Actual { occurrence },
                        calls::BindingProjection::Whole,
                    ) => arguments.push((binding.formal, *occurrence)),
                    (calls::BindingSource::EmptyVarargs | calls::BindingSource::EmptyKwargs, _) => {
                    }
                    _ => return Err(K::DefaultUnavailable.into()),
                }
            }
            for parameter in bindings
                .parameters
                .iter()
                .filter(|p| p.signature == checked.bound().signature())
            {
                let shape = need(&bindings.shapes, parameter.shape)?;
                if !shape.required
                    || matches!(
                        shape.kind,
                        calls::ParameterKind::VarPositional | calls::ParameterKind::VarKeyword
                    )
                {
                    return Err(K::DefaultUnavailable.into());
                }
            }
            let syntax = need(&bindings.syntax, attempt.syntax.ok_or(K::MissingEvidence)?)?;
            if syntax.site != event.site || syntax.in_annotation {
                return Err(K::UnsupportedUnpacking.into());
            }
            if bindings
                .decorators
                .iter()
                .any(|d| d.declaration == declaration)
            {
                return Err(K::DefaultUnavailable.into());
            }
            // Header children are independently checked; body children cannot prove header availability.
            for placement in bindings
                .placements
                .iter()
                .filter(|p| p.parent == Some(declaration))
            {
                let child = need(&bindings.occurrences, placement.occurrence)?;
                if placement.field != SyntaxField::Body
                    && !matches!(
                        child.syntax_kind,
                        SyntaxKind::Identifier | SyntaxKind::Parameters
                    )
                {
                    return Err(K::DefaultUnavailable.into());
                }
                if child.syntax_kind == SyntaxKind::Parameters {
                    for p in bindings
                        .placements
                        .iter()
                        .filter(|p| p.parent == Some(child.id()))
                    {
                        let parameter = need(&bindings.occurrences, p.occurrence)?;
                        if parameter.syntax_kind != SyntaxKind::ParameterWithDefault {
                            return Err(K::DefaultUnavailable.into());
                        }
                        for part in bindings
                            .placements
                            .iter()
                            .filter(|part| part.parent == Some(parameter.id()))
                        {
                            let value = need(&bindings.occurrences, part.occurrence)?;
                            if !matches!(
                                value.syntax_kind,
                                SyntaxKind::Parameter | SyntaxKind::Identifier
                            ) {
                                return Err(K::DefaultUnavailable.into());
                            }
                        }
                    }
                }
            }
            let mut actuals = bindings
                .arguments
                .iter()
                .filter(|a| a.call == syntax.id())
                .collect::<Vec<_>>();
            charge.grow(actuals.len() * size_of::<&calls::CallArgument>() * 2)?;
            actuals.sort_by_key(|a| a.ordinal);
            if actuals.len() != arguments.len()
                || actuals.iter().enumerate().any(|(i, a)| {
                    a.ordinal != i as i64
                        || !matches!(
                            a.kind,
                            calls::ArgumentKind::Positional | calls::ArgumentKind::Keyword
                        )
                        || arguments
                            .iter()
                            .filter(|(_, site)| *site == a.value)
                            .count()
                            != 1
                })
            {
                return Err(K::UnsupportedUnpacking.into());
            }
            arguments.sort_by_key(|(_, site)| actuals.iter().position(|a| a.value == *site));
            let mut evidence = Evidence {
                data,
                request,
                source: site.source,
                rows: Rows::new(budget),
                status: EvidenceStatus::StructurallyObserved,
            };
            if let normalized::binding_normalization::SourceBodySignatureClosure::DeclaredEnumeration {
                enumeration, members, support,
            } = admission.signature_closure() {
                let header = need(&bindings.signature_enumerations, enumeration)?;
                if header.members != members { return Err(K::MissingEvidence.into()) }
                let premise = NativeAssertionPremise::SignatureEnumerationObservation {
                    assertion: enumeration,
                    support,
                };
                evidence.include(header, header.qualification, Some(&premise))?;
                for member in bindings.signature_enumeration_members.iter().filter(|m| m.enumeration == enumeration) {
                    let signature = need(&bindings.signatures, member.signature)?;
                    evidence.include(signature, signature.qualification, None)?;
                }
            }
            evidence.include(native_declaration, native_declaration.qualification, None)?;
            evidence.include(syntax, syntax.qualification, None)?;
            // Direct adjacent suite statements ensure no intervening action can replace the fresh binding.
            let definition_placement = unique(bindings.placements.iter().filter(|p| {
                p.occurrence == declaration
                    && p.parent == Some(caller)
                    && p.field == SyntaxField::Body
            }))?;
            evidence.include(
                definition_placement,
                definition_placement.qualification,
                None,
            )?;
            let mut current = event.site;
            let mut remaining = bindings.placements.len() + 1;
            let call_statement = loop {
                if remaining == 0 {
                    return Err(K::ExpressionDepthLimit.into());
                }
                remaining -= 1;
                let placement = unique(
                    bindings
                        .placements
                        .iter()
                        .filter(|p| p.occurrence == current),
                )?;
                evidence.include(placement, placement.qualification, None)?;
                if placement.parent == Some(caller) && placement.field == SyntaxField::Body {
                    break placement;
                }
                current = placement.parent.ok_or(K::ScopeBoundary)?;
            };
            if call_statement.ordinal != definition_placement.ordinal + 1 {
                return Err(K::EntryValueUnknown.into());
            }
            let mut literal_prefix = None;
            if definition_placement.ordinal != 0 {
                if definition_placement.ordinal != 1 {
                    return Err(K::EntryValueUnknown.into());
                }
                let prefix = unique(bindings.placements.iter().filter(|p| {
                    p.parent == Some(caller) && p.field == SyntaxField::Body && p.ordinal == 0
                }))?;
                let statement = need(&bindings.occurrences, prefix.occurrence)?;
                if statement.syntax_kind == SyntaxKind::StmtAssign {
                    let target = unique(data.placements.iter().filter(|p| {
                        p.parent == Some(statement.id()) && p.field == SyntaxField::Target
                    }))?;
                    let value = unique(data.placements.iter().filter(|p| {
                        p.parent == Some(statement.id()) && p.field == SyntaxField::Value
                    }))?;
                    if !super::capture_bridge::closed_literal_prefix(
                        data,
                        statement.id(),
                        target.occurrence,
                        value.occurrence,
                    )? {
                        return Err(K::EntryValueUnknown.into());
                    }
                    literal_prefix = Some((statement.id(), value.occurrence));
                } else {
                    let caller_declaration = unique(bindings.declarations.iter().filter(|d| {
                        d.declaration == caller
                            && bindings
                                .qualifications
                                .get(d.qualification)
                                .is_some_and(|q| q.context == request.context)
                    }))?;
                    let docstring = need(
                        &bindings.occurrences,
                        caller_declaration.docstring.ok_or(K::EntryValueUnknown)?,
                    )?;
                    if statement.syntax_kind != SyntaxKind::StmtExpr {
                        return Err(K::EntryValueUnknown.into());
                    }
                    let value = unique(bindings.placements.iter().filter(|p| {
                        p.parent == Some(statement.id()) && p.field == SyntaxField::Value
                    }))?;
                    let literal = need(&bindings.occurrences, value.occurrence)?;
                    if literal.syntax_kind != SyntaxKind::ExprStringLiteral
                        || literal.source != docstring.source
                        || literal.start > docstring.start
                        || literal.end < docstring.end
                    {
                        return Err(K::EntryValueUnknown.into());
                    }
                    evidence.include(caller_declaration, caller_declaration.qualification, None)?;
                    evidence.include(value, value.qualification, None)?;
                }
                evidence.include(prefix, prefix.qualification, None)?;
            }
            let mut captures = Vec::new();
            // Captures outside the bounded own-frame lane retain their original refusal.
            for resolution in bindings.lexical_resolutions.iter().filter(|r| {
                r.captured
                    && bindings
                        .qualifications
                        .get(r.qualification)
                        .is_some_and(|q| q.context == request.context)
            }) {
                let read = need(&bindings.occurrences, resolution.read)?;
                if read.source == declared.source
                    && read.start >= declared.start
                    && read.end <= declared.end
                    && read.structural_path.starts_with(&declared.structural_path)
                {
                    let proof = super::capture_bridge::CheckedCaptureOrigin::derive(
                        data,
                        flow,
                        bindings,
                        request,
                        admission.owner_entity(),
                        admission.callee(),
                        caller,
                        declaration,
                        resolution,
                        &mut evidence,
                        budget,
                    )?;
                    charge.grow(size_of::<super::capture_bridge::CheckedCaptureOrigin>() * 2)?;
                    captures.push(proof);
                }
            }
            if literal_prefix.is_some_and(|prefix| {
                !captures
                    .iter()
                    .any(|origin| origin.literal_prefix() == Some(prefix))
            }) {
                return Err(K::CapturedStateUnavailable.into());
            }
            let callee = need(&bindings.occurrences, syntax.callee)?;
            let use_ = unique(flow.uses.iter().filter(|u| {
                flow.occurrences
                    .get(u.occurrence)
                    .is_some_and(|o| same(o, callee))
            }))?;
            let use_observation = unique(flow.use_observations.iter().filter(|o| {
                o.use_ == use_.id()
                    && !o.annotation
                    && flow
                        .qualifications
                        .get(o.qualification)
                        .is_some_and(|q| q.context == request.context)
            }))?;
            let scope = need(&flow.lexical_scopes, use_observation.scope)?;
            if scope.kind != LexicalScopeKind::Function || scope.owner != caller {
                return Err(K::ScopeBoundary.into());
            }
            let use_support = unique(flow.use_supports.iter().filter(|s| {
                s.assertion == use_observation.id()
                    && flow
                        .runs
                        .get(s.run)
                        .is_some_and(|r| r.input == request.input && r.context == request.context)
            }))?;
            let run = need(&flow.runs, use_support.run)?;
            evidence.include(
                use_observation,
                use_observation.qualification,
                Some(&NativeAssertionPremise::Use {
                    assertion: use_observation.id(),
                    support: use_support.id(),
                }),
            )?;
            let reaching = unique(flow.reaching.iter().filter(|r| {
                r.use_ == use_.id()
                    && flow
                        .qualifications
                        .get(r.qualification)
                        .is_some_and(|q| q.context == request.context)
                    && flow
                        .reaching_supports
                        .iter()
                        .any(|s| s.assertion == r.id() && s.run == run.id())
            }))?;
            if reaching.loop_carried {
                return Err(K::EntryValueUnknown.into());
            }
            let definition = match need(&flow.targets, reaching.target)? {
                ReachingDefinition::Bound { definition } => need(&flow.definitions, *definition)?,
                _ => return Err(K::EntryValueUnknown.into()),
            };
            let reaching_support = supported(&flow.reaching_supports, reaching.id(), run.id())?;
            evidence.include(
                reaching,
                reaching.qualification,
                Some(&NativeAssertionPremise::Reaching {
                    assertion: reaching.id(),
                    support: reaching_support.id(),
                }),
            )?;
            if !same(
                need(&flow.occurrences, definition.occurrence)?,
                need(&bindings.occurrences, native_declaration.name)?,
            ) || definition.place != use_.place
            {
                return Err(K::EntryValueUnknown.into());
            }
            let definition_observation = unique(flow.definition_observations.iter().filter(|d| {
                d.definition == definition.id()
                    && d.scope == use_observation.scope
                    && flow
                        .definition_supports
                        .iter()
                        .any(|s| s.assertion == d.id() && s.run == run.id())
            }))?;
            if definition_observation.kind != BindingEventKind::FunctionDef {
                return Err(K::EntryValueUnknown.into());
            }
            let support = supported(
                &flow.definition_supports,
                definition_observation.id(),
                run.id(),
            )?;
            evidence.include(
                definition_observation,
                definition_observation.qualification,
                Some(&NativeAssertionPremise::Definition {
                    assertion: definition_observation.id(),
                    support: support.id(),
                }),
            )?;
            let mut complete = false;
            for c in flow.coverage.iter().filter(|c| {
                c.run == Some(run.id())
                    && c.provider == Some(run.provider)
                    && c.context == request.context
                    && c.family == FactFamily::Flow
            }) {
                let relevant = match need(&flow.scopes, c.scope)? {
                    CoverageScope::Input { input } => *input == request.input,
                    CoverageScope::Artifact { artifact } => *artifact == site.source,
                    CoverageScope::Module { module } => {
                        need(&flow.modules, *module)?.source == site.source
                    }
                    _ => false,
                };
                if relevant {
                    if c.status != attribution::CoverageStatus::CompleteUnderStatedModel {
                        return Err(K::IncompleteCoverage.into());
                    }
                    complete = true;
                }
            }
            if !complete {
                return Err(K::IncompleteCoverage.into());
            }
            Ok(Self {
                request,
                attempt: checked.attempt(),
                caller: admission.owner_entity(),
                callee: admission.callee(),
                declaration,
                qualification: syntax.qualification,
                status: evidence.status,
                premises: evidence.rows,
                arguments,
                captures,
                _charge: charge,
            })
        })();
        match result {
            Ok(v) => Ok(Ok(v)),
            Err(HeaderError::Boundary(r)) => Ok(Err(r)),
            Err(HeaderError::Model(e)) => Err(e),
        }
    }
}

pub fn stage(
    profile: stages::Profile,
    definition: &analysis::AnalysisDefinition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    super::source_call_records::stage(profile, definition, model, order)
}
