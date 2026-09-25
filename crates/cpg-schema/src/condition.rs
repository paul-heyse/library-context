//! The closed condition language of the behavior model (ADR-0022 §Conditions; DESIGN §3.9).
//!
//! A condition is in **disjunctive normal form**: a disjunction of conjunctions of literals, a
//! literal being an atom with a polarity. Atoms test a **place**, written as spelled in the
//! record's scope (a root name, then at most two attribute segments):
//! `is_none(p)`, `equals(p,v)`, `member_of(p,{v…})`, `truthy(p)`, `isinstance(p,C)`; any other
//! test is `opaque("text")`.
//!
//! **Encoding** (the id's input, DP-04). A translated literal is
//! `[!]kind(place[,value])#module:source`; values are `None`,
//! `True`, `False`, decimal integers and JSON strings; a `member_of` set is sorted and
//! deduplicated. A conjunction's literals are sorted and deduplicated and joined by ` & `; a
//! disjunction's conjunctions are sorted, deduplicated and absorbed (a conjunction containing
//! another is dropped) and joined by ` | `. `true` is the empty conjunction and `false` the empty
//! disjunction. Two conditions are equal exactly when their encodings are: syntactic
//! equivalence, declared.
//!
//! **Normalization** is syntactic (ADR-0022 §Conditions, the Stage 2 review's F2): literals sorted
//! bytewise by encoding and deduplicated; a conjunction holding a literal and its negation dropped;
//! `x | !x` is `true`; self-subsuming resolution, repeated to a fixpoint: a literal `!y` is
//! dropped from a conjunction when another conjunction holds `y` and otherwise only literals the
//! first holds too (`a & r & !y | r & y` is `a & r | r & y`; `a | !a & b` is `a | b`); then
//! conjunctions sorted, deduplicated and absorbed.
//!
//! **Budget.** At most [`MAX_CONJUNCTIONS`] conjunctions of [`MAX_LITERALS`] literals; anything
//! larger is [`Condition::OverBudget`], which states nothing (its record is `unknown`,
//! `budget_reached`).
//!
//! Compatibility of two conditions waits for its first question (Stage 3's Q9; the review's F11).

use std::collections::BTreeSet;
use std::fmt;

use crate::codebook::ConditionAtom;
use crate::id::{Id, IdHasher};

/// Conjunctions a stated condition may hold.
pub const MAX_CONJUNCTIONS: usize = 16;
/// Literals one conjunction may hold.
pub const MAX_LITERALS: usize = 8;

/// A literal value a test compares a place with.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Str(String),
}

impl Value {
    pub fn encode(&self) -> String {
        match self {
            Value::None => "None".to_owned(),
            Value::Bool(true) => "True".to_owned(),
            Value::Bool(false) => "False".to_owned(),
            Value::Int(i) => i.to_string(),
            Value::Str(s) => serde_json::to_string(s).expect("a string serializes"),
        }
    }
}

/// What a literal tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// The values, sorted by encoding and deduplicated (see [`Atom::member_of`]).
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
    /// `type(p) is C`, with both names resolved to builtins before construction.
    TypeIs {
        place: String,
        class: String,
    },
    /// Any other test, as its source text with whitespace runs collapsed. `version` is retained
    /// only to parse prior Stage 2 encodings; translated tests carry evaluation identity.
    Opaque {
        text: String,
        version: Option<usize>,
    },
    /// The identity of an evaluation. A definition set permits sharing only for stable tests;
    /// all other tests keep their source site. The inner atom supplies the readable label.
    Evaluated {
        atom: Box<Atom>,
        identity: EvaluationIdentity,
    },
}

/// Identity beyond the test's printed operands. The module key includes the relative path and
/// source content; definition offsets are sorted and deduplicated before construction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EvaluationIdentity {
    Site {
        module: String,
        start: u32,
        end: u32,
    },
    Definitions {
        module: String,
        starts: Vec<u32>,
    },
    Synthetic {
        module: String,
        predicate: String,
    },
}

impl EvaluationIdentity {
    fn encode(&self) -> String {
        match self {
            Self::Site { module, start, end } => format!("{module}:s{start}-{end}"),
            Self::Definitions { module, starts } => format!(
                "{module}:d{}",
                starts
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Synthetic { module, predicate } => format!("{module}:p{predicate}"),
        }
    }

    fn parse(text: &str) -> Result<Self, String> {
        let (module, rest) = text.split_once(':').ok_or("identity needs a module")?;
        if module.len() != 32 || !module.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("identity needs a 16-byte module hex key".to_owned());
        }
        if let Some(site) = rest.strip_prefix('s') {
            let (start, end) = site.split_once('-').ok_or("site needs a span")?;
            return Ok(Self::Site {
                module: module.to_owned(),
                start: start.parse().map_err(|e| format!("site start: {e}"))?,
                end: end.parse().map_err(|e| format!("site end: {e}"))?,
            });
        }
        if let Some(defs) = rest.strip_prefix('d') {
            let starts = defs
                .split(',')
                .map(|x| x.parse::<u32>().map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            if starts.is_empty() || starts.windows(2).any(|x| x[0] >= x[1]) {
                return Err("definition offsets must be nonempty, sorted and unique".to_owned());
            }
            return Ok(Self::Definitions {
                module: module.to_owned(),
                starts,
            });
        }
        if let Some(predicate) = rest.strip_prefix('p') {
            if predicate.len() != 32 || !predicate.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("predicate identity needs a 16-byte hex key".to_owned());
            }
            return Ok(Self::Synthetic {
                module: module.to_owned(),
                predicate: predicate.to_owned(),
            });
        }
        Err("unknown evaluation identity".to_owned())
    }
}

impl Atom {
    /// Parse one canonical atom encoding, without a polarity prefix.
    pub fn parse_encoded(text: &str) -> Result<Self, String> {
        let literal = parse_literal(text)?;
        if !literal.positive {
            return Err("an atom encoding cannot include a polarity prefix".to_owned());
        }
        Ok(literal.atom)
    }

    pub fn evaluated(self, identity: EvaluationIdentity) -> Atom {
        Atom::Evaluated {
            atom: Box::new(self),
            identity,
        }
    }

    /// A `member_of` atom with its values in canonical order. Membership keeps its operator even
    /// for a one-element container: Python may dispatch equality with different operands.
    pub fn member_of(place: impl Into<String>, values: impl IntoIterator<Item = Value>) -> Atom {
        let mut values: Vec<Value> = values.into_iter().collect();
        values.sort_by_key(Value::encode);
        values.dedup();
        let place = place.into();
        Atom::MemberOf { place, values }
    }

    /// An opaque atom over `text`, whitespace runs collapsed to one space.
    pub fn opaque(text: &str) -> Atom {
        Atom::opaque_at(text, None)
    }

    /// An opaque atom reading a rebound place's value as of `version` (see [`Atom::Opaque`]).
    pub fn opaque_at(text: &str, version: Option<usize>) -> Atom {
        Atom::Opaque {
            text: text.split_whitespace().collect::<Vec<_>>().join(" "),
            version,
        }
    }

    pub fn kind(&self) -> ConditionAtom {
        match self {
            Atom::IsNone { .. } => ConditionAtom::IsNone,
            Atom::IsValue { .. } => ConditionAtom::IsValue,
            Atom::Equals { .. } => ConditionAtom::Equals,
            Atom::MemberOf { .. } => ConditionAtom::MemberOf,
            Atom::Truthy { .. } => ConditionAtom::Truthy,
            Atom::IsInstance { .. } => ConditionAtom::IsInstance,
            Atom::TypeIs { .. } => ConditionAtom::TypeIs,
            Atom::Opaque { .. } => ConditionAtom::Opaque,
            Atom::Evaluated { atom, .. } => atom.kind(),
        }
    }

    /// The place tested; `None` for an opaque atom.
    pub fn place(&self) -> Option<&str> {
        match self {
            Atom::IsNone { place }
            | Atom::IsValue { place, .. }
            | Atom::Equals { place, .. }
            | Atom::MemberOf { place, .. }
            | Atom::Truthy { place }
            | Atom::IsInstance { place, .. }
            | Atom::TypeIs { place, .. } => Some(place),
            Atom::Opaque { .. } => None,
            Atom::Evaluated { atom, .. } => atom.place(),
        }
    }

    /// The atom's argument after the place, as encoded: a value, a sorted value set, a class, or
    /// the opaque text.
    pub fn argument(&self) -> Option<String> {
        match self {
            Atom::IsNone { .. } | Atom::Truthy { .. } => None,
            Atom::Equals { value, .. } => Some(value.encode()),
            Atom::IsValue { value, .. } => Some(value.encode()),
            Atom::MemberOf { values, .. } => Some(format!(
                "{{{}}}",
                values
                    .iter()
                    .map(Value::encode)
                    .collect::<Vec<_>>()
                    .join(",")
            )),
            Atom::IsInstance { class, .. } => Some(class.clone()),
            Atom::TypeIs { class, .. } => Some(class.clone()),
            Atom::Opaque { text, version } => {
                let text = serde_json::to_string(text).expect("a string serializes");
                Some(match version {
                    Some(v) => format!("{text}@{v}"),
                    None => text,
                })
            }
            Atom::Evaluated { atom, .. } => atom.argument(),
        }
    }

    pub fn encode(&self) -> String {
        if let Atom::Evaluated { atom, identity } = self {
            return format!("{}#{}", atom.encode(), identity.encode());
        }
        let kind = crate::Codebook::text(self.kind());
        if let Atom::Opaque { text, version } = self {
            let text = serde_json::to_string(text).expect("a string serializes");
            return match version {
                Some(v) => format!("{kind}({text})@{v}"),
                None => format!("{kind}({text})"),
            };
        }
        match (self.place(), self.argument()) {
            (Some(p), Some(a)) => format!("{kind}({p},{a})"),
            (Some(p), None) => format!("{kind}({p})"),
            (None, Some(a)) => format!("{kind}({a})"),
            (None, None) => format!("{kind}()"),
        }
    }
}

/// An atom with a polarity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Literal {
    pub atom: Atom,
    pub positive: bool,
}

impl Literal {
    pub fn new(atom: Atom, positive: bool) -> Self {
        Literal { atom, positive }
    }

    pub fn encode(&self) -> String {
        if self.positive {
            self.atom.encode()
        } else {
            format!("!{}", self.atom.encode())
        }
    }

    fn negated(&self) -> Literal {
        Literal {
            atom: self.atom.clone(),
            positive: !self.positive,
        }
    }
}

/// A condition in normal form, or one too large to state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition {
    /// Conjunctions, each sorted by encoding; the list sorted and absorbed.
    Dnf(Vec<Vec<Literal>>),
    /// Past the budget: states nothing.
    OverBudget,
}

impl Condition {
    pub fn always() -> Self {
        Condition::Dnf(vec![Vec::new()])
    }

    pub fn never() -> Self {
        Condition::Dnf(Vec::new())
    }

    pub fn literal(l: Literal) -> Self {
        Condition::from_dnf(vec![vec![l]])
    }

    pub fn atom(a: Atom) -> Self {
        Condition::literal(Literal::new(a, true))
    }

    pub fn is_always(&self) -> bool {
        matches!(self, Condition::Dnf(d) if d.len() == 1 && d[0].is_empty())
    }

    pub fn is_never(&self) -> bool {
        matches!(self, Condition::Dnf(d) if d.is_empty())
    }

    /// Whether some literal is opaque.
    pub fn has_opaque(&self) -> bool {
        match self {
            Condition::Dnf(d) => d
                .iter()
                .flatten()
                .any(|l| l.atom.kind() == ConditionAtom::Opaque),
            Condition::OverBudget => false,
        }
    }

    /// Normalize raw conjunctions (DESIGN §3.9; see the module docs): literals sorted and
    /// deduplicated; a conjunction holding a literal and its negation dropped; self-subsuming
    /// resolution (`a & !y | y` is `a | y`); the rest sorted, deduplicated and absorbed; then the
    /// budget.
    pub fn from_dnf(raw: Vec<Vec<Literal>>) -> Self {
        let mut conjunctions: Vec<Vec<Literal>> = Vec::new();
        for mut c in raw {
            c.sort_by_cached_key(Literal::encode);
            c.dedup();
            let encodings: BTreeSet<String> = c.iter().map(Literal::encode).collect();
            if c.iter().any(|l| encodings.contains(&l.negated().encode())) {
                continue;
            }
            conjunctions.push(c);
        }
        // Resolution below is order-sensitive, so it runs over the sorted, deduplicated
        // conjunctions: one input, in any order, has one encoding (the Stage 2 end review's R9).
        conjunctions.sort_by_cached_key(|c| encode_conjunction(c));
        conjunctions.dedup();
        // `x | !x` is `true`: a literal and its negation each standing alone.
        let alone: BTreeSet<String> = conjunctions
            .iter()
            .filter(|c| c.len() == 1)
            .map(|c| c[0].encode())
            .collect();
        if conjunctions
            .iter()
            .filter(|c| c.len() == 1)
            .any(|c| alone.contains(&c[0].negated().encode()))
        {
            return Condition::always();
        }
        // Self-subsuming resolution: `a & r & !y | r & y` is `a & r | r & y`. A literal is dropped
        // from a conjunction when another conjunction holds its negation and otherwise only
        // literals it holds too (`x | !x & r` is `x | r` is the case with nothing else). A
        // conjunction left empty makes the condition `true`.
        loop {
            let sets: Vec<BTreeSet<String>> = conjunctions
                .iter()
                .map(|c| c.iter().map(Literal::encode).collect())
                .collect();
            let mut change = None;
            'find: for (i, c) in conjunctions.iter().enumerate() {
                for (k, l) in c.iter().enumerate() {
                    let negated = l.negated().encode();
                    let own = l.encode();
                    let resolves = sets.iter().enumerate().any(|(j, s)| {
                        j != i
                            && s.contains(&negated)
                            && s.iter()
                                .all(|x| *x == negated || (*x != own && sets[i].contains(x)))
                    });
                    if resolves {
                        change = Some((i, k));
                        break 'find;
                    }
                }
            }
            let Some((i, k)) = change else {
                break;
            };
            conjunctions[i].remove(k);
            if conjunctions[i].is_empty() {
                return Condition::always();
            }
        }
        conjunctions.sort_by_cached_key(|c| encode_conjunction(c));
        conjunctions.dedup();
        let sets: Vec<BTreeSet<String>> = conjunctions
            .iter()
            .map(|c| c.iter().map(Literal::encode).collect())
            .collect();
        let kept: Vec<Vec<Literal>> = conjunctions
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                !sets
                    .iter()
                    .enumerate()
                    .any(|(j, s)| j != *i && s.len() < sets[*i].len() && s.is_subset(&sets[*i]))
            })
            .map(|(_, c)| c.clone())
            .collect();
        if kept.len() > MAX_CONJUNCTIONS || kept.iter().any(|c| c.len() > MAX_LITERALS) {
            return Condition::OverBudget;
        }
        Condition::Dnf(kept)
    }

    pub fn and(&self, other: &Condition) -> Condition {
        match (self, other) {
            (Condition::Dnf(a), Condition::Dnf(b)) => {
                if a.len() * b.len() > MAX_CONJUNCTIONS * MAX_CONJUNCTIONS {
                    return Condition::OverBudget;
                }
                let mut out = Vec::with_capacity(a.len() * b.len());
                for x in a {
                    for y in b {
                        let mut c = x.clone();
                        c.extend(y.iter().cloned());
                        out.push(c);
                    }
                }
                Condition::from_dnf(out)
            }
            _ => Condition::OverBudget,
        }
    }

    pub fn or(&self, other: &Condition) -> Condition {
        match (self, other) {
            (Condition::Dnf(a), Condition::Dnf(b)) => {
                let mut out = a.clone();
                out.extend(b.iter().cloned());
                Condition::from_dnf(out)
            }
            _ => Condition::OverBudget,
        }
    }

    /// The negation, by De Morgan: the conjunction of each conjunction's negated literals taken as
    /// a disjunction.
    pub fn not(&self) -> Condition {
        match self {
            Condition::Dnf(d) => {
                let mut acc = Condition::always();
                for c in d {
                    let clause = Condition::from_dnf(c.iter().map(|l| vec![l.negated()]).collect());
                    acc = acc.and(&clause);
                    if acc == Condition::OverBudget {
                        return acc;
                    }
                }
                acc
            }
            Condition::OverBudget => Condition::OverBudget,
        }
    }

    /// This condition with a factor assumed true: when every conjunction contains a conjunction
    /// of `factor`, every conjunction of `factor` is used, and the remainders agree, that
    /// remainder; otherwise the condition unchanged. It factors out a function's normal path (the
    /// negation of a guard that raises) from what a fate is stated under.
    pub fn given(&self, factor: &Condition) -> Condition {
        // A condition past the budget states nothing, so nothing factors out of it (the Stage 2
        // end review's R3): `OverBudget` given `OverBudget` stays `OverBudget`.
        let (Condition::Dnf(c), Condition::Dnf(f)) = (self, factor) else {
            return self.clone();
        };
        if c == f {
            return Condition::always();
        }
        if f.is_empty() || f.iter().any(Vec::is_empty) {
            return self.clone();
        }
        let set =
            |x: &Vec<Literal>| -> BTreeSet<String> { x.iter().map(Literal::encode).collect() };
        let factors: Vec<BTreeSet<String>> = f.iter().map(set).collect();
        let mut used = BTreeSet::new();
        let mut rest: Option<BTreeSet<String>> = None;
        for conj in c {
            let have = set(conj);
            let Some((i, b)) = factors.iter().enumerate().find(|(_, b)| b.is_subset(&have)) else {
                return self.clone();
            };
            used.insert(i);
            let r: BTreeSet<String> = have.difference(b).cloned().collect();
            match &rest {
                None => rest = Some(r),
                Some(x) if *x == r => {}
                Some(_) => return self.clone(),
            }
        }
        if used.len() != factors.len() {
            return self.clone();
        }
        let rest = rest.unwrap_or_default();
        let literals: Vec<Literal> = c
            .iter()
            .flatten()
            .filter(|l| rest.contains(&l.encode()))
            .cloned()
            .collect();
        Condition::from_dnf(vec![literals])
    }

    /// The canonical encoding (`over_budget` for a condition past the budget).
    pub fn encode(&self) -> String {
        match self {
            Condition::OverBudget => "over_budget".to_owned(),
            Condition::Dnf(d) if d.is_empty() => "false".to_owned(),
            Condition::Dnf(d) if d.len() == 1 && d[0].is_empty() => "true".to_owned(),
            Condition::Dnf(d) => d
                .iter()
                .map(|c| encode_conjunction(c))
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }

    pub fn id(&self) -> Id {
        IdHasher::new("condition").str(&self.encode()).finish_id()
    }

    /// Parse an encoding back (tests and the known answers; `parse(encode(c)) == c`).
    pub fn parse(text: &str) -> Result<Condition, String> {
        match text {
            "true" => return Ok(Condition::always()),
            "false" => return Ok(Condition::never()),
            "over_budget" => return Ok(Condition::OverBudget),
            _ => {}
        }
        let mut raw = Vec::new();
        for conj in split_top(text, " | ") {
            let mut c = Vec::new();
            for lit in split_top(&conj, " & ") {
                c.push(parse_literal(&lit)?);
            }
            raw.push(c);
        }
        Ok(Condition::from_dnf(raw))
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

fn encode_conjunction(c: &[Literal]) -> String {
    if c.is_empty() {
        return "true".to_owned();
    }
    c.iter()
        .map(Literal::encode)
        .collect::<Vec<_>>()
        .join(" & ")
}

/// Split at `sep` outside parentheses, braces and JSON strings.
fn split_top(text: &str, sep: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut depth, mut in_str, mut escaped) = (0i32, false, false);
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
        } else if b == b'"' {
            in_str = true;
        } else if b == b'(' || b == b'{' {
            depth += 1;
        } else if b == b')' || b == b'}' {
            depth -= 1;
        } else if depth == 0 && text[i..].starts_with(sep) {
            out.push(text[start..i].to_owned());
            i += sep.len();
            start = i;
            continue;
        }
        i += 1;
    }
    out.push(text[start..].to_owned());
    out
}

fn parse_value(text: &str) -> Result<Value, String> {
    match text {
        "None" => Ok(Value::None),
        "True" => Ok(Value::Bool(true)),
        "False" => Ok(Value::Bool(false)),
        t if t.starts_with('"') => serde_json::from_str::<String>(t)
            .map(Value::Str)
            .map_err(|e| format!("{t}: {e}")),
        t => t
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|e| format!("{t}: {e}")),
    }
}

fn parse_literal(text: &str) -> Result<Literal, String> {
    let (positive, rest) = match text.strip_prefix('!') {
        Some(r) => (false, r),
        None => (true, text),
    };
    // The identity suffix follows the closing parenthesis (and any legacy opaque version).
    // A `)#` inside an opaque JSON string is label text, not an identity suffix.
    let (rest, identity) = match rest.rsplit_once(")#") {
        Some((head, suffix)) => match EvaluationIdentity::parse(suffix) {
            Ok(identity) => (format!("{head})"), Some(identity)),
            Err(_) => (rest.to_owned(), None),
        },
        None => (rest.to_owned(), None),
    };
    let open = rest.find('(').ok_or_else(|| format!("no `(` in {text}"))?;
    let kind = &rest[..open];
    // A versioned opaque atom: `opaque("…")@line`.
    let (rest, version) = match (kind, rest.rsplit_once(")@")) {
        ("opaque", Some((head, v))) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
            let head = format!("{head})");
            (
                head,
                Some(v.parse::<usize>().map_err(|e| format!("{text}: {e}"))?),
            )
        }
        _ => (rest.to_owned(), None),
    };
    let rest = rest.as_str();
    let inner = rest[open + 1..]
        .strip_suffix(')')
        .ok_or_else(|| format!("no `)` in {text}"))?;
    let args = split_top(inner, ",");
    let place = || args[0].clone();
    let atom = match kind {
        "is_none" => Atom::IsNone { place: place() },
        "is_value" => Atom::IsValue {
            place: place(),
            value: parse_value(args.get(1).ok_or("is_value needs a value")?)?,
        },
        "truthy" => Atom::Truthy { place: place() },
        "equals" => Atom::Equals {
            place: place(),
            value: parse_value(args.get(1).ok_or("equals needs a value")?)?,
        },
        "isinstance" => Atom::IsInstance {
            place: place(),
            class: args.get(1).ok_or("isinstance needs a class")?.clone(),
        },
        "type_is" => Atom::TypeIs {
            place: place(),
            class: args.get(1).ok_or("type_is needs a class")?.clone(),
        },
        "member_of" => {
            let set = args.get(1).ok_or("member_of needs a set")?;
            let set = set
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
                .ok_or("member_of's set is braced")?;
            let values = split_top(set, ",")
                .iter()
                .map(|v| parse_value(v))
                .collect::<Result<Vec<_>, _>>()?;
            Atom::member_of(place(), values)
        }
        "opaque" => Atom::opaque_at(
            &serde_json::from_str::<String>(inner).map_err(|e| format!("{inner}: {e}"))?,
            version,
        ),
        other => return Err(format!("unknown atom kind {other}")),
    };
    Ok(Literal::new(
        match identity {
            Some(i) => atom.evaluated(i),
            None => atom,
        },
        positive,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(text: &str) -> Condition {
        Condition::parse(text).unwrap()
    }

    #[test]
    fn the_encoding_round_trips_and_is_canonical() {
        let a = c("!is_none(x) & equals(mode,\"http\")");
        assert_eq!(a.encode(), "!is_none(x) & equals(mode,\"http\")");
        let b = c("equals(mode,\"http\") & !is_none(x)");
        assert_eq!(a, b);
        assert_eq!(a.id(), b.id());
        let m = c("member_of(t,{\"sse\",\"http\",\"sse\"})");
        assert_eq!(m.encode(), "member_of(t,{\"http\",\"sse\"})");
        assert_eq!(c("member_of(t,{\"x\"})").encode(), "member_of(t,{\"x\"})");
        let o = c("opaque(\"a  and\\n b\")");
        assert_eq!(Condition::atom(Atom::opaque("a  and\n b")), o);
        assert_eq!(c("opaque(\"x)#y\")").encode(), "opaque(\"x)#y\")");
        let exact_type = c("type_is(x,str)");
        assert_eq!(
            exact_type,
            Condition::atom(Atom::TypeIs {
                place: "x".into(),
                class: "str".into()
            })
        );
        assert_ne!(exact_type.id(), c("isinstance(x,str)").id());
    }

    #[test]
    fn evaluations_keep_operator_and_site_identity() {
        let module = "0123456789abcdef0123456789abcdef".to_owned();
        let site = |start| EvaluationIdentity::Site {
            module: module.clone(),
            start,
            end: start + 7,
        };
        let a = Condition::atom(
            Atom::Equals {
                place: "x".into(),
                value: Value::None,
            }
            .evaluated(site(4)),
        );
        let b = Condition::atom(
            Atom::Equals {
                place: "x".into(),
                value: Value::None,
            }
            .evaluated(site(20)),
        );
        let identity = Condition::atom(Atom::IsNone { place: "x".into() }.evaluated(site(4)));
        let definitions = Condition::atom(Atom::Truthy { place: "x".into() }.evaluated(
            EvaluationIdentity::Definitions {
                module: module.clone(),
                starts: vec![4, 20],
            },
        ));
        let synthetic = Condition::atom(
            Atom::Opaque {
                text: "unknown branch".into(),
                version: None,
            }
            .evaluated(EvaluationIdentity::Synthetic {
                module,
                predicate: "fedcba9876543210fedcba9876543210".into(),
            }),
        );
        assert_ne!(a.id(), b.id());
        assert_ne!(a.id(), identity.id());
        assert!(!a.and(&b.not()).is_never());
        for c in [a, b, identity, definitions, synthetic] {
            assert_eq!(Condition::parse(&c.encode()).unwrap(), c);
        }
    }

    #[test]
    fn contradictions_and_absorption_normalize() {
        assert!(c("is_none(x) & !is_none(x)").is_never());
        assert_eq!(c("truthy(a) | truthy(a) & truthy(b)").encode(), "truthy(a)");
        let n = c("truthy(a) & !is_none(b)").not();
        assert_eq!(n.encode(), "!truthy(a) | is_none(b)");
        assert!(Condition::always().not().is_never());
        assert!(Condition::never().not().is_always());
    }

    #[test]
    fn a_negated_literal_beside_its_lone_positive_is_dropped() {
        assert_eq!(
            c("truthy(a) | !truthy(a) & truthy(b)").encode(),
            "truthy(a) | truthy(b)"
        );
        // A tautology reduces to `true`.
        assert!(c("is_none(x) | !is_none(x) & !truthy(y) | truthy(y)").is_always());
        assert!(c("opaque(\"s\") | !opaque(\"s\")").is_always());
    }

    #[test]
    fn a_normal_path_factors_out() {
        let guard = c("!is_none(n) & opaque(\"n <= 0\")").not();
        assert!(guard.given(&guard).is_always());
        let fate = guard.and(&c("truthy(verbose)"));
        assert_eq!(fate.given(&guard).encode(), "truthy(verbose)");
        let other = c("truthy(x) | truthy(y)");
        assert_eq!(other.given(&guard), other, "no factor, no change");
    }

    #[test]
    fn the_budget_bounds_a_condition() {
        let mut acc = Condition::never();
        for i in 0..MAX_CONJUNCTIONS {
            acc = acc.or(&c(&format!("truthy(p{i})")));
        }
        assert!(matches!(acc, Condition::Dnf(ref d) if d.len() == MAX_CONJUNCTIONS));
        assert_eq!(acc.or(&c("truthy(extra)")), Condition::OverBudget);
    }
}
