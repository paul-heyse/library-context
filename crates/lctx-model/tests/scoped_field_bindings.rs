//! Model-owned consumed property inventories, independent of compiler tables and native stores.
use lctx_model::domain::{catalog::build::CatalogData, retrieval::build::Data, stages::*, *};
use std::any::TypeId;

type Bind = fn(&ValidatedModel, &[ValidationInput]) -> Result<Vec<Vec<Option<usize>>>, ModelError>;

fn owners() -> [(Vec<ValidationInput>, Bind); 2] {
    [
        (
            CatalogData::validation_inputs(),
            CatalogData::scoped_field_bindings,
        ),
        (Data::inputs(), Data::scoped_field_bindings),
    ]
}

#[test]
fn actual_inventories_bind_every_consumed_field_and_refuse_missing_readers() {
    let model = model().unwrap();
    for (inputs, bind) in owners() {
        let bindings = bind(&model, &inputs).unwrap();
        assert_eq!(bindings.len(), inputs.len());
        let mut consumed = std::collections::BTreeSet::new();
        for (source, row) in bindings.iter().enumerate() {
            let relation = model.relation(inputs[source].name()).unwrap();
            assert_eq!(row.len(), relation.fields().len());
            for (field, target) in relation.fields().iter().zip(row) {
                if let Some(target) = target {
                    let (kind, name) = field.target().unwrap();
                    assert_eq!(inputs[*target].type_id(), kind);
                    assert_eq!(inputs[*target].name(), name);
                    let expected = if is_vocabulary(name) {
                        Some(
                            inputs[source]
                                .prefix()
                                .unwrap_or(PublicationBoundary::Facts),
                        )
                    } else {
                        None
                    };
                    assert_eq!(
                        inputs[*target].prefix(),
                        expected,
                        "{}.{}",
                        relation.name(),
                        field.name()
                    );
                    consumed.insert(*target);
                }
            }
        }
        assert!(!consumed.is_empty());
        for missing in consumed {
            let mut available = inputs.clone();
            available.remove(missing);
            assert!(
                bind(&model, &available).is_err(),
                "{} {:?}",
                inputs[missing].name(),
                inputs[missing].prefix()
            );
        }
    }
}

#[test]
fn facts_qualifications_never_fall_back_to_one_other_epoch_or_unknown_reader() {
    let model = model().unwrap();
    for (inputs, bind) in owners() {
        let qualification = inputs
            .iter()
            .position(|input| {
                input.type_id() == TypeId::of::<assertion::AssertionQualification>()
                    && input.prefix() == Some(PublicationBoundary::Facts)
            })
            .unwrap();
        let mut wrong = inputs.clone();
        wrong[qualification] = ValidationInput::of::<assertion::AssertionQualification>(&["id"])
            .at_epoch(PublicationBoundary::Synthesis);
        assert!(bind(&model, &wrong).is_err());
        wrong[qualification] = ValidationInput::of::<assertion::AssertionQualification>(&["id"]);
        assert!(bind(&model, &wrong).is_err());
        let mut duplicate = inputs.clone();
        duplicate.push(inputs[qualification].clone());
        assert!(bind(&model, &duplicate).is_err());
        let mut unknown = inputs.clone();
        unknown.push(
            ValidationInput::of::<embedding::value::FullValue>(&["id"])
                .at_epoch(PublicationBoundary::AnalyticEmbedding),
        );
        assert!(bind(&model, &unknown).is_err());
    }
}

#[test]
fn retrieval_uses_facts_for_first_native_field_and_omits_unconsumed_s0_vocabulary() {
    let model = model().unwrap();
    let inputs = Data::inputs();
    let bindings = Data::scoped_field_bindings(&model, &inputs).unwrap();
    let source = inputs
        .iter()
        .position(|input| input.type_id() == TypeId::of::<syntax::ClassFieldSyntaxObservation>())
        .unwrap();
    let fields = model.relation(inputs[source].name()).unwrap().fields();
    let qualification = fields
        .iter()
        .position(|field| field.name() == "qualification")
        .unwrap();
    let target = bindings[source][qualification].unwrap();
    assert_eq!(inputs[target].prefix(), Some(PublicationBoundary::Facts));
    let documentary = inputs
        .iter()
        .position(|input| {
            input.type_id() == TypeId::of::<synthesis::documentary::DocumentaryConclusion>()
        })
        .unwrap();
    let fields = model.relation(inputs[documentary].name()).unwrap().fields();
    assert_eq!(
        bindings[documentary][fields
            .iter()
            .position(|field| field.name() == "qualification")
            .unwrap()],
        None
    );
    for (index, input) in inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == TypeId::of::<assertion::AssertionQualification>())
    {
        for (field, target) in model
            .relation(input.name())
            .unwrap()
            .fields()
            .iter()
            .zip(&bindings[index])
        {
            if matches!(field.name(), "condition" | "assumptions") {
                assert_eq!(*target, None);
            }
        }
    }
    // Runtime locations are not E0 renderer inputs: adding one without its typed decoder refuses.
    let mut unknown = inputs;
    unknown.push(ValidationInput::of::<local_fields::FieldLocation>(&["id"]));
    assert!(Data::scoped_field_bindings(&model, &unknown).is_err());
}
