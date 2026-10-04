//! Shared entry-value producer and stored replay retain unsupported cases as obligations.
#[path = "fixtures/stability.rs"]
mod fixture;
use fixture::Fixture;
use lctx_model::domain::{
    assertion::AssertionQualification,
    attribution::*,
    conditions::{
        entry::{EntryAccessSource, EntryValueWitness},
        stability::*,
        *,
    },
    flow::*,
    lexical::BindingEventKind,
    normalized::Rows,
    value::*,
    *,
};
fn replace_reaching(f: &mut Fixture, row: FlowReachingObservation) {
    let mut support = f.data.reaching_supports.iter().next().unwrap().clone();
    support.assertion = row.id();
    f.data.reaching = Rows::new(&f.budget);
    f.data.reaching.insert(row).unwrap();
    f.data.reaching_supports = Rows::new(&f.budget);
    f.data.reaching_supports.insert(support).unwrap();
}
#[test]
fn entry_value_and_guard_stability_share_one_replayed_access() {
    let f = Fixture::new();
    let entry = f.guard_entry().unwrap();
    f.validate(entry.witness()).unwrap();
    assert!(matches!(entry.root(), PlaceRoot::Entry { .. }));
    let proof = StabilityWitness::derive(&f.data, f.guard.id(), &entry).unwrap();
    assert_eq!(proof.witness().entry, entry.witness().id());
    let inputs = stability_invariants().remove(0).inputs;
    assert!(
        !inputs
            .iter()
            .any(|i| i.name() == GuardSubstitution::NAME || i.name().contains("control_influence")),
        "Local stability must not depend on Summary outputs"
    );
}
#[test]
fn assignment_nested_unbound_and_loop_reaches_never_prove_entry_identity() {
    for mutation in 0..5 {
        let mut f = Fixture::new();
        let stored = f.derive().unwrap().witness().clone();
        match mutation {
            0 => {
                let mut row = f.definition.clone();
                row.kind = BindingEventKind::Assignment;
                let mut s = f.data.definition_supports.iter().next().unwrap().clone();
                s.assertion = row.id();
                f.data.definition_observations = Rows::new(&f.budget);
                f.data.definition_observations.insert(row).unwrap();
                f.data.definition_supports = Rows::new(&f.budget);
                f.data.definition_supports.insert(s).unwrap();
            }
            1 | 2 => {
                let target = if mutation == 1 {
                    ReachingDefinition::Nested
                } else {
                    ReachingDefinition::Unbound
                };
                f.data.targets.insert(target.clone()).unwrap();
                let row = FlowReachingObservation {
                    target: target.id(),
                    ..f.reaching.clone()
                };
                replace_reaching(&mut f, row);
            }
            3 => {
                let row = FlowReachingObservation {
                    loop_carried: true,
                    ..f.reaching.clone()
                };
                replace_reaching(&mut f, row);
            }
            _ => {
                let target = ReachingDefinition::Unbound;
                f.data.targets.insert(target.clone()).unwrap();
                let row = FlowReachingObservation {
                    target: target.id(),
                    ..f.reaching.clone()
                };
                let mut support = f.data.reaching_supports.iter().next().unwrap().clone();
                support.assertion = row.id();
                f.data.reaching.insert(row).unwrap();
                f.data.reaching_supports.insert(support).unwrap();
            }
        }
        assert!(
            matches!(f.derive(), Err(ObligationKind::EntryValueUnknown)),
            "mutation {mutation}"
        );
        assert!(f.validate(&stored).is_err());
    }
}
#[test]
fn provider_context_support_and_coverage_are_selected_together() {
    for mutation in 0..7 {
        let mut f = Fixture::new();
        let stored = f.derive().unwrap().witness().clone();
        match mutation {
            0 => {
                f.data.use_supports = Rows::new(&f.budget);
            }
            1 => {
                let mut c = f.coverage.clone();
                c.status = CoverageStatus::Failed;
                c.reason = Some(ObligationKind::MissingEvidence);
                f.data.coverage = Rows::new(&f.budget);
                f.data.coverage.insert(c).unwrap();
            }
            2 => {
                let mut q = f.q.clone();
                let mut context = AnalysisContext {
                    python_version: "3.14.7".into(),
                    python_platform: "foreign".into(),
                    search_path: vec![],
                    site_package_path: vec![],
                    config_digest: ContentHash::of(b"foreign"),
                    environment_digest: ContentHash::of(b"foreign"),
                    lock_digest: None,
                };
                q.context = context.id();
                context.python_platform = "foreign".into();
                f.data.qualifications.insert(q.clone()).unwrap();
                let row = FlowReachingObservation {
                    qualification: q.id(),
                    ..f.reaching.clone()
                };
                replace_reaching(&mut f, row);
            }
            3 => {
                let mut s = f.data.reaching_supports.iter().next().unwrap().clone();
                s.fidelity = Fidelity::ReportProjection;
                f.data.reaching_supports = Rows::new(&f.budget);
                f.data.reaching_supports.insert(s).unwrap();
            }
            4 => {
                f.data.declaration_supports = Rows::new(&f.budget);
            }
            5 => {
                let mut c = f.coverage.clone();
                c.provider = Some(
                    Provider {
                        tool: "foreign".into(),
                        revision: "1".into(),
                        build_digest: ContentHash::of(b"foreign"),
                    }
                    .id(),
                );
                f.data.coverage = Rows::new(&f.budget);
                f.data.coverage.insert(c).unwrap();
            }
            _ => {
                let mut q = f.q.clone();
                q.modality = Modality::Candidate;
                f.data.qualifications.insert(q.clone()).unwrap();
                let row = FlowReachingObservation {
                    qualification: q.id(),
                    ..f.reaching.clone()
                };
                replace_reaching(&mut f, row);
            }
        }
        assert!(f.derive().is_err(), "mutation {mutation}");
        assert!(f.validate(&stored).is_err());
    }
}
#[test]
fn binding_identity_never_becomes_mutable_predicate_stability() {
    let mut f = Fixture::new();
    let entry = f.guard_entry().unwrap();
    for predicate in [
        Predicate::Truthy,
        Predicate::Equals {
            value: Literal::None.id(),
        },
        Predicate::Opaque {
            text: "field changed".into(),
        },
    ] {
        f.data.predicates.insert(predicate.clone()).unwrap();
        let atom = EvaluationAtom {
            predicate: predicate.id(),
            ..f.guard.clone()
        };
        f.data.atoms.insert(atom.clone()).unwrap();
        assert_eq!(
            StabilityWitness::derive(&f.data, atom.id(), &entry).unwrap_err(),
            ObligationKind::ConditionTransferUnsupported
        );
    }
    assert!(
        StabilityBasis::ParameterOnlyReaching.eligible(&Predicate::IsValue {
            value: Literal::None.id()
        })
    );
    assert!(
        !StabilityBasis::ParameterOnlyReaching.eligible(&Predicate::Equals {
            value: Literal::None.id()
        })
    );
}
#[test]
fn stored_witness_cannot_choose_a_different_existing_coverage_premise() {
    let mut f = Fixture::new();
    let artifact = f.data.artifacts.iter().next().unwrap().id();
    let scope = lctx_model::domain::source::CoverageScope::Artifact { artifact };
    f.data.scopes.insert(scope.clone()).unwrap();
    let other = ProviderCoverage {
        scope: scope.id(),
        ..f.coverage.clone()
    };
    f.data.coverage.insert(other.clone()).unwrap();
    f.seed_inventory();
    let proof = f.derive().unwrap();
    f.validate(proof.witness()).unwrap();
    let mut forged = proof.witness().clone();
    forged.coverage = if forged.coverage == other.id() {
        f.coverage.id()
    } else {
        other.id()
    };
    assert!(f.validate(&forged).is_err());
}
#[test]
fn entry_allowance_follows_the_checked_guard_until_its_last_consumer_drops() {
    let f = Fixture::new();
    let before = f.budget.reserved();
    let entry = f.guard_entry().unwrap();
    let proof = StabilityWitness::derive(&f.data, f.guard.id(), &entry).unwrap();
    let retained = f.budget.reserved();
    assert!(retained > before);
    let clone = proof.clone();
    drop(entry);
    drop(proof);
    assert_eq!(f.budget.reserved(), retained);
    drop(clone);
    assert_eq!(f.budget.reserved(), before);
}

#[test]
fn a_guarded_read_proves_entry_only_when_its_condition_is_covered_by_parameter_reaching() {
    let mut f = Fixture::new();
    let guarded = Diagram::from_atom(f.guard.id());
    let (always, nodes) = Diagram::always().records();
    f.data.conditions.insert(always).unwrap();
    for row in nodes {
        f.data.condition_nodes.insert(row).unwrap();
    }
    let (condition, nodes) = guarded.records();
    f.data.conditions.insert(condition).unwrap();
    for row in nodes {
        f.data.condition_nodes.insert(row).unwrap();
    }
    let q = AssertionQualification {
        condition: guarded.id(),
        ..f.q.clone()
    };
    f.data.qualifications.insert(q.clone()).unwrap();
    let old = f.data.use_observations.iter().next().unwrap().clone();
    let changed = FlowUseObservation {
        qualification: q.id(),
        ..old
    };
    let mut support = f.data.use_supports.iter().next().unwrap().clone();
    support.assertion = changed.id();
    f.data.use_observations = Rows::new(&f.budget);
    f.data.use_observations.insert(changed).unwrap();
    f.data.use_supports = Rows::new(&f.budget);
    f.data.use_supports.insert(support).unwrap();
    f.seed_inventory();
    let proof = f.derive().unwrap();
    f.validate(proof.witness()).unwrap();
    let impossible = Diagram::never();
    let (condition, nodes) = impossible.records();
    f.data.conditions.insert(condition).unwrap();
    for row in nodes {
        f.data.condition_nodes.insert(row).unwrap();
    }
    let q = AssertionQualification {
        condition: impossible.id(),
        ..f.q.clone()
    };
    f.data.qualifications.insert(q.clone()).unwrap();
    let row = FlowReachingObservation {
        qualification: q.id(),
        ..f.reaching.clone()
    };
    replace_reaching(&mut f, row);
    assert!(f.derive().is_err());
    assert!(f.validate(proof.witness()).is_err());
}
#[test]
fn guard_identity_must_hold_before_both_truth_arms() {
    use lctx_model::domain::{normalized::entities::OccurrenceOwnership, source::*};
    let mut f = Fixture::new();
    let read = f.data.occurrences.get(f.request.access).unwrap();
    let owner = f
        .data
        .owners
        .iter()
        .find(|o| o.occurrence == read.id())
        .unwrap()
        .clone();
    let statement = Occurrence {
        source: read.source,
        start: 17,
        end: read.end + 20,
        syntax_kind: SyntaxKind::StmtIf,
        role: OccurrenceRole::Syntax,
        structural_path: vec![0, 0, 1],
    };
    f.data.occurrences.insert(statement.clone()).unwrap();
    f.data
        .owners
        .insert(OccurrenceOwnership {
            occurrence: statement.id(),
            ..owner
        })
        .unwrap();
    let use_support = f.data.use_supports.iter().next().unwrap().clone();
    let scope = f.data.use_observations.iter().next().unwrap().scope;
    let region = FlowRegionObservation {
        qualification: f.q.id(),
        statement: statement.id(),
        scope,
    };
    let region_support = FlowRegionSupport {
        assertion: region.id(),
        run: use_support.run,
        surface: use_support.surface,
        evidence: use_support.evidence,
        origin: use_support.origin,
        mode: use_support.mode,
        fidelity: use_support.fidelity,
    };
    f.data.regions.insert(region).unwrap();
    f.data.region_supports.insert(region_support).unwrap();
    let leaf_condition = Diagram::from_atom(f.guard.id());
    let (c, nodes) = leaf_condition.records();
    f.data.conditions.insert(c).unwrap();
    for row in nodes {
        f.data.condition_nodes.insert(row).unwrap();
    }
    let q = AssertionQualification {
        condition: leaf_condition.id(),
        ..f.q.clone()
    };
    f.data.qualifications.insert(q.clone()).unwrap();
    let leaf = FlowTestLeafObservation {
        qualification: q.id(),
        test: f.guard.evaluation,
        atom: f.guard.id(),
        operand: Some(f.request.access),
    };
    let support = FlowTestLeafSupport {
        assertion: leaf.id(),
        run: use_support.run,
        surface: use_support.surface,
        evidence: use_support.evidence,
        origin: use_support.origin,
        mode: use_support.mode,
        fidelity: use_support.fidelity,
    };
    f.data.leaves.insert(leaf.clone()).unwrap();
    f.data.leaf_supports.insert(support.clone()).unwrap();
    let source = EntryAccessSource::guard(&f.data, f.request, leaf.id(), support.id()).unwrap();
    let entry = EntryValueWitness::derive_for(&f.data, f.request, &source, &f.budget)
        .unwrap()
        .unwrap();
    assert_eq!(entry.condition().id(), Diagram::always().id());
    StabilityWitness::derive(&f.data, f.guard.id(), &entry).unwrap();
    let old = f.data.use_observations.iter().next().unwrap().clone();
    let changed = FlowUseObservation {
        qualification: q.id(),
        ..old
    };
    let mut use_support = f.data.use_supports.iter().next().unwrap().clone();
    use_support.assertion = changed.id();
    f.data.use_observations = Rows::new(&f.budget);
    f.data.use_observations.insert(changed).unwrap();
    f.data.use_supports = Rows::new(&f.budget);
    f.data.use_supports.insert(use_support).unwrap();
    let row = FlowReachingObservation {
        qualification: q.id(),
        ..f.reaching.clone()
    };
    replace_reaching(&mut f, row);
    f.seed_inventory();
    let use_entry = f.derive().unwrap();
    assert!(
        StabilityWitness::derive(&f.data, f.guard.id(), &use_entry).is_err(),
        "narrow Use entry cannot certify guard stability"
    );
    assert!(
        EntryValueWitness::derive_for(&f.data, f.request, &source, &f.budget)
            .unwrap()
            .is_err(),
        "positive leaf formula cannot narrow an entry-identity proof"
    );
}


#[test]
fn partial_coverage_is_evidence_while_exact_false_singleton_uses_its_inventory() {
    let mut f=Fixture::new();
    let mut coverage=f.coverage.clone();coverage.status=CoverageStatus::Partial;coverage.reason=Some(ObligationKind::IncompleteCoverage);
    f.data.coverage=Rows::new(&f.budget);f.data.coverage.insert(coverage).unwrap();
    let proof=f.derive().expect("complete finite per-use inventory survives Partial family coverage");
    f.validate(proof.witness()).unwrap();drop(proof);
    let (condition,nodes)=Diagram::never().records();f.data.conditions.insert(condition.clone()).unwrap();for n in nodes{f.data.condition_nodes.insert(n).unwrap();}
    let q=AssertionQualification{condition:condition.id(),..f.q.clone()};f.data.qualifications.insert(q.clone()).unwrap();
    let observation=FlowUseObservation{qualification:q.id(),..f.data.use_observations.iter().next().unwrap().clone()};
    let mut support=f.data.use_supports.iter().next().unwrap().clone();support.assertion=observation.id();
    f.data.use_observations=Rows::new(&f.budget);f.data.use_observations.insert(observation).unwrap();f.data.use_supports=Rows::new(&f.budget);f.data.use_supports.insert(support).unwrap();
    let reaching=FlowReachingObservation{qualification:q.id(),..f.reaching.clone()};replace_reaching(&mut f,reaching);
    f.seed_inventory();
    let proof=f.derive().expect("exact false complete singleton retains its parameter-entry proof");
    assert_eq!(proof.condition().id(),Diagram::never().id());f.validate(proof.witness()).unwrap();drop(proof);
    f.data.inventories=Rows::new(&f.budget);
    assert!(matches!(f.derive(),Err(ObligationKind::EntryValueUnknown)),"Partial family coverage cannot replace missing per-use closure");
}

#[test]
fn singleton_inventory_refuses_a_forged_original_view_snapshot() {
    for changed_length in [false,true] {
        let mut f=Fixture::new();
        let proof=f.derive().unwrap();let witness=proof.witness().clone();drop(proof);
        let mut inventory=f.data.inventories.get(witness.inventory).unwrap().clone();
        let mut view=f.data.source_views.get(inventory.view).unwrap().clone();
        if changed_length {view.byte_len+=1;}else{view.original_content=ContentHash::of(b"forged original source");view.view_content=view.original_content;}
        let mut view_support=f.data.source_view_supports.iter().next().unwrap().clone();view_support.assertion=view.id();
        inventory.view=view.id();
        let mut inventory_support=f.data.inventory_supports.get(witness.inventory_support).unwrap().clone();inventory_support.assertion=inventory.id();
        let mut candidate=f.data.inventory_candidates.iter().next().unwrap().clone();candidate.inventory=inventory.id();
        let mut member=f.data.inventory_members.iter().next().unwrap().clone();member.inventory=inventory.id();
        f.data.source_views=Rows::new(&f.budget);f.data.source_views.insert(view).unwrap();
        f.data.source_view_supports=Rows::new(&f.budget);f.data.source_view_supports.insert(view_support).unwrap();
        f.data.inventories=Rows::new(&f.budget);f.data.inventories.insert(inventory).unwrap();
        f.data.inventory_supports=Rows::new(&f.budget);f.data.inventory_supports.insert(inventory_support).unwrap();
        f.data.inventory_candidates=Rows::new(&f.budget);f.data.inventory_candidates.insert(candidate).unwrap();
        f.data.inventory_members=Rows::new(&f.budget);f.data.inventory_members.insert(member).unwrap();
        assert!(matches!(f.derive(),Err(ObligationKind::EntryValueUnknown)),"nominal matching view source must retain actual original digest and length");
    }
}

#[test]
fn raw_singleton_selector_checks_source_qualifications_and_bound_target() {
    use lctx_model::domain::flow_inventory::*;
    fn selected(f:&Fixture)->bool {
        let observation=f.data.use_observations.iter().next().unwrap();
        let support=f.data.use_supports.iter().find(|s|s.assertion==observation.id()).unwrap();
        let reaching=f.data.reaching.iter().next().unwrap();
        let rs=f.data.reaching_supports.iter().find(|s|s.assertion==reaching.id()).unwrap();
        complete_native_singleton(&f.data,observation,support,reaching,rs,&f.budget).unwrap().is_some()
    }
    assert!(selected(&Fixture::new()));
    for mutation in 0..4 {
        let mut f=Fixture::new();
        if mutation<2 {
            let mut q=f.q.clone();
            if mutation==0 {q.modality=Modality::Candidate;}else{let foreign=source::SourceArtifact::from_bytes(f.data.artifacts.iter().next().unwrap().input,"foreign.py".into(),b"foreign").unwrap();let scope=source::CoverageScope::Artifact{artifact:foreign.id()};f.data.artifacts.insert(foreign).unwrap();f.data.scopes.insert(scope.clone()).unwrap();q.scope=scope.id();}
            f.data.qualifications.insert(q.clone()).unwrap();
            let observation=FlowUseObservation{qualification:q.id(),..f.data.use_observations.iter().next().unwrap().clone()};
            let mut support=f.data.use_supports.iter().next().unwrap().clone();support.assertion=observation.id();
            f.data.use_observations=Rows::new(&f.budget);f.data.use_observations.insert(observation).unwrap();f.data.use_supports=Rows::new(&f.budget);f.data.use_supports.insert(support).unwrap();
            let reaching=FlowReachingObservation{qualification:q.id(),..f.reaching.clone()};replace_reaching(&mut f,reaching);f.seed_inventory();
        }else if mutation==2 {
            let q=AssertionQualification{approximation:assertion::Approximation::Over,..f.q.clone()};f.data.qualifications.insert(q.clone()).unwrap();
            let mut view=f.data.source_views.iter().next().unwrap().clone();view.qualification=q.id();
            let mut vs=f.data.source_view_supports.iter().next().unwrap().clone();vs.assertion=view.id();
            let mut inv=f.data.inventories.iter().next().unwrap().clone();inv.view=view.id();let mut is=f.data.inventory_supports.iter().next().unwrap().clone();is.assertion=inv.id();
            let mut c=f.data.inventory_candidates.iter().next().unwrap().clone();c.inventory=inv.id();let mut m=f.data.inventory_members.iter().next().unwrap().clone();m.inventory=inv.id();
            f.data.source_views=Rows::new(&f.budget);f.data.source_views.insert(view).unwrap();f.data.source_view_supports=Rows::new(&f.budget);f.data.source_view_supports.insert(vs).unwrap();
            f.data.inventories=Rows::new(&f.budget);f.data.inventories.insert(inv).unwrap();f.data.inventory_supports=Rows::new(&f.budget);f.data.inventory_supports.insert(is).unwrap();f.data.inventory_candidates=Rows::new(&f.budget);f.data.inventory_candidates.insert(c).unwrap();f.data.inventory_members=Rows::new(&f.budget);f.data.inventory_members.insert(m).unwrap();
        }else {
            f.data.targets.insert(ReachingDefinition::Unbound).unwrap();let row=FlowReachingObservation{target:ReachingDefinition::Unbound.id(),..f.reaching.clone()};replace_reaching(&mut f,row);
            let old=f.data.inventories.iter().next().unwrap().clone();let reach=f.data.reaching.iter().next().unwrap();let rs=f.data.reaching_supports.iter().next().unwrap();let state=f.data.inventory_candidates.iter().next().unwrap().state();
            let (inv,candidates,members)=FlowUseInventoryObservation::new(old.qualification,old.use_,old.scope,old.view,&[state],&[(0,reach.id(),rs.id())]).unwrap();let mut support=f.data.inventory_supports.iter().next().unwrap().clone();support.assertion=inv.id();
            f.data.inventories=Rows::new(&f.budget);f.data.inventories.insert(inv).unwrap();f.data.inventory_supports=Rows::new(&f.budget);f.data.inventory_supports.insert(support).unwrap();f.data.inventory_candidates=Rows::new(&f.budget);for c in candidates{f.data.inventory_candidates.insert(c).unwrap();}f.data.inventory_members=Rows::new(&f.budget);for m in members{f.data.inventory_members.insert(m).unwrap();}
        }
        assert!(!selected(&f),"raw selector must independently reject qualification/scope/target mutation {mutation}");
    }
}
