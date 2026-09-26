//! Source-expression completion, independent of calls, name resolution and storage.
//!
//! Only closed primitive expressions are evaluated here. A normal outcome does not assert a
//! normal callee return. Unsupported/effectful operands stop evaluation unless Python skips
//! them. The root syntax fact is a reproducible witness, not a synthetic execution observation.
use std::collections::HashMap;

use cpg_schema::behavior::ClosedExpressionEvaluationsRow;
use cpg_schema::codebook::{BoundaryReason, SyntaxField, SyntaxKind};
use cpg_schema::id::Id;
use cpg_schema::tables::SyntaxNodesRow;

const MAX_DEPTH: usize = 64;
const MAX_WORK: usize = 1024;

#[derive(Clone, Copy)]
enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    None,
    /// Literal evaluation is normal even when the bounded evaluator cannot represent its value.
    Literal,
}

impl Value {
    fn truth(self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(b), Self::Int(i) => Some(i != 0),
            Self::Float(f) => Some(f != 0.0), Self::None => Some(false), Self::Literal => None,
        }
    }
    fn number(self) -> Option<Self> {
        match self {
            Self::Bool(b) => Some(Self::Int(i64::from(b))),
            Self::Int(_) | Self::Float(_) => Some(self), _ => None,
        }
    }
}

type Refusal = BoundaryReason;
const UNSUPPORTED: Refusal = BoundaryReason::UnsupportedControlFlow;

struct Evaluator<'a> {
    children: HashMap<(Id, Id, Id), Vec<&'a SyntaxNodesRow>>,
    remaining: usize,
    depth_limit: usize,
}

impl Evaluator<'_> {
    fn eval(&mut self, node: &SyntaxNodesRow, depth: usize) -> Result<Value, Refusal> {
        self.remaining = self.remaining.checked_sub(1).ok_or(Refusal::ExpressionWorkLimit)?;
        if depth > self.depth_limit { return Err(Refusal::ExpressionDepthLimit); }
        // Only selected children are recursively evaluated, but every child of a supported
        // operator must have one unambiguous structural role and belong to the same owner.
        let children = self.children.get(&(node.snapshot_id, node.module_node_id, node.node_id))
            .map(Vec::as_slice).unwrap_or_default();
        if children.len() > self.remaining { return Err(Refusal::ExpressionWorkLimit); }
        self.remaining -= children.len();
        let children = children.to_vec();
        if children.iter().any(|child| child.owner_node_id != node.owner_node_id
            || child.start_byte < node.start_byte || child.end_byte > node.end_byte) {
            return Err(UNSUPPORTED);
        }
        let one = |field| -> Result<&SyntaxNodesRow, Refusal> {
            let mut matching = children.iter().filter(|child| child.field == field);
            let child = matching.next().ok_or(UNSUPPORTED)?;
            if matching.next().is_some() { return Err(UNSUPPORTED); }
            Ok(*child)
        };
        let detail = node.detail.as_deref().unwrap_or("");
        match node.kind {
            SyntaxKind::ExprBooleanLiteral if children.is_empty() => match detail {
                "True" => Ok(Value::Bool(true)), "False" => Ok(Value::Bool(false)), _ => Err(UNSUPPORTED),
            },
            SyntaxKind::ExprNumberLiteral if children.is_empty() => Ok(number(detail)),
            SyntaxKind::ExprNoneLiteral if children.is_empty() => Ok(Value::None),
            SyntaxKind::ExprStringLiteral | SyntaxKind::ExprBytesLiteral
                | SyntaxKind::ExprEllipsisLiteral if children.is_empty() => Ok(Value::Literal),
            SyntaxKind::ExprUnaryOp if children.len() == 1 => {
                let value = self.eval(one(SyntaxField::Operand)?, depth + 1)?;
                match detail {
                    "not" => value.truth().map(|b| Value::Bool(!b)).ok_or(UNSUPPORTED),
                    "+" => value.number().ok_or(UNSUPPORTED),
                    "-" => match value.number() {
                        Some(Value::Int(i)) => i.checked_neg().map(Value::Int).ok_or(UNSUPPORTED),
                        Some(Value::Float(f)) => Ok(Value::Float(-f)), _ => Err(UNSUPPORTED),
                    },
                    _ => Err(UNSUPPORTED),
                }
            },
            SyntaxKind::ExprBinOp if children.len() == 2 && matches!(detail, "+" | "-") => {
                let left = self.eval(one(SyntaxField::Left)?, depth + 1)?.number().ok_or(UNSUPPORTED)?;
                let right = self.eval(one(SyntaxField::Right)?, depth + 1)?.number().ok_or(UNSUPPORTED)?;
                match (left, right) {
                    (Value::Int(a), Value::Int(b)) => if detail == "+" { a.checked_add(b) }
                        else { a.checked_sub(b) }.map(Value::Int).ok_or(UNSUPPORTED),
                    (a, b) => {
                        let float = |value| match value { Value::Int(i) => i as f64,
                            Value::Float(f) => f, _ => unreachable!("numeric operands") };
                        let (a, b) = (float(a), float(b));
                        Ok(Value::Float(if detail == "+" { a + b } else { a - b }))
                    }
                }
            },
            SyntaxKind::ExprBoolOp if children.len() >= 2 && matches!(detail, "and" | "or") => {
                for (ordinal, child) in children.iter().enumerate() {
                    if child.field != SyntaxField::Operand || child.ordinal != ordinal as i64 {
                        return Err(UNSUPPORTED);
                    }
                }
                let mut value = self.eval(children[0], depth + 1)?;
                for child in children.iter().skip(1) {
                    let truth = value.truth().ok_or(UNSUPPORTED)?;
                    if (detail == "and" && !truth) || (detail == "or" && truth) { return Ok(value); }
                    value = self.eval(child, depth + 1)?;
                }
                Ok(value)
            },
            SyntaxKind::ExprIf if children.len() == 3 => {
                let test = one(SyntaxField::Test)?;
                let yes = one(SyntaxField::Value)?;
                let no = one(SyntaxField::Orelse)?;
                let choose = self.eval(test, depth + 1)?.truth().ok_or(UNSUPPORTED)?;
                self.eval(if choose { yes } else { no }, depth + 1)
            },
            _ => Err(UNSUPPORTED),
        }
    }
}

/// Parse only bounded, source-provided numeric literals, not Python expressions. Integer
/// overflow stays an opaque literal (normal to read, unknown in arithmetic); in particular we
/// never coerce an unbounded Python integer to float and suppress its possible OverflowError.
fn number(text: &str) -> Value {
    if text.len() > 128 { return Value::Literal; }
    let clean = text.replace('_', "");
    let lower = clean.to_ascii_lowercase();
    let radix = [("0x", 16), ("0o", 8), ("0b", 2)].into_iter()
        .find_map(|(prefix, base)| lower.strip_prefix(prefix).map(|digits| (digits, base)));
    if let Some((digits, base)) = radix {
        return i64::from_str_radix(digits, base).map(Value::Int).unwrap_or(Value::Literal);
    }
    if lower.contains(['.', 'e']) && !lower.ends_with('j') {
        return lower.parse::<f64>().map(Value::Float).unwrap_or(Value::Literal);
    }
    lower.parse::<i64>().map(Value::Int).unwrap_or(Value::Literal)
}

/// Evaluate every explicit argument expression once, in stable root order. The source graph
/// index is shared; each root has its own bounded work/depth accounting and explicit refusal.
pub fn closed_arguments(nodes: &[SyntaxNodesRow]) -> Vec<ClosedExpressionEvaluationsRow> {
    closed_arguments_with_limits(nodes, MAX_DEPTH, MAX_WORK)
}

fn closed_arguments_with_limits(nodes: &[SyntaxNodesRow], depth: usize, work: usize)
    -> Vec<ClosedExpressionEvaluationsRow> {
    let mut children: HashMap<_, Vec<_>> = HashMap::new();
    for node in nodes {
        children.entry((node.snapshot_id, node.module_node_id, node.parent_node_id)).or_default().push(node);
    }
    for group in children.values_mut() {
        group.sort_by_key(|node| (node.ordinal, node.start_byte, node.fact_id));
    }
    let mut evaluator = Evaluator { children, remaining: work, depth_limit: depth };
    let mut out = Vec::new();
    for node in nodes.iter().filter(|node| node.field == SyntaxField::Argument) {
        evaluator.remaining = work;
        let result = evaluator.eval(node, 0);
        out.push(ClosedExpressionEvaluationsRow {
            snapshot_id: node.snapshot_id, syntax_fact_id: node.fact_id,
            normal: result.is_ok(), boolean_value: match result { Ok(Value::Bool(b)) => Some(b), _ => None },
            reason: result.err(), work: work.saturating_sub(evaluator.remaining).max(1) as i64,
        });
    }
    out.sort_by_key(|row| (row.snapshot_id, row.syntax_fact_id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u8, parent: u8, kind: SyntaxKind, field: SyntaxField,
        ordinal: i64, detail: &str) -> SyntaxNodesRow {
        SyntaxNodesRow {
            snapshot_id: Id([1; 16]), fact_id: Id([id; 16]), node_id: Id([id; 16]),
            module_node_id: Id([2; 16]), owner_node_id: Some(Id([3; 16])),
            parent_node_id: Id([parent; 16]), kind, field, ordinal,
            start_byte: 0, end_byte: 100, detail: Some(detail.to_owned()),
        }
    }

    fn nested_condition(selected: bool) -> Vec<SyntaxNodesRow> {
        use SyntaxField as F;
        use SyntaxKind as K;
        // (not False and True and True) if selected else unknown_call()
        vec![node(10, 0, K::ExprIf, F::Argument, 0, ""),
            node(11, 10, K::ExprBooleanLiteral, F::Test, 0, if selected { "True" } else { "False" }),
            node(12, 10, K::ExprBoolOp, F::Value, 0, "and"),
            node(13, 12, K::ExprUnaryOp, F::Operand, 0, "not"),
            node(14, 13, K::ExprBooleanLiteral, F::Operand, 0, "False"),
            node(15, 12, K::ExprBooleanLiteral, F::Operand, 1, "True"),
            node(16, 12, K::ExprBooleanLiteral, F::Operand, 2, "True"),
            node(17, 10, K::ExprCall, F::Orelse, 0, "")]
    }

    #[test]
    fn nested_selected_expressions_preserve_boolean_values_and_refusals() {
        let nodes = nested_condition(true);
        let rows = closed_arguments(&nodes);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].normal);
        assert_eq!(rows[0].boolean_value, Some(true));
        let refused = closed_arguments(&nested_condition(false));
        assert!(!refused[0].normal);
        assert_eq!(refused[0].reason, Some(UNSUPPORTED));
        assert_eq!(refused[0].boolean_value, None);
        let mut shuffled = nodes;
        shuffled.reverse();
        assert_eq!(closed_arguments(&shuffled), rows);
    }

    #[test]
    fn short_circuit_returns_selected_value_and_never_evaluates_skipped_operand() {
        use SyntaxField as F;
        use SyntaxKind as K;
        for (operator, first, normal, boolean) in [("and", "False", true, Some(false)),
            ("or", "True", true, Some(true)), ("and", "True", false, None),
            ("or", "False", false, None)] {
            let nodes = vec![node(10, 0, K::ExprBoolOp, F::Argument, 0, operator),
                node(11, 10, K::ExprBooleanLiteral, F::Operand, 0, first),
                node(12, 10, K::ExprCall, F::Operand, 1, "")];
            let result = closed_arguments(&nodes);
            assert_eq!((result[0].normal, result[0].boolean_value), (normal, boolean));
        }
        let nodes = vec![node(10, 0, K::ExprBoolOp, F::Argument, 0, "and"),
            node(11, 10, K::ExprBooleanLiteral, F::Operand, 0, "True"),
            node(12, 10, K::ExprNumberLiteral, F::Operand, 1, "42")];
        let result = closed_arguments(&nodes);
        assert!(result[0].normal);
        assert_eq!(result[0].boolean_value, None, "normal numeric result is not an exact Boolean");
    }

    #[test]
    fn numeric_composition_is_closed_and_bounded() {
        use SyntaxField as F;
        use SyntaxKind as K;
        let mut nodes = vec![node(10, 0, K::ExprBinOp, F::Argument, 0, "+"),
            node(11, 10, K::ExprUnaryOp, F::Left, 0, "-"),
            node(12, 11, K::ExprNumberLiteral, F::Operand, 0, "0x10"),
            node(13, 10, K::ExprNumberLiteral, F::Right, 0, "2.5")];
        assert!(closed_arguments(&nodes)[0].normal);
        nodes[3].detail = Some("9".repeat(400));
        assert_eq!(closed_arguments(&nodes)[0].reason, Some(UNSUPPORTED));
        nodes[0].detail = Some("/".to_owned());
        assert_eq!(closed_arguments(&nodes)[0].reason, Some(UNSUPPORTED));
    }

    #[test]
    fn malformed_children_and_each_budget_are_explicit_unknowns() {
        let nodes = nested_condition(true);
        assert_eq!(closed_arguments_with_limits(&nodes, 0, 1024)[0].reason,
            Some(Refusal::ExpressionDepthLimit));
        assert_eq!(closed_arguments_with_limits(&nodes, 64, 1)[0].reason,
            Some(Refusal::ExpressionWorkLimit));
        let mut malformed = nodes.clone();
        malformed[6].ordinal = 1;
        assert_eq!(closed_arguments(&malformed)[0].reason, Some(UNSUPPORTED));
        let mut malformed = nodes;
        malformed[4].owner_node_id = None;
        assert_eq!(closed_arguments(&malformed)[0].reason, Some(UNSUPPORTED));
    }
}
