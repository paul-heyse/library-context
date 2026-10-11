//! The one SQL entry point (DESIGN §4.3; review F7): DDL, DML and statements are disallowed, so no
//! compute query can write to a completed input and bypass its compiler owner.

use datafusion::dataframe::DataFrame;
use datafusion::error::Result;
use datafusion::execution::context::SQLOptions;
use datafusion::prelude::SessionContext;

pub fn read_only() -> SQLOptions {
    SQLOptions::new()
        .with_allow_ddl(false)
        .with_allow_dml(false)
        .with_allow_statements(false)
}

/// Plan a read-only query. Any other code path that calls `ctx.sql` is a rule violation
/// (`rules/sql-through-helper.yml`).
pub async fn query(ctx: &SessionContext, sql: &str) -> Result<DataFrame> {
    ctx.sql_with_options(sql, read_only()).await
}

/// Keep provider terminality/resource errors typed across DataFusion execution contexts.
/// A source error cannot become evidence that a disposable product is corrupt.
pub fn model_error(
    error: datafusion::error::DataFusionError,
) -> lctx_model::domain::ModelError {
    use lctx_model::domain::ModelError;
    match error {
        datafusion::error::DataFusionError::External(error) => match error.downcast::<ModelError>()
        {
            Ok(error) => *error,
            Err(error) => ModelError::Cause(error),
        },
        datafusion::error::DataFusionError::Context(_, error) => model_error(*error),
        datafusion::error::DataFusionError::Shared(error) => {
            match std::sync::Arc::try_unwrap(error) {
                Ok(error) => model_error(error),
                Err(error) => ModelError::Cause(Box::new(error)),
            }
        }
        error => ModelError::Cause(Box::new(error)),
    }
}

/// Inspect shared provider errors without losing native uncertainty hidden by wrapper errors.
pub fn product_fallback_allowed(error: &lctx_model::domain::ModelError) -> bool {
    fn allowed(error: &(dyn std::error::Error + 'static)) -> bool {
        use lctx_model::domain::ModelError;
        if let Some(model) = error.downcast_ref::<ModelError>() {
            if !model.permits_storage_cleanup() || model.has_committed_effect() {
                return false;
            }
            if let ModelError::Cause(cause) = model {
                return allowed(cause.as_ref());
            }
            if let ModelError::SharedCause(cause) = model {
                return allowed(cause.as_ref());
            }
        }
        if let Some(datafusion::error::DataFusionError::Collection(errors)) =
            error.downcast_ref::<datafusion::error::DataFusionError>()
        {
            return errors.iter().all(|error| allowed(error));
        }
        error.source().is_none_or(allowed)
    }
    allowed(error)
}

#[cfg(test)]
mod completion_controls {
    use super::*;
    use datafusion::error::DataFusionError;
    use lctx_model::domain::{
        ModelError,
        completion::{self, Completion, LocalState, RemoteState},
    };
    use std::sync::Arc;

    #[test]
    fn shared_execution_context_preserves_native_completion_metadata() {
        let primary = Arc::new(ModelError::Schema("native primary"));
        for has_primary in [true, false] {
            let mut terminal = Completion {
                local: LocalState::Outstanding,
                remote: RemoteState::Unknown,
                ..Default::default()
            };
            terminal.committed("publication", "exact-view");
            terminal.step("drain", Err(ModelError::Schema("cleanup cause")));
            let result = if has_primary {
                Err(ModelError::SharedCause(primary.clone()))
            } else {
                Ok(())
            };
            let shared = Arc::new(DataFusionError::Context(
                "join execution".into(),
                Box::new(DataFusionError::External(Box::new(
                    completion::complete(result, terminal).unwrap_err(),
                ))),
            ));
            // A live sibling holds the same execution error, preventing unwrapping.
            let sibling = shared.clone();
            let wrapped = model_error(DataFusionError::Shared(shared));
            assert_eq!(Arc::strong_count(&sibling), 2);
            assert_eq!(wrapped.primary().is_some(), has_primary);
            if has_primary {
                assert!(std::ptr::eq(wrapped.primary().unwrap(), primary.as_ref()));
            }
            assert!(!wrapped.permits_storage_cleanup());
            assert!(wrapped.has_committed_effect());
            assert!(!product_fallback_allowed(&wrapped));
            let mut aggregate = Completion::default();
            aggregate.step("execution", Err(wrapped));
            assert_eq!(aggregate.local, LocalState::Outstanding);
            assert_eq!(aggregate.remote, RemoteState::Unknown);
            assert!(aggregate.committed.is_empty());
            let aggregated = completion::complete(Ok(()), aggregate).unwrap_err();
            assert!(aggregated.has_committed_effect());
            assert!(aggregated.to_string().contains("exact-view"));
        }
    }
}
