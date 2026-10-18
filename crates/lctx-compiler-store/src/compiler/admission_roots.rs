//! Explicit operation-owned selection prerequisites for physical completion.
use super::*;

impl NativeCompilerStore {
    /// Retain exact registered views needed by this producing operation's semantic
    /// admission. These are selection requests, not installed capabilities. The
    /// normal physical completion independently earns their rows, copies and holds.
    /// The mutation lease makes registration atomic with the content-freeze cut.
    pub async fn retain_admission_views(
        self: &Arc<Self>,
        views: Vec<CompletedView>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let lease = self.admit_mutation("admission selection roots", true)?;
        let result = lease
            .within(async {
                self.check_failed()?;
                {
                    let retained = self
                        .admission_views
                        .lock()
                        .map_err(|_| ModelError::Conflict("native admission selection roots"))?;
                    if retained
                        .values()
                        .any(|(_, grant)| !grant.shares_pool(budget))
                    {
                        return Err(ModelError::Conflict(
                            "native admission selection root foreign resource pool",
                        ));
                    }
                }
                let mut pending = Vec::new();
                for view in views {
                    self.registered_view(&view).await?;
                    let grant = budget.reserve(
                        "native admission selection root",
                        lctx_native::installed_view::view_heap(&view)
                            .saturating_mul(2)
                            .saturating_add(512),
                    )?;
                    pending.push((view, grant));
                }
                let mut retained = self
                    .admission_views
                    .lock()
                    .map_err(|_| ModelError::Conflict("native admission selection roots"))?;
                // Every fallible check precedes publication of the complete local set.
                for (view, grant) in &pending {
                    if !grant.shares_pool(budget)
                        || retained
                            .get(&view.identity)
                            .is_some_and(|(known, own)| known != view || !own.shares_pool(budget))
                    {
                        return Err(ModelError::Conflict(
                            "native admission selection root collision or pool",
                        ));
                    }
                }
                for (view, grant) in pending {
                    retained.entry(view.identity).or_insert((view, grant));
                }
                Ok(())
            })
            .await;
        lease.finish_with(result)
    }
}
