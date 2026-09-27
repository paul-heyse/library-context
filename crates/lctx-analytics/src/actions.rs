//! Pure composition of authored action candidates with independently proved call triggers.
use std::collections::BTreeMap;
use cpg_schema::action::{Candidate,ModeledActionAssessmentsRow,ModeledActionPostconditionsRow,
    Support,admit,identity,normal_digest,admit_postcondition,postcondition_identity};
use cpg_schema::behavior::{ModeledEffectSitesRow,ModeledCallbackSitesRow,ModeledResourceSitesRow,
    ModelApplicationsRow,ExpressionEvaluationsRow,ExpressionEvaluationStepsRow};
use cpg_schema::call_execution::{CallExecutionsRow,CallExecutionStepsRow};
use cpg_schema::codebook::{ModelExit,BoundaryReason};
use cpg_schema::id::{Id,recipe::SummaryFlowProofStep};

#[derive(Default)]
pub struct Inputs<'a> {
    pub bindings:&'a [cpg_schema::behavior::ModelArgumentBindingsRow],
    pub arguments:&'a [cpg_schema::tables::ArgumentsRow],
    pub effects:&'a [ModeledEffectSitesRow],pub callbacks:&'a [ModeledCallbackSitesRow],
    pub resources:&'a [ModeledResourceSitesRow],pub applications:&'a [ModelApplicationsRow],
    pub executions:&'a [CallExecutionsRow],pub execution_steps:&'a [CallExecutionStepsRow],
    pub expressions:&'a [ExpressionEvaluationsRow],pub expression_steps:&'a [ExpressionEvaluationStepsRow],
    pub frames:&'a [cpg_schema::frame_exit::ModelFrameExitsRow],
    pub frame_arguments:&'a [cpg_schema::frame_exit::ModelFrameExitArgumentsRow],
}

fn unique_index<'a,T,K:Ord>(rows:&'a [T],key:impl Fn(&T)->K)->BTreeMap<K,Option<&'a T>> {
    let mut index=BTreeMap::new();
    for row in rows {index.entry(key(row)).and_modify(|r|*r=None).or_insert(Some(row));}
    index
}

#[derive(Default)]
pub struct Outcome {
    pub assessments:Vec<ModeledActionAssessmentsRow>,
    pub postconditions:Vec<ModeledActionPostconditionsRow>,
}

pub fn assess(inputs:Inputs<'_>)->Outcome {
    let applications=unique_index(inputs.applications,|r|(r.snapshot_id,r.call_site_node_id,r.pysa_fact_id,r.model_id));
    let executions=unique_index(inputs.executions,|r|(r.snapshot_id,r.call_node_id,r.pysa_fact_id,r.model_id));
    let expressions=unique_index(inputs.expressions,|r|(r.snapshot_id,r.syntax_fact_id));
    let mut invocation_steps:BTreeMap<_,Vec<_>>=BTreeMap::new();
    for s in inputs.execution_steps {invocation_steps.entry((s.snapshot_id,s.execution_id)).or_default().push(s);}
    for steps in invocation_steps.values_mut() {steps.sort_by_key(|s|s.ordinal);}
    let mut expression_steps:BTreeMap<_,Vec<_>>=BTreeMap::new();
    for s in inputs.expression_steps {expression_steps.entry((s.snapshot_id,s.syntax_fact_id)).or_default().push(s.clone());}
    for steps in expression_steps.values_mut() {steps.sort_by_key(|s|s.ordinal);}
    let mut bindings:BTreeMap<_,Vec<_>>=BTreeMap::new();
    for b in inputs.bindings {bindings.entry((b.snapshot_id,b.call_site_node_id,b.pysa_fact_id,b.model_id,b.rule_id)).or_default().push(b.clone());}
    let mut arguments:BTreeMap<_,Vec<_>>=BTreeMap::new();
    for a in inputs.arguments {arguments.entry((a.snapshot_id,a.call_node_id)).or_default().push(a.clone());}
    let mut out=Outcome::default();
    for candidate in inputs.effects.iter().map(Candidate::Effect)
        .chain(inputs.callbacks.iter().map(Candidate::Callback)).chain(inputs.resources.iter().map(Candidate::Resource)) {
        let v=candidate.view();let key=(v.snapshot_id,v.call_site_node_id,v.pysa_fact_id,v.model_id);
        let execution=executions.get(&key).copied().flatten();
        let application=applications.get(&key).copied().flatten();
        let steps=execution.and_then(|e|invocation_steps.get(&(e.snapshot_id,e.execution_id))).map(Vec::as_slice).unwrap_or_default();
        let ordered=steps.iter().enumerate().all(|(i,s)|s.ordinal==i as i64);
        let proof:Vec<_>=steps.iter().map(|s|SummaryFlowProofStep {kind:s.kind,evidence_id:s.evidence_id,condition_id:s.condition_id}).collect();
        let normal=execution.and_then(|e|expressions.get(&(e.snapshot_id,e.syntax_fact_id))).copied().flatten();
        let normal_steps=normal.and_then(|e|expression_steps.get(&(e.snapshot_id,e.syntax_fact_id))).map(Vec::as_slice).unwrap_or_default();
        let use_normal=matches!(v.trigger,ModelExit::Normal|ModelExit::Finally);
        let mut row=ModeledActionAssessmentsRow {snapshot_id:v.snapshot_id,assessment_id:Id::ZERO,
            candidate_id:v.candidate_id,channel:v.channel,function_node_id:v.function_node_id,call_site_node_id:v.call_site_node_id,
            pysa_fact_id:v.pysa_fact_id,model_id:v.model_id,rule_id:v.rule_id,
            execution_id:execution.map(|e|e.execution_id),condition_id:execution.map(|e|e.condition_id),
            normal_syntax_fact_id:if use_normal {normal.map(|n|n.syntax_fact_id)} else {None},
            normal_step_count:if use_normal {normal_steps.len() as i64} else {0},
            normal_steps_digest:normal_digest(if use_normal {normal_steps} else {&[]}),reason:None};
        row.assessment_id=identity(&row);
        let support=Support {
            bindings:bindings.get(&(v.snapshot_id,v.call_site_node_id,v.pysa_fact_id,v.model_id,v.rule_id)).map(Vec::as_slice).unwrap_or_default(),
            arguments:arguments.get(&(v.snapshot_id,v.call_site_node_id)).map(Vec::as_slice).unwrap_or_default(),
            application,execution,invocation_steps:&proof,normal,normal_steps,frames:inputs.frames,frame_arguments:inputs.frame_arguments};
        let refusal=if !ordered {Some(BoundaryReason::MissingEvidence)}
            else {admit(&row,candidate,&support).err().map(|e|e.reason)};
        if let Some(reason)=refusal {
            row.reason=Some(reason);row.normal_syntax_fact_id=None;row.normal_step_count=0;
            row.normal_steps_digest=normal_digest(&[]);row.assessment_id=identity(&row);
        }
        out.assessments.push(row);
        if v.trigger==ModelExit::Normal {
            let mut post=ModeledActionPostconditionsRow {snapshot_id:v.snapshot_id,postcondition_id:Id::ZERO,
                candidate_id:v.candidate_id,channel:v.channel,function_node_id:v.function_node_id,
                call_site_node_id:v.call_site_node_id,pysa_fact_id:v.pysa_fact_id,model_id:v.model_id,rule_id:v.rule_id,
                execution_id:execution.map(|e|e.execution_id),condition_id:execution.map(|e|e.condition_id),
                outcome_obligation:ModelExit::Normal,reason:None};
            post.postcondition_id=postcondition_identity(&post);
            post.reason=if !ordered {Some(BoundaryReason::MissingEvidence)}
                else {admit_postcondition(&post,candidate,&support).err().map(|e|e.reason)};
            post.postcondition_id=postcondition_identity(&post);
            out.postconditions.push(post);
        }
    }
    out.assessments.sort_by_key(|r|(r.snapshot_id,r.assessment_id));
    out.postconditions.sort_by_key(|r|(r.snapshot_id,r.postcondition_id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpg_schema::codebook::{InvocationPhase,Modality,Origin,ModelEffectKind,ModelEffectSubjectStatus,
        SummaryFlowStepKind as K,ModeledArgumentEvaluationStatus as E,SummaryChannel};
    use cpg_schema::completion_proof::proof_digest;
    fn id(n:u8)->Id {Id([n;16])}
    struct Case {
        effect:ModeledEffectSitesRow,app:ModelApplicationsRow,execution:CallExecutionsRow,
        steps:Vec<CallExecutionStepsRow>,normal:ExpressionEvaluationsRow,normal_steps:Vec<ExpressionEvaluationStepsRow>,
        frame:cpg_schema::frame_exit::ModelFrameExitsRow,frame_arguments:Vec<cpg_schema::frame_exit::ModelFrameExitArgumentsRow>,
        arguments:Vec<cpg_schema::tables::ArgumentsRow>,
    }
    impl Case {
        fn new()->Self {
            let condition=cpg_schema::condition_kernel::Diagram::always().id();
            let proof:Vec<_>=[(K::ModuleImportBinding,id(20)),(K::ModuleImportRegion,id(21)),(K::CalleeResolution,id(22)),
                (K::ExpressionSyntax,id(33)),(K::ParameterBinding,id(34)),
                (K::CallSite,id(5)),(K::CallTarget,id(6)),(K::ModelInvocation,id(8))].into_iter()
                .map(|(kind,evidence_id)|SummaryFlowProofStep {kind,evidence_id,condition_id:condition}).collect();
            let mut execution=CallExecutionsRow {snapshot_id:id(1),execution_id:Id::ZERO,function_node_id:id(4),
                call_node_id:id(2),call_fact_id:id(5),syntax_fact_id:id(12),target_node_id:id(7),pysa_fact_id:id(6),
                model_id:id(8),condition_id:condition,phase:InvocationPhase::Call,argument_count:1,default_formal_count:Some(0),
                default_formals_digest:Some(cpg_schema::call_execution::defaults_digest(&[])),prefix_count:0,
                invocation_count:8,prefix_digest:proof_digest(&[]),invocation_digest:proof_digest(&proof),reason:None,work:1};
            execution.execution_id=cpg_schema::call_execution::identity(&execution);
            let steps=proof.iter().enumerate().map(|(i,s)|CallExecutionStepsRow {snapshot_id:id(1),
                execution_id:execution.execution_id,ordinal:i as i64,kind:s.kind,evidence_id:s.evidence_id,condition_id:condition}).collect();
            let mut normal_steps:Vec<_>=proof[..7].iter().enumerate().map(|(i,s)|ExpressionEvaluationStepsRow {
                snapshot_id:id(1),syntax_fact_id:id(12),ordinal:i as i64,operand_fact_id:if matches!(i,3|4){id(33)}else{id(12)},
                evidence_id:s.evidence_id,status:E::CalleeEntryNormal,kind:s.kind}).collect();
            let mut frame_arguments=vec![cpg_schema::frame_exit::ModelFrameExitArgumentsRow {snapshot_id:id(1),frame_exit_id:Id::ZERO,
                ordinal:0,argument_fact_id:id(32),expression_fact_id:id(33),safety:cpg_schema::codebook::ReleaseSafety::Closed,
                parameter_name:"val".into(),expression_offset:3,expression_count:1,
                expression_digest:cpg_schema::frame_exit::steps_digest(&normal_steps[3..4]),parameters_digest:cpg_schema::frame_exit::parameters_digest(&[id(34)])}];
            let mut frame=cpg_schema::frame_exit::ModelFrameExitsRow {snapshot_id:id(1),frame_exit_id:Id::ZERO,
                function_node_id:id(4),call_node_id:id(2),call_fact_id:id(5),syntax_fact_id:id(12),target_node_id:id(7),pysa_fact_id:id(6),model_id:id(8),
                return_parameter:"val".into(),return_argument_fact_id:id(32),argument_count:1,signature_count:1,
                arguments_digest:cpg_schema::frame_exit::arguments_digest(&frame_arguments),invocation_count:7,
                invocation_digest:cpg_schema::frame_exit::steps_digest(&normal_steps)};
            frame.frame_exit_id=cpg_schema::frame_exit::identity(&frame);frame_arguments[0].frame_exit_id=frame.frame_exit_id;
            for(kind,evidence_id)in [(K::ModelFrameExit,frame.frame_exit_id),(K::PrecedingCallNormal,id(8)),(K::ExpressionSyntax,id(12))] {
                normal_steps.push(ExpressionEvaluationStepsRow {snapshot_id:id(1),syntax_fact_id:id(12),ordinal:normal_steps.len() as i64,
                    operand_fact_id:id(12),evidence_id,status:E::PinnedCallNormal,kind});
            }
            Self {
                arguments:vec![cpg_schema::tables::ArgumentsRow {snapshot_id:id(1),fact_id:id(32),node_id:id(31),call_node_id:id(2),ordinal:0,
                    kind:cpg_schema::codebook::ArgumentKind::Positional,keyword:None,start_byte:1,end_byte:2,value_start_byte:1,value_end_byte:2}],
                effect:ModeledEffectSitesRow {snapshot_id:id(1),call_site_node_id:id(2),function_node_id:Some(id(4)),
                    call_fact_id:id(5),pysa_fact_id:id(6),target_node_id:id(7),model_id:id(8),rule_id:id(9),
                    target_definition_fact_id:id(10),effect:ModelEffectKind::IoWrite,exit:ModelExit::Invocation,
                    argument:None,schema_kind:None,schema_class_node_id:None,schema_class_fact_id:None,schema_path_id:None,
                    schema_path_kind:None,schema_expression_node_id:None,schema_expression_fact_id:None,schema_reason:None,
                    subject_path_id:None,subject_path_kind:None,subject_expression_node_id:None,subject_expression_fact_id:None,
                    subject_status:ModelEffectSubjectStatus::Unqualified,subject_reason:None,target_modality:Modality::Definite,
                    model_modality:Modality::Potential,candidate_set_complete_under_model:true,has_unresolved_remainder:false,origin:Origin::SyntheticModel},
                app:ModelApplicationsRow {snapshot_id:id(1),call_site_node_id:id(2),module_node_id:id(3),function_node_id:Some(id(4)),
                    call_fact_id:id(5),pysa_fact_id:id(6),target_node_id:id(7),model_id:id(8),target_module_fact_id:id(11),
                    target_definition_fact_id:id(10),revision:1,target_modality:Modality::Definite,target_origin:Origin::SyntheticModel,
                    phase:InvocationPhase::Call,candidate_set_complete_under_model:true,has_unresolved_remainder:false,target_count:1,
                    target_body_return_parameter:Some("val".into()),target_call_defaults_available:false,model_origin:Origin::SyntheticModel},
                execution,steps,normal_steps,frame,frame_arguments,
                normal:ExpressionEvaluationsRow {snapshot_id:id(1),syntax_fact_id:id(12),normal:true,boolean_value:None,release_safety:cpg_schema::codebook::ReleaseSafety::Closed,
                    status:E::PinnedCallNormal,evidence_id:Some(id(12)),reason:None,work:1},
            }
        }
        fn assess(&self)->ModeledActionAssessmentsRow {
            let mut rows=assess(Inputs {effects:std::slice::from_ref(&self.effect),applications:std::slice::from_ref(&self.app),
                executions:std::slice::from_ref(&self.execution),execution_steps:&self.steps,expressions:std::slice::from_ref(&self.normal),
                arguments:&self.arguments,expression_steps:&self.normal_steps,frames:std::slice::from_ref(&self.frame),frame_arguments:&self.frame_arguments,..Default::default()}).assessments;
            assert_eq!(rows.len(),1);rows.pop().unwrap()
        }
        fn admits(&self,row:&ModeledActionAssessmentsRow)->bool {
            let proof:Vec<_>=self.steps.iter().map(|s|SummaryFlowProofStep {kind:s.kind,evidence_id:s.evidence_id,condition_id:s.condition_id}).collect();
            admit(row,Candidate::Effect(&self.effect),&Support {bindings:&[],arguments:&self.arguments,application:Some(&self.app),execution:Some(&self.execution),
                invocation_steps:&proof,normal:Some(&self.normal),normal_steps:&self.normal_steps,frames:std::slice::from_ref(&self.frame),frame_arguments:&self.frame_arguments}).is_ok()
        }
        fn postcondition(&self)->ModeledActionPostconditionsRow {
            let mut out=assess(Inputs {effects:std::slice::from_ref(&self.effect),applications:std::slice::from_ref(&self.app),
                executions:std::slice::from_ref(&self.execution),execution_steps:&self.steps,arguments:&self.arguments,..Default::default()});
            assert_eq!(out.postconditions.len(),1);out.postconditions.pop().unwrap()
        }
        fn admits_postcondition(&self,row:&ModeledActionPostconditionsRow)->bool {
            let proof:Vec<_>=self.steps.iter().map(|s|SummaryFlowProofStep {kind:s.kind,evidence_id:s.evidence_id,condition_id:s.condition_id}).collect();
            admit_postcondition(row,Candidate::Effect(&self.effect),&Support {bindings:&[],arguments:&self.arguments,application:Some(&self.app),
                execution:Some(&self.execution),invocation_steps:&proof,normal:None,normal_steps:&[],frames:&[],frame_arguments:&[]}).is_ok()
        }
    }
    #[test]
    fn normal_postconditions_preserve_the_obligation_without_activating_fallible_actions() {
        let mut c=Case::new();c.effect.exit=ModelExit::Normal;c.app.target_body_return_parameter=None;
        c.normal.normal=false;c.normal.reason=Some(BoundaryReason::UnsupportedControlFlow);c.normal_steps.clear();
        let good=c.postcondition();assert_eq!(good.reason,None);assert!(c.admits_postcondition(&good));
        assert_eq!(c.assess().reason,Some(BoundaryReason::ActionTriggerUnavailable));
        for mutation in 0..5 {
            let mut altered=good.clone();
            match mutation {0=>altered.outcome_obligation=ModelExit::Invocation,1=>altered.execution_id=None,
                2=>altered.execution_id=Some(id(80)),3=>altered.call_site_node_id=id(80),_=>altered.condition_id=Some(id(80))}
            altered.postcondition_id=postcondition_identity(&altered);
            assert!(!c.admits_postcondition(&altered),"mutation {mutation}");
        }
        c.effect.has_unresolved_remainder=true;
        assert_eq!(c.postcondition().reason,Some(BoundaryReason::UnresolvedTarget));
        c.effect.has_unresolved_remainder=false;c.steps.clear();
        assert!(c.postcondition().reason.is_some());
    }

    #[test]
    fn symbolic_acquisition_is_only_a_normal_output_postcondition_never_a_resource_identity() {
        use cpg_schema::codebook::{ModelPathKind,ModelPathRole,ModelResourceAction,ModelResourceSourceStatus};
        let c=Case::new();
        let resource=ModeledResourceSitesRow {snapshot_id:id(1),call_site_node_id:id(2),function_node_id:Some(id(4)),
            call_fact_id:id(5),pysa_fact_id:id(6),target_node_id:id(7),model_id:id(8),rule_id:id(9),target_definition_fact_id:id(10),
            resource_path_id:id(30),resource_role:ModelPathRole::Output,resource_path_kind:ModelPathKind::ReturnValue,
            source_expression_node_id:Some(id(2)),source_expression_fact_id:Some(id(5)),source_status:ModelResourceSourceStatus::CallResult,
            source_reason:None,action:ModelResourceAction::Acquire,exit:ModelExit::Normal,target_modality:Modality::Definite,
            model_modality:Modality::Definite,candidate_set_complete_under_model:true,has_unresolved_remainder:false,origin:Origin::SyntheticModel};
        for mutation in 0..8 {
            let mut r=resource.clone();
            match mutation {1=>r.action=ModelResourceAction::Release,2=>r.resource_role=ModelPathRole::Input,
                3=>r.source_status=ModelResourceSourceStatus::Unknown,4=>r.source_expression_fact_id=Some(id(80)),
                5=>r.has_unresolved_remainder=true,6=>r.source_reason=Some(BoundaryReason::MissingEvidence),
                7=>r.model_modality=Modality::Candidate,_=>{}}
            let out=assess(Inputs {resources:&[r],applications:std::slice::from_ref(&c.app),
                executions:std::slice::from_ref(&c.execution),execution_steps:&c.steps,arguments:&c.arguments,..Default::default()});
            assert_eq!(out.postconditions[0].reason.is_none(),mutation==0,"mutation {mutation}");
            assert!(out.assessments[0].reason.is_some(),"a symbolic endpoint never proves an acquired resource");
        }
    }

    #[test]
    fn normal_postconditions_preserve_the_original_invocation_proof_cap() {
        for prefix_count in [56,57] {
            let mut c=Case::new();c.effect.exit=ModelExit::Normal;
            let prefix:Vec<_>=(0..prefix_count).map(|_|SummaryFlowProofStep {
                kind:K::CompletionStatement,evidence_id:id(80),condition_id:c.execution.condition_id}).collect();
            let mut steps:Vec<_>=prefix.iter().map(|s|CallExecutionStepsRow {snapshot_id:id(1),execution_id:Id::ZERO,
                ordinal:0,kind:s.kind,evidence_id:s.evidence_id,condition_id:s.condition_id}).collect();
            steps.extend(c.steps);c.execution.prefix_count=prefix_count;c.execution.prefix_digest=proof_digest(&prefix);
            c.execution.execution_id=cpg_schema::call_execution::identity(&c.execution);
            for(i,s)in steps.iter_mut().enumerate(){s.ordinal=i as i64;s.execution_id=c.execution.execution_id;}
            c.steps=steps;
            assert_eq!(c.postcondition().reason.is_none(),prefix_count==56);
        }
    }
    #[test]
    fn action_trigger_selection_preserves_modality_and_subjectless_effects() {
        let mut c=Case::new();
        for trigger in [ModelExit::Invocation,ModelExit::Normal,ModelExit::Finally] {
            c.effect.exit=trigger;
            let row=c.assess();assert_eq!(row.reason,None);assert!(c.admits(&row));
            assert_eq!(Candidate::Effect(&c.effect).view().modality,Modality::Potential);
            assert_eq!(row.channel,SummaryChannel::Effect);
        }
        c.effect.exit=ModelExit::Exceptional;
        assert_eq!(c.assess().reason,Some(BoundaryReason::ActionTriggerUnavailable));
        c.normal.normal=false;c.normal.reason=Some(BoundaryReason::UnsupportedControlFlow);
        c.normal.evidence_id=None;c.normal.status=E::Unknown;c.normal_steps.clear();
        for trigger in [ModelExit::Normal,ModelExit::Finally] {
            c.effect.exit=trigger;assert_eq!(c.assess().reason,Some(BoundaryReason::ActionTriggerUnavailable));
        }
        c.effect.exit=ModelExit::Invocation;assert_eq!(c.assess().reason,None);
        c.effect.subject_status=ModelEffectSubjectStatus::Unknown;c.effect.subject_reason=Some(BoundaryReason::AmbiguousBinding);
        assert_eq!(c.assess().reason,Some(BoundaryReason::AmbiguousBinding));
    }
    #[test]
    fn action_proofs_reject_foreign_scope_and_deleted_or_reordered_evidence() {
        let c=Case::new();let good=c.assess();
        for field in 0..7 {
            let mut forged=good.clone();
            match field {
                0=>forged.call_site_node_id=id(70),1=>forged.model_id=id(70),2=>forged.condition_id=Some(id(70)),
                3=>forged.execution_id=Some(id(70)),4=>forged.candidate_id=id(70),5=>forged.channel=SummaryChannel::Callback,
                _=>forged.function_node_id=Some(id(70)),
            }
            forged.assessment_id=identity(&forged);assert!(!c.admits(&forged),"field {field}");
        }
        let mut c=Case::new();c.app.phase=InvocationPhase::Init;assert!(c.assess().reason.is_some());
        for mutation in 0..5 {
            let mut c=Case::new();c.effect.exit=ModelExit::Normal;
            let good=c.assess();assert_eq!(good.reason,None);
            match mutation {
                0=>c.steps.clear(),1=>{c.steps.swap(0,1);for(i,s)in c.steps.iter_mut().enumerate(){s.ordinal=i as i64;}},2=>c.normal_steps.clear(),
                3=>{c.normal_steps[5].evidence_id=id(70);},_=>{c.normal_steps.swap(0,1);for(i,s)in c.normal_steps.iter_mut().enumerate(){s.ordinal=i as i64;}},
            }
            assert!(!c.admits(&good),"mutation {mutation}");
            assert!(c.assess().reason.is_some(),"mutation {mutation}");
        }
        let mut shuffled=Case::new();shuffled.effect.exit=ModelExit::Normal;let before=shuffled.assess();
        shuffled.steps.reverse();shuffled.normal_steps.reverse();assert_eq!(shuffled.assess(),before);
        let mut c=Case::new();c.effect.exit=ModelExit::Normal;c.normal.normal=false;
        c.normal.reason=Some(BoundaryReason::ExpressionWorkLimit);
        assert_eq!(c.assess().reason,Some(BoundaryReason::ExpressionWorkLimit));
    }
    #[test]
    fn action_subject_requires_the_exact_call_binding_and_argument() {
        use cpg_schema::codebook::{ModelPathKind,ModelArgumentStatus,ModelPathRole,ArgumentKind};
        use cpg_schema::behavior::ModelArgumentBindingsRow;
        use cpg_schema::tables::ArgumentsRow;
        let mut c=Case::new();c.effect.subject_status=ModelEffectSubjectStatus::BoundArgument;
        c.effect.subject_path_id=Some(id(30));c.effect.subject_path_kind=Some(ModelPathKind::Parameter);
        c.effect.subject_expression_node_id=Some(id(31));c.effect.subject_expression_fact_id=Some(id(32));
        let binding=ModelArgumentBindingsRow {snapshot_id:id(1),call_site_node_id:id(2),pysa_fact_id:id(6),model_id:id(8),
            target_node_id:id(7),rule_id:id(9),path_role:ModelPathRole::Input,path_id:id(30),formal_name:"stream".into(),
            signature_count:1,matched_signatures:1,argument_node_id:Some(id(31)),argument_fact_id:Some(id(32)),
            status:ModelArgumentStatus::Bound,reason:None};
        let argument=ArgumentsRow {snapshot_id:id(1),fact_id:id(32),node_id:id(31),call_node_id:id(2),ordinal:0,
            kind:ArgumentKind::Positional,keyword:None,start_byte:1,end_byte:2,value_start_byte:1,value_end_byte:2};
        // Include the argument's syntax and binding in the invocation commitment.
        c.execution.argument_count=1;
        for(kind,evidence_id)in [(K::ParameterBinding,id(34)),(K::ExpressionSyntax,id(33))] {
            c.steps.insert(3,CallExecutionStepsRow {snapshot_id:id(1),execution_id:Id::ZERO,ordinal:0,
                kind,evidence_id,condition_id:c.execution.condition_id});
        }
        for(i,s)in c.steps.iter_mut().enumerate() {s.ordinal=i as i64;}
        let proof:Vec<_>=c.steps.iter().map(|s|SummaryFlowProofStep {kind:s.kind,evidence_id:s.evidence_id,condition_id:s.condition_id}).collect();
        c.execution.invocation_count=proof.len() as i64;c.execution.invocation_digest=proof_digest(&proof);
        c.execution.execution_id=cpg_schema::call_execution::identity(&c.execution);
        for s in &mut c.steps {s.execution_id=c.execution.execution_id;}
        // This is a contract-only candidate, not a production declaration or source proof.
        for mutation in 0..4 {
            let mut binding=binding.clone();let mut argument=argument.clone();
            match mutation {1=>binding.call_site_node_id=id(70),2=>argument.call_node_id=id(70),3=>binding.matched_signatures=0,_=>{}}
            let rows=assess(Inputs {effects:std::slice::from_ref(&c.effect),applications:std::slice::from_ref(&c.app),
                executions:std::slice::from_ref(&c.execution),execution_steps:&c.steps,
                bindings:std::slice::from_ref(&binding),arguments:std::slice::from_ref(&argument),..Default::default()});
            assert_eq!(rows.assessments[0].reason.is_none(),mutation==0);
            let mut normal_effect=c.effect.clone();normal_effect.exit=ModelExit::Normal;
            let rows=assess(Inputs {effects:&[normal_effect],applications:std::slice::from_ref(&c.app),
                executions:std::slice::from_ref(&c.execution),execution_steps:&c.steps,
                bindings:&[binding],arguments:&[argument],..Default::default()});
            assert_eq!(rows.postconditions[0].reason.is_none(),mutation==0);
        }
    }
    #[test]
    fn normal_action_composition_keeps_the_original_64_step_cap() {
        for prefix_count in [54,55] {
            let mut c=Case::new();c.effect.exit=ModelExit::Normal;
            let prefix:Vec<_>=(0..prefix_count).map(|_|SummaryFlowProofStep {
                kind:K::CompletionStatement,evidence_id:id(80),condition_id:c.execution.condition_id}).collect();
            let mut steps:Vec<_>=prefix.iter().map(|s|CallExecutionStepsRow {snapshot_id:id(1),execution_id:Id::ZERO,
                ordinal:0,kind:s.kind,evidence_id:s.evidence_id,condition_id:s.condition_id}).collect();
            steps.extend(c.steps);
            c.execution.prefix_count=prefix_count;c.execution.prefix_digest=proof_digest(&prefix);
            c.execution.execution_id=cpg_schema::call_execution::identity(&c.execution);
            for(i,s)in steps.iter_mut().enumerate(){s.ordinal=i as i64;s.execution_id=c.execution.execution_id;}
            c.steps=steps;
            assert_eq!(c.assess().reason,if prefix_count==54 {None} else {Some(BoundaryReason::SummaryProofLimit)});
        }
    }

    #[test]
    fn malformed_authored_modalities_and_validation_schemas_cannot_activate() {
        let mut c=Case::new();c.effect.model_modality=Modality::Candidate;
        assert!(c.assess().reason.is_some());
        c.effect.model_modality=Modality::Potential;c.effect.effect=ModelEffectKind::Validate;
        assert!(c.assess().reason.is_some());
        c.effect.schema_kind=Some(cpg_schema::codebook::ModelSchemaKind::StaticClass);
        c.effect.schema_class_node_id=Some(id(90));c.effect.schema_class_fact_id=Some(id(91));
        assert_eq!(c.assess().reason,None);
        c.effect.schema_path_id=Some(id(92));assert!(c.assess().reason.is_some());
    }

}
