//! Finite canonical graph records back into request-scoped model kernel batches.
use crate::reader::{NativeReader, canonical_error};
use arrow_array::RecordBatch;
use lctx_model::domain::{
    ModelError, Record,
    graph::{Assertion, Entity},
    resources::{Reservation, ResourceBudget},
};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Bytes, Variables};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalNode {
    pub node_kind: String,
    pub canonical: Bytes,
}
/// Charges both canonical transport and encoded kernel rows until request scope is dropped.
pub struct CanonicalBatches {
    pub batches: Vec<(&'static str, RecordBatch)>,
    _charge: Box<dyn Reservation>,
}
impl NativeReader {
    /// SQL is supplied by the finite operation owner. Request values remain SDK bindings.
    /// Ordinary operation callers select indexed roots and their declared semantic closure.
    pub async fn canonical_batches(
        &self,
        sql: String,
        bindings: Variables,
        budget: &ResourceBudget,
    ) -> Result<CanonicalBatches, ModelError> {
        let nodes: Vec<CanonicalNode> = self.query(sql, bindings).await?;
        let mut charge = budget.reserve(
            "native-kernel-canonical-input",
            nodes
                .iter()
                .map(|n| n.canonical.len() + size_of::<CanonicalNode>())
                .sum(),
        )?;
        let mut entities = Vec::new();
        let mut assertions = Vec::new();
        for node in nodes {
            match node.node_kind.as_str() {
                "entity" => entities.push(
                    serde_json::from_slice::<Entity>(&node.canonical)
                        .map_err(ModelError::codec)
                        .map_err(canonical_error)?,
                ),
                "assertion" => assertions.push(
                    serde_json::from_slice::<Assertion>(&node.canonical)
                        .map_err(ModelError::codec)
                        .map_err(canonical_error)?,
                ),
                _ => return Err(ModelError::Schema("native canonical node kind")),
            }
        }
        let mut entity_groups = std::collections::BTreeMap::<&'static str, Vec<&Entity>>::new();
        macro_rules! entity_group {($($variant:ident:$ty:ty,)*)=>{for entity in &entities {let name=match entity{$(Entity::$variant(_)=><$ty>::NAME,)*};entity_groups.entry(name).or_default().push(entity);}};}
        lctx_model::graph_entity_records!(entity_group);
        let mut assertion_groups = std::collections::BTreeMap::<&str, Vec<&Assertion>>::new();
        for assertion in &assertions {
            let name = assertion
                .source
                .as_ref()
                .ok_or(ModelError::Schema("native kernel assertion source"))?
                .domain();
            assertion_groups.entry(name).or_default().push(assertion);
        }
        let mut batches = Vec::new();
        macro_rules! entity_batches {($($variant:ident:$ty:ty,)*)=>{$({
            let rows:Vec<$ty>=entity_groups.get(<$ty>::NAME).into_iter().flat_map(|rows| rows.iter()).filter_map(|e| match e {Entity::$variant(r)=>Some(r.clone()),_=>None}).collect();
            if !rows.is_empty(){let batch=<$ty>::encode(&rows)?;charge.try_resize(charge.size().saturating_add(lctx_model::domain::logical_batch_bytes(&batch)?))?;batches.push((<$ty>::NAME,batch));}
        })*};}
        lctx_model::graph_entity_records!(entity_batches);
        macro_rules! assertion_batches {($($variant:ident:$ty:ty,)*)=>{$({
            let rows:Vec<$ty>=assertion_groups.get(<$ty>::NAME).into_iter().flat_map(|rows|rows.iter()).map(|a|crate::codec::assertion_record::<$ty>(a).map_err(canonical_error)).collect::<Result<_,_>>()?;
            if !rows.is_empty(){let batch=<$ty>::encode(&rows)?;charge.try_resize(charge.size().saturating_add(lctx_model::domain::logical_batch_bytes(&batch)?))?;batches.push((<$ty>::NAME,batch));}
        })*};}
        lctx_model::graph_assertion_records!(assertion_batches);
        // A selected source cannot disappear through an undeclared dispatch arm.
        if batches.iter().map(|(_, b)| b.num_rows()).sum::<usize>()
            != entities.len() + assertions.len()
        {
            return Err(ModelError::Schema(
                "native selected semantic batch inventory",
            ));
        }
        batches.sort_by_key(|(name, _)| *name);
        Ok(CanonicalBatches {
            batches,
            _charge: charge,
        })
    }
}
