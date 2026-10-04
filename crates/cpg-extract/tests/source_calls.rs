#[path = "fixtures/transfer_composition.rs"]
mod fixture;
#[path = "fixtures/source_execution.rs"]
mod source_fixture;
use lctx_model::domain::{
    analysis::native::NativeInventory,
    conditions::entry::EntryData,
    execution::{evaluation::EvaluationData, source_call::*},
    obligation::ObligationKind,
    *,
};
#[tokio::test]
async fn fresh_binding_is_independent_of_body_and_exact_to_admitted_event() {
    let f = fixture::native().await;
    let mut data = EvaluationData::new(&f.budget);
    let mut flow = EntryData::new(&f.budget);
    let mut inventory = NativeInventory::new(&f.budget);
    let inputs = NativeInventory::inputs();
    for (name, batch) in f.tables.lock().unwrap().iter() {
        data.visit(name, batch).unwrap();
        flow.visit(name, batch).unwrap();
        if inputs.iter().any(|input| input.name() == *name) {
            inventory.visit(name, batch).unwrap();
        }
    }
    macro_rules! normalized{($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap();data.visit(<$ty>::NAME,&batch).unwrap();flow.visit(<$ty>::NAME,&batch).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(normalized);
    let inventory = inventory.collect().unwrap();
    data.premises = inventory.premises;
    data.native = inventory.qualifications;
    let id = f.attempt("fresh()");
    let checked = f.verified.bound(id).unwrap();
    let admission = f.verified.composition(id).unwrap();
    let site = f.data.occurrences.get(checked.bound().site()).unwrap();
    let request = SourceCallRequest {
        input: f.data.artifacts.get(site.source).unwrap().input,
        context: checked.context(),
        event: checked.event(),
    };
    let result = CheckedSourceBinding::derive(
        &data, &flow, &f.data, &f.output, checked, admission, request, &f.budget,
    )
    .unwrap();
    assert!(result.is_ok(), "fresh: {:?}", result.as_ref().err());
    let token = result.ok().unwrap();
    assert!(!token.premises().is_empty());
    assert_eq!(token.callee(), admission.callee());
    // A body that calls an unknown operation does not invalidate independently proven fresh binding.
    let body = f.attempt("fresh_effect()");
    let body_checked = f.verified.bound(body).unwrap();
    let body_admission = f.verified.composition(body).unwrap();
    assert!(
        CheckedSourceBinding::derive(
            &data,
            &flow,
            &f.data,
            &f.output,
            body_checked,
            body_admission,
            SourceCallRequest {
                event: body_checked.event(),
                ..request
            },
            &f.budget
        )
        .unwrap()
        .is_ok()
    );
    let metadata = f.attempt("metadata_inner()");
    let m = f.verified.bound(metadata).unwrap();
    assert!(
        CheckedSourceBinding::derive(
            &data,
            &flow,
            &f.data,
            &f.output,
            m,
            f.verified.composition(metadata).unwrap(),
            SourceCallRequest {
                event: m.event(),
                ..request
            },
            &f.budget
        )
        .unwrap()
        .is_ok()
    );
    for (text, reason) in [
        ("fresh_default()", ObligationKind::DefaultUnavailable),
        ("fresh_async()", ObligationKind::NoSourceDeclaration),
        ("captured_inner()", ObligationKind::CapturedStateUnavailable),
        ("intervening_inner()", ObligationKind::EntryValueUnknown),
    ] {
        let a = f.attempt(text);
        let b = f.verified.bound(a).unwrap();
        let c = f.verified.composition(a).unwrap();
        assert!(
            matches!(CheckedSourceBinding::derive(&data,&flow,&f.data,&f.output,b,c,SourceCallRequest{event:b.event(),..request},&f.budget).unwrap(),Err(r) if r==reason),
            "{text}"
        );
    }
    let coverage = std::mem::replace(&mut flow.coverage, normalized::Rows::new(&f.budget));
    assert!(matches!(
        CheckedSourceBinding::derive(
            &data, &flow, &f.data, &f.output, checked, admission, request, &f.budget
        )
        .unwrap(),
        Err(ObligationKind::IncompleteCoverage)
    ));
    flow.coverage = coverage;
    let native = std::mem::replace(&mut data.native, normalized::Rows::new(&f.budget));
    assert!(matches!(
        CheckedSourceBinding::derive(
            &data, &flow, &f.data, &f.output, checked, admission, request, &f.budget
        )
        .unwrap(),
        Err(ObligationKind::MissingEvidence)
    ));
    data.native = native;
    let other = f.attempt("identity(seed)");
    let foreign = f.verified.bound(other).unwrap();
    assert!(matches!(
        CheckedSourceBinding::derive(
            &data,
            &flow,
            &f.data,
            &f.output,
            checked,
            admission,
            SourceCallRequest {
                event: foreign.event(),
                ..request
            },
            &f.budget
        )
        .unwrap(),
        Err(ObligationKind::IncompatibleContexts)
    ));
    let tiny = resources::ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        CheckedSourceBinding::derive(
            &data, &flow, &f.data, &f.output, checked, admission, request, &tiny
        ),
        Err(ModelError::Resource { .. })
    ));
    drop(token);
    // Pure model replay uses explicit fixture invocations; it does not claim publication authority.

    let mut source_data = execution::source_call_records::SourceCallData::new(&f.budget);
    source_data.evaluation = data;
    source_data.flow = flow;
    source_data.bindings = f.data;
    source_data.output = f.output;
    macro_rules! raw_data{($($field:ident:$ty:ty,)*)=>{$(source_data.completed.visit(<$ty>::NAME,&<$ty as Record>::encode(&source_data.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::execution_evaluation_inputs!(raw_data);
    macro_rules! entry_data{($($field:ident:$ty:ty,)*)=>{$(source_data.completed.visit(<$ty>::NAME,&<$ty as Record>::encode(&source_data.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::entry_value_inputs!(entry_data);
    let (_, evaluation_definition) = execution::configuration::base_evaluation();
    let (evaluation_invocation, _) = analysis::base_evaluation::AnalysisInvocation::new(
        request.input,
        request.context,
        evaluation_definition.id(),
        None,
        [],
    );
    let empty_entries = normalized::Rows::new(&f.budget);
    let empty_sources = normalized::Rows::new(&f.budget);
    let evaluations = execution::production::evaluate_all(
        &source_data.evaluation,
        &source_data.flow,
        &empty_entries,
        &empty_sources,
        &evaluation_invocation,
        &evaluation_definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    macro_rules! earlier {
        ($ty:ty,$rows:expr) => {
            source_data
                .visit(
                    <$ty>::NAME,
                    &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                )
                .unwrap()
        };
    }
    earlier!(
        analysis::base_evaluation::AnalysisInvocation,
        [evaluation_invocation]
    );
    earlier!(analysis::AnalysisDefinition, [evaluation_definition]);
    earlier!(
        execution::records::ExpressionEvaluation,
        evaluations.evaluations.iter().cloned().collect::<Vec<_>>()
    );
    earlier!(
        execution::records::EvaluationSource,
        evaluations.sources.iter().cloned().collect::<Vec<_>>()
    );
    earlier!(
        execution::records::EvaluationMember,
        evaluations.members.iter().cloned().collect::<Vec<_>>()
    );
    earlier!(
        execution::records::EvaluationOperand,
        evaluations.operands.iter().cloned().collect::<Vec<_>>()
    );
    let (_, base_definition) = execution::configuration::base_completion();
    let (base, _) = analysis::base_completion::AnalysisInvocation::new(
        request.input,
        request.context,
        base_definition.id(),
        None,
        [],
    );
    let bodies = execution::completion_production::complete_all(
        &source_data.completed,
        &base,
        &base_definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    earlier!(
        analysis::base_completion::AnalysisInvocation,
        std::slice::from_ref(&base)
    );
    earlier!(
        analysis::AnalysisDefinition,
        std::slice::from_ref(&base_definition)
    );
    earlier!(
        execution::body_records::SourceBodyCompletion,
        bodies.bodies.iter().cloned().collect::<Vec<_>>()
    );
    let data = source_data;
    let (_, definition) = execution::configuration::source_calls();
    let parent = analysis::source_call::InvocationSource::BaseCompletion {
        invocation: base.id(),
    };
    let (invocation, _) = analysis::source_call::AnalysisInvocation::new(
        request.input,
        request.context,
        definition.id(),
        None,
        [parent.id()],
    );
    let records = execution::source_call_records::prepare_all(
        &data,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    assert!(records.headers.len() >= 2);
    assert!(!records.boundaries.is_empty());
    assert!(!records.invocations.is_empty());
    assert!(!records.invocation_boundaries.is_empty());
    for mutation in 0..5 {
        let relation = Relation::of::<execution::source_call_records::SourceCallRun>();
        let invariant = &relation.invariants()[0];
        let mut check = (invariant.create)(&f.budget);
        for input in execution::source_call_records::SourceCallData::inputs() {
            macro_rules! feed{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME{let batch=<$ty as Record>::encode(&data.bindings.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}
            lctx_model::normalized_binding_inputs!(feed);
            macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME&&!normalized::binding_normalization::BindingData::validation_inputs().iter().any(|i|i.name()==input.name()){let batch=<$ty as Record>::encode(&data.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}
            lctx_model::execution_evaluation_inputs!(raw);
            macro_rules! flow{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME&&!normalized::binding_normalization::BindingData::validation_inputs().iter().any(|i|i.name()==input.name())&&!execution::evaluation::EvaluationData::validation_inputs().iter().any(|i|i.name()==input.name()){let batch=<$ty as Record>::encode(&data.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}
            lctx_model::entry_value_inputs!(flow);
            macro_rules! outputs{($($field:ident:$ty:ty,)*)=>{$(if input.name()==<$ty>::NAME{let batch=<$ty as Record>::encode(&data.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit_input(&input,&batch).unwrap();})*};}
            lctx_model::normalized_binding_outputs!(outputs);
        }
        macro_rules! put {
            ($ty:ty,$rows:expr) => {
                check
                    .visit(
                        <$ty>::NAME,
                        &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                    )
                    .unwrap()
            };
        }
        put!(
            analysis::source_call::AnalysisInvocation,
            std::slice::from_ref(&invocation)
        );
        put!(
            analysis::base_completion::AnalysisInvocation,
            std::slice::from_ref(&base)
        );
        put!(
            analysis::AnalysisDefinition,
            std::slice::from_ref(&definition)
        );
        put!(
            analysis::source_call::AnalysisOutcome,
            std::slice::from_ref(&records.outcome)
        );
        put!(
            execution::source_call_records::SourceCallBoundary,
            records.boundaries.iter().cloned().collect::<Vec<_>>()
        );
        let mut invocations = records.invocations.iter().cloned().collect::<Vec<_>>();
        let mut outcomes = records.call_outcomes.iter().cloned().collect::<Vec<_>>();
        if mutation == 3 {
            invocations[0].status = analysis::policy::EvidenceStatus::Documented;
        }
        if mutation == 4 {
            let outcome = execution::source_call_records::SourceCallOutcome::Raised {
                site: records.headers.iter().next().unwrap().declaration,
                exception: execution::ExactRuntimeException::TypeError,
            };
            invocations[0].outcome = outcome.id();
            outcomes.push(outcome);
        }
        put!(
            execution::source_call_records::SourceInvocation,
            invocations
        );
        put!(
            execution::source_call_records::SourceFrameRelease,
            records.releases.iter().cloned().collect::<Vec<_>>()
        );
        put!(execution::source_call_records::SourceCallOutcome, outcomes);
        put!(
            execution::source_call_records::InvocationBoundary,
            records
                .invocation_boundaries
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        );
        put!(
            analysis::base_evaluation::AnalysisInvocation,
            vec![evaluations.run.invocation]
                .into_iter()
                .map(|_| analysis::base_evaluation::AnalysisInvocation::new(
                    request.input,
                    request.context,
                    execution::configuration::base_evaluation().1.id(),
                    None,
                    []
                )
                .0)
                .collect::<Vec<_>>()
        );
        put!(
            analysis::AnalysisDefinition,
            [
                execution::configuration::base_evaluation().1,
                base_definition.clone()
            ]
        );
        put!(
            execution::records::ExpressionEvaluation,
            evaluations.evaluations.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::records::EvaluationSource,
            evaluations.sources.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::records::EvaluationMember,
            evaluations.members.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::records::EvaluationOperand,
            evaluations.operands.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::body_records::SourceBodyCompletion,
            bodies.bodies.iter().cloned().collect::<Vec<_>>()
        );
        let mut run = records.run.clone();
        let mut headers = records.headers.iter().cloned().collect::<Vec<_>>();
        let mut members = records.members.iter().cloned().collect::<Vec<_>>();
        if mutation == 1 {
            let old = headers[0].id();
            headers[0].status = analysis::policy::EvidenceStatus::Documented;
            let new = headers[0].id();
            for member in &mut members {
                if member.header == old {
                    member.header = new;
                }
            }
        }
        if mutation == 2 {
            let removed = headers.pop().unwrap();
            members.retain(|m| m.header != removed.id());
            run.bound -= 1;
        }
        put!(execution::source_call_records::SourceCallRun, [run]);
        put!(execution::source_call_records::SourceCallHeader, headers);
        put!(execution::source_call_records::HeaderMember, members);
        let result = check.finish();
        assert_eq!(
            result.is_ok(),
            mutation == 0,
            "mutation={mutation}: {result:?}"
        );
    }
}

#[tokio::test]
async fn retained_source_shapes_preserve_invocation_and_frame_boundaries() {
    let f = fixture::native_from("source_body_shapes").await;
    let mut data = source_fixture::data(&f);
    let input = f.rows::<input::InputRevision>()[0].id();
    let context = f.data.event_events.iter().next().unwrap().context;
    let base = source_fixture::base_rows_with_entries(
        &mut data,
        input,
        context,
        &f.budget,
        &normalized::Rows::new(&f.budget),
        &normalized::Rows::new(&f.budget),
    )
    .0;
    let (_, definition) = execution::configuration::source_calls();
    let parent = analysis::source_call::InvocationSource::BaseCompletion {
        invocation: base.id(),
    };
    let (invocation, _) = analysis::source_call::AnalysisInvocation::new(
        input,
        context,
        definition.id(),
        None,
        [parent.id()],
    );
    let records = execution::source_call_records::prepare_all(
        &data,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    let name = |owner: Id<normalized::entities::EntityRef>| {
        let normalized::entities::EntityRef::Callable { callable } =
            data.bindings.refs.get(owner).unwrap()
        else {
            panic!("source caller")
        };
        let normalized::entities::CallableEntity::Source { declaration, .. } =
            data.bindings.callables.get(*callable).unwrap()
        else {
            panic!("source caller")
        };
        let name = data
            .bindings
            .declarations
            .iter()
            .find(|row| row.declaration == *declaration)
            .unwrap()
            .name;
        data.evaluation
            .spellings
            .iter()
            .find(|row| row.occurrence == name)
            .unwrap()
            .spelling
            .clone()
    };
    for caller in ["call_literal", "call_fallthrough", "call_return_finally"] {
        let headers = records
            .headers
            .iter()
            .filter(|row| name(row.owner) == caller)
            .collect::<Vec<_>>();
        if headers.len() != 1 {
            for boundary in records.boundaries.iter().take(16) {
                eprintln!("SOURCE_CALL_BOUNDARY {boundary:?}");
                for attempt in data.output.attempts.iter().filter(|a| a.event == boundary.event).take(4) {
                    eprintln!("SOURCE_CALL_ATTEMPT {attempt:?}");
                    if let Some(signature) = attempt.signature.and_then(|id| data.bindings.signatures.get(id)) {
                        eprintln!("SOURCE_CALL_SELECTED_SIGNATURE {signature:?}");
                        for support in data.bindings.signature_supports.iter().filter(|s| s.assertion == signature.id()).take(4) {
                            eprintln!("SOURCE_CALL_SIGNATURE_SUPPORT {support:?}");
                            for coverage in data.bindings.coverage.iter().filter(|c| {
                                c.scope == signature.scope && c.run == Some(support.run)
                                    && c.family == attribution::FactFamily::Signatures
                            }).take(4) {
                                eprintln!("SOURCE_CALL_SIGNATURE_COVERAGE {coverage:?}");
                            }
                        }
                        for boundary in f.rows::<attribution::SubjectBoundary>().iter().filter(|b| {
                            b.scope == signature.scope && b.family == attribution::FactFamily::Signatures
                        }).take(8) {
                            eprintln!("SOURCE_CALL_SIGNATURE_BOUNDARY {boundary:?}");
                        }
                    }
                    if let Some(effective) = attempt.effective.and_then(|id| data.bindings.callable_assessments.get(id)) {
                        eprintln!("SOURCE_CALL_EFFECTIVE {effective:?}");
                        for variant in data.bindings.callable_variants.iter().filter(|v| {
                            v.assessment == Some(effective.id())
                        }).take(8) {
                            eprintln!("SOURCE_CALL_SIGNATURE {:?}", data.bindings.signatures.get(variant.signature));
                        }
                    }
                    for member in data.output.members.iter().filter(|m| m.attempt == attempt.id()).take(4) {
                        let variant = data.output.variants.get(member.variant).unwrap();
                        eprintln!("SOURCE_CALL_SET {:?}", data.output.sets.get(variant.set));
                    }
                }
            }
        }
        assert_eq!(headers.len(), 1, "{caller}: fresh header");
        let release = records
            .releases
            .iter()
            .find(|row| row.header == headers[0].id())
            .unwrap_or_else(|| panic!("{caller}: frame unavailable"));
        let call = records
            .invocations
            .iter()
            .find(|row| row.release == release.id())
            .unwrap();
        assert_eq!(
            records.call_outcomes.get(call.outcome),
            Some(&execution::source_call_records::SourceCallOutcome::Normal),
            "{caller}"
        );
    }
    let raised = records
        .headers
        .iter()
        .find(|row| name(row.owner) == "call_raises")
        .unwrap();
    let release = records
        .releases
        .iter()
        .find(|row| row.header == raised.id())
        .unwrap();
    let call = records
        .invocations
        .iter()
        .find(|row| row.release == release.id())
        .unwrap();
    assert!(matches!(
        records.call_outcomes.get(call.outcome),
        Some(execution::source_call_records::SourceCallOutcome::Raised {
            exception: execution::ExactRuntimeException::TypeError,
            ..
        })
    ));
    for caller in [
        "call_local_read",
        "call_default",
        "call_captured",
        "call_intervening",
        "call_alias",
        "call_failed_header",
        "call_unknown_body",
        "call_generator",
        "call_unreachable_yield",
    ] {
        assert!(
            !records
                .invocations
                .iter()
                .any(|row| records.call_outcomes.get(row.outcome)
                    == Some(&execution::source_call_records::SourceCallOutcome::Normal)
                    && records
                        .releases
                        .get(row.release)
                        .and_then(|release| records.headers.get(release.header))
                        .is_some_and(|header| name(header.owner) == caller)),
            "{caller}: fabricated normal invocation"
        );
    }
    let (_, replayed) =
        execution::enriched::with_frame(&data, &invocation, &definition, &f.budget, |frame| {
            for caller in [
                "call_literal",
                "call_fallthrough",
                "call_return_finally",
                "call_raises",
            ] {
                let header = records
                    .headers
                    .iter()
                    .find(|row| name(row.owner) == caller)
                    .unwrap();
                let event = data.bindings.event_events.get(header.event).unwrap();
                let mut current = event.site;
                let statement = loop {
                    let placement = data
                        .evaluation
                        .placements
                        .iter()
                        .find(|row| row.occurrence == current)
                        .unwrap();
                    if data
                        .evaluation
                        .occurrences
                        .get(current)
                        .unwrap()
                        .syntax_kind
                        == source::SyntaxKind::StmtExpr
                    {
                        break current;
                    }
                    current = placement.parent.unwrap();
                };
                let result = frame
                    .complete(execution::completion::CompletionRequest {
                        input,
                        context,
                        owner: header.owner,
                        statement,
                    })?
                    .unwrap();
                if caller == "call_raises" {
                    assert!(matches!(
                        result.outcome(),
                        execution::outcome::PendingOutcome::Raise {
                            exception: execution::ExactRuntimeException::TypeError,
                            ..
                        }
                    ));
                } else {
                    assert_eq!(result.outcome(), execution::outcome::PendingOutcome::Normal);
                }
                assert!(
                    result
                        .emit_base(
                            &base,
                            &execution::configuration::base_completion().1,
                            &[],
                            &f.budget
                        )
                        .is_err(),
                    "enriched evidence laundered into Base"
                );
            }
            Ok(())
        })
        .unwrap();
    assert!(records.invocations.same(&replayed.invocations));
    // Authored call_modeled needs the earlier applicability operation, separate from source-only
    // invocation. Caller reach and Summary witnesses are qualified by downstream controls.
}

#[tokio::test]
async fn modeled_return_replays_every_actual_and_then_releases_the_exact_fresh_source_frame() {
    let f = fixture::native_from("source_body_shapes").await;
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
    let output = execution::enriched_production::enrich_all(
        &data,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    let modeled = output
        .modeled_calls
        .iter()
        .find(|row| {
            let o = data
                .source
                .evaluation
                .occurrences
                .get(row.expression)
                .unwrap();
            let artifact = data.source.evaluation.artifacts.get(o.source).unwrap();
            f.source_bytes(artifact.id())[o.start as usize..o.end as usize] == *b"cast(int, 10)"
        })
        .unwrap_or_else(|| {
            panic!(
                "cast normal call absent; shapes {} modeled {}",
                f.output
                    .attempts
                    .iter()
                    .filter(|a| f.verified.shape(a.id()).is_some())
                    .count(),
                output.modeled_calls.len()
            )
        });
    let retained = output
        .modeled_calls
        .iter()
        .find(|row| {
            let o = data
                .source
                .evaluation
                .occurrences
                .get(row.expression)
                .unwrap();
            f.source_bytes(o.source)[o.start as usize..o.end as usize] == *b"cast(int, value)"
        })
        .expect("actual Use entry argument holder");
    assert_eq!(
        retained.release,
        execution::evaluation::ReleaseSafety::CallerRetained
    );
    assert!(
        output
            .releases
            .iter()
            .any(|row| row.expression == retained.expression
                && row.safety == execution::evaluation::ReleaseSafety::CallerRetained)
    );
    let reordered = output
        .modeled_calls
        .iter()
        .find(|row| {
            let o = data
                .source
                .evaluation
                .occurrences
                .get(row.expression)
                .unwrap();
            f.source_bytes(o.source)[o.start as usize..o.end as usize] == *b"cast(val=10, typ=int)"
        })
        .expect("reordered keyword model call");
    assert_eq!(
        output
            .modeled_arguments
            .iter()
            .find(|row| row.call == reordered.id() && row.ordinal == 0)
            .unwrap()
            .actual,
        reordered.returned_actual,
        "actual evaluation order is independent of formal slot order"
    );
    assert_eq!(
        modeled.release,
        execution::evaluation::ReleaseSafety::Closed
    );
    assert_eq!(
        output
            .modeled_arguments
            .iter()
            .filter(|row| row.call == modeled.id())
            .count(),
        2,
        "type and returned value are independently evaluated"
    );
    let header = data
        .source_headers
        .iter()
        .find(|row| {
            let normalized::entities::EntityRef::Callable { callable } =
                data.source.bindings.refs.get(row.owner).unwrap()
            else {
                return false;
            };
            let normalized::entities::CallableEntity::Source { declaration, .. } =
                data.source.bindings.callables.get(*callable).unwrap()
            else {
                return false;
            };
            let o = data
                .source
                .evaluation
                .occurrences
                .get(*declaration)
                .unwrap();
            let artifact = data.source.evaluation.artifacts.get(o.source).unwrap();
            f.source_bytes(artifact.id())[o.start as usize..o.end as usize]
                .starts_with(b"def call_modeled(")
        })
        .unwrap();
    let call = output
        .fresh_calls
        .iter()
        .find(|row| row.header == header.id())
        .unwrap();
    assert_eq!(
        output.outcomes.get(call.outcome),
        Some(&execution::enriched_records::ExecutionOutcome::Normal)
    );
    assert!(output.body_sources.iter().any(|row|matches!(row,execution::enriched_records::BodySource::Statement{execution} if output.sources.iter().any(|source|matches!(source,execution::enriched_records::ExecutionSource::ModeledCall{call}if *call==modeled.id())&&output.members.iter().any(|member|member.execution==*execution&&member.source==source.id())))));
    // Completing a definition evaluates defaults, never its deferred body.
    let owner_name = |owner| {
        let normalized::entities::EntityRef::Callable { callable } =
            data.source.evaluation.refs.get(owner).unwrap()
        else {
            panic!("callable")
        };
        let normalized::entities::CallableEntity::Source { declaration, .. } =
            data.source.evaluation.callables.get(*callable).unwrap()
        else {
            panic!("source")
        };
        let name = data
            .source
            .evaluation
            .declarations
            .iter()
            .find(|d| d.declaration == *declaration)
            .unwrap()
            .name;
        data.source
            .evaluation
            .spellings
            .iter()
            .find(|s| s.occurrence == name)
            .unwrap()
            .spelling
            .clone()
    };
    for name in [
        "literal_default_header",
        "keyword_default_header",
        "unexecuted_body_header",
    ] {
        assert!(
            output
                .definition_evaluations
                .iter()
                .any(|row| owner_name(row.owner) == name),
            "definition {name} absent"
        );
        assert!(
            output
                .bodies
                .iter()
                .any(|row| owner_name(row.owner) == name),
            "outer body {name} absent"
        );
    }
    for name in [
        "missing_default_header",
        "rebound_definition_header",
        "call_failed_header",
    ] {
        assert!(
            !output
                .definition_evaluations
                .iter()
                .any(|row| owner_name(row.owner) == name),
            "fabricated definition {name}"
        );
    }
    let invariant =
        Relation::of::<execution::enriched_production::ExecutionRun>().invariants()[0].clone();
    for mutation in 0..4 {
        let mut check = (invariant.create)(&f.budget);
        for (name, batch) in f.tables.lock().unwrap().iter() {
            check.visit(name, batch).unwrap();
        }
        macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_binding_inputs!(raw);
        macro_rules! raw_evaluation{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.source.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(raw_evaluation);
        macro_rules! raw_entry{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.source.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(raw_entry);
        macro_rules! bound{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_binding_outputs!(bound);
        for (name, batch) in &base_rows {
            check.visit(name, batch).unwrap();
        }
        macro_rules! put {
            ($ty:ty,$rows:expr) => {
                check
                    .visit(
                        <$ty>::NAME,
                        &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                    )
                    .unwrap()
            };
        }
        put!(
            analysis::AnalysisDefinition,
            data.source
                .definitions
                .iter()
                .cloned()
                .chain([definition.clone()])
                .collect::<Vec<_>>()
        );
        put!(
            analysis::MethodParameters,
            data.parameters.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            models::ModelCatalog,
            data.catalogs.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            analysis::source_call::AnalysisInvocation,
            data.source_invocations.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            analysis::enriched_execution::AnalysisInvocation,
            std::slice::from_ref(&invocation)
        );
        put!(
            analysis::enriched_execution::InvocationSource,
            std::slice::from_ref(&parent)
        );
        macro_rules! earlier{($($field:ident:$ty:ty,)*)=>{$(put!($ty,data.$field.iter().cloned().collect::<Vec<_>>());)*};}
        earlier! {source_runs:execution::source_call_records::SourceCallRun,source_headers:execution::source_call_records::SourceCallHeader,source_members:execution::source_call_records::HeaderMember,source_boundaries:execution::source_call_records::SourceCallBoundary,source_results:analysis::source_call::AnalysisOutcome,source_calls:execution::source_call_records::SourceInvocation,source_releases:execution::source_call_records::SourceFrameRelease,source_arguments:execution::source_call_records::SourceFrameArgument,source_outcomes:execution::source_call_records::SourceCallOutcome,source_invocation_boundaries:execution::source_call_records::InvocationBoundary,}
        let mut calls = output.modeled_calls.iter().cloned().collect::<Vec<_>>();
        let mut arguments = output.modeled_arguments.iter().cloned().collect::<Vec<_>>();
        let mut native = output.modeled_native.iter().cloned().collect::<Vec<_>>();
        if mutation == 1 || mutation == 2 {
            let row = calls
                .iter_mut()
                .find(|row| row.id() == modeled.id())
                .unwrap();
            let old = row.id();
            if mutation == 1 {
                row.status = analysis::policy::EvidenceStatus::Documented;
            } else {
                row.returned_actual = arguments
                    .iter()
                    .find(|arg| arg.call == old && arg.actual != row.returned_actual)
                    .unwrap()
                    .actual;
            }
            let new = row.id();
            for arg in &mut arguments {
                if arg.call == old {
                    arg.call = new;
                }
            }
            for premise in &mut native {
                if premise.call == old {
                    premise.call = new;
                }
            }
        }
        let erase = mutation == 3;
        put!(
            execution::modeled_call::ModeledCallEvaluation,
            if erase { vec![] } else { calls }
        );
        put!(
            execution::modeled_call::ModeledCallArgument,
            if erase { vec![] } else { arguments }
        );
        put!(
            execution::modeled_call::ModeledCallNative,
            if erase { vec![] } else { native }
        );
        macro_rules! owned{($($field:ident:$ty:ty,)*)=>{$(put!($ty,if erase{vec![]}else{output.$field.iter().cloned().collect::<Vec<_>>()});)*};}
        owned! {executions:execution::enriched_records::StatementExecution,outcomes:execution::enriched_records::ExecutionOutcome,sources:execution::enriched_records::ExecutionSource,members:execution::enriched_records::ExecutionMember,entered:execution::enriched_records::EnteredStatement,boundaries:execution::enriched_production::ExecutionBoundary,bodies:execution::enriched_records::BodyExecution,body_sources:execution::enriched_records::BodySource,body_members:execution::enriched_records::BodyMember,releases:execution::enriched_records::BodyReleaseInput,body_boundaries:execution::enriched_production::BodyBoundary,fresh_calls:execution::enriched_records::SourceExecutionInvocation,fresh_arguments:execution::enriched_records::SourceExecutionArgument,definition_evaluations:execution::definition::DefinitionEvaluation,definition_sources:execution::definition::DefinitionSource,definition_members:execution::definition::DefinitionMember,contexts:execution::context_execution::ContextExecution,context_items:execution::context_execution::ContextItem,context_sources:execution::context_execution::ContextSource,context_members:execution::context_execution::ContextMember,context_bindings:execution::context_binding::ContextEntryBinding,context_binding_sources:execution::context_binding::BindingSource,context_binding_members:execution::context_binding::BindingMember,}
        let mut run = output.run.clone();
        if erase {
            run.executed = 0;
            run.refused = 0;
            run.bodied = 0;
            run.body_refused = 0;
        }
        put!(execution::enriched_production::ExecutionRun, [run]);
        put!(
            analysis::enriched_execution::AnalysisOutcome,
            std::slice::from_ref(&output.outcome)
        );
        let result = check.finish();
        assert_eq!(
            result.is_ok(),
            mutation == 0,
            "modeled replay mutation {mutation}: {result:?}"
        );
    }
    // Erasing the independently evaluated non-returned type argument cannot preserve normal call.
    let missing = output
        .modeled_arguments
        .iter()
        .find(|row| row.call == modeled.id() && row.actual != modeled.returned_actual)
        .unwrap()
        .evaluation;
    let mut replay = execution::completion_production::CompletedEvaluations::new(&f.budget);
    macro_rules! facts{($($field:ident:$ty:ty,)*)=>{$(replay.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.source.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::execution_evaluation_inputs!(facts);
    macro_rules! entry{($($field:ident:$ty:ty,)*)=>{$(replay.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.source.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::entry_value_inputs!(entry);
    for (name, batch) in &base_rows {
        if *name == execution::records::ExpressionEvaluation::NAME {
            let rows = execution::records::ExpressionEvaluation::decode(batch)
                .unwrap()
                .into_iter()
                .filter(|row| row.id() != missing)
                .collect::<Vec<_>>();
            replay
                .visit(
                    name,
                    &execution::records::ExpressionEvaluation::encode(&rows).unwrap(),
                )
                .unwrap();
        } else {
            replay.visit(name, batch).unwrap();
        }
    }
    data.source.completed = replay;
    let refused = execution::enriched_production::enrich_all(
        &data,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    assert!(
        !refused
            .modeled_calls
            .iter()
            .any(|row| row.expression == modeled.expression)
    );
    assert!(
        !refused
            .fresh_calls
            .iter()
            .any(|row| row.header == header.id())
    );
}

#[tokio::test]
async fn source_frame_releases_only_exact_bound_externally_held_actuals() {
    let f = fixture::native_from("source_body_shapes").await;
    let mut data = source_fixture::data(&f);
    let input = f.rows::<input::InputRevision>()[0].id();
    let context = f.data.event_events.iter().next().unwrap().context;
    let mut entries = normalized::Rows::new(&f.budget);
    let mut sources = normalized::Rows::new(&f.budget);
    for observation in data.flow.use_observations.iter() {
        let use_ = data.flow.uses.get(observation.use_).unwrap();
        let Some(owner) = data
            .flow
            .owners
            .iter()
            .find(|owner| owner.occurrence == use_.occurrence)
        else {
            continue;
        };
        let Some(support) = data
            .flow
            .use_supports
            .iter()
            .find(|support| support.assertion == observation.id())
        else {
            continue;
        };
        for formal in data.flow.formals.iter() {
            let request = conditions::entry::EntryRequest {
                owner: owner.entity,
                formal: formal.id(),
                access: use_.occurrence,
                context,
                run: support.run,
            };
            if let Ok(proof) =
                conditions::entry::EntryValueWitness::derive(&data.flow, request, &f.budget)
                    .unwrap()
            {
                entries.insert(proof.witness().clone()).unwrap();
                sources.insert(proof.source().clone()).unwrap();
            }
        }
    }
    let (base, base_rows) = source_fixture::base_rows_with_entries(
        &mut data, input, context, &f.budget, &entries, &sources,
    );
    let (_, definition) = execution::configuration::source_calls();
    let parent = analysis::source_call::InvocationSource::BaseCompletion {
        invocation: base.id(),
    };
    let (invocation, _) = analysis::source_call::AnalysisInvocation::new(
        input,
        context,
        definition.id(),
        None,
        [parent.id()],
    );
    let records = execution::source_call_records::prepare_all(
        &data,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    for (text, count) in [
        ("inner(17)", 1),
        ("inner(value)", 1),
        ("inner(second=19, first=value)", 2),
    ] {
        let event = f.output.attempts.get(f.attempt(text)).unwrap().event;
        let header = records
            .headers
            .iter()
            .find(|row| row.event == event)
            .unwrap_or_else(|| panic!("header {text} absent"));
        let release = records
            .releases
            .iter()
            .find(|row| row.header == header.id())
            .unwrap_or_else(|| panic!("release {text} absent"));
        assert_eq!(
            records
                .arguments
                .iter()
                .filter(|row| row.release == release.id())
                .count(),
            count
        );
        assert_eq!(
            release.release,
            if text == "inner(17)" {
                execution::evaluation::ReleaseSafety::Closed
            } else {
                execution::evaluation::ReleaseSafety::CallerRetained
            },
            "callee retained formal does not certify caller exit"
        );
        assert!(
            records
                .invocations
                .iter()
                .any(|row| row.release == release.id())
        );
    }
    let event = f
        .output
        .attempts
        .get(f.attempt("inner(missing_actual)"))
        .unwrap()
        .event;
    let header = records
        .headers
        .iter()
        .find(|row| row.event == event)
        .unwrap();
    assert!(!records.releases.iter().any(|row| row.header == header.id()));
    // Remove the actual caller holder while preserving callee/body/native binding evidence.
    assert!(records.arguments.len() >= 4);
    let held_header = records
        .headers
        .iter()
        .find(|row| {
            row.event
                == f.output
                    .attempts
                    .get(f.attempt("inner(value)"))
                    .unwrap()
                    .event
        })
        .unwrap();
    let held_release = records
        .releases
        .iter()
        .find(|row| row.header == held_header.id())
        .unwrap();
    let held_actual = records
        .arguments
        .iter()
        .find(|row| row.release == held_release.id())
        .unwrap();
    let invariant =
        Relation::of::<execution::source_call_records::SourceCallRun>().invariants()[0].clone();
    for mutation in 0..3 {
        let mut check = (invariant.create)(&f.budget);
        for (name, batch) in f.tables.lock().unwrap().iter() {
            check.visit(name, batch).unwrap();
        }
        macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_binding_inputs!(raw);
        macro_rules! bound{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_binding_outputs!(bound);
        macro_rules! raw_evaluation{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(raw_evaluation);
        for (name, batch) in &base_rows {
            check.visit(name, batch).unwrap();
        }
        macro_rules! put {
            ($ty:ty,$rows:expr) => {
                check
                    .visit(
                        <$ty>::NAME,
                        &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                    )
                    .unwrap()
            };
        }
        put!(
            analysis::AnalysisDefinition,
            std::slice::from_ref(&definition)
        );
        put!(
            analysis::source_call::AnalysisInvocation,
            std::slice::from_ref(&invocation)
        );
        put!(
            analysis::source_call::InvocationSource,
            std::slice::from_ref(&parent)
        );
        put!(
            execution::source_call_records::SourceCallRun,
            std::slice::from_ref(&records.run)
        );
        put!(
            analysis::source_call::AnalysisOutcome,
            std::slice::from_ref(&records.outcome)
        );
        macro_rules! outputs{($($field:ident:$ty:ty,)*)=>{$(put!($ty,records.$field.iter().cloned().collect::<Vec<_>>());)*};}
        outputs! {headers:execution::source_call_records::SourceCallHeader,members:execution::source_call_records::HeaderMember,boundaries:execution::source_call_records::SourceCallBoundary,invocations:execution::source_call_records::SourceInvocation,releases:execution::source_call_records::SourceFrameRelease,call_outcomes:execution::source_call_records::SourceCallOutcome,invocation_boundaries:execution::source_call_records::InvocationBoundary,}
        let mut arguments = records.arguments.iter().cloned().collect::<Vec<_>>();
        if mutation == 1 {
            arguments.retain(|row| row.release != held_release.id());
        } else if mutation == 2 {
            arguments
                .iter_mut()
                .find(|row| row.release == held_release.id())
                .unwrap()
                .evaluation = records
                .arguments
                .iter()
                .find(|row| row.evaluation != held_actual.evaluation)
                .unwrap()
                .evaluation;
        }
        put!(
            execution::source_call_records::SourceFrameArgument,
            arguments
        );
        let result = check.finish();
        assert_eq!(
            result.is_ok(),
            mutation == 0,
            "missing/foreign outside holder {mutation}: {result:?}"
        );
    }
}

#[tokio::test]
async fn ordered_context_execution_replays_actual_entry_body_reverse_exit_and_suppression() {
    let f = fixture::native_from("source_body_shapes").await;
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
    let output = execution::enriched_production::enrich_all(
        &data,
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    let owner_name = |owner| {
        let normalized::entities::EntityRef::Callable { callable } =
            data.source.evaluation.refs.get(owner).unwrap()
        else {
            panic!("callable")
        };
        let normalized::entities::CallableEntity::Source { declaration, .. } =
            data.source.evaluation.callables.get(*callable).unwrap()
        else {
            panic!("source")
        };
        let name = data
            .source
            .evaluation
            .declarations
            .iter()
            .find(|d| d.declaration == *declaration)
            .unwrap()
            .name;
        data.source
            .evaluation
            .spellings
            .iter()
            .find(|s| s.occurrence == name)
            .unwrap()
            .spelling
            .clone()
    };
    let context_row = |name: &str| {
        output
            .contexts
            .iter()
            .find(|row| owner_name(row.owner) == name)
            .unwrap_or_else(|| {
                panic!(
                    "context {name} absent: {:?}",
                    output
                        .boundaries
                        .iter()
                        .filter(|r| r.owner.is_some_and(|o| owner_name(o) == name))
                        .collect::<Vec<_>>()
                )
            })
    };
    for name in [
        "with_preserve",
        "with_suppress",
        "with_multiple",
        "with_nested",
    ] {
        assert_eq!(
            output.outcomes.get(context_row(name).outcome),
            Some(&execution::enriched_records::ExecutionOutcome::Normal),
            "{name}"
        );
    }
    assert!(matches!(
        output
            .outcomes
            .get(context_row("with_preserve_return").outcome),
        Some(execution::enriched_records::ExecutionOutcome::Return { .. })
    ));
    assert!(matches!(
        output.outcomes.get(context_row("with_nonmatch").outcome),
        Some(execution::enriched_records::ExecutionOutcome::Raise {
            exception: execution::ExactRuntimeException::TypeError,
            ..
        })
    ));
    assert!(matches!(
        output
            .outcomes
            .get(context_row("with_finalizer_return").outcome),
        Some(execution::enriched_records::ExecutionOutcome::Return { .. })
    ));
    for name in ["with_async", "with_unknown_body"] {
        assert!(
            !output
                .contexts
                .iter()
                .any(|row| owner_name(row.owner) == name),
            "unproven {name}"
        );
    }
    assert!(matches!(
        output.outcomes.get(context_row("with_target").outcome),
        Some(execution::enriched_records::ExecutionOutcome::Return { .. })
    ));
    assert!(matches!(
        output.outcomes.get(context_row("with_none_target").outcome),
        Some(execution::enriched_records::ExecutionOutcome::Return { .. })
    ));
    for name in ["with_rebound_target", "with_missing_target_constructor"] {
        assert!(
            !output
                .contexts
                .iter()
                .any(|row| owner_name(row.owner) == name),
            "unproven target {name}"
        );
    }
    assert!(!output.context_bindings.is_empty());
    let held = output
        .context_bindings
        .iter()
        .find(|row| owner_name(row.owner) == "with_held_target")
        .expect("actual parameter holder retained under context entry");
    assert!(held.entry_actual.is_some());
    assert_eq!(
        held.release,
        execution::evaluation::ReleaseSafety::CallerRetained
    );
    assert!(matches!(
        output.outcomes.get(context_row("with_held_target").outcome),
        Some(execution::enriched_records::ExecutionOutcome::Return { .. })
    ));
    assert!(
        output
            .releases
            .iter()
            .any(|row| row.expression == held.access
                && row.safety == execution::evaluation::ReleaseSafety::CallerRetained)
    );
    let multiple = context_row("with_multiple");
    let outer = output
        .context_items
        .iter()
        .find(|item| item.execution == multiple.id() && item.ordinal == 0)
        .unwrap();
    let inner = output
        .context_items
        .iter()
        .find(|item| item.execution == multiple.id() && item.ordinal == 1)
        .unwrap();
    assert!(inner.suppressed);
    assert!(!outer.suppressed);
    assert!(matches!(
        output.outcomes.get(inner.exit_input),
        Some(execution::enriched_records::ExecutionOutcome::Raise { .. })
    ));
    assert_eq!(inner.exit_output, outer.exit_input);
    assert_eq!(
        output.outcomes.get(outer.exit_output),
        Some(&execution::enriched_records::ExecutionOutcome::Normal)
    );
    let invariant =
        Relation::of::<execution::enriched_production::ExecutionRun>().invariants()[0].clone();
    for mutation in 0..5 {
        let mut check = (invariant.create)(&f.budget);
        for (name, batch) in f.tables.lock().unwrap().iter() {
            check.visit(name, batch).unwrap();
        }
        macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_binding_inputs!(raw);
        macro_rules! raw_evaluation{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.source.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(raw_evaluation);
        macro_rules! raw_entry{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.source.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(raw_entry);
        macro_rules! bound{($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::normalized_binding_outputs!(bound);
        for (name, batch) in &base_rows {
            check.visit(name, batch).unwrap();
        }
        macro_rules! put {
            ($ty:ty,$rows:expr) => {
                check
                    .visit(
                        <$ty>::NAME,
                        &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                    )
                    .unwrap()
            };
        }
        put!(
            analysis::AnalysisDefinition,
            data.source
                .definitions
                .iter()
                .cloned()
                .chain([definition.clone()])
                .collect::<Vec<_>>()
        );
        put!(
            analysis::MethodParameters,
            data.parameters.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            models::ModelCatalog,
            data.catalogs.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            analysis::source_call::AnalysisInvocation,
            data.source_invocations.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            analysis::enriched_execution::AnalysisInvocation,
            std::slice::from_ref(&invocation)
        );
        put!(
            analysis::enriched_execution::InvocationSource,
            std::slice::from_ref(&parent)
        );
        macro_rules! earlier{($($field:ident:$ty:ty,)*)=>{$(put!($ty,data.$field.iter().cloned().collect::<Vec<_>>());)*};}
        earlier! {source_runs:execution::source_call_records::SourceCallRun,source_headers:execution::source_call_records::SourceCallHeader,source_members:execution::source_call_records::HeaderMember,source_boundaries:execution::source_call_records::SourceCallBoundary,source_results:analysis::source_call::AnalysisOutcome,source_calls:execution::source_call_records::SourceInvocation,source_releases:execution::source_call_records::SourceFrameRelease,source_arguments:execution::source_call_records::SourceFrameArgument,source_outcomes:execution::source_call_records::SourceCallOutcome,source_invocation_boundaries:execution::source_call_records::InvocationBoundary,}
        let mut contexts = output.contexts.iter().cloned().collect::<Vec<_>>();
        let mut items = output.context_items.iter().cloned().collect::<Vec<_>>();
        if mutation == 1 {
            contexts[0].status = analysis::policy::EvidenceStatus::Documented;
        }
        if mutation == 2 {
            items[0].exit_output = execution::enriched_records::ExecutionOutcome::Normal.id();
            items[0].suppressed = !items[0].suppressed;
        }
        let mut bindings = output.context_bindings.iter().cloned().collect::<Vec<_>>();
        if mutation == 4 {
            bindings[0].target = bindings[0].access;
        }
        put!(
            execution::context_binding::ContextEntryBinding,
            if mutation == 3 { vec![] } else { bindings }
        );
        let erase = mutation == 3;
        put!(
            execution::context_execution::ContextExecution,
            if erase { vec![] } else { contexts }
        );
        put!(
            execution::context_execution::ContextItem,
            if erase { vec![] } else { items }
        );
        put!(
            execution::modeled_call::ModeledCallEvaluation,
            output.modeled_calls.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::modeled_call::ModeledCallArgument,
            output.modeled_arguments.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::modeled_call::ModeledCallNative,
            output.modeled_native.iter().cloned().collect::<Vec<_>>()
        );
        macro_rules! owned{($($field:ident:$ty:ty,)*)=>{$(put!($ty,if erase{vec![]}else{output.$field.iter().cloned().collect::<Vec<_>>()});)*};}
        owned! {executions:execution::enriched_records::StatementExecution,outcomes:execution::enriched_records::ExecutionOutcome,sources:execution::enriched_records::ExecutionSource,members:execution::enriched_records::ExecutionMember,entered:execution::enriched_records::EnteredStatement,boundaries:execution::enriched_production::ExecutionBoundary,bodies:execution::enriched_records::BodyExecution,body_sources:execution::enriched_records::BodySource,body_members:execution::enriched_records::BodyMember,releases:execution::enriched_records::BodyReleaseInput,body_boundaries:execution::enriched_production::BodyBoundary,fresh_calls:execution::enriched_records::SourceExecutionInvocation,fresh_arguments:execution::enriched_records::SourceExecutionArgument,definition_evaluations:execution::definition::DefinitionEvaluation,definition_sources:execution::definition::DefinitionSource,definition_members:execution::definition::DefinitionMember,context_sources:execution::context_execution::ContextSource,context_members:execution::context_execution::ContextMember,context_binding_sources:execution::context_binding::BindingSource,context_binding_members:execution::context_binding::BindingMember,}
        let mut run = output.run.clone();
        if erase {
            run.executed = 0;
            run.refused = 0;
            run.bodied = 0;
            run.body_refused = 0;
        }
        put!(execution::enriched_production::ExecutionRun, [run]);
        put!(
            analysis::enriched_execution::AnalysisOutcome,
            std::slice::from_ref(&output.outcome)
        );
        let result = check.finish();
        assert_eq!(
            result.is_ok(),
            mutation == 0,
            "context replay mutation {mutation}: {result:?}"
        );
    }
}
