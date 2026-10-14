//! Pure production-factory and consumer controls, without native admission authority.
use super::*;
use crate::domain::{analysis::owner::AnalysisOwner, *};

struct Fixture {
    budget: ResourceBudget,
    proofs: Vec<uniform_outcome::ValidatedUniformOwner>,
    batches: Vec<(&'static str, arrow_array::RecordBatch)>,
    core: core::Invocation,
    evidence: evidence::Invocation,
    sources: Rows<evidence::InvocationSource>,
    inputs: Rows<evidence::AnalysisInput>,
}
fn batch<R: Record>(rows: &[R]) -> (&'static str, arrow_array::RecordBatch) {
    (R::NAME, <R as Record>::encode(rows).unwrap())
}
fn fixture() -> Fixture {
    let budget = ResourceBudget::fixed(128 << 20).unwrap();
    let input = input::InputRevision::from_entries(vec![]).unwrap();
    let context = attribution::AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"compact-catalog-frame-control"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let mut proofs = Vec::new();
    let mut batches = Vec::new();
    let mut compact = uniform_outcome::AnalysisPredecessors::new(&budget);
    let declared = super::invariants().remove(0).inputs;
    for kind in [AnalysisOwner::Local, AnalysisOwner::SourceCalls] {
        let selected = uniform_outcome::test_support::empty_owner(kind, &input, &context, &budget);
        assert!(selected.domain.is_explicit_empty());
        compact.include(selected.domain).unwrap();
        proofs.push(selected.owner);
        batches.extend(
            selected
                .batches
                .into_iter()
                .filter(|(name, _)| declared.iter().any(|input| input.name() == *name)),
        );
    }
    let (core, _) = core::Invocation::new(
        input.id(),
        context.id(),
        catalog::build::definition().1.id(),
        None,
        [],
    );
    let mut lower = super::super::runtime::RuntimeData::new(&budget);
    lower.compact = compact;
    let mut sources = Rows::new(&budget);
    for source in super::sources(&core, &lower.lower()).unwrap() {
        sources.insert(source).unwrap();
    }
    let mut expected = Rows::new(&budget);
    expected
        .insert(evidence::InvocationSource::CatalogCore {
            invocation: core.id(),
        })
        .unwrap();
    for proof in &proofs {
        expected
            .insert(evidence::InvocationSource::UniformAnalysis {
                descriptor: proof.descriptor().id(),
            })
            .unwrap();
    }
    assert!(sources.same(&expected));
    let (evidence, links) = evidence::Invocation::new(
        input.id(),
        context.id(),
        super::super::build::definition().1.id(),
        None,
        sources.iter().map(Record::id),
    );
    let mut inputs = Rows::new(&budget);
    for link in links {
        inputs.insert(link).unwrap();
    }
    batches.extend([
        batch(std::slice::from_ref(&core)),
        batch(std::slice::from_ref(&evidence)),
        batch(&sources.iter().cloned().collect::<Vec<_>>()),
        batch(&inputs.iter().cloned().collect::<Vec<_>>()),
    ]);
    Fixture {
        budget,
        proofs,
        batches,
        core,
        evidence,
        sources,
        inputs,
    }
}
fn evidence_check(f: &Fixture, protected: bool) -> Result<(), ModelError> {
    let mut check = (super::invariants().remove(0).create)(&f.budget);
    if protected {
        for proof in &f.proofs {
            check.admit_uniform_owner(proof)?;
        }
    }
    for (name, batch) in &f.batches {
        check.visit(name, batch)?;
    }
    check.finish()
}
#[test]
fn compact_catalog_parent_checks_do_not_consume_unrelated_analytic_text_or_retrieval_configuration()
{
    for invariant in [
        super::invariants().remove(0),
        selection::frames::invariants().remove(0),
    ] {
        for unrelated in [
            analysis::settings::AnalyticsConfiguration::NAME,
            embedding::text::TextDefinition::NAME,
            retrieval::RetrievalDefinition::NAME,
            models::ModelCatalog::NAME,
        ] {
            assert!(
                invariant
                    .inputs
                    .iter()
                    .all(|input| input.name() != unrelated),
                "{} unexpectedly consumes {unrelated}",
                invariant.name
            );
        }
    }
}
#[test]
fn catalog_evidence_accepts_protected_compact_parents_for_empty_input_and_refuses_missing_proof_or_overlap()
 {
    let mut f = fixture();
    evidence_check(&f, true).unwrap();
    assert!(evidence_check(&f, false).is_err());
    let (ordinary, _) = analysis::local::Invocation::new(
        f.core.input,
        f.core.context,
        local_semantics::definition().1.id(),
        None,
        [],
    );
    f.batches.push(batch(&[ordinary]));
    assert!(evidence_check(&f, true).is_err());
}
#[test]
fn catalog_selection_rechecks_compact_evidence_parents_and_exact_empty_frame_membership() {
    let f = fixture();
    let source = analysis::selection::InvocationSource::CatalogEvidence {
        invocation: f.evidence.id(),
    };
    let (invocation, inputs) = analysis::selection::Invocation::new(
        f.core.input,
        f.core.context,
        selection::build::definition().1.id(),
        None,
        [source.id()],
    );
    let make = |protected: bool, include_input: bool| -> Result<(), ModelError> {
        let mut check = (selection::frames::invariants().remove(0).create)(&f.budget);
        if protected {
            for proof in &f.proofs {
                check.admit_uniform_owner(proof)?;
            }
        }
        for (name, batch) in &f.batches {
            check.visit(name, batch)?;
        }
        for (name, batch) in [
            batch(std::slice::from_ref(&invocation)),
            batch(std::slice::from_ref(&source)),
        ] {
            check.visit(name, &batch)?;
        }
        if include_input {
            let (name, batch) = batch(&inputs);
            check.visit(name, &batch)?;
        }
        check.finish()
    };
    make(true, true).unwrap();
    assert!(make(false, true).is_err());
    assert!(make(true, false).is_err());
    assert_eq!(f.sources.len(), 3);
    assert_eq!(f.inputs.len(), 3);
}
