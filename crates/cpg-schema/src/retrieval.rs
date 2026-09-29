//! Finite retrieval views and addressable immutable renderings (ADR-0077).
use crate::{Digest, Id, IdHasher, evidence::EvidenceRef, wire::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod catalog;

pub const VIEW_REVISION: u32 = 1;
pub const RENDER_REVISION: u32 = 1;
pub const FUSION_REVISION: u32 = 1;
pub const RRF_K: f64 = 60.0;
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    ApiOptions,
    DocumentationDeployment,
    Scenario,
    Source,
}
#[derive(Debug, Clone, Copy)]
pub struct View {
    pub family: Family,
    pub name: &'static str,
    pub description: &'static str,
}
pub const VIEWS: &[View] = &[
    View {
        family: Family::ApiOptions,
        name: "api_options",
        description: "Declared APIs, signatures, parameters and configuration fields",
    },
    View {
        family: Family::Source,
        name: "source",
        description: "Original implementation bodies",
    },
    View {
        family: Family::Scenario,
        name: "scenario",
        description: "Original enclosing usage scenarios with explicit intent and checks",
    },
    View {
        family: Family::DocumentationDeployment,
        name: "documentation_deployment",
        description: "Original documentation and release-scoped deployment declarations",
    },
];
impl Family {
    pub fn name(self) -> &'static str {
        VIEWS
            .iter()
            .find(|v| v.family == self)
            .expect("finite family")
            .name
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Subject {
    Member(PublicMemberId),
    Release(Id),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Anchor {
    Original { evidence: EvidenceRef },
    Catalog { evidence: EvidenceId },
    Declaration { fact_id: Id },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Unit {
    pub unit_id: RetrievalUnitId,
    pub source_key: Id,
    pub family: Family,
    pub subjects: Vec<Subject>,
    pub anchors: Vec<Anchor>,
    pub title: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Fragment {
    pub fragment_id: Id,
    pub unit_id: RetrievalUnitId,
    pub family: Family,
    pub ordinal: u32,
    pub rendering_revision: u32,
    pub text: String,
    pub content_digest: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RetrievalReceipt {
    pub snapshot_id: SnapshotId,
    pub input_digest: Digest,
    pub view_revision: u32,
    pub render_revision: u32,
    pub spec_hash: Option<Digest>,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Lexical,
    Vector,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Winner {
    pub member_id: PublicMemberId,
    pub family: Family,
    pub channel: Channel,
    pub unit_id: RetrievalUnitId,
    pub fragment_id: Id,
    pub score: f64,
    pub rank: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RankedMember {
    pub member_id: PublicMemberId,
    pub score: f64,
    pub promoted: bool,
    pub winners: Vec<Winner>,
}

pub fn unit_id(family: Family, source_key: Id) -> RetrievalUnitId {
    RetrievalUnitId::from_storage(
        IdHasher::new("retrieval-unit-v1")
            .str(family.name())
            .id(source_key)
            .finish_id(),
    )
}
/// Rendering identity is independent of the embedding spec and of row/rank order.
pub fn fragments(unit: &Unit, window: usize) -> Result<Vec<Fragment>, WireError> {
    if window < 4 {
        return Err(WireError("invalid fragment window".into()));
    }
    let mut start = 0;
    let mut out = vec![];
    while start < unit.text.len() {
        let mut end = (start + window).min(unit.text.len());
        while !unit.text.is_char_boundary(end) {
            end -= 1;
        }
        let text = unit.text[start..end].to_owned();
        let ordinal = out.len() as u32;
        let content_digest = IdHasher::new("retrieval-content")
            .str(&text)
            .finish_digest();
        let fragment_id = IdHasher::new("retrieval-fragment-v1")
            .id(unit.unit_id.storage())
            .i64(RENDER_REVISION.into())
            .i64(ordinal.into())
            .digest_field(content_digest)
            .finish_id();
        out.push(Fragment {
            fragment_id,
            unit_id: unit.unit_id,
            family: unit.family,
            ordinal,
            rendering_revision: RENDER_REVISION,
            text,
            content_digest,
        });
        start = end;
    }
    Ok(out)
}
/// Collapse duplicates before ranks. Every retained winner belongs to the selected member.
pub fn channel_winners(mut rows: Vec<Winner>) -> Result<Vec<Winner>, WireError> {
    if rows.len() > 200_000 || rows.iter().any(|r| !r.score.is_finite()) {
        return Err(WireError(
            "resource_refused or invalid retrieval scores".into(),
        ));
    }
    rows.sort_by(|a, b| {
        (a.family, a.channel, a.member_id)
            .cmp(&(b.family, b.channel, b.member_id))
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| (a.unit_id, a.fragment_id).cmp(&(b.unit_id, b.fragment_id)))
    });
    rows.dedup_by(|a, b| (a.family, a.channel, a.member_id) == (b.family, b.channel, b.member_id));
    rows.sort_by(|a, b| {
        (a.family, a.channel)
            .cmp(&(b.family, b.channel))
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.member_id.cmp(&b.member_id))
    });
    let mut previous = None;
    let mut rank = 0;
    for r in &mut rows {
        let key = (r.family, r.channel);
        if previous != Some(key) {
            previous = Some(key);
            rank = 0;
        }
        rank += 1;
        r.rank = rank;
    }
    Ok(rows)
}
/// Independent Rust policy oracle and native validator. Python uses the same bounded policy.
pub fn fuse(rows: &[Winner], promoted: &BTreeSet<PublicMemberId>) -> Vec<RankedMember> {
    let mut family_scores: BTreeMap<(Family, PublicMemberId), f64> = BTreeMap::new();
    let mut witnesses = BTreeMap::<PublicMemberId, Vec<Winner>>::new();
    for row in rows {
        witnesses
            .entry(row.member_id)
            .or_default()
            .push(row.clone());
        *family_scores
            .entry((row.family, row.member_id))
            .or_default() += 1.0 / (RRF_K + f64::from(row.rank));
    }
    let mut ranked: Vec<_> = family_scores.into_iter().collect();
    ranked.sort_by(|((fa, ma), sa), ((fb, mb), sb)| {
        fa.cmp(fb)
            .then_with(|| sb.total_cmp(sa))
            .then_with(|| ma.cmp(mb))
    });
    let mut scores: BTreeMap<PublicMemberId, f64> = BTreeMap::new();
    let mut previous = None;
    let mut rank = 0;
    for ((family, member), _) in ranked {
        if previous != Some(family) {
            previous = Some(family);
            rank = 0;
        }
        rank += 1;
        *scores.entry(member).or_default() += 1.0 / (RRF_K + f64::from(rank));
    }
    for member in promoted {
        scores.entry(*member).or_default();
    }
    let mut out: Vec<_> = scores
        .into_iter()
        .map(|(member_id, score)| RankedMember {
            member_id,
            score,
            promoted: promoted.contains(&member_id),
            winners: witnesses.remove(&member_id).unwrap_or_default(),
        })
        .collect();
    out.sort_by(|a, b| {
        b.promoted
            .cmp(&a.promoted)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.member_id.cmp(&b.member_id))
    });
    out
}
pub fn files(dimensions: i32) -> Vec<crate::bundle::ServingFile> {
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    let id = |n| Field::new(n, DataType::FixedSizeBinary(16), false);
    let text = |n| Field::new(n, DataType::Utf8, false);
    let file = |name, fields, key| crate::bundle::ServingFile {
        name,
        schema: Arc::new(Schema::new(fields)),
        key,
    };
    vec![
        file(
            "retrieval_units",
            vec![id("unit_id"), text("family"), text("detail")],
            &["unit_id"],
        ),
        file(
            "retrieval_subjects",
            vec![
                id("unit_id"),
                Field::new("member_id", DataType::FixedSizeBinary(16), true),
                Field::new("release_id", DataType::FixedSizeBinary(16), true),
            ],
            &["unit_id", "member_id", "release_id"],
        ),
        file(
            "retrieval_fragments",
            vec![
                id("fragment_id"),
                id("unit_id"),
                text("family"),
                text("text"),
                Field::new("content_digest", DataType::FixedSizeBinary(32), false),
                text("embedding_status"),
            ],
            &["fragment_id"],
        ),
        file(
            "retrieval_vectors",
            vec![
                id("fragment_id"),
                Field::new("input_hash", DataType::FixedSizeBinary(32), false),
                Field::new("vector", crate::bundle::vector_type(dimensions), false),
            ],
            &["fragment_id"],
        ),
        file("retrieval_receipt", vec![text("detail")], &["detail"]),
    ]
}
pub fn rank_schema() -> arrow_schema::SchemaRef {
    use arrow_schema::{DataType, Field, Schema};
    std::sync::Arc::new(Schema::new(vec![
        Field::new("member_id", DataType::FixedSizeBinary(16), false),
        Field::new("family", DataType::Utf8, false),
        Field::new("channel", DataType::Utf8, false),
        Field::new("unit_id", DataType::FixedSizeBinary(16), false),
        Field::new("fragment_id", DataType::FixedSizeBinary(16), false),
        Field::new("rank", DataType::UInt32, false),
        Field::new("score", DataType::Float64, false),
    ]))
}

/// One validator is used by publication, import and read-back. Rendering is not evidence proof.
pub fn validate_projection(
    tables: &BTreeMap<String, Vec<arrow_array::RecordBatch>>,
) -> Result<(), crate::serving_projection::ProjectionError> {
    use crate::serving_projection::{corrupt, projected_rows as read};
    use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch, StringArray};
    let string = |b: &RecordBatch,
                  name: &str,
                  row|
     -> Result<String, crate::serving_projection::ProjectionError> {
        Ok(b.column_by_name(name)
            .and_then(|a| a.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| corrupt("retrieval text column"))?
            .value(row)
            .to_owned())
    };
    let id = |b: &RecordBatch,
              name: &str,
              row|
     -> Result<Option<Id>, crate::serving_projection::ProjectionError> {
        let a = b
            .column_by_name(name)
            .and_then(|a| a.as_any().downcast_ref::<FixedSizeBinaryArray>())
            .ok_or_else(|| corrupt("retrieval identity column"))?;
        if a.is_null(row) {
            return Ok(None);
        }
        Ok(Some(Id(a
            .value(row)
            .try_into()
            .map_err(|_| corrupt("retrieval identity width"))?)))
    };
    let members: BTreeSet<_> = read::<crate::catalog::CatalogMembers>(tables)?
        .iter()
        .map(|m| m.member_id)
        .collect();
    let bindings = read::<crate::catalog::CatalogBindings>(tables)?;
    let signatures = read::<crate::catalog::CatalogSignatures>(tables)?;
    let fields = read::<crate::catalog::CatalogConfigurations>(tables)?;
    let declaration_facts: BTreeSet<_> = bindings
        .iter()
        .map(|b| b.source_fact_id)
        .chain(signatures.iter().map(|s| s.source_fact_id))
        .chain(fields.iter().map(|f| f.source_fact_id))
        .collect();
    let releases: BTreeSet<_> = read::<crate::evidence::CatalogArtifacts>(tables)?
        .iter()
        .map(|a| a.release_id)
        .collect();
    let evidence: BTreeSet<_> = read::<crate::catalog::CatalogEvidence>(tables)?
        .iter()
        .map(|e| e.evidence_id)
        .collect();
    let spans: BTreeSet<_> = read::<crate::evidence::CatalogSpans>(tables)?
        .iter()
        .map(|s| s.span_id)
        .collect();
    let scenarios: BTreeSet<_> = read::<crate::evidence::CatalogScenarios>(tables)?
        .iter()
        .map(|s| s.scenario_id)
        .collect();
    let deployments: BTreeSet<_> = read::<crate::evidence::CatalogDeployments>(tables)?
        .iter()
        .map(|s| s.deployment_id)
        .collect();
    let mut units = BTreeMap::new();
    let mut expected_subjects = BTreeSet::new();
    let mut expected_fragments = BTreeMap::new();
    for b in &tables["retrieval_units"] {
        for row in 0..b.num_rows() {
            let unit: Unit = serde_json::from_str(&string(b, "detail", row)?)
                .map_err(|_| corrupt("retrieval unit contract"))?;
            if unit.unit_id != unit_id(unit.family, unit.source_key)
                || id(b, "unit_id", row)? != Some(unit.unit_id.storage())
                || string(b, "family", row)? != unit.family.name()
            {
                return Err(corrupt("retrieval unit identity/family"));
            }
            if unit.subjects.iter().collect::<BTreeSet<_>>().len() != unit.subjects.len() {
                return Err(corrupt("duplicate unit subject"));
            }
            for s in &unit.subjects {
                match s {
                    Subject::Member(m) => {
                        if !members.contains(&m.storage()) {
                            return Err(corrupt("foreign retrieval member"));
                        }
                        expected_subjects.insert((unit.unit_id.storage(), Some(m.storage()), None));
                    }
                    Subject::Release(r) => {
                        if !releases.contains(r) {
                            return Err(corrupt("foreign retrieval release"));
                        }
                        expected_subjects.insert((unit.unit_id.storage(), None, Some(*r)));
                    }
                }
            }
            for a in &unit.anchors {
                let present = match a {
                    Anchor::Catalog { evidence: id } => evidence.contains(&id.storage()),
                    Anchor::Original {
                        evidence: EvidenceRef::Span(id),
                    } => spans.contains(&id.storage()),
                    Anchor::Original {
                        evidence: EvidenceRef::Scenario(id),
                    } => scenarios.contains(&id.storage()),
                    Anchor::Original {
                        evidence: EvidenceRef::Deployment(id),
                    } => deployments.contains(&id.storage()),
                    Anchor::Declaration { fact_id } => declaration_facts.contains(fact_id),
                };
                if !present {
                    return Err(corrupt("retrieval anchor closure"));
                }
            }
            for f in fragments(&unit, 4096).map_err(|e| corrupt(e.to_string()))? {
                expected_fragments.insert(f.fragment_id, f);
            }
            if units.insert(unit.unit_id, unit).is_some() {
                return Err(corrupt("duplicate retrieval unit"));
            }
        }
    }
    let expected_units =
        catalog::derive(&catalog::from_projection(tables)?).map_err(|e| corrupt(e.to_string()))?;
    if units.values().ne(expected_units.iter()) {
        return Err(corrupt(
            "retrieval units differ from canonical catalog rendering",
        ));
    }
    let mut subjects = BTreeSet::new();
    for b in &tables["retrieval_subjects"] {
        for row in 0..b.num_rows() {
            let m = id(b, "member_id", row)?;
            let r = id(b, "release_id", row)?;
            if m.is_some() == r.is_some()
                || !subjects.insert((
                    id(b, "unit_id", row)?.ok_or_else(|| corrupt("null unit"))?,
                    m,
                    r,
                ))
            {
                return Err(corrupt("retrieval subject shape/duplicate"));
            }
        }
    }
    if subjects != expected_subjects {
        return Err(corrupt("retrieval subject closure"));
    }
    let mut observed = BTreeSet::new();
    let mut embedded = BTreeSet::new();
    for b in &tables["retrieval_fragments"] {
        for row in 0..b.num_rows() {
            let fid = id(b, "fragment_id", row)?.ok_or_else(|| corrupt("null fragment"))?;
            let f = expected_fragments
                .get(&fid)
                .ok_or_else(|| corrupt("unknown rendering fragment"))?;
            if !observed.insert(fid)
                || id(b, "unit_id", row)? != Some(f.unit_id.storage())
                || string(b, "family", row)? != f.family.name()
                || string(b, "text", row)? != f.text
            {
                return Err(corrupt("fragment rendering differs"));
            }
            let digest = b
                .column_by_name("content_digest")
                .and_then(|a| a.as_any().downcast_ref::<FixedSizeBinaryArray>())
                .ok_or_else(|| corrupt("content digest"))?
                .value(row);
            if digest != f.content_digest.0 {
                return Err(corrupt("fragment content digest"));
            }
            match string(b, "embedding_status", row)?.as_str() {
                "embedded" => {
                    embedded.insert(fid);
                }
                "not_requested" | "token_refused" | "token_unavailable" | "embedding_failed" => {}
                _ => return Err(corrupt("fragment embedding state")),
            }
        }
    }
    if observed.len() != expected_fragments.len() {
        return Err(corrupt("fragment rendering closure"));
    }
    let mut vectors = BTreeSet::new();
    for b in &tables["retrieval_vectors"] {
        for row in 0..b.num_rows() {
            if !vectors
                .insert(id(b, "fragment_id", row)?.ok_or_else(|| corrupt("vector fragment"))?)
            {
                return Err(corrupt("duplicate retrieval vector"));
            }
        }
    }
    if embedded != vectors {
        return Err(corrupt("retrieval vector receipt closure"));
    }
    let receipts = &tables["retrieval_receipt"];
    if receipts.iter().map(RecordBatch::num_rows).sum::<usize>() != 1 {
        return Err(corrupt("retrieval receipt count"));
    }
    let b = receipts
        .iter()
        .find(|b| b.num_rows() > 0)
        .expect("one receipt");
    let receipt: RetrievalReceipt = serde_json::from_str(&string(b, "detail", 0)?)
        .map_err(|_| corrupt("retrieval receipt contract"))?;
    let spec = tables["embedding_spec"]
        .iter()
        .find(|b| b.num_rows() > 0)
        .map(|b| crate::embedding_spec::Spec::parse(&string(b, "spec", 0)?).map_err(corrupt))
        .transpose()?;
    if receipt.spec_hash != spec.as_ref().map(crate::embedding_spec::Spec::hash) {
        return Err(corrupt("retrieval specification identity"));
    }
    if let Some(spec) = spec {
        use sha2::{Digest as _, Sha256};
        for b in &tables["retrieval_vectors"] {
            for row in 0..b.num_rows() {
                let fid = id(b, "fragment_id", row)?.ok_or_else(|| corrupt("vector identity"))?;
                let f = expected_fragments
                    .get(&fid)
                    .ok_or_else(|| corrupt("unknown vector fragment"))?;
                let hash = b
                    .column_by_name("input_hash")
                    .and_then(|a| a.as_any().downcast_ref::<FixedSizeBinaryArray>())
                    .ok_or_else(|| corrupt("vector input hash"))?
                    .value(row);
                if hash != Sha256::digest(spec.document_text(&f.text).as_bytes()).as_slice() {
                    return Err(corrupt("retrieval input hash"));
                }
            }
        }
    }
    let units = units.into_values().collect::<Vec<_>>();
    let digest = IdHasher::new("retrieval-inputs-v1")
        .str(&serde_json::to_string(&units).map_err(|_| corrupt("retrieval input encoding"))?)
        .finish_digest();
    if receipt.input_digest != digest
        || receipt.view_revision != VIEW_REVISION
        || receipt.render_revision != RENDER_REVISION
    {
        return Err(corrupt("retrieval input/render identity"));
    }
    Ok(())
}

/// The enclosing publisher binds the pure artifacts to a canonical snapshot.
pub fn validate_snapshot(
    tables: &BTreeMap<String, Vec<arrow_array::RecordBatch>>,
    snapshot: &str,
) -> Result<(), crate::serving_projection::ProjectionError> {
    use crate::serving_projection::corrupt;
    let batch = tables["retrieval_receipt"]
        .iter()
        .find(|b| b.num_rows() > 0)
        .ok_or_else(|| corrupt("missing retrieval receipt"))?;
    let text = batch
        .column_by_name("detail")
        .and_then(|a| a.as_any().downcast_ref::<arrow_array::StringArray>())
        .ok_or_else(|| corrupt("retrieval receipt"))?
        .value(0);
    let receipt: RetrievalReceipt =
        serde_json::from_str(text).map_err(|_| corrupt("retrieval receipt"))?;
    if receipt.snapshot_id.storage().hex() != snapshot {
        return Err(corrupt("retrieval snapshot identity"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn winner(member: u8, family: Family, channel: Channel, unit: u8, score: f64) -> Winner {
        Winner {
            member_id: PublicMemberId::from_storage(Id([member; 16])),
            family,
            channel,
            unit_id: RetrievalUnitId::from_storage(Id([unit; 16])),
            fragment_id: Id([unit; 16]),
            score,
            rank: 0,
        }
    }
    #[test]
    fn large_unit_metadata_still_has_a_bounded_header() {
        let subjects = (0u128..10000)
            .map(|i| Subject::Member(PublicMemberId::from_storage(Id(i.to_le_bytes()))))
            .collect();
        let anchors = (0u128..10000)
            .map(|i| Anchor::Declaration {
                fact_id: Id(i.to_le_bytes()),
            })
            .collect();
        let unit = Unit {
            unit_id: RetrievalUnitId::from_storage(Id([1; 16])),
            source_key: Id([1; 16]),
            family: Family::Scenario,
            title: "長".repeat(10000),
            text: "original scenario".into(),
            subjects,
            anchors,
        };
        let header = RetrievalUnitHeader::from(&unit);
        assert!(header.metadata_omitted);
        assert_eq!(header.subjects.len(), 16);
        assert_eq!(header.anchors.len(), 16);
        assert!(serde_json::to_vec(&header).unwrap().len() < 8192);
        assert_eq!(
            unit.anchors.len(),
            10000,
            "continuation retains every original anchor"
        );
    }
    #[test]
    fn family_budget_preserves_both_channels_and_stable_ties() {
        let rows = vec![
            winner(1, Family::Scenario, Channel::Lexical, 2, 3.),
            winner(1, Family::Scenario, Channel::Vector, 3, 4.),
            winner(2, Family::Source, Channel::Vector, 4, 1.),
        ];
        let mut repeated = rows.clone();
        repeated.extend(vec![rows[0].clone(); 20]);
        repeated.reverse();
        let a = fuse(&channel_winners(rows).unwrap(), &BTreeSet::new());
        let b = fuse(&channel_winners(repeated).unwrap(), &BTreeSet::new());
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert_eq!(a[0].winners.len(), 2);
        assert_ne!(a[0].winners[0].unit_id, a[0].winners[1].unit_id);
    }
    #[test]
    fn exact_priority_does_not_change_rrf_score() {
        let rows =
            channel_winners(vec![winner(1, Family::Source, Channel::Vector, 2, 3.)]).unwrap();
        let promoted = BTreeSet::from([PublicMemberId::from_storage(Id([3; 16]))]);
        let ranks = fuse(&rows, &promoted);
        assert_eq!(
            ranks[0].member_id,
            PublicMemberId::from_storage(Id([3; 16]))
        );
        assert_eq!(ranks[0].score, 0.);
    }
    #[test]
    fn unicode_fragments_are_stable_and_addressable() {
        let unit = Unit {
            unit_id: RetrievalUnitId::from_storage(Id([1; 16])),
            source_key: Id([1; 16]),
            family: Family::Source,
            subjects: vec![],
            anchors: vec![],
            title: "x".into(),
            text: "hello🦀world".into(),
        };
        let a = fragments(&unit, 5).unwrap();
        assert_eq!(
            a.iter().map(|f| f.text.as_str()).collect::<String>(),
            unit.text
        );
        assert_eq!(
            a[0].fragment_id,
            fragments(&unit, 5).unwrap()[0].fragment_id
        );
    }
}
