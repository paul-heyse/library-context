//! Candidate-backed action assessments. A supported trigger activates the authored modality,
//! never converts a potential action to an observed action, and never closes coverage.
use crate::behavior::{ModeledEffectSitesRow,ModeledCallbackSitesRow,ModeledResourceSitesRow,
    ModelApplicationsRow,ModelArgumentBindingsRow,ExpressionEvaluationsRow,ExpressionEvaluationStepsRow};
use crate::call_execution::CallExecutionsRow;
use crate::codebook::{Codebook,BoundaryReason as B,SummaryChannel as C,ModelExit,Modality,
    ModelEffectSubjectStatus,ModelArgumentStatus,ModelResourceSourceStatus,ModelPathKind,
    ModelSchemaKind,ModelPathRole,ModelEffectKind,InvocationPhase,Origin,SummaryFlowStepKind as K,ModeledArgumentEvaluationStatus as E};
use crate::id::{Id,Digest,IdHasher,recipe::SummaryFlowProofStep};
use crate::summary_contract::{ProofAdmissionError,MAX_SUMMARY_PROOF_STEPS};
use crate::table::table;

table!(
    /// One assessment for each effect/callback/resource candidate, including unresolved ones.
    /// Descriptor, modality and subject stay in the typed candidate named by channel and key.
    /// A supported Potential rule is still potential, never an established occurrence.
    ModeledActionAssessments,ModeledActionAssessmentsRow = "modeled_action_assessments",
    family = Findings,
    key = [snapshot_id,assessment_id],
    checks = [("normal_count_nonnegative","normal_step_count >= 0")],
    {
        snapshot_id:Id,
        assessment_id:Id,
        candidate_id:Id,
        channel:C,
        function_node_id:Option<Id>,
        call_site_node_id:Id,
        pysa_fact_id:Id,
        model_id:Id,
        rule_id:Id,
        execution_id:Option<Id>,
        condition_id:Option<Id>,
        normal_syntax_fact_id:Option<Id>,
        normal_step_count:i64,
        normal_steps_digest:Digest,
        reason:Option<B>,
    }
);

table!(
    /// If this exact reached invocation returns normally, its authored Normal rule holds.
    /// A non-refused row proves the implication, never feasibility/occurrence of the outcome.
    /// Returned resource paths are symbolic results here, never concrete resource identities.
    ModeledActionPostconditions,ModeledActionPostconditionsRow = "modeled_action_postconditions",
    family = Findings,
    key = [snapshot_id,postcondition_id],
    checks = [("normal_obligation","outcome_obligation = 0")],
    {
        snapshot_id:Id,
        postcondition_id:Id,
        candidate_id:Id,
        channel:C,
        function_node_id:Option<Id>,
        call_site_node_id:Id,
        pysa_fact_id:Id,
        model_id:Id,
        rule_id:Id,
        execution_id:Option<Id>,
        condition_id:Option<Id>,
        /// Mandatory even when an activated assessment separately proves this outcome.
        outcome_obligation:ModelExit,
        reason:Option<B>,
    }
);

pub fn postcondition_identity(r:&ModeledActionPostconditionsRow)->Id {
    IdHasher::new("modeled-action-postcondition").id(r.candidate_id).i64(i64::from(r.channel.code()))
        .opt_id(r.function_node_id).id(r.call_site_node_id).id(r.pysa_fact_id).id(r.model_id).id(r.rule_id)
        .opt_id(r.execution_id).opt_id(r.condition_id).i64(i64::from(r.outcome_obligation.code()))
        .opt_i64(r.reason.map(|x|i64::from(x.code()))).finish_id()
}

/// Subject interpretation belongs to the proof kind, not to a consumer's missing-ID fallback.
enum SubjectMeaning { ActualValue, NormalPostcondition }

impl Candidate<'_> {
    fn symbolic_normal_acquisition(self)->bool {
        matches!(self,Self::Resource(r) if r.exit==ModelExit::Normal
            && r.action==crate::codebook::ModelResourceAction::Acquire
            && r.resource_role==ModelPathRole::Output && r.resource_path_kind==ModelPathKind::ReturnValue
            && r.source_status==ModelResourceSourceStatus::CallResult && r.source_reason.is_none()
            && r.source_expression_node_id==Some(r.call_site_node_id) && r.source_expression_fact_id==Some(r.call_fact_id))
    }
}

pub fn admit_postcondition(r:&ModeledActionPostconditionsRow,candidate:Candidate<'_>,support:&Support<'_>)
    ->Result<(),ProofAdmissionError> {
    let v=candidate.view();
    if r.postcondition_id!=postcondition_identity(r) || r.snapshot_id!=v.snapshot_id || r.candidate_id!=v.candidate_id
        || r.channel!=v.channel || r.function_node_id!=v.function_node_id || r.call_site_node_id!=v.call_site_node_id
        || r.pysa_fact_id!=v.pysa_fact_id || r.model_id!=v.model_id || r.rule_id!=v.rule_id
        || r.outcome_obligation!=ModelExit::Normal || v.trigger!=ModelExit::Normal {
        return Err("postcondition lacks its exact candidate and mandatory Normal obligation".into());
    }
    if r.reason.is_some() {return Ok(());}
    let (_,execution)=admit_invocation(candidate,support,SubjectMeaning::NormalPostcondition)?;
    if r.execution_id!=Some(execution.execution_id) || r.condition_id!=Some(execution.condition_id) {
        return Err("postcondition outcome obligation belongs to another invocation".into());
    }
    Ok(())
}

#[derive(Clone,Copy)]
pub enum Candidate<'a> {
    Effect(&'a ModeledEffectSitesRow),
    Callback(&'a ModeledCallbackSitesRow),
    Resource(&'a ModeledResourceSitesRow),
}

/// Mechanical common view of existing channel-specific declarations; no second model grammar.
pub struct CandidateView {
    pub snapshot_id:Id,pub candidate_id:Id,pub channel:C,pub function_node_id:Option<Id>,
    pub call_site_node_id:Id,pub call_fact_id:Id,pub pysa_fact_id:Id,pub target_node_id:Id,
    pub target_definition_fact_id:Id,pub model_id:Id,pub rule_id:Id,pub trigger:ModelExit,
    pub modality:Modality,pub source_reason:Option<B>,
}

impl Candidate<'_> {
    fn bindings(self)->Vec<(ModelPathRole,Id,Option<Id>,Option<Id>)> {
        match self {
            Self::Effect(r)=>r.subject_path_id.map(|p|(ModelPathRole::Input,p,r.subject_expression_node_id,r.subject_expression_fact_id))
                .into_iter().chain(r.schema_path_id.map(|p|(ModelPathRole::Schema,p,r.schema_expression_node_id,r.schema_expression_fact_id))).collect(),
            Self::Callback(r)=>vec![(ModelPathRole::Input,r.callback_path_id,r.argument_node_id,r.argument_fact_id)],
            Self::Resource(r)=>vec![(r.resource_role,r.resource_path_id,r.source_expression_node_id,r.source_expression_fact_id)],
        }
    }

    pub fn view(self)->CandidateView {
        let mut h=IdHasher::new("modeled-action-candidate");
        let (mut v,target_modality,complete,open,origin)=match self {
            Self::Effect(r)=>{
                h.i64(i64::from(r.effect.code())).opt_str(r.argument.as_deref())
                    .opt_i64(r.schema_kind.map(|x|i64::from(x.code())))
                    .opt_id(r.schema_class_node_id).opt_id(r.schema_class_fact_id)
                    .opt_id(r.schema_path_id).opt_i64(r.schema_path_kind.map(|x|i64::from(x.code())))
                    .opt_id(r.schema_expression_node_id).opt_id(r.schema_expression_fact_id)
                    .opt_i64(r.schema_reason.map(|x|i64::from(x.code())))
                    .opt_id(r.subject_path_id).opt_i64(r.subject_path_kind.map(|x|i64::from(x.code())))
                    .opt_id(r.subject_expression_node_id).opt_id(r.subject_expression_fact_id)
                    .i64(i64::from(r.subject_status.code())).opt_i64(r.subject_reason.map(|x|i64::from(x.code())));
                let subject=match r.subject_status {
                    ModelEffectSubjectStatus::Unqualified if r.subject_path_id.is_none()
                        && r.subject_path_kind.is_none() && r.subject_expression_node_id.is_none()
                        && r.subject_expression_fact_id.is_none() && r.subject_reason.is_none()=>None,
                    ModelEffectSubjectStatus::BoundArgument if r.subject_path_id.is_some()
                        && r.subject_path_kind==Some(ModelPathKind::Parameter) && r.subject_expression_node_id.is_some()
                        && r.subject_expression_fact_id.is_some() && r.subject_reason.is_none()=>None,
                    _=>Some(r.subject_reason.unwrap_or(B::MissingEvidence)),
                };
                let schema=match r.schema_kind {
                    None if r.effect!=ModelEffectKind::Validate && r.schema_class_node_id.is_none()
                        && r.schema_class_fact_id.is_none() && r.schema_path_id.is_none() && r.schema_path_kind.is_none()
                        && r.schema_expression_node_id.is_none() && r.schema_expression_fact_id.is_none() && r.schema_reason.is_none()=>None,
                    Some(ModelSchemaKind::StaticClass) if r.effect==ModelEffectKind::Validate
                        && r.schema_class_node_id.is_some() && r.schema_class_fact_id.is_some() && r.schema_reason.is_none()
                        && r.schema_path_id.is_none() && r.schema_path_kind.is_none()
                        && r.schema_expression_node_id.is_none() && r.schema_expression_fact_id.is_none()=>None,
                    Some(ModelSchemaKind::RuntimeValue) if r.effect==ModelEffectKind::Validate
                        && r.schema_class_node_id.is_none() && r.schema_class_fact_id.is_none() && r.schema_path_id.is_some()
                        && r.schema_path_kind==Some(ModelPathKind::Parameter)
                        && r.schema_expression_node_id.is_some() && r.schema_expression_fact_id.is_some()
                        && r.schema_reason.is_none()=>None,
                    _=>Some(r.schema_reason.unwrap_or(B::MissingEvidence)),
                };
                (CandidateView {snapshot_id:r.snapshot_id,candidate_id:Id::ZERO,channel:C::Effect,
                    function_node_id:r.function_node_id,call_site_node_id:r.call_site_node_id,call_fact_id:r.call_fact_id,
                    pysa_fact_id:r.pysa_fact_id,target_node_id:r.target_node_id,target_definition_fact_id:r.target_definition_fact_id,
                    model_id:r.model_id,rule_id:r.rule_id,trigger:r.exit,modality:r.model_modality,source_reason:subject.or(schema)},
                    r.target_modality,r.candidate_set_complete_under_model,r.has_unresolved_remainder,r.origin)
            },
            Self::Callback(r)=>{
                h.id(r.callback_path_id).opt_id(r.argument_node_id).opt_id(r.argument_fact_id)
                    .i64(i64::from(r.action.code())).i64(i64::from(r.binding_status.code()))
                    .opt_i64(r.binding_reason.map(|x|i64::from(x.code())));
                let reason=if r.binding_status==ModelArgumentStatus::Bound && r.argument_node_id.is_some()
                    && r.argument_fact_id.is_some() && r.binding_reason.is_none() {None}
                    else {Some(r.binding_reason.unwrap_or(B::MissingEvidence))};
                (CandidateView {snapshot_id:r.snapshot_id,candidate_id:Id::ZERO,channel:C::Callback,
                    function_node_id:r.function_node_id,call_site_node_id:r.call_site_node_id,call_fact_id:r.call_fact_id,
                    pysa_fact_id:r.pysa_fact_id,target_node_id:r.target_node_id,target_definition_fact_id:r.target_definition_fact_id,
                    model_id:r.model_id,rule_id:r.rule_id,trigger:r.exit,modality:r.model_modality,source_reason:reason},
                    r.target_modality,r.candidate_set_complete_under_model,r.has_unresolved_remainder,r.origin)
            },
            Self::Resource(r)=>{
                h.id(r.resource_path_id).i64(i64::from(r.resource_role.code())).i64(i64::from(r.resource_path_kind.code()))
                    .opt_id(r.source_expression_node_id).opt_id(r.source_expression_fact_id)
                    .i64(i64::from(r.source_status.code())).opt_i64(r.source_reason.map(|x|i64::from(x.code())))
                    .i64(i64::from(r.action.code()));
                // No source-occurrence ID is relabelled as a returned resource identity.
                let reason=if r.resource_path_kind==ModelPathKind::ReturnValue {Some(B::ResourceIdentityUnavailable)}
                    else if r.source_status==ModelResourceSourceStatus::BoundArgument && r.source_expression_node_id.is_some()
                        && r.source_expression_fact_id.is_some() && r.source_reason.is_none() {None}
                    else {Some(r.source_reason.unwrap_or(B::MissingEvidence))};
                (CandidateView {snapshot_id:r.snapshot_id,candidate_id:Id::ZERO,channel:C::Resource,
                    function_node_id:r.function_node_id,call_site_node_id:r.call_site_node_id,call_fact_id:r.call_fact_id,
                    pysa_fact_id:r.pysa_fact_id,target_node_id:r.target_node_id,target_definition_fact_id:r.target_definition_fact_id,
                    model_id:r.model_id,rule_id:r.rule_id,trigger:r.exit,modality:r.model_modality,source_reason:reason},
                    r.target_modality,r.candidate_set_complete_under_model,r.has_unresolved_remainder,r.origin)
            },
        };
        h.i64(i64::from(v.channel.code())).opt_id(v.function_node_id).id(v.call_site_node_id).id(v.call_fact_id)
            .id(v.pysa_fact_id).id(v.target_node_id).id(v.target_definition_fact_id).id(v.model_id).id(v.rule_id)
            .i64(i64::from(v.trigger.code())).i64(i64::from(v.modality.code())).i64(i64::from(target_modality.code()))
            .bool(complete).bool(open).i64(i64::from(origin.code()));
        v.candidate_id=h.finish_id();
        if target_modality!=Modality::Definite || !complete || open {v.source_reason=Some(B::UnresolvedTarget);}
        if origin!=Origin::SyntheticModel || !matches!(v.modality,Modality::Definite|Modality::Potential) {
            v.source_reason=Some(B::MissingEvidence);
        }
        v
    }
}

pub fn normal_digest(steps:&[ExpressionEvaluationStepsRow])->Digest {
    let mut h=IdHasher::new("action-normal-proof");
    h.i64(steps.len() as i64);
    for s in steps {h.id(s.syntax_fact_id).i64(s.ordinal).id(s.operand_fact_id).id(s.evidence_id)
        .i64(i64::from(s.status.code())).i64(i64::from(s.kind.code()));}
    h.finish_digest()
}

pub fn identity(r:&ModeledActionAssessmentsRow)->Id {
    IdHasher::new("modeled-action-assessment").id(r.candidate_id).i64(i64::from(r.channel.code()))
        .opt_id(r.function_node_id).id(r.call_site_node_id).id(r.pysa_fact_id).id(r.model_id).id(r.rule_id)
        .opt_id(r.execution_id).opt_id(r.condition_id).opt_id(r.normal_syntax_fact_id)
        .i64(r.normal_step_count).bytes(&r.normal_steps_digest.0)
        .opt_i64(r.reason.map(|x|i64::from(x.code()))).finish_id()
}

pub struct Support<'a> {
    pub bindings:&'a [ModelArgumentBindingsRow],
    pub arguments:&'a [crate::tables::ArgumentsRow],
    pub application:Option<&'a ModelApplicationsRow>,
    pub execution:Option<&'a CallExecutionsRow>,
    pub invocation_steps:&'a [SummaryFlowProofStep],
    pub normal:Option<&'a ExpressionEvaluationsRow>,
    pub normal_steps:&'a [ExpressionEvaluationStepsRow],
}

/// Shared structural closure. Source publication also reconstructs the candidates, callee,
/// argument bindings, ordered invocation and normal-expression semantics from source facts.
pub fn admit(r:&ModeledActionAssessmentsRow,candidate:Candidate<'_>,support:&Support<'_>)
    ->Result<(),ProofAdmissionError> {
    let v=candidate.view();
    if r.assessment_id!=identity(r) || r.snapshot_id!=v.snapshot_id || r.candidate_id!=v.candidate_id
        || r.channel!=v.channel || r.function_node_id!=v.function_node_id || r.call_site_node_id!=v.call_site_node_id
        || r.pysa_fact_id!=v.pysa_fact_id || r.model_id!=v.model_id || r.rule_id!=v.rule_id || r.normal_step_count<0 {
        return Err("action candidate identity or scope mismatch".into());
    }
    if r.reason.is_some() {
        if r.normal_syntax_fact_id.is_some() || r.normal_step_count!=0 || r.normal_steps_digest!=normal_digest(&[]) {
            return Err("unresolved action retains positive outcome proof".into());
        }
        return Ok(());
    }
    let(app,e)=admit_invocation(candidate,support,SubjectMeaning::ActualValue)?;
    if r.execution_id!=Some(e.execution_id) || r.condition_id!=Some(e.condition_id) {
        return Err("action execution scope mismatch".into());
    }

    match v.trigger {
        ModelExit::Invocation=>{
            if r.normal_syntax_fact_id.is_some() || r.normal_step_count!=0 || r.normal_steps_digest!=normal_digest(&[]) {
                return Err("invocation action must not borrow outcome evidence".into());
            }
        },
        ModelExit::Exceptional=>return Err(ProofAdmissionError {reason:B::ActionTriggerUnavailable,message:"no exact exceptional callee outcome"}),
        ModelExit::Normal|ModelExit::Finally=>{
            let unavailable=ProofAdmissionError {reason:B::ActionTriggerUnavailable,message:"no exact normal callee outcome"};
            let n=support.normal.ok_or(unavailable)?;
            if !n.normal {return Err(ProofAdmissionError {reason:match n.reason {
                Some(B::ExpressionDepthLimit)=>B::ExpressionDepthLimit,Some(B::ExpressionWorkLimit)=>B::ExpressionWorkLimit,
                _=>B::ActionTriggerUnavailable},message:"callee normal outcome unresolved"});}
            if n.snapshot_id!=e.snapshot_id || n.syntax_fact_id!=e.syntax_fact_id || n.status!=E::PinnedCallNormal
                || n.reason.is_some() || n.evidence_id!=Some(e.syntax_fact_id) || !app.target_normal_return
                || r.normal_syntax_fact_id!=Some(e.syntax_fact_id) || r.normal_step_count as usize!=support.normal_steps.len()
                || r.normal_steps_digest!=normal_digest(support.normal_steps) {
                return Err("action normal outcome differs from this callee".into());
            }
            let invoke=&support.invocation_steps[e.prefix_count as usize..];
            let base=&invoke[..invoke.len()-1]; // Exclude entry marker; normal proof adds its own outcome.
            let steps=support.normal_steps;
            if steps.len()<base.len()+2 || steps.len()>base.len()+3
                || steps.iter().enumerate().any(|(i,s)|s.snapshot_id!=e.snapshot_id || s.syntax_fact_id!=e.syntax_fact_id || s.ordinal!=i as i64)
                || base.iter().zip(steps).any(|(a,b)|a.kind!=b.kind || a.evidence_id!=b.evidence_id) {
                return Err("normal proof does not extend this invocation".into());
            }
            let tail=&steps[base.len()..];
            if (tail[0].kind,tail[0].evidence_id)!=(K::PrecedingCallNormal,e.model_id)
                || (tail.last().unwrap().kind,tail.last().unwrap().evidence_id)!=(K::ExpressionSyntax,e.syntax_fact_id)
                || (tail.len()==3 && tail[1].kind!=K::ModelRule)
                || tail.iter().any(|s|s.operand_fact_id!=e.syntax_fact_id || s.status!=E::PinnedCallNormal) {
                return Err("normal proof lacks this callee outcome".into());
            }
            if e.prefix_count as usize+steps.len()>MAX_SUMMARY_PROOF_STEPS {
                return Err(ProofAdmissionError {reason:B::SummaryProofLimit,message:"action proof limit"});
            }
        },
    }
    Ok(())
}

/// One owner for candidate, application, subject and invocation premises. Outcome meaning
/// remains with the assessment/postcondition consumer; no Normal proof is inferred here.
fn admit_invocation<'a>(candidate:Candidate<'_>,support:&Support<'a>,meaning:SubjectMeaning)
    ->Result<(&'a ModelApplicationsRow,&'a CallExecutionsRow),ProofAdmissionError> {
    let v=candidate.view();
    let symbolic=matches!(meaning,SubjectMeaning::NormalPostcondition) && candidate.symbolic_normal_acquisition();
    if let Some(reason)=v.source_reason {
        if !(symbolic && reason==B::ResourceIdentityUnavailable) {
            return Err(ProofAdmissionError {reason,message:"action subject or target unavailable"});
        }
    }
    let app=support.application.ok_or("action lacks its exact model application")?;
    if app.snapshot_id!=v.snapshot_id || app.call_site_node_id!=v.call_site_node_id || app.call_fact_id!=v.call_fact_id
        || app.function_node_id!=v.function_node_id || app.pysa_fact_id!=v.pysa_fact_id || app.model_id!=v.model_id
        || app.target_node_id!=v.target_node_id || app.target_definition_fact_id!=v.target_definition_fact_id
        || app.model_origin!=Origin::SyntheticModel || app.phase!=InvocationPhase::Call || app.target_count!=1 || app.target_modality!=Modality::Definite
        || !app.candidate_set_complete_under_model || app.has_unresolved_remainder {
        return Err("action application differs from the admitted callee".into());
    }
    for(role,path,node,fact)in if symbolic {Vec::new()} else {candidate.bindings()} {
        let found:Vec<_>=support.bindings.iter().filter(|b|b.snapshot_id==v.snapshot_id
            && b.call_site_node_id==v.call_site_node_id && b.pysa_fact_id==v.pysa_fact_id
            && b.model_id==v.model_id && b.rule_id==v.rule_id && b.path_role==role && b.path_id==path).collect();
        let [binding]=found.as_slice() else {return Err("action lacks one exact subject binding".into());};
        if binding.target_node_id!=v.target_node_id || binding.argument_node_id!=node || binding.argument_fact_id!=fact
            || binding.status!=ModelArgumentStatus::Bound || binding.reason.is_some() || binding.signature_count<=0
            || binding.matched_signatures!=binding.signature_count || node.is_none() || fact.is_none()
            || support.arguments.iter().filter(|a|a.snapshot_id==v.snapshot_id && a.call_node_id==v.call_site_node_id
                && Some(a.node_id)==node && Some(a.fact_id)==fact).count()!=1 {
            return Err("action subject is not this call's bound argument".into());
        }
    }
    let e=support.execution.ok_or("action lacks reached invocation")?;
    if e.snapshot_id!=v.snapshot_id || Some(e.function_node_id)!=v.function_node_id || e.call_node_id!=v.call_site_node_id
        || e.call_fact_id!=v.call_fact_id || e.pysa_fact_id!=v.pysa_fact_id || e.target_node_id!=v.target_node_id
        || e.model_id!=v.model_id || e.phase!=app.phase {
        return Err("action execution scope mismatch".into());
    }
    if let Some(reason)=e.reason {return Err(ProofAdmissionError {reason,message:"action invocation unresolved"});}
    crate::call_execution::admit(e,support.invocation_steps)?;
    if e.default_formal_count.is_some_and(|n|n>0) && !app.target_call_defaults_available {
        return Err("action lacks an authored pinned-default availability promise".into());
    }

    if support.arguments.len()!=e.argument_count as usize
        || support.arguments.iter().any(|a|a.snapshot_id!=e.snapshot_id || a.call_node_id!=e.call_node_id) {
        return Err("action argument domain differs from its invocation".into());
    }
    Ok((app,e))
}
