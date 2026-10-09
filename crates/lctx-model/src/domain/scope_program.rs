//! Bounded relation intent. Native and finite realizations do not own scope meaning.
use super::{
    resources::{Reservation, ResourceBudget},
    source::Occurrence,
    *,
};
use fixedbitset::FixedBitSet;
use petgraph::{Directed, Graph, graph::NodeIndex};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopePort {
    pub input: usize,
    /// Only an explicitly requested owner can enter this namespace.
    pub virtual_owner: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeDirection {
    Forward,
    OwnedReverse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopeColumn {
    pub row: usize,
    pub field: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeOptionalJoin {
    pub row: usize,
    pub keys: Vec<(ScopeColumn, ScopeColumn)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopePredicate {
    CanonicalOccurrence {
        row: usize,
    },
    Boolean(ScopeColumn, bool),
    Integer(ScopeColumn, i64),
    NotCode(ScopeColumn, i16),
    CodeNotIn(ScopeColumn, &'static [i16]),
    SpanContains {
        outer_start: ScopeColumn,
        outer_end: ScopeColumn,
        inner_start: ScopeColumn,
        inner_end: ScopeColumn,
    },
    PathPrefix {
        parent: usize,
        child: usize,
    },
    QualifiedName {
        wanted: ScopeColumn,
        module: ScopeColumn,
        leaf: ScopeColumn,
        leaf_match: bool,
    },
    Equal(ScopeColumn, ScopeColumn),
    Code(ScopeColumn, i16),
    CodeIn(ScopeColumn, &'static [i16]),
    Text(ScopeColumn, &'static str),
    TextSuffix(ScopeColumn, &'static str),
    TextSuffixIn(ScopeColumn, &'static [&'static str]),
    Parameter(ScopeColumn, usize),
    ParameterIn(ScopeColumn, usize),
    IsNull(ScopeColumn, bool),
    EqualCoalesce {
        column: ScopeColumn,
        primary: ScopeColumn,
        fallback: ScopeColumn,
    },
    BodyContains {
        parent: usize,
        child: usize,
    },
}
impl ScopePredicate {
    pub fn columns(&self) -> Vec<ScopeColumn> {
        match self {
            Self::CanonicalOccurrence { row } => vec![
                ScopeColumn {
                    row: *row,
                    field: "id",
                },
                ScopeColumn {
                    row: *row,
                    field: "source",
                },
                ScopeColumn {
                    row: *row,
                    field: "structural_path",
                },
            ],
            Self::Equal(a, b) => vec![*a, *b],
            Self::Code(c, _)
            | Self::CodeIn(c, _)
            | Self::NotCode(c, _)
            | Self::CodeNotIn(c, _)
            | Self::Boolean(c, _)
            | Self::Integer(c, _)
            | Self::Text(c, _)
            | Self::TextSuffix(c, _)
            | Self::TextSuffixIn(c, _)
            | Self::Parameter(c, _)
            | Self::ParameterIn(c, _)
            | Self::IsNull(c, _) => vec![*c],
            Self::EqualCoalesce {
                column,
                primary,
                fallback,
            } => vec![*column, *primary, *fallback],
            Self::SpanContains {
                outer_start,
                outer_end,
                inner_start,
                inner_end,
            } => vec![*outer_start, *outer_end, *inner_start, *inner_end],
            Self::QualifiedName {
                wanted,
                module,
                leaf,
                ..
            } => vec![*wanted, *module, *leaf],
            Self::PathPrefix { parent, child } | Self::BodyContains { parent, child } => {
                let mut columns = vec![
                    ScopeColumn {
                        row: *parent,
                        field: "source",
                    },
                    ScopeColumn {
                        row: *parent,
                        field: "structural_path",
                    },
                    ScopeColumn {
                        row: *child,
                        field: "source",
                    },
                    ScopeColumn {
                        row: *child,
                        field: "structural_path",
                    },
                ];
                if matches!(self, Self::BodyContains { .. }) {
                    for row in [parent, child] {
                        columns.extend([
                            ScopeColumn {
                                row: *row,
                                field: "start",
                            },
                            ScopeColumn {
                                row: *row,
                                field: "end",
                            },
                        ]);
                    }
                }
                columns
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeRule {
    NearestPairs {
        source: usize,
        target: usize,
        rows: Vec<usize>,
        rank_rows: usize,
        predicates: Vec<ScopePredicate>,
        post: Vec<ScopePredicate>,
        source_key: ScopeColumn,
        target_key: ScopeColumn,
        event_key: ScopeColumn,
        ancestor: usize,
    },
    Reference {
        source: usize,
        field: &'static str,
        target: usize,
        direction: ScopeDirection,
        list: bool,
    },
    Pairs {
        source: usize,
        target: usize,
        rows: Vec<usize>,
        predicates: Vec<ScopePredicate>,
        source_key: ScopeColumn,
        target_key: ScopeColumn,
    },
    FirstPairs {
        source: usize,
        target: usize,
        rows: Vec<usize>,
        predicates: Vec<ScopePredicate>,
        source_key: ScopeColumn,
        target_key: ScopeColumn,
    },
    OptionalPairs {
        source: usize,
        target: usize,
        rows: Vec<usize>,
        optional: Vec<ScopeOptionalJoin>,
        predicates: Vec<ScopePredicate>,
        source_key: ScopeColumn,
        target_key: ScopeColumn,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeValue {
    Nominal([u8; 16]),
    Nominals(Vec<[u8; 16]>),
    Text(String),
    Code(i16),
    Boolean(bool),
    Integer(i64),
}
/// Per-execution values never participate in immutable program identity.
pub struct ScopeParameters(pub Vec<ScopeValue>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeRootOutcome {
    Present,
    Absent,
    Virtual,
}
#[derive(Clone)]
pub struct ScopeProgram {
    pub inputs: Vec<ValidationInput>,
    pub ports: Vec<ScopePort>,
    pub rules: Vec<ScopeRule>,
}
/// The same containment meaning supplies finite and relational lowerings. Role is identity,
/// not an equality requirement: a declaration body legitimately contains other occurrence roles.
pub fn body_contains(parent: &Occurrence, child: &Occurrence) -> bool {
    parent.source == child.source
        && child.start >= parent.start
        && child.end <= parent.end
        && child.structural_path.starts_with(&parent.structural_path)
}
impl ScopeProgram {
    pub fn validate(&self, model: &ValidatedModel) -> Result<(), ModelError> {
        let relation = |port: usize| -> Result<&Relation, ModelError> {
            let p = self
                .ports
                .get(port)
                .ok_or(ModelError::Schema("scope port absent"))?;
            let input = self
                .inputs
                .get(p.input)
                .ok_or(ModelError::Schema("scope input absent"))?;
            model
                .relation(input.name())
                .ok_or(ModelError::Schema("scope relation absent"))
        };
        for port in 0..self.ports.len() {
            relation(port)?;
        }
        for rule in &self.rules {
            match rule {
                ScopeRule::Reference {
                    source,
                    field,
                    target,
                    list,
                    ..
                } => {
                    let member = relation(*source)?;
                    let target = relation(*target)?;
                    let f = member
                        .fields()
                        .iter()
                        .find(|f| f.name() == *field)
                        .ok_or(ModelError::Schema("scope reference field absent"))?;
                    if f.list() != *list
                        || !f.target().is_some_and(|(_, name)| name == target.name())
                    {
                        return Err(ModelError::Schema("scope reference nominal type"));
                    }
                }
                ScopeRule::Pairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                }
                | ScopeRule::FirstPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                }
                | ScopeRule::OptionalPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                    ..
                }
                | ScopeRule::NearestPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                    ..
                } => {
                    let column = |c: &ScopeColumn| -> Result<(), ModelError> {
                        let r = relation(
                            *rows
                                .get(c.row)
                                .ok_or(ModelError::Schema("scope row absent"))?,
                        )?;
                        if c.field != "id" && !r.fields().iter().any(|f| f.name() == c.field) {
                            return Err(ModelError::Schema("scope column absent"));
                        }
                        Ok(())
                    };
                    relation(*source)?;
                    relation(*target)?;
                    for row in rows {
                        relation(*row)?;
                    }
                    column(source_key)?;
                    column(target_key)?;
                    if let ScopeRule::OptionalPairs { optional, .. } = rule {
                        for join in optional {
                            if join.row == 0 || join.row >= rows.len() || join.keys.is_empty() {
                                return Err(ModelError::Schema("scope optional join domain"));
                            }
                            for (a, b) in &join.keys {
                                column(a)?;
                                column(b)?;
                                if a.row != join.row || b.row >= join.row {
                                    return Err(ModelError::Schema("scope optional join ordering"));
                                }
                            }
                        }
                    }
                    if let ScopeRule::NearestPairs {
                        rank_rows,
                        event_key,
                        ancestor,
                        ..
                    } = rule
                    {
                        if *rank_rows == 0
                            || *rank_rows > rows.len()
                            || source_key.row >= *rank_rows
                            || event_key.row >= *rank_rows
                            || *ancestor >= *rank_rows
                        {
                            return Err(ModelError::Schema("scope nearest rank domain"));
                        }
                        column(event_key)?;
                        if predicates
                            .iter()
                            .flat_map(ScopePredicate::columns)
                            .any(|c| c.row >= *rank_rows)
                        {
                            return Err(ModelError::Schema(
                                "scope nearest precondition beyond rank domain",
                            ));
                        }
                        if relation(rows[*ancestor])?.name() != Occurrence::NAME {
                            return Err(ModelError::Schema("scope nearest occurrence type"));
                        }
                    }
                    let post = if let ScopeRule::NearestPairs { post, .. } = rule {
                        post.as_slice()
                    } else {
                        &[]
                    };
                    for predicate in predicates.iter().chain(post) {
                        match predicate {
                            ScopePredicate::CanonicalOccurrence { row } => {
                                if relation(
                                    *rows
                                        .get(*row)
                                        .ok_or(ModelError::Schema("scope canonical row absent"))?,
                                )?
                                .name()
                                    != Occurrence::NAME
                                {
                                    return Err(ModelError::Schema(
                                        "scope canonical occurrence type",
                                    ));
                                }
                            }
                            ScopePredicate::Equal(a, b) => {
                                column(a)?;
                                column(b)?;
                            }
                            ScopePredicate::Code(c, _)
                            | ScopePredicate::NotCode(c, _)
                            | ScopePredicate::CodeNotIn(c, _)
                            | ScopePredicate::Boolean(c, _)
                            | ScopePredicate::Integer(c, _) => column(c)?,
                            ScopePredicate::Text(c, _)
                            | ScopePredicate::TextSuffix(c, _)
                            | ScopePredicate::Parameter(c, _)
                            | ScopePredicate::ParameterIn(c, _)
                            | ScopePredicate::CodeIn(c, _)
                            | ScopePredicate::TextSuffixIn(c, _) => column(c)?,
                            ScopePredicate::IsNull(c, _) => column(c)?,
                            ScopePredicate::EqualCoalesce {
                                column: c,
                                primary,
                                fallback,
                            } => {
                                column(c)?;
                                column(primary)?;
                                column(fallback)?;
                            }
                            ScopePredicate::SpanContains {
                                outer_start,
                                outer_end,
                                inner_start,
                                inner_end,
                            } => {
                                for c in [outer_start, outer_end, inner_start, inner_end] {
                                    column(c)?;
                                }
                            }
                            ScopePredicate::QualifiedName {
                                wanted,
                                module,
                                leaf,
                                ..
                            } => {
                                for c in [wanted, module, leaf] {
                                    column(c)?;
                                }
                            }
                            ScopePredicate::BodyContains { parent, child }
                            | ScopePredicate::PathPrefix { parent, child } => {
                                for row in [parent, child] {
                                    if relation(
                                        *rows
                                            .get(*row)
                                            .ok_or(ModelError::Schema("scope body row absent"))?,
                                    )?
                                    .name()
                                        != Occurrence::NAME
                                    {
                                        return Err(ModelError::Schema(
                                            "scope body occurrence type",
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    /// Owned metadata plus canonical bytes. Count with the authoritative encoder rather than
    /// retaining a large heuristic SQL arena as the canonical buffer's capacity.
    pub fn allowance(&self) -> usize {
        self.metadata_bytes()
            .saturating_add(self.canonical_len())
            .saturating_add(1024)
    }
    fn canonical_len(&self) -> usize {
        let mut sink = KeySink::counting("scope-program/v1");
        sink.part(b"model", &[0; 32]);
        self.encode(&mut sink);
        sink.encoded_len()
    }
    fn metadata_bytes(&self) -> usize {
        let mut bytes = size_of::<Self>()
            .saturating_add(
                self.inputs
                    .capacity()
                    .saturating_mul(size_of::<ValidationInput>()),
            )
            .saturating_add(self.ports.capacity().saturating_mul(size_of::<ScopePort>()))
            .saturating_add(self.rules.capacity().saturating_mul(size_of::<ScopeRule>()));
        for input in &self.inputs {
            bytes = bytes.saturating_add(input.order().len().saturating_mul(size_of::<&str>()));
        }
        for rule in &self.rules {
            match rule {
                ScopeRule::Reference { .. } => {}
                ScopeRule::Pairs {
                    rows, predicates, ..
                }
                | ScopeRule::FirstPairs {
                    rows, predicates, ..
                } => {
                    bytes = bytes
                        .saturating_add(rows.capacity().saturating_mul(size_of::<usize>()))
                        .saturating_add(
                            predicates
                                .capacity()
                                .saturating_mul(size_of::<ScopePredicate>()),
                        );
                }
                ScopeRule::NearestPairs {
                    rows,
                    predicates,
                    post,
                    ..
                } => {
                    bytes = bytes
                        .saturating_add(rows.capacity().saturating_mul(size_of::<usize>()))
                        .saturating_add(
                            predicates
                                .capacity()
                                .saturating_add(post.capacity())
                                .saturating_mul(size_of::<ScopePredicate>()),
                        );
                }
                ScopeRule::OptionalPairs {
                    rows,
                    predicates,
                    optional,
                    ..
                } => {
                    bytes = bytes
                        .saturating_add(rows.capacity().saturating_mul(size_of::<usize>()))
                        .saturating_add(
                            predicates
                                .capacity()
                                .saturating_mul(size_of::<ScopePredicate>()),
                        )
                        .saturating_add(
                            optional
                                .capacity()
                                .saturating_mul(size_of::<ScopeOptionalJoin>()),
                        );
                    for join in optional {
                        bytes = bytes.saturating_add(
                            join.keys
                                .capacity()
                                .saturating_mul(size_of::<(ScopeColumn, ScopeColumn)>()),
                        );
                    }
                }
            }
        }
        bytes
    }
    fn encode(&self, sink: &mut KeySink) {
        fn number(s: &mut KeySink, tag: &[u8], n: usize) {
            s.part(tag, &(n as u64).to_le_bytes());
        }
        fn column(s: &mut KeySink, c: &ScopeColumn) {
            number(s, b"row", c.row);
            s.part(b"field", c.field.as_bytes());
        }
        sink.part(b"scope-revision", b"relation-scopes/v1");
        number(sink, b"inputs", self.inputs.len());
        for input in &self.inputs {
            input.encode_contract(sink);
        }
        number(sink, b"ports", self.ports.len());
        for p in &self.ports {
            number(sink, b"input", p.input);
            sink.part(b"virtual-owner", &[u8::from(p.virtual_owner)]);
        }
        number(sink, b"rules", self.rules.len());
        for rule in &self.rules {
            match rule {
                ScopeRule::Reference {
                    source,
                    field,
                    target,
                    direction,
                    list,
                } => {
                    sink.part(b"operation", b"reference");
                    number(sink, b"source", *source);
                    number(sink, b"target", *target);
                    sink.part(b"field", field.as_bytes());
                    sink.part(
                        b"direction",
                        &[match direction {
                            ScopeDirection::Forward => 0,
                            ScopeDirection::OwnedReverse => 1,
                        }],
                    );
                    sink.part(b"list", &[u8::from(*list)]);
                }
                ScopeRule::Pairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                }
                | ScopeRule::FirstPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                }
                | ScopeRule::OptionalPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                    ..
                }
                | ScopeRule::NearestPairs {
                    source,
                    target,
                    rows,
                    predicates,
                    source_key,
                    target_key,
                    ..
                } => {
                    sink.part(
                        b"operation",
                        match rule {
                            ScopeRule::NearestPairs { .. } => b"nearest-pairs",
                            ScopeRule::FirstPairs { .. } => b"first-pairs",
                            ScopeRule::OptionalPairs { .. } => b"optional-pairs",
                            _ => b"pairs",
                        },
                    );
                    number(sink, b"source", *source);
                    number(sink, b"target", *target);
                    number(sink, b"rows", rows.len());
                    for r in rows {
                        number(sink, b"port", *r);
                    }
                    if let ScopeRule::OptionalPairs { optional, .. } = rule {
                        number(sink, b"optional", optional.len());
                        for join in optional {
                            number(sink, b"optional-row", join.row);
                            number(sink, b"optional-keys", join.keys.len());
                            for (a, b) in &join.keys {
                                column(sink, a);
                                column(sink, b);
                            }
                        }
                    }
                    column(sink, source_key);
                    column(sink, target_key);
                    number(sink, b"predicates", predicates.len());
                    for p in predicates {
                        encode_predicate(sink, p);
                    }
                    if let ScopeRule::NearestPairs {
                        rank_rows,
                        event_key,
                        ancestor,
                        post,
                        ..
                    } = rule
                    {
                        number(sink, b"rank-rows", *rank_rows);
                        column(sink, event_key);
                        number(sink, b"ancestor", *ancestor);
                        number(sink, b"post-predicates", post.len());
                        for p in post {
                            encode_predicate(sink, p);
                        }
                    }
                }
            }
        }
    }
}
fn encode_predicate(sink: &mut KeySink, p: &ScopePredicate) {
    fn number(s: &mut KeySink, tag: &[u8], n: usize) {
        s.part(tag, &(n as u64).to_le_bytes());
    }
    fn column(s: &mut KeySink, c: &ScopeColumn) {
        number(s, b"row", c.row);
        s.part(b"field", c.field.as_bytes());
    }
    match p {
        ScopePredicate::CanonicalOccurrence { row } => {
            sink.part(b"predicate", b"canonical-occurrence");
            number(sink, b"row", *row);
        }
        ScopePredicate::Boolean(c, v) => {
            sink.part(b"predicate", b"boolean");
            column(sink, c);
            sink.part(b"boolean", &[u8::from(*v)]);
        }
        ScopePredicate::Integer(c, v) => {
            sink.part(b"predicate", b"integer");
            column(sink, c);
            sink.part(b"integer", &v.to_le_bytes());
        }
        ScopePredicate::NotCode(c, v) => {
            sink.part(b"predicate", b"not-code");
            column(sink, c);
            sink.part(b"code", &v.to_le_bytes());
        }
        ScopePredicate::CodeNotIn(c, vs) => {
            sink.part(b"predicate", b"code-not-in");
            column(sink, c);
            number(sink, b"codes", vs.len());
            for v in *vs {
                sink.part(b"code", &v.to_le_bytes());
            }
        }
        ScopePredicate::SpanContains {
            outer_start,
            outer_end,
            inner_start,
            inner_end,
        } => {
            sink.part(b"predicate", b"span-contains");
            for c in [outer_start, outer_end, inner_start, inner_end] {
                column(sink, c);
            }
        }
        ScopePredicate::PathPrefix { parent, child } => {
            sink.part(b"predicate", b"path-prefix");
            number(sink, b"parent", *parent);
            number(sink, b"child", *child);
        }
        ScopePredicate::QualifiedName {
            wanted,
            module,
            leaf,
            leaf_match,
        } => {
            sink.part(b"predicate", b"qualified-name");
            for c in [wanted, module, leaf] {
                column(sink, c);
            }
            sink.part(b"leaf-match", &[u8::from(*leaf_match)]);
        }
        ScopePredicate::Equal(a, b) => {
            sink.part(b"predicate", b"equal");
            column(sink, a);
            column(sink, b);
        }
        ScopePredicate::Code(c, v) => {
            sink.part(b"predicate", b"code");
            column(sink, c);
            sink.part(b"code", &v.to_le_bytes());
        }
        ScopePredicate::CodeIn(c, values) => {
            sink.part(b"predicate", b"code-in");
            column(sink, c);
            number(sink, b"codes", values.len());
            for v in *values {
                sink.part(b"code", &v.to_le_bytes());
            }
        }
        ScopePredicate::Text(c, v) => {
            sink.part(b"predicate", b"text");
            column(sink, c);
            sink.part(b"text", v.as_bytes());
        }
        ScopePredicate::TextSuffix(c, v) => {
            sink.part(b"predicate", b"text-suffix");
            column(sink, c);
            sink.part(b"suffix", v.as_bytes());
        }
        ScopePredicate::TextSuffixIn(c, values) => {
            sink.part(b"predicate", b"text-suffix-in");
            column(sink, c);
            number(sink, b"suffixes", values.len());
            for v in *values {
                sink.part(b"suffix", v.as_bytes());
            }
        }
        ScopePredicate::Parameter(c, index) => {
            sink.part(b"predicate", b"parameter");
            column(sink, c);
            number(sink, b"parameter", *index);
        }
        ScopePredicate::ParameterIn(c, index) => {
            sink.part(b"predicate", b"parameter-in");
            column(sink, c);
            number(sink, b"parameter", *index);
        }
        ScopePredicate::IsNull(c, yes) => {
            sink.part(b"predicate", b"is-null");
            column(sink, c);
            sink.part(b"null", &[u8::from(*yes)]);
        }
        ScopePredicate::EqualCoalesce {
            column: c,
            primary,
            fallback,
        } => {
            sink.part(b"predicate", b"equal-coalesce");
            column(sink, c);
            column(sink, primary);
            column(sink, fallback);
        }
        ScopePredicate::BodyContains { parent, child } => {
            sink.part(b"predicate", b"body");
            number(sink, b"parent", *parent);
            number(sink, b"child", *child);
        }
    }
}
/// A lifetime-owned, non-evicting interner. Hashes select buckets; bytes decide identity.
pub struct ScopeInterner {
    buckets: BTreeMap<u128, Vec<Arc<CompiledScopeProgram>>>,
    budget: ResourceBudget,
    charge: Box<dyn Reservation>,
    entries: usize,
}
pub struct CompiledScopeProgram {
    program: ScopeProgram,
    bytes: Vec<u8>,
    identity: ContentHash,
    _charge: Box<dyn Reservation>,
}
impl CompiledScopeProgram {
    pub fn program(&self) -> &ScopeProgram {
        &self.program
    }
    pub fn identity(&self) -> ContentHash {
        self.identity
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
impl ScopeInterner {
    pub fn new(budget: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            buckets: BTreeMap::new(),
            budget: budget.clone(),
            charge: budget.reserve("scope-interner", 0)?,
            entries: 0,
        })
    }
    pub fn intern(
        &mut self,
        program: ScopeProgram,
        model: &ValidatedModel,
    ) -> Result<Arc<CompiledScopeProgram>, ModelError> {
        program.validate(model)?;
        self.intern_with(program, model.digest(), xxhash_rust::xxh3::xxh3_128)
    }
    fn intern_with(
        &mut self,
        program: ScopeProgram,
        model: ContentHash,
        hash: fn(&[u8]) -> u128,
    ) -> Result<Arc<CompiledScopeProgram>, ModelError> {
        let capacity = program.canonical_len();
        let charge = self.budget.reserve(
            "scope-program",
            program
                .metadata_bytes()
                .saturating_add(capacity)
                .saturating_add(1024),
        )?;
        let mut sink = KeySink::recording("scope-program/v1", capacity);
        sink.part(b"model", &model.0);
        program.encode(&mut sink);
        let bytes = sink.recorded();
        let bucket = hash(&bytes);
        if let Some(existing) = self
            .buckets
            .get(&bucket)
            .and_then(|v| v.iter().find(|v| v.bytes == bytes))
        {
            return Ok(existing.clone());
        }
        self.charge
            .try_resize((self.entries + 1).saturating_mul(256))?;
        let compiled = Arc::new(CompiledScopeProgram {
            identity: ContentHash::of(&bytes),
            program,
            bytes,
            _charge: charge,
        });
        self.buckets
            .entry(bucket)
            .or_default()
            .push(compiled.clone());
        self.entries += 1;
        Ok(compiled)
    }
}

/// Dense prepared universe retains isolated vertices and identity-bearing parallel arcs.
pub struct CompactScope {
    graph: Graph<(usize, [u8; 16]), usize, Directed>,
    generation: ContentHash,
    _charge: Box<dyn Reservation>,
}
pub struct ScopeSelection {
    bits: FixedBitSet,
    generation: ContentHash,
    _charge: Box<dyn Reservation>,
}
impl ScopeSelection {
    pub fn contains(&self, index: usize) -> bool {
        self.bits.contains(index)
    }
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.bits.ones()
    }
    pub fn generation(&self) -> ContentHash {
        self.generation
    }
}
impl CompactScope {
    pub fn new(
        vertices: &[(usize, [u8; 16])],
        arcs: &[(usize, usize)],
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        if vertices.len() > u32::MAX as usize || arcs.len() > u32::MAX as usize {
            return Err(ModelError::Invalid("compact scope index overflow".into()));
        }
        let charge = budget.reserve(
            "compact-scope",
            vertices
                .len()
                .saturating_mul(128)
                .saturating_add(arcs.len().saturating_mul(64))
                .saturating_add(1024),
        )?;
        let mut graph = Graph::with_capacity(vertices.len(), arcs.len());
        let mut sink = KeySink::new("compact-scope-universe/v1");
        for &(port, key) in vertices {
            graph.add_node((port, key));
            sink.part(b"port", &(port as u64).to_le_bytes());
            sink.part(b"key", &key);
        }
        for (ordinal, &(a, b)) in arcs.iter().enumerate() {
            if a >= vertices.len() || b >= vertices.len() {
                return Err(ModelError::Schema("compact scope arc endpoint"));
            }
            graph.add_edge(NodeIndex::new(a), NodeIndex::new(b), ordinal);
        }
        Ok(Self {
            graph,
            generation: sink.finish(),
            _charge: charge,
        })
    }
    pub fn select(
        &self,
        seeds: &[usize],
        budget: &ResourceBudget,
    ) -> Result<ScopeSelection, ModelError> {
        let n = self.graph.node_count();
        let charge = budget.reserve(
            "compact-scope-selection",
            n.saturating_mul(16).saturating_add(1024),
        )?;
        let mut bits = FixedBitSet::with_capacity(n);
        let mut pending = Vec::new();
        for &seed in seeds {
            if seed >= n {
                return Err(ModelError::Schema("compact scope root absent"));
            }
            if !bits.put(seed) {
                pending.push(seed);
            }
        }
        while let Some(node) = pending.pop() {
            for next in self.graph.neighbors(NodeIndex::new(node)) {
                if !bits.put(next.index()) {
                    pending.push(next.index());
                }
            }
        }
        Ok(ScopeSelection {
            bits,
            generation: self.generation,
            _charge: charge,
        })
    }
}

/// Resolve an exact declared nominal input, including its immutable epoch.
pub fn input_binding(
    inputs: &[ValidationInput],
    expected: &ValidationInput,
) -> Result<usize, ModelError> {
    let mut candidates = inputs.iter().enumerate().filter(|(_, input)| {
        input.type_id() == expected.type_id() && input.prefix() == expected.prefix()
    });
    let (index, _) = candidates
        .next()
        .ok_or(ModelError::Conflict("scope input immutable binding absent"))?;
    if candidates.next().is_some() {
        return Err(ModelError::Conflict(
            "ambiguous scope input immutable binding",
        ));
    }
    Ok(index)
}
/// A unique target can cross epochs; multiple target declarations require the source's epoch.
pub fn field_target(
    inputs: &[ValidationInput],
    source: usize,
    kind: std::any::TypeId,
) -> Result<Option<usize>, ModelError> {
    let source = inputs
        .get(source)
        .ok_or(ModelError::Schema("scope reference input absent"))?;
    let mut all = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == kind);
    let Some((first, _)) = all.next() else {
        return Ok(None);
    };
    if all.next().is_none() {
        return Ok(Some(first));
    }
    let mut matching = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| input.type_id() == kind && input.prefix() == source.prefix());
    let (index, _) = matching
        .next()
        .ok_or(ModelError::Conflict("scope dependency epoch absent"))?;
    if matching.next().is_some() {
        return Err(ModelError::Conflict("ambiguous scope dependency epoch"));
    }
    Ok(Some(index))
}
/// Mechanical nominal dependencies remain one model-owned declaration for every consumer.
pub fn forward_rules(
    inputs: &[ValidationInput],
    model: &ValidatedModel,
) -> Result<Vec<ScopeRule>, ModelError> {
    let mut rules = Vec::new();
    for (source, input) in inputs.iter().enumerate() {
        let relation = model
            .relation(input.name())
            .ok_or(ModelError::Schema("scope relation absent"))?;
        if relation.type_id() != input.type_id() {
            return Err(ModelError::Conflict("scope nominal input binding"));
        }
        for field in relation.fields() {
            if let Some((kind, _)) = field.target()
                && let Some(target) = field_target(inputs, source, kind)?
            {
                rules.push(ScopeRule::Reference {
                    source,
                    field: field.name(),
                    target,
                    direction: ScopeDirection::Forward,
                    list: field.list(),
                });
            }
        }
    }
    Ok(rules)
}

#[cfg(test)]
mod controls {
    use super::*;
    fn program() -> ScopeProgram {
        ScopeProgram {
            inputs: vec![ValidationInput::of::<source::Occurrence>(&["id"])],
            ports: vec![ScopePort {
                input: 0,
                virtual_owner: false,
            }],
            rules: vec![],
        }
    }
    #[test]
    fn scope_hash_collisions_use_full_bytes_and_retain_charges() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut interner = ScopeInterner::new(&budget).unwrap();
        let a = interner
            .intern_with(program(), ContentHash([0; 32]), |_| 0)
            .unwrap();
        let b = interner
            .intern_with(program(), ContentHash([0; 32]), |_| 0)
            .unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        let mut changed = program();
        changed.ports[0].virtual_owner = true;
        let c = interner
            .intern_with(changed, ContentHash([0; 32]), |_| 0)
            .unwrap();
        assert!(!Arc::ptr_eq(&a, &c));
        assert_ne!(a.identity(), c.identity());
        assert_eq!(a.identity(), ContentHash::of(a.canonical_bytes()));
        drop(interner);
        assert!(budget.reserved() > 0);
        drop(a);
        drop(b);
        drop(c);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn nested_program_buffer_is_exact_and_preserves_ordinary_framed_digest() {
        let mut program = program();
        let column = |row, field| ScopeColumn { row, field };
        program.rules = vec![
            ScopeRule::OptionalPairs {
                source: 0,
                target: 0,
                rows: vec![0, 0],
                optional: vec![ScopeOptionalJoin {
                    row: 1,
                    keys: vec![(column(0, "id"), column(1, "id"))],
                }],
                predicates: vec![ScopePredicate::IsNull(column(1, "id"), false)],
                source_key: column(0, "id"),
                target_key: column(1, "id"),
            },
            ScopeRule::NearestPairs {
                source: 0,
                target: 0,
                rows: vec![0, 0],
                rank_rows: 2,
                predicates: vec![
                    ScopePredicate::CanonicalOccurrence { row: 1 },
                    ScopePredicate::PathPrefix {
                        parent: 1,
                        child: 0,
                    },
                ],
                post: vec![ScopePredicate::SpanContains {
                    outer_start: column(1, "start"),
                    outer_end: column(1, "end"),
                    inner_start: column(0, "start"),
                    inner_end: column(0, "end"),
                }],
                source_key: column(0, "id"),
                target_key: column(1, "id"),
                event_key: column(0, "id"),
                ancestor: 1,
            },
        ];
        let model = ContentHash([7; 32]);
        let budget = ResourceBudget::fixed(64 << 10).unwrap();
        let mut ordinary = KeySink::new("scope-program/v1");
        ordinary.part(b"model", &model.0);
        program.encode(&mut ordinary);
        let expected = ordinary.finish();
        let mut interner = ScopeInterner::new(&budget).unwrap();
        let compiled = interner
            .intern_with(program, model, xxhash_rust::xxh3::xxh3_128)
            .unwrap();
        assert_eq!(compiled.identity(), expected);
        assert_eq!(
            compiled.bytes.capacity(),
            compiled.bytes.len(),
            "canonical ownership must not retain a heuristic SQL arena"
        );
        assert!(budget.reserved() < 16 << 10);
        drop(interner);
        drop(compiled);
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn compact_scope_retains_isolates_parallel_arcs_cycles_and_partitions() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let graph = CompactScope::new(
            &[(0, [1; 16]), (0, [2; 16]), (1, [1; 16]), (0, [3; 16])],
            &[(0, 1), (0, 1), (1, 1), (1, 2), (2, 0)],
            &budget,
        )
        .unwrap();
        assert_eq!(graph.graph.edge_count(), 5);
        assert_eq!(
            graph
                .select(&[0], &budget)
                .unwrap()
                .iter()
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            graph
                .select(&[3], &budget)
                .unwrap()
                .iter()
                .collect::<Vec<_>>(),
            vec![3]
        );
    }
}
