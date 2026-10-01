//! Deterministic contextual evidence closure, shared by publication and independent controls.
use super::*;
use crate::domain::{
    assertion::Evidence,
    attribution::*,
    charged::{ChargedSet, StateCharge},
    input::*,
    normalized::Rows,
    resources::ResourceBudget,
    source::*,
    stages::*,
};
pub struct EvidenceData {
    pub core: catalog::build::CatalogData,
    pub catalog: catalog::build::CatalogOutput,
    pub facts: EvidenceFacts,
    pub runtime: super::runtime::RuntimeData,
}
impl EvidenceData {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            core: catalog::build::CatalogData::new(b),
            catalog: catalog::build::CatalogOutput::new(b),
            facts: EvidenceFacts::new(b),
            runtime: super::runtime::RuntimeData::new(b),
        }
    }
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        let core = self.core.visit(name, batch)?;
        let catalog = self.catalog.visit(name, batch)?;
        let facts = self.facts.visit(name, batch)?;
        let runtime = self.runtime.visit(name, batch)?;
        Ok(core || catalog || facts || runtime)
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut rows = catalog::build::CatalogData::validation_inputs();
        rows.extend(catalog::build::CatalogOutput::validation_inputs());
        rows.extend(EvidenceFacts::inputs());
        rows.extend(super::runtime::RuntimeData::inputs());
        rows.sort_by_key(|r| r.name());
        rows.dedup_by_key(|r| r.name());
        rows
    }
}
macro_rules! data {($($f:ident:$ty:ty,)*)=>{
 pub struct EvidenceFacts {$(pub $f:Rows<$ty>,)*}
 impl EvidenceFacts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$f.decode(batch)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}fn stage_inputs()->Vec<RelationUse> {vec![$(RelationUse::stored::<$ty>()),*]}}
};}
crate::catalog_evidence_inputs!(data);
macro_rules! output {($($f:ident:$ty:ty,)*)=>{
 pub struct EvidenceOutput {$(pub $f:Rows<$ty>,)*}
 impl EvidenceOutput {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$f.decode(batch)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}pub fn matches(&self,other:&Self)->Result<(),ModelError> {$(if !self.$f.same(&other.$f) {return Err(invalid(format!("catalog evidence closure differs: {}",<$ty>::NAME)));})*Ok(())}}
};}
crate::catalog_evidence_outputs!(output);
pub(super) fn invalid(msg: impl Into<String>) -> ModelError {
    ModelError::Invalid(msg.into())
}
pub(super) fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("catalog evidence premise absent: {}", R::NAME)))
}
pub(super) fn qualification(
    data: &EvidenceData,
    id: Id<assertion::AssertionQualification>,
) -> Result<&assertion::AssertionQualification, ModelError> {
    need(&data.core.qualifications, id)
}
pub(super) fn exact(q: &assertion::AssertionQualification, context: Id<AnalysisContext>) -> bool {
    q.context == context
        && q.approximation == assertion::Approximation::Exact
        && q.modality == Modality::Definite
}
fn root(
    out: &mut EvidenceOutput,
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
    subject: RootSubject,
) -> Result<(), ModelError> {
    let subject = out.subjects.insert(subject)?;
    out.roots.insert(EvidenceRoot {
        input,
        context,
        subject,
    })?;
    Ok(())
}
fn span(
    out: &mut EvidenceOutput,
    scenario: Id<CatalogScenario>,
    ordinal: i64,
    role: SpanRole,
    source: OriginalSource,
) -> Result<(), ModelError> {
    let source = out.original_sources.insert(source)?;
    out.spans.insert(ScenarioSpan {
        scenario,
        ordinal,
        role,
        source,
    })?;
    Ok(())
}
fn dependency(
    out: &mut EvidenceOutput,
    scenario: Id<CatalogScenario>,
    dependency: SetupDependency,
) -> Result<(), ModelError> {
    let dependency = out.setup.insert(dependency)?;
    out.dependencies.insert(ScenarioDependency {
        scenario,
        dependency,
        status: CheckStatus::Blocked,
    })?;
    Ok(())
}
pub(super) fn span_artifact(
    data: &EvidenceData,
    id: EvidenceSourceSpanId,
) -> Result<Id<SourceArtifact>, ModelError> {
    match need(&data.facts.canonical_evidence, id.id())? {
        Evidence::SourceSpan { source, .. } => Ok(*source),
        _ => Err(invalid("original span has wrong canonical evidence kind")),
    }
}
fn parse(
    data: &EvidenceData,
    artifact: Id<SourceArtifact>,
    context: Id<AnalysisContext>,
) -> CheckStatus {
    let scope = CoverageScope::Artifact { artifact }.id();
    let mut status = None;
    for row in data
        .core
        .native_coverage
        .iter()
        .filter(|r| r.scope == scope && r.context == context && r.family == FactFamily::Syntax)
    {
        let next = if row.reason == Some(obligation::ObligationKind::SyntaxError)
            || row.status == CoverageStatus::Failed
        {
            CheckStatus::Failed
        } else if row.status == CoverageStatus::CompleteUnderStatedModel {
            CheckStatus::Passed
        } else {
            CheckStatus::Blocked
        };
        status = Some(if status.is_some_and(|old| old != next) {
            CheckStatus::Blocked
        } else {
            next
        });
    }
    status.unwrap_or(CheckStatus::Blocked)
}
fn selected(data: &EvidenceData, artifact: Id<SourceArtifact>, role: SourceRole) -> bool {
    data.facts.uses.iter().any(|r| {
        r.artifact == artifact
            && r.role == role
            && data
                .core
                .artifacts
                .get(artifact)
                .is_some_and(|a| a.input == r.input)
    })
}
fn source_artifact(
    data: &EvidenceData,
    source: &ScenarioSource,
) -> Result<(Id<SourceArtifact>, Id<AnalysisContext>), ModelError> {
    match source {
        ScenarioSource::Python {
            artifact, context, ..
        } => Ok((*artifact, *context)),
        ScenarioSource::Fence { observation } => {
            let block = need(&data.facts.blocks, *observation)?;
            let context = qualification(data, block.qualification)?.context;
            let artifact = block.materialized.unwrap_or(span_artifact(
                data,
                need(&data.facts.nodes, block.block.id())?.span(),
            )?);
            Ok((artifact, context))
        }
    }
}
fn execution(
    data: &EvidenceData,
    artifact: Id<SourceArtifact>,
    context: Id<AnalysisContext>,
) -> Result<CheckStatus, ModelError> {
    let mut found = None;
    for row in data
        .facts
        .report_observations
        .iter()
        .filter(|r| r.target == artifact)
    {
        if !exact(qualification(data, row.qualification)?, context) {
            continue;
        }
        let next = need(&data.facts.reports, row.report)?.execution;
        found = Some(if found.is_some_and(|old| old != next) {
            CheckStatus::Blocked
        } else {
            next
        });
    }
    Ok(found.unwrap_or(CheckStatus::NotRun))
}
fn scenario(
    data: &EvidenceData,
    out: &mut EvidenceOutput,
    source: ScenarioSource,
) -> Result<Id<CatalogScenario>, ModelError> {
    let (artifact, context) = source_artifact(data, &source)?;
    let source_id = out.scenario_sources.insert(source.clone())?;
    let intent = match &source {
        ScenarioSource::Python {
            role: SourceRole::Test,
            ..
        } => Intent::AssertionTest,
        _ => Intent::Demonstration,
    };
    let parse = match &source {
        ScenarioSource::Python { .. } => parse(data, artifact, context),
        ScenarioSource::Fence { observation } => {
            let block = need(&data.facts.blocks, *observation)?;
            if block.materialized.is_some() {
                parse(data, artifact, context)
            } else {
                CheckStatus::NotRun
            }
        }
    };
    let id = out.scenarios.insert(CatalogScenario {
        source: source_id,
        extraction: CheckStatus::Passed,
        parse,
        binding: CheckStatus::NotRun,
        environment: CheckStatus::NotRun,
        execution: execution(data, artifact, context)?,
        intent,
    })?;
    match &source {
        ScenarioSource::Python {
            artifact,
            declaration,
            role,
            ..
        } => {
            if let Some(declaration) = declaration {
                let row = need(&data.core.declarations, *declaration)?;
                span(
                    out,
                    id,
                    0,
                    SpanRole::Primary,
                    OriginalSource::Occurrence {
                        occurrence: row.declaration,
                    },
                )?;
                span(
                    out,
                    id,
                    1,
                    SpanRole::EnclosingModule,
                    OriginalSource::Artifact {
                        artifact: *artifact,
                    },
                )?;
                dependency(
                    out,
                    id,
                    SetupDependency::ModuleContext {
                        artifact: *artifact,
                    },
                )?;
                for syntax in data.core.parameter_syntax.iter().filter(|r| {
                    r.function == row.declaration
                        && data
                            .core
                            .qualifications
                            .get(r.qualification)
                            .is_some_and(|q| q.context == context)
                }) {
                    dependency(
                        out,
                        id,
                        SetupDependency::Formal {
                            syntax: syntax.id(),
                        },
                    )?;
                }
            } else {
                span(
                    out,
                    id,
                    0,
                    SpanRole::Primary,
                    OriginalSource::Artifact {
                        artifact: *artifact,
                    },
                )?;
            }
            if *role == SourceRole::Test {
                dependency(
                    out,
                    id,
                    SetupDependency::TestEnvironment {
                        artifact: *artifact,
                    },
                )?;
            }
        }
        ScenarioSource::Fence { observation } => {
            let block = need(&data.facts.blocks, *observation)?;
            span(
                out,
                id,
                0,
                SpanRole::Primary,
                OriginalSource::Span {
                    span: need(&data.facts.nodes, block.block.id())?.span(),
                },
            )?;
            span(
                out,
                id,
                1,
                SpanRole::EnclosingPassage,
                OriginalSource::Span {
                    span: need(&data.facts.nodes, block.passage.id())?.span(),
                },
            )?;
            if let Some(artifact) = block.materialized {
                span(
                    out,
                    id,
                    2,
                    SpanRole::ExtractedPython,
                    OriginalSource::Artifact { artifact },
                )?;
                dependency(out, id, SetupDependency::ModuleContext { artifact })?;
            }
        }
    }
    dependency(out, id, SetupDependency::RuntimeInputs { artifact })?;
    root(
        out,
        need(&data.core.artifacts, artifact)?.input,
        context,
        RootSubject::Scenario { scenario: id },
    )?;
    Ok(id)
}
fn scenarios_for(
    data: &EvidenceData,
    out: &EvidenceOutput,
    event: &NormalizedCallEvent,
    b: &ResourceBudget,
) -> Result<Rows<CatalogScenario>, ModelError> {
    let source = need(&data.core.occurrences, event.site)?.source;
    let owner = need(&data.core.ownership, event.owner)?;
    let declaration = match need(&data.core.refs, owner.entity)? {
        EntityRef::Callable { callable } => match data.core.source_callables.get(*callable) {
            Some(CallableEntity::Source { declaration, .. }) => Some(*declaration),
            _ => None,
        },
        _ => None,
    };
    let mut rows = Rows::new(b);
    for row in out.scenarios.iter() {
        let matched = match need(&out.scenario_sources, row.source)? {
            ScenarioSource::Python {
                artifact,
                context,
                declaration: decl,
                ..
            } if *artifact == source && *context == event.context => {
                decl.and_then(|id| data.core.declarations.get(id).map(|r| r.declaration))
                    == declaration
            }
            ScenarioSource::Fence { observation } => {
                let block = need(&data.facts.blocks, *observation)?;
                block.materialized == Some(source)
                    && qualification(data, block.qualification)?.context == event.context
            }
            _ => false,
        };
        if matched {
            rows.insert(row.clone())?;
        }
    }
    Ok(rows)
}
pub fn build(data: &EvidenceData, b: &ResourceBudget) -> Result<EvidenceOutput, ModelError> {
    let mut out = EvidenceOutput::new(b);
    let mut charge = StateCharge::new(b, "catalog-evidence-frames");
    let mut frames = ChargedSet::default();
    for link in data.facts.core_links.iter() {
        let member = need(&data.catalog.members, link.member)?;
        let invocation = need(&data.facts.core_invocations, link.invocation)?;
        if invocation.input != member.input {
            return Err(invalid("C0 member invocation has foreign owning input"));
        }
        let module = need(&data.core.modules, member.access)?;
        let artifact = need(&data.core.artifacts, module.source)?;
        if artifact.input != member.input {
            return Err(invalid(
                "C0 public slot access module has foreign owning input",
            ));
        }
        out.original_sources.insert(OriginalSource::Artifact {
            artifact: module.source,
        })?;
        frames.insert(&mut charge, (member.id(), invocation.context))?;
        root(
            &mut out,
            member.input,
            invocation.context,
            RootSubject::Member {
                member: member.id(),
            },
        )?;
    }
    for option in data.catalog.options.iter() {
        let member = need(&data.catalog.members, option.member)?;
        for (_, context) in frames.iter().filter(|(m, _)| *m == member.id()) {
            root(
                &mut out,
                member.input,
                *context,
                RootSubject::Option {
                    option: option.id(),
                },
            )?;
        }
    }
    let mut scopes = ChargedSet::default();
    for coverage in data
        .core
        .native_coverage
        .iter()
        .filter(|r| r.family == FactFamily::Syntax)
    {
        for artifact in data
            .core
            .artifacts
            .iter()
            .filter(|a| CoverageScope::Artifact { artifact: a.id() }.id() == coverage.scope)
        {
            scopes.insert(&mut charge, (artifact.id(), coverage.context))?;
        }
    }
    for (artifact, context) in scopes.iter() {
        for usage in data.facts.uses.iter().filter(|r| {
            r.artifact == *artifact
                && data
                    .core
                    .artifacts
                    .get(*artifact)
                    .is_some_and(|a| a.input == r.input)
                && matches!(
                    r.role,
                    SourceRole::Release | SourceRole::Example | SourceRole::Test
                )
        }) {
            scenario(
                data,
                &mut out,
                ScenarioSource::Python {
                    artifact: *artifact,
                    role: usage.role,
                    context: *context,
                    declaration: None,
                },
            )?;
            for declaration in data.core.declarations.iter().filter(|r| {
                matches!(
                    r.kind,
                    DeclarationKind::Function | DeclarationKind::AsyncFunction
                )
            }) {
                if need(&data.core.occurrences, declaration.declaration)?.source == *artifact
                    && qualification(data, declaration.qualification)?.context == *context
                {
                    scenario(
                        data,
                        &mut out,
                        ScenarioSource::Python {
                            artifact: *artifact,
                            role: usage.role,
                            context: *context,
                            declaration: Some(declaration.id()),
                        },
                    )?;
                }
            }
        }
    }
    for block in data.facts.blocks.iter() {
        let original = span_artifact(data, need(&data.facts.nodes, block.block.id())?.span())?;
        if selected(data, original, SourceRole::Document) {
            scenario(
                data,
                &mut out,
                ScenarioSource::Fence {
                    observation: block.id(),
                },
            )?;
        }
    }
    for document in data.facts.documents.iter() {
        if selected(data, document.source, SourceRole::Document) {
            root(
                &mut out,
                need(&data.core.artifacts, document.source)?.input,
                qualification(data, document.qualification)?.context,
                RootSubject::Document {
                    observation: document.id(),
                },
            )?;
        }
    }
    for candidate in data.facts.mention_candidates.iter() {
        let exposure = need(&data.core.exposures, candidate.exposure)?;
        let assessment = need(&data.facts.mention_assessments, candidate.assessment)?;
        let mention = need(&data.facts.mentions, assessment.observation)?;
        let q = qualification(data, mention.qualification)?;
        let artifact = span_artifact(data, need(&data.facts.nodes, mention.mention.id())?.span())?;
        if q.context != exposure.context || !selected(data, artifact, SourceRole::Document) {
            continue;
        }
        for link in data.catalog.exposures.iter().filter(|r| {
            r.exposure == exposure.id()
                && data
                    .catalog
                    .members
                    .get(r.member)
                    .is_some_and(|m| m.path.len() == 1)
        }) {
            out.document_associations.insert(DocumentAssociation {
                member: link.member,
                candidate: candidate.id(),
                basis: AssociationBasis::DocumentCandidate,
            })?;
        }
    }
    for event in data.facts.events.iter() {
        let scenarios = scenarios_for(data, &out, event, b)?;
        for scenario_row in scenarios.iter() {
            let scenario = scenario_row.id();
            let intent = super::intent::classify(data, event, scenario_row.intent)?;
            for alternative in data
                .facts
                .alternatives
                .iter()
                .filter(|r| r.event == event.id())
            {
                let raw = need(
                    &data.facts.raw_targets,
                    need(&data.facts.alternative_sources, alternative.source)?.target(),
                )?;
                let q = qualification(data, raw.qualification)?;
                if q.context != event.context {
                    return Err(invalid("scenario alternative has foreign qualification"));
                }
                let Some(entity) = alternative.entity else {
                    continue;
                };
                let EntityRef::Callable { callable } = need(&data.core.refs, entity)? else {
                    continue;
                };
                for catalog in data.catalog.callables.iter().filter(|r| {
                    data.core
                        .assessments
                        .get(r.assessment)
                        .is_some_and(|a| a.callable == *callable && a.context == event.context)
                }) {
                    let member = need(&data.catalog.members, catalog.member)?;
                    if !frames.contains(&(member.id(), event.context)) {
                        continue;
                    }
                    let exact = alternative.status == ResolutionStatus::Resolved
                        && exact(q, event.context)
                        && catalog.basis == CatalogContractBasis::PublicCandidate
                        && data.facts.event_assessments.iter().any(|a| {
                            a.event == event.id()
                                && a.complete
                                && a.unique
                                && a.exact
                                && !a.unresolved
                                && !a.disagreement
                        });
                    let association = out.associations.insert(ScenarioAssociation {
                        scenario,
                        member: member.id(),
                        alternative: alternative.id(),
                        qualification: raw.qualification,
                        phase: raw.phase,
                        basis: if exact {
                            AssociationBasis::ResolvedTarget
                        } else {
                            AssociationBasis::CandidateTarget
                        },
                        intent,
                    })?;
                    for attempt in data
                        .facts
                        .attempts
                        .iter()
                        .filter(|r| r.alternative == alternative.id())
                    {
                        out.scenario_bindings.insert(ScenarioBinding {association,attempt:attempt.id(),status:match attempt.outcome {BindingOutcome::Bound=>if attempt.authority==normalized::signature_applicability::BindingAuthority::EffectiveInvocation {CheckStatus::Passed}else {CheckStatus::Blocked},BindingOutcome::ProvenIncompatible=>CheckStatus::Failed,BindingOutcome::Undetermined=>CheckStatus::Blocked}})?;
                        for binding in data
                            .facts
                            .bindings
                            .iter()
                            .filter(|r| r.attempt == attempt.id())
                        {
                            for option in data.catalog.options.iter().filter(|o| {
                                o.member == member.id()
                                    && data.catalog.subjects.get(o.subject)
                                        == Some(&CatalogOptionSubject::Parameter {
                                            slot: binding.slot,
                                        })
                            }) {
                                out.options.insert(ScenarioOption {
                                    association,
                                    binding: binding.id(),
                                    option: option.id(),
                                })?;
                            }
                        }
                    }
                }
            }
        }
    }
    super::fields::derive(data, &mut out)?;
    super::runtime::derive(data, &mut out)?;
    deployments(data, &mut out)?;
    checks(data, &mut out)?;
    dependencies(data, &mut out, b)?;
    finish_scenarios(data, &mut out, b)?;
    Ok(out)
}
fn deployments(data: &EvidenceData, out: &mut EvidenceOutput) -> Result<(), ModelError> {
    for observation in data.facts.deployment.iter() {
        let artifact = span_artifact(data, observation.span)?;
        if !data.facts.uses.iter().any(|r| {
            r.artifact == artifact
                && matches!(
                    r.role,
                    SourceRole::Configuration
                        | SourceRole::DistributionMetadata
                        | SourceRole::TaskReceipt
                        | SourceRole::Document
                )
        }) {
            continue;
        }
        let deployment = out.deployments.insert(CatalogDeployment {
            observation: observation.id(),
        })?;
        let input = need(&data.core.artifacts, artifact)?.input;
        let context = qualification(data, observation.qualification)?.context;
        root(out, input, context, RootSubject::Deployment { deployment })?;
        for ownership in data
            .facts
            .artifact_ownership
            .iter()
            .filter(|r| r.artifact == artifact)
        {
            let release = need(&data.facts.verifications, ownership.distribution)?.release;
            out.release_deployments.insert(ReleaseDeployment {
                deployment,
                ownership: ownership.id(),
                release,
            })?;
            root(out, input, context, RootSubject::Release { release })?;
        }
    }
    Ok(())
}
fn checks(data: &EvidenceData, out: &mut EvidenceOutput) -> Result<(), ModelError> {
    for scenario in out.scenarios.iter() {
        let (artifact, context) =
            source_artifact(data, need(&out.scenario_sources, scenario.source)?)?;
        for observation in data
            .facts
            .report_observations
            .iter()
            .filter(|r| r.target == artifact)
        {
            if !exact(qualification(data, observation.qualification)?, context) {
                continue;
            }
            let report = need(&data.facts.reports, observation.report)?;
            let environment = need(&data.facts.reported_environments, report.environment)?;
            let mut fingerprint = None;
            for candidate in data.facts.fingerprints.iter() {
                let acquisition = need(&data.facts.acquisitions, candidate.acquisition)?;
                let source_input = need(&data.core.artifacts, artifact)?.input;
                let connected = acquisition.input == source_input
                    || data
                        .facts
                        .corpus_libraries
                        .iter()
                        .any(|r| r.corpus == source_input && r.library == acquisition.input);
                if connected
                    && candidate.release == environment.release
                    && candidate.lock_digest == environment.lock_digest
                    && candidate.environment_digest == environment.environment_digest
                    && candidate.python_version == environment.python_version
                    && candidate.platform == environment.platform
                {
                    if fingerprint.is_some_and(|id| id != candidate.id()) {
                        fingerprint = None;
                        break;
                    }
                    fingerprint = Some(candidate.id());
                }
            }
            out.checks.insert(ScenarioCheck {
                scenario: scenario.id(),
                observation: observation.id(),
                fingerprint,
                environment: if fingerprint.is_some() {
                    CheckStatus::Passed
                } else {
                    CheckStatus::Blocked
                },
                execution: report.execution,
            })?;
        }
    }
    Ok(())
}
fn dependencies(
    data: &EvidenceData,
    out: &mut EvidenceOutput,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut pending = Rows::new(b);
    let mut subjects = Rows::new(b);
    for scenario in out.scenarios.iter() {
        let source = need(&out.scenario_sources, scenario.source)?;
        let (artifact, context) = source_artifact(data, source)?;
        let declaration = match source {
            ScenarioSource::Python {
                declaration: Some(id),
                ..
            } => Some(need(&data.core.declarations, *id)?.declaration),
            _ => None,
        };
        for assessment in data
            .core
            .reference_assessments
            .iter()
            .filter(|r| r.status != ResolutionStatus::Resolved)
        {
            let reference = need(&data.core.references, assessment.reference)?;
            if need(&data.core.occurrences, reference.read)?.source != artifact
                || qualification(data, reference.qualification)?.context != context
            {
                continue;
            }
            if declaration.is_some_and(|decl| {
                !data
                    .core
                    .ownership
                    .iter()
                    .any(|r| r.occurrence == reference.read && r.owner == decl)
            }) {
                continue;
            }
            let dependency = subjects.insert(SetupDependency::UnresolvedReference {
                assessment: assessment.id(),
            })?;
            pending.insert(ScenarioDependency {
                scenario: scenario.id(),
                dependency,
                status: CheckStatus::Blocked,
            })?;
        }
        for association in out
            .associations
            .iter()
            .filter(|r| r.scenario == scenario.id())
        {
            let event = need(
                &data.facts.events,
                need(&data.facts.alternatives, association.alternative)?.event,
            )?;
            let site = need(&data.core.occurrences, event.site)?;
            let owner = need(&data.core.ownership, event.owner)?;
            // Source order preserves setup evidence; it does not establish a runtime reaching definition.
            for binding in data.core.bindings.iter() {
                if !exact(qualification(data, binding.qualification)?, context) {
                    continue;
                }
                let scope = need(&data.core.lexical_scopes, binding.scope)?;
                let mutation = need(
                    &data.core.occurrences,
                    need(&data.core.binding_events, binding.event)?.site,
                )?;
                if scope.owner == owner.owner
                    && mutation.source == site.source
                    && mutation.end <= site.start
                {
                    let dependency = subjects.insert(SetupDependency::PrecedingMutation {
                        binding: binding.id(),
                    })?;
                    pending.insert(ScenarioDependency {
                        scenario: scenario.id(),
                        dependency,
                        status: CheckStatus::Blocked,
                    })?;
                }
            }
            let mut current = event.site;
            for _ in 0..256 {
                let mut placements = data.core.placements.iter().filter(|r| {
                    r.occurrence == current
                        && data
                            .core
                            .qualifications
                            .get(r.qualification)
                            .is_some_and(|q| exact(q, context))
                });
                let Some(edge) = placements.next() else { break };
                if placements.next().is_some() {
                    break;
                }
                let Some(parent) = edge.parent else { break };
                let kind = need(&data.core.occurrences, parent)?.syntax_kind;
                if kind == SyntaxKind::StmtWith && edge.field == lexical::SyntaxField::Body {
                    let dependency = subjects.insert(SetupDependency::WithContext {
                        placement: edge.id(),
                    })?;
                    pending.insert(ScenarioDependency {
                        scenario: scenario.id(),
                        dependency,
                        status: CheckStatus::Blocked,
                    })?;
                }
                if matches!(kind, SyntaxKind::StmtFunctionDef | SyntaxKind::ExprLambda)
                    && matches!(
                        edge.field,
                        lexical::SyntaxField::Body | lexical::SyntaxField::Value
                    )
                {
                    break;
                }
                current = parent;
            }
        }
    }
    for subject in subjects.iter() {
        out.setup.insert(subject.clone())?;
    }
    for row in pending.iter() {
        out.dependencies.insert(row.clone())?;
    }
    Ok(())
}
fn finish_scenarios(
    _data: &EvidenceData,
    out: &mut EvidenceOutput,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let mut rows = Rows::new(b);
    for scenario in out.scenarios.iter() {
        let mut row = scenario.clone();
        let mut intent = None;
        let mut binding = None;
        let mut environment = None;
        for association in out
            .associations
            .iter()
            .filter(|r| r.scenario == scenario.id())
        {
            intent = Some(match intent {
                Some(old) if old != association.intent => Intent::Mixed,
                _ => association.intent,
            });
            for evidence in out
                .scenario_bindings
                .iter()
                .filter(|r| r.association == association.id())
            {
                binding = Some(match binding {
                    Some(old) if old != evidence.status => CheckStatus::Blocked,
                    _ => evidence.status,
                });
            }
        }
        for check in out.checks.iter().filter(|r| r.scenario == scenario.id()) {
            environment = Some(match environment {
                Some(old) if old != check.environment => CheckStatus::Blocked,
                _ => check.environment,
            });
        }
        row.intent = intent.unwrap_or(row.intent);
        row.binding = binding.unwrap_or(CheckStatus::NotRun);
        row.environment = environment.unwrap_or(CheckStatus::NotRun);
        rows.insert(row)?;
    }
    out.scenarios = rows;
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = EvidenceData::inputs();
    inputs.extend(EvidenceOutput::inputs());
    vec![Invariant {
        name: "catalog_contextual_evidence",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: EvidenceData::new(b),
                out: EvidenceOutput::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: EvidenceData,
    out: EvidenceOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if !self.data.visit(name, batch)? && !self.out.visit(name, batch)? {
            return Err(invalid("undeclared catalog evidence input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.out.matches(&build(&self.data, &self.budget)?)
    }
}
pub fn invocation_links(
    out: &EvidenceOutput,
    invocations: &Rows<analysis::catalog_evidence::Invocation>,
    b: &ResourceBudget,
) -> Result<Rows<EvidenceInvocation>, ModelError> {
    let mut links = Rows::new(b);
    for root in out.roots.iter() {
        let mut found = 0;
        for invocation in invocations
            .iter()
            .filter(|r| r.input == root.input && r.context == root.context)
        {
            links.insert(EvidenceInvocation {
                root: root.id(),
                invocation: invocation.id(),
            })?;
            found += 1;
        }
        if found != 1 {
            return Err(invalid(
                "evidence root requires exact input/context invocation",
            ));
        }
    }
    Ok(links)
}
pub fn invocation_invariants() -> Vec<Invariant> {
    let mut invariants = super::frames::invariants();
    invariants.push(Invariant {
        name: "catalog_evidence_invocation_closure",
        inputs: vec![
            ValidationInput::of::<EvidenceRoot>(&["id"]),
            ValidationInput::of::<analysis::catalog_evidence::Invocation>(&["id"]),
            ValidationInput::of::<EvidenceInvocation>(&["id"]),
        ],
        create: std::sync::Arc::new(|b| {
            Box::new(LinkCheck {
                roots: EvidenceOutput::new(b),
                invocations: Rows::new(b),
                links: Rows::new(b),
                budget: b.clone(),
            })
        }),
    });
    invariants
}
struct LinkCheck {
    roots: EvidenceOutput,
    invocations: Rows<analysis::catalog_evidence::Invocation>,
    links: Rows<EvidenceInvocation>,
    budget: ResourceBudget,
}
impl InvariantCheck for LinkCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == EvidenceRoot::NAME {
            self.roots.roots.decode(batch)?;
        } else if name == analysis::catalog_evidence::Invocation::NAME {
            self.invocations.decode(batch)?;
        } else if name == EvidenceInvocation::NAME {
            self.links.decode(batch)?;
        } else {
            return Err(invalid("undeclared evidence invocation input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if !self.links.same(&invocation_links(
            &self.roots,
            &self.invocations,
            &self.budget,
        )?) {
            return Err(invalid("evidence invocation closure differs"));
        }
        Ok(())
    }
}
pub fn stage(profile: Profile, model: &ValidatedModel) -> Result<Stage, ModelError> {
    use std::collections::BTreeSet;
    let parent = catalog::build::stage(profile);
    let mut inputs = parent.inputs;
    inputs.extend(parent.outputs.into_iter().map(|r| r.completed_store()));
    inputs.extend(EvidenceFacts::stage_inputs());
    inputs.extend(super::runtime::RuntimeData::stage_inputs());
    inputs.extend(analysis::preparation::native_stage(profile).inputs);
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    use analysis::catalog_evidence::*;
    macro_rules! add {($($ty:ty),*)=>{$(outputs.push(RelationUse::of::<$ty>());)*};}
    add!(
        Invocation,
        InvocationSource,
        AnalysisInput,
        ProjectionInput,
        SourceReceipt,
        AnalysisOutcome,
        AnalysisCoverage,
        CoverageSource,
        AnalysisCoveragePremise,
        CoverageRequirement,
        CoverageRequiredSource
    );
    let own = outputs.iter().map(|r| r.name()).collect::<BTreeSet<_>>();
    let facts = facts_relations()
        .iter()
        .map(Relation::name)
        .collect::<BTreeSet<_>>();
    let relation = |name| {
        model
            .relations()
            .iter()
            .find(|r| r.name() == name)
            .ok_or_else(|| invalid(format!("C1 relation absent: {name}")))
    };
    let mut pending = inputs.iter().map(|r| r.name()).collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        let row = relation(name)?;
        for required in row
            .fields()
            .iter()
            .filter_map(|f| f.target().map(|(_, n)| n))
            .chain(
                row.invariants()
                    .iter()
                    .flat_map(|i| i.inputs.iter().map(ValidationInput::name)),
            )
        {
            if own.contains(required) {
                return Err(invalid(format!(
                    "C1 predecessor requires own output: {required}"
                )));
            }
            if (!facts.contains(required) || is_vocabulary(required))
                && !inputs.iter().any(|r| r.name() == required)
            {
                inputs.push(RelationUse::of_relation(relation(required)?).completed_store());
                pending.push(required);
            }
        }
    }
    for input in &mut inputs {
        if is_vocabulary(input.name()) {
            *input = input.clone().at_epoch(PublicationBoundary::Local);
        }
    }
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    Ok(Stage {
        name: "catalog_evidence",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("build.rs")),
        configuration: definition().1.semantic_version,
    })
}

pub fn definition() -> (analysis::MethodParameters, analysis::AnalysisDefinition) {
    let (parameters, _) = catalog::build::definition();
    let mut version = KeySink::new("catalog-context-evidence/v2");
    parameters.id().encode(&mut version);
    for bytes in [
        include_bytes!("build.rs").as_slice(),
        include_bytes!("fields.rs").as_slice(),
        include_bytes!("runtime.rs").as_slice(),
        include_bytes!("frames.rs").as_slice(),
        include_bytes!("intent.rs").as_slice(),
    ] {
        ContentHash::of(bytes).encode(&mut version);
    }
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::CatalogEvidence,
        interpretation: analysis::Interpretation::Structural,
        parameters: parameters.id(),
        semantic_version: version.finish(),
    };
    (parameters, definition)
}
