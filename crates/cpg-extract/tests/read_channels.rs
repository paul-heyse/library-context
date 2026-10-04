#[path = "fixtures/transfer_composition.rs"]
mod fixture;
#[path = "fixtures/source_data.rs"]
mod source_fixture;
#[path = "fixtures/read_diagnostics.rs"]
mod read_diagnostics;
use lctx_model::domain::{
    analysis,
    execution::{self, read_channels::*},
    normalized::Rows,
    source::*,
    *,
};
fn text(f: &fixture::NativeFixture, id: Id<Occurrence>) -> String {
    let row = f.data.occurrences.get(id).unwrap();
    let bytes = f.source_bytes(row.source);
    String::from_utf8(bytes[row.start as usize..row.end as usize].to_vec()).unwrap()
}
#[tokio::test]
async fn actual_read_inventory_and_complete_negative_are_replayed() {
    let f = fixture::native_from("transfer_alternatives").await;
    let data = source_fixture::data(&f);
    let input = data
        .flow
        .artifacts
        .iter()
        .find(|a| a.path == "transferpkg/__init__.py")
        .unwrap()
        .input;
    let run = data
        .flow
        .runs
        .get(
            data.flow
                .coverage
                .iter()
                .find(|c| {
                    c.family == attribution::FactFamily::Flow
                        && c.run.is_some_and(|r| {
                            data.flow.runs.get(r).is_some_and(|r| r.input == input)
                        })
                })
                .unwrap()
                .run
                .unwrap(),
        )
        .unwrap();
    let (_, definition) = execution::configuration::base_evaluation();
    let (local, _) =
        analysis::local::AnalysisInvocation::new(run.input, run.context, definition.id(), None, []);
    let parent = analysis::base_evaluation::InvocationSource::Local {
        invocation: local.id(),
    };
    let (invocation, _) = analysis::base_evaluation::AnalysisInvocation::new(
        run.input,
        run.context,
        definition.id(),
        None,
        [parent.id()],
    );
    let records = execution::production::evaluate_all(
        &data.evaluation,
        &data.flow,
        &Rows::new(&f.budget),
        &Rows::new(&f.budget),
        &invocation,
        &definition,
        stages::Profile::Behavioral,
        &f.budget,
    )
    .unwrap();
    assert!(!records.reads.reads.is_empty());
    assert!(!records.reads.dependencies.is_empty());
    assert!(!records.reads.attributes.is_empty());
    let owner_text = |id: Id<normalized::entities::EntityRef>| {
        let normalized::entities::EntityRef::Callable { callable } =
            data.flow.refs.get(id).unwrap()
        else {
            panic!("noncallable")
        };
        let normalized::entities::CallableEntity::Source { declaration, .. } =
            data.flow.callables.get(*callable).unwrap()
        else {
            panic!("nonsource")
        };
        text(&f, *declaration)
    };
    let ignored=records.reads.formals.iter().find(|row|matches!(data.flow.formals.get(row.formal),Some(normalized::entities::ParameterEntity::Source{declaration})if text(&f,*declaration)=="ignored")&&owner_text(row.owner).starts_with("def unused(")).unwrap();
    assert_eq!(
        ignored.status,
        ReadAssessment::CompleteNoReadUnderModel,
        "{ignored:?}"
    );
    let value=records.reads.formals.iter().find(|row|matches!(data.flow.formals.get(row.formal),Some(normalized::entities::ParameterEntity::Source{declaration})if text(&f,*declaration)=="value")&&owner_text(row.owner).starts_with("def unused(")).unwrap();
    assert_eq!(value.status, ReadAssessment::ObservedRead);
    if !records.reads.reads.iter().any(|r| r.location == ReadLocation::DeclaredGlobal) {
        read_diagnostics::dump(&f, &data, &records.reads);
    }
    assert!(
        records
            .reads
            .reads
            .iter()
            .any(|r| r.location == ReadLocation::DeclaredGlobal)
    );
    assert!(
        records
            .reads
            .reads
            .iter()
            .any(|r| r.execution_region.is_some() && r.region_premise.is_some())
    );
    let stable_class=data.evaluation.classes.iter().find(|c|matches!(c,normalized::entities::ClassEntity::Source{declaration}if text(&f,*declaration).starts_with("class Stable:"))).unwrap().id();
    let choice=data.evaluation.classes.iter().find(|c|matches!(c,normalized::entities::ClassEntity::Source{declaration}if text(&f,*declaration).starts_with("class ChoiceA:"))).unwrap().id();
    let known = records
        .reads
        .dynamic
        .iter()
        .filter(|d| d.declared_class == Some(stable_class))
        .collect::<Vec<_>>();
    assert_eq!(known.len(), 2);
    assert!(
        known
            .iter()
            .any(|d| d.kind == execution::read_dynamic::DynamicKind::GetAttr)
    );
    assert!(
        known
            .iter()
            .any(|d| d.kind == execution::read_dynamic::DynamicKind::Dictionary)
    );
    assert!(records.reads.dynamic.iter().any(|d|text(&f,d.site)=="getattr(fixed, name)"&&d.declared_class==Some(choice)));
    assert_eq!(
        records
            .reads
            .dynamic
            .iter()
            .filter(|d| d.declared_class.is_none())
            .count(),
        6
    );
    assert!(
        records
            .reads
            .dynamic
            .iter()
            .filter(|d| d.declared_class.is_none())
            .all(
                |d| d.inspection == execution::read_dynamic::ClassInspection::Unknown
                    && d.reason == Some(obligation::ObligationKind::DynamicAccess)
            )
    );
    let unrelated=data.evaluation.classes.iter().find(|c|matches!(c,normalized::entities::ClassEntity::Source{declaration}if text(&f,*declaration).starts_with("class Unrelated:"))).unwrap().id();
    let negative = records
        .reads
        .fields
        .assessments
        .iter()
        .find(|row| row.class == unrelated && row.name == "unread")
        .unwrap();
    assert_eq!(negative.status, ReadAssessment::Unknown);
    assert_eq!(
        negative.reason,
        Some(obligation::ObligationKind::DynamicAccess)
    );
    // Recomputed admission and native input inventory, rather than output counts, own completeness.
    let validate = |shrink: bool,
                    forge: bool,
                    dynamic_forge: bool,
                    dynamic_shrink: bool,
                    field_forge: bool,
                    field_shrink: bool| {
        let invariant = execution::production::EvaluationRun::invariants().remove(0);
        let mut check = (invariant.create)(&f.budget);
        for (name, batch) in f.tables.lock().unwrap().iter() {
            check.visit(name, batch).unwrap();
        }
        macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&data.flow.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit(<$ty>::NAME,&batch).unwrap();)*};}
        lctx_model::entry_value_inputs!(raw);
        macro_rules! eval{($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&data.evaluation.$field.iter().cloned().collect::<Vec<_>>()).unwrap();check.visit(<$ty>::NAME,&batch).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(eval);
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
            analysis::local::AnalysisInvocation,
            std::slice::from_ref(&local)
        );
        put!(
            analysis::base_evaluation::AnalysisInvocation,
            std::slice::from_ref(&invocation)
        );
        put!(
            analysis::AnalysisDefinition,
            std::slice::from_ref(&definition)
        );
        put!(
            execution::production::EvaluationRun,
            std::slice::from_ref(&records.run)
        );
        put!(
            analysis::base_evaluation::AnalysisOutcome,
            std::slice::from_ref(&records.outcome)
        );
        put!(
            execution::records::ExpressionEvaluation,
            records.evaluations.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::records::EvaluationSource,
            records.sources.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::records::EvaluationMember,
            records.members.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::records::EvaluationOperand,
            records.operands.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::production::EvaluationBoundary,
            records.boundaries.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            execution::read_dynamic::DynamicAccessObservation,
            records
                .reads
                .dynamic
                .iter()
                .filter(|row| !dynamic_shrink || row.declared_class != Some(stable_class))
                .cloned()
                .map(|mut row| {
                    if dynamic_forge && row.declared_class == Some(stable_class) {
                        row.declared_class = Some(choice);
                        row.status = analysis::policy::EvidenceStatus::FixtureChecked;
                    }
                    row
                })
                .collect::<Vec<_>>()
        );
        put!(
            execution::read_dynamic::DynamicAccessPremise,
            records
                .reads
                .dynamic_premises
                .iter()
                .filter(|row| !dynamic_shrink
                    || records
                        .reads
                        .dynamic
                        .get(row.access)
                        .unwrap()
                        .declared_class
                        != Some(stable_class))
                .cloned()
                .collect::<Vec<_>>()
        );
        put!(
            execution::read_fields::FieldLocationObservation,
            records
                .reads
                .fields
                .locations
                .iter()
                .filter(|r| !field_shrink || r.class != unrelated)
                .cloned()
                .collect::<Vec<_>>()
        );
        put!(
            execution::read_fields::FieldReadAssessment,
            records
                .reads
                .fields
                .assessments
                .iter()
                .filter(|r| !field_shrink || r.class != unrelated)
                .cloned()
                .map(|mut r| {
                    if field_forge && r.class == unrelated && r.name == "unread" {
                        r.status = ReadAssessment::CompleteNoReadUnderModel;
                        r.reason = None;
                    }
                    r
                })
                .collect::<Vec<_>>()
        );
        put!(
            execution::read_fields::GlobalClassInspection,
            records
                .reads
                .fields
                .globals
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        );
        put!(
            execution::read_fields::GlobalFieldReadAssessment,
            records
                .reads
                .fields
                .global_assessments
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        );
        put!(
            ReadObservation,
            records
                .reads
                .reads
                .iter()
                .filter(|r| !shrink || r.owner != value.owner)
                .cloned()
                .collect::<Vec<_>>()
        );
        put!(ReadDependency,records.reads.dependencies.iter().filter(|r|!shrink||records.reads.reads.get(r.read).unwrap().owner!=value.owner).cloned().collect::<Vec<_>>());
        put!(
            AttributeRead,
            records.reads.attributes.iter().cloned().collect::<Vec<_>>()
        );
        put!(
            FormalReadAssessment,
            records
                .reads
                .formals
                .iter()
                .filter(|r| !shrink || r.owner != value.owner)
                .cloned()
                .map(|mut row| {
                    if forge && row.id() == value.id() {
                        row.status = ReadAssessment::CompleteNoReadUnderModel;
                    }
                    row
                })
                .collect::<Vec<_>>()
        );
        check.finish()
    };
    validate(false, false, false, false, false, false).unwrap();
    assert!(validate(true, false, false, false, false, false).is_err());
    assert!(validate(false, true, false, false, false, false).is_err());
    assert!(validate(false, false, true, false, false, false).is_err());
    assert!(validate(false, false, false, true, false, false).is_err());
    assert!(validate(false, false, false, false, true, false).is_err());
    assert!(validate(false, false, false, false, false, true).is_err());
    let mut incomplete = conditions::entry::EntryData::new(&f.budget);
    macro_rules! copy{($($field:ident:$ty:ty,)*)=>{$(for row in data.flow.$field.iter(){incomplete.$field.insert(row.clone()).unwrap();})*};}
    lctx_model::entry_value_inputs!(copy);
    incomplete.coverage = Rows::new(&f.budget);
    let roots = data
        .flow
        .artifacts
        .iter()
        .filter(|a| a.input == run.input)
        .map(Record::id)
        .collect();
    let out = produce(
        &data.evaluation,
        &incomplete,
        &invocation,
        &roots,
        &f.budget,
    )
    .unwrap();
    assert_eq!(
        out.formals
            .iter()
            .find(|r| r.formal == ignored.formal)
            .unwrap()
            .status,
        ReadAssessment::Unknown
    );
    assert!(
        out.fields
            .assessments
            .iter()
            .filter(|r| r.status != ReadAssessment::ObservedRead)
            .all(|r| r.status == ReadAssessment::Unknown
                && r.reason == Some(obligation::ObligationKind::IncompleteCoverage))
    );
    drop(out);
    let mut foreign = invocation.clone();
    let mut context = f
        .rows::<attribution::AnalysisContext>()
        .into_iter()
        .find(|context| context.id() == run.context)
        .unwrap();
    context.config_digest = ContentHash::of(b"negative foreign read context");
    foreign.context = context.id();
    let foreign_output =
        produce(&data.evaluation, &data.flow, &foreign, &roots, &f.budget).unwrap();
    assert!(foreign_output.reads.is_empty());
    assert!(
        foreign_output
            .formals
            .iter()
            .all(|r| r.status != ReadAssessment::CompleteNoReadUnderModel)
    );
    drop(foreign_output);
    let before = f.budget.reserved();
    let output = produce(&data.evaluation, &data.flow, &invocation, &roots, &f.budget).unwrap();
    assert!(f.budget.reserved() > before);
    drop(output);
    assert_eq!(f.budget.reserved(), before);
    assert!(
        produce(
            &data.evaluation,
            &data.flow,
            &invocation,
            &roots,
            &resources::ResourceBudget::fixed(1).unwrap()
        )
        .is_err()
    );
}

#[tokio::test]
async fn complete_field_screen_retains_positive_hierarchy_and_global_boundaries() {
    let f = fixture::native_from("field_read_screen").await;
    let data = source_fixture::data(&f);
    let run = data
        .flow
        .runs
        .get(
            data.flow
                .coverage
                .iter()
                .find(|c| c.family == attribution::FactFamily::Flow && c.run.is_some())
                .unwrap()
                .run
                .unwrap(),
        )
        .unwrap();
    let (_, definition) = execution::configuration::base_evaluation();
    let (invocation, _) = analysis::base_evaluation::AnalysisInvocation::new(
        run.input,
        run.context,
        definition.id(),
        None,
        [],
    );
    let roots = data
        .flow
        .artifacts
        .iter()
        .filter(|a| a.input == run.input)
        .map(Record::id)
        .collect();
    let output = produce(&data.evaluation, &data.flow, &invocation, &roots, &f.budget).unwrap();
    let class = |name: &str| {
        data.evaluation.classes.iter().find(|c|matches!(c,normalized::entities::ClassEntity::Source{declaration}if text(&f,*declaration).starts_with(&format!("class {name}")))).unwrap().id()
    };
    let assessment = |c, name: &str| {
        output
            .fields
            .assessments
            .iter()
            .find(|r| r.class == c && r.name == name)
            .unwrap()
    };
    if assessment(class("Cold:"), "unread").status != ReadAssessment::CompleteNoReadUnderModel {
        read_diagnostics::dump(&f, &data, &output);
    }
    assert_eq!(
        assessment(class("Cold:"), "unread").status,
        ReadAssessment::CompleteNoReadUnderModel,
        "{:?}",
        assessment(class("Cold:"), "unread")
    );
    assert_eq!(
        assessment(class("Cold:"), "declared").status,
        ReadAssessment::CompleteNoReadUnderModel
    );
    assert_eq!(
        assessment(class("Hot:"), "loaded").status,
        ReadAssessment::ObservedRead
    );
    assert_eq!(
        assessment(class("Named:"), "literal_seen").status,
        ReadAssessment::ObservedRead
    );
    assert_eq!(
        assessment(class("Named:"), "alias_seen").status,
        ReadAssessment::ObservedRead
    );
    assert_eq!(
        assessment(class("Dynamic:"), "reachable").reason,
        Some(obligation::ObligationKind::DynamicAccess)
    );
    assert_eq!(
        assessment(class("Child("), "child_field").reason,
        Some(obligation::ObligationKind::DynamicAccess)
    );
    assert!(
        output
            .fields
            .global_assessments
            .iter()
            .any(|r| r.name == "unread" && r.status == ReadAssessment::CompleteNoReadUnderModel)
    );
    let mut no_calls = conditions::entry::EntryData::new(&f.budget);
    macro_rules! copy_entry{($($field:ident:$ty:ty,)*)=>{$(for row in data.flow.$field.iter(){no_calls.$field.insert(row.clone()).unwrap();})*};}
    lctx_model::entry_value_inputs!(copy_entry);
    no_calls.coverage = Rows::new(&f.budget);
    for c in data
        .flow
        .coverage
        .iter()
        .filter(|c| c.family != attribution::FactFamily::Calls)
    {
        no_calls.coverage.insert(c.clone()).unwrap();
    }
    let missing_calls =
        produce(&data.evaluation, &no_calls, &invocation, &roots, &f.budget).unwrap();
    assert!(
        missing_calls
            .fields
            .assessments
            .iter()
            .filter(|r| r.status != ReadAssessment::ObservedRead)
            .all(|r| r.status == ReadAssessment::Unknown
                && r.reason == Some(obligation::ObligationKind::IncompleteCoverage))
    );
    assert!(
        missing_calls
            .formals
            .iter()
            .all(|r| r.status != ReadAssessment::CompleteNoReadUnderModel)
    );
    let mut missing = execution::evaluation::EvaluationData::new(&f.budget);
    macro_rules! copy{($($field:ident:$ty:ty,)*)=>{$(for row in data.evaluation.$field.iter(){missing.$field.insert(row.clone()).unwrap();})*};}
    lctx_model::execution_evaluation_inputs!(copy);
    missing.ancestry = Rows::new(&f.budget);
    let degraded = produce(&missing, &data.flow, &invocation, &roots, &f.budget).unwrap();
    assert!(
        degraded
            .fields
            .assessments
            .iter()
            .filter(|r| r.class == class("Cold:"))
            .all(|r| r.status == ReadAssessment::Unknown
                && r.reason == Some(obligation::ObligationKind::MissingEvidence))
    );
}

#[tokio::test]
async fn release_wide_exec_eval_refuse_field_and_formal_negatives() {
    let f = fixture::native_from("field_dynamic_all").await;
    let data = source_fixture::data(&f);
    let run = data
        .flow
        .runs
        .get(
            data.flow
                .coverage
                .iter()
                .find(|c| c.family == attribution::FactFamily::Flow && c.run.is_some())
                .unwrap()
                .run
                .unwrap(),
        )
        .unwrap();
    let (_, definition) = execution::configuration::base_evaluation();
    let (invocation, _) = analysis::base_evaluation::AnalysisInvocation::new(
        run.input,
        run.context,
        definition.id(),
        None,
        [],
    );
    let roots = data
        .flow
        .artifacts
        .iter()
        .filter(|a| a.input == run.input)
        .map(Record::id)
        .collect();
    let output = produce(&data.evaluation, &data.flow, &invocation, &roots, &f.budget).unwrap();
    assert!(
        output
            .dynamic
            .iter()
            .any(|d| d.kind == execution::read_dynamic::DynamicKind::Exec)
    );
    assert!(
        output
            .dynamic
            .iter()
            .any(|d| d.kind == execution::read_dynamic::DynamicKind::Eval)
    );
    assert!(
        output
            .dynamic
            .iter()
            .any(|d| d.kind == execution::read_dynamic::DynamicKind::Vars)
    );
    assert!(output.fields.assessments.iter().any(|r| r.name == "unread"
        && r.status == ReadAssessment::Unknown
        && r.reason == Some(obligation::ObligationKind::DynamicAccess)));
    assert!(
        output
            .formals
            .iter()
            .filter(|r| r.status != ReadAssessment::ObservedRead)
            .all(|r| r.status == ReadAssessment::Unknown)
    );
}
