//! Calls, named call policies and the one binder (DESIGN §15.5).
//!
//! "Which calls count" is decided only by a [`CallPolicy`]: a typed admission predicate over a call
//! target's facts, evaluated in Rust ([`CallPolicy::admits`]) and rendered once to SQL
//! ([`CallPolicy::sql`]) from the same predicate, so a view and a Rust consumer cannot disagree.
//! [`bind`] is the one binder: a pure function of call syntax, one target alternative and one
//! signature variant, producing call bindings keyed by site × target × variant × actual → formal.

use crate::decl::codebook::Codebook;
use crate::id::Id;
use crate::obligation::ObligationKind;
use crate::vocab::FormalRef;

crate::codebook!(
    /// How certain a call target alternative is. Append-only.
    Modality = "modality" {
        Definite = 0 => "definite",
        Candidate = 1 => "candidate",
        Potential = 2 => "potential",
    }
);

crate::codebook!(
    /// Who asserts a call target. Append-only.
    CallOrigin = "call_origin" {
        InputContext = 0 => "input_context",
        SourceObservation = 1 => "source_observation",
        AnalyzerAssertion = 2 => "analyzer_assertion",
        DerivedAnalysis = 3 => "derived_analysis",
        SyntheticModel = 4 => "synthetic_model",
    }
);

crate::codebook!(
    /// What the call does at its site. Definition arcs are kept apart from calls. Append-only.
    CallPhase = "call_phase" {
        Call = 0 => "call",
        New = 1 => "new",
        Init = 2 => "init",
        Decorator = 3 => "decorator",
        PropertyGet = 4 => "property_get",
        PropertySet = 5 => "property_set",
        Definition = 6 => "definition",
    }
);

crate::codebook!(
    /// What a target alternative resolves to. Append-only.
    TargetKind = "target_kind" {
        Function = 0 => "function",
        Method = 1 => "method",
        Class = 2 => "class",
        External = 3 => "external",
        Synthetic = 4 => "synthetic",
        Unresolved = 5 => "unresolved",
    }
);

/// The facts a call policy reads about one target alternative: exactly the policy columns of the
/// call-target relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetFacts {
    pub modality: Modality,
    pub origin: CallOrigin,
    pub phase: CallPhase,
    pub target_kind: TargetKind,
    /// An implicit invocation (a decorator application), disclosed, never merged with calls.
    pub implicit: bool,
    /// The site has exactly one target alternative.
    pub unique: bool,
    /// The site's resolution is complete under the provider model.
    pub complete: bool,
}

/// A fact a policy predicate reads, with its relation column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fact {
    Modality,
    Origin,
    Phase,
    TargetKind,
    Implicit,
    Unique,
    Complete,
}

impl Fact {
    pub fn column(self) -> &'static str {
        match self {
            Self::Modality => "modality",
            Self::Origin => "origin",
            Self::Phase => "phase",
            Self::TargetKind => "target_kind",
            Self::Implicit => "implicit",
            Self::Unique => "is_unique",
            Self::Complete => "is_complete",
        }
    }

    fn code(self, facts: &TargetFacts) -> i16 {
        match self {
            Self::Modality => facts.modality.code(),
            Self::Origin => facts.origin.code(),
            Self::Phase => facts.phase.code(),
            Self::TargetKind => facts.target_kind.code(),
            Self::Implicit | Self::Unique | Self::Complete => i16::from(self.flag(facts)),
        }
    }

    fn flag(self, facts: &TargetFacts) -> bool {
        match self {
            Self::Implicit => facts.implicit,
            Self::Unique => facts.unique,
            Self::Complete => facts.complete,
            _ => false,
        }
    }
}

/// A policy's admission predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admit {
    All,
    /// A codebook fact is one of these codes.
    In(Fact, Vec<i16>),
    /// A Boolean fact holds.
    Flag(Fact),
    And(Vec<Admit>),
}

impl Admit {
    fn codes<C: Codebook>(fact: Fact, values: &[C]) -> Self {
        Self::In(fact, values.iter().map(|v| v.code()).collect())
    }

    pub fn eval(&self, facts: &TargetFacts) -> bool {
        match self {
            Self::All => true,
            Self::In(fact, codes) => codes.contains(&fact.code(facts)),
            Self::Flag(fact) => fact.flag(facts),
            Self::And(parts) => parts.iter().all(|p| p.eval(facts)),
        }
    }

    pub fn sql(&self) -> String {
        match self {
            Self::All => "TRUE".into(),
            Self::In(fact, codes) => format!(
                "{} IN ({})",
                fact.column(),
                codes.iter().map(i16::to_string).collect::<Vec<_>>().join(", ")
            ),
            Self::Flag(fact) => fact.column().into(),
            Self::And(parts) => parts
                .iter()
                .map(|p| format!("({})", p.sql()))
                .collect::<Vec<_>>()
                .join(" AND "),
        }
    }
}

/// The five named call policies (DESIGN §15.5). A consumer selects a policy view; it never
/// re-filters targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CallPolicy {
    /// Analyzer assertions, definite or candidate, in call and property phases: the invocation
    /// projection and delegation. Definition arcs are kept separate.
    Invocation,
    /// Function or method targets in call and init phases: flow composition and handoffs.
    Dataflow,
    /// A definite, single, complete target between callables at a call or init: summary
    /// instantiation. Every call it admits, the dataflow policy admits too.
    Summary,
    /// Definite or candidate targets with declared origins: usage ranking.
    Usage,
    /// Every origin, with origin and implicit flag disclosed: catalog evidence.
    Association,
}

impl CallPolicy {
    pub const ALL: [CallPolicy; 5] = [
        Self::Invocation,
        Self::Dataflow,
        Self::Summary,
        Self::Usage,
        Self::Association,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Invocation => "invocation",
            Self::Dataflow => "dataflow",
            Self::Summary => "summary",
            Self::Usage => "usage",
            Self::Association => "association",
        }
    }

    pub fn predicate(self) -> Admit {
        use CallPhase as P;
        use TargetKind as T;
        let callable = Admit::codes(Fact::TargetKind, &[T::Function, T::Method]);
        match self {
            Self::Invocation => Admit::And(vec![
                Admit::codes(Fact::Origin, &[CallOrigin::AnalyzerAssertion]),
                Admit::codes(Fact::Modality, &[Modality::Definite, Modality::Candidate]),
                Admit::codes(Fact::Phase, &[P::Call, P::PropertyGet, P::PropertySet]),
            ]),
            Self::Dataflow => Admit::And(vec![
                callable,
                Admit::codes(Fact::Phase, &[P::Call, P::Init]),
            ]),
            Self::Summary => Admit::And(vec![
                Admit::codes(Fact::Modality, &[Modality::Definite]),
                Admit::Flag(Fact::Unique),
                Admit::Flag(Fact::Complete),
                callable,
                Admit::codes(Fact::Phase, &[P::Call, P::Init]),
            ]),
            Self::Usage => Admit::And(vec![
                Admit::codes(Fact::Modality, &[Modality::Definite, Modality::Candidate]),
                Admit::codes(
                    Fact::Origin,
                    &[CallOrigin::AnalyzerAssertion, CallOrigin::SourceObservation],
                ),
            ]),
            Self::Association => Admit::All,
        }
    }

    pub fn admits(self, facts: &TargetFacts) -> bool {
        self.predicate().eval(facts)
    }

    /// The SQL admission predicate over the call-target relation's policy columns.
    pub fn sql(self) -> String {
        self.predicate().sql()
    }
}

crate::codebook!(
    /// A formal's kind. Append-only.
    ParameterKind = "parameter_kind" {
        PositionalOnly = 0 => "positional_only",
        PositionalOrKeyword = 1 => "positional_or_keyword",
        VarPositional = 2 => "var_positional",
        KeywordOnly = 3 => "keyword_only",
        VarKeyword = 4 => "var_keyword",
    }
);

crate::codebook!(
    /// An actual's syntax. Append-only.
    ArgumentKind = "argument_kind" {
        Positional = 0 => "positional",
        Starred = 1 => "starred",
        Keyword = 2 => "keyword",
        DoubleStarred = 3 => "double_starred",
        /// A value an implicit invocation passes (a decorated definition).
        Implicit = 4 => "implicit",
    }
);

crate::codebook!(
    /// How an actual reaches a formal. Append-only.
    BindingKind = "binding_kind" {
        Positional = 0 => "positional",
        Keyword = 1 => "keyword",
        Default = 2 => "default",
        Varargs = 3 => "varargs",
        Kwargs = 4 => "kwargs",
        Receiver = 5 => "receiver",
        Implicit = 6 => "implicit",
    }
);

crate::codebook!(
    /// Whether a binding holds. Append-only.
    BindingStatus = "binding_status" {
        Bound = 0 => "bound",
        /// Unpacking leaves the formal undetermined.
        Ambiguous = 1 => "ambiguous",
        /// The actual matches no formal of this variant.
        Unmapped = 2 => "unmapped",
        /// This variant cannot accept the call (a missing required or doubly bound formal).
        Refused = 3 => "refused",
    }
);

/// One formal of a signature variant, in declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formal {
    pub formal: FormalRef,
    pub name: String,
    pub kind: ParameterKind,
    pub has_default: bool,
}

/// How a target alternative receives its first formal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Receiver {
    /// Called as a plain function: no implicit first argument.
    None,
    /// Called through an instance or class attribute: the receiver binds the first formal.
    Bound,
}

/// One signature variant of an effective callable. Alternatives stay separate: independent
/// variants never jointly establish one valid invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureVariant {
    pub callable: Id,
    pub variant: u32,
    pub formals: Vec<Formal>,
}

/// A declaration's effective callable after the single decorator and wrapper normalization
/// (DESIGN §15.5). Produced in cutover phase 3; the contract is fixed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveCallable {
    pub declaration: Id,
    pub callable: Id,
    /// The decorator and wrapper applications normalized away, outermost first.
    pub normalization: Vec<Id>,
    pub variants: Vec<SignatureVariant>,
}

/// One actual at a call site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actual {
    pub occurrence: Id,
    pub kind: ArgumentKind,
    pub keyword: Option<String>,
}

/// A call site's syntax: its actuals in source order and its receiver occurrence, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSyntax {
    pub site: Id,
    pub receiver: Option<Id>,
    pub actuals: Vec<Actual>,
}

/// One target alternative at a site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetAlternative {
    pub target: Id,
    pub receiver: Receiver,
}

/// One call binding: site × target × variant × actual → formal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallBinding {
    pub site: Id,
    pub target: Id,
    pub variant: u32,
    /// The actual occurrence; none for a default.
    pub actual: Option<Id>,
    /// The formal; none when the actual is unmapped or ambiguous.
    pub formal: Option<FormalRef>,
    pub kind: BindingKind,
    pub status: BindingStatus,
}

impl CallBinding {
    /// The obligation a non-bound binding leaves on anything that flows through it.
    pub fn obligation(&self) -> Option<ObligationKind> {
        match self.status {
            BindingStatus::Bound => None,
            BindingStatus::Ambiguous => Some(if self.kind == BindingKind::Varargs
                || self.kind == BindingKind::Kwargs
            {
                ObligationKind::UnsupportedUnpacking
            } else {
                ObligationKind::AmbiguousBinding
            }),
            BindingStatus::Unmapped | BindingStatus::Refused => {
                Some(ObligationKind::AmbiguousBinding)
            }
        }
    }
}

/// Bind a call's actuals to one target variant's formals (the one binder).
///
/// - A bound receiver binds the first formal.
/// - Positional actuals fill positional formals in order, then the `*args` formal; keyword
///   actuals match positional-or-keyword and keyword-only formals by name, then `**kwargs`.
/// - After a starred (`*xs`) actual, every later positional binding is ambiguous; a
///   double-starred (`**kw`) actual leaves every still-unbound keyword-capable formal ambiguous.
/// - An actual no formal accepts is unmapped; a formal bound twice, or a required formal left
///   unbound with no ambiguity to cover it, is refused; an unbound formal with a default binds as
///   a default. Unpacking is never an error.
pub fn bind(
    call: &CallSyntax,
    target: TargetAlternative,
    variant: &SignatureVariant,
) -> Vec<CallBinding> {
    let row = |actual: Option<Id>, formal: Option<&Formal>, kind, status| CallBinding {
        site: call.site,
        target: target.target,
        variant: variant.variant,
        actual,
        formal: formal.map(|f| f.formal.clone()),
        kind,
        status,
    };
    let mut out = Vec::new();
    let mut bound = vec![false; variant.formals.len()];
    let mut formals = variant.formals.iter().enumerate();
    let mut positional: Vec<usize> = Vec::new();
    if target.receiver == Receiver::Bound {
        match formals.next() {
            Some((i, f)) => {
                bound[i] = true;
                out.push(row(call.receiver, Some(f), BindingKind::Receiver, BindingStatus::Bound));
            }
            None => out.push(row(call.receiver, None, BindingKind::Receiver, BindingStatus::Unmapped)),
        }
    }
    for (i, f) in formals {
        if matches!(f.kind, ParameterKind::PositionalOnly | ParameterKind::PositionalOrKeyword) {
            positional.push(i);
        }
    }
    let varargs = variant.formals.iter().position(|f| f.kind == ParameterKind::VarPositional);
    let kwargs = variant.formals.iter().position(|f| f.kind == ParameterKind::VarKeyword);
    let mut next = 0usize;
    let mut starred = false;
    let mut double_starred = false;
    for actual in &call.actuals {
        let a = Some(actual.occurrence);
        match actual.kind {
            ArgumentKind::Positional | ArgumentKind::Implicit if starred => {
                out.push(row(a, None, BindingKind::Varargs, BindingStatus::Ambiguous));
            }
            ArgumentKind::Positional | ArgumentKind::Implicit => {
                let kind = if actual.kind == ArgumentKind::Implicit {
                    BindingKind::Implicit
                } else {
                    BindingKind::Positional
                };
                if let Some(&i) = positional.get(next) {
                    next += 1;
                    let status = if std::mem::replace(&mut bound[i], true) {
                        BindingStatus::Refused
                    } else {
                        BindingStatus::Bound
                    };
                    out.push(row(a, Some(&variant.formals[i]), kind, status));
                } else if let Some(v) = varargs {
                    bound[v] = true;
                    out.push(row(a, Some(&variant.formals[v]), BindingKind::Varargs, BindingStatus::Bound));
                } else {
                    out.push(row(a, None, kind, BindingStatus::Unmapped));
                }
            }
            ArgumentKind::Starred => {
                starred = true;
                out.push(row(a, None, BindingKind::Varargs, BindingStatus::Ambiguous));
            }
            ArgumentKind::Keyword => {
                let name = actual.keyword.as_deref().unwrap_or_default();
                let named = variant.formals.iter().position(|f| {
                    f.name == name
                        && matches!(f.kind, ParameterKind::PositionalOrKeyword | ParameterKind::KeywordOnly)
                });
                match (named, kwargs) {
                    (Some(i), _) => {
                        let status = if std::mem::replace(&mut bound[i], true) {
                            BindingStatus::Refused
                        } else {
                            BindingStatus::Bound
                        };
                        out.push(row(a, Some(&variant.formals[i]), BindingKind::Keyword, status));
                    }
                    (None, Some(k)) => {
                        bound[k] = true;
                        out.push(row(a, Some(&variant.formals[k]), BindingKind::Kwargs, BindingStatus::Bound));
                    }
                    (None, None) => out.push(row(a, None, BindingKind::Keyword, BindingStatus::Unmapped)),
                }
            }
            ArgumentKind::DoubleStarred => {
                double_starred = true;
                out.push(row(a, None, BindingKind::Kwargs, BindingStatus::Ambiguous));
            }
        }
    }
    for (i, f) in variant.formals.iter().enumerate() {
        if bound[i] || matches!(f.kind, ParameterKind::VarPositional | ParameterKind::VarKeyword) {
            continue;
        }
        let coverable = match f.kind {
            ParameterKind::PositionalOnly => starred,
            ParameterKind::PositionalOrKeyword => starred || double_starred,
            ParameterKind::KeywordOnly => double_starred,
            _ => false,
        };
        if coverable {
            out.push(row(None, Some(f), BindingKind::Default, BindingStatus::Ambiguous));
        } else if f.has_default {
            out.push(row(None, Some(f), BindingKind::Default, BindingStatus::Bound));
        } else {
            out.push(row(None, Some(f), BindingKind::Default, BindingStatus::Refused));
        }
    }
    out
}

/// Whether one variant accepts the call: every binding is bound.
pub fn accepts(bindings: &[CallBinding]) -> bool {
    bindings.iter().all(|b| b.status == BindingStatus::Bound)
}
