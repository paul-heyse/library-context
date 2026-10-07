//! Private spillable compilation workspace. Completed inputs are immutable IPC streams.
//!
//! Producers write bounded batches. A completed relation is globally ordered and deduplicated
//! before it becomes visible; interrupted writes never enter the completed-input map.
use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch};
use datafusion::{
    arrow::ipc::{reader::FileReader, writer::FileWriter},
    execution::options::ArrowReadOptions,
    execution::{
        disk_manager::{DiskManagerBuilder, DiskManagerMode},
        memory_pool::{MemoryConsumer, MemoryPool, MemoryReservation},
        runtime_env::RuntimeEnvBuilder,
    },
    prelude::{SessionConfig, SessionContext},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    Batch, ContentHash, ModelError, Record, Relation, ValidatedModel,
    batching::TransferLimits,
    charged::StateCharge,
    resources::{Reservation, ResourceBudget, ResourcePool},
    stages::{Profile, ProviderOutcome},
};
use std::{
    any::Any,
    collections::BTreeMap,
    fs::File,
    io::Seek,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceOptions {
    pub memory_bytes: usize,
    pub partitions: usize,
    pub batch_rows: usize,
}
impl Default for WorkspaceOptions {
    fn default() -> Self {
        Self {
            memory_bytes: 256 << 20,
            partitions: 4,
            batch_rows: 4096,
        }
    }
}
/// Cancellation is cooperative between bounded I/O/compute batches. Owned tasks must be drained
/// by their caller before releasing the workspace.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn check(&self) -> Result<(), ModelError> {
        if self.0.load(Ordering::Acquire) {
            Err(ModelError::Invalid("compilation cancelled".into()))
        } else {
            Ok(())
        }
    }
}
struct WorkspaceFiles {
    directory: tempfile::TempDir,
}
#[derive(PartialEq, Eq)]
struct CompilationCompletion {
    frontier: lctx_model::domain::admission::Frontier,
    profile: Profile,
    configuration: ContentHash,
    captures: ContentHash,
    content: ContentHash,
}
fn capture_identity(
    captured: &cpg_extract::bundle::CapturedInputs,
    budget: &ResourceBudget,
) -> Result<ContentHash, ModelError> {
    use lctx_model::domain::Key;
    let _keys = budget.reserve(
        "compiler-capture-keys",
        captured.inputs().len().saturating_mul(64),
    )?;
    let mut ids = captured
        .inputs()
        .iter()
        .map(|input| input.captured().revision().id())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ModelError::Invalid("duplicate compiler capture".into()));
    }
    let mut sink = lctx_model::domain::KeySink::new("compiler-captures/v1");
    for id in ids {
        id.encode(&mut sink);
    }
    Ok(sink.finish())
}
/// The attempt owns one runtime, spill directory, buffer budget, and completed relation registry.
pub struct Workspace {
    files: Arc<WorkspaceFiles>,
    context: SessionContext,
    options: WorkspaceOptions,
    budget: ResourceBudget,
    model: Arc<ValidatedModel>,
    completed: Mutex<BTreeMap<&'static str, Arc<CompletedRelation>>>,
    frozen_shared: Mutex<
        BTreeMap<
            (
                lctx_model::domain::stages::PublicationBoundary,
                &'static str,
            ),
            Arc<CompletedRelation>,
        >,
    >,
    next: AtomicU64,
    cancellation: Cancellation,
    completion_gate: tokio::sync::Mutex<()>,
    compilation_complete: Mutex<Option<CompilationCompletion>>,
}
impl Workspace {
    pub fn new(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
    ) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes == 0 || options.partitions == 0 || options.batch_rows == 0 {
            return Err(ModelError::Invalid(
                "workspace sizes must be positive".into(),
            ));
        }
        let directory = tempfile::tempdir().map_err(ModelError::codec)?;
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_limit(options.memory_bytes, 1.0)
            .with_disk_manager_builder(DiskManagerBuilder::default().with_mode(
                DiskManagerMode::Directories(vec![directory.path().to_path_buf()]),
            ))
            .build_arc()
            .map_err(ModelError::codec)?;
        let config = SessionConfig::new()
            .with_target_partitions(options.partitions)
            .with_batch_size(options.batch_rows)
            .set_usize(
                "datafusion.execution.sort_spill_reservation_bytes",
                (options.memory_bytes / 16).min(10 << 20),
            )
            .set_usize(
                "datafusion.execution.sort_in_place_threshold_bytes",
                (options.memory_bytes / 8).min(1 << 20),
            );
        Ok(Arc::new(Self {
            files: Arc::new(WorkspaceFiles { directory }),
            context: SessionContext::new_with_config_rt(config, runtime.clone()),
            options,
            budget: ResourceBudget::from_pool(Arc::new(WorkspacePool {
                memory: runtime.memory_pool.clone(),
                limit: options.memory_bytes,
            }))?,
            model,
            completed: Mutex::default(),
            frozen_shared: Mutex::default(),
            next: AtomicU64::new(0),
            cancellation: Cancellation::default(),
            completion_gate: tokio::sync::Mutex::new(()),
            compilation_complete: Mutex::default(),
        }))
    }
    /// Reuse a captured native configuration's pool. DataFusion allocations and typed retained
    /// state draw from this same ceiling; the custom pool adapts allocations rather than copying.
    pub fn with_budget(
        model: Arc<ValidatedModel>,
        options: WorkspaceOptions,
        budget: ResourceBudget,
    ) -> Result<Arc<Self>, ModelError> {
        if options.memory_bytes != budget.limit() {
            return Err(ModelError::Invalid(
                "workspace memory differs from supplied pool".into(),
            ));
        }
        let mut workspace = Self::new(model, options)?;
        let owner = Arc::get_mut(&mut workspace).expect("new workspace has one owner");
        let pool = Arc::new(BudgetMemoryPool {
            budget: budget.clone(),
            allocated: Mutex::new(budget.reserve("workspace-datafusion", 0)?),
        });
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_pool(pool)
            .with_disk_manager_builder(DiskManagerBuilder::default().with_mode(
                DiskManagerMode::Directories(vec![owner.files.directory.path().to_path_buf()]),
            ))
            .build_arc()
            .map_err(ModelError::codec)?;
        owner.context = SessionContext::new_with_config_rt(owner.context.copied_config(), runtime);
        owner.budget = budget;
        Ok(workspace)
    }
    fn writable(&self) -> Result<(), ModelError> {
        if self
            .compilation_complete
            .lock()
            .map_err(|_| poisoned())?
            .is_some()
        {
            return Err(ModelError::Invalid(
                "completed compiler workspace is immutable".into(),
            ));
        }
        Ok(())
    }
    pub(crate) async fn finish_compilation(
        &self,
        captures: ContentHash,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
        configuration: ContentHash,
    ) -> Result<(), ModelError> {
        let _gate = self.completion_gate.lock().await;
        self.cancellation.check()?;
        self.writable()?;
        let content = self.content()?;
        *self.compilation_complete.lock().map_err(|_| poisoned())? = Some(CompilationCompletion {
            frontier,
            profile,
            configuration,
            captures,
            content,
        });
        Ok(())
    }
    pub(crate) fn captures(
        &self,
        captured: &cpg_extract::bundle::CapturedInputs,
    ) -> Result<ContentHash, ModelError> {
        capture_identity(captured, &self.budget)
    }
    pub(crate) fn require_compilation(
        &self,
        captured: &cpg_extract::bundle::CapturedInputs,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
        configuration: ContentHash,
    ) -> Result<(), ModelError> {
        let expected = CompilationCompletion {
            frontier,
            profile,
            configuration,
            captures: self.captures(captured)?,
            content: self.content()?,
        };
        if self
            .compilation_complete
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            != Some(&expected)
        {
            return Err(ModelError::Invalid(
                "artifact requires the completed requested compilation and exact captures".into(),
            ));
        }
        Ok(())
    }
    fn coverage_rows<R: Record>(&self) -> Result<(Vec<R>, StateCharge), ModelError> {
        let mut rows = Vec::new();
        let mut charge = StateCharge::new(&self.budget, "facts-coverage-input");
        for batch in self
            .completed::<R>()?
            .read::<R>(self.model.clone(), self.budget.clone())?
        {
            for row in batch?.rows() {
                charge.grow(row.row_bytes().saturating_add(size_of::<R>()))?;
                rows.push(row.clone());
            }
        }
        Ok((rows, charge))
    }
    pub fn facts_availability(
        &self,
        profile: Profile,
    ) -> Result<lctx_model::domain::admission::ScopedAvailability, ModelError> {
        use lctx_model::domain::{
            admission::{Expected, FrontierContract, ScopedAvailability},
            attribution::ProviderCoverage,
            input::{ArtifactUse, InputRevision},
            source::{CoverageScope, SourceArtifact},
            stages::Schedule,
        };
        let providers = crate::facts::providers(ContentHash::of(b"facts-coverage-contract"));
        let schedule = Schedule::build(
            &self.model,
            providers.iter().map(|p| p.declaration(profile)).collect(),
            &[],
            profile,
        )?;
        let contract = FrontierContract::facts(&self.model, profile)?.preflight(&schedule)?;
        let (inputs, _inputs) = self.coverage_rows::<InputRevision>()?;
        let (artifacts, _artifacts) = self.coverage_rows::<SourceArtifact>()?;
        let (uses, _uses) = self.coverage_rows::<ArtifactUse>()?;
        let expected = contract
            .expected_coverage(&inputs, &artifacts, &uses)?
            .into_keys()
            .collect::<std::collections::BTreeSet<Expected>>();
        let (scope_rows, _scopes) = self.coverage_rows::<CoverageScope>()?;
        let scopes = scope_rows.into_iter().map(|r| (r.id(), r)).collect();
        let (rows, _rows) = self.coverage_rows::<ProviderCoverage>()?;
        ScopedAvailability::from_completed(profile, &expected, &rows, &scopes, &self.budget)
    }
    /// Semantic content over the actual completed typed streams, independent of IPC bytes.
    pub fn content(&self) -> Result<ContentHash, ModelError> {
        let mut sink = lctx_model::domain::KeySink::new("compiler-workspace-content/v1");
        for relation in self.completed_relations()? {
            sink.part(relation.name().as_bytes(), &relation.content().0);
        }
        Ok(sink.finish())
    }
    /// Reuse the model's ordered invariant state machines with spillable SQL ordering. No global
    /// graph concatenation or resident copy of all completed relations is constructed.
    pub async fn validate(&self) -> Result<ContentHash, ModelError> {
        let profile = self
            .completed_relations()?
            .first()
            .map_or(Profile::Catalog, |r| r.profile);
        self.validate_scope(profile, false, None).await
    }
    /// Establish necessary semantic properties for immutable completed input descriptors.
    pub async fn checked_inputs(
        &self,
        inputs: &CompletedInputs,
    ) -> Result<CheckedInputs, ModelError> {
        self.validate_scope(inputs.profile, true, Some(inputs))
            .await?;
        Ok(CheckedInputs {
            inputs: inputs.clone(),
            attempt: self.files.clone(),
            policy: self.model.digest(),
        })
    }
    pub async fn admit_semantics(&self, profile: Profile) -> Result<CheckedInputs, ModelError> {
        let relations = self.completed_relations()?;
        let mut inputs = self.inputs(
            "semantic-admission",
            profile,
            relations.iter().map(|r| r.name()),
        )?;
        // Whole-artifact admission validates actual frozen owner premises in a live attempt.
        // A portable import has only canonical streams and uses the explicitly detached path.
        // Consumer admission remains exact and never substitutes a missing selected epoch.
        self.validate_scope(profile, true, None).await?;
        for ((boundary, name), relation) in
            self.frozen_shared.lock().map_err(|_| poisoned())?.iter()
        {
            inputs
                .relations
                .insert((*name, Some(*boundary)), relation.clone());
        }
        Ok(CheckedInputs {
            inputs,
            attempt: self.files.clone(),
            policy: self.model.digest(),
        })
    }
    async fn validate_scope(
        &self,
        profile: Profile,
        admission: bool,
        selected: Option<&CompletedInputs>,
    ) -> Result<ContentHash, ModelError> {
        let relations = selected
            .map(|inputs| inputs.relations().cloned().collect())
            .unwrap_or(self.completed_relations()?);
        let names = relations
            .iter()
            .map(|r| r.name())
            .collect::<std::collections::BTreeSet<_>>();
        // A checked dependency closure retains its selected publication epochs.
        // Reconstructing this set by name would silently substitute the latest stream.
        let inputs = match selected {
            Some(inputs) => inputs.clone(),
            None => self.inputs("artifact-admission", profile, names.iter().copied())?,
        };
        let session = inputs.session(self).await?;
        self.validate_references(&session, &relations).await?;
        // Catalog explicitly leaves native flow unrequested. Empty premises come from that
        // provider declaration, never from arbitrary missing requested relations.
        let mut unrequested = std::collections::BTreeSet::new();
        if profile == Profile::Catalog {
            let declaration = crate::facts::providers(ContentHash::of(b"flow-profile-premises"))
                .into_iter()
                .map(|provider| provider.declaration(Profile::Behavioral))
                .find(|stage| stage.name == "ty_flow")
                .ok_or_else(|| ModelError::Invalid("native flow declaration missing".into()))?;
            for relation in declaration
                .outputs
                .iter()
                .filter(|relation| !lctx_model::domain::stages::is_vocabulary(relation.name()))
            {
                if !names.contains(relation.name()) {
                    session
                        .register_batch(
                            relation.name(),
                            RecordBatch::new_empty(
                                self.model
                                    .relation(relation.name())
                                    .ok_or(ModelError::Schema(relation.name()))?
                                    .schema()
                                    .clone(),
                            ),
                        )
                        .map_err(ModelError::codec)?;
                    unrequested.insert(relation.name());
                }
            }
        }
        let frozen_tables = match selected {
            Some(inputs) => inputs
                .relations
                .keys()
                .filter_map(|(name, prefix)| {
                    prefix.map(|boundary| {
                        (
                            (boundary, *name),
                            CompletedInputs::table(name, Some(boundary)),
                        )
                    })
                })
                .collect(),
            None => self.validation_views(&session).await?,
        };
        let checks = if admission {
            self.model.admission_candidates_for_scope(&names)?
        } else {
            self.model
                .invariants_for_scope_with_premises(&names, &unrequested)?
        };
        for invariant in checks {
            let mut check = (invariant.create)(self.budget());
            let execution_scope = check.execution_scope();
            if let Some(scope) = &execution_scope {
                let alias = if let Some(inputs) = selected {
                    inputs.validation_table(&scope.root)?
                } else {
                    Self::validation_table(&scope.root, &frozen_tables)?.to_owned()
                };
                let sql = format!(
                    "SELECT id FROM {} LIMIT 1",
                    crate::consumed_rows::identifier(&alias)
                );
                let mut roots = crate::sql::query(&session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                let mut nonempty = false;
                while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    nonempty |= batch.num_rows() != 0;
                }
                if !nonempty {
                    continue;
                }
            }
            let normalization_scope = check.normalization_scope();
            let projection_scope = check.projection_scope();
            let applicability_roots = normalization_scope
                .map(|scope| scope.roots())
                .or_else(|| projection_scope.map(|scope| scope.roots()));
            if let Some(declared_roots) = applicability_roots {
                // A missing requested candidate stream cannot establish an empty domain.
                // Only actual complete-empty roots and declared unrequested flow roots do.
                if declared_roots
                    .iter()
                    .all(|root| names.contains(root.name()) || unrequested.contains(root.name()))
                {
                    let mut nonempty = false;
                    for root in &declared_roots {
                        let alias = if unrequested.contains(root.name()) {
                            root.name().to_owned()
                        } else if let Some(inputs) = selected {
                            inputs.validation_table(root)?
                        } else {
                            Self::validation_table(root, &frozen_tables)?.to_owned()
                        };
                        let mut rows = crate::sql::query(
                            &session,
                            &format!(
                                "SELECT id FROM {} LIMIT 1",
                                crate::consumed_rows::identifier(&alias)
                            ),
                        )
                        .await
                        .map_err(ModelError::codec)?
                        .execute_stream()
                        .await
                        .map_err(ModelError::codec)?;
                        while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
                            self.cancellation.check()?;
                            nonempty |= batch.num_rows() != 0;
                        }
                        if nonempty {
                            break;
                        }
                    }
                    if !nonempty {
                        continue;
                    }
                }
            }
            if let Some(input) = invariant
                .inputs
                .iter()
                .find(|input| !names.contains(input.name()) && !unrequested.contains(input.name()))
            {
                return Err(ModelError::Invalid(format!(
                    "{} requires validation premise {} outside scope",
                    invariant.name,
                    input.name()
                )));
            }
            if let Some(scope) = check.retrieval_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_retrieval::validate_retrieval(
                    &invariant,
                    scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                continue;
            }
            if let Some(scope) = projection_scope {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::normalize::validate_projections(
                    scope,
                    &invariant,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                continue;
            }
            if let Some(scope) = normalization_scope {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                use lctx_model::domain::normalized::admission::Scope;
                match scope {
                    Scope::Callables => {
                        crate::normalize::validate_callables(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                    Scope::Receivers => {
                        crate::normalize::validate_receivers(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                    Scope::Events => {
                        crate::normalize::validate_events(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                    Scope::Bindings => {
                        crate::normalize::validate_bindings(
                            &invariant,
                            tables,
                            &session,
                            self.budget(),
                            &self.cancellation,
                        )
                        .await?
                    }
                }
                continue;
            }
            if let Some(scope) = execution_scope {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_execution::validate_execution(
                    &invariant,
                    &scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                continue;
            }
            if let Some(scope) = check.aspect_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_aspects::validate_aspects(
                    &invariant,
                    &scope,
                    tables,
                    &self.model,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                continue;
            }
            if let Some(scope) = check.inventory_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_inventory::validate_inventory(
                    &invariant,
                    &scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                continue;
            }
            if let Some(scope) = check.support_scope() {
                drop(check);
                let mut tables = Vec::with_capacity(invariant.inputs.len());
                for input in &invariant.inputs {
                    let alias = if unrequested.contains(input.name()) {
                        input.name().to_owned()
                    } else if let Some(inputs) = selected {
                        inputs.validation_table(input)?
                    } else {
                        Self::validation_table(input, &frozen_tables)?.to_owned()
                    };
                    tables.push(crate::consumed_rows::ClosureTable {
                        relation: self
                            .model
                            .relation(input.name())
                            .ok_or(ModelError::Schema(input.name()))?
                            .clone(),
                        alias,
                    });
                }
                crate::scoped_admission::validate_support(
                    &invariant,
                    &scope,
                    tables,
                    &session,
                    self.budget(),
                    &self.cancellation,
                )
                .await?;
                continue;
            }
            for input in &invariant.inputs {
                let order = input
                    .order()
                    .iter()
                    .map(|name| format!("\"{name}\""))
                    .collect::<Vec<_>>()
                    .join(",");
                let selected_table;
                let table = if unrequested.contains(input.name()) {
                    input.name()
                } else if let Some(inputs) = selected {
                    selected_table = inputs.validation_table(input)?;
                    selected_table.as_str()
                } else {
                    Self::validation_table(input, &frozen_tables)?
                };
                let sql = format!(
                    "SELECT * FROM \"{}\"{}",
                    table,
                    if order.is_empty() {
                        String::new()
                    } else {
                        format!(" ORDER BY {order}")
                    }
                );
                let mut stream = crate::sql::query(&session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    check.visit_input(input, &batch)?;
                }
            }
            check.finish()?;
        }
        self.content()
    }
    /// Independent frontier obligations use the actual profile and retained capture/method
    /// definitions. They require no producer replay or historical publication receipts.
    pub async fn admit_frontier(
        &self,
        frontier: lctx_model::domain::admission::Frontier,
        profile: Profile,
    ) -> Result<(), ModelError> {
        use lctx_model::domain::admission::Frontier;
        let ids: &[&str] = match frontier {
            Frontier::Facts | Frontier::Normalized => &[],
            Frontier::Conformance => {
                return Err(ModelError::Frontier(
                    "diagnostic conformance is not a complete artifact".into(),
                ));
            }
            Frontier::Analysis => &["complete_analysis_frontier"],
            Frontier::Catalog => &["complete_analysis_frontier", "complete_catalog_frontier"],
        };
        let relations = self.completed_relations()?;
        let access = self.inputs(
            "frontier-admission",
            profile,
            relations.iter().map(|r| r.name()),
        )?;
        let session = access.session(self).await?;
        for id in ids {
            let invariant = self.model.publication_check(id)?;
            let mut check = (invariant.create)(self.budget());
            for input in &invariant.inputs {
                let mut stream = crate::sql::query(
                    &session,
                    &format!("SELECT * FROM \"{}\" ORDER BY id", input.name()),
                )
                .await
                .map_err(ModelError::codec)?
                .execute_stream()
                .await
                .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    check.visit_input(input, &batch)?;
                }
            }
            check.finish(&[], profile)?;
        }
        Ok(())
    }
    async fn validation_views(
        &self,
        session: &SessionContext,
    ) -> Result<
        BTreeMap<
            (
                lctx_model::domain::stages::PublicationBoundary,
                &'static str,
            ),
            String,
        >,
        ModelError,
    > {
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?.clone();
        let mut tables = BTreeMap::new();
        for ((boundary, name), source) in &frozen {
            let table = format!("_frozen_{}_{name}", boundary.name().to_ascii_lowercase());
            session
                .register_arrow(
                    &table,
                    source.path.to_string_lossy(),
                    ArrowReadOptions::default().schema(source.relation.schema().as_ref()),
                )
                .await
                .map_err(ModelError::codec)?;
            tables.insert((*boundary, *name), table);
        }
        Ok(tables)
    }
    fn validation_table<'a>(
        input: &lctx_model::domain::ValidationInput,
        tables: &'a BTreeMap<
            (
                lctx_model::domain::stages::PublicationBoundary,
                &'static str,
            ),
            String,
        >,
    ) -> Result<&'a str, ModelError> {
        match input.prefix() {
            Some(boundary) if !tables.is_empty() => tables
                .get(&(boundary, input.name()))
                .map(String::as_str)
                .ok_or_else(|| {
                    ModelError::Invalid(format!("missing frozen validation input {}", input.name()))
                }),
            _ => Ok(input.name()),
        }
    }
    /// Explicit fixture replay for selected owner source/coverage contracts. Artifact admission
    /// does not automatically recompute every compiler algorithm.
    pub async fn validate_publications(&self, ids: &[&str]) -> Result<(), ModelError> {
        let relations = self.completed_relations()?;
        let inputs = self.inputs(
            "selected-publication-controls",
            Profile::Catalog,
            relations.iter().map(|r| r.name()),
        )?;
        let session = inputs.session(self).await?;
        let frozen_tables = self.validation_views(&session).await?;
        let mut checks = BTreeMap::new();
        for source in &relations {
            for id in source
                .relation
                .publication_refs()
                .iter()
                .filter(|id| ids.contains(id))
            {
                if let Some(previous) = checks.insert(*id, source.clone())
                    && (previous.producer != source.producer || previous.inputs != source.inputs)
                {
                    return Err(ModelError::Invalid(
                        "publication invariant spans incompatible producer inputs".into(),
                    ));
                }
            }
        }
        if ids.iter().any(|id| !checks.contains_key(id)) {
            return Err(ModelError::Invalid(
                "selected publication control is absent".into(),
            ));
        }
        for (id, source) in checks {
            let invariant = self.model.publication_check(id)?;
            let mut check = (invariant.create)(self.budget());
            for input in &invariant.inputs {
                let order = input
                    .order()
                    .iter()
                    .map(|name| format!("\"{name}\""))
                    .collect::<Vec<_>>()
                    .join(",");
                let table = Self::validation_table(input, &frozen_tables)?;
                let sql = format!(
                    "SELECT * FROM \"{}\"{}",
                    table,
                    if order.is_empty() {
                        String::new()
                    } else {
                        format!(" ORDER BY {order}")
                    }
                );
                let mut stream = crate::sql::query(&session, &sql)
                    .await
                    .map_err(ModelError::codec)?
                    .execute_stream()
                    .await
                    .map_err(ModelError::codec)?;
                while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
                    self.cancellation.check()?;
                    check.visit_input(input, &batch)?;
                }
            }
            check.finish(&source.inputs, source.profile)?;
        }
        Ok(())
    }
    async fn validate_references(
        &self,
        session: &SessionContext,
        relations: &[Arc<CompletedRelation>],
    ) -> Result<(), ModelError> {
        use arrow_array::{
            Int16Array,
            builder::{FixedSizeBinaryBuilder, Int16Builder, StringBuilder},
        };
        use arrow_schema::{DataType, Field, Schema};
        let target_schema = Arc::new(Schema::new(vec![
            Field::new("relation", DataType::Utf8, false),
            Field::new("id", DataType::FixedSizeBinary(16), false),
            Field::new("subtype", DataType::Int16, true),
        ]));
        let refs_schema = Arc::new(Schema::new(vec![
            Field::new("source", DataType::Utf8, false),
            Field::new("field", DataType::Utf8, false),
            Field::new("target", DataType::Utf8, false),
            Field::new("id", DataType::FixedSizeBinary(16), false),
            Field::new("subtype", DataType::Int16, true),
        ]));
        let target_path = self.path("nominal-keys", "validation");
        let refs_path = self.path("nominal-references", "validation");
        let mut targets = FileWriter::try_new(
            File::create(&target_path).map_err(ModelError::codec)?,
            &target_schema,
        )
        .map_err(ModelError::codec)?;
        let mut references = FileWriter::try_new(
            File::create(&refs_path).map_err(ModelError::codec)?,
            &refs_schema,
        )
        .map_err(ModelError::codec)?;
        let mut source_names = StringBuilder::new();
        let mut fields = StringBuilder::new();
        let mut target_names = StringBuilder::new();
        let mut ids = FixedSizeBinaryBuilder::new(16);
        let mut tags = Int16Builder::new();
        let mut count = 0;
        let _buffer = self.budget.reserve(
            "nominal-reference-transfer",
            self.options.batch_rows.saturating_mul(1024),
        )?;
        macro_rules! flush_refs {
            () => {{
                if count > 0 {
                    let batch = RecordBatch::try_new(
                        refs_schema.clone(),
                        vec![
                            Arc::new(source_names.finish()),
                            Arc::new(fields.finish()),
                            Arc::new(target_names.finish()),
                            Arc::new(ids.finish()),
                            Arc::new(tags.finish()),
                        ],
                    )
                    .map_err(ModelError::codec)?;
                    references.write(&batch).map_err(ModelError::codec)?;
                    count = 0;
                }
            }};
        }
        // Each completed batch is decoded once for its namespace keys and declared nominal fields.
        for source in relations {
            for batch in source.batches()? {
                self.cancellation.check()?;
                let batch = batch.map_err(ModelError::codec)?;
                let _input = self.budget.reserve(
                    "nominal-reference-input",
                    lctx_model::domain::logical_batch_bytes(&batch)?,
                )?;
                let row_ids = batch
                    .column_by_name("id")
                    .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
                    .ok_or(ModelError::Schema(source.name()))?;
                let subtype = source
                    .relation
                    .sum()
                    .map(|sum| {
                        batch
                            .column_by_name(sum.tag)
                            .and_then(|c| c.as_any().downcast_ref::<Int16Array>())
                            .ok_or(ModelError::Schema(source.name()))
                    })
                    .transpose()?;
                let mut names = StringBuilder::new();
                let mut key_ids = FixedSizeBinaryBuilder::new(16);
                let mut key_tags = Int16Builder::new();
                for row in 0..batch.num_rows() {
                    names.append_value(source.name());
                    key_ids
                        .append_value(row_ids.value(row))
                        .map_err(ModelError::codec)?;
                    if let Some(tags) = subtype {
                        key_tags.append_value(tags.value(row));
                    } else {
                        key_tags.append_null();
                    }
                }
                targets
                    .write(
                        &RecordBatch::try_new(
                            target_schema.clone(),
                            vec![
                                Arc::new(names.finish()),
                                Arc::new(key_ids.finish()),
                                Arc::new(key_tags.finish()),
                            ],
                        )
                        .map_err(ModelError::codec)?,
                    )
                    .map_err(ModelError::codec)?;
                for field in source.relation.fields().iter().filter(|f| !f.list()) {
                    let Some((_, target)) = field.target() else {
                        continue;
                    };
                    let column = batch
                        .column_by_name(field.name())
                        .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
                        .ok_or(ModelError::Schema(source.name()))?;
                    for row in 0..batch.num_rows() {
                        if column.is_null(row) {
                            continue;
                        }
                        source_names.append_value(source.name());
                        fields.append_value(field.name());
                        target_names.append_value(target);
                        ids.append_value(column.value(row))
                            .map_err(ModelError::codec)?;
                        if let Some(tag) = field.subtype() {
                            tags.append_value(tag);
                        } else {
                            tags.append_null();
                        }
                        count += 1;
                        if count >= self.options.batch_rows {
                            flush_refs!();
                        }
                    }
                }
            }
        }
        flush_refs!();
        debug_assert_eq!(count, 0);
        targets.finish().map_err(ModelError::codec)?;
        references.finish().map_err(ModelError::codec)?;
        drop(targets);
        drop(references);
        drop(_buffer);
        session
            .register_arrow(
                "_nominal_targets",
                target_path.to_string_lossy(),
                ArrowReadOptions::default().schema(&target_schema),
            )
            .await
            .map_err(ModelError::codec)?;
        session
            .register_arrow(
                "_nominal_references",
                refs_path.to_string_lossy(),
                ArrowReadOptions::default().schema(&refs_schema),
            )
            .await
            .map_err(ModelError::codec)?;
        let mut stream=crate::sql::query(session,"SELECT r.source,r.field,r.target FROM _nominal_references r LEFT JOIN _nominal_targets t ON r.target=t.relation AND r.id=t.id WHERE t.id IS NULL OR (r.subtype IS NOT NULL AND (t.subtype IS NULL OR r.subtype<>t.subtype)) LIMIT 1").await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
            if batch.num_rows() > 0 {
                return Err(ModelError::Invalid(format!(
                    "missing or wrong-subtype nominal reference: {batch:?}"
                )));
            }
        }
        Ok(())
    }
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
    pub fn options(&self) -> WorkspaceOptions {
        self.options
    }
    pub fn model(&self) -> &Arc<ValidatedModel> {
        &self.model
    }
    pub fn cancellation(&self) -> Cancellation {
        self.cancellation.clone()
    }
    fn path(&self, relation: &str, suffix: &str) -> PathBuf {
        self.files.directory.path().join(format!(
            "{}-{relation}-{suffix}.arrow",
            self.next.fetch_add(1, Ordering::Relaxed)
        ))
    }
    pub fn completed<R: Record>(&self) -> Result<Arc<CompletedRelation>, ModelError> {
        self.relation(R::NAME)
    }
    pub fn relation(&self, name: &str) -> Result<Arc<CompletedRelation>, ModelError> {
        self.completed
            .lock()
            .map_err(|_| poisoned())?
            .get(name)
            .cloned()
            .ok_or_else(|| ModelError::Invalid(format!("input {name} is not completed")))
    }
    /// Resolve a replay's declared completed view without widening it to later shared values.
    pub fn input_relation(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<Arc<CompletedRelation>, ModelError> {
        if let Some(boundary) = input.prefix() {
            return self
                .frozen_shared
                .lock()
                .map_err(|_| poisoned())?
                .get(&(boundary, input.name()))
                .cloned()
                .ok_or_else(|| {
                    ModelError::Invalid(format!(
                        "missing frozen {:?} input {}",
                        boundary,
                        input.name()
                    ))
                });
        }
        self.relation(input.name())
    }
    pub fn completed_relations(&self) -> Result<Vec<Arc<CompletedRelation>>, ModelError> {
        Ok(self
            .completed
            .lock()
            .map_err(|_| poisoned())?
            .values()
            .cloned()
            .collect())
    }
    /// Build an explicit immutable input set. Later completions do not change this view.
    pub fn inputs(
        &self,
        name: &'static str,
        profile: Profile,
        names: impl IntoIterator<Item = &'static str>,
    ) -> Result<CompletedInputs, ModelError> {
        let mut relations = BTreeMap::new();
        for name in names {
            relations.insert((name, None), self.relation(name)?);
        }
        Ok(CompletedInputs {
            name,
            profile,
            relations,
        })
    }
    /// Bind each declared semantic boundary to immutable shared streams. Only small
    /// descriptors are retained; later contributions cannot widen an earlier producer's inputs.
    pub fn freeze_inputs(
        &self,
        boundary: lctx_model::domain::stages::PublicationBoundary,
    ) -> Result<(), ModelError> {
        let completed = self.completed.lock().map_err(|_| poisoned())?;
        let mut frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        if frozen.keys().any(|(existing, _)| *existing == boundary) {
            return Err(ModelError::Invalid(
                "compiler input boundary already frozen".into(),
            ));
        }
        for (name, source) in completed
            .iter()
            .filter(|(name, _)| lctx_model::domain::stages::is_epoch_shared(name))
        {
            frozen.insert((boundary, *name), source.clone());
        }
        Ok(())
    }
    pub fn stage_inputs(
        &self,
        declaration: &lctx_model::domain::stages::Stage,
        profile: Profile,
    ) -> Result<CompletedInputs, ModelError> {
        self.stage_inputs_selected(declaration, declaration, profile)
    }
    pub(crate) fn stage_inputs_selected(
        &self,
        declaration: &lctx_model::domain::stages::Stage,
        selected: &lctx_model::domain::stages::Stage,
        profile: Profile,
    ) -> Result<CompletedInputs, ModelError> {
        if declaration.inputs.len() != selected.inputs.len() {
            return Err(ModelError::Invalid(
                "selected compiler inputs differ from declaration".into(),
            ));
        }
        let frozen = self.frozen_shared.lock().map_err(|_| poisoned())?;
        let mut relations = BTreeMap::new();
        for (declared, input) in declaration.inputs.iter().zip(&selected.inputs) {
            if declared.name() != input.name() {
                return Err(ModelError::Invalid(
                    "selected compiler input type changed".into(),
                ));
            }
            let source = if let Some(boundary) = input.prefix() {
                frozen
                    .get(&(boundary, input.name()))
                    .cloned()
                    .ok_or_else(|| {
                        ModelError::Invalid(format!(
                            "{} lacks frozen {:?} input {}",
                            declaration.name,
                            boundary,
                            input.name()
                        ))
                    })?
            } else {
                self.relation(input.name())?
            };
            if relations
                .insert((declared.name(), declared.prefix()), source)
                .is_some()
            {
                return Err(ModelError::Invalid(
                    "duplicate selected compiler input".into(),
                ));
            }
        }
        Ok(CompletedInputs {
            name: declaration.name,
            profile,
            relations,
        })
    }
    pub fn output(
        self: &Arc<Self>,
        name: &'static str,
        profile: Profile,
        implementation: ContentHash,
        inputs: CompletedInputs,
    ) -> ProducerOutput {
        ProducerOutput {
            workspace: self.clone(),
            name,
            profile,
            implementation,
            configuration: None,
            inputs,
            expected: None,
            allowed: None,
            writers: Mutex::default(),
            outcome: Mutex::new(None),
            failed: AtomicBool::new(false),
        }
    }
    pub fn producer(
        self: &Arc<Self>,
        declaration: &lctx_model::domain::stages::Stage,
        profile: Profile,
        inputs: CompletedInputs,
    ) -> ProducerOutput {
        let mut output = self.output(declaration.name, profile, declaration.code, inputs);
        output.configuration = Some(declaration.configuration);
        output.expected = Some(declaration.outputs.iter().map(|r| r.name()).collect());
        output.allowed = Some(
            declaration
                .outputs
                .iter()
                .chain(&declaration.contributes)
                .map(|r| r.name())
                .collect(),
        );
        output
    }
    async fn order(
        &self,
        producer: &'static str,
        implementation: ContentHash,
        configuration: Option<ContentHash>,
        pending: PendingRelation,
        inputs: Arc<[lctx_model::domain::analysis::sources::SourceSnapshot]>,
        profile: Profile,
    ) -> Result<Arc<CompletedRelation>, ModelError> {
        self.cancellation.check()?;
        let name = pending.relation.name();
        let path = self.path(name, "complete");
        // Only compact identity/IPC coordinates enter external sorting. Rich original rows
        // remain in immutable pending files and are gathered into bounded output batches.
        let mut sources = vec![crate::ordered_stream::Source {
            path: pending.path.clone(),
            blocks: pending.blocks.clone(),
        }];
        if let Ok(previous) = self.relation(name) {
            if pending.contribution || previous.contribution {
                sources.push(crate::ordered_stream::Source {
                    path: previous.path.clone(),
                    blocks: previous.blocks.clone(),
                });
            } else {
                return Err(ModelError::Invalid(format!(
                    "completed output {name} already has an owner"
                )));
            }
        }
        let index = self.path(name, "order-index");
        let (rows, content, blocks) = crate::ordered_stream::order(
            &pending.relation,
            sources,
            &self.context,
            self.budget(),
            &self.cancellation,
            &index,
            &path,
            self.options.batch_rows,
        )
        .await?;
        self.cancellation.check()?;
        Ok(Arc::new(CompletedRelation {
            relation: pending.relation,
            producer,
            implementation,
            configuration,
            contract: self.model.digest(),
            content,
            rows,
            inputs,
            profile,
            contribution: pending.contribution,
            snapshot: (pending.snapshot)(
                producer,
                self.model.digest(),
                implementation,
                content,
                rows,
            )?,
            path,
            blocks,
            _files: self.files.clone(),
        }))
    }
}
fn poisoned() -> ModelError {
    ModelError::Invalid("workspace ownership poisoned".into())
}

/// A stream descriptor, not a resident collection or a database read capability.
pub struct CompletedRelation {
    blocks: Arc<[usize]>,
    relation: Relation,
    producer: &'static str,
    implementation: ContentHash,
    configuration: Option<ContentHash>,
    contract: ContentHash,
    content: ContentHash,
    rows: u64,
    path: PathBuf,
    inputs: Arc<[lctx_model::domain::analysis::sources::SourceSnapshot]>,
    profile: Profile,
    contribution: bool,
    snapshot: lctx_model::domain::analysis::sources::SourceSnapshot,
    _files: Arc<WorkspaceFiles>,
}
impl CompletedRelation {
    pub fn name(&self) -> &'static str {
        self.relation.name()
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn content(&self) -> ContentHash {
        self.content
    }
    pub fn rows(&self) -> u64 {
        self.rows
    }
    pub fn producer(&self) -> &'static str {
        self.producer
    }
    pub fn implementation(&self) -> ContentHash {
        self.implementation
    }
    pub fn configuration(&self) -> Option<ContentHash> {
        self.configuration
    }
    pub fn contract(&self) -> ContentHash {
        self.contract
    }
    pub fn snapshot(&self) -> lctx_model::domain::analysis::sources::SourceSnapshot {
        self.snapshot.clone()
    }
    pub fn batches(&self) -> Result<FileReader<File>, ModelError> {
        FileReader::try_new(File::open(&self.path).map_err(ModelError::codec)?, None)
            .map_err(ModelError::codec)
    }
    pub fn read<R: Record>(
        &self,
        model: Arc<ValidatedModel>,
        budget: ResourceBudget,
    ) -> Result<TypedBatches<R>, ModelError> {
        if self.name() != R::NAME || self.relation.schema().as_ref() != R::schema().as_ref() {
            return Err(ModelError::Schema(R::NAME));
        }
        Ok(TypedBatches {
            reader: self.batches()?,
            model,
            budget,
            marker: Default::default(),
            _files: self._files.clone(),
        })
    }
}
pub struct TypedBatches<R: Record> {
    reader: FileReader<File>,
    model: Arc<ValidatedModel>,
    budget: ResourceBudget,
    marker: std::marker::PhantomData<R>,
    _files: Arc<WorkspaceFiles>,
}
impl<R: Record> Iterator for TypedBatches<R> {
    type Item = Result<Batch<R>, ModelError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next().map(|batch| {
            batch
                .map_err(ModelError::codec)
                .and_then(|batch| Batch::read(&self.model, &batch, &self.budget))
        })
    }
}
/// Privately constructed after owner admission succeeds. Its immutable descriptors keep their
/// attempt files alive. Consumers compare descriptors and policy, without rehashing input bytes.
#[derive(Clone)]
pub struct CheckedInputs {
    inputs: CompletedInputs,
    attempt: Arc<WorkspaceFiles>,
    policy: ContentHash,
}
impl CheckedInputs {
    pub fn inputs(&self) -> &CompletedInputs {
        &self.inputs
    }
    /// Project an admitted authority onto an exact immutable dependency subset. No rows are
    /// read again; the selected descriptors retain the same attempt and admission policy.
    pub fn select(
        &self,
        required: &[lctx_model::domain::ValidationInput],
    ) -> Result<Self, ModelError> {
        Ok(Self {
            inputs: self.inputs.select(required)?,
            attempt: self.attempt.clone(),
            policy: self.policy,
        })
    }
    pub fn require(
        &self,
        workspace: &Workspace,
        inputs: &CompletedInputs,
    ) -> Result<(), ModelError> {
        if self.inputs.relations.len() != inputs.relations.len() {
            return Err(ModelError::Conflict("checked compiler input domain"));
        }
        self.require_subset(workspace, inputs)
    }
    /// A consumer may declare additional inputs; every descriptor underlying this authority must
    /// still be the identical completed relation in its selected dependency closure.
    pub fn require_subset(
        &self,
        workspace: &Workspace,
        inputs: &CompletedInputs,
    ) -> Result<(), ModelError> {
        if !Arc::ptr_eq(&self.attempt, &workspace.files)
            || self.policy != workspace.model.digest()
            || self.inputs.profile != inputs.profile
            || self.inputs.relations.iter().any(|(key, source)| {
                inputs
                    .relations
                    .get(key)
                    .is_none_or(|other| !Arc::ptr_eq(source, other))
            })
        {
            return Err(ModelError::Conflict("checked compiler inputs"));
        }
        workspace.cancellation.check()
    }
}
#[derive(Clone)]
pub struct CompletedInputs {
    name: &'static str,
    profile: Profile,
    relations: BTreeMap<
        (
            &'static str,
            Option<lctx_model::domain::stages::PublicationBoundary>,
        ),
        Arc<CompletedRelation>,
    >,
}
impl CompletedInputs {
    /// Check lifetime and identity of an actual producer's immutable streams. This does not
    /// admit semantic contents: only the model owner's opaque produced value carries authority.
    pub fn require_subset(
        &self,
        workspace: &Workspace,
        consumer: &CompletedInputs,
    ) -> Result<(), ModelError> {
        if self.profile != consumer.profile
            || self.relations.is_empty()
            || self.relations.iter().any(|(key, source)| {
                !Arc::ptr_eq(&source._files, &workspace.files)
                    || consumer
                        .relations
                        .get(key)
                        .is_none_or(|other| !Arc::ptr_eq(source, other))
            })
        {
            return Err(ModelError::Conflict(
                "completed producer dependency identity",
            ));
        }
        workspace.cancellation.check()
    }
    /// Resolve the exact table selected by a model-owned validation declaration.
    pub fn table_for(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<String, ModelError> {
        self.validation_table(input)
    }
    fn validation_table(
        &self,
        input: &lctx_model::domain::ValidationInput,
    ) -> Result<String, ModelError> {
        let key = (input.name(), input.prefix());
        if self.relations.contains_key(&key) {
            return Ok(Self::table(key.0, key.1));
        }
        if input.prefix().is_some() {
            return Err(ModelError::Conflict(
                "checked admission requires a missing input epoch",
            ));
        }
        let mut choices = self
            .relations
            .iter()
            .filter(|((name, _), _)| *name == input.name());
        let (key, source) = choices
            .next()
            .ok_or(ModelError::Conflict("checked admission input is absent"))?;
        if choices.any(|(_, other)| !Arc::ptr_eq(source, other)) {
            return Err(ModelError::Conflict(
                "checked admission input epoch is ambiguous",
            ));
        }
        Ok(Self::table(key.0, key.1))
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    /// Select a model-declared immutable dependency closure without copying or rehashing rows.
    pub fn select(
        &self,
        required: &[lctx_model::domain::ValidationInput],
    ) -> Result<Self, ModelError> {
        let mut relations = BTreeMap::new();
        for input in required {
            let key = (input.name(), input.prefix());
            let source = if let Some(source) = self.relations.get(&key) {
                source
            } else {
                if input.prefix().is_some() {
                    return Err(ModelError::Conflict("missing checked input prefix"));
                }
                let mut choices = self
                    .relations
                    .iter()
                    .filter(|((name, _), _)| *name == input.name());
                let (_, source) = choices
                    .next()
                    .ok_or(ModelError::Conflict("missing checked input"))?;
                if choices.any(|(_, other)| !Arc::ptr_eq(source, other)) {
                    return Err(ModelError::Conflict("ambiguous checked input"));
                }
                source
            };
            // Retain the original declaration key so later subset checks cannot erase epochs.
            let original = self
                .relations
                .iter()
                .find(|((name, prefix), candidate)| {
                    *name == input.name()
                        && (input.prefix().is_none() || *prefix == input.prefix())
                        && Arc::ptr_eq(source, candidate)
                })
                .ok_or(ModelError::Conflict("checked input selector"))?
                .0;
            relations.insert(*original, source.clone());
        }
        if relations.is_empty() {
            return Err(ModelError::Conflict("empty checked input closure"));
        }
        Ok(Self {
            name: self.name,
            profile: self.profile,
            relations,
        })
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn contains<R: Record>(&self) -> bool {
        self.relations.keys().any(|(name, _)| *name == R::NAME)
    }
    pub fn relation<R: Record>(&self) -> Result<&Arc<CompletedRelation>, ModelError> {
        let mut sources = self
            .relations
            .iter()
            .filter(|((name, _), _)| *name == R::NAME)
            .map(|(_, source)| source);
        let source = sources.next().ok_or_else(|| {
            ModelError::Invalid(format!("{} lacks explicit input {}", self.name, R::NAME))
        })?;
        if sources.any(|other| other.path != source.path) {
            return Err(ModelError::Invalid(format!(
                "{} must select a completed view of {}",
                self.name,
                R::NAME
            )));
        }
        Ok(source)
    }
    pub fn relation_at<R: Record>(
        &self,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<&Arc<CompletedRelation>, ModelError> {
        if let Some(source) = self.relations.get(&(R::NAME, prefix)) {
            return Ok(source);
        }
        if prefix.is_none() {
            return self.relation::<R>();
        }
        Err(ModelError::Invalid(format!(
            "{} lacks declared {:?} input {}",
            self.name,
            prefix,
            R::NAME
        )))
    }
    pub fn read<R: Record>(
        &self,
    ) -> Result<lctx_model::domain::analysis::sources::CompletedInput<R>, ModelError> {
        Self::read_source(self.relation::<R>()?)
    }
    pub fn read_at<R: Record>(
        &self,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<lctx_model::domain::analysis::sources::CompletedInput<R>, ModelError> {
        Self::read_source(self.relation_at::<R>(prefix)?)
    }
    fn read_source<R: Record>(
        source: &CompletedRelation,
    ) -> Result<lctx_model::domain::analysis::sources::CompletedInput<R>, ModelError> {
        lctx_model::domain::analysis::sources::CompletedInput::new(
            source.producer(),
            source.contract(),
            source.implementation(),
            source.content(),
            source.rows(),
        )
    }
    fn table(
        name: &str,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> String {
        format!(
            "_view_{}_{}",
            prefix
                .map(|p| p.name())
                .unwrap_or("default")
                .to_ascii_lowercase(),
            name
        )
    }
    pub fn table_at<R: Record>(
        &self,
        prefix: Option<lctx_model::domain::stages::PublicationBoundary>,
    ) -> Result<String, ModelError> {
        let selected = self.relation_at::<R>(prefix)?;
        let actual_prefix = if self.relations.contains_key(&(R::NAME, prefix)) {
            prefix
        } else {
            self.relations
                .iter()
                .find(|((name, _), source)| *name == R::NAME && source.path == selected.path)
                .map(|((_, prefix), _)| *prefix)
                .expect("selected declared source")
        };
        Ok(Self::table(R::NAME, actual_prefix))
    }
    pub fn snapshots(
        &self,
    ) -> impl Iterator<Item = lctx_model::domain::analysis::sources::SourceSnapshot> + '_ {
        self.relations.values().map(|source| source.snapshot())
    }
    pub fn relations(&self) -> impl Iterator<Item = &Arc<CompletedRelation>> {
        self.relations.values()
    }
    pub async fn session(&self, workspace: &Workspace) -> Result<SessionContext, ModelError> {
        let context = SessionContext::new_with_config_rt(
            workspace.context.copied_config(),
            workspace.context.runtime_env(),
        );
        for ((name, prefix), source) in &self.relations {
            let table = Self::table(name, *prefix);
            context
                .register_arrow(
                    &table,
                    source.path.to_string_lossy(),
                    ArrowReadOptions::default().schema(source.relation.schema().as_ref()),
                )
                .await
                .map_err(ModelError::codec)?;
        }
        // A plain typed read is available only when every declared selector names the same
        // immutable stream. Distinct semantic views require their qualified alias.
        let names = self
            .relations
            .keys()
            .map(|(name, _)| *name)
            .collect::<std::collections::BTreeSet<_>>();
        for name in names {
            let mut sources = self
                .relations
                .iter()
                .filter(|((candidate, _), _)| *candidate == name)
                .map(|(_, source)| source);
            let source = sources.next().expect("declared input");
            if sources.all(|other| other.path == source.path) {
                context
                    .register_arrow(
                        name,
                        source.path.to_string_lossy(),
                        ArrowReadOptions::default().schema(source.relation.schema().as_ref()),
                    )
                    .await
                    .map_err(ModelError::codec)?;
            }
        }
        Ok(context)
    }
}

type SnapshotBuilder =
    fn(
        &str,
        ContentHash,
        ContentHash,
        ContentHash,
        u64,
    ) -> Result<lctx_model::domain::analysis::sources::SourceSnapshot, ModelError>;
fn snapshot<R: Record>(
    producer: &str,
    contract: ContentHash,
    implementation: ContentHash,
    content: ContentHash,
    rows: u64,
) -> Result<lctx_model::domain::analysis::sources::SourceSnapshot, ModelError> {
    Ok(
        lctx_model::domain::analysis::sources::CompletedInput::<R>::new(
            producer,
            contract,
            implementation,
            content,
            rows,
        )?
        .snapshot(),
    )
}
struct PendingRelation {
    blocks: Arc<[usize]>,
    relation: Relation,
    path: PathBuf,
    contribution: bool,
    snapshot: SnapshotBuilder,
}
trait ErasedWriter: Send {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn close(
        self: Box<Self>,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<PendingRelation, ModelError>;
}
struct Writer<R: Record> {
    blocks: Vec<usize>,
    ipc: FileWriter<File>,
    path: PathBuf,
    pending: Vec<R>,
    bytes: usize,
    charge: Box<dyn Reservation>,
    limits: TransferLimits,
    contribution: bool,
}
impl<R: Record> Writer<R> {
    fn write_arrow(&mut self, batch: &RecordBatch) -> Result<(), ModelError> {
        let before = self
            .ipc
            .get_mut()
            .stream_position()
            .map_err(ModelError::codec)?;
        self.ipc.write(batch).map_err(ModelError::codec)?;
        let after = self
            .ipc
            .get_mut()
            .stream_position()
            .map_err(ModelError::codec)?;
        self.blocks
            .push(usize::try_from(after - before).map_err(ModelError::codec)?);
        Ok(())
    }
    fn flush(&mut self, model: &ValidatedModel, budget: &ResourceBudget) -> Result<(), ModelError> {
        if !self.pending.is_empty() {
            // Transfer the existing row reservation into encoding; these are the same rows,
            // rather than an additional resident copy of the pending batch.
            let charge = std::mem::replace(&mut self.charge, budget.reserve(R::NAME, 0)?);
            let batch = Batch::with_reservation(model, std::mem::take(&mut self.pending), charge)?;
            self.write_arrow(batch.arrow())?;
            self.bytes = 0;
        }
        Ok(())
    }
}
impl<R: Record> ErasedWriter for Writer<R> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn close(
        mut self: Box<Self>,
        model: &ValidatedModel,
        budget: &ResourceBudget,
    ) -> Result<PendingRelation, ModelError> {
        self.flush(model, budget)?;
        self.ipc.finish().map_err(ModelError::codec)?;
        self.ipc
            .into_inner()
            .map_err(ModelError::codec)?
            .sync_all()
            .map_err(ModelError::codec)?;
        Ok(PendingRelation {
            blocks: self.blocks.into(),
            relation: Relation::of::<R>(),
            path: self.path,
            contribution: self.contribution,
            snapshot: snapshot::<R>,
        })
    }
}
/// One producer owns pending streams; completion makes its whole output set visible atomically.
pub struct ProducerOutput {
    workspace: Arc<Workspace>,
    name: &'static str,
    profile: Profile,
    implementation: ContentHash,
    configuration: Option<ContentHash>,
    inputs: CompletedInputs,
    expected: Option<std::collections::BTreeSet<&'static str>>,
    allowed: Option<std::collections::BTreeSet<&'static str>>,
    writers: Mutex<BTreeMap<&'static str, Box<dyn ErasedWriter>>>,
    outcome: Mutex<Option<ProviderOutcome>>,
    failed: AtomicBool,
}
impl ProducerOutput {
    pub fn inputs(&self) -> &CompletedInputs {
        &self.inputs
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn workspace(&self) -> &Arc<Workspace> {
        &self.workspace
    }
    fn check(&self) -> Result<(), ModelError> {
        self.workspace.cancellation.check()?;
        self.workspace.writable()?;
        if self.failed.load(Ordering::Acquire)
            || self.outcome.lock().map_err(|_| poisoned())?.is_some()
        {
            return Err(ModelError::Invalid(
                "producer output is closed or failed".into(),
            ));
        }
        Ok(())
    }
    fn guarded<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ModelError>,
    ) -> Result<T, ModelError> {
        let result = self.check().and_then(|_| operation());
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub fn declare<R: Record>(&self) -> Result<(), ModelError> {
        self.declare_kind::<R>(lctx_model::domain::stages::is_epoch_shared(R::NAME))
    }
    fn declare_kind<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.guarded(|| self.declare_kind_inner::<R>(contribution))
    }
    fn declare_kind_inner<R: Record>(&self, contribution: bool) -> Result<(), ModelError> {
        self.check()?;
        if self
            .allowed
            .as_ref()
            .is_some_and(|names| !names.contains(R::NAME))
        {
            self.failed.store(true, Ordering::Release);
            return Err(ModelError::Invalid(format!(
                "{} did not declare {}",
                self.name,
                R::NAME
            )));
        }
        self.workspace.model.require::<R>()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        if writers.contains_key(R::NAME) {
            return Err(ModelError::Invalid(format!(
                "output {} declared twice",
                R::NAME
            )));
        }
        let path = self.workspace.path(R::NAME, "pending");
        let ipc = FileWriter::try_new(
            File::create(&path).map_err(ModelError::codec)?,
            R::schema().as_ref(),
        )
        .map_err(ModelError::codec)?;
        writers.insert(
            R::NAME,
            Box::new(Writer::<R> {
                blocks: Vec::new(),
                ipc,
                path,
                pending: Vec::new(),
                bytes: 0,
                charge: self.workspace.budget().reserve(R::NAME, 0)?,
                limits: TransferLimits {
                    rows: self.workspace.options.batch_rows,
                    bytes: (self.workspace.options.memory_bytes / 8)
                        .clamp(1, lctx_model::domain::resources::TRANSFER_BYTES),
                    ..Default::default()
                },
                contribution,
            }),
        );
        Ok(())
    }
    pub fn write<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        self.guarded(|| self.write_inner(batch))
    }
    fn write_inner<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        self.check()?;
        let mut writers = self.writers.lock().map_err(|_| poisoned())?;
        let writer = writers
            .get_mut(R::NAME)
            .and_then(|w| w.as_any_mut().downcast_mut::<Writer<R>>())
            .ok_or_else(|| ModelError::Invalid(format!("output {} was not declared", R::NAME)))?;
        writer.flush(&self.workspace.model, self.workspace.budget())?;
        let result = writer.write_arrow(batch.arrow());
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub fn contribute<R: Record>(&self, batch: &Batch<R>) -> Result<(), ModelError> {
        if !self
            .writers
            .lock()
            .map_err(|_| poisoned())?
            .contains_key(R::NAME)
        {
            self.declare_kind::<R>(true)?;
        }
        self.write(batch)
    }
    pub async fn push<R: Record>(&self, row: R) -> Result<(), ModelError> {
        self.push_sync(row)
    }
    pub fn push_sync<R: Record>(&self, row: R) -> Result<(), ModelError> {
        let result = (|| {
            self.check()?;
            row.validate()?;
            let mut writers = self.writers.lock().map_err(|_| poisoned())?;
            let writer = writers
                .get_mut(R::NAME)
                .and_then(|w| w.as_any_mut().downcast_mut::<Writer<R>>())
                .ok_or_else(|| {
                    ModelError::Invalid(format!("output {} was not declared", R::NAME))
                })?;
            let bytes = row.row_bytes();
            if bytes > writer.limits.max_row {
                return Err(ModelError::Invalid(format!(
                    "{} exceeds row limit",
                    R::NAME
                )));
            }
            if !writer.pending.is_empty()
                && (writer.pending.len() >= writer.limits.rows
                    || writer.bytes.saturating_add(bytes) > writer.limits.bytes)
            {
                writer.flush(&self.workspace.model, self.workspace.budget())?;
            }
            writer
                .charge
                .try_resize(writer.charge.size().saturating_add(bytes))?;
            writer.pending.push(row);
            writer.bytes += bytes;
            Ok(())
        })();
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }
    pub fn mark_finished(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.check()?;
        if outcome == ProviderOutcome::Failed {
            self.failed.store(true, Ordering::Release);
            return Err(ModelError::Invalid(
                "failed producer cannot complete outputs".into(),
            ));
        }
        *self.outcome.lock().map_err(|_| poisoned())? = Some(outcome);
        Ok(())
    }
    pub async fn finish(self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.mark_finished(outcome)?;
        self.complete().await
    }
    pub async fn complete(self) -> Result<(), ModelError> {
        self.workspace.cancellation.check()?;
        self.workspace.writable()?;
        if self.failed.load(Ordering::Acquire)
            || self.outcome.lock().map_err(|_| poisoned())?.is_none()
        {
            return Err(ModelError::Invalid(
                "producer did not complete successfully".into(),
            ));
        }
        let _completion = self.workspace.completion_gate.lock().await;
        self.workspace.writable()?;
        let writers = self.writers.into_inner().map_err(|_| poisoned())?;
        if let Some(expected) = &self.expected
            && let Some(name) = expected.iter().find(|name| !writers.contains_key(**name))
        {
            return Err(ModelError::Invalid(format!(
                "{} omitted completed output {name}",
                self.name
            )));
        }
        let input_snapshots: Arc<[_]> = self.inputs.snapshots().collect::<Vec<_>>().into();
        // Seal every pending typed batch before external ordering starts. Keeping the other
        // output writers alive here otherwise retains unrelated rich rows during each sort.
        let pending = writers
            .into_values()
            .map(|writer| writer.close(&self.workspace.model, self.workspace.budget()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut completed = Vec::new();
        for pending in pending {
            completed.push(
                self.workspace
                    .order(
                        self.name,
                        self.implementation,
                        self.configuration,
                        pending,
                        input_snapshots.clone(),
                        self.profile,
                    )
                    .await?,
            );
        }
        self.workspace.cancellation.check()?;
        let mut visible = self.workspace.completed.lock().map_err(|_| poisoned())?;
        for source in completed {
            visible.insert(source.name(), source);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct WorkspacePool {
    memory: Arc<dyn MemoryPool>,
    limit: usize,
}
#[derive(Debug)]
struct WorkspaceReservation {
    reservation: MemoryReservation,
    owner: &'static str,
    pool: Arc<dyn MemoryPool>,
    limit: usize,
}
impl ResourcePool for WorkspacePool {
    fn reserve(
        &self,
        owner: &'static str,
        bytes: usize,
    ) -> Result<Box<dyn Reservation>, ModelError> {
        let mut reserved = WorkspaceReservation {
            reservation: MemoryConsumer::new(owner).register(&self.memory),
            owner,
            pool: self.memory.clone(),
            limit: self.limit,
        };
        reserved.try_resize(bytes)?;
        Ok(Box::new(reserved))
    }
    fn reserved(&self) -> usize {
        self.memory.reserved()
    }
    fn limit(&self) -> usize {
        self.limit
    }
}
impl Reservation for WorkspaceReservation {
    fn size(&self) -> usize {
        self.reservation.size()
    }
    fn try_resize(&mut self, bytes: usize) -> Result<(), ModelError> {
        self.reservation
            .try_resize(bytes)
            .map_err(|_| ModelError::Resource {
                owner: self.owner,
                requested: bytes.saturating_sub(self.reservation.size()),
                used: self.pool.reserved(),
                limit: self.limit,
            })
    }
}

impl cpg_extract::bundle::ProviderSink for ProducerOutput {
    fn read<R: Record>(
        &self,
    ) -> Result<Box<dyn Iterator<Item = Result<Batch<R>, ModelError>> + Send>, ModelError> {
        self.check()?;
        Ok(Box::new(self.inputs.relation::<R>()?.read::<R>(
            self.workspace.model.clone(),
            self.workspace.budget.clone(),
        )?))
    }
    fn declare<R: Record>(&self) -> Result<(), ModelError> {
        ProducerOutput::declare::<R>(self)
    }
    fn write<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        ProducerOutput::write(self, &batch)
    }
    fn contribute<R: Record>(&self, batch: Batch<R>) -> Result<(), ModelError> {
        ProducerOutput::contribute(self, &batch)
    }
    fn finish(&self, outcome: ProviderOutcome) -> Result<(), ModelError> {
        self.mark_finished(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lctx_model::domain::input::Package;
    fn model() -> Arc<ValidatedModel> {
        Arc::new(ValidatedModel::declared(vec![Relation::of::<Package>()]).unwrap())
    }
    async fn packages(memory: usize, batch: usize, reverse: bool) -> Arc<CompletedRelation> {
        let workspace = Workspace::new(
            model(),
            WorkspaceOptions {
                memory_bytes: memory,
                partitions: 1,
                batch_rows: batch,
            },
        )
        .unwrap();
        let inputs = workspace.inputs("packages", Profile::Catalog, []).unwrap();
        let output = workspace.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            inputs,
        );
        output.declare::<Package>().unwrap();
        let rows: Vec<_> = (0..20000)
            .map(|n| Package {
                name: format!("package-{n:08}"),
            })
            .collect();
        if reverse {
            for row in rows.into_iter().rev() {
                output.push(row).await.unwrap();
            }
        } else {
            for row in rows {
                output.push(row).await.unwrap();
            }
        }
        output.finish(ProviderOutcome::Complete).await.unwrap();
        workspace.completed::<Package>().unwrap()
    }
    #[tokio::test]
    async fn canonical_stream_is_independent_of_batching_order_and_memory_limit() {
        let ordinary = packages(64 << 20, 4096, false).await;
        let constrained = packages(2 << 20, 128, true).await;
        assert_eq!(ordinary.content(), constrained.content());
        assert_eq!(constrained.rows(), 20000);
        let mut last = None;
        let mut rows = 0;
        for batch in constrained.batches().unwrap() {
            for row in Package::decode(&batch.unwrap()).unwrap() {
                let id = row.id();
                assert!(last.is_none_or(|previous| previous < id));
                last = Some(id);
                rows += 1;
            }
        }
        assert_eq!(rows, 20000);
    }
    #[tokio::test]
    async fn cancellation_and_incomplete_output_never_become_completed() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default()).unwrap();
        let output = workspace.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("packages", Profile::Catalog, []).unwrap(),
        );
        output.declare::<Package>().unwrap();
        output
            .push(Package {
                name: "private".into(),
            })
            .await
            .unwrap();
        assert!(workspace.completed::<Package>().is_err());
        workspace.cancellation().cancel();
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(workspace.completed::<Package>().is_err());
    }
    #[tokio::test]
    async fn omitted_declared_outputs_and_ignored_row_refusals_never_complete() {
        use lctx_model::domain::stages::{Effect, RelationUse, Stage};
        let workspace = Workspace::new(model(), WorkspaceOptions::default()).unwrap();
        let declaration = Stage {
            name: "packages",
            inputs: vec![],
            outputs: vec![RelationUse::of::<Package>()],
            contributes: vec![],
            coverage: vec![],
            profiles: vec![Profile::Catalog],
            effect: Effect::Pure,
            code: ContentHash::of(b"fixture"),
            configuration: ContentHash::of(b"fixture"),
        };
        let output = workspace.producer(
            &declaration,
            Profile::Catalog,
            workspace.inputs("packages", Profile::Catalog, []).unwrap(),
        );
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(workspace.completed::<Package>().is_err());
        let budget = ResourceBudget::fixed(4096).unwrap();
        let constrained = Workspace::with_budget(
            model(),
            WorkspaceOptions {
                memory_bytes: 4096,
                ..Default::default()
            },
            budget,
        )
        .unwrap();
        let output = constrained.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            constrained
                .inputs("packages", Profile::Catalog, [])
                .unwrap(),
        );
        output.declare::<Package>().unwrap();
        assert!(
            output
                .push(Package {
                    name: "x".repeat(5000)
                })
                .await
                .is_err()
        );
        assert!(output.finish(ProviderOutcome::Complete).await.is_err());
        assert!(constrained.completed::<Package>().is_err());
    }
    #[tokio::test]
    async fn conflicting_payload_refuses_the_whole_output_set() {
        use lctx_model::domain::{input::InputRevision, source::SourceArtifact};
        let workspace = Workspace::new(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions::default(),
        )
        .unwrap();
        let output = workspace.output(
            "source",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("source", Profile::Catalog, []).unwrap(),
        );
        output.declare::<Package>().unwrap();
        output.declare::<SourceArtifact>().unwrap();
        output
            .push(Package {
                name: "private".into(),
            })
            .await
            .unwrap();
        let first = SourceArtifact {
            input: InputRevision {
                manifest: ContentHash::of(b"manifest"),
            }
            .id(),
            path: "module.py".into(),
            content: ContentHash::of(b"x"),
            byte_len: 1,
        };
        let mut second = first.clone();
        second.byte_len = 2;
        output.push(first).await.unwrap();
        output.push(second).await.unwrap();
        assert!(matches!(
            output.finish(ProviderOutcome::Complete).await,
            Err(ModelError::Conflict(_))
        ));
        assert!(workspace.completed::<Package>().is_err());
        assert!(workspace.completed::<SourceArtifact>().is_err());
    }
    #[tokio::test]
    async fn ordinary_owner_adopts_native_contributions_then_refuses_another_owner() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default()).unwrap();
        let contributed = workspace.output(
            "native",
            Profile::Catalog,
            ContentHash::of(b"native"),
            workspace.inputs("native", Profile::Catalog, []).unwrap(),
        );
        let batch = Batch::new(
            workspace.model(),
            vec![Package {
                name: "package".into(),
            }],
            workspace.budget(),
        )
        .unwrap();
        contributed.contribute(&batch).unwrap();
        drop(batch);
        contributed.finish(ProviderOutcome::Complete).await.unwrap();
        let owner = workspace.output(
            "assembly",
            Profile::Catalog,
            ContentHash::of(b"assembly"),
            workspace.inputs("assembly", Profile::Catalog, []).unwrap(),
        );
        owner.declare::<Package>().unwrap();
        owner.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 1);
        let duplicate = workspace.output(
            "duplicate",
            Profile::Catalog,
            ContentHash::of(b"duplicate"),
            workspace.inputs("duplicate", Profile::Catalog, []).unwrap(),
        );
        duplicate.declare::<Package>().unwrap();
        assert!(duplicate.finish(ProviderOutcome::Complete).await.is_err());
        assert_eq!(
            workspace.completed::<Package>().unwrap().producer(),
            "assembly"
        );
    }
    #[tokio::test]
    async fn nominal_reference_closure_reuses_completed_streams_and_refuses_missing_targets() {
        use lctx_model::domain::input::Release;
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        for valid in [true, false] {
            let workspace = Workspace::new(model.clone(), WorkspaceOptions::default()).unwrap();
            let output = workspace.output(
                "release",
                Profile::Catalog,
                ContentHash::of(b"fixture"),
                workspace.inputs("release", Profile::Catalog, []).unwrap(),
            );
            output.declare::<Package>().unwrap();
            output.declare::<Release>().unwrap();
            let package = Package {
                name: "package".into(),
            };
            let reference = if valid {
                package.id()
            } else {
                Package {
                    name: "absent".into(),
                }
                .id()
            };
            output.push(package).await.unwrap();
            output
                .push(Release {
                    package: reference,
                    version: "1.0".into(),
                })
                .await
                .unwrap();
            output.finish(ProviderOutcome::Complete).await.unwrap();
            let content = workspace.content().unwrap();
            let admitted = workspace.validate().await;
            if valid {
                assert_eq!(admitted.unwrap(), content);
            } else {
                assert!(admitted.is_err());
            }
        }
    }
    #[tokio::test]
    async fn completed_input_keeps_files_alive_and_later_contributions_do_not_change_it() {
        let workspace = Workspace::new(model(), WorkspaceOptions::default()).unwrap();
        let output = workspace.output(
            "packages",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("packages", Profile::Catalog, []).unwrap(),
        );
        output.declare::<Package>().unwrap();
        output.push(Package { name: "one".into() }).await.unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let original = workspace.completed::<Package>().unwrap();
        let bound = workspace
            .inputs("reader", Profile::Catalog, [Package::NAME])
            .unwrap();
        let next = workspace.output(
            "more",
            Profile::Catalog,
            ContentHash::of(b"fixture"),
            workspace.inputs("more", Profile::Catalog, []).unwrap(),
        );
        let batch = Batch::new(
            workspace.model(),
            vec![Package { name: "two".into() }],
            workspace.budget(),
        )
        .unwrap();
        next.contribute(&batch).unwrap();
        drop(batch);
        next.finish(ProviderOutcome::Complete).await.unwrap();
        assert_eq!(
            bound.relation::<Package>().unwrap().content(),
            original.content()
        );
        assert_eq!(original.rows(), 1);
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 2);
        let path = original.path().to_path_buf();
        drop(bound);
        drop(workspace);
        assert!(path.exists());
        assert_eq!(
            original
                .batches()
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .num_rows(),
            1
        );
        drop(original);
        assert!(!path.exists());
    }
}

#[derive(Debug)]
struct BudgetMemoryPool {
    budget: ResourceBudget,
    allocated: Mutex<Box<dyn Reservation>>,
}
impl std::fmt::Display for BudgetMemoryPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shared compiler allocation pool")
    }
}
impl MemoryPool for BudgetMemoryPool {
    fn name(&self) -> &str {
        "CompilerResourcePool"
    }
    fn grow(&self, reservation: &MemoryReservation, bytes: usize) {
        self.try_grow(reservation, bytes)
            .expect("infallible DataFusion allocation must already fit");
    }
    fn shrink(&self, _: &MemoryReservation, bytes: usize) {
        let mut allocated = self.allocated.lock().expect("memory pool poisoned");
        let size = allocated.size();
        allocated
            .try_resize(size - bytes)
            .expect("returning reservation");
    }
    fn try_grow(&self, _: &MemoryReservation, bytes: usize) -> datafusion::error::Result<()> {
        let mut allocated = self.allocated.lock().map_err(|_| {
            datafusion::error::DataFusionError::Internal("memory pool poisoned".into())
        })?;
        let size = allocated.size();
        allocated
            .try_resize(size.saturating_add(bytes))
            .map_err(|e| datafusion::error::DataFusionError::ResourcesExhausted(e.to_string()))
    }
    fn reserved(&self) -> usize {
        self.budget.reserved()
    }
    fn memory_limit(&self) -> datafusion::execution::memory_pool::MemoryLimit {
        datafusion::execution::memory_pool::MemoryLimit::Finite(self.budget.limit())
    }
}

#[cfg(test)]
mod empty_execution_admission_controls {
    use super::*;
    use lctx_model::domain::execution::source_call_records::SourceCallHeader;
    #[tokio::test]
    async fn complete_empty_execution_root_needs_no_unrequested_parent_streams() {
        let workspace = Workspace::new(
            Arc::new(lctx_model::domain::model().unwrap()),
            WorkspaceOptions::default(),
        )
        .unwrap();
        let inputs = workspace
            .inputs("empty-source-headers", Profile::Catalog, [])
            .unwrap();
        let output = workspace.output(
            "empty-source-headers",
            Profile::Catalog,
            ContentHash::of(b"empty-header-control"),
            inputs,
        );
        output.declare::<SourceCallHeader>().unwrap();
        output.finish(ProviderOutcome::Complete).await.unwrap();
        let selected = workspace
            .inputs(
                "empty-header-consumer",
                Profile::Catalog,
                [SourceCallHeader::NAME],
            )
            .unwrap();
        workspace.checked_inputs(&selected).await.unwrap();
        assert_eq!(workspace.completed::<SourceCallHeader>().unwrap().rows(), 0);
    }
}
