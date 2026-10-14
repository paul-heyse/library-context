//! Deterministic contextual evidence closure, shared by publication and independent controls.
// Increment for a meaning/rule change; implementation source bytes live in producer provenance.
const SEMANTIC_RULE_REVISION: i64 = 2;
use super::*;
use crate::domain::{
    assertion::Evidence,
    attribution::*,
    charged::{ChargedMap, ChargedSet, StateCharge},
    input::*,
    normalized::{Rows, copy_selected_rows},
    resources::ResourceBudget,
    source::*,
    stages::*,
};
pub struct EvidenceData {
    pub core: catalog::build::CatalogData,
    pub catalog: catalog::build::CatalogOutput,
    pub facts: EvidenceFacts,
    pub runtime: super::runtime::RuntimeData,
    /// Cumulative Local qualifications belong to derived location provenance, not native C0.
    pub local_qualifications: Rows<assertion::AssertionQualification>,
}
impl EvidenceData {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            core: catalog::build::CatalogData::new(b),
            catalog: catalog::build::CatalogOutput::new(b),
            facts: EvidenceFacts::new(b),
            runtime: super::runtime::RuntimeData::new(b),
            local_qualifications: Rows::new(b),
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
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        let name = input.name();
        if !is_vocabulary(name) {
            return self.visit(name, batch);
        }
        match input.prefix() {
            Some(PublicationBoundary::Facts) => {
                let core = self.core.visit(name, batch)?;
                let facts = self.facts.visit(name, batch)?;
                Ok(core || facts)
            }
            Some(PublicationBoundary::Local) if name == assertion::AssertionQualification::NAME => {
                self.local_qualifications.decode(batch)?;
                Ok(true)
            }
            _ => Err(invalid(format!(
                "catalog evidence input {name} changes its completed vocabulary view"
            ))),
        }
    }
    /// Copy one exact selected partition into charged builder-owned premises. The union
    /// was decoded once; facts and later qualification epochs retain separate membership.
    pub fn selected_copy<I: Iterator<Item = [u8; 16]>>(
        &self,
        inputs: &[ValidationInput],
        keys: &mut dyn FnMut(usize) -> Result<I, ModelError>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut selected = Self::new(budget);
        macro_rules! copy_core{($($field:ident:$ty:ty,)*)=>{$(copy_selected_rows(&self.core.$field,&mut selected.core.$field,inputs,None,keys)?;)*};}
        crate::catalog_inputs!(copy_core);
        macro_rules! copy_catalog{($($field:ident:$ty:ty,)*)=>{$(copy_selected_rows(&self.catalog.$field,&mut selected.catalog.$field,inputs,None,keys)?;)*};}
        crate::catalog_outputs!(copy_catalog);
        macro_rules! copy_facts{($($field:ident:$ty:ty,)*)=>{$(copy_selected_rows(&self.facts.$field,&mut selected.facts.$field,inputs,None,keys)?;)*};}
        crate::catalog_evidence_inputs!(copy_facts);
        macro_rules! copy_runtime{($($field:ident:$ty:ty,)*)=>{$(copy_selected_rows(&self.runtime.$field,&mut selected.runtime.$field,inputs,None,keys)?;)*};}
        crate::catalog_runtime_inputs!(copy_runtime);
        selected.runtime.compact = self.runtime.compact.retain(budget)?;
        copy_selected_rows(
            &self.local_qualifications,
            &mut selected.local_qualifications,
            inputs,
            Some(PublicationBoundary::Local),
            keys,
        )?;
        Ok(selected)
    }
    pub fn consumed_inputs(_profile: Profile) -> Vec<ValidationInput> {
        Self::inputs()
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut rows = crate::domain::normalized::facts_inputs(
            catalog::build::CatalogData::validation_inputs(),
        );
        rows.extend(catalog::build::CatalogOutput::validation_inputs());
        rows.extend(crate::domain::normalized::facts_inputs(
            EvidenceFacts::inputs(),
        ));
        rows.extend(super::runtime::RuntimeData::inputs());
        rows.push(
            ValidationInput::of::<assertion::AssertionQualification>(&["id"])
                .at_epoch(PublicationBoundary::Local),
        );
        rows.sort_by_key(|r| (r.name(), r.prefix()));
        rows.dedup_by_key(|r| (r.name(), r.prefix()));
        rows
    }
}

macro_rules! data {($($f:ident:$ty:ty,)*)=>{
 pub struct EvidenceFacts {$(pub $f:Rows<$ty>,)*}
 impl EvidenceFacts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$f.decode(batch)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}fn stage_inputs()->Vec<RelationUse> {vec![$(RelationUse::completed::<$ty>()),*]}}
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
/// Invocation-local ordered correspondence. It borrows the already charged premises and
/// reserves its fixed backing before allocation. The identity tie-break retains canonical
/// row order inside every plural bucket; no candidate is collapsed into a singleton.
struct Correspondence<'a, K, R> {
    entries: Vec<(K, &'a R)>,
    _charge: StateCharge,
}
impl<'a, K: Ord + Copy, R: Record> Correspondence<'a, K, R> {
    fn new(
        rows: &'a Rows<R>,
        key: impl Fn(&R) -> K,
        charge: &mut StateCharge,
    ) -> Result<Self, ModelError> {
        Self::try_new(rows, |row| Ok(key(row)), charge)
    }
    fn try_new(
        rows: &'a Rows<R>,
        key: impl Fn(&R) -> Result<K, ModelError>,
        charge: &mut StateCharge,
    ) -> Result<Self, ModelError> {
        let mut retained = StateCharge::new(
            charge
                .budget()
                .ok_or_else(|| invalid("catalog evidence correspondence has no attempt budget"))?,
            "catalog-evidence-correspondence",
        );
        let bytes = rows
            .len()
            .checked_mul(size_of::<(K, &R)>())
            .ok_or_else(|| invalid("catalog evidence correspondence size overflow"))?;
        retained.grow(bytes)?;
        let mut entries = Vec::with_capacity(rows.len());
        for row in rows.iter() {
            entries.push((key(row)?, row));
        }
        entries.sort_unstable_by(|(a, ar), (b, br)| a.cmp(b).then_with(|| ar.id().cmp(&br.id())));
        Ok(Self {
            entries,
            _charge: retained,
        })
    }
    fn get(&self, key: K) -> impl Iterator<Item = &'a R> + '_ {
        let start = self
            .entries
            .partition_point(|(candidate, _)| *candidate < key);
        self.entries[start..]
            .iter()
            .take_while(move |(candidate, _)| *candidate == key)
            .map(|(_, row)| *row)
    }
}
struct SourceCorrespondence<'a> {
    coverage: Correspondence<'a, (Id<CoverageScope>, Id<AnalysisContext>), ProviderCoverage>,
    uses: Correspondence<'a, Id<SourceArtifact>, ArtifactUse>,
    reports: Correspondence<'a, Id<SourceArtifact>, deployment::TaskReportObservation>,
    parameters: Correspondence<'a, Id<Occurrence>, syntax::ParameterSyntaxObservation>,
}
impl<'a> SourceCorrespondence<'a> {
    fn new(data: &'a EvidenceData, charge: &mut StateCharge) -> Result<Self, ModelError> {
        Ok(Self {
            coverage: Correspondence::new(
                &data.core.native_coverage,
                |r| (r.scope, r.context),
                charge,
            )?,
            uses: Correspondence::new(&data.facts.uses, |r| r.artifact, charge)?,
            reports: Correspondence::new(&data.facts.report_observations, |r| r.target, charge)?,
            parameters: Correspondence::new(&data.core.parameter_syntax, |r| r.function, charge)?,
        })
    }
}
fn parse(
    prepared: &SourceCorrespondence<'_>,
    artifact: Id<SourceArtifact>,
    context: Id<AnalysisContext>,
) -> CheckStatus {
    let scope = CoverageScope::Artifact { artifact }.id();
    let mut status = None;
    for row in prepared
        .coverage
        .get((scope, context))
        .filter(|r| r.family == FactFamily::Syntax)
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
fn selected(
    data: &EvidenceData,
    prepared: &SourceCorrespondence<'_>,
    artifact: Id<SourceArtifact>,
    role: SourceRole,
) -> bool {
    prepared.uses.get(artifact).any(|r| {
        r.role == role
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
    prepared: &SourceCorrespondence<'_>,
    artifact: Id<SourceArtifact>,
    context: Id<AnalysisContext>,
) -> Result<CheckStatus, ModelError> {
    let mut found = None;
    for row in prepared.reports.get(artifact) {
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
    prepared: &SourceCorrespondence<'_>,
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
        ScenarioSource::Python { .. } => parse(prepared, artifact, context),
        ScenarioSource::Fence { observation } => {
            let block = need(&data.facts.blocks, *observation)?;
            if block.materialized.is_some() {
                parse(prepared, artifact, context)
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
        execution: execution(data, prepared, artifact, context)?,
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
                for syntax in prepared.parameters.get(row.declaration).filter(|r| {
                    data.core
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
type ScenarioKey = (
    Option<Id<SourceArtifact>>,
    Id<AnalysisContext>,
    Option<Id<Occurrence>>,
    bool,
);
fn scenarios_for(
    data: &EvidenceData,
    scenarios: &Correspondence<'_, ScenarioKey, CatalogScenario>,
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
    for row in scenarios
        .get((Some(source), event.context, declaration, false))
        .chain(scenarios.get((Some(source), event.context, None, true)))
    {
        rows.insert(row.clone())?;
    }
    Ok(rows)
}
pub fn build(data: &EvidenceData, b: &ResourceBudget) -> Result<EvidenceOutput, ModelError> {
    let mut out = EvidenceOutput::new(b);
    let mut charge = StateCharge::new(b, "catalog-evidence-frames");
    let prepared = SourceCorrespondence::new(data, &mut charge)?;
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
    let frame_contexts =
        Correspondence::try_new(&data.facts.core_links, |link| Ok(link.member), &mut charge)?;
    for option in data.catalog.options.iter() {
        let member = need(&data.catalog.members, option.member)?;
        for link in frame_contexts.get(member.id()) {
            let context = need(&data.facts.core_invocations, link.invocation)?.context;
            root(
                &mut out,
                member.input,
                context,
                RootSubject::Option {
                    option: option.id(),
                },
            )?;
        }
    }
    let coverage_artifacts = Correspondence::new(
        &data.core.artifacts,
        |a| CoverageScope::Artifact { artifact: a.id() }.id(),
        &mut charge,
    )?;
    let mut scopes = ChargedSet::default();
    for coverage in data
        .core
        .native_coverage
        .iter()
        .filter(|r| r.family == FactFamily::Syntax)
    {
        for artifact in coverage_artifacts.get(coverage.scope) {
            scopes.insert(&mut charge, (artifact.id(), coverage.context))?;
        }
    }
    let mut declarations = None;
    for (artifact, context) in scopes.iter() {
        let source = need(&data.core.artifacts, *artifact)?;
        root(
            &mut out,
            source.input,
            *context,
            RootSubject::Source {
                artifact: *artifact,
            },
        )?;
        for usage in prepared.uses.get(*artifact).filter(|r| {
            data.core
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
                &prepared,
                &mut out,
                ScenarioSource::Python {
                    artifact: *artifact,
                    role: usage.role,
                    context: *context,
                    declaration: None,
                },
            )?;
            if declarations.is_none() {
                declarations = Some(Correspondence::try_new(
                    &data.core.declarations,
                    |r| {
                        if matches!(
                            r.kind,
                            DeclarationKind::Function | DeclarationKind::AsyncFunction
                        ) {
                            Ok(Some(need(&data.core.occurrences, r.declaration)?.source))
                        } else {
                            Ok(None)
                        }
                    },
                    &mut charge,
                )?);
            }
            for declaration in declarations
                .as_ref()
                .expect("prepared above")
                .get(Some(*artifact))
            {
                if qualification(data, declaration.qualification)?.context != *context {
                    continue;
                }
                scenario(
                    data,
                    &prepared,
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
    for block in data.facts.blocks.iter() {
        let original = span_artifact(data, need(&data.facts.nodes, block.block.id())?.span())?;
        if selected(data, &prepared, original, SourceRole::Document) {
            scenario(
                data,
                &prepared,
                &mut out,
                ScenarioSource::Fence {
                    observation: block.id(),
                },
            )?;
        }
    }
    for document in data.facts.documents.iter() {
        if selected(data, &prepared, document.source, SourceRole::Document) {
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
    let exposures = Correspondence::new(&data.catalog.exposures, |r| r.exposure, &mut charge)?;
    for candidate in data.facts.mention_candidates.iter() {
        let exposure = need(&data.core.exposures, candidate.exposure)?;
        let assessment = need(&data.facts.mention_assessments, candidate.assessment)?;
        let mention = need(&data.facts.mentions, assessment.observation)?;
        let q = qualification(data, mention.qualification)?;
        let artifact = span_artifact(data, need(&data.facts.nodes, mention.mention.id())?.span())?;
        if q.context != exposure.context
            || !selected(data, &prepared, artifact, SourceRole::Document)
        {
            continue;
        }
        for link in exposures.get(exposure.id()).filter(|r| {
            data.catalog
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
    let scenario_index = Correspondence::try_new(
        &out.scenarios,
        |row| match need(&out.scenario_sources, row.source)? {
            ScenarioSource::Python {
                artifact,
                context,
                declaration,
                ..
            } => Ok((
                Some(*artifact),
                *context,
                declaration.and_then(|id| data.core.declarations.get(id).map(|r| r.declaration)),
                false,
            )),
            ScenarioSource::Fence { observation } => {
                let block = need(&data.facts.blocks, *observation)?;
                Ok((
                    block.materialized,
                    qualification(data, block.qualification)?.context,
                    None,
                    true,
                ))
            }
        },
        &mut charge,
    )?;
    let alternatives = Correspondence::new(&data.facts.alternatives, |r| r.event, &mut charge)?;
    let assessments = Correspondence::new(&data.facts.event_assessments, |r| r.event, &mut charge)?;
    let callables = Correspondence::new(&data.catalog.callables, |r| r.assessment, &mut charge)?;
    let callable_assessments = Correspondence::new(
        &data.core.assessments,
        |r| (r.callable, r.context),
        &mut charge,
    )?;
    let attempts = Correspondence::new(&data.facts.attempts, |r| r.alternative, &mut charge)?;
    let bindings = Correspondence::new(&data.facts.bindings, |r| r.attempt, &mut charge)?;
    let options = Correspondence::new(
        &data.catalog.options,
        |r| (r.member, r.subject),
        &mut charge,
    )?;
    for event in data.facts.events.iter() {
        let scenarios = scenarios_for(data, &scenario_index, event, b)?;
        for scenario_row in scenarios.iter() {
            let scenario = scenario_row.id();
            let intent = super::intent::classify(data, event, scenario_row.intent)?;
            for alternative in alternatives.get(event.id()) {
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
                for catalog in callable_assessments
                    .get((*callable, event.context))
                    .flat_map(|a| callables.get(a.id()))
                {
                    let member = need(&data.catalog.members, catalog.member)?;
                    if !frames.contains(&(member.id(), event.context)) {
                        continue;
                    }
                    let exact = alternative.status == ResolutionStatus::Resolved
                        && exact(q, event.context)
                        && catalog.basis == CatalogContractBasis::PublicCandidate
                        && assessments.get(event.id()).any(|a| {
                            a.complete && a.unique && a.exact && !a.unresolved && !a.disagreement
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
                    for attempt in attempts.get(alternative.id()) {
                        out.scenario_bindings.insert(ScenarioBinding {association,attempt:attempt.id(),status:match attempt.outcome {BindingOutcome::Bound=>if attempt.authority==normalized::signature_applicability::BindingAuthority::EffectiveInvocation {CheckStatus::Passed}else {CheckStatus::Blocked},BindingOutcome::ProvenIncompatible=>CheckStatus::Failed,BindingOutcome::Undetermined=>CheckStatus::Blocked}})?;
                        for binding in bindings.get(attempt.id()) {
                            for option in options
                                .get((
                                    member.id(),
                                    CatalogOptionSubject::Parameter { slot: binding.slot }.id(),
                                ))
                                .filter(|o| {
                                    data.catalog.subjects.get(o.subject)
                                        == Some(&CatalogOptionSubject::Parameter {
                                            slot: binding.slot,
                                        })
                                })
                            {
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
    drop(scenario_index);
    super::fields::derive(data, &mut out)?;
    super::symbolic::derive(data, &mut out)?;
    super::runtime::derive(data, &mut out)?;
    deployments(data, &prepared, &mut out, &mut charge)?;
    checks(data, &prepared, &mut out, &mut charge)?;
    dependencies(data, &mut out, b)?;
    finish_scenarios(data, &mut out, b)?;
    super::characterization::derive(data, &mut out)?;
    super::diagnostic_uses::derive(data, &mut out, b)?;
    Ok(out)
}
fn deployments(
    data: &EvidenceData,
    prepared: &SourceCorrespondence<'_>,
    out: &mut EvidenceOutput,
    charge: &mut StateCharge,
) -> Result<(), ModelError> {
    let ownerships = Correspondence::new(&data.facts.artifact_ownership, |r| r.artifact, charge)?;
    for observation in data.facts.deployment.iter() {
        let artifact = span_artifact(data, observation.span)?;
        if !prepared.uses.get(artifact).any(|r| {
            matches!(
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
        for ownership in ownerships.get(artifact) {
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
fn checks(
    data: &EvidenceData,
    prepared: &SourceCorrespondence<'_>,
    out: &mut EvidenceOutput,
    charge: &mut StateCharge,
) -> Result<(), ModelError> {
    let corpus = Correspondence::new(&data.facts.corpus_libraries, |r| r.corpus, charge)?;
    let mut environments = ChargedMap::default();
    let mut environment_charge = StateCharge::new(
        charge.budget().ok_or_else(|| {
            invalid("catalog evidence environment correspondence has no attempt budget")
        })?,
        "catalog-evidence-environments",
    );
    for scenario in out.scenarios.iter() {
        let (artifact, context) =
            source_artifact(data, need(&out.scenario_sources, scenario.source)?)?;
        for observation in prepared.reports.get(artifact) {
            if !exact(qualification(data, observation.qualification)?, context) {
                continue;
            }
            let report = need(&data.facts.reports, observation.report)?;
            let environment = need(&data.facts.reported_environments, report.environment)?;
            let source_input = need(&data.core.artifacts, artifact)?.input;
            let environment_key = (source_input, report.environment);
            let fingerprint = if let Some(found) = environments.get(&environment_key) {
                *found
            } else {
                let mut found = None;
                // Prepare only consumed environment questions. Retain the original whole
                // candidate order and ambiguity stop, including validation of acquisitions.
                for candidate in data.facts.fingerprints.iter() {
                    let acquisition = need(&data.facts.acquisitions, candidate.acquisition)?;
                    let connected = acquisition.input == source_input
                        || corpus
                            .get(source_input)
                            .any(|r| r.library == acquisition.input);
                    if connected
                        && candidate.release == environment.release
                        && candidate.lock_digest == environment.lock_digest
                        && candidate.environment_digest == environment.environment_digest
                        && candidate.python_version == environment.python_version
                        && candidate.platform == environment.platform
                    {
                        if found.is_some_and(|id| id != candidate.id()) {
                            found = None;
                            break;
                        }
                        found = Some(candidate.id());
                    }
                }
                environments.insert(&mut environment_charge, environment_key, found)?;
                found
            };
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
    let mut charge = StateCharge::new(b, "catalog-evidence-dependency-correspondence");
    let associations = Correspondence::new(&out.associations, |r| r.scenario, &mut charge)?;
    let placements = Correspondence::new(&data.core.placements, |r| r.occurrence, &mut charge)?;
    let ownerships = Correspondence::new(&data.core.ownership, |r| r.occurrence, &mut charge)?;
    // Bindings remain qualified at consumption; grouping by lexical scope preserves the
    // complete alternatives of each owner without manufacturing reaching definitions.
    let bindings = Correspondence::new(&data.core.bindings, |r| r.scope, &mut charge)?;
    let scopes = Correspondence::new(&data.core.lexical_scopes, |r| r.owner, &mut charge)?;
    let mut checked_contexts = ChargedSet::default();
    let unresolved = if out.scenarios.is_empty() {
        None
    } else {
        Some(Correspondence::try_new(
            &data.core.reference_assessments,
            |r| {
                if r.status == ResolutionStatus::Resolved {
                    return Ok(None);
                }
                let reference = need(&data.core.references, r.reference)?;
                Ok(Some(need(&data.core.occurrences, reference.read)?.source))
            },
            &mut charge,
        )?)
    };
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
        for assessment in unresolved
            .as_ref()
            .expect("a scenario prepared correspondence")
            .get(Some(artifact))
        {
            let reference = need(&data.core.references, assessment.reference)?;
            if need(&data.core.occurrences, reference.read)?.source != artifact
                || qualification(data, reference.qualification)?.context != context
            {
                continue;
            }
            if declaration
                .is_some_and(|decl| !ownerships.get(reference.read).any(|r| r.owner == decl))
            {
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
        for association in associations.get(scenario.id()) {
            let event = need(
                &data.facts.events,
                need(&data.facts.alternatives, association.alternative)?.event,
            )?;
            let site = need(&data.core.occurrences, event.site)?;
            let owner = need(&data.core.ownership, event.owner)?;
            // Preserve premise validation even for bindings outside this owner's scope.
            // The old all-binding join performed it for every association; one pass per
            // consumed context suffices before the indexed owner correspondence is used.
            if checked_contexts.insert(&mut charge, context)? {
                for binding in data.core.bindings.iter() {
                    if exact(qualification(data, binding.qualification)?, context) {
                        need(&data.core.lexical_scopes, binding.scope)?;
                        let mutation = need(&data.core.binding_events, binding.event)?;
                        need(&data.core.occurrences, mutation.site)?;
                    }
                }
            }
            // Source order preserves setup evidence; it does not establish a runtime reaching definition.
            for binding in scopes
                .get(owner.owner)
                .flat_map(|scope| bindings.get(scope.id()))
            {
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
                let mut placements = placements.get(current).filter(|r| {
                    data.core
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
    let mut charge = StateCharge::new(b, "catalog-evidence-scenario-correspondence");
    let associations = Correspondence::new(&out.associations, |r| r.scenario, &mut charge)?;
    let bindings = Correspondence::new(&out.scenario_bindings, |r| r.association, &mut charge)?;
    let checks = Correspondence::new(&out.checks, |r| r.scenario, &mut charge)?;
    for scenario in out.scenarios.iter() {
        let mut row = scenario.clone();
        let mut intent = None;
        let mut binding = None;
        let mut environment = None;
        for association in associations.get(scenario.id()) {
            intent = Some(match intent {
                Some(old) if old != association.intent => Intent::Mixed,
                _ => association.intent,
            });
            for evidence in bindings.get(association.id()) {
                binding = Some(match binding {
                    Some(old) if old != evidence.status => CheckStatus::Blocked,
                    _ => evidence.status,
                });
            }
        }
        for check in checks.get(scenario.id()) {
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
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
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
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if is_vocabulary(input.name()) {
            if !self.data.visit_input(input, batch)? {
                return Err(invalid("undeclared catalog evidence view"));
            }
            Ok(())
        } else {
            self.visit(input.name(), batch)
        }
    }
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
    let mut charge = StateCharge::new(b, "catalog-evidence-invocation-correspondence");
    let selected = Correspondence::new(invocations, |r| (r.input, r.context), &mut charge)?;
    for root in out.roots.iter() {
        let mut found = 0;
        for invocation in selected.get((root.input, root.context)) {
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
    let mut invariants = Vec::new();
    invariants.push(Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 1,
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
pub fn stage(
    profile: Profile,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Stage, ModelError> {
    let parent = catalog::build::stage(profile, model, order)?;
    let mut inputs = parent.inputs;
    inputs.extend(parent.outputs.into_iter().map(|r| r.completed_input()));
    inputs.extend(EvidenceFacts::stage_inputs());
    inputs.extend(super::runtime::RuntimeData::stage_inputs());
    inputs.extend(analysis::preparation::native_stage(profile, model, order)?.inputs);
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    outputs.extend(
        analysis::catalog_evidence::publication_relations()
            .iter()
            .map(stages::RelationUse::of_relation),
    );
    let mut roots = dependency_closure::DependencyClosure::roots_from_uses(model, &inputs)?;
    roots.extend(EvidenceData::inputs());
    let inputs = dependency_closure::DependencyClosure::grants(
        model,
        roots,
        inputs,
        &outputs,
        PublicationBoundary::Local,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    Ok(Stage {
        captured_binding: None,
        name: "catalog_evidence",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
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
    SEMANTIC_RULE_REVISION.encode(&mut version);
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::CatalogEvidence,
        interpretation: analysis::Interpretation::Structural,
        parameters: parameters.id(),
        semantic_version: version.finish(),
    };
    (parameters, definition)
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["catalog_contextual_evidence"]
}
pub(crate) fn invocation_invariants_refs() -> Vec<&'static str> {
    let mut refs = vec!["catalog_evidence_invocation_closure"];
    refs.extend(super::frames::invariants_refs());
    refs
}
#[cfg(test)]
mod selected_copy_controls {
    use super::*;
    fn nominal<T>(value: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([value; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn selected_copy_keeps_facts_and_local_qualification_membership_separate() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut union = EvidenceData::new(&budget);
        let qualification = |context| assertion::AssertionQualification {
            context,
            scope: nominal(2),
            condition: conditions::Diagram::always().id(),
            assumptions: assumptions::AssumptionSet::empty_id(),
            modality: Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        let facts = qualification(nominal(3));
        let local = qualification(nominal(4));
        union.core.qualifications.insert_borrowed(&facts).unwrap();
        union.local_qualifications.insert_borrowed(&local).unwrap();
        let inputs = vec![
            ValidationInput::of::<assertion::AssertionQualification>(&["id"])
                .at_epoch(PublicationBoundary::Facts),
            ValidationInput::of::<assertion::AssertionQualification>(&["id"])
                .at_epoch(PublicationBoundary::Local),
        ];
        let keys = [vec![], vec![*local.id().bytes(), [255; 16]]];
        let selected = union
            .selected_copy(
                &inputs,
                &mut |table| Ok(keys[table].iter().copied()),
                &budget,
            )
            .unwrap();
        assert!(selected.core.qualifications.is_empty());
        assert_eq!(selected.local_qualifications.get(local.id()), Some(&local));
        assert!(selected.local_qualifications.get(facts.id()).is_none());
        drop(selected);
        drop(union);
        assert_eq!(budget.reserved(), 0);
    }
}
