//! Portable execution admission selects each actual owner before rich decoding.
use crate::{consumed_rows::{ClosureTable, NominalClosure, identifier}, scoped_admission::{column, declared, field_target, root_predicate}, workspace::Cancellation};
use datafusion::prelude::SessionContext;
use futures::TryStreamExt;
use lctx_model::domain::{execution::fidelity::ExecutionScope, resources::ResourceBudget, *};

pub(crate) async fn validate_execution(
    invariant:&Invariant, scope:&ExecutionScope, mut tables:Vec<ClosureTable>,
    session:&SessionContext, budget:&ResourceBudget, cancellation:&Cancellation,
) -> Result<(), ModelError> {
    let root = declared(&tables, &invariant.inputs, &scope.root)?;
    // The private root namespace has no reverse rules. A referenced ancestor cannot become
    // another root, nor fan out that ancestor's body or another declaration's arguments.
    let root_namespace = tables.len();
    tables.push(tables[root].clone());
    let mut plan = NominalClosure::new(tables.clone())?;
    for (source, table) in tables.iter().take(invariant.inputs.len()).enumerate() {
        for field in table.relation.fields() {
            let Some((target, _)) = field.target() else {continue;};
            let Some(target) = field_target(&invariant.inputs, source, target)? else {continue;};
            if field.list() {
                plan.pairs(source, target, format!("SELECT id AS source_id, UNNEST({}) AS target_id FROM {}", identifier(field.name()), identifier(&table.alias)))?;
            } else {plan.follow(source, field.name(), target)?;}
        }
    }
    plan.pairs(root_namespace,root,format!("SELECT id AS source_id,id AS target_id FROM {}",identifier(&tables[root].alias)))?;
    for (member, field, owner) in &scope.memberships {
        plan.own(declared(&tables, &invariant.inputs, member)?, field, declared(&tables, &invariant.inputs, owner)?)?;
    }
    let prepared = plan.prepare(session, budget).await?;
    let _roots = budget.reserve("execution-fidelity-root", 128)?;
    let sql = format!("SELECT id FROM {} ORDER BY id", identifier(&tables[root].alias));
    let mut roots = crate::sql::query(session, &sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = roots.try_next().await.map_err(ModelError::codec)? {
        cancellation.check()?;
        for row in 0..batch.num_rows() {
            let id = column(&batch, "id", row)?.ok_or(ModelError::Schema("execution owner root ID"))?;
            let grain = prepared.grain(root_namespace, &root_predicate(&[id]), budget).await?;
            let mut check = (invariant.create)(budget);
            for (index, input) in invariant.inputs.iter().enumerate() {
                let index = if index == root {root_namespace} else {index};
                let order = input.order().iter().map(|field| identifier(field)).collect::<Vec<_>>().join(",");
                let sql = format!("SELECT * FROM ({}) AS owner_rows{}", grain.select(index)?,
                    if order.is_empty() {String::new()} else {format!(" ORDER BY {order}")});
                let mut rows = crate::sql::query(grain.session(), &sql).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
                while let Some(batch) = rows.try_next().await.map_err(ModelError::codec)? {
                    cancellation.check()?; check.visit_input(input, &batch)?;
                }
            }
            check.finish()?;
        }
    }
    Ok(())
}
