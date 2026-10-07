//! Optimizer-visible projected reads over one exact completed native membership universe.
use crate::{compiler::{NativeCompilerStore, NativePredicate}, projected_arrow::ProjectedBuilder};
use arrow_schema::SchemaRef;
use async_trait::async_trait;
use datafusion::{catalog::{Session, TableProvider}, common::{ScalarValue, Statistics, stats::Precision},
    execution::TaskContext, logical_expr::{Expr, Operator, TableProviderFilterPushDown, TableType},
    physical_plan::{ExecutionPlan, SendableRecordBatchStream, stream::RecordBatchStreamAdapter,
        streaming::{PartitionStream, StreamingTableExec}}};
use lctx_model::domain::{ModelError, Relation, completed::CompletedView, resources::ResourceBudget};
use std::sync::Arc;
use surrealdb::types::{Number, Value, Variables};

pub fn table_provider(store: Arc<NativeCompilerStore>, view: CompletedView, relation: Relation,
    budget: ResourceBudget, batch_rows: usize) -> Result<Arc<dyn TableProvider>, ModelError> {
    view.validate()?;
    if view.relation != relation.name() || batch_rows == 0 { return Err(ModelError::Schema("native compiler table binding")); }
    Ok(Arc::new(NativeTable { store, view, relation, budget, batch_rows }))
}
struct NativeTable { store: Arc<NativeCompilerStore>, view: CompletedView, relation: Relation, budget: ResourceBudget, batch_rows: usize }
impl std::fmt::Debug for NativeTable {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("NativeCompilerTable").field("view",&self.view).finish_non_exhaustive()}
}
#[async_trait]
impl TableProvider for NativeTable {
    fn schema(&self)->SchemaRef {self.relation.schema().clone()}
    fn table_type(&self)->TableType {TableType::Base}
    fn statistics(&self)->Option<Statistics> {
        let mut statistics=Statistics::new_unknown(self.schema().as_ref());
        statistics.num_rows=usize::try_from(self.view.rows).map(Precision::Exact).unwrap_or(Precision::Absent);
        Some(statistics)
    }
    fn supports_filters_pushdown(&self, filters:&[&Expr])->datafusion::error::Result<Vec<TableProviderFilterPushDown>> {
        Ok(filters.iter().map(|expr|if translate(expr,&self.relation).is_some(){TableProviderFilterPushDown::Exact}else{TableProviderFilterPushDown::Unsupported}).collect())
    }
    async fn scan(&self,_state:&dyn Session,projection:Option<&Vec<usize>>,filters:&[Expr],_limit:Option<usize>)->datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        let schema=match projection {Some(projection)=>Arc::new(self.schema().project(projection)?),None=>self.schema()};
        let mut bindings=Variables::new(); let mut predicates=Vec::new();
        for (index,filter) in filters.iter().enumerate() {
            if let Some(mut translated)=translate(filter,&self.relation) {
                // Every translated predicate uses its own value names.
                let prefix=format!("f{index}_");
                for (name,value) in translated.values {bindings.insert(format!("{prefix}{name}"),value);}
                for name in translated.names.into_iter().rev() {translated.sql=translated.sql.replace(&format!("${name}"),&format!("${prefix}{name}"));}
                predicates.push(format!("({})",translated.sql));
            }
        }
        let predicate=(!predicates.is_empty()).then(||NativePredicate::Sql{sql:predicates.join(" AND "),bindings});
        let partition=Arc::new(NativePartition {store:self.store.clone(),view:self.view.clone(),relation:self.relation.clone(),projection:projection.cloned(),schema:schema.clone(),predicate,budget:self.budget.clone(),batch_rows:self.batch_rows});
        // The partition already projects natively. Never layer a rich full-schema read under it.
        Ok(Arc::new(StreamingTableExec::try_new(schema,vec![partition],None,[],false,None)?))
    }
}
struct NativePartition {store:Arc<NativeCompilerStore>,view:CompletedView,relation:Relation,projection:Option<Vec<usize>>,schema:SchemaRef,predicate:Option<NativePredicate>,budget:ResourceBudget,batch_rows:usize}
impl std::fmt::Debug for NativePartition {fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("NativeCompilerPartition").field("view",&self.view.identity).finish_non_exhaustive()}}
impl PartitionStream for NativePartition {
    fn schema(&self)->&SchemaRef {&self.schema}
    fn execute(&self,_context:Arc<TaskContext>)->SendableRecordBatchStream {
        use futures::TryStreamExt;
        let store=self.store.clone();let view=self.view.clone();let relation=self.relation.clone();let projection=self.projection.clone();let predicate=self.predicate.clone();let budget=self.budget.clone();let batch_rows=self.batch_rows;
        let stream=futures::stream::once(async move {scan_batches(store,view,relation,projection,predicate,budget,batch_rows).await.map_err(df_error)}).try_flatten();
        Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(),stream))
    }
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
struct Predicate {sql:String,values:Vec<(String,Value)>,names:Vec<String>}
fn translate(expr:&Expr,relation:&Relation)->Option<Predicate> {
    fn field(expr:&Expr,relation:&Relation)->Option<String>{
        let Expr::Column(column)=expr else{return None;};
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
    fn walk(expr:&Expr,relation:&Relation,values:&mut Vec<(String,Value)>)->Option<String>{
        let mut bind=|value|{let name=format!("v{}",values.len());values.push((name.clone(),value));format!("${name}")};
        match expr {
            Expr::BinaryExpr(binary) if matches!(binary.op,Operator::And|Operator::Or)=>{
                let left=walk(&binary.left,relation,values)?;let right=walk(&binary.right,relation,values)?;
                Some(format!("({left}) {} ({right})",if binary.op==Operator::And{"AND"}else{"OR"}))
            }
            Expr::BinaryExpr(binary) if matches!(binary.op,Operator::Eq|Operator::NotEq|Operator::Lt|Operator::LtEq|Operator::Gt|Operator::GtEq)=>{
                let column=field(&binary.left,relation)?;let value=literal(&binary.right,column=="semantic_key")?;
                let bound=bind(value);
                if binary.op==Operator::Eq && column.starts_with("body.") {
                    let Expr::Column(field)=binary.left.as_ref() else{return None;};
                    if crate::schema::SCOPE_FIELDS.contains(&field.name.as_str()) {
                        return Some(format!("scope_keys CONTAINS ('{}|{}|'+<string>{bound})",relation.name(),field.name));
                    }
                }
                Some(format!("({column} IS NOT NULL AND {column} IS NOT NONE AND {column} {} {bound})",binary.op))
            }
            Expr::InList(list) if !list.negated=>{
                let column=field(&list.expr,relation)?;
                let items=list.list.iter().map(|value|literal(value,column=="semantic_key")).collect::<Option<Vec<_>>>()?;
                if column.starts_with("body.") {
                    let Expr::Column(field)=list.expr.as_ref() else{return None;};
                    if crate::schema::SCOPE_FIELDS.contains(&field.name.as_str()) {
                        let predicates=items.into_iter().map(|item|format!("scope_keys CONTAINS ('{}|{}|'+<string>{})",relation.name(),field.name,bind(item))).collect::<Vec<_>>();
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
    let mut values=Vec::new();let sql=walk(expr,relation,&mut values)?;let names=values.iter().map(|(name,_)|name.clone()).collect();Some(Predicate{sql,values,names})
}
