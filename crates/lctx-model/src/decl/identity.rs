//! Identity recompute (DESIGN §15.2–§15.3): a declared relation's id column must equal its recipe
//! over exactly the declared identity inputs. The encoding is [`RecipeField`]'s, read from Arrow
//! columns, so a producer, a validator and the `lctx_id_v2` UDF agree.
//!
//! [`RecipeField`]: crate::id::RecipeField

use arrow_array::cast::AsArray;
use arrow_array::types::{Int16Type, Int64Type};
use arrow_array::{Array, RecordBatch};
use arrow_schema::{ArrowError, DataType};

use crate::decl::relation::RelationDecl;
use crate::id::{Id, IdHasher};

/// Recompute every row's id from the declared identity inputs. `None` when the relation declares
/// no identity.
pub fn recompute(decl: &RelationDecl, batch: &RecordBatch) -> Result<Option<Vec<Id>>, ArrowError> {
    let Some(identity) = decl.identity else {
        return Ok(None);
    };
    let inputs = identity
        .inputs
        .iter()
        .map(|name| {
            batch.column_by_name(name).cloned().ok_or_else(|| {
                ArrowError::SchemaError(format!("{}: no identity input {name}", decl.name))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let seed = IdHasher::v2(identity.recipe.kind);
    (0..batch.num_rows())
        .map(|row| {
            let mut h = seed.clone();
            for column in &inputs {
                feed(&mut h, column.as_ref(), row)?;
            }
            Ok(h.finish_id())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// The rows whose stored id differs from the recomputed one.
pub fn mismatches(decl: &RelationDecl, batch: &RecordBatch) -> Result<Vec<usize>, ArrowError> {
    let Some(identity) = decl.identity else {
        return Ok(Vec::new());
    };
    let Some(expected) = recompute(decl, batch)? else {
        return Ok(Vec::new());
    };
    let stored = batch
        .column_by_name(identity.column)
        .ok_or_else(|| ArrowError::SchemaError(format!("{}: no id column", decl.name)))?;
    let stored = stored.as_fixed_size_binary_opt().ok_or_else(|| {
        ArrowError::SchemaError(format!("{}: id column is not FixedSizeBinary", decl.name))
    })?;
    Ok(expected
        .iter()
        .enumerate()
        .filter(|(row, id)| stored.is_null(*row) || stored.value(*row) != id.0)
        .map(|(row, _)| row)
        .collect())
}

/// One field in the presence-byte encoding of its Arrow type.
fn feed(h: &mut IdHasher, column: &dyn Array, row: usize) -> Result<(), ArrowError> {
    if column.is_null(row) {
        h.bytes(&[0]);
        return Ok(());
    }
    h.bytes(&[1]);
    match column.data_type() {
        DataType::Utf8 => {
            h.str(column.as_string::<i32>().value(row));
        }
        DataType::Int64 => {
            h.i64(column.as_primitive::<Int64Type>().value(row));
        }
        DataType::Int16 => {
            h.i64(i64::from(column.as_primitive::<Int16Type>().value(row)));
        }
        DataType::Boolean => {
            h.bool(column.as_boolean().value(row));
        }
        DataType::FixedSizeBinary(_) => {
            h.bytes(column.as_fixed_size_binary().value(row));
        }
        DataType::Binary => {
            h.bytes(column.as_binary::<i32>().value(row));
        }
        other => {
            return Err(ArrowError::InvalidArgumentError(format!(
                "identity input of type {other} has no encoding"
            )));
        }
    }
    Ok(())
}
