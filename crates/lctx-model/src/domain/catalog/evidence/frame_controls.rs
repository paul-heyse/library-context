//! Pure production-factory and consumer controls, without native admission authority.
use super::*;
use crate::domain::{analysis::owner::AnalysisOwner, *};

struct Fixture {
    budget: ResourceBudget,
    context: attribution::AnalysisContext,
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
        context,
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
fn retrieval_reconstructs_admitted_compact_evidence_frames_and_refuses_missing_or_wrong_context_owner()
 {
    use analysis::{
        expected::CoverageAdmission,
        sources::{CapturedSources, CompletedInput},
    };
    let f = fixture();
    let mut inputs = Rows::<input::InputRevision>::new(&f.budget);
    let mut scopes = Rows::<source::CoverageScope>::new(&f.budget);
    for (name, batch) in &f.batches {
        if *name == input::InputRevision::NAME {
            inputs.decode(batch).unwrap();
        }
        if *name == source::CoverageScope::NAME {
            scopes.decode(batch).unwrap();
        }
    }
    let local = f
        .proofs
        .iter()
        .find(|owner| owner.analysis_owner() == Some(AnalysisOwner::Local))
        .unwrap();
    let source = f
        .proofs
        .iter()
        .find(|owner| owner.analysis_owner() == Some(AnalysisOwner::SourceCalls))
        .unwrap();
    let mut foreign_context = f.context.clone();
    foreign_context.config_digest = ContentHash::of(b"foreign-retrieval-frame-context");
    let foreign = uniform_outcome::test_support::empty_owner(
        AnalysisOwner::Local,
        inputs.iter().next().unwrap(),
        &foreign_context,
        &f.budget,
    );
    let model = model().unwrap().digest();
    fn permit<R: Record>(rows: &Rows<R>, model: ContentHash) -> CompletedInput<R> {
        CompletedInput::new(
            "retrieval-compact-frame-control",
            model,
            ContentHash::of(b"retrieval-compact-frame-control/v1"),
            ContentHash::of(R::NAME.as_bytes()),
            rows.len() as u64,
        )
        .unwrap()
    }
    let check = |local_owner: Option<&uniform_outcome::ValidatedUniformOwner>,
                 local_descriptor: &uniform_outcome::UniformNotRequested|
     -> Result<(), ModelError> {
        let mut descriptors = Rows::new(&f.budget);
        descriptors.insert(local_descriptor.clone())?;
        descriptors.insert(source.descriptor().clone())?;
        let input_permit = permit(&inputs, model);
        let scope_permit = permit(&scopes, model);
        let descriptor_permit = permit(&descriptors, model);
        let captured = CapturedSources::capture(
            stages::Profile::Catalog,
            [
                input_permit.snapshot(),
                scope_permit.snapshot(),
                descriptor_permit.snapshot(),
            ],
            &f.budget,
        )?;
        let mut admission = CoverageAdmission::new(&captured, &f.budget)?;
        for owner in local_owner.into_iter().chain(std::iter::once(source)) {
            admission.admit_uniform_owner(owner)?;
        }
        admission.visit(
            &input_permit,
            &input::InputRevision::encode(&inputs.iter().cloned().collect::<Vec<_>>())?,
        )?;
        admission.visit(
            &scope_permit,
            &<source::CoverageScope as Record>::encode(
                &scopes.iter().cloned().collect::<Vec<_>>(),
            )?,
        )?;
        admission.visit(
            &descriptor_permit,
            &uniform_outcome::UniformNotRequested::encode(
                &descriptors.iter().cloned().collect::<Vec<_>>(),
            )?,
        )?;
        let mut data = retrieval::build::Data::new(&f.budget);
        for (name, batch) in &f.batches {
            data.visit(name, batch)?;
        }
        assert!(data.source.runtime.local_invocations.is_empty());
        assert!(data.source.runtime.source_invocations.is_empty());
        assert!(matches!(super::verify(
            &data.source.facts.runs, &data.source.facts.core_invocations,
            &data.facts.evidence_invocations, &data.facts.evidence_sources,
            &data.facts.evidence_inputs, &data.source.runtime.lower(), &f.budget,
        ), Err(ModelError::Invalid(message)) if message == "C1 named lower native frame absent"));
        data.verify_evidence_frames(&admission, &f.budget)?;
        assert!(data.facts.evidence_sources.same(&f.sources));
        assert!(data.facts.evidence_inputs.same(&f.inputs));
        Ok(())
    };
    check(Some(local), local.descriptor()).unwrap();
    assert!(check(None, local.descriptor()).is_err());
    assert!(
        matches!(check(Some(&foreign.owner), foreign.owner.descriptor()),
        Err(ModelError::Invalid(message)) if message == "C1 named lower native frame absent")
    );
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
