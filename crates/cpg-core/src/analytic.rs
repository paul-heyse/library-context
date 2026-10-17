//! A1 owns optional nominal results over stored A0 frames and admitted E1 winning bytes.
use crate::producer_operations::{Declaration, declare, declare_ordered, emit};
use crate::{
    analysis_graphs::PreparedGraphs,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use lctx_model::domain::{
    analysis::{self, analytic as owner},
    analytics::{self as semantic, build, frames},
    stages::*,
    *,
};
use std::sync::Arc;
// Decoder reachability is separate from the model-owned consumed source inventory.
macro_rules! decoder_inputs {($apply:ident)=>{
        lctx_model::normalized_binding_inputs!($apply);
        lctx_model::structural_outputs!($apply);
        $apply! {qualifications:assertion::AssertionQualification,conditions:conditions::Condition,nodes:conditions::ConditionNode,sets:assumptions::AssumptionSet,members:assumptions::AssumptionSetMember,}
        lctx_model::analytic_extra_inputs!($apply);
        lctx_model::analytic_consumption_inputs!($apply);
        $apply! {full_values:embedding::value::FullValue,projected_values:embedding::projection::ProjectedValue,}
        lctx_model::projection_outputs!($apply);

};}
type MetadataLoader = for<'a, 'sources> fn(
    &'a CompletedInputs,
    &'a datafusion::prelude::SessionContext,
    &'a mut crate::consumed_rows::ConsumedInputs,
    &'a mut analysis::expected::CoverageAdmission<'sources>,
    &'a mut build::Data,
) -> BoxFuture<'a, Result<(), ModelError>>;

fn read_metadata<'a, R: Record>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'_>,
    data: &'a mut build::Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        while let Some((input, permit)) = consumed.next::<R>(access)? {
            if crate::consumed_rows::stream_artifact_admission(access, &input, session, admission)
                .await?
            {
                continue;
            }
            crate::consumed_rows::stream_at(&permit, &input, access, session, |permit, batch| {
                if [
                    std::any::TypeId::of::<analysis::settings::AnalyticsConfiguration>(),
                    std::any::TypeId::of::<analysis::AnalysisDefinition>(),
                    std::any::TypeId::of::<analysis::MethodParameters>(),
                    std::any::TypeId::of::<structural::StructuralFrame>(),
                    std::any::TypeId::of::<analysis::structural::Invocation>(),
                ]
                .contains(&input.type_id())
                {
                    data.visit_input(&input, batch)?;
                }
                admission.visit_if_expected(permit, batch)?;
                Ok(())
            })
            .await?;
        }
        Ok(())
    })
}
fn load_metadata<'a>(
    access: &'a CompletedInputs,
    session: &'a datafusion::prelude::SessionContext,
    consumed: &'a mut crate::consumed_rows::ConsumedInputs,
    admission: &'a mut analysis::expected::CoverageAdmission<'_>,
    data: &'a mut build::Data,
) -> BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        const METADATA: &[MetadataLoader] = &[
            read_metadata::<analysis::settings::AnalyticsConfiguration>,
            read_metadata::<analysis::AnalysisDefinition>,
            read_metadata::<analysis::MethodParameters>,
            read_metadata::<structural::StructuralFrame>,
            read_metadata::<analysis::structural::Invocation>,
        ];
        macro_rules! inventory {($($field:ident:$ty:ty,)*) => { const EXPECTED: &[MetadataLoader] = &[$(read_metadata::<$ty>,)*]; };}
        lctx_model::expected_domain_inputs!(inventory);
        for load in METADATA.iter().chain(EXPECTED) {
            load(access, session, consumed, admission, data).await?;
        }
        Ok(())
    })
}
fn declare_outputs(output: &ProducerOutput) -> BoxFuture<'_, Result<(), ModelError>> {
    Box::pin(async move {
        let mut declarations: Vec<Declaration> = Vec::new();
        macro_rules! common {($($record:ident,)*) => { $(declarations.push(declare::<owner::$record>);)* };}
        lctx_model::analysis_publication!(common);
        macro_rules! outputs {($($field:ident:$ty:ty,)*) => { $(declarations.push(declare::<$ty>);)* };}
        lctx_model::analytic_outputs!(outputs);
        declarations.extend([
            declare::<assertion::AssertionQualification> as Declaration,
            declare::<conditions::Condition>,
            declare::<conditions::ConditionNode>,
        ]);
        declare_ordered(output, &declarations).await
    })
}
fn load_frame<'a>(
    scopes: &'a crate::analytical_scopes::FrameWindow<'a>,
    access: &'a CompletedInputs,
    grain: usize,
    data: &'a mut build::Data,
    input: Id<input::InputRevision>,
    context: Id<attribution::AnalysisContext>,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Loader = for<'a> fn(
        &'a crate::analytical_scopes::FrameWindow<'a>,
        &'a CompletedInputs,
        usize,
        &'a mut build::Data,
        Id<input::InputRevision>,
        Id<attribution::AnalysisContext>,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    fn read_frame<'a, R: Record>(
        scopes: &'a crate::analytical_scopes::FrameWindow<'a>,
        _access: &'a CompletedInputs,
        grain: usize,
        data: &'a mut build::Data,
        input: Id<input::InputRevision>,
        context: Id<attribution::AnalysisContext>,
    ) -> BoxFuture<'a, Result<(), ModelError>> {
        Box::pin(async move {
            scopes.read::<R>(grain, move |binding, batch| {
                data.visit_frame_input(binding, batch, input, context)
            })
        })
    }
    let mut loaders: Vec<Loader> = Vec::new();
    macro_rules! inventory {($($field:ident:$ty:ty,)*) => { $(loaders.push(read_frame::<$ty>);)* };}
    decoder_inputs!(inventory);
    Box::pin(async move {
        for load in loaders {
            load(scopes, access, grain, data, input, context).await?;
        }
        Ok(())
    })
}
fn emit_results<'a>(
    rows: &'a semantic::Output,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a semantic::Output,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident:$ty:ty,)*) => {
        $(fn $field<'a>(rows: &'a semantic::Output, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[Emission] = &[$($field,)*];
    };}
    lctx_model::analytic_outputs!(adapters);
    Box::pin(async move {
        for emission in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}
fn emit_context<'a>(
    rows: &'a frames::Context,
    output: &'a ProducerOutput,
) -> BoxFuture<'a, Result<(), ModelError>> {
    type Emission = for<'a> fn(
        &'a frames::Context,
        &'a ProducerOutput,
    ) -> BoxFuture<'a, Result<(), ModelError>>;
    macro_rules! adapters {($($field:ident),*) => {
        $(fn $field<'a>(rows: &'a frames::Context, output: &'a ProducerOutput) -> BoxFuture<'a, Result<(), ModelError>> { emit(&rows.$field, output) })*
        const EMISSIONS: &[Emission] = &[$($field,)*];
    };}
    adapters!(invocations, sources, inputs);
    Box::pin(async move {
        for emission in EMISSIONS {
            emission(rows, output).await?;
        }
        Ok(())
    })
}

pub async fn produce(
    access: CompletedInputs,
    output: ProducerOutput,
    runtime: &Arc<Workspace>,
    model: &Arc<ValidatedModel>,
    graphs: Option<&Arc<PreparedGraphs>>,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = runtime.coverage_admission(&access, &sources).await?;
    let mut data = build::Data::new(runtime.budget());
    let session = access.session(runtime).await?;
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        {
            let mut inputs = vec![
                ValidationInput::of::<analysis::settings::AnalyticsConfiguration>(&["id"]),
                ValidationInput::of::<analysis::AnalysisDefinition>(&["id"]),
                ValidationInput::of::<analysis::MethodParameters>(&["id"]),
                ValidationInput::of::<structural::StructuralFrame>(&["id"]),
                ValidationInput::of::<analysis::structural::Invocation>(&["id"]),
            ];
            for method in build::METHODS {
                inputs.extend(analysis::expected::inputs(method));
            }
            inputs
        },
        runtime.budget(),
    )?;
    load_metadata(&access, &session, &mut consumed, &mut admission, &mut data).await?;
    consumed.finish(access.name())?;
    let settings = data.configuration()?.clone();
    declare_outputs(&output).await?;
    let mut roots = normalized::Rows::new(runtime.budget());
    for frame in data.structural.frames.iter() {
        roots.insert(frame.clone())?;
    }
    let requested = build::requested(&settings);
    let scopes = if requested {
        Some(
            crate::analytical_scopes::FrameScopes::prepare(
                &access,
                &session,
                model,
                build::Data::demanded_inputs(access.profile(), &settings),
                crate::analytical_scopes::Kind::Analytic,
                runtime.budget(),
            )
            .await?,
        )
    } else {
        None
    };
    drop(session);
    let metadata = data;
    let cancellation = runtime.cancellation();
    let mut readers = Vec::new();
    macro_rules! window_readers {($($field:ident:$ty:ty,)*)=>{$(readers.push(crate::analytical_scopes::window_reader::<$ty>());)*};}
    decoder_inputs!(window_readers);
    {
        let mut remaining = roots.iter().peekable();
        while remaining.peek().is_some() {
            let _selected = runtime.budget().reserve(
                "analytic-window-roots",
                crate::analytical_scopes::WINDOW_ROOTS
                    * (size_of::<&structural::StructuralFrame>()
                        + size_of::<Id<structural::StructuralFrame>>()),
            )?;
            let selected = remaining
                .by_ref()
                .take(crate::analytical_scopes::WINDOW_ROOTS)
                .collect::<Vec<_>>();
            let ids = selected.iter().map(|frame| frame.id()).collect::<Vec<_>>();
            let window = match &scopes {
                Some(scopes) => Some(
                    scopes
                        .window(&access, &ids, &readers, runtime.budget(), &cancellation)
                        .await?,
                ),
                None => None,
            };
            let job_count = selected.len();
            let mut jobs = Vec::new();
            let mut jobs_charge =
                charged::StateCharge::new(runtime.budget(), "prepared work transport slots");
            for (partition, sf) in selected.into_iter().enumerate() {
                let mut data = build::Data::new(runtime.budget());
                if let Some(window) = &window {
                    // Parent is fixed metadata. Native dependencies can name other provider runs; they
                    // cannot widen the E1 consumer's expected invocation domain into another frame.
                    let mut parent_rows =
                        normalized::Rows::<analysis::structural::Invocation>::new(runtime.budget());
                    window.read::<analysis::structural::Invocation>(partition, |_, batch| {
                        parent_rows.decode(batch)?;
                        Ok(())
                    })?;
                    let frame_parent = parent_rows
                        .get(sf.invocation)
                        .ok_or(ModelError::Schema("analytic selected parent"))?
                        .clone();
                    drop(parent_rows);
                    load_frame(
                        window,
                        &access,
                        partition,
                        &mut data,
                        frame_parent.input,
                        frame_parent.context,
                    )
                    .await?;
                } else {
                    data.structural.frames.insert(sf.clone())?;
                    macro_rules! metadata {($($field:ident),*)=>{$(for row in metadata.$field.iter(){data.$field.insert(row.clone())?;})*};}
                    metadata!(settings, definitions, parameters);
                    data.structural_invocations.insert(
                        metadata
                            .structural_invocations
                            .get(sf.invocation)
                            .ok_or(ModelError::Schema("analytic selected parent"))?
                            .clone(),
                    )?;
                }
                let mut context = frames::Context::new(runtime.budget());
                let mut receipts = normalized::Rows::new(runtime.budget());
                let mut projections = normalized::Rows::new(runtime.budget());
                let mut results = semantic::Output::new(runtime.budget());
                let parent = data
                    .structural_invocations
                    .get(sf.invocation)
                    .ok_or_else(|| {
                        ModelError::Invalid("analytic structural parent absent".into())
                    })?;
                for method in build::METHODS {
                    let (parameters, definition) = build::definition(&settings, method)?;
                    if data.definitions.get(definition.id()) != Some(&definition)
                        || data.parameters.get(parameters.id()) != Some(&parameters)
                    {
                        return Err(ModelError::Invalid(
                            "analytic canonical definition absent".into(),
                        ));
                    }
                    let parents = frames::parents(&data, sf, method)?;
                    let (invocation, inputs, source_rows, projection_rows) =
                        owner::Invocation::admitted(
                            parent.input,
                            parent.context,
                            definition.id(),
                            None,
                            parents.iter().map(Record::id),
                            &sources,
                            [analysis::ProjectionDefinition::builtin(
                                projection::ProjectionName::CallableInvocation,
                            )
                            .id()],
                            runtime.budget(),
                        )?;
                    for p in parents {
                        context.sources.insert(p)?;
                    }
                    for row in inputs {
                        context.inputs.insert(row)?;
                    }
                    for row in source_rows {
                        receipts.insert(row)?;
                    }
                    for row in projection_rows {
                        projections.insert(row)?;
                    }
                    context.invocations.insert(invocation)?;
                }
                let frame = semantic::AnalyticFrame {
                    structural: sf.id(),
                    configuration: settings.id(),
                };
                let frame_key = projection::normalization::ProjectionKey {
                    input: parent.input,
                    context: parent.context,
                    name: projection::ProjectionName::CallableInvocation,
                };
                let graphs = graphs.cloned();
                let runtime = runtime.clone();
                let access = access.clone();
                let job = async move {
                    let produced = if requested {
                        let graph = graphs
                            .as_ref()
                            .ok_or(ModelError::Schema("requested analytic graph owner"))?
                            .graph(&access, &runtime, frame_key)?;
                        let preparation =
                            crate::stage_runtime::borrowed_cpu(access.name(), || {
                                build::prepare(
                                    &data,
                                    &frame,
                                    &context.invocations,
                                    graph,
                                    runtime.budget(),
                                    semantic::policy::RETAINED.attributes,
                                )
                            })?;
                        let preparation =
                            run_techniques(preparation, &runtime, access.name()).await?;
                        crate::stage_runtime::borrowed_cpu(access.name(), || {
                            preparation.finish(&data)
                        })?
                    } else {
                        build::not_requested(
                            &data,
                            &frame,
                            &context.invocations,
                            runtime.budget(),
                            semantic::policy::RETAINED.attributes,
                        )?
                    };
                    results.extend(produced)?;
                    Ok((data, context, results, receipts, projections))
                };
                if jobs.is_empty() {
                    jobs_charge.grow(job_count.saturating_mul(std::mem::size_of_val(&job)))?;
                    jobs.reserve_exact(job_count);
                }
                jobs.push(job);
            }
            for (data, context, results, receipts, projections) in
                crate::owned_work::ordered(runtime, jobs).await?
            {
                emit_results(&results, &output).await?;
                emit(&results.qualifications, &output).await?;
                if !results.qualifications.is_empty() {
                    let (c, nodes) = conditions::Diagram::always().records();
                    output.push(c).await?;
                    for n in nodes {
                        output.push(n).await?;
                    }
                }
                emit_context(&context, &output).await?;
                emit(&receipts, &output).await?;
                emit(&projections, &output).await?;

                for result in results.results.iter() {
                    let invocation = context.invocations.get(result.invocation).unwrap();
                    let definition = data.definitions.get(invocation.definition).unwrap();
                    let outcome = frames::outcome(result);
                    let domain = owner::coverage::admit(
                        invocation,
                        definition,
                        build::capability(result.method)?,
                        &admission,
                        runtime.budget(),
                    )?;
                    for scope in domain.scopes() {
                        let (r, members) = scope.expectation().records()?;
                        output.push(r).await?;
                        for r in members {
                            output.push(r).await?;
                        }
                        for r in scope.observations() {
                            output.push(r.source().clone()).await?;
                        }
                        let (r, members) = owner::coverage::assess(
                            scope.expectation(),
                            scope.observations(),
                            outcome.status,
                            outcome.reason,
                            runtime.budget(),
                        )?;
                        output.push(r).await?;
                        for r in members {
                            output.push(r).await?;
                        }
                    }
                    output.push(outcome).await?;
                }
                drop(results);
                drop(context);
                drop(data);
                drop(receipts);
                drop(projections);
                tokio::task::yield_now().await;
            }
        }
    }
    drop(roots);
    drop(scopes);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}

/// The model owns this finite technique DAG. Private completion immediately unlocks
/// its dependants; unrelated slow ranking/concept work is not a community barrier.
async fn run_techniques(
    preparation: build::TechniquePreparation,
    runtime: &Arc<Workspace>,
    name: &'static str,
) -> Result<build::TechniquePreparation, ModelError> {
    run_techniques_with(
        preparation,
        runtime.budget(),
        runtime.cancellation(),
        |job| {
            runtime
                .spawn_stage(async move { crate::stage_runtime::borrowed_cpu(name, || job.run()) })
        },
    )
    .await
}

type TechniqueCompletion = BoxFuture<'static, Result<build::TechniqueOutput, ModelError>>;

/// The concrete driver is shared by production and gated finite-model controls.
/// A failed coordinator has no further use for preparation, but every accepted
/// task still owns its actual input backing and must be joined before returning.
async fn run_techniques_with(
    preparation: build::TechniquePreparation,
    budget: &resources::ResourceBudget,
    cancellation: crate::workspace::Cancellation,
    mut launch: impl FnMut(build::TechniqueJob) -> Result<TechniqueCompletion, ModelError>,
) -> Result<build::TechniquePreparation, ModelError> {
    const TECHNIQUES: usize = 6;
    type Delivery = BoxFuture<'static, (usize, Result<build::TechniqueOutput, ModelError>)>;
    let _delivery = budget.reserve(
        "analytic private technique delivery",
        TECHNIQUES * (size_of::<Delivery>() + size_of::<Option<build::TechniqueOutput>>() + 128),
    )?;
    let mut pending = FuturesUnordered::<Delivery>::new();
    let mut failures = std::collections::BTreeMap::new();
    let mut preparation = Some(preparation);
    loop {
        if failures.is_empty() {
            match preparation
                .as_mut()
                .expect("live technique preparation")
                .ready()
            {
                Ok(ready) => {
                    for job in ready {
                        let ordinal = job.ordinal();
                        if ordinal >= TECHNIQUES {
                            failures.insert(
                                ordinal,
                                ModelError::Conflict("analytic technique ordinal"),
                            );
                            cancellation.cancel();
                            break;
                        }
                        if let Err(error) = cancellation.check() {
                            failures.insert(ordinal, error);
                            break;
                        }
                        match launch(job) {
                            Ok(task) => pending.push(async move { (ordinal, task.await) }.boxed()),
                            Err(error) => {
                                failures.insert(ordinal, error);
                                cancellation.cancel();
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    failures.insert(TECHNIQUES, error);
                    cancellation.cancel();
                }
            }
        }
        if !failures.is_empty() {
            // Drop integrated results, retained vectors and unlaunched jobs
            // before awaiting any remaining accepted work.
            drop(preparation.take());
        }
        let Some((ordinal, result)) = pending.next().await else {
            break;
        };
        match result {
            Ok(output) if failures.is_empty() => {
                if let Err(error) = preparation
                    .as_mut()
                    .expect("live technique preparation")
                    .integrate(output)
                {
                    failures.insert(ordinal, error);
                    cancellation.cancel();
                }
            }
            Ok(_) => {}
            Err(error) => {
                failures.insert(ordinal, error);
                cancellation.cancel();
            }
        }
    }
    if !failures.is_empty() {
        let primary = failures.iter()
            .find(|(_, error)| !matches!(error, ModelError::Invalid(message) if message == "compilation cancelled"))
            .or_else(|| failures.first_key_value())
            .map(|(ordinal, _)| *ordinal).expect("technique failure");
        let primary = failures
            .remove(&primary)
            .expect("selected technique failure");
        let mut terminal = completion::Completion::default();
        for (_, error) in failures {
            terminal.step("analytic peer technique completion", Err(error));
        }
        return completion::complete(Err(primary), terminal);
    }
    cancellation.check()?;
    Ok(preparation.expect("successful technique preparation"))
}

#[cfg(test)]
mod decoder_tests {
    use super::*;
    #[test]
    fn declared_sources_have_decoder_reachability_in_both_profiles() {
        let mut decoders = std::collections::BTreeSet::new();
        macro_rules! collect {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
        decoder_inputs!(collect);
        for profile in Profile::ALL {
            crate::consumed_rows::assert_decoder_reachability(
                build::Data::consumed_inputs(profile),
                &decoders,
            );
        }
    }
}

#[cfg(test)]
mod technique_lifetime_tests {
    use super::*;
    use resources::{Reservation, ResourceBudget, ResourcePool};
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// Observe actual finite-model reservations by owner; the underlying pool
    /// still enforces every admission and all resize/refusal semantics.
    #[derive(Debug)]
    struct ObservedPool {
        budget: ResourceBudget,
        owners: Arc<Mutex<BTreeMap<&'static str, usize>>>,
    }
    #[derive(Debug)]
    struct ObservedReservation {
        inner: Box<dyn Reservation>,
        owner: &'static str,
        owners: Arc<Mutex<BTreeMap<&'static str, usize>>>,
    }
    impl Reservation for ObservedReservation {
        fn size(&self) -> usize {
            self.inner.size()
        }
        fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
            let previous = self.size();
            self.inner.try_resize(bytes)?;
            let mut owners = self.owners.lock().unwrap();
            let used = owners.entry(self.owner).or_default();
            *used = *used - previous + bytes;
            Ok(())
        }
    }
    impl Drop for ObservedReservation {
        fn drop(&mut self) {
            *self.owners.lock().unwrap().entry(self.owner).or_default() -= self.size();
        }
    }
    impl ResourcePool for ObservedPool {
        fn reserve(
            &self,
            owner: &'static str,
            bytes: usize,
        ) -> Result<Box<dyn Reservation>, ModelError> {
            let inner = self.budget.reserve(owner, bytes)?;
            *self.owners.lock().unwrap().entry(owner).or_default() += bytes;
            Ok(Box::new(ObservedReservation {
                inner,
                owner,
                owners: self.owners.clone(),
            }))
        }
        fn reserved(&self) -> usize {
            self.budget.reserved()
        }
        fn limit(&self) -> usize {
            self.budget.limit()
        }
    }
    impl ObservedPool {
        fn bytes(&self, owner: &'static str) -> usize {
            self.owners
                .lock()
                .unwrap()
                .get(owner)
                .copied()
                .unwrap_or_default()
        }
    }
    fn budget() -> (ResourceBudget, Arc<ObservedPool>) {
        let pool = Arc::new(ObservedPool {
            budget: ResourceBudget::fixed(32 << 20).unwrap(),
            owners: Arc::default(),
        });
        (ResourceBudget::from_pool(pool.clone()).unwrap(), pool)
    }
    fn id<T>(n: u8) -> Id<T> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    struct Fixture {
        data: build::Data,
        frame: semantic::AnalyticFrame,
        invocations: normalized::Rows<owner::Invocation>,
        graph: projection::snapshot::MaterializedGraph,
    }
    impl Fixture {
        fn new(budget: &ResourceBudget) -> Self {
            let mut data = build::Data::new(budget);
            let settings = analysis::settings::AnalyticsConfiguration {
                module_prefixes: vec!["api".into()],
                public_roots: vec!["api".into()],
                configured_seeds: vec![],
                depth: 2,
                vertices: 64,
                arcs: 128,
                witnesses: 1,
                brief_budget: 0,
                communities: true,
                pagerank: true,
                fca: false,
                rca: false,
                knn: false,
                type_layer: true,
                mention_layer: false,
                knn_layer: true,
            };
            let mut text = embedding::text::TextDefinition::builtin();
            text.requested = false;
            data.vectors.text_definitions.insert(text).unwrap();
            data.settings.insert(settings.clone()).unwrap();
            let (parent, _) = analysis::structural::Invocation::new(id(1), id(2), id(3), None, []);
            data.structural_invocations.insert(parent.clone()).unwrap();
            let mut projected = projection::normalization::ProjectionData::new(budget);
            let run = projected
                .runs
                .insert(attribution::ProviderRun {
                    provider: id(4),
                    input: parent.input,
                    context: parent.context,
                    configuration: ContentHash::of(b"control"),
                    requested_families: ContentHash::of(b"empty"),
                })
                .unwrap();
            let scope = source::CoverageScope::Input {
                input: parent.input,
            };
            projected.scopes.insert(scope.clone()).unwrap();
            for family in
                projection::ProjectionSpec::builtin(projection::ProjectionName::CallableInvocation)
                    .families()
            {
                projected
                    .coverage
                    .insert(attribution::ProviderCoverage {
                        scope: scope.id(),
                        provider: Some(id(4)),
                        context: parent.context,
                        family: *family,
                        run: Some(run),
                        status: attribution::CoverageStatus::CompleteUnderStatedModel,
                        reason: None,
                        diagnostic: None,
                    })
                    .unwrap();
            }
            let input = projection::normalization::describe(
                &projected,
                projection::normalization::ProjectionKey {
                    input: parent.input,
                    context: parent.context,
                    name: projection::ProjectionName::CallableInvocation,
                },
                budget,
            )
            .unwrap();
            let assessment = input.assessment();
            let graph = projection::snapshot::MaterializedGraph::build(&input, budget).unwrap();
            data.graphs.assessments.insert(assessment.clone()).unwrap();
            let structural = structural::StructuralFrame {
                invocation: parent.id(),
                usage_invocation: parent.id(),
                handoff_invocation: parent.id(),
                control_invocation: parent.id(),
                controls_requested: false,
                configuration: settings.id(),
                invocation_graph: assessment.id(),
                definition_graph: id(5),
            };
            data.structural.frames.insert(structural.clone()).unwrap();
            let frame = semantic::AnalyticFrame {
                structural: structural.id(),
                configuration: settings.id(),
            };
            let mut invocations = normalized::Rows::new(budget);
            for method in build::METHODS {
                let (parameters, definition) = build::definition(&settings, method).unwrap();
                data.parameters.insert(parameters).unwrap();
                data.definitions.insert(definition.clone()).unwrap();
                invocations
                    .insert(
                        owner::Invocation::new(
                            parent.input,
                            parent.context,
                            definition.id(),
                            None,
                            [],
                        )
                        .0,
                    )
                    .unwrap();
            }
            Self {
                data,
                frame,
                invocations,
                graph,
            }
        }
        fn prepare(&self, budget: &ResourceBudget) -> build::TechniquePreparation {
            build::prepare(
                &self.data,
                &self.frame,
                &self.invocations,
                &self.graph,
                budget,
                semantic::policy::RETAINED.attributes,
            )
            .unwrap()
        }
    }

    async fn refused_peer_retires_preparation_before_other_jobs_finish(foreign_completion: bool) {
        let (budget, observed) = budget();
        let fixture = Fixture::new(&budget);
        let preparation = fixture.prepare(&budget);
        let foreign = foreign_completion.then(|| {
            let separate = ResourceBudget::fixed(32 << 20).unwrap();
            let fixture = Fixture::new(&separate);
            let mut preparation = fixture.prepare(&separate);
            preparation.ready().unwrap().remove(1).run().unwrap()
        });
        let (refuse, refusal) = tokio::sync::oneshot::channel::<()>();
        let mut refusal = Some(refusal);
        let mut foreign = foreign;
        let peers = Arc::new(tokio::sync::Semaphore::new(0));
        let cancellation = crate::workspace::Cancellation::default();
        let mut launched = Vec::new();
        let operation = run_techniques_with(preparation, &budget, cancellation.clone(), |job| {
            let ordinal = job.ordinal();
            launched.push(ordinal);
            Ok(match ordinal {
                0 => async move { job.run() }.boxed(),
                1 => {
                    let refusal = refusal.take().unwrap();
                    let foreign = foreign.take();
                    async move {
                        refusal.await.unwrap();
                        drop(job);
                        foreign.ok_or(ModelError::Conflict("analytic peer refused"))
                    }
                    .boxed()
                }
                2 | 3 => {
                    let peers = peers.clone();
                    async move {
                        let permit = peers.acquire().await.unwrap();
                        permit.forget();
                        job.run()
                    }
                    .boxed()
                }
                _ => panic!("dependent technique launched before vectors complete"),
            })
        });
        let mut operation = Box::pin(operation);
        assert!(futures::poll!(operation.as_mut()).is_pending());
        // Rank has integrated; nearest vectors are still live, and the type
        // layer sits in the dependent community job that cannot launch yet.
        assert!(observed.bytes(semantic::TechniqueResult::NAME) > 0);
        assert!(observed.bytes(semantic::LayerResult::NAME) > 0);
        assert!(observed.bytes(semantic::AnalyticFrame::NAME) > 0);
        refuse.send(()).unwrap();
        assert!(futures::poll!(operation.as_mut()).is_pending());
        assert!(cancellation.check().is_err());
        assert_eq!(observed.bytes(semantic::TechniqueResult::NAME), 0);
        assert_eq!(observed.bytes(semantic::LayerResult::NAME), 0);
        assert_eq!(observed.bytes(semantic::AnalyticFrame::NAME), 0);
        // Accepted peers still own the real FramePreparation and delivery
        // slots; cancellation did not pretend those jobs were joined.
        assert!(observed.bytes("analytic-technique-owners") > 0);
        assert!(observed.bytes("analytic private technique delivery") > 0);
        peers.add_permits(2);
        let error = match operation.as_mut().await {
            Ok(_) => panic!("refused technique unexpectedly succeeded"),
            Err(error) => error,
        };
        if foreign_completion {
            assert!(format!("{error:?}").contains("another preparation"));
        } else {
            assert!(format!("{error:?}").contains("analytic peer refused"));
        }
        drop(operation);
        assert_eq!(observed.bytes("analytic-technique-owners"), 0);
        assert_eq!(observed.bytes("analytic private technique delivery"), 0);
        assert_eq!(launched, vec![0, 1, 2, 3]);
        drop(fixture);
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn worker_refusal_retires_integrated_and_unlaunched_technique_state_before_drain() {
        refused_peer_retires_preparation_before_other_jobs_finish(false).await;
    }
    #[tokio::test]
    async fn integration_refusal_retires_integrated_and_unlaunched_technique_state_before_drain() {
        refused_peer_retires_preparation_before_other_jobs_finish(true).await;
    }

    #[tokio::test]
    async fn launch_refusal_and_cancellation_release_preparation_but_join_accepted_work() {
        for cancel_at_launch in [false, true] {
            let (budget, observed) = budget();
            let fixture = Fixture::new(&budget);
            let cancellation = crate::workspace::Cancellation::default();
            let peers = Arc::new(tokio::sync::Semaphore::new(0));
            let mut launched = Vec::new();
            let operation = run_techniques_with(
                fixture.prepare(&budget),
                &budget,
                cancellation.clone(),
                |job| {
                    let ordinal = job.ordinal();
                    launched.push(ordinal);
                    if ordinal == 1 {
                        return Err(ModelError::Conflict("analytic launch refused"));
                    }
                    assert_eq!(ordinal, 0);
                    if cancel_at_launch {
                        cancellation.cancel();
                    }
                    let peers = peers.clone();
                    Ok(async move {
                        let permit = peers.acquire().await.unwrap();
                        permit.forget();
                        job.run()
                    }
                    .boxed())
                },
            );
            let mut operation = Box::pin(operation);
            assert!(futures::poll!(operation.as_mut()).is_pending());
            assert_eq!(observed.bytes(semantic::AnalyticFrame::NAME), 0);
            assert_eq!(observed.bytes(semantic::LayerResult::NAME), 0);
            assert!(observed.bytes("analytic-technique-owners") > 0);
            peers.add_permits(1);
            let error = match operation.as_mut().await {
                Ok(_) => panic!("failed launch unexpectedly succeeded"),
                Err(error) => error,
            };
            assert!(format!("{error:?}").contains(if cancel_at_launch {
                "compilation cancelled"
            } else {
                "analytic launch refused"
            }));
            drop(operation);
            assert_eq!(
                launched,
                if cancel_at_launch {
                    vec![0]
                } else {
                    vec![0, 1]
                }
            );
            assert_eq!(observed.bytes("analytic-technique-owners"), 0);
            drop(fixture);
            assert_eq!(budget.reserved(), 0);
        }
    }

    #[tokio::test]
    async fn successful_technique_driver_matches_complete_finite_output() {
        let (budget, _) = budget();
        let fixture = Fixture::new(&budget);
        let expected = build::produce(
            &fixture.data,
            &fixture.frame,
            &fixture.invocations,
            &fixture.graph,
            &budget,
        )
        .unwrap();
        let preparation = run_techniques_with(
            fixture.prepare(&budget),
            &budget,
            crate::workspace::Cancellation::default(),
            |job| Ok(async move { job.run() }.boxed()),
        )
        .await
        .unwrap();
        let actual = preparation.finish(&fixture.data).unwrap();
        actual.matches(&expected).unwrap();
        drop((actual, expected, fixture));
        assert_eq!(budget.reserved(), 0);
    }
}
