//! Canonical representatives of repeated pinned signature observations. Raw facts stay intact;
//! distinct reports may agree on a slot, while conflicting or duplicated facts are not agreement.
use crate::id::Id;
use crate::tables::ContextParametersRow;
use std::collections::{BTreeMap, BTreeSet};

pub fn parameters<'a>(
    rows: impl IntoIterator<Item = &'a ContextParametersRow>,
) -> Result<Vec<&'a ContextParametersRow>, &'static str> {
    let mut facts = BTreeSet::new();
    let mut slots = BTreeMap::new();
    for row in rows {
        if !facts.insert((row.snapshot_id, row.fact_id)) {
            return Err("duplicate parameter fact");
        }
        let key = (
            row.snapshot_id,
            row.module_node_id,
            row.symbol_node_id,
            row.signature_index,
            row.ordinal,
        );
        match slots.get_mut(&key) {
            None => {
                slots.insert(key, row);
            }
            Some(old) => {
                let mut a: ContextParametersRow = (*old).clone();
                let mut b = row.clone();
                a.fact_id = Id::ZERO;
                b.fact_id = Id::ZERO;
                if a != b {
                    return Err("conflicting parameter observations");
                }
                if row.fact_id < old.fact_id {
                    *old = row;
                }
            }
        }
    }
    Ok(slots.into_values().collect())
}

/// SQL consumers need the same slot agreement and representative fact. A conflicting slot is
/// absent, so a complete-signature consumer must withhold its binding, not choose one report.
pub fn parameters_sql() -> String {
    "WITH shapes AS (SELECT snapshot_id,module_node_id,symbol_node_id,signature_index,ordinal,\
       form,kind,name,required,min(fact_id) AS fact_id,count(*) AS observations,count(DISTINCT fact_id) AS distinct_facts FROM context_parameters \
       GROUP BY snapshot_id,module_node_id,symbol_node_id,signature_index,ordinal,form,kind,name,required), \
     counted AS (SELECT *,count(*) OVER (PARTITION BY snapshot_id,module_node_id,symbol_node_id,signature_index,ordinal) AS shapes FROM shapes), \
     agreed AS (SELECT *,min(CASE WHEN shapes=1 AND observations=distinct_facts THEN 1 ELSE 0 END) OVER (PARTITION BY snapshot_id,module_node_id,symbol_node_id) AS valid FROM counted) \
     SELECT snapshot_id,fact_id,symbol_node_id,module_node_id,signature_index,form,ordinal,kind,name,required \
     FROM agreed WHERE valid=1".to_owned()
}
