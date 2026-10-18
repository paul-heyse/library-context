//! Producing-process semantic properties cross only after actual protected native
//! rows match their immutable private inputs. Receiving never acquires this owner.
use super::*;
use crate::private_validation::{
    PrivateCheckInput, PrivateCheckPremise, PrivateExecutedCheck, PrivateValidation,
};

impl AdmissionRuntime {
    /// Attach executed private properties, not a serialized certificate. Each
    /// predicate still earns native correspondence before it can be adopted.
    pub fn attach_private_validation(
        &self,
        private: Arc<PrivateValidation>,
    ) -> Result<(), ModelError> {
        self.require_complete_inventory()?;
        private.check_operation(self.model.digest(), self.budget(), &self.cancellation)?;
        self.native.check()?;
        let mut attached = self.private_validation.lock().map_err(|_| poisoned())?;
        if attached.is_some() {
            return Err(ModelError::Conflict("private validation already attached"));
        }
        *attached = Some(private);
        Ok(())
    }

    pub(in crate::admission) async fn reconcile_private_check(
        &self,
        prepared: &mut PreparedScopedCheck,
        inventory: &ScopedInventory,
    ) -> Result<Option<Arc<PrivateExecutedCheck>>, ModelError> {
        let Some(private) = self
            .private_validation
            .lock()
            .map_err(|_| poisoned())?
            .clone()
        else {
            return Ok(None);
        };
        let _scratch = self.budget.reserve(
            "private predicate native matching workspace",
            prepared.charge.reserved(),
        )?;
        let premise = PrivateCheckPremise {
            model: prepared.premise.model,
            profile: prepared.premise.profile,
            name: prepared.premise.name,
            revision: prepared.premise.revision,
            definition: prepared.premise.definition,
            factory: prepared.premise.factory,
            inputs: prepared
                .premise
                .inputs
                .iter()
                .map(|input| PrivateCheckInput {
                    name: input.name,
                    order: input.order.clone(),
                    requested: input.requested,
                    selected: input
                        .selected
                        .as_ref()
                        .map(|(resolved, _, source)| (*resolved, source.clone())),
                    not_requested: input.not_requested,
                })
                .collect(),
            uniform_owners: prepared.premise.uniform.clone(),
        };
        let Some(check) = private.find_executed(&premise, self.budget(), &self.cancellation)?
        else {
            return Ok(None);
        };
        for binding in check.bindings() {
            let cap = if let Some(cap) = prepared
                .caps
                .iter()
                .find(|cap| cap.completion().view.identity == binding.source.view())
            {
                cap.clone()
            } else {
                // Compact qualification may depend on protected sources beyond
                // this invariant's declared payload roles. Acquire those exact
                // capabilities too; metadata correspondence alone cannot omit them.
                let view = CompletedView::new(
                    binding.relation.name().into(),
                    binding.contributions.clone(),
                    binding.immutable_source().rows(),
                )?;
                if view.identity != binding.source.view() {
                    return Err(ModelError::Conflict(
                        "private qualification exact native view",
                    ));
                }
                let mut acquired = inventory.caps.lock().await;
                let cap = if let Some(cap) = acquired.0.get(&view.identity) {
                    cap.clone()
                } else {
                    acquired.1.grow(128)?;
                    let native = self.native.clone();
                    let budget = self.budget.clone();
                    let identity = view.identity;
                    let cap = self
                        .native_call(async move { native.installed_view(&view, &budget).await })
                        .await?;
                    acquired.0.insert(identity, cap.clone());
                    cap
                };
                prepared
                    .charge
                    .grow(size_of::<Arc<lctx_native::installed_view::HeldInstalledView>>())?;
                prepared.caps.push(cap.clone());
                cap
            };
            self.cancellation.check()?;
            self.native.check()?;
            let native = self.native.clone();
            let retained = cap.clone();
            let budget = self.budget.clone();
            self.native_call(
                async move { native.revalidate_scoped_source(retained, &budget).await },
            )
            .await?;
            let source = binding.immutable_source();
            let mut matched = inventory.private_matches.lock().await;
            if matched
                .0
                .iter()
                .any(|(view, known)| *view == binding.source.view() && Arc::ptr_eq(known, source))
            {
                continue;
            }
            // The actual reader borrows exactly the capability retained by the
            // adopted property, including complete membership/EOF and cleanup.
            let native = self.native.clone();
            let source = source.clone();
            let budget = self.budget.clone();
            let cancellation = self.cancellation.clone();
            let batch_rows = self.options.batch_rows;
            self.native_call(compare_native_rows(
                native,
                source,
                cap,
                budget,
                cancellation,
                batch_rows,
            )?)
            .await?;
            matched.1.grow(size_of::<(
                ContentHash,
                Arc<crate::private_source::PrivateRelation>,
            )>())?;
            matched
                .0
                .push((binding.source.view(), binding.immutable_source().clone()));
        }
        private.check_operation(self.model.digest(), self.budget(), &self.cancellation)?;
        Ok(Some(check))
    }
}

// Erase and charge the actual native reader continuation before composing it
// into admission's predicate loop; its concrete nested setup stays off that frame.
#[inline(never)]
fn compare_native_rows(
    native: Arc<NativeCompilerStore>,
    source: Arc<crate::private_source::PrivateRelation>,
    cap: Arc<lctx_native::installed_view::HeldInstalledView>,
    budget: ResourceBudget,
    cancellation: Cancellation,
    batch_rows: usize,
) -> Result<lctx_native::continuation::Continuation<'static, Result<(), ModelError>>, ModelError> {
    let allocation_budget = budget.clone();
    lctx_native::continuation::admit(
        &allocation_budget,
        "private native correspondence continuation",
        async move {
            let relation = source.relation().clone();
            let columns = relation
                .schema()
                .fields()
                .iter()
                .map(|field| field.name().clone())
                .collect::<Vec<_>>();
            let rows = native
                .scan_rows_with_installed(
                    &cap.completion().view,
                    &relation,
                    Some(&columns),
                    None,
                    &budget,
                    cap.clone(),
                )
                .await?;
            let mut actual = crate::compiler_provider::batches_from_rows(
                rows,
                relation.clone(),
                relation.schema().clone(),
                &budget,
                batch_rows,
            )?;
            let mut correspondence = BatchCorrespondence::new(source.read_batches()?);
            while let Some(batch) = actual.try_next().await.map_err(ModelError::codec)? {
                cancellation.check()?;
                correspondence.visit(&batch)?;
            }
            correspondence.finish()?;
            native.revalidate_scoped_source(cap, &budget).await?;
            cancellation.check()
        },
    )
}

impl PreparedScopedCheck {
    pub(in crate::admission) fn finish_transferred(
        self,
        private: Arc<PrivateExecutedCheck>,
        cancellation: &Cancellation,
    ) -> Result<Arc<ExecutedCheck>, ModelError> {
        cancellation.check()?;
        let execution = private.execution();
        private.check_current(
            self.premise.model,
            self.charge
                .budget()
                .ok_or(ModelError::Conflict("private transfer missing actual pool"))?,
            cancellation,
        )?;
        if private.premise().definition != self.premise.definition
            || private.premise().factory != self.premise.factory
            || private.premise().profile != self.premise.profile
        {
            return Err(ModelError::Conflict(
                "private transfer changed executable context",
            ));
        }
        if (execution == Execution::WholeInput) != private.whole().is_some() {
            return Err(ModelError::Conflict(
                "private transfer execution differs from earned whole ledger",
            ));
        }
        Ok(Arc::new(ExecutedCheck {
            premise: self.premise,
            operation: self.operation,
            caps: self.caps,
            execution,
            _whole: None,
            _private: Some(private),
            _charge: self.charge,
        }))
    }
}

/// Compare complete canonical values, not fingerprints. Different Arrow batch
/// boundaries do not change equality. A mismatch is reported after actual EOF.
struct BatchCorrespondence<I> {
    expected: I,
    current: Option<RecordBatch>,
    offset: usize,
    differs: bool,
}
impl<I: Iterator<Item = Result<RecordBatch, ModelError>>> BatchCorrespondence<I> {
    fn new(expected: I) -> Self {
        Self {
            expected,
            current: None,
            offset: 0,
            differs: false,
        }
    }
    fn advance(&mut self) -> Result<bool, ModelError> {
        while self
            .current
            .as_ref()
            .is_none_or(|batch| self.offset == batch.num_rows())
        {
            self.current = self.expected.next().transpose()?;
            self.offset = 0;
            if self.current.is_none() {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn visit(&mut self, actual: &RecordBatch) -> Result<(), ModelError> {
        let mut offset = 0;
        while offset < actual.num_rows() {
            if !self.advance()? {
                self.differs = true;
                break;
            }
            let expected = self.current.as_ref().expect("advanced private batch");
            let len = (expected.num_rows() - self.offset).min(actual.num_rows() - offset);
            self.differs |= expected.slice(self.offset, len) != actual.slice(offset, len);
            self.offset += len;
            offset += len;
        }
        Ok(())
    }
    fn finish(mut self) -> Result<(), ModelError> {
        if self.advance()? || self.differs {
            return Err(ModelError::Conflict(
                "private/native actual typed correspondence",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use arrow_array::Int64Array;
    use arrow_schema::{DataType, Field, Schema};
    fn batch(values: &[i64]) -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new(
                "value",
                DataType::Int64,
                false,
            )])),
            vec![Arc::new(Int64Array::from(values.to_vec()))],
        )
        .unwrap()
    }
    #[test]
    fn correspondence_compares_actual_values_across_batch_boundaries_and_complete_eof() {
        let expected = || vec![Ok(batch(&[])), Ok(batch(&[1, 2])), Ok(batch(&[3]))].into_iter();
        let mut matching = BatchCorrespondence::new(expected());
        matching.visit(&batch(&[1])).unwrap();
        matching.visit(&batch(&[2, 3])).unwrap();
        matching.finish().unwrap();
        for values in [&[1, 7, 3][..], &[1, 2][..], &[1, 2, 3, 4][..]] {
            let mut changed = BatchCorrespondence::new(expected());
            changed.visit(&batch(values)).unwrap();
            assert!(changed.finish().is_err());
        }
        BatchCorrespondence::new(std::iter::empty())
            .finish()
            .unwrap();
    }
}
