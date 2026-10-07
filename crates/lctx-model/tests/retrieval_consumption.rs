//! Pure exact-value replay; service fixtures do not establish live-service availability.
use lctx_model::domain::{
    analysis::retrieval::{AnalysisInvocation, AnalysisOutcome},
    catalog::evidence as c1,
    embedding::{
        analytic::VectorAvailability,
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
    r.dimensions = 4096;
    r.source_dimensions = 4096;
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
    let document=DocumentRecipe::new(&spec()).unwrap();
    let policy=embedding::projection::ProjectionDefinition::initial(&spec());
    d.documents.insert(document.clone()).unwrap();d.projections.insert(policy.clone()).unwrap();
    d.services
        .insert(configuration::ServiceConfiguration {
            specification: specification.id(),
            endpoint: "fixture://service".into(),document:document.id(),projection:policy.id(),
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

fn canonical(d:&ConsumptionData,b:&ResourceBudget)->(FullValue,embedding::projection::ProjectedValue,embedding::consumption::PublishedValue){
    let window=d.output.windows.iter().next().unwrap();let mut vector=vec![0.0;4096];vector[0]=1.0;vector[2]=-0.0;
    let admitted=AdmittedValue::new(&spec(),window.input_text.as_str(),window.tokens.unwrap() as u32,&vector,b).unwrap();
    let full=FullValue::new(d.selected_spec().unwrap(),&admitted).unwrap();let projection=embedding::projection::ProjectedValue::new(&full,d.policy().unwrap()).unwrap();
    let published=embedding::consumption::PublishedValue{value:full.id(),projection:projection.id(),input:full.input,tokens:full.tokens as u32};(full,projection,published)
}
fn uses(d:&ConsumptionData,i:&AnalysisInvocation,published:&embedding::consumption::PublishedValue,b:&ResourceBudget)->Rows<RetrievalEmbeddingUse>{let mut uses=Rows::new(b);for window in d.output.windows.iter(){RetrievalEmbeddingUse::admit_into(&mut uses,i.id(),window.id(),d.selected_consumption().unwrap(),published,b).unwrap();}uses}
fn seed(d:&mut ConsumptionData,full:&FullValue,projection:&embedding::projection::ProjectedValue){let encoder=d.selected_spec().unwrap().clone();let policy=d.policy().unwrap().clone();d.values.admit_full(full,&encoder,&policy).unwrap();d.values.admit_projection(projection).unwrap();}
#[test]
fn canonical_companions_roundtrip_preserves_signed_zero_and_reference_only_uses(){
    let(b,mut d,i,_)=fixture(true);let(full,projection,published)=canonical(&d,&b);
    let full=FullValue::decode(&FullValue::encode(&[full]).unwrap()).unwrap().pop().unwrap();
    let projected=embedding::projection::ProjectedValue::decode(&embedding::projection::ProjectedValue::encode(&[projection]).unwrap()).unwrap().pop().unwrap();
    let vector=decode_vector(&full.bytes.0,4096).unwrap();assert_eq!(vector[2].to_bits(),(-0.0f32).to_bits());assert_eq!(projected.values().unwrap()[2].to_bits(),(-0.0f32).to_bits());
    seed(&mut d,&full,&projected);let uses=uses(&d,&i,&published,&b);let(invocations,outcomes)=frames(&d,&i,&uses,&b);d.verify(&invocations,&outcomes,&uses,&b).unwrap();
    let relation=Relation::of::<RetrievalEmbeddingUse>();let fields=relation.fields();assert!(!fields.iter().any(|f|matches!(f.name(),"bytes"|"codec"|"value_digest")));
}
#[test]
fn missing_canonical_companions_refuse_even_when_local_reference_keys_match(){
    let(b,mut d,i,_)=fixture(true);let(full,projection,published)=canonical(&d,&b);let uses=uses(&d,&i,&published,&b);
    verify_uses(&d.output,&i,Some(d.selected_consumption().unwrap()),&uses,&b).unwrap();assert!(d.verify_canonical_uses(&uses).is_err());
    let encoder=d.selected_spec().unwrap().clone();let policy=d.policy().unwrap().clone();d.values.admit_full(&full,&encoder,&policy).unwrap();assert!(d.verify_canonical_uses(&uses).is_err());
    d.values.admit_projection(&projection).unwrap();d.verify_canonical_uses(&uses).unwrap();let mut forged=projection.clone();forged.source_digest=ContentHash::of(b"foreign full");assert!(d.values.admit_projection(&forged).is_err());
}
#[test]
fn request_recipe_reference_token_and_consumption_erasure_refuse(){
    let(b,mut d,i,_)=fixture(true);let(full,projection,published)=canonical(&d,&b);seed(&mut d,&full,&projection);let uses=uses(&d,&i,&published,&b);let(invocations,outcomes)=frames(&d,&i,&uses,&b);d.verify(&invocations,&outcomes,&uses,&b).unwrap();
    for case in 0..6{let mut row=uses.iter().next().unwrap().clone();match case{0=>row.input=ContentHash::of(b"foreign"),1=>row.value=Some(id(81)),2=>row.projection=Some(id(82)),3=>row.document=id(83),4=>row.admitted_tokens=Some(0),_=>row.invocation=id(84)};let mut forged=Rows::new(&b);forged.insert(row).unwrap();assert!(d.verify(&invocations,&outcomes,&forged,&b).is_err());}
    assert!(d.verify(&invocations,&outcomes,&Rows::new(&b),&b).is_err());
}
#[test]
fn service_and_token_refusals_retain_lexical_units_and_disabled_vectors_are_completed(){
    for selected in [false,true]{let(b,d,i,specification)=fixture(selected);let window=d.output.windows.iter().next().unwrap();for(availability,tokens)in[(VectorAvailability::ServiceUnavailable,None),(VectorAvailability::TokenLimit,Some(2049))]{let mut uses=Rows::new(&b);if selected{uses.insert(RetrievalEmbeddingUse{invocation:i.id(),window:window.id(),specification:specification.id(),document:d.document().unwrap().id(),input:window.encoded_digest,availability,admitted_tokens:tokens,value:None,projection:None}).unwrap();}let(invocations,outcomes)=frames(&d,&i,&uses,&b);d.verify(&invocations,&outcomes,&uses,&b).unwrap();assert_eq!(outcomes.iter().next().unwrap().status,if selected{analysis::AnalysisStatus::Partial}else{analysis::AnalysisStatus::Completed});assert_eq!(d.output.units.len(),1);}}
}
#[test]
fn foreign_encoder_and_tiny_budget_refuse_reference_publication(){
    let(b,d,i,_)=fixture(true);let(_,_,mut published)=canonical(&d,&b);published.value=id(88);let mut uses=Rows::new(&b);assert!(RetrievalEmbeddingUse::admit_into(&mut uses,i.id(),d.output.windows.iter().next().unwrap().id(),d.selected_consumption().unwrap(),&published,&b).is_err());
    let(_,_,published)=canonical(&d,&b);let tiny=ResourceBudget::fixed(1).unwrap();let mut uses=Rows::new(&tiny);assert!(RetrievalEmbeddingUse::admit_into(&mut uses,i.id(),d.output.windows.iter().next().unwrap().id(),d.selected_consumption().unwrap(),&published,&tiny).is_err());assert!(uses.is_empty());assert_eq!(tiny.reserved(),0);
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
