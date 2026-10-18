use super::*;
use lctx_model::domain::{self as d, Record};

fn row(ordinal: i64, value: f64) -> Value {
    let quality = d::analytics::QualityStep {
        run: serde_json::from_value(serde_json::to_value([7u8; 16]).unwrap()).unwrap(),
        ordinal,
        value: d::FiniteF64::new(value).unwrap(),
    };
    let view = crate::codec::views(std::slice::from_ref(&quality))
        .unwrap()
        .remove(0);
    let canonical = serde_json::to_vec(&view.body).unwrap();
    let content = ContentHash::of(&canonical);
    crate::adapter::physical_row(
        crate::loader::payload_id(
            "compiler_record",
            d::analytics::QualityStep::NAME,
            quality.id().bytes(),
            content,
        )
        .unwrap(),
        content,
        canonical,
        None,
        view,
    )
    .unwrap()
}
fn mut_object(value: &mut Value) -> &mut Object {
    let Value::Object(object) = value else {
        panic!("fixture object");
    };
    object
}
fn budget() -> ResourceBudget {
    ResourceBudget::fixed(32 << 20).unwrap()
}

#[test]
fn block_selected_logical_row_is_independent_of_packing_and_other_slots() {
    let budget = budget();
    let a = row(0, 0.25);
    let b = row(1, 0.75);
    let contribution = ContentHash::of(b"packing-independent-selector");
    {
        let single = PreparedBlocks::prepare(contribution, vec![a.clone()], &budget).unwrap();
        let pair = PreparedBlocks::prepare(contribution, vec![b, a.clone()], &budget).unwrap();
        assert_eq!(single.blocks().len(), 1);
        assert_eq!(pair.blocks().len(), 1);
        assert_ne!(single.blocks()[0], pair.blocks()[0]);
        validate_block(&single.blocks()[0], &budget).unwrap();
        validate_block(&pair.blocks()[0], &budget).unwrap();
        let locator = pair
            .locators()
            .iter()
            .find(|locator| {
                locator.as_object().unwrap().get("node") == a.as_object().unwrap().get("id")
            })
            .unwrap();
        assert_eq!(selected_row(&pair.blocks()[0], locator).unwrap(), &a);
        assert_eq!(
            logical_membership(&single.locators()[0]).unwrap(),
            logical_membership(locator).unwrap()
        );
        // A named locator selects one semantic member, even when its container has siblings.
        assert_ne!(pair.locators().len(), single.locators().len());
        assert!(selected_row(&single.blocks()[0], locator).is_err());
    }
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn block_framing_and_selected_correspondence_refuse_independent_corruption() {
    let budget = budget();
    let packet =
        PreparedBlocks::prepare(ContentHash::of(b"corruption"), vec![row(0, 0.25)], &budget)
            .unwrap();
    let mut bad = packet.blocks()[0].clone();
    mut_object(&mut bad).insert("content", ContentHash::of(b"wrong-block").hex());
    assert!(validate_block(&bad, &budget).is_err());
    let mut bad = packet.locators()[0].clone();
    mut_object(&mut bad).insert("slot", 1i64);
    assert!(selected_row(&packet.blocks()[0], &bad).is_err());
    let mut bad = packet.locators()[0].clone();
    mut_object(&mut bad).insert("content", ContentHash::of(b"wrong-selected-content").hex());
    assert!(selected_row(&packet.blocks()[0], &bad).is_err());
    let mut bad = packet.locators()[0].clone();
    mut_object(&mut bad).insert("unexpected", true);
    assert!(validate_locator(&bad).is_err());
    assert!(PhysicalLocation::graph(RecordId::new("compiler_record", "virtual")).is_err());
    assert!(PhysicalLocation::block(RecordId::new("compiler_membership", "locator"), 0).is_err());
}

#[test]
fn block_packets_deduplicate_full_equals_and_reject_nominal_conflicts() {
    let budget = budget();
    let a = row(0, 0.25);
    let contribution = ContentHash::of(b"nominal-conflict");
    let packet =
        PreparedBlocks::prepare(contribution, vec![a.clone(), a.clone()], &budget).unwrap();
    assert_eq!(packet.locators().len(), 1);
    assert!(PreparedBlocks::prepare(contribution, vec![a, row(0, 0.75)], &budget).is_err());
}

#[test]
fn block_framing_refuses_nonfinite_values_in_an_unselected_nested_body() {
    let budget = budget();
    for nonfinite in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let selected = row(0, 0.25);
        let mut malformed = row(1, 0.75);
        let mut nested = Object::new();
        nested.insert(
            "nested",
            Value::Array(vec![Value::Number(surrealdb::types::Number::Float(nonfinite))].into()),
        );
        let object = mut_object(&mut malformed);
        let Some(Value::Object(body)) = object.get_mut("body") else {
            panic!("fixture body");
        };
        body.insert("value", Value::Object(nested));
        let canonical = serde_json::to_vec(&Value::Object(body.clone())).unwrap();
        let content = ContentHash::of(&canonical);
        let Some(Value::String(key)) = object.get("semantic_key") else {
            panic!("fixture key");
        };
        let key = crate::completed_rows::decode_nominal_key(key).unwrap();
        object.insert(
            "id",
            crate::loader::payload_id(
                "compiler_record",
                d::analytics::QualityStep::NAME,
                &key,
                content,
            )
            .unwrap(),
        );
        object.insert("content", content.hex());
        object.insert("canonical", surrealdb::types::Bytes::from(canonical));
        let packet = PreparedBlocks::prepare(
            ContentHash::of(b"nonfinite unselected slot"),
            vec![selected.clone(), malformed],
            &budget,
        )
        .unwrap();
        let locator = packet
            .locators()
            .iter()
            .find(|locator| {
                locator.as_object().unwrap().get("node") == selected.as_object().unwrap().get("id")
            })
            .unwrap();
        assert_eq!(
            selected_row(&packet.blocks()[0], locator).unwrap(),
            &selected
        );
        assert!(
            validate_block(&packet.blocks()[0], &budget).is_err(),
            "a legitimate selected slot cannot hide forged nonfinite framing elsewhere in its physical container"
        );
    }
}
