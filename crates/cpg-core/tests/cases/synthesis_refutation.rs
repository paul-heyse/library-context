//! Native checked-false sources retain negative authority; proofless twins cannot invent it.
use crate::catalog_runtime;
use cpg_core::workspace::Workspace;
use lctx_model::domain::{
    admission::Frontier,
    analysis::{
        policy::{AssertionKind, BriefSection, EvidenceStatus, FindingKind},
        synthesis as owner,
    },
    execution::summary_consequences::{ClaimProof, ClaimRefutationCoverage},
    normalized::Rows,
    stages::Profile,
    synthesis::{self, assertions, documentary, observations, summary},
    *,
};
fn batches<R: Record>(workspace: &Workspace) -> Vec<arrow_array::RecordBatch> {
    workspace
        .completed::<R>()
        .unwrap()
        .batches()
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
fn rows<R: Record>(workspace: &Workspace, budget: &resources::ResourceBudget) -> Rows<R> {
    let mut rows = Rows::new(budget);
    for batch in batches::<R>(workspace) {
        rows.decode(&batch).unwrap();
    }
    rows
}
#[tokio::test]
async fn native_false_source_compiles_exact_negative_s0_authority() {
    let mut settings = catalog_runtime::settings("cases");
    settings.configured_seeds = vec!["cases.false_source_control".into()];
    settings.witnesses = 128;
    settings.brief_budget = 1;
    let fixture = catalog_runtime::compile(
        "synthesis_refutation",
        Profile::Behavioral,
        Frontier::Catalog,
        settings,
        None,
    )
    .await;
    let reader = fixture.workspace.clone();
    let budget = reader.budget();
    let mut data = summary::Data::new(budget);
    macro_rules! load_summary {($($f:ident:$t:ty,)*)=>{$(for b in batches::<$t>(&reader) { data.$f.decode(&b).unwrap(); })*};}
    lctx_model::synthesis_summary_inputs!(load_summary);
    let mut docs = documentary::Data::new(budget);
    macro_rules! load_docs {($($f:ident:$t:ty,)*)=>{$(for b in batches::<$t>(&reader) { docs.$f.decode(&b).unwrap(); })*};}
    lctx_model::synthesis_documentary_inputs!(load_docs);
    let mut original = observations::Output::new(budget);
    macro_rules! load_outputs {($($f:ident:$t:ty,)*)=>{$(for b in batches::<$t>(&reader) { original.$f.decode(&b).unwrap(); })*};}
    lctx_model::synthesis_observation_outputs!(load_outputs);
    let frames = rows::<synthesis::frames::Frame>(&reader, budget);
    let invocations = rows::<owner::Invocation>(&reader, budget);
    let coverage = rows::<owner::AnalysisCoverage>(&reader, budget);
    let facets = rows::<summary::SummaryFacet>(&reader, budget);
    let authored = rows::<assertions::ProgrammaticAssertion>(&reader, budget);
    let templates = rows::<assertions::AssertionTemplate>(&reader, budget);
    let supports = rows::<assertions::ProgrammaticAssertionSupport>(&reader, budget);
    let assertion_sources = rows::<assertions::AssertionSource>(&reader, budget);
    let brief_links = rows::<synthesis::briefs::BriefAssertion>(&reader, budget);
    let negatives = data
        .conclusions
        .iter()
        .filter(|c| c.verdict == obligation::Verdict::RefutedUnderModel)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        !negatives.is_empty(),
        "native checked-false source must retain a proof-backed false alternative; complete Flow coverage may not be inferred from an absent result. Conclusions: {:?}; proofs: {:?}",
        data.conclusions.iter().collect::<Vec<_>>(),
        data.proofs.iter().collect::<Vec<_>>()
    );
    for conclusion in &negatives {
        assert_eq!(
            conclusion.coverage,
            attribution::CoverageStatus::CompleteUnderStatedModel
        );
        let proof = data
            .proofs
            .get(conclusion.proof.expect("negative requires exact proof"))
            .unwrap();
        let ClaimProof::Refutation {
            invocation,
            claim,
            qualification,
            status,
            heuristic,
            ..
        } = proof
        else {
            panic!("negative lacks nominal Refutation proof")
        };
        assert_eq!(*invocation, conclusion.invocation);
        assert_eq!(Some(*qualification), conclusion.qualification);
        assert_eq!(
            (*status, *heuristic),
            (EvidenceStatus::StructurallyObserved, false)
        );
        let q = original.qualifications.get(*qualification).unwrap();
        assert_eq!(
            (q.modality, q.approximation),
            (
                attribution::Modality::Definite,
                assertion::Approximation::Exact
            )
        );
        let nodes = original.nodes.iter().cloned().collect::<Vec<_>>();
        assert_eq!(
            conditions::Diagram::from_records(
                original.conditions.get(q.condition).unwrap(),
                &nodes
            )
            .unwrap()
            .id(),
            conditions::Diagram::never().id()
        );
        let exact = summary::evidence(&data, conclusion).unwrap().unwrap();
        let owner::SupportSource::Summary {
            derivation: earlier,
        } = exact
        else {
            panic!("negative changes Summary owner")
        };
        let source = analysis::summary::SupportSource::ClaimProof {
            witness: proof.id(),
        };
        assert!(
            data.premises
                .iter()
                .any(|p| p.derivation == earlier && p.source == source.id())
        );
        let matches = facets
            .iter()
            .filter(|f| f.conclusion == conclusion.id())
            .collect::<Vec<_>>();
        assert!(!matches.is_empty());
        for facet in matches {
            assert_eq!(
                (facet.claim, facet.qualification, facet.source),
                (Some(*claim), Some(*qualification), Some(exact.id()))
            );
            let subject = owner::ObligationSubject::SummaryClaim { transfer: *claim };
            let finding = original
                .findings
                .iter()
                .find(|f| f.subject == subject.id() && f.kind == FindingKind::BehavioralRefutation)
                .expect("persisted kind18 finding");
            assert_eq!(
                (finding.qualification, finding.status),
                (*qualification, EvidenceStatus::StructurallyObserved)
            );
            let finding_support = original
                .supports
                .iter()
                .find(|s| s.finding == finding.id())
                .unwrap();
            let owner::SupportSource::AnalysisDerivation { derivation } =
                original.sources.get(finding_support.source).unwrap()
            else {
                panic!("finding must cite own exact derivation")
            };
            assert!(
                original
                    .premises
                    .iter()
                    .any(|p| p.derivation == *derivation && p.source == exact.id())
            );
            if let Some(member) = facet.member {
                let template = assertions::AssertionTemplate::Summary { facet: facet.id() };
                assert_eq!(templates.get(template.id()), Some(&template));
                let assertion = authored
                    .iter()
                    .find(|a| a.member == member && a.template == template.id())
                    .expect("persisted negative assertion");
                assert_eq!(
                    (
                        assertion.kind(),
                        assertion.section(),
                        assertion.status(),
                        assertion.qualification()
                    ),
                    (
                        AssertionKind::BehavioralRefutation,
                        BriefSection::Limits,
                        EvidenceStatus::StructurallyObserved,
                        *qualification
                    )
                );
                assert!(assertion.text().contains("RefutedUnderModel"));
                let support = supports
                    .iter()
                    .find(|s| s.assertion == assertion.id())
                    .unwrap();
                assert_eq!(
                    assertion_sources.get(support.source),
                    Some(&assertions::AssertionSource::Summary { facet: facet.id() })
                );
                assert!(
                    brief_links.iter().any(|b| b.assertion == assertion.id()),
                    "selected public brief retains the Limits assertion"
                );
            }
        }
    }
    assert!(
        facets
            .iter()
            .any(|f| negatives.iter().any(|c| c.id() == f.conclusion) && f.member.is_some()),
        "public negative must reach assertion and brief consumers"
    );

    // Shared proof validator accepts the persisted complete false question and refuses missing
    // membership or Partial coverage. Reads and mutations are typed, never SQL writes.
    let invariant = lctx_model::domain::validation::invariants_for::<ClaimProof>()[0].clone();
    let mut proof_inputs = Vec::new();
    macro_rules! proof_input {($($t:ty),*)=>{$(for batch in batches::<$t>(&reader) { proof_inputs.push((<$t>::NAME, batch)); })*};}
    proof_input!(
        ClaimProof,
        ClaimRefutationCoverage,
        execution::summary_consequences::SummaryClaim,
        assertion::AssertionQualification,
        conditions::Condition,
        conditions::ConditionNode,
        attribution::ProviderCoverage,
        attribution::ProviderRun,
        analysis::summary::Invocation
    );
    for mutation in 0..=2 {
        let mut check = (invariant.create)(budget);
        for (name, original) in &proof_inputs {
            let batch = if mutation == 1 && *name == ClaimRefutationCoverage::NAME {
                <ClaimRefutationCoverage as Record>::encode(&[]).unwrap()
            } else if mutation == 2 && *name == attribution::ProviderCoverage::NAME {
                let mut rows = <attribution::ProviderCoverage as Record>::decode(original).unwrap();
                for row in &mut rows {
                    if row.family == attribution::FactFamily::Flow {
                        row.status = attribution::CoverageStatus::Partial;
                        row.reason = Some(obligation::ObligationKind::IncompleteCoverage);
                    }
                }
                <attribution::ProviderCoverage as Record>::encode(&rows).unwrap()
            } else {
                original.clone()
            };
            check.visit(name, &batch).unwrap();
        }
        let result = check.finish();
        if mutation == 0 {
            result.unwrap();
        } else {
            assert!(
                result.is_err(),
                "refutation mutation {mutation} must refuse"
            );
        }
    }

    let mut observed = observations::Data::new(budget);
    for q in original.qualifications.iter() {
        observed.qualifications.insert(q.clone()).unwrap();
    }
    for c in original.conditions.iter() {
        observed.conditions.insert(c.clone()).unwrap();
    }
    for n in original.nodes.iter() {
        observed.nodes.insert(n.clone()).unwrap();
    }
    let accepted = negatives[0].clone();
    for unknown in [false, true] {
        let mut twin = accepted.clone();
        twin.proof = None;
        if unknown {
            twin.verdict = obligation::Verdict::Unknown;
            twin.coverage = attribution::CoverageStatus::Partial;
            twin.reason = Some(obligation::ObligationKind::IncompleteCoverage);
        }
        data.conclusions = Rows::new(budget);
        data.conclusions.insert(twin.clone()).unwrap();
        assert!(summary::evidence(&data, &twin).unwrap().is_none());
        let (facets, sources) =
            summary::build(&data, &docs, &frames, &invocations, budget).unwrap();
        assert!(!facets.is_empty() && facets.iter().all(|f| f.source.is_none()));
        assert!(sources.is_empty());
        let mut output = observations::Output::new(budget);
        summary::extend_observations(
            &data,
            &observed,
            &facets,
            &frames,
            &invocations,
            &coverage,
            &mut output,
            budget,
        )
        .unwrap();
        assert!(output.findings.is_empty() && output.derivations.is_empty());
        let mut assertions = assertions::Output::new(budget);
        assertions::extend_summary(
            &data,
            &observed,
            &facets,
            &frames,
            &invocations,
            &mut assertions,
            budget,
        )
        .unwrap();
        assert!(
            assertions.assertions.is_empty(),
            "Unknown/proofNone cannot acquire finding or assertion authority"
        );
    }
}

