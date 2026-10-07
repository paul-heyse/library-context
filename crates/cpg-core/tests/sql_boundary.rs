//! Retained SQL binding/null controls use the active read-only boundary and typed Record codec.
use datafusion::{common::ScalarValue, prelude::SessionContext};
use lctx_model::{Domain, domain::*};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "sql_boundary_items")]
struct Item {
    #[model(key)]
    name: String,
    payload: String,
}

fn fixture() -> (SessionContext, Vec<Item>) {
    let rows = ["a", "o'brien", "o'brien'); DROP TABLE typed_names; --"]
        .into_iter()
        .map(|name| Item {
            name: name.into(),
            payload: "present".into(),
        })
        .collect::<Vec<_>>();
    let ctx = SessionContext::new();
    ctx.register_batch("typed_names", Item::encode(&rows).unwrap())
        .unwrap();
    (ctx, rows)
}

#[tokio::test]
async fn quoted_and_injection_shaped_values_are_bound_and_empty_id_lists_select_nothing() {
    let (ctx, rows) = fixture();
    for row in &rows[1..] {
        let batches = cpg_core::sql::query(&ctx, "SELECT * FROM typed_names WHERE name = $name")
            .await
            .unwrap()
            .with_param_values(vec![("name", ScalarValue::Utf8(Some(row.name.clone())))])
            .unwrap()
            .collect()
            .await
            .unwrap();
        let actual = batches
            .iter()
            .flat_map(|b| Item::decode(b).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(actual.as_slice(), std::slice::from_ref(row));
    }
    for selected in [
        vec![ScalarValue::FixedSizeBinary(
            16,
            Some(rows[1].id().bytes().to_vec()),
        )],
        vec![],
    ] {
        let expected = if selected.is_empty() {
            vec![]
        } else {
            vec![rows[1].clone()]
        };
        let list = ScalarValue::List(ScalarValue::new_list_nullable(
            &selected,
            &arrow_schema::DataType::FixedSizeBinary(16),
        ));
        let batches =
            cpg_core::sql::query(&ctx, "SELECT * FROM typed_names WHERE array_has($ids, id)")
                .await
                .unwrap()
                .with_param_values(vec![("ids", list)])
                .unwrap()
                .collect()
                .await
                .unwrap();
        let actual = batches
            .iter()
            .flat_map(|b| Item::decode(b).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
    let batches = cpg_core::sql::query(&ctx, "SELECT * FROM typed_names")
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(
        batches.iter().map(|b| b.num_rows()).sum::<usize>(),
        rows.len()
    );
}

#[tokio::test]
async fn null_required_payload_cannot_cross_the_typed_record_boundary() {
    let (ctx, _) = fixture();
    let batches = cpg_core::sql::query(
        &ctx,
        "SELECT id, name, CAST(NULL AS VARCHAR) AS payload FROM typed_names",
    )
    .await
    .unwrap()
    .collect()
    .await
    .unwrap();
    assert!(!batches.is_empty());
    for batch in batches {
        assert!(
            Item::decode(&batch).is_err(),
            "required payload accepted SQL null"
        );
    }
    // The active SQL boundary itself refuses writes, even when an actual typed table exists.
    assert!(
        cpg_core::sql::query(&ctx, "DROP TABLE typed_names")
            .await
            .is_err()
    );
}

#[path = "fixtures/native.rs"]
mod native_fixture;
