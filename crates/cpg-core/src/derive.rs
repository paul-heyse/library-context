//! Stage C/D (DESIGN §4.1, §4.3): run a derivation's query over the snapshot session, then cast the
//! result strictly to the declared schema and sort it canonically, so a derived table passes the
//! same local type check as a raw one before it reaches `DeltaTable::write`.

use std::sync::Arc;

use arrow_array::builder::FixedSizeBinaryBuilder;
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use cpg_schema::derived::Derived;
use cpg_schema::id::Id;
use cpg_schema::table::canonical_sort;
use datafusion::prelude::SessionContext;

use crate::delta::to_declared;
use crate::{CoreError, sql};

fn snapshot_column(snapshot_id: Id, rows: usize) -> Result<ArrayRef, CoreError> {
    let mut b = FixedSizeBinaryBuilder::with_capacity(rows, 16);
    for _ in 0..rows {
        b.append_value(snapshot_id.0)?;
    }
    Ok(Arc::new(b.finish()))
}

/// The derived table `T` of this snapshot, as a declared, canonically sorted batch.
pub async fn derive<T: Derived>(
    ctx: &SessionContext,
    snapshot_id: Id,
) -> Result<RecordBatch, CoreError> {
    let schema = T::schema();
    let mut declared = Vec::new();
    for batch in sql::query(ctx, &T::sql()).await?.collect().await? {
        let mut fields = vec![Arc::new(Field::new(
            "snapshot_id",
            DataType::FixedSizeBinary(16),
            false,
        ))];
        fields.extend(batch.schema().fields().iter().cloned());
        let mut columns = vec![snapshot_column(snapshot_id, batch.num_rows())?];
        columns.extend(batch.columns().iter().cloned());
        let with_snapshot = RecordBatch::try_new(Arc::new(Schema::new(fields)), columns)?;
        declared.push(to_declared::<T>(&with_snapshot)?);
    }
    let batch = arrow_select::concat::concat_batches(&schema, &declared)?;
    Ok(canonical_sort(&batch, T::key())?)
}
