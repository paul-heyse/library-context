//! Finite necessary nominal demand from already coerced DataFusion expressions.
//! This is pruning only: it neither evaluates residuals nor establishes native authority.
use arrow_schema::{DataType, Schema};
use datafusion::{
    common::ScalarValue,
    logical_expr::{Expr, Operator},
};
use lctx_model::domain::{ModelError, charged::StateCharge, resources::ResourceBudget};

type Key = [u8; 16];

#[derive(Clone, Copy)]
struct Size {
    keys: usize,
    vectors: usize,
}
impl Size {
    fn combine(self, other: Self) -> Option<Self> {
        Some(Self {
            keys: self.keys.checked_add(other.keys)?,
            vectors: self.vectors.checked_add(other.vectors)?,
        })
    }
    fn bytes(self) -> Option<usize> {
        self.keys
            .checked_mul(size_of::<Key>())?
            .checked_add(self.vectors.checked_mul(size_of::<Vec<Key>>())?)?
            .checked_add(size_of::<StateCharge>() + 4 * size_of::<usize>())
    }
}
fn id(expr: &Expr) -> bool {
    matches!(expr, Expr::Column(column) if column.name == "id")
}
// None is an unsupported representation; Some(None) is SQL NULL. A non-null
// value must already have exactly the native nominal type: never strip casts.
fn literal(expr: &Expr) -> Option<Option<Key>> {
    let Expr::Literal(value, _) = expr else {
        return None;
    };
    if value.is_null() {
        return Some(None);
    }
    match value {
        ScalarValue::FixedSizeBinary(16, Some(bytes)) => {
            Some(Some(bytes.as_slice().try_into().ok()?))
        }
        _ => None,
    }
}
fn equal_literal(expr: &Expr) -> Option<Option<Key>> {
    let Expr::BinaryExpr(binary) = expr else {
        return None;
    };
    if binary.op != Operator::Eq {
        return None;
    }
    if id(&binary.left) {
        literal(&binary.right)
    } else if id(&binary.right) {
        literal(&binary.left)
    } else {
        None
    }
}
fn shape(expr: &Expr) -> Option<Size> {
    if let Expr::BinaryExpr(binary) = expr
        && binary.op == Operator::And
    {
        return match (shape(&binary.left), shape(&binary.right)) {
            (Some(left), Some(right)) => left.combine(right),
            (left, right) => left.or(right),
        };
    }
    if let Some(key) = equal_literal(expr) {
        return Some(Size {
            keys: usize::from(key.is_some()),
            vectors: 1,
        });
    }
    if let Expr::InList(list) = expr
        && !list.negated
        && id(&list.expr)
    {
        let mut keys = 0usize;
        for value in &list.list {
            keys = keys.checked_add(usize::from(literal(value)?.is_some()))?;
        }
        return Some(Size { keys, vectors: 1 });
    }
    // In particular OR is not a mandatory branch, even if each side is finite.
    None
}
// This narrower check is only used after the existing native evaluator has
// accepted a filter as Exact. Such a filter can be discharged by exact nominal
// membership itself, instead of hydrating bodies to repeat the same id test.
pub(super) fn only_nominal(expr: &Expr) -> bool {
    if let Expr::BinaryExpr(binary) = expr
        && binary.op == Operator::And
    {
        return only_nominal(&binary.left) && only_nominal(&binary.right);
    }
    shape(expr).is_some()
}
fn nominal_schema(schema: &Schema) -> bool {
    schema
        .field_with_name("id")
        .is_ok_and(|field| field.data_type() == &DataType::FixedSizeBinary(16))
}
pub(super) fn recognizes(expr: &Expr, schema: &Schema) -> bool {
    nominal_schema(schema) && shape(expr).is_some()
}
fn intersect(mut left: Vec<Key>, right: Vec<Key>) -> Vec<Key> {
    left.retain(|key| right.binary_search(key).is_ok());
    left
}
fn materialize(expr: &Expr) -> Option<Vec<Key>> {
    if let Expr::BinaryExpr(binary) = expr
        && binary.op == Operator::And
    {
        return match (materialize(&binary.left), materialize(&binary.right)) {
            (Some(left), Some(right)) => Some(intersect(left, right)),
            (left, right) => left.or(right),
        };
    }
    if let Some(key) = equal_literal(expr) {
        return Some(key.into_iter().collect());
    }
    let Expr::InList(list) = expr else {
        return None;
    };
    let size = shape(expr)?;
    let mut keys = Vec::with_capacity(size.keys);
    for value in &list.list {
        if let Some(key) = literal(value)? {
            keys.push(key);
        }
    }
    keys.sort_unstable();
    keys.dedup();
    Some(keys)
}
/// Admit peak literal/intersection storage before allocating it. The returned charge
/// follows the selected keys through planning and execution. None means no finite
/// demand; Some(empty) remains a real request and must execute view validation.
pub(super) fn extract(
    filters: &[Expr],
    schema: &Schema,
    budget: &ResourceBudget,
) -> Result<Option<(Vec<Key>, StateCharge)>, ModelError> {
    if !nominal_schema(schema) {
        return Ok(None);
    }
    let mut size: Option<Size> = None;
    for expr in filters {
        if let Some(next) = shape(expr) {
            size = Some(match size {
                None => next,
                Some(prior) => prior
                    .combine(next)
                    .ok_or(ModelError::Schema("nominal demand size overflow"))?,
            });
        }
    }
    let Some(size) = size else {
        return Ok(None);
    };
    let bytes = size
        .bytes()
        .ok_or(ModelError::Schema("nominal demand size overflow"))?;
    let mut charge = StateCharge::new(budget, "native nominal filter demand");
    charge.grow(bytes)?;
    let mut keys = None;
    for expr in filters {
        if let Some(next) = materialize(expr) {
            keys = Some(match keys {
                None => next,
                Some(prior) => intersect(prior, next),
            });
        }
    }
    let keys = keys.expect("recognized demand was sized before allocation");
    let retained = keys
        .capacity()
        .checked_mul(size_of::<Key>())
        .and_then(|bytes| {
            bytes.checked_add(
                size_of::<Vec<Key>>() + size_of::<StateCharge>() + 4 * size_of::<usize>(),
            )
        })
        .ok_or(ModelError::Schema("nominal demand size overflow"))?;
    charge.release(bytes.saturating_sub(retained));
    Ok(Some((keys, charge)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::{
        logical_expr::Cast,
        prelude::{col, lit},
    };
    use lctx_model::domain::{Relation, input::Release};
    fn key(byte: u8) -> Expr {
        lit(ScalarValue::FixedSizeBinary(16, Some(vec![byte; 16])))
    }
    fn demand(expr: Expr) -> Option<Vec<Key>> {
        extract(
            &[expr],
            Relation::of::<Release>().schema().as_ref(),
            &ResourceBudget::fixed(1 << 20).unwrap(),
        )
        .unwrap()
        .map(|(keys, _)| keys)
    }
    #[test]
    fn equality_both_orders_and_null_keep_explicit_empty() {
        assert_eq!(demand(col("id").eq(key(1))), Some(vec![[1; 16]]));
        assert_eq!(demand(key(2).eq(col("id"))), Some(vec![[2; 16]]));
        assert_eq!(
            demand(col("id").eq(lit(ScalarValue::FixedSizeBinary(16, None)))),
            Some(vec![])
        );
        assert_eq!(demand(lit(ScalarValue::Null).eq(col("id"))), Some(vec![]));
    }
    #[test]
    fn positive_in_is_sorted_distinct_and_discards_only_nulls() {
        assert_eq!(
            demand(col("id").in_list(vec![key(2), lit(ScalarValue::Null), key(1), key(2)], false)),
            Some(vec![[1; 16], [2; 16]])
        );
        assert_eq!(demand(col("id").in_list(vec![], false)), Some(vec![]));
        assert_eq!(
            demand(col("id").in_list(vec![lit(ScalarValue::Null)], false)),
            Some(vec![])
        );
        assert_eq!(
            demand(col("id").in_list(vec![key(1), lit("wrong")], false)),
            None
        );
        assert_eq!(demand(col("id").in_list(vec![key(1)], true)), None);
    }
    #[test]
    fn conjunction_intersects_but_disjunction_is_never_mandatory() {
        let first = col("id").in_list(vec![key(1), key(2)], false);
        let residual = col("version").like(lit("%x%"));
        assert_eq!(
            demand(
                residual
                    .clone()
                    .and(first.clone().and(col("id").eq(key(2))))
            ),
            Some(vec![[2; 16]])
        );
        assert!(!only_nominal(&first.clone().and(residual.clone())));
        assert!(only_nominal(&first.clone().and(col("id").eq(key(2)))));
        assert_eq!(demand(first.clone().or(col("id").eq(key(3)))), None);
        assert_eq!(
            demand(first.clone().and(residual.clone().or(col("id").eq(key(3))))),
            Some(vec![[1; 16], [2; 16]])
        );
        assert_eq!(demand(first.and(col("id").eq(key(3)))), Some(vec![]));
    }
    #[test]
    fn casts_aliases_and_non_nominal_representations_do_not_mint_keys() {
        let cast = Expr::Cast(Cast::new(Box::new(col("id")), DataType::Binary));
        assert_eq!(
            demand(cast.eq(lit(ScalarValue::Binary(Some(vec![1; 16]))))),
            None
        );
        assert_eq!(demand(col("id").alias("alias").eq(key(1))), None);
        for wrong in [
            ScalarValue::Binary(Some(vec![1; 16])),
            ScalarValue::FixedSizeBinary(8, Some(vec![1; 8])),
            ScalarValue::FixedSizeBinary(16, Some(vec![1; 15])),
            ScalarValue::Utf8(Some("01".into())),
        ] {
            assert_eq!(demand(col("id").eq(lit(wrong))), None);
        }
        assert_eq!(
            demand(col("id").eq(Expr::Cast(Cast::new(
                Box::new(key(1)),
                DataType::FixedSizeBinary(16)
            )))),
            None
        );
    }
    #[test]
    fn demand_storage_refuses_before_allocation_and_releases_with_owner() {
        let expr = col("id").in_list((0..128).map(key).collect(), false);
        let relation = Relation::of::<Release>();
        let tight = ResourceBudget::fixed(64).unwrap();
        assert!(
            extract(
                std::slice::from_ref(&expr),
                relation.schema().as_ref(),
                &tight
            )
            .is_err()
        );
        assert_eq!(tight.reserved(), 0);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let (_, charge) = extract(&[expr], relation.schema().as_ref(), &budget)
            .unwrap()
            .unwrap();
        assert!(budget.reserved() >= 128 * 16);
        drop(charge);
        assert_eq!(budget.reserved(), 0);
    }
}
