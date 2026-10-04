//! Native coordinates through the typed facts writer and shared invariants.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{
    assertion::*, attribution::*, conditions::*, flow::*, source::*, value::*, *,
};
use std::collections::{BTreeMap, BTreeSet};
use typed_driver::{Tables, rows};
inspector!(
    Flow,
    FlowUse,
    lctx_model::domain::flow_inventory::FlowUseInventoryObservation,
    lctx_model::domain::flow_inventory::FlowUseCandidate,
    lctx_model::domain::flow_inventory::FlowUseInventoryMember,
    FlowDefinition,
    FlowDefinitionObservation,
    FlowReachingObservation,
    FlowNarrowingObservation,
    FlowSourceViewObservation,
    lctx_model::domain::flow_capture::FlowCaptureTimingObservation,
    lctx_model::domain::flow_capture::FlowCaptureInventory,
    lctx_model::domain::flow_capture::FlowCaptureCandidate,
    lctx_model::domain::flow_capture::FlowCaptureTarget,
    FlowValueObservation,
    FlowCallPath,
    FlowCallStep,
    EvaluationAtom,
    ProviderCoverage,
    SourceArtifact
);
#[tokio::test]
async fn native_narrowing_and_runtime_source_view_keep_distinct_qualified_meaning() {
    let source = "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    imported = 1\nelse:\n    imported = 2\ndef narrow(x: int | None):\n    if x is None:\n        return 0\n    return x\n";
    let input = BTreeMap::from([("example.py".into(), source.as_bytes().to_vec())]);
    let tables = Tables::default();
    typed_driver::run_behavioral(&input, Flow(tables.clone()))
        .await
        .unwrap();
    let views = rows::<FlowSourceViewObservation>(&tables);
    assert_eq!(views.len(), 1);
    let original = rows::<SourceArtifact>(&tables)
        .into_iter()
        .find(|row| row.path == "example.py")
        .unwrap();
    let (runtime, count) = cpg_flow::rename(source).unwrap();
    assert_eq!(count, 2);
    assert_eq!(views[0].source, original.id());
    assert_eq!(
        views[0].original_content,
        ContentHash::of(source.as_bytes())
    );
    assert_eq!(views[0].view_content, ContentHash::of(runtime.as_bytes()));
    assert_ne!(views[0].view_content, views[0].original_content);
    assert_eq!(views[0].byte_len, source.len() as i64);
    assert_eq!(views[0].renamed_type_checking, 2);
    let occurrences = rows::<Occurrence>(&tables);
    let x = occurrences
        .iter()
        .find(|row| {
            row.start == source.rfind('x').unwrap() as i64
                && row.syntax_kind == SyntaxKind::ExprName
        })
        .unwrap();
    let use_ = rows::<FlowUse>(&tables)
        .into_iter()
        .find(|row| row.occurrence == x.id())
        .unwrap();
    let narrowing = rows::<FlowNarrowingObservation>(&tables);
    let selected: Vec<_> = narrowing
        .iter()
        .filter(|row| row.use_ == use_.id())
        .collect();
    assert!(!selected.is_empty(), "actual native parameter candidate");
    let qualifications = rows::<AssertionQualification>(&tables);
    let always = Diagram::always().records().0.id();
    assert!(selected.iter().any(
        |row| qualifications.iter().any(|q| q.id() == row.qualification
            && q.condition != always
            && q.approximation == Approximation::Exact)
    ));
    assert!(selected.iter().all(|row| !row.precision_lost));
    let predicates = rows::<Predicate>(&tables);
    assert!(
        predicates
            .iter()
            .any(|row| matches!(row, Predicate::IsNone))
    );
    let support = rows::<FlowNarrowingSupport>(&tables);
    assert!(
        selected
            .iter()
            .all(|row| support.iter().any(|support| support.assertion == row.id()))
    );
}
#[tokio::test]
async fn reaching_places_keep_the_binding_owner_across_scope_and_member_reads() {
    let cases = [
        "x = 1\nx = 2\ndef read():\n    return x\n",
        "def outer(flag):\n    x = 1\n    if flag:\n        x = 2\n    def read():\n        return x\n    return read\n",
        "class C:\n    def f(self):\n        self.x = 1\n        return self.x\n",
        "def f(x):\n    x.a = 1\n    return x.a\n",
        "def f(x):\n    x[0] = 1\n    return x[0]\n",
        "def f(items):\n    return [path for item in items if (path := item)]\n",
        "def f(path, items):\n    return [path for item in items if (path := item)]\n",
        "def f(items):\n    return [[path for item in group if (path := item)] for group in items]\n",
        "items = [1]\npaths = [path for item in items if (path := item)]\n",
    ];
    for (position, source) in cases.iter().enumerate() {
        let input = BTreeMap::from([("example.py".into(), source.as_bytes().to_vec())]);
        let tables = Tables::default();
        typed_driver::run_behavioral(&input, Flow(tables.clone()))
            .await
            .unwrap_or_else(|error| panic!("case {position}: {error}"));
        if position >= 5 {
            let occurrences = rows::<Occurrence>(&tables);
            let scopes = rows::<lctx_model::domain::lexical::LexicalScope>(&tables);
            let places = rows::<Place>(&tables);
            let roots = rows::<PlaceRoot>(&tables);
            let targets: BTreeSet<_> =
                rows::<lctx_model::domain::lexical::BindingObservation>(&tables)
                    .iter()
                    .filter(|binding| {
                        binding.kind == lctx_model::domain::lexical::BindingEventKind::Walrus
                    })
                    .map(|binding| {
                        rows::<lctx_model::domain::lexical::BindingEvent>(&tables)
                            .into_iter()
                            .find(|event| event.id() == binding.event)
                            .unwrap()
                            .site
                    })
                    .collect();
            let definitions = rows::<FlowDefinition>(&tables);
            assert!(
                definitions
                    .iter()
                    .any(|definition| targets.contains(&definition.occurrence))
            );
            for definition in definitions
                .iter()
                .filter(|definition| targets.contains(&definition.occurrence))
            {
                let place = places
                    .iter()
                    .find(|place| place.id() == definition.place)
                    .unwrap();
                let root = roots.iter().find(|root| root.id() == place.root).unwrap();
                match root {
                    PlaceRoot::Local { scope, name } => {
                        assert_eq!(name, "path");
                        assert!(scopes.iter().any(|owner| owner.owner == *scope
                            && owner.kind
                                != lctx_model::domain::lexical::LexicalScopeKind::Comprehension));
                    }
                    PlaceRoot::Formal { declaration } => {
                        assert_eq!(position, 6);
                        assert_eq!(
                            occurrences
                                .iter()
                                .find(|site| site.id() == *declaration)
                                .unwrap()
                                .start,
                            6
                        );
                    }
                    root => panic!("unexpected walrus root {root:?}"),
                }
            }
        }
    }
}
#[tokio::test]
async fn nested_call_paths_survive_exact_attachment_and_shared_validation() {
    let tables = Tables::default();
    let input = typed_driver::files("flow_call_paths");
    typed_driver::run_behavioral(&input, Flow(tables.clone()))
        .await
        .unwrap();
    let uses = rows::<FlowUse>(&tables);
    let values = rows::<FlowValueObservation>(&tables);
    let steps = rows::<FlowCallStep>(&tables);
    assert!(!uses.is_empty());
    assert!(values.iter().any(|v| v.through_call));
    let mut counts: BTreeMap<_, usize> = BTreeMap::new();
    for step in &steps {
        *counts.entry(step.path).or_default() += 1;
    }
    assert!(counts.values().any(|n| *n == 2));
    assert!(
        rows::<FlowValuePathObservation>(&tables)
            .iter()
            .all(|link| counts.contains_key(&link.path))
    );
    assert!(
        rows::<ProviderCoverage>(&tables)
            .iter()
            .any(|c| c.family == FactFamily::Flow
                && c.status == CoverageStatus::CompleteUnderStatedModel)
    );
}
#[tokio::test]
async fn repeated_predicates_keep_distinct_atoms_and_nonlocal_markers_stay_nested() {
    let input=BTreeMap::from([("example.py".into(),b"def f(x):\n    if x is None:\n        x = 1\n    def change():\n        nonlocal x\n        x = 2\n    change()\n    if x is None:\n        return 3\n    return x\n".to_vec())]);
    let tables = Tables::default();
    typed_driver::run_behavioral(&input, Flow(tables.clone()))
        .await
        .unwrap();
    let atoms = rows::<EvaluationAtom>(&tables);
    let predicates = rows::<Predicate>(&tables);
    let none = predicates
        .iter()
        .find(|p| matches!(p, Predicate::IsNone))
        .unwrap()
        .id();
    let evaluations: BTreeSet<_> = atoms
        .iter()
        .filter(|a| a.predicate == none)
        .map(|a| a.evaluation)
        .collect();
    assert_eq!(evaluations.len(), 2);
    let leaves = rows::<FlowTestLeafObservation>(&tables);
    assert_eq!(
        leaves
            .iter()
            .filter(|l| atoms
                .iter()
                .any(|a| a.id() == l.atom && a.predicate == none))
            .count(),
        2
    );
    assert!(
        rows::<ReachingDefinition>(&tables)
            .iter()
            .any(|r| matches!(r, ReachingDefinition::Nested))
    );
    assert!(
        rows::<AssertionQualification>(&tables)
            .iter()
            .all(|q| q.approximation == Approximation::Exact
                || q.approximation == Approximation::Unknown)
    );
    let again = Tables::default();
    let digest = typed_driver::run_behavioral(&input, Flow(again.clone()))
        .await
        .unwrap();
    let first = Tables::default();
    assert_eq!(
        digest,
        typed_driver::run_behavioral(&input, Flow(first.clone()))
            .await
            .unwrap()
    );
    assert_eq!(
        rows::<EvaluationAtom>(&first),
        rows::<EvaluationAtom>(&again)
    );
}
#[tokio::test]
async fn runtime_specials_require_the_resolved_import_binding() {
    let mut input=BTreeMap::from([("example.py".into(),b"from typing import TYPE_CHECKING as TC\nimport sys as system\nif TC:\n    checker = 1\nelse:\n    runtime = 2\ndef f(TC):\n    if TC:\n        return 3\n    return 4\nif system.platform == 'linux':\n    linux = 1\n".to_vec())]);
    let example = input["example.py"].clone();
    for n in 0..16 {
        input.insert(format!("unrelated{n}.py"), example.clone());
    }

    let tables = Tables::default();
    typed_driver::run_behavioral(&input, Flow(tables.clone()))
        .await
        .unwrap();
    let occurrences = rows::<Occurrence>(&tables);
    let regions = rows::<FlowRegionObservation>(&tables);
    let qs = rows::<AssertionQualification>(&tables);
    let conditions = rows::<Condition>(&tables);
    let nodes = rows::<ConditionNode>(&tables);
    let bytes = &input["example.py"];
    let sources = rows::<SourceArtifact>(&tables);
    let condition = |path: &str, text: &[u8]| {
        let source = sources.iter().find(|s| s.path == path).unwrap().id();
        let start = bytes.windows(text.len()).position(|s| s == text).unwrap() as i64;
        let region = regions
            .iter()
            .find(|r| {
                occurrences
                    .iter()
                    .any(|o| o.id() == r.statement && o.source == source && o.start == start)
            })
            .unwrap();
        let q = qs.iter().find(|q| q.id() == region.qualification).unwrap();
        Diagram::from_records(
            conditions.iter().find(|c| c.id() == q.condition).unwrap(),
            &nodes,
        )
        .unwrap()
    };
    for path in input.keys() {
        assert!(condition(path, b"runtime = 2").is_true());
        assert!(!condition(path, b"return 3").is_true());
        assert!(condition(path, b"linux = 1").is_true());
        assert!(condition(path, b"checker = 1").is_false());
    }
}

#[tokio::test]
async fn retired_runtime_resolution_answers_keep_typed_operand_identity() {
    let input = typed_driver::files("type_guard");
    let tables = Tables::default();
    typed_driver::run_behavioral(&input, Flow(tables.clone()))
        .await
        .unwrap();
    let predicates = rows::<Predicate>(&tables);
    let atoms = rows::<EvaluationAtom>(&tables);
    let leaves = rows::<FlowTestLeafObservation>(&tables);
    let type_is = predicates
        .iter()
        .filter(|p| matches!(p,Predicate::TypeIs {class_expression} if class_expression=="str"))
        .map(Record::id)
        .collect::<BTreeSet<_>>();
    let exact = atoms
        .iter()
        .filter(|a| type_is.contains(&a.predicate))
        .map(Record::id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        leaves.iter().filter(|l| exact.contains(&l.atom)).count(),
        4,
        "unshadowed builtin guards are exact"
    );
    let opaque = predicates
        .iter()
        .filter(|p| matches!(p, Predicate::Opaque { .. }))
        .map(Record::id)
        .collect::<BTreeSet<_>>();
    assert!(
        atoms
            .iter()
            .filter(|a| opaque.contains(&a.predicate))
            .count()
            >= 2,
        "shadowed names remain opaque"
    );
    let source = &input["guardpkg/cases.py"];
    let start = source
        .windows(b"if type(x) is str:".len())
        .position(|b| b == b"if type(x) is str:")
        .unwrap()
        + 8;
    let occurrences = rows::<Occurrence>(&tables);
    assert!(
        leaves
            .iter()
            .filter(|l| exact.contains(&l.atom))
            .any(|l| l.operand.is_some_and(|id| occurrences
                .iter()
                .any(|o| o.id() == id && o.start == start as i64 && o.end == (start + 1) as i64)))
    );
    let input = typed_driver::files("flow_shapes");
    let tables = Tables::default();
    typed_driver::run_behavioral(&input, Flow(tables.clone()))
        .await
        .unwrap();
    let occurrences = rows::<Occurrence>(&tables);
    let regions = rows::<FlowRegionObservation>(&tables);
    let qs = rows::<AssertionQualification>(&tables);
    let conditions = rows::<Condition>(&tables);
    let nodes = rows::<ConditionNode>(&tables);
    let artifacts = rows::<SourceArtifact>(&tables);
    let path = "release/flowpkg/shapes.py";
    let source = &input[path];
    let artifact = artifacts.iter().find(|a| a.path == path).unwrap();
    let condition = |function: &str, statement: &str| {
        let text = std::str::from_utf8(source).unwrap();
        let begin = text.find(function).unwrap();
        let start = begin + text[begin..].find(statement).unwrap();
        let region = regions
            .iter()
            .find(|r| {
                occurrences.iter().any(|o| {
                    o.id() == r.statement && o.source == artifact.id() && o.start == start as i64
                })
            })
            .unwrap();
        let q = qs.iter().find(|q| q.id() == region.qualification).unwrap();
        Diagram::from_records(
            conditions.iter().find(|c| c.id() == q.condition).unwrap(),
            &nodes,
        )
        .unwrap()
    };
    assert!(!condition("def choose", "return \"parameter\"").is_true());
    assert!(!condition("def config_check", "return \"ordinary attribute\"").is_true());
    assert!(condition("def checking_alias", "return \"checker only\"").is_false());
    assert!(condition("def checking_module_alias", "return \"checker only\"").is_false());
    assert!(condition("def version_prefix", "above = True").is_true());
    assert!(condition("def version_prefix", "at_most = True").is_false());
}

#[tokio::test]
async fn native_capture_timing_keeps_candidate_basis_and_actual_outer_definition() {
    use lctx_model::domain::{captures::CaptureTiming, flow_capture::*};
    let source = "def outer(value):\n    def inner():\n        return value\n    return inner()\n";
    let tables = Tables::default();
    typed_driver::run_behavioral(
        &BTreeMap::from([("example.py".into(), source.as_bytes().to_vec())]),
        Flow(tables.clone()),
    )
    .await
    .unwrap();
    let snapshots = rows::<FlowCaptureTimingObservation>(&tables);
    assert_eq!(snapshots.len(), 1);
    let snapshot = &snapshots[0];
    assert_eq!(snapshot.state, FlowSnapshotState::FoundBindings);
    assert_eq!(snapshot.timing, CaptureTiming::LazySnapshot);
    assert_ne!(snapshot.nested_scope, snapshot.enclosing_scope);
    let qualifications = rows::<AssertionQualification>(&tables);
    let q = qualifications
        .iter()
        .find(|q| q.id() == snapshot.qualification)
        .unwrap();
    assert_eq!(q.modality, Modality::Candidate);
    assert_eq!(q.assumptions, assumptions::AssumptionSet::empty_id());
    let candidates = rows::<FlowCaptureCandidate>(&tables);
    let candidates = candidates
        .iter()
        .filter(|c| Some(c.inventory) == snapshot.inventory)
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1);
    let targets = rows::<FlowCaptureTarget>(&tables);
    let target = targets
        .iter()
        .find(|t| t.id() == candidates[0].target)
        .unwrap();
    let FlowCaptureTarget::Bound { definition } = target else {
        panic!("actual source formal candidate")
    };
    let observed = rows::<FlowDefinitionObservation>(&tables);
    let definition = observed
        .iter()
        .find(|d| d.definition == *definition)
        .unwrap();
    assert_eq!(definition.scope, snapshot.enclosing_scope);
    assert_eq!(definition.kind, lexical::BindingEventKind::Parameter);
    let supports = rows::<FlowCaptureTimingSupport>(&tables);
    assert_eq!(
        supports
            .iter()
            .filter(|s| s.assertion == snapshot.id())
            .count(),
        1
    );
    let catalog = Tables::default();
    typed_driver::run(
        &BTreeMap::from([("example.py".into(), source.as_bytes().to_vec())]),
        Flow(catalog.clone()),
    )
    .await
    .unwrap();
    assert!(
        rows::<FlowCaptureTimingObservation>(&catalog).is_empty(),
        "catalog never requests ty timing"
    );
}

#[tokio::test]
async fn actual_native_inventory_retains_competitors_and_loop_limits() {
    use lctx_model::domain::flow_inventory::*;
    let files = typed_driver::files("guarded_origin_inventory");
    let tables = Tables::default();
    typed_driver::run_behavioral(&files, Flow(tables.clone()))
        .await
        .unwrap();
    let inventories = rows::<FlowUseInventoryObservation>(&tables);
    let candidates = rows::<FlowUseCandidate>(&tables);
    let members = rows::<FlowUseInventoryMember>(&tables);
    assert!(!inventories.is_empty());
    assert!(inventories.iter().any(|i| i.complete));
    assert!(
        inventories.iter().any(|i| !i.complete
            && candidates
                .iter()
                .any(|c| c.inventory == i.id() && c.loop_expanded)),
        "native loop header never certifies enumeration closure"
    );
    assert!(
        inventories
            .iter()
            .any(|i| i.complete && i.native_count >= 2),
        "native complete join retains all alternatives"
    );
    for inventory in &inventories {
        let budget = typed_driver::budget();
        let explanation =
            explain(inventory.use_, inventory, &candidates, &members, &budget).unwrap();
        assert_eq!(explanation.members.len(), inventory.mapped_count as usize);
    }
    let occurrences = rows::<Occurrence>(&tables);
    let uses = rows::<FlowUse>(&tables);
    let targets = rows::<ReachingDefinition>(&tables);
    let definitions = rows::<FlowDefinitionObservation>(&tables);
    let values = rows::<FlowValueObservation>(&tables);
    let regions = rows::<FlowRegionObservation>(&tables);
    let reaches = rows::<FlowReachingObservation>(&tables);
    let qualifications = rows::<AssertionQualification>(&tables);
    let conditions = rows::<Condition>(&tables);
    let nodes = rows::<ConditionNode>(&tables);
    let diagram = |qualification| {
        let q = qualifications
            .iter()
            .find(|q| q.id() == qualification)
            .unwrap();
        Diagram::from_records(
            conditions.iter().find(|c| c.id() == q.condition).unwrap(),
            &nodes,
        )
        .unwrap()
    };
    let mut investigated = 0;
    let mut useful = 0;
    for value in values.iter().filter(|v| {
        v.kind == FlowSinkKind::Return
            && v.transfer == lctx_model::domain::transfer::TransferKind::Identity
            && !v.through_call
    }) {
        let Some(inventory) = inventories
            .iter()
            .find(|i| i.use_ == value.use_ && i.complete && i.mapped_count >= 2)
        else {
            continue;
        };
        let use_ = uses.iter().find(|u| u.id() == value.use_).unwrap();
        let site = occurrences
            .iter()
            .find(|o| o.id() == use_.occurrence)
            .unwrap();
        let Some(region) = regions
            .iter()
            .filter(|r| r.scope == inventory.scope)
            .filter_map(|r| {
                occurrences
                    .iter()
                    .find(|o| o.id() == r.statement)
                    .map(|o| (r, o))
            })
            .filter(|(_, o)| o.source == site.source && o.start <= site.start && o.end >= site.end)
            .min_by_key(|(_, o)| o.end - o.start)
            .map(|(r, _)| r)
        else {
            continue;
        };
        let q = diagram(value.qualification)
            .and(&diagram(region.qualification))
            .unwrap();
        if q.is_false() {
            continue;
        }
        let candidates = reaches
            .iter()
            .filter(|r| r.use_ == value.use_)
            .collect::<Vec<_>>();
        let parameter = |r: &&FlowReachingObservation| {
            targets
                .iter()
                .find(|t| t.id() == r.target)
                .is_some_and(|t| match t {
                    ReachingDefinition::Bound { definition } => definitions.iter().any(|d| {
                        d.definition == *definition
                            && d.kind == lctx_model::domain::lexical::BindingEventKind::Parameter
                    }),
                    _ => false,
                })
        };
        let originals = candidates
            .iter()
            .copied()
            .filter(parameter)
            .collect::<Vec<_>>();
        let competitors = candidates
            .iter()
            .copied()
            .filter(|r| !parameter(r))
            .collect::<Vec<_>>();
        investigated += 1;
        if originals.len() == 1
            && diagram(originals[0].qualification).and(&q).unwrap().id() == q.id()
            && competitors
                .iter()
                .all(|r| diagram(r.qualification).and(&q).unwrap().is_false())
        {
            useful += 1;
        }
    }
    eprintln!(
        "inventories={} candidates={} complete={} incomplete={} guarded_multicandidate_reads={} useful={}",
        inventories.len(),
        candidates.len(),
        inventories.iter().filter(|i| i.complete).count(),
        inventories.iter().filter(|i| !i.complete).count(),
        investigated,
        useful
    );
    assert!(
        investigated >= 3,
        "actual adapter retains representative multi-candidate identity reads"
    );
    assert_eq!(
        useful, 0,
        "no native useful guarded-origin case established by this bounded fixture search"
    );
    let catalog = Tables::default();
    typed_driver::run(&files, Flow(catalog.clone()))
        .await
        .unwrap();
    assert!(
        rows::<FlowUseInventoryObservation>(&catalog).is_empty(),
        "Catalog does not request flow"
    );
}

#[tokio::test]
async fn collapsed_annotation_capture_scopes_keep_an_attributed_boundary_and_ordinary_uses() {
    use lctx_model::domain::{flow_capture::*, obligation::ObligationKind, syntax::SubjectBoundary};
    let tables = Tables::default();
    typed_driver::run_behavioral(
        &typed_driver::files("normalized_relations"),
        Flow(tables.clone()),
    )
    .await
    .unwrap();
    let boundaries = rows::<SubjectBoundary>(&tables);
    let collapsed: Vec<_> = boundaries
        .iter()
        .filter(|b| {
            b.family == FactFamily::Flow
                && b.reason == ObligationKind::ScopeBoundary
                && b.detail.as_deref()
                    == Some("native annotation snapshot scopes collapse to one canonical lexical scope")
        })
        .collect();
    assert!(!collapsed.is_empty(), "actual PEP 695 annotation scope collision");
    let uses = rows::<FlowUse>(&tables);
    let inventories = rows::<lctx_model::domain::flow_inventory::FlowUseInventoryObservation>(&tables);
    for boundary in collapsed {
        let occurrence = boundary.subject.expect("exact annotation read boundary");
        let use_ = uses.iter().find(|u| u.occurrence == occurrence).unwrap();
        assert!(inventories.iter().any(|i| i.use_ == use_.id()), "snapshot scope loss does not erase ordinary per-use enumeration");
    }
    assert!(rows::<FlowCaptureTimingObservation>(&tables)
        .iter()
        .all(|s| s.nested_scope != s.enclosing_scope));
    assert!(rows::<ProviderCoverage>(&tables).iter().any(|c| {
        c.family == FactFamily::Flow && c.status == CoverageStatus::Partial
    }));
}
