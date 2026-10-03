//! One finite declaration-domain producer and shared exact closure replay.
use super::*;
use crate::domain::{
    catalog::{
        self,
        evidence::{self as c1, build::EvidenceData},
    },
    normalized::{Rows, callables::Knowledge},
    resources::ResourceBudget,
    stages::*,
};
pub fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
pub fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("selection premise absent: {}", R::NAME)))
}
macro_rules! facts {($($f:ident:$ty:ty,)*)=>{pub struct Facts {$(pub $f:Rows<$ty>,)*}impl Facts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}fn uses()->Vec<RelationUse> {vec![$(RelationUse::stored::<$ty>()),*]}}};}
crate::catalog_selection_inputs!(facts);
pub struct Data {
    pub source: EvidenceData,
    pub evidence: c1::build::EvidenceOutput,
    pub facts: Facts,
}
impl Data {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            source: EvidenceData::new(b),
            evidence: c1::build::EvidenceOutput::new(b),
            facts: Facts::new(b),
        }
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        // One nominal input may feed more than one projection (notably decorators).
        // Hydrate every declared consumer, including invariant replay.
        let source=self.source.visit(n,b)?;
        let evidence=self.evidence.visit(n,b)?;
        let facts=self.facts.visit(n,b)?;
        Ok(source||evidence||facts)
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut r = EvidenceData::inputs();
        r.extend(c1::build::EvidenceOutput::inputs());
        r.extend(Facts::inputs());
        r.sort_by_key(|r| r.name());
        r.dedup_by_key(|r| r.name());
        r
    }
}
macro_rules! outputs {($($f:ident:$ty:ty,)*)=>{pub struct Output {$(pub $f:Rows<$ty>,)*}impl Output {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}pub fn matches(&self,other:&Self)->Result<(),ModelError> {$(if !self.$f.same(&other.$f) {return Err(invalid(format!("selection declaration closure differs: {}",<$ty>::NAME)));})*Ok(())}}};}
crate::catalog_selection_outputs!(outputs);
type Members = crate::domain::charged::ChargedSet<(Id<Context>, Id<Witness>)>;
fn add(
    out: &mut Output,
    members: &mut Members,
    charge: &mut crate::domain::charged::StateCharge,
    context: Context,
    witness: Witness,
) -> Result<(), ModelError> {
    let context = out.contexts.insert(context)?;
    let witness = out.witnesses.insert(witness)?;
    members.insert(charge, (context, witness))?;
    Ok(())
}
fn qualified(
    d: &Data,
    member: &catalog::CatalogMember,
    context: Id<attribution::AnalysisContext>,
) -> Result<bool, ModelError> {
    let mut found = false;
    for e in d
        .source
        .catalog
        .exposures
        .iter()
        .filter(|r| r.member == member.id())
    {
        let e = need(&d.source.core.exposures, e.exposure)?;
        if e.context == context {
            found = true;
            if e.status != normalized::entities::ResolutionStatus::Resolved {
                return Ok(false);
            }
        }
    }
    Ok(found)
}
fn ready(
    d: &Data,
    artifact: Id<source::SourceArtifact>,
    context: Id<attribution::AnalysisContext>,
    kind: DomainKind,
) -> bool {
    let family = match kind {
        DomainKind::SignatureVariants => attribution::FactFamily::Signatures,
        DomainKind::PublicExposures | DomainKind::SourceArtifacts => return true,
        _ => return false,
    };
    let scope = source::CoverageScope::Artifact { artifact }.id();
    let mut count = 0;
    let mut complete = true;
    for r in d
        .source
        .core
        .native_coverage
        .iter()
        .filter(|r| r.context == context && r.scope == scope && r.family == family)
    {
        count += 1;
        complete &= r.status == attribution::CoverageStatus::CompleteUnderStatedModel;
    }
    count > 0 && complete
}
pub fn build(d: &Data, b: &ResourceBudget) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    let mut charge = charged::StateCharge::new(b, "selection-declaration-domains");
    let mut frames = charged::ChargedSet::default();
    for link in d.source.facts.core_links.iter() {
        let member = need(&d.source.catalog.members, link.member)?;
        let i = need(&d.source.facts.core_invocations, link.invocation)?;
        if member.input != i.input {
            return Err(invalid("selection member crosses admitted C0 input"));
        }
        frames.insert(&mut charge, (link.member, i.context))?;
    }
    for (member_id, context) in frames.iter() {
        let member = need(&d.source.catalog.members, *member_id)?;
        let module = need(&d.source.core.modules, member.access)?;
        let artifact = need(&d.source.core.artifacts, module.source)?;
        if artifact.input != member.input {
            return Err(invalid("selection access owner crosses input"));
        }
        for kind in [
            DomainKind::PublicExposures,
            DomainKind::SignatureVariants,
            DomainKind::ConfigurationFields,
            DomainKind::Relationships,
            DomainKind::Scenarios,
            DomainKind::SourceArtifacts,
            DomainKind::ReleaseDeclarations,
        ] {
            let mut members = Members::default();
            let mut closure = charged::ChargedSet::default();
            let witness = out
                .witnesses
                .insert(Witness::Member { member: *member_id })?;
            closure.insert(&mut charge, witness)?;
            for r in d.facts.core_coverage.iter().filter(|r| {
                r.context == *context
                    && d.source
                        .facts
                        .core_invocations
                        .get(r.invocation)
                        .is_some_and(|i| i.input == member.input)
            }) {
                closure.insert(
                    &mut charge,
                    out.witnesses
                        .insert(Witness::CoreCoverage { coverage: r.id() })?,
                )?;
            }
            for r in d.facts.evidence_coverage.iter().filter(|r| {
                r.context == *context
                    && d.facts
                        .evidence_invocations
                        .get(r.invocation)
                        .is_some_and(|i| i.input == member.input)
            }) {
                closure.insert(
                    &mut charge,
                    out.witnesses
                        .insert(Witness::EvidenceCoverage { coverage: r.id() })?,
                )?;
            }
            let mut semantic_complete = false;
            match kind {
                DomainKind::PublicExposures => {
                    add(
                        &mut out,
                        &mut members,
                        &mut charge,
                        Context::Member {
                            member: *member_id,
                            analysis: *context,
                        },
                        Witness::Member { member: *member_id },
                    )?;
                    for r in d.source.catalog.candidates.iter() {
                        let link = need(&d.source.catalog.exposures, r.exposure)?;
                        if link.member != *member_id
                            || need(&d.source.core.exposures, link.exposure)?.context != *context
                        {
                            continue;
                        }
                        add(
                            &mut out,
                            &mut members,
                            &mut charge,
                            Context::Binding {
                                member: *member_id,
                                candidate: r.id(),
                                analysis: *context,
                            },
                            Witness::Candidate { candidate: r.id() },
                        )?;
                    }
                    for candidate in d.source.catalog.candidates.iter() {
                        let exposure = need(&d.source.catalog.exposures, candidate.exposure)?;
                        if exposure.member != *member_id { continue; }
                        let entity = if let Some(path) = candidate.path { Some(need(&d.source.catalog.paths, path)?.entity) }
                            else if let Some(alias) = candidate.alias { Some(need(&d.source.catalog.aliases, alias)?.entity) }
                            else { candidate.entity.map(|entity| need(&d.source.core.entity_candidates, entity).map(|row| row.entity)).transpose()? };
                        for result in d.facts.exception_outcomes.iter().filter(|row| row.context == *context && row.input == member.input && Some(row.owner) == entity) {
                            add(&mut out, &mut members, &mut charge, Context::Binding { member: *member_id, candidate: candidate.id(), analysis: *context }, Witness::SummaryException { outcome: result.id() })?;
                        }
                    }
                    semantic_complete = qualified(d, member, *context)?;
                }
                DomainKind::SignatureVariants => {
                    semantic_complete = qualified(d, member, *context)?;
                    let mut count = 0;
                    for r in d.source.catalog.invocations.iter() {
                        let callable = need(&d.source.catalog.callables, r.callable)?;
                        if callable.member != *member_id {
                            continue;
                        }
                        let assessment = need(&d.source.core.assessments, callable.assessment)?;
                        if assessment.context != *context {
                            continue;
                        }
                        let variant = need(&d.source.core.variants, r.variant)?;
                        let sig = need(&d.facts.signatures, variant.signature)?;
                        let complete = assessment.signatures == Knowledge::Known
                            && variant.adjustment
                                != normalized::callables::SignatureAdjustment::Unknown
                            && sig.form == calls::SignatureForm::List
                            && callable.basis == catalog::CatalogContractBasis::PublicCandidate;
                        semantic_complete &= complete;
                        count += 1;
                        add(
                            &mut out,
                            &mut members,
                            &mut charge,
                            Context::Signature {
                                member: *member_id,
                                candidate: callable.candidate,
                                invocation: r.id(),
                                analysis: *context,
                            },
                            Witness::Invocation { invocation: r.id() },
                        )?;
                        let context_row = Context::Signature {
                            member: *member_id,
                            candidate: callable.candidate,
                            invocation: r.id(),
                            analysis: *context,
                        };
                        for typed in d.source.core.return_types.iter().filter(|t| t.variant == variant.id()) {
                            add(&mut out, &mut members, &mut charge, context_row.clone(), Witness::SignatureTypeObservation { observation: typed.observation })?;
                        }
                        for typed in d.source.core.slot_types.iter().filter(|t| d.source.core.slots.get(t.slot).is_some_and(|s| s.variant == variant.id())) {
                            add(&mut out, &mut members, &mut charge, context_row.clone(), Witness::SignatureTypeObservation { observation: typed.observation })?;
                        }

                        for slot in d
                            .source
                            .core
                            .slots
                            .iter()
                            .filter(|s| s.variant == r.variant)
                        {
                            add(
                                &mut out,
                                &mut members,
                                &mut charge,
                                context_row.clone(),
                                Witness::SignatureSlot { slot: slot.id() },
                            )?;
                            for option in d.source.catalog.options.iter().filter(|o|o.member==*member_id && d.source.catalog.subjects.get(o.subject).is_some_and(|s|matches!(s,catalog::CatalogOptionSubject::Parameter {slot:s} if *s==slot.id()))) {add(&mut out,&mut members,&mut charge,context_row.clone(),Witness::Option {option:option.id()})?;}
                            for entity in d
                                .source
                                .core
                                .slot_entities
                                .iter()
                                .filter(|e| e.slot == slot.id())
                            {
                                let link = need(&d.source.core.parameter_links, entity.link)?;
                                let normalized::entities::ParameterEntity::Source { declaration } =
                                    need(&d.source.core.parameters, link.entity)?
                                else {
                                    continue;
                                };
                                for observation in d.facts.type_observations.iter().filter(|o| {
                                    o.subject == *declaration
                                        && o.role == types::TypeRole::Parameter
                                        && o.declared
                                }) {
                                    let q = need(
                                        &d.source.core.qualifications,
                                        observation.qualification,
                                    )?;
                                    if q.context == *context
                                        && q.approximation == assertion::Approximation::Exact
                                        && q.modality == attribution::Modality::Definite
                                        && q.condition == conditions::Diagram::always().id()
                                    {
                                        add(
                                            &mut out,
                                            &mut members,
                                            &mut charge,
                                            context_row.clone(),
                                            Witness::TypeObservation {
                                                observation: observation.id(),
                                            },
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                    semantic_complete &= count > 0;
                }
                DomainKind::ConfigurationFields => {
                    for r in d
                        .source
                        .catalog
                        .options
                        .iter()
                        .filter(|r| r.member == *member_id)
                    {
                        let catalog::CatalogOptionSubject::Field { field } =
                            need(&d.source.catalog.subjects, r.subject)?
                        else {
                            continue;
                        };
                        let field = need(&d.source.core.fields, *field)?;
                        add(
                            &mut out,
                            &mut members,
                            &mut charge,
                            Context::Configuration {
                                member: *member_id,
                                owner: field.class,
                                scope: ConfigurationScope::Object,
                                analysis: *context,
                            },
                            Witness::Option { option: r.id() },
                        )?;
                        for access in d.evidence.accesses.iter().filter(|a| {
                            a.option == r.id()
                                && d.source
                                    .core
                                    .qualifications
                                    .get(a.qualification)
                                    .is_some_and(|q| q.context == *context)
                        }) {
                            for link in d
                                .evidence
                                .field_locations
                                .iter()
                                .filter(|l| l.assessment == access.id())
                            {
                                add(
                                    &mut out,
                                    &mut members,
                                    &mut charge,
                                    Context::Configuration {
                                        member: *member_id,
                                        owner: field.class,
                                        scope: ConfigurationScope::Object,
                                        analysis: *context,
                                    },
                                    Witness::ReceiverLocation { link: link.id() },
                                )?;
                            }
                        }
                    }
                }
                DomainKind::Relationships | DomainKind::Scenarios => {
                    for r in d
                        .evidence
                        .associations
                        .iter()
                        .filter(|r| r.member == *member_id)
                    {
                        if need(&d.source.core.qualifications, r.qualification)?.context != *context
                        {
                            continue;
                        }
                        add(
                            &mut out,
                            &mut members,
                            &mut charge,
                            Context::Scenario {
                                member: *member_id,
                                scenario: r.scenario,
                                analysis: *context,
                            },
                            Witness::Association {
                                association: r.id(),
                            },
                        )?;
                        for link in d
                            .evidence
                            .constructor_candidates
                            .iter()
                            .filter(|l| l.association == r.id())
                        {
                            add(
                                &mut out,
                                &mut members,
                                &mut charge,
                                Context::Scenario {
                                    member: *member_id,
                                    scenario: r.scenario,
                                    analysis: *context,
                                },
                                Witness::ConstructorCandidate { link: link.id() },
                            )?;
                        }
                    }
                    if kind == DomainKind::Relationships {
                        for r in d.evidence.accesses.iter() {
                            let option = need(&d.source.catalog.options, r.option)?;
                            if option.member == *member_id
                                && need(&d.source.core.qualifications, r.qualification)?.context
                                    == *context
                            {
                                add(
                                    &mut out,
                                    &mut members,
                                    &mut charge,
                                    Context::Member {
                                        member: *member_id,
                                        analysis: *context,
                                    },
                                    Witness::FieldAccess { assessment: r.id() },
                                )?;
                                for link in d
                                    .evidence
                                    .field_locations
                                    .iter()
                                    .filter(|l| l.assessment == r.id())
                                {
                                    add(
                                        &mut out,
                                        &mut members,
                                        &mut charge,
                                        Context::Member {
                                            member: *member_id,
                                            analysis: *context,
                                        },
                                        Witness::ReceiverLocation { link: link.id() },
                                    )?;
                                }
                            }
                        }
                        for r in d
                            .evidence
                            .document_associations
                            .iter()
                            .filter(|r| r.member == *member_id)
                        {
                            let candidate = need(&d.source.facts.mention_candidates, r.candidate)?;
                            let assessment =
                                need(&d.source.facts.mention_assessments, candidate.assessment)?;
                            let mention = need(&d.source.facts.mentions, assessment.observation)?;
                            if need(&d.source.core.qualifications, mention.qualification)?.context
                                == *context
                            {
                                add(
                                    &mut out,
                                    &mut members,
                                    &mut charge,
                                    Context::Member {
                                        member: *member_id,
                                        analysis: *context,
                                    },
                                    Witness::Document {
                                        association: r.id(),
                                    },
                                )?;
                            }
                        }
                    }
                }
                DomainKind::SourceArtifacts => {
                    let original = c1::OriginalSource::Artifact {
                        artifact: module.source,
                    };
                    need(&d.evidence.original_sources, original.id())?;
                    add(
                        &mut out,
                        &mut members,
                        &mut charge,
                        Context::Source {
                            member: *member_id,
                            source: original.id(),
                            analysis: *context,
                        },
                        Witness::Original {
                            source: original.id(),
                        },
                    )?;
                    semantic_complete = true;
                    for association in d
                        .evidence
                        .associations
                        .iter()
                        .filter(|r| r.member == *member_id)
                    {
                        if need(&d.source.core.qualifications, association.qualification)?.context
                            != *context
                        {
                            continue;
                        }
                        for span in d
                            .evidence
                            .spans
                            .iter()
                            .filter(|r| r.scenario == association.scenario)
                        {
                            need(&d.evidence.original_sources, span.source)?;
                            add(
                                &mut out,
                                &mut members,
                                &mut charge,
                                Context::Source {
                                    member: *member_id,
                                    source: span.source,
                                    analysis: *context,
                                },
                                Witness::Original {
                                    source: span.source,
                                },
                            )?;
                        }
                    }
                }
                DomainKind::ReleaseDeclarations => {
                    for ownership in d
                        .source
                        .facts
                        .artifact_ownership
                        .iter()
                        .filter(|r| r.artifact == module.source)
                    {
                        let verification =
                            need(&d.source.facts.verifications, ownership.distribution)?;
                        for r in d
                            .evidence
                            .release_deployments
                            .iter()
                            .filter(|r| r.release == verification.release)
                        {
                            let deployment = need(&d.evidence.deployments, r.deployment)?;
                            let obs = need(&d.source.facts.deployment, deployment.observation)?;
                            if need(&d.source.core.qualifications, obs.qualification)?.context
                                != *context
                            {
                                continue;
                            }
                            add(
                                &mut out,
                                &mut members,
                                &mut charge,
                                Context::Release {
                                    release: verification.release,
                                    analysis: *context,
                                },
                                Witness::Deployment {
                                    deployment: deployment.id(),
                                },
                            )?;
                        }
                    }
                }
            }
            let mut context_digest = KeySink::new("selection-domain-contexts/v1");
            for (c, w) in members.iter() {
                c.encode(&mut context_digest);
                w.encode(&mut context_digest);
            }
            let mut closure_digest = KeySink::new("selection-domain-closure/v1");
            for w in closure.iter() {
                w.encode(&mut closure_digest);
            }
            let complete = ready(d, module.source, *context, kind) && semantic_complete;
            let domain = out.domains.insert(SelectionDomain {
                member: *member_id,
                analysis: *context,
                kind,
                corpus_complete: true,
                analyzer_complete: complete,
                contexts: context_digest.finish(),
                closure: closure_digest.finish(),
            })?;
            for (c, w) in members.iter() {
                out.members.insert(DomainContext {
                    domain,
                    context: *c,
                    complete,
                })?;
                out.evidence.insert(DomainEvidence {
                    domain,
                    context: *c,
                    witness: *w,
                })?;
            }
            for w in closure.iter() {
                out.closure.insert(DomainClosure {
                    domain,
                    evidence: *w,
                })?;
            }
        }
    }
    // Request-time answers only reference persisted canonical witness rows. These rows
    // characterize available facts; context admission and truth remain evaluator decisions.
    // Later stages append vocabulary rows. Only qualifications referenced by these
    // immutable native inputs belong to C2's located-typing witness universe.
    for row in d.source.core.native_signatures.iter(){need(&d.source.core.qualifications,row.qualification)?;out.witnesses.insert(Witness::Qualification{qualification:row.qualification})?;}
    for row in d.facts.generic_specializations.iter(){need(&d.source.core.qualifications,row.qualification)?;out.witnesses.insert(Witness::Qualification{qualification:row.qualification})?;}
    for row in d.source.core.signature_types.iter(){out.witnesses.insert(Witness::SignatureTypeObservation{observation:row.id()})?;}
    for row in d.facts.type_observations.iter(){out.witnesses.insert(Witness::TypeObservation{observation:row.id()})?;}
    for row in d.facts.generic_specializations.iter(){out.witnesses.insert(Witness::GenericSpecialization{observation:row.id()})?;}
    for support in d.facts.binding_supports.iter(){need(&d.source.core.bindings,support.assertion)?;out.witnesses.insert(Witness::LexicalDefinition{observation:support.assertion,support:support.id()})?;}
    for support in d.facts.declaration_supports.iter(){need(&d.source.core.declarations,support.assertion)?;out.witnesses.insert(Witness::SourceCharacterization{observation:support.assertion,support:support.id()})?;}
    for support in d.facts.metadata_supports.iter(){need(&d.source.core.class_metadata,support.assertion)?;out.witnesses.insert(Witness::ClassMetadata{observation:support.assertion,support:support.id()})?;}
    for support in d.facts.native_signature_supports.iter(){need(&d.source.core.native_signatures,support.assertion)?;out.witnesses.insert(Witness::NativeCallableMetadata{observation:support.assertion,support:support.id()})?;}
    for support in d.facts.type_supports.iter(){let row=need(&d.facts.type_observations,support.assertion)?;if row.role==types::TypeRole::Raised{out.witnesses.insert(Witness::RaisedType{observation:row.id(),support:support.id()})?;}}
    let inputs=normalized::decorator_identity::Inputs{qualifications:&d.source.core.qualifications,occurrences:&d.source.core.occurrences,placements:&d.source.core.placements,references:&d.source.core.references,assessments:&d.source.core.reference_assessments,candidates:&d.source.core.reference_candidates,targets:&d.source.core.reference_targets,resolutions:&d.source.core.lexical_resolutions};
    for support in d.facts.decorator_supports.iter(){let row=need(&d.facts.decorators,support.assertion)?;let q=need(&d.source.core.qualifications,row.qualification)?;let selected=inputs.select(row,q.context,b)?;for selection in selected.selections.iter(){if let(Some(assessment),Some(candidate))=(selection.assessment,selection.candidate){out.witnesses.insert(Witness::ResolvedDecorator{observation:row.id(),support:support.id(),assessment,candidate})?;}}}
    Ok(out)
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::inputs();
    inputs.extend(Output::inputs());
    vec![Invariant {
        name: "catalog_selection_declaration_closure",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: Data::new(b),
                out: Output::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: Data,
    out: Output,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if !self.data.visit(n, b)? && !self.out.visit(n, b)? {
            return Err(invalid("undeclared selection replay input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.out.matches(&build(&self.data, &self.budget)?)
    }
}
pub fn stage(
    profile: Profile,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Stage, ModelError> {
    let parent = c1::build::stage(profile, model, order)?;
    let mut inputs = parent.inputs;
    inputs.extend(parent.outputs.into_iter().map(|r| r.completed_store()));
    inputs.extend(Facts::uses());
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    use analysis::selection::*;
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
    let roots = inputs
        .iter()
        .map(|r| {
            let relation = model
                .relations()
                .iter()
                .find(|row| row.name() == r.name())
                .ok_or_else(|| invalid("closure root absent"))?;
            let mut input = ValidationInput::of_relation(relation, &["id"]);
            if let Some(epoch) = r.prefix() {
                input = input.at_epoch(epoch);
            }
            Ok(input)
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let inputs = dependency_closure::DependencyClosure::build(
        model,
        roots,
        inputs,
        &outputs,
        PublicationBoundary::Local,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?
    .grants;
    Ok(Stage {
        name: "catalog_selection",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("build.rs")),
        configuration: ContentHash::of(b"selection-declaration/v1"),
    })
}
pub fn definition() -> (analysis::MethodParameters, analysis::AnalysisDefinition) {
    let (parameters, _) = catalog::build::definition();
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::CatalogSelection,
        interpretation: analysis::Interpretation::Structural,
        parameters: parameters.id(),
        semantic_version: {
            let mut k = KeySink::new("catalog-selection/v2");
            for bytes in [
                include_bytes!("build.rs").as_slice(),
                include_bytes!("evaluate.rs").as_slice(),
                include_bytes!("classification.rs").as_slice(),
                include_bytes!("preparation.rs").as_slice(),
                include_bytes!("admission.rs").as_slice(),
                include_bytes!("vocabulary.rs").as_slice(),
                include_bytes!("frames.rs").as_slice(),
                include_bytes!("facets.rs").as_slice(),
                include_bytes!("specialization.rs").as_slice(),
                include_bytes!("structural_facets.rs").as_slice(),
                include_bytes!("../normalized/decorator_identity.rs").as_slice(),
            ] {
                ContentHash::of(bytes).encode(&mut k);
            }
            k.finish()
        },
    };
    (parameters, definition)
}
