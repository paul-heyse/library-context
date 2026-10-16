//! Explicit installed-campaign controls. These create no service, table or
//! operator selection; the enclosing fixture/campaign owns the selected target.
use futures::StreamExt;
use lctx_model::domain::{ModelError, completion::Completion, resources::ResourceBudget};
use lctx_native::{
    NativeReader, RuntimeConfig,
    control::{RecoveryFactoryOwner, with_recovery_scope},
    reader,
};
use std::{num::NonZeroU32, path::Path, sync::Arc};
use surrealdb::method::BatchItem;
use surrealdb::types::{Bytes, Object, RecordId, Value, Variables};

// Use the actual production budget adapter, rather than a second policy or a
// mock provider. NativeReader below uses its ordinary compiled implementation.
#[path = "../src/reader/ingress.rs"]
mod native_admission;

fn config() -> Result<RuntimeConfig, ModelError> {
    let path = std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").ok_or(ModelError::Schema(
        "explicit owned ingress campaign runtime required",
    ))?;
    let config = RuntimeConfig::read(Path::new(&path))?;
    if config.database.as_str() != "validation" {
        return Err(ModelError::Conflict(
            "ingress controls require the validation campaign target",
        ));
    }
    Ok(config)
}
fn require(condition: bool, reason: &'static str) -> Result<(), ModelError> {
    if condition {
        Ok(())
    } else {
        Err(ModelError::Schema(reason))
    }
}
fn envelope() -> Value {
    let mut nested = Object::new();
    nested.insert(
        "target",
        Value::RecordId(RecordId::new("ingress_evidence", "leaf")),
    );
    nested.insert(
        "children",
        Value::Array(vec![Value::from_t(-1), Value::Null, Value::from_t(true)].into()),
    );
    let mut value = Object::new();
    value.insert("payload", Value::String("catalog fact".repeat(24 * 1024)));
    value.insert("binary", Value::Bytes(Bytes::from(vec![173; 128 * 1024])));
    value.insert("nested", Value::Object(nested));
    Value::Object(value)
}
async fn parallel_values(
    reader: &NativeReader<()>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let expected = envelope();
    let mut bindings = Variables::new();
    bindings.insert("value", expected.clone());
    let baseline = budget.reserved();
    let mut first = reader.query_stream("RETURN $value", bindings.clone(), 1)?;
    let mut second = reader.query_stream("RETURN $value", bindings, 1)?;
    let (left, right) = tokio::join!(first.next(), second.next());
    let result = (|| {
        let left = left?.ok_or(ModelError::Schema("first useful native value missing"))?;
        let right = right?.ok_or(ModelError::Schema("second useful native value missing"))?;
        require(
            left == expected && right == expected,
            "native nested/bytes values changed",
        )?;
        require(
            budget.reserved() > baseline + 2 * 128 * 1024,
            "parallel decoded values did not retain shared pool admission",
        )?;
        // The cursors retain each value's original charge until the next poll.
        // Drop the values before that acknowledgment, as production consumers do.
        drop((left, right));
        Ok(())
    })();
    let mut terminal = Completion::default();
    if result.is_ok() {
        let (left, right) = tokio::join!(first.next(), second.next());
        terminal.step(
            "first native value terminal",
            left.and_then(|value| require(value.is_none(), "unexpected first extra row")),
        );
        terminal.step(
            "second native value terminal",
            right.and_then(|value| require(value.is_none(), "unexpected second extra row")),
        );
    }
    let (left, right) = tokio::join!(first.drain_transport(), second.drain_transport());
    terminal.step("first native ingress drainage", left);
    terminal.step("second native ingress drainage", right);
    drop((first, second));
    lctx_model::domain::completion::complete(result, terminal)
}
async fn byte_framing(
    client: &Arc<surrealdb::Surreal<surrealdb::engine::remote::grpc::Client>>,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let values = (0..32)
        .map(|ordinal| {
            let mut row = Object::new();
            row.insert("ordinal", Value::from_t(ordinal));
            row.insert("payload", Value::String("f".repeat(8192)));
            Value::Object(row)
        })
        .collect::<Vec<_>>();
    let mut bindings = Variables::new();
    bindings.insert("rows", Value::Array(values.into()));
    let options =
        native_admission::options(budget)?.max_batch_bytes(NonZeroU32::new(64 * 1024).unwrap());
    let mut stream = client
        .query("SELECT * FROM $rows")
        .bind(bindings)
        .stream_batches_with_options(options)
        .map_err(reader::sdk_error)?;
    let mut count = 0;
    let mut batches = 0;
    let mut terminals = 0;
    let mut completion = Completion::default();
    while let Some(item) = stream.next().await {
        let (item, leases) = match item {
            Ok(item) => item.into_parts(),
            Err(error) => {
                let (error, leases) = error.into_parts();
                completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
                completion.step(
                    "framed stream receive failure",
                    Err(ModelError::Cause(Box::new(
                        native_admission::LeasedFailure {
                            source: reader::sdk_error(error),
                            _reservations: leases,
                        },
                    ))),
                );
                continue;
            }
        };
        match item {
            BatchItem::Rows { values, .. } => {
                count += values.len();
                batches += 1;
            }
            BatchItem::Finished { error: None, .. } => terminals += 1,
            BatchItem::Finished {
                error: Some(error), ..
            } => {
                completion.step(
                    "framed statement failure",
                    Err(ModelError::Cause(Box::new(
                        native_admission::LeasedFailure {
                            source: reader::sdk_error(error),
                            _reservations: leases,
                        },
                    ))),
                );
                continue;
            }
            item => {
                drop(item);
                completion.step(
                    "framed result shape",
                    Err(ModelError::Schema(
                        "framed SELECT changed row-oriented shape",
                    )),
                );
            }
        }
        // Actual row data drops before its original frame reservations.
        drop(leases);
    }
    lctx_model::domain::completion::complete(Ok(()), completion)?;
    require(
        count == 32 && batches > 1 && terminals == 1,
        "patched peer did not preserve rows through byte-framed batches",
    )
}

#[tokio::test]
#[ignore = "explicit owned patched-peer campaign via LCTX_COMPILER_RUNTIME_CONFIG"]
async fn patched_peer_admits_parallel_catalog_values_and_byte_framing() -> Result<(), ModelError> {
    let config = config()?;
    let budget = ResourceBudget::fixed(256 * 1024 * 1024)?;
    let factory = RecoveryFactoryOwner::new(config.receipt_binding()?, budget.clone())?;
    let (client, lease) = reader::connect_owned(
        &config.endpoint,
        &config.writer_credentials(),
        config.namespace.as_str(),
        config.database.as_str(),
        &factory,
    )
    .await?;
    let reader = NativeReader::for_views(client.clone(), vec![]).with_budget(&budget);
    let result: Result<(), ModelError> =
        with_recovery_scope(lease.recovery_scope().clone(), async {
            parallel_values(&reader, &budget).await?;
            byte_framing(&client, &budget).await?;
            let tiny = ResourceBudget::scoped(&budget, 1)?;
            let refusal = reader.with_request_budget_clone(&tiny).query_stream(
                "RETURN 1",
                Variables::new(),
                1,
            );
            require(
                refusal.is_err(),
                "tiny pool did not refuse SDK queue before issue",
            )?;
            require(
                tiny.reserved() == 0,
                "preissue refusal retained a nonexistent accepted request",
            )?;
            Ok(())
        })
        .await;
    let mut completion = Completion::default();
    if result
        .as_ref()
        .is_err_and(|error| !error.permits_storage_cleanup())
    {
        completion.remote = lctx_model::domain::completion::RemoteState::Unknown;
    }
    completion.step("native ingress reader close", reader.close().await);
    completion.step(
        "native ingress factory session finality",
        lease.complete_with(&completion).await,
    );
    lctx_model::domain::completion::complete(result, completion)
}
