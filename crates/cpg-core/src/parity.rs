//! The parity harness (cutover plan §3.4–§3.6): exact multiset comparison of legacy and new
//! relations, with declared divergences applied as DataFusion predicates over the differing rows,
//! and SQL adapters that reproduce legacy relations (legacy identities through `legacy_ids_*`).
//! Parity is migration evidence, never a semantic control (CI-12).

use std::sync::Arc;

use arrow_array::{ArrayRef, FixedSizeBinaryArray, Float64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use datafusion::error::Result;
use datafusion::prelude::SessionContext;
use lctx_model::legacy::parity::{self, RelationVerdict, Report};
use lctx_model::legacy::{AdapterDecl, Computation, Divergence, Side};

use crate::CoreError;

const SAMPLES: usize = 3;

async fn count(ctx: &SessionContext, sql: &str) -> Result<usize> {
    let batches = crate::sql::query(ctx, sql).await?.collect().await?;
    Ok(batches
        .first()
        .map(|b| {
            b.column(0)
                .as_any()
                .downcast_ref::<arrow_array::Int64Array>()
                .map_or(0, |a| a.value(0) as usize)
        })
        .unwrap_or(0))
}

/// The predicate a side's declared divergences admit, or `FALSE`.
fn declared(divergences: &[Divergence], side: Side) -> String {
    let parts: Vec<String> = divergences
        .iter()
        .filter(|d| d.side == side)
        .map(|d| format!("COALESCE(({}), FALSE)", d.predicate))
        .collect();
    if parts.is_empty() {
        "FALSE".into()
    } else {
        parts.join(" OR ")
    }
}

/// Compare one relation's legacy and new rows under its declared divergences.
pub async fn compare(
    relation: &str,
    legacy: &RecordBatch,
    new: &RecordBatch,
    divergences: &[Divergence],
) -> Result<RelationVerdict> {
    let diff = match parity::diff(relation, legacy, new) {
        Ok(diff) => diff,
        Err(e) => {
            return Ok(RelationVerdict {
                relation: relation.to_owned(),
                legacy_rows: legacy.num_rows(),
                new_rows: new.num_rows(),
                error: Some(e.to_string()),
                ..Default::default()
            });
        }
    };
    let mut verdict = RelationVerdict {
        relation: relation.to_owned(),
        legacy_rows: diff.legacy_rows,
        new_rows: diff.new_rows,
        legacy_only: diff.legacy_only.num_rows(),
        new_only: diff.new_only.num_rows(),
        ..Default::default()
    };
    if diff.equal() {
        return Ok(verdict);
    }
    let ctx = crate::session::session();
    ctx.register_batch("legacy_only", diff.legacy_only.clone())?;
    ctx.register_batch("new_only", diff.new_only.clone())?;
    for (table, side, rows) in [
        ("legacy_only", Side::Legacy, diff.legacy_only.num_rows()),
        ("new_only", Side::New, diff.new_only.num_rows()),
    ] {
        if rows == 0 {
            continue;
        }
        let predicate = declared(divergences, side);
        let admitted = count(&ctx, &format!("SELECT count(*) FROM {table} WHERE {predicate}")).await?;
        verdict.declared += admitted;
        verdict.undeclared += rows - admitted;
        if admitted < rows {
            let sample = crate::sql::query(
                &ctx,
                &format!("SELECT * FROM {table} WHERE NOT ({predicate}) LIMIT {SAMPLES}"),
            )
            .await?
            .collect()
            .await?;
            if let Ok(text) = datafusion::arrow::util::pretty::pretty_format_batches(&sample) {
                verdict.samples.push(format!("{table}:\n{text}"));
            }
        }
    }
    Ok(verdict)
}

/// Compute an adapter's legacy relation over its registered inputs.
pub async fn run_adapter(ctx: &SessionContext, adapter: &AdapterDecl) -> Result<RecordBatch, CoreError> {
    let Computation::Sql(sql) = adapter.computation else {
        return Err(CoreError::Analysis(format!(
            "adapter {} computes in Rust; call its function",
            adapter.produces
        )));
    };
    let frame = crate::sql::query(ctx, sql).await?;
    let schema = Arc::new(frame.schema().as_arrow().clone());
    let batches = frame.collect().await?;
    Ok(arrow_select::concat::concat_batches(&schema, &batches)?)
}

/// One self-test case: a named report and whether it must pass.
pub struct Case {
    pub name: &'static str,
    pub report: Report,
    pub must_pass: bool,
}

impl Case {
    pub fn behaved(&self) -> bool {
        self.report.passed() == self.must_pass
    }
}

fn id(n: u8) -> [u8; 16] {
    [n; 16]
}

fn sample_batch(ids: &[[u8; 16]], labels: &[&str], scores: &[f64]) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("occurrence_id", DataType::FixedSizeBinary(16), false),
        Field::new("label", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]));
    let columns: Vec<ArrayRef> = vec![
        Arc::new(FixedSizeBinaryArray::try_from_iter(ids.iter()).expect("16 bytes")),
        Arc::new(StringArray::from(labels.to_vec())),
        Arc::new(Float64Array::from(scores.to_vec())),
    ];
    RecordBatch::try_new(schema, columns).expect("a sample batch")
}

fn mapping(pairs: &[([u8; 16], [u8; 16])]) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("new_id", DataType::FixedSizeBinary(16), false),
        Field::new("legacy_id", DataType::FixedSizeBinary(16), false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(FixedSizeBinaryArray::try_from_iter(pairs.iter().map(|p| p.0)).expect("16 bytes")),
            Arc::new(FixedSizeBinaryArray::try_from_iter(pairs.iter().map(|p| p.1)).expect("16 bytes")),
        ],
    )
    .expect("a mapping batch")
}

const RELABELLED: Divergence = Divergence {
    name: "relabelled",
    finding: "self-test",
    reason: "the new producer renames one label on purpose",
    side: Side::New,
    predicate: "label = 'renamed'",
};
const RELABELLED_LEGACY: Divergence = Divergence {
    side: Side::Legacy,
    predicate: "label = 'b'",
    ..RELABELLED
};

const ADAPTER: AdapterDecl = AdapterDecl {
    produces: "samples",
    inputs: &["new_samples", "legacy_ids_samples"],
    computation: Computation::Sql(
        "SELECT m.legacy_id AS occurrence_id, n.label, n.score \
         FROM new_samples n JOIN legacy_ids_samples m ON m.new_id = n.occurrence_id",
    ),
    quirks: &[],
    divergences: &[],
    delete_in_phase: 5,
};

/// The harness's own controls (cutover plan WP0.8): identical relations pass; an injected
/// difference fails; declaring it passes; a changed multiplicity fails; a legacy-id adapter
/// reproduces legacy ids exactly, and a corrupted mapping fails.
pub async fn self_test() -> Result<Vec<Case>, CoreError> {
    let report = |relations| Report {
        phase: 0,
        corpus: "self-test".into(),
        relations,
    };
    let legacy = sample_batch(&[id(1), id(2), id(3)], &["a", "b", "c"], &[f64::NAN, -0.0, 1.5]);
    let mut cases = Vec::new();
    cases.push(Case {
        name: "identical relations pass (NaN and -0.0 by value)",
        report: report(vec![compare("samples", &legacy, &legacy, &[]).await?]),
        must_pass: true,
    });
    let injected = sample_batch(&[id(1), id(2), id(3)], &["a", "renamed", "c"], &[f64::NAN, -0.0, 1.5]);
    cases.push(Case {
        name: "an injected difference fails",
        report: report(vec![compare("samples", &legacy, &injected, &[]).await?]),
        must_pass: false,
    });
    cases.push(Case {
        name: "the same difference, declared on both sides, passes",
        report: report(vec![
            compare("samples", &legacy, &injected, &[RELABELLED, RELABELLED_LEGACY]).await?,
        ]),
        must_pass: true,
    });
    cases.push(Case {
        name: "a declaration on one side only still fails the other side",
        report: report(vec![compare("samples", &legacy, &injected, &[RELABELLED]).await?]),
        must_pass: false,
    });
    let doubled = sample_batch(&[id(1), id(1), id(2), id(3)], &["a", "a", "b", "c"], &[f64::NAN, f64::NAN, -0.0, 1.5]);
    cases.push(Case {
        name: "a changed multiplicity fails",
        report: report(vec![compare("samples", &doubled, &legacy, &[]).await?]),
        must_pass: false,
    });
    // The new producer keys rows by new ids; the adapter reproduces legacy ids through the
    // side relation.
    let new_rows = sample_batch(&[id(11), id(12), id(13)], &["a", "b", "c"], &[f64::NAN, -0.0, 1.5]);
    for (name, pairs, must_pass) in [
        (
            "a legacy-id adapter reproduces legacy ids exactly",
            [(id(11), id(1)), (id(12), id(2)), (id(13), id(3))],
            true,
        ),
        (
            "a corrupted id mapping fails",
            [(id(11), id(2)), (id(12), id(1)), (id(13), id(3))],
            false,
        ),
    ] {
        let ctx = crate::session::session();
        ctx.register_batch("new_samples", new_rows.clone())?;
        ctx.register_batch("legacy_ids_samples", mapping(&pairs))?;
        let adapted = run_adapter(&ctx, &ADAPTER).await?;
        cases.push(Case {
            name,
            report: report(vec![compare("samples", &legacy, &adapted, ADAPTER.divergences).await?]),
            must_pass,
        });
    }
    Ok(cases)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn the_self_test_controls_behave() {
        let cases = super::self_test().await.unwrap();
        assert_eq!(cases.len(), 7);
        for case in &cases {
            assert!(case.behaved(), "{}: {:#?}", case.name, case.report.relations);
        }
    }
}
