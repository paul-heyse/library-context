#[path = "fixtures/flow_inventory.rs"]
mod fixture;
use lctx_model::domain::{flow::*, flow_inventory::*, resources::ResourceBudget, *};
#[test]
fn inventory_replays_complete_and_incomplete_sets_and_refuses_forgery() {
    for incomplete in [false, true] {
        let mut f = fixture::Fixture::new(incomplete);
        f.check().unwrap();
        assert_eq!(f.inventory.complete, !incomplete);
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let explanation = explain(
            f.flow.use_.id(),
            &f.inventory,
            &f.candidates,
            &f.members,
            &budget,
        )
        .unwrap();
        assert_eq!(
            explanation.candidates.len(),
            f.inventory.native_count as usize
        );
        drop(explanation);
        assert_eq!(budget.reserved(), 0);
        f.flow.base.put(Vec::<FlowUseInventoryMember>::new());
        assert!(f.check().is_err());
        f.flow.base.put(f.members.clone());
        f.flow.base.put(Vec::<FlowReachingSupport>::new());
        assert!(f.check().is_err());
        f.flow.base.put(vec![f.flow.reaching_support.clone()]);
        let mut wrong = f.candidates.clone();
        wrong[0].kind = FlowCandidateKind::Deleted;
        f.flow.base.put(wrong);
        assert!(f.check().is_err());
        f.flow.base.put(f.candidates.clone());
        let mut wrong = f.inventory.clone();
        wrong.complete = !wrong.complete;
        let id = wrong.id();
        let altered_candidates = f
            .candidates
            .iter()
            .cloned()
            .map(|mut c| {
                c.inventory = id;
                c
            })
            .collect::<Vec<_>>();
        let altered_members = f
            .members
            .iter()
            .cloned()
            .map(|mut m| {
                m.inventory = id;
                m
            })
            .collect::<Vec<_>>();
        let original_support = f.flow.base.rows::<FlowUseInventorySupport>()[0].clone();
        let mut altered_support = original_support.clone();
        altered_support.assertion = id;
        if incomplete {
            assert!(
                Batch::new(&f.flow.base.model, vec![wrong], &budget).is_err(),
                "complete flag with hidden candidate is refused before row admission"
            );
        } else {
            f.flow.base.put(vec![wrong]);
            f.flow.base.put(altered_candidates);
            f.flow.base.put(altered_members);
            f.flow.base.put(vec![altered_support]);
            assert!(f.check().is_err());
            f.flow.base.put(vec![f.inventory.clone()]);
            f.flow.base.put(f.candidates.clone());
            f.flow.base.put(f.members.clone());
            f.flow.base.put(vec![original_support]);
        }
        let mut view = f.flow.base.rows::<FlowSourceViewObservation>()[0].clone();
        view.source = f.flow.base.foreign.source;
        f.flow.base.put(vec![view]);
        assert!(f.check().is_err());
    }
}
#[test]
fn missing_hidden_boundary_and_partial_explanation_never_certify_closure() {
    let mut f = fixture::Fixture::new(true);
    f.check().unwrap();
    let budget = ResourceBudget::fixed(1 << 20).unwrap();
    assert!(
        explain(
            f.flow.use_.id(),
            &f.inventory,
            &f.candidates[..1],
            &f.members,
            &budget
        )
        .is_err()
    );
    f.flow.base.put(f.candidates[..1].to_vec());
    assert!(f.check().is_err());
    f.flow.base.put(f.candidates.clone());
    f.check().unwrap();
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(
        explain(
            f.flow.use_.id(),
            &f.inventory,
            &f.candidates,
            &f.members,
            &tiny
        )
        .is_err()
    );
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn each_enumeration_or_lowering_boundary_blocks_complete_closure() {
    let f = fixture::Fixture::new(false);
    let base = f.candidates[0].state();
    let variants = [
        CandidateState {
            pruned: true,
            mapped_count: 0,
            ..base.clone()
        },
        CandidateState {
            kind: FlowCandidateKind::LoopHeader,
            loop_expanded: true,
            mapped_count: 0,
            ..base.clone()
        },
        CandidateState {
            unattached: true,
            mapped_count: 0,
            ..base.clone()
        },
        CandidateState {
            condition_unavailable: true,
            reachability: None,
            mapped_count: 0,
            ..base.clone()
        },
        CandidateState {
            reachability_lost: true,
            ..base.clone()
        },
    ];
    for state in variants {
        let members = if state.mapped_count == 0 {
            vec![]
        } else {
            vec![(0, f.flow.reaching.id(), f.flow.reaching_support.id())]
        };
        let (inventory, _, _) = FlowUseInventoryObservation::new(
            f.inventory.qualification,
            f.inventory.use_,
            f.inventory.scope,
            f.inventory.view,
            &[state],
            &members,
        )
        .unwrap();
        assert!(!inventory.complete);
    }
    let (empty, _, _) = FlowUseInventoryObservation::new(
        f.inventory.qualification,
        f.inventory.use_,
        f.inventory.scope,
        f.inventory.view,
        &[],
        &[],
    )
    .unwrap();
    assert!(
        !empty.complete,
        "empty enumeration is not a completeness proof"
    );
    let too_many = vec![base; MAX_USE_CANDIDATES + 1];
    assert!(
        FlowUseInventoryObservation::new(
            f.inventory.qualification,
            f.inventory.use_,
            f.inventory.scope,
            f.inventory.view,
            &too_many,
            &[]
        )
        .is_err()
    );
}

#[path = "fixtures/stability.rs"]
mod entry_fixture;
#[test]
fn stored_singleton_outcomes_remain_distinct_from_enumeration_closure() {
    let f = entry_fixture::Fixture::new();
    let proof = f.derive().unwrap();
    let witness = proof.witness().clone();
    f.validate(&witness).unwrap();
    let artifact = f
        .data
        .artifacts
        .iter()
        .find(|a| a.id() == f.data.occurrences.get(f.use_.occurrence).unwrap().source)
        .unwrap();
    let view = FlowSourceViewObservation {
        qualification: f.q.id(),
        source: artifact.id(),
        original_content: artifact.content,
        view_content: artifact.content,
        byte_len: artifact.byte_len,
        renamed_type_checking: 0,
    };
    let scope = f
        .data
        .use_observations
        .get(witness.use_observation)
        .unwrap()
        .scope;
    let state = CandidateState {
        kind: FlowCandidateKind::Bound,
        pruned: false,
        loop_expanded: false,
        unattached: false,
        reachability: Some(f.q.id()),
            narrowing: Some(f.q.id()),
            narrowing_unavailable: false,
            narrowing_precision_lost: false,
            condition_unavailable: false,
        reachability_lost: false,
        mapped_count: 1,
    };
    let (inventory, candidates, members) = FlowUseInventoryObservation::new(
        f.q.id(),
        f.use_.id(),
        scope,
        view.id(),
        &[state],
        &[(0, witness.reaching, witness.reaching_support)],
    )
    .unwrap();
    let explanation = explain(f.use_.id(), &inventory, &candidates, &members, &f.budget).unwrap();
    let witnesses = vec![witness.clone()];
    let selected = explanation
        .entry_outcomes(
            &f.use_,
            f.request.context,
            &[f.request.run],
            &witnesses,
            &f.budget,
        )
        .unwrap();
    assert_eq!(selected.witnesses.len(), 1);
    assert_eq!(selected.reason(), None);
    let absent = explanation
        .entry_outcomes(&f.use_, f.request.context, &[], &witnesses, &f.budget)
        .unwrap();
    assert!(absent.witnesses.is_empty());
    assert_eq!(
        absent.reason(),
        Some(obligation::ObligationKind::EntryValueUnknown)
    );
    let other = FlowUse {
        occurrence: f
            .data
            .occurrences
            .iter()
            .find(|o| o.id() != f.use_.occurrence)
            .unwrap()
            .id(),
        ..f.use_.clone()
    };
    assert!(
        explanation
            .entry_outcomes(
                &other,
                f.request.context,
                &[f.request.run],
                &witnesses,
                &f.budget
            )
            .is_err()
    );
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(
        explanation
            .entry_outcomes(
                &f.use_,
                f.request.context,
                &[f.request.run],
                &witnesses,
                &tiny
            )
            .is_err()
    );
    assert_eq!(tiny.reserved(), 0);
}

#[test]
fn candidate_formula_identity_replays_unattached_and_separate_narrowing_fidelity() {
    let mut f = fixture::Fixture::bound_unattached();
    f.check().unwrap();
    assert!(!f.inventory.complete);
    assert_eq!(f.inventory.mapped_count, 0);
    assert_eq!(f.candidates[0].kind, FlowCandidateKind::Bound);
    assert!(f.candidates[0].reachability.is_some() && f.candidates[0].narrowing.is_some());
    let first = f.inventory.id();
    let unavailable = CandidateState { condition_unavailable: true, reachability: None, ..f.candidates[0].state() };
    f.replace_inventory(&[unavailable], &[]);
    f.check().unwrap();
    assert_ne!(first, f.inventory.id(), "availability belongs to inventory identity");
    let mut attached = fixture::Fixture::new(false);
    let first = attached.inventory.id();
    let state = CandidateState { narrowing: None, narrowing_unavailable: true, narrowing_precision_lost: true, ..attached.candidates[0].state() };
    let members = vec![(0, attached.flow.reaching.id(), attached.flow.reaching_support.id())];
    attached.replace_inventory(&[state], &members);
    attached.check().unwrap();
    assert!(attached.inventory.complete, "narrowing fidelity does not change reaching closure");
    assert_ne!(first, attached.inventory.id());
    let mut shuffled = fixture::Fixture::new(true);
    let original = shuffled.inventory.id();
    shuffled.candidates.reverse();
    shuffled.members.reverse();
    shuffled.flow.base.put(shuffled.candidates.clone());
    shuffled.flow.base.put(shuffled.members.clone());
    let mut qualifications = shuffled.flow.base.rows::<lctx_model::domain::assertion::AssertionQualification>();
    qualifications.reverse();
    shuffled.flow.base.put(qualifications);
    shuffled.check().unwrap();
    assert_eq!(shuffled.inventory.id(), original);
    explain(shuffled.inventory.use_, &shuffled.inventory, &shuffled.candidates, &shuffled.members, &ResourceBudget::fixed(1 << 20).unwrap()).unwrap();
}

#[test]
fn candidate_formula_replay_refuses_foreign_context_source_and_precision() {
    use lctx_model::domain::{assertion::*, conditions::*, value::*};
    for damage in ["context", "scope", "source", "precision", "narrowing_precision", "kind", "cardinality"] {
        let mut f = fixture::Fixture::new(false);
        let mut state = f.candidates[0].state();
        let mut q = f.flow.base.rows::<AssertionQualification>()[0].clone();
        let mut qualifications = f.flow.base.rows::<AssertionQualification>();
        match damage {
            "context" => { q.context = serde_json::from_value(serde_json::to_value([19u8; 16]).unwrap()).unwrap(); }
            "scope" => { q.scope = serde_json::from_value(serde_json::to_value([20u8; 16]).unwrap()).unwrap(); }
            "source" => {
                let atom = EvaluationAtom { evaluation: f.flow.base.foreign.id(), context: q.context,
                    predicate: Predicate::Truthy.id(), operand: None };
                let diagram = Diagram::from_atom(atom.id());
                let (condition, nodes) = diagram.records();
                f.flow.base.put(vec![atom]);
                let mut conditions = f.flow.base.rows::<Condition>(); conditions.push(condition.clone()); f.flow.base.put(conditions);
                let mut all_nodes = f.flow.base.rows::<ConditionNode>(); all_nodes.extend(nodes); f.flow.base.put(all_nodes);
                q.condition = condition.id();
            }
            "precision" => { state.reachability_lost = true; }
            "narrowing_precision" => { state.narrowing_precision_lost = true; }
            "kind" => { state.kind = FlowCandidateKind::Undefined; }
            "cardinality" => { state.mapped_count = 0; }
            _ => unreachable!(),
        }
        qualifications.push(q.clone());
        f.flow.base.put(qualifications);
        state.reachability = Some(q.id());
        let members = vec![(0, f.flow.reaching.id(), f.flow.reaching_support.id())];
        f.replace_inventory(&[state], &members);
        assert!(f.check().is_err(), "{damage}");
    }
    let f = fixture::Fixture::new(false);
    for state in [
        CandidateState { condition_unavailable: true, ..f.candidates[0].state() },
        CandidateState { narrowing_unavailable: true, ..f.candidates[0].state() },
        CandidateState { unattached: true, ..f.candidates[0].state() },
        CandidateState { mapped_count: 2, ..f.candidates[0].state() },
    ] {
        assert!(FlowUseInventoryObservation::new(f.inventory.qualification, f.inventory.use_, f.inventory.scope, f.inventory.view, &[state], &[]).is_err());
    }
}

#[test]
fn partially_attached_loop_header_keeps_members_without_kind_escape() {
    let mut f = fixture::Fixture::new(false);
    f.flow.reaching.loop_carried = true;
    f.flow.reaching_support.assertion = f.flow.reaching.id();
    f.flow.base.put(vec![f.flow.reaching.clone()]);
    f.flow.base.put(vec![f.flow.reaching_support.clone()]);
    let state = CandidateState { kind: FlowCandidateKind::LoopHeader, loop_expanded: true,
        unattached: true, ..f.candidates[0].state() };
    let members = vec![(0, f.flow.reaching.id(), f.flow.reaching_support.id())];
    f.replace_inventory(&[state], &members);
    f.check().unwrap();
    assert!(!f.inventory.complete);
    assert_eq!(f.inventory.mapped_count, 1);
}
