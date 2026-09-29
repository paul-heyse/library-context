//! The model's vocabulary: entities, occurrences and places (DESIGN §15.4).
//!
//! **Entities** are relatively stable semantic objects; **occurrences** are source events owned by
//! exactly one entity through [`owner_of`]; a **place** is a root plus a bounded access path, the
//! only place encoding in the model. Rendered strings, node keys and name paths are presentations.

use crate::id::{Code, Id, recipes};

crate::codebook!(
    /// What an entity is. Append-only.
    EntityKind = "entity_kind" {
        Module = 0 => "module",
        Class = 1 => "class",
        Function = 2 => "function",
        Parameter = 3 => "parameter",
        Field = 4 => "field",
        Type = 5 => "type",
        ExternalSymbol = 6 => "external_symbol",
        SyntheticCallable = 7 => "synthetic_callable",
    }
);

crate::codebook!(
    /// The role an occurrence plays. One syntax node may carry several roles; identity is the
    /// occurrence's span and syntax kind, never its role. Append-only.
    OccurrenceKind = "occurrence_kind" {
        CallSite = 0 => "call_site",
        Argument = 1 => "argument",
        Binding = 2 => "binding",
        Use = 3 => "use",
        Return = 4 => "return",
        Yield = 5 => "yield",
        Raise = 6 => "raise",
        PredicateEvaluation = 7 => "predicate_evaluation",
        AttributeAccess = 8 => "attribute_access",
        Subscript = 9 => "subscript",
        DecoratorApplication = 10 => "decorator_application",
        Import = 11 => "import",
        Annotation = 12 => "annotation",
    }
);

crate::codebook!(
    /// A place's root kind. Append-only.
    PlaceRootKind = "place_root_kind" {
        Formal = 0 => "formal",
        Receiver = 1 => "receiver",
        Return = 2 => "return",
        Yield = 3 => "yield",
        Raise = 4 => "raise",
        Field = 5 => "field",
        Global = 6 => "global",
        Occurrence = 7 => "occurrence",
    }
);

/// A callable's formal. A source callable's formal is its parameter entity; an external or
/// modeled callable's is its position or name, resolved once by the binder.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FormalRef {
    Parameter(Id),
    Position { callable: Id, position: u32 },
    Keyword { callable: Id, name: String },
}

/// Where a place is rooted.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PlaceRoot {
    Formal(FormalRef),
    Receiver { callable: Id },
    Return { callable: Id },
    Yield { callable: Id },
    Raise { callable: Id },
    Field { class: Id, name: String },
    Global { module: Id, name: String },
    /// The value at an occurrence: a local binding, an argument or a call's result.
    Occurrence(Id),
}

impl PlaceRoot {
    pub fn kind(&self) -> PlaceRootKind {
        match self {
            Self::Formal(_) => PlaceRootKind::Formal,
            Self::Receiver { .. } => PlaceRootKind::Receiver,
            Self::Return { .. } => PlaceRootKind::Return,
            Self::Yield { .. } => PlaceRootKind::Yield,
            Self::Raise { .. } => PlaceRootKind::Raise,
            Self::Field { .. } => PlaceRootKind::Field,
            Self::Global { .. } => PlaceRootKind::Global,
            Self::Occurrence(_) => PlaceRootKind::Occurrence,
        }
    }

    /// `(owner, position, name)` identity fields, by root kind.
    fn fields(&self) -> (Option<Id>, Option<i64>, Option<&str>) {
        match self {
            Self::Formal(FormalRef::Parameter(p)) => (Some(*p), None, None),
            Self::Formal(FormalRef::Position { callable, position }) => {
                (Some(*callable), Some(i64::from(*position)), None)
            }
            Self::Formal(FormalRef::Keyword { callable, name }) => (Some(*callable), None, Some(name)),
            Self::Receiver { callable }
            | Self::Return { callable }
            | Self::Yield { callable }
            | Self::Raise { callable } => (Some(*callable), None, None),
            Self::Field { class, name } => (Some(*class), None, Some(name)),
            Self::Global { module, name } => (Some(*module), None, Some(name)),
            Self::Occurrence(o) => (Some(*o), None, None),
        }
    }
}

/// A literal item key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemKey {
    Int(i64),
    Str(String),
}

/// One access-path segment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Segment {
    Attribute(String),
    Item(ItemKey),
    AnyItem,
}

/// The most segments an access path keeps (k-limit).
pub const MAX_SEGMENTS: usize = 2;

/// A bounded access path. Extending a full path sets `unknown_suffix` instead of growing, so a
/// place never claims more precision than it keeps.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccessPath {
    segments: Vec<Segment>,
    unknown_suffix: bool,
}

impl AccessPath {
    pub fn new() -> Self {
        Self::default()
    }

    /// Some unknown sub-value: no segment is known.
    pub fn unknown() -> Self {
        Self {
            segments: Vec::new(),
            unknown_suffix: true,
        }
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn unknown_suffix(&self) -> bool {
        self.unknown_suffix
    }

    /// This path followed by `segment`, saturating at [`MAX_SEGMENTS`].
    pub fn then(&self, segment: Segment) -> Self {
        let mut out = self.clone();
        if out.unknown_suffix {
            return out;
        }
        if out.segments.len() == MAX_SEGMENTS {
            out.unknown_suffix = true;
        } else {
            out.segments.push(segment);
        }
        out
    }

    /// This path followed by all of `other`, saturating.
    pub fn join(&self, other: &Self) -> Self {
        let mut out = self.clone();
        for segment in &other.segments {
            out = out.then(segment.clone());
        }
        if other.unknown_suffix {
            out.unknown_suffix = true;
        }
        out
    }

    /// How `self` relates to `prefix`: the rest of `self` after `prefix`, disjoint, or
    /// undecidable because an unknown suffix hides where they diverge.
    pub fn strip_prefix(&self, prefix: &Self) -> PathRelation {
        for (i, segment) in prefix.segments.iter().enumerate() {
            match self.segments.get(i) {
                Some(own) if own == segment => {}
                Some(own) if matches!((own, segment), (Segment::AnyItem, Segment::Item(_)) | (Segment::Item(_), Segment::AnyItem)) => {
                    return PathRelation::Unknown;
                }
                Some(_) => return PathRelation::Disjoint,
                None if self.unknown_suffix => return PathRelation::Unknown,
                None => return PathRelation::Shorter(Self {
                    segments: prefix.segments[i..].to_vec(),
                    unknown_suffix: prefix.unknown_suffix,
                }),
            }
        }
        if prefix.unknown_suffix {
            return PathRelation::Unknown;
        }
        PathRelation::Rest(Self {
            segments: self.segments[prefix.segments.len()..].to_vec(),
            unknown_suffix: self.unknown_suffix,
        })
    }

    /// The canonical text the place id covers: `.name`, `[json]` or `[*]` per segment, then `…`
    /// for an unknown suffix.
    pub fn encode(&self) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Attribute(name) => {
                    out.push('.');
                    out.push_str(name);
                }
                Segment::Item(ItemKey::Int(i)) => out.push_str(&format!("[{i}]")),
                Segment::Item(ItemKey::Str(s)) => {
                    out.push('[');
                    out.push_str(&crate::condition::Value::Str(s.clone()).encode());
                    out.push(']');
                }
                Segment::AnyItem => out.push_str("[*]"),
            }
        }
        if self.unknown_suffix {
            out.push('…');
        }
        out
    }
}

/// How one access path relates to a prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathRelation {
    /// The path extends the prefix by this rest (possibly empty).
    Rest(AccessPath),
    /// The prefix extends the path by this rest: the path names a container of the prefix.
    Shorter(AccessPath),
    /// They name different sub-values.
    Disjoint,
    /// An unknown suffix or an item wildcard hides whether they overlap.
    Unknown,
}

/// A root plus a bounded access path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Place {
    pub root: PlaceRoot,
    pub path: AccessPath,
}

impl Place {
    pub fn root(root: PlaceRoot) -> Self {
        Self {
            root,
            path: AccessPath::new(),
        }
    }

    pub fn attribute(&self, name: &str) -> Self {
        Self {
            root: self.root.clone(),
            path: self.path.then(Segment::Attribute(name.to_owned())),
        }
    }

    /// This place's root with `path` appended to its own path.
    pub fn extended(&self, path: &AccessPath) -> Self {
        Self {
            root: self.root.clone(),
            path: self.path.join(path),
        }
    }

    pub fn id(&self) -> Id {
        let (owner, position, name) = self.root.fields();
        recipes::place(
            Code(self.root.kind()),
            owner,
            position,
            name,
            &self.path.encode(),
        )
    }
}

/// A declaration's body region, for the owner rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub entity: Id,
    pub module: Id,
    pub start: i64,
    pub end: i64,
}

/// The owner rule (DESIGN §15.4): the innermost declaration whose body holds the span, otherwise
/// the module. The only definition of "caller". Equal regions resolve to the lowest entity id, so
/// the answer never depends on input order.
pub fn owner_of(module: Id, start: i64, end: i64, regions: &[Region]) -> Id {
    regions
        .iter()
        .filter(|r| r.module == module && r.start <= start && end <= r.end)
        .min_by(|a, b| {
            (a.end - a.start)
                .cmp(&(b.end - b.start))
                .then(a.entity.cmp(&b.entity))
        })
        .map_or(module, |r| r.entity)
}

/// What a provider observation denotes, for the region join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedPlace {
    Name,
    Attribute,
    Subscript,
}

/// A candidate occurrence for the region join.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccurrenceSpan<'a> {
    pub occurrence: Id,
    pub module: Id,
    pub start: i64,
    pub end: i64,
    pub syntax_kind: &'a str,
}

/// How an observation attached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    /// An occurrence of the observed syntax with exactly the observed span.
    Exact(Id),
    /// No exact match: the innermost occurrence of the observed syntax enclosing the span.
    Innermost(Id),
    /// Nothing of the observed syntax encloses it: a coverage gap, never a guess.
    Unmatched,
}

/// The one join of an external observation onto the owning parse's occurrences (DESIGN §15.4):
/// `(module, span)` with the same rule for names, attributes and subscripts. Ties resolve to the
/// lowest occurrence id.
pub fn innermost_region_join(
    module: Id,
    start: i64,
    end: i64,
    observed: ObservedPlace,
    occurrences: &[OccurrenceSpan<'_>],
) -> Join {
    let syntax = match observed {
        ObservedPlace::Name => "name",
        ObservedPlace::Attribute => "attribute",
        ObservedPlace::Subscript => "subscript",
    };
    let candidates = occurrences
        .iter()
        .filter(|o| o.module == module && o.syntax_kind == syntax);
    if let Some(exact) = candidates
        .clone()
        .filter(|o| o.start == start && o.end == end)
        .map(|o| o.occurrence)
        .min()
    {
        return Join::Exact(exact);
    }
    candidates
        .filter(|o| o.start <= start && end <= o.end)
        .min_by(|a, b| (a.end - a.start).cmp(&(b.end - b.start)).then(a.occurrence.cmp(&b.occurrence)))
        .map_or(Join::Unmatched, |o| Join::Innermost(o.occurrence))
}
