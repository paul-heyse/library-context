//! Read-only presentation of the nominal analysis owners. These reports are not stored authority.
use crate::generation_read::{GenerationSession, InspectionSession};
use futures::TryStreamExt;
use lctx_model::domain::{analysis, charged::StateCharge, normalized::Rows, *};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
pub struct InvocationReport {
    pub owner: &'static str,
    pub invocation: String,
    pub input: String,
    pub context: String,
    pub subject: Option<String>,
    pub method: String,
    pub interpretation: String,
    pub status: String,
    pub reason: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct CapabilityReport {
    pub owner: &'static str,
    pub invocation: String,
    pub capability: String,
    pub scope: String,
    pub context: String,
    pub availability: String,
    pub reason: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct DiagnosticReport {
    pub owner: &'static str,
    pub invocation: String,
    pub elapsed_micros: Option<i64>,
    pub iterations: Option<i64>,
    pub examined_members: Option<i64>,
    pub residual: Option<FiniteF64>,
    pub converged: Option<bool>,
}
/// The reservation remains with the presented rows, including after the read lease closes.
#[derive(Debug, Serialize)]
pub struct AnalysisReport {
    pub outcomes: Vec<InvocationReport>,
    pub capabilities: Vec<CapabilityReport>,
    pub diagnostics: Vec<DiagnosticReport>,
    #[serde(skip)]
    charge: StateCharge,
}
impl AnalysisReport {
    fn new(budget: &resources::ResourceBudget) -> Self {
        Self { outcomes: vec![], capabilities: vec![], diagnostics: vec![], charge: StateCharge::new(budget,"analysis-report") }
    }
    // Fixed-shape rows contain only bounded model names, enum labels and 128-bit IDs. This
    // allowance covers their strings and geometric Vec capacity before each row is constructed.
    fn admit(&mut self) -> Result<(),ModelError> { self.charge.grow(2048) }
}
async fn rows<R: Record>(session:&InspectionSession)->Result<Rows<R>,ModelError> {
    let mut rows=Rows::new(session.budget());
    let query=session.query(&format!("SELECT * FROM \"{}\" ORDER BY id",R::NAME)).await.map_err(ModelError::codec)?;
    let mut stream=query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)? {rows.decode(&batch)?;}
    Ok(rows)
}
/// Opens no independent store route: callers supply the ordinary leased, frontier-checked reader.
/// The same presentation is used by compile and generation show.
pub async fn read(session:GenerationSession,model:&ValidatedModel)->Result<AnalysisReport,ModelError> {
    let held=session.frontier().descriptor().relations(model)?;
    let inspection=InspectionSession::new(session).map_err(ModelError::codec)?;
    let result=collect(&inspection,&held).await;
    let closed=inspection.close().await.map_err(ModelError::codec);
    match (result,closed) { (Ok(report),Ok(()))=>Ok(report),(Err(error),_)=>Err(error),(_,Err(error))=>Err(error) }
}
async fn collect(session:&InspectionSession,held:&BTreeSet<&str>)->Result<AnalysisReport,ModelError> {
    let mut report=AnalysisReport::new(session.budget());
    if !held.contains(analysis::AnalysisDefinition::NAME) {return Ok(report);}
    let definitions=rows::<analysis::AnalysisDefinition>(session).await?;
    macro_rules! owner {($owner:ident)=>{{
        use analysis::$owner as owner;
        if held.contains(owner::AnalysisInvocation::NAME) {
            let invocations=rows::<owner::AnalysisInvocation>(session).await?;
            let outcomes=rows::<owner::AnalysisOutcome>(session).await?;
            for outcome in outcomes.iter() {
                let invocation=invocations.get(outcome.invocation).ok_or_else(||ModelError::Invalid("report outcome lacks its nominal invocation".into()))?;
                let definition=definitions.get(invocation.definition).ok_or_else(||ModelError::Invalid("report invocation lacks its definition".into()))?;
                report.admit()?;
                report.outcomes.push(InvocationReport {owner:stringify!($owner),invocation:invocation.id().hex(),input:invocation.input.hex(),context:invocation.context.hex(),subject:invocation.subject.map(|id|id.hex()),method:format!("{:?}",definition.method),interpretation:format!("{:?}",definition.interpretation),status:format!("{:?}",outcome.status),reason:outcome.reason.map(|r|format!("{r:?}"))});
            }
            for row in rows::<owner::AnalysisCoverage>(session).await?.iter() {
                report.admit()?;
                report.capabilities.push(CapabilityReport {owner:stringify!($owner),invocation:row.invocation.hex(),capability:format!("{:?}",row.capability),scope:row.scope.hex(),context:row.context.hex(),availability:format!("{:?}",row.availability),reason:row.reason.map(|r|format!("{r:?}"))});
            }
            for row in rows::<owner::AnalysisDiagnostic>(session).await?.iter() {
                report.admit()?;
                report.diagnostics.push(DiagnosticReport {owner:stringify!($owner),invocation:row.invocation.hex(),elapsed_micros:row.elapsed_micros,iterations:row.iterations,examined_members:row.examined_members,residual:row.residual,converged:row.converged});
            }
        }
    }};}
    owner!(local);owner!(base_evaluation);owner!(base_completion);owner!(source_call);owner!(enriched_execution);owner!(model);owner!(summary);owner!(structural);owner!(analytic_embedding);owner!(analytic);owner!(catalog_core);owner!(catalog_evidence);owner!(selection);owner!(synthesis);owner!(retrieval);
    Ok(report)
}
