//! The known-answer shape library (cutover plan WP0.9).
//!
//! The review's P0 shapes (`fixtures/python/semantic_shapes/p0shapes`) as model data: each
//! callable's signature and the transfers a summary producer would state for it, and each call
//! site's syntax. Expected answers are stated beside the shapes in the tests that use them. Ids are
//! stable: every entity and occurrence id is a v2 recipe over the shape's own names.

#![allow(dead_code)]

use lctx_model::calls::{
    Actual, ArgumentKind, CallSyntax, Formal, ParameterKind, Receiver, SignatureVariant,
    TargetAlternative,
};
use lctx_model::condition::{Atom, Diagram, Predicate};
use lctx_model::id::{Id, recipes};
use lctx_model::transfer::{ProvenanceClass, Transfer, TransferKind};
use lctx_model::vocab::{FormalRef, Place, PlaceRoot};

/// A stable entity id for a qualified name in the fixture.
pub fn entity(qualified: &str) -> Id {
    recipes::symbol_key("p0shapes", qualified, "")
}

/// A stable occurrence id: the fixture module, a span and a syntax kind.
pub fn occurrence(start: i64, end: i64, syntax: &str) -> Id {
    recipes::occurrence(entity("p0shapes"), start, end, syntax)
}

pub fn formal(callable: &str, name: &str) -> FormalRef {
    FormalRef::Parameter(entity(&format!("{callable}.{name}")))
}

pub fn formal_place(callable: &str, name: &str) -> Place {
    Place::root(PlaceRoot::Formal(formal(callable, name)))
}

pub fn ret(callable: &str) -> Place {
    Place::root(PlaceRoot::Return {
        callable: entity(callable),
    })
}

pub fn at(occurrence: Id) -> Place {
    Place::root(PlaceRoot::Occurrence(occurrence))
}

pub fn field(class: &str, name: &str) -> Place {
    Place::root(PlaceRoot::Field {
        class: entity(class),
        name: name.to_owned(),
    })
}

pub fn formal_decl(callable: &str, name: &str, kind: ParameterKind, has_default: bool) -> Formal {
    Formal {
        formal: formal(callable, name),
        name: name.to_owned(),
        kind,
        has_default,
    }
}

pub fn signature(callable: &str, formals: &[(&str, ParameterKind, bool)]) -> SignatureVariant {
    SignatureVariant {
        callable: entity(callable),
        variant: 0,
        formals: formals
            .iter()
            .map(|(name, kind, default)| formal_decl(callable, name, *kind, *default))
            .collect(),
    }
}

pub fn transfer(owner: &str, input: Place, output: Place, kind: TransferKind, condition: Diagram) -> Transfer {
    Transfer {
        owner: entity(owner),
        input,
        output,
        kind,
        condition,
        context: None,
        provenance: ProvenanceClass::DerivedSummary,
    }
}

pub fn identity(owner: &str, input: Place, output: Place) -> Transfer {
    transfer(owner, input, output, TransferKind::Identity, Diagram::always())
}

pub fn positional(occurrence: Id) -> Actual {
    Actual {
        occurrence,
        kind: ArgumentKind::Positional,
        keyword: None,
    }
}

pub fn keyword(occurrence: Id, name: &str) -> Actual {
    Actual {
        occurrence,
        kind: ArgumentKind::Keyword,
        keyword: Some(name.to_owned()),
    }
}

pub fn call(site: Id, actuals: Vec<Actual>) -> CallSyntax {
    CallSyntax {
        site,
        receiver: None,
        actuals,
    }
}

pub fn function(target: &str) -> TargetAlternative {
    TargetAlternative {
        target: entity(target),
        receiver: Receiver::None,
    }
}

use ParameterKind::PositionalOrKeyword as Pk;

/// `class Config: def __init__(self, timeout=30.0, title="x")`, storing each into its field.
pub struct ConfigShape {
    pub init: SignatureVariant,
    pub stores: Vec<Transfer>,
    /// `Config(timeout=t, title=n)` in some caller.
    pub site: CallSyntax,
    pub t: Id,
    pub n: Id,
}

pub fn config() -> ConfigShape {
    let init = signature("Config.__init__", &[("self", Pk, false), ("timeout", Pk, true), ("title", Pk, true)]);
    let stores = vec![
        identity("Config.__init__", formal_place("Config.__init__", "timeout"), field("Config", "timeout")),
        identity("Config.__init__", formal_place("Config.__init__", "title"), field("Config", "title")),
    ];
    let (t, n) = (occurrence(100, 101, "name"), occurrence(109, 110, "name"));
    let site = CallSyntax {
        site: occurrence(90, 111, "call"),
        receiver: Some(occurrence(90, 96, "name")),
        actuals: vec![keyword(t, "timeout"), keyword(n, "title")],
    };
    ConfigShape { init, stores, site, t, n }
}

/// `def read_timeout(config): return config.timeout`.
pub fn read_timeout() -> (SignatureVariant, Transfer) {
    (
        signature("read_timeout", &[("config", Pk, false)]),
        identity("read_timeout", formal_place("read_timeout", "config").attribute("timeout"), ret("read_timeout")),
    )
}

/// `def identity(value): return value`.
pub fn identity_fn() -> (SignatureVariant, Transfer) {
    (
        signature("identity", &[("value", Pk, false)]),
        identity("identity", formal_place("identity", "value"), ret("identity")),
    )
}

/// `def pair(first, second): a = identity(first); b = identity(second); return b`.
pub struct PairShape {
    pub first_call: CallSyntax,
    pub second_call: CallSyntax,
    pub first_arg: Id,
    pub second_arg: Id,
}

pub fn pair() -> PairShape {
    let (first_arg, second_arg) = (occurrence(40, 45, "name"), occurrence(70, 76, "name"));
    PairShape {
        first_call: call(occurrence(31, 46, "call"), vec![positional(first_arg)]),
        second_call: call(occurrence(61, 77, "call"), vec![positional(second_arg)]),
        first_arg,
        second_arg,
    }
}

/// `def select_timeout(timeout, fallback): if timeout is None: return fallback; return timeout`.
pub struct SelectShape {
    pub signature: SignatureVariant,
    /// The callee's entry atom: `timeout is None`, evaluated at the guard.
    pub entry_atom: Id,
    pub flows: Vec<Transfer>,
}

pub fn select_timeout() -> SelectShape {
    let entry = Atom {
        evaluation: occurrence(210, 225, "compare"),
        operand: Some(formal_place("select_timeout", "timeout").id()),
        predicate: Predicate::IsNone,
    }
    .id();
    let guard = Diagram::from_atom(entry);
    SelectShape {
        signature: signature("select_timeout", &[("timeout", Pk, false), ("fallback", Pk, false)]),
        entry_atom: entry,
        flows: vec![
            transfer(
                "select_timeout",
                formal_place("select_timeout", "fallback"),
                ret("select_timeout"),
                TransferKind::Identity,
                guard.clone(),
            ),
            transfer(
                "select_timeout",
                formal_place("select_timeout", "timeout"),
                ret("select_timeout"),
                TransferKind::Identity,
                guard.not().unwrap(),
            ),
        ],
    }
}

/// `def fetch(url, timeout=None, fallback=30): selected = select_timeout(timeout, fallback);
/// return transport(url, timeout=selected)` — `transport` is unresolved.
pub struct FetchShape {
    pub select_call: CallSyntax,
    pub timeout_arg: Id,
    pub fallback_arg: Id,
    /// The caller-side atom `timeout is None` about fetch's own formal, instantiated at the call.
    pub instantiated_atom: Id,
}

pub fn fetch() -> FetchShape {
    let (timeout_arg, fallback_arg) = (occurrence(320, 327, "name"), occurrence(329, 337, "name"));
    let select_call = call(occurrence(305, 338, "call"), vec![positional(timeout_arg), positional(fallback_arg)]);
    let instantiated_atom = Atom {
        evaluation: select_call.site,
        operand: Some(formal_place("fetch", "timeout").id()),
        predicate: Predicate::IsNone,
    }
    .id();
    FetchShape {
        select_call,
        timeout_arg,
        fallback_arg,
        instantiated_atom,
    }
}

/// `def facade(x): return _inner(x)` and `def _inner(y): return y`.
pub struct FacadeShape {
    pub inner: (SignatureVariant, Transfer),
    pub site: CallSyntax,
    pub arg: Id,
}

pub fn facade() -> FacadeShape {
    let arg = occurrence(420, 421, "name");
    FacadeShape {
        inner: (
            signature("_inner", &[("y", Pk, false)]),
            identity("_inner", formal_place("_inner", "y"), ret("_inner")),
        ),
        site: call(occurrence(413, 422, "call"), vec![positional(arg)]),
        arg,
    }
}
