#[path = "fixtures/transfer_composition.rs"]
mod fixture;
#[path = "fixtures/source_execution.rs"]
mod source_fixture;
use lctx_model::domain::{analysis, execution, *};
#[tokio::test]
async fn model_inventory_replays_actual_calls_transfers_and_coupled_erasure() {
    let f = fixture::native_from("phase4_models").await;
    let mut source = source_fixture::data(&f);
    let input = f.rows::<input::InputRevision>()[0].id();
    let context = f.data.event_events.iter().next().unwrap().context;
    let mut entries = normalized::Rows::new(&f.budget);
    let mut entry_sources = normalized::Rows::new(&f.budget);
    for observation in source.flow.use_observations.iter() {
        let use_ = source.flow.uses.get(observation.use_).unwrap();
        let Some(owner) = source
            .flow
            .owners
            .iter()
            .find(|owner| owner.occurrence == use_.occurrence)
        else {
            continue;
        };
        let Some(support) = source
            .flow
            .use_supports
            .iter()
            .find(|support| support.assertion == observation.id())
        else {
            continue;
        };
        for formal in source.flow.formals.iter() {
            let request = conditions::entry::EntryRequest {
                owner: owner.entity,
                formal: formal.id(),
                access: use_.occurrence,
                context,
                run: support.run,
            };
            if let Ok(proof) =
                conditions::entry::EntryValueWitness::derive(&source.flow, request, &f.budget)
                    .unwrap()
            {
                entries.insert(proof.witness().clone()).unwrap();
                entry_sources.insert(proof.source().clone()).unwrap();
            }
        }
    }
    let (base, base_rows) = source_fixture::base_rows_with_entries(
        &mut source,
        input,
        context,
        &f.budget,
        &entries,
        &entry_sources,
    );
    let (_, source_definition) = execution::configuration::source_calls();
    source
        .definitions
        .insert(source_definition.clone())
        .unwrap();
    let parent = analysis::source_call::InvocationSource::BaseCompletion {
        invocation: base.id(),
    };
    let (source_invocation, _) = analysis::source_call::AnalysisInvocation::new(
        input,
        context,
        source_definition.id(),
        None,
        [parent.id()],
    );
    let predecessor = execution::source_call_records::prepare_all(
        &source,
        &source_invocation,
        &source_definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    let mut data = execution::enriched_production::EnrichedData::new(&f.budget);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        data.application.visit(name, batch).unwrap();
    }
    macro_rules! binding_facts{($($field:ident:$ty:ty,)*)=>{$(data.application.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(binding_facts);
    data.application
        .premises
        .decode(
            &<analysis::native::NativeAssertionPremise as Record>::encode(
                &source
                    .evaluation
                    .premises
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        )
        .unwrap();
    data.application
        .native
        .decode(
            &analysis::native::NativeQualification::encode(
                &source.evaluation.native.iter().cloned().collect::<Vec<_>>(),
            )
            .unwrap(),
        )
        .unwrap();
    data.source = source;
    data.source_invocations.insert(source_invocation).unwrap();
    data.source_runs.insert(predecessor.run.clone()).unwrap();
    data.source_results
        .insert(predecessor.outcome.clone())
        .unwrap();
    macro_rules! predecessor{($($field:ident:$source:ident,)*)=>{$(for row in predecessor.$source.iter(){data.$field.insert(row.clone()).unwrap();})*};}
    predecessor! {source_headers:headers,source_members:members,source_boundaries:boundaries,source_calls:invocations,source_releases:releases,source_arguments:arguments,source_outcomes:call_outcomes,source_invocation_boundaries:invocation_boundaries,}
    let catalog = models::Catalog::committed().unwrap();
    let (parameters, definition) =
        execution::configuration::enriched_execution(catalog.declaration().id());
    data.parameters.insert(parameters).unwrap();
    data.catalogs.insert(catalog.declaration().clone()).unwrap();
    let parent = analysis::enriched_execution::InvocationSource::SourceCallAnalysis {
        invocation: data.source_invocations.iter().next().unwrap().id(),
    };
    let (invocation, _) = analysis::enriched_execution::AnalysisInvocation::new(
        input,
        context,
        definition.id(),
        None,
        [parent.id()],
    );
    let enriched = execution::enriched_production::enrich_all(
        &data,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();

    let mut batches = f
        .tables
        .lock()
        .unwrap()
        .iter()
        .map(|(name, batch)| (*name, batch.clone()))
        .collect::<Vec<_>>();
    macro_rules! put {
        ($ty:ty,$rows:expr) => {
            batches.push((
                <$ty>::NAME,
                <$ty as Record>::encode(($rows).as_ref()).unwrap(),
            ));
        };
    }
    macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(put!($ty,f.data.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_inputs!(raw);
    macro_rules! bound{($($field:ident:$ty:ty,)*)=>{$(put!($ty,f.output.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_outputs!(bound);
    macro_rules! raw_evaluation{($($field:ident:$ty:ty,)*)=>{$(put!($ty,data.source.evaluation.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::execution_evaluation_inputs!(raw_evaluation);
    macro_rules! raw_entry{($($field:ident:$ty:ty,)*)=>{$(put!($ty,data.source.flow.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::entry_value_inputs!(raw_entry);
    batches.extend(base_rows);
    macro_rules! earlier{($($field:ident:$ty:ty,)*)=>{$(put!($ty,data.$field.iter().cloned().collect::<Vec<_>>());)*};}
    earlier! {source_invocations:analysis::source_call::AnalysisInvocation,source_runs:execution::source_call_records::SourceCallRun,source_headers:execution::source_call_records::SourceCallHeader,source_members:execution::source_call_records::HeaderMember,source_boundaries:execution::source_call_records::SourceCallBoundary,source_results:analysis::source_call::AnalysisOutcome,source_calls:execution::source_call_records::SourceInvocation,source_releases:execution::source_call_records::SourceFrameRelease,source_arguments:execution::source_call_records::SourceFrameArgument,source_outcomes:execution::source_call_records::SourceCallOutcome,source_invocation_boundaries:execution::source_call_records::InvocationBoundary,}
    put!(
        execution::context_binding::ContextEntryBinding,
        enriched
            .context_bindings
            .iter()
            .cloned()
            .collect::<Vec<_>>()
    );
    put!(
        execution::context_binding::BindingSource,
        enriched
            .context_binding_sources
            .iter()
            .cloned()
            .collect::<Vec<_>>()
    );
    put!(
        execution::context_binding::BindingMember,
        enriched
            .context_binding_members
            .iter()
            .cloned()
            .collect::<Vec<_>>()
    );
    put!(
        execution::context_execution::ContextExecution,
        enriched.contexts.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::context_execution::ContextItem,
        enriched.context_items.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::context_execution::ContextSource,
        enriched.context_sources.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::context_execution::ContextMember,
        enriched.context_members.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::enriched_records::ExecutionOutcome,
        enriched.outcomes.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        analysis::enriched_execution::AnalysisInvocation,
        std::slice::from_ref(&invocation)
    );
    put!(
        execution::modeled_call::ModeledCallEvaluation,
        enriched.modeled_calls.iter().cloned().collect::<Vec<_>>()
    );
    put!(
        execution::modeled_call::ModeledCallArgument,
        enriched
            .modeled_arguments
            .iter()
            .cloned()
            .collect::<Vec<_>>()
    );
    put!(
        execution::modeled_call::ModeledCallNative,
        enriched.modeled_native.iter().cloned().collect::<Vec<_>>()
    );
    let (model_parameters, model_definition) =
        execution::configuration::models(catalog.declaration().id());
    put!(
        analysis::AnalysisDefinition,
        data.source
            .definitions
            .iter()
            .cloned()
            .chain([definition.clone(), model_definition.clone()])
            .collect::<Vec<_>>()
    );
    put!(
        analysis::MethodParameters,
        data.parameters
            .iter()
            .cloned()
            .chain([model_parameters])
            .collect::<Vec<_>>()
    );
    put!(
        models::ModelCatalog,
        data.catalogs.iter().cloned().collect::<Vec<_>>()
    );
    let parent = analysis::model::InvocationSource::EnrichedExecution {
        invocation: invocation.id(),
    };
    let (model_invocation, _) = analysis::model::AnalysisInvocation::new(
        input,
        context,
        model_definition.id(),
        None,
        [
            parent.id(),
            analysis::model::InvocationSource::SourceCallAnalysis {
                invocation: data.source_invocations.iter().next().unwrap().id(),
            }
            .id(),
        ],
    );
    put!(
        analysis::model::AnalysisInvocation,
        std::slice::from_ref(&model_invocation)
    );
    let mut model_data = execution::model_production::ModelData::new(&f.budget);
    for (name, batch) in &batches {
        model_data.visit(name, batch).unwrap();
    }
    let output = execution::model_production::apply_all(
        &model_data,
        &model_invocation,
        &model_definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    assert!(!output.applications.is_empty());
    assert!(
        !output.transfer_witnesses.is_empty(),
        "exact actual entry to callsite proof"
    );
    assert_eq!(output.outcome.status, analysis::AnalysisStatus::Partial);
    assert!(
        !output.context_resources.is_empty(),
        "source-owned lifecycle admits actual resources"
    );
    assert!(
        output
            .context_postconditions
            .iter()
            .any(|p| p.phase == execution::model_rules::ActionPhase::Exceptional)
            && output
                .context_postconditions
                .iter()
                .any(|p| p.phase == execution::model_rules::ActionPhase::Finally)
    );
    assert!(
        !output.context_transfers.is_empty(),
        "actual Local Entry to checked WithTarget"
    );
    for witness in output.context_transfers.iter() {
        let binding = model_data.context_bindings.get(witness.binding).unwrap();
        assert!(
            matches!(output.transfer_roots.get(output.transfer_places.get(witness.output).unwrap().root),Some(value::PlaceRoot::Occurrence{occurrence})if *occurrence==binding.target)
        );
        assert_ne!(binding.target, binding.site);
        assert!(model_data.entries.get(witness.entry).is_some());
        assert!(matches!(
            output
                .transfer_roots
                .get(output.transfer_places.get(witness.input).unwrap().root),
            Some(value::PlaceRoot::Entry { .. })
        ));
    }
    for witness in output.transfer_witnesses.iter() {
        let call = data
            .source
            .evaluation
            .occurrences
            .get(
                output
                    .transfer_roots
                    .get(output.transfer_places.get(witness.output).unwrap().root)
                    .and_then(|r| match r {
                        value::PlaceRoot::Occurrence { occurrence } => Some(*occurrence),
                        _ => None,
                    })
                    .expect("modeled returned place is actual callsite"),
            )
            .unwrap();
        assert_eq!(call.syntax_kind, source::SyntaxKind::ExprCall);
        assert_eq!(
            model_data
                .modeled_calls
                .get(witness.call)
                .unwrap()
                .expression,
            call.id()
        );
        assert!(model_data.entries.get(witness.entry).is_some());
        assert!(matches!(
            output
                .transfer_roots
                .get(output.transfer_places.get(witness.input).unwrap().root),
            Some(value::PlaceRoot::Entry { .. })
        ));
    }
    let owner_name = |owner: Id<normalized::entities::EntityRef>| {
        let normalized::entities::EntityRef::Callable { callable } =
            f.data.refs.get(owner).unwrap()
        else {
            panic!("source owner")
        };
        let normalized::entities::CallableEntity::Source { declaration, .. } =
            f.data.callables.get(*callable).unwrap()
        else {
            panic!("source owner")
        };
        let occurrence = f.data.occurrences.get(*declaration).unwrap();
        String::from_utf8(
            f.source_bytes(occurrence.source)[occurrence.start as usize..occurrence.end as usize]
                .to_vec(),
        )
        .unwrap()
    };
    for resource in output.context_resources.iter() {
        let name = owner_name(resource.owner);
        let item = model_data.context_items.get(resource.item).unwrap();
        let exits = output
            .context_postconditions
            .iter()
            .filter(|p| {
                p.resource == resource.id()
                    && p.event == execution::model_protocol::ContextLifecycle::Exit
            })
            .collect::<Vec<_>>();
        assert_eq!(exits.len(), 2);
        assert!(
            exits
                .iter()
                .any(|p| p.phase == execution::model_rules::ActionPhase::Finally)
        );
        assert!(exits.iter().all(|p| p.input == Some(item.exit_input)
            && p.output == Some(item.exit_output)
            && p.suppressed == item.suppressed));
        let before = model_data.context_outcomes.get(item.exit_input).unwrap();
        let after = model_data.context_outcomes.get(item.exit_output).unwrap();
        if name.starts_with("def suppressed_context(")
            || name.starts_with("def finalizing_context(")
        {
            assert!(matches!(
                before,
                execution::enriched_records::ExecutionOutcome::Raise { .. }
            ));
            assert_eq!(
                after,
                &execution::enriched_records::ExecutionOutcome::Normal
            );
            assert!(item.suppressed);
        }
        if name.starts_with("def preserving_context(")
            || name.starts_with("def nonmatching_context(")
        {
            assert!(matches!(
                before,
                execution::enriched_records::ExecutionOutcome::Raise { .. }
            ));
            assert_eq!(before, after);
            assert!(!item.suppressed);
        }
        if name.starts_with("def ordered_context(") {
            assert_eq!(
                after,
                &execution::enriched_records::ExecutionOutcome::Normal
            );
            assert_eq!(item.suppressed, item.ordinal == 1);
            assert_eq!(
                matches!(
                    before,
                    execution::enriched_records::ExecutionOutcome::Raise { .. }
                ),
                item.ordinal == 1
            );
        }
        if name.starts_with("def valued_context(") {
            assert!(output.context_values.iter().any(|v|matches!(v,execution::model_protocol::ContextEntryValue::Actual{resource:r,actual}if *r==resource.id()&&Some(*actual)==item.entry_actual)));
        }
    }
    assert!(
        output
            .context_resources
            .iter()
            .any(|r| owner_name(r.owner).starts_with("def valued_context("))
    );
    assert!(
        output
            .context_values
            .iter()
            .any(|v| matches!(v, execution::model_protocol::ContextEntryValue::None { .. }))
    );
    let invariant = lctx_model::domain::validation::invariants_for::<execution::model_production::ModelRun>()[0].clone();
    for mutation in 0..12 {
        let mut check = (invariant.create)(&f.budget);
        let mut shuffled = batches.clone();
        if mutation == 4 {
            shuffled.reverse();
        }
        for (name, batch) in &shuffled {
            if mutation == 6 && *name == analysis::model::AnalysisInvocation::NAME {
                continue;
            }
            if mutation == 11
                && matches!(
                    *name,
                    execution::context_binding::ContextEntryBinding::NAME
                        | execution::context_binding::BindingSource::NAME
                        | execution::context_binding::BindingMember::NAME
                )
            {
                continue;
            }
            let input = invariant
                .inputs
                .iter()
                .find(|i| {
                    i.name() == *name
                        && (!stages::is_vocabulary(name)
                            || i.prefix() == Some(stages::PublicationBoundary::Facts))
                })
                .cloned()
                .unwrap_or_else(|| ValidationInput::of::<input::InputRevision>(&["id"]));
            if input.name() == *name {
                check.visit_input(&input, batch).unwrap();
            }
        }
        macro_rules! emit {
            ($ty:ty,$rows:expr) => {
                check
                    .visit_input(
                        &ValidationInput::of::<$ty>(&["id"]),
                        &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                    )
                    .unwrap();
            };
        }
        let erase = mutation == 2 || mutation == 6;
        let mut run = output.run.clone();
        if erase {
            run.applied = 0;
            run.refused = 0;
        }
        emit!(
            execution::model_production::ModelRun,
            if mutation == 6 { vec![] } else { vec![run] }
        );
        let mut outcome = output.outcome.clone();
        if mutation == 3 {
            outcome.status = analysis::AnalysisStatus::Completed;
            outcome.reason = None;
        }
        emit!(analysis::model::AnalysisOutcome, [outcome]);
        macro_rules! outputs{($($field:ident:$ty:ty,)*)=>{$(emit!($ty,if erase{vec![]}else{let mut rows=output.$field.iter().cloned().collect::<Vec<_>>();if mutation==4{rows.reverse();}rows});)*};}
        outputs! {closed_targets:execution::closed_targets::ClosedTargetAssessment,protocol_actions:execution::protocol_interpretation::ProtocolActionAssessment,terminal_assessments:execution::protocol_interpretation::TerminalFrontierAssessment,terminal_frontiers:execution::protocol_interpretation::ConditionalTerminalFrontier,normal_restrictions:execution::protocol_interpretation::NormalContinuationRestriction,exit_characterizations:execution::protocol_interpretation::NativeExitCharacterization,assumption_sets:assumptions::AssumptionSet,assumption_members:assumptions::AssumptionSetMember,assumptions:assumptions::Assumption,assumption_universes:assumptions::AssumptionUniverse,universe_supports:assumptions_universe::AssumptionUniverseSupport,context_resources:execution::model_protocol::ContextResource,context_values:execution::model_protocol::ContextEntryValue,applications:execution::model_production::ModelApplication,application_premises:execution::model_production::ApplicationPremise,boundaries:execution::model_production::ApplicationBoundary,targets:execution::model_production::TargetAssessment,rules:execution::model_rules::AppliedRule,channels:execution::model_rules::ChannelAssessment,operations:execution::model_rules::ModeledOperation,resources:execution::model_rules::ResourceIdentity,paths:execution::model_rules::ModelValuePath,action_assessments:execution::model_production::ActionAssessment,action_sources:execution::model_production::ActionSource,
        transfer_keys:transfer::model::TransferKey,transfer_alternatives:transfer::model::TransferAlternative,transfer_supports:transfer::model::TransferSupport,
        transfer_roots:value::PlaceRoot,transfer_places:value::Place,qualifications:assertion::AssertionQualification,conditions:conditions::Condition,condition_nodes:conditions::ConditionNode,
        subjects:analysis::model::ObligationSubject,support_sources:analysis::model::SupportSource,derivations:analysis::model::AnalysisDerivation,propositions:analysis::model::AnalysisProposition,derivation_premises:analysis::model::AnalysisDerivationPremise,}
        let mut context_transfers = output.context_transfers.iter().cloned().collect::<Vec<_>>();
        if mutation == 8 {
            context_transfers[0].output = context_transfers[0].input;
        }
        if mutation == 9 {
            context_transfers[0].status = analysis::policy::EvidenceStatus::Documented;
        }
        emit!(
            execution::model_context_transfer::ContextTransferWitness,
            if erase || mutation == 10 {
                vec![]
            } else {
                context_transfers
            }
        );
        let mut contexts = output
            .context_postconditions
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if mutation == 7 {
            let exit = contexts
                .iter_mut()
                .find(|p| p.phase == execution::model_rules::ActionPhase::Exceptional)
                .unwrap();
            exit.phase = execution::model_rules::ActionPhase::Normal;
        }
        emit!(
            execution::model_protocol::ContextPostcondition,
            if erase { vec![] } else { contexts }
        );
        let mut posts = output.postconditions.iter().cloned().collect::<Vec<_>>();
        if mutation == 5 {
            let post = posts
                .iter_mut()
                .find(|p| p.phase == execution::model_rules::ActionPhase::Normal)
                .unwrap();
            let assessment = output.action_assessments.get(post.assessment).unwrap();
            let rule = output.rules.get(assessment.rule).unwrap();
            post.source = execution::model_production::ActionSource::Invocation {
                application: rule.application,
            }
            .id();
        }
        emit!(
            execution::model_production::ActionPostcondition,
            if erase { vec![] } else { posts }
        );
        let mut witnesses = output
            .transfer_witnesses
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if mutation == 1 {
            witnesses[0].status = analysis::policy::EvidenceStatus::Documented;
        }
        emit!(
            execution::model_transfer::ModelTransferWitness,
            if erase { vec![] } else { witnesses }
        );
        let result = check.finish();
        assert_eq!(
            result.is_ok(),
            mutation == 0 || mutation == 4,
            "Model replay mutation {mutation}: {result:?}"
        );
    }
    assert!(
        execution::model_production::apply_all(
            &model_data,
            &model_invocation,
            &model_definition,
            stages::Profile::Behavioral,
            &resources::ResourceBudget::fixed(1).unwrap()
        )
        .is_err()
    );
    let catalog_output = execution::model_production::apply_all(
        &model_data,
        &model_invocation,
        &model_definition,
        stages::Profile::Catalog,
        &f.budget,
    )
    .unwrap();
    assert_eq!(
        catalog_output.outcome.status,
        analysis::AnalysisStatus::NotRequested
    );
    assert!(catalog_output.applications.is_empty() && catalog_output.transfer_witnesses.is_empty());
}
