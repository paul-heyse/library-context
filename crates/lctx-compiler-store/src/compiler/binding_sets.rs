//! Complete mutable binding inventories use one owned operation and bounded pages.
use super::*;

pub(super) struct BindingMutation<'a> {
    store: &'a NativeCompilerStore,
    armed: bool,
}
impl<'a> BindingMutation<'a> {
    pub(super) fn new(store: &'a NativeCompilerStore) -> Self {
        Self { store, armed: true }
    }
    pub(super) fn finish(&mut self, successful: bool) {
        self.armed = !successful;
    }
}
impl Drop for BindingMutation<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.store.fail();
        }
    }
}
fn descriptor_bytes(binding: &CompletedBinding) -> Result<usize, ModelError> {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("binding size overflow"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, binding).map_err(ModelError::codec)?;
    Ok(count.0)
}

impl NativeCompilerStore {
    /// Install a complete caller-selected binding set. Readers of this attempt do not
    /// observe its page prefix. A failed prefix poisons the attempt and retains its
    /// genuine operation root for recovery; it cannot become completed content.
    pub async fn bind_set(
        self: &Arc<Self>,
        bindings: Vec<CompletedBinding>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let lease = self.admit_mutation("bind_set", true)?;
        let _set = self.binding_sets.write().await;
        let mut mutation = BindingMutation::new(self);
        let result = lease.within(self.bind_set_inner(bindings, budget)).await;
        let result = lease.finish_with(result);
        if result.is_err() {
            self.fail();
        }
        mutation.finish(result.is_ok());
        result
    }

    async fn bind_set_inner(
        self: &Arc<Self>,
        bindings: Vec<CompletedBinding>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.check_failed()?;
        let mut inventory = d::charged::StateCharge::new(budget, "binding set inventory");
        let mut keys = BTreeSet::new();
        // Validate the whole set and its actual immutable dependencies before effects.
        for binding in &bindings {
            binding.validate()?;
            inventory.grow(
                descriptor_bytes(binding)?
                    .saturating_mul(2)
                    .saturating_add(512),
            )?;
            if !keys.insert(binding.key()) {
                return Err(ModelError::Conflict("duplicate native binding set key"));
            }
            self.registered_view(&binding.view).await?;
        }
        if bindings.is_empty() {
            return Ok(());
        }
        let operation = Arc::new(crate::control::operation_pages::OperationWriter::new(
            crate::control::RecoveryOwner::OpenAttempt(self.attempt),
            None,
            budget,
        )?);
        let mut writer =
            crate::control::WindowWriter::new(&self.client, Some(self.attempt), Some(budget))
                .with_operation(operation.clone());
        let owner = RecordId::new("native_attempt", self.attempt.hex());
        let mut page = Vec::new();
        let mut bytes = 0usize;
        let mut page_charges = Vec::new();
        for binding in bindings {
            // Count without constructing encoded output, then admit every container,
            // projection and control expansion before allocating the physical row.
            let expanded = descriptor_bytes(&binding)?
                .saturating_mul(64)
                .saturating_add(16384);
            let construction = match budget.reserve("native binding set row", expanded) {
                Err(ModelError::Resource { .. }) if !page.is_empty() => {
                    Self::install_binding_page(&mut writer, &owner, std::mem::take(&mut page))
                        .await?;
                    page_charges.clear();
                    bytes = 0;
                    budget.reserve("native binding set row", expanded)?
                }
                result => result?,
            };
            let mut row = Object::new();
            row.insert(
                "id",
                RecordId::new(
                    "compiler_binding",
                    attempt_binding(self.attempt, binding.key()).hex(),
                ),
            );
            row.insert(
                "descriptor",
                Bytes::from(serde_json::to_vec(&binding).map_err(ModelError::codec)?),
            );
            row.insert("attempt", owner.clone());
            for (field, value) in binding_projection(&binding) {
                row.insert(field, value);
            }
            let row = Value::Object(row);
            let entry = (
                row,
                binding.boundary.is_some(),
                RecordId::new("compiler_view", binding.view.identity.hex()),
            );
            let (objects, references, variables) = binding_page_plan(&owner, vec![entry.clone()])?;
            let (wire, controls) = writer.guarded_with_holds_wire_bound(
                objects,
                references,
                BINDING_PAGE_EFFECT,
                variables,
            )?;
            if !page.is_empty()
                && (page.len() == crate::loader::NATIVE_WINDOW_ROWS
                    || bytes.saturating_add(wire).saturating_add(controls)
                        > d::resources::TRANSFER_BYTES)
            {
                Self::install_binding_page(&mut writer, &owner, std::mem::take(&mut page)).await?;
                page_charges.clear();
                bytes = 0;
            }
            if wire.saturating_add(controls) > d::resources::TRANSFER_BYTES {
                return Err(ModelError::Limit {
                    owner: "native binding set",
                    limit: "complete page bytes",
                    observed: wire.saturating_add(controls),
                    bound: d::resources::TRANSFER_BYTES,
                });
            }
            bytes = bytes.saturating_add(wire);
            page.push(entry);
            page_charges.push(construction);
        }
        if !page.is_empty() {
            Self::install_binding_page(&mut writer, &owner, page).await?;
        }
        // Exactly one inventory revision belongs to the completed set, rather than
        // one generic effect/recovery envelope for every replaceable slot.
        writer.guarded_with_holds(vec![owner.clone()], Vec::new(),
            "IF $__owner.admitted { THROW 'native binding inventory admitted'; }; UPDATE $__attempt SET revision+=1 RETURN NONE",
            Variables::new()).await?;
        operation.finish(&self.client, budget).await
    }

    async fn install_binding_page(
        writer: &mut crate::control::WindowWriter<'_>,
        owner: &RecordId,
        page: Vec<(Value, bool, RecordId)>,
    ) -> Result<(), ModelError> {
        let (objects, references, variables) = binding_page_plan(owner, page)?;
        writer
            .guarded_with_holds(objects, references, BINDING_PAGE_EFFECT, variables)
            .await
    }
}

const BINDING_PAGE_EFFECT: &str = "IF $__owner.admitted { THROW 'native binding inventory admitted'; }; FOR $binding IN $bindings { LET $record=$binding.row.id; LET $existing=SELECT * FROM ONLY $record FOR UPDATE; IF $binding.frozen AND $existing!=NONE AND $existing!=$binding.row { THROW 'frozen native binding replacement'; }; UPSERT $record CONTENT $binding.row RETURN NONE; }";
fn binding_page_plan(
    owner: &RecordId,
    page: Vec<(Value, bool, RecordId)>,
) -> Result<(Vec<RecordId>, Vec<(RecordId, RecordId)>, Variables), ModelError> {
    let mut objects = Vec::with_capacity(page.len());
    let mut references = Vec::with_capacity(page.len().saturating_mul(2));
    let mut rows = Vec::with_capacity(page.len());
    for (row, frozen, view) in page {
        let Some(Value::RecordId(slot)) = value_object(&row).and_then(|row| row.get("id")) else {
            return Err(ModelError::Schema("native binding set slot"));
        };
        let slot = slot.clone();
        objects.push(slot.clone());
        references.push((owner.clone(), slot));
        references.push((owner.clone(), view));
        let mut entry = Object::new();
        entry.insert("row", row);
        entry.insert("frozen", frozen);
        rows.push(Value::Object(entry));
    }
    let mut variables = Variables::new();
    variables.insert("bindings", rows);
    Ok((objects, references, variables))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;

    #[tokio::test(flavor = "multi_thread")]
    async fn complete_binding_set_uses_one_root_and_preserves_frozen_slots() {
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        let config = RuntimeConfig::read(std::path::Path::new(
            &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned validation runtime"),
        ))
        .unwrap();
        let recovery_budget = ResourceBudget::fixed(128 << 20).unwrap();
        assert!(
            !recovery_budget.shares_pool(&budget),
            "recovery metadata and payload have independent admission"
        );
        let factory = recovery_factory(&config, recovery_budget).unwrap();
        let store = NativeCompilerStore::begin(&config, Frontier::Facts, &factory)
            .await
            .unwrap();
        let mut expected_refusal = false;
        let result = std::panic::AssertUnwindSafe(async {
            let relation = Relation::of::<d::input::Package>();
            let spec = ContributionSpec {
                captured_binding: None,
                producer: format!("binding-set-{}", store.attempt.hex()),
                profile: OutputProfile::Specific(d::stages::Profile::Catalog),
                model: d::model()?.digest(),
                implementation: ContentHash::of(b"binding-set control"),
                configuration: None,
                inputs: Vec::new(),
                outputs: [relation.name().to_owned()].into(),
            };
            let contribution = store.begin_contribution(spec.clone()).await?;
            let views = store
                .complete_contribution(
                    contribution,
                    ProviderOutcome::Complete,
                    std::slice::from_ref(&relation),
                    &BTreeMap::new(),
                    &budget,
                )
                .await?;
            let view = views[relation.name()].clone();
            let bindings = (0..(crate::loader::NATIVE_WINDOW_ROWS + 1))
                .map(|index| {
                    Ok(CompletedBinding {
                        boundary: Some(format!("binding-set-{index}")),
                        source: d::analysis::sources::SourceSnapshot::of_completed_view(
                            &relation, spec.model, &view,
                        )?,
                        view: view.clone(),
                        configuration: None,
                    })
                })
                .collect::<Result<Vec<_>, ModelError>>()?;
            let roots = || async {
                let mut response = store
                    .client
                    .query("SELECT VALUE id FROM native_operation WHERE owner=$owner")
                    .bind((
                        "owner",
                        RecordId::new("native_attempt", store.attempt.hex()),
                    ))
                    .await
                    .map_err(ModelError::codec)?
                    .check()
                    .map_err(ModelError::codec)?;
                response.take::<Vec<RecordId>>(0).map_err(ModelError::codec)
            };
            let before = roots().await?;
            store.bind_set(bindings.clone(), &budget).await?;
            let after = roots().await?;
            let added = after
                .into_iter()
                .filter(|id| !before.contains(id))
                .collect::<Vec<_>>();
            assert_eq!(added.len(), 1, "one root owns the complete multi-page set");
            let mut response = store
                .client
                .query("SELECT committed_prefix,state FROM $root")
                .bind(("root", added[0].clone()))
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            let rows: Vec<Object> = response.take(0).map_err(ModelError::codec)?;
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].get("state"),
                Some(&Value::String("completed".into()))
            );
            let inventory = store.bindings().await?;
            assert_eq!(inventory.len(), bindings.len());
            for expected in &bindings {
                assert!(inventory.contains(expected));
            }
            // Equal frozen slots retain exact content when reused by a later set.
            store.bind_set(bindings[..1].to_vec(), &budget).await?;
            // Force a genuine late-page refusal after a new prefix has committed.
            // Changing configuration preserves the validated view and slot key but
            // must not replace the already frozen complete descriptor.
            let mut tail = bindings[0].clone();
            tail.configuration = Some(ContentHash::of(b"refused frozen replacement"));
            let mut partial = bindings[..crate::loader::NATIVE_WINDOW_ROWS]
                .iter()
                .enumerate()
                .map(|(index, binding)| CompletedBinding {
                    boundary: Some(format!("refused-prefix-{index}")),
                    ..binding.clone()
                })
                .collect::<Vec<_>>();
            let prefix = RecordId::new(
                "compiler_binding",
                attempt_binding(store.attempt, partial[0].key()).hex(),
            );
            let frozen = RecordId::new(
                "compiler_binding",
                attempt_binding(store.attempt, tail.key()).hex(),
            );
            partial.push(tail);
            let error = store.bind_set(partial, &budget).await.unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("frozen native binding replacement"),
                "{error}"
            );
            assert!(
                error.permits_storage_cleanup(),
                "the refused suffix is acknowledged"
            );
            let mut response = store
                .client
                .query("SELECT descriptor FROM ONLY $prefix; SELECT descriptor FROM ONLY $frozen")
                .bind(("prefix", prefix))
                .bind(("frozen", frozen))
                .await
                .map_err(ModelError::codec)?
                .check()
                .map_err(ModelError::codec)?;
            assert_ne!(
                response.take::<Value>(0).map_err(ModelError::codec)?,
                Value::None,
                "a physical prefix actually committed before the refusal"
            );
            let frozen: Value = response.take(1).map_err(ModelError::codec)?;
            assert_eq!(
                frozen.as_object().and_then(|row| row.get("descriptor")),
                Some(&Value::Bytes(Bytes::from(
                    serde_json::to_vec(&bindings[0]).map_err(ModelError::codec)?
                )))
            );
            assert!(
                store.bindings().await.is_err(),
                "failed prefix cannot be a successful inventory"
            );
            expected_refusal = true;
            Ok::<_, ModelError>(())
        })
        .catch_unwind()
        .await;
        let mut cleanup = store.drain_finality_report().await;
        cleanup.merge(store.abandon_finality_report().await);
        let mut diagnostics = d::completion::Completion::default();
        store.append_operation_failures(&mut diagnostics);
        if expected_refusal {
            assert!(
                !diagnostics.failures.is_empty(),
                "expected refusal remains diagnostic evidence"
            );
            assert!(
                diagnostics
                    .failures
                    .iter()
                    .all(|failure| failure.error.permits_storage_cleanup())
            );
        } else {
            cleanup.merge(diagnostics);
        }
        let connection = store.client.connection_completion().unwrap();
        let closed = d::completion::complete(Ok::<_, ModelError>(()), cleanup);
        drop(store);
        drop(factory);
        if closed.is_ok() {
            connection.wait().await.unwrap();
        }
        match result {
            Ok(result) => {
                result.unwrap();
                closed.unwrap();
            }
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}
