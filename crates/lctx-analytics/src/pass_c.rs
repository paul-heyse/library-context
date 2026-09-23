//! Pass C (DESIGN §9.3): direct handoffs in the official usage code, over the declared handoffs
//! relation (`cpg_schema::flows::handoffs_sql`).
//!
//! For one seed, every occurrence in which the seed consumes what another release callable
//! returns (or another consumes what the seed returns) is grouped by `(other callable, formal)`.
//! Each group is one `handoff` finding. Its score is the number of occurrences, and its members
//! are the consumer formal and up to `max_occurrences` occurrences: examples first, then doc
//! blocks, then tests, each by module path and position. A finding is `structurally_observed`:
//! the usage code does this; type compatibility alone is never a handoff.

use std::collections::BTreeMap;

use arrow_array::{
    Array, BooleanArray, FixedSizeBinaryArray, Int16Array, Int64Array, RecordBatch, StringArray,
};
use cpg_schema::codebook::{Codebook, CoverageStatus, FindingKind, MemberRole, SourceRole};
use cpg_schema::findings::{
    FINDING_STATUS, FindingMembersRow, FindingsRow, MemberKey, recipe::FindingKey,
};
use cpg_schema::id::Id;

use crate::AnalyticsError;

/// One handoff occurrence (a row of the handoffs relation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    pub consumer: Id,
    pub producer: Id,
    pub formal: Id,
    pub formal_name: String,
    pub path: String,
    pub role: SourceRole,
    pub consumer_start: i64,
    pub consumer_site: Id,
    pub producer_site: Id,
    pub named: bool,
}

/// The relation's rows, in their total order.
#[derive(Debug, Default)]
pub struct Handoffs {
    pub rows: Vec<Handoff>,
}

fn col<'a, T: 'static>(b: &'a RecordBatch, name: &str) -> Result<&'a T, AnalyticsError> {
    b.column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<T>())
        .ok_or_else(|| AnalyticsError::Graph(format!("column {name} has the wrong type")))
}

fn id_at(a: &FixedSizeBinaryArray, i: usize) -> Result<Id, AnalyticsError> {
    if a.is_null(i) {
        return Err(AnalyticsError::Graph(
            "a null id in the handoffs relation".to_owned(),
        ));
    }
    Ok(Id(<[u8; 16]>::try_from(a.value(i)).expect("16 bytes")))
}

impl Handoffs {
    pub fn build(batches: &[RecordBatch]) -> Result<Self, AnalyticsError> {
        let mut rows = Vec::new();
        for b in batches {
            let ids = |n| col::<FixedSizeBinaryArray>(b, n);
            let (consumer, producer, formal, csite, psite) = (
                ids("consumer_node_id")?,
                ids("producer_node_id")?,
                ids("formal_node_id")?,
                ids("consumer_site_node_id")?,
                ids("producer_site_node_id")?,
            );
            let formal_name = col::<StringArray>(b, "formal_name")?;
            let path = col::<StringArray>(b, "path")?;
            let role = col::<Int16Array>(b, "role")?;
            let start = col::<Int64Array>(b, "consumer_start_byte")?;
            let named = col::<BooleanArray>(b, "named")?;
            for i in 0..b.num_rows() {
                rows.push(Handoff {
                    consumer: id_at(consumer, i)?,
                    producer: id_at(producer, i)?,
                    formal: id_at(formal, i)?,
                    formal_name: formal_name.value(i).to_owned(),
                    path: path.value(i).to_owned(),
                    role: SourceRole::from_code(role.value(i))
                        .ok_or_else(|| AnalyticsError::Graph("a handoff's role".to_owned()))?,
                    consumer_start: start.value(i),
                    consumer_site: id_at(csite, i)?,
                    producer_site: id_at(psite, i)?,
                    named: named.value(i),
                });
            }
        }
        Ok(Self { rows })
    }
}

/// Official examples first, then doc blocks, then tests.
pub fn role_rank(role: SourceRole) -> u8 {
    match role {
        SourceRole::Example => 0,
        SourceRole::DocBlock => 1,
        SourceRole::Test => 2,
        SourceRole::Release => 3,
    }
}

/// One seed's Pass C results.
#[derive(Debug, Clone, PartialEq)]
pub struct PassCResult {
    pub completion: CoverageStatus,
    pub occurrences: i64,
    pub findings: Vec<FindingsRow>,
    pub members: Vec<FindingMembersRow>,
}

/// Pass C from one seed.
pub fn run(
    handoffs: &Handoffs,
    seed: Id,
    max_occurrences: usize,
    snapshot_id: Id,
    invocation_id: Id,
) -> Result<PassCResult, AnalyticsError> {
    // (other callable, formal) → occurrences, the seed as consumer or as producer.
    let mut groups: BTreeMap<(Id, Id), Vec<&Handoff>> = BTreeMap::new();
    let mut occurrences = 0i64;
    for h in &handoffs.rows {
        let other = if h.consumer == seed {
            h.producer
        } else if h.producer == seed {
            h.consumer
        } else {
            continue;
        };
        if other == seed {
            continue;
        }
        occurrences += 1;
        groups.entry((other, h.formal)).or_default().push(h);
    }
    let status = FINDING_STATUS
        .iter()
        .find(|(k, _)| *k == FindingKind::Handoff)
        .map(|(_, s)| *s)
        .ok_or_else(|| AnalyticsError::Graph("no status policy for handoff".to_owned()))?;
    let mut findings = Vec::new();
    let mut members = Vec::new();
    for ((other, formal), mut group) in groups {
        group.sort_by(|a, b| {
            (
                role_rank(a.role),
                &a.path,
                a.consumer_start,
                a.consumer_site,
                a.producer_site,
            )
                .cmp(&(
                    role_rank(b.role),
                    &b.path,
                    b.consumer_start,
                    b.consumer_site,
                    b.producer_site,
                ))
        });
        let kept: Vec<&&Handoff> = group.iter().take(max_occurrences.max(1)).collect();
        let mut rows: Vec<(MemberRole, Id, String)> =
            vec![(MemberRole::Formal, formal, group[0].formal_name.clone())];
        for h in &kept {
            rows.push((MemberRole::ProducerSite, h.producer_site, h.path.clone()));
            rows.push((MemberRole::ConsumerSite, h.consumer_site, h.path.clone()));
        }
        let keys: Vec<MemberKey> = rows
            .iter()
            .enumerate()
            .map(|(ordinal, (role, node, label))| MemberKey {
                role: role.code(),
                ordinal: ordinal as i64,
                node: Some(*node),
                cited_fact: None,
                label: Some(label.clone()),
            })
            .collect();
        let omitted = group.len() > kept.len();
        let finding_id = FindingKey {
            finding_kind: FindingKind::Handoff.code(),
            subject: seed,
            related: Some(other),
            condition: None,
            evidence_status: status.code(),
            depth: None,
            stop_reason: None,
            witnesses_omitted: omitted,
            paths: &[],
            members: &keys,
        }
        .id();
        for (ordinal, (role, node, label)) in rows.into_iter().enumerate() {
            members.push(FindingMembersRow {
                snapshot_id,
                finding_id,
                role,
                ordinal: ordinal as i64,
                node_id: Some(node),
                cited_fact_id: None,
                label: Some(label),
                weight: None,
            });
        }
        findings.push(FindingsRow {
            snapshot_id,
            finding_id,
            invocation_id,
            finding_kind: FindingKind::Handoff,
            subject_node_id: seed,
            related_node_id: Some(other),
            evidence_status: status,
            depth: None,
            stop_reason: None,
            witnesses_omitted: omitted,
            score: Some(group.len() as f64),
            condition_node_id: None,
        });
    }
    Ok(PassCResult {
        completion: CoverageStatus::CompleteUnderStatedModel,
        occurrences,
        findings,
        members,
    })
}
