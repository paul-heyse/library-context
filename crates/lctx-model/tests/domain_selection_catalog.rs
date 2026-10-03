//! Hand-expected declaration controls; actual native/store qualification is separate.
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    catalog::evidence as c1,
    catalog::*,
    input::*,
    normalized::{Rows, callables::*, entities::*},
    resources::ResourceBudget,
    selection::evaluate::Prepared,
    selection::{
        build::{Data, Output},
        *,
    },
    source::*,
    *,
};
fn id<T>(n: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([n; 16].into_iter()))
    .unwrap()
}
fn fixture() -> (
    ResourceBudget,
    Data,
    Id<CatalogMember>,
    Id<AnalysisContext>,
    SourceArtifact,
) {
    let b = ResourceBudget::fixed(64 << 20).unwrap();
    let mut d = Data::new(&b);
    let source =
        SourceArtifact::from_bytes(id(1), "api.py".into(), b"def run(flag=False): pass").unwrap();
    d.source.core.artifacts.insert(source.clone()).unwrap();
    let module = d
        .source
        .core
        .modules
        .insert(Module {
            source: source.id(),
            qualified_name: "pkg.api".into(),
        })
        .unwrap();
    let member = d
        .source
        .catalog
        .members
        .insert(CatalogMember {
            input: source.input,
            access: module,
            path: vec!["run".into()],
            name: "run".into(),
        })
        .unwrap();
    let context = id(2);
    let q = d
        .source
        .core
        .qualifications
        .insert(AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context,
            scope: CoverageScope::Artifact {
                artifact: source.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        })
        .unwrap();
    let core = d
        .source
        .facts
        .core_invocations
        .insert(
            analysis::catalog_core::Invocation::new(
                source.input,
                context,
                catalog::build::definition().1.id(),
                None,
                [],
            )
            .0,
        )
        .unwrap();
    d.source
        .facts
        .core_links
        .insert(CatalogMemberInvocation {
            member,
            invocation: core,
        })
        .unwrap();
    let parent = analysis::catalog_evidence::InvocationSource::CatalogCore { invocation: core };
    d.facts.evidence_sources.insert(parent.clone()).unwrap();
    let (earlier, links) = analysis::catalog_evidence::Invocation::new(
        source.input,
        context,
        c1::build::definition().1.id(),
        None,
        [parent.id()],
    );
    d.facts.evidence_invocations.insert(earlier).unwrap();
    for link in links {
        d.facts.evidence_inputs.insert(link).unwrap();
    }
    d.source
        .facts
        .runs
        .insert(ProviderRun {
            provider: id(3),
            context,
            input: source.input,
            configuration: ContentHash::of(b"c"),
            requested_families: ContentHash::of(b"r"),
        })
        .unwrap();
    d.source
        .core
        .native_coverage
        .insert(ProviderCoverage {
            scope: CoverageScope::Artifact {
                artifact: source.id(),
            }
            .id(),
            provider: Some(id(3)),
            context,
            family: FactFamily::Signatures,
            run: Some(d.source.facts.runs.iter().next().unwrap().id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        })
        .unwrap();
    let exposure = d
        .source
        .core
        .exposures
        .insert(PublicExposure {
            access: module,
            context,
            observation: id(4),
            origin: id(5),
            enumeration: None,
            publicity: lctx_model::domain::normalized::entities::PublicPathKnowledge::Known,
            status: ResolutionStatus::Resolved,
            reason: EntityReason::DeclarationAgreement,
        })
        .unwrap();
    let exposure = d
        .source
        .catalog
        .exposures
        .insert(CatalogExposure { member, exposure })
        .unwrap();
    let entity = d
        .source
        .core
        .refs
        .insert(EntityRef::Callable { callable: id(6) })
        .unwrap();
    let lower = d
        .source
        .core
        .entity_candidates
        .insert(SymbolEntityCandidate {
            resolution: id(7),
            entity,
        })
        .unwrap();
    let candidate = d
        .source
        .catalog
        .candidates
        .insert(CatalogCandidate {
            exposure,
            candidate: None,
            entity: Some(lower),
            path: None,
            alias: None,
        })
        .unwrap();
    let assessment = d
        .source
        .core
        .assessments
        .insert(EffectiveCallableAssessment {
            callable: id(6),
            context,
            decorators: ContentHash::of(b"d"),
            policy: ContentHash::of(b"p"),
            identity: Knowledge::Known,
            identity_reason: CallableReason::EvidenceAgreement,
            signatures: Knowledge::Known,
            signature_reason: CallableReason::EvidenceAgreement,
            descriptor: Knowledge::Known,
            descriptor_kind: Some(DescriptorKind::Function),
            descriptor_reason: CallableReason::EvidenceAgreement,
            body: Knowledge::Unknown,
            body_admitted: false,
            body_reason: CallableReason::MissingBodyEvidence,
            asynchronous: Some(false),
            generator: Some(false),
        })
        .unwrap();
    let callable = d
        .source
        .catalog
        .callables
        .insert(CatalogCallable {
            member,
            candidate,
            assessment,
            basis: CatalogContractBasis::PublicCandidate,
        })
        .unwrap();
    let sig = d
        .facts
        .signatures
        .insert(Signature {
            role: lctx_model::domain::calls::SignatureRole::Source, native: None,
            qualification: q,
            scope: CoverageScope::Artifact {
                artifact: source.id(),
            }
            .id(),
            symbol: id(8),
            variant: 0,
            form: SignatureForm::List,
            parameters: ContentHash::of(b"params"),
        })
        .unwrap();
    let variant = d
        .source
        .core
        .variants
        .insert(SignatureVariant { role: lctx_model::domain::calls::SignatureRole::Source, native: None,
            signature: sig,
            context,
            resolution: id(7),
            callable: Some(id(6)),
            assessment: Some(assessment),
            adjustment: SignatureAdjustment::None,
        })
        .unwrap();
    d.source
        .catalog
        .invocations
        .insert(CatalogInvocation { callable, variant })
        .unwrap();
    let shape = d
        .facts
        .shapes
        .insert(ParameterShape {
            name: Some("flag".into()),
            kind: ParameterKind::PositionalOrKeyword,
            required: false,
        })
        .unwrap();
    let raw = d
        .facts
        .signature_parameters
        .insert(SignatureParameter {
            signature: sig,
            ordinal: 0,
            shape,
        })
        .unwrap();
    let slot = d
        .source
        .core
        .slots
        .insert(SignatureSlot {
            parameter: raw,
            variant,
            ordinal: 0,
            default: DefaultSlot::DefinitionTime,
        })
        .unwrap();
    let subject = d
        .source
        .catalog
        .subjects
        .insert(CatalogOptionSubject::Parameter { slot })
        .unwrap();
    let literal = d
        .facts
        .literals
        .insert(value::Literal::Bool { value: false })
        .unwrap();
    let default = d
        .source
        .catalog
        .defaults
        .insert(CatalogDefault::Literal { literal })
        .unwrap();
    let evidence = d
        .source
        .catalog
        .evidence
        .insert(CatalogOptionEvidence::NativeParameter { slot })
        .unwrap();
    d.source
        .catalog
        .options
        .insert(CatalogOption {
            member,
            subject,
            evidence,
            default,
        })
        .unwrap();
    d.evidence = c1::build::build(&d.source, &b).unwrap();
    (b, d, member, context, source)
}
fn classify(
    d: &Data,
    out: &Output,
    member: Id<CatalogMember>,
    context: Id<AnalysisContext>,
    predicate: Predicate,
    quantifier: Quantifier,
    b: &ResourceBudget,
) -> Outcome {
    Prepared::new(d, out, b)
        .unwrap()
        .classify(
            member,
            context,
            &Requirement {
                predicate,
                quantifier,
            },
            b,
        )
        .unwrap()
        .outcome
}
#[test]
fn ordinary_defaults_are_typed_and_missing_type_or_wrapper_default_is_unresolved() {
    let (b, mut d, m, c, _) = fixture();
    let zero = d
        .facts
        .literals
        .insert(value::Literal::Integer {
            decimal: "0".into(),
        })
        .unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    let test = |p| classify(&d, &output, m, c, p, Quantifier::AnyApplicable, &b);
    assert_eq!(
        test(Predicate::DeclaresParameter {
            name: "flag".into()
        }),
        Outcome::Supported
    );
    assert_eq!(
        test(Predicate::DeclaresParameter {
            name: "absent".into()
        }),
        Outcome::Contradicted
    );
    assert_eq!(
        test(Predicate::ParameterDefault {
            name: "flag".into(),
            value: value::Literal::Bool { value: false }.id()
        }),
        Outcome::Supported
    );
    assert_eq!(
        test(Predicate::ParameterDefault {
            name: "flag".into(),
            value: zero
        }),
        Outcome::Contradicted
    );
    assert_eq!(
        test(Predicate::ParameterDefault {
            name: "flag".into(),
            value: id(90)
        }),
        Outcome::Unresolved
    );
    assert_eq!(
        test(Predicate::ParameterType {
            name: "flag".into(),
            r#type: StructuralType::Category { kind: 0 }
        }),
        Outcome::Unresolved
    );
    // Original-source None is a distinct option; it cannot establish the effective wrapper slot.
    let mut effective = d.source.catalog.options.iter().next().unwrap().clone();
    d.source.catalog.options = Rows::new(&b);
    effective.default = d
        .source
        .catalog
        .defaults
        .insert(CatalogDefault::Unknown {})
        .unwrap();
    d.source.catalog.options.insert(effective).unwrap();
    let literal = d.facts.literals.insert(value::Literal::None).unwrap();
    let default = d
        .source
        .catalog
        .defaults
        .insert(CatalogDefault::Literal { literal })
        .unwrap();
    let subject = d
        .source
        .catalog
        .subjects
        .insert(CatalogOptionSubject::SourceParameter { parameter: id(91) })
        .unwrap();
    d.source
        .catalog
        .options
        .insert(CatalogOption {
            member: m,
            subject,
            evidence: id(92),
            default,
        })
        .unwrap();
    d.evidence = c1::build::build(&d.source, &b).unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::ParameterDefault {
                name: "flag".into(),
                value: literal
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Unresolved
    );
}
#[test]
fn original_sources_are_member_owned_and_shared_replay_refuses_coupled_domain_erasure() {
    let (b, mut d, m, c, source) = fixture();
    let output = selection::build::build(&d, &b).unwrap();
    assert_eq!(output.domains.len(), 7);
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::SourceAlignment { exact: true },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Supported
    );
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::SourceAlignment { exact: false },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Unresolved
    );
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::PublicPath {
                path: vec!["foreign".into()]
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Contradicted
    );
    d.evidence
        .original_sources
        .insert(c1::OriginalSource::Artifact { artifact: id(93) })
        .unwrap();
    let out = selection::build::build(&d, &b).unwrap();
    assert_eq!(
        out.contexts
            .iter()
            .filter(|r| matches!(r, Context::Source { .. }))
            .count(),
        1
    );
    assert!(out.contexts.iter().any(|r|matches!(r,Context::Source {source:s,..} if *s==c1::OriginalSource::Artifact {artifact:source.id()}.id())));
    let mut omitted = selection::build::build(&d, &b).unwrap();
    omitted.domains = Rows::new(&b);
    omitted.members = Rows::new(&b);
    omitted.closure = Rows::new(&b);
    omitted.evidence = Rows::new(&b);
    assert!(Prepared::new(&d, &omitted, &b).is_err());
    let mut check = (selection::build::invariants().remove(0).create)(&b);
    macro_rules! core {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.source.core.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_inputs!(core);
    macro_rules! catalog {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.source.catalog.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_outputs!(catalog);
    macro_rules! facts {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.source.facts.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_evidence_inputs!(facts);
    macro_rules! evidence {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.evidence.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_evidence_outputs!(evidence);
    macro_rules! extra {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.facts.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_selection_inputs!(extra);
    macro_rules! output {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&omitted.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::catalog_selection_outputs!(output);
    assert!(check.finish().is_err());
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        selection::build::build(&d, &tiny),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn invocation_universe_cannot_be_deleted_when_no_selection_domains_exist() {
    let (b, d, _, _, _) = fixture();
    let empty = Output::new(&b);
    let invocations = Rows::new(&b);
    let sources = Rows::new(&b);
    let inputs = Rows::new(&b);
    let links = Rows::new(&b);
    assert!(
        selection::frames::verify(&d, &empty, &invocations, &sources, &inputs, &links, &b).is_err()
    );
}
#[test]
fn discovery_groups_remain_separate_and_strict_eligibility_requires_support() {
    let (b, d, m, c, _) = fixture();
    let output = selection::build::build(&d, &b).unwrap();
    let prepared = Prepared::new(&d, &output, &b).unwrap();
    let selection = Selection {
        requirements: vec![],
        mode: Mode::Discovery,
        joint: JointPolicy::IndependentRecords,
    };
    let all = prepared.select(&selection, &b).unwrap();
    assert_eq!(all.candidates.len(), 1);
    assert_eq!(all.candidates[0].member, m);
    assert_eq!(all.candidates[0].analysis, c);
    assert_eq!(all.candidates[0].outcome, Outcome::Supported);
    let selection = Selection {
        requirements: vec![Requirement {
            predicate: Predicate::ParameterType {
                name: "flag".into(),
                r#type: StructuralType::Category { kind: 0 },
            },
            quantifier: Quantifier::AnyApplicable,
        }],
        ..selection
    };
    let open = prepared.select(&selection, &b).unwrap();
    assert_eq!(open.group(Outcome::Unresolved).count(), 1);
    assert_eq!(open.eligible().count(), 1);
    let strict = prepared
        .select(
            &Selection {
                mode: Mode::Strict,
                ..selection
            },
            &b,
        )
        .unwrap();
    assert_eq!(strict.group(Outcome::Unresolved).count(), 1);
    assert_eq!(strict.eligible().count(), 0);
}
#[test]
fn release_declarations_require_exact_ownership_and_passed_interpretation() {
    let (b, mut d, m, c, source) = fixture();
    let package = d
        .facts
        .packages
        .insert(Package { name: "pkg".into() })
        .unwrap();
    let release = d
        .facts
        .releases
        .insert(Release {
            package,
            version: "1".into(),
        })
        .unwrap();
    let verification = d
        .source
        .facts
        .verifications
        .insert(DistributionVerification {
            acquisition: id(100),
            release,
            record_digest: ContentHash::of(b"record"),
            artifact_sha256: vec![],
        })
        .unwrap();
    d.source
        .facts
        .artifact_ownership
        .insert(ArtifactOwnership {
            artifact: source.id(),
            distribution: verification,
        })
        .unwrap();
    let metadata = SourceArtifact::from_bytes(
        source.input,
        "pkg.dist-info/METADATA".into(),
        b"Requires-Dist: dependency>=1",
    )
    .unwrap();
    d.source.core.artifacts.insert(metadata.clone()).unwrap();
    d.source
        .facts
        .uses
        .insert(ArtifactUse {
            artifact: metadata.id(),
            input: source.input,
            role: SourceRole::DistributionMetadata,
        })
        .unwrap();
    d.source
        .facts
        .artifact_ownership
        .insert(ArtifactOwnership {
            artifact: metadata.id(),
            distribution: verification,
        })
        .unwrap();
    let span = Evidence::SourceSpan {
        source: metadata.id(),
        start: 0,
        end: metadata.byte_len,
    };
    let span_id = EvidenceSourceSpanId::of(&span).unwrap();
    d.source.facts.canonical_evidence.insert(span).unwrap();
    let q = d
        .source
        .core
        .qualifications
        .insert(AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: c,
            scope: CoverageScope::Input {
                input: source.input,
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        })
        .unwrap();
    let observation = deployment::DeploymentObservation {
        qualification: q,
        span: span_id,
        ordinal: 0,
        distribution: Some("pkg".into()),
        version: Some("1".into()),
        field: "requires-dist".into(),
        original: "dependency>=1".into(),
        name: Some("dependency".into()),
        extras: vec![],
        marker: None,
        constraint: Some(">=1".into()),
        interpretation: deployment::CheckStatus::Passed,
        diagnostic: None,
        environment_digest: None,
        lock_digest: None,
        referenced_path: None,
    };
    d.source
        .facts
        .deployment
        .insert(observation.clone())
        .unwrap();
    d.evidence = c1::build::build(&d.source, &b).unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    let test = |p| classify(&d, &output, m, c, p, Quantifier::AnyApplicable, &b);
    assert_eq!(
        test(Predicate::DeploymentDeclaration {
            field: DeploymentField::RequiresDist,
            name: "dependency".into()
        }),
        Outcome::Supported
    );
    assert_eq!(
        test(Predicate::ReleaseVersion {
            distribution: "pkg".into(),
            version: "1".into()
        }),
        Outcome::Supported
    );
    assert_eq!(
        test(Predicate::ReleaseVersion {
            distribution: "foreign".into(),
            version: "1".into()
        }),
        Outcome::Unresolved
    );
    d.source.facts.deployment = Rows::new(&b);
    d.source
        .facts
        .deployment
        .insert(deployment::DeploymentObservation {
            interpretation: deployment::CheckStatus::Failed,
            ..observation
        })
        .unwrap();
    d.evidence = c1::build::build(&d.source, &b).unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::DeploymentDeclaration {
                field: DeploymentField::RequiresDist,
                name: "dependency".into()
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Unresolved
    );
    // A release-matching metadata artifact still needs exact ownership of this API source.
    d.source.facts.artifact_ownership = Rows::new(&b);
    d.source
        .facts
        .artifact_ownership
        .insert(ArtifactOwnership {
            artifact: metadata.id(),
            distribution: verification,
        })
        .unwrap();
    d.evidence = c1::build::build(&d.source, &b).unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::ReleaseVersion {
                distribution: "pkg".into(),
                version: "1".into()
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Unresolved
    );
}
#[test]
fn comparable_type_observations_keep_conflict_and_foreign_nominal_terms_stay_unknown() {
    let (b, mut d, m, c, source) = fixture();
    let slot = d.source.core.slots.iter().next().unwrap().clone();
    let declaration = d
        .source
        .core
        .occurrences
        .insert(Occurrence {
            source: source.id(),
            start: 8,
            end: 12,
            syntax_kind: SyntaxKind::Parameter,
            role: OccurrenceRole::Syntax,
            structural_path: vec![8],
        })
        .unwrap();
    let entity = d
        .source
        .core
        .parameters
        .insert(ParameterEntity::Source { declaration })
        .unwrap();
    let link = d
        .source
        .core
        .parameter_links
        .insert(ParameterEntityLink {
            parameter: slot.parameter,
            entity,
            declaration: None,
        })
        .unwrap();
    d.source
        .core
        .slot_entities
        .insert(SignatureSlotEntity {
            slot: slot.id(),
            link,
        })
        .unwrap();
    let q = d.source.core.qualifications.iter().next().unwrap().id();
    let term = d.facts.type_terms.insert(types::TypeTerm::None).unwrap();
    d.facts
        .type_observations
        .insert(types::TypeObservation {
            qualification: q,
            subject: declaration,
            role: types::TypeRole::Parameter,
            declared: true,
            term,
        })
        .unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::ParameterType {
                name: "flag".into(),
                r#type: StructuralType::CanonicalTerm { term: id(199) }
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Unresolved
    );
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::ParameterType {
                name: "flag".into(),
                r#type: StructuralType::CanonicalTerm { term }
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Supported
    );
    let other = d
        .facts
        .type_terms
        .insert(types::TypeTerm::LiteralString)
        .unwrap();
    d.facts
        .type_observations
        .insert(types::TypeObservation {
            qualification: q,
            subject: declaration,
            role: types::TypeRole::Parameter,
            declared: true,
            term: other,
        })
        .unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    let result = Prepared::new(&d, &output, &b)
        .unwrap()
        .classify(
            m,
            c,
            &Requirement {
                predicate: Predicate::ParameterType {
                    name: "flag".into(),
                    r#type: StructuralType::CanonicalTerm { term },
                },
                quantifier: Quantifier::AnyApplicable,
            },
            &b,
        )
        .unwrap();
    assert_eq!(result.outcome, Outcome::Conflicting);
    assert!(result.witnesses.iter().any(|w|matches!(w,selection::algebra::RequirementWitness::Conflict {positive,negative,..} if positive.iter().any(|w|matches!(w,Witness::TypeObservation {..})) && negative.iter().any(|w|matches!(w,Witness::TypeObservation {..})))));
}
#[test]
fn configuration_literal_domain_is_independent_of_stored_default() {
    let (b, mut d, _, c, source) = fixture();
    let module = d.source.core.modules.iter().next().unwrap().id();
    let member = d
        .source
        .catalog
        .members
        .insert(CatalogMember {
            input: source.input,
            access: module,
            path: vec!["Options".into()],
            name: "Options".into(),
        })
        .unwrap();
    let core = d.source.facts.core_invocations.iter().next().unwrap().id();
    d.source
        .facts
        .core_links
        .insert(CatalogMemberInvocation {
            member,
            invocation: core,
        })
        .unwrap();
    let class = d
        .source
        .core
        .source_classes
        .insert(ClassEntity::Source {
            declaration: id(130),
        })
        .unwrap();
    let field = d
        .source
        .core
        .fields
        .insert(FieldEntity {
            class,
            name: "flag".into(),
        })
        .unwrap();
    let q = d.source.core.qualifications.iter().next().unwrap().id();
    let expected = d
        .facts
        .literals
        .insert(value::Literal::Bool { value: true })
        .unwrap();
    let default = d
        .facts
        .literals
        .insert(value::Literal::Bool { value: false })
        .unwrap();
    let term = d
        .facts
        .type_terms
        .insert(types::TypeTerm::Literal { value: expected })
        .unwrap();
    let raw = d
        .source
        .core
        .field_observations
        .insert(types::RecordFieldObservation {
            qualification: q,
            class: id(131),
            name: "flag".into(),
            record: types::RecordKind::TypedDict,
            ordinal: 0,
            term,
            declared: true,
            declaration: None,
            default_term: None,
            has_default: None,
            init: None,
            alias: None,
            kw_only: None,
            required: Some(true),
            read_only: Some(false),
        })
        .unwrap();
    let link = d
        .source
        .core
        .field_links
        .insert(FieldEntityLink {
            field,
            observation: raw,
        })
        .unwrap();
    let subject = d
        .source
        .catalog
        .subjects
        .insert(CatalogOptionSubject::Field { field })
        .unwrap();
    let evidence = d
        .source
        .catalog
        .evidence
        .insert(CatalogOptionEvidence::NativeField {
            link,
            observation: raw,
        })
        .unwrap();
    let default_id = d
        .source
        .catalog
        .defaults
        .insert(CatalogDefault::Literal { literal: default })
        .unwrap();
    d.source
        .catalog
        .options
        .insert(CatalogOption {
            member,
            subject,
            evidence,
            default: default_id,
        })
        .unwrap();
    d.evidence = c1::build::build(&d.source, &b).unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    assert_eq!(
        classify(
            &d,
            &output,
            member,
            c,
            Predicate::ConfigurationDefault {
                name: "flag".into(),
                value: default
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Supported
    );
    assert_eq!(
        classify(
            &d,
            &output,
            member,
            c,
            Predicate::ConfigurationLiteral {
                name: "flag".into(),
                value: expected
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Supported
    );
    assert_ne!(
        classify(
            &d,
            &output,
            member,
            c,
            Predicate::ConfigurationLiteral {
                name: "flag".into(),
                value: default
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Supported
    );
}
#[test]
fn conditional_native_type_evidence_does_not_establish_an_unconditional_requirement() {
    let (b, mut d, m, c, source) = fixture();
    let slot = d.source.core.slots.iter().next().unwrap().clone();
    let declaration = d
        .source
        .core
        .occurrences
        .insert(Occurrence {
            source: source.id(),
            start: 8,
            end: 12,
            syntax_kind: SyntaxKind::Parameter,
            role: OccurrenceRole::Syntax,
            structural_path: vec![8],
        })
        .unwrap();
    let entity = d
        .source
        .core
        .parameters
        .insert(ParameterEntity::Source { declaration })
        .unwrap();
    let link = d
        .source
        .core
        .parameter_links
        .insert(ParameterEntityLink {
            parameter: slot.parameter,
            entity,
            declaration: None,
        })
        .unwrap();
    d.source
        .core
        .slot_entities
        .insert(SignatureSlotEntity {
            slot: slot.id(),
            link,
        })
        .unwrap();
    let original = d.source.core.qualifications.iter().next().unwrap().clone();
    let q = d
        .source
        .core
        .qualifications
        .insert(AssertionQualification {
            condition: conditions::Diagram::never().id(),
            ..original
        })
        .unwrap();
    let term = d.facts.type_terms.insert(types::TypeTerm::None).unwrap();
    d.facts
        .type_observations
        .insert(types::TypeObservation {
            qualification: q,
            subject: declaration,
            role: types::TypeRole::Parameter,
            declared: true,
            term,
        })
        .unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    assert_eq!(
        classify(
            &d,
            &output,
            m,
            c,
            Predicate::ParameterType {
                name: "flag".into(),
                r#type: StructuralType::CanonicalTerm { term }
            },
            Quantifier::AnyApplicable,
            &b
        ),
        Outcome::Unresolved
    );
}
#[test]
fn public_path_is_full_and_universal_candidate_kind_ignores_the_name_only_context() {
    let (b, d, m, c, _) = fixture();
    let output = selection::build::build(&d, &b).unwrap();
    for q in [Quantifier::AnyApplicable, Quantifier::AllApplicable] {
        assert_eq!(
            classify(
                &d,
                &output,
                m,
                c,
                Predicate::PublicPath {
                    path: vec!["pkg".into(), "api".into(), "run".into()]
                },
                q,
                &b
            ),
            Outcome::Supported
        );
        assert_eq!(
            classify(
                &d,
                &output,
                m,
                c,
                Predicate::MemberKind {
                    kind: MemberKind::Function
                },
                q,
                &b
            ),
            Outcome::Supported
        );
        assert_eq!(
            classify(
                &d,
                &output,
                m,
                c,
                Predicate::MemberKind {
                    kind: MemberKind::Class
                },
                q,
                &b
            ),
            Outcome::Contradicted
        );
    }
    let all = Prepared::new(&d, &output, &b)
        .unwrap()
        .select(&Selection::default(), &b)
        .unwrap();
    assert_eq!(all.candidates[0].path, vec!["pkg", "api", "run"]);
}
#[test]
fn receiver_location_witness_is_retained_without_exact_state_promotion() {
    let (b, mut d, m, c, _) = fixture();
    let class = d
        .source
        .core
        .source_classes
        .insert(ClassEntity::Source {
            declaration: id(160),
        })
        .unwrap();
    let field = d
        .source
        .core
        .fields
        .insert(FieldEntity {
            class,
            name: "flag".into(),
        })
        .unwrap();
    let subject = d
        .source
        .catalog
        .subjects
        .insert(CatalogOptionSubject::Field { field })
        .unwrap();
    let default = d
        .source
        .catalog
        .defaults
        .insert(CatalogDefault::Unknown {})
        .unwrap();
    let option = d
        .source
        .catalog
        .options
        .insert(CatalogOption {
            member: m,
            subject,
            evidence: id(161),
            default,
        })
        .unwrap();
    let q = d.source.core.qualifications.iter().next().unwrap().id();
    // This pure C2 control assumes earlier C1 replay; it tests downstream retention and policy.
    let assessment = d
        .evidence
        .accesses
        .insert(c1::FieldAccessAssessment {
            option,
            occurrence: id(162),
            qualification: q,
            owner: id(163),
            receiver: None,
            field,
            constructor: None,
            kind: c1::FieldAccessKind::Read,
            phase: CallPhase::Call,
            basis: c1::AssociationBasis::SourceFieldCandidate,
            applicability: Knowledge::Unknown,
        })
        .unwrap();
    let link = d
        .evidence
        .field_locations
        .insert(c1::FieldLocationLink {
            assessment,
            location: id(164),
            candidate: id(165),
            phase: CallPhase::Call,
            applicability: Knowledge::Unknown,
            state: obligation::ObligationKind::HeapFieldStateUnavailable,
        })
        .unwrap();
    let output = selection::build::build(&d, &b).unwrap();
    assert!(
        output
            .witnesses
            .get(Witness::ReceiverLocation { link }.id())
            .is_some()
    );
    for kind in [
        FieldRelationship::ExactReader,
        FieldRelationship::ExactStorage,
    ] {
        assert_eq!(
            classify(
                &d,
                &output,
                m,
                c,
                Predicate::ConfigurationRelationship {
                    name: "flag".into(),
                    kind,
                    target: FieldTarget::Declaration { entity: id(166) }
                },
                Quantifier::AnyApplicable,
                &b
            ),
            Outcome::Unresolved
        );
    }
    let mut omitted = selection::build::build(&d, &b).unwrap();
    omitted.witnesses = Rows::new(&b);
    for w in output
        .witnesses
        .iter()
        .filter(|w| !matches!(w, Witness::ReceiverLocation { .. }))
    {
        omitted.witnesses.insert(w.clone()).unwrap();
    }
    assert!(Prepared::new(&d, &omitted, &b).is_err());
}
#[test]
fn completed_selection_inputs_close_new_lower_coverage_sum_arms() {
    let model = lctx_model::domain::model().unwrap();
    let stage = selection::build::stage(
        stages::Profile::Catalog,
        &model,
        &fixture_publication_order(),
    )
    .unwrap();
    let names = stage
        .inputs
        .iter()
        .map(|r| r.name())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(names.contains(analysis::local::AnalysisCoverage::NAME));
    assert!(names.contains(analysis::source_call::AnalysisCoverage::NAME));
    assert!(
        stage.outputs.iter().all(|r| !names.contains(r.name())),
        "a completed target closure cannot create a C2 backedge"
    );
}

#[test]
fn strict_preparation_owns_metadata_and_reuses_charged_indexes() {
    let (b, d, m, c, _) = fixture();
    let out = selection::build::build(&d, &b).unwrap();
    let prepared = Prepared::new(&d, &out, &b).unwrap();
    drop(d);
    drop(out);
    assert!(b.reserved() > 0);
    let requirement = Requirement {
        predicate: Predicate::PublicModule {
            module: "pkg.api".into(),
        },
        quantifier: Quantifier::AnyApplicable,
    };
    for _ in 0..3 {
        let result = prepared.classify(m, c, &requirement, &b).unwrap();
        assert_eq!(result.outcome, Outcome::Supported);
    }
    drop(prepared);
    assert_eq!(b.reserved(), 0);
}
#[test]
fn local_preparation_inventory_and_missing_membership_refusal() {
    use selection::classification::ClassificationData;
    assert_eq!(ClassificationData::inputs().len(), 79);
    assert!(ClassificationData::inputs().iter().any(|r| r.name() == execution::summary_exceptions::SummaryExceptionOutcome::NAME));
    assert!(
        ClassificationData::inputs()
            .iter()
            .all(|r| r.prefix().is_none())
    );
    let (b, d, _, _, _) = fixture();
    let expected = selection::build::build(&d, &b).unwrap();
    let mut missing = Output::new(&b);
    // A complete domain cannot silently lose its nominal context membership.
    macro_rules! rows {($($f:ident:$ty:ty,)*)=>{$(for row in expected.$f.iter().collect::<Vec<_>>().into_iter().rev() {if <$ty>::NAME!=DomainContext::NAME {missing.$f.insert(row.clone()).unwrap();}})*};}
    lctx_model::catalog_selection_outputs!(rows);
    assert!(
        Prepared::from_local_rows(ClassificationData::project(&d, &b).unwrap(), missing, &b)
            .is_err()
    );
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(Prepared::new(&d, &expected, &tiny).is_err());
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn shuffled_narrow_rows_classify_like_strict_replay() {
    use selection::classification::ClassificationData;
    let (b, d, m, c, _) = fixture();
    let expected = selection::build::build(&d, &b).unwrap();
    let strict = Prepared::new(&d, &expected, &b).unwrap();
    let mut data = ClassificationData::project(&d, &b).unwrap();
    // Feed actual typed metadata batches in an order different from the canonical inventory.
    data.source.catalog.members = Rows::new(&b);
    data.source.core.modules = Rows::new(&b);
    let members = d.source.catalog.members.iter().cloned().collect::<Vec<_>>();
    let modules = d.source.core.modules.iter().cloned().collect::<Vec<_>>();
    assert!(
        data.visit(
            CatalogMember::NAME,
            &CatalogMember::encode(&members.into_iter().rev().collect::<Vec<_>>()).unwrap()
        )
        .unwrap()
    );
    assert!(
        data.visit(
            Module::NAME,
            &Module::encode(&modules.into_iter().rev().collect::<Vec<_>>()).unwrap()
        )
        .unwrap()
    );
    let mut shuffled = Output::new(&b);
    macro_rules! rows {($($f:ident:$ty:ty,)*)=>{$(let rows=expected.$f.iter().cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();assert!(shuffled.visit(<$ty>::NAME,&<$ty as Record>::encode(&rows).unwrap()).unwrap());)*};}
    lctx_model::catalog_selection_outputs!(rows);
    let narrow = Prepared::from_local_rows(data, shuffled, &b).unwrap();
    for predicate in [
        Predicate::PublicModule {
            module: "pkg.api".into(),
        },
        Predicate::DeclaresParameter {
            name: "flag".into(),
        },
        Predicate::ParameterRequired {
            name: "flag".into(),
            required: false,
        },
    ] {
        let requirement = Requirement {
            predicate,
            quantifier: Quantifier::AnyApplicable,
        };
        let actual = narrow.classify(m, c, &requirement, &b).unwrap();
        let replay = strict.classify(m, c, &requirement, &b).unwrap();
        assert_eq!(
            (
                actual.outcome,
                actual.reason,
                actual.examined,
                actual.total,
                actual.corpus_complete,
                actual.analyzer_complete,
                &actual.closure
            ),
            (
                replay.outcome,
                replay.reason,
                replay.examined,
                replay.total,
                replay.corpus_complete,
                replay.analyzer_complete,
                &replay.closure
            )
        );
        assert_eq!(
            format!("{:?}", actual.witnesses),
            format!("{:?}", replay.witnesses)
        );
        assert_eq!(
            actual.admissible().collect::<Vec<_>>(),
            replay.admissible().collect::<Vec<_>>()
        );
    }
}

fn fixture_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    lctx_model::domain::stages::PublicationOrder::registered(
        lctx_model::domain::ContentHash::of(b"fixture publication order"),
        &[
            (0, lctx_model::domain::stages::PublicationBoundary::Facts),
            (1, lctx_model::domain::stages::PublicationBoundary::Local),
            (2, lctx_model::domain::stages::PublicationBoundary::Model),
            (3, lctx_model::domain::stages::PublicationBoundary::Summary),
            (
                4,
                lctx_model::domain::stages::PublicationBoundary::Structural,
            ),
            (5, lctx_model::domain::stages::PublicationBoundary::Analytic),
            (
                6,
                lctx_model::domain::stages::PublicationBoundary::Synthesis,
            ),
        ],
    )
    .unwrap()
}

#[test]
fn prepared_public_formal_domain_survives_absent_native_context_and_refuses_foreign_inputs() {
    use native_requests::{
        Assumptions, ExactScalar, FormalDomain, PreparationInputs, PreparedNativeSemantics,
    };
    use serving::{ExactInputBinding, InspectValuePathsRequest, PageRequest};
    let (budget, mut data, member, analysis, source) = fixture();
    let slot = data.source.core.slots.iter().next().unwrap().clone();
    assert_eq!(slot.default, DefaultSlot::DefinitionTime);
    let declaration = data
        .source
        .core
        .occurrences
        .insert(Occurrence {
            source: source.id(),
            start: 8,
            end: 12,
            syntax_kind: SyntaxKind::Parameter,
            role: OccurrenceRole::Syntax,
            structural_path: vec![8],
        })
        .unwrap();
    let formal = data
        .source
        .core
        .parameters
        .insert(ParameterEntity::Source { declaration })
        .unwrap();
    data.source
        .core
        .parameter_links
        .insert(ParameterEntityLink {
            parameter: slot.parameter,
            entity: formal,
            declaration: None,
        })
        .unwrap();
    let classification = classification::ClassificationData::project(&data, &budget).unwrap();
    let inputs = PreparationInputs::new(&budget);
    let prepared = PreparedNativeSemantics::prepare(inputs, &classification, &budget).unwrap();
    let mut request = InspectValuePathsRequest {
        member,
        analysis,
        inputs: vec![ExactInputBinding {
            formal,
            value: ExactScalar::None {},
        }],
        assumptions: Assumptions::default(),
        page: PageRequest::default(),
    };
    let owner = prepared.domain().resolve(&mut request).unwrap();
    assert_eq!(prepared.path_count(owner, formal, analysis), None);
    assert!(prepared.unexamined(owner, formal, analysis));
    let mut duplicate = request.clone();
    duplicate.inputs.push(duplicate.inputs[0].clone());
    assert!(prepared.domain().resolve(&mut duplicate).is_err());
    let mut foreign = request.clone();
    foreign.inputs[0].formal = id(250);
    assert!(prepared.domain().resolve(&mut foreign).is_err());
    let mut foreign_analysis = request.clone();
    foreign_analysis.analysis = id(251);
    assert!(prepared.domain().resolve(&mut foreign_analysis).is_err());
    let tiny = ResourceBudget::fixed(8192).unwrap();
    assert!(FormalDomain::prepare(&classification, &tiny).is_err());
    assert_eq!(
        tiny.reserved(),
        0,
        "failed index construction releases every charge"
    );
    let independent = ResourceBudget::fixed(1 << 20).unwrap();
    {
        let index = FormalDomain::prepare(&classification, &independent).unwrap();
        assert!(index.contains_member(member));
        assert!(independent.reserved() > 0);
    }
    assert_eq!(independent.reserved(), 0);
}

#[test]
fn an_unobserved_native_role_is_unknown_even_when_source_enumeration_is_complete() {
    let (b,d,member,context,_) = fixture();
    let output = lctx_model::domain::selection::build::build(&d,&b).unwrap();
    assert_eq!(classify(&d,&output,member,context,Predicate::VariantReturnType {role:SignatureRole::EffectiveTyped,r#type:StructuralType::Category {kind:0}},Quantifier::AnyApplicable,&b),Outcome::Unresolved);
}

#[test]
fn exact_exception_raises_uses_runtime_summary_and_never_native_typing_absence() {
    use execution::{ExactRuntimeException, summary_exceptions::SummaryExceptionOutcome};
    let (b, mut data, member, context, _) = fixture();
    let owner = data.source.core.entity_candidates.iter().next().unwrap().entity;
    let q = data.source.core.qualifications.iter().next().unwrap().id();
    let result = SummaryExceptionOutcome { invocation: id(190), body: id(191), input: data.source.catalog.members.get(member).unwrap().input,
        context, owner, qualification: q, outcome: id(192), exception: Some(ExactRuntimeException::ValueError), status: analysis::policy::EvidenceStatus::StructurallyObserved };
    let requirement = |exception| Requirement { predicate: Predicate::BehavioralRaises { exception }, quantifier: Quantifier::AnyApplicable };
    let classify = |data: &Data, exception| {
        let output = selection::build::build(data, &b).unwrap();
        Prepared::new(data, &output, &b).unwrap().classify(member, context, &requirement(exception), &b).unwrap()
    };
    assert_eq!(classify(&data, ExactRuntimeException::ValueError).outcome, Outcome::Unresolved, "no completed body is not absence");
    data.facts.exception_outcomes.insert(result.clone()).unwrap();
    let supported = classify(&data, ExactRuntimeException::ValueError);
    assert_eq!(supported.outcome, Outcome::Supported);
    assert!(supported.witnesses.iter().any(|w| matches!(w, selection::algebra::RequirementWitness::Positive { basis: EvidenceBasis::BoundedModel, evidence, .. } if evidence.iter().any(|w|matches!(w,Witness::SummaryException {outcome} if *outcome==result.id())))));
    assert_eq!(classify(&data, ExactRuntimeException::TypeError).outcome, Outcome::Contradicted);
    data.facts.exception_outcomes = Rows::new(&b);
    data.facts.exception_outcomes.insert(SummaryExceptionOutcome { context: id(193), ..result.clone() }).unwrap();
    assert_eq!(classify(&data, ExactRuntimeException::ValueError).outcome, Outcome::Unresolved, "foreign context cannot supply runtime evidence");
    data.facts.exception_outcomes = Rows::new(&b);
    data.facts.exception_outcomes.insert(SummaryExceptionOutcome { input: id(194), ..result.clone() }).unwrap();
    assert_eq!(classify(&data, ExactRuntimeException::ValueError).outcome, Outcome::Unresolved, "foreign input cannot supply runtime evidence");
    data.facts.exception_outcomes = Rows::new(&b);
    data.facts.exception_outcomes.insert(SummaryExceptionOutcome { exception: None, ..result }).unwrap();
    assert_eq!(classify(&data, ExactRuntimeException::ValueError).outcome, Outcome::Contradicted, "normal admitted body supplies its own finite absence evidence");
}

#[test]
fn later_unreferenced_qualification_does_not_expand_c2_witness_closure() {
    let (b, mut d, _, _, _) = fixture();
    let expected = lctx_model::domain::selection::build::build(&d, &b).unwrap();
    let mut later = d.source.core.qualifications.iter().next().unwrap().clone();
    later.condition = conditions::Diagram::never().id();
    d.source.core.qualifications.insert(later).unwrap();
    let replayed = lctx_model::domain::selection::build::build(&d, &b).unwrap();
    expected.matches(&replayed).unwrap();
}
