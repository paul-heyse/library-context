//! Provider-local predicate shapes. Coordinates are observations, never persisted identities.
use crate::Span;
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Str(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EvaluationSite {
    Source(Span),
    Unavailable { reason: &'static str },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atom {
    IsNone {
        place: String,
    },
    IsValue {
        place: String,
        value: Value,
    },
    Equals {
        place: String,
        value: Value,
    },
    MemberOf {
        place: String,
        values: Vec<Value>,
    },
    Truthy {
        place: String,
    },
    IsInstance {
        place: String,
        class: String,
    },
    TypeIs {
        place: String,
        class: String,
    },
    NonTerminalCall {
        awaiting: bool,
    },
    NonEmptyIterable,
    ContextManagerSuppresses {
        asynchronous: bool,
    },
    FinallyNormalPathImpossible,
    Opaque {
        text: String,
    },
    Evaluated {
        atom: Box<Atom>,
        site: EvaluationSite,
    },
}
impl Atom {
    /// Retained bytes when a pipeline-owned map clones this native shape.
    pub fn heap_bytes(&self) -> usize {
        let value = |v: &Value| match v {
            Value::Str(s) => s.capacity(),
            _ => 0,
        };
        match self {
            Self::IsNone { place } | Self::Truthy { place } => place.capacity(),
            Self::IsValue { place, value: v } | Self::Equals { place, value: v } => {
                place.capacity() + value(v)
            }
            Self::MemberOf { place, values } => {
                place.capacity()
                    + values.capacity() * size_of::<Value>()
                    + values.iter().map(value).sum::<usize>()
            }
            Self::IsInstance { place, class } | Self::TypeIs { place, class } => {
                place.capacity() + class.capacity()
            }
            Self::Opaque { text } => text.capacity(),
            Self::NonTerminalCall { .. }
            | Self::NonEmptyIterable
            | Self::ContextManagerSuppresses { .. }
            | Self::FinallyNormalPathImpossible => 0,
            Self::Evaluated { atom, .. } => size_of::<Atom>() + atom.heap_bytes(),
        }
    }
    pub fn evaluated(self, site: EvaluationSite) -> Self {
        Self::Evaluated {
            atom: Box::new(self),
            site,
        }
    }
    pub fn opaque(text: &str) -> Self {
        Self::Opaque {
            text: text.split_whitespace().collect::<Vec<_>>().join(" "),
        }
    }
    pub fn member_of(place: impl Into<String>, values: impl IntoIterator<Item = Value>) -> Self {
        let mut values: Vec<_> = values.into_iter().collect();
        values.sort();
        values.dedup();
        Self::MemberOf {
            place: place.into(),
            values,
        }
    }
    pub fn place(&self) -> Option<&str> {
        match self {
            Self::IsNone { place }
            | Self::IsValue { place, .. }
            | Self::Equals { place, .. }
            | Self::MemberOf { place, .. }
            | Self::Truthy { place }
            | Self::IsInstance { place, .. }
            | Self::TypeIs { place, .. } => Some(place),
            Self::Evaluated { atom, .. } => atom.place(),
            Self::Opaque { .. }
            | Self::NonTerminalCall { .. }
            | Self::NonEmptyIterable
            | Self::ContextManagerSuppresses { .. }
            | Self::FinallyNormalPathImpossible => None,
        }
    }
    pub fn site(&self) -> Option<Span> {
        match self {
            Self::Evaluated {
                site: EvaluationSite::Source(span),
                ..
            } => Some(*span),
            _ => None,
        }
    }
    pub fn predicate(&self) -> &Self {
        match self {
            Self::Evaluated { atom, .. } => atom,
            _ => self,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceSegment {
    Attribute(String),
    Item(Value),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub root: String,
    pub segments: Vec<PlaceSegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub span: Span,
    pub kind: Option<lctx_model::domain::source::SyntaxKind>,
    pub role: lctx_model::domain::source::OccurrenceRole,
}
