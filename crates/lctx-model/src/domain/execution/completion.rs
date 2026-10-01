//! The base completion channel consumes independently admitted base evaluations. It does not
//! evaluate source calls, read enriched outcomes, or use its own body outcome as a premise.
use crate::domain::{*, analysis::native::NativeAssertionPremise, lexical::SyntaxField as F, obligation::ObligationKind, resources::ResourceBudget, source::{Occurrence,SyntaxKind as S}, normalized::entities::EntityRef, syntax::SyntaxPlacement};
use super::{evaluation::{EvaluationData,ExpressionRequest,CheckedEvaluation,ReleaseSafety,Evaluator,EvaluationError,boundary,with_completion_syntax},outcome::PendingOutcome,ExactRuntimeException};

pub const COMPLETION_DEPTH_LIMIT:usize=128;
pub const COMPLETION_WORK_LIMIT:usize=4096;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct CompletionRequest {pub input:Id<input::InputRevision>,pub context:Id<attribution::AnalysisContext>,pub owner:Id<EntityRef>,pub statement:Id<Occurrence>}
pub struct CheckedCompletion {
    request:CompletionRequest, outcome:PendingOutcome,
    native:Vec<Id<NativeAssertionPremise>>, statements:Vec<Id<Occurrence>>,
    expressions:Vec<Id<Occurrence>>, releases:Vec<(Id<Occurrence>,ReleaseSafety)>,
    facts:Vec<super::records::EvaluationFacts>, headers:Vec<Id<super::source_call_records::SourceCallHeader>>,qualification:Id<assertion::AssertionQualification>, status:analysis::policy::EvidenceStatus,
    _charge:charged::StateCharge, _results_charge:charged::StateCharge,
}
impl CheckedCompletion {
    pub fn request(&self)->CompletionRequest {self.request}
    pub fn outcome(&self)->PendingOutcome {self.outcome}
    pub fn native_premises(&self)->&[Id<NativeAssertionPremise>] {&self.native}
    pub fn entered_statements(&self)->&[Id<Occurrence>] {&self.statements}
    pub fn evaluated_expressions(&self)->&[Id<Occurrence>] {&self.expressions}
    pub fn release_inputs(&self)->&[(Id<Occurrence>,ReleaseSafety)] {&self.releases}
    pub fn qualification(&self)->Id<assertion::AssertionQualification> {self.qualification}
    pub fn status(&self)->analysis::policy::EvidenceStatus {self.status}
    pub(crate) fn header_premises(&self)->&[Id<super::source_call_records::SourceCallHeader>]{&self.headers}
    pub(crate) fn evaluation_facts(&self)->&[super::records::EvaluationFacts] {&self.facts}
}
struct Kernel<'a,'b,'c> {data:&'a EvaluationData,request:CompletionRequest,evaluations:&'a [&'a CheckedEvaluation],available_headers:&'a [(&'a super::source_call::CheckedSourceBinding,&'a super::source_call_records::SourceCallHeader)],headers:Vec<Id<super::source_call_records::SourceCallHeader>>,syntax:&'b mut Evaluator<'c>,statements:Vec<Id<Occurrence>>,expressions:Vec<Id<Occurrence>>,releases:Vec<(Id<Occurrence>,ReleaseSafety)>,facts:Vec<super::records::EvaluationFacts>,status:analysis::policy::EvidenceStatus,charge:charged::StateCharge,active:Option<ExactRuntimeException>}
type Result<T>=std::result::Result<T,EvaluationError>;
fn one(nodes:&[SyntaxPlacement],field:F)->Result<Id<Occurrence>> {let mut rows=nodes.iter().filter(|row|row.field==field);let row=rows.next().ok_or_else(||boundary(ObligationKind::UnsupportedControlFlow))?;if rows.next().is_some(){return Err(boundary(ObligationKind::MissingEvidence));}Ok(row.occurrence)}
impl Kernel<'_,'_,'_> {
    fn expression(&mut self,expression:Id<Occurrence>)->Result<&CheckedEvaluation> {
        self.syntax.tick(self.evaluations.len()).map_err(boundary)?;
        let mut matching=self.evaluations.iter().filter(|proof|proof.request().expression==expression);
        let proof=*matching.next().ok_or_else(||boundary(ObligationKind::MissingEvidence))?;
        if matching.next().is_some() || proof.request().input!=self.request.input || proof.request().context!=self.request.context || proof.request().owner!=self.request.owner {return Err(boundary(ObligationKind::IncompatibleContexts));}
        if self.expressions.len()==64 {return Err(boundary(ObligationKind::SummaryProofLimit));}
        self.charge.grow(size_of::<Id<Occurrence>>() * 2+size_of::<(Id<Occurrence>,ReleaseSafety)>() * 2+size_of::<super::records::EvaluationFacts>()*2)?;
        self.expressions.push(expression);self.releases.push((expression,proof.release()));self.facts.push(super::records::EvaluationFacts::of(proof));self.status=analysis::support::inferred_status(analysis::Interpretation::Structural,[self.status,proof.status()]);Ok(proof)
    }
    fn suite(&mut self,children:&[SyntaxPlacement],field:F,depth:usize)->Result<PendingOutcome> {
        self.charge.grow(children.len().checked_mul(size_of::<&SyntaxPlacement>()*2).ok_or_else(||ModelError::Invalid("completion suite allowance overflow".into()))?)?;
        let mut statements=children.iter().filter(|row|row.field==field).collect::<Vec<_>>();
        statements.sort_by_key(|row|(row.ordinal,row.occurrence));
        for (ordinal,row) in statements.iter().enumerate() {if row.ordinal!=ordinal as i64{return Err(boundary(ObligationKind::MissingEvidence));}}
        for row in statements {let result=self.statement(row.occurrence,depth+1)?;if !result.is_normal(){return Ok(result);}}
        Ok(PendingOutcome::Normal)
    }
    fn statement(&mut self,site:Id<Occurrence>,depth:usize)->Result<PendingOutcome> {
        if depth>COMPLETION_DEPTH_LIMIT {return Err(boundary(ObligationKind::CompletionDepthLimit));}
        self.syntax.observe(site)?;
        let kind=self.data.occurrences.get(site).ok_or_else(||boundary(ObligationKind::MissingEvidence))?.syntax_kind;
        let children=self.syntax.children(site)?;
        let outcome=match kind {
            S::StmtPass if children.is_empty()=>PendingOutcome::Normal,
            S::StmtFunctionDef=>{
                let mut admitted=self.available_headers.iter().filter(|(proof,_)|proof.declaration()==site&&proof.caller()==self.request.owner&&proof.request().input==self.request.input&&proof.request().context==self.request.context);
                let(proof,row)=admitted.next().ok_or_else(||boundary(ObligationKind::MissingEvidence))?;if admitted.next().is_some(){return Err(boundary(ObligationKind::AmbiguousBinding));}
                self.charge.grow(size_of::<Id<super::source_call_records::SourceCallHeader>>()*2)?;self.headers.push(row.id());self.status=analysis::support::inferred_status(analysis::Interpretation::Structural,[self.status,proof.status()]);PendingOutcome::Normal
            },
            S::StmtExpr if children.len()==1=>{match self.expression(one(&children,F::Value)?)?.exception(){Some((site,exception))=>PendingOutcome::Raise{site,exception},None=>PendingOutcome::Normal}},
            S::StmtReturn=>{let exception=if !children.is_empty(){if children.len()!=1{return Err(boundary(ObligationKind::UnsupportedControlFlow));}self.expression(one(&children,F::Value)?)?.exception()}else{None};match exception{Some((site,exception))=>PendingOutcome::Raise{site,exception},None=>PendingOutcome::Return{site}}},
            S::StmtRaise if children.is_empty()=>PendingOutcome::Raise{site,exception:self.active.ok_or_else(||boundary(ObligationKind::UnsupportedControlFlow))?},
            S::StmtRaise=>{
                let expression=one(&children,F::Exc)?;self.expression(expression)?;
                let primitive=self.data.occurrences.get(expression).is_some_and(|node|matches!(node.syntax_kind,S::ExprNoneLiteral|S::ExprBooleanLiteral|S::ExprNumberLiteral|S::ExprStringLiteral|S::ExprBytesLiteral|S::ExprEllipsisLiteral));
                if !primitive || children.iter().any(|row|!matches!(row.field,F::Exc|F::Cause)){return Err(boundary(ObligationKind::UnsupportedControlFlow));}
                if children.iter().any(|row|row.field==F::Cause){self.expression(one(&children,F::Cause)?)?;}
                PendingOutcome::Raise{site,exception:ExactRuntimeException::TypeError}
            },
            S::StmtBreak if children.is_empty()=>PendingOutcome::Break{site},
            S::StmtContinue if children.is_empty()=>PendingOutcome::Continue{site},
            S::StmtIf=>{
                let truth=self.expression(one(&children,F::Test)?)?.truth().ok_or_else(||boundary(ObligationKind::UnsupportedControlFlow))?;
                if truth {self.suite(&children,F::Body,depth)?} else {
                    let mut outcome=PendingOutcome::Normal;
                    for clause in children.iter().filter(|row|row.field==F::Orelse){self.syntax.observe(clause.occurrence)?;
                        if self.data.occurrences.get(clause.occurrence).is_none_or(|node|node.syntax_kind!=S::ElifElseClause){return Err(boundary(ObligationKind::UnsupportedControlFlow));}
                        let clause_children=self.syntax.children(clause.occurrence)?;
                        let selected=if clause_children.iter().any(|row|row.field==F::Test){self.expression(one(&clause_children,F::Test)?)?.truth().ok_or_else(||boundary(ObligationKind::UnsupportedControlFlow))?}else{true};
                        if selected{outcome=self.suite(&clause_children,F::Body,depth+1)?;break;}
                    }outcome
                }
            },
            S::StmtTry=>{
                if children.iter().any(|row|!matches!(row.field,F::Body|F::Handler|F::Orelse|F::Finalbody)){return Err(boundary(ObligationKind::UnsupportedControlFlow));}
                let mut pending=self.suite(&children,F::Body,depth)?;
                if let Some(exception)=pending.exception(){
                    // A bare handler matches the pending exception without a hierarchy claim.
                    // Typed matching and named-handler disposal need their own earlier certificates.
                    for handler in children.iter().filter(|row|row.field==F::Handler){self.syntax.observe(handler.occurrence)?;
                        if self.data.occurrences.get(handler.occurrence).is_none_or(|node|node.syntax_kind!=S::ExceptHandlerExceptHandler){return Err(boundary(ObligationKind::MissingEvidence));}
                        let handler_children=self.syntax.children(handler.occurrence)?;
                        // Python requires a type for except*. A valid, fully captured handler
                        // whose only children are body statements is the bare except form.
                        if handler_children.iter().any(|row|row.field!=F::Body){return Err(boundary(ObligationKind::UnsupportedControlFlow));}
                        let active=self.active;self.active=Some(exception);let result=self.suite(&handler_children,F::Body,depth+1);self.active=active;pending=result?;break;
                    }
                } else if pending.is_normal(){pending=self.suite(&children,F::Orelse,depth)?;}
                let active=self.active;if let Some(exception)=pending.exception(){self.active=Some(exception);}
                let finalizer=self.suite(&children,F::Finalbody,depth);self.active=active;
                pending.after_finalizer(finalizer?)
            },
            _=>return Err(boundary(ObligationKind::UnsupportedControlFlow)),
        };
        if self.statements.len()==64 {return Err(boundary(ObligationKind::SummaryProofLimit));}
        self.charge.grow(size_of::<Id<Occurrence>>() * 2)?;self.statements.push(site);Ok(outcome)
    }
}

/// Complete one entered statement using base evidence only. The caller must retain the actual
/// earlier evaluation tokens while this operation runs; persisted inputs are independently replayed.
pub fn complete(data:&EvaluationData,request:CompletionRequest,evaluations:&[&CheckedEvaluation],budget:&ResourceBudget)->std::result::Result<std::result::Result<CheckedCompletion,ObligationKind>,ModelError> {
    complete_with_headers(data,request,evaluations,&[],budget)
}
pub(crate) fn complete_with_headers(data:&EvaluationData,request:CompletionRequest,evaluations:&[&CheckedEvaluation],headers:&[(&super::source_call::CheckedSourceBinding,&super::source_call_records::SourceCallHeader)],budget:&ResourceBudget)->std::result::Result<std::result::Result<CheckedCompletion,ObligationKind>,ModelError>{
    let expression_request=ExpressionRequest{input:request.input,context:request.context,owner:request.owner,expression:request.statement};
    with_completion_syntax(data,expression_request,budget,|syntax| {
        let mut kernel=Kernel{data,request,evaluations,available_headers:headers,headers:Vec::new(),syntax,statements:Vec::new(),expressions:Vec::new(),releases:Vec::new(),facts:Vec::new(),status:analysis::policy::EvidenceStatus::StructurallyObserved,charge:charged::StateCharge::new(budget,"base-completion-results"),active:None};
        let outcome=kernel.statement(request.statement,0)?;
        let qualification=kernel.syntax.qualification(request.statement).map_err(boundary)?;
        let status=analysis::support::inferred_status(analysis::Interpretation::Structural,[kernel.syntax.status(),kernel.status]);
        let (native,native_charge)=kernel.syntax.take_admission();
        Ok(CheckedCompletion {request,outcome,native,statements:kernel.statements,expressions:kernel.expressions,releases:kernel.releases,facts:kernel.facts,headers:kernel.headers,qualification,status,_charge:native_charge,_results_charge:kernel.charge})
    })
}
