//! C1 contextual evidence consumes completed C0 and typed original facts, independently of brief seeds.
use crate::workspace::{CompletedInputs, ProducerOutput, Workspace};
use datafusion::execution::context::SessionContext;
use futures::{TryStreamExt, future::BoxFuture};
use lctx_model::domain::stages::ProviderOutcome;
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
#[cfg(test)]
use std::any::TypeId;

type InventoryLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut InventoryData,
) -> BoxFuture<'a, Result<(), ModelError>>;
type ScopedLoader = for<'a> fn(
    &'a CompletedInputs,
    &'a [ValidationInput],
    &'a crate::consumed_rows::PreparedClosure,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut EvidenceData,
) -> BoxFuture<'a, Result<(), ModelError>>;
struct InputLoader {
    #[cfg(test)]
    record: fn() -> TypeId,
    inventory: InventoryLoader,
    scoped: ScopedLoader,
}
// Each group is emitted by its existing semantic inventory. Overlapping records remain
// ordered entries; ConsumedInputs alone owns whether a declaration still needs dispatch.
macro_rules! define_loaders {
    ($($field:ident:$ty:ty,)*) => {
        pub(super) static LOADERS: &[InputLoader] = &[$(InputLoader {
            #[cfg(test)]
            record: TypeId::of::<$ty>,
            inventory: load_inventory::<$ty>,
            scoped: load_scoped::<$ty>,
        },)*];
    };
}
macro_rules! loader_group {
    ($module:ident, $inventory:ident) => {
        mod $module {
            use super::*;
            lctx_model::$inventory!(define_loaders);
        }
    };
}
loader_group!(catalog_inputs, catalog_inputs);
loader_group!(catalog_outputs, catalog_outputs);
loader_group!(evidence_inputs, catalog_evidence_inputs);
loader_group!(runtime_inputs, catalog_runtime_inputs);
mod authored_inputs {
    use super::*;
    define_loaders! {definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,}
}
loader_group!(expected_inputs, expected_domain_inputs);
const LOADER_GROUPS: [&[InputLoader]; 6] = [
    catalog_inputs::LOADERS,
    catalog_outputs::LOADERS,
    evidence_inputs::LOADERS,
    runtime_inputs::LOADERS,
    authored_inputs::LOADERS,
    expected_inputs::LOADERS,
];

struct InventoryData {
    frames: EvidenceData,
    definitions: Rows<analysis::AnalysisDefinition>,
    parameters: Rows<analysis::MethodParameters>,
}
impl InventoryData {
    fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            frames: EvidenceData::new(budget),
            definitions: Rows::new(budget),
            parameters: Rows::new(budget),
        }
    }
    fn visit(&mut self, input: &ValidationInput, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if is_frame(input.name()) { self.frames.visit_input(input, batch)?; }
        if input.name() == analysis::AnalysisDefinition::NAME { self.definitions.decode(batch)?; }
        if input.name() == analysis::MethodParameters::NAME { self.parameters.decode(batch)?; }
        Ok(())
    }
}
fn consumed_inputs(_profile: stages::Profile) -> Vec<ValidationInput> {
    let mut declarations = analysis::expected::inputs(build::definition().1.method);
    declarations.extend([
        ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ValidationInput::of::<analysis::catalog_core::Invocation>(&["id"]),
        ValidationInput::of::<analysis::local::Invocation>(&["id"]),
        ValidationInput::of::<analysis::local::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<analysis::source_call::Invocation>(&["id"]),
        ValidationInput::of::<analysis::source_call::AnalysisOutcome>(&["id"]),
        ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
        ValidationInput::of::<analysis::MethodParameters>(&["id"]),
    ]);
    declarations
}
fn is_frame(name: &str) -> bool {
    [
        attribution::ProviderRun::NAME,
        analysis::catalog_core::Invocation::NAME,
        analysis::local::Invocation::NAME,
        analysis::local::AnalysisOutcome::NAME,
        analysis::source_call::Invocation::NAME,
        analysis::source_call::AnalysisOutcome::NAME,
    ]
    .contains(&name)
}
fn load_inventory<'a, 'sources, R: Record>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    inventory: &'a mut InventoryData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission).await? {
                continue;
            }
            crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
                admission.visit_if_expected(permit, batch)?;
                inventory.visit(&input, batch)
            }).await?;
        }
        Ok(())
    })
}
fn load_scoped<'a, R: Record>(
    access: &'a CompletedInputs,
    inputs: &'a [ValidationInput],
    scope: &'a crate::consumed_rows::PreparedClosure,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    data: &'a mut EvidenceData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            let table = inputs.iter().position(|candidate| candidate.type_id() == input.type_id() && candidate.prefix() == input.prefix())
                .ok_or(ModelError::Conflict("C1 scoped input declaration"))?;
            crate::consumed_rows::stream_query_at(&permit, &input, access, scope.session(), &scope.select(table)?, |_, batch| {
                data.visit_input(&input, batch)?;
                Ok(())
            }).await?;
        }
        Ok(())
    })
}
fn load_inventory_groups<'a, 'sources>(
    groups: &'a [&'a [InputLoader]],
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    inventory: &'a mut InventoryData,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        for group in groups {
            for loader in *group {
                (loader.inventory)(access, session, consumed, admission, inventory).await?;
            }
        }
        Ok(())
    })
}
fn inventory_phase<'a, 'sources>(
    access: &'a CompletedInputs,
    session: &'a SessionContext,
    admission: &'a mut analysis::expected::CoverageAdmission<'sources>,
    budget: &'a resources::ResourceBudget,
) -> BoxFuture<'a, Result<InventoryData, ModelError>> {
    Box::pin(async move {
        let mut inventory = InventoryData::new(budget);
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(consumed_inputs(access.profile()), budget)?;
        load_inventory_groups(&LOADER_GROUPS, access, session, &mut consumed, admission, &mut inventory).await?;
        consumed.finish(access.name())?;
        let (expected_parameters, definition) = build::definition();
        if inventory.definitions.get(definition.id()) != Some(&definition)
            || inventory.parameters.get(expected_parameters.id()) != Some(&expected_parameters)
        {
            return Err(ModelError::Invalid("C1 requires its completed authored definition".into()));
        }
        Ok(inventory)
    })
}
fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! declare_outputs {($($f:ident:$ty:ty,)*)=>{$(output.declare_async::<$ty>().await?;)*};}
        lctx_model::catalog_evidence_outputs!(declare_outputs);
        macro_rules! common_publication {($($record:ident,)*)=>{$(output.declare_async::<analysis::catalog_evidence::$record>().await?;)*};}
        lctx_model::analysis_publication!(common_publication);
        output.declare_async::<evidence::EvidenceInvocation>().await?;
        Ok(())
    })
}
fn publish_frames<'a, 'sources>(
    inventory: InventoryData,
    output: &'a ProducerOutput,
    sources: &'a analysis::sources::CapturedSources,
    admission: &'a analysis::expected::CoverageAdmission<'sources>,
    budget: &'a resources::ResourceBudget,
) -> BoxFuture<'a, Result<Rows<Invocation>, ModelError>> {
    Box::pin(async move {
        declare_outputs(output).await?;
        let (_, definition) = build::definition();
    let mut invocations = Rows::new(budget);
    let expected_parents = evidence::frames::parents(
        &inventory.frames.facts.runs,
        &inventory.frames.facts.core_invocations,
        budget,
    )?;
    for parent in expected_parents.iter() {
        let parent_sources = evidence::frames::sources(parent, &inventory.frames.runtime.lower())?;
        let (invocation, parents, receipts, projections) = Invocation::admitted(
            parent.input,
            parent.context,
            definition.id(),
            None,
            parent_sources.iter().map(Record::id),
            sources,
            [],
            budget,
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
            admission,
            budget,
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
                budget,
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
    // The native frame domain remains total, including inputs without any evidence roots.
    // Rich premises and outputs have only one original artifact's lifetime.
    drop(expected_parents);
    drop(inventory.frames);
    drop(inventory.definitions);
    drop(inventory.parameters);
        Ok(invocations)
    })
}
fn load_artifact<'a>(
    access: &'a CompletedInputs,
    scopes: &'a crate::catalog_evidence_scope::EvidenceScopes,
    predicate: String,
    budget: &'a resources::ResourceBudget,
) -> BoxFuture<'a, Result<EvidenceData, ModelError>> {
    Box::pin(async move {
        let scope = scopes.edges.grain(scopes.root, &predicate, budget).await?;
        let mut data = EvidenceData::new(budget);
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(scopes.inputs.clone(), budget)?;
        for group in LOADER_GROUPS {
            for loader in group {
                (loader.scoped)(access, &scopes.inputs, &scope, &mut consumed, &mut data).await?;
            }
        }
        consumed.finish(access.name())?;
        // The temporary key tables and compact scope charge end before rich computation.
        drop(scope);
        Ok(data)
    })
}
fn build_artifact(
    data: EvidenceData,
    budget: resources::ResourceBudget,
) -> BoxFuture<'static, Result<build::EvidenceOutput, ModelError>> {
    Box::pin(async move {
        // Move the whole charged premise owner into the worker, never an uncharged clone.
        tokio::task::spawn_blocking(move || build::build(&data, &budget))
            .await.map_err(ModelError::codec)?
    })
}
fn emit_artifact<'a>(
    rows: build::EvidenceOutput,
    output: &'a ProducerOutput,
    invocations: &'a Rows<Invocation>,
    budget: &'a resources::ResourceBudget,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        macro_rules! write {($($f:ident:$ty:ty,)*)=>{$(for row in rows.$f.iter() {output.push(row.clone()).await?;})*};}
        lctx_model::catalog_evidence_outputs!(write);
        let links = build::invocation_links(&rows, invocations, budget)?;
        for row in links.iter() { output.push(row.clone()).await?; }
        drop(links);
        drop(rows);
        Ok(())
    })
}
fn artifact_phase<'a>(
    access: &'a CompletedInputs,
    output: &'a ProducerOutput,
    scopes: &'a crate::catalog_evidence_scope::EvidenceScopes,
    predicate: String,
    invocations: &'a Rows<Invocation>,
    budget: &'a resources::ResourceBudget,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let data = load_artifact(access, scopes, predicate, budget).await?;
        let rows = build_artifact(data, budget.clone()).await?;
        emit_artifact(rows, output, invocations, budget).await
    })
}
pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Workspace,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(access.profile(), access.snapshots(), runtime.budget())?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let session = access.session(runtime).await?;
    let inventory = inventory_phase(&access, &session, &mut admission, runtime.budget()).await?;
    let invocations = publish_frames(inventory, &output, &sources, &admission, runtime.budget()).await?;
    // Frame premises have been dropped before constructing artifact scopes.
    let scopes = crate::catalog_evidence_scope::EvidenceScopes::prepare(&access, model, &session, runtime.budget()).await?;
    let artifacts = access.table_for(&ValidationInput::of::<source::SourceArtifact>(&["id"]))?;
    let mut roots = crate::sql::query(
        &session,
        &format!(
            "SELECT id FROM {} ORDER BY id",
            crate::consumed_rows::identifier(&artifacts)
        ),
    )
    .await
    .map_err(ModelError::codec)?
    .execute_stream()
    .await
    .map_err(ModelError::codec)?;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        let ids = batch
            .column_by_name("id")
            .and_then(|column| {
                column
                    .as_any()
                    .downcast_ref::<arrow_array::FixedSizeBinaryArray>()
            })
            .ok_or(ModelError::Schema("C1 artifact root identity"))?;
        for index in 0..batch.num_rows() {
            runtime.cancellation().check()?;
            let predicate = format!(
                "id=X'{}'",
                ids.value(index)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            artifact_phase(&access, &output, &scopes, predicate, &invocations, runtime.budget()).await?;
        }
    }
    drop(roots);
    drop(scopes);
    drop(session);
    drop(invocations);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_views_have_profile_decoder_reachability() {
        for profile in [stages::Profile::Catalog, stages::Profile::Behavioral] {
            let decoders = LOADER_GROUPS.iter().flat_map(|group| group.iter()).map(|loader| (loader.record)()).collect();
            crate::consumed_rows::assert_decoder_reachability(consumed_inputs(profile), &decoders);
        }
    }
    #[test]
    fn loader_groups_preserve_inventory_order_and_overlaps() {
        let mut expected = Vec::new();
        macro_rules! records {($($field:ident:$ty:ty,)*)=>{$(expected.push(TypeId::of::<$ty>());)*};}
        lctx_model::catalog_inputs!(records);
        lctx_model::catalog_outputs!(records);
        lctx_model::catalog_evidence_inputs!(records);
        lctx_model::catalog_runtime_inputs!(records);
        records! {definitions:analysis::AnalysisDefinition,parameters:analysis::MethodParameters,}
        lctx_model::expected_domain_inputs!(records);
        let actual: Vec<_> = LOADER_GROUPS.iter().flat_map(|group| group.iter()).map(|loader| (loader.record)()).collect();
        assert_eq!(actual, expected);
        assert!(actual.iter().filter(|record| **record == TypeId::of::<source::SourceArtifact>()).count() > 1);
    }
    fn refuse_inventory<'a, 'sources>(
        _: &'a CompletedInputs,
        _: &'a SessionContext,
        _: &'a mut crate::consumed_rows::ConsumedInputs,
        _: &'a mut analysis::expected::CoverageAdmission<'sources>,
        _: &'a mut InventoryData,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        Box::pin(async { Err(ModelError::Conflict("C1 loader stop control")) })
    }
    fn unexpected_inventory<'a, 'sources>(
        _: &'a CompletedInputs,
        _: &'a SessionContext,
        _: &'a mut crate::consumed_rows::ConsumedInputs,
        _: &'a mut analysis::expected::CoverageAdmission<'sources>,
        _: &'a mut InventoryData,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        panic!("a later loader ran after the first error")
    }
    #[tokio::test]
    async fn inventory_loop_stops_before_the_next_loader_on_error() {
        use lctx_model::domain::input::Package;
        let workspace = Workspace::new(Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap()), crate::workspace::WorkspaceOptions::default(), crate::test_native::store()).unwrap();
        let access = workspace.inputs("C1-loader-stop", stages::Profile::Catalog, []).unwrap();
        let session = SessionContext::new();
        let sources = analysis::sources::CapturedSources::capture(access.profile(), access.snapshots(), workspace.budget()).unwrap();
        let mut admission = analysis::expected::CoverageAdmission::new(&sources, workspace.budget()).unwrap();
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(vec![], workspace.budget()).unwrap();
        let mut inventory = InventoryData::new(workspace.budget());
        let first = [InputLoader { record: TypeId::of::<Package>, inventory: refuse_inventory, scoped: load_scoped::<Package> }];
        let later = [InputLoader { record: TypeId::of::<Package>, inventory: unexpected_inventory, scoped: load_scoped::<Package> }];
        assert!(matches!(load_inventory_groups(&[&first, &later], &access, &session, &mut consumed, &mut admission, &mut inventory).await, Err(ModelError::Conflict("C1 loader stop control"))));
        workspace.drain().await.unwrap();
    }
    fn place(name: &str) -> value::Place {
        let input = input::InputRevision { manifest: ContentHash::of(b"C1-loader-prefix") };
        let source = source::SourceArtifact::from_bytes(input.id(), "loader.py".into(), b"x=1\n").unwrap();
        let module = source::Module { source: source.id(), qualified_name: "loader".into() };
        value::Place { root: value::PlaceRoot::Global { module: module.id(), name: name.into() }.id(), path: value::AccessPath::empty().id() }
    }
    async fn contribute_place(workspace: &Arc<Workspace>, producer: &'static str, name: &str) {
        let output = workspace.output(producer, stages::Profile::Catalog, ContentHash::of(producer.as_bytes()), workspace.inputs(producer, stages::Profile::Catalog, []).unwrap(), [value::Place::NAME]);
        output.declare_async::<value::Place>().await.unwrap();
        output.push(place(name)).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
    }
    #[tokio::test]
    async fn inventory_adapter_drains_exact_prefixes_and_overlapping_entries_once() {
        use stages::{PublicationBoundary, RelationUse};
        let workspace = Workspace::new(Arc::new(model().unwrap()), crate::workspace::WorkspaceOptions::default(), crate::test_native::store()).unwrap();
        contribute_place(&workspace, "C1-loader-facts", "facts").await;
        workspace.freeze_inputs_async(PublicationBoundary::Facts).await.unwrap();
        contribute_place(&workspace, "C1-loader-model", "model").await;
        workspace.freeze_inputs_async(PublicationBoundary::Model).await.unwrap();
        let declaration = stages::Stage {
            captured_binding: None,
            name: "C1-loader-prefixes",
            inputs: vec![RelationUse::completed::<value::Place>().at_epoch(PublicationBoundary::Facts), RelationUse::completed::<value::Place>().at_epoch(PublicationBoundary::Model)],
            outputs: vec![], contributes: vec![], coverage: vec![], profiles: stages::Profile::ALL.to_vec(), effect: stages::Effect::Pure,
            code: ContentHash::of(b"C1-loader-prefixes"), configuration: ContentHash::of(b"C1-loader-prefixes-configuration"),
        };
        let access = workspace.stage_inputs(&declaration, stages::Profile::Catalog).unwrap();
        assert_ne!(access.read_at::<value::Place>(Some(PublicationBoundary::Facts)).unwrap().source().view(), access.read_at::<value::Place>(Some(PublicationBoundary::Model)).unwrap().source().view());
        let session = access.session(&workspace).await.unwrap();
        let sources = analysis::sources::CapturedSources::capture(access.profile(), access.snapshots(), workspace.budget()).unwrap();
        let mut admission = analysis::expected::CoverageAdmission::new(&sources, workspace.budget()).unwrap();
        let declarations = [PublicationBoundary::Facts, PublicationBoundary::Model].map(|prefix| ValidationInput::of::<value::Place>(&["id"]).at_epoch(prefix));
        let mut consumed = crate::consumed_rows::ConsumedInputs::new(declarations.to_vec(), workspace.budget()).unwrap();
        let mut inventory = InventoryData::new(workspace.budget());
        let duplicate = [InputLoader { record: TypeId::of::<value::Place>, inventory: load_inventory::<value::Place>, scoped: load_scoped::<value::Place> }];
        load_inventory_groups(&[&duplicate, &duplicate], &access, &session, &mut consumed, &mut admission, &mut inventory).await.unwrap();
        consumed.finish(access.name()).unwrap();
        assert_eq!(access.read_at::<value::Place>(Some(PublicationBoundary::Facts)).unwrap().source().rows(), 1);
        assert_eq!(access.read_at::<value::Place>(Some(PublicationBoundary::Model)).unwrap().source().rows(), 2);
        workspace.drain().await.unwrap();
    }
    #[test]
    fn unpolled_artifact_build_keeps_and_releases_the_complete_charged_owner() {
        let budget = resources::ResourceBudget::fixed(1 << 20).unwrap();
        let mut data = EvidenceData::new(&budget);
        let input = input::InputRevision { manifest: ContentHash::of(b"C1-build-owner") };
        data.core.artifacts.insert(source::SourceArtifact::from_bytes(input.id(), "owner.py".into(), b"x=1\n").unwrap()).unwrap();
        let before = budget.reserved();
        assert!(before > 0);
        let pending = build_artifact(data, budget.clone());
        assert_eq!(budget.reserved(), before);
        drop(pending);
        assert_eq!(budget.reserved(), 0);
    }
}
