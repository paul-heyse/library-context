//! Structural values and places. Display spellings are never access-path identity encodings.
pub mod presentation;
use super::charged::{ChargedMap, StateCharge};
use super::source::{Module, Occurrence};
use super::{Id, ModelError, Record};
use crate::{Domain, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "literal_values", validate = validate_literal)]
pub enum Literal {
    #[model(code = 0)]
    None,
    #[model(code = 1)]
    Bool { value: bool },
    /// Arbitrary precision integer, canonical decimal; no narrowing through i64.
    #[model(code = 2)]
    Integer { decimal: String },
    #[model(code = 3)]
    String { value: super::Utf8Text },
    #[model(code = 4)]
    Bytes { value: super::EvidenceBytes },
    /// Raw IEEE bits preserve signed zero and NaN payloads without float equality in keys.
    #[model(code = 5)]
    Float { bits: i64 },
}
fn validate_literal(row: &Literal) -> Result<(), ModelError> {
    if let Literal::Integer { decimal } = row {
        let digits = decimal.strip_prefix('-').unwrap_or(decimal);
        if digits.is_empty()
            || !digits.bytes().all(|b| b.is_ascii_digit())
            || (digits.len() > 1 && digits.starts_with('0'))
            || decimal == "-0"
        {
            return Err(ModelError::Invalid(
                "integer literal needs canonical decimal digits".into(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "place_roots", validate = validate_root)]
pub enum PlaceRoot {
    #[model(code = 0)]
    Formal { declaration: Id<Occurrence> },
    #[model(code = 1)]
    Receiver { callable: Id<Occurrence> },
    #[model(code = 2)]
    Return { callable: Id<Occurrence> },
    #[model(code = 3)]
    Yield { callable: Id<Occurrence> },
    #[model(code = 4)]
    Raise { callable: Id<Occurrence> },
    #[model(code = 5)]
    Field { class: Id<Occurrence>, name: String },
    #[model(code = 6)]
    Global { module: Id<Module>, name: String },
    #[model(code = 7)]
    Occurrence { occurrence: Id<Occurrence> },
    /// A local variable of the scope opened by `scope`. Every definition and use of the same
    /// name in that scope shares this root, which reaching flow requires.
    #[model(code = 8)]
    Local { scope: Id<Occurrence>, name: String },
    /// The value bound to the parameter declared at `declaration` when its callable was entered.
    /// `Formal` is the parameter variable, which the body may rebind; only `Entry` (and the bound
    /// `Receiver`) denote the caller's value, so only they are ports of a call summary.
    #[model(code = 9)]
    Entry { declaration: Id<Occurrence> },
    /// The runtime class value of the exact actual expression, distinct from that instance.
    #[model(code = 10)]
    ClassOf { actual: Id<Occurrence> },
}
fn validate_root(row: &PlaceRoot) -> Result<(), ModelError> {
    match row {
        PlaceRoot::Local { name, .. }
        | PlaceRoot::Field { name, .. }
        | PlaceRoot::Global { name, .. }
            if name.is_empty() =>
        {
            Err(ModelError::Invalid("named place root needs a name".into()))
        }
        _ => Ok(()),
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "path_segments")]
pub enum PathSegment {
    #[model(code = 0)]
    Attribute { name: String },
    #[model(code = 1)]
    Item { key: Id<Literal> },
    #[model(code = 2)]
    AnyItem,
}
/// Two explicit segments plus an unknown suffix are the bounded place contract.
/// Reference collections are columns with distinct semantic positions, not a Vec<Id<_>>.
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "access_paths", validate = validate_path)]
pub struct AccessPath {
    #[model(key)]
    pub first: Option<Id<PathSegment>>,
    #[model(key)]
    pub second: Option<Id<PathSegment>>,
    #[model(key)]
    pub unknown_suffix: bool,
}
fn validate_path(row: &AccessPath) -> Result<(), ModelError> {
    if row.first.is_none() && row.second.is_some() {
        return Err(ModelError::Invalid(
            "access path has a missing leading segment".into(),
        ));
    }
    Ok(())
}
impl AccessPath {
    pub fn empty() -> Self {
        Self {
            first: None,
            second: None,
            unknown_suffix: false,
        }
    }
    pub fn extend(&self, segment: Id<PathSegment>) -> Self {
        if self.unknown_suffix {
            return self.clone();
        }
        match (self.first, self.second) {
            (None, _) => Self {
                first: Some(segment),
                second: None,
                unknown_suffix: false,
            },
            (Some(first), None) => Self {
                first: Some(first),
                second: Some(segment),
                unknown_suffix: false,
            },
            _ => Self {
                unknown_suffix: true,
                ..self.clone()
            },
        }
    }
    pub fn append(&self, tail: &Self) -> Self {
        let mut out = self.clone();
        for segment in [tail.first, tail.second].into_iter().flatten() {
            out = out.extend(segment);
        }
        out.unknown_suffix |= tail.unknown_suffix;
        out
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "places")]
pub struct Place {
    #[model(key)]
    pub root: Id<PlaceRoot>,
    #[model(key)]
    pub path: Id<AccessPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "predicates")]
pub enum Predicate {
    #[model(code = 0)]
    IsNone,
    #[model(code = 1)]
    IsValue { value: Id<Literal> },
    #[model(code = 2)]
    Equals { value: Id<Literal> },
    #[model(code = 3)]
    MemberOf { values: Id<LiteralSet> },
    #[model(code = 4)]
    Truthy,
    /// The provider's class expression; resolution remains a separately qualified type observation.
    #[model(code = 5)]
    IsInstance { class_expression: String },
    #[model(code = 6)]
    TypeIs { class_expression: String },
    #[model(code = 7)]
    Opaque { text: String },
    /// A callee-local guard instantiated at a caller evaluation. The source remains a typed
    /// provenance dependency; this is not equality with a caller operand or an erased condition.
    #[model(code = 8)]
    InvokedGuard {
        source: Id<super::conditions::EvaluationAtom>,
    },
    /// A callee guard on a formal, substituted at a call by the bound actual. The atom's operand is
    /// the actual; a stability witness and the call argument justify it (`GuardSubstitution`).
    #[model(code = 9)]
    BoundGuard {
        source: Id<super::conditions::EvaluationAtom>,
    },
    /// Native completion candidate. Its truth requires a separate admitted callee/phase basis.
    #[model(code = 10)]
    NonTerminalCall { awaiting: bool },
    #[model(code = 11)]
    NonEmptyIterable,
    /// Exception exit and normal exit result observations remain independent evidence.
    #[model(code = 12)]
    ContextManagerSuppresses { asynchronous: bool },
    #[model(code = 13)]
    FinallyNormalPathImpossible,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "literal_sets", invariant_refs = literal_set_invariants_refs)]
pub struct LiteralSet {
    #[model(key)]
    pub members: super::ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "literal_set_members")]
pub struct LiteralSetMember {
    #[model(key)]
    pub set: Id<LiteralSet>,
    #[model(key)]
    pub value: Id<Literal>,
}
impl LiteralSet {
    pub fn of(values: impl IntoIterator<Item = Id<Literal>>) -> (Self, Vec<LiteralSetMember>) {
        let values: std::collections::BTreeSet<_> = values.into_iter().collect();
        let mut builder = SetDigest::new();
        for value in &values {
            builder.push(*value);
        }
        let row = Self {
            members: builder.finish(),
        };
        let members = values
            .into_iter()
            .map(|value| LiteralSetMember {
                set: row.id(),
                value,
            })
            .collect();
        (row, members)
    }
}
struct SetDigest {
    sink: super::KeySink,
    count: i64,
}
impl SetDigest {
    fn new() -> Self {
        Self {
            sink: super::KeySink::new("literal-set"),
            count: 0,
        }
    }
    fn push(&mut self, value: Id<Literal>) {
        super::Key::encode(&value, &mut self.sink);
        self.count += 1;
    }
    fn finish(mut self) -> super::ContentHash {
        super::Key::encode(&self.count, &mut self.sink);
        self.sink.finish()
    }
}
pub(crate) fn literal_set_invariants() -> Vec<super::Invariant> {
    vec![super::Invariant {
        revision: 1,
        name: "literal_set_membership",
        inputs: vec![
            super::ValidationInput::of::<LiteralSet>(&["id"]),
            super::ValidationInput::of::<LiteralSetMember>(&["set", "value"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(SetCheck {
                charge: StateCharge::new(budget, "literal_set_membership"),
                expected: Default::default(),
                current: None,
            })
        }),
    }]
}
struct SetCheck {
    charge: StateCharge,
    expected: ChargedMap<Id<LiteralSet>, super::ContentHash>,
    current: Option<(Id<LiteralSet>, SetDigest, Option<Id<Literal>>)>,
}
impl SetCheck {
    fn flush(&mut self) -> Result<(), ModelError> {
        if let Some((id, builder, _)) = self.current.take()
            && self.expected.remove(&mut self.charge, &id) != Some(builder.finish())
        {
            return Err(ModelError::Invalid(
                "literal set members differ from identity".into(),
            ));
        }
        Ok(())
    }
}
impl super::InvariantCheck for SetCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if relation == LiteralSet::NAME {
            for row in LiteralSet::decode(batch)? {
                if self
                    .expected
                    .insert(&mut self.charge, row.id(), row.members)?
                    .is_some()
                {
                    return Err(ModelError::Conflict(LiteralSet::NAME));
                }
            }
        } else if relation == LiteralSetMember::NAME {
            for row in LiteralSetMember::decode(batch)? {
                if self
                    .current
                    .as_ref()
                    .is_none_or(|(id, _, _)| *id != row.set)
                {
                    self.flush()?;
                    if !self.expected.contains_key(&row.set) {
                        return Err(ModelError::Invalid("literal set absent or repeated".into()));
                    }
                    self.current = Some((row.set, SetDigest::new(), None));
                }
                let (_, digest, prior) = self.current.as_mut().expect("initialized set");
                if prior.is_some_and(|id| id >= row.value) {
                    return Err(ModelError::Invalid(
                        "literal members must be unique and ordered".into(),
                    ));
                }
                digest.push(row.value);
                *prior = Some(row.value);
            }
        } else {
            return Err(ModelError::Invalid("undeclared literal-set input".into()));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        self.flush()?;
        let empty = SetDigest::new().finish();
        if self.expected.values().any(|digest| *digest != empty) {
            return Err(ModelError::Invalid("literal set missing members".into()));
        }
        Ok(())
    }
}

pub(crate) fn literal_set_invariants_refs() -> Vec<&'static str> {
    vec!["literal_set_membership"]
}
