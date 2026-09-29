//! Known-answer suites for the model's vocabulary and named policies (cutover plan WP0.4), over
//! the shape library (WP0.9).

mod shapes;

use std::collections::BTreeSet;

use lctx_model::calls::{
    self, Actual, ArgumentKind, BindingKind, BindingStatus, CallBinding, CallOrigin, CallPhase,
    CallPolicy, CallSyntax, Modality, ParameterKind as P, Receiver, TargetAlternative, TargetFacts,
    TargetKind,
};
use lctx_model::condition::Diagram;
use lctx_model::decl::codebook::Codebook;
use lctx_model::id::Id;
use lctx_model::obligation::{
    self, Budget, ConditionState, Decisions, ObligationClass, ObligationKind as O, Standing,
    Verdict, VerdictInput,
};
use lctx_model::transfer::{
    self, CallComposition, CallSite, Composition, ProvenanceClass, Transfer, TransferKind as T,
    compose_call, compose_seq,
};
use lctx_model::vocab::{self, AccessPath, ItemKey, Join, ObservedPlace, OccurrenceSpan, PathRelation, Place, PlaceRoot, Region, Segment};
use shapes::*;

// ---------------------------------------------------------------------------------------------
// The binder

fn occ(n: u8) -> Id {
    Id([n; 16])
}

fn sig(formals: &[(&str, P, bool)]) -> calls::SignatureVariant {
    signature("f", formals)
}

fn bound_to<'a>(bindings: &'a [CallBinding], actual: Id) -> &'a CallBinding {
    bindings.iter().find(|b| b.actual == Some(actual)).expect("a binding for the actual")
}

fn formal_of(b: &CallBinding) -> Option<String> {
    b.formal.as_ref().map(|f| format!("{f:?}"))
}

#[test]
fn positional_and_keyword_actuals_bind_by_position_and_name() {
    let s = sig(&[("a", P::PositionalOrKeyword, false), ("b", P::PositionalOrKeyword, false)]);
    let by_position = calls::bind(&call(occ(1), vec![positional(occ(2)), positional(occ(3))]), function("f"), &s);
    assert!(calls::accepts(&by_position));
    assert_eq!(formal_of(bound_to(&by_position, occ(2))), Some(format!("{:?}", formal("f", "a"))));
    let by_name = calls::bind(&call(occ(1), vec![keyword(occ(3), "b"), keyword(occ(2), "a")]), function("f"), &s);
    assert!(calls::accepts(&by_name));
    assert_eq!(bound_to(&by_name, occ(3)).kind, BindingKind::Keyword);
    assert_eq!(formal_of(bound_to(&by_name, occ(3))), Some(format!("{:?}", formal("f", "b"))));
}

#[test]
fn an_omitted_default_binds_as_a_default_and_a_missing_required_is_refused() {
    let s = sig(&[("a", P::PositionalOrKeyword, false), ("b", P::PositionalOrKeyword, true)]);
    let one = calls::bind(&call(occ(1), vec![positional(occ(2))]), function("f"), &s);
    assert!(calls::accepts(&one));
    let default = one.iter().find(|b| b.kind == BindingKind::Default).unwrap();
    assert_eq!((default.actual, default.status), (None, BindingStatus::Bound));
    let none = calls::bind(&call(occ(1), vec![]), function("f"), &s);
    assert!(!calls::accepts(&none));
    assert!(none.iter().any(|b| b.status == BindingStatus::Refused && formal_of(b) == Some(format!("{:?}", formal("f", "a")))));
    // A formal bound twice is refused (a TypeError at runtime).
    let twice = calls::bind(&call(occ(1), vec![positional(occ(2)), keyword(occ(3), "a")]), function("f"), &s);
    assert_eq!(bound_to(&twice, occ(3)).status, BindingStatus::Refused);
}

#[test]
fn surplus_actuals_collect_into_variadics_or_stay_unmapped() {
    let collecting = sig(&[("a", P::PositionalOrKeyword, false), ("rest", P::VarPositional, false), ("kw", P::VarKeyword, false)]);
    let b = calls::bind(
        &call(occ(1), vec![positional(occ(2)), positional(occ(3)), positional(occ(4)), keyword(occ(5), "z")]),
        function("f"),
        &collecting,
    );
    assert!(calls::accepts(&b));
    assert_eq!(bound_to(&b, occ(3)).kind, BindingKind::Varargs);
    assert_eq!(bound_to(&b, occ(4)).kind, BindingKind::Varargs);
    assert_eq!(bound_to(&b, occ(5)).kind, BindingKind::Kwargs);
    let strict = sig(&[("a", P::PositionalOrKeyword, false)]);
    let b = calls::bind(&call(occ(1), vec![positional(occ(2)), positional(occ(3)), keyword(occ(4), "z")]), function("f"), &strict);
    assert_eq!(bound_to(&b, occ(3)).status, BindingStatus::Unmapped);
    assert_eq!(bound_to(&b, occ(4)).status, BindingStatus::Unmapped);
    assert_eq!(bound_to(&b, occ(3)).obligation(), Some(O::AmbiguousBinding));
}

#[test]
fn keyword_only_formals_take_only_keywords() {
    let s = sig(&[("k", P::KeywordOnly, false)]);
    assert!(calls::accepts(&calls::bind(&call(occ(1), vec![keyword(occ(2), "k")]), function("f"), &s)));
    let b = calls::bind(&call(occ(1), vec![positional(occ(2))]), function("f"), &s);
    assert_eq!(bound_to(&b, occ(2)).status, BindingStatus::Unmapped);
    assert!(b.iter().any(|x| x.status == BindingStatus::Refused));
}

#[test]
fn a_bound_receiver_takes_the_first_formal() {
    let s = sig(&[("self", P::PositionalOrKeyword, false), ("x", P::PositionalOrKeyword, false)]);
    let site = CallSyntax {
        site: occ(1),
        receiver: Some(occ(9)),
        actuals: vec![positional(occ(2))],
    };
    let b = calls::bind(&site, TargetAlternative { target: entity("f"), receiver: Receiver::Bound }, &s);
    assert!(calls::accepts(&b));
    let receiver = bound_to(&b, occ(9));
    assert_eq!(receiver.kind, BindingKind::Receiver);
    assert_eq!(formal_of(receiver), Some(format!("{:?}", formal("f", "self"))));
    assert_eq!(formal_of(bound_to(&b, occ(2))), Some(format!("{:?}", formal("f", "x"))));
}

#[test]
fn unpacking_is_ambiguous_never_an_error() {
    let s = sig(&[("a", P::PositionalOrKeyword, false), ("b", P::PositionalOrKeyword, false), ("k", P::KeywordOnly, false)]);
    let starred = Actual { occurrence: occ(2), kind: ArgumentKind::Starred, keyword: None };
    let b = calls::bind(&call(occ(1), vec![starred, positional(occ(3))]), function("f"), &s);
    assert_eq!(bound_to(&b, occ(2)).status, BindingStatus::Ambiguous);
    assert_eq!(bound_to(&b, occ(2)).obligation(), Some(O::UnsupportedUnpacking));
    // After `*xs`, a later positional's formal is unknown.
    assert_eq!(bound_to(&b, occ(3)).status, BindingStatus::Ambiguous);
    // `a` and `b` may be covered by the unpacking; keyword-only `k` cannot be and is refused.
    let ambiguous: BTreeSet<_> = b.iter().filter(|x| x.actual.is_none() && x.status == BindingStatus::Ambiguous).filter_map(formal_of).collect();
    assert_eq!(ambiguous.len(), 2);
    assert!(b.iter().any(|x| x.status == BindingStatus::Refused && formal_of(x) == Some(format!("{:?}", formal("f", "k")))));
    let double = Actual { occurrence: occ(4), kind: ArgumentKind::DoubleStarred, keyword: None };
    let b = calls::bind(&call(occ(1), vec![positional(occ(2)), double]), function("f"), &s);
    assert!(!b.iter().any(|x| x.status == BindingStatus::Refused), "**kw may cover b and k");
}

#[test]
fn an_implicit_invocation_binds_its_value_as_implicit() {
    let s = sig(&[("fn", P::PositionalOrKeyword, false)]);
    let decorated = Actual { occurrence: occ(2), kind: ArgumentKind::Implicit, keyword: None };
    let b = calls::bind(&call(occ(1), vec![decorated]), function("f"), &s);
    assert_eq!(bound_to(&b, occ(2)).kind, BindingKind::Implicit);
    assert!(calls::accepts(&b));
}

// ---------------------------------------------------------------------------------------------
// Call policies

fn every_fact() -> Vec<TargetFacts> {
    let mut out = Vec::new();
    for &modality in Modality::all() {
        for &origin in CallOrigin::all() {
            for &phase in CallPhase::all() {
                for &target_kind in TargetKind::all() {
                    for implicit in [false, true] {
                        for unique in [false, true] {
                            for complete in [false, true] {
                                out.push(TargetFacts { modality, origin, phase, target_kind, implicit, unique, complete });
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

#[test]
fn the_policy_admission_matrix_holds_its_invariants() {
    let facts = every_fact();
    for f in &facts {
        assert!(CallPolicy::Association.admits(f), "association admits every origin");
        if CallPolicy::Summary.admits(f) {
            assert!(CallPolicy::Dataflow.admits(f), "summary ⊆ dataflow: {f:?}");
            assert_eq!(f.modality, Modality::Definite);
            assert!(f.unique && f.complete);
        }
        if CallPolicy::Invocation.admits(f) {
            assert_ne!(f.phase, CallPhase::Definition, "definition arcs are kept separate");
            assert_eq!(f.origin, CallOrigin::AnalyzerAssertion);
        }
        if f.modality == Modality::Potential {
            assert!(!CallPolicy::Usage.admits(f) && !CallPolicy::Invocation.admits(f));
        }
    }
    let counts: Vec<(&str, usize)> = CallPolicy::ALL
        .iter()
        .map(|p| (p.name(), facts.iter().filter(|f| p.admits(f)).count()))
        .collect();
    let sql: Vec<(&str, String)> = CallPolicy::ALL.iter().map(|p| (p.name(), p.sql())).collect();
    insta::assert_debug_snapshot!((counts, sql));
}

#[test]
fn an_override_candidate_is_invoked_but_never_summarized() {
    // `self._inner(x)` in `Service.run`: a candidate target a subclass may override.
    let facts = TargetFacts {
        modality: Modality::Candidate,
        origin: CallOrigin::AnalyzerAssertion,
        phase: CallPhase::Call,
        target_kind: TargetKind::Method,
        implicit: false,
        unique: false,
        complete: false,
    };
    assert!(CallPolicy::Invocation.admits(&facts));
    assert!(!CallPolicy::Summary.admits(&facts));
}

// ---------------------------------------------------------------------------------------------
// Transfers and composition

#[test]
fn the_composition_table_is_explicit() {
    let table: Vec<String> = T::all()
        .iter()
        .flat_map(|a| T::all().iter().map(move |b| format!("{} · {} = {:?}", a.text(), b.text(), compose_seq(*a, *b))))
        .collect();
    insta::assert_debug_snapshot!(table);
    assert_eq!(compose_seq(T::Identity, T::Identity), Composition::Transfer(T::Identity));
    assert_eq!(compose_seq(T::Derived, T::Identity), Composition::Transfer(T::Derived));
    assert_eq!(compose_seq(T::Identity, T::Control), Composition::Selection);
}

fn caller(owner: &str, input: Place, actual: Id) -> Transfer {
    identity(owner, input, at(actual))
}

fn compose(caller: &Transfer, site: &CallSyntax, binding: &CallBinding, callee: &Transfer, formal_atoms: &[(Id, &Diagram)]) -> CallComposition {
    compose_call(caller, &CallSite { site: site.site, binding, formal_atoms }, callee)
}

fn composed(c: CallComposition) -> Transfer {
    match c {
        CallComposition::Transfer(t) => t,
        other => panic!("expected a transfer, got {other:?}"),
    }
}

/// `Config(timeout=t, title=n)`: each option reaches its own field; `title` never reaches `timeout`.
#[test]
fn config_options_reach_their_own_fields_only() {
    let shape = config();
    let target = TargetAlternative { target: shape.init.callable, receiver: Receiver::Bound };
    let bindings = calls::bind(&shape.site, target, &shape.init);
    assert!(calls::accepts(&bindings));
    let t_in = caller("main", formal_place("main", "t"), shape.t);
    let n_in = caller("main", formal_place("main", "n"), shape.n);
    let t_binding = bound_to(&bindings, shape.t);
    let n_binding = bound_to(&bindings, shape.n);
    let timeout_store = &shape.stores[0];
    let title_store = &shape.stores[1];
    let timeout = composed(compose(&t_in, &shape.site, t_binding, timeout_store, &[]));
    assert_eq!(timeout.input, formal_place("main", "t"));
    assert_eq!(timeout.output, field("Config", "timeout"));
    assert_eq!(timeout.kind, T::Identity);
    assert_eq!(timeout.provenance, ProvenanceClass::Composed);
    assert_eq!(timeout.context, Some(shape.site.site));
    let title = composed(compose(&n_in, &shape.site, n_binding, title_store, &[]));
    assert_eq!(title.output, field("Config", "title"));
    // Control: `title`'s transfer never composes with the `timeout` binding or store.
    assert!(matches!(compose(&n_in, &shape.site, t_binding, timeout_store, &[]), CallComposition::Obligation(_)));
    assert!(matches!(compose(&n_in, &shape.site, n_binding, timeout_store, &[]), CallComposition::Obligation(_)));
}

/// `read_timeout(c)`: the reader's field access path composes onto the caller's value.
#[test]
fn a_field_read_extends_the_callers_place() {
    let (signature, read) = read_timeout();
    let arg = occurrence(500, 501, "name");
    let site = call(occurrence(488, 502, "call"), vec![positional(arg)]);
    let bindings = calls::bind(&site, function("read_timeout"), &signature);
    let c = caller("main", formal_place("main", "c"), arg);
    let t = composed(compose(&c, &site, bound_to(&bindings, arg), &read, &[]));
    assert_eq!(t.input, formal_place("main", "c").attribute("timeout"));
    assert_eq!(t.input.path.encode(), ".timeout");
    assert_eq!(t.output, at(site.site));
    // A caller that delivers into `.title` of the actual does not reach a `.timeout` read.
    let into_title = identity("main", formal_place("main", "n"), at(arg).attribute("title"));
    assert!(matches!(compose(&into_title, &site, bound_to(&bindings, arg), &read, &[]), CallComposition::Disjoint));
}

/// `pair`: two calls to one callee never cross; only `second` reaches the return.
#[test]
fn two_calls_to_one_callee_never_cross() {
    let (signature, id_flow) = identity_fn();
    let shape = pair();
    let first = calls::bind(&shape.first_call, function("identity"), &signature);
    let second = calls::bind(&shape.second_call, function("identity"), &signature);
    let first_in = caller("pair", formal_place("pair", "first"), shape.first_arg);
    let second_in = caller("pair", formal_place("pair", "second"), shape.second_arg);
    let a = composed(compose(&first_in, &shape.first_call, bound_to(&first, shape.first_arg), &id_flow, &[]));
    let b = composed(compose(&second_in, &shape.second_call, bound_to(&second, shape.second_arg), &id_flow, &[]));
    assert_eq!(a.output, at(shape.first_call.site));
    assert_eq!(b.output, at(shape.second_call.site));
    // Matched by call site: `first` never composes through the second call.
    assert!(matches!(
        compose(&first_in, &shape.second_call, bound_to(&second, shape.second_arg), &id_flow, &[]),
        CallComposition::Obligation(O::AmbiguousBinding)
    ));
    // `return b`: only the second call's result reaches the return, unchanged.
    let returned = identity("pair", at(shape.second_call.site), ret("pair"));
    assert_eq!(compose_seq(b.kind, returned.kind), Composition::Transfer(T::Identity));
    assert_ne!(a.output, returned.input);
}

/// `fetch` → `select_timeout(timeout, fallback)`: each formal supplies the result under its own
/// branch, stated over the caller's instantiated atom (review F06).
#[test]
fn per_branch_supply_composes_through_a_summarized_callee() {
    let select = select_timeout();
    let shape = fetch();
    let bindings = calls::bind(&shape.select_call, function("select_timeout"), &select.signature);
    let instantiated = Diagram::from_atom(shape.instantiated_atom);
    let formal_atoms = [(select.entry_atom, &instantiated)];
    let fallback_in = caller("fetch", formal_place("fetch", "fallback"), shape.fallback_arg);
    let timeout_in = caller("fetch", formal_place("fetch", "timeout"), shape.timeout_arg);
    let fallback = composed(compose(&fallback_in, &shape.select_call, bound_to(&bindings, shape.fallback_arg), &select.flows[0], &formal_atoms));
    let timeout = composed(compose(&timeout_in, &shape.select_call, bound_to(&bindings, shape.timeout_arg), &select.flows[1], &formal_atoms));
    assert_eq!(fallback.condition.id(), instantiated.id(), "fallback when timeout is None");
    assert_eq!(timeout.condition.id(), instantiated.not().unwrap().id(), "timeout otherwise");
    assert_eq!(fallback.output, at(shape.select_call.site));
    // The two supplies are complementary: together they always supply the result.
    let merged = transfer::merge(vec![
        Transfer { input: formal_place("fetch", "x"), ..fallback.clone() },
        Transfer { input: formal_place("fetch", "x"), ..timeout.clone() },
    ])
    .unwrap();
    assert_eq!(merged.len(), 1);
    assert!(merged[0].condition.is_true());
    // `transport` is unresolved: the value crossing it is an obligation, nothing reaches fetch's
    // return, and the verdict names the reason.
    let open = transfer::open_call(TargetKind::Unresolved);
    assert_eq!(open, O::UnresolvedTarget);
    let (verdict, reason) = obligation::verdict(&VerdictInput {
        condition: ConditionState::Always,
        open: &[open],
        coverage_complete: true,
        approximated: false,
    });
    assert_eq!((verdict, reason), (Verdict::Unknown, Some(O::UnresolvedTarget)));
}

/// `facade(x) → _inner(x)`: identity through identity is identity, never "computed" (review F04).
#[test]
fn a_facade_forwards_its_argument_unchanged() {
    let shape = facade();
    let (signature, inner) = &shape.inner;
    let bindings = calls::bind(&shape.site, function("_inner"), signature);
    let x_in = caller("facade", formal_place("facade", "x"), shape.arg);
    let through = composed(compose(&x_in, &shape.site, bound_to(&bindings, shape.arg), inner, &[]));
    assert_eq!(through.kind, T::Identity);
    let returned = identity("facade", at(shape.site.site), ret("facade"));
    assert_eq!(compose_seq(through.kind, returned.kind), Composition::Transfer(T::Identity));
}

#[test]
fn callee_local_atoms_are_eliminated_and_bindings_gate_composition() {
    let (signature, id_flow) = identity_fn();
    let shape = pair();
    let bindings = calls::bind(&shape.first_call, function("identity"), &signature);
    let local = Diagram::from_atom(Id([42; 16]));
    let guarded = Transfer { condition: local, ..id_flow.clone() };
    let first_in = caller("pair", formal_place("pair", "first"), shape.first_arg);
    let t = composed(compose(&first_in, &shape.first_call, bound_to(&bindings, shape.first_arg), &guarded, &[]));
    assert!(t.condition.is_true(), "∃ local . local");
    let mut ambiguous = bound_to(&bindings, shape.first_arg).clone();
    ambiguous.status = BindingStatus::Ambiguous;
    ambiguous.kind = BindingKind::Varargs;
    assert!(matches!(
        compose(&first_in, &shape.first_call, &ambiguous, &id_flow, &[]),
        CallComposition::Obligation(O::UnsupportedUnpacking)
    ));
}

#[test]
fn transfer_identity_is_the_semantic_key() {
    let base = identity("f", formal_place("f", "a"), ret("f"));
    let mut other_provenance = base.clone();
    other_provenance.provenance = ProvenanceClass::AuthoredModel;
    assert_ne!(base.id(), other_provenance.id(), "a model and our summary stay distinct rows");
    let mut other_context = base.clone();
    other_context.context = Some(occ(3));
    assert_ne!(base.id(), other_context.id());
    assert_eq!(base.id(), base.clone().id());
}

// ---------------------------------------------------------------------------------------------
// Obligations, verdicts and discharge

#[test]
fn obligations_have_one_priority() {
    assert_eq!(obligation::first([O::MissingEvidence, O::UnresolvedTarget, O::ConditionWorkLimit]), Some(O::ConditionWorkLimit));
    assert_eq!(obligation::first([O::NotRequested, O::OverrideDispatch]), Some(O::OverrideDispatch));
    assert_eq!(O::CallTransfer.class(), ObligationClass::Resolution);
    assert_eq!(obligation::first([]), None);
    // Every legacy boundary code maps to the obligation of the same code, and nothing else does.
    for code in 0..=37 {
        assert_eq!(obligation::legacy::from_boundary_code(code).map(|o| o.code()), Some(code));
    }
    assert_eq!(obligation::legacy::from_boundary_code(38), None);
    assert_eq!(obligation::legacy::from_kernel_code("work_preflight"), Some(O::ConditionWorkLimit));
    assert_eq!(obligation::legacy::from_selection("Witness"), None);
    assert_eq!(obligation::legacy::from_selection("ResourceRefused"), Some(O::ResourceRefused));
}

#[test]
fn the_verdict_function_cases() {
    let v = |condition, open: &[O], coverage_complete, approximated| {
        obligation::verdict(&VerdictInput { condition, open, coverage_complete, approximated })
    };
    let cond = ConditionState::When(occ(1));
    assert_eq!(v(ConditionState::Always, &[], true, false), (Verdict::Established, None));
    assert_eq!(v(cond, &[], true, false), (Verdict::Conditional, None));
    assert_eq!(v(ConditionState::Never, &[], true, false), (Verdict::RefutedUnderModel, None));
    assert_eq!(v(ConditionState::Never, &[], false, false).0, Verdict::Unknown, "no refutation without coverage");
    assert_eq!(v(ConditionState::Never, &[], true, true).0, Verdict::Unknown, "no refutation under approximation");
    assert_eq!(v(ConditionState::Always, &[O::SummaryDepthLimit], true, false), (Verdict::Unknown, Some(O::SummaryDepthLimit)), "a cut proof is not a negative");
    assert_eq!(v(ConditionState::Always, &[O::NotRequested], true, false), (Verdict::NotAnalyzed, Some(O::NotRequested)));
    assert_eq!(v(ConditionState::Always, &[O::NotRequested, O::MissingEvidence], true, false).0, Verdict::Unknown);
}

#[test]
fn discharge_needs_every_member_proved() {
    let (a, b, c) = (occ(1), occ(2), occ(3));
    let mut d = Decisions::default();
    d.proof(a, occ(10), Verdict::Established);
    d.proof(b, occ(11), Verdict::Conditional);
    d.proof(b, occ(9), Verdict::Established);
    d.proof(c, occ(12), Verdict::Unknown);
    assert_eq!(d.decide(b), Standing::Proved { proof: occ(9) }, "the lowest proof id");
    assert_eq!(d.decide(c), Standing::Open { obligation: O::CallTransfer }, "an unknown proof proves nothing");
    let members: BTreeSet<Id> = [a, b].into();
    assert!(obligation::discharge(&members, &d).0);
    assert!(!obligation::discharge(&BTreeSet::new(), &d).0, "no members prove nothing");
    d.open(a, O::MissingEvidence);
    d.open(a, O::ConditionNodeLimit);
    assert_eq!(d.decide(a), Standing::Open { obligation: O::ConditionNodeLimit }, "an obligation overrides a proof");
    assert!(!obligation::discharge(&members, &d).0);
}

#[test]
fn named_budgets_refuse_with_their_own_obligation() {
    let mut meter = Budget::new(O::SummaryPairWorkLimit, 10).meter();
    assert_eq!(meter.charge(6), Ok(()));
    assert_eq!(meter.charge(4), Ok(()));
    assert_eq!(meter.charge(1), Err(O::SummaryPairWorkLimit));
    assert_eq!(meter.used(), 10);
    assert_eq!(obligation::from_kernel(lctx_model::condition::KernelBoundary::NodeLimit), O::ConditionNodeLimit);
}

// ---------------------------------------------------------------------------------------------
// Vocabulary

#[test]
fn access_paths_saturate_with_an_unknown_suffix() {
    let p = AccessPath::new()
        .then(Segment::Attribute("a".into()))
        .then(Segment::Item(ItemKey::Str("k".into())))
        .then(Segment::AnyItem);
    assert_eq!(p.segments().len(), 2);
    assert!(p.unknown_suffix());
    assert_eq!(p.encode(), ".a[\"k\"]…");
    assert_eq!(p.then(Segment::Attribute("z".into())), p, "a saturated path stays put");
    let a = AccessPath::new().then(Segment::Attribute("a".into()));
    let ab = a.then(Segment::Attribute("b".into()));
    assert_eq!(ab.strip_prefix(&a), PathRelation::Rest(AccessPath::new().then(Segment::Attribute("b".into()))));
    assert_eq!(a.strip_prefix(&ab), PathRelation::Shorter(AccessPath::new().then(Segment::Attribute("b".into()))));
    let c = AccessPath::new().then(Segment::Attribute("c".into()));
    assert_eq!(c.strip_prefix(&a), PathRelation::Disjoint);
    assert_eq!(AccessPath::unknown().strip_prefix(&a), PathRelation::Unknown);
    let any = AccessPath::new().then(Segment::AnyItem);
    let key = AccessPath::new().then(Segment::Item(ItemKey::Int(0)));
    assert_eq!(any.strip_prefix(&key), PathRelation::Unknown);
}

#[test]
fn places_are_identified_by_root_and_path() {
    let f = formal_place("f", "a");
    assert_ne!(f.id(), f.attribute("x").id());
    assert_ne!(f.id(), ret("f").id());
    assert_ne!(field("C", "x").id(), Place::root(PlaceRoot::Global { module: entity("C"), name: "x".into() }).id());
    assert_eq!(f.attribute("x").id(), f.attribute("x").id());
    let snapshot: Vec<String> = [f.clone(), f.attribute("x"), ret("f"), field("Config", "timeout")].iter().map(|p| p.id().hex()).collect();
    insta::assert_debug_snapshot!(snapshot);
}

#[test]
fn the_owner_rule_picks_the_innermost_declaration() {
    let module = entity("m");
    let (outer, inner) = (entity("m.Outer"), entity("m.Outer.method"));
    let regions = [
        Region { entity: outer, module, start: 0, end: 100 },
        Region { entity: inner, module, start: 20, end: 60 },
    ];
    assert_eq!(vocab::owner_of(module, 30, 40, &regions), inner);
    assert_eq!(vocab::owner_of(module, 70, 80, &regions), outer);
    assert_eq!(vocab::owner_of(module, 150, 160, &regions), module, "module-level callers are owned by the module");
    assert_eq!(vocab::owner_of(entity("other"), 30, 40, &regions), entity("other"));
}

#[test]
fn the_region_join_treats_names_attributes_and_subscripts_alike() {
    let module = entity("m");
    let occs = [
        OccurrenceSpan { occurrence: occ(1), module, start: 10, end: 20, syntax_kind: "attribute" },
        OccurrenceSpan { occurrence: occ(2), module, start: 10, end: 14, syntax_kind: "name" },
        OccurrenceSpan { occurrence: occ(3), module, start: 30, end: 40, syntax_kind: "subscript" },
    ];
    assert_eq!(vocab::innermost_region_join(module, 10, 20, ObservedPlace::Attribute, &occs), Join::Exact(occ(1)));
    assert_eq!(vocab::innermost_region_join(module, 10, 14, ObservedPlace::Name, &occs), Join::Exact(occ(2)));
    assert_eq!(vocab::innermost_region_join(module, 11, 13, ObservedPlace::Name, &occs), Join::Innermost(occ(2)));
    assert_eq!(vocab::innermost_region_join(module, 31, 35, ObservedPlace::Subscript, &occs), Join::Innermost(occ(3)));
    assert_eq!(vocab::innermost_region_join(module, 31, 35, ObservedPlace::Name, &occs), Join::Unmatched);
}
