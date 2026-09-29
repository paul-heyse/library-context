//! Evaluation atoms (DESIGN §15.3, §15.7).
//!
//! An atom is one predicate **evaluation**: the occurrence that evaluates it, the predicate and
//! its literal argument, and the place it tests. Its id is the v2 atom recipe over exactly those,
//! so the same spelling at two occurrences is two atoms; equating them needs an effect-stability
//! witness, never shared identity. No provider-internal index or synthetic predicate digest
//! enters the id.

use crate::id::{Code, Id};

crate::codebook!(
    /// What an atom tests. Append-only.
    PredicateKind = "predicate_kind" {
        /// `p is None` (negated: `p is not None`).
        IsNone = 0 => "is_none",
        /// `p is v` for a literal `None`/`True`/`False`.
        IsValue = 1 => "is_value",
        /// `p == v` for a literal `v`.
        Equals = 2 => "equals",
        /// `p in {v, …}` for literals.
        MemberOf = 3 => "member_of",
        /// The truthiness of `p`.
        Truthy = 4 => "truthy",
        /// `isinstance(p, C)`, `C` as written.
        IsInstance = 5 => "isinstance",
        /// Resolved builtin `type(p) is C`.
        TypeIs = 6 => "type_is",
        /// Any other test, as its source text with whitespace runs collapsed.
        Opaque = 7 => "opaque",
    }
);

/// A literal value an atom compares its operand with.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Str(String),
}

impl Value {
    /// The canonical text: `None`, `True`, `False`, a decimal integer or a JSON string.
    pub fn encode(&self) -> String {
        match self {
            Value::None => "None".to_owned(),
            Value::Bool(true) => "True".to_owned(),
            Value::Bool(false) => "False".to_owned(),
            Value::Int(i) => i.to_string(),
            Value::Str(s) => json_string(s),
        }
    }
}

/// A JSON string literal, escaped as `serde_json` would (quotes, backslashes, controls).
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// What an atom tests, with its canonical argument.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Predicate {
    IsNone,
    IsValue(Value),
    Equals(Value),
    /// Sorted by encoding and deduplicated ([`Predicate::member_of`]).
    MemberOf(Vec<Value>),
    Truthy,
    IsInstance(String),
    TypeIs(String),
    Opaque(String),
}

impl Predicate {
    /// A membership test with its values in canonical order. It keeps its operator even for one
    /// value: Python may dispatch equality differently.
    pub fn member_of(values: impl IntoIterator<Item = Value>) -> Self {
        let mut values: Vec<Value> = values.into_iter().collect();
        values.sort_by_key(Value::encode);
        values.dedup();
        Self::MemberOf(values)
    }

    /// An opaque test over source text, whitespace runs collapsed.
    pub fn opaque(text: &str) -> Self {
        Self::Opaque(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    pub fn kind(&self) -> PredicateKind {
        match self {
            Self::IsNone => PredicateKind::IsNone,
            Self::IsValue(_) => PredicateKind::IsValue,
            Self::Equals(_) => PredicateKind::Equals,
            Self::MemberOf(_) => PredicateKind::MemberOf,
            Self::Truthy => PredicateKind::Truthy,
            Self::IsInstance(_) => PredicateKind::IsInstance,
            Self::TypeIs(_) => PredicateKind::TypeIs,
            Self::Opaque(_) => PredicateKind::Opaque,
        }
    }

    /// The canonical argument text, if the predicate has one.
    pub fn argument(&self) -> Option<String> {
        match self {
            Self::IsNone | Self::Truthy => None,
            Self::IsValue(v) | Self::Equals(v) => Some(v.encode()),
            Self::MemberOf(values) => Some(format!(
                "{{{}}}",
                values.iter().map(Value::encode).collect::<Vec<_>>().join(",")
            )),
            Self::IsInstance(class) | Self::TypeIs(class) => Some(class.clone()),
            Self::Opaque(text) => Some(json_string(text)),
        }
    }
}

/// One predicate evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Atom {
    /// The occurrence that evaluates the predicate.
    pub evaluation: Id,
    /// The place tested; `None` for an opaque test.
    pub operand: Option<Id>,
    pub predicate: Predicate,
}

impl Atom {
    pub fn id(&self) -> Id {
        crate::id::recipes::atom(
            self.evaluation,
            Code(self.predicate.kind()),
            self.operand,
            self.predicate.argument().as_deref(),
        )
    }

    /// A readable label, for renderings only.
    pub fn label(&self) -> String {
        use crate::decl::codebook::Codebook;
        match self.predicate.argument() {
            Some(argument) => format!("{}({argument})", self.predicate.kind().text()),
            None => format!("{}()", self.predicate.kind().text()),
        }
    }
}
