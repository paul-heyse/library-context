//! Hand-expected N2 correspondence follows captured CorpusLibrary identity only.
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    conditions::Diagram,
    documents::*,
    input::*,
    normalized::{
        entities::*,
        links::*,
        relation_normalization::{self, RelationData, RelationOutput},
    },
    resources::ResourceBudget,
    source::*,
    symbols::*,
    *,
};
fn nominal<T>(value: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([value; 16].into_iter()))
    .unwrap()
}
fn captured(label: &str) -> Id<InputRevision> {
    InputRevision::from_entries(vec![ManifestEntry {
        path: format!("{label}.py"),
        content: ContentHash::of(label.as_bytes()),
        byte_len: label.len() as i64,
    }])
    .unwrap()
    .id()
}
fn context(label: &str) -> Id<AnalysisContext> {
    AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(label.as_bytes()),
        environment_digest: ContentHash::of(b"same-environment-does-not-link-inputs"),
        lock_digest: None,
    }
    .id()
}
fn qualification(
    data: &mut RelationData,
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
) -> Id<AssertionQualification> {
    let scope = data.scopes.insert(CoverageScope::Input { input }).unwrap();
    let (condition, _) = Diagram::always().records();
    data.facts
        .qualifications
        .insert(AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context,
            scope,
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        })
        .unwrap()
}
fn mention(
    data: &mut RelationData,
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
) -> Id<DocumentMentionObservation> {
    let source = SourceArtifact::from_bytes(input, "guide.md".into(), b"api.Widget").unwrap();
    let span = EvidenceSourceSpanId::of(&Evidence::SourceSpan {
        source: source.id(),
        start: 0,
        end: 10,
    })
    .unwrap();
    data.artifacts.insert(source).unwrap();
    let qualification = qualification(data, input, context);
    data.mentions
        .insert(DocumentMentionObservation {
            qualification,
            mention: DocumentNodeMentionId::of(&DocumentNode::Mention { span }).unwrap(),
            passage: DocumentNodePassageId::of(&DocumentNode::Passage { span, ordinal: 0 })
                .unwrap(),
            class: MentionClass::Exact,
            source: MentionSource::InlineCode,
            form: "api.Widget".into(),
            access_path: Some("api.Widget".into()),
            qualified_name: Some("api.Widget".into()),
        })
        .unwrap()
}
fn target(
    data: &mut RelationData,
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
    marker: u8,
    unresolved: bool,
) -> (
    Id<PublicExposure>,
    Id<SymbolObservation>,
    Id<SymbolEntityResolution>,
) {
    let source =
        SourceArtifact::from_bytes(input, format!("api{marker}.py"), b"class Widget: pass")
            .unwrap();
    let module = data
        .facts
        .modules
        .insert(Module {
            source: source.id(),
            qualified_name: "api".into(),
        })
        .unwrap();
    data.artifacts.insert(source).unwrap();
    let provider_module = data
        .facts
        .provider_modules
        .insert(ProviderModule::Acquired { module })
        .unwrap();
    let symbol = data
        .facts
        .symbols
        .insert(ProviderSymbol {
            provider: nominal(marker),
            context,
            module: provider_module,
            native_key: format!("target-{marker}"),
            name: "Widget".into(),
            kind: SymbolKind::Class,
        })
        .unwrap();
    let qualification = qualification(data, input, context);
    let observation = data
        .symbol_observations
        .insert(SymbolObservation {
            qualification,
            symbol,
            parent: None,
        })
        .unwrap();
    let entity = data
        .entities
        .refs
        .insert(EntityRef::Class {
            class: ClassEntity::External { symbol }.id(),
        })
        .unwrap();
    let status = if unresolved {
        ResolutionStatus::Unresolved
    } else {
        ResolutionStatus::Resolved
    };
    let resolution = data
        .entities
        .resolutions
        .insert(SymbolEntityResolution {
            symbol,
            context,
            policy: ContentHash::of(b"pure-target"),
            status,
            entity: (!unresolved).then_some(entity),
            reason: if unresolved {
                EntityReason::MissingCorrespondence
            } else {
                EntityReason::ProviderExternal
            },
        })
        .unwrap();
    // An unresolved candidate may retain alternatives; they never establish a resolved answer.
    data.entities
        .candidates
        .insert(SymbolEntityCandidate { resolution, entity })
        .unwrap();
    let origin = data
        .facts
        .export_origins
        .insert(ExportOrigin::Traced {
            module: provider_module,
            name: "Widget".into(),
            kind: None,
        })
        .unwrap();
    let public_name = data
        .facts
        .public_names
        .insert(PublicNameObservation {
            qualification,
            access: module,
            name: "Widget".into(),
            via_dunder_all: false,
            origin,
        })
        .unwrap();
    let exposure = data
        .entities
        .exposures
        .insert(PublicExposure {
            access: module,
            context,
            observation: public_name,
            origin,
            enumeration: None,
            publicity: PublicPathKnowledge::Known,
            status,
            reason: EntityReason::DeclarationAgreement,
        })
        .unwrap();
    data.entities
        .exposure_candidates
        .insert(PublicExposureCandidate {
            exposure,
            resolution,
            support: nominal(marker),
        })
        .unwrap();
    (exposure, observation, resolution)
}
fn answer(
    output: &RelationOutput,
    observation: Id<DocumentMentionObservation>,
) -> &MentionEntityAssessment {
    output
        .mention_entity_assessments
        .iter()
        .find(|row| row.observation == observation)
        .unwrap()
}
#[test]
fn explicit_corpus_link_resolves_original_exposure_and_symbol_in_distinct_context() {
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    let corpus = captured("corpus");
    let library = captured("installed");
    let corpus_context = context("corpus-context");
    let library_context = context("installed-context");
    assert_ne!(corpus_context, library_context);
    let mention = mention(&mut data, corpus, corpus_context);
    let (exposure, observation, resolution) = target(&mut data, library, library_context, 1, false);
    data.corpus_libraries
        .insert(CorpusLibrary { corpus, library })
        .unwrap();
    let output = relation_normalization::normalize(&data, &budget).unwrap();
    let assessment = answer(&output, mention);
    assert_eq!(assessment.status, ResolutionStatus::Resolved);
    assert_eq!(output.mention_entity_candidates.len(), 1);
    assert_eq!(
        output
            .mention_entity_candidates
            .iter()
            .next()
            .unwrap()
            .exposure,
        exposure
    );
    let candidate = output.mention_symbol_candidates.iter().next().unwrap();
    assert_eq!(
        (candidate.observation, candidate.resolution),
        (observation, resolution)
    );
    assert_eq!(
        data.entities.resolutions.get(resolution).unwrap().context,
        library_context
    );
    assert_eq!(
        data.entities.exposures.get(exposure).unwrap().context,
        library_context
    );
}
#[test]
fn missing_wrong_and_reverse_links_do_not_resolve_equal_paths_or_environment() {
    for link in [
        None,
        Some((captured("corpus"), captured("other"))),
        Some((captured("installed"), captured("corpus"))),
    ] {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let mut data = RelationData::new(&budget);
        let mention = mention(&mut data, captured("corpus"), context("corpus-context"));
        target(
            &mut data,
            captured("installed"),
            context("installed-context"),
            1,
            false,
        );
        if let Some((corpus, library)) = link {
            data.corpus_libraries
                .insert(CorpusLibrary { corpus, library })
                .unwrap();
        }
        let output = relation_normalization::normalize(&data, &budget).unwrap();
        assert_eq!(
            answer(&output, mention).status,
            ResolutionStatus::Unresolved
        );
        assert_eq!(output.mention_entity_candidates.len(), 0);
        assert_eq!(output.mention_symbol_candidates.len(), 0);
    }
}
#[test]
fn same_input_requires_exact_context_even_with_a_self_link() {
    for self_link in [false, true] {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let mut data = RelationData::new(&budget);
        let input = captured("installed");
        let mention = mention(&mut data, input, context("foreign-context"));
        target(&mut data, input, context("installed-context"), 1, false);
        if self_link {
            data.corpus_libraries
                .insert(CorpusLibrary {
                    corpus: input,
                    library: input,
                })
                .unwrap();
        }
        let output = relation_normalization::normalize(&data, &budget).unwrap();
        assert_eq!(
            answer(&output, mention).status,
            ResolutionStatus::Unresolved
        );
        assert_eq!(output.mention_entity_candidates.len(), 0);
        assert_eq!(output.mention_symbol_candidates.len(), 0);
    }
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    let input = captured("installed");
    let mention = mention(&mut data, input, context("installed-context"));
    target(&mut data, input, context("installed-context"), 1, false);
    assert_eq!(
        answer(
            &relation_normalization::normalize(&data, &budget).unwrap(),
            mention
        )
        .status,
        ResolutionStatus::Resolved
    );
}
#[test]
fn distinct_linked_targets_and_entities_are_retained_as_ambiguous() {
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    let corpus = captured("corpus");
    let mention = mention(&mut data, corpus, context("corpus-context"));
    for (library, marker) in [
        (captured("first-library"), 1),
        (captured("second-library"), 2),
    ] {
        data.corpus_libraries
            .insert(CorpusLibrary { corpus, library })
            .unwrap();
        target(
            &mut data,
            library,
            context("installed-context"),
            marker,
            false,
        );
    }
    let output = relation_normalization::normalize(&data, &budget).unwrap();
    assert_eq!(answer(&output, mention).status, ResolutionStatus::Ambiguous);
    assert_eq!(output.mention_entity_candidates.len(), 2);
    assert_eq!(output.mention_symbol_candidates.len(), 2);
}
#[test]
fn unresolved_linked_candidates_never_become_resolved() {
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    let corpus = captured("corpus");
    let library = captured("installed");
    let mention = mention(&mut data, corpus, context("corpus-context"));
    data.corpus_libraries
        .insert(CorpusLibrary { corpus, library })
        .unwrap();
    target(&mut data, library, context("installed-context"), 1, true);
    let output = relation_normalization::normalize(&data, &budget).unwrap();
    assert_eq!(
        answer(&output, mention).status,
        ResolutionStatus::Unresolved
    );
    assert_eq!(output.mention_entity_candidates.len(), 1);
    assert_eq!(output.mention_symbol_candidates.len(), 1);
}
#[test]
fn corpus_links_are_complete_replay_premises_and_acknowledged_metadata_inputs() {
    assert!(
        RelationData::validation_inputs()
            .iter()
            .any(|input| input.name() == CorpusLibrary::NAME)
    );
    assert!(
        lctx_model::domain::normalized::binding_normalization::BindingData::validation_inputs()
            .iter()
            .any(|input| input.name() == CorpusLibrary::NAME),
        "N5 upstream replay retains every N2 premise"
    );
    for profile in [stages::Profile::Catalog, stages::Profile::Behavioral] {
        let stage = relation_normalization::stage(profile);
        let input = stage
            .inputs
            .iter()
            .find(|input| input.name() == CorpusLibrary::NAME)
            .unwrap();
        assert_eq!(input.transport(), stages::InputTransport::CompletedStore);
        assert_eq!(
            input.requirement(),
            None,
            "acquisition metadata is not a provider fact family"
        );
    }
    let budget = ResourceBudget::fixed(16 << 20).unwrap();
    let mut data = RelationData::new(&budget);
    let row = CorpusLibrary {
        corpus: captured("corpus"),
        library: captured("installed"),
    };
    assert!(
        data.visit(
            CorpusLibrary::NAME,
            &CorpusLibrary::encode(std::slice::from_ref(&row)).unwrap()
        )
        .unwrap()
    );
    assert_eq!(data.corpus_libraries.get(row.id()), Some(&row));
}
