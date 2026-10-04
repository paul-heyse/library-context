//! Hand-expected retrieval contracts: lexical preparation is independent of services and ranking.
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    catalog::{self, evidence as c1},
    deployment::*,
    documents::*,
    input::*,
    normalized::Rows,
    resources::ResourceBudget,
    retrieval::{
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
fn artifact(d: &mut Data, input: Id<InputRevision>, path: &str, bytes: &[u8]) -> SourceArtifact {
    let a = SourceArtifact::from_bytes(input, path.into(), bytes).unwrap();
    d.source.core.artifacts.insert(a.clone()).unwrap();
    for c in artifact::ArtifactChunk::split(&a, bytes).unwrap() {
        d.facts.chunks.insert(c).unwrap();
    }
    a
}
fn root(
    d: &mut Data,
    input: Id<InputRevision>,
    context: Id<AnalysisContext>,
    subject: c1::RootSubject,
) {
    let subject = d.evidence.subjects.insert(subject).unwrap();
    d.evidence
        .roots
        .insert(c1::EvidenceRoot {
            input,
            context,
            subject,
        })
        .unwrap();
}
fn fixture() -> (
    ResourceBudget,
    Data,
    Id<catalog::CatalogMember>,
    SourceArtifact,
) {
    let b = ResourceBudget::fixed(64 << 20).unwrap();
    let mut d = Data::new(&b);
    d.facts
        .definitions
        .insert(Definition::builtin(false))
        .unwrap();
    let a = artifact(&mut d, id(1), "api.py", b"def run():\n    pass\n");
    let module = d
        .source
        .core
        .modules
        .insert(Module {
            source: a.id(),
            qualified_name: "pkg.api".into(),
        })
        .unwrap();
    let m = d
        .source
        .catalog
        .members
        .insert(catalog::CatalogMember {
            input: a.input,
            access: module,
            path: vec!["run".into()],
            name: "run".into(),
        })
        .unwrap();
    d.evidence
        .original_sources
        .insert(c1::OriginalSource::Artifact { artifact: a.id() })
        .unwrap();
    root(
        &mut d,
        a.input,
        id(2),
        c1::RootSubject::Member { member: m },
    );
    (b, d, m, a)
}
#[test]
fn distinct_occurrences_deduplicate_text_without_losing_members_or_anchors() {
    let (b, mut d, m, a) = fixture();
    let alias = d
        .source
        .catalog
        .members
        .insert(catalog::CatalogMember {
            path: vec!["alias".into()],
            name: "alias".into(),
            ..d.source.catalog.members.get(m).unwrap().clone()
        })
        .unwrap();
    root(
        &mut d,
        a.input,
        id(2),
        c1::RootSubject::Member { member: alias },
    );
    let out = retrieval::build::build(&d, &b).unwrap();
    assert_eq!(
        out.units.len(),
        3,
        "source occurrence merges member roots while API slots remain distinct"
    );
    assert_eq!(out.corpus.len(), 3);
    let source = out
        .units
        .iter()
        .find(|r| r.family == Family::Source)
        .unwrap();
    assert_eq!(
        out.unit_subjects
            .iter()
            .filter(|r| r.unit == source.id())
            .count(),
        2
    );
    assert_eq!(
        out.roots.iter().filter(|r| r.unit == source.id()).count(),
        2
    );
    assert_eq!(
        out.anchors.iter().filter(|r| r.unit == source.id()).count(),
        1
    );
    assert!(
        out.corpus
            .iter()
            .any(|r| r.text.as_str() == "def run():\n    pass\n")
    );
    // The same source in another exact analysis context remains another occurrence of one corpus.
    root(
        &mut d,
        a.input,
        id(3),
        c1::RootSubject::Member { member: m },
    );
    let next = retrieval::build::build(&d, &b).unwrap();
    assert_eq!(next.units.len(), 5);
    assert_eq!(next.corpus.len(), 3);
    assert_eq!(next.fragments.len(), 3);
}
#[test]
fn unicode_fragments_are_canonical_contiguous_and_addressable() {
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let definition = Definition {
        fragment_bytes: 5,
        ..Definition::builtin(false)
    };
    let text = "hello🦀worldλ";
    let corpus = CorpusText {
        family: Family::Source,
        rendering_version: RENDER_VERSION,
        digest: ContentHash::of(text.as_bytes()),
        text: Utf8Text::from(text),
    };
    let mut out = Rows::new(&b);
    retrieval::build::fragments(&definition, &corpus, &mut out, &b).unwrap();
    let mut rows = out.iter().collect::<Vec<_>>();
    rows.sort_by_key(|r| r.ordinal);
    assert_eq!(
        rows.iter().map(|r| r.text.as_str()).collect::<String>(),
        text
    );
    assert_eq!(rows[0].start, 0);
    for pair in rows.windows(2) {
        assert_eq!(pair[0].end, pair[1].start);
    }
    assert!(rows.iter().all(|r| r.text.as_str().len() <= 5));
    let mut again = Rows::new(&b);
    retrieval::build::fragments(&definition, &corpus, &mut again, &b).unwrap();
    assert!(out.same(&again));
    let mut empty = Rows::new(&b);
    let corpus = CorpusText {
        digest: ContentHash::of(b""),
        text: Utf8Text::from(""),
        ..corpus
    };
    retrieval::build::fragments(&definition, &corpus, &mut empty, &b).unwrap();
    assert!(empty.is_empty());
}
#[test]
fn vector_selection_and_specification_do_not_change_lexical_fragment_identity() {
    let b = ResourceBudget::fixed(1 << 20).unwrap();
    let text = "same lexical text";
    let corpus = CorpusText {
        family: Family::Source,
        rendering_version: RENDER_VERSION,
        digest: ContentHash::of(text.as_bytes()),
        text: text.into(),
    };
    let mut ids = vec![];
    for selected in [false, true] {
        let mut rows = Rows::new(&b);
        retrieval::build::fragments(&Definition::builtin(selected), &corpus, &mut rows, &b)
            .unwrap();
        ids.push(rows.iter().next().unwrap().id());
    }
    assert_eq!(ids[0], ids[1]);
}
#[test]
fn all_four_families_preserve_original_setup_execution_and_release_scope() {
    let (b, mut d, _, a) = fixture();
    let q = d
        .source
        .core
        .qualifications
        .insert(AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: id(2),
            scope: CoverageScope::Artifact { artifact: a.id() }.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        })
        .unwrap();
    let original = c1::OriginalSource::Artifact { artifact: a.id() }.id();
    let source = d
        .evidence
        .scenario_sources
        .insert(c1::ScenarioSource::Python {
            artifact: a.id(),
            role: SourceRole::Example,
            context: id(2),
            declaration: None,
        })
        .unwrap();
    let scenario = d
        .evidence
        .scenarios
        .insert(c1::CatalogScenario {
            source,
            extraction: CheckStatus::Passed,
            parse: CheckStatus::Passed,
            binding: CheckStatus::Blocked,
            environment: CheckStatus::Blocked,
            execution: CheckStatus::NotRun,
            intent: c1::Intent::Demonstration,
        })
        .unwrap();
    d.evidence
        .spans
        .insert(c1::ScenarioSpan {
            scenario,
            ordinal: 0,
            role: c1::SpanRole::EnclosingModule,
            source: original,
        })
        .unwrap();
    let dependency = d
        .evidence
        .setup
        .insert(c1::SetupDependency::RuntimeInputs { artifact: a.id() })
        .unwrap();
    d.evidence
        .dependencies
        .insert(c1::ScenarioDependency {
            scenario,
            dependency,
            status: CheckStatus::Blocked,
        })
        .unwrap();
    root(
        &mut d,
        a.input,
        id(2),
        c1::RootSubject::Scenario { scenario },
    );
    let doc = artifact(
        &mut d,
        a.input,
        "guide.mdx",
        b"# Setup\nwith prepare():\n    run()\n",
    );
    let span = Evidence::SourceSpan {
        source: doc.id(),
        start: 0,
        end: doc.byte_len,
    };
    let span_id = EvidenceSourceSpanId::of(&span).unwrap();
    d.source.facts.canonical_evidence.insert(span).unwrap();
    let node = DocumentNode::Passage {
        span: span_id,
        ordinal: 0,
    };
    let passage = DocumentNodePassageId::of(&node).unwrap();
    d.source.facts.nodes.insert(node).unwrap();
    d.source
        .facts
        .passages
        .insert(PassageObservation {
            qualification: q,
            passage,
            level: 1,
            heading: Some("Setup".into()),
            heading_path: vec![],
            text: "provider interpretation must not replace original bytes".into(),
        })
        .unwrap();
    let observation = d
        .source
        .facts
        .documents
        .insert(DocumentObservation {
            qualification: q,
            source: doc.id(),
            title: Some("Guide".into()),
            parsed: true,
        })
        .unwrap();
    root(
        &mut d,
        doc.input,
        id(2),
        c1::RootSubject::Document { observation },
    );
    let observation = d
        .source
        .facts
        .deployment
        .insert(DeploymentObservation {
            qualification: q,
            span: span_id,
            ordinal: 0,
            distribution: Some("pkg".into()),
            version: Some("1".into()),
            field: "requires".into(),
            original: "interpreted text".into(),
            name: Some("dep".into()),
            extras: vec![],
            marker: None,
            constraint: None,
            interpretation: CheckStatus::Blocked,
            diagnostic: None,
            environment_digest: None,
            lock_digest: None,
            referenced_path: None,
        })
        .unwrap();
    let deployment = d
        .evidence
        .deployments
        .insert(c1::CatalogDeployment { observation })
        .unwrap();
    d.evidence
        .release_deployments
        .insert(c1::ReleaseDeployment {
            deployment,
            ownership: id(9),
            release: id(10),
        })
        .unwrap();
    root(
        &mut d,
        a.input,
        id(2),
        c1::RootSubject::Deployment { deployment },
    );
    let out = retrieval::build::build(&d, &b).unwrap();
    for family in [
        Family::ApiOptions,
        Family::Source,
        Family::Scenario,
        Family::DocumentationDeployment,
    ] {
        assert!(out.units.iter().any(|r| r.family == family));
    }
    assert!(
        out.corpus
            .iter()
            .any(|r| r.text.as_str().contains("execution=NotRun")
                && r.text.as_str().contains("Setup RuntimeInputs"))
    );
    assert!(
        out.corpus
            .iter()
            .any(|r| r.text.as_str() == "# Setup\nwith prepare():\n    run()\n")
    );
    assert!(
        !out.corpus
            .iter()
            .any(|r| r.text.as_str().contains("provider interpretation"))
    );
    assert_eq!(
        out.subjects
            .iter()
            .filter(|r| matches!(r, retrieval::Subject::Release { .. }))
            .count(),
        1
    );
    assert!(
        out.units
            .iter()
            .filter(|r| r.family == Family::DocumentationDeployment)
            .any(|u| !out.unit_subjects.iter().any(|s| s.unit == u.id()))
    );
}
fn replay(d: &Data, out: &Output, b: &ResourceBudget) -> Result<(), ModelError> {
    let mut check = (retrieval::build::invariants().remove(0).create)(b);
    macro_rules! core {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.source.core.$f.iter().cloned().collect::<Vec<_>>())?).unwrap();)*};}
    lctx_model::catalog_inputs!(core);
    macro_rules! catalog {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.source.catalog.$f.iter().cloned().collect::<Vec<_>>())?).unwrap();)*};}
    lctx_model::catalog_outputs!(catalog);
    macro_rules! facts {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.source.facts.$f.iter().cloned().collect::<Vec<_>>())?).unwrap();)*};}
    lctx_model::catalog_evidence_inputs!(facts);
    macro_rules! evidence {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.evidence.$f.iter().cloned().collect::<Vec<_>>())?).unwrap();)*};}
    lctx_model::catalog_evidence_outputs!(evidence);
    macro_rules! extra {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&d.facts.$f.iter().cloned().collect::<Vec<_>>())?).unwrap();)*};}
    lctx_model::retrieval_inputs!(extra);
    macro_rules! output {($($f:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&out.$f.iter().cloned().collect::<Vec<_>>())?).unwrap();)*};}
    lctx_model::retrieval_outputs!(output);
    check.finish()
}
#[test]
fn replay_refuses_forged_text_subjects_anchors_or_coupled_erasure() {
    let (b, d, _, _) = fixture();
    let out = retrieval::build::build(&d, &b).unwrap();
    replay(&d, &out, &b).unwrap();
    for case in 0..4 {
        let mut out = retrieval::build::build(&d, &b).unwrap();
        match case {
            0 => {
                out.units = Rows::new(&b);
                out.corpus = Rows::new(&b);
                out.fragments = Rows::new(&b);
                out.unit_subjects = Rows::new(&b);
                out.roots = Rows::new(&b);
                out.anchors = Rows::new(&b);
            }
            1 => {
                out.unit_subjects = Rows::new(&b);
            }
            2 => {
                let mut row = out.corpus.iter().next().unwrap().clone();
                row.text = Utf8Text::from(format!("{} forged", row.text));
                row.digest = ContentHash::of(row.text.as_str().as_bytes());
                out.corpus = Rows::new(&b);
                out.corpus.insert(row).unwrap();
            }
            _ => {
                out.anchors = Rows::new(&b);
            }
        }
        assert!(replay(&d, &out, &b).is_err());
    }
}
#[test]
fn authored_definition_original_bytes_and_resource_bounds_are_mandatory() {
    let (b, mut d, _, _) = fixture();
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        retrieval::build::build(&d, &tiny),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(tiny.reserved(), 0);
    d.facts.chunks = Rows::new(&b);
    assert!(retrieval::build::build(&d, &b).is_err());
    d.facts.definitions = Rows::new(&b);
    assert!(retrieval::build::build(&d, &b).is_err());
}

#[test]
fn nested_public_slots_have_distinct_full_paths_in_retrieval_text() {
    let (b, mut d, _, _) = fixture();
    let parent = d.source.catalog.members.iter().next().unwrap().clone();
    let original_root = d.evidence.roots.iter().next().unwrap().clone();
    for path in [
        vec!["Outer".into(), "run".into()],
        vec!["Other".into(), "run".into()],
    ] {
        let member = d
            .source
            .catalog
            .members
            .insert(catalog::CatalogMember {
                name: path.join("."),
                path,
                ..parent.clone()
            })
            .unwrap();
        let subject = d
            .evidence
            .subjects
            .insert(c1::RootSubject::Member { member })
            .unwrap();
        d.evidence
            .roots
            .insert(c1::EvidenceRoot {
                subject,
                ..original_root.clone()
            })
            .unwrap();
    }
    let out = retrieval::build::build(&d, &b).unwrap();
    assert!(
        out.units
            .iter()
            .any(|r| r.title.as_str().ends_with(".Outer.run"))
    );
    assert!(
        out.units
            .iter()
            .any(|r| r.title.as_str().ends_with(".Other.run"))
    );
    assert!(
        out.corpus
            .iter()
            .any(|r| r.text.as_str().contains(".Outer.run\n"))
    );
    assert!(
        out.corpus
            .iter()
            .any(|r| r.text.as_str().contains(".Other.run\n"))
    );
}

#[test]
fn final_stage_has_completed_named_owners_and_exact_immutable_effect() {
    let model = model().unwrap();
    for profile in stages::Profile::ALL {
        for requested in [false, true] {
            let definition = Definition::builtin(requested);
            let stage = retrieval::build::stage(profile, &definition, &model, &alignment_publication_order()).unwrap();
            assert_eq!(
                stage.effect,
                if requested {
                    stages::Effect::Embedding
                } else {
                    stages::Effect::Pure
                }
            );
            assert!(stage.reads::<synthesis::frames::Frame>());
            assert!(stage.reads::<analysis::synthesis::Invocation>());
            assert!(stage.reads::<analysis::catalog_evidence::Invocation>());
            assert!(stage.reads::<embedding::analytic::AnalysisEmbeddingUse>());
            assert!(stage.writes::<analysis::retrieval::Invocation>());
            assert!(stage.writes::<retrieval::consumption::RetrievalEmbeddingUse>());
            assert!(
                stage
                    .inputs
                    .iter()
                    .all(|i| i.transport() == stages::InputTransport::CompletedStore)
            );
            assert!(
                stage
                    .inputs
                    .iter()
                    .filter(|i| stages::is_vocabulary(i.name()))
                    .all(|i| i.prefix() == Some(stages::PublicationBoundary::Synthesis))
            );
        }
    }
}

fn alignment_publication_order() -> lctx_model::domain::stages::PublicationOrder {
    use lctx_model::domain::stages::*;
    PublicationOrder::planning(&[
        PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
        PublicationGroup::new(PublicationBoundary::Local, vec!["local"]),
        PublicationGroup::new(PublicationBoundary::Model, vec!["model"]),
        PublicationGroup::new(PublicationBoundary::Summary, vec!["summary"]),
        PublicationGroup::new(PublicationBoundary::Structural, vec!["structural"]),
        PublicationGroup::new(PublicationBoundary::Analytic, vec!["analytic"]),
        PublicationGroup::new(PublicationBoundary::Synthesis, vec!["synthesis"]),
    ]).unwrap()
}
