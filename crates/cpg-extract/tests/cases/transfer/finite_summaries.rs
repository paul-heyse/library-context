//! Actual captured native/normalized/Local inputs drive the finite summary kernel.
use crate::fixture;
use lctx_model::domain::{
    analysis,
    execution::{self, summary_production::*},
    projection::{
        self,
        normalization::{ProjectionData, ProjectionKey},
        snapshot::MaterializedGraph,
    },
    stages::{Profile, PublicationBoundary},
    *,
};
#[tokio::test]
async fn native_call_paths_reach_caller_returns_and_keep_recursive_boundaries() {
    let f = fixture::native_from("phase4_summaries").await;
    let budget = &f.budget;
    let input = f.rows::<input::InputRevision>()[0].id();
    let context = f.data.event_events.iter().next().unwrap().context;
    let mut local = local_semantics::LocalData::new(budget);
    let mut summary = SummaryData::new(budget);
    let mut projection = ProjectionData::new(budget);
    let mut inventory = analysis::native::NativeInventory::new(budget);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        local.visit(name, batch).unwrap();
        projection.visit(name, batch).unwrap();
        if analysis::native::NativeInventory::inputs()
            .iter()
            .any(|i| i.name() == *name)
        {
            inventory.visit(name, batch).unwrap();
        }
    }
    macro_rules! load {
        ($ty:ty,$rows:expr) => {{
            let batch = <$ty as Record>::encode(($rows).as_ref()).unwrap();
            local.visit(<$ty>::NAME, &batch).unwrap();
            projection.visit(<$ty>::NAME, &batch).unwrap();
            let i = ValidationInput::of::<$ty>(&["id"]);
            if stages::is_vocabulary(i.name()) {
                summary
                    .visit_input(&i.clone().at_epoch(PublicationBoundary::Facts), &batch)
                    .unwrap();
                summary
                    .visit_input(&i.at_epoch(PublicationBoundary::Model), &batch)
                    .unwrap();
            } else {
                summary.visit_input(&i, &batch).unwrap();
            }
        }};
    }
    macro_rules! facts{($($field:ident:$ty:ty,)*)=>{$(load!($ty,f.data.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_inputs!(facts);
    macro_rules! output{($($field:ident:$ty:ty,)*)=>{$(load!($ty,f.output.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_outputs!(output);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        for i in SummaryData::inputs().iter().filter(|i| i.name() == *name) {
            summary.visit_input(i, batch).unwrap();
        }
    }
    let inventory = inventory.collect().unwrap();
    load!(
        analysis::native::NativeQualification,
        inventory.qualifications.iter().cloned().collect::<Vec<_>>()
    );
    load!(
        analysis::native::NativeAssertionPremise,
        inventory.premises.iter().cloned().collect::<Vec<_>>()
    );
    let (_, definition) = local_semantics::definition();
    let (invocation, _) =
        analysis::local::AnalysisInvocation::new(input, context, definition.id(), None, []);
    load!(
        analysis::local::AnalysisInvocation,
        std::slice::from_ref(&invocation)
    );
    load!(
        analysis::AnalysisDefinition,
        std::slice::from_ref(&definition)
    );
    let output = local_semantics::produce(&local, &invocation, &definition, budget).unwrap();
    load!(
        analysis::local::AnalysisOutcome,
        [analysis::local::AnalysisOutcome {
            invocation: invocation.id(),
            status: analysis::AnalysisStatus::Partial,
            reason: Some(obligation::ObligationKind::IncompleteDomain)
        }]
    );
    macro_rules! local_rows{($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap();let i=ValidationInput::of::<$ty>(&["id"]);summary.visit_input(&if stages::is_vocabulary(i.name()){i.at_epoch(PublicationBoundary::Model)}else{i},&batch).unwrap();)*};}
    lctx_model::local_semantic_outputs!(local_rows);
    let key = ProjectionKey {
        input,
        context,
        name: projection::ProjectionName::CallableInvocation,
    };
    let graph = MaterializedGraph::build(
        &projection::normalization::describe(&projection, key, budget).unwrap(),
        budget,
    )
    .unwrap();
    let catalog = models::Catalog::committed().unwrap();
    for depth in [0, 1, 2] {
        let (p, d) = execution::configuration::summaries(
            catalog.declaration().id(),
            execution::configuration::SummaryLimits {
                depth,
                ..Default::default()
            },
        )
        .unwrap();
        summary.parameters.insert(p).unwrap();
        let (invocation, _) =
            analysis::summary::AnalysisInvocation::new(input, context, d.id(), None, []);
        let output = produce(
            &summary,
            &invocation,
            &d,
            Profile::Behavioral,
            Some(&graph),
            budget,
        )
        .unwrap();
        eprintln!(
            "summary depth={depth} Local={} calls={} witnesses={} paths={} residuals={:?}",
            summary.local_contributions.len(),
            output.call_members.len(),
            output.witnesses.len(),
            output.path_witnesses.len(),
            output
                .residuals
                .iter()
                .map(|r| r.reason)
                .collect::<Vec<_>>()
        );
        if depth > 0 {
            assert!(!output.witnesses.is_empty());
            assert!(
                !output.path_witnesses.is_empty(),
                "proven call result must reach its source return"
            );
        }
        if depth > 0 {
            assert!(
                output
                    .path_routes
                    .iter()
                    .any(|r| matches!(r, execution::summary_path::SummaryPathRoute::Alias { .. })),
                "caller-held local alias must preserve the proven value"
            );
            for name in ["relay", "nested", "aliases"] {
                let declarations = summary
                    .entry
                    .symbol_declarations
                    .iter()
                    .filter(|d| {
                        summary
                            .entry
                            .symbols
                            .get(d.symbol)
                            .is_some_and(|s| s.name == name)
                    })
                    .map(|d| d.declaration)
                    .collect::<std::collections::BTreeSet<_>>();
                assert!(
                    !declarations.is_empty(),
                    "native declaration missing: {name}"
                );
                assert!(output.alternatives.iter().any(|a|{let key=output.keys.get(a.transfer).unwrap();let place=output.vocabulary.places.get(&key.output).or_else(||summary.vocabulary.places.get(&key.output)).unwrap();matches!(output.vocabulary.roots.get(&place.root).or_else(||summary.vocabulary.roots.get(&place.root)),Some(value::PlaceRoot::Return{callable})if declarations.contains(callable))}),"finite exact call result must reach {name}'s Return");
            }
        }

        for name in ["opaque", "opaque_two"] {
            let owners = summary
                .entry
                .symbol_declarations
                .iter()
                .filter(|d| {
                    summary
                        .entry
                        .symbols
                        .get(d.symbol)
                        .is_some_and(|s| s.name == name)
                })
                .map(|d| d.declaration)
                .collect::<std::collections::BTreeSet<_>>();
            let mut native_depths = std::collections::BTreeSet::new();
            let mut longest_guard = 0;
            for cost in output.costs.iter() {
                let (key, q) = match output.premises.get(cost.proof).unwrap() {
                    transfer::summary::SummaryPremise::Witness { witness } => {
                        let w = output.witnesses.get(*witness).unwrap();
                        (w.transfer, w.qualification)
                    }
                    transfer::summary::SummaryPremise::Path { witness } => {
                        let w = output.path_witnesses.get(*witness).unwrap();
                        (w.transfer, w.qualification)
                    }
                    _ => unreachable!(),
                };
                let key = output.keys.get(key).unwrap();
                let normalized::entities::EntityRef::Callable { callable } =
                    summary.entry.refs.get(key.owner).unwrap()
                else {
                    continue;
                };
                let normalized::entities::CallableEntity::Source { declaration, .. } =
                    summary.entry.callables.get(*callable).unwrap()
                else {
                    continue;
                };
                if !owners.contains(declaration) {
                    continue;
                }
                native_depths.insert(cost.depth);
                let q = output
                    .vocabulary
                    .qualifications
                    .get(&q)
                    .or_else(|| summary.vocabulary.qualifications.get(&q))
                    .unwrap();
                let condition = output
                    .vocabulary
                    .conditions
                    .get(&q.condition)
                    .or_else(|| summary.vocabulary.conditions.get(&q.condition))
                    .unwrap();
                let nodes = output
                    .vocabulary
                    .nodes
                    .values()
                    .chain(summary.vocabulary.nodes.values())
                    .map(|n| (n.id(), n.clone()))
                    .collect::<std::collections::BTreeMap<_, _>>()
                    .into_values()
                    .collect::<Vec<_>>();
                let diagram = conditions::Diagram::from_records(condition, &nodes).unwrap();
                for atom in diagram.support() {
                    let mut at = *atom;
                    let mut length = 0;
                    loop {
                        let atom = output
                            .vocabulary
                            .atoms
                            .get(&at)
                            .or_else(|| summary.vocabulary.atoms.get(&at))
                            .unwrap();
                        let predicate = output
                            .vocabulary
                            .predicates
                            .get(&atom.predicate)
                            .or_else(|| summary.vocabulary.predicates.get(&atom.predicate))
                            .unwrap();
                        match predicate {
                            value::Predicate::InvokedGuard { source } => {
                                assert!(atom.operand.is_none());
                                at = *source;
                                length += 1;
                            }
                            value::Predicate::BoundGuard { source } => {
                                at = *source;
                                length += 1;
                            }
                            _ => break,
                        }
                        assert!(length < 32);
                    }
                    longest_guard = longest_guard.max(length);
                }
            }
            eprintln!(
                "summary {name}: finite depths={native_depths:?}, exact guard lineage={longest_guard}"
            );
            if depth > 0 {
                assert!(
                    native_depths.contains(&i64::from(depth)),
                    "the same recursive event must retain {depth} finite laps for {name}"
                );
                assert!(
                    longest_guard >= depth,
                    "distinct invoked opaque guards cannot be collapsed at a repeated event"
                );
            }
        }
        let returns = |name: &str| {
            let declarations = summary
                .entry
                .symbol_declarations
                .iter()
                .filter(|d| {
                    summary
                        .entry
                        .symbols
                        .get(d.symbol)
                        .is_some_and(|s| s.name == name)
                })
                .map(|d| d.declaration)
                .collect::<std::collections::BTreeSet<_>>();
            output.alternatives.iter().filter(|a|{let key=output.keys.get(a.transfer).unwrap();let p=output.vocabulary.places.get(&key.output).or_else(||summary.vocabulary.places.get(&key.output)).unwrap();matches!(output.vocabulary.roots.get(&p.root).or_else(||summary.vocabulary.roots.get(&p.root)),Some(value::PlaceRoot::Return{callable})if declarations.contains(callable))}).count()
        };
        assert_eq!(
            returns("no_base"),
            0,
            "a recursive cycle without a base witness cannot manufacture a return"
        );
        assert_eq!(
            returns("rebound_alias"),
            0,
            "a later assignment cannot inherit the earlier proven RHS value"
        );
        if depth == 2 {
            assert!(
                output.costs.iter().any(|c| c.depth == 2),
                "actual native recursive/compound paths retain a second finite call layer"
            );
        }
        let invariant = lctx_model::domain::validation::invariants_for::<
            execution::summary_proof::SummaryProofCost,
        >()[0]
            .clone();
        for forged in [false, true] {
            if forged && output.costs.is_empty() {
                continue;
            }
            let mut check = (invariant.create)(budget);
            macro_rules! emit {
                ($t:ty,$rows:expr) => {
                    check
                        .visit(
                            <$t>::NAME,
                            &<$t as Record>::encode(($rows).as_ref()).unwrap(),
                        )
                        .unwrap();
                };
            }
            let mut costs = output.costs.iter().cloned().collect::<Vec<_>>();
            if forged {
                costs[0].rank += 1;
            }
            emit!(execution::summary_proof::SummaryProofCost, costs);
            emit!(
                transfer::summary::SummaryPremise,
                output.premises.iter().cloned().collect::<Vec<_>>()
            );
            emit!(
                transfer::summary::SummaryWitness,
                output.witnesses.iter().cloned().collect::<Vec<_>>()
            );
            emit!(
                execution::summary_path::SummaryPathWitness,
                output.path_witnesses.iter().cloned().collect::<Vec<_>>()
            );
            emit!(
                analysis::summary::AnalysisInvocation,
                std::slice::from_ref(&invocation)
            );
            emit!(analysis::AnalysisDefinition, std::slice::from_ref(&d));
            emit!(
                analysis::MethodParameters,
                summary.parameters.iter().cloned().collect::<Vec<_>>()
            );
            let result = check.finish();
            assert_eq!(
                result.is_ok(),
                !forged,
                "finite proof cost replay: {result:?}"
            );
        }
        assert!(output.costs.iter().all(|c| c.depth <= i64::from(depth)));
        assert!(
            output.call_members.iter().any(|m| m.reason.is_some()),
            "unresolved siblings remain explicit"
        );
    }
}
