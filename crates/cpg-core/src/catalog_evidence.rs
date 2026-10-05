//! C1 contextual evidence consumes completed C0 and typed original facts, independently of brief seeds.
use datafusion::execution::context::SessionContext;
use crate::{
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{self, catalog_evidence::*},
    catalog::evidence::{
        self,
        build::{self, EvidenceData},
    },
    normalized::Rows,
    *,
};
use std::sync::Arc;
use lctx_model::domain::stages::ProviderOutcome;
async fn load<R: Record>(
    session: &SessionContext,
    rows: &mut Rows<R>,
    permit: &analysis::sources::CompletedInput<R>,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<(), ModelError> {
    let query = crate::sql::query(&session,&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        admission.visit_if_expected(permit, &batch)?;
        rows.decode(&batch)?;
    }
    Ok(())
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(access.profile(), access.snapshots(), runtime.budget())?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let session = access.session(runtime).await?;
    let mut registered = charged::ChargedSet::default();
    let mut registration =
        charged::StateCharge::new(runtime.budget(), "expected-input-registration");
    let mut data = EvidenceData::new(runtime.budget());
    macro_rules! core {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;registered.insert(&mut registration,<$ty>::NAME)?;load(&session,&mut data.core.$f,&permit,&mut admission).await?;)*};}
    lctx_model::catalog_inputs!(core);
    macro_rules! catalog {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;registered.insert(&mut registration,<$ty>::NAME)?;load(&session,&mut data.catalog.$f,&permit,&mut admission).await?;)*};}
    lctx_model::catalog_outputs!(catalog);
    macro_rules! facts {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;registered.insert(&mut registration,<$ty>::NAME)?;load(&session,&mut data.facts.$f,&permit,&mut admission).await?;)*};}
    lctx_model::catalog_evidence_inputs!(facts);
    macro_rules! lower {($($f:ident:$ty:ty,)*)=>{$(let permit=access.read::<$ty>()?;registered.insert(&mut registration,<$ty>::NAME)?;load(&session,&mut data.runtime.$f,&permit,&mut admission).await?;)*};}
    lctx_model::catalog_runtime_inputs!(lower);
    let mut definitions = Rows::<analysis::AnalysisDefinition>::new(runtime.budget());
    let mut parameters = Rows::<analysis::MethodParameters>::new(runtime.budget());
    macro_rules! meta {
        ($ty:ty,$rows:ident,$admit:expr) => {{
            let permit = access.read::<$ty>()?;
            
            registered.insert(&mut registration, <$ty>::NAME)?;
            load(&session, &mut $rows, &permit, $admit).await?;
        }};
    }
    meta!(analysis::AnalysisDefinition, definitions, &mut admission);
    meta!(analysis::MethodParameters, parameters, &mut admission);
    macro_rules! expected {($($field:ident:$ty:ty,)*)=>{$({if access.contains::<$ty>() && !registered.contains(<$ty>::NAME){let mut rows=Rows::<$ty>::new(runtime.budget());meta!($ty,rows,&mut admission);}})*};}
    lctx_model::expected_domain_inputs!(expected);
    drop(session);
    let budget = runtime.budget().clone();
    let (data, rows) = tokio::task::spawn_blocking(move || {
        let rows = build::build(&data, &budget)?;
        Ok::<_, ModelError>((data, rows))
    })
    .await
    .map_err(ModelError::codec)??;
    let (expected_parameters, definition) = build::definition();
    if definitions.get(definition.id()) != Some(&definition)
        || parameters.get(expected_parameters.id()) != Some(&expected_parameters)
    {
        return Err(ModelError::Invalid(
            "C1 requires its completed authored definition".into(),
        ));
    }
    macro_rules! write {($($f:ident:$ty:ty,)*)=>{$(output.declare::<$ty>()?;for row in rows.$f.iter() {output.push(row.clone()).await?;})*};}
    lctx_model::catalog_evidence_outputs!(write);
    macro_rules! declare {($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare::<analysis::catalog_evidence::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    declare!(evidence::EvidenceInvocation);
    let mut invocations = Rows::new(runtime.budget());
    let expected_parents = evidence::frames::parents(
        &data.facts.runs,
        &data.facts.core_invocations,
        runtime.budget(),
    )?;
    for parent in expected_parents.iter() {
        let parent_sources = evidence::frames::sources(parent, &data.runtime.lower())?;
        let (invocation, parents, receipts, projections) = Invocation::admitted(
            parent.input,
            parent.context,
            definition.id(),
            None,
            parent_sources.iter().map(Record::id),
            &sources,
            [],
            runtime.budget(),
        )?;
        if !projections.is_empty() {
            return Err(ModelError::Invalid(
                "C1 has no projection requirement".into(),
            ));
        }
        for source in parent_sources {
            output.push(source).await?;
        }
        for row in parents {
            output.push(row).await?;
        }
        for receipt in receipts {
            output.push(receipt).await?;
        }
        let admitted = coverage::admit(
            &invocation,
            &definition,
            analysis::AnalysisCapability::CatalogEvidence,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            let (requirement, members) = scope.expectation().records()?;
            output.push(requirement).await?;
            for member in members {
                output.push(member).await?;
            }
            for observed in scope.observations() {
                output.push(observed.source().clone()).await?;
            }
            let (coverage, members) = coverage::assess(
                scope.expectation(),
                scope.observations(),
                analysis::AnalysisStatus::Completed,
                None,
                runtime.budget(),
            )?;
            output.push(coverage).await?;
            for member in members {
                output.push(member).await?;
            }
        }
        output
            .push(AnalysisOutcome {
                invocation: invocation.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            })
            .await?;
        output.push(invocation.clone()).await?;
        invocations.insert(invocation)?;
    }
    let links = build::invocation_links(&rows, &invocations, runtime.budget())?;
    for row in links.iter() {
        output.push(row.clone()).await?;
    }
    drop(links);
    drop(invocations);
    drop(expected_parents);
    drop(rows);
    drop(data);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}
