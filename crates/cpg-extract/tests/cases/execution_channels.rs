use crate::typed_driver;
use lctx_model::domain::{
    analysis::native::NativeInventory, execution::evaluation::*, normalized::entity_normalization,
    obligation::ObligationKind, resources::ResourceBudget, source::*, *,
};
use typed_driver::{files, rows};
struct Facts(typed_driver::Tables);
impl typed_driver::Inspector for Facts {
    fn tables(&self) -> typed_driver::Tables {
        self.0.clone()
    }
}
async fn data() -> (
    EvaluationData,
    lctx_model::domain::conditions::entry::EntryData,
    ResourceBudget,
) {
    let tables = typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("execution_channels"), Facts(tables.clone()))
        .await
        .unwrap();
    let budget = ResourceBudget::fixed(256 << 20).unwrap();
    let mut facts = entity_normalization::EntityData::new(&budget);
    macro_rules! inputs {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables){facts.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(inputs);
    let normalized = entity_normalization::normalize(facts.inputs(), &budget).unwrap();
    let mut data = EvaluationData::new(&budget);
    let mut entries = lctx_model::domain::conditions::entry::EntryData::new(&budget);
    let mut inventory = NativeInventory::new(&budget);
    let native_inputs = NativeInventory::inputs();
    for (name, batch) in tables.lock().unwrap().iter() {
        data.visit(name, batch).unwrap();
        entries.visit(name, batch).unwrap();
        if native_inputs.iter().any(|input| input.name() == *name) {
            inventory.visit(name, batch).unwrap();
        }
    }
    macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&normalized.$field.iter().cloned().collect::<Vec<_>>()).unwrap();data.visit(<$ty>::NAME,&batch).unwrap();entries.visit(<$ty>::NAME,&batch).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(outputs);
    let inventory = inventory.collect().unwrap();
    data.premises = inventory.premises;
    data.native = inventory.qualifications;
    (data, entries, budget)
}
fn request(data: &EvaluationData, text: &str) -> ExpressionRequest {
    let files = files("execution_channels");
    let occurrence = data
        .occurrences
        .iter()
        .find(|o| {
            o.role == OccurrenceRole::Syntax
                && !matches!(
                    o.syntax_kind,
                    SyntaxKind::StringLiteral | SyntaxKind::BytesLiteral | SyntaxKind::Identifier
                )
                && {
                    let source = data.artifacts.get(o.source).unwrap();
                    files[&source.path][o.start as usize..o.end as usize] == *text.as_bytes()
                }
        })
        .unwrap_or_else(|| panic!("native syntax absent {text}"));
    let owner = data
        .owners
        .iter()
        .find(|owner| owner.occurrence == occurrence.id())
        .unwrap();
    let context = data
        .placements
        .iter()
        .find(|row| row.occurrence == occurrence.id())
        .map(|row| data.qualifications.get(row.qualification).unwrap().context)
        .unwrap();
    ExpressionRequest {
        input: data.artifacts.get(occurrence.source).unwrap().input,
        context,
        owner: owner.entity,
        expression: occurrence.id(),
    }
}
fn builtin_request(data: &EvaluationData, function: &str) -> ExpressionRequest {
    use lctx_model::domain::normalized::entities::*;
    let source_files = files("execution_channels");
    let callable=data.callables.iter().find(|row|matches!(row,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let source=data.artifacts.get(o.source).unwrap();source_files[&source.path][o.start as usize..o.end as usize].starts_with(format!("def {function}(").as_bytes())})).unwrap();
    let owner = EntityRef::Callable {
        callable: callable.id(),
    }
    .id();
    let expression = data
        .occurrences
        .iter()
        .find(|row| {
            row.syntax_kind == SyntaxKind::ExprName
                && row.role == OccurrenceRole::Read
                && data
                    .owners
                    .iter()
                    .any(|o| o.occurrence == row.id() && o.entity == owner)
                && data
                    .spellings
                    .iter()
                    .any(|s| s.occurrence == row.id() && s.spelling == "int")
        })
        .unwrap();
    let qualification = data
        .spellings
        .iter()
        .find(|s| s.occurrence == expression.id())
        .unwrap()
        .qualification;
    ExpressionRequest {
        input: data.artifacts.get(expression.source).unwrap().input,
        context: data.qualifications.get(qualification).unwrap().context,
        owner,
        expression: expression.id(),
    }
}
#[tokio::test]
async fn native_builtin_read_is_structural_and_refuses_shadowing_frame_and_forged_support() {
    use lctx_model::domain::{
        analysis::{native::NativeAssertionPremise, policy::EvidenceStatus},
        execution::builtin_read::CheckedBuiltinRead,
        normalized::Rows,
    };
    let (mut data, _, budget) = data().await;
    let request = builtin_request(&data, "builtin_positive");
    let proof = CheckedBuiltinRead::derive(&data, request, &budget)
        .unwrap()
        .unwrap();
    assert_eq!(proof.release(), ReleaseSafety::CallerRetained);
    assert_eq!(proof.status(), EvidenceStatus::StructurallyObserved);
    let evaluated = evaluate(&data, request, &budget).unwrap().unwrap();
    assert_eq!(evaluated.release(), ReleaseSafety::CallerRetained);
    assert!(evaluated.entry_premises().is_empty());
    for name in ["builtin_shadowed", "builtin_rebound"] {
        assert!(
            CheckedBuiltinRead::derive(&data, builtin_request(&data, name), &budget)
                .unwrap()
                .is_err(),
            "{name}"
        );
    }
    let mut foreign = request;
    foreign.owner = builtin_request(&data, "builtin_shadowed").owner;
    assert!(
        CheckedBuiltinRead::derive(&data, foreign, &budget)
            .unwrap()
            .is_err()
    );
    let context = attribution::AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "foreign".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"foreign"),
        environment_digest: ContentHash::of(b"foreign"),
        lock_digest: None,
    };
    foreign = request;
    foreign.context = context.id();
    assert!(
        CheckedBuiltinRead::derive(&data, foreign, &budget)
            .unwrap()
            .is_err()
    );
    let selected = proof.native_premises()[2];
    let mut kept = Rows::new(&budget);
    for row in data.premises.iter().filter(|row| row.id() != selected) {
        kept.insert(row.clone()).unwrap();
    }
    data.premises = kept;
    assert!(matches!(proof.native_premises().len(), 3));
    assert!(!matches!(
        CheckedBuiltinRead::derive(&data, request, &budget),
        Ok(Ok(_))
    ));
    assert!(
        data.premises
            .iter()
            .any(|row| matches!(row, NativeAssertionPremise::LexicalResolution { .. }))
    );
}
#[tokio::test]
async fn native_closed_expressions_skip_unentered_operands_and_preserve_limits() {
    let (data, _, budget) = data().await;
    for (text, truth) in [
        ("False and effect()", Some(false)),
        ("True or effect()", Some(true)),
        ("3 if True else effect()", Some(true)),
        ("1 + 2", Some(true)),
        ("-2", Some(true)),
        ("not ()", Some(true)),
        ("\"hello\"", None),
        ("1j", None),
    ] {
        let req = request(&data, text);
        let result = evaluate(&data, req, &budget)
            .unwrap()
            .unwrap_or_else(|reason| panic!("{text}: {reason:?}"));
        assert_eq!(result.truth(), truth, "{text}");
        assert_eq!(result.release(), ReleaseSafety::Closed);
        assert!(!result.native_premises().is_empty());
        assert!(
            result.evaluated_operands().iter().all(|id| data
                .occurrences
                .get(*id)
                .unwrap()
                .syntax_kind
                != SyntaxKind::ExprCall)
        );
    }
    let entered = request(&data, "True and effect()");
    let effect = data
        .references
        .iter()
        .find(|r| {
            r.name == "effect"
                && data.occurrences.get(r.read).is_some_and(|o| {
                    o.start > data.occurrences.get(entered.expression).unwrap().start
                        && o.end < data.occurrences.get(entered.expression).unwrap().end
                })
        })
        .unwrap();
    assert!(
        data.lexical_resolutions
            .iter()
            .any(|r| r.read == effect.read
                && matches!(
                    data.lexical_targets.get(r.target),
                    Some(lexical::LexicalTarget::Binding { .. })
                )),
        "entered source-function name is a binding, never a builtin token"
    );
    for (text, reason) in [
        ("True and effect()", ObligationKind::EntryValueUnknown),
        (
            "999999999999999999999999999999999999999 + 1",
            ObligationKind::UnsupportedControlFlow,
        ),
    ] {
        let result = evaluate(&data, request(&data, text), &budget).unwrap();
        assert!(
            matches!(&result, Err(actual) if *actual == reason),
            "{text}: expected {reason:?}, got {:?}",
            result.as_ref().err()
        );
    }
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        evaluate(&data, request(&data, "1 + 2"), &tiny),
        Err(ModelError::Resource { .. })
    ));
}

#[tokio::test]
async fn name_evaluation_needs_the_exact_private_entry_proof_and_retains_its_allowance() {
    use lctx_model::domain::{conditions::entry::*, normalized::entities::*};
    let (data, entries, budget) = data().await;
    let source_files = files("execution_channels");
    let callable = data
        .callables
        .iter()
        .find(|row| {
            matches!(row,
                CallableEntity::Source { declaration, .. } if {
                    let occurrence = data.occurrences.get(*declaration).unwrap();
                    let source = data.artifacts.get(occurrence.source).unwrap();
                    source_files[&source.path][occurrence.start as usize..occurrence.end as usize]
                        .starts_with(b"def read(")
                }
            )
        })
        .unwrap();
    let expected_owner = EntityRef::Callable {
        callable: callable.id(),
    }
    .id();
    let read = data
        .occurrences
        .iter()
        .find(|o| {
            o.syntax_kind == SyntaxKind::ExprName
                && o.role == OccurrenceRole::Read
                && data
                    .owners
                    .iter()
                    .any(|owner| owner.occurrence == o.id() && owner.entity == expected_owner)
                && {
                    let source = data.artifacts.get(o.source).unwrap();
                    files("execution_channels")[&source.path][o.start as usize..o.end as usize]
                        == *b"value"
                }
        })
        .unwrap();
    let owner = entries
        .owners
        .iter()
        .find(|o| o.occurrence == read.id())
        .unwrap();
    let formal = entries.formals.iter().find(|formal| matches!(formal,
        ParameterEntity::Source { declaration } if entries.occurrences.get(*declaration).is_some_and(|o| {
            let source = entries.artifacts.get(o.source).unwrap();
            o.structural_path.starts_with(&entries.occurrences.get(owner.owner).unwrap().structural_path)
                && source_files[&source.path][o.start as usize..o.end as usize] == *b"value"
        })
    )).unwrap();
    let use_ = entries
        .uses
        .iter()
        .find(|u| u.occurrence == read.id())
        .unwrap();
    let observation = entries
        .use_observations
        .iter()
        .find(|o| o.use_ == use_.id())
        .unwrap();
    let run = entries
        .use_supports
        .iter()
        .find(|s| s.assertion == observation.id())
        .unwrap()
        .run;
    let context = entries.runs.get(run).unwrap().context;
    let request = ExpressionRequest {
        input: data.artifacts.get(read.source).unwrap().input,
        context,
        owner: owner.entity,
        expression: read.id(),
    };
    assert!(evaluate(&data, request, &budget).unwrap().is_err());
    let entry = EntryValueWitness::derive(
        &entries,
        EntryRequest {
            owner: owner.entity,
            formal: formal.id(),
            access: read.id(),
            context,
            run,
        },
        &budget,
    )
    .unwrap()
    .unwrap();
    let result = evaluate_with_entries(&data, request, &[&entry], &budget)
        .unwrap()
        .unwrap();
    assert_eq!(result.release(), ReleaseSafety::CallerRetained);
    assert_eq!(result.entry_premises(), &[entry.witness().id()]);
    assert_eq!(result.status(), entry.evidence_status());
    let value = entries
        .values
        .iter()
        .find(|row| row.use_ == use_.id())
        .unwrap();
    let value_support = entries
        .value_supports
        .iter()
        .find(|row| row.assertion == value.id())
        .unwrap();
    let access_source = EntryAccessSource::value(
        &entries,
        entry.witness().request(),
        value.id(),
        value_support.id(),
    )
    .unwrap();
    let value_entry =
        EntryValueWitness::derive_for(&entries, entry.witness().request(), &access_source, &budget)
            .unwrap()
            .unwrap();
    assert!(matches!(
        evaluate_with_entries(&data, request, &[&value_entry], &budget).unwrap(),
        Err(ObligationKind::EntryValueUnknown)
    ));
    drop(value_entry);

    let reservation = budget.reserved();
    drop(entry);
    assert_eq!(budget.reserved(), reservation);
    drop(result);
    assert!(budget.reserved() < reservation);
}

#[tokio::test]
async fn native_base_completion_consumes_independent_evaluations_and_replays_pending_order() {
    use lctx_model::domain::{
        execution::{ExactRuntimeException, completion::*, outcome::PendingOutcome},
        normalized::entities::CallableEntity,
    };
    let (data, _, budget) = data().await;
    for (function, raised) in [
        ("finally_returns", false),
        ("finally_raises", true),
        ("bare_handler", true),
    ] {
        let source_files = files("execution_channels");
        let callable=data.callables.iter().find(|c|matches!(c,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let source=data.artifacts.get(o.source).unwrap();source_files[&source.path][o.start as usize..o.end as usize].starts_with(format!("def {function}(").as_bytes())})).unwrap();
        let owner = lctx_model::domain::normalized::entities::EntityRef::Callable {
            callable: callable.id(),
        }
        .id();
        let statement = data
            .occurrences
            .iter()
            .find(|o| {
                o.syntax_kind == SyntaxKind::StmtTry
                    && data
                        .owners
                        .iter()
                        .any(|r| r.occurrence == o.id() && r.entity == owner)
            })
            .unwrap();
        let context = data
            .placements
            .iter()
            .find(|p| p.occurrence == statement.id())
            .map(|p| data.qualifications.get(p.qualification).unwrap().context)
            .unwrap();
        let input = data.artifacts.get(statement.source).unwrap().input;
        let mut earlier = Vec::new();
        for o in data.occurrences.iter().filter(|o| {
            matches!(
                o.syntax_kind,
                SyntaxKind::ExprNoneLiteral | SyntaxKind::ExprNumberLiteral
            ) && data
                .owners
                .iter()
                .any(|r| r.occurrence == o.id() && r.entity == owner)
        }) {
            earlier.push(
                evaluate(
                    &data,
                    ExpressionRequest {
                        input,
                        context,
                        owner,
                        expression: o.id(),
                    },
                    &budget,
                )
                .unwrap()
                .unwrap(),
            );
        }
        let request = CompletionRequest {
            input,
            context,
            owner,
            statement: statement.id(),
        };
        assert!(complete(&data, request, &[], &budget).unwrap().is_err());
        let refs = earlier.iter().collect::<Vec<_>>();
        let result = complete(&data, request, &refs, &budget)
            .unwrap()
            .unwrap_or_else(|reason| panic!("{function}: {reason:?}"));
        if raised {
            assert!(
                matches!(
                    result.outcome(),
                    PendingOutcome::Raise {
                        exception: ExactRuntimeException::TypeError,
                        ..
                    }
                ),
                "{function}"
            );
        } else {
            assert!(
                matches!(result.outcome(), PendingOutcome::Return { .. }),
                "{function}"
            );
        }
        assert!(!result.native_premises().is_empty());
        assert!(
            result
                .release_inputs()
                .iter()
                .all(|(_, safety)| *safety == ReleaseSafety::Closed)
        );
        let mut foreign = request;
        foreign.owner = data.refs.iter().find(|r| r.id() != owner).unwrap().id();
        assert!(complete(&data, foreign, &refs, &budget).unwrap().is_err());
    }
}

#[tokio::test]
async fn base_evaluation_shared_replay_refuses_value_disposal_status_and_coupled_membership_forgery()
 {
    use lctx_model::domain::{
        analysis::{base_evaluation::AnalysisInvocation, policy::EvidenceStatus, *},
        execution::records::*,
    };
    let (data, entry, budget) = data().await;
    let req = request(&data, "1 + 2");
    let parameters = MethodParameters {
        depth: None,
        proof_steps: None,
        work: None,
        members: None,
        seed: None,
        iterations: None,
        threshold: None,
        resolution: None,
        damping: None,
        model_catalog: None,
    };
    let (_, definition) = lctx_model::domain::execution::configuration::base_evaluation();
    let (invocation, _) =
        AnalysisInvocation::new(req.input, req.context, definition.id(), Some(req.owner), []);
    let checked = evaluate(&data, req, &budget).unwrap().unwrap();
    let mut wrong_version = definition.clone();
    wrong_version.semantic_version = ContentHash::of(b"unbound-base-version");
    assert!(
        checked
            .emit_base(&invocation, &wrong_version, &budget)
            .is_err()
    );
    let mut changed_parameters = parameters.clone();
    changed_parameters.work = Some(1);
    let mut wrong_limits = definition.clone();
    wrong_limits.parameters = changed_parameters.id();
    let (wrong_invocation, _) = AnalysisInvocation::new(
        req.input,
        req.context,
        wrong_limits.id(),
        Some(req.owner),
        [],
    );
    assert!(
        checked
            .emit_base(&wrong_invocation, &wrong_limits, &budget)
            .is_err()
    );

    for mutation in 0..6 {
        let mut output = checked
            .emit_base(&invocation, &definition, &budget)
            .unwrap();
        match mutation {
            1 => output.evaluation.boolean_value = Some(false),
            2 => output.evaluation.release = ReleaseSafety::CallerRetained,
            3 => output.evaluation.status = EvidenceStatus::FixtureChecked,
            4 => {
                let mut q = data
                    .qualifications
                    .get(output.evaluation.qualification)
                    .unwrap()
                    .clone();
                q.approximation = lctx_model::domain::assertion::Approximation::Over;
                output.evaluation.qualification = q.id();
            }
            5 => {
                output.sources.pop();
                output.members.pop();
                let mut sink = KeySink::new("base-evaluation-sources");
                for source in &output.sources {
                    source.id().encode(&mut sink);
                }
                output.evaluation.sources = sink.finish();
            }
            _ => {}
        }
        let invariant = base_invariants().remove(0);
        let mut check = (invariant.create)(&budget);
        macro_rules! visit_data {($($field:ident:$ty:ty,)*)=>{$(let input=ValidationInput::of::<$ty>(&["id"]);let input=if stages::is_vocabulary(input.name()){input.at_epoch(stages::PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(visit_data);
        macro_rules! visit_entry {($($field:ident:$ty:ty,)*)=>{$(let input=ValidationInput::of::<$ty>(&["id"]);let input=if stages::is_vocabulary(input.name()){input.at_epoch(stages::PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&entry.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(visit_entry);
        check
            .visit(
                AnalysisInvocation::NAME,
                &AnalysisInvocation::encode(std::slice::from_ref(&invocation)).unwrap(),
            )
            .unwrap();
        check
            .visit(
                AnalysisDefinition::NAME,
                &AnalysisDefinition::encode(std::slice::from_ref(&definition)).unwrap(),
            )
            .unwrap();
        check
            .visit(
                ExpressionEvaluation::NAME,
                &ExpressionEvaluation::encode(&[output.evaluation]).unwrap(),
            )
            .unwrap();
        check
            .visit(
                EvaluationSource::NAME,
                &<EvaluationSource as Record>::encode(&output.sources).unwrap(),
            )
            .unwrap();
        check
            .visit(
                EvaluationMember::NAME,
                &EvaluationMember::encode(&output.members).unwrap(),
            )
            .unwrap();
        check
            .visit(
                EvaluationOperand::NAME,
                &EvaluationOperand::encode(&output.operands).unwrap(),
            )
            .unwrap();
        let result = check.finish();
        if mutation == 0 {
            result.unwrap();
        } else {
            assert!(result.is_err(), "mutation {mutation}");
        }
    }
}

#[tokio::test]
async fn base_completion_stored_replay_rejects_pending_order_and_coupled_proof_forgery() {
    use lctx_model::domain::{
        analysis::{self, policy::EvidenceStatus, *},
        execution::{completion::*, completion_records::*, records::*},
        normalized::entities::CallableEntity,
    };
    let (data, entry, budget) = data().await;
    let source_files = files("execution_channels");
    let callable=data.callables.iter().find(|c|matches!(c,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let source=data.artifacts.get(o.source).unwrap();source_files[&source.path][o.start as usize..o.end as usize].starts_with(b"def finally_returns(")})).unwrap();
    let owner = lctx_model::domain::normalized::entities::EntityRef::Callable {
        callable: callable.id(),
    }
    .id();
    let statement = data
        .occurrences
        .iter()
        .find(|o| {
            o.syntax_kind == SyntaxKind::StmtTry
                && data
                    .owners
                    .iter()
                    .any(|r| r.occurrence == o.id() && r.entity == owner)
        })
        .unwrap();
    let context = data
        .placements
        .iter()
        .find(|p| p.occurrence == statement.id())
        .map(|p| data.qualifications.get(p.qualification).unwrap().context)
        .unwrap();
    let input = data.artifacts.get(statement.source).unwrap().input;
    let (_, definition) = lctx_model::domain::execution::configuration::base_evaluation();
    let (_, completion_definition) =
        lctx_model::domain::execution::configuration::base_completion();
    let (eval_invocation, _) = analysis::base_evaluation::AnalysisInvocation::new(
        input,
        context,
        definition.id(),
        Some(owner),
        [],
    );
    let (invocation, _) = analysis::base_completion::AnalysisInvocation::new(
        input,
        context,
        completion_definition.id(),
        Some(owner),
        [],
    );
    let mut checked = Vec::new();
    let mut earlier = Vec::new();
    for o in data.occurrences.iter().filter(|o| {
        matches!(
            o.syntax_kind,
            SyntaxKind::ExprNoneLiteral | SyntaxKind::ExprNumberLiteral
        ) && data
            .owners
            .iter()
            .any(|r| r.occurrence == o.id() && r.entity == owner)
    }) {
        let proof = evaluate(
            &data,
            ExpressionRequest {
                input,
                context,
                owner,
                expression: o.id(),
            },
            &budget,
        )
        .unwrap()
        .unwrap();
        earlier.push(
            proof
                .emit_base(&eval_invocation, &definition, &budget)
                .unwrap(),
        );
        checked.push(proof);
    }
    let refs = checked.iter().collect::<Vec<_>>();
    let completion = complete(
        &data,
        CompletionRequest {
            input,
            context,
            owner,
            statement: statement.id(),
        },
        &refs,
        &budget,
    )
    .unwrap()
    .unwrap();
    let entered_evaluations = completion
        .evaluated_expressions()
        .iter()
        .map(|expression| CompletionEvaluation {
            evaluation: &earlier
                .iter()
                .find(|r| r.evaluation.expression == *expression)
                .unwrap()
                .evaluation,
            invocation: &eval_invocation,
        })
        .collect::<Vec<_>>();
    for mutation in 0..6 {
        let mut output = completion
            .emit_base(
                &invocation,
                &completion_definition,
                &entered_evaluations,
                &budget,
            )
            .unwrap();
        match mutation {
            1 => {
                output.outcome = CompletionOutcome::Normal;
                output.completion.outcome = output.outcome.id();
            }
            2 => output.completion.status = EvidenceStatus::FixtureChecked,
            3 => {
                output.entered.reverse();
                for (ordinal, row) in output.entered.iter_mut().enumerate() {
                    row.ordinal = ordinal as i64;
                }
                let mut sink = KeySink::new("base-completion-entered");
                for row in &output.entered {
                    row.statement.encode(&mut sink);
                }
                output.completion.entered = sink.finish();
            }
            4 => {
                output.sources.pop();
                output.members.pop();
                let mut sink = KeySink::new("base-completion-sources");
                for row in &output.sources {
                    row.id().encode(&mut sink);
                }
                output.completion.sources = sink.finish();
            }
            5 => {
                let mut q = data
                    .qualifications
                    .get(output.completion.qualification)
                    .unwrap()
                    .clone();
                q.modality = attribution::Modality::Potential;
                output.completion.qualification = q.id();
            }
            _ => {}
        }
        // Re-key every child with the forged conclusion. Refusal must come from recomputation,
        // rather than an incidental dangling link caused by changing the conclusion identity.
        for member in &mut output.members {
            member.completion = output.completion.id();
        }
        for entered in &mut output.entered {
            entered.completion = output.completion.id();
        }
        let invariant = completion_invariants().remove(0);
        let mut check = (invariant.create)(&budget);
        macro_rules! visit_data {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(visit_data);
        macro_rules! visit_entry {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&entry.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(visit_entry);
        check
            .visit(
                AnalysisDefinition::NAME,
                &AnalysisDefinition::encode(&[definition.clone(), completion_definition.clone()])
                    .unwrap(),
            )
            .unwrap();
        check
            .visit(
                analysis::base_evaluation::AnalysisInvocation::NAME,
                &analysis::base_evaluation::AnalysisInvocation::encode(std::slice::from_ref(
                    &eval_invocation,
                ))
                .unwrap(),
            )
            .unwrap();
        check
            .visit(
                analysis::base_completion::AnalysisInvocation::NAME,
                &analysis::base_completion::AnalysisInvocation::encode(std::slice::from_ref(
                    &invocation,
                ))
                .unwrap(),
            )
            .unwrap();
        for rows in &earlier {
            check
                .visit(
                    ExpressionEvaluation::NAME,
                    &ExpressionEvaluation::encode(std::slice::from_ref(&rows.evaluation)).unwrap(),
                )
                .unwrap();
            check
                .visit(
                    EvaluationSource::NAME,
                    &<EvaluationSource as Record>::encode(&rows.sources).unwrap(),
                )
                .unwrap();
            check
                .visit(
                    EvaluationMember::NAME,
                    &EvaluationMember::encode(&rows.members).unwrap(),
                )
                .unwrap();
            check
                .visit(
                    EvaluationOperand::NAME,
                    &EvaluationOperand::encode(&rows.operands).unwrap(),
                )
                .unwrap();
        }
        check
            .visit(
                StatementCompletion::NAME,
                &StatementCompletion::encode(&[output.completion]).unwrap(),
            )
            .unwrap();
        check
            .visit(
                CompletionOutcome::NAME,
                &<CompletionOutcome as Record>::encode(&[output.outcome]).unwrap(),
            )
            .unwrap();
        check
            .visit(
                CompletionSource::NAME,
                &<CompletionSource as Record>::encode(&output.sources).unwrap(),
            )
            .unwrap();
        check
            .visit(
                CompletionMember::NAME,
                &CompletionMember::encode(&output.members).unwrap(),
            )
            .unwrap();
        check
            .visit(
                EnteredStatement::NAME,
                &EnteredStatement::encode(&output.entered).unwrap(),
            )
            .unwrap();
        let result = check.finish();
        if mutation == 0 {
            result.unwrap();
        } else {
            assert!(result.is_err(), "mutation {mutation}");
        }
    }
}

#[tokio::test]
async fn source_body_is_under_entry_and_preserves_required_frame_cleanup() {
    use lctx_model::domain::{
        execution::{body::*, completion::*, outcome::PendingOutcome},
        normalized::entities::CallableEntity,
    };
    let (data, _, budget) = data().await;
    let source_files = files("execution_channels");
    for function in ["body_pass", "body_stops"] {
        let callable=data.callables.iter().find(|c|matches!(c,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let source=data.artifacts.get(o.source).unwrap();source_files[&source.path][o.start as usize..o.end as usize].starts_with(format!("def {function}(").as_bytes())})).unwrap();
        let CallableEntity::Source { declaration, .. } = callable else {
            unreachable!()
        };
        let owner = lctx_model::domain::normalized::entities::EntityRef::Callable {
            callable: callable.id(),
        }
        .id();
        let q = data
            .placements
            .iter()
            .find(|p| p.occurrence == *declaration)
            .map(|p| data.qualifications.get(p.qualification).unwrap())
            .unwrap();
        let context = q.context;
        let input = data
            .artifacts
            .get(data.occurrences.get(*declaration).unwrap().source)
            .unwrap()
            .input;
        let first = data
            .placements
            .iter()
            .find(|p| {
                p.parent == Some(*declaration)
                    && p.field == lctx_model::domain::lexical::SyntaxField::Body
                    && p.ordinal == 0
            })
            .unwrap()
            .occurrence;
        let mut evaluations = Vec::new();
        for o in data.occurrences.iter().filter(|o| {
            o.syntax_kind == SyntaxKind::ExprNumberLiteral
                && data
                    .owners
                    .iter()
                    .any(|r| r.occurrence == o.id() && r.entity == owner)
        }) {
            evaluations.push(
                evaluate(
                    &data,
                    ExpressionRequest {
                        input,
                        context,
                        owner,
                        expression: o.id(),
                    },
                    &budget,
                )
                .unwrap()
                .unwrap(),
            );
        }
        let refs = evaluations.iter().collect::<Vec<_>>();
        let statement = complete(
            &data,
            CompletionRequest {
                input,
                context,
                owner,
                statement: first,
            },
            &refs,
            &budget,
        )
        .unwrap()
        .unwrap();
        let request = SourceBodyRequest {
            input,
            context,
            callee: owner,
        };
        let body = complete_body(&data, request, &[&statement], &budget)
            .unwrap()
            .unwrap();
        assert_eq!(body.frame_obligation(), ObligationKind::FrameExitCleanup);
        assert_eq!(body.entered_statements(), &[first]);
        assert_eq!(body.outcome().is_normal(), function == "body_pass");
        if function == "body_stops" {
            assert!(matches!(body.outcome(), PendingOutcome::Return { .. }));
        }
        assert!(
            complete_body(&data, request, &[], &budget)
                .unwrap()
                .is_err()
        );
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(
            complete_body(&data, request, &[&statement], &tiny),
            Err(ModelError::Resource { .. })
        ));
    }
}

#[tokio::test]
async fn base_producer_reconciles_the_entire_root_inventory_and_actual_request_profile() {
    use lctx_model::domain::{
        analysis::{self, base_evaluation as publication},
        conditions::entry::{EntryAccessSource, EntryValueWitness},
        execution::{production::*, read_channels::*, read_dynamic::*, read_fields::*, records::*},
        normalized::Rows,
        stages::Profile,
    };
    let (data, entry, budget) = data().await;
    let req = request(&data, "1 + 2");
    let (_, definition) = lctx_model::domain::execution::configuration::base_evaluation();
    let mut local_definition = definition.clone();
    local_definition.method = analysis::AnalysisMethod::LocalTransfers;
    let (local, _) = analysis::local::AnalysisInvocation::new(
        req.input,
        req.context,
        local_definition.id(),
        None,
        [],
    );
    let parent = publication::InvocationSource::Local {
        invocation: local.id(),
    };
    let (invocation, _) = publication::AnalysisInvocation::new(
        req.input,
        req.context,
        definition.id(),
        None,
        [parent.id()],
    );
    let entries = Rows::<EntryValueWitness>::new(&budget);
    let sources = Rows::<EntryAccessSource>::new(&budget);
    let output = evaluate_all(
        &data,
        &entry,
        &entries,
        &sources,
        &invocation,
        &definition,
        Profile::Behavioral,
        &budget,
    )
    .unwrap();
    assert!(output.run.evaluated > 0 && output.run.refused > 0);
    assert_eq!(output.outcome.status, analysis::AnalysisStatus::Partial);
    assert!(
        output
            .evaluations
            .iter()
            .any(|row| row.expression == req.expression)
    );
    let invariant = lctx_model::domain::validation::invariants_for::<EvaluationRun>()
        .iter()
        .find(|check| check.name == "base_evaluation_inventory")
        .unwrap()
        .clone();
    let check = |shrink: bool, forge_outcome: bool, omit_frame: bool| {
        let mut checker = (invariant.create)(&budget);
        macro_rules! put {
            ($ty:ty,$values:expr) => {
                checker
                    .visit(
                        <$ty>::NAME,
                        &<$ty as Record>::encode(($values).as_ref()).unwrap(),
                    )
                    .unwrap();
            };
        }
        macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{$(put!($ty,data.$field.iter().cloned().collect::<Vec<_>>());)*};}
        lctx_model::execution_evaluation_inputs!(inputs);
        macro_rules! entry_inputs {($($field:ident:$ty:ty,)*)=>{$(put!($ty,entry.$field.iter().cloned().collect::<Vec<_>>());)*};}
        lctx_model::entry_value_inputs!(entry_inputs);
        put!(
            analysis::local::AnalysisInvocation,
            std::slice::from_ref(&local)
        );
        put!(
            publication::AnalysisInvocation,
            if omit_frame {
                vec![]
            } else {
                vec![invocation.clone()]
            }
        );
        put!(
            analysis::AnalysisDefinition,
            std::slice::from_ref(&definition)
        );
        let mut run = output.run.clone();
        let mut boundaries = output.boundaries.iter().cloned().collect::<Vec<_>>();
        if shrink {
            boundaries.pop();
            run.refused -= 1;
        }
        let mut outcome = output.outcome.clone();
        if forge_outcome {
            outcome.status = analysis::AnalysisStatus::Completed;
            outcome.reason = None;
        }
        put!(EvaluationRun, if omit_frame { vec![] } else { vec![run] });
        put!(
            EvaluationBoundary,
            if omit_frame { vec![] } else { boundaries }
        );
        put!(
            publication::AnalysisOutcome,
            if omit_frame { vec![] } else { vec![outcome] }
        );
        put!(
            ExpressionEvaluation,
            if omit_frame {
                vec![]
            } else {
                output.evaluations.iter().cloned().collect::<Vec<_>>()
            }
        );
        put!(
            EvaluationSource,
            if omit_frame {
                vec![]
            } else {
                output.sources.iter().cloned().collect::<Vec<_>>()
            }
        );
        put!(
            EvaluationMember,
            if omit_frame {
                vec![]
            } else {
                output.members.iter().cloned().collect::<Vec<_>>()
            }
        );
        put!(
            EvaluationOperand,
            if omit_frame {
                vec![]
            } else {
                output.operands.iter().cloned().collect::<Vec<_>>()
            }
        );
        macro_rules! read_outputs {($($field:ident:$ty:ty,)*)=>{$(put!($ty,
            if omit_frame { vec![] } else { output.reads.$field.iter().cloned().collect::<Vec<_>>() }
        );)*};}
        read_outputs! {
            reads:ReadObservation,
            dependencies:ReadDependency,
            attributes:AttributeRead,
            formals:FormalReadAssessment,
            dynamic:DynamicAccessObservation,
            dynamic_premises:DynamicAccessPremise,
        }
        macro_rules! field_outputs {($($field:ident:$ty:ty,)*)=>{$(put!($ty,
            if omit_frame { vec![] } else { output.reads.fields.$field.iter().cloned().collect::<Vec<_>>() }
        );)*};}
        field_outputs! {
            locations:FieldLocationObservation,
            assessments:FieldReadAssessment,
            globals:GlobalClassInspection,
            global_assessments:GlobalFieldReadAssessment,
        }
        checker.finish()
    };
    check(false, false, false).unwrap();
    assert!(check(true, false, false).is_err());
    assert!(check(false, true, false).is_err());
    assert!(check(false, false, true).is_err());
    let unrequested = evaluate_all(
        &data,
        &entry,
        &entries,
        &sources,
        &invocation,
        &definition,
        Profile::Catalog,
        &budget,
    )
    .unwrap();
    assert!(!unrequested.run.requested);
    assert!(unrequested.evaluations.is_empty() && unrequested.boundaries.is_empty());
    assert_eq!(
        unrequested.outcome.status,
        analysis::AnalysisStatus::NotRequested
    );
    let profile_check =
        lctx_model::domain::validation::publication_checks_for::<EvaluationRun>()[0].clone();
    let verify_profile = |profile| {
        let mut check = (profile_check.create)(&budget);
        check
            .visit(
                EvaluationRun::NAME,
                &<EvaluationRun as Record>::encode(std::slice::from_ref(&unrequested.run)).unwrap(),
            )
            .unwrap();
        check
            .visit(
                publication::AnalysisInvocation::NAME,
                &<publication::AnalysisInvocation as Record>::encode(std::slice::from_ref(
                    &invocation,
                ))
                .unwrap(),
            )
            .unwrap();
        check.finish(&[], profile)
    };
    verify_profile(Profile::Catalog).unwrap();
    assert!(verify_profile(Profile::Behavioral).is_err());
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(matches!(
        evaluate_all(
            &data,
            &entry,
            &entries,
            &sources,
            &invocation,
            &definition,
            Profile::Behavioral,
            &tiny
        ),
        Err(ModelError::Resource { .. })
    ));
}

#[tokio::test]
async fn completion_producer_reconciles_statement_inventory_and_finalizer_order() {
    use lctx_model::domain::{
        analysis::{self, base_completion as publication, base_evaluation as earlier},
        conditions::entry::{EntryAccessSource, EntryValueWitness},
        execution::{
            body_records::*, completion_production::*, completion_records::*, production,
            records::*,
        },
        normalized::Rows,
        stages::Profile,
    };
    let (data, entry, budget) = data().await;
    let req = request(&data, "1 + 2");
    let (_, eval_definition) = lctx_model::domain::execution::configuration::base_evaluation();
    let (_, definition) = lctx_model::domain::execution::configuration::base_completion();
    let (base, _) =
        earlier::AnalysisInvocation::new(req.input, req.context, eval_definition.id(), None, []);
    let parent = publication::InvocationSource::BaseEvaluation {
        invocation: base.id(),
    };
    let (invocation, _) = publication::AnalysisInvocation::new(
        req.input,
        req.context,
        definition.id(),
        None,
        [parent.id()],
    );
    let evaluations = production::evaluate_all(
        &data,
        &entry,
        &Rows::<EntryValueWitness>::new(&budget),
        &Rows::<EntryAccessSource>::new(&budget),
        &base,
        &eval_definition,
        Profile::Behavioral,
        &budget,
    )
    .unwrap();
    let mut captured = CompletedEvaluations::new(&budget);
    macro_rules! put_captured {
        ($ty:ty,$rows:expr) => {
            captured
                .visit(
                    <$ty>::NAME,
                    &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                )
                .unwrap();
        };
    }
    macro_rules! facts {($($field:ident:$ty:ty,)*)=>{$(put_captured!($ty,data.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::execution_evaluation_inputs!(facts);
    macro_rules! entry_facts {($($field:ident:$ty:ty,)*)=>{$(put_captured!($ty,entry.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::entry_value_inputs!(entry_facts);
    put_captured!(
        analysis::AnalysisDefinition,
        [eval_definition.clone(), definition.clone()]
    );
    put_captured!(earlier::AnalysisInvocation, std::slice::from_ref(&base));
    put_captured!(
        ExpressionEvaluation,
        evaluations.evaluations.iter().cloned().collect::<Vec<_>>()
    );
    put_captured!(
        EvaluationSource,
        evaluations.sources.iter().cloned().collect::<Vec<_>>()
    );
    put_captured!(
        EvaluationMember,
        evaluations.members.iter().cloned().collect::<Vec<_>>()
    );
    put_captured!(
        EvaluationOperand,
        evaluations.operands.iter().cloned().collect::<Vec<_>>()
    );
    let output = complete_all(
        &captured,
        &invocation,
        &definition,
        Profile::Behavioral,
        &budget,
    )
    .unwrap();
    assert!(
        output.run.completed > 0
            && output.run.refused > 0
            && output.run.bodied > 0
            && output.run.body_refused > 0
    );
    assert_eq!(output.outcome.status, analysis::AnalysisStatus::Partial);
    let source_files = files("execution_channels");
    for (name, returns) in [("finally_returns", true), ("finally_raises", false)] {
        let callable=data.callables.iter().find(|row|matches!(row,lctx_model::domain::normalized::entities::CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let artifact=data.artifacts.get(o.source).unwrap();source_files[&artifact.path][o.start as usize..o.end as usize].starts_with(format!("def {name}(").as_bytes())})).unwrap();
        let owner = lctx_model::domain::normalized::entities::EntityRef::Callable {
            callable: callable.id(),
        }
        .id();
        let row = output
            .completions
            .iter()
            .find(|row| {
                row.owner == owner
                    && data.occurrences.get(row.statement).unwrap().syntax_kind
                        == SyntaxKind::StmtTry
            })
            .unwrap();
        let outcome = output.outcomes.get(row.outcome).unwrap();
        assert_eq!(matches!(outcome, CompletionOutcome::Return { .. }), returns);
        if !returns {
            assert!(matches!(
                outcome,
                CompletionOutcome::Raise {
                    exception: lctx_model::domain::execution::ExactRuntimeException::TypeError,
                    ..
                }
            ));
        }
    }
    let invariant = lctx_model::domain::validation::invariants_for::<CompletionRun>()
        .iter()
        .find(|i| i.name == "base_completion_inventory")
        .unwrap()
        .clone();
    let verify = |shrink: bool, omit_frame: bool, forge_body: bool| {
        let mut check = (invariant.create)(&budget);
        macro_rules! put {
            ($ty:ty,$rows:expr) => {
                check
                    .visit(
                        <$ty>::NAME,
                        &<$ty as Record>::encode(($rows).as_ref()).unwrap(),
                    )
                    .unwrap();
            };
        }
        macro_rules! facts {($($field:ident:$ty:ty,)*)=>{$(put!($ty,data.$field.iter().cloned().collect::<Vec<_>>());)*};}
        lctx_model::execution_evaluation_inputs!(facts);
        macro_rules! entry_facts {($($field:ident:$ty:ty,)*)=>{$(put!($ty,entry.$field.iter().cloned().collect::<Vec<_>>());)*};}
        lctx_model::entry_value_inputs!(entry_facts);
        put!(
            analysis::AnalysisDefinition,
            [eval_definition.clone(), definition.clone()]
        );
        put!(earlier::AnalysisInvocation, std::slice::from_ref(&base));
        put!(
            publication::AnalysisInvocation,
            if omit_frame {
                vec![]
            } else {
                vec![invocation.clone()]
            }
        );
        put!(
            ExpressionEvaluation,
            evaluations.evaluations.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            EvaluationSource,
            evaluations.sources.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            EvaluationMember,
            evaluations.members.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            EvaluationOperand,
            evaluations.operands.iter().cloned().collect::<Vec<_>>()
        );
        let mut run = output.run.clone();
        let mut boundaries = output.boundaries.iter().cloned().collect::<Vec<_>>();
        if shrink {
            boundaries.pop();
            run.refused -= 1;
        }
        put!(CompletionRun, if omit_frame { vec![] } else { vec![run] });
        put!(
            CompletionBoundary,
            if omit_frame { vec![] } else { boundaries }
        );
        put!(
            publication::AnalysisOutcome,
            if omit_frame {
                vec![]
            } else {
                vec![output.outcome.clone()]
            }
        );
        macro_rules! output {($($field:ident:$ty:ty,)*)=>{$(put!($ty,if omit_frame{vec![]}else{output.$field.iter().cloned().collect::<Vec<_>>()});)*};}
        output! {completions:StatementCompletion,outcomes:CompletionOutcome,sources:CompletionSource,members:CompletionMember,entered:EnteredStatement,body_sources:BodySource,body_boundaries:BodyBoundary,}
        let mut bodies = output.bodies.iter().cloned().collect::<Vec<_>>();
        let mut members = output.body_members.iter().cloned().collect::<Vec<_>>();
        let mut releases = output.body_releases.iter().cloned().collect::<Vec<_>>();
        if forge_body {
            let previous = bodies[0].id();
            bodies[0].status = analysis::policy::EvidenceStatus::FixtureChecked;
            let changed = bodies[0].id();
            for row in &mut members {
                if row.body == previous {
                    row.body = changed;
                }
            }
            for row in &mut releases {
                if row.body == previous {
                    row.body = changed;
                }
            }
        }
        put!(
            SourceBodyCompletion,
            if omit_frame { vec![] } else { bodies }
        );
        put!(BodyMember, if omit_frame { vec![] } else { members });
        put!(BodyReleaseInput, if omit_frame { vec![] } else { releases });
        check.finish()
    };
    verify(false, false, false).unwrap();
    assert!(verify(true, false, false).is_err());
    assert!(verify(false, true, false).is_err());
    assert!(verify(false, false, true).is_err());
    let unrequested = complete_all(
        &captured,
        &invocation,
        &definition,
        Profile::Catalog,
        &budget,
    )
    .unwrap();
    assert!(!unrequested.run.requested);
    assert_eq!(
        unrequested.outcome.status,
        analysis::AnalysisStatus::NotRequested
    );
    assert!(unrequested.completions.is_empty() && unrequested.boundaries.is_empty());
    let profile_check =
        lctx_model::domain::validation::publication_checks_for::<CompletionRun>()[0].clone();
    let verify_profile = |profile| {
        let mut check = (profile_check.create)(&budget);
        check
            .visit(
                CompletionRun::NAME,
                &CompletionRun::encode(std::slice::from_ref(&unrequested.run)).unwrap(),
            )
            .unwrap();
        check
            .visit(
                publication::AnalysisInvocation::NAME,
                &publication::AnalysisInvocation::encode(std::slice::from_ref(&invocation))
                    .unwrap(),
            )
            .unwrap();
        check.finish(&[], profile)
    };
    verify_profile(Profile::Catalog).unwrap();
    assert!(verify_profile(Profile::Behavioral).is_err());
}

#[tokio::test]
async fn field_location_does_not_establish_exact_attribute_readiness() {
    let (data, _, budget) = data().await;
    assert!(matches!(
        evaluate(&data, request(&data, "value.field"), &budget).unwrap(),
        Err(ObligationKind::HeapFieldStateUnavailable)
    ));
}

#[tokio::test]
async fn body_under_entry_refuses_async_and_unreachable_generator_yield() {
    use lctx_model::domain::{
        execution::{body::*, completion::*},
        normalized::entities::*,
    };
    let (data, _, budget) = data().await;
    let source_files = files("execution_channels");
    for prefix in ["async def async_body(", "def generator_body("] {
        let callable=data.callables.iter().find(|row|matches!(row,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let artifact=data.artifacts.get(o.source).unwrap();source_files[&artifact.path][o.start as usize..o.end as usize].starts_with(prefix.as_bytes())})).unwrap();
        let CallableEntity::Source { declaration, .. } = callable else {
            unreachable!()
        };
        let owner = EntityRef::Callable {
            callable: callable.id(),
        }
        .id();
        let context = data
            .placements
            .iter()
            .find(|row| row.occurrence == *declaration)
            .map(|row| data.qualifications.get(row.qualification).unwrap().context)
            .unwrap();
        let input = data
            .artifacts
            .get(data.occurrences.get(*declaration).unwrap().source)
            .unwrap()
            .input;
        let first = data
            .placements
            .iter()
            .find(|row| {
                row.parent == Some(*declaration)
                    && row.field == lctx_model::domain::lexical::SyntaxField::Body
                    && row.ordinal == 0
            })
            .unwrap()
            .occurrence;
        let mut checked = Vec::new();
        for row in data.occurrences.iter().filter(|row| {
            row.syntax_kind == SyntaxKind::ExprNumberLiteral
                && data
                    .owners
                    .iter()
                    .any(|owned| owned.entity == owner && owned.occurrence == row.id())
        }) {
            checked.push(
                evaluate(
                    &data,
                    ExpressionRequest {
                        input,
                        context,
                        owner,
                        expression: row.id(),
                    },
                    &budget,
                )
                .unwrap()
                .unwrap(),
            );
        }
        let refs = checked.iter().collect::<Vec<_>>();
        let statement = complete(
            &data,
            CompletionRequest {
                input,
                context,
                owner,
                statement: first,
            },
            &refs,
            &budget,
        )
        .unwrap()
        .unwrap();
        assert!(
            matches!(
                complete_body(
                    &data,
                    SourceBodyRequest {
                        input,
                        context,
                        callee: owner
                    },
                    &[&statement],
                    &budget
                )
                .unwrap(),
                Err(ObligationKind::ScopeBoundary)
            ),
            "{prefix}"
        );
    }
}
