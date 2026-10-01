//! Hand-expected contextual controls over canonical identities, independent of native extraction.
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    catalog::{
        evidence::{build::*, *},
        *,
    },
    deployment::*,
    documents::*,
    input::*,
    lexical::*,
    normalized::{callables::*, entities::*, events::*, links::*},
    resources::ResourceBudget,
    source::*,
    syntax::*,
    *,
};
fn nominal<T>(n: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([n; 16].into_iter()))
    .unwrap()
}
fn setup(
    path: &str,
    bytes: &[u8],
    role: SourceRole,
) -> (
    ResourceBudget,
    EvidenceData,
    SourceArtifact,
    AssertionQualification,
) {
    let b = ResourceBudget::fixed(32 << 20).unwrap();
    let mut d = EvidenceData::new(&b);
    let source =
        SourceArtifact::from_bytes(nominal::<InputRevision>(1), path.into(), bytes).unwrap();
    d.core.artifacts.insert(source.clone()).unwrap();
    d.facts
        .uses
        .insert(ArtifactUse {
            artifact: source.id(),
            input: source.input,
            role,
        })
        .unwrap();
    let q = AssertionQualification {
        context: nominal(2),
        scope: CoverageScope::Artifact {
            artifact: source.id(),
        }
        .id(),
        condition: conditions::Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    d.core.qualifications.insert(q.clone()).unwrap();
    d.core
        .native_coverage
        .insert(ProviderCoverage {
            scope: q.scope,
            provider: Some(nominal(3)),
            context: q.context,
            family: FactFamily::Syntax,
            run: Some(nominal(4)),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        })
        .unwrap();
    (b, d, source, q)
}
fn occurrence(
    d: &mut EvidenceData,
    source: &SourceArtifact,
    start: i64,
    end: i64,
    kind: SyntaxKind,
) -> Id<Occurrence> {
    d.core
        .occurrences
        .insert(Occurrence {
            source: source.id(),
            start,
            end,
            syntax_kind: kind,
            role: OccurrenceRole::Syntax,
            structural_path: vec![start as i32],
        })
        .unwrap()
}
fn placement(
    d: &mut EvidenceData,
    q: &AssertionQualification,
    child: Id<Occurrence>,
    parent: Id<Occurrence>,
    field: SyntaxField,
    ordinal: i64,
) {
    d.core
        .placements
        .insert(SyntaxPlacement {
            qualification: q.id(),
            occurrence: child,
            parent: Some(parent),
            field,
            ordinal,
        })
        .unwrap();
}
fn public_callable(
    d: &mut EvidenceData,
    source: &SourceArtifact,
    q: &AssertionQualification,
) -> (Id<CatalogMember>, Id<CallableEntity>) {
    let callable = CallableEntity::Source {
        declaration: nominal(5),
        kind: CallableKind::Function,
    };
    d.core.source_callables.insert(callable.clone()).unwrap();
    let a = d
        .core
        .assessments
        .insert(EffectiveCallableAssessment {
            callable: callable.id(),
            context: q.context,
            decorators: ContentHash::of(b"decorators"),
            policy: ContentHash::of(b"policy"),
            identity: Knowledge::Unknown,
            identity_reason: CallableReason::UnsupportedDecorator,
            signatures: Knowledge::Known,
            signature_reason: CallableReason::EvidenceAgreement,
            descriptor: Knowledge::Unknown,
            descriptor_kind: None,
            descriptor_reason: CallableReason::UnsupportedDecorator,
            body: Knowledge::Unknown,
            body_admitted: false,
            body_reason: CallableReason::UnsupportedDecorator,
            asynchronous: None,
            generator: None,
        })
        .unwrap();
    let module = d
        .core
        .modules
        .insert(Module {
            source: source.id(),
            qualified_name: "api".into(),
        })
        .unwrap();
    let member = d
        .catalog
        .members
        .insert(CatalogMember {
            input: source.input,
            access: module,
            path: vec!["outside_brief_seeds".into()],
            name: "outside_brief_seeds".into(),
        })
        .unwrap();
    d.catalog
        .callables
        .insert(CatalogCallable {
            member,
            candidate: nominal(7),
            assessment: a,
            basis: CatalogContractBasis::PublicCandidate,
        })
        .unwrap();
    let invocation = d
        .facts
        .core_invocations
        .insert(analysis::catalog_core::Invocation {
            input: source.input,
            context: q.context,
            definition: nominal(8),
            subject: None,
            inputs: ContentHash::of(b"parents"),
            sources: ContentHash::of(b"sources"),
            projections: ContentHash::of(b"projections"),
        })
        .unwrap();
    d.facts
        .core_links
        .insert(CatalogMemberInvocation { member, invocation })
        .unwrap();
    (member, callable.id())
}
fn event(
    d: &mut EvidenceData,
    source: &SourceArtifact,
    q: &AssertionQualification,
    site: Id<Occurrence>,
    callable: Id<CallableEntity>,
    target_index: u8,
) -> Id<NormalizedCallEvent> {
    let owner = occurrence(d, source, 0, source.byte_len, SyntaxKind::ModModule);
    let entity = d
        .core
        .refs
        .insert(EntityRef::Module { module: nominal(9) })
        .unwrap();
    let ownership = d
        .core
        .ownership
        .insert(OccurrenceOwnership {
            occurrence: site,
            owner,
            entity,
        })
        .unwrap();
    let event = d
        .facts
        .events
        .insert(NormalizedCallEvent {
            site,
            origin: nominal(target_index),
            context: q.context,
            owner: ownership,
        })
        .unwrap();
    let raw = d
        .facts
        .raw_targets
        .insert(CallTarget {
            qualification: q.id(),
            site,
            origin: nominal(target_index),
            destination: nominal(10),
            channel: CallChannel::Direct {}.id(),
            phase: CallPhase::Call,
            receiver: Receiver::None {}.id(),
            implicit: false,
            receiver_class: None,
            passing: None,
            class_method: None,
            static_method: None,
        })
        .unwrap();
    let source = d
        .facts
        .alternative_sources
        .insert(CallAlternativeSource::Native { target: raw })
        .unwrap();
    let entity = d
        .core
        .refs
        .insert(EntityRef::Callable { callable })
        .unwrap();
    d.facts
        .alternatives
        .insert(NormalizedCallAlternative {
            event,
            source,
            resolution: None,
            correspondence: None,
            entity: Some(entity),
            status: ResolutionStatus::Resolved,
            reason: LinkReason::ExplicitIdentity,
        })
        .unwrap();
    event
}
#[test]
fn original_mdx_fence_and_extracted_python_coordinates_are_separate_and_complete() {
    let bytes = "# λ\n\n```python\nf()\n```\n".as_bytes();
    let (b, mut d, document, q) = setup("guide.mdx", bytes, SourceRole::Document);
    let original = Evidence::SourceSpan {
        source: document.id(),
        start: 6,
        end: bytes.len() as i64,
    };
    let original_id = EvidenceSourceSpanId::of(&original).unwrap();
    d.facts.canonical_evidence.insert(original).unwrap();
    let passage = Evidence::SourceSpan {
        source: document.id(),
        start: 0,
        end: bytes.len() as i64,
    };
    let passage_id = EvidenceSourceSpanId::of(&passage).unwrap();
    d.facts.canonical_evidence.insert(passage).unwrap();
    let block = d
        .facts
        .nodes
        .insert(DocumentNode::CodeBlock {
            span: original_id,
            ordinal: 0,
        })
        .unwrap();
    let passage = d
        .facts
        .nodes
        .insert(DocumentNode::Passage {
            span: passage_id,
            ordinal: 0,
        })
        .unwrap();
    let python =
        SourceArtifact::from_bytes(document.input, "_lctx/code.py".into(), b"f()").unwrap();
    d.core.artifacts.insert(python.clone()).unwrap();
    d.facts
        .blocks
        .insert(CodeBlockObservation {
            qualification: q.id(),
            block: DocumentNodeCodeBlockId::of(d.facts.nodes.get(block).unwrap()).unwrap(),
            passage: DocumentNodePassageId::of(d.facts.nodes.get(passage).unwrap()).unwrap(),
            language: Some("python".into()),
            meta: None,
            code: "f()".into(),
            content: python.content,
            module_path: Some(python.path.clone()),
            materialized: Some(python.id()),
        })
        .unwrap();
    d.facts
        .documents
        .insert(DocumentObservation {
            qualification: q.id(),
            source: document.id(),
            title: Some("λ".into()),
            parsed: true,
        })
        .unwrap();
    let out = build(&d, &b).unwrap();
    assert_eq!(out.scenarios.len(), 1);
    assert_eq!(out.spans.len(), 3);
    assert!(
        out.original_sources
            .iter()
            .any(|r| *r == OriginalSource::Span { span: original_id })
    );
    assert!(out.original_sources.iter().any(|r| *r
        == OriginalSource::Artifact {
            artifact: python.id()
        }));
    assert!(
        out.scenarios
            .iter()
            .all(|r| r.execution == CheckStatus::NotRun)
    );
    assert!(
        out.subjects
            .iter()
            .any(|r| matches!(r, RootSubject::Document { .. }))
    );
    assert!(invocation_links(&out, &normalized::Rows::new(&b), &b).is_err());
}
#[test]
fn two_fields_do_not_share_unrelated_readers_and_source_links_remain_unknown() {
    let (b, mut d, source, q) = setup("api.py", b"class C:\n    pass\n", SourceRole::Release);
    let class = ClassEntity::Source {
        declaration: nominal(20),
    };
    d.core.source_classes.insert(class.clone()).unwrap();
    let method_site = occurrence(&mut d, &source, 1, 4, SyntaxKind::StmtFunctionDef);
    let callable = CallableEntity::Source {
        declaration: method_site,
        kind: CallableKind::Function,
    };
    d.core.source_callables.insert(callable.clone()).unwrap();
    d.core
        .declarations
        .insert(DeclarationObservation {
            qualification: q.id(),
            declaration: method_site,
            name: nominal(21),
            kind: DeclarationKind::Function,
            parent: Some(nominal(20)),
            overload: false,
            docstring: None,
        })
        .unwrap();
    let entity = d
        .core
        .refs
        .insert(EntityRef::Callable {
            callable: callable.id(),
        })
        .unwrap();
    let attribute = occurrence(&mut d, &source, 5, 9, SyntaxKind::ExprAttribute);
    d.core
        .ownership
        .insert(OccurrenceOwnership {
            occurrence: attribute,
            owner: method_site,
            entity,
        })
        .unwrap();
    let name = occurrence(&mut d, &source, 6, 7, SyntaxKind::Identifier);
    placement(&mut d, &q, name, attribute, SyntaxField::Child, 0);
    d.facts
        .spellings
        .insert(SyntaxObservation {
            qualification: q.id(),
            occurrence: name,
            spelling: "left".into(),
        })
        .unwrap();
    let module = d
        .core
        .modules
        .insert(Module {
            source: source.id(),
            qualified_name: "api".into(),
        })
        .unwrap();
    let member = d
        .catalog
        .members
        .insert(CatalogMember {
            input: source.input,
            access: module,
            path: vec!["C".into()],
            name: "C".into(),
        })
        .unwrap();
    let invocation = d
        .facts
        .core_invocations
        .insert(
            analysis::catalog_core::Invocation::new(
                source.input,
                q.context,
                catalog::build::definition().1.id(),
                None,
                [],
            )
            .0,
        )
        .unwrap();
    d.facts
        .core_links
        .insert(CatalogMemberInvocation { member, invocation })
        .unwrap();
    let mut left = None;
    let mut right = None;
    for name in ["left", "right"] {
        let field = d
            .core
            .fields
            .insert(FieldEntity {
                class: class.id(),
                name: name.into(),
            })
            .unwrap();
        let subject = d
            .catalog
            .subjects
            .insert(CatalogOptionSubject::Field { field })
            .unwrap();
        let option = d
            .catalog
            .options
            .insert(CatalogOption {
                member,
                subject,
                evidence: nominal(23),
                default: CatalogDefault::Unknown {}.id(),
            })
            .unwrap();
        if name == "left" {
            left = Some(option);
        } else {
            right = Some(option);
        }
    }
    let out = build(&d, &b).unwrap();
    assert_eq!(out.accesses.len(), 1);
    let access = out.accesses.iter().next().unwrap();
    assert_eq!(access.option, left.unwrap());
    assert_ne!(access.option, right.unwrap());
    assert_eq!(access.kind, FieldAccessKind::Read);
    assert_eq!(access.applicability, Knowledge::Unknown);
    assert_eq!(access.basis, AssociationBasis::SourceFieldCandidate);
    // Pure C1 join assumes the Local owner replayed this stored location; nominal support IDs
    // here are not a native/store qualification. The actual native control uses its opaque derive.
    let local = analysis::local::Invocation::new(
        source.input,
        q.context,
        local_semantics::definition().1.id(),
        None,
        [],
    )
    .0;
    d.runtime.local_invocations.insert(local.clone()).unwrap();
    let receiver = occurrence(&mut d, &source, 5, 6, SyntaxKind::ExprName);
    let value = SyntaxPlacement {
        qualification: q.id(),
        occurrence: receiver,
        parent: Some(attribute),
        field: SyntaxField::Value,
        ordinal: 0,
    };
    d.core.placements.insert(value.clone()).unwrap();
    let location = local_fields::FieldLocation {
        invocation: local.id(),
        load: nominal(110),
        support: nominal(111),
        placement: value.id(),
        placement_support: nominal(112),
        entry: nominal(113),
        receiver: nominal(114),
        owner: entity,
        qualification: q.id(),
        phase: CallPhase::Call,
        state: local_fields::FieldStateBoundary::AllocationAliasAndMutationUnavailable,
    };
    d.runtime.locations.insert(location.clone()).unwrap();
    let candidate = local_fields::FieldLocationCandidate {
        location: location.id(),
        field: access.field,
        receiver_type: nominal(115),
        type_support: nominal(116),
        class_resolution: nominal(117),
        declaration: nominal(118),
    };
    d.runtime.location_candidates.insert(candidate).unwrap();
    let linked = build(&d, &b).unwrap();
    assert_eq!(linked.field_locations.len(), 1);
    let link = linked.field_locations.iter().next().unwrap();
    assert_eq!(link.assessment, access.id());
    assert_eq!(link.phase, CallPhase::Call);
    assert_eq!(link.applicability, Knowledge::Unknown);
    assert_eq!(
        link.state,
        obligation::ObligationKind::HeapFieldStateUnavailable
    );
    let retained_link = link.clone();
    let mut replay = build(&d, &b).unwrap();
    replay.field_locations = normalized::Rows::new(&b);
    assert!(replay.matches(&linked).is_err());
    replay
        .field_locations
        .insert(FieldLocationLink {
            phase: CallPhase::Init,
            ..retained_link.clone()
        })
        .unwrap();
    assert!(replay.matches(&linked).is_err());
    replay.field_locations = normalized::Rows::new(&b);
    replay
        .field_locations
        .insert(FieldLocationLink {
            candidate: nominal(120),
            ..retained_link
        })
        .unwrap();
    assert!(replay.matches(&linked).is_err());
    assert!(
        linked
            .accesses
            .iter()
            .all(|r| r.applicability == Knowledge::Unknown
                && r.basis == AssociationBasis::SourceFieldCandidate)
    );
    let before = d.runtime.local_invocations.iter().next().unwrap().clone();
    d.runtime.local_invocations = normalized::Rows::new(&b);
    d.runtime
        .local_invocations
        .insert(analysis::local::Invocation {
            context: nominal(119),
            ..before.clone()
        })
        .unwrap();
    assert!(build(&d, &b).is_err());
    d.runtime.local_invocations = normalized::Rows::new(&b);
    d.runtime.local_invocations.insert(before).unwrap();
    // An earlier C0 invocation in another context cannot admit a source-only field candidate.
    d.facts.core_links = normalized::Rows::new(&b);
    assert_eq!(build(&d, &b).unwrap().accesses.len(), 0);
    d.facts
        .core_links
        .insert(CatalogMemberInvocation { member, invocation })
        .unwrap();
    // A same-named read owned by a function outside this class supplies no field association.
    d.core.declarations = normalized::Rows::new(&b);
    d.core
        .declarations
        .insert(DeclarationObservation {
            qualification: q.id(),
            declaration: method_site,
            name: nominal(21),
            kind: DeclarationKind::Function,
            parent: None,
            overload: false,
            docstring: None,
        })
        .unwrap();
    assert_eq!(build(&d, &b).unwrap().accesses.len(), 0);
}
#[test]
fn explicit_test_intent_respects_with_body_headers_and_deferred_generator_elements() {
    let (b, mut d, source, q) = setup(
        "test.py",
        b"with raises():\n    g = (f() for x in f())\n",
        SourceRole::Test,
    );
    let (_, callable) = public_callable(&mut d, &source, &q);
    let with = occurrence(&mut d, &source, 0, source.byte_len, SyntaxKind::StmtWith);
    let item = occurrence(&mut d, &source, 1, 14, SyntaxKind::WithItem);
    let raises = occurrence(&mut d, &source, 2, 12, SyntaxKind::ExprCall);
    placement(&mut d, &q, item, with, SyntaxField::Item, 0);
    placement(&mut d, &q, raises, item, SyntaxField::Value, 0);
    let module = d
        .core
        .provider_modules
        .insert(ProviderModule::Bundled {
            provider: nominal(30),
            bundle: ModuleBundle::TypeshedThirdParty,
            name: "pytest".into(),
        })
        .unwrap();
    let symbol = d
        .core
        .symbols
        .insert(ProviderSymbol {
            provider: nominal(30),
            context: q.context,
            module,
            native_key: "pytest.raises".into(),
            name: "raises".into(),
            kind: SymbolKind::Function,
        })
        .unwrap();
    let destination = d
        .facts
        .destinations
        .insert(CallDestination::Resolved { symbol })
        .unwrap();
    let entity = d
        .core
        .refs
        .insert(EntityRef::Callable {
            callable: nominal(31),
        })
        .unwrap();
    d.core
        .resolutions
        .insert(SymbolEntityResolution {
            symbol,
            context: q.context,
            policy: ContentHash::of(b"p"),
            status: ResolutionStatus::Resolved,
            entity: Some(entity),
            reason: EntityReason::ProviderExternal,
        })
        .unwrap();
    d.facts
        .raw_targets
        .insert(CallTarget {
            qualification: q.id(),
            site: raises,
            origin: nominal(32),
            destination,
            channel: CallChannel::Direct {}.id(),
            phase: CallPhase::Call,
            receiver: Receiver::None {}.id(),
            implicit: false,
            receiver_class: None,
            passing: None,
            class_method: None,
            static_method: None,
        })
        .unwrap();
    let generator = occurrence(
        &mut d,
        &source,
        16,
        source.byte_len,
        SyntaxKind::ExprGenerator,
    );
    let element = occurrence(&mut d, &source, 20, 23, SyntaxKind::ExprCall);
    let clause = occurrence(&mut d, &source, 24, 39, SyntaxKind::Comprehension);
    let eager = occurrence(&mut d, &source, 33, 36, SyntaxKind::ExprCall);
    placement(&mut d, &q, generator, with, SyntaxField::Body, 0);
    placement(&mut d, &q, element, generator, SyntaxField::Element, 0);
    placement(&mut d, &q, clause, generator, SyntaxField::Item, 0);
    placement(&mut d, &q, eager, clause, SyntaxField::Iter, 0);
    event(&mut d, &source, &q, element, callable, 33);
    event(&mut d, &source, &q, eager, callable, 34);
    let filter = occurrence(&mut d, &source, 25, 26, SyntaxKind::ExprCall);
    placement(&mut d, &q, filter, clause, SyntaxField::Test, 0);
    event(&mut d, &source, &q, filter, callable, 35);
    let second = occurrence(&mut d, &source, 36, 39, SyntaxKind::Comprehension);
    let later_iter = occurrence(&mut d, &source, 37, 38, SyntaxKind::ExprCall);
    placement(&mut d, &q, second, generator, SyntaxField::Item, 1);
    placement(&mut d, &q, later_iter, second, SyntaxField::Iter, 0);
    event(&mut d, &source, &q, later_iter, callable, 36);
    let header = occurrence(&mut d, &source, 13, 14, SyntaxKind::ExprCall);
    placement(&mut d, &q, header, raises, SyntaxField::Argument, 0);
    event(&mut d, &source, &q, header, callable, 37);
    let function = occurrence(&mut d, &source, 15, 25, SyntaxKind::StmtFunctionDef);
    let deferred = occurrence(&mut d, &source, 18, 19, SyntaxKind::ExprCall);
    placement(&mut d, &q, function, with, SyntaxField::Body, 1);
    placement(&mut d, &q, deferred, function, SyntaxField::Body, 0);
    event(&mut d, &source, &q, deferred, callable, 38);
    let out = build(&d, &b).unwrap();
    assert_eq!(out.associations.len(), 6);
    assert_eq!(
        out.associations
            .iter()
            .filter(|r| r.intent == Intent::ExpectedFailure)
            .count(),
        1
    );
    assert_eq!(
        out.associations
            .iter()
            .filter(|r| r.intent == Intent::AssertionTest)
            .count(),
        5
    );
    assert!(
        out.associations
            .iter()
            .any(|r| r.intent == Intent::ExpectedFailure)
    );
    assert!(
        out.associations
            .iter()
            .any(|r| r.intent == Intent::AssertionTest)
    );
    assert!(out.scenarios.iter().any(|r| r.intent == Intent::Mixed));
    assert!(
        out.associations
            .iter()
            .all(|r| r.basis == AssociationBasis::CandidateTarget)
    );
    // One captured artifact can be declared for multiple roles. Each role retains its own evidence.
    d.facts
        .uses
        .insert(ArtifactUse {
            artifact: source.id(),
            input: source.input,
            role: SourceRole::Example,
        })
        .unwrap();
    let multiple = build(&d, &b).unwrap();
    assert_eq!(multiple.scenarios.len(), 2);
    assert_eq!(multiple.associations.len(), 12);
    assert!(out.roots.iter().any(|r| {
        out.subjects
            .get(r.subject)
            .is_some_and(|s| matches!(s, RootSubject::Member { .. }))
    }));
}
#[test]
fn dependency_role_is_not_a_scenario_root_and_parse_failure_is_separate_from_execution() {
    let (b, mut d, source, q) = setup("bad.py", b"def broken(\n", SourceRole::Dependency);
    assert_eq!(build(&d, &b).unwrap().scenarios.len(), 0);
    d.facts
        .uses
        .insert(ArtifactUse {
            artifact: source.id(),
            input: source.input,
            role: SourceRole::Test,
        })
        .unwrap();
    d.core.native_coverage = normalized::Rows::new(&b);
    d.core
        .native_coverage
        .insert(ProviderCoverage {
            scope: q.scope,
            provider: Some(nominal(3)),
            context: q.context,
            family: FactFamily::Syntax,
            run: Some(nominal(4)),
            status: CoverageStatus::Partial,
            reason: Some(obligation::ObligationKind::SyntaxError),
            diagnostic: None,
        })
        .unwrap();
    let out = build(&d, &b).unwrap();
    let scenario = out.scenarios.iter().next().unwrap();
    assert_eq!(scenario.parse, CheckStatus::Failed);
    assert_eq!(scenario.execution, CheckStatus::NotRun);
    assert_eq!(scenario.environment, CheckStatus::NotRun);
    assert!(
        out.dependencies
            .iter()
            .all(|r| r.status == CheckStatus::Blocked)
    );
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(build(&d, &tiny), Err(ModelError::Resource { .. })));
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn release_requirements_are_owned_once_and_never_matched_by_distribution_spelling() {
    let (b, mut d, source, q) = setup(
        "pkg.dist-info/METADATA",
        b"Requires-Dist: dependency[extra]>=1\n",
        SourceRole::DistributionMetadata,
    );
    let span = Evidence::SourceSpan {
        source: source.id(),
        start: 0,
        end: source.byte_len,
    };
    let span_id = EvidenceSourceSpanId::of(&span).unwrap();
    d.facts.canonical_evidence.insert(span).unwrap();
    for ordinal in 0..4 {
        d.facts
            .deployment
            .insert(DeploymentObservation {
                qualification: q.id(),
                span: span_id,
                ordinal,
                distribution: Some("pkg".into()),
                version: Some("1".into()),
                field: "requires_dist".into(),
                original: "dependency[extra]>=1".into(),
                name: Some("dependency".into()),
                extras: vec!["extra".into()],
                marker: None,
                constraint: Some(">=1".into()),
                interpretation: CheckStatus::Passed,
                diagnostic: None,
                environment_digest: None,
                lock_digest: None,
                referenced_path: None,
            })
            .unwrap();
    }
    let unowned = build(&d, &b).unwrap();
    assert_eq!(unowned.deployments.len(), 4);
    assert_eq!(unowned.release_deployments.len(), 0);
    let release = nominal(40);
    let verification = d
        .facts
        .verifications
        .insert(DistributionVerification {
            acquisition: nominal(41),
            release,
            record_digest: ContentHash::of(b"record"),
            artifact_sha256: vec![],
        })
        .unwrap();
    d.facts
        .artifact_ownership
        .insert(ArtifactOwnership {
            artifact: source.id(),
            distribution: verification,
        })
        .unwrap();
    let out = build(&d, &b).unwrap();
    assert_eq!(out.release_deployments.len(), 4);
    assert_eq!(
        out.subjects
            .iter()
            .filter(|r| matches!(r, RootSubject::Release { .. }))
            .count(),
        1
    );
    assert_eq!(
        out.subjects
            .iter()
            .filter(|r| matches!(r, RootSubject::Member { .. }))
            .count(),
        0
    );
    // The same metadata spelling does not connect an ownership proof for another artifact.
    d.facts.artifact_ownership = normalized::Rows::new(&b);
    d.facts
        .artifact_ownership
        .insert(ArtifactOwnership {
            artifact: nominal(42),
            distribution: verification,
        })
        .unwrap();
    assert_eq!(build(&d, &b).unwrap().release_deployments.len(), 0);
}
#[test]
fn completed_stage_inventory_requires_core_and_original_receipts_without_flow_or_briefs() {
    let model = lctx_model::domain::model().unwrap();
    let stage = stage(stages::Profile::Catalog, &model).unwrap();
    let names = stage
        .inputs
        .iter()
        .map(|r| r.name())
        .collect::<std::collections::BTreeSet<_>>();
    for name in [
        CatalogMember::NAME,
        analysis::catalog_core::Invocation::NAME,
        TaskReportObservation::NAME,
        DocumentObservation::NAME,
        artifact::ArtifactChunk::NAME,
    ] {
        assert!(names.contains(name), "missing {name}");
    }
    assert!(!names.contains(flow::FlowUseObservation::NAME));
    assert!(
        !names
            .iter()
            .any(|r| r.contains("brief") || r.contains("embedding"))
    );
    assert!(
        !catalog::build::stage(stages::Profile::Catalog)
            .outputs
            .iter()
            .any(|r| r.name() == CatalogScenario::NAME)
    );
}

#[test]
fn supplied_report_is_scoped_by_actual_target_context_and_acquired_environment() {
    let (b, mut d, source, q) = setup("example.py", b"f()", SourceRole::Example);
    let environment = d
        .facts
        .reported_environments
        .insert(ReportedEnvironment {
            release: nominal(60),
            lock_digest: ContentHash::of(b"lock"),
            environment_digest: ContentHash::of(b"env"),
            runtime_digest: ContentHash::of(b"runtime"),
            interpreter_digest: ContentHash::of(b"python"),
            python_version: "3.14.7".into(),
            platform: "linux".into(),
            requirement: "pkg==1".into(),
            metadata: nominal(61),
        })
        .unwrap();
    let report = d
        .facts
        .reports
        .insert(TaskReport {
            format: 1,
            policy: "captured".into(),
            task: "example".into(),
            runner_sha256: "a".repeat(64),
            source_path: source.path.clone(),
            source_sha256: "b".repeat(64),
            environment,
            invocation: nominal(62),
            tool: "fixture".into(),
            elapsed_ms: Milliseconds(1),
            timeout_seconds: 5,
            execution: CheckStatus::Passed,
            result: Some("pass".into()),
            diagnostic: None,
        })
        .unwrap();
    let receipt = Evidence::SourceSpan {
        source: source.id(),
        start: 0,
        end: source.byte_len,
    };
    let receipt_id = EvidenceSourceSpanId::of(&receipt).unwrap();
    d.facts.canonical_evidence.insert(receipt).unwrap();
    let row = TaskReportObservation {
        qualification: q.id(),
        receipt: receipt_id,
        target: source.id(),
        report,
    };
    d.facts.report_observations.insert(row.clone()).unwrap();
    let unverified = build(&d, &b).unwrap();
    assert!(
        unverified
            .scenarios
            .iter()
            .all(|s| s.execution == CheckStatus::Passed && s.environment == CheckStatus::Blocked)
    );
    assert!(unverified.checks.iter().all(|c| c.fingerprint.is_none()));
    let acquisition = d
        .facts
        .acquisitions
        .insert(InputAcquisition {
            input: source.input,
            origin: nominal(63),
        })
        .unwrap();
    let e = d.facts.reported_environments.get(environment).unwrap();
    let fingerprint = EnvironmentFingerprint {
        acquisition,
        release: e.release,
        lock_digest: e.lock_digest,
        environment_digest: e.environment_digest,
        python_version: e.python_version.clone(),
        platform: e.platform.clone(),
    };
    d.facts.fingerprints.insert(fingerprint.clone()).unwrap();
    assert!(
        build(&d, &b)
            .unwrap()
            .checks
            .iter()
            .all(|c| c.environment == CheckStatus::Passed)
    );
    // Equal environment spelling from a foreign input cannot qualify this scenario.
    d.facts.acquisitions = normalized::Rows::new(&b);
    let foreign = d
        .facts
        .acquisitions
        .insert(InputAcquisition {
            input: nominal(64),
            origin: nominal(63),
        })
        .unwrap();
    d.facts.fingerprints = normalized::Rows::new(&b);
    d.facts
        .fingerprints
        .insert(EnvironmentFingerprint {
            acquisition: foreign,
            ..fingerprint
        })
        .unwrap();
    assert!(
        build(&d, &b)
            .unwrap()
            .checks
            .iter()
            .all(|c| c.environment == CheckStatus::Blocked)
    );
    // Reported source path alone cannot establish a target or a context link.
    d.facts.report_observations = normalized::Rows::new(&b);
    let foreign_q = AssertionQualification {
        context: nominal(65),
        ..q
    };
    d.core.qualifications.insert(foreign_q.clone()).unwrap();
    d.facts
        .report_observations
        .insert(TaskReportObservation {
            qualification: foreign_q.id(),
            ..row
        })
        .unwrap();
    assert!(
        build(&d, &b)
            .unwrap()
            .scenarios
            .iter()
            .all(|s| s.execution == CheckStatus::NotRun)
    );
}
#[test]
fn unittest_intent_requires_exact_provider_method_parent_and_defining_class() {
    let (b, mut d, source, q) = setup("test_case.py", b"with cm():\n    api()\n", SourceRole::Test);
    let (_, callable) = public_callable(&mut d, &source, &q);
    let with = occurrence(&mut d, &source, 0, source.byte_len, SyntaxKind::StmtWith);
    let item = occurrence(&mut d, &source, 1, 7, SyntaxKind::WithItem);
    let manager = occurrence(&mut d, &source, 2, 6, SyntaxKind::ExprCall);
    let call = occurrence(&mut d, &source, 15, 18, SyntaxKind::ExprCall);
    placement(&mut d, &q, item, with, SyntaxField::Item, 0);
    placement(&mut d, &q, manager, item, SyntaxField::Value, 0);
    placement(&mut d, &q, call, with, SyntaxField::Body, 0);
    event(&mut d, &source, &q, call, callable, 70);
    let module = d
        .core
        .provider_modules
        .insert(ProviderModule::Bundled {
            provider: nominal(71),
            bundle: ModuleBundle::Typeshed,
            name: "unittest.case".into(),
        })
        .unwrap();
    let class = d
        .core
        .symbols
        .insert(ProviderSymbol {
            provider: nominal(71),
            context: q.context,
            module,
            native_key: "unittest.case.TestCase".into(),
            name: "TestCase".into(),
            kind: SymbolKind::Class,
        })
        .unwrap();
    let method = d
        .core
        .symbols
        .insert(ProviderSymbol {
            provider: nominal(71),
            context: q.context,
            module,
            native_key: "unittest.case.TestCase.assertRaises".into(),
            name: "assertRaises".into(),
            kind: SymbolKind::Method,
        })
        .unwrap();
    let symbol_observation = symbols::SymbolObservation {
        qualification: q.id(),
        symbol: method,
        parent: Some(class),
    };
    d.facts
        .symbol_observations
        .insert(symbol_observation.clone())
        .unwrap();
    let traits = symbols::FunctionTraitObservation {
        qualification: q.id(),
        symbol: method,
        overload: false,
        staticmethod: false,
        classmethod: false,
        property_getter: false,
        property_setter: false,
        stub: true,
        origin: symbols::FunctionOrigin::DefStatement,
        defining_class: Some(class),
        overrides: None,
    };
    d.core.traits.insert(traits.clone()).unwrap();
    let entity = d
        .core
        .refs
        .insert(EntityRef::Callable {
            callable: nominal(72),
        })
        .unwrap();
    d.core
        .resolutions
        .insert(SymbolEntityResolution {
            symbol: method,
            context: q.context,
            policy: ContentHash::of(b"p"),
            status: ResolutionStatus::Resolved,
            entity: Some(entity),
            reason: EntityReason::ProviderExternal,
        })
        .unwrap();
    let destination = d
        .facts
        .destinations
        .insert(CallDestination::Resolved { symbol: method })
        .unwrap();
    d.facts
        .raw_targets
        .insert(CallTarget {
            qualification: q.id(),
            site: manager,
            origin: nominal(73),
            destination,
            channel: CallChannel::Direct {}.id(),
            phase: CallPhase::Call,
            receiver: Receiver::None {}.id(),
            implicit: false,
            receiver_class: None,
            passing: None,
            class_method: None,
            static_method: None,
        })
        .unwrap();
    assert!(
        build(&d, &b)
            .unwrap()
            .associations
            .iter()
            .all(|a| a.intent == Intent::ExpectedFailure)
    );
    let other = d
        .core
        .symbols
        .insert(ProviderSymbol {
            provider: nominal(71),
            context: q.context,
            module,
            native_key: "unittest.case.Other".into(),
            name: "Other".into(),
            kind: SymbolKind::Class,
        })
        .unwrap();
    d.facts.symbol_observations = normalized::Rows::new(&b);
    d.facts
        .symbol_observations
        .insert(symbols::SymbolObservation {
            parent: Some(other),
            ..symbol_observation
        })
        .unwrap();
    d.core.traits = normalized::Rows::new(&b);
    d.core
        .traits
        .insert(symbols::FunctionTraitObservation {
            defining_class: Some(other),
            ..traits
        })
        .unwrap();
    assert!(
        build(&d, &b)
            .unwrap()
            .associations
            .iter()
            .all(|a| a.intent == Intent::AssertionTest)
    );
}
#[test]
fn empty_evidence_cannot_erase_the_native_invocation_domain() {
    let b = ResourceBudget::fixed(8 << 20).unwrap();
    let mut runs = normalized::Rows::new(&b);
    let run = ProviderRun {
        provider: nominal(80),
        context: nominal(81),
        input: nominal(82),
        configuration: ContentHash::of(b"c"),
        requested_families: ContentHash::of(b"r"),
    };
    runs.insert(run.clone()).unwrap();
    let mut core = normalized::Rows::new(&b);
    let parent = analysis::catalog_core::Invocation::new(
        run.input,
        run.context,
        catalog::build::definition().1.id(),
        None,
        [],
    )
    .0;
    core.insert(parent.clone()).unwrap();
    // Pure frame inventory assumes canonical earlier owners were validated separately.
    let mut lower = runtime::RuntimeData::new(&b);
    let local = analysis::local::Invocation::new(
        run.input,
        run.context,
        local_semantics::definition().1.id(),
        None,
        [],
    )
    .0;
    lower
        .local_outcomes
        .insert(analysis::local::AnalysisOutcome {
            invocation: local.id(),
            status: analysis::AnalysisStatus::NotRequested,
            reason: Some(obligation::ObligationKind::NotRequested),
        })
        .unwrap();
    lower.local_invocations.insert(local).unwrap();
    let source = analysis::source_call::Invocation::new(
        run.input,
        run.context,
        execution::configuration::source_calls().1.id(),
        None,
        [],
    )
    .0;
    lower
        .source_outcomes
        .insert(analysis::source_call::AnalysisOutcome {
            invocation: source.id(),
            status: analysis::AnalysisStatus::NotRequested,
            reason: Some(obligation::ObligationKind::NotRequested),
        })
        .unwrap();
    lower.source_invocations.insert(source).unwrap();
    let mut invocations = normalized::Rows::new(&b);
    let mut sources = normalized::Rows::new(&b);
    let mut inputs = normalized::Rows::new(&b);
    assert!(
        frames::verify(
            &runs,
            &core,
            &invocations,
            &sources,
            &inputs,
            &lower.lower(),
            &b
        )
        .is_err()
    );
    let lower_sources = frames::sources(&parent, &lower.lower()).unwrap();
    let (invocation, links) = analysis::catalog_evidence::Invocation::new(
        run.input,
        run.context,
        definition().1.id(),
        None,
        lower_sources.iter().map(Record::id),
    );
    invocations.insert(invocation.clone()).unwrap();
    for source in lower_sources {
        sources.insert(source).unwrap();
    }
    for link in links {
        inputs.insert(link).unwrap();
    }
    frames::verify(
        &runs,
        &core,
        &invocations,
        &sources,
        &inputs,
        &lower.lower(),
        &b,
    )
    .unwrap();
    // A second native provider run in a new context cannot be hidden by an empty root set.
    runs.insert(ProviderRun {
        context: nominal(83),
        ..run
    })
    .unwrap();
    assert!(
        frames::verify(
            &runs,
            &core,
            &invocations,
            &sources,
            &inputs,
            &lower.lower(),
            &b
        )
        .is_err()
    );
}
#[test]
fn root_document_candidate_does_not_spread_to_nested_members_or_foreign_contexts() {
    let (b, mut d, source, q) = setup("guide.md", b"`C`", SourceRole::Document);
    let span = Evidence::SourceSpan {
        source: source.id(),
        start: 0,
        end: 3,
    };
    let span_id = EvidenceSourceSpanId::of(&span).unwrap();
    d.facts.canonical_evidence.insert(span).unwrap();
    let node = DocumentNode::Mention { span: span_id };
    let mention_node = DocumentNodeMentionId::of(&node).unwrap();
    d.facts.nodes.insert(node).unwrap();
    let passage = DocumentNode::Passage {
        span: span_id,
        ordinal: 0,
    };
    let passage_id = DocumentNodePassageId::of(&passage).unwrap();
    d.facts.nodes.insert(passage).unwrap();
    let mention = DocumentMentionObservation {
        qualification: q.id(),
        mention: mention_node,
        passage: passage_id,
        class: MentionClass::Exact,
        source: MentionSource::InlineCode,
        form: "C".into(),
        access_path: Some("api.C".into()),
        qualified_name: None,
    };
    let observation = d.facts.mentions.insert(mention.clone()).unwrap();
    let assessment = d
        .facts
        .mention_assessments
        .insert(MentionEntityAssessment {
            observation,
            status: ResolutionStatus::Resolved,
            reason: LinkReason::ExplicitIdentity,
        })
        .unwrap();
    let exposure = d
        .core
        .exposures
        .insert(PublicExposure {
            access: nominal(90),
            context: q.context,
            observation: nominal(91),
            origin: nominal(92),
            status: ResolutionStatus::Resolved,
            reason: EntityReason::DeclarationAgreement,
        })
        .unwrap();
    d.facts
        .mention_candidates
        .insert(MentionEntityCandidate {
            assessment,
            exposure,
        })
        .unwrap();
    let root = d
        .catalog
        .members
        .insert(CatalogMember {
            input: source.input,
            access: nominal(90),
            path: vec!["C".into()],
            name: "C".into(),
        })
        .unwrap();
    let nested = d
        .catalog
        .members
        .insert(CatalogMember {
            input: source.input,
            access: nominal(90),
            path: vec!["C".into(), "read_left".into()],
            name: "C.read_left".into(),
        })
        .unwrap();
    for member in [root, nested] {
        d.catalog
            .exposures
            .insert(CatalogExposure { member, exposure })
            .unwrap();
    }
    let output = build(&d, &b).unwrap();
    assert_eq!(output.document_associations.len(), 1);
    assert_eq!(
        output.document_associations.iter().next().unwrap().member,
        root
    );
    let foreign = AssertionQualification {
        context: nominal(93),
        ..q
    };
    d.core.qualifications.insert(foreign.clone()).unwrap();
    d.facts.mentions = normalized::Rows::new(&b);
    let observation = d
        .facts
        .mentions
        .insert(DocumentMentionObservation {
            qualification: foreign.id(),
            ..mention
        })
        .unwrap();
    d.facts.mention_assessments = normalized::Rows::new(&b);
    let assessment = d
        .facts
        .mention_assessments
        .insert(MentionEntityAssessment {
            observation,
            status: ResolutionStatus::Resolved,
            reason: LinkReason::ExplicitIdentity,
        })
        .unwrap();
    d.facts.mention_candidates = normalized::Rows::new(&b);
    d.facts
        .mention_candidates
        .insert(MentionEntityCandidate {
            assessment,
            exposure,
        })
        .unwrap();
    assert_eq!(build(&d, &b).unwrap().document_associations.len(), 0);
}

#[test]
fn admitted_public_members_keep_original_artifacts_without_a_scenario_role() {
    let (b, mut d, source, q) = setup("dependency.py", b"def api(): pass", SourceRole::Dependency);
    public_callable(&mut d, &source, &q);
    let output = build(&d, &b).unwrap();
    assert_eq!(output.scenarios.len(), 0);
    assert_eq!(output.original_sources.len(), 1);
    assert_eq!(
        output.original_sources.iter().next().unwrap(),
        &OriginalSource::Artifact {
            artifact: source.id()
        }
    );
    // The shared publication replay, not a test-only validator, rejects an omitted wrapper.
    for forged in [false, true] {
        let invariant = catalog::evidence::build::invariants().remove(0);
        let mut check = (invariant.create)(&b);
        macro_rules! core {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.core.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::catalog_inputs!(core);
        macro_rules! catalog {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.catalog.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::catalog_outputs!(catalog);
        macro_rules! facts {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.facts.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::catalog_evidence_inputs!(facts);
        macro_rules! result {($($f:ident:$ty:ty,)*)=>{$(if <$ty>::NAME!=OriginalSource::NAME {check.visit(<$ty>::NAME,&<$ty as Record>::encode(&output.$f.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();})*};}
        lctx_model::catalog_evidence_outputs!(result);
        if forged {
            let other = OriginalSource::Artifact {
                artifact: nominal(101),
            };
            check
                .visit(
                    OriginalSource::NAME,
                    &<OriginalSource as Record>::encode(&[other]).unwrap(),
                )
                .unwrap();
        }
        assert!(check.finish().is_err());
    }
}
#[test]
fn normalized_constructor_candidate_keeps_own_public_slot_and_never_claims_state() {
    use lctx_model::domain::normalized::{bindings::*, signature_applicability::*};
    let (b, mut d, source, q) = setup("example.py", b"make()", SourceRole::Example);
    let (member, callee) = public_callable(&mut d, &source, &q);
    let site = occurrence(&mut d, &source, 0, 6, SyntaxKind::ExprCall);
    let event = event(&mut d, &source, &q, site, callee, 140);
    let alternative = d.facts.alternatives.iter().next().unwrap().clone();
    let caller = d
        .core
        .ownership
        .get(d.facts.events.get(event).unwrap().owner)
        .unwrap()
        .entity;
    let attempt = d
        .facts
        .attempts
        .insert(CallBindingAttempt {
            alternative: alternative.id(),
            variant: None,
            syntax: None,
            policy: ContentHash::of(b"earlier-owner-policy"),
            event,
            signature: None,
            arguments: None,
            receiver: Receiver::None {}.id(),
            receiver_assessment: None,
            dispatch_member: None,
            effective: None,
            adjustment: SignatureAdjustment::Unknown,
            authority: BindingAuthority::SourceInspection,
            authority_reason: AuthorityReason::SignatureUnknown,
            outcome: BindingOutcome::Undetermined,
            reason: BindingReason::MissingSignature,
            refusal: None,
            bindings: ContentHash::of(b"earlier-owner-bindings"),
        })
        .unwrap();
    let original = d.catalog.callables.iter().next().unwrap().clone();
    let cls = d
        .core
        .source_classes
        .insert(ClassEntity::Source {
            declaration: nominal(141),
        })
        .unwrap();
    let alias = d
        .catalog
        .members
        .insert(CatalogMember {
            path: vec!["alias".into()],
            name: "alias".into(),
            ..d.catalog.members.get(member).unwrap().clone()
        })
        .unwrap();
    let inv = d.facts.core_invocations.iter().next().unwrap().id();
    d.facts
        .core_links
        .insert(CatalogMemberInvocation {
            member: alias,
            invocation: inv,
        })
        .unwrap();
    let alias_callable = d
        .catalog
        .callables
        .insert(CatalogCallable {
            member: alias,
            ..original.clone()
        })
        .unwrap();
    for (slot, callable, candidate) in [
        (member, original.id(), original.candidate),
        (alias, alias_callable, nominal(142)),
    ] {
        let class = d
            .catalog
            .classes
            .insert(CatalogClass {
                member: slot,
                candidate,
                class: cls,
            })
            .unwrap();
        d.catalog
            .constructors
            .insert(CatalogConstructor {
                class,
                callable,
                traits: nominal(143),
                ancestry: None,
                origin: ConstructorOrigin::Own,
                kind: ConstructorKind::Init,
                applicability: Knowledge::Unknown,
                disposition: ConstructorDisposition::Candidate,
            })
            .unwrap();
    }
    // Pure C1 candidate admission assumes earlier normalized target/correspondence authority.
    let raw = d.facts.raw_targets.iter().next().unwrap().clone();
    d.facts.raw_targets = normalized::Rows::new(&b);
    let target = d
        .facts
        .raw_targets
        .insert(CallTarget {
            phase: CallPhase::Init,
            receiver_class: Some(nominal(144)),
            ..raw
        })
        .unwrap();
    let alternative_source = d
        .facts
        .alternative_sources
        .insert(CallAlternativeSource::Native { target })
        .unwrap();
    d.facts.alternatives = normalized::Rows::new(&b);
    let new_alternative = d
        .facts
        .alternatives
        .insert(NormalizedCallAlternative {
            source: alternative_source,
            ..alternative
        })
        .unwrap();
    let prior = d.facts.attempts.get(attempt).unwrap().clone();
    d.facts.attempts = normalized::Rows::new(&b);
    d.facts
        .attempts
        .insert(CallBindingAttempt {
            alternative: new_alternative,
            ..prior
        })
        .unwrap();
    d.core
        .resolutions
        .insert(SymbolEntityResolution {
            symbol: nominal(144),
            context: q.context,
            policy: ContentHash::of(b"earlier-correspondence-owner"),
            status: ResolutionStatus::Resolved,
            reason: EntityReason::DeclarationAgreement,
            entity: Some(EntityRef::Class { class: cls }.id()),
        })
        .unwrap();
    let output = build(&d, &b).unwrap();
    assert_eq!(output.constructor_candidates.len(), 2);
    for link in output.constructor_candidates.iter() {
        let association = output.associations.get(link.association).unwrap();
        let constructor = d.catalog.constructors.get(link.constructor).unwrap();
        assert_eq!(
            association.member,
            d.catalog.classes.get(constructor.class).unwrap().member
        );
        assert_eq!(
            association.member,
            d.catalog
                .callables
                .get(constructor.callable)
                .unwrap()
                .member
        );
        assert_eq!(link.phase, CallPhase::Init);
        assert_eq!(link.applicability, Knowledge::Unknown);
        assert_eq!(
            link.state,
            obligation::ObligationKind::HeapFieldStateUnavailable
        );
    }
    let prior = d.facts.attempts.iter().next().unwrap().clone();
    for wrong in 0..2 {
        d.facts.attempts = normalized::Rows::new(&b);
        d.facts
            .attempts
            .insert(CallBindingAttempt {
                receiver: if wrong == 0 {
                    nominal(145)
                } else {
                    prior.receiver
                },
                effective: if wrong == 1 {
                    Some(nominal(146))
                } else {
                    prior.effective
                },
                ..prior.clone()
            })
            .unwrap();
        assert!(build(&d, &b).unwrap().constructor_candidates.is_empty());
    }
}
