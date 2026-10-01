#[path="typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{execution::evaluation::*, analysis::native::NativeInventory, normalized::entity_normalization, obligation::ObligationKind, source::*, *, resources::ResourceBudget};
use typed_driver::{files,rows};
struct Facts(typed_driver::Tables);
impl typed_driver::Inspector for Facts {fn tables(&self)->typed_driver::Tables {self.0.clone()}}
async fn data()->(EvaluationData,lctx_model::domain::conditions::entry::EntryData,ResourceBudget) {
    let tables=typed_driver::Tables::default();
    typed_driver::run_behavioral(&files("execution_channels"),Facts(tables.clone())).await.unwrap();
    let budget=ResourceBudget::fixed(256<<20).unwrap();
    let mut facts=entity_normalization::EntityData::new(&budget);
    macro_rules! inputs {($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in rows::<$ty>(&tables){facts.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(inputs);
    let normalized=entity_normalization::normalize(facts.inputs(),&budget).unwrap();
    let mut data=EvaluationData::new(&budget);
    let mut entries=lctx_model::domain::conditions::entry::EntryData::new(&budget);
    let mut inventory=NativeInventory::new(&budget);
    let native_inputs=NativeInventory::inputs();
    for (name,batch) in tables.lock().unwrap().iter() {data.visit(name,batch).unwrap();entries.visit(name,batch).unwrap();if native_inputs.iter().any(|input|input.name()==*name) {inventory.visit(name,batch).unwrap();}}
    macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&normalized.$field.iter().cloned().collect::<Vec<_>>()).unwrap();data.visit(<$ty>::NAME,&batch).unwrap();entries.visit(<$ty>::NAME,&batch).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(outputs);
    let inventory=inventory.collect().unwrap();
    data.premises=inventory.premises;data.native=inventory.qualifications;
    (data,entries,budget)
}
fn request(data:&EvaluationData,text:&str)->ExpressionRequest {
    let files=files("execution_channels");
    let occurrence=data.occurrences.iter().find(|o|o.role==OccurrenceRole::Syntax && !matches!(o.syntax_kind,SyntaxKind::StringLiteral|SyntaxKind::BytesLiteral|SyntaxKind::Identifier) && {
        let source=data.artifacts.get(o.source).unwrap();files[&source.path][o.start as usize..o.end as usize]==*text.as_bytes()
    }).unwrap_or_else(||panic!("native syntax absent {text}"));
    let owner=data.owners.iter().find(|owner|owner.occurrence==occurrence.id()).unwrap();
    let context=data.placements.iter().find(|row|row.occurrence==occurrence.id()).map(|row|data.qualifications.get(row.qualification).unwrap().context).unwrap();
    ExpressionRequest {input:data.artifacts.get(occurrence.source).unwrap().input,context,owner:owner.entity,expression:occurrence.id()}
}
#[tokio::test]
async fn native_closed_expressions_skip_unentered_operands_and_preserve_limits() {
    let (data,_,budget)=data().await;
    for (text,truth) in [("False and effect()",Some(false)),("True or effect()",Some(true)),("3 if True else effect()",Some(true)),("1 + 2",Some(true)),("-2",Some(true)),("not ()",Some(true)),("\"hello\"",None),("1j",None)] {
        let req=request(&data,text);
        let result=evaluate(&data,req,&budget).unwrap().unwrap_or_else(|reason|panic!("{text}: {reason:?}"));
        assert_eq!(result.truth(),truth,"{text}");assert_eq!(result.release(),ReleaseSafety::Closed);assert!(!result.native_premises().is_empty());
        assert!(result.evaluated_operands().iter().all(|id|data.occurrences.get(*id).unwrap().syntax_kind!=SyntaxKind::ExprCall));
    }
    for text in ["True and effect()","999999999999999999999999999999999999999 + 1"] {assert!(matches!(evaluate(&data,request(&data,text),&budget).unwrap(),Err(ObligationKind::UnsupportedControlFlow)));}
    let tiny=ResourceBudget::fixed(1).unwrap();assert!(matches!(evaluate(&data,request(&data,"1 + 2"),&tiny),Err(ModelError::Resource{..})));
}

#[tokio::test]
async fn name_evaluation_needs_the_exact_private_entry_proof_and_retains_its_allowance() {
    use lctx_model::domain::{conditions::entry::*, normalized::entities::*};
    let (data,entries,budget)=data().await;
    let read=data.occurrences.iter().find(|o|o.syntax_kind==SyntaxKind::ExprName && o.role==OccurrenceRole::Read && {
        let source=data.artifacts.get(o.source).unwrap();files("execution_channels")[&source.path][o.start as usize..o.end as usize]==*b"value"
    }).unwrap();
    let owner=entries.owners.iter().find(|o|o.occurrence==read.id()).unwrap();
    let formal=entries.formals.iter().find(|formal|matches!(formal,ParameterEntity::Source{declaration} if entries.occurrences.get(*declaration).is_some_and(|o|o.structural_path.starts_with(&entries.occurrences.get(owner.owner).unwrap().structural_path)))).unwrap();
    let use_=entries.uses.iter().find(|u|u.occurrence==read.id()).unwrap();
    let observation=entries.use_observations.iter().find(|o|o.use_==use_.id()).unwrap();
    let run=entries.use_supports.iter().find(|s|s.assertion==observation.id()).unwrap().run;
    let context=entries.runs.get(run).unwrap().context;
    let request=ExpressionRequest {input:data.artifacts.get(read.source).unwrap().input,context,owner:owner.entity,expression:read.id()};
    assert!(evaluate(&data,request,&budget).unwrap().is_err());
    let entry=EntryValueWitness::derive(&entries,EntryRequest{owner:owner.entity,formal:formal.id(),access:read.id(),context,run},&budget).unwrap().unwrap();
    let result=evaluate_with_entries(&data,request,&[&entry],&budget).unwrap().unwrap();
    assert_eq!(result.release(),ReleaseSafety::CallerRetained);assert_eq!(result.entry_premises(),&[entry.witness().id()]);
    assert_eq!(result.status(),entry.evidence_status());
    let value=entries.values.iter().find(|row|row.use_==use_.id()).unwrap();let value_support=entries.value_supports.iter().find(|row|row.assertion==value.id()).unwrap();
    let access_source=EntryAccessSource::value(&entries,entry.witness().request(),value.id(),value_support.id()).unwrap();
    let value_entry=EntryValueWitness::derive_for(&entries,entry.witness().request(),&access_source,&budget).unwrap().unwrap();
    assert!(matches!(evaluate_with_entries(&data,request,&[&value_entry],&budget).unwrap(),Err(ObligationKind::EntryValueUnknown)));
    drop(value_entry);

    let reservation=budget.reserved();drop(entry);assert_eq!(budget.reserved(),reservation);drop(result);assert!(budget.reserved()<reservation);
}

#[tokio::test]
async fn native_base_completion_consumes_independent_evaluations_and_replays_pending_order() {
    use lctx_model::domain::{execution::{completion::*,outcome::PendingOutcome,ExactRuntimeException},normalized::entities::CallableEntity};
    let (data,_,budget)=data().await;
    for (function,raised) in [("finally_returns",false),("finally_raises",true),("bare_handler",true)] {
        let source_files=files("execution_channels");
        let callable=data.callables.iter().find(|c|matches!(c,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let source=data.artifacts.get(o.source).unwrap();source_files[&source.path][o.start as usize..o.end as usize].starts_with(format!("def {function}(").as_bytes())})).unwrap();
        let owner=lctx_model::domain::normalized::entities::EntityRef::Callable{callable:callable.id()}.id();
        let statement=data.occurrences.iter().find(|o|o.syntax_kind==SyntaxKind::StmtTry && data.owners.iter().any(|r|r.occurrence==o.id() && r.entity==owner)).unwrap();
        let context=data.placements.iter().find(|p|p.occurrence==statement.id()).map(|p|data.qualifications.get(p.qualification).unwrap().context).unwrap();
        let input=data.artifacts.get(statement.source).unwrap().input;
        let mut earlier=Vec::new();
        for o in data.occurrences.iter().filter(|o|matches!(o.syntax_kind,SyntaxKind::ExprNoneLiteral|SyntaxKind::ExprNumberLiteral) && data.owners.iter().any(|r|r.occurrence==o.id() && r.entity==owner)) {
            earlier.push(evaluate(&data,ExpressionRequest{input,context,owner,expression:o.id()},&budget).unwrap().unwrap());
        }
        let request=CompletionRequest{input,context,owner,statement:statement.id()};
        assert!(complete(&data,request,&[],&budget).unwrap().is_err());
        let refs=earlier.iter().collect::<Vec<_>>();
        let result=complete(&data,request,&refs,&budget).unwrap().unwrap_or_else(|reason|panic!("{function}: {reason:?}"));
        if raised {assert!(matches!(result.outcome(),PendingOutcome::Raise{exception:ExactRuntimeException::TypeError,..}),"{function}");}else {assert!(matches!(result.outcome(),PendingOutcome::Return{..}),"{function}");}
        assert!(!result.native_premises().is_empty());assert!(result.release_inputs().iter().all(|(_,safety)|*safety==ReleaseSafety::Closed));
        let mut foreign=request;foreign.owner=data.refs.iter().find(|r|r.id()!=owner).unwrap().id();assert!(complete(&data,foreign,&refs,&budget).unwrap().is_err());
    }
}

#[tokio::test]
async fn base_evaluation_shared_replay_refuses_value_disposal_status_and_coupled_membership_forgery() {
    use lctx_model::domain::{analysis::{*,base_evaluation::AnalysisInvocation,policy::EvidenceStatus},execution::records::*};
    let (data,entry,budget)=data().await;
    let req=request(&data,"1 + 2");
    let parameters=MethodParameters {depth:None,proof_steps:None,work:None,members:None,seed:None,iterations:None,threshold:None,resolution:None,damping:None,model_catalog:None};
    let (_,definition)=lctx_model::domain::execution::configuration::base_evaluation();
    let (invocation,_)=AnalysisInvocation::new(req.input,req.context,definition.id(),Some(req.owner),[]);
    let checked=evaluate(&data,req,&budget).unwrap().unwrap();
    let mut wrong_version=definition.clone();wrong_version.semantic_version=ContentHash::of(b"unbound-base-version");assert!(checked.emit_base(&invocation,&wrong_version,&budget).is_err());
    let mut changed_parameters=parameters.clone();changed_parameters.work=Some(1);let mut wrong_limits=definition.clone();wrong_limits.parameters=changed_parameters.id();let (wrong_invocation,_)=AnalysisInvocation::new(req.input,req.context,wrong_limits.id(),Some(req.owner),[]);assert!(checked.emit_base(&wrong_invocation,&wrong_limits,&budget).is_err());

    for mutation in 0..6 {
        let mut output=checked.emit_base(&invocation,&definition,&budget).unwrap();
        match mutation {
            1=>output.evaluation.boolean_value=Some(false),
            2=>output.evaluation.release=ReleaseSafety::CallerRetained,
            3=>output.evaluation.status=EvidenceStatus::FixtureChecked,
            4=>{let mut q=data.qualifications.get(output.evaluation.qualification).unwrap().clone();q.approximation=lctx_model::domain::assertion::Approximation::Over;output.evaluation.qualification=q.id();},
            5=>{
                output.sources.pop();output.members.pop();
                let mut sink=KeySink::new("base-evaluation-sources");for source in &output.sources {source.id().encode(&mut sink);}output.evaluation.sources=sink.finish();
            },
            _=>{},
        }
        let invariant=base_invariants().remove(0);let mut check=(invariant.create)(&budget);
        macro_rules! visit_data {($($field:ident:$ty:ty,)*)=>{$(let input=ValidationInput::of::<$ty>(&["id"]);let input=if stages::is_vocabulary(input.name()){input.at_epoch(stages::PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(visit_data);
        macro_rules! visit_entry {($($field:ident:$ty:ty,)*)=>{$(let input=ValidationInput::of::<$ty>(&["id"]);let input=if stages::is_vocabulary(input.name()){input.at_epoch(stages::PublicationBoundary::Facts)}else{input};check.visit_input(&input,&<$ty as Record>::encode(&entry.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(visit_entry);
        check.visit(AnalysisInvocation::NAME,&AnalysisInvocation::encode(&[invocation.clone()]).unwrap()).unwrap();
        check.visit(AnalysisDefinition::NAME,&AnalysisDefinition::encode(&[definition.clone()]).unwrap()).unwrap();
        check.visit(ExpressionEvaluation::NAME,&ExpressionEvaluation::encode(&[output.evaluation]).unwrap()).unwrap();
        check.visit(EvaluationSource::NAME,&<EvaluationSource as Record>::encode(&output.sources).unwrap()).unwrap();
        check.visit(EvaluationMember::NAME,&EvaluationMember::encode(&output.members).unwrap()).unwrap();
        check.visit(EvaluationOperand::NAME,&EvaluationOperand::encode(&output.operands).unwrap()).unwrap();
        let result=check.finish();if mutation==0 {result.unwrap();}else{assert!(result.is_err(),"mutation {mutation}");}
    }
}

#[tokio::test]
async fn base_completion_stored_replay_rejects_pending_order_and_coupled_proof_forgery() {
    use lctx_model::domain::{analysis::{self,*,policy::EvidenceStatus},execution::{completion::*,completion_records::*,records::*},normalized::entities::CallableEntity};
    let (data,entry,budget)=data().await;
    let source_files=files("execution_channels");
    let callable=data.callables.iter().find(|c|matches!(c,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let source=data.artifacts.get(o.source).unwrap();source_files[&source.path][o.start as usize..o.end as usize].starts_with(b"def finally_returns(")})).unwrap();
    let owner=lctx_model::domain::normalized::entities::EntityRef::Callable{callable:callable.id()}.id();
    let statement=data.occurrences.iter().find(|o|o.syntax_kind==SyntaxKind::StmtTry && data.owners.iter().any(|r|r.occurrence==o.id() && r.entity==owner)).unwrap();
    let context=data.placements.iter().find(|p|p.occurrence==statement.id()).map(|p|data.qualifications.get(p.qualification).unwrap().context).unwrap();
    let input=data.artifacts.get(statement.source).unwrap().input;
    let parameters=MethodParameters {depth:None,proof_steps:None,work:None,members:None,seed:None,iterations:None,threshold:None,resolution:None,damping:None,model_catalog:None};
    let (_,definition)=lctx_model::domain::execution::configuration::base_evaluation();
    let (_,completion_definition)=lctx_model::domain::execution::configuration::base_completion();
    let (eval_invocation,_)=analysis::base_evaluation::AnalysisInvocation::new(input,context,definition.id(),Some(owner),[]);
    let (invocation,_)=analysis::base_completion::AnalysisInvocation::new(input,context,completion_definition.id(),Some(owner),[]);
    let mut checked=Vec::new();let mut earlier=Vec::new();
    for o in data.occurrences.iter().filter(|o|matches!(o.syntax_kind,SyntaxKind::ExprNoneLiteral|SyntaxKind::ExprNumberLiteral) && data.owners.iter().any(|r|r.occurrence==o.id() && r.entity==owner)) {
        let proof=evaluate(&data,ExpressionRequest{input,context,owner,expression:o.id()},&budget).unwrap().unwrap();
        earlier.push(proof.emit_base(&eval_invocation,&definition,&budget).unwrap());checked.push(proof);
    }
    let refs=checked.iter().collect::<Vec<_>>();let completion=complete(&data,CompletionRequest{input,context,owner,statement:statement.id()},&refs,&budget).unwrap().unwrap();
    let entered_evaluations=completion.evaluated_expressions().iter().map(|expression|CompletionEvaluation{evaluation:&earlier.iter().find(|r|r.evaluation.expression==*expression).unwrap().evaluation,invocation:&eval_invocation}).collect::<Vec<_>>();
    for mutation in 0..6 {
        let mut output=completion.emit_base(&invocation,&completion_definition,&entered_evaluations,&budget).unwrap();
        match mutation {
            1=>{output.outcome=CompletionOutcome::Normal;output.completion.outcome=output.outcome.id();},
            2=>output.completion.status=EvidenceStatus::FixtureChecked,
            3=>{output.entered.reverse();for (ordinal,row) in output.entered.iter_mut().enumerate(){row.ordinal=ordinal as i64;}let mut sink=KeySink::new("base-completion-entered");for row in &output.entered {row.statement.encode(&mut sink);}output.completion.entered=sink.finish();},
            4=>{output.sources.pop();output.members.pop();let mut sink=KeySink::new("base-completion-sources");for row in &output.sources {row.id().encode(&mut sink);}output.completion.sources=sink.finish();},
            5=>{let mut q=data.qualifications.get(output.completion.qualification).unwrap().clone();q.modality=attribution::Modality::Potential;output.completion.qualification=q.id();},
            _=>{},
        }
        // Re-key every child with the forged conclusion. Refusal must come from recomputation,
        // rather than an incidental dangling link caused by changing the conclusion identity.
        for member in &mut output.members {member.completion=output.completion.id();}for entered in &mut output.entered {entered.completion=output.completion.id();}
        let invariant=completion_invariants().remove(0);let mut check=(invariant.create)(&budget);
        macro_rules! visit_data {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::execution_evaluation_inputs!(visit_data);
        macro_rules! visit_entry {($($field:ident:$ty:ty,)*)=>{$(check.visit(<$ty>::NAME,&<$ty as Record>::encode(&entry.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
        lctx_model::entry_value_inputs!(visit_entry);
        check.visit(AnalysisDefinition::NAME,&AnalysisDefinition::encode(&[definition.clone(),completion_definition.clone()]).unwrap()).unwrap();
        check.visit(analysis::base_evaluation::AnalysisInvocation::NAME,&analysis::base_evaluation::AnalysisInvocation::encode(&[eval_invocation.clone()]).unwrap()).unwrap();
        check.visit(analysis::base_completion::AnalysisInvocation::NAME,&analysis::base_completion::AnalysisInvocation::encode(&[invocation.clone()]).unwrap()).unwrap();
        for rows in &earlier {
            check.visit(ExpressionEvaluation::NAME,&ExpressionEvaluation::encode(&[rows.evaluation.clone()]).unwrap()).unwrap();
            check.visit(EvaluationSource::NAME,&<EvaluationSource as Record>::encode(&rows.sources).unwrap()).unwrap();
            check.visit(EvaluationMember::NAME,&EvaluationMember::encode(&rows.members).unwrap()).unwrap();
            check.visit(EvaluationOperand::NAME,&EvaluationOperand::encode(&rows.operands).unwrap()).unwrap();
        }
        check.visit(StatementCompletion::NAME,&StatementCompletion::encode(&[output.completion]).unwrap()).unwrap();
        check.visit(CompletionOutcome::NAME,&<CompletionOutcome as Record>::encode(&[output.outcome]).unwrap()).unwrap();
        check.visit(CompletionSource::NAME,&<CompletionSource as Record>::encode(&output.sources).unwrap()).unwrap();
        check.visit(CompletionMember::NAME,&CompletionMember::encode(&output.members).unwrap()).unwrap();
        check.visit(EnteredStatement::NAME,&EnteredStatement::encode(&output.entered).unwrap()).unwrap();
        let result=check.finish();if mutation==0 {result.unwrap();}else{assert!(result.is_err(),"mutation {mutation}");}
    }
}






#[tokio::test]
async fn source_body_is_under_entry_and_preserves_required_frame_cleanup() {
    use lctx_model::domain::{execution::{body::*,completion::*,outcome::PendingOutcome},normalized::entities::CallableEntity};
    let (data,_,budget)=data().await;let source_files=files("execution_channels");
    for function in ["body_pass","body_stops"] {
        let callable=data.callables.iter().find(|c|matches!(c,CallableEntity::Source{declaration,..} if {let o=data.occurrences.get(*declaration).unwrap();let source=data.artifacts.get(o.source).unwrap();source_files[&source.path][o.start as usize..o.end as usize].starts_with(format!("def {function}(").as_bytes())})).unwrap();
        let CallableEntity::Source{declaration,..}=callable else{unreachable!()};
        let owner=lctx_model::domain::normalized::entities::EntityRef::Callable{callable:callable.id()}.id();
        let q=data.placements.iter().find(|p|p.occurrence==*declaration).map(|p|data.qualifications.get(p.qualification).unwrap()).unwrap();let context=q.context;
        let input=data.artifacts.get(data.occurrences.get(*declaration).unwrap().source).unwrap().input;
        let first=data.placements.iter().find(|p|p.parent==Some(*declaration) && p.field==lctx_model::domain::lexical::SyntaxField::Body && p.ordinal==0).unwrap().occurrence;
        let mut evaluations=Vec::new();for o in data.occurrences.iter().filter(|o|o.syntax_kind==SyntaxKind::ExprNumberLiteral && data.owners.iter().any(|r|r.occurrence==o.id() && r.entity==owner)){evaluations.push(evaluate(&data,ExpressionRequest{input,context,owner,expression:o.id()},&budget).unwrap().unwrap());}
        let refs=evaluations.iter().collect::<Vec<_>>();let statement=complete(&data,CompletionRequest{input,context,owner,statement:first},&refs,&budget).unwrap().unwrap();
        let request=SourceBodyRequest{input,context,callee:owner};let body=complete_body(&data,request,&[&statement],&budget).unwrap().unwrap();
        assert_eq!(body.frame_obligation(),ObligationKind::FrameExitCleanup);assert_eq!(body.entered_statements(),&[first]);
        assert_eq!(body.outcome().is_normal(),function=="body_pass");if function=="body_stops"{assert!(matches!(body.outcome(),PendingOutcome::Return{..}));}
        assert!(complete_body(&data,request,&[],&budget).unwrap().is_err());
        let tiny=ResourceBudget::fixed(1).unwrap();assert!(matches!(complete_body(&data,request,&[&statement],&tiny),Err(ModelError::Resource{..})));
    }
}
