//! Optimizer-visible projected reads over one exact completed native membership universe.
use crate::{compiler::{NativeCompilerStore, NativePredicate}, projected_arrow::ProjectedBuilder};
use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::{catalog::{Session, TableProvider}, common::{ScalarValue, Statistics, stats::Precision},
    execution::TaskContext, logical_expr::{Expr, Operator, TableProviderFilterPushDown, TableType},
    physical_plan::{DisplayAs,DisplayFormatType,ExecutionPlan,PlanProperties, SendableRecordBatchStream, stream::RecordBatchStreamAdapter,
        streaming::{PartitionStream, StreamingTableExec}}};
use lctx_model::domain::{ModelError, Relation, completed::CompletedView, charged::StateCharge, resources::{ResourceBudget,TRANSFER_ROWS}};
use std::sync::Arc;
use surrealdb::types::{Number, Value, Variables};

pub fn table_provider(store: Arc<NativeCompilerStore>, view: CompletedView, relation: Relation,
    budget: ResourceBudget, batch_rows: usize) -> Result<Arc<dyn TableProvider>, ModelError> {
    view.validate()?;
    if view.relation != relation.name() || batch_rows == 0 { return Err(ModelError::Schema("native compiler table binding")); }
    Ok(Arc::new(NativeTable { store, view, relation, budget, batch_rows, keys:vec![] }))
}
#[derive(Clone)]
struct SelectedKeys {keys:Arc<Vec<[u8;16]>>, field:Option<&'static str>, _charge:Arc<StateCharge>}
#[derive(Clone)]
struct NativeTable { keys:Vec<SelectedKeys>, store: Arc<NativeCompilerStore>, view: CompletedView, relation: Relation, budget: ResourceBudget, batch_rows: usize }
/// Bind sorted, distinct nominal keys before payload selection. The existing key owner charge
/// follows providers, physical plans and active streams even after the grain scope is dropped.
/// Detached finite tables return None and continue through the compact local key join.
pub fn select_table(provider:&Arc<dyn TableProvider>,keys:Arc<Vec<[u8;16]>>,charge:Arc<StateCharge>)->Result<Option<Arc<dyn TableProvider>>,ModelError> {
    let Some(source)=provider.downcast_ref::<NativeTable>() else{return Ok(None);};
    if keys.windows(2).any(|pair|pair[0]>=pair[1]){return Err(ModelError::Schema("native selected keys must be sorted and distinct"));}
    let mut selected=source.clone();selected.keys.push(SelectedKeys{keys,field:None,_charge:charge});Ok(Some(Arc::new(selected)))
}
/// One-hop ownership uses a bounded frontier and the relation's declared atomic reference.
/// Larger frontiers are split by the shared closure executor before this binding is created.
pub const REFERENCE_KEYS:usize=1024;
pub fn is_native_table(provider:&Arc<dyn TableProvider>)->bool{provider.downcast_ref::<NativeTable>().is_some()}
pub fn select_field_table(provider:&Arc<dyn TableProvider>,field:&str,keys:Arc<Vec<[u8;16]>>,charge:Arc<StateCharge>)->Result<Option<Arc<dyn TableProvider>>,ModelError>{
    let Some(source)=provider.downcast_ref::<NativeTable>() else{return Ok(None);};
    let descriptor=source.relation.fields().iter().find(|descriptor|descriptor.name()==field).ok_or(ModelError::Schema("native one-hop field"))?;
    if descriptor.target().is_none()||descriptor.list()||!crate::schema::atomic_scope_fields().contains(descriptor.name())||keys.len()>REFERENCE_KEYS||keys.windows(2).any(|pair|pair[0]>=pair[1]){return Err(ModelError::Schema("native one-hop atomic frontier"));}
    let mut selected=source.clone();selected.keys.push(SelectedKeys{keys,field:Some(descriptor.name()),_charge:charge});Ok(Some(Arc::new(selected)))
}
fn row_statistics(schema:&SchemaRef,rows:u64,keys:&[SelectedKeys],filtered:bool)->Statistics {
    let mut statistics=Statistics::new_unknown(schema.as_ref());
    statistics.num_rows=if filtered || keys.iter().any(|selection|selection.field.is_some()) {Precision::Absent} else if keys.is_empty(){usize::try_from(rows).map(Precision::Exact).unwrap_or(Precision::Absent)}else{
        let upper=keys.iter().map(|selection|selection.keys.len()).min().unwrap_or(0);
        Precision::Inexact(usize::try_from(rows).map_or(upper,|rows|rows.min(upper)))
    };
    statistics
}
impl std::fmt::Debug for NativeTable {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("NativeCompilerTable").field("view",&self.view).finish_non_exhaustive()}
}
#[async_trait]
impl TableProvider for NativeTable {
    fn schema(&self)->SchemaRef {self.relation.schema().clone()}
    fn table_type(&self)->TableType {TableType::Base}
    fn statistics(&self)->Option<Statistics> {
        Some(row_statistics(&self.schema(),self.view.rows,&self.keys,false))
    }
    fn supports_filters_pushdown(&self, filters:&[&Expr])->datafusion::error::Result<Vec<TableProviderFilterPushDown>> {
        Ok(filters.iter().map(|expr|if translate(expr,&self.relation).is_some(){TableProviderFilterPushDown::Exact}else{TableProviderFilterPushDown::Unsupported}).collect())
    }
    async fn scan(&self,_state:&dyn Session,projection:Option<&Vec<usize>>,filters:&[Expr],_limit:Option<usize>)->datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        let schema=match projection {Some(projection)=>Arc::new(self.schema().project(projection)?),None=>self.schema()};
        let mut bindings=Variables::new(); let mut predicates=Vec::new();let mut preparation=Vec::new();let mut driver=None;
        for (index,filter) in filters.iter().enumerate() {
            if let Some(mut translated)=translate(filter,&self.relation) {
                if driver.is_none(){driver=translated.driver.take();}
                // Every translated predicate uses its own value names.
                let prefix=format!("f{index}_");
                for (name,value) in translated.values {bindings.insert(format!("{prefix}{name}"),value);}
                for name in translated.names.into_iter().rev() {
                    translated.sql=translated.sql.replace(&format!("${name}"),&format!("${prefix}{name}"));
                    for statement in &mut translated.preparation {*statement=statement.replace(&format!("${name}"),&format!("${prefix}{name}"));}
                }
                preparation.extend(translated.preparation);
                predicates.push(format!("({})",translated.sql));
            }
        }
        let predicate=(!predicates.is_empty()).then(||{
            let sql=predicates.join(" AND ");
            if self.keys.is_empty() && let Some((field,values))=driver {NativePredicate::FieldSql{field,values,sql,bindings,preparation}}
            else{NativePredicate::Sql{sql,bindings,preparation}}
        });
        let partition=Arc::new(NativePartition {store:self.store.clone(),view:self.view.clone(),relation:self.relation.clone(),projection:projection.cloned(),schema:schema.clone(),predicate,keys:self.keys.clone(),budget:self.budget.clone(),batch_rows:self.batch_rows});
        // The partition already projects natively. Never layer a rich full-schema read under it.
        let statistics=Arc::new(row_statistics(&schema,self.view.rows,&self.keys,!filters.is_empty()));
        let inner=StreamingTableExec::try_new(schema,vec![partition],None,[],false,None)?;
        Ok(Arc::new(NativeExec{inner,statistics}))
    }
}
struct NativePartition {store:Arc<NativeCompilerStore>,view:CompletedView,relation:Relation,projection:Option<Vec<usize>>,schema:SchemaRef,predicate:Option<NativePredicate>,keys:Vec<SelectedKeys>,budget:ResourceBudget,batch_rows:usize}
impl std::fmt::Debug for NativePartition {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("NativeCompilerPartition").field("view",&self.view.identity).finish_non_exhaustive()}}
impl PartitionStream for NativePartition {
    fn schema(&self)->&SchemaRef {&self.schema}
    fn execute(&self,_context:Arc<TaskContext>)->SendableRecordBatchStream {
        use futures::TryStreamExt;
        let store=self.store.clone();let view=self.view.clone();let relation=self.relation.clone();let projection=self.projection.clone();let predicate=self.predicate.clone();let budget=self.budget.clone();let batch_rows=self.batch_rows;
        let keys=self.keys.clone();
        if keys.is_empty(){
            let stream=futures::stream::once(async move {scan_batches(store,view,relation,projection,predicate,budget,batch_rows).await.map_err(df_error)}).try_flatten();
            Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(),stream)) as SendableRecordBatchStream
        }else{
            selected_batches(store,view,relation,projection,predicate,keys,budget,batch_rows,self.schema.clone())
        }
    }
}
/// A source leaf forwards streaming behavior and publishes the same view/selection cardinality
/// at the physical optimizer boundary. StreamingTableExec has no statistics setter at DF55.1.
#[derive(Debug)]
struct NativeExec {inner:StreamingTableExec,statistics:Arc<Statistics>}
impl DisplayAs for NativeExec {fn fmt_as(&self,t:DisplayFormatType,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{self.inner.fmt_as(t,f)}}
impl ExecutionPlan for NativeExec {
    fn name(&self)->&'static str{"NativeCompilerExec"}
    fn properties(&self)->&Arc<PlanProperties>{self.inner.properties()}
    fn children(&self)->Vec<&Arc<dyn ExecutionPlan>>{vec![]}
    fn apply_expressions(&self,f:&mut dyn FnMut(&Arc<dyn datafusion::physical_expr::PhysicalExpr>)->datafusion::error::Result<datafusion::common::tree_node::TreeNodeRecursion>)->datafusion::error::Result<datafusion::common::tree_node::TreeNodeRecursion>{self.inner.apply_expressions(f)}
    fn with_new_children(self:Arc<Self>,children:Vec<Arc<dyn ExecutionPlan>>)->datafusion::error::Result<Arc<dyn ExecutionPlan>>{
        if children.is_empty(){Ok(self)}else{Err(datafusion::error::DataFusionError::Internal("native compiler source has no children".into()))}
    }
    fn execute(&self,partition:usize,context:Arc<TaskContext>)->datafusion::error::Result<SendableRecordBatchStream>{self.inner.execute(partition,context)}
    fn partition_statistics(&self,partition:Option<usize>)->datafusion::error::Result<Arc<Statistics>>{
        if partition.is_some_and(|partition|partition!=0){return Err(datafusion::error::DataFusionError::Internal("native compiler partition absent".into()));}Ok(self.statistics.clone())
    }
    fn metrics(&self)->Option<datafusion::physical_plan::metrics::MetricsSet>{self.inner.metrics()}
}
#[allow(clippy::too_many_arguments,reason="Selected native streams retain the exact view, projection, static predicates and existing key owner")]
fn selected_batches(store:Arc<NativeCompilerStore>,view:CompletedView,relation:Relation,projection:Option<Vec<usize>>,predicate:Option<NativePredicate>,keys:Vec<SelectedKeys>,budget:ResourceBudget,batch_rows:usize,schema:SchemaRef)->SendableRecordBatchStream {
    use futures::{StreamExt,TryStreamExt};
    // Every selection is sorted and distinct; disjoint windows preserve native semantic-key
    // order. Choose the smallest demand and intersect any nested selection before transfer.
    let driver=keys.iter().enumerate().filter(|(_,selection)|selection.field.is_none()).min_by_key(|(_,selection)|selection.keys.len()).map(|(index,_)|index).unwrap_or(0);
    let windows=futures::stream::try_unfold((0usize,false),move |(offset,done)|{
        let store=store.clone();let view=view.clone();let relation=relation.clone();let projection=projection.clone();let predicate=predicate.clone();let keys=keys.clone();let budget=budget.clone();
        async move {
            if done{return Ok::<_,datafusion::error::DataFusionError>(None);}
            let end=if keys[driver].field.is_some(){keys[driver].keys.len()}else{offset.saturating_add(TRANSFER_ROWS).min(keys[driver].keys.len())};
            let field_keys=keys.iter().filter(|selection|selection.field.is_some()).map(|selection|selection.keys.len()).sum::<usize>();
            let transfer=budget.reserve("native-selected-key-transfer",(end-offset).saturating_mul(192).saturating_add(field_keys.saturating_mul(1536)).saturating_add(4096)).map_err(df_error)?;
            let mut bindings=Variables::new();let mut predicates=Vec::new();let mut preparation=Vec::new();let mut nominal=None;let mut field_driver=None;
            let mut empty=keys.iter().any(|selection|selection.keys.is_empty());
            if keys[driver].field.is_none(){
                let selected=keys[driver].keys[offset..end].iter().filter(|key|keys.iter().filter(|selection|selection.field.is_none()).all(|selection|selection.keys.binary_search(key).is_ok())).copied().collect::<Vec<_>>();
                empty|=selected.is_empty();nominal=Some(selected);
            }
            for (index,selection) in keys.iter().enumerate(){
                let Some(field)=selection.field else{continue;};
                let name=format!("closure_selected_fields_{index}");
                let values=selection.keys.iter().map(|key|Value::Array(key.iter().map(|byte|Value::Number(Number::Int(i64::from(*byte)))).collect())).collect::<Vec<_>>();
                if index==driver {field_driver=Some((field.to_string(),values));continue;}
                bindings.insert(name.clone(),values);
                let scope=format!("closure_selected_scope_{index}");
                preparation.push(format!("LET ${scope} = ${name}.map(|$value| '{}|{field}|'+<string>$value)",relation.name()));
                predicates.push(format!("scope_keys CONTAINSANY ${scope}"));
            }
            match predicate {
                Some(NativePredicate::Sql{sql,bindings:static_bindings,preparation:static_preparation})=>{bindings.extend(static_bindings);preparation.extend(static_preparation);predicates.push(format!("({sql})"));},
                None=>{},
                _=>return Err(df_error(ModelError::Schema("native table static predicate"))),
            }
            let sql=if predicates.is_empty(){"true".into()}else{predicates.join(" AND ")};
            // Even empty demand checks the exact view/pin through the ordinary native reader.
            let predicate=if empty {NativePredicate::Keys(vec![])}else if let Some(keys)=nominal {NativePredicate::KeysSql{keys,sql,bindings,preparation}}else if let Some((field,values))=field_driver {NativePredicate::FieldSql{field,values,sql,bindings,preparation}}else{return Err(df_error(ModelError::Schema("native selected driver")));};
            let rows=scan_batches(store,view,relation,projection,Some(predicate),budget,batch_rows).await.map_err(df_error)?;
            let rows=rows.map(move |batch|{let _held=&transfer;batch});
            Ok(Some((rows,(end,end==keys[driver].keys.len()))))
        }
    }).try_flatten();
    Box::pin(RecordBatchStreamAdapter::new(schema,windows))
}
pub async fn scan_batches(store:Arc<NativeCompilerStore>,view:CompletedView,relation:Relation,
    projection:Option<Vec<usize>>,predicate:Option<NativePredicate>,budget:ResourceBudget,batch_rows:usize)->Result<SendableRecordBatchStream,ModelError> {
    if batch_rows==0 {return Err(ModelError::Schema("native Arrow batch rows"));}
    let schema=match &projection {Some(projection)=>Arc::new(relation.schema().project(projection).map_err(ModelError::codec)?),None=>relation.schema().clone()};
    let columns=schema.fields().iter().map(|field|field.name().clone()).collect::<Vec<_>>();
    let rows=store.scan_rows(&view,&relation,Some(&columns),predicate).await?;
    let builder=ProjectedBuilder::new(relation,schema.clone(),&budget)?;
    let stream=futures::stream::try_unfold((rows,builder,false),move |(mut rows,mut builder,done)|async move {
        if done {return Ok(None);}
        builder.release().map_err(df_error)?;
        loop {
            match rows.next().await.map_err(df_error)? {
                Some(row)=>builder.push(row).map_err(df_error)?,
                None=>{
                    if builder.rows()==0 {return Ok(None);}
                    let batch=builder.finish().map_err(df_error)?;
                    return Ok(Some((batch,(rows,builder,true))));
                }
            }
            if builder.rows()>=batch_rows || builder.bytes()>=lctx_model::domain::resources::TRANSFER_BYTES {
                let batch=builder.finish().map_err(df_error)?;
                return Ok(Some((batch,(rows,builder,false))));
            }
        }
    });
    Ok(Box::pin(RecordBatchStreamAdapter::new(schema,stream)))
}
fn df_error(error:ModelError)->datafusion::error::DataFusionError {datafusion::error::DataFusionError::External(Box::new(error))}
struct Predicate {sql:String,values:Vec<(String,Value)>,names:Vec<String>,preparation:Vec<String>,driver:Option<(String,Vec<Value>)>}
fn translate(expr:&Expr,relation:&Relation)->Option<Predicate> {
    fn field(expr:&Expr,relation:&Relation)->Option<String>{
        let Expr::Column(column)=expr else{return None;};
        if relation.name()==<lctx_model::domain::artifact::ArtifactChunk as lctx_model::domain::Record>::NAME && column.name=="body" {return None;}
        relation.schema().field_with_name(&column.name).ok()?;
        if column.name=="id" {Some("semantic_key".into())} else {Some(format!("body.`{}`",column.name.replace('`',"``")))}
    }
    fn literal(expr:&Expr,id:bool)->Option<Value>{
        let Expr::Literal(value,_)=expr else{return None;};
        Some(match value {
            ScalarValue::Boolean(Some(value))=>Value::Bool(*value),
            ScalarValue::Int16(Some(value))=>Value::Number(Number::Int(i64::from(*value))),
            ScalarValue::Int32(Some(value))=>Value::Number(Number::Int(i64::from(*value))),
            ScalarValue::Int64(Some(value))=>Value::Number(Number::Int(*value)),
            ScalarValue::Float64(Some(value)) if value.is_finite()=>Value::Number(Number::Float(*value)),
            ScalarValue::Utf8(Some(value))|ScalarValue::LargeUtf8(Some(value))=>Value::String(value.clone()),
            ScalarValue::FixedSizeBinary(_,Some(value))=>if id {Value::String(hex::encode(value))}else{Value::Array(value.iter().map(|byte|Value::Number(Number::Int(i64::from(*byte)))).collect())},
            _=>return None,
        })
    }
    // Only a mandatory conjunct can narrow the candidate universe independently of the full
    // residual expression. An OR branch alone cannot establish an exact atomic field demand.
    fn driver(expr:&Expr,relation:&Relation)->Option<(String,Vec<Value>)>{
        match expr {
            Expr::BinaryExpr(binary) if binary.op==Operator::And=>driver(&binary.left,relation).or_else(||driver(&binary.right,relation)),
            Expr::BinaryExpr(binary) if binary.op==Operator::Eq=>{
                field(&binary.left,relation)?;
                let Expr::Column(column)=binary.left.as_ref() else{return None;};
                if !crate::schema::atomic_scope_fields().contains(column.name.as_str()){return None;}
                Some((column.name.clone(),vec![literal(&binary.right,false)?]))
            },
            Expr::InList(list) if !list.negated=>{
                field(&list.expr,relation)?;
                let Expr::Column(column)=list.expr.as_ref() else{return None;};
                if !crate::schema::atomic_scope_fields().contains(column.name.as_str()){return None;}
                let values=list.list.iter().map(|value|literal(value,false)).collect::<Option<Vec<_>>>()?;
                Some((column.name.clone(),values))
            },
            _=>None,
        }
    }
    fn walk(expr:&Expr,relation:&Relation,values:&mut Vec<(String,Value)>,preparation:&mut Vec<String>)->Option<String>{
        let mut bind=|value|{let name=format!("v{}",values.len());values.push((name.clone(),value));format!("${name}")};
        match expr {
            Expr::BinaryExpr(binary) if matches!(binary.op,Operator::And|Operator::Or)=>{
                let left=walk(&binary.left,relation,values,preparation)?;let right=walk(&binary.right,relation,values,preparation)?;
                Some(format!("({left}) {} ({right})",if binary.op==Operator::And{"AND"}else{"OR"}))
            }
            Expr::BinaryExpr(binary) if matches!(binary.op,Operator::Eq|Operator::NotEq|Operator::Lt|Operator::LtEq|Operator::Gt|Operator::GtEq)=>{
                let column=field(&binary.left,relation)?;let value=literal(&binary.right,column=="semantic_key")?;
                let bound=bind(value);
                if binary.op==Operator::Eq && column.starts_with("body.") {
                    let Expr::Column(field)=binary.left.as_ref() else{return None;};
                    if crate::schema::atomic_scope_fields().contains(field.name.as_str()) {
                        let scope=format!("scope{}",preparation.len());
                        preparation.push(format!("LET ${scope} = '{}|{}|'+<string>{bound}",relation.name(),field.name));
                        return Some(format!("scope_keys CONTAINS ${scope}"));
                    }
                }
                Some(format!("({column} IS NOT NULL AND {column} IS NOT NONE AND {column} {} {bound})",binary.op))
            }
            Expr::InList(list) if !list.negated=>{
                let column=field(&list.expr,relation)?;
                let items=list.list.iter().map(|value|literal(value,column=="semantic_key")).collect::<Option<Vec<_>>>()?;
                if column.starts_with("body.") {
                    let Expr::Column(field)=list.expr.as_ref() else{return None;};
                    if crate::schema::atomic_scope_fields().contains(field.name.as_str()) {
                        let predicates=items.chunks(32).map(|window|{
                            let bound=bind(Value::Array(window.to_vec().into()));
                            let scope=format!("scope{}",preparation.len());
                            preparation.push(format!("LET ${scope} = {bound}.map(|$value| '{}|{}|'+<string>$value)",relation.name(),field.name));
                            format!("scope_keys CONTAINSANY ${scope}")
                        }).collect::<Vec<_>>();
                        return Some(if predicates.is_empty(){"false".into()}else{format!("({})",predicates.join(" OR "))});
                    }
                }
                Some(format!("({column} IS NOT NULL AND {column} IS NOT NONE AND {column} IN {})",bind(Value::Array(items.into()))))
            }
            Expr::IsNull(expr)=>{let field=field(expr,relation)?;Some(format!("({field} IS NULL OR {field} IS NONE)"))},
            Expr::IsNotNull(expr)=>{let field=field(expr,relation)?;Some(format!("({field} IS NOT NULL AND {field} IS NOT NONE)"))},
            _=>None,
        }
    }
    let mut values=Vec::new();let mut preparation=Vec::new();let sql=walk(expr,relation,&mut values,&mut preparation)?;
    let names=values.iter().map(|(name,_)|name.clone()).chain((0..preparation.len()).map(|index|format!("scope{index}"))).collect();
    Some(Predicate{sql,values,names,preparation,driver:driver(expr,relation)})
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::{common::tree_node::{TreeNode, TreeNodeRecursion}, logical_expr::LogicalPlan, prelude::SessionContext};
    use lctx_model::domain::{ContentHash, Record, input::Release, serving::Name};
    use std::collections::BTreeSet;

    #[tokio::test]
    async fn textual_predicates_push_down_before_projection() {
        // Planning uses the actual provider, but submits no query and requires no server.
        let store=NativeCompilerStore::from_existing(Arc::new(surrealdb::Surreal::init()),Name::new("planning").unwrap(),Name::new("planning").unwrap());
        let relation=Relation::of::<Release>();
        let view=CompletedView::new(Release::NAME.into(),BTreeSet::from([ContentHash::of(b"planning")]),1).unwrap();
        let provider=table_provider(store,view,relation.clone(),ResourceBudget::fixed(4<<20).unwrap(),128).unwrap();
        let session=SessionContext::new();session.register_table("releases",provider).unwrap();
        // This foundation cannot depend upward on core's helper. The planning-only control
        // supplies the same read-only restrictions explicitly, with no connected server.
        let options=datafusion::execution::context::SQLOptions::new()
            .with_allow_ddl(false).with_allow_dml(false).with_allow_statements(false);
        // ast-grep-ignore: sql-through-helper
        let frame=session.sql_with_options("SELECT id AS source_id,package AS target_id FROM releases WHERE version='version-0'",options).await.unwrap();
        let plan=frame.clone().into_optimized_plan().unwrap();
        let physical=frame.create_physical_plan().await.unwrap();
        assert_eq!(physical.schema().fields().iter().map(|field|field.name().as_str()).collect::<Vec<_>>(),["source_id","target_id"]);
        let mut scans=0;
        plan.apply(|plan| {
            match plan {
                LogicalPlan::Filter(_)=>panic!("exact native text predicate must have no residual filter"),
                LogicalPlan::TableScan(scan)=>{
                    scans+=1;
                    assert_eq!(scan.projection.as_deref(),Some([0,1].as_slice()),"filter field is omitted from projected payloads");
                    assert_eq!(scan.filters.len(),1);
                    let translated=translate(&scan.filters[0],&relation).expect("native exact textual predicate");
                    assert!(translated.sql.contains("body.`version`"));
                    assert_eq!(translated.values,vec![("v0".into(),Value::String("version-0".into()))]);
                    assert_eq!(scan.source.supports_filters_pushdown(&[&scan.filters[0]]).unwrap(),[TableProviderFilterPushDown::Exact]);
                },
                _=>{},
            }
            Ok(TreeNodeRecursion::Continue)
        }).unwrap();
        assert_eq!(scans,1);
    }
}
