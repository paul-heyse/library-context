//! Independent transfer answers through real facts and normalized binding admission.
#[path = "fixtures/transfer_composition.rs"]
mod fixture;
use lctx_model::domain::composition::*;
#[tokio::test]
async fn normalized_native_call_receipts_admit_the_composition_fixture() {
    let f = fixture::native().await;
    for text in [
        "identity(seed)",
        "mutate(seed, target)",
        "variadic(seed, target, key=target)",
        "guarded(seed)",
        "rebound(seed)",
    ] {
        let attempt = f.attempt(text);
        assert!(
            CallBindingFrame::new(&f.verified, attempt, &f.data, &f.output).is_ok(),
            "{text}"
        );
    }
    assert!(!f.output.attempts.is_empty());
}
use lctx_model::domain::{assertion::*, attribution::*, conditions::*, transfer::*, value::*, *};
use std::collections::BTreeMap;
fn transfer(results: CompositionResults) -> Box<ComposedTransfer> {
    match results.into_iter().next().unwrap() {
        CallComposition::Transfer(row) => row,
        other => panic!("expected transfer: {other:?}"),
    }
}
fn obligation(results: CompositionResults) -> ObligationKind {
    match results.as_slice() {
        [CallComposition::Obligation(kind)] => *kind,
        other => panic!("expected obligation: {other:?}"),
    }
}
fn ends(t: &ComposedTransfer) -> ((PlaceRoot, AccessPath), (PlaceRoot, AccessPath)) {
    let end = |id| {
        let place = t.records.places.iter().find(|p| p.id() == id).unwrap();
        let root = t
            .records
            .roots
            .iter()
            .find(|p| p.id() == place.root)
            .unwrap()
            .clone();
        let path = t
            .records
            .paths
            .iter()
            .find(|p| p.id() == place.path)
            .unwrap()
            .clone();
        (root, path)
    };
    (end(t.descriptor.input), end(t.descriptor.output))
}
#[tokio::test]
async fn paths_compose_only_through_identity_and_roots_map_through_verified_binding() {
    let f = fixture::native().await;
    let mut c = f.case("identity(seed)");
    let attr = c.places.attribute("timeout");
    let source = c.actual(0, AccessPath::empty());
    let delivered = c.branch(
        &f,
        c.caller_owner,
        &source,
        &source,
        TransferKind::Identity,
        Diagram::always(),
    );
    let entry = c.entry(0, AccessPath::empty().extend(attr));
    let out = c.returned(AccessPath::empty());
    let reads = c.branch(
        &f,
        c.callee_owner,
        &entry,
        &out,
        TransferKind::Identity,
        Diagram::always(),
    );
    let t = transfer(c.compose(&f, &delivered, &reads));
    assert_eq!(
        ends(&t),
        (
            (
                PlaceRoot::Occurrence {
                    occurrence: c.actuals[0]
                },
                AccessPath::empty().extend(attr)
            ),
            (
                PlaceRoot::Occurrence {
                    occurrence: c.site.id()
                },
                AccessPath::empty()
            )
        )
    );
    assert_eq!(
        (
            t.descriptor.owner,
            t.descriptor.call_site,
            t.descriptor.kind,
            t.descriptor.provenance
        ),
        (
            c.caller_owner,
            Some(c.site.id()),
            TransferKind::Identity,
            ProvenanceClass::Composed
        )
    );
    assert_eq!(
        t.witness.caller.reference(),
        derivation::RowRef::of(delivered.alternative().id())
    );
    let parsed = c.branch(
        &f,
        c.caller_owner,
        &source,
        &source,
        TransferKind::Derived,
        Diagram::always(),
    );
    let t = transfer(c.compose(&f, &parsed, &reads));
    assert_eq!(ends(&t).0.1, AccessPath::empty());
    assert_eq!(t.descriptor.kind, TransferKind::Derived);
    let field = c.actual(0, AccessPath::empty().extend(attr));
    let delivery = c.branch(
        &f,
        c.caller_owner,
        &source,
        &field,
        TransferKind::Identity,
        Diagram::always(),
    );
    let entry = c.entry(0, AccessPath::empty());
    for kind in [TransferKind::Identity, TransferKind::Derived] {
        let reads = c.branch(&f, c.callee_owner, &entry, &out, kind, Diagram::always());
        let t = transfer(c.compose(&f, &delivery, &reads));
        assert_eq!(
            ends(&t).1.1,
            if kind == TransferKind::Identity {
                AccessPath::empty().extend(attr)
            } else {
                AccessPath::empty()
            }
        );
        assert_eq!(t.descriptor.kind, kind);
    }
    let foreign = c.places.place(
        PlaceRoot::Occurrence {
            occurrence: c.callee_declaration.declaration,
        },
        AccessPath::empty(),
    );
    let unrelated = c.branch(
        &f,
        c.caller_owner,
        &source,
        &foreign,
        TransferKind::Identity,
        Diagram::always(),
    );
    assert!(matches!(
        c.compose(&f, &unrelated, &reads).as_slice(),
        [CallComposition::Disjoint]
    ));
}
#[tokio::test]
async fn outputs_map_mutations_but_rebinding_and_captured_cells_do_not_cross_calls() {
    let f = fixture::native().await;
    let mut c = f.case("mutate(seed, target)");
    let attr = c.places.attribute("field");
    let source = c.actual(0, AccessPath::empty());
    let delivered = c.branch(
        &f,
        c.caller_owner,
        &source,
        &source,
        TransferKind::Identity,
        Diagram::always(),
    );
    let input = c.entry(0, AccessPath::empty());
    let output = c.entry(1, AccessPath::empty().extend(attr));
    let store = c.branch(
        &f,
        c.callee_owner,
        &input,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    let t = transfer(c.compose(&f, &delivered, &store));
    assert_eq!(
        ends(&t).1,
        (
            PlaceRoot::Occurrence {
                occurrence: c.actuals[1]
            },
            AccessPath::empty().extend(attr)
        )
    );
    assert_eq!(t.descriptor.owner, c.caller_owner);
    for output in [
        c.entry(1, AccessPath::empty()),
        c.formal(1, AccessPath::empty()),
    ] {
        let store = c.branch(
            &f,
            c.callee_owner,
            &input,
            &output,
            TransferKind::Identity,
            Diagram::always(),
        );
        assert!(matches!(
            c.compose(&f, &delivered, &store).as_slice(),
            [CallComposition::Disjoint]
        ));
    }
    let output = c.formal(1, AccessPath::empty().extend(attr));
    let store = c.branch(
        &f,
        c.callee_owner,
        &input,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    assert_eq!(
        obligation(c.compose(&f, &delivered, &store)),
        ObligationKind::EntryValueUnknown
    );
    for root in [
        PlaceRoot::Yield {
            callable: c.callee_declaration.declaration,
        },
        PlaceRoot::Raise {
            callable: c.callee_declaration.declaration,
        },
    ] {
        let output = c.places.place(root, AccessPath::empty());
        let store = c.branch(
            &f,
            c.callee_owner,
            &input,
            &output,
            TransferKind::Identity,
            Diagram::always(),
        );
        assert_eq!(
            obligation(c.compose(&f, &delivered, &store)),
            ObligationKind::UnsupportedControlFlow
        );
    }
    let output = c.places.place(
        PlaceRoot::Occurrence {
            occurrence: c.callee_declaration.declaration,
        },
        AccessPath::empty(),
    );
    let store = c.branch(
        &f,
        c.callee_owner,
        &input,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    assert_eq!(
        obligation(c.compose(&f, &delivered, &store)),
        ObligationKind::CapturedStateUnavailable
    );
    let foreign = c.places.place(
        PlaceRoot::Entry {
            declaration: c.caller_declaration.declaration,
        },
        AccessPath::empty(),
    );
    let store = c.branch(
        &f,
        c.callee_owner,
        &foreign,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    assert!(matches!(
        c.compose(&f, &delivered, &store).as_slice(),
        [CallComposition::Disjoint]
    ));
    let formal = c.formal(0, AccessPath::empty());
    let store = c.branch(
        &f,
        c.callee_owner,
        &formal,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    assert_eq!(
        obligation(c.compose(&f, &delivered, &store)),
        ObligationKind::EntryValueUnknown
    );
}
#[tokio::test]
async fn variadic_projection_crosses_only_strictly_below_the_element_slot() {
    let f = fixture::native().await;
    let mut c = f.case("variadic(seed, target, key=target)");
    let index = c.places.integer(0);
    let attr = c.places.attribute("field");
    let source = c.actual(0, AccessPath::empty());
    let delivered = c.branch(
        &f,
        c.caller_owner,
        &source,
        &source,
        TransferKind::Identity,
        Diagram::always(),
    );
    let input = c.entry(0, AccessPath::empty());
    for (path, should_flow) in [
        (AccessPath::empty(), false),
        (AccessPath::empty().extend(index), false),
        (AccessPath::empty().extend(index).extend(attr), true),
    ] {
        let output = c.entry(1, path);
        let store = c.branch(
            &f,
            c.callee_owner,
            &input,
            &output,
            TransferKind::Identity,
            Diagram::always(),
        );
        let results = c.compose(&f, &delivered, &store);
        if should_flow {
            assert_eq!(
                ends(&transfer(results)).1,
                (
                    PlaceRoot::Occurrence {
                        occurrence: c.actuals[1]
                    },
                    AccessPath::empty().extend(attr)
                )
            );
        } else {
            assert!(
                results
                    .iter()
                    .all(|r| matches!(r, CallComposition::Disjoint))
            );
        }
    }
    let kwargs = c
        .places
        .literals
        .values()
        .find(|l| matches!(l,Literal::String {value} if *value=="key"))
        .map(Record::id)
        .unwrap_or_else(|| {
            let row = Literal::String {
                value: "key".into(),
            };
            let id = row.id();
            c.places.literals.insert(id, row);
            id
        });
    let seg = PathSegment::Item { key: kwargs };
    let id = seg.id();
    c.places.segments.insert(id, seg);
    let output = c.entry(2, AccessPath::empty().extend(id).extend(attr));
    let store = c.branch(
        &f,
        c.callee_owner,
        &input,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    assert_eq!(
        ends(&transfer(c.compose(&f, &delivered, &store))).1,
        (
            PlaceRoot::Occurrence {
                occurrence: c.actuals[2]
            },
            AccessPath::empty().extend(attr)
        )
    );
}
#[tokio::test]
async fn admission_and_total_ports_refuse_missing_foreign_or_mutated_owners() {
    let f = fixture::native().await;
    let mut c = f.case("mutate(seed, target)");
    let actual = c.actual(0, AccessPath::empty());
    let entry = c.entry(0, AccessPath::empty());
    let output = c.returned(AccessPath::empty());
    let delivered = c.branch(
        &f,
        c.caller_owner,
        &actual,
        &actual,
        TransferKind::Identity,
        Diagram::always(),
    );
    let identity = c.branch(
        &f,
        c.callee_owner,
        &entry,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    let empty = BTreeMap::new();
    let compose = |frame: &CalleeFrame<'_>, call: &CallFrame<'_>, caller: &CallerFrame<'_>| {
        compose_call(
            &delivered,
            &identity,
            call,
            caller,
            frame,
            &c.places.catalog(),
            &f.budget,
        )
    };
    let mut call = c.call(&f);
    call.binding = None;
    assert_eq!(
        obligation(compose(&c.callee(Some(&empty)), &call, &c.caller()).unwrap()),
        ObligationKind::NonDefiniteAlternative
    );
    let frame = CalleeFrame {
        links: &c.links[..1],
        ..c.callee(Some(&empty))
    };
    assert_eq!(
        obligation(compose(&frame, &c.call(&f), &c.caller()).unwrap()),
        ObligationKind::NoSourceDeclaration
    );
    let mut bad = c.parameters.clone();
    bad[0].ordinal += 1;
    let frame = CalleeFrame {
        parameters: &bad,
        ..c.callee(Some(&empty))
    };
    assert!(compose(&frame, &c.call(&f), &c.caller()).is_err());
    let caller = CallerFrame {
        owner: c.callee_owner,
        ..c.caller()
    };
    assert!(compose(&c.callee(Some(&empty)), &c.call(&f), &caller).is_err());
    let budget = resources::ResourceBudget::fixed(1).unwrap();
    assert!(
        compose_call(
            &delivered,
            &identity,
            &c.call(&f),
            &c.caller(),
            &c.callee(Some(&empty)),
            &c.places.catalog(),
            &budget
        )
        .is_err()
    );
    let mut data = normalized::binding_normalization::BindingData::new(&f.budget);
    macro_rules! copy {($($field:ident:$ty:ty,)*)=>{$(for row in f.data.$field.iter() {data.$field.insert(row.clone()).unwrap();})*};}
    lctx_model::normalized_binding_inputs!(copy);
    let owner = f.verified.composition(c.attempt).unwrap().owner();
    let owners = data.owners.iter().cloned().collect::<Vec<_>>();
    data.owners = normalized::Rows::new(&f.budget);
    for mut row in owners {
        if row.id() == owner {
            row.entity = c.callee_owner;
        }
        data.owners.insert(row).unwrap();
    }
    let call = CallFrame {
        binding: Some(CallBindingFrame::new(&f.verified, c.attempt, &data, &f.output).unwrap()),
        ..c.call(&f)
    };
    assert!(
        compose_call(
            &delivered,
            &identity,
            &call,
            &c.caller(),
            &c.callee(Some(&empty)),
            &c.places.catalog(),
            &f.budget
        )
        .unwrap_err()
        .to_string()
        .contains("normalized composition owners"),
        "raw mutable payload cannot relabel a private admission"
    );
}
#[tokio::test]
async fn complementary_caller_conditions_keep_finite_alternatives_and_merge_to_true() {
    let f = fixture::native().await;
    let mut c = f.case("identity(seed)");
    let actual = c.actual(0, AccessPath::empty());
    let entry = c.entry(0, AccessPath::empty());
    let out = c.returned(AccessPath::empty());
    let through = c.branch(
        &f,
        c.callee_owner,
        &entry,
        &out,
        TransferKind::Identity,
        Diagram::always(),
    );
    let predicate = Predicate::Opaque {
        text: "caller control".into(),
    };
    let atom = EvaluationAtom {
        evaluation: c.site.id(),
        context: c.qualification.context,
        predicate: predicate.id(),
        operand: None,
    };
    c.places.predicates.insert(predicate.id(), predicate);
    c.places.atoms.insert(atom.id(), atom.clone());
    let mut branches = Vec::new();
    for condition in [
        Diagram::from_atom(atom.id()),
        Diagram::from_atom(atom.id()).not().unwrap(),
    ] {
        let delivered = c.branch(
            &f,
            c.caller_owner,
            &actual,
            &actual,
            TransferKind::Identity,
            condition,
        );
        let t = transfer(c.compose(&f, &delivered, &through));
        let branch = TransferBranch::<transfer::summary::TransferKey>::new(
            transfer::summary::TransferKey::from_descriptor(t.descriptor),
            t.qualification,
            t.condition,
            &f.budget,
        )
        .unwrap();
        branches.push(branch);
    }
    assert_eq!(branches[0].key(), branches[1].key());
    assert_ne!(branches[0].alternative(), branches[1].alternative());
    let merged = merge(branches, &f.budget).unwrap();
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].condition.id(), Diagram::always().id());
    assert_eq!(merged[0].alternatives.len(), 2);
}
#[tokio::test]
async fn condition_limits_are_obligations_and_modality_approximation_never_strengthen() {
    let f = fixture::native().await;
    let mut c = f.case("identity(seed)");
    let actual = c.actual(0, AccessPath::empty());
    let entry = c.entry(0, AccessPath::empty());
    let out = c.returned(AccessPath::empty());
    let mut diagrams = Vec::new();
    for name in ["caller", "callee"] {
        let mut condition = Diagram::always();
        for n in 0..100 {
            let predicate = Predicate::Opaque {
                text: format!("{name}{n}"),
            };
            let atom = EvaluationAtom {
                evaluation: c.site.id(),
                context: c.qualification.context,
                predicate: predicate.id(),
                operand: None,
            };
            c.places.predicates.insert(predicate.id(), predicate);
            c.places.atoms.insert(atom.id(), atom.clone());
            condition = condition.and(&Diagram::from_atom(atom.id())).unwrap();
        }
        diagrams.push(condition);
    }
    let caller = c.branch(
        &f,
        c.caller_owner,
        &actual,
        &actual,
        TransferKind::Identity,
        diagrams.remove(0),
    );
    let callee = c.branch(
        &f,
        c.callee_owner,
        &entry,
        &out,
        TransferKind::Identity,
        diagrams.remove(0),
    );
    assert!(matches!(
        obligation(c.compose(&f, &caller, &callee)),
        ObligationKind::ConditionAtomLimit
            | ObligationKind::ConditionNodeLimit
            | ObligationKind::ConditionWorkLimit
    ));
    let original = c.qualification.clone();
    for (modality, approximation) in [
        (Modality::Candidate, Approximation::Under),
        (Modality::Definite, Approximation::Over),
    ] {
        c.qualification = AssertionQualification {
            modality,
            approximation,
            ..original.clone()
        };
        let caller = c.branch(
            &f,
            c.caller_owner,
            &actual,
            &actual,
            TransferKind::Identity,
            Diagram::always(),
        );
        c.qualification = AssertionQualification {
            modality: Modality::Definite,
            approximation: Approximation::Over,
            ..original.clone()
        };
        let callee = c.branch(
            &f,
            c.callee_owner,
            &entry,
            &out,
            TransferKind::Identity,
            Diagram::always(),
        );
        c.qualification = original.clone();
        let t = transfer(c.compose(&f, &caller, &callee));
        assert_eq!(t.descriptor.modality, modality);
        assert_eq!(
            t.descriptor.approximation,
            approximation.join(Approximation::Over)
        );
    }
}
#[tokio::test]
async fn defaults_and_bound_or_unbound_receiver_ports_preserve_value_mapping() {
    let f = fixture::native().await;
    let mut c = f.case("defaulted(seed)");
    let attr = c.places.attribute("field");
    let source = c.actual(0, AccessPath::empty());
    let input = c.entry(0, AccessPath::empty());
    let output = c.entry(1, AccessPath::empty().extend(attr));
    let caller = c.branch(
        &f,
        c.caller_owner,
        &source,
        &source,
        TransferKind::Identity,
        Diagram::always(),
    );
    let callee = c.branch(
        &f,
        c.callee_owner,
        &input,
        &output,
        TransferKind::Identity,
        Diagram::always(),
    );
    assert_eq!(
        obligation(c.compose(&f, &caller, &callee)),
        ObligationKind::DefaultUnavailable
    );
    let instance = f.attempts_at("instance.update(seed)");
    assert!(!instance.is_empty());
    assert!(instance.iter().all(|a| a.authority
        == normalized::signature_applicability::BindingAuthority::SourceInspection
        && f.verified.composition(a.id()).is_none()));
    {
        let (text, arg, receiver) = ("ReceiverCase.update(instance, seed)", 1, Some(0));
        let mut c = f.case(text);
        let attr = c.places.attribute("field");
        let source = c.actual(arg, AccessPath::empty());
        let input = c.entry(1, AccessPath::empty());
        let output = c.entry(0, AccessPath::empty().extend(attr));
        let caller = c.branch(
            &f,
            c.caller_owner,
            &source,
            &source,
            TransferKind::Identity,
            Diagram::always(),
        );
        let callee = c.branch(
            &f,
            c.callee_owner,
            &input,
            &output,
            TransferKind::Identity,
            Diagram::always(),
        );
        let expected = match receiver {
            Some(index) => c.actuals[index],
            None => match c.receiver {
                calls::Receiver::Bound { actual } => actual,
                _ => panic!("source method missing bound receiver"),
            },
        };
        assert_eq!(
            ends(&transfer(c.compose(&f, &caller, &callee))).1,
            (
                PlaceRoot::Occurrence {
                    occurrence: expected
                },
                AccessPath::empty().extend(attr)
            )
        );
    }
}
#[tokio::test]
async fn finite_summary_witness_uses_actual_native_lineage_and_ordinary_companion() {
    let f = fixture::native().await;
    let w = fixture::WitnessFixture::new(&f);
    assert_eq!(
        w.emission.witness.status,
        analysis::policy::EvidenceStatus::StructurallyObserved
    );
    assert!(!w.emission.witness.heuristic);
    let admission = f.verified.composition(w.emission.witness.attempt).unwrap();
    match admission.signature_closure() {
        normalized::binding_normalization::SourceBodySignatureClosure::GlobalCoverage { .. } => {
            assert_eq!(w.emission.witness.selected_signature_enumeration, None);
            assert_eq!(w.emission.witness.selected_signature_enumeration_support, None);
        }
        normalized::binding_normalization::SourceBodySignatureClosure::DeclaredEnumeration {
            enumeration, support, ..
        } => {
            assert_eq!(w.emission.witness.selected_signature_enumeration, Some(enumeration));
            assert_eq!(w.emission.witness.selected_signature_enumeration_support, Some(support));
        }
    }
    assert_eq!(w.rows::<transfer::summary::TransferSupport>().len(), 1);
    assert_eq!(w.rows::<analysis::summary::AnalysisDerivation>().len(), 1);
}
#[tokio::test]
async fn stored_witness_replay_preserves_membership_paths_condition_and_evidence() {
    let f = fixture::native().await;
    let w = fixture::WitnessFixture::new(&f);
    w.check::<transfer::summary::SummaryWitness>(&f.budget)
        .unwrap();
    w.check::<transfer::summary::TransferSupport>(&f.budget)
        .unwrap();
    w.check::<analysis::summary::AnalysisDerivation>(&f.budget)
        .unwrap();
    for (mutation, reason) in [
        (
            fixture::Mutation::CalleeFromAnotherOwner,
            "exact normalized binding",
        ),
        (fixture::Mutation::NotComposed, "caller's transfer"),
        (
            fixture::Mutation::UnrestatedCondition,
            "restated at the call",
        ),
        (fixture::Mutation::ForeignOutput, "ports or paths"),
        (fixture::Mutation::ForgedPath, "ports or paths"),
        (fixture::Mutation::ForgedStatus, "evidence lineage"),
    ] {
        let w = fixture::WitnessFixture::mutated(&f, Some(mutation));
        let error = w
            .check::<transfer::summary::SummaryWitness>(&f.budget)
            .unwrap_err();
        assert!(error.to_string().contains(reason), "{mutation:?}: {error}");
    }
}
#[tokio::test]
async fn callee_conditions_remain_qualified_and_unjustified_guards_refuse() {
    let f = fixture::native().await;
    let mut c = f.case("guarded(seed)");
    let source = c.actual(0, AccessPath::empty());
    let input = c.entry(0, AccessPath::empty());
    let output = c.returned(AccessPath::empty());
    let witnesses = c.guards(&f);
    let atom = *witnesses.keys().next().unwrap();
    let condition = Diagram::from_atom(atom).not().unwrap();
    let caller = c.branch(
        &f,
        c.caller_owner,
        &source,
        &source,
        TransferKind::Identity,
        Diagram::always(),
    );
    let callee = c.branch(
        &f,
        c.callee_owner,
        &input,
        &output,
        TransferKind::Identity,
        condition,
    );
    let compose = |witnesses| {
        compose_call(
            &caller,
            &callee,
            &c.call(&f),
            &c.caller(),
            &c.callee(witnesses),
            &c.places.catalog(),
            &f.budget,
        )
        .unwrap()
    };
    let t = transfer(compose(Some(&witnesses)));
    assert_eq!(t.condition.support().len(), 1);
    assert_ne!(t.condition.id(), Diagram::always().id());
    assert_ne!(t.condition.id(), callee.condition().id());
    assert_eq!(t.records.substitutions.len(), 1);
    assert_eq!(t.records.influences.len(), 1);
    assert!(
        t.records
            .substitutions
            .iter()
            .all(|s| s.qualification == t.qualification.id())
    );
    assert!(
        t.records
            .influences
            .iter()
            .all(|s| s.qualification == t.qualification.id())
    );
    assert_eq!(t.descriptor.owner, c.caller_owner);
    let empty = BTreeMap::new();
    assert_eq!(obligation(compose(None)), ObligationKind::NotRequested);
    assert!(matches!(
        compose(Some(&empty)).as_slice(),
        [CallComposition::Obligation(_)]
    ));
    let before = f.budget.reserved();
    let t = transfer(compose(Some(&witnesses)));
    assert!(f.budget.reserved() > before);
    drop(t);
    assert_eq!(f.budget.reserved(), before);
}
#[tokio::test]
async fn site_composition_preserves_alternatives_and_refuses_pair_work_before_execution() {
    let f = fixture::native().await;
    let mut c = f.case("identity(seed)");
    let actual = c.actual(0, AccessPath::empty());
    let entry = c.entry(0, AccessPath::empty());
    let out = c.returned(AccessPath::empty());
    let predicate = Predicate::Opaque {
        text: "site branch".into(),
    };
    let atom = EvaluationAtom {
        evaluation: c.site.id(),
        context: c.qualification.context,
        predicate: predicate.id(),
        operand: None,
    };
    c.places.predicates.insert(predicate.id(), predicate);
    c.places.atoms.insert(atom.id(), atom.clone());
    let a = Diagram::from_atom(atom.id());
    let callers = [
        c.branch(
            &f,
            c.caller_owner,
            &actual,
            &actual,
            TransferKind::Identity,
            a.clone(),
        ),
        c.branch(
            &f,
            c.caller_owner,
            &actual,
            &actual,
            TransferKind::Identity,
            a.not().unwrap(),
        ),
    ];
    let through = c.branch(
        &f,
        c.callee_owner,
        &entry,
        &out,
        TransferKind::Identity,
        Diagram::always(),
    );
    let branches = [through.clone()];
    let empty = BTreeMap::new();
    let calls = [SiteCall {
        frame: c.call(&f),
        callee: Some(c.callee(Some(&empty))),
        branches: &branches,
    }];
    let before = f.budget.reserved();
    let result = compose_site(
        &callers,
        &calls,
        &c.caller(),
        &c.places.catalog(),
        &f.budget,
    )
    .unwrap();
    assert_eq!(result.len(), 2);
    assert!(f.budget.reserved() > before);
    let conditions = result
        .iter()
        .map(|r| match r {
            CallComposition::Transfer(t) => t.condition.id(),
            r => panic!("{r:?}"),
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(conditions.len(), 2);
    drop(result);
    assert_eq!(f.budget.reserved(), before);
    let _fixture_charge = f
        .budget
        .reserve("pair work fixture", 100_001 * size_of::<TransferBranch>())
        .unwrap();
    let branches = vec![through; 100_001];
    let calls = [SiteCall {
        frame: c.call(&f),
        callee: Some(c.callee(Some(&empty))),
        branches: &branches,
    }];
    assert_eq!(
        obligation(
            compose_site(
                &callers[..1],
                &calls,
                &c.caller(),
                &c.places.catalog(),
                &f.budget
            )
            .unwrap()
        ),
        ObligationKind::SummaryPairWorkLimit
    );
}

#[tokio::test]
async fn selected_source_signature_witness_replay_refuses_missing_and_foreign_domain() {
    use normalized::binding_normalization::SourceBodySignatureClosure;
    let f = fixture::native_from("source_body_shapes").await;
    let text = "inner(value)";
    let witness = fixture::WitnessFixture::mutated_at(&f, text, None);
    let admission = f.verified.composition(witness.emission.witness.attempt).unwrap();
    let SourceBodySignatureClosure::DeclaredEnumeration { enumeration, support, .. } =
        admission.signature_closure()
    else {
        panic!("selected source closure must preserve unrelated incomplete signature family")
    };
    assert_eq!(witness.emission.witness.selected_signature_enumeration, Some(enumeration));
    assert_eq!(witness.emission.witness.selected_signature_enumeration_support, Some(support));
    witness.check::<transfer::summary::SummaryWitness>(&f.budget).unwrap();
    for mutation in [
        fixture::Mutation::MissingSelectedEnumeration,
        fixture::Mutation::MissingSelectedEnumerationSupport,
        fixture::Mutation::ForeignSelectedEnumeration,
        fixture::Mutation::ForeignSelectedEnumerationSupport,
    ] {
        let witness = fixture::WitnessFixture::mutated_at(&f, text, Some(mutation));
        let error = witness.check::<transfer::summary::SummaryWitness>(&f.budget).unwrap_err();
        assert!(error.to_string().contains("exact normalized binding"), "{mutation:?}: {error}");
    }
}
