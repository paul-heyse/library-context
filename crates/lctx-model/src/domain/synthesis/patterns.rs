//! Authored official code documents exact source statements, never execution or normal completion.
use super::{documentary, frames, source_code, source_setup};
use crate::domain::{
    analysis::{
        self,
        support::{DerivedEvidence, SourceFacts},
        synthesis as owner,
    },
    catalog::{CatalogMemberInvocation, evidence as c1},
    normalized::Rows,
    resources::ResourceBudget,
    structural::handoffs,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum BoundaryReason {
    MissingAssociation = 0,
    ForeignSource = 1,
    OriginalWrapperUnavailable = 2,
    MappingUnavailable = 3,
    SetupUnavailable = 4,
    FlowUnavailable = 5,
    StatementLimit = 6,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(
    name = "synthesis_authored_code_sources",
    rule = "authored_code_source"
)]
pub struct AuthoredCodeSource {
    #[model(key)]
    pub frame: Id<frames::Frame>,
    #[model(key)]
    pub member: Id<CatalogMemberInvocation>,
    #[model(key)]
    pub group: Id<handoffs::Group>,
    #[model(premise)]
    pub handoff: Id<handoffs::Handoff>,
    #[model(premise)]
    pub association: Id<c1::ScenarioAssociation>,
    pub source_input: Id<input::InputRevision>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="synthesis_authored_code_conclusions",rule="original_official_code",invariant_refs=invariants_refs,semantic_source=include_bytes!("patterns.rs"))]
pub struct AuthoredCodeConclusion {
    #[model(key, premise)]
    pub source: Id<AuthoredCodeSource>,
    pub code: ContentHash,
    qualification: Id<assertion::AssertionQualification>,
    status: analysis::policy::EvidenceStatus,
}
impl AuthoredCodeConclusion {
    pub fn qualification(&self) -> Id<assertion::AssertionQualification> {
        self.qualification
    }
}
impl analysis::support::sealed::DerivedEvidence for AuthoredCodeConclusion {}
impl DerivedEvidence for AuthoredCodeConclusion {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(
    name = "synthesis_authored_code_anchors",
    rule = "authored_code_anchor"
)]
pub enum CodeAnchor {
    #[model(code = 0)]
    Occurrence {
        occurrence: Id<source::Occurrence>,
        #[model(premise)]
        wrapper: Id<c1::OriginalSource>,
    },
    #[model(code = 1)]
    Fence {
        #[model(premise)]
        block: Id<documents::CodeBlockObservation>,
        occurrence: Id<source::Occurrence>,
        #[model(premise)]
        wrapper: Id<c1::OriginalSource>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="synthesis_authored_code_statements",rule="authored_statement",conclusion=conclusion)]
pub struct AuthoredStatement {
    #[model(key)]
    pub conclusion: Id<AuthoredCodeConclusion>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub placement: Id<analysis::native::NativeAssertionPremise>,
    pub anchor: Id<CodeAnchor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "synthesis_authored_setup_sources")]
pub enum SetupSource {
    #[model(code = 0)]
    Builtin {
        reference: Id<analysis::native::NativeAssertionPremise>,
        resolution: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 1)]
    Import {
        reference: Id<analysis::native::NativeAssertionPremise>,
        resolution: Id<analysis::native::NativeAssertionPremise>,
        binding: Id<analysis::native::NativeAssertionPremise>,
        alias: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 2)]
    Internal {
        reference: Id<analysis::native::NativeAssertionPremise>,
        resolution: Id<analysis::native::NativeAssertionPremise>,
        binding: Id<analysis::native::NativeAssertionPremise>,
    },
    #[model(code = 3)]
    Named {
        reference: Id<analysis::native::NativeAssertionPremise>,
        observation: Id<flow::FlowUseObservation>,
        support: Id<flow::FlowUseSupport>,
        inventory: Id<flow_inventory::FlowUseInventoryObservation>,
        inventory_support: Id<flow_inventory::FlowUseInventorySupport>,
        reaching: Id<flow::FlowReachingObservation>,
        reaching_support: Id<flow::FlowReachingSupport>,
        definition: Id<flow::FlowDefinitionObservation>,
        definition_support: Id<flow::FlowDefinitionSupport>,
        region: Option<Id<flow::FlowRegionObservation>>,
        region_support: Option<Id<flow::FlowRegionSupport>>,
        coverage: Id<attribution::ProviderCoverage>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="synthesis_authored_setup_dependencies",rule="authored_setup_dependency",conclusion=conclusion)]
pub struct SetupDependency {
    #[model(key)]
    pub conclusion: Id<AuthoredCodeConclusion>,
    #[model(key)]
    pub ordinal: i64,
    #[model(premise)]
    pub source: Id<SetupSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "synthesis_authored_code_boundaries")]
pub struct CodeBoundary {
    #[model(key)]
    pub frame: Id<frames::Frame>,
    #[model(key)]
    pub member: Id<CatalogMemberInvocation>,
    #[model(key)]
    pub group: Id<handoffs::Group>,
    #[model(key)]
    pub handoff: Id<handoffs::Handoff>,
    pub association: Option<Id<c1::ScenarioAssociation>>,
    pub reason: BoundaryReason,
}
#[macro_export]
macro_rules! synthesis_pattern_inputs{($m:ident)=>{$m!{
 associations:$crate::domain::catalog::evidence::ScenarioAssociation,scenarios:$crate::domain::catalog::evidence::CatalogScenario,scenario_sources:$crate::domain::catalog::evidence::ScenarioSource,spans:$crate::domain::catalog::evidence::ScenarioSpan,originals:$crate::domain::catalog::evidence::OriginalSource,blocks:$crate::domain::documents::CodeBlockObservation,
 handoffs:$crate::domain::structural::handoffs::Handoff,groups:$crate::domain::structural::handoffs::Group,members:$crate::domain::structural::handoffs::Member,alternatives:$crate::domain::normalized::events::NormalizedCallAlternative,events:$crate::domain::normalized::events::NormalizedCallEvent,roles:$crate::domain::input::ArtifactUse,
}};}
#[macro_export]
macro_rules! synthesis_pattern_named_inputs{($m:ident)=>{$m!{
 artifacts:$crate::domain::source::SourceArtifact,modules:$crate::domain::source::Module,scopes:$crate::domain::source::CoverageScope,occurrences:$crate::domain::source::Occurrence,qualifications:$crate::domain::assertion::AssertionQualification,conditions:$crate::domain::conditions::Condition,condition_nodes:$crate::domain::conditions::ConditionNode,
 inventories:$crate::domain::flow_inventory::FlowUseInventoryObservation,inventory_supports:$crate::domain::flow_inventory::FlowUseInventorySupport,inventory_candidates:$crate::domain::flow_inventory::FlowUseCandidate,inventory_members:$crate::domain::flow_inventory::FlowUseInventoryMember,source_views:$crate::domain::flow::FlowSourceViewObservation,source_view_supports:$crate::domain::flow::FlowSourceViewSupport,providers:$crate::domain::attribution::Provider,surfaces:$crate::domain::assertion::ProviderSurface,evidence:$crate::domain::assertion::Evidence,
 owners:$crate::domain::normalized::entities::OccurrenceOwnership,refs:$crate::domain::normalized::entities::EntityRef,lexical_scopes:$crate::domain::lexical::LexicalScope,regions:$crate::domain::flow::FlowRegionObservation,region_supports:$crate::domain::flow::FlowRegionSupport,
 uses:$crate::domain::flow::FlowUse,use_observations:$crate::domain::flow::FlowUseObservation,use_supports:$crate::domain::flow::FlowUseSupport,definitions:$crate::domain::flow::FlowDefinition,definition_observations:$crate::domain::flow::FlowDefinitionObservation,definition_supports:$crate::domain::flow::FlowDefinitionSupport,targets:$crate::domain::flow::ReachingDefinition,reaching:$crate::domain::flow::FlowReachingObservation,reaching_supports:$crate::domain::flow::FlowReachingSupport,runs:$crate::domain::attribution::ProviderRun,coverage:$crate::domain::attribution::ProviderCoverage,native:$crate::domain::analysis::native::NativeQualification,
}};}
fn named_inputs(profile: stages::Profile) -> Vec<ValidationInput> {
    let mut inputs = vec![];
    macro_rules! named{($($field:ident:$ty:ty,)*)=>{$(if profile==stages::Profile::Behavioral||![flow::FlowUse::NAME,flow::FlowUseObservation::NAME,flow::FlowUseSupport::NAME,flow::FlowDefinition::NAME,flow::FlowDefinitionObservation::NAME,flow::FlowDefinitionSupport::NAME,flow::ReachingDefinition::NAME,flow::FlowReachingObservation::NAME,flow::FlowReachingSupport::NAME,flow::FlowSourceViewObservation::NAME,flow::FlowSourceViewSupport::NAME,flow::FlowRegionObservation::NAME,flow::FlowRegionSupport::NAME,flow_inventory::FlowUseInventoryObservation::NAME,flow_inventory::FlowUseInventorySupport::NAME,flow_inventory::FlowUseCandidate::NAME,flow_inventory::FlowUseInventoryMember::NAME].contains(&<$ty>::NAME){inputs.push(ValidationInput::of::<$ty>(&["id"]));})*};}
    crate::synthesis_pattern_named_inputs!(named);
    inputs
}
macro_rules! data{($($f:ident:$t:ty,)*)=>{pub struct Data{$(pub $f:Rows<$t>,)*pub setup:source_setup::Data,pub flow:handoffs::Data}impl Data{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*setup:source_setup::Data::new(b),flow:handoffs::Data::new(b)}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{let s=self.setup.visit(n,b)?;let flow=self.flow.visit(n,b)?;$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(s||flow)}pub fn inputs(profile:stages::Profile)->Vec<ValidationInput>{let mut inputs=vec![$(ValidationInput::of::<$t>(&["id"]),)*];inputs.extend(source_setup::Data::inputs());inputs.extend(named_inputs(profile));inputs}}};}
crate::synthesis_pattern_inputs!(data);
#[macro_export]
macro_rules! synthesis_pattern_outputs{($m:ident)=>{$m!{sources:$crate::domain::synthesis::patterns::AuthoredCodeSource,conclusions:$crate::domain::synthesis::patterns::AuthoredCodeConclusion,anchors:$crate::domain::synthesis::patterns::CodeAnchor,statements:$crate::domain::synthesis::patterns::AuthoredStatement,setup_sources:$crate::domain::synthesis::patterns::SetupSource,dependencies:$crate::domain::synthesis::patterns::SetupDependency,boundaries:$crate::domain::synthesis::patterns::CodeBoundary,qualifications:$crate::domain::assertion::AssertionQualification,}};}
macro_rules! output{($($f:ident:$t:ty,)*)=>{pub struct Output{$(pub $f:Rows<$t>,)*}impl Output{pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]}pub fn matches(&self,o:&Self)->Result<(),ModelError>{$(if !stages::is_vocabulary(<$t>::NAME)&&!self.$f.same(&o.$f){return Err(invalid(concat!("authored code closure differs: ",stringify!($f))));}for row in o.$f.iter(){if self.$f.get(row.id())!=Some(row){return Err(invalid("authored code output erased"));}})*Ok(())}}};}
crate::synthesis_pattern_outputs!(output);
fn invalid(s: &str) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(r: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    r.get(id)
        .ok_or_else(|| invalid("exact authored code premise absent"))
}
fn setup_source(d: &source_setup::Dependency) -> Result<SetupSource, ModelError> {
    Ok(match d {
        source_setup::Dependency::Builtin {
            reference,
            resolution,
        } => SetupSource::Builtin {
            reference: *reference,
            resolution: *resolution,
        },
        source_setup::Dependency::Import {
            reference,
            resolution,
            binding,
            alias,
        } => SetupSource::Import {
            reference: *reference,
            resolution: *resolution,
            binding: *binding,
            alias: *alias,
        },
        source_setup::Dependency::Internal {
            reference,
            resolution,
            binding,
        } => SetupSource::Internal {
            reference: *reference,
            resolution: *resolution,
            binding: *binding,
        },
        source_setup::Dependency::Named { reference, value } => {
            let handoffs::ValueSource::Named {
                observation,
                support,
                inventory,
                inventory_support,
                reaching,
                reaching_support,
                definition,
                definition_support,
                region,
                region_support,
                coverage,
            } = value
            else {
                return Err(invalid("named setup has no exact reaching proof"));
            };
            SetupSource::Named {
                reference: *reference,
                observation: *observation,
                support: *support,
                inventory: *inventory,
                inventory_support: *inventory_support,
                reaching: *reaching,
                reaching_support: *reaching_support,
                definition: *definition,
                definition_support: *definition_support,
                region: *region,
                region_support: *region_support,
                coverage: *coverage,
            }
        }
    })
}
fn wrapper(
    d: &Data,
    scenario: Id<c1::CatalogScenario>,
    expected: &c1::OriginalSource,
) -> Result<Id<c1::OriginalSource>, ModelError> {
    if d.originals.get(expected.id()) != Some(expected)
        || !d
            .spans
            .iter()
            .any(|s| s.scenario == scenario && s.source == expected.id())
    {
        return Err(invalid("authored code original wrapper erased or forged"));
    }
    Ok(expected.id())
}
fn anchor(
    d: &Data,
    docs: &documentary::Data,
    association: &c1::ScenarioAssociation,
    occurrence: Id<source::Occurrence>,
    context: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<Result<CodeAnchor, BoundaryReason>, ModelError> {
    let scenario = need(&d.scenarios, association.scenario)?;
    let source = need(&d.scenario_sources, scenario.source)?;
    let occurrence = need(&docs.occurrences, occurrence)?;
    Ok(match source {
        c1::ScenarioSource::Python {
            artifact,
            role,
            context: source_context,
            ..
        } => {
            if *source_context != context
                || *artifact != occurrence.source
                || !matches!(
                    role,
                    input::SourceRole::Example
                        | input::SourceRole::DocBlock
                        | input::SourceRole::Test
                )
            {
                return Ok(Err(BoundaryReason::ForeignSource));
            }
            let original = c1::OriginalSource::Artifact {
                artifact: *artifact,
            };
            let wrapper = wrapper(d, scenario.id(), &original)?;
            Ok(CodeAnchor::Occurrence {
                occurrence: occurrence.id(),
                wrapper,
            })
        }
        c1::ScenarioSource::Fence { observation } => {
            let block = need(&d.blocks, *observation)?;
            let mapping = match source_code::admit_fence(docs, block, context, b)? {
                Ok(mapping) => mapping,
                Err(_) => return Ok(Err(BoundaryReason::MappingUnavailable)),
            };
            mapping.occurrence(docs, occurrence, b)?;
            let original = c1::OriginalSource::Span {
                span: mapping.span(),
            };
            let wrapper = wrapper(d, scenario.id(), &original)?;
            Ok(CodeAnchor::Fence {
                block: *observation,
                occurrence: occurrence.id(),
                wrapper,
            })
        }
    })
}
pub fn original(
    d: &Data,
    docs: &documentary::Data,
    anchor: &CodeAnchor,
    context: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<source_code::OriginalRange, ModelError> {
    match anchor {
        CodeAnchor::Occurrence {
            occurrence,
            wrapper,
        } => {
            let occurrence = need(&docs.occurrences, *occurrence)?;
            if !matches!(need(&d.originals,*wrapper)?,c1::OriginalSource::Artifact{artifact}if *artifact==occurrence.source)
            {
                return Err(invalid(
                    "authored statement changes original artifact wrapper",
                ));
            }
            Ok(source_code::OriginalRange {
                artifact: occurrence.source,
                start: occurrence.start,
                end: occurrence.end,
            })
        }
        CodeAnchor::Fence {
            block,
            occurrence,
            wrapper,
        } => {
            let mapping = source_code::admit_fence(docs, need(&d.blocks, *block)?, context, b)?
                .map_err(|_| invalid("original fence mapping no longer admitted"))?;
            if !matches!(need(&d.originals,*wrapper)?,c1::OriginalSource::Span{span}if *span==mapping.span())
            {
                return Err(invalid("authored statement changes original fence wrapper"));
            }
            mapping.occurrence(docs, need(&docs.occurrences, *occurrence)?, b)
        }
    }
}
pub fn code(
    d: &Data,
    docs: &documentary::Data,
    out: &Output,
    conclusion: &AuthoredCodeConclusion,
    context: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<documentary::Bytes, ModelError> {
    let _order = b.reserve(
        "authored-code-statement-order",
        out.statements.len() * size_of::<&AuthoredStatement>(),
    )?;
    let mut rows = out
        .statements
        .iter()
        .filter(|s| s.conclusion == conclusion.id())
        .collect::<Vec<_>>();
    rows.sort_by_key(|r| r.ordinal);
    if rows.is_empty() || rows.iter().enumerate().any(|(i, r)| r.ordinal != i as i64) {
        return Err(invalid("authored code ordered statements incomplete"));
    }
    let mut len = 0usize;
    for row in &rows {
        let range = original(d, docs, need(&out.anchors, row.anchor)?, context, b)?;
        len = len
            .checked_add((range.end - range.start) as usize + 1)
            .ok_or_else(|| invalid("authored code allowance overflow"))?;
    }
    let reservation = b.reserve(
        "authored-code-original-subset",
        len + size_of::<documentary::Bytes>(),
    )?;
    let mut text = String::with_capacity(len);
    for row in rows {
        let range = original(d, docs, need(&out.anchors, row.anchor)?, context, b)?;
        let bytes = documentary::read_range(docs, range.artifact, range.start, range.end, b)?;
        text.push_str(&bytes.value);
        if !bytes.value.ends_with('\n') {
            text.push('\n');
        }
    }
    Ok(documentary::Bytes {
        value: text,
        _reservation: reservation,
    })
}
/// Earlier C1 association and A0 handoff bound this operation's entire expected universe.
pub fn build(
    d: &Data,
    docs: &documentary::Data,
    frames: &Rows<frames::Frame>,
    invocations: &Rows<owner::Invocation>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    for frame in frames.iter() {
        let inv = need(invocations, frame.invocation)?;
        for group in d.groups.iter().filter(|g| g.frame == frame.structural) {
            for member in docs.member_frames.iter().filter(|m| {
                docs.core_invocations
                    .get(m.invocation)
                    .is_some_and(|i| (i.input, i.context) == (inv.input, inv.context))
            }) {
                let mut candidate = false;
                for c in docs.candidates.iter().filter(|c| {
                    docs.exposures
                        .get(c.exposure)
                        .is_some_and(|e| e.member == member.member)
                }) {
                    if documentary::entity(docs, c)? == Some(group.seed) {
                        candidate = true;
                        break;
                    }
                }
                if !candidate {
                    continue;
                }
                let _order = b.reserve(
                    "authored-handoff-witness-order",
                    d.members.len() * size_of::<&handoffs::Member>(),
                )?;
                let mut witnesses = d
                    .members
                    .iter()
                    .filter(|m| m.group == group.id())
                    .collect::<Vec<_>>();
                witnesses.sort_by_key(|m| m.ordinal);
                'witness: for witness in witnesses {
                    let handoff = need(&d.handoffs, witness.occurrence)?;
                    if handoff.frame != frame.structural {
                        return Err(invalid("authored code handoff changes structural frame"));
                    }
                    let producer = need(&d.alternatives, handoff.producer)?;
                    let consumer = need(&d.alternatives, handoff.consumer)?;
                    let p = need(&d.events, producer.event)?;
                    let c = need(&d.events, consumer.event)?;
                    let role = need(&d.roles, handoff.role)?;
                    let source = need(&docs.occurrences, c.site)?.source;
                    if p.context != inv.context
                        || c.context != inv.context
                        || role.input != inv.input
                        || role.artifact != source
                        || !matches!(
                            role.role,
                            input::SourceRole::Example
                                | input::SourceRole::DocBlock
                                | input::SourceRole::Test
                        )
                        || need(&docs.occurrences, p.site)?.source != source
                    {
                        return Err(invalid(
                            "authored code changes exact official handoff source",
                        ));
                    }
                    let mut associated = false;
                    for association in d.associations.iter().filter(|a| {
                        a.member == member.member
                            && (a.alternative == handoff.producer
                                || a.alternative == handoff.consumer)
                    }) {
                        associated = true;
                        let plan = match source_setup::close(
                            docs,
                            &d.setup,
                            d.flow
                                .entry
                                .coverage
                                .iter()
                                .any(|r| {
                                    r.family == attribution::FactFamily::Flow
                                        && r.status != attribution::CoverageStatus::NotRequested
                                })
                                .then_some(&d.flow),
                            &[p.site, c.site],
                            inv.context,
                            b,
                        )? {
                            Ok(plan) => plan,
                            Err(reason) => {
                                out.boundaries.insert(CodeBoundary {
                                    frame: frame.id(),
                                    member: member.id(),
                                    group: group.id(),
                                    handoff: handoff.id(),
                                    association: Some(association.id()),
                                    reason: match reason {
                                        source_setup::Boundary::FlowUnavailable => {
                                            BoundaryReason::FlowUnavailable
                                        }
                                        source_setup::Boundary::StatementLimit => {
                                            BoundaryReason::StatementLimit
                                        }
                                        _ => BoundaryReason::SetupUnavailable,
                                    },
                                })?;
                                continue;
                            }
                        };
                        let code_source = AuthoredCodeSource {
                            frame: frame.id(),
                            member: member.id(),
                            group: group.id(),
                            handoff: handoff.id(),
                            association: association.id(),
                            source_input: need(&docs.artifacts, source)?.input,
                        };
                        let q = assertion::AssertionQualification {
                            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
                            context: inv.context,
                            scope: (source::CoverageScope::Input { input: inv.input }).id(),
                            condition: conditions::Diagram::always().id(),
                            modality: attribution::Modality::Candidate,
                            approximation: assertion::Approximation::Over,
                        };
                        let mut anchors = Rows::new(b);
                        let mut digest = KeySink::new("authored-code-statement-subset");
                        for (occurrence, premise) in plan.statements() {
                            let anchor =
                                match anchor(d, docs, association, *occurrence, inv.context, b)? {
                                    Ok(anchor) => anchor,
                                    Err(reason) => {
                                        out.boundaries.insert(CodeBoundary {
                                            frame: frame.id(),
                                            member: member.id(),
                                            group: group.id(),
                                            handoff: handoff.id(),
                                            association: Some(association.id()),
                                            reason,
                                        })?;
                                        continue 'witness;
                                    }
                                };
                            let range = original(d, docs, &anchor, inv.context, b)?;
                            anchor.id().encode(&mut digest);
                            premise.encode(&mut digest);
                            ContentHash::of(
                                documentary::read_range(
                                    docs,
                                    range.artifact,
                                    range.start,
                                    range.end,
                                    b,
                                )?
                                .value
                                .as_bytes(),
                            )
                            .encode(&mut digest);
                            anchors.insert(anchor)?;
                        }
                        let conclusion = AuthoredCodeConclusion {
                            source: code_source.id(),
                            code: digest.finish(),
                            qualification: q.id(),
                            status: analysis::policy::EvidenceStatus::Documented,
                        };
                        out.sources.insert(code_source)?;
                        out.qualifications.insert(q)?;
                        for (ordinal, (occurrence, premise)) in plan.statements().iter().enumerate()
                        {
                            let anchor = anchors
                                .iter()
                                .find(|a| match a {
                                    CodeAnchor::Occurrence { occurrence: o, .. }
                                    | CodeAnchor::Fence { occurrence: o, .. } => o == occurrence,
                                })
                                .ok_or_else(|| invalid("authored statement anchor absent"))?;
                            out.statements.insert(AuthoredStatement {
                                conclusion: conclusion.id(),
                                ordinal: ordinal as i64,
                                placement: *premise,
                                anchor: anchor.id(),
                            })?;
                        }
                        for anchor in anchors.iter() {
                            out.anchors.insert(anchor.clone())?;
                        }
                        for (ordinal, dependency) in plan.dependencies().iter().enumerate() {
                            let source = out.setup_sources.insert(setup_source(dependency)?)?;
                            out.dependencies.insert(SetupDependency {
                                conclusion: conclusion.id(),
                                ordinal: ordinal as i64,
                                source,
                            })?;
                        }
                        out.conclusions.insert(conclusion)?;
                        break 'witness;
                    }
                    if !associated {
                        out.boundaries.insert(CodeBoundary {
                            frame: frame.id(),
                            member: member.id(),
                            group: group.id(),
                            handoff: handoff.id(),
                            association: None,
                            reason: BoundaryReason::MissingAssociation,
                        })?;
                    }
                }
            }
        }
    }
    Ok(out)
}
pub fn relations() -> Vec<Relation> {
    macro_rules! rows{($($f:ident:$t:ty,)*)=>{vec![$(Relation::of::<$t>()),*]};}
    let mut rows = crate::synthesis_pattern_outputs!(rows);
    rows.retain(|r| !stages::is_vocabulary(r.name()));
    rows
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::inputs(stages::Profile::Behavioral);
    inputs.extend(documentary::Data::validation_inputs());
    inputs.extend(Output::inputs());
    inputs.extend([
        ValidationInput::of::<frames::Frame>(&["id"]),
        ValidationInput::of::<owner::Invocation>(&["id"]),
    ]);
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    vec![Invariant {
        revision: 1,
        name: "authored_original_statement_replay",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                docs: documentary::Data::new(b),
                frames: Rows::new(b),
                invocations: Rows::new(b),
                output: Output::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: Data,
    docs: documentary::Data,
    frames: Rows<frames::Frame>,
    invocations: Rows<owner::Invocation>,
    output: Output,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        let d = self.data.visit(n, b)?;
        let docs = self.docs.visit(n, b)?;
        let out = self.output.visit(n, b)?;
        if n == frames::Frame::NAME {
            self.frames.decode(b)?;
            return Ok(());
        }
        if n == owner::Invocation::NAME {
            self.invocations.decode(b)?;
            return Ok(());
        }
        if !d && !docs && !out {
            return Err(invalid("undeclared authored code replay input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.output.matches(&build(
            &self.data,
            &self.docs,
            &self.frames,
            &self.invocations,
            &self.budget,
        )?)
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> { vec!["authored_original_statement_replay"] }

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::domain::normalized::events::{NormalizedCallAlternative, NormalizedCallEvent};
    use crate::domain::{analysis::native::*, lexical::*, source::*, syntax::SyntaxPlacement};
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn native(
        d: &mut documentary::Data,
        p: NativeAssertionPremise,
        q: Id<assertion::AssertionQualification>,
    ) {
        d.native_qualifications
            .insert(NativeQualification {
                premise: p.id(),
                qualification: q,
                family: p.family(),
                fidelity: attribution::Fidelity::NativeStructural,
                status: analysis::policy::native_status(
                    p.family(),
                    attribution::Fidelity::NativeStructural,
                ),
            })
            .unwrap();
        d.native.insert(p).unwrap();
    }
    /// This pure renderer fixture assumes admitted native/normalized/C1/A0 rows. It does not
    /// substitute for the final native generation's extraction, binding and store qualification.
    pub(crate) fn fixture() -> (
        ResourceBudget,
        Data,
        documentary::Data,
        Rows<frames::Frame>,
        Rows<owner::Invocation>,
    ) {
        let (b, mut docs, member) = documentary::tests::fixture("\"Run.\"", "Run.");
        let core = docs.core_invocations.iter().next().unwrap().clone();
        let inv = owner::Invocation::new(
            core.input,
            core.context,
            super::super::build::definition().1.id(),
            None,
            [],
        )
        .0;
        let frame = frames::Frame {
            invocation: inv.id(),
            configuration: id(80),
            core: core.id(),
            evidence: id(81),
            selection: id(82),
            structural: id(83),
            analytic: id(84),
            summary: id(85),
        };
        let mut d = Data::new(&b);
        let bytes = "from api import run, other\nother(run())\n";
        let artifact =
            SourceArtifact::from_bytes(core.input, "examples/handoff.py".into(), bytes.as_bytes())
                .unwrap();
        docs.artifacts.insert(artifact.clone()).unwrap();
        for chunk in artifact::ArtifactChunk::split(&artifact, bytes.as_bytes()).unwrap() {
            docs.chunks.insert(chunk).unwrap();
        }
        let q = assertion::AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: core.context,
            scope: CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        };
        docs.qualifications.insert(q.clone()).unwrap();
        let base = docs.occurrences.iter().next().unwrap().clone();
        struct OccurrenceSite<'a> {
            artifact: Id<SourceArtifact>,
            start: i64,
            end: i64,
            kind: SyntaxKind,
            path: &'a str,
            parent: Option<Id<Occurrence>>,
            field: SyntaxField,
            ordinal: i64,
            q: Id<assertion::AssertionQualification>,
        }

        fn occurrence(
            d: &mut documentary::Data,
            base: &Occurrence,
            occurrence_site: OccurrenceSite<'_>,
        ) -> Id<Occurrence> {
            let OccurrenceSite {
                artifact,
                start,
                end,
                kind,
                path,
                parent,
                field,
                ordinal,
                q,
            } = occurrence_site;
            let o = Occurrence {
                source: artifact,
                start,
                end,
                syntax_kind: kind,
                structural_path: path.bytes().map(i32::from).collect(),
                ..base.clone()
            };
            let id = docs_insert(d, o);
            let p = SyntaxPlacement {
                qualification: q,
                occurrence: id,
                parent,
                field,
                ordinal,
            };
            d.placements.insert(p.clone()).unwrap();
            native(
                d,
                NativeAssertionPremise::SyntaxPlacement {
                    assertion: p.id(),
                    support: super::tests::id(100 + ordinal as u8),
                },
                q,
            );
            id
        }
        fn docs_insert(d: &mut documentary::Data, o: Occurrence) -> Id<Occurrence> {
            d.occurrences.insert(o).unwrap()
        }
        let module = occurrence(
            &mut docs,
            &base,
            OccurrenceSite {
                artifact: artifact.id(),
                start: 0,
                end: bytes.len() as i64,
                kind: SyntaxKind::ModModule,
                path: "m",
                parent: None,
                field: SyntaxField::Body,
                ordinal: 0,
                q: q.id(),
            },
        );
        let import = occurrence(
            &mut docs,
            &base,
            OccurrenceSite {
                artifact: artifact.id(),
                start: 0,
                end: 26,
                kind: SyntaxKind::StmtImportFrom,
                path: "m.i",
                parent: Some(module),
                field: SyntaxField::Body,
                ordinal: 0,
                q: q.id(),
            },
        );
        let statement = occurrence(
            &mut docs,
            &base,
            OccurrenceSite {
                artifact: artifact.id(),
                start: 27,
                end: 39,
                kind: SyntaxKind::StmtExpr,
                path: "m.s",
                parent: Some(module),
                field: SyntaxField::Body,
                ordinal: 1,
                q: q.id(),
            },
        );
        let consumer = occurrence(
            &mut docs,
            &base,
            OccurrenceSite {
                artifact: artifact.id(),
                start: 27,
                end: 39,
                kind: SyntaxKind::ExprCall,
                path: "m.s.c",
                parent: Some(statement),
                field: SyntaxField::Value,
                ordinal: 0,
                q: q.id(),
            },
        );
        let producer = occurrence(
            &mut docs,
            &base,
            OccurrenceSite {
                artifact: artifact.id(),
                start: 33,
                end: 38,
                kind: SyntaxKind::ExprCall,
                path: "m.s.c.p",
                parent: Some(consumer),
                field: SyntaxField::Argument,
                ordinal: 0,
                q: q.id(),
            },
        );
        for (name, start, end, site, parent, ordinal) in [
            ("run", 16, 19, producer, producer, 0),
            ("other", 21, 26, consumer, consumer, 1),
        ] {
            let alias = occurrence(
                &mut docs,
                &base,
                OccurrenceSite {
                    artifact: artifact.id(),
                    start,
                    end,
                    kind: SyntaxKind::Alias,
                    path: &format!("m.i.a{ordinal}"),
                    parent: Some(import),
                    field: SyntaxField::Child,
                    ordinal,
                    q: q.id(),
                },
            );
            let event = BindingEvent {
                site: alias,
                name: name.into(),
            };
            d.setup.events.insert(event.clone()).unwrap();
            let binding = BindingObservation {
                qualification: q.id(),
                event: event.id(),
                scope: id(86),
                kind: BindingEventKind::FromImport,
                ordinal,
                value: None,
                static_branch: None,
                static_polarity: None,
            };
            d.setup.bindings.insert(binding.clone()).unwrap();
            native(
                &mut docs,
                NativeAssertionPremise::BindingObservation {
                    assertion: binding.id(),
                    support: id(110 + ordinal as u8),
                },
                q.id(),
            );
            let a = syntax::ImportAliasObservation {
                qualification: q.id(),
                statement: import,
                alias,
                level: 0,
                resolved_module: Some("api".into()),
            };
            d.setup.imports.insert(a.clone()).unwrap();
            native(
                &mut docs,
                NativeAssertionPremise::ImportAliasObservation {
                    assertion: a.id(),
                    support: id(120 + ordinal as u8),
                },
                q.id(),
            );
            let read = occurrence(
                &mut docs,
                &base,
                OccurrenceSite {
                    artifact: artifact.id(),
                    start: if ordinal == 0 { 33 } else { 27 },
                    end: if ordinal == 0 { 36 } else { 32 },
                    kind: SyntaxKind::ExprName,
                    path: &format!("m.s.n{ordinal}"),
                    parent: Some(site),
                    field: SyntaxField::Callee,
                    ordinal,
                    q: q.id(),
                },
            );
            let reference = ReferenceObservation {
                qualification: q.id(),
                read,
                scope: id(86),
                parent,
                field: SyntaxField::Callee,
                name: name.into(),
            };
            d.setup.references.insert(reference.clone()).unwrap();
            native(
                &mut docs,
                NativeAssertionPremise::ReferenceObservation {
                    assertion: reference.id(),
                    support: id(130 + ordinal as u8),
                },
                q.id(),
            );
            let target = LexicalTarget::Binding { event: event.id() };
            d.setup.targets.insert(target.clone()).unwrap();
            let resolution = LexicalResolution {
                qualification: q.id(),
                read,
                target: target.id(),
                captured: false,
            };
            d.setup.resolutions.insert(resolution.clone()).unwrap();
            native(
                &mut docs,
                NativeAssertionPremise::LexicalResolution {
                    assertion: resolution.id(),
                    support: id(140 + ordinal as u8),
                },
                q.id(),
            );
        }
        let entity = documentary::entity(&docs, docs.candidates.iter().next().unwrap())
            .unwrap()
            .unwrap();
        let event = |site| NormalizedCallEvent {
            site,
            origin: id(87),
            context: core.context,
            owner: id(88),
        };
        let pe = event(producer);
        let ce = event(consumer);
        d.events.insert(pe.clone()).unwrap();
        d.events.insert(ce.clone()).unwrap();
        let alternative = |event, entity| NormalizedCallAlternative {
            event,
            source: id(89),
            resolution: None,
            correspondence: None,
            entity: Some(entity),
            status: normalized::entities::ResolutionStatus::Resolved,
            reason: normalized::links::LinkReason::ExplicitIdentity,
        };
        let p = alternative(pe.id(), entity);
        let c = alternative(ce.id(), id(90));
        d.alternatives.insert(p.clone()).unwrap();
        d.alternatives.insert(c.clone()).unwrap();
        let role = input::ArtifactUse {
            artifact: artifact.id(),
            input: core.input,
            role: input::SourceRole::Example,
        };
        d.roles.insert(role.clone()).unwrap();
        let handoff = handoffs::Handoff {
            frame: frame.structural,
            consumer: c.id(),
            producer: p.id(),
            binding: id(91),
            value: id(92),
            role: role.id(),
        };
        d.handoffs.insert(handoff.clone()).unwrap();
        let group = handoffs::Group {
            frame: frame.structural,
            seed: entity,
            other: id(90),
            formal: id(93),
            producer_phase: calls::CallPhase::Call,
            consumer_phase: calls::CallPhase::Call,
            producer_modality: attribution::Modality::Candidate,
            consumer_modality: attribution::Modality::Candidate,
            occurrences: 1,
            retained: 1,
        };
        d.groups.insert(group.clone()).unwrap();
        d.members
            .insert(handoffs::Member {
                group: group.id(),
                ordinal: 0,
                occurrence: handoff.id(),
            })
            .unwrap();
        let scenario_source = c1::ScenarioSource::Python {
            artifact: artifact.id(),
            role: input::SourceRole::Example,
            context: core.context,
            declaration: None,
        };
        d.scenario_sources.insert(scenario_source.clone()).unwrap();
        let scenario = c1::CatalogScenario {
            source: scenario_source.id(),
            extraction: deployment::CheckStatus::Passed,
            parse: deployment::CheckStatus::Passed,
            binding: deployment::CheckStatus::NotRun,
            environment: deployment::CheckStatus::NotRun,
            execution: deployment::CheckStatus::NotRun,
            intent: c1::Intent::Demonstration,
        };
        d.scenarios.insert(scenario.clone()).unwrap();
        let original = c1::OriginalSource::Artifact {
            artifact: artifact.id(),
        };
        d.originals.insert(original.clone()).unwrap();
        d.spans
            .insert(c1::ScenarioSpan {
                scenario: scenario.id(),
                ordinal: 0,
                role: c1::SpanRole::Primary,
                source: original.id(),
            })
            .unwrap();
        d.associations
            .insert(c1::ScenarioAssociation {
                scenario: scenario.id(),
                member: docs.member_frames.get(member).unwrap().member,
                alternative: p.id(),
                qualification: q.id(),
                phase: calls::CallPhase::Call,
                basis: c1::AssociationBasis::CandidateTarget,
                intent: c1::Intent::Demonstration,
            })
            .unwrap();
        let mut frames = Rows::new(&b);
        frames.insert(frame).unwrap();
        let mut invocations = Rows::new(&b);
        invocations.insert(inv).unwrap();
        (b, d, docs, frames, invocations)
    }
    #[test]
    fn catalog_pattern_inputs_leave_native_flow_unrequested() {
        let catalog = Data::inputs(stages::Profile::Catalog);
        let behavioral = Data::inputs(stages::Profile::Behavioral);
        let flow_inputs = [
            flow::FlowSourceViewObservation::NAME,
            flow::FlowSourceViewSupport::NAME,
            flow_inventory::FlowUseInventoryObservation::NAME,
            flow_inventory::FlowUseInventorySupport::NAME,
            flow_inventory::FlowUseCandidate::NAME,
            flow_inventory::FlowUseInventoryMember::NAME,
        ];
        for name in flow_inputs {
            assert!(!catalog.iter().any(|input| input.name() == name), "{name}");
            assert!(
                behavioral.iter().any(|input| input.name() == name),
                "{name}"
            );
        }
        let model = crate::domain::model().unwrap();
        for relation in model.relations() {
            if relation.family() == Some(attribution::FactFamily::Flow) {
                assert!(
                    !catalog.iter().any(|input| input.name() == relation.name()),
                    "Catalog must not require a native Flow writer: {}",
                    relation.name()
                );
            }
        }
    }
    #[test]
    fn official_nested_handoff_retains_whole_statement_import_setup_and_original_bytes() {
        let (b, d, docs, f, i) = fixture();
        let out = build(&d, &docs, &f, &i, &b).unwrap();
        assert_eq!(out.conclusions.len(), 1);
        assert_eq!(out.statements.len(), 2);
        assert_eq!(out.dependencies.len(), 2);
        let proof = out.conclusions.iter().next().unwrap();
        assert_eq!(
            proof.source_facts().status,
            analysis::policy::EvidenceStatus::Documented
        );
        let q = out.qualifications.get(proof.qualification()).unwrap();
        assert_eq!(
            (q.modality, q.approximation),
            (
                attribution::Modality::Candidate,
                assertion::Approximation::Over
            )
        );
        let bytes = code(&d, &docs, &out, proof, i.iter().next().unwrap().context, &b).unwrap();
        assert_eq!(bytes.value, "from api import run, other\nother(run())\n");
        for a in out.anchors.iter() {
            let range = original(&d, &docs, a, q.context, &b).unwrap();
            assert_eq!(
                docs.artifacts.get(range.artifact).unwrap().path,
                "examples/handoff.py"
            );
        }
    }
    #[test]
    fn missing_setup_foreign_source_and_original_wrapper_refuse_or_record_boundary() {
        for case in 0..3 {
            let (b, mut d, mut docs, f, i) = fixture();
            match case {
                0 => d.setup.imports = Rows::new(&b),
                1 => {
                    let old = d.roles.iter().next().unwrap().clone();
                    d.roles = Rows::new(&b);
                    d.roles
                        .insert(input::ArtifactUse {
                            input: id(155),
                            ..old
                        })
                        .unwrap();
                }
                _ => d.spans = Rows::new(&b),
            }
            let result = build(&d, &docs, &f, &i, &b);
            if case == 0 {
                let out = result.unwrap();
                assert!(out.conclusions.is_empty());
                assert!(
                    out.boundaries
                        .iter()
                        .any(|r| r.reason == BoundaryReason::SetupUnavailable)
                );
            } else {
                assert!(result.is_err());
            }
            docs.chunks = Rows::new(&b);
            assert!(build(&d, &docs, &f, &i, &b).is_err() || case == 0);
        }
    }
    #[test]
    fn full_replay_refuses_setup_removal_relocated_anchor_and_coupled_output_erasure() {
        let (b, d, docs, f, i) = fixture();
        let expected = build(&d, &docs, &f, &i, &b).unwrap();
        for case in 0..3 {
            let mut actual = build(&d, &docs, &f, &i, &b).unwrap();
            match case {
                0 => actual.dependencies = Rows::new(&b),
                1 => {
                    let mut row = actual.statements.iter().next().unwrap().clone();
                    row.anchor = id(156);
                    actual.statements = Rows::new(&b);
                    actual.statements.insert(row).unwrap();
                }
                _ => actual = Output::new(&b),
            }
            assert!(actual.matches(&expected).is_err());
        }
    }
}
