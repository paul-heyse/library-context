//! Pure exact-value replay; service fixtures do not establish live-service availability.
use lctx_model::domain::{
    analysis::retrieval::{AnalysisInvocation, AnalysisOutcome},
    catalog::evidence as c1,
    embedding::{
        analytic::{AnalysisEmbeddingUse, VectorAvailability},
        text::*,
        value::*,
        *,
    },
    normalized::Rows,
    resources::ResourceBudget,
    retrieval::{self, consumption::*, *},
    *,
};
fn id<T>(n: u8) -> Id<T> {
    serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
}
fn spec() -> Spec {
    let mut r = Spec::parse(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/embedding/qwen3-embedding-8b.json"
    )))
    .unwrap();
    r.reduction = "none".into();
    r.admission = None;
    r.dimensions = 3;
    r.source_dimensions = 3;
    r.document_template = "prefix: {text}".into();
    r
}
fn fixture(
    selected: bool,
) -> (
    ResourceBudget,
    ConsumptionData,
    AnalysisInvocation,
    EmbeddingSpec,
) {
    let b = ResourceBudget::fixed(8 << 20).unwrap();
    let mut d = ConsumptionData::new(&b);
    d.render
        .facts
        .definitions
        .insert(Definition::builtin(selected))
        .unwrap();
    let text = b"canonical same source";
    let a = lctx_model::domain::source::SourceArtifact::from_bytes(id(1), "guide.md".into(), text)
        .unwrap();
    d.render.source.core.artifacts.insert(a.clone()).unwrap();
    for c in artifact::ArtifactChunk::split(&a, text).unwrap() {
        d.render.facts.chunks.insert(c).unwrap();
    }
    let q = d
        .render
        .source
        .core
        .qualifications
        .insert(assertion::AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: id(2),
            scope: lctx_model::domain::source::CoverageScope::Artifact { artifact: a.id() }.id(),
            condition: conditions::Diagram::always().id(),
            modality: attribution::Modality::Definite,
            approximation: assertion::Approximation::Exact,
        })
        .unwrap();
    let document = d
        .render
        .source
        .facts
        .documents
        .insert(documents::DocumentObservation {
            qualification: q,
            source: a.id(),
            title: None,
            parsed: true,
        })
        .unwrap();
    let subject = d
        .render
        .evidence
        .subjects
        .insert(c1::RootSubject::Document {
            observation: document,
        })
        .unwrap();
    d.render
        .evidence
        .roots
        .insert(c1::EvidenceRoot {
            input: a.input,
            context: id(2),
            subject,
        })
        .unwrap();
    d.render
        .source
        .facts
        .runs
        .insert(attribution::ProviderRun {
            provider: id(3),
            input: a.input,
            context: id(2),
            configuration: ContentHash::of(b"c"),
            requested_families: ContentHash::of(b"r"),
        })
        .unwrap();
    struct FixtureTokenizer;
    impl retrieval::partition::Tokenizer for FixtureTokenizer {
        fn identity(&self)->ContentHash{ContentHash::of(b"independent-consumption-fixture")}
        fn encode(&self,text:&str)->Result<retrieval::partition::EncodedInput,ModelError>{
            let input=format!("prefix: {text}");
            let offsets=input.char_indices().map(|(a,c)|(a,a+c.len_utf8())).chain(std::iter::once((0,0))).collect::<Vec<_>>();
            Ok(retrieval::partition::EncodedInput{text:input,body_start:8,body_end:8+text.len(),specials:vec![false;offsets.len()],offsets})
        }
    }
    d.render.set_tokenizer(std::sync::Arc::new(FixtureTokenizer));
    d.output = retrieval::build::build(&d.render, &b).unwrap();
    let specification = EmbeddingSpec::new(&spec()).unwrap();
    d.specifications.insert(specification.clone()).unwrap();
    d.services
        .insert(configuration::ServiceConfiguration {
            specification: specification.id(),
            endpoint: "fixture://service".into(),
        })
        .unwrap(); // Pure vector replay assumes these nominal earlier parents have completed their own replay;
    // actual store qualification uses the full scheduled S0 producer, never these fixture rows.
    for (p, def) in [
        synthesis::build::definition(),
        retrieval::build::definition(),
    ] {
        d.render.synthesis.parameters.insert(p).unwrap();
        d.render.synthesis.analysis_definitions.insert(def).unwrap();
    }
    let c1 = analysis::catalog_evidence::Invocation::new(
        a.input,
        id(2),
        c1::build::definition().1.id(),
        None,
        [],
    )
    .0;
    d.render
        .facts
        .evidence_invocations
        .insert(c1.clone())
        .unwrap();
    d.render
        .synthesis
        .evidence_outcomes
        .insert(analysis::catalog_evidence::AnalysisOutcome {
            invocation: c1.id(),
            status: analysis::AnalysisStatus::Completed,
            reason: None,
        })
        .unwrap();
    let s0 = analysis::synthesis::Invocation::new(
        a.input,
        id(2),
        synthesis::build::definition().1.id(),
        None,
        [],
    )
    .0;
    d.render
        .synthesis
        .synthesis_outcomes
        .insert(analysis::synthesis::AnalysisOutcome {
            invocation: s0.id(),
            status: analysis::AnalysisStatus::Completed,
            reason: None,
        })
        .unwrap();
    d.render
        .synthesis
        .synthesis_invocations
        .insert(s0.clone())
        .unwrap();
    d.render
        .synthesis
        .synthesis_frames
        .insert(synthesis::frames::Frame {
            invocation: s0.id(),
            configuration: id(6),
            core: id(7),
            evidence: c1.id(),
            selection: id(8),
            structural: id(9),
            analytic: id(10),
            summary: id(11),
        })
        .unwrap();
    let mut parents = vec![];
    for source in d.render.parents(a.input, id(2)).unwrap() {
        parents.push(d.sources.insert(source).unwrap());
    }
    let (i, inputs) = AnalysisInvocation::new(
        a.input,
        id(2),
        retrieval::build::definition().1.id(),
        None,
        parents,
    );
    for row in inputs {
        d.parents.insert(row).unwrap();
    }
    (b, d, i, specification)
}
fn frames(
    d: &ConsumptionData,
    i: &AnalysisInvocation,
    u: &Rows<RetrievalEmbeddingUse>,
    b: &ResourceBudget,
) -> (Rows<AnalysisInvocation>, Rows<AnalysisOutcome>) {
    let mut rows = Rows::new(b);
    rows.insert(i.clone()).unwrap();
    let mut outcomes = Rows::new(b);
    outcomes.insert(d.outcome(i, u).unwrap()).unwrap();
    (rows, outcomes)
}
#[test]
fn exact_retrieval_and_analytic_winners_cold_replay_preserves_signed_zero() {
    let (b, mut d, i, specification) = fixture(true);
    let spec = spec();
    let window = d.output.windows.iter().next().unwrap().clone();
    let text = window.text.as_str();
    let v = AdmittedValue::new(&spec, &spec.document_text(text), 7, &[1.0, 0.0, -0.0], &b).unwrap();
    let mut uses = Rows::new(&b);
    let key =
        RetrievalEmbeddingUse::admit_into(&mut uses, i.id(), window.id(), &specification, &v, &b)
            .unwrap();
    let analytic_window = TextWindow {
        assessment: id(4),
        ordinal: 0,
        start: 0,
        end: text.len() as i64,
        text: text.into(),
        content: ContentHash::of(text.as_bytes()),
    };
    d.windows.insert(analytic_window.clone()).unwrap();
    AnalysisEmbeddingUse::admit_into(
        &mut d.analytic_uses,
        id(5),
        analytic_window.id(),
        &specification,
        &v,
        &b,
    )
    .unwrap();
    drop(v);
    let (invocations, outcomes) = frames(&d, &i, &uses, &b);
    d.verify(&invocations, &outcomes, &uses, &b).unwrap();
    let decoded = embedding::value::decode(
        &spec,
        &uses.get(key).unwrap().bytes.as_ref().unwrap().0,
        uses.get(key).unwrap().value_digest.unwrap(),
        7,
        &b,
    )
    .unwrap();
    assert_eq!(decoded.values()[2].to_bits(), (-0.0f32).to_bits());
    drop(decoded);
    // Both are individually valid values for the same request; exact generation winner still differs.
    let changed =
        AdmittedValue::new(&spec, &spec.document_text(text), 7, &[1.0, 0.0, 0.0], &b).unwrap();
    let mut forged = Rows::new(&b);
    RetrievalEmbeddingUse::admit_into(
        &mut forged,
        i.id(),
        window.id(),
        &specification,
        &changed,
        &b,
    )
    .unwrap();
    assert!(
        d.verify(&invocations, &outcomes, &forged, &b)
            .unwrap_err()
            .to_string()
            .contains("exact winning bytes")
    );
    drop(changed);
    drop(d);
    drop(uses);
    drop(forged);
    drop(invocations);
    drop(outcomes);
    assert_eq!(b.reserved(), 0);
}
#[test]
fn corruption_spec_request_codec_or_consumption_erasure_refuses() {
    let (b, d, i, specification) = fixture(true);
    let spec = spec();
    let window = d.output.windows.iter().next().unwrap();
    let value = AdmittedValue::new(
        &spec,
        &spec.document_text(window.text.as_str()),
        2,
        &[1.0, 0.0, 0.0],
        &b,
    )
    .unwrap();
    let mut uses = Rows::new(&b);
    RetrievalEmbeddingUse::admit_into(&mut uses, i.id(), window.id(), &specification, &value, &b)
        .unwrap();
    let (invocations, outcomes) = frames(&d, &i, &uses, &b);
    d.verify(&invocations, &outcomes, &uses, &b).unwrap();
    for case in 0..5 {
        let mut row = uses.iter().next().unwrap().clone();
        match case {
            0 => row.input = ContentHash::of(b"foreign"),
            1 => row.value_digest = Some(ContentHash::of(b"corrupt")),
            2 => row.specification = id(6),
            3 => row.bytes.as_mut().unwrap().0.pop().map(|_| ()).unwrap(),
            _ => row.codec = Some(2),
        }
        let mut forged = Rows::new(&b);
        let inserted = forged.insert(row);
        if inserted.is_ok() {
            assert!(d.verify(&invocations, &outcomes, &forged, &b).is_err());
        }
    }
    assert!(
        d.verify(&invocations, &outcomes, &Rows::new(&b), &b)
            .is_err()
    );
    assert!(
        d.verify(&Rows::new(&b), &Rows::new(&b), &Rows::new(&b), &b)
            .is_err()
    );
}
#[test]
fn service_and_token_refusals_retain_lexical_units_and_disabled_vectors_are_completed() {
    for selected in [false, true] {
        let (b, d, i, specification) = fixture(selected);
        let window = d.output.windows.iter().next().unwrap();
        let spec = spec();
        for (availability, tokens) in [
            (VectorAvailability::ServiceUnavailable, None),
            (
                VectorAvailability::TokenLimit,
                Some(i64::from(spec.max_document_tokens) + 1),
            ),
        ] {
            let mut uses = Rows::new(&b);
            if selected {
                uses.insert(RetrievalEmbeddingUse {
                    invocation: i.id(),
                    window: window.id(),
                    specification: specification.id(),
                    input: input_hash(&spec.document_text(window.text.as_str())),
                    availability,
                    admitted_tokens: tokens,
                    codec: None,
                    value_digest: None,
                    bytes: None,
                })
                .unwrap();
            }
            let (invocations, outcomes) = frames(&d, &i, &uses, &b);
            d.verify(&invocations, &outcomes, &uses, &b).unwrap();
            assert_eq!(
                outcomes.iter().next().unwrap().status,
                if selected {
                    analysis::AnalysisStatus::Partial
                } else {
                    analysis::AnalysisStatus::Completed
                }
            );
            assert_eq!(d.output.units.len(), 1);
            assert_eq!(d.output.windows.len(), 1);
        }
    }
}
#[test]
fn foreign_spec_refuses_without_changing_lexical_preparation() {
    let (b, d, i, specification) = fixture(true);
    let mut other = spec();
    other.revision.push('x');
    let changed = EmbeddingSpec::new(&other).unwrap();
    assert_ne!(changed.id(), specification.id());
    let window = d.output.windows.iter().next().unwrap();
    let value = AdmittedValue::new(
        &other,
        &other.document_text(window.text.as_str()),
        1,
        &[1.0, 0.0, 0.0],
        &b,
    )
    .unwrap();
    let mut uses = Rows::new(&b);
    assert!(
        RetrievalEmbeddingUse::admit_into(
            &mut uses,
            i.id(),
            window.id(),
            &specification,
            &value,
            &b
        )
        .is_err()
    );
    let tiny = ResourceBudget::fixed(1).unwrap();
    let mut uses = Rows::new(&tiny);
    assert!(
        RetrievalEmbeddingUse::admit_into(
            &mut uses,
            i.id(),
            window.id(),
            &changed,
            &value,
            &tiny
        )
        .is_err()
    );
    assert!(uses.is_empty());
    assert_eq!(tiny.reserved(), 0);
}

#[test]
fn native_frames_require_exact_synthesis_catalog_parents_even_without_any_briefs() {
    for case in 0..7 {
        let (b, mut d, i, _) = fixture(false);
        let uses = Rows::new(&b);
        let (mut invocations, outcomes) = frames(&d, &i, &uses, &b);
        d.verify(&invocations, &outcomes, &uses, &b).unwrap();
        match case {
            0 => {
                d.render.synthesis.synthesis_invocations = Rows::new(&b);
                d.render.synthesis.synthesis_outcomes = Rows::new(&b);
                d.render.synthesis.synthesis_frames = Rows::new(&b);
            }
            1 => d.render.synthesis.synthesis_outcomes = Rows::new(&b),
            2 => {
                let mut frame = d
                    .render
                    .synthesis
                    .synthesis_frames
                    .iter()
                    .next()
                    .unwrap()
                    .clone();
                frame.evidence = id(93);
                d.render.synthesis.synthesis_frames = Rows::new(&b);
                d.render.synthesis.synthesis_frames.insert(frame).unwrap();
            }
            3 => {
                d.sources = Rows::new(&b);
                d.parents = Rows::new(&b);
            }
            4 => {
                let mut wrong = i.clone();
                wrong.inputs = ContentHash::of(b"forged parent membership");
                invocations = Rows::new(&b);
                invocations.insert(wrong).unwrap();
            }
            5 => {
                d.render.synthesis.analysis_definitions = Rows::new(&b);
                d.render.synthesis.parameters = Rows::new(&b);
            }
            _ => d.render.synthesis.evidence_outcomes = Rows::new(&b),
        }
        assert!(d.verify(&invocations, &outcomes, &uses, &b).is_err());
    }
}

#[test]
fn unit_consumption_preserves_exact_finite_oracle_and_refuses_missing_foreign_or_request_drift() {
    let (b, data, invocation, specification) = fixture(true);
    let spec = spec();
    let mut uses = Rows::new(&b);
    for window in data.output.windows.iter() {
        let value = AdmittedValue::new(
            &spec,
            &spec.document_text(window.text.as_str()),
            7,
            &[1.0, 0.0, -0.0],
            &b,
        )
        .unwrap();
        RetrievalEmbeddingUse::admit_into(
            &mut uses,
            invocation.id(),
            window.id(),
            &specification,
            &value,
            &b,
        )
        .unwrap();
    }
    verify_uses(&data.output, &invocation, Some(&specification), &uses, &b).unwrap();
    let (invocations, outcomes) = frames(&data, &invocation, &uses, &b);
    data.verify_completion(&invocations, &outcomes, &uses, &b)
        .unwrap();
    data.verify_frames(&invocations, &outcomes, &outcomes, &b)
        .unwrap();
    assert!(
        verify_uses(
            &data.output,
            &invocation,
            Some(&specification),
            &Rows::new(&b),
            &b
        )
        .is_err()
    );
    let mut foreign = Rows::new(&b);
    for row in uses.iter() {
        let mut row = row.clone();
        row.invocation = id(88);
        foreign.insert(row).unwrap();
    }
    assert!(
        verify_uses(
            &data.output,
            &invocation,
            Some(&specification),
            &foreign,
            &b
        )
        .is_err()
    );
    drop(foreign);
    let mut changed = Rows::new(&b);
    for row in uses.iter() {
        let mut row = row.clone();
        row.input = ContentHash::of(b"different exact request");
        changed.insert(row).unwrap();
    }
    assert!(
        verify_uses(
            &data.output,
            &invocation,
            Some(&specification),
            &changed,
            &b
        )
        .is_err()
    );
    let mut disposition = Disposition::default();
    disposition.observe(&uses);
    assert_eq!(
        disposition.outcome(&invocation),
        data.outcome(&invocation, &uses).unwrap()
    );
    drop(changed);
    drop(uses);
    drop(invocations);
    drop(outcomes);
    drop(data);
    assert_eq!(b.reserved(), 0);
}
