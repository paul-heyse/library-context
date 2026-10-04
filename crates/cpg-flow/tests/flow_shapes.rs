//! The flow provider's known answers on `fixtures/python/flow_shapes` (ADR-0022 §The flow
//! provider; the plan's Stage 2.2): reaching definitions and their conditions, value sources,
//! statement regions, and the runtime view.

use std::path::Path;
use std::sync::OnceLock;

use cpg_flow::native::{Atom, Value};
use cpg_flow::{
    BindingKind, Condition, Input, ModuleFlow, RuntimeBindings, RuntimeContext, Sink, Span,
};
use lctx_model::domain::{attribution::*, conditions::*, input::*, source::*, *};

// A test-only presentation over the model's canonical kernel. No native provider encoding or
// alternate truth representation is used. Each native evaluation remains a distinct atom.
fn atom_label(atom: &Atom) -> String {
    let value = |v: &Value| match v {
        Value::None => "None".into(),
        Value::Bool(v) => {
            if *v {
                "True".into()
            } else {
                "False".into()
            }
        }
        Value::Int(v) => v.to_string(),
        Value::Str(v) => format!("{v:?}"),
    };
    match atom.predicate() {
        Atom::IsNone { place } => format!("is_none({place})"),
        Atom::Truthy { place } => format!("truthy({place})"),
        Atom::IsValue { place, value: v } => format!("is_value({place},{})", value(v)),
        Atom::Equals { place, value: v } => format!("equals({place},{})", value(v)),
        Atom::MemberOf { place, values } => format!(
            "member_of({place},{{{}}})",
            values.iter().map(value).collect::<Vec<_>>().join(",")
        ),
        Atom::IsInstance { place, class } => format!("isinstance({place},{class})"),
        Atom::TypeIs { place, class } => format!("type_is({place},{class})"),
        Atom::NonTerminalCall { awaiting } => format!("nonterminal(await={awaiting})"),
        Atom::NonEmptyIterable => "nonempty_iterable".into(),
        Atom::ContextManagerSuppresses { asynchronous } => {
            format!("suppresses(async={asynchronous})")
        }
        Atom::FinallyNormalPathImpossible => "finally_normal_impossible".into(),
        Atom::Opaque { text } => format!("opaque({text:?})"),
        Atom::Evaluated { .. } => unreachable!(),
    }
}
fn lower(
    c: &Condition,
) -> Result<
    (
        Diagram,
        std::collections::BTreeMap<Id<EvaluationAtom>, String>,
    ),
    KernelBoundary,
> {
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "test.py".into(), b"").unwrap();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"flow-test"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let mut names = std::collections::BTreeMap::new();
    if c.leaves().any(|a| a.site().is_none()) {
        return Err(KernelBoundary::TransferUnsupported);
    }
    let graph = c.map(|a| {
        let span = a.site().unwrap();
        let label = atom_label(a);
        let occurrence = Occurrence {
            source: source.id(),
            start: span.start as i64,
            end: span.end as i64,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Predicate,
            structural_path: vec![],
        };
        let atom = EvaluationAtom {
            evaluation: occurrence.id(),
            context: context.id(),
            predicate: lctx_model::domain::value::Predicate::Opaque {
                text: label.clone(),
            }
            .id(),
            operand: None,
        };
        names.insert(atom.id(), label);
        atom.id()
    });
    Ok((Diagram::from_graph(&graph)?.diagram, names))
}
fn labels(c: &Condition) -> String {
    let Ok((diagram, names)) = lower(c) else {
        return "unavailable".into();
    };
    if diagram.is_true() {
        return "true".into();
    }
    if diagram.is_false() {
        return "false".into();
    }
    let mut terms: Vec<_> = diagram
        .render_terms(256)
        .unwrap()
        .terms
        .iter()
        .map(|term| {
            let mut labels: Vec<_> = term
                .iter()
                .map(|(a, positive)| format!("{}{}", if *positive { "" } else { "!" }, names[a]))
                .collect();
            labels.sort();
            labels.dedup();
            labels.join(" & ")
        })
        .collect();
    terms.sort();
    terms.dedup();
    terms.join(" | ")
}

fn text() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/python/flow_shapes/release/flowpkg/shapes.py");
        std::fs::read_to_string(path).unwrap()
    })
}

fn flow() -> &'static ModuleFlow {
    static FLOW: OnceLock<ModuleFlow> = OnceLock::new();
    FLOW.get_or_init(|| {
        let mut runtime = RuntimeBindings::default();
        for (at, _) in text().match_indices("if TYPE_CHECKING:") {
            if at > text().find("def choose(TYPE_CHECKING)").unwrap() {
                continue;
            }
            let start = (at + 3) as u32;
            runtime.checking_names.insert(Span {
                start,
                end: start + 13,
            });
        }
        for (at, _) in text().match_indices("sys.") {
            let start = at as u32;
            runtime.sys_modules.insert(Span {
                start,
                end: start + 3,
            });
        }
        for (at, _) in text().match_indices("os.") {
            let start = at as u32;
            runtime.os_modules.insert(Span {
                start,
                end: start + 2,
            });
        }
        for (at, _) in text().match_indices("if TC:") {
            let start = (at + 3) as u32;
            runtime.checking_names.insert(Span {
                start,
                end: start + 2,
            });
        }
        let at = text().find("x if TYPE_CHECKING else y").unwrap() + 5;
        runtime.checking_names.insert(Span {
            start: at as u32,
            end: at as u32 + 13,
        });
        for (at, _) in text().match_indices("typing_alias.TYPE_CHECKING") {
            let start = at as u32;
            runtime.typing_modules.insert(Span {
                start,
                end: start + 12,
            });
        }
        let mut out = cpg_flow::index(
            &[Input {
                path: "flowpkg/shapes.py".to_owned(),
                text: text().to_owned(),
                runtime,
            }],
            &RuntimeContext {
                python_version: (3, 14, 7),
                platform: "linux".to_owned(),
            },
        );
        let m = out.pop().unwrap();
        assert_eq!(m.error, None);
        m
    })
}

#[test]
fn test_leaves_keep_compound_operands_and_distinct_match_arms() {
    let leaves = &flow().test_leaves;
    let compound: Vec<_> = leaves
        .iter()
        .filter(|leaf| {
            line(leaf.test_span.start) == line_of("if stateless is not None", "def alias_fallback")
        })
        .collect();
    assert!(
        compound
            .iter()
            .any(|leaf| leaf.atom.place() == Some("stateless"))
    );
    assert!(
        compound
            .iter()
            .any(|leaf| leaf.atom.place() == Some("stateless_http"))
    );
    for leaf in &compound {
        let atom = &leaf.atom;
        if let Some(place) = atom.place() {
            let operand = leaf.operand_span.expect("modeled place has one operand");
            assert_eq!(slice(operand.start, operand.end), place);
            assert!(
                flow()
                    .uses
                    .iter()
                    .any(|u| { u.span == operand && u.place == place && !u.annotation })
            );
        }
    }

    let instance = leaves
        .iter()
        .find(|leaf| {
            line(leaf.test_span.start)
                == line_of("if isinstance(x, C):", "def rebound_isinstance_class")
                && matches!(leaf.atom.predicate(), Atom::IsInstance { .. })
        })
        .expect("isinstance test leaf");
    let operand = instance.operand_span.expect("first argument is tested");
    assert_eq!(slice(operand.start, operand.end), "x");
    assert!(slice(instance.test_span.start, instance.test_span.end).contains("C"));

    let matched: Vec<_> = leaves
        .iter()
        .filter(|leaf| {
            (line_of("case ", "def matched")..line_of("return state", "def matched"))
                .contains(&line(leaf.test_span.start))
        })
        .collect();
    let sites: std::collections::BTreeSet<_> = matched.iter().map(|leaf| leaf.leaf_span).collect();
    assert!(sites.len() >= 2, "match arms collapsed: {matched:?}");
    assert!(matched.iter().all(|leaf| leaf.operand_span.is_none()));
}

#[test]
fn flow_producer_keeps_a_wide_predicate_as_a_structural_graph() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/python/flow_shapes/release/flowpkg/wide.py"),
    )
    .unwrap();
    let flow = cpg_flow::index(
        &[Input {
            path: "flowpkg/wide.py".to_owned(),
            text: text.clone(),
            runtime: RuntimeBindings::default(),
        }],
        &RuntimeContext {
            python_version: (3, 14, 7),
            platform: "linux".to_owned(),
        },
    )
    .pop()
    .unwrap();
    assert!(flow.error.is_none(), "{:?}", flow.error);
    let at = text.find("return \"reachable\"").unwrap() as u32;
    let region = flow.regions.iter().find(|r| r.span.start == at).unwrap();
    let (diagram, _) = lower(&region.condition).unwrap();
    assert!(!diagram.is_false());
    assert!(diagram.render_terms(16).unwrap().truncated);
}

fn line(byte: u32) -> usize {
    1 + text().as_bytes()[..byte as usize]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
}

/// The 1-based line of the first line whose text contains `needle`, after `after`.
fn line_of(needle: &str, after: &str) -> usize {
    let start = text().find(after).unwrap_or_else(|| panic!("no {after:?}"));
    let at = start
        + text()[start..]
            .find(needle)
            .unwrap_or_else(|| panic!("no {needle:?}"));
    line(at as u32)
}

fn slice(start: u32, end: u32) -> &'static str {
    &text()[start as usize..end as usize]
}

/// The use of `place` on the line holding `needle` (searched after `after`).
fn use_ix(place: &str, needle: &str, after: &str) -> u32 {
    let l = line_of(needle, after);
    let f = flow();
    f.uses
        .iter()
        .position(|u| u.place == place && line(u.span.start) == l)
        .unwrap_or_else(|| panic!("no use of {place} on line {l}")) as u32
}

/// The definitions reaching a use: `(value or target text, kind, condition, loop carried)`.
fn reaching(u: u32) -> Vec<(String, Option<BindingKind>, String, bool)> {
    let f = flow();
    let mut out: Vec<_> = f
        .reaching
        .iter()
        .filter(|r| r.use_ix == u)
        .map(|r| match r.def_ix {
            Some(d) => {
                let d = &f.defs[d as usize];
                let shown = d
                    .value
                    .map(|v| slice(v.start, v.end))
                    .unwrap_or_else(|| slice(d.target.start, d.target.end));
                (
                    shown.to_owned(),
                    Some(d.kind),
                    labels(&r.condition),
                    r.loop_carried,
                )
            }
            None => ("<unbound>".to_owned(), None, labels(&r.condition), false),
        })
        .collect();
    out.sort_by(|a, b| {
        (&a.0, a.1.map(|k| k as i16), &a.2, a.3).cmp(&(&b.0, b.1.map(|k| k as i16), &b.2, b.3))
    });
    out
}

/// The region condition of the statement starting with `needle` (searched after `after`).
fn region(needle: &str, after: &str) -> String {
    let start = text().find(after).unwrap();
    let at = (start + text()[start..].find(needle).unwrap()) as u32;
    let f = flow();
    let condition = &f
        .regions
        .iter()
        .find(|r| r.span.start == at)
        .unwrap_or_else(|| panic!("no region at {needle:?}"))
        .condition;
    labels(condition)
}

#[test]
fn a_fallback_rebinding_carries_the_value_under_its_test() {
    let u = use_ix("timeout", "return timeout", "def fallback");
    let r = reaching(u);
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0].0, "timeout if timeout is not None else 30");
    assert_eq!(r[0].2, "true");
    // The rebinding's value: the parameter passes unchanged when it is not None.
    let f = flow();
    let def = f
        .defs
        .iter()
        .find(|d| {
            d.value
                .is_some_and(|v| slice(v.start, v.end).starts_with("timeout if"))
        })
        .unwrap();
    let v = def.value.unwrap();
    let sources: Vec<(String, bool, String)> = f
        .values
        .iter()
        .filter(|s| s.sink == Sink::Definition && s.span == v)
        .map(|s| {
            let u = &f.uses[s.use_ix as usize];
            (
                slice(u.span.start, u.span.end).to_owned(),
                s.identity,
                labels(&s.condition),
            )
        })
        .collect();
    assert!(
        sources.contains(&("timeout".to_owned(), true, "!is_none(timeout)".to_owned())),
        "{sources:?}"
    );
    assert!(
        sources.contains(&("timeout".to_owned(), false, "true".to_owned())),
        "the test reads it too: {sources:?}"
    );
}

#[test]
fn a_settings_fallback_reaches_under_both_polarities() {
    let u = use_ix("host", "return host", "def settings_fallback");
    let r = reaching(u);
    assert_eq!(
        r,
        vec![
            (
                "host".to_owned(),
                Some(BindingKind::Parameter),
                "!is_none(host)".to_owned(),
                false
            ),
            (
                "settings.host".to_owned(),
                Some(BindingKind::Assignment),
                "is_none(host)".to_owned(),
                false
            ),
        ]
    );
}

#[test]
fn a_guard_states_the_raise_and_the_return_regions() {
    assert_eq!(region("raise ValueError", "def guarded"), "!truthy(path)");
    assert_eq!(
        region("return path.upper()", "def guarded"),
        "truthy(path) & truthy(strict)"
    );
    assert_eq!(
        region("return path\n", "def guarded"),
        "!truthy(strict) & truthy(path)"
    );
}

#[test]
fn a_boolean_operand_is_an_identity_source_under_its_truth() {
    let f = flow();
    let at = text().find("return a or b").unwrap() as u32 + 7;
    let sources: Vec<(String, bool, String)> = f
        .values
        .iter()
        .filter(|s| s.sink == Sink::Return && s.span.start == at)
        .map(|s| {
            let u = &f.uses[s.use_ix as usize];
            (u.place.clone(), s.identity, labels(&s.condition))
        })
        .collect();
    assert!(sources.contains(&("a".to_owned(), true, "truthy(a)".to_owned())));
    assert!(sources.contains(&("b".to_owned(), true, "!truthy(a)".to_owned())));
}

#[test]
fn a_nested_fallback_keeps_its_test_and_a_call_is_a_transfer() {
    let f = flow();
    let sources = |sink: Sink, needle: &str| -> Vec<(String, bool, bool, String)> {
        let at = text().find(needle).unwrap() as u32;
        f.values
            .iter()
            .filter(|s| s.sink == sink && s.span.start == at)
            .map(|s| {
                let u = &f.uses[s.use_ix as usize];
                (
                    u.place.clone(),
                    s.identity,
                    s.through_call,
                    labels(&s.condition),
                )
            })
            .collect()
    };
    // The fallback sits in the receiver of `.lower()`: each branch keeps its test, and both
    // reach `chosen` only through the call.
    let chosen = sources(Sink::Definition, "(level if level");
    assert!(
        chosen.contains(&(
            "settings.level".to_owned(),
            false,
            true,
            "is_none(level)".to_owned()
        )),
        "{chosen:?}"
    );
    assert!(
        chosen.contains(&(
            "level".to_owned(),
            false,
            true,
            "!is_none(level)".to_owned()
        )),
        "{chosen:?}"
    );
    // An f-string is computed from its operand directly.
    assert_eq!(
        sources(Sink::Definition, "f\"[{name}]\""),
        vec![("name".to_owned(), false, false, "true".to_owned())]
    );
    // A returned call's argument reaches the result only through the callee.
    let returned = sources(Sink::Return, "fallback(name), chosen");
    assert!(
        returned.contains(&("name".to_owned(), false, true, "true".to_owned())),
        "{returned:?}"
    );
    assert!(
        returned.contains(&("chosen".to_owned(), false, false, "true".to_owned())),
        "{returned:?}"
    );
    // The argument itself passes unchanged into the callee's formal.
    assert_eq!(
        sources(Sink::Argument, "name), chosen"),
        vec![("name".to_owned(), true, false, "true".to_owned())]
    );
}

#[test]
fn a_test_after_a_rebinding_has_a_distinct_evaluation_site() {
    // The second test may read the parameter or the alias binding. Its site differs from the
    // first test, so the two tests cannot be collapsed into one Boolean variable.
    let u = use_ix(
        "stateless_http",
        "return stateless_http",
        "def alias_fallback",
    );
    let r = reaching(u);
    assert_eq!(r.len(), 3, "{r:?}");
    assert!(r.iter().all(|row| row.2 != "false"), "{r:?}");
    let f = flow();
    let atom_ids: Vec<_> = f
        .tests
        .iter()
        .filter(|t| slice(t.span.start, t.span.end).contains("stateless_http is None"))
        .map(|t| t.condition.clone())
        .collect();
    assert_eq!(atom_ids.len(), 2, "{atom_ids:?}");
    assert_ne!(atom_ids[0], atom_ids[1]);
    assert!(
        atom_ids
            .iter()
            .all(|graph| graph.leaves().all(|atom| atom.site().is_some())),
        "{atom_ids:?}"
    );
    assert_eq!(
        region("stateless_http = stateless", "def alias_fallback"),
        "!is_none(stateless) & is_none(stateless_http)"
    );
}

#[test]
fn distinct_evaluations_admit_the_external_reviews_feasible_paths() {
    for (function, statement) in [
        ("def two_calls", "return \"second call changed\""),
        ("def mutated", "return \"mutated\""),
        ("def cleared", "return \"cleared\""),
        ("def eq_vs_is", "return \"equal, not identical\""),
        ("def eq_none", "return \"equal, not identical\""),
    ] {
        let at = text().find(function).unwrap();
        let statement_at = at + text()[at..].find(statement).unwrap();
        let condition = &flow()
            .regions
            .iter()
            .find(|r| r.span.start == statement_at as u32)
            .unwrap_or_else(|| panic!("no region for {function}"))
            .condition;
        assert!(!condition.is_never(), "{function}: {}", labels(condition));
    }
    let u = use_ix("found", "return found", "def two_ranges");
    let reached: Vec<_> = reaching(u).into_iter().map(|r| r.0).collect();
    assert!(reached.iter().any(|r| r == "i"), "{reached:?}");
    assert!(reached.iter().any(|r| r == "j"), "{reached:?}");
}

#[test]
fn same_line_and_merged_bindings_keep_distinct_site_identity() {
    for function in ["def same_line_bindings", "def merged_definitions"] {
        let at = text().find(function).unwrap();
        let end = text()[at..].find("\n\n").map_or(text().len(), |n| at + n);
        let conditions: Vec<_> = flow()
            .tests
            .iter()
            .filter(|t| {
                (at as u32..end as u32).contains(&t.span.start)
                    && slice(t.span.start, t.span.end) == "x is None"
            })
            .map(|t| t.condition.clone())
            .collect();
        if function == "def same_line_bindings" {
            assert_eq!(conditions.len(), 2, "{function}: {conditions:?}");
            assert_ne!(conditions[0], conditions[1], "{function}");
        } else {
            assert_eq!(conditions.len(), 1, "{function}: {conditions:?}");
        }
        assert!(
            conditions
                .iter()
                .all(|c| c.leaves().all(|a| a.site().is_some())),
            "{function}: {conditions:?}"
        );
    }
}

#[test]
fn the_runtime_view_requires_a_resolved_binding_and_compares_full_tuples() {
    assert_eq!(
        region("return \"parameter\"", "def choose"),
        "truthy(TYPE_CHECKING)"
    );
    assert_eq!(
        region("return \"checker only\"", "def checking_alias"),
        "false"
    );
    assert_eq!(
        region("return \"checker only\"", "def checking_module_alias"),
        "false"
    );
    assert_eq!(
        region("return \"ordinary attribute\"", "def config_check"),
        "truthy(config.TYPE_CHECKING)"
    );
    for (statement, expected) in [
        ("above = True", "true"),
        ("at_least = True", "true"),
        ("at_most = True", "false"),
        ("equal = True", "false"),
    ] {
        assert_eq!(
            region(statement, "def version_prefix"),
            expected,
            "{statement}"
        );
    }
    for (statement, expected) in [
        ("above_micro = True", "true"),
        ("at_most_micro = True", "false"),
        ("equal_micro = True", "false"),
    ] {
        assert_eq!(
            region(statement, "def version_micro_prefix"),
            expected,
            "{statement}"
        );
    }
}

#[test]
fn rebinding_an_isinstance_class_and_distinct_literal_objects_keep_paths_open() {
    assert_ne!(
        region("return \"rebound class\"", "def rebound_isinstance_class"),
        "false"
    );
    assert_ne!(
        region("return \"changed nonlocal\"", "def nonlocal_change"),
        "false"
    );
    let at = text().find("def non_singleton_identity").unwrap() as u32;
    let tests: Vec<_> = flow()
        .tests
        .iter()
        .filter(|t| t.span.start > at && labels(&t.condition).contains("is_value(x,1000)"))
        .map(|t| t.condition.clone())
        .collect();
    assert_eq!(tests.len(), 2, "{tests:?}");
    assert_ne!(tests[0], tests[1]);
}

#[test]
fn retained_false_candidates_are_not_counted_as_discarded_branches() {
    let skips = &flow().skips;
    assert!(skips.reaching_runtime_view > 0, "{skips:?}");
    assert_eq!(
        skips.reaching_ty_false, 0,
        "exact false-only candidates are retained: {skips:?}"
    );
    assert_eq!(skips.reaching_stable_contradiction, 0, "{skips:?}");
    assert_eq!(
        skips.values_runtime_view, 0,
        "direct false identity candidates are retained: {skips:?}"
    );
    assert!(flow().reaching.iter().any(|r| r.condition.is_never()));
    assert!(flow().values.iter().any(|v| v.condition.is_never()));
}

#[test]
fn runtime_false_identity_keeps_native_source_and_live_parameter_origin() {
    let x = use_ix(
        "x",
        "return x if TYPE_CHECKING else y",
        "def value_runtime_branch",
    );
    let y = use_ix(
        "y",
        "return x if TYPE_CHECKING else y",
        "def value_runtime_branch",
    );
    let source = flow()
        .values
        .iter()
        .find(|v| v.use_ix == x && v.sink == Sink::Return)
        .expect("actual checked-false source candidate is retained");
    assert!(source.identity && !source.through_call && source.call_path.is_empty());
    assert!(source.condition.is_never());
    let origin = reaching(x);
    assert_eq!(
        origin,
        vec![(
            "x".into(),
            Some(BindingKind::Parameter),
            "false".into(),
            false
        )]
    );
    let live = flow()
        .values
        .iter()
        .find(|v| v.use_ix == y && v.sink == Sink::Return)
        .expect("live branch retains its independent source");
    assert!(live.identity && live.condition.is_always());
}

#[test]
fn false_only_candidates_preserve_unbound_competitors_and_do_not_poison_live_reads() {
    let source = "from typing import TYPE_CHECKING\n\ndef blocked(value, enabled):\n    if enabled:\n        target = value\n    return target if TYPE_CHECKING else None\n\ndef live(value, other):\n    target = value\n    if TYPE_CHECKING:\n        target = other\n    return target\n\ndef computed(value):\n    return value + 1 if TYPE_CHECKING else None\n";
    let mut runtime = RuntimeBindings::default();
    for (at, _) in source.match_indices("TYPE_CHECKING") {
        runtime.checking_names.insert(Span {
            start: at as u32,
            end: at as u32 + 13,
        });
    }
    let flow = cpg_flow::index(
        &[Input {
            path: "false_candidates.py".into(),
            text: source.into(),
            runtime,
        }],
        &RuntimeContext {
            python_version: (3, 14, 7),
            platform: "linux".into(),
        },
    )
    .pop()
    .unwrap();
    assert_eq!(flow.error, None);
    let at = source.find("return target if").unwrap() as u32 + 7;
    let use_ix = flow.uses.iter().position(|u| u.span.start == at).unwrap() as u32;
    let blocked = flow
        .reaching
        .iter()
        .filter(|r| r.use_ix == use_ix)
        .collect::<Vec<_>>();
    assert_eq!(
        blocked.len(),
        2,
        "the native Defined and Unbound alternatives must both survive: {blocked:?}"
    );
    assert!(
        blocked
            .iter()
            .all(|r| r.condition.is_never() && !r.condition.approximated())
    );
    assert!(blocked.iter().any(|r| r.def_ix.is_none()));
    assert!(blocked.iter().any(|r| r.def_ix.is_some()));
    let at = source.find("return target\n").unwrap() as u32 + 7;
    let use_ix = flow.uses.iter().position(|u| u.span.start == at).unwrap() as u32;
    let live = flow
        .reaching
        .iter()
        .filter(|r| r.use_ix == use_ix)
        .collect::<Vec<_>>();
    assert_eq!(
        live.len(),
        1,
        "inactive competing definitions cannot poison the live Entry inventory"
    );
    assert!(live[0].condition.is_always());
    let def = &flow.defs[live[0].def_ix.unwrap() as usize];
    let value = def.value.unwrap();
    assert_eq!(&source[value.start as usize..value.end as usize], "value");
    let at = source.find("return value + 1").unwrap() as u32 + 7;
    let use_ix = flow.uses.iter().position(|u| u.span.start == at).unwrap() as u32;
    assert!(
        !flow
            .values
            .iter()
            .any(|v| v.use_ix == use_ix && v.sink == Sink::Return),
        "computed false expressions remain outside the retained identity lane"
    );
}

#[test]
fn an_untranslatable_test_is_opaque_and_keeps_its_text() {
    assert_eq!(
        region("return n", "def walrus"),
        "opaque(\"(n := len(items)) > 3\")"
    );
}

#[test]
fn a_comprehension_reads_its_enclosing_scope_as_unbound_here() {
    let u = use_ix("prefix", "prefix + n", "def comprehension");
    let r = reaching(u);
    assert!(r.iter().all(|x| x.1.is_none()), "{r:?}");
    let n = use_ix("n", "prefix + n", "def comprehension");
    assert_eq!(reaching(n)[0].1, Some(BindingKind::ComprehensionTarget));
}

#[test]
fn a_loop_carries_its_definitions_around_the_back_edge() {
    let u = use_ix("total", "total = total + v", "def loop");
    let r = reaching(u);
    assert!(
        r.iter().any(|x| x.0 == "0" && !x.3),
        "the entry value: {r:?}"
    );
    assert!(
        r.iter().any(|x| x.0 == "total + v" && x.3),
        "the loop-carried value, never ty's loop header: {r:?}"
    );
    let ret = use_ix("total", "return total", "def loop");
    let r = reaching(ret);
    assert!(r.iter().any(|x| x.0 == "-1"), "the else branch: {r:?}");
}

#[test]
fn a_handler_rebinds_what_the_try_body_may_not_have_bound() {
    let u = use_ix("result", "return result, cleanup", "def handled");
    let shown: Vec<String> = reaching(u).into_iter().map(|x| x.0).collect();
    assert!(shown.contains(&"fn()".to_owned()), "{shown:?}");
    assert!(shown.contains(&"error".to_owned()), "{shown:?}");
}

#[test]
fn a_context_manager_may_suppress_and_says_so() {
    let u = use_ix("value", "return value", "def suppressed");
    let r = reaching(u);
    assert!(r.iter().any(|x| x.0 == "compute()"), "{r:?}");
    let early = r
        .iter()
        .find(|x| x.0 == "1")
        .expect("the pre-with value survives a suppression");
    assert!(early.2.contains("suppresses"), "{r:?}");
}

#[test]
fn admitted_ambiguous_paths_keep_approximation_provenance() {
    let f = flow();
    assert!(
        f.reaching.iter().any(|r| r.condition.approximated())
            || f.regions.iter().any(|r| r.condition.approximated()),
        "the try/with fixture should exercise ty's ambiguous terminal"
    );
}

#[test]
fn a_match_states_each_case() {
    let u = use_ix("state", "return state", "def matched");
    let r = reaching(u);
    let by_value = |v: &str| r.iter().find(|x| x.0 == v).unwrap().2.clone();
    assert_eq!(by_value("1"), "equals(command,\"start\")");
    assert_eq!(
        by_value("0"),
        "!equals(command,\"start\") & is_none(command)"
    );
    assert_eq!(
        by_value("-1"),
        "!equals(command,\"start\") & !is_none(command)"
    );
}

#[test]
fn a_membership_test_guards_its_branch() {
    assert_eq!(
        region("return host", "def modes"),
        "member_of(transport,{\"http\",\"sse\"})"
    );
}

#[test]
fn the_runtime_view_decides_static_branches() {
    // TYPE_CHECKING is false: the checker branch never runs, the runtime one always does.
    assert_eq!(region("marker = \"checker\"", "if TYPE_CHECKING:"), "false");
    assert_eq!(region("marker = \"runtime\"", "if TYPE_CHECKING:"), "true");
    // Python 3.14 takes the new branch.
    assert_eq!(
        region("version_branch = \"new\"", "sys.version_info"),
        "true"
    );
    assert_eq!(
        region("version_branch = \"old\"", "sys.version_info"),
        "false"
    );
    // Linux is not `nt`.
    assert_eq!(region("return \"windows\"", "def platform_branch"), "false");
    // Inside a function the checker branch's rebinding never reaches.
    let u = use_ix("value", "return value", "def type_checking_flag");
    assert_eq!(
        reaching(u),
        vec![(
            "value".to_owned(),
            Some(BindingKind::Parameter),
            "true".to_owned(),
            false
        )]
    );
    assert!(flow().renamed >= 3, "{}", flow().renamed);
}

#[test]
fn an_augmented_target_is_a_use_and_a_definition() {
    let target = use_ix("n", "n += 1", "def augmented");
    assert_eq!(reaching(target)[0].1, Some(BindingKind::Parameter));
    let ret = use_ix("n", "return n", "def augmented");
    assert_eq!(reaching(ret)[0].1, Some(BindingKind::AugAssignment));
}

#[test]
fn the_rename_touches_names_only_and_keeps_every_byte_offset() {
    let source = "if TYPE_CHECKING: x = 'TYPE_CHECKING'  # TYPE_CHECKING\n\
                  y = f\"{TYPE_CHECKING}\"\nMY_TYPE_CHECKING = typing.TYPE_CHECKING\n";
    let (renamed, n) = cpg_flow::rename(source).unwrap();
    assert_eq!(n, 3, "the name, the f-string field and the attribute");
    assert_eq!(renamed.len(), source.len());
    assert_eq!(
        renamed,
        "if TYPE_CHECKIN_: x = 'TYPE_CHECKING'  # TYPE_CHECKING\n\
         y = f\"{TYPE_CHECKIN_}\"\nMY_TYPE_CHECKING = typing.TYPE_CHECKIN_\n"
    );
    assert!(
        cpg_flow::rename("TYPE_CHECKIN_ = 1\n").is_err(),
        "a sentinel in use is refused"
    );
}

#[test]
fn an_elif_chain_states_each_arm() {
    let u = use_ix("result", "return result", "def elif_chain");
    let r = reaching(u);
    let by_value = |v: &str| r.iter().find(|x| x.0 == v).unwrap().2.clone();
    assert_eq!(by_value("1"), "equals(mode,\"a\")");
    assert_eq!(by_value("2"), "!equals(mode,\"a\") & equals(mode,\"b\")");
    assert_eq!(by_value("3"), "!equals(mode,\"a\") & !equals(mode,\"b\")");
}

#[test]
fn a_handler_around_safe_code_is_unreachable_in_the_stated_model() {
    // Ambient exceptions (`KeyboardInterrupt`) are outside the model (ADR-0022 §The flow
    // provider): the handler's rebinding never reaches.
    let u = use_ix("marker_value", "return marker_value", "def safe_try");
    let shown: Vec<String> = reaching(u).into_iter().map(|x| x.0).collect();
    assert_eq!(shown, vec!["1".to_owned()]);
}

#[test]
fn a_definition_in_both_type_checking_branches_runs_its_runtime_one() {
    assert_eq!(
        region("def both(x: int)", "if TYPE_CHECKING:\n\n    def both"),
        "false"
    );
    assert_eq!(region("def both(x):", "else:\n\n    def both"), "true");
}

#[test]
fn a_string_holding_the_word_keeps_it() {
    assert_eq!(
        region("return 1", "def string_mode"),
        "equals(flag,\"TYPE_CHECKING\")"
    );
}

#[test]
fn opaque_text_drops_comments() {
    assert_eq!(region("return a\n", "def commented"), "opaque(\"a > b\")");
}

#[test]
fn an_empty_literal_loop_keeps_the_entry_value() {
    let u = use_ix("last", "return last", "def empty_loop");
    let shown: Vec<String> = reaching(u).into_iter().map(|x| x.0).collect();
    assert!(shown.contains(&"None".to_owned()), "{shown:?}");
}

#[test]
fn all_native_graph_leaves_retain_a_coordinate_or_explicit_unavailability() {
    use cpg_flow::native::EvaluationSite;
    for graph in flow()
        .reaching
        .iter()
        .map(|r| &r.condition)
        .chain(flow().values.iter().map(|v| &v.condition))
        .chain(flow().regions.iter().map(|r| &r.condition))
    {
        for atom in graph.leaves() {
            assert!(matches!(
                atom,
                Atom::Evaluated {
                    site: EvaluationSite::Source(_) | EvaluationSite::Unavailable { .. },
                    ..
                }
            ));
        }
    }
}

#[test]
fn native_guarded_origin_search_keeps_evaluation_occurrences_and_reports_supply() {
    let source = include_str!("../../../fixtures/python/guarded_origin_inventory/cases.py");
    let flow = cpg_flow::index(
        &[Input {
            path: "cases.py".into(),
            text: source.into(),
            runtime: RuntimeBindings::default(),
        }],
        &RuntimeContext {
            python_version: (3, 14, 7),
            platform: "linux".into(),
        },
    )
    .pop()
    .unwrap();
    assert!(flow.error.is_none(), "{:?}", flow.error);
    let mut multi = 0;
    let mut useful = 0;
    for value in flow
        .values
        .iter()
        .filter(|v| v.identity && !v.through_call && v.sink == Sink::Return)
    {
        let use_ = &flow.uses[value.use_ix as usize];
        if use_.place != "value" {
            continue;
        }
        let candidates = flow
            .reaching
            .iter()
            .filter(|r| r.use_ix == value.use_ix)
            .collect::<Vec<_>>();
        let inventory = flow
            .candidates
            .iter()
            .filter(|r| r.use_ix == value.use_ix)
            .collect::<Vec<_>>();
        assert!(!inventory.is_empty());
        let region = flow
            .regions
            .iter()
            .filter(|r| {
                r.scope == use_.scope
                    && r.span.start <= value.span.start
                    && r.span.end >= value.span.end
            })
            .min_by_key(|r| r.span.end - r.span.start)
            .unwrap();
        let access = value.condition.and(&region.condition);
        let q = lower(&access).unwrap().0;
        let complete = inventory
            .iter()
            .all(|c| !c.pruned && !c.loop_expanded && !c.unattached && !c.reachability_lost);
        let parameters = candidates
            .iter()
            .filter(|r| {
                r.def_ix
                    .is_some_and(|ix| flow.defs[ix as usize].kind == BindingKind::Parameter)
            })
            .collect::<Vec<_>>();
        let competitors = candidates
            .iter()
            .filter(|r| {
                !r.def_ix
                    .is_some_and(|ix| flow.defs[ix as usize].kind == BindingKind::Parameter)
            })
            .collect::<Vec<_>>();
        if candidates.len() >= 2 {
            multi += 1;
        }
        let admitted = complete
            && candidates.len() >= 2
            && parameters.len() == 1
            && !q.is_false()
            && lower(&parameters[0].condition)
                .unwrap()
                .0
                .and(&q)
                .unwrap()
                .id()
                == q.id()
            && competitors
                .iter()
                .all(|r| lower(&r.condition).unwrap().0.and(&q).unwrap().is_false());
        if admitted {
            useful += 1;
        }
        eprintln!(
            "read={} retained={} native={} complete={} guarded_useful={}",
            use_.span.start,
            candidates.len(),
            inventory.len(),
            complete,
            admitted
        );
    }
    assert!(
        multi >= 3,
        "representative native joins must retain competing alternatives"
    );
    assert_eq!(
        useful, 0,
        "these cases do not admit guarded authority: repeated tests are distinct evaluation atoms and branch-local reads are already singleton"
    );
}
