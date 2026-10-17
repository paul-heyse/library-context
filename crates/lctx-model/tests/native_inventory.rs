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
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
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
    let invariant =
        lctx_model::domain::validation::invariants_for::<NativeQualification>().remove(0);
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
fn borrowed_pairs_match_complete_projection_without_a_second_inventory() {
    let frames = facts();
    let budget = budget();
    let native = inventory(&frames, &budget).unwrap();
    let retained = budget.reserved();
    let output = native.collect().unwrap();
    assert!(budget.reserved() > retained);
    // Qualification identity order is independent of premise identity order. Compare
    // each authoritative pair by its own ID instead of assuming the maps align.
    for pair in native.pairs() {
        let (premise, qualification) = pair.unwrap();
        assert_eq!(output.premises.get(premise.id()), Some(premise));
        assert_eq!(
            output.qualifications.get(qualification.id()),
            Some(&qualification)
        );
    }
    drop(output);
    assert_eq!(budget.reserved(), retained);
    native.for_each(|_, _| Ok(())).unwrap();
    assert_eq!(native.pairs().count(), 3);
    assert_eq!(budget.reserved(), retained);
    drop(native);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn borrowed_pairs_preserve_complete_projection_refusal() {
    let frames = facts();
    for missing in [SyntaxObservation::NAME, AssertionQualification::NAME] {
        let remaining = frames
            .iter()
            .filter(|(name, _)| *name != missing)
            .cloned()
            .collect::<Vec<_>>();
        let native = inventory(&remaining, &budget()).unwrap();
        assert!(native.for_each(|_, _| Ok(())).is_err());
        assert!(native.pairs().any(|pair| pair.is_err()));
    }
}

#[test]
fn inventory_inputs_have_only_native_authority_and_require_native_support_validation() {
    let inputs = NativeInventory::inputs();
    let stored = NativeInventory::stage_inputs(lctx_model::domain::stages::Profile::Behavioral);
    let mut expected = std::collections::BTreeSet::from([AssertionQualification::NAME]);
    macro_rules! pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        assert!(expected.insert(<$assertion>::NAME));
        assert!(expected.insert(<$support>::NAME));
        let support = stored.iter().find(|i| i.name() == <$support>::NAME).unwrap();
        assert!(support.validators().contains(&<$support>::NAME), "native support validation remains required");
    )*};}
    lctx_model::native_analysis_pairs!(pairs);
    assert_eq!(inputs.len(), expected.len());
    assert_eq!(
        inputs
            .iter()
            .map(|i| i.name())
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
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

fn properties(name: &str, batch: &RecordBatch) -> RecordBatch {
    let positions = NativeInventory::property_columns(name)
        .unwrap()
        .iter()
        .map(|name| batch.schema().index_of(name).unwrap())
        .collect::<Vec<_>>();
    batch.project(&positions).unwrap()
}

#[test]
fn admitted_properties_preserve_exact_pairs_fidelity_alternatives_and_release() {
    let frames = facts();
    let expected = result(&frames);
    let syntax = SyntaxObservation::decode(&frames[1].1).unwrap().remove(0);
    let syntax_supports = SyntaxSupport::decode(&frames[2].1).unwrap();
    let doc = DocumentObservation::decode(&frames[3].1).unwrap().remove(0);
    let doc_support = DocumentSupport::decode(&frames[4].1).unwrap().remove(0);
    let expected_premises = [
        NativeAssertionPremise::SyntaxObservation {
            assertion: syntax.id(),
            support: syntax_supports[0].id(),
        },
        NativeAssertionPremise::SyntaxObservation {
            assertion: syntax.id(),
            support: syntax_supports[1].id(),
        },
        NativeAssertionPremise::DocumentObservation {
            assertion: doc.id(),
            support: doc_support.id(),
        },
    ];
    assert_eq!(
        expected
            .0
            .iter()
            .map(Record::id)
            .collect::<std::collections::BTreeSet<_>>(),
        expected_premises.iter().map(Record::id).collect()
    );
    let budget = budget();
    let mut compact = NativeInventory::new(&budget);
    for (name, batch) in &frames {
        compact
            .visit_properties(name, &properties(name, batch))
            .unwrap();
    }
    let output = compact.collect().unwrap();
    assert_eq!(
        output.premises.iter().cloned().collect::<Vec<_>>(),
        expected.0
    );
    assert_eq!(
        output.qualifications.iter().cloned().collect::<Vec<_>>(),
        expected.1
    );
    drop(output);
    drop(compact);
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn every_native_adapter_declares_exact_nonnullable_sufficient_properties() {
    let mut count = 0;
    macro_rules! pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        for name in [<$assertion>::NAME, <$support>::NAME] {
            let schema = NativeInventory::property_schema(name).unwrap();
            assert!(schema.fields().iter().all(|field| !field.is_nullable()));
            let mut inventory = NativeInventory::new(&budget());
            inventory.visit_properties(name, &RecordBatch::new_empty(schema)).unwrap();
            inventory.for_each(|_, _| Ok(())).unwrap();
        }
        assert_eq!(NativeInventory::property_columns(<$assertion>::NAME).unwrap(), &["id", "qualification"]);
        assert_eq!(NativeInventory::property_columns(<$support>::NAME).unwrap(), &["id", "assertion", "fidelity"]);
        count += 1;
    )*};}
    lctx_model::native_analysis_pairs!(pairs);
    assert_eq!(count, 75);
}

#[test]
fn sufficient_properties_refuse_missing_foreign_qualification_and_late_support() {
    let frames = facts();
    for missing in [SyntaxObservation::NAME, AssertionQualification::NAME] {
        let mut inventory = NativeInventory::new(&budget());
        for (name, batch) in &frames {
            if *name != missing {
                inventory
                    .visit_properties(name, &properties(name, batch))
                    .unwrap();
            }
        }
        assert!(inventory.collect().is_err());
    }
    let mut inventory = NativeInventory::new(&budget());
    for (name, batch) in &frames {
        inventory
            .visit_properties(name, &properties(name, batch))
            .unwrap();
    }
    let mut support = SyntaxSupport::decode(&frames[2].1).unwrap().remove(0);
    support.assertion = nominal(233);
    inventory
        .visit_properties(
            SyntaxSupport::NAME,
            &properties(
                SyntaxSupport::NAME,
                &SyntaxSupport::encode(&[support]).unwrap(),
            ),
        )
        .unwrap();
    assert!(inventory.for_each(|_, _| Ok(())).is_err());
    let mut qualification = AssertionQualification::decode(&frames[0].1)
        .unwrap()
        .remove(0);
    qualification.assumptions = nominal(234);
    let mut inventory = NativeInventory::new(&budget());
    assert!(
        inventory
            .visit_properties(
                AssertionQualification::NAME,
                &properties(
                    AssertionQualification::NAME,
                    &AssertionQualification::encode(&[qualification]).unwrap()
                )
            )
            .is_err()
    );
}

#[test]
fn sufficient_properties_reject_wrong_schema_null_width_and_unknown_fidelity() {
    use arrow_array::{ArrayRef, FixedSizeBinaryArray, Int16Array};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    let frames = facts();
    let (name, batch) = &frames[2];
    let compact = properties(name, batch);
    let mut inventory = NativeInventory::new(&budget());
    assert!(
        inventory.visit_properties(name, batch).is_err(),
        "rich input is not silently accepted as projected data"
    );
    let mut columns = compact.columns().to_vec();
    columns[2] = Arc::new(Int16Array::from(vec![i16::MAX; compact.num_rows()]));
    assert!(
        inventory
            .visit_properties(
                name,
                &RecordBatch::try_new(compact.schema(), columns).unwrap()
            )
            .is_err()
    );
    let mut columns = compact.columns().to_vec();
    columns[0] = Arc::new(
        FixedSizeBinaryArray::try_from_sparse_iter_with_size(
            std::iter::repeat_n(None::<&[u8]>, compact.num_rows()),
            16,
        )
        .unwrap(),
    );
    let mut fields = compact
        .schema()
        .fields()
        .iter()
        .map(|field| field.as_ref().clone())
        .collect::<Vec<_>>();
    fields[0] = Field::new("id", DataType::FixedSizeBinary(16), true);
    assert!(
        inventory
            .visit_properties(
                name,
                &RecordBatch::try_new(Arc::new(Schema::new(fields)), columns).unwrap()
            )
            .is_err()
    );
    let mut columns = compact.columns().to_vec();
    columns[0] = Arc::new(
        FixedSizeBinaryArray::try_from_iter(std::iter::repeat_n([0u8; 32], compact.num_rows()))
            .unwrap(),
    ) as ArrayRef;
    let mut fields = compact
        .schema()
        .fields()
        .iter()
        .map(|field| field.as_ref().clone())
        .collect::<Vec<_>>();
    fields[0] = Field::new("id", DataType::FixedSizeBinary(32), false);
    assert!(
        inventory
            .visit_properties(
                name,
                &RecordBatch::try_new(Arc::new(Schema::new(fields)), columns).unwrap()
            )
            .is_err()
    );
}

#[test]
fn independent_receiving_still_reidentifies_rich_payload_before_projection() {
    let mut frames = facts();
    let batch = &mut frames[1].1;
    let id = batch.column_by_name("id").unwrap().clone();
    let mut rows = SyntaxObservation::decode(batch).unwrap();
    rows[0].spelling = "forged while retaining plausible old ID".into();
    let changed = SyntaxObservation::encode(&rows).unwrap();
    let mut columns = changed.columns().to_vec();
    columns[0] = id;
    *batch = RecordBatch::try_new(changed.schema(), columns).unwrap();
    assert!(inventory(&frames, &budget()).is_err());
    assert!(validate(&frames, &[], &[]).is_err());
}

#[test]
fn compact_native_selection_keeps_profile_filter_and_full_support_validators() {
    use lctx_model::domain::assertion::Assertion;
    use lctx_model::domain::stages::Profile;
    let catalog = NativeInventory::stage_inputs(Profile::Catalog);
    let behavioral = NativeInventory::stage_inputs(Profile::Behavioral);
    macro_rules! pairs {($($code:literal:$variant:ident=>$assertion:ty,$support:ty;)*)=>{$(
        let present = <$assertion as Assertion>::FAMILY != FactFamily::Flow;
        assert_eq!(catalog.iter().any(|input| input.name() == <$assertion>::NAME), present);
        assert_eq!(catalog.iter().any(|input| input.name() == <$support>::NAME), present);
        let support = behavioral.iter().find(|input| input.name() == <$support>::NAME).unwrap();
        assert!(support.validators().contains(&<$support>::NAME));
    )*};}
    lctx_model::native_analysis_pairs!(pairs);
}

#[test]
fn sufficient_properties_preserve_qualified_alternatives_and_reject_foreign_basis() {
    let mut frames = facts();
    let mut q = AssertionQualification::decode(&frames[0].1)
        .unwrap()
        .remove(0);
    q.condition = nominal(240);
    let mut assertion = SyntaxObservation::decode(&frames[1].1).unwrap().remove(0);
    assertion.qualification = q.id();
    let mut support = SyntaxSupport::decode(&frames[2].1).unwrap().remove(0);
    support.assertion = assertion.id();
    frames.extend([frame(&[q.clone()]), frame(&[assertion]), frame(&[support])]);
    let expected = result(&frames);
    assert_eq!(expected.0.len(), 4);
    let mut compact = NativeInventory::new(&budget());
    for (name, batch) in &frames {
        compact
            .visit_properties(name, &properties(name, batch))
            .unwrap();
    }
    let output = compact.collect().unwrap();
    assert_eq!(
        output.qualifications.iter().cloned().collect::<Vec<_>>(),
        expected.1
    );
    assert_eq!(
        output
            .qualifications
            .iter()
            .map(|row| row.qualification)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    let remaining = frames
        .iter()
        .take(frames.len() - 3)
        .chain(frames.iter().skip(frames.len() - 2));
    let mut missing = NativeInventory::new(&budget());
    for (name, batch) in remaining {
        missing
            .visit_properties(name, &properties(name, batch))
            .unwrap();
    }
    assert!(missing.collect().is_err());
}

#[test]
fn compact_authority_requires_model_owned_transitive_native_facts_closure() {
    use lctx_model::domain::stages::{Profile, PublicationBoundary};
    let model = model().unwrap();
    for profile in Profile::ALL {
        let inputs = NativeInventory::checked_inputs(profile, &model).unwrap();
        assert!(
            inputs.len() > NativeInventory::stage_inputs(profile).len(),
            "attribution, qualifications and references remain semantic prerequisites"
        );
        assert!(
            inputs.iter().all(|input| input.prefix().is_none()
                || input.prefix() == Some(PublicationBoundary::Facts))
        );
        for direct in NativeInventory::stage_inputs(profile) {
            assert!(inputs.iter().any(|input| input.name() == direct.name()));
        }
        for required in [
            attribution::ProviderRun::NAME,
            attribution::AnalysisContext::NAME,
            assertion::ProviderSurface::NAME,
            source::SourceArtifact::NAME,
        ] {
            assert!(
                inputs.iter().any(|input| input.name() == required),
                "native closure cannot omit {required}"
            );
        }
    }
}
