#[allow(
    dead_code,
    reason = "shared typed fixtures expose controls beyond this focused substitution suite"
)]
#[path = "fixtures/types.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    assertion::*, attribution::*, calls::*, normalized::generic_specialization::*,
    resources::ResourceBudget, source::*, types::*, *,
};
fn fixture() -> (Fixture, Graph, Environment, ResourceBudget) {
    let mut f = Fixture::new(false);
    f.native_signature_ports("none");
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let mut graph = Graph::new(&budget);
    for (name, batch) in &f.base.batches {
        graph.visit(name, batch).unwrap();
    }
    let native = f.base.rows::<NativeSignatureObservation>()[0].clone();
    let qualification = native.qualification;
    let site = f
        .base
        .rows::<Occurrence>()
        .into_iter()
        .find(|o| o.id() != f.base.foreign.id())
        .unwrap()
        .id();
    let argument = TypeTerm::None;
    graph.terms.insert(argument.clone()).unwrap();
    let binding = GenericSpecializationObservation {
        qualification,
        site,
        declaration: native.id(),
        variable: f.variable.id(),
        argument: argument.id(),
        scope: native.scope,
        receiver: f.term.id(),
    };
    let environment = Environment {
        qualification,
        declaration: native.id(),
        site,
        bindings: vec![binding],
    };
    (f, graph, environment, budget)
}
#[test]
fn distinct_native_binders_substitute_and_shadow_without_spelling_authority() {
    let (f, mut graph, env, budget) = fixture();
    let root = TypeTerm::TypeVar {
        variable: f.variable.id(),
    };
    graph.terms.insert(root.clone()).unwrap();
    let out = substitute(&graph, root.id(), &env, &budget).unwrap();
    assert_eq!(out.status, SpecializationStatus::Resolved);
    assert_eq!(out.term, TypeTerm::None.id());
    assert_eq!(out.support, vec![env.bindings[0].id()]);
    let mut other = f.variable.clone();
    other.slot += 1;
    let other_id = graph.variables.insert(other).unwrap();
    let other = TypeTerm::TypeVar { variable: other_id };
    graph.terms.insert(other.clone()).unwrap();
    let out = substitute(&graph, other.id(), &env, &budget).unwrap();
    assert_eq!(out.term, other.id());
    assert_eq!(out.status, SpecializationStatus::Partial);
    assert!(
        out.boundaries
            .contains(&Boundary::UnboundVariable(other_id))
    );
    let (parameters, members) =
        TypeSequence::new(&[(TypeChildRole::TypeParameter, root.id())]).unwrap();
    let parameters = graph.sequences.insert(parameters).unwrap();
    for m in members {
        graph.members.insert(m).unwrap();
    }
    let generic = TypeTerm::Generic {
        parameters,
        body: root.id(),
    };
    graph.terms.insert(generic.clone()).unwrap();
    let out = substitute(&graph, generic.id(), &env, &budget).unwrap();
    assert_eq!(
        out.term,
        generic.id(),
        "inner binder shadows receiver environment"
    );
    assert!(
        out.boundaries
            .contains(&Boundary::UnboundVariable(f.variable.id()))
    );
    let mut duplicated = Environment {
        bindings: vec![env.bindings[0].clone(), env.bindings[0].clone()],
        ..env
    };
    duplicated.bindings.reverse();
    let out = substitute(&graph, root.id(), &duplicated, &budget).unwrap();
    assert_eq!(out.support.len(), 1);
}
#[test]
fn unresolved_paramspec_recursive_alias_named_int_and_work_bounds_remain_explicit() {
    let (f, mut graph, env, budget) = fixture();
    let mut p = f.variable.clone();
    p.slot += 2;
    p.kind = TypeVariableKind::ParamSpec;
    let p = graph.variables.insert(p).unwrap();
    let pterm = TypeTerm::ParamSpec { variable: p };
    graph.terms.insert(pterm.clone()).unwrap();
    let out = substitute(&graph, pterm.id(), &env, &budget).unwrap();
    assert!(out.boundaries.contains(&Boundary::UnresolvedParamSpec(p)));
    assert_eq!(out.status, SpecializationStatus::Partial);
    let mut i = f.variable.clone();
    i.slot += 3;
    i.kind = TypeVariableKind::IntVar;
    let i = graph.variables.insert(i).unwrap();
    let iterm = TypeTerm::TypeVar { variable: i };
    graph.terms.insert(iterm.clone()).unwrap();
    assert!(
        substitute(&graph, iterm.id(), &env, &budget)
            .unwrap()
            .boundaries
            .contains(&Boundary::NamedInt(i))
    );
    let (arguments, _) = TypeSequence::new(&[]).unwrap();
    let arguments = graph.sequences.insert(arguments).unwrap();
    let alias = TypeTerm::TypeAliasReference {
        module: f.variable.module,
        name: "Recursive".into(),
        untyped: false,
        arguments,
    };
    graph.terms.insert(alias.clone()).unwrap();
    let out = substitute(&graph, alias.id(), &env, &budget).unwrap();
    assert_eq!(out.term, alias.id());
    assert!(
        out.boundaries
            .contains(&Boundary::AliasReference(alias.id()))
    );
    let mut root = TypeTerm::None.id();
    graph.terms.insert(TypeTerm::None).unwrap();
    for _ in 0..140 {
        let term = TypeTerm::Annotated { target: root };
        root = graph.terms.insert(term).unwrap();
    }
    let out = substitute(&graph, root, &env, &budget).unwrap();
    assert!(
        out.boundaries
            .iter()
            .any(|b| matches!(b, Boundary::WorkBound(_)))
    );
    assert_eq!(out.status, SpecializationStatus::Partial);
    let empty = Environment {
        bindings: vec![],
        ..env
    };
    let out = substitute(&graph, pterm.id(), &empty, &budget).unwrap();
    assert_eq!(out.status, SpecializationStatus::Unknown);
    assert!(out.boundaries.contains(&Boundary::MissingNativeBinding));
}
#[test]
fn contradictory_site_context_and_receiver_maps_are_refused() {
    let (f, mut graph, env, budget) = fixture();
    let term = TypeTerm::TypeVar {
        variable: f.variable.id(),
    };
    graph.terms.insert(term.clone()).unwrap();
    let mut bad = env.bindings[0].clone();
    bad.argument = term.id();
    let mut mixed = Environment {
        qualification: env.qualification,
        declaration: env.declaration,
        site: env.site,
        bindings: vec![env.bindings[0].clone(), bad],
    };
    assert!(substitute(&graph, term.id(), &mixed, &budget).is_err());
    mixed.bindings.truncate(1);
    mixed.bindings[0].site = f.base.foreign.id();
    assert!(substitute(&graph, term.id(), &mixed, &budget).is_err());
    mixed.bindings[0] = env.bindings[0].clone();
    let mut other = mixed.bindings[0].clone();
    other.receiver = term.id();
    mixed.bindings.push(other);
    assert!(substitute(&graph, term.id(), &mixed, &budget).is_err());
    mixed.bindings.truncate(1);
    let mut q = f.base.rows::<AssertionQualification>()[0].clone();
    q.approximation = Approximation::Unknown;
    mixed.bindings[0].qualification = q.id();
    assert!(substitute(&graph, term.id(), &mixed, &budget).is_err());
}
#[test]
fn shared_specialization_invariant_rejects_foreign_context_provider_and_noncallee_sites() {
    for mismatch in ["none", "context", "provider", "site"] {
        let (mut f, _, env, _) = fixture();
        let mut row = env.bindings[0].clone();
        let q = f.base.rows::<AssertionQualification>()[0].clone();
        let call = CallSyntax::new(q.id(), row.site, row.site, false, &[])
            .unwrap()
            .0;
        f.base.put(vec![call]);
        if mismatch == "context" {
            let mut other = q.clone();
            let mut context = f.base.rows::<AnalysisContext>()[0].clone();
            context.config_digest = ContentHash::of(b"other-context");
            other.context = context.id();
            let mut rows = f.base.rows::<AssertionQualification>();
            rows.push(other.clone());
            f.base.put(rows);
            row.qualification = other.id();
        }
        if mismatch == "provider" {
            let mut v = f.variable.clone();
            v.provider = Provider {
                tool: "foreign".into(),
                revision: "1".into(),
                build_digest: ContentHash::of(b"foreign"),
            }
            .id();
            let mut rows = f.base.rows::<TypeVariable>();
            rows.push(v.clone());
            f.base.put(rows);
            row.variable = v.id();
        }
        if mismatch == "site" {
            row.site = f.base.foreign.id();
        }
        f.base.put(vec![row]);
        let check = GenericSpecializationObservation::invariants()
            .into_iter()
            .find(|i| i.name == "generic_specialization_basis")
            .unwrap();
        assert_eq!(
            f.base.check(&check).is_ok(),
            mismatch == "none",
            "{mismatch}"
        );
    }
}
