//! Pure exact-projection controls. Native provider fidelity itself remains fact validation's job.
use arrow_array::RecordBatch;
use lctx_model::domain::{
    analysis::{native::*, policy::EvidenceStatus},
    assertion::{Approximation, AssertionQualification},
    attribution::{ExtractionMode, FactFamily, Fidelity, Modality, Origin},
    calls::{SignatureEnumerationObservation, SignatureEnumerationSupport},
    documents::{DocumentObservation, DocumentSupport},
    resources::ResourceBudget,
    source::{SyntaxObservation, SyntaxSupport},
    *,
};

fn nominal<T>(value: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([value; 16].into_iter()))
    .unwrap()
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(32 << 20).unwrap()
}
fn frame<R: Record>(rows: &[R]) -> (&'static str, RecordBatch) {
    (R::NAME, R::encode(rows).unwrap())
}
fn facts() -> Vec<(&'static str, RecordBatch)> {
    let q = AssertionQualification {
        context: nominal(1),
        scope: nominal(2),
        condition: nominal(3),
        modality: Modality::Candidate,
        approximation: Approximation::Over,
    };
    let syntax = SyntaxObservation {
        qualification: q.id(),
        occurrence: nominal(4),
        spelling: "x".into(),
    };
    let syntax_support = SyntaxSupport {
        assertion: syntax.id(),
        run: nominal(5),
        surface: nominal(6),
        evidence: nominal(7),
        origin: Origin::SourceObservation,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    let display = SyntaxSupport {
        surface: nominal(8),
        fidelity: Fidelity::DisplayOnly,
        ..syntax_support.clone()
    };
    let doc = DocumentObservation {
        qualification: q.id(),
        source: nominal(9),
        title: Some("Evidence".into()),
        parsed: true,
    };
    let doc_support = DocumentSupport {
        assertion: doc.id(),
        run: nominal(5),
        surface: nominal(10),
        evidence: nominal(11),
        origin: Origin::SourceObservation,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    vec![
        frame(&[q]),
        frame(&[syntax]),
        frame(&[syntax_support, display]),
        frame(&[doc]),
        frame(&[doc_support]),
    ]
}
fn inventory(
    frames: &[(&str, RecordBatch)],
    budget: &ResourceBudget,
) -> Result<NativeInventory, ModelError> {
    let mut native = NativeInventory::new(budget);
    for (name, batch) in frames {
        native.visit(name, batch)?;
    }
    Ok(native)
}
fn validate(
    facts: &[(&str, RecordBatch)],
    premises: &[NativeAssertionPremise],
    qualifications: &[NativeQualification],
) -> Result<(), ModelError> {
    let invariant = NativeQualification::invariants().remove(0);
    let mut check = (invariant.create)(&budget());
    for (name, batch) in facts {
        check.visit(name, batch)?;
    }
    check.visit(
        NativeAssertionPremise::NAME,
        &<NativeAssertionPremise as Record>::encode(premises)?,
    )?;
    check.visit(
        NativeQualification::NAME,
        &NativeQualification::encode(qualifications)?,
    )?;
    check.finish()
}
fn result(
    facts: &[(&str, RecordBatch)],
) -> (Vec<NativeAssertionPremise>, Vec<NativeQualification>) {
    let output = inventory(facts, &budget()).unwrap().collect().unwrap();
    (
        output.premises.iter().cloned().collect(),
        output.qualifications.iter().cloned().collect(),
    )
}

#[test]
fn exact_native_domain_preserves_multiple_supports_and_interpretations() {
    let frames = facts();
    let (premises, qualifications) = result(&frames);
    assert_eq!(premises.len(), 3);
    assert_eq!(qualifications.len(), 3);
    let statuses = qualifications
        .iter()
        .map(|q| (q.family.code(), q.fidelity.code(), q.status.code()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        statuses,
        [
            (
                FactFamily::Syntax.code(),
                Fidelity::NativeStructural.code(),
                EvidenceStatus::StructurallyObserved.code()
            ),
            (
                FactFamily::Syntax.code(),
                Fidelity::DisplayOnly.code(),
                EvidenceStatus::Unresolved.code()
            ),
            (
                FactFamily::Docs.code(),
                Fidelity::NativeStructural.code(),
                EvidenceStatus::Documented.code()
            ),
        ]
        .into_iter()
        .collect()
    );
    let q = AssertionQualification::decode(&frames[0].1)
        .unwrap()
        .remove(0);
    assert!(qualifications.iter().all(|row| row.qualification == q.id()));
    validate(&frames, &premises, &qualifications).unwrap();
    let mut shuffled = frames.clone();
    shuffled.reverse();
    shuffled.extend(frames.clone());
    assert_eq!(result(&shuffled), (premises, qualifications));
    validate(&[], &[], &[]).unwrap();
}

#[test]
fn stored_projection_refuses_missing_extra_or_mismatched_pairs() {
    let frames = facts();
    let (premises, qualifications) = result(&frames);
    assert!(validate(&frames, &premises[1..], &qualifications).is_err());
    assert!(validate(&frames, &premises, &qualifications[1..]).is_err());
    let mut extra = premises.clone();
    extra.push(NativeAssertionPremise::SyntaxObservation {
        assertion: nominal(99),
        support: nominal(100),
    });
    assert!(validate(&frames, &extra, &qualifications).is_err());
    let mut changed = premises.clone();
    let first = changed
        .iter_mut()
        .find(|row| matches!(row, NativeAssertionPremise::SyntaxObservation { .. }))
        .unwrap();
    if let NativeAssertionPremise::SyntaxObservation { assertion, .. } = first {
        *assertion = nominal(99);
    }
    assert!(validate(&frames, &changed, &qualifications).is_err());
}

#[test]
fn stored_projection_recomputes_every_semantic_payload_field() {
    let frames = facts();
    let (premises, qualifications) = result(&frames);
    let index = qualifications
        .iter()
        .position(|row| row.status == EvidenceStatus::Documented)
        .unwrap();
    for change in 0..4 {
        let mut forged = qualifications.clone();
        match change {
            0 => forged[index].qualification = nominal(100),
            1 => forged[index].family = FactFamily::Syntax,
            2 => forged[index].fidelity = Fidelity::DisplayOnly,
            _ => forged[index].status = EvidenceStatus::StructurallyObserved,
        }
        assert!(
            validate(&frames, &premises, &forged).is_err(),
            "payload mutation {change}"
        );
    }
}

#[test]
fn missing_native_rows_refuse_instead_of_shrinking_the_projection() {
    let frames = facts();
    for missing in [SyntaxObservation::NAME, AssertionQualification::NAME] {
        let remaining = frames
            .iter()
            .filter(|(name, _)| *name != missing)
            .cloned()
            .collect::<Vec<_>>();
        let native = inventory(&remaining, &budget()).unwrap();
        assert!(native.collect().is_err());
    }
}

#[test]
fn inventory_and_collected_output_retain_and_release_their_reservations() {
    let frames = facts();
    let budget = budget();
    let native = inventory(&frames, &budget).unwrap();
    assert!(budget.reserved() > 0);
    let output = native.collect().unwrap();
    drop(native);
    assert!(budget.reserved() > 0);
    drop(output);
    assert_eq!(budget.reserved(), 0);
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(inventory(&frames, &tiny).is_err());
    assert_eq!(tiny.reserved(), 0);
}

#[test]
fn inventory_inputs_have_only_native_authority_and_require_native_support_validation() {
    let inputs = NativeInventory::inputs();
    assert_eq!(inputs.len(), 105); // 52 native pairs plus the shared qualification.
    assert!(
        inputs
            .iter()
            .any(|input| { input.name() == SignatureEnumerationObservation::NAME })
    );
    assert!(
        inputs
            .iter()
            .any(|input| { input.name() == SignatureEnumerationSupport::NAME })
    );
    let facts = lctx_model::domain::facts_relations();
    assert!(
        inputs
            .iter()
            .all(|input| facts.iter().any(|relation| relation.name() == input.name()))
    );
    assert_eq!(
        NativeInventory::stage_inputs(lctx_model::domain::stages::Profile::Behavioral).len(),
        inputs.len()
    );
    let premise = NativeAssertionPremise::SyntaxObservation {
        assertion: nominal(1),
        support: nominal(2),
    };
    let proof = premise.proof().unwrap();
    assert_eq!(proof.premises.len(), 2);
    assert_eq!(proof.premises[0].relation(), SyntaxObservation::NAME);
    assert_eq!(proof.premises[1].relation(), SyntaxSupport::NAME);
}
