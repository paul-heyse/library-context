//! The model's call-policy SQL equals its Rust evaluator on every combination of target facts
//! (cutover plan WP0.4): one predicate, two renderings, no disagreement.

use std::sync::Arc;

use arrow_array::{ArrayRef, BooleanArray, Int16Array, Int64Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;
use lctx_model::calls::{CallOrigin, CallPhase, CallPolicy, Modality, TargetFacts, TargetKind};
use lctx_model::decl::codebook::Codebook;

fn every_fact() -> Vec<TargetFacts> {
    let mut out = Vec::new();
    for &modality in Modality::all() {
        for &origin in CallOrigin::all() {
            for &phase in CallPhase::all() {
                for &target_kind in TargetKind::all() {
                    for implicit in [false, true] {
                        for unique in [false, true] {
                            for complete in [false, true] {
                                out.push(TargetFacts { modality, origin, phase, target_kind, implicit, unique, complete });
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

#[tokio::test]
async fn every_policy_view_admits_exactly_what_rust_admits() {
    let facts = every_fact();
    let codes = |f: fn(&TargetFacts) -> i16| -> ArrayRef {
        Arc::new(Int16Array::from_iter_values(facts.iter().map(f)))
    };
    let flags = |f: fn(&TargetFacts) -> bool| -> ArrayRef {
        Arc::new(BooleanArray::from_iter(facts.iter().map(|x| Some(f(x)))))
    };
    let schema = Arc::new(Schema::new(vec![
        Field::new("idx", DataType::Int64, false),
        Field::new("modality", DataType::Int16, false),
        Field::new("origin", DataType::Int16, false),
        Field::new("phase", DataType::Int16, false),
        Field::new("target_kind", DataType::Int16, false),
        Field::new("implicit", DataType::Boolean, false),
        Field::new("is_unique", DataType::Boolean, false),
        Field::new("is_complete", DataType::Boolean, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int64Array::from_iter_values(0..facts.len() as i64)),
            codes(|f| f.modality.code()),
            codes(|f| f.origin.code()),
            codes(|f| f.phase.code()),
            codes(|f| f.target_kind.code()),
            flags(|f| f.implicit),
            flags(|f| f.unique),
            flags(|f| f.complete),
        ],
    )
    .unwrap();
    let ctx = SessionContext::new();
    ctx.register_table("call_targets", Arc::new(MemTable::try_new(schema, vec![vec![batch]]).unwrap()))
        .unwrap();
    for policy in CallPolicy::ALL {
        let sql = format!("SELECT idx FROM call_targets WHERE {} ORDER BY idx", policy.sql());
        let rows = ctx.sql(&sql).await.unwrap().collect().await.unwrap();
        let from_sql: Vec<i64> = rows
            .iter()
            .flat_map(|b| {
                b.column(0)
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .unwrap()
                    .values()
                    .to_vec()
            })
            .collect();
        let from_rust: Vec<i64> = facts
            .iter()
            .enumerate()
            .filter(|(_, f)| policy.admits(f))
            .map(|(i, _)| i as i64)
            .collect();
        assert_eq!(from_sql, from_rust, "{}: {sql}", policy.name());
    }
}
