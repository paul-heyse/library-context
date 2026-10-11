//! Explicit completed-input dispatch for pure kernels. There are no persisted grants or epochs.
use crate::workspace::CompletedInputs;
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::sources::CompletedInput, charged, resources::ResourceBudget, *,
};
use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet},
};
/// Expected coverage needs artifact identity, input ownership and the model's bounded
/// classifier property. Ordinary selected algorithm reads still receive canonical rows.
pub(crate) async fn stream_artifact_admission(
    access: &CompletedInputs,
    input: &ValidationInput,
    session: &SessionContext,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
) -> Result<bool, ModelError> {
    if input.type_id() != TypeId::of::<source::SourceArtifact>() {
        return Ok(false);
    }
    let permit = access.read_at::<source::SourceArtifact>(input.prefix())?;
    let table = access.table_for(input)?;
    let selected = format!(
        "SELECT {} FROM {}",
        analysis::expected::CoverageAdmission::artifact_property_columns(),
        identifier(&table)
    );
    stream_query_at(
        &permit,
        input,
        access,
        session,
        &selected,
        |permit, batch| admission.visit_artifact_properties(permit, batch),
    )
    .await?;
    Ok(true)
}
pub struct ConsumedInputs {
    declarations: Vec<ValidationInput>,
    dispatched: BTreeSet<usize>,
    _charge: charged::StateCharge,
}
impl ConsumedInputs {
    pub fn new(
        mut declarations: Vec<ValidationInput>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        declarations.sort_by_key(|input| (input.name(), input.prefix(), input.order().to_vec()));
        declarations.dedup_by(|a, b| {
            a.name() == b.name() && a.prefix() == b.prefix() && a.order() == b.order()
        });
        let mut charge = charged::StateCharge::new(budget, "completed-consumer-inputs");
        charge.grow(
            declarations
                .capacity()
                .saturating_mul(size_of::<ValidationInput>()),
        )?;
        Ok(Self {
            declarations,
            dispatched: Default::default(),
            _charge: charge,
        })
    }
    pub fn next<R: Record>(
        &mut self,
        inputs: &CompletedInputs,
    ) -> Result<Option<(ValidationInput, CompletedInput<R>)>, ModelError> {
        let Some((index, declaration)) =
            self.declarations
                .iter()
                .enumerate()
                .find(|(index, declaration)| {
                    declaration.type_id() == TypeId::of::<R>() && !self.dispatched.contains(index)
                })
        else {
            return Ok(None);
        };
        let source = inputs.read_at::<R>(declaration.prefix())?;
        self.dispatched.insert(index);
        Ok(Some((declaration.clone(), source)))
    }
    pub fn finish(self, producer: &str) -> Result<(), ModelError> {
        if let Some((_, input)) = self
            .declarations
            .iter()
            .enumerate()
            .find(|(index, _)| !self.dispatched.contains(index))
        {
            return Err(ModelError::Invalid(format!(
                "producer {producer} has no typed loader for {}",
                input.name()
            )));
        }
        Ok(())
    }
}
pub fn stream_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    stream_where_at(input, declaration, inputs, session, None, consume)
}
/// Read the exact immutable input through an owner-selected predicate before rich decoding.
/// The predicate is compiler SQL, never a provider string or a source of semantic authority.
pub fn stream_where_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    predicate: Option<&str>,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    let selected = inputs.table_at::<R>(declaration.prefix()).map(|table| {
        format!(
            "SELECT * FROM {}{}",
            identifier(&table),
            predicate
                .map(|predicate| format!(" WHERE ({predicate})"))
                .unwrap_or_default()
        )
    });
    // Table selection errors precede permit errors on this route. Checking immutable
    // captured identities does not plan or start a stream; errors are delivered on polling.
    let checked = if selected.is_ok() {
        check_stream_input(input, declaration, inputs)
    } else {
        Ok(())
    };
    let mut consume = consume;
    let visit: BatchConsumer<'a> = Box::new(move |batch| consume(input, batch));
    stream_checked_owned(
        checked,
        declaration,
        session,
        selected,
        None,
        Some(inputs),
        visit,
    )
}
/// Stream an owner-declared SELECT after checking the exact nominal record and captured view.
/// Joins and closure predicates are supplied by the owning semantic kernel.
pub fn stream_query_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    selected: &str,
    consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    stream_query_filter_at(input, declaration, inputs, session, selected, None, consume)
}
/// Preserve an owner-authored DataFusion predicate at the checked completed-input boundary.
/// The predicate remains an Expr; it is never converted into SQL text.
pub(crate) fn stream_query_filter_at<'a, R: Record>(
    input: &'a CompletedInput<R>,
    declaration: &'a ValidationInput,
    inputs: &'a CompletedInputs,
    session: &'a SessionContext,
    selected: &str,
    predicate: Option<datafusion::logical_expr::Expr>,
    mut consume: impl FnMut(&CompletedInput<R>, &arrow_array::RecordBatch) -> Result<(), ModelError>
    + Send
    + 'a,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    let checked = check_stream_input(input, declaration, inputs);
    let visit: BatchConsumer<'a> = Box::new(move |batch| consume(input, batch));
    stream_checked_owned(
        checked,
        declaration,
        session,
        Ok(selected.to_owned()),
        predicate,
        Some(inputs),
        visit,
    )
}

fn check_stream_input<R: Record>(
    input: &CompletedInput<R>,
    declaration: &ValidationInput,
    inputs: &CompletedInputs,
) -> Result<(), ModelError> {
    if declaration.type_id() != TypeId::of::<R>() || declaration.name() != R::NAME {
        return Err(ModelError::Invalid(
            "scoped stream record declaration mismatch".into(),
        ));
    }
    if input.source() != inputs.read_at::<R>(declaration.prefix())?.source() {
        return Err(ModelError::Conflict(
            "scoped stream completed input mismatch",
        ));
    }
    Ok(())
}

type BatchConsumer<'a> =
    Box<dyn FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError> + Send + 'a>;

/// Own callback erasure and selected SQL before creating the asynchronous state machine.
/// Record-specific validation/decoding remain at the typed caller boundary.
fn stream_checked_owned<'a>(
    checked: Result<(), ModelError>,
    declaration: &'a ValidationInput,
    session: &'a SessionContext,
    selected: Result<String, ModelError>,
    predicate: Option<datafusion::logical_expr::Expr>,
    inputs: Option<&'a CompletedInputs>,
    mut consume: BatchConsumer<'a>,
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let selected = selected?;
        checked?;
        stream_batches_prepared(
            declaration,
            session,
            &selected,
            predicate,
            inputs,
            consume.as_mut(),
        )
        .await
    })
}

/// Record-independent physical planning and iteration. Dispatch occurs once per batch.
/// Detached validation callers establish their invariant/table/type binding before entering.
/// This physical loop grants no completed-input authority of its own.
fn stream_batches_prepared<'a>(
    declaration: &'a ValidationInput,
    session: &'a SessionContext,
    selected: &'a str,
    predicate: Option<datafusion::logical_expr::Expr>,
    inputs: Option<&'a CompletedInputs>,
    consume: &'a mut (dyn FnMut(&arrow_array::RecordBatch) -> Result<(), ModelError> + Send),
) -> futures::future::BoxFuture<'a, Result<(), ModelError>> {
    Box::pin(async move {
        let order = declaration
            .order()
            .iter()
            .map(|column| identifier(column))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT * FROM ({selected}) AS scoped_input{}",
            if order.is_empty() {
                String::new()
            } else {
                format!(" ORDER BY {order}")
            }
        );
        let mut selected = match inputs {
            Some(inputs) => inputs.query_template(session, declaration, &sql).await?,
            None => crate::sql::query(session, &sql)
                .await
                .map_err(crate::sql::model_error)?,
        };
        lctx_compiler_store::consumed_rows::stream_planned(selected,predicate,consume).await?;
        Ok(())
    })
}
#[cfg(test)]
mod checked_stream_controls {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[tokio::test]
    async fn stream_adapter_refuses_before_planning_or_visiting_and_releases_callback() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let charge = budget.reserve("stream-callback-control", 4096).unwrap();
        let visited = Arc::new(AtomicBool::new(false));
        let observed = visited.clone();
        let consume: BatchConsumer<'_> = Box::new(move |_| {
            let _retained = &charge;
            observed.store(true, Ordering::Relaxed);
            Ok(())
        });
        let declaration = ValidationInput::of::<input::Package>(&["id"]);
        let session = SessionContext::new();
        let future = stream_checked_owned(
            Err(ModelError::Conflict("stream admission refusal")),
            &declaration,
            &session,
            Ok("not valid SQL".to_owned()),
            None,
            None,
            consume,
        );
        assert_eq!(budget.reserved(), 4096);
        assert!(!visited.load(Ordering::Relaxed));
        assert!(matches!(
            future.await,
            Err(ModelError::Conflict("stream admission refusal"))
        ));
        assert!(!visited.load(Ordering::Relaxed));
        assert_eq!(budget.reserved(), 0);
    }

    #[tokio::test]
    async fn stream_table_selection_error_precedes_admission_error() {
        let declaration = ValidationInput::of::<input::Package>(&["id"]);
        let session = SessionContext::new();
        let mut visited = false;
        let consume: BatchConsumer<'_> = Box::new(|_| {
            visited = true;
            Ok(())
        });
        let result = stream_checked_owned(
            Err(ModelError::Conflict("later permit failure")),
            &declaration,
            &session,
            Err(ModelError::Conflict("earlier table selection failure")),
            None,
            None,
            consume,
        )
        .await;
        assert!(matches!(
            result,
            Err(ModelError::Conflict("earlier table selection failure"))
        ));
        assert!(!visited);
    }
}

#[cfg(test)]
pub(crate) fn assert_decoder_reachability(
    declarations: Vec<ValidationInput>,
    decoders: &BTreeSet<TypeId>,
) {
    let missing: Vec<_> = declarations
        .iter()
        .filter(|input| !decoders.contains(&input.type_id()))
        .map(ValidationInput::name)
        .collect();
    assert!(
        missing.is_empty(),
        "completed kernel input lacks typed decoder: {missing:?}"
    );
}

pub use lctx_compiler_store::consumed_rows::*;
