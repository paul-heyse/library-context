//! One finite declaration-domain producer and shared exact closure replay.
// Increment for a meaning/rule change; implementation source bytes live in producer provenance.
const SEMANTIC_RULE_REVISION: i64 = 1;
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
macro_rules! facts {($($f:ident:$ty:ty,)*)=>{pub struct Facts {$(pub $f:Rows<$ty>,)*}impl Facts {pub fn new(b:&ResourceBudget)->Self {Self {$($f:Rows::new(b),)*}}pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*Ok(false)}pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"])),*]}fn uses()->Vec<RelationUse> {crate::domain::normalized::facts_stage_inputs(vec![$(RelationUse::completed::<$ty>()),*])}}};}
crate::catalog_selection_inputs!(facts);
pub struct Data {
    pub source: EvidenceData,
    pub evidence: c1::build::EvidenceOutput,
    pub facts: Facts,
    verification_releases:
        charged::ChargedMap<Id<input::DistributionVerification>, Id<input::Release>>,
    ownership_charge: charged::StateCharge,
}
impl Data {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            source: EvidenceData::new(b),
            evidence: c1::build::EvidenceOutput::new(b),
            facts: Facts::new(b),
            verification_releases: Default::default(),
            ownership_charge: charged::StateCharge::new(b, "selection-distribution-ownership"),
        }
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        if n == input::DistributionVerification::NAME {
            use arrow_array::Array;
            let ids = b
                .column_by_name("id")
                .and_then(|array| {
                    array
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .filter(|array| array.value_length() == 16 && array.null_count() == 0)
                .ok_or(ModelError::Schema("selection distribution identity"))?;
            let releases = b
                .column_by_name("release")
                .and_then(|array| {
                    array
                        .as_any()
                        .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
                })
                .filter(|array| array.value_length() == 16 && array.null_count() == 0)
                .ok_or(ModelError::Schema("selection distribution release"))?;
            fn id<T>(bytes: &[u8]) -> Result<Id<T>, ModelError> {
                serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                    _,
                    serde::de::value::Error,
                >::new(bytes.iter().copied()))
                .map_err(ModelError::codec)
            }
            for row in 0..b.num_rows() {
                self.insert_verification(id(ids.value(row))?, id(releases.value(row))?)?;
            }
            return Ok(true);
        }
        // One nominal input may feed more than one projection (notably decorators).
        // Hydrate every declared consumer, including invariant replay.
        let source = self.source.visit(n, b)?;
        let evidence = self.evidence.visit(n, b)?;
        let facts = self.facts.visit(n, b)?;
        Ok(source || evidence || facts)
    }
    fn insert_verification(
        &mut self,
        id: Id<input::DistributionVerification>,
        release: Id<input::Release>,
    ) -> Result<(), ModelError> {
        if self
            .verification_releases
            .get(&id)
            .is_some_and(|old| *old != release)
        {
            return Err(ModelError::Conflict(input::DistributionVerification::NAME));
        }
        self.verification_releases
            .insert(&mut self.ownership_charge, id, release)?;
        Ok(())
    }
    /// The same compact ownership preparation for direct typed finite-oracle callers.
    pub fn add_verification(
        &mut self,
        row: &input::DistributionVerification,
    ) -> Result<Id<input::DistributionVerification>, ModelError> {
        row.validate()?;
        self.insert_verification(row.id(), row.release)?;
        Ok(row.id())
    }
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        let n = input.name();
        if !is_vocabulary(n) {
            return self.visit(n, b);
        }
        if input.prefix() == Some(PublicationBoundary::Facts) {
            let source = self.source.visit_input(input, b)?;
            let facts = self.facts.visit(n, b)?;
            return Ok(source || facts);
        }
        self.source.visit_input(input, b)
    }
    pub fn consumed_inputs(_profile: Profile) -> Vec<ValidationInput> {
        Self::inputs()
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut rows = crate::domain::normalized::facts_inputs(vec![
            ValidationInput::of::<crate::domain::analysis::catalog_core::AnalysisCoverage>(&["id"]),
            ValidationInput::of::<crate::domain::analysis::catalog_core::Invocation>(&["id"]),
            ValidationInput::of::<crate::domain::analysis::catalog_evidence::AnalysisCoverage>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::analysis::catalog_evidence::Invocation>(&["id"]),
            ValidationInput::of::<crate::domain::assertion::AssertionQualification>(&["id"]),
            ValidationInput::of::<crate::domain::attribution::ProviderCoverage>(&["id"]),
            ValidationInput::of::<crate::domain::calls::Signature>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogAlias>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogCallable>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogCandidate>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogExposure>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogInvocation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogMember>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogMemberInvocation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogOption>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogOptionSubject>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::CatalogPath>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::CatalogDeployment>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ConstructorCandidateLink>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::catalog::evidence::DocumentAssociation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::FieldAccessAssessment>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::FieldLocationLink>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::OriginalSource>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ReleaseDeployment>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ScenarioAssociation>(&["id"]),
            ValidationInput::of::<crate::domain::catalog::evidence::ScenarioSpan>(&["id"]),
            ValidationInput::of::<crate::domain::class_metadata::ClassMetadataObservation>(&["id"]),
            ValidationInput::of::<crate::domain::class_metadata::ClassMetadataSupport>(&["id"]),
            ValidationInput::of::<crate::domain::deployment::DeploymentObservation>(&["id"]),
            ValidationInput::of::<crate::domain::documents::DocumentMentionObservation>(&["id"]),
            ValidationInput::of::<
                crate::domain::execution::summary_exceptions::SummaryExceptionOutcome,
            >(&["id"]),
            ValidationInput::of::<crate::domain::input::ArtifactOwnership>(&["id"]),
            ValidationInput::of::<crate::domain::input::DistributionVerification>(&["id"]),
            ValidationInput::of::<crate::domain::lexical::BindingObservation>(&["id"]),
            ValidationInput::of::<crate::domain::lexical::BindingSupport>(&["id"]),
            ValidationInput::of::<crate::domain::lexical::LexicalResolution>(&["id"]),
            ValidationInput::of::<crate::domain::lexical::ReferenceObservation>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::callables::EffectiveCallableAssessment>(
                &["id"],
            ),
            ValidationInput::of::<crate::domain::normalized::callables::SignatureReturnType>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::callables::SignatureSlot>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::callables::SignatureSlotEntity>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::callables::SignatureSlotType>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::callables::SignatureVariant>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::entities::FieldEntity>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::entities::ParameterEntity>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::entities::ParameterEntityLink>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::entities::PublicExposure>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::entities::SymbolEntityCandidate>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::links::MentionEntityAssessment>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::links::MentionEntityCandidate>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::links::ReferenceEntityAssessment>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::links::ReferenceEntityCandidate>(&[
                "id",
            ]),
            ValidationInput::of::<crate::domain::normalized::links::ReferenceEntityTarget>(&["id"]),
            ValidationInput::of::<crate::domain::source::Module>(&["id"]),
            ValidationInput::of::<crate::domain::source::Occurrence>(&["id"]),
            ValidationInput::of::<crate::domain::source::SourceArtifact>(&["id"]),
            ValidationInput::of::<crate::domain::syntax::DeclarationDecorator>(&["id"]),
            ValidationInput::of::<crate::domain::syntax::DeclarationDecoratorSupport>(&["id"]),
            ValidationInput::of::<crate::domain::syntax::DeclarationObservation>(&["id"]),
            ValidationInput::of::<crate::domain::syntax::DeclarationSupport>(&["id"]),
            ValidationInput::of::<crate::domain::syntax::SyntaxPlacement>(&["id"]),
            ValidationInput::of::<crate::domain::types::GenericSpecializationObservation>(&["id"]),
            ValidationInput::of::<crate::domain::types::NativeSignatureObservation>(&["id"]),
            ValidationInput::of::<crate::domain::types::NativeSignatureSupport>(&["id"]),
            ValidationInput::of::<crate::domain::types::SignatureTypeObservation>(&["id"]),
            ValidationInput::of::<crate::domain::types::TypeObservation>(&["id"]),
            ValidationInput::of::<crate::domain::types::TypeSupport>(&["id"]),
            ValidationInput::of::<crate::domain::source::CoverageScope>(&["id"]),
            ValidationInput::of::<crate::domain::normalized::entities::EntityRef>(&["id"]),
        ]);
        rows.extend(super::frames::Frames::inputs());
        rows.sort_by_key(|input| (input.name(), input.prefix()));
        rows.dedup_by_key(|input| (input.name(), input.prefix()));
        rows
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
    let mut out = domains(d, frames.iter(), b)?;
    extend_witnesses(d, &mut out, b)?;
    Ok(out)
}
/// The complete declaration domain of one actual C0 member/context grain.
pub fn member(
    d: &Data,
    member: Id<catalog::CatalogMember>,
    context: Id<attribution::AnalysisContext>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let owner = need(&d.source.catalog.members, member)?;
    if !d.source.facts.core_links.iter().any(|link| {
        link.member == member
            && d.source
                .facts
                .core_invocations
                .get(link.invocation)
                .is_some_and(|invocation| {
                    invocation.context == context && invocation.input == owner.input
                })
    }) {
        return Err(invalid("selection member/context has no admitted C0 root"));
    }
    domains(d, [(member, context)].iter(), b)
}
fn domains<'a>(
    d: &Data,
    frames: impl Iterator<Item = &'a (Id<catalog::CatalogMember>, Id<attribution::AnalysisContext>)>,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    let mut charge = charged::StateCharge::new(b, "selection-declaration-domains");
    for (member_id, context) in frames {
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
                        if exposure.member != *member_id {
                            continue;
                        }
                        let entity = if let Some(path) = candidate.path {
                            Some(need(&d.source.catalog.paths, path)?.entity)
                        } else if let Some(alias) = candidate.alias {
                            Some(need(&d.source.catalog.aliases, alias)?.entity)
                        } else {
                            candidate
                                .entity
                                .map(|entity| {
                                    need(&d.source.core.entity_candidates, entity)
                                        .map(|row| row.entity)
                                })
                                .transpose()?
                        };
                        for result in d.facts.exception_outcomes.iter().filter(|row| {
                            row.context == *context
                                && row.input == member.input
                                && Some(row.owner) == entity
                        }) {
                            add(
                                &mut out,
                                &mut members,
                                &mut charge,
                                Context::Binding {
                                    member: *member_id,
                                    candidate: candidate.id(),
                                    analysis: *context,
                                },
                                Witness::SummaryException {
                                    outcome: result.id(),
                                },
                            )?;
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
                        for typed in d
                            .source
                            .core
                            .return_types
                            .iter()
                            .filter(|t| t.variant == variant.id())
                        {
                            add(
                                &mut out,
                                &mut members,
                                &mut charge,
                                context_row.clone(),
                                Witness::SignatureTypeObservation {
                                    observation: typed.observation,
                                },
                            )?;
                        }
                        for typed in d.source.core.slot_types.iter().filter(|t| {
                            d.source
                                .core
                                .slots
                                .get(t.slot)
                                .is_some_and(|s| s.variant == variant.id())
                        }) {
                            add(
                                &mut out,
                                &mut members,
                                &mut charge,
                                context_row.clone(),
                                Witness::SignatureTypeObservation {
                                    observation: typed.observation,
                                },
                            )?;
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
                        let release = *d
                            .verification_releases
                            .get(&ownership.distribution)
                            .ok_or_else(|| {
                                invalid("selection distribution release premise absent")
                            })?;
                        for r in d
                            .evidence
                            .release_deployments
                            .iter()
                            .filter(|r| r.release == release)
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
                                    release,
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
    Ok(out)
}
/// Located witnesses also have roots without public members. Scoped consumers call this
/// kernel on each actual native observation/support closure; no member filter defines it.
pub fn witnesses(d: &Data, b: &ResourceBudget) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    extend_witnesses(d, &mut out, b)?;
    Ok(out)
}
fn extend_witnesses(d: &Data, out: &mut Output, b: &ResourceBudget) -> Result<(), ModelError> {
    // Request-time answers only reference persisted canonical witness rows. These rows
    // characterize available facts; context admission and truth remain evaluator decisions.
    // Later stages append vocabulary rows. Only qualifications referenced by these
    // immutable native inputs belong to C2's located-typing witness universe.
    for row in d.source.core.native_signatures.iter() {
        need(&d.source.core.qualifications, row.qualification)?;
        out.witnesses.insert(Witness::Qualification {
            qualification: row.qualification,
        })?;
    }
    for row in d.facts.generic_specializations.iter() {
        need(&d.source.core.qualifications, row.qualification)?;
        out.witnesses.insert(Witness::Qualification {
            qualification: row.qualification,
        })?;
    }
    for row in d.source.core.signature_types.iter() {
        out.witnesses.insert(Witness::SignatureTypeObservation {
            observation: row.id(),
        })?;
    }
    for row in d.facts.type_observations.iter() {
        out.witnesses.insert(Witness::TypeObservation {
            observation: row.id(),
        })?;
    }
    for row in d
        .source
        .core
        .native_coverage
        .iter()
        .filter(|r| r.family == attribution::FactFamily::Types)
    {
        out.witnesses
            .insert(Witness::NativeTypingCoverage { coverage: row.id() })?;
    }
    for row in d.facts.generic_specializations.iter() {
        out.witnesses.insert(Witness::GenericSpecialization {
            observation: row.id(),
        })?;
    }
    for support in d.facts.binding_supports.iter() {
        need(&d.source.core.bindings, support.assertion)?;
        out.witnesses.insert(Witness::LexicalDefinition {
            observation: support.assertion,
            support: support.id(),
        })?;
    }
    for support in d.facts.declaration_supports.iter() {
        need(&d.source.core.declarations, support.assertion)?;
        out.witnesses.insert(Witness::SourceCharacterization {
            observation: support.assertion,
            support: support.id(),
        })?;
    }
    for support in d.facts.metadata_supports.iter() {
        need(&d.source.core.class_metadata, support.assertion)?;
        out.witnesses.insert(Witness::ClassMetadata {
            observation: support.assertion,
            support: support.id(),
        })?;
    }
    for support in d.facts.native_signature_supports.iter() {
        need(&d.source.core.native_signatures, support.assertion)?;
        out.witnesses.insert(Witness::NativeCallableMetadata {
            observation: support.assertion,
            support: support.id(),
        })?;
    }
    for support in d.facts.type_supports.iter() {
        let row = need(&d.facts.type_observations, support.assertion)?;
        if row.role == types::TypeRole::Raised {
            out.witnesses.insert(Witness::RaisedType {
                observation: row.id(),
                support: support.id(),
            })?;
        }
    }
    let inputs = normalized::decorator_identity::Inputs {
        qualifications: &d.source.core.qualifications,
        occurrences: &d.source.core.occurrences,
        placements: &d.source.core.placements,
        references: &d.source.core.references,
        assessments: &d.source.core.reference_assessments,
        candidates: &d.source.core.reference_candidates,
        targets: &d.source.core.reference_targets,
        resolutions: &d.source.core.lexical_resolutions,
    };
    for support in d.facts.decorator_supports.iter() {
        let row = need(&d.facts.decorators, support.assertion)?;
        let q = need(&d.source.core.qualifications, row.qualification)?;
        let selected = inputs.select(row, q.context, b)?;
        for selection in selected.selections.iter() {
            if let (Some(assessment), Some(candidate)) = (selection.assessment, selection.candidate)
            {
                out.witnesses.insert(Witness::ResolvedDecorator {
                    observation: row.id(),
                    support: support.id(),
                    assessment,
                    candidate,
                })?;
            }
        }
    }
    Ok(())
}

/// Actual observation/support roots whose located witnesses exist without a catalog member.
pub fn witness_roots() -> Vec<std::any::TypeId> {
    use std::any::TypeId;
    vec![
        TypeId::of::<types::NativeSignatureObservation>(),
        TypeId::of::<types::GenericSpecializationObservation>(),
        TypeId::of::<types::SignatureTypeObservation>(),
        TypeId::of::<types::TypeObservation>(),
        TypeId::of::<attribution::ProviderCoverage>(),
        TypeId::of::<lexical::BindingSupport>(),
        TypeId::of::<syntax::DeclarationSupport>(),
        TypeId::of::<class_metadata::ClassMetadataSupport>(),
        TypeId::of::<types::NativeSignatureSupport>(),
        TypeId::of::<types::TypeSupport>(),
        TypeId::of::<syntax::DeclarationDecoratorSupport>(),
    ]
}
/// Complete inverse memberships read by the declaration-domain and located-witness kernels.
/// Forward nominal premises remain ordinary references: a referenced member is not a new root.
pub fn memberships() -> Vec<(std::any::TypeId, &'static str)> {
    use catalog::{evidence::*, *};
    use normalized::{callables::*, links::*};
    use std::any::TypeId;
    vec![
        (TypeId::of::<CatalogExposure>(), "member"),
        (TypeId::of::<CatalogCandidate>(), "exposure"),
        (TypeId::of::<CatalogCallable>(), "member"),
        (TypeId::of::<CatalogInvocation>(), "callable"),
        (TypeId::of::<CatalogOption>(), "member"),
        (TypeId::of::<CatalogMemberInvocation>(), "member"),
        (TypeId::of::<ScenarioAssociation>(), "member"),
        (TypeId::of::<ConstructorCandidateLink>(), "association"),
        (TypeId::of::<DocumentAssociation>(), "member"),
        (TypeId::of::<FieldAccessAssessment>(), "option"),
        (TypeId::of::<FieldLocationLink>(), "assessment"),
        (TypeId::of::<ScenarioSpan>(), "scenario"),
        (TypeId::of::<SignatureSlot>(), "variant"),
        (TypeId::of::<SignatureSlotEntity>(), "slot"),
        (TypeId::of::<SignatureSlotType>(), "slot"),
        (TypeId::of::<SignatureReturnType>(), "variant"),
        (TypeId::of::<types::TypeObservation>(), "subject"),
        (TypeId::of::<input::ArtifactOwnership>(), "artifact"),
        (TypeId::of::<ReleaseDeployment>(), "release"),
        (TypeId::of::<OriginalSource>(), "artifact_artifact"),
        (TypeId::of::<source::CoverageScope>(), "artifact_artifact"),
        (TypeId::of::<attribution::ProviderCoverage>(), "scope"),
        (
            TypeId::of::<execution::summary_exceptions::SummaryExceptionOutcome>(),
            "owner",
        ),
        (TypeId::of::<lexical::ReferenceObservation>(), "read"),
        (TypeId::of::<ReferenceEntityAssessment>(), "reference"),
        (TypeId::of::<ReferenceEntityCandidate>(), "assessment"),
    ]
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = Data::inputs();
    inputs.extend(Output::inputs());
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
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
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if is_vocabulary(input.name()) {
            if !self.data.visit_input(input, b)? {
                return Err(invalid("undeclared completed rendering/source view"));
            }
            Ok(())
        } else {
            self.visit(input.name(), b)
        }
    }
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
    inputs.extend(parent.outputs.into_iter().map(|r| r.completed_input()));
    inputs.extend(Facts::uses());
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    outputs.extend(
        analysis::selection::publication_relations()
            .iter()
            .map(stages::RelationUse::of_relation),
    );
    let roots = dependency_closure::DependencyClosure::roots_from_uses(model, &inputs)?;
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
            SEMANTIC_RULE_REVISION.encode(&mut k);
            k.finish()
        },
    };
    (parameters, definition)
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["catalog_selection_declaration_closure"]
}

#[cfg(test)]
mod compact_ownership_controls {
    use super::*;
    fn id<R>(byte: u8) -> Id<R> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([byte; 16].into_iter()))
        .unwrap()
    }
    #[test]
    fn verification_projection_releases_unused_distribution_inventory() {
        let budget = ResourceBudget::fixed(64 << 10).unwrap();
        let mut data = Data::new(&budget);
        let row = input::DistributionVerification {
            acquisition: id(1),
            release: id(2),
            record_digest: ContentHash::of(b"record"),
            artifact_sha256: (0..100_000).map(|index| format!("{index:064x}")).collect(),
        };
        row.validate().unwrap();
        let batch = input::DistributionVerification::encode(std::slice::from_ref(&row)).unwrap();
        data.visit(input::DistributionVerification::NAME, &batch)
            .unwrap();
        assert_eq!(
            data.verification_releases.get(&row.id()),
            Some(&row.release)
        );
        assert!(data.source.facts.verifications.is_empty());
        drop(data);
        assert_eq!(budget.reserved(), 0);
    }
}
