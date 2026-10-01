//! A native contradictory composition crosses Summary, S0 and the real PG18 generation store.
//! Refusal twins change only typed views of the published Summary input; they never alter storage.
use cpg_core::{
    compilation::{PreparedCompilation, publish},
    generation_read::{GenerationSession, ProviderOptions},
    model_runtime::{AttemptRuntime, RuntimeOptions},
};
use cpg_extract::{acquisition::AcquiredInput, bundle::CapturedInputs, capture::CapturedInput};
use datafusion::prelude::SessionContext;
use lctx_model::domain::{
    admission::Frontier,
    analysis::{policy::{AssertionKind, BriefSection, EvidenceStatus, FindingKind}, synthesis as owner},
    execution::summary_consequences::{ClaimProof, ClaimRefutationCoverage},
    normalized::Rows,
    stages::Profile,
    synthesis::{self, assertions, documentary, observations, summary},
    *,
};
use lctx_postgres::{generations::GenerationStore, roles::{Role, RoleConfig}, testing::DisposableDatabase};
use std::sync::Arc;

async fn batches<R: Record>(reader: &GenerationSession) -> Vec<arrow_array::RecordBatch> {
    SessionContext::new().read_table(reader.table::<R>().unwrap()).unwrap().collect().await.unwrap()
}
async fn rows<R: Record>(reader: &GenerationSession, budget: &resources::ResourceBudget) -> Rows<R> {
    let mut rows = Rows::new(budget);
    for batch in batches::<R>(reader).await { rows.decode(&batch).unwrap(); }
    rows
}

#[tokio::test]
async fn native_false_composition_persists_exact_negative_s0_authority() {
    let profile = Profile::Behavioral;
    let runtime = AttemptRuntime::new(RuntimeOptions { memory_bytes: 2 << 30, partitions: 2 }).unwrap();
    let budget = runtime.budget();
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let importer = RoleConfig { format: 1, role: Role::Importer, url: db.url("lctx_importer"), max_connections: 6, provider_connections: 4, acquire_timeout_seconds: 5, statement_timeout_seconds: 60, lock_timeout_seconds: 10 };
    let serving = RoleConfig { role: Role::Serving, url: db.url("lctx_serving"), max_connections: 6, provider_connections: 2, ..importer.clone() };
    let model = Arc::new(model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone()).await.unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/synthesis_refutation");
    let captured = Arc::new(CapturedInputs::new(
        vec![AcquiredInput::tree(CapturedInput::capture(&root, &["cases.py".into()], budget).unwrap(), "negative-Summary")],
        cpg_extract::native_context::NativeContextConfig::committed(profile, budget).unwrap(),
    ));
    let settings = analysis::settings::AnalyticsConfiguration {
        module_prefixes: vec!["cases".into()], public_roots: vec!["cases".into()],
        configured_seeds: vec!["cases.recursive_false_control".into()], depth: 2,
        vertices: 512, arcs: 2048, witnesses: 128, brief_budget: 1,
        communities: false, pagerank: false, fca: false, knn: false, rca: false,
        type_layer: false, mention_layer: false, knn_layer: false,
    };
    let prepared = PreparedCompilation::new(Frontier::Catalog, settings, captured.config().catalog(), None, budget).unwrap();
    let published = publish(&store, &importer, db.writer.clone(), captured.clone(), &runtime, profile,
        ContentHash::of(b"native finite false composition"), &prepared, None, None).await.unwrap();
    let reader = GenerationSession::open(&serving, model.clone(), published.generation, ProviderOptions::default()).await.unwrap();
    let mut data = summary::Data::new(budget);
    macro_rules! load_summary {($($f:ident:$t:ty,)*)=>{$(for b in batches::<$t>(&reader).await { data.$f.decode(&b).unwrap(); })*};}
    lctx_model::synthesis_summary_inputs!(load_summary);
    let mut docs = documentary::Data::new(budget);
    macro_rules! load_docs {($($f:ident:$t:ty,)*)=>{$(for b in batches::<$t>(&reader).await { docs.$f.decode(&b).unwrap(); })*};}
    lctx_model::synthesis_documentary_inputs!(load_docs);
    let mut original = observations::Output::new(budget);
    macro_rules! load_outputs {($($f:ident:$t:ty,)*)=>{$(for b in batches::<$t>(&reader).await { original.$f.decode(&b).unwrap(); })*};}
    lctx_model::synthesis_observation_outputs!(load_outputs);
    let frames = rows::<synthesis::frames::Frame>(&reader, budget).await;
    let invocations = rows::<owner::Invocation>(&reader, budget).await;
    let coverage = rows::<owner::AnalysisCoverage>(&reader, budget).await;
    let facets = rows::<summary::SummaryFacet>(&reader, budget).await;
    let authored = rows::<assertions::ProgrammaticAssertion>(&reader, budget).await;
    let templates = rows::<assertions::AssertionTemplate>(&reader, budget).await;
    let supports = rows::<assertions::ProgrammaticAssertionSupport>(&reader, budget).await;
    let assertion_sources = rows::<assertions::AssertionSource>(&reader, budget).await;
    let brief_links = rows::<synthesis::briefs::BriefAssertion>(&reader, budget).await;
    let negatives = data.conclusions.iter().filter(|c| c.verdict == obligation::Verdict::RefutedUnderModel).cloned().collect::<Vec<_>>();
    assert!(!negatives.is_empty(), "native composition must retain a proof-backed false alternative; complete Flow coverage may not be inferred from an absent result");
    for conclusion in &negatives {
        assert_eq!(conclusion.coverage, attribution::CoverageStatus::CompleteUnderStatedModel);
        let proof = data.proofs.get(conclusion.proof.expect("negative requires exact proof")).unwrap();
        let ClaimProof::Refutation { invocation, claim, qualification, status, heuristic, .. } = proof else { panic!("negative lacks nominal Refutation proof") };
        assert_eq!(*invocation, conclusion.invocation);
        assert_eq!(Some(*qualification), conclusion.qualification);
        assert_eq!((*status, *heuristic), (EvidenceStatus::StructurallyObserved, false));
        let q = original.qualifications.get(*qualification).unwrap();
        assert_eq!((q.modality, q.approximation), (attribution::Modality::Definite, assertion::Approximation::Exact));
        let nodes = original.nodes.iter().cloned().collect::<Vec<_>>();
        assert_eq!(conditions::Diagram::from_records(original.conditions.get(q.condition).unwrap(), &nodes).unwrap().id(), conditions::Diagram::never().id());
        let exact = summary::evidence(&data, conclusion).unwrap().unwrap();
        let owner::SupportSource::Summary { derivation: earlier } = exact else { panic!("negative changes Summary owner") };
        let source = analysis::summary::SupportSource::ClaimProof { witness: proof.id() };
        assert!(data.premises.iter().any(|p| p.derivation == earlier && p.source == source.id()));
        let matches = facets.iter().filter(|f| f.conclusion == conclusion.id()).collect::<Vec<_>>();
        assert!(!matches.is_empty());
        for facet in matches {
            assert_eq!((facet.claim, facet.qualification, facet.source), (Some(*claim), Some(*qualification), Some(exact.id())));
            let subject = owner::ObligationSubject::SummaryClaim { transfer: *claim };
            let finding = original.findings.iter().find(|f| f.subject == subject.id() && f.kind == FindingKind::BehavioralRefutation).expect("persisted kind18 finding");
            assert_eq!((finding.qualification, finding.status), (*qualification, EvidenceStatus::StructurallyObserved));
            let finding_support = original.supports.iter().find(|s| s.finding == finding.id()).unwrap();
            let owner::SupportSource::AnalysisDerivation { derivation } = original.sources.get(finding_support.source).unwrap() else { panic!("finding must cite own exact derivation") };
            assert!(original.premises.iter().any(|p| p.derivation == *derivation && p.source == exact.id()));
            if let Some(member) = facet.member {
                let template = assertions::AssertionTemplate::Summary { facet: facet.id() };
                assert_eq!(templates.get(template.id()), Some(&template));
                let assertion = authored.iter().find(|a| a.member == member && a.template == template.id()).expect("persisted negative assertion");
                assert_eq!((assertion.kind(), assertion.section(), assertion.status(), assertion.qualification()), (AssertionKind::BehavioralRefutation, BriefSection::Limits, EvidenceStatus::StructurallyObserved, *qualification));
                assert!(assertion.text().contains("RefutedUnderModel"));
                let support = supports.iter().find(|s| s.assertion == assertion.id()).unwrap();
                assert_eq!(assertion_sources.get(support.source), Some(&assertions::AssertionSource::Summary { facet: facet.id() }));
                assert!(brief_links.iter().any(|b| b.assertion == assertion.id()), "selected public brief retains the Limits assertion");
            }
        }
    }
    assert!(facets.iter().any(|f| negatives.iter().any(|c| c.id() == f.conclusion) && f.member.is_some()), "public negative must reach assertion and brief consumers");

    // Shared proof validator accepts the persisted complete false question and refuses missing
    // membership or Partial coverage. Reads and mutations are typed, never SQL writes.
    let invariant = Relation::of::<ClaimProof>().invariants()[0].clone();
    let mut proof_inputs = Vec::new();
    macro_rules! proof_input {($($t:ty),*)=>{$(for batch in batches::<$t>(&reader).await { proof_inputs.push((<$t>::NAME, batch)); })*};}
    proof_input!(ClaimProof, ClaimRefutationCoverage, execution::summary_consequences::SummaryClaim,
        assertion::AssertionQualification, conditions::Condition, conditions::ConditionNode,
        attribution::ProviderCoverage, attribution::ProviderRun, analysis::summary::Invocation);
    for mutation in 0..=2 {
        let mut check = (invariant.create)(budget);
        for (name, original) in &proof_inputs {
            let batch = if mutation == 1 && *name == ClaimRefutationCoverage::NAME {
                <ClaimRefutationCoverage as Record>::encode(&[]).unwrap()
            } else if mutation == 2 && *name == attribution::ProviderCoverage::NAME {
                let mut rows = <attribution::ProviderCoverage as Record>::decode(original).unwrap();
                for row in &mut rows { if row.family == attribution::FactFamily::Flow { row.status = attribution::CoverageStatus::Partial; row.reason = Some(obligation::ObligationKind::IncompleteCoverage); } }
                <attribution::ProviderCoverage as Record>::encode(&rows).unwrap()
            } else { original.clone() };
            check.visit(name, &batch).unwrap();
        }
        let result = check.finish();
        if mutation == 0 { result.unwrap(); } else { assert!(result.is_err(), "refutation mutation {mutation} must refuse"); }
    }
    reader.close().await.unwrap();

    let mut observed = observations::Data::new(budget);
    for q in original.qualifications.iter() { observed.qualifications.insert(q.clone()).unwrap(); }
    for c in original.conditions.iter() { observed.conditions.insert(c.clone()).unwrap(); }
    for n in original.nodes.iter() { observed.nodes.insert(n.clone()).unwrap(); }
    let accepted = negatives[0].clone();
    for unknown in [false, true] {
        let mut twin = accepted.clone();
        twin.proof = None;
        if unknown { twin.verdict = obligation::Verdict::Unknown; twin.coverage = attribution::CoverageStatus::Partial; twin.reason = Some(obligation::ObligationKind::IncompleteCoverage); }
        data.conclusions = Rows::new(budget);
        data.conclusions.insert(twin.clone()).unwrap();
        assert!(summary::evidence(&data, &twin).unwrap().is_none());
        let (facets, sources) = summary::build(&data, &docs, &frames, &invocations, budget).unwrap();
        assert!(!facets.is_empty() && facets.iter().all(|f| f.source.is_none()));
        assert!(sources.is_empty());
        let mut output = observations::Output::new(budget);
        summary::extend_observations(&data, &observed, &facets, &frames, &invocations, &coverage, &mut output, budget).unwrap();
        assert!(output.findings.is_empty() && output.derivations.is_empty());
        let mut assertions = assertions::Output::new(budget);
        assertions::extend_summary(&data, &facets, &frames, &invocations, &mut assertions, budget).unwrap();
        assert!(assertions.assertions.is_empty(), "Unknown/proofNone cannot acquire finding or assertion authority");
    }
    store.retire(published.generation).await.unwrap();
}
