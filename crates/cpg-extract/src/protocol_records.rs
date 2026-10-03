//! Owned native observations encoded while the originating Pyrefly transaction is live.
use crate::syntax_records::Spans;
use lctx_model::domain::{assertion::AssertionQualification, attribution::Fidelity, charged::StateCharge, obligation::ObligationKind, protocols::*, resources::ResourceBudget, source::{Occurrence, SyntaxKind}, types::TypeTerm, *};
use pyrefly::{alt::observations as native, binding::binding::IsAsync, state::state::Transaction};
use pyrefly_build::handle::Handle;
use pyrefly_types::types::Type;
use ruff_python_ast_latest::{Expr, ModModule, Stmt, visitor::{Visitor, walk_stmt}};
use ruff_text_size_latest::{Ranged, TextRange};

pub struct Records {
    charge: StateCharge,
    pub exits: Vec<(NativeExitObservation,Fidelity)>,
    pub terminals: Vec<(NativeTerminalObservation,Fidelity)>,
    pub diagnostics: Vec<NativeExitDiagnostic>,
    pub boundaries: Vec<(Option<Id<Occurrence>>,ObligationKind,String)>,
}
impl Records {
    fn hold<T:HeapSize>(&mut self,row:&T)->Result<(),ModelError> {self.charge.grow(size_of::<T>().saturating_mul(4).saturating_add(row.heap_bytes()))}
    fn boundary(&mut self,subject:Option<Id<Occurrence>>,message:&str)->Result<(),ModelError> {
        let row=(subject,ObligationKind::NativeUnavailable,message.to_owned());self.charge.grow(256+message.len())?;self.boundaries.push(row);Ok(())
    }
}
enum Query {Exit {range:TextRange,asynchronous:bool,kind:SyntaxKind},Terminal {range:TextRange}}
struct Queries<'a> {charge:StateCharge,rows:Vec<Query>,error:Option<ModelError>,spans:&'a Spans}
impl<'a> Visitor<'a> for Queries<'a> {
    fn visit_stmt(&mut self,stmt:&'a Stmt) {
        if self.error.is_some() {return;}
        let mut add=|query| {if let Err(error)=self.charge.grow(128) {self.error=Some(error);}else{self.rows.push(query);}};
        match stmt {
            Stmt::With(s)=>{for item in &s.items {add(Query::Exit {range:item.context_expr.range(),asynchronous:s.is_async,kind:crate::typed_syntax::kind(ruff_python_ast_latest::AnyNodeRef::from(&item.context_expr).kind())});}},
            Stmt::Expr(s) if matches!(&*s.value,Expr::Call(_)) => add(Query::Terminal {range:s.value.range()}),
            _=>{},
        }
        walk_stmt(self,stmt);
    }
}
fn native_range(range:TextRange)->ruff_text_size::TextRange {ruff_text_size::TextRange::new(ruff_text_size::TextSize::new(range.start().to_u32()),ruff_text_size::TextSize::new(range.end().to_u32()))}
fn status(value:native::NativeCallStatus)->NativeCallStatus {match value {native::NativeCallStatus::NoHardDiagnostics=>NativeCallStatus::NoHardDiagnostics,native::NativeCallStatus::HardDiagnostics=>NativeCallStatus::HardDiagnostics}}
fn awaiting(value:native::ContextExitAwaitStatus)->ExitAwaitability {match value {native::ContextExitAwaitStatus::Synchronous=>ExitAwaitability::Synchronous,native::ContextExitAwaitStatus::Awaitable=>ExitAwaitability::Awaitable,native::ContextExitAwaitStatus::NotAwaitable=>ExitAwaitability::NotAwaitable}}
fn decision(value:native::TerminalCallDecision)->TerminalDecision {match value {
    native::TerminalCallDecision::DeclaredOrInferredDivergence=>TerminalDecision::DeclaredOrInferredDivergence,
    native::TerminalCallDecision::NarrowedNeverCallee=>TerminalDecision::NarrowedNeverCallee,
    native::TerminalCallDecision::InferredMethodReturn=>TerminalDecision::InferredMethodReturn,
    native::TerminalCallDecision::NotImplementedBody=>TerminalDecision::NotImplementedBody,
    native::TerminalCallDecision::NonDivergentCallable=>TerminalDecision::NonDivergentCallable,
    native::TerminalCallDecision::NoNativeCallableSignature=>TerminalDecision::NoNativeCallableSignature,
}}
/// The sole type encoder supplies fidelity together with identity, so unknown native terms cannot
/// become structural protocol evidence through an Id-only seam.
#[allow(clippy::too_many_arguments,reason="Native observations must use the live transaction and sole type encoder")]
pub fn records(transaction:&Transaction<'_>,handle:&Handle,ast:&ModModule,spans:&Spans,q:&AssertionQualification,budget:&ResourceBudget,mut encode:impl FnMut(&Type)->Result<(Id<TypeTerm>,Fidelity),ModelError>)->Result<Records,ModelError> {
    let mut queries=Queries {charge:StateCharge::new(budget,"native_protocol_queries"),rows:vec![],error:None,spans};
    for stmt in &ast.body {queries.visit_stmt(stmt);}
    if let Some(error)=queries.error {return Err(error);}
    let mut out=Records {charge:StateCharge::new(budget,"native_protocol_records"),exits:vec![],terminals:vec![],diagnostics:vec![],boundaries:vec![]};
    let source=queries.spans.source().ok_or_else(||ModelError::Invalid("native protocols need canonical source".into()))?;
    for query in queries.rows {
        match query {
            Query::Exit {range,asynchronous,kind}=>{
                let subject=spans.event(range,Some(kind));
                let Some(subject)=subject else {out.boundary(None,"native context expression has no unique canonical attachment")?;continue;};
                let Some(value)=transaction.observe_context_exit(handle,native_range(range),IsAsync::new(asynchronous)) else {out.boundary(Some(subject),"located native context exit unavailable")?;continue;};
                if value.range!=native_range(range) || value.kind!=IsAsync::new(asynchronous) {return Err(ModelError::Invalid("native exit query changed origin".into()));}
                let (receiver,rf)=encode(&value.receiver)?;let(member,mf)=encode(&value.member)?;let(normal_result,nf)=encode(&value.normal.result)?;let(exceptional_result,ef)=encode(&value.exceptional.result)?;
                let fidelity=if [rf,mf,nf,ef].iter().all(|f|*f==Fidelity::NativeStructural) {Fidelity::NativeStructural} else {Fidelity::DisplayOnly};
                let row=NativeExitObservation {qualification:q.id(),subject,asynchronous,receiver,member,member_name:value.member_name.to_string(),normal_result,exceptional_result,normal_status:status(value.normal.status),exceptional_status:status(value.exceptional.status),normal_awaitability:awaiting(value.normal.awaitability),exceptional_awaitability:awaiting(value.exceptional.awaitability)};
                row.validate()?;
                for (phase,diagnostics) in [(ExitDiagnosticPhase::Member,value.member_diagnostics),(ExitDiagnosticPhase::Normal,value.normal.diagnostics),(ExitDiagnosticPhase::Exceptional,value.exceptional.diagnostics),(ExitDiagnosticPhase::Await,value.await_diagnostics)] {
                    for (ordinal,diagnostic) in diagnostics.into_iter().enumerate() {
                        let diagnostic=NativeExitDiagnostic {qualification:q.id(),exit:row.id(),subject,phase,ordinal:ordinal as i64,source,start:i64::from(diagnostic.range.start().to_u32()),end:i64::from(diagnostic.range.end().to_u32()),kind:format!("{:?}",diagnostic.kind),message:diagnostic.message,details:diagnostic.details};
                        out.hold(&diagnostic)?;out.diagnostics.push(diagnostic);
                    }
                }
                out.hold(&row)?;out.exits.push((row,fidelity));
            },
            Query::Terminal {range}=>{
                let Some(subject)=spans.event(range,Some(SyntaxKind::ExprCall)) else {out.boundary(None,"native terminal call has no unique canonical attachment")?;continue;};
                let Some(value)=transaction.observe_terminal_call(handle,native_range(range)) else {out.boundary(Some(subject),"located native terminal decision unavailable")?;continue;};
                if value.range!=native_range(range) {return Err(ModelError::Invalid("native terminal query changed origin".into()));}
                let(callee,cf)=encode(&value.callee)?;
                let returns=value.return_type.as_ref().map(&mut encode).transpose()?;
                let fidelity=if cf==Fidelity::NativeStructural && returns.as_ref().is_none_or(|(_,f)|*f==Fidelity::NativeStructural) {Fidelity::NativeStructural} else {Fidelity::DisplayOnly};
                let row=NativeTerminalObservation {qualification:q.id(),subject,callee,return_type:returns.map(|(id,_)|id),return_is_inferred:value.return_is_inferred,is_bound_method:value.is_bound_method,decision:decision(value.decision)};
                out.hold(&row)?;out.terminals.push((row,fidelity));
            },
        }
    }
    Ok(out)
}
