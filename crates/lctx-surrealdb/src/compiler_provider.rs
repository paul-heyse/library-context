//! Optimizer-visible projected reads over one exact completed native membership universe.
use crate::{
    compiler::{NativeCompilerStore, NativePredicate, ProducingScope},
    projected_arrow::ProjectedBuilder,
};
use arrow_schema::{SchemaRef, SortOptions};
use async_trait::async_trait;
use datafusion::{
    catalog::{Session, TableProvider},
    common::{ScalarValue, Statistics, stats::Precision},
    execution::TaskContext,
    logical_expr::{Expr, Operator, TableProviderFilterPushDown, TableType},
    physical_expr::{LexOrdering, PhysicalSortExpr, expressions::Column},
    physical_plan::{
        DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, SendableRecordBatchStream,
        stream::RecordBatchStreamAdapter,
        streaming::{PartitionStream, StreamingTableExec},
    },
};
use lctx_model::domain::{
    ModelError, Relation,
    charged::StateCharge,
    completed::CompletedView,
    resources::{ResourceBudget, TRANSFER_ROWS},
};
use std::sync::Arc;
use surrealdb::types::{Number, Value, Variables};

pub fn table_provider(
    store: Arc<NativeCompilerStore>,
    view: CompletedView,
    relation: Relation,
    budget: ResourceBudget,
    batch_rows: usize,
) -> Result<Arc<dyn TableProvider>, ModelError> {
    view.validate()?;
    if view.relation != relation.name() || batch_rows == 0 {
        return Err(ModelError::Schema("native compiler table binding"));
    }
    Ok(Arc::new(NativeTable {
        store,
        view,
        relation,
        budget,
        batch_rows,
        keys: vec![],
        producing: None,
    }))
}
#[derive(Clone)]
struct SelectedKeys {
    keys: Arc<Vec<[u8; 16]>>,
    field: Option<&'static str>,
    _charge: Arc<StateCharge>,
}
#[derive(Clone)]
struct NativeTable {
    producing: Option<ProducingScope>,
    keys: Vec<SelectedKeys>,
    store: Arc<NativeCompilerStore>,
    view: CompletedView,
    relation: Relation,
    budget: ResourceBudget,
    batch_rows: usize,
}
/// Bind a producer's physical read owner before plans or delayed streams are created.
/// Finite tables need no native read owner and retain their existing route.
pub fn bind_producing(provider: &Arc<dyn TableProvider>, scope: &ProducingScope) -> Result<Arc<dyn TableProvider>, ModelError> {
    let Some(source) = provider.downcast_ref::<NativeTable>() else { return Ok(provider.clone()); };
    if !scope.belongs_to(&source.store) { return Err(ModelError::Conflict("foreign producing provider store")); }
    let mut selected = source.clone();
    selected.producing = Some(scope.clone());
    Ok(Arc::new(selected))
}
/// Bind sorted, distinct nominal keys before payload selection. The existing key owner charge
/// follows providers, physical plans and active streams even after the grain scope is dropped.
/// Detached finite tables return None and continue through the compact local key join.
pub fn select_table(
    provider: &Arc<dyn TableProvider>,
    keys: Arc<Vec<[u8; 16]>>,
    charge: Arc<StateCharge>,
) -> Result<Option<Arc<dyn TableProvider>>, ModelError> {
    let Some(source) = provider.downcast_ref::<NativeTable>() else {
        return Ok(None);
    };
    if keys.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ModelError::Schema(
            "native selected keys must be sorted and distinct",
        ));
    }
    let mut selected = source.clone();
    selected.keys.push(SelectedKeys {
        keys,
        field: None,
        _charge: charge,
    });
    Ok(Some(Arc::new(selected)))
}
/// One-hop ownership uses a bounded frontier and the relation's declared atomic reference.
/// Larger frontiers are split by the shared closure executor before this binding is created.
pub const REFERENCE_KEYS: usize = 1024;
pub fn is_native_table(provider: &Arc<dyn TableProvider>) -> bool {
    provider.downcast_ref::<NativeTable>().is_some()
}
/// Compact current tokens for finer product domains; finite providers retain their own route.
pub async fn content_tokens(provider:&Arc<dyn TableProvider>,keys:&[[u8;16]],budget:&ResourceBudget)->Result<Option<Vec<([u8;16],lctx_model::domain::ContentHash)>>,ModelError> {
    let Some(source)=provider.downcast_ref::<NativeTable>() else{return Ok(None)};
    if !source.keys.is_empty() {return Ok(None)} // A filtered provider requires its declared domain.
    let read = source.store.row_tokens(&source.view,&source.relation,keys,budget);
    match &source.producing {
        Some(scope) => scope.run(read).await,
        None => read.await,
    }.map(Some)
}
pub fn select_field_table(
    provider: &Arc<dyn TableProvider>,
    field: &str,
    keys: Arc<Vec<[u8; 16]>>,
    charge: Arc<StateCharge>,
) -> Result<Option<Arc<dyn TableProvider>>, ModelError> {
    let Some(source) = provider.downcast_ref::<NativeTable>() else {
        return Ok(None);
    };
    let descriptor = source
        .relation
        .fields()
        .iter()
        .find(|descriptor| descriptor.name() == field)
        .ok_or(ModelError::Schema("native one-hop field"))?;
    if descriptor.target().is_none()
        || descriptor.list()
        || !crate::schema::atomic_scope_field(source.relation.name(), descriptor.name())
        || keys.len() > REFERENCE_KEYS
        || keys.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ModelError::Schema("native one-hop atomic frontier"));
    }
    let mut selected = source.clone();
    selected.keys.push(SelectedKeys {
        keys,
        field: Some(descriptor.name()),
        _charge: charge,
    });
    Ok(Some(Arc::new(selected)))
}
fn row_statistics(
    schema: &SchemaRef,
    rows: u64,
    keys: &[SelectedKeys],
    filtered: bool,
) -> Statistics {
    let mut statistics = Statistics::new_unknown(schema.as_ref());
    // Every predicate is a subset of the immutable view. Nominal ID selections also
    // have a key-count upper bound; non-unique field selections do not. Keep estimates
    // inexact whenever selection/filter exists, even for zero: native authority must
    // still be checked when the source is executed.
    let upper = keys.iter()
        .filter(|selection| selection.field.is_none())
        .map(|selection| selection.keys.len())
        .chain(usize::try_from(rows).ok())
        .min();
    statistics.num_rows = match upper {
        Some(rows) if !filtered && keys.is_empty() => Precision::Exact(rows),
        Some(upper) => Precision::Inexact(upper),
        None => Precision::Absent,
    };
    statistics
}
impl std::fmt::Debug for NativeTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeCompilerTable")
            .field("view", &self.view)
            .finish_non_exhaustive()
    }
}
#[async_trait]
impl TableProvider for NativeTable {
    fn schema(&self) -> SchemaRef {
        self.relation.schema().clone()
    }
    fn table_type(&self) -> TableType {
        TableType::Base
    }
    fn statistics(&self) -> Option<Statistics> {
        Some(row_statistics(
            &self.schema(),
            self.view.rows,
            &self.keys,
            false,
        ))
    }
    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> datafusion::error::Result<Vec<TableProviderFilterPushDown>> {
        Ok(filters
            .iter()
            .map(|expr| {
                if SupportedFilter::interpret(expr, &self.relation).is_some() {
                    TableProviderFilterPushDown::Exact
                } else {
                    TableProviderFilterPushDown::Unsupported
                }
            })
            .collect())
    }
    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        _limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        let schema = match projection {
            Some(projection) => Arc::new(self.schema().project(projection)?),
            None => self.schema(),
        };
        let mut bindings = Variables::new();
        let mut predicates = Vec::new();
        let mut preparation = Vec::new();
        let mut driver = None;
        for (index, filter) in filters.iter().enumerate() {
            if let Some(supported) = SupportedFilter::interpret(filter, &self.relation) {
                let mut translated = supported.render(&self.relation, &format!("f{index}_"));
                if driver.is_none() {
                    driver = translated.driver.take();
                }
                bindings.extend(translated.values.into_iter().collect());
                preparation.extend(translated.preparation);
                predicates.push(format!("({})", translated.sql));
            }
        }
        let predicate = (!predicates.is_empty()).then(|| {
            let sql = predicates.join(" AND ");
            if self.keys.is_empty()
                && let Some((field, values)) = driver
            {
                NativePredicate::FieldSql {
                    field,
                    values,
                    sql,
                    bindings,
                    preparation,
                }
            } else {
                NativePredicate::Sql {
                    sql,
                    bindings,
                    preparation,
                }
            }
        });
        let partition = Arc::new(NativePartition {
            producing: self.producing.clone(),
            store: self.store.clone(),
            view: self.view.clone(),
            relation: self.relation.clone(),
            projection: projection.cloned(),
            schema: schema.clone(),
            predicate,
            keys: self.keys.clone(),
            budget: self.budget.clone(),
            batch_rows: self.batch_rows,
        });
        // The partition already projects natively. Never layer a rich full-schema read under it.
        let statistics = Arc::new(row_statistics(
            &schema,
            self.view.rows,
            &self.keys,
            !filters.is_empty(),
        ));
        // scan_rows orders globally by nominal semantic_key, which is exactly Arrow id:
        // explicit keys and candidate sort are ordered; selected windows are disjoint and
        // sorted, while an atomic-field driver is one globally ordered window.
        let ordering = schema.index_of("id").ok().and_then(|index| {
            LexOrdering::new([PhysicalSortExpr::new(
                Arc::new(Column::new("id", index)),
                SortOptions {
                    descending: false,
                    nulls_first: false,
                },
            )])
        });
        let inner =
            StreamingTableExec::try_new(schema, vec![partition], None, ordering, false, None)?;
        Ok(Arc::new(NativeExec {
            inner: Arc::new(inner),
            statistics,
        }))
    }
}
struct NativePartition {
    producing: Option<ProducingScope>,
    store: Arc<NativeCompilerStore>,
    view: CompletedView,
    relation: Relation,
    projection: Option<Vec<usize>>,
    schema: SchemaRef,
    predicate: Option<NativePredicate>,
    keys: Vec<SelectedKeys>,
    budget: ResourceBudget,
    batch_rows: usize,
}
impl std::fmt::Debug for NativePartition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeCompilerPartition")
            .field("view", &self.view.identity)
            .finish_non_exhaustive()
    }
}
impl PartitionStream for NativePartition {
    fn schema(&self) -> &SchemaRef {
        &self.schema
    }
    fn execute(&self, _context: Arc<TaskContext>) -> SendableRecordBatchStream {
        use futures::TryStreamExt;
        let store = self.store.clone();
        let view = self.view.clone();
        let relation = self.relation.clone();
        let projection = self.projection.clone();
        let predicate = self.predicate.clone();
        let budget = self.budget.clone();
        let batch_rows = self.batch_rows;
        let keys = self.keys.clone();
        let stream = if keys.is_empty() {
            let stream = futures::stream::once(async move {
                scan_batches(
                    store, view, relation, projection, predicate, budget, batch_rows,
                )
                .await
                .map_err(df_error)
            })
            .try_flatten();
            Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(), stream))
                as SendableRecordBatchStream
        } else {
            selected_batches(
                store,
                view,
                relation,
                projection,
                predicate,
                keys,
                budget,
                batch_rows,
                self.schema.clone(),
            )
        };
        match &self.producing {
            Some(scope) => match scope.bind_stream(stream) {
                Ok(stream) => Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(), stream)),
                Err(error) => Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(), futures::stream::once(async move { Err(df_error(error)) }))),
            },
            None => stream,
        }
    }
}
/// A source leaf forwards streaming behavior and publishes the same view/selection cardinality
/// at the physical optimizer boundary. StreamingTableExec has no statistics setter at DF55.1.
#[derive(Debug)]
struct NativeExec {
    inner: Arc<dyn ExecutionPlan>,
    statistics: Arc<Statistics>,
}
impl DisplayAs for NativeExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt_as(t, f)
    }
}
impl ExecutionPlan for NativeExec {
    fn name(&self) -> &'static str {
        "NativeCompilerExec"
    }
    fn properties(&self) -> &Arc<PlanProperties> {
        self.inner.properties()
    }
    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }
    fn apply_expressions(
        &self,
        f: &mut dyn FnMut(
            &Arc<dyn datafusion::physical_expr::PhysicalExpr>,
        ) -> datafusion::error::Result<
            datafusion::common::tree_node::TreeNodeRecursion,
        >,
    ) -> datafusion::error::Result<datafusion::common::tree_node::TreeNodeRecursion> {
        self.inner.apply_expressions(f)
    }
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        if children.is_empty() {
            Ok(self)
        } else {
            Err(datafusion::error::DataFusionError::Internal(
                "native compiler source has no children".into(),
            ))
        }
    }
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> datafusion::error::Result<SendableRecordBatchStream> {
        self.inner.execute(partition, context)
    }
    fn partition_statistics(
        &self,
        partition: Option<usize>,
    ) -> datafusion::error::Result<Arc<Statistics>> {
        if partition.is_some_and(|partition| partition != 0) {
            return Err(datafusion::error::DataFusionError::Internal(
                "native compiler partition absent".into(),
            ));
        }
        Ok(Arc::new(self.statistics.as_ref().clone().with_fetch(
            self.inner.fetch(),
            0,
            1,
        )?))
    }
    fn metrics(&self) -> Option<datafusion::physical_plan::metrics::MetricsSet> {
        self.inner.metrics()
    }
    fn fetch(&self) -> Option<usize> {
        self.inner.fetch()
    }
    fn with_fetch(&self, limit: Option<usize>) -> Option<Arc<dyn ExecutionPlan>> {
        self.inner.with_fetch(limit).map(|inner| {
            Arc::new(Self {
                inner,
                statistics: self.statistics.clone(),
            }) as Arc<dyn ExecutionPlan>
        })
    }
}
/// Invariant field bindings are prepared on first poll and retained (and charged) by the
/// selected scan. Native scan ownership still takes SDK value copies per active window.
struct PreparedSelection {
    sql: String,
    bindings: Variables,
    preparation: Vec<String>,
    field_driver: Option<(String, Vec<Value>)>,
    empty: bool,
    field_copy_bytes: usize,
    _reservation: Box<dyn lctx_model::domain::resources::Reservation>,
}
impl PreparedSelection {
    fn new(
        keys: &[SelectedKeys],
        driver: usize,
        relation: &Relation,
        predicate: Option<NativePredicate>,
        budget: &ResourceBudget,
    ) -> Result<Self, ModelError> {
        let field_keys = keys
            .iter()
            .filter(|selection| selection.field.is_some())
            .map(|selection| selection.keys.len())
            .sum::<usize>();
        let reservation = budget.reserve(
            "native-selected-field-preparation",
            field_keys.saturating_mul(1536).saturating_add(4096),
        )?;
        let mut bindings = Variables::new();
        let mut predicates = Vec::new();
        let mut preparation = Vec::new();
        let mut field_driver = None;
        for (index, selection) in keys.iter().enumerate() {
            let Some(field) = selection.field else {
                continue;
            };
            let name = format!("closure_selected_fields_{index}");
            let values = selection
                .keys
                .iter()
                .map(|key| {
                    Value::Array(
                        key.iter()
                            .map(|byte| Value::Number(Number::Int(i64::from(*byte))))
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            if index == driver {
                field_driver = Some((field.to_string(), values));
                continue;
            }
            bindings.insert(name.clone(), values);
            let scope = format!("closure_selected_scope_{index}");
            predicates.push(crate::prepared::prepare_scope(
                &mut preparation,
                &scope,
                &format!("'{}'", relation.name()),
                field,
                &format!("${name}"),
            ));
        }
        match predicate {
            Some(NativePredicate::Sql {
                sql,
                bindings: static_bindings,
                preparation: static_preparation,
            }) => {
                bindings.extend(static_bindings);
                preparation.extend(static_preparation);
                predicates.push(format!("({sql})"));
            }
            None => {}
            _ => return Err(ModelError::Schema("native table static predicate")),
        }
        Ok(Self {
            sql: if predicates.is_empty() {
                "true".into()
            } else {
                predicates.join(" AND ")
            },
            bindings,
            preparation,
            field_driver,
            empty: keys.iter().any(|selection| selection.keys.is_empty()),
            field_copy_bytes: field_keys.saturating_mul(1536),
            _reservation: reservation,
        })
    }
    fn predicate(&self, nominal: Option<Vec<[u8; 16]>>) -> Result<NativePredicate, ModelError> {
        // Empty demand must still check the exact view through the ordinary native reader.
        if self.empty || nominal.as_ref().is_some_and(Vec::is_empty) {
            return Ok(NativePredicate::Keys(vec![]));
        }
        if let Some(keys) = nominal {
            Ok(NativePredicate::KeysSql {
                keys,
                sql: self.sql.clone(),
                bindings: self.bindings.clone(),
                preparation: self.preparation.clone(),
            })
        } else if let Some((field, values)) = &self.field_driver {
            Ok(NativePredicate::FieldSql {
                field: field.clone(),
                values: values.clone(),
                sql: self.sql.clone(),
                bindings: self.bindings.clone(),
                preparation: self.preparation.clone(),
            })
        } else {
            Err(ModelError::Schema("native selected driver"))
        }
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "Selected native streams retain the exact view, projection, static predicates and existing key owner"
)]
fn selected_batches(
    store: Arc<NativeCompilerStore>,
    view: CompletedView,
    relation: Relation,
    projection: Option<Vec<usize>>,
    predicate: Option<NativePredicate>,
    keys: Vec<SelectedKeys>,
    budget: ResourceBudget,
    batch_rows: usize,
    schema: SchemaRef,
) -> SendableRecordBatchStream {
    use futures::{StreamExt, TryStreamExt};
    // Every selection is sorted and distinct; disjoint windows preserve native semantic-key
    // order. Choose the smallest demand and intersect any nested selection before transfer.
    let driver = keys
        .iter()
        .enumerate()
        .filter(|(_, selection)| selection.field.is_none())
        .min_by_key(|(_, selection)| selection.keys.len())
        .map(|(index, _)| index)
        .unwrap_or(0);
    let windows = futures::stream::try_unfold(
        (0usize, false, None::<Arc<PreparedSelection>>, predicate),
        move |(offset, done, prepared, predicate)| {
            let store = store.clone();
            let view = view.clone();
            let relation = relation.clone();
            let projection = projection.clone();
            let keys = keys.clone();
            let budget = budget.clone();
            async move {
                if done {
                    return Ok::<_, datafusion::error::DataFusionError>(None);
                }
                let end = if keys[driver].field.is_some() {
                    keys[driver].keys.len()
                } else {
                    offset
                        .saturating_add(TRANSFER_ROWS)
                        .min(keys[driver].keys.len())
                };
                let prepared = match prepared {
                    Some(prepared) => prepared,
                    None => Arc::new(
                        PreparedSelection::new(&keys, driver, &relation, predicate, &budget)
                            .map_err(df_error)?,
                    ),
                };
                // Changing nominal-window state and SDK-owned copies have a separate lifetime.
                let transfer = budget
                    .reserve(
                        "native-selected-key-transfer",
                        (end - offset)
                            .saturating_mul(192)
                            .saturating_add(prepared.field_copy_bytes)
                            .saturating_add(4096),
                    )
                    .map_err(df_error)?;
                let nominal = if keys[driver].field.is_none() {
                    Some(
                        keys[driver].keys[offset..end]
                            .iter()
                            .filter(|key| {
                                keys.iter()
                                    .filter(|selection| selection.field.is_none())
                                    .all(|selection| selection.keys.binary_search(key).is_ok())
                            })
                            .copied()
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                };
                let predicate = prepared.predicate(nominal).map_err(df_error)?;
                let rows = scan_batches(
                    store,
                    view,
                    relation,
                    projection,
                    Some(predicate),
                    budget,
                    batch_rows,
                )
                .await
                .map_err(df_error)?;
                let held_prepared = prepared.clone();
                let rows = rows.map(move |batch| {
                    let _held = (&transfer, &held_prepared);
                    batch
                });
                Ok(Some((
                    rows,
                    (end, end == keys[driver].keys.len(), Some(prepared), None),
                )))
            }
        },
    )
    .try_flatten();
    Box::pin(RecordBatchStreamAdapter::new(schema, windows))
}
pub async fn scan_batches(
    store: Arc<NativeCompilerStore>,
    view: CompletedView,
    relation: Relation,
    projection: Option<Vec<usize>>,
    predicate: Option<NativePredicate>,
    budget: ResourceBudget,
    batch_rows: usize,
) -> Result<SendableRecordBatchStream, ModelError> {
    if batch_rows == 0 {
        return Err(ModelError::Schema("native Arrow batch rows"));
    }
    let schema = match &projection {
        Some(projection) => Arc::new(
            relation
                .schema()
                .project(projection)
                .map_err(ModelError::codec)?,
        ),
        None => relation.schema().clone(),
    };
    let columns = schema
        .fields()
        .iter()
        .map(|field| field.name().clone())
        .collect::<Vec<_>>();
    let rows = store
        .scan_rows(&view, &relation, Some(&columns), predicate, &budget)
        .await?;
    batches_from_rows(rows, relation, schema, &budget, batch_rows)
}
/// Share bounded Arrow transfer after the native owner has admitted the exact row selection.
pub(crate) fn batches_from_rows(
    rows: crate::compiler::CompilerRows,
    relation: Relation,
    schema: SchemaRef,
    budget: &ResourceBudget,
    batch_rows: usize,
) -> Result<SendableRecordBatchStream, ModelError> {
    let builder = ProjectedBuilder::new(relation, schema.clone(), budget)?;
    let stream = futures::stream::try_unfold(
        (rows, builder, false),
        move |(mut rows, mut builder, done)| async move {
            if done {
                return Ok(None);
            }
            builder.release().map_err(df_error)?;
            loop {
                match rows.next().await.map_err(df_error)? {
                    Some(row) => builder.push(row).map_err(df_error)?,
                    None => {
                        if builder.rows() == 0 {
                            return Ok(None);
                        }
                        let batch = builder.finish().map_err(df_error)?;
                        return Ok(Some((batch, (rows, builder, true))));
                    }
                }
                if builder.rows() >= batch_rows
                    || builder.bytes() >= lctx_model::domain::resources::TRANSFER_BYTES
                {
                    let batch = builder.finish().map_err(df_error)?;
                    return Ok(Some((batch, (rows, builder, false))));
                }
            }
        },
    );
    Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
}
fn df_error(error: ModelError) -> datafusion::error::DataFusionError {
    datafusion::error::DataFusionError::External(Box::new(error))
}
struct Predicate {
    sql: String,
    values: Vec<(String, Value)>,
    preparation: Vec<String>,
    driver: Option<(String, Vec<Value>)>,
}
/// Recognition borrows operands and performs no native-value or SQL materialization.
/// Both capability reporting and rendering consume this same supported interpretation.
#[derive(Clone, Copy)]
struct FilterField<'a> {
    name: &'a str,
    atomic: bool,
}
impl FilterField<'_> {
    fn sql_expression(self) -> String {
        if self.name == "id" {
            "semantic_key".into()
        } else {
            format!("body.`{}`", self.name.replace('`', "``"))
        }
    }
}
enum SupportedFilter<'a> {
    Boolean(Box<Self>, Operator, Box<Self>),
    Compare(FilterField<'a>, Operator, &'a ScalarValue),
    In(FilterField<'a>, &'a [Expr]),
    Null(FilterField<'a>, bool),
}
fn supported_literal(expr: &Expr) -> Option<&ScalarValue> {
    let Expr::Literal(value, _) = expr else {
        return None;
    };
    match value {
        ScalarValue::Boolean(Some(_))
        | ScalarValue::Int16(Some(_))
        | ScalarValue::Int32(Some(_))
        | ScalarValue::Int64(Some(_))
        | ScalarValue::Utf8(Some(_))
        | ScalarValue::LargeUtf8(Some(_))
        | ScalarValue::FixedSizeBinary(_, Some(_)) => Some(value),
        ScalarValue::Float64(Some(number)) if number.is_finite() => Some(value),
        _ => None,
    }
}
fn native_literal(value: &ScalarValue, id: bool) -> Value {
    match value {
        ScalarValue::Boolean(Some(value)) => Value::Bool(*value),
        ScalarValue::Int16(Some(value)) => Value::Number(Number::Int(i64::from(*value))),
        ScalarValue::Int32(Some(value)) => Value::Number(Number::Int(i64::from(*value))),
        ScalarValue::Int64(Some(value)) => Value::Number(Number::Int(*value)),
        ScalarValue::Float64(Some(value)) => Value::Number(Number::Float(*value)),
        ScalarValue::Utf8(Some(value)) | ScalarValue::LargeUtf8(Some(value)) => {
            Value::String(value.clone())
        }
        ScalarValue::FixedSizeBinary(_, Some(value)) if id => Value::String(hex::encode(value)),
        ScalarValue::FixedSizeBinary(_, Some(value)) => Value::Array(
            value
                .iter()
                .map(|byte| Value::Number(Number::Int(i64::from(*byte))))
                .collect(),
        ),
        _ => unreachable!("only recognized literals are rendered"),
    }
}
impl<'a> SupportedFilter<'a> {
    fn interpret(expr: &'a Expr, relation: &Relation) -> Option<Self> {
        fn field<'a>(expr: &'a Expr, relation: &Relation) -> Option<FilterField<'a>> {
            let Expr::Column(column) = expr else {
                return None;
            };
            if relation.name()
                == <lctx_model::domain::artifact::ArtifactChunk as lctx_model::domain::Record>::NAME
                && column.name == "body"
            {
                return None;
            }
            relation.schema().field_with_name(&column.name).ok()?;
            Some(FilterField {
                name: &column.name,
                atomic: crate::schema::atomic_scope_field(relation.name(), &column.name),
            })
        }
        match expr {
            Expr::BinaryExpr(binary) if matches!(binary.op, Operator::And | Operator::Or) => {
                Some(Self::Boolean(
                    Box::new(Self::interpret(&binary.left, relation)?),
                    binary.op,
                    Box::new(Self::interpret(&binary.right, relation)?),
                ))
            }
            Expr::BinaryExpr(binary)
                if matches!(
                    binary.op,
                    Operator::Eq
                        | Operator::NotEq
                        | Operator::Lt
                        | Operator::LtEq
                        | Operator::Gt
                        | Operator::GtEq
                ) =>
            {
                Some(Self::Compare(
                    field(&binary.left, relation)?,
                    binary.op,
                    supported_literal(&binary.right)?,
                ))
            }
            Expr::InList(list) if !list.negated => {
                let field = field(&list.expr, relation)?;
                for value in &list.list {
                    supported_literal(value)?;
                }
                Some(Self::In(field, &list.list))
            }
            Expr::IsNull(expr) => Some(Self::Null(field(expr, relation)?, true)),
            Expr::IsNotNull(expr) => Some(Self::Null(field(expr, relation)?, false)),
            _ => None,
        }
    }
    // Only mandatory conjuncts can narrow the candidate universe; OR supplies no driver.
    fn driver(&self) -> Option<(String, Vec<Value>)> {
        match self {
            Self::Boolean(left, Operator::And, right) => left.driver().or_else(|| right.driver()),
            Self::Compare(field, Operator::Eq, value) if field.atomic => {
                Some((field.name.into(), vec![native_literal(value, false)]))
            }
            Self::In(field, list) if field.atomic => Some((
                field.name.into(),
                list.iter()
                    .map(|expr| {
                        native_literal(supported_literal(expr).expect("recognized literal"), false)
                    })
                    .collect(),
            )),
            _ => None,
        }
    }
    fn render(&self, relation: &Relation, namespace: &str) -> Predicate {
        fn walk(
            filter: &SupportedFilter<'_>,
            relation: &Relation,
            namespace: &str,
            values: &mut Vec<(String, Value)>,
            preparation: &mut Vec<String>,
        ) -> String {
            let mut bind = |value| {
                let name = format!("{namespace}v{}", values.len());
                values.push((name.clone(), value));
                format!("${name}")
            };
            match filter {
                SupportedFilter::Boolean(left, op, right) => {
                    let left = walk(left, relation, namespace, values, preparation);
                    let right = walk(right, relation, namespace, values, preparation);
                    format!("({left}) {op} ({right})")
                }
                SupportedFilter::Compare(field, op, value) => {
                    let column = field.sql_expression();
                    let bound = bind(native_literal(value, field.name == "id"));
                    if *op == Operator::Eq && field.atomic {
                        let scope = format!("{namespace}scope{}", preparation.len());
                        preparation.push(format!(
                            "LET ${scope} = {}",
                            crate::prepared::scope_constant(
                                &format!("'{}'", relation.name()),
                                field.name,
                                &bound
                            )
                        ));
                        format!("scope_keys CONTAINS ${scope}")
                    } else {
                        format!(
                            "({column} IS NOT NULL AND {column} IS NOT NONE AND {column} {op} {bound})"
                        )
                    }
                }
                SupportedFilter::In(field, list) => {
                    let items = list
                        .iter()
                        .map(|expr| {
                            native_literal(
                                supported_literal(expr).expect("recognized literal"),
                                field.name == "id",
                            )
                        })
                        .collect::<Vec<_>>();
                    if field.atomic {
                        let predicates = items
                            .chunks(32)
                            .map(|window| {
                                let bound = bind(Value::Array(window.to_vec().into()));
                                let scope = format!("{namespace}scope{}", preparation.len());
                                crate::prepared::prepare_scope(
                                    preparation,
                                    &scope,
                                    &format!("'{}'", relation.name()),
                                    field.name,
                                    &bound,
                                )
                            })
                            .collect::<Vec<_>>();
                        if predicates.is_empty() {
                            "false".into()
                        } else {
                            format!("({})", predicates.join(" OR "))
                        }
                    } else {
                        let column = field.sql_expression();
                        format!(
                            "({column} IS NOT NULL AND {column} IS NOT NONE AND {column} IN {})",
                            bind(Value::Array(items.into()))
                        )
                    }
                }
                SupportedFilter::Null(field, null) => {
                    let field = field.sql_expression();
                    if *null {
                        format!("({field} IS NULL OR {field} IS NONE)")
                    } else {
                        format!("({field} IS NOT NULL AND {field} IS NOT NONE)")
                    }
                }
            }
        }
        let mut values = Vec::new();
        let mut preparation = Vec::new();
        let sql = walk(self, relation, namespace, &mut values, &mut preparation);
        Predicate {
            sql,
            values,
            preparation,
            driver: self.driver(),
        }
    }
}
#[cfg(test)]
fn translate(expr: &Expr, relation: &Relation) -> Option<Predicate> {
    Some(SupportedFilter::interpret(expr, relation)?.render(relation, ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::{
        common::tree_node::{TreeNode, TreeNodeRecursion},
        logical_expr::LogicalPlan,
        prelude::SessionContext,
    };
    use lctx_model::domain::{ContentHash, Record, input::Release, serving::Name};
    use std::collections::BTreeSet;

    #[test]
    fn atomic_lowering_preserves_mandatory_conjuncts_or_and_null_residuals() {
        use datafusion::prelude::{col, lit};
        let relation = Relation::of::<Release>();
        let key = |byte| lit(ScalarValue::FixedSizeBinary(16, Some(vec![byte; 16])));
        let atomic = col("package").eq(key(1));
        let residual = col("version").eq(lit("v"));
        let combined = translate(&atomic.clone().and(residual.clone()), &relation).unwrap();
        assert_eq!(
            combined.driver.as_ref().map(|(field, _)| field.as_str()),
            Some("package")
        );
        assert!(combined.sql.contains("AND"));
        assert!(combined.sql.contains("body.`version`"));
        let disjunction = translate(&atomic.clone().or(residual), &relation).unwrap();
        assert!(disjunction.driver.is_none());
        assert!(disjunction.sql.contains("OR"));
        let two = translate(&atomic.and(col("package").eq(key(2))), &relation).unwrap();
        assert_eq!(two.preparation.len(), 2);
        assert!(two.sql.contains("AND"));
        let many = translate(
            &col("package").in_list((0..40).map(key).collect(), false),
            &relation,
        )
        .unwrap();
        assert_eq!(many.driver.as_ref().unwrap().1.len(), 40);
        assert_eq!(many.preparation.len(), 2);
        let null = translate(&col("package").is_null(), &relation).unwrap();
        assert!(null.driver.is_none());
        assert!(null.sql.contains("IS NULL OR"));
        assert!(null.preparation.is_empty());
    }
    #[test]
    fn supported_interpretation_borrows_operands_and_renders_disjoint_namespaces() {
        use datafusion::prelude::{col, lit};
        let relation = Relation::of::<Release>();
        let supported = [
            ScalarValue::Boolean(Some(true)),
            ScalarValue::Int16(Some(1)),
            ScalarValue::Int32(Some(2)),
            ScalarValue::Int64(Some(3)),
            ScalarValue::Float64(Some(4.5)),
            ScalarValue::Utf8(Some("v0 $scope0".into())),
            ScalarValue::LargeUtf8(Some("alias".into())),
            ScalarValue::FixedSizeBinary(16, Some(vec![7; 16])),
        ];
        for value in supported {
            let expr = col("version").eq(lit(value));
            let demand = SupportedFilter::interpret(&expr, &relation).unwrap();
            let SupportedFilter::Compare(_, _, value) = &demand else {
                panic!("borrowed comparison");
            };
            let Expr::BinaryExpr(binary) = &expr else {
                unreachable!();
            };
            let Expr::Literal(original, _) = binary.right.as_ref() else {
                unreachable!();
            };
            assert!(std::ptr::eq(*value, original));
            assert_eq!(demand.render(&relation, "f10_").values[0].0, "f10_v0");
        }
        for value in [
            ScalarValue::Int8(Some(1)),
            ScalarValue::UInt64(Some(1)),
            ScalarValue::Float32(Some(1.0)),
            ScalarValue::Float64(Some(f64::NAN)),
            ScalarValue::Utf8(None),
        ] {
            assert!(
                SupportedFilter::interpret(&col("version").eq(lit(value)), &relation).is_none()
            );
        }
        assert!(SupportedFilter::interpret(&lit("v").eq(col("version")), &relation).is_none());
        assert!(
            SupportedFilter::interpret(&col("version").eq(col("version")), &relation).is_none()
        );
        assert!(
            SupportedFilter::interpret(&col("version").in_list(vec![lit("v")], true), &relation)
                .is_none()
        );
        let keys = (0..40)
            .map(|byte| lit(ScalarValue::FixedSizeBinary(16, Some(vec![byte; 16]))))
            .collect();
        let expr = col("package")
            .in_list(keys, false)
            .and(col("version").eq(lit("$v0")));
        let interpreted = SupportedFilter::interpret(&expr, &relation).unwrap();
        for prefix in ["f1_", "f10_"] {
            let rendered = interpreted.render(&relation, prefix);
            assert_eq!(rendered.preparation.len(), 2);
            assert!(rendered.sql.contains(&format!("${prefix}scope0")));
            assert!(rendered.sql.contains(&format!("${prefix}scope1")));
            assert!(rendered.sql.contains(&format!("${prefix}v2")));
            assert_eq!(rendered.values[2].1, Value::String("$v0".into()));
        }
        let id = col("id").eq(lit(ScalarValue::FixedSizeBinary(16, Some(vec![1; 16]))));
        assert_eq!(
            SupportedFilter::interpret(&id, &relation)
                .unwrap()
                .render(&relation, "id_")
                .values[0]
                .1,
            Value::String(hex::encode([1; 16]))
        );
    }
    #[tokio::test]
    async fn selected_preparation_is_lazy_charged_and_retained_across_windows() {
        let relation = Relation::of::<Release>();
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let charge = Arc::new(StateCharge::new(&budget, "selected-preparation-control"));
        let keys = vec![
            SelectedKeys {
                keys: Arc::new(vec![[1; 16]; TRANSFER_ROWS + 1]),
                field: None,
                _charge: charge.clone(),
            },
            SelectedKeys {
                keys: Arc::new(vec![[2; 16]]),
                field: Some("package"),
                _charge: charge,
            },
        ];
        let store = NativeCompilerStore::from_existing(
            Arc::new(surrealdb::Surreal::init()),
            Name::new("lazy").unwrap(),
            Name::new("lazy").unwrap(),
        );
        let view = CompletedView::new(
            Release::NAME.into(),
            BTreeSet::from([ContentHash::of(b"lazy")]),
            1,
        )
        .unwrap();
        let stream = selected_batches(
            store,
            view,
            relation.clone(),
            None,
            None,
            keys.clone(),
            budget.clone(),
            1,
            relation.schema().clone(),
        );
        assert_eq!(
            budget.reserved(),
            0,
            "an unpolled stream has no preparation effect"
        );
        drop(stream);
        assert_eq!(budget.reserved(), 0);
        let prepared =
            Arc::new(PreparedSelection::new(&keys, 0, &relation, None, &budget).unwrap());
        let reserved = budget.reserved();
        assert!(reserved > 0);
        let first = prepared.predicate(Some(vec![[1; 16]])).unwrap();
        let second = prepared.predicate(Some(vec![[3; 16]])).unwrap();
        let NativePredicate::KeysSql {
            keys: first_keys,
            sql: first_sql,
            bindings: first_bindings,
            preparation: first_preparation,
        } = first
        else {
            panic!("nominal window");
        };
        let NativePredicate::KeysSql {
            keys: second_keys,
            sql: second_sql,
            bindings: second_bindings,
            preparation: second_preparation,
        } = second
        else {
            panic!("nominal window");
        };
        assert_ne!(first_keys, second_keys);
        assert_eq!(
            (first_sql, first_bindings, first_preparation),
            (second_sql, second_bindings, second_preparation)
        );
        assert_eq!(
            budget.reserved(),
            reserved,
            "invariant preparation remains one charged owner"
        );
        assert!(
            matches!(prepared.predicate(Some(vec![])).unwrap(), NativePredicate::Keys(keys) if keys.is_empty())
        );
        let held = prepared.clone();
        drop(prepared);
        assert_eq!(budget.reserved(), reserved);
        drop(held);
        assert_eq!(budget.reserved(), 0);
    }
    #[tokio::test]
    async fn fetch_delegates_after_native_filtering_without_scan_limit() {
        let store = NativeCompilerStore::from_existing(
            Arc::new(surrealdb::Surreal::init()),
            Name::new("planning").unwrap(),
            Name::new("planning").unwrap(),
        );
        let relation = Relation::of::<Release>();
        let view = CompletedView::new(
            Release::NAME.into(),
            BTreeSet::from([ContentHash::of(b"fetch")]),
            7,
        )
        .unwrap();
        let provider = table_provider(
            store,
            view,
            relation,
            ResourceBudget::fixed(4 << 20).unwrap(),
            128,
        )
        .unwrap();
        let session = SessionContext::new();
        // The provider never applies the TableProvider's advisory limit before exact selected
        // membership/residuals. Physical fetch is delegated only when the optimizer requests it.
        let plan = provider
            .scan(&session.state(), None, &[], Some(1))
            .await
            .unwrap();
        assert_eq!(plan.fetch(), None);
        let limited = plan.with_fetch(Some(1)).unwrap();
        assert_eq!(limited.fetch(), Some(1));
        assert_eq!(limited.schema(), plan.schema());
        let context = datafusion::physical_plan::StatisticsContext::new();
        let args = datafusion::physical_plan::StatisticsArgs::new();
        assert_eq!(
            context.compute(limited.as_ref(), &args).unwrap().num_rows,
            Precision::Exact(1)
        );
        assert_eq!(
            context
                .compute(limited.with_fetch(None).unwrap().as_ref(), &args)
                .unwrap()
                .num_rows,
            Precision::Exact(7)
        );
        assert_eq!(limited.with_fetch(None).unwrap().fetch(), None);
        let unsupported = datafusion::prelude::col("version").like(datafusion::prelude::lit("%x%"));
        let exact = datafusion::prelude::col("version").eq(datafusion::prelude::lit("v"));
        assert_eq!(
            provider
                .supports_filters_pushdown(&[&exact, &unsupported, &exact])
                .unwrap(),
            [
                TableProviderFilterPushDown::Exact,
                TableProviderFilterPushDown::Unsupported,
                TableProviderFilterPushDown::Exact
            ],
            "capability reporting returns exactly one answer per filter"
        );
    }
    #[tokio::test]
    async fn selection_statistics_preserve_conservative_cardinality_and_transfer_rows() {
        let store = NativeCompilerStore::from_existing(Arc::new(surrealdb::Surreal::init()),
            Name::new("statistics").unwrap(), Name::new("statistics").unwrap());
        let relation = Relation::of::<Release>();
        let view = CompletedView::new(Release::NAME.into(),
            [ContentHash::of(b"statistics")].into(), 64).unwrap();
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let provider = table_provider(store, view, relation.clone(), budget.clone(), 7).unwrap();
        assert_eq!(provider.downcast_ref::<NativeTable>().unwrap().batch_rows, 7);
        let selected = SelectedKeys { keys: Arc::new(vec![[1; 16], [2; 16]]), field: None,
            _charge: Arc::new(StateCharge::new(&budget, "statistics-selected-keys")) };
        let field = SelectedKeys { field: Some("package"), ..selected.clone() };
        let empty = SelectedKeys { keys: Arc::new(vec![]), ..selected.clone() };
        assert_eq!(row_statistics(relation.schema(), 64, &[], false).num_rows, Precision::Exact(64));
        assert_eq!(row_statistics(relation.schema(), 64, &[], true).num_rows, Precision::Inexact(64));
        assert_eq!(row_statistics(relation.schema(), 64, &[selected.clone()], false).num_rows, Precision::Inexact(2));
        assert_eq!(row_statistics(relation.schema(), 64, &[field.clone()], false).num_rows, Precision::Inexact(64),
            "non-unique field keys cannot certify a nominal row-count bound");
        assert_eq!(row_statistics(relation.schema(), 64, &[selected, field], true).num_rows, Precision::Inexact(2));
        assert_eq!(row_statistics(relation.schema(), 64, &[empty], true).num_rows, Precision::Inexact(0));
    }
    #[tokio::test]
    async fn native_id_ordering_survives_selection_projection_and_fetch() {
        use datafusion::physical_plan::ExecutionPlanProperties;
        let store = NativeCompilerStore::from_existing(
            Arc::new(surrealdb::Surreal::init()),
            Name::new("ordering").unwrap(),
            Name::new("ordering").unwrap(),
        );
        let relation = Relation::of::<Release>();
        let view = CompletedView::new(
            Release::NAME.into(),
            BTreeSet::from([ContentHash::of(b"ordering")]),
            7,
        )
        .unwrap();
        let budget = ResourceBudget::fixed(4 << 20).unwrap();
        let provider = table_provider(store, view, relation, budget.clone(), 128).unwrap();
        let mut charge = StateCharge::new(&budget, "ordering-key-control");
        charge.grow(16).unwrap();
        let charge = Arc::new(charge);
        let keys = Arc::new(vec![[1; 16]]);
        let nominal = select_table(&provider, keys.clone(), charge.clone())
            .unwrap()
            .unwrap();
        let field = select_field_table(&provider, "package", keys, charge)
            .unwrap()
            .unwrap();
        let session = SessionContext::new();
        for provider in [&provider, &nominal, &field] {
            let plan = provider
                .scan(&session.state(), Some(&vec![1, 0]), &[], None)
                .await
                .unwrap();
            let ordering = plan.output_ordering().unwrap();
            assert_eq!(ordering.len(), 1);
            let column = ordering[0].expr.downcast_ref::<Column>().unwrap();
            assert_eq!((column.name(), column.index()), ("id", 1));
            assert_eq!(
                ordering[0].options,
                SortOptions {
                    descending: false,
                    nulls_first: false
                }
            );
            let limited = plan.with_fetch(Some(2)).unwrap();
            assert_eq!(limited.output_ordering(), plan.output_ordering());
            assert_eq!(
                limited.properties().output_partitioning().partition_count(),
                1
            );
            assert!(
                provider
                    .scan(&session.state(), Some(&vec![1]), &[], None)
                    .await
                    .unwrap()
                    .output_ordering()
                    .is_none()
            );
        }
        session.register_table("releases", provider).unwrap();
        let options = datafusion::execution::context::SQLOptions::new()
            .with_allow_ddl(false)
            .with_allow_dml(false)
            .with_allow_statements(false);
        fn has_sort(plan: &Arc<dyn ExecutionPlan>) -> bool {
            plan.name() == "SortExec" || plan.children().into_iter().any(has_sort)
        }
        for (field, sorted) in [("id", false), ("package", true)] {
            // ast-grep-ignore: sql-through-helper
            let frame = session
                .sql_with_options(
                    &format!("SELECT id,package FROM releases ORDER BY {field} ASC NULLS LAST"),
                    options,
                )
                .await
                .unwrap();
            assert_eq!(
                has_sort(&frame.create_physical_plan().await.unwrap()),
                sorted,
                "only id has guaranteed native order"
            );
        }
    }
    #[tokio::test]
    async fn textual_predicates_push_down_before_projection() {
        // Planning uses the actual provider, but submits no query and requires no server.
        let store = NativeCompilerStore::from_existing(
            Arc::new(surrealdb::Surreal::init()),
            Name::new("planning").unwrap(),
            Name::new("planning").unwrap(),
        );
        let relation = Relation::of::<Release>();
        let view = CompletedView::new(
            Release::NAME.into(),
            BTreeSet::from([ContentHash::of(b"planning")]),
            1,
        )
        .unwrap();
        let provider = table_provider(
            store,
            view,
            relation.clone(),
            ResourceBudget::fixed(4 << 20).unwrap(),
            128,
        )
        .unwrap();
        let session = SessionContext::new();
        session.register_table("releases", provider).unwrap();
        // This foundation cannot depend upward on core's helper. The planning-only control
        // supplies the same read-only restrictions explicitly, with no connected server.
        let options = datafusion::execution::context::SQLOptions::new()
            .with_allow_ddl(false)
            .with_allow_dml(false)
            .with_allow_statements(false);
        // ast-grep-ignore: sql-through-helper
        let frame=session.sql_with_options("SELECT id AS source_id,package AS target_id FROM releases WHERE version='version-0'",options).await.unwrap();
        let plan = frame.clone().into_optimized_plan().unwrap();
        let physical = frame.create_physical_plan().await.unwrap();
        assert_eq!(
            physical
                .schema()
                .fields()
                .iter()
                .map(|field| field.name().as_str())
                .collect::<Vec<_>>(),
            ["source_id", "target_id"]
        );
        let mut scans = 0;
        plan.apply(|plan| {
            match plan {
                LogicalPlan::Filter(_) => {
                    panic!("exact native text predicate must have no residual filter")
                }
                LogicalPlan::TableScan(scan) => {
                    scans += 1;
                    assert_eq!(
                        scan.projection.as_deref(),
                        Some([0, 1].as_slice()),
                        "filter field is omitted from projected payloads"
                    );
                    assert_eq!(scan.filters.len(), 1);
                    let translated = translate(&scan.filters[0], &relation)
                        .expect("native exact textual predicate");
                    assert!(translated.sql.contains("body.`version`"));
                    assert_eq!(
                        translated.values,
                        vec![("v0".into(), Value::String("version-0".into()))]
                    );
                    assert_eq!(
                        scan.source
                            .supports_filters_pushdown(&[&scan.filters[0]])
                            .unwrap(),
                        [TableProviderFilterPushDown::Exact]
                    );
                }
                _ => {}
            }
            Ok(TreeNodeRecursion::Continue)
        })
        .unwrap();
        assert_eq!(scans, 1);
    }
}
