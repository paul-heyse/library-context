//! Shared bounded support-closure validation for file publication, loading and PG import.
use crate::serving_projection::{ProjectionError, corrupt, refused, validate_batch};
use arrow_array::{Array, FixedSizeBinaryArray, Int64Array, RecordBatch, StringArray};
use std::collections::{BTreeMap, BTreeSet};
pub const FILES: &[&str] = &[
    "supports",
    "support_findings",
    "support_witnesses",
    "support_members",
    "support_attributes",
    "support_attribute_incidences",
    "evidence",
];
const LIMIT: usize = 100_000;
#[derive(Clone, Copy)]
struct Row<'a> {
    batch: &'a RecordBatch,
    index: usize,
}
impl<'a> Row<'a> {
    fn id(self, f: &str) -> Option<&'a [u8]> {
        let a = self
            .batch
            .column_by_name(f)
            .unwrap()
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .unwrap();
        (!a.is_null(self.index)).then(|| a.value(self.index))
    }
    fn text(self, f: &str) -> Option<&'a str> {
        let a = self
            .batch
            .column_by_name(f)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        (!a.is_null(self.index)).then(|| a.value(self.index))
    }
    fn int(self, f: &str) -> i64 {
        self.batch
            .column_by_name(f)
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(self.index)
    }
    fn null(self, f: &str) -> bool {
        self.batch.column_by_name(f).unwrap().is_null(self.index)
    }
}
fn rows<'a>(tables: &'a BTreeMap<String, Vec<RecordBatch>>, name: &str) -> Vec<Row<'a>> {
    tables[name]
        .iter()
        .flat_map(|batch| (0..batch.num_rows()).map(move |index| Row { batch, index }))
        .collect()
}

pub fn validate(tables: &BTreeMap<String, Vec<RecordBatch>>) -> Result<(), ProjectionError> {
    for name in FILES {
        let batches = tables
            .get(*name)
            .ok_or_else(|| corrupt(format!("missing {name}")))?;
        if batches.iter().map(RecordBatch::num_rows).sum::<usize>() > LIMIT {
            return Err(refused(format!(
                "{name} exceeds the support projection row limit"
            )));
        }
        for b in batches {
            validate_batch(name, 0, b)?;
        }
    }
    let mut findings = BTreeMap::new();
    for r in rows(tables, "support_findings") {
        if findings.insert(r.id("finding_id"), r).is_some() {
            return Err(corrupt("duplicate support finding"));
        }
    }
    let evidence: BTreeSet<_> = rows(tables, "evidence")
        .iter()
        .map(|r| r.id("evidence_id"))
        .collect();
    let mut cited = BTreeSet::new();
    for s in rows(tables, "supports") {
        let id = s.id("finding_id");
        if id.is_some() {
            cited.insert(id);
            if findings
                .get(&id)
                .is_none_or(|f| f.text("finding_kind") != s.text("finding_kind"))
            {
                return Err(corrupt("assertion support has no matching finding closure"));
            }
        }
        if s.id("evidence_id").is_some() && !evidence.contains(&s.id("evidence_id")) {
            return Err(corrupt("assertion support has no matching evidence"));
        }
    }
    if cited != findings.keys().copied().collect() {
        return Err(corrupt(
            "finding closure differs from served assertion supports",
        ));
    }
    for w in rows(tables, "support_witnesses") {
        if !findings.contains_key(&w.id("finding_id")) {
            return Err(corrupt("witness has no served finding"));
        }
        if ["source_path", "start_byte", "end_byte"]
            .iter()
            .any(|f| w.null(f))
        {
            return Err(corrupt("witness source span is unavailable"));
        }
    }
    let mut attributes = BTreeMap::new();
    for r in rows(tables, "support_attributes") {
        if attributes.insert(r.id("attribute_id"), r).is_some() {
            return Err(corrupt("duplicate support attribute"));
        }
    }
    let mut referenced = BTreeSet::new();
    let mut members: BTreeMap<_, Vec<Row<'_>>> = BTreeMap::new();
    for m in rows(tables, "support_members") {
        if !findings.contains_key(&m.id("finding_id")) {
            return Err(corrupt("member has no served finding"));
        }
        if m.id("cited_fact_id").is_some() && m.text("fact_table").is_none() {
            return Err(corrupt("member cited fact is unavailable"));
        }
        members.entry(m.id("finding_id")).or_default().push(m);
        if m.id("attribute_id").is_some() {
            referenced.insert(m.id("attribute_id"));
            if !attributes.contains_key(&m.id("attribute_id")) {
                return Err(corrupt("member attribute is unavailable"));
            }
        }
    }
    if referenced != attributes.keys().copied().collect() {
        return Err(corrupt("attribute closure differs from served members"));
    }
    let mut required = BTreeSet::new();
    let mut handoff_pairs = BTreeMap::new();
    let mut handoff_formals = BTreeMap::new();
    for (finding, ms) in &members {
        if findings[finding].text("finding_kind") == Some("handoff") {
            let ordered: BTreeMap<_, _> = ms.iter().map(|m| (m.int("ordinal"), m)).collect();
            if ms.len() < 4 || ms.len() % 2 != 0 || ordered.keys().copied().ne(0..ms.len() as i64) {
                return Err(corrupt("handoff members have an invalid order"));
            }
            for (n, m) in ordered {
                let role = if n == 0 {
                    "formal"
                } else if n == ms.len() as i64 - 1 {
                    "handoff_attribute"
                } else if n % 2 == 1 {
                    "producer_site"
                } else {
                    "consumer_site"
                };
                if m.text("role") != Some(role)
                    || (role == "handoff_attribute") != m.id("attribute_id").is_some()
                {
                    return Err(corrupt("handoff members have an invalid role"));
                }
                if role != "handoff_attribute" && m.id("node_id").is_none() {
                    return Err(corrupt("handoff member has no source node"));
                }
            }
        }
        let objects: BTreeSet<_> = ms
            .iter()
            .filter(|m| m.text("role") == Some("extent_member"))
            .map(|m| m.id("node_id"))
            .collect();
        let attrs: BTreeSet<_> = ms
            .iter()
            .filter(|m| {
                m.id("attribute_id").is_some() && m.text("role") != Some("handoff_attribute")
            })
            .map(|m| m.id("attribute_id"))
            .collect();
        if required.len() + objects.len() * attrs.len() > LIMIT {
            return Err(refused(
                "attribute supporter closure exceeds the support budget",
            ));
        }
        for obj in objects {
            for attr in &attrs {
                required.insert((*finding, obj, *attr));
            }
        }
        for m in ms {
            if m.text("role") == Some("handoff_attribute") {
                required.insert((
                    *finding,
                    findings[finding].id("subject_node_id"),
                    m.id("attribute_id"),
                ));
            }
        }
        if required.len() > LIMIT {
            return Err(refused(
                "attribute supporter closure exceeds the support budget",
            ));
        }
        let consumers: BTreeMap<_, _> = ms
            .iter()
            .filter(|m| m.text("role") == Some("consumer_site"))
            .map(|m| (m.int("ordinal"), m.id("node_id")))
            .collect();
        let pairs: BTreeSet<_> = ms
            .iter()
            .filter(|m| m.text("role") == Some("producer_site"))
            .filter_map(|m| {
                consumers
                    .get(&(m.int("ordinal") + 1))
                    .map(|c| (m.id("node_id"), *c))
            })
            .collect();
        if ms
            .iter()
            .any(|m| m.text("role") == Some("handoff_attribute"))
        {
            if pairs.is_empty() {
                return Err(corrupt("handoff attribute has no retained pair"));
            }
            handoff_pairs.insert(*finding, pairs);
            handoff_formals.insert(
                *finding,
                ms.iter()
                    .filter(|m| m.text("role") == Some("formal"))
                    .map(|m| m.id("node_id"))
                    .collect::<BTreeSet<_>>(),
            );
        }
    }
    let mut observed = BTreeSet::new();
    let mut incidence_keys = BTreeSet::new();
    let mut observed_pairs: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    for r in rows(tables, "support_attribute_incidences") {
        if !incidence_keys.insert((r.id("finding_id"), r.id("incidence_id"))) {
            return Err(corrupt("duplicate attribute incidence"));
        }
        let scope = (
            r.id("finding_id"),
            r.id("object_node_id"),
            r.id("attribute_id"),
        );
        if !required.contains(&scope) {
            return Err(corrupt("foreign attribute incidence"));
        }
        observed.insert(scope);
        if ["source_fact_id", "fact_table", "fact_model_id"]
            .iter()
            .any(|f| r.null(f))
        {
            return Err(corrupt("attribute source evidence is unavailable"));
        }
        let calls = ["site_node_id", "edge_id"];
        let pair = [
            "other_site_node_id",
            "other_edge_id",
            "consumer_formal_id",
            "other_fact_id",
            "other_fact_table",
            "other_fact_model_id",
        ];
        match attributes[&r.id("attribute_id")].text("kind") {
            Some("hands_off" | "takes_from") => {
                if calls.iter().chain(pair.iter()).any(|f| r.null(f)) {
                    return Err(corrupt("paired attribute evidence is unavailable"));
                }
            }
            Some("calls") => {
                if calls.iter().any(|f| r.null(f)) || pair.iter().any(|f| !r.null(f)) {
                    return Err(corrupt("call attribute evidence has an invalid shape"));
                }
            }
            Some("parameter" | "parameter_type" | "returns" | "raises" | "decorator") => {
                if calls.iter().chain(pair.iter()).any(|f| !r.null(f)) {
                    return Err(corrupt(
                        "structural attribute evidence has an invalid shape",
                    ));
                }
            }
            _ => return Err(corrupt("unknown attribute kind")),
        }
        let finding = r.id("finding_id");
        if let Some(pairs) = handoff_pairs.get(&finding) {
            let pair = (r.id("site_node_id"), r.id("other_site_node_id"));
            if !pairs.contains(&pair)
                || !handoff_formals[&finding].contains(&r.id("consumer_formal_id"))
            {
                return Err(corrupt(
                    "attribute incidence does not support retained handoff",
                ));
            }
            observed_pairs.entry(finding).or_default().insert(pair);
        }
    }
    if findings
        .iter()
        .filter(|(_, r)| r.text("finding_kind") == Some("handoff"))
        .map(|(k, _)| *k)
        .collect::<BTreeSet<_>>()
        != handoff_pairs.keys().copied().collect()
    {
        return Err(corrupt("handoff has no complete retained pair"));
    }
    if observed_pairs != handoff_pairs {
        return Err(corrupt(
            "missing attribute incidence for retained handoff pair",
        ));
    }
    if observed != required {
        return Err(corrupt("missing attribute incidence for finding supporter"));
    }
    Ok(())
}
