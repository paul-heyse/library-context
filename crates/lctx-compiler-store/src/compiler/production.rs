//! Earned singleton completion and effect-free optional retention preparation.
use super::*;

/// Exact immutable producer result earned by this store, not a caller supplied DTO.
/// Its backing and source survive until every completion/retention consumer releases it.
pub struct CompletedProduction {
    pub(super) store: Arc<NativeCompilerStore>,
    pub(super) physical: ContentHash,
    pub(super) descriptor: CompletedContribution,
    pub(super) outputs: BTreeMap<String, CompletedView>,
    pub(super) budget: ResourceBudget,
    pub(super) _charge: Box<dyn Reservation>,
}
pub(super) fn production_bytes(
    descriptor: &CompletedContribution,
    outputs: &BTreeMap<String, CompletedView>,
) -> usize {
    size_of::<CompletedProduction>()
        .saturating_add(lctx_native::installed_view::contribution_heap(descriptor))
        .saturating_add(
            outputs
                .iter()
                .map(|(name, view)| {
                    size_of::<(String, CompletedView)>()
                        + 32
                        + name.capacity()
                        + lctx_native::installed_view::view_heap(view)
                })
                .fold(0usize, usize::saturating_add),
        )
}
impl CompletedProduction {
    pub fn descriptor(&self) -> &CompletedContribution {
        &self.descriptor
    }
    pub fn physical(&self) -> ContentHash {
        self.physical
    }
    pub fn outputs(&self) -> &BTreeMap<String, CompletedView> {
        &self.outputs
    }
    fn require(
        &self,
        store: &Arc<NativeCompilerStore>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        if !Arc::ptr_eq(&self.store, store) || !self.budget.shares_pool(budget) {
            return Err(ModelError::Conflict(
                "completed production source/resource context",
            ));
        }
        store.check_failed()
    }
}

/// Private exact composition over one captured predecessor. The original completed
/// production and admitted backing survive preparation, rebase and host publication.
/// This is not a new persisted selector or authority supplied by a caller.
pub struct PreparedProductionView {
    predecessor: Option<CompletedView>,
    view: CompletedView,
    _production: Arc<CompletedProduction>,
    mappings: SelectedViewMappings,
    _mapping_charge: d::charged::StateCharge,
    _charge: Box<dyn Reservation>,
}
impl PreparedProductionView {
    pub fn predecessor(&self) -> Option<&CompletedView> {
        self.predecessor.as_ref()
    }
    pub fn view(&self) -> &CompletedView {
        &self.view
    }
}

/// A decline has no native request, reader, pin or mutation admission behind it.
pub enum RetentionPreparation {
    Declined,
    Ready(PreparedRetention),
}
pub struct PreparedRetention {
    production: Arc<CompletedProduction>,
    request: ContentHash,
    pending: Vec<CompletedView>,
    charge: d::charged::StateCharge,
    _request: tokio::sync::OwnedMutexGuard<()>,
}
impl NativeCompilerStore {
    pub async fn complete_singleton_scoped(
        self: &Arc<Self>,
        scope: &ProducingScope,
        id: ContentHash,
        outcome: ProviderOutcome,
        outputs: &[Relation],
        budget: &ResourceBudget,
    ) -> Result<Arc<CompletedProduction>, ModelError> {
        if !scope.belongs_to(self) || scope.contribution != id {
            return Err(ModelError::Conflict(
                "native producing contribution binding",
            ));
        }
        scope.close_and_wait().await?;
        let lease = self.admit_mutation("complete producer singleton", true)?;
        let result = lease
            .within(self.complete_production_inner(id, outcome, outputs, &BTreeMap::new(), budget))
            .await;
        lease.finish_with(result).map(Arc::new)
    }

    /// Prepare one exact immutable union without the host visibility gate. The
    /// caller must compare its predecessor again under that gate before binding.
    /// Failure remains terminal: submitted native work and Unknowns are never replayed.
    pub async fn prepare_production_view(
        self: &Arc<Self>,
        production: Arc<CompletedProduction>,
        relation: String,
        previous: Option<CompletedView>,
        budget: &ResourceBudget,
    ) -> Result<PreparedProductionView, ModelError> {
        production.require(self, budget)?;
        let singleton = production
            .outputs
            .get(&relation)
            .ok_or(ModelError::Schema("completed production output relation"))?;
        let lease = self.admit_mutation("prepare completed producer visibility", true)?;
        let result = lease
            .within(async {
                let contributors_count = singleton
                    .contributions
                    .len()
                    .saturating_add(previous.as_ref().map_or(0, |view| view.contributions.len()));
                let charge = budget.reserve(
                    "composed production view backing",
                    size_of::<PreparedProductionView>()
                        + 512
                        + relation.len().saturating_mul(4)
                        + contributors_count.saturating_mul(192),
                )?;
                let mut contributors = singleton.contributions.clone();
                if let Some(prior) = &previous {
                    if prior.relation != relation {
                        return Err(ModelError::Conflict("previous completed view relation"));
                    }
                    self.registered_view(prior).await?;
                    contributors.extend(prior.contributions.iter().copied());
                }
                let view = if contributors == singleton.contributions {
                    singleton.clone()
                } else {
                    let candidate = CompletedView::new(relation, contributors, 0)?;
                    let rows = self.count_completed_memberships(&candidate, budget).await?;
                    CompletedView::new(candidate.relation, candidate.contributions, rows)?
                };
                let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
                let reader = if let Some(flag) = self.read_cancellation()? {
                    reader.with_read_cancellation(flag)
                } else {
                    reader
                };
                let mut observed = BTreeMap::new();
                let result = self
                    .selected_view_mappings_read(&reader, &view, budget, &mut observed, false)
                    .await;
                let mut terminal = d::completion::Completion::default();
                terminal.step(
                    "private composition mapping reader close",
                    reader.close().await,
                );
                let (mappings, mapping_charge) = d::completion::complete(result, terminal)?;
                Ok((view, mappings, mapping_charge, charge))
            })
            .await;
        let (view, mappings, mapping_charge, charge) = lease.finish_with(result)?;
        Ok(PreparedProductionView {
            predecessor: previous,
            view,
            _production: production,
            mappings,
            _mapping_charge: mapping_charge,
            _charge: charge,
        })
    }

    /// Publish only after the host has rechecked the exact predecessor. Private
    /// mapping discovery has no persistent declaration/hold/registry side effects.
    pub async fn publish_prepared_production_view(
        self: &Arc<Self>,
        prepared: PreparedProductionView,
        binding: CompletedBinding,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        prepared._production.require(self, budget)?;
        if binding.view != prepared.view {
            return Err(ModelError::Conflict("prepared production binding view"));
        }
        let lease = self.admit_mutation("publish prepared producer visibility", true)?;
        let result = lease
            .within(async {
                let _gate = self.root_gates([prepared.view.identity], budget).await?;
                self.declare_view_window(&[(&prepared.view, prepared.mappings.as_slice())], budget)
                    .await?;
                self.remember_selected_view_mapping(&prepared.view, &prepared.mappings)?;
                self.remember_view(&prepared.view)?;
                self.bind_inner(binding).await
            })
            .await;
        lease.finish_with(result)
    }

    /// Only local reservations and the local request gate occur here. In particular,
    /// replay is deliberately below the mandatory native-admission cutoff.
    pub async fn prepare_product_retention(
        self: &Arc<Self>,
        request: ContentHash,
        production: Arc<CompletedProduction>,
        budget: &ResourceBudget,
    ) -> Result<RetentionPreparation, ModelError> {
        production.require(self, budget)?;
        let prepare = async {
            let gate = self.product_retention_gate(request, budget)?;
            let guard = gate.lock_owned().await;
            self.check_failed()?;
            let mut charge =
                d::charged::StateCharge::new(budget, "retained product selector inventory");
            // Charge copies before constructing them. No lookup or protection is needed:
            // this exact result already belongs to the required completed producer.
            charge.grow(
                production
                    .outputs
                    .values()
                    .map(|view| {
                        size_of::<CompletedView>() + lctx_native::installed_view::view_heap(view)
                    })
                    .fold(0usize, usize::saturating_add),
            )?;
            let mut pending = Vec::with_capacity(production.outputs.len());
            pending.extend(production.outputs.values().cloned());
            Ok(PreparedRetention {
                production,
                request,
                pending,
                charge,
                _request: guard,
            })
        }
        .await;
        match prepare {
            Err(ModelError::Resource { .. } | ModelError::Limit { .. }) => {
                Ok(RetentionPreparation::Declined)
            }
            result => result.map(RetentionPreparation::Ready),
        }
    }
    pub(super) fn product_retention_gate(
        &self,
        request: ContentHash,
        budget: &ResourceBudget,
    ) -> Result<Arc<tokio::sync::Mutex<()>>, ModelError> {
        let mut gates = self
            .product_retention_gates
            .lock()
            .map_err(|_| ModelError::Conflict("product retention gate registry"))?;
        if let Some(entry) = gates.get(&request) {
            return Ok(entry.gate.clone());
        }
        let charge = Arc::new(budget.reserve("product retention request gate", 1024)?);
        let gate = Arc::new(tokio::sync::Mutex::new(()));
        gates.insert(
            request,
            RootInstallationGate {
                gate: gate.clone(),
                _charge: charge,
            },
        );
        Ok(gate)
    }
    pub async fn retain_prepared_product(
        self: &Arc<Self>,
        prepared: PreparedRetention,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        prepared.production.require(self, budget)?;
        let lease = self.admit_mutation("retain prepared independent product boundary", true)?;
        let result = lease
            .within(async {
                let PreparedRetention {
                    production,
                    request,
                    pending,
                    charge,
                    _request,
                } = prepared;
                // The first native request (including a replay read/pin) is mandatory.
                self.retain_product_effects(request, production.physical, pending, charge, budget)
                    .await
            })
            .await;
        lease.finish_with(result)
    }
}

impl NativeCompilerStore {
    pub(super) async fn retain_product_effects(
        self: &Arc<Self>,
        request: ContentHash,
        contribution: ContentHash,
        mut pending: Vec<CompletedView>,
        mut charge: d::charged::StateCharge,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let phase = crate::phase::Phase::begin("native_product_retention");
        let result = async {
            self.check_failed()?;
            if lctx_native::completed_state::check_retained_product_replay(
                self.client.clone(),
                self.attempt,
                request,
                contribution,
                budget,
            )
            .await?
            {
                return Ok(());
            }
            let mut selections = BTreeMap::new();
            let mut mapping_charges = Vec::new();
            let mapping_reader =
                crate::NativeReader::private(self.client.clone()).with_budget(budget);
            let mapping_reader = if let Some(flag) = self.read_cancellation()? {
                mapping_reader.with_read_cancellation(flag)
            } else {
                mapping_reader
            };
            let mut observed = BTreeMap::new();
            let mapping_phase = crate::phase::Phase::begin("native_product_mapping_discovery");
            let mapping_result = async {
                while let Some(view) = pending.pop() {
                    if selections.contains_key(&view.identity) {
                        continue;
                    }
                    self.registered_view(&view).await?;
                    let (mappings, mapping_charge) = self
                        .selected_view_mappings_read(
                            &mapping_reader,
                            &view,
                            budget,
                            &mut observed,
                            false,
                        )
                        .await?;
                    for mapping in mappings.as_slice() {
                        if mapping.view != view.identity && !selections.contains_key(&mapping.view)
                        {
                            let dependency = {
                                let views = self.known_views.lock().map_err(|_| {
                                    ModelError::Conflict("retained product dependency inventory")
                                })?;
                                let dependency =
                                    views.get(&mapping.view).ok_or(ModelError::Conflict(
                                        "retained product dependency descriptor absent",
                                    ))?;
                                charge.grow(
                                    size_of::<CompletedView>()
                                        + 1024
                                        + lctx_native::installed_view::view_heap(dependency),
                                )?;
                                dependency.clone()
                            };
                            pending.push(dependency);
                        }
                    }
                    mapping_charges.push(mapping_charge);
                    charge.grow(2048usize.saturating_add(mappings.len().saturating_mul(
                        size_of::<lctx_native::installed_view::ContributorRealization>(),
                    )))?;
                    selections.insert(
                        view.identity,
                        lctx_native::completed_state::ViewSelection {
                            view,
                            mappings: mappings.as_slice().to_vec(),
                        },
                    );
                }
                Ok(())
            }
            .await;
            mapping_phase.finish_result(&mapping_result);
            let mut mapping_terminal = d::completion::Completion::default();
            mapping_terminal.step(
                "retained product mapping reader close",
                mapping_reader.close().await,
            );
            d::completion::complete(mapping_result, mapping_terminal)?;
            drop(observed);
            let selections = selections.into_values().collect::<Vec<_>>();
            let mut scope = self.view_audit.lock().await;
            if scope.is_none() {
                *scope = Some(
                    lctx_native::installed_view::InstallationAudit::new(
                        self.client.clone(),
                        budget,
                    )
                    .await?,
                );
            }
            let audit = scope
                .as_ref()
                .filter(|audit| audit.shares_pool(budget))
                .cloned();
            drop(scope);
            let boundary = match audit {
                Some(audit) => {
                    let source = audit.protected_source(&self.client, budget).await?;
                    lctx_native::completed_state::complete_retained_content_from_source(
                        self.client.clone(),
                        self.attempt,
                        &[],
                        &selections,
                        budget,
                        &source,
                    )
                    .await?
                }
                None => {
                    lctx_native::completed_state::complete_retained_content(
                        self.client.clone(),
                        self.attempt,
                        &[],
                        &selections,
                        budget,
                    )
                    .await?
                }
            };
            let product = RecordId::new(
                "native_product",
                format!("{}_{}", request.hex(), self.attempt.hex()),
            );
            let mut row = Object::new();
            row.insert("id", product.clone());
            row.insert("request", request.hex());
            row.insert("released", false);
            row.insert(
                "contribution",
                RecordId::new("compiler_contribution", contribution.hex()),
            );
            crate::control::ensure_rows(&self.client, Some(self.attempt), vec![Value::Object(row)])
                .await?;
            let reader = crate::NativeReader::private(self.client.clone()).with_budget(budget);
            let result = async {
                let address = boundary.completion().record()?;
                let mut observed = crate::control::inspect_expected_window_read(
                    &reader,
                    std::slice::from_ref(&address),
                    budget,
                )
                .await?;
                let expected = observed.take_required(&address)?;
                observed.finish()?;
                if expected.incarnation() != boundary.incarnation()
                    || expected.row() != boundary.completion().value(true)?
                {
                    return Err(ModelError::Conflict(
                        "retained product completed boundary changed",
                    ));
                }
                crate::control::hold_expected(
                    &self.client,
                    self.attempt,
                    product.clone(),
                    expected,
                )
                .await?;
                crate::control::hold_many(
                    &self.client,
                    Some(self.attempt),
                    vec![
                        (
                            product.clone(),
                            RecordId::new("compiler_contribution", contribution.hex()),
                        ),
                        (RecordId::new("native_attempt", self.attempt.hex()), product),
                    ],
                )
                .await
            }
            .await;
            let mut terminal = d::completion::Completion::default();
            let closed = reader.close().await;
            if closed.is_err()
                || result
                    .as_ref()
                    .err()
                    .is_some_and(|error: &ModelError| !error.permits_storage_cleanup())
            {
                boundary.retain_unknown();
            }
            terminal.step("retained product receiving reader close", closed);
            terminal.step(
                "retained product independent boundary pin close",
                boundary.close().await,
            );
            d::completion::complete(result, terminal)
        }
        .await;
        phase.finish_result(&result);
        result
    }
}
