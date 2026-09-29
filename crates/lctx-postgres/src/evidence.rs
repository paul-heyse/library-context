//! Generation-qualified original evidence. Metadata admission precedes body reads.
use crate::{
    Error,
    repository::PinnedGeneration,
    serving::{QueryLease, ServingStore},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use cpg_schema::{
    Id,
    evidence::{DeploymentDetail, EvidenceKind, EvidenceRef, ScenarioDetail},
    serving_projection::{corrupt, refused},
    wire::{EvidenceAssociation, EvidencePage, EvidenceResult, OriginalContent},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgConnection, Row};

#[derive(Default)]
pub struct EvidenceOptions {
    pub expanded: bool,
    pub limit: u32,
    pub cursor: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    format: u32,
    generation: String,
    target: String,
    position: u64,
    offset: i64,
    expanded: bool,
}
fn cursor(
    g: &PinnedGeneration,
    target: &str,
    position: u64,
    offset: i64,
    expanded: bool,
) -> Result<String, Error> {
    Ok(URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&Cursor {
            format: cpg_schema::wire::FORMAT,
            generation: g.generation(),
            target: target.into(),
            position,
            offset,
            expanded,
        })
        .map_err(|_| corrupt("cursor encoding"))?,
    ))
}
fn position(
    g: &PinnedGeneration,
    target: &str,
    raw: Option<&str>,
    expanded: bool,
) -> Result<(u64, i64), Error> {
    let Some(raw) = raw else { return Ok((0, 0)) };
    if raw.len() > 2048 {
        return Err(Error::Request("evidence cursor length".into()));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| Error::Request("invalid evidence cursor".into()))?;
    let c: Cursor = serde_json::from_slice(&bytes)
        .map_err(|_| Error::Request("invalid evidence cursor".into()))?;
    if c.format != cpg_schema::wire::FORMAT
        || c.generation != g.generation()
        || c.target != target
        || c.expanded != expanded
        || c.offset < 0
        || c.position > i64::MAX as u64
    {
        return Err(Error::Request(
            "evidence cursor belongs to another generation, target or representation".into(),
        ));
    }
    Ok((c.position, c.offset))
}
fn id(bytes: Vec<u8>) -> Result<Id, Error> {
    Ok(Id(bytes
        .try_into()
        .map_err(|_| corrupt("evidence identity"))?))
}
fn kind(value: &str) -> Result<EvidenceKind, Error> {
    serde_json::from_value(json!(value)).map_err(|_| corrupt("evidence kind").into())
}

impl ServingStore {
    pub async fn get_evidence(
        &self,
        g: &PinnedGeneration,
        snapshot: &str,
        reference: EvidenceRef,
        raw_cursor: Option<&str>,
        expanded: bool,
    ) -> Result<Value, Error> {
        g.check_snapshot(snapshot)?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let result = read(
            &mut lease.connection,
            g,
            reference,
            raw_cursor,
            expanded,
            if expanded { 256 * 1024 } else { 32 * 1024 },
        )
        .await?;
        lease.complete();
        serde_json::to_value(result).map_err(|_| corrupt("evidence response encoding").into())
    }
}
async fn read(
    conn: &mut PgConnection,
    g: &PinnedGeneration,
    reference: EvidenceRef,
    raw_cursor: Option<&str>,
    expanded: bool,
    budget: usize,
) -> Result<EvidenceResult, Error> {
    let target = serde_json::to_string(&reference).map_err(|_| corrupt("evidence target"))?;
    let (position, offset) = position(g, &target, raw_cursor, expanded)?;
    let mut result = EvidenceResult {
        snapshot_id: serde_json::from_value(json!(g.manifest.snapshot_id))
            .map_err(|_| corrupt("snapshot ID"))?,
        generation: serde_json::from_value(json!(g.generation()))
            .map_err(|_| corrupt("generation ID"))?,
        evidence: reference.clone(),
        scenario: None,
        deployment: None,
        metadata_omitted: false,
        content: vec![],
        next_cursor: None,
    };
    let spans = match reference.kind() {
        EvidenceKind::Span => vec![reference.id()],
        EvidenceKind::Scenario | EvidenceKind::Deployment => {
            let query = if reference.kind() == EvidenceKind::Scenario {
                "SELECT CASE WHEN octet_length(detail)<=$3 THEN detail END AS detail,primary_span_id AS span_id,octet_length(detail) AS size FROM lctx_serving.catalog_scenarios WHERE generation_digest=$1 AND scenario_id=$2"
            } else {
                "SELECT CASE WHEN octet_length(detail)<=$3 THEN detail END AS detail,span_id,octet_length(detail) AS size FROM lctx_serving.catalog_deployments WHERE generation_digest=$1 AND deployment_id=$2"
            };
            // Large typed details remain canonical; admission falls back to the independently
            // addressable primary original span, with an explicit metadata omission.
            let row = sqlx::query(query)
                .bind(g.id.0.as_slice())
                .bind(reference.id().0.as_slice())
                .bind((budget / 2) as i32)
                .fetch_optional(&mut *conn)
                .await?
                .ok_or_else(|| {
                    Error::Request("evidence reference is not in the pinned generation".into())
                })?;
            if row.try_get::<i32, _>("size")? as usize > budget / 2 {
                result.metadata_omitted = true;
                vec![id(row.try_get("span_id")?)?]
            } else {
                let detail: String = row.try_get("detail")?;
                if reference.kind() == EvidenceKind::Scenario {
                    let d: ScenarioDetail =
                        serde_json::from_str(&detail).map_err(|_| corrupt("scenario detail"))?;
                    let spans = d.spans.clone();
                    result.scenario = Some(d);
                    spans
                } else {
                    result.deployment = Some(
                        serde_json::from_str::<DeploymentDetail>(&detail)
                            .map_err(|_| corrupt("deployment detail"))?,
                    );
                    vec![id(row.try_get("span_id")?)?]
                }
            }
        }
    };
    let span = *spans
        .get(position as usize)
        .ok_or_else(|| Error::Request("evidence cursor position".into()))?;
    let row=sqlx::query("SELECT s.artifact_id,a.release_id,s.start_byte,s.end_byte,a.source_digest,a.path,a.source_kind,a.alignment,a.provenance FROM lctx_serving.catalog_spans s JOIN lctx_serving.catalog_artifacts a USING(generation_digest,artifact_id) WHERE s.generation_digest=$1 AND s.span_id=$2")
        .bind(g.id.0.as_slice()).bind(span.0.as_slice()).fetch_optional(&mut *conn).await?.ok_or_else(||Error::Request("evidence span is not in the pinned generation".into()))?;
    let start: i64 = row.try_get("start_byte")?;
    let end: i64 = row.try_get("end_byte")?;
    if offset > end - start {
        return Err(Error::Request("evidence cursor byte position".into()));
    }
    let artifact = id(row.try_get("artifact_id")?)?;
    let digest = cpg_schema::Digest(
        row.try_get::<Vec<u8>, _>("source_digest")?
            .try_into()
            .map_err(|_| corrupt("source digest"))?,
    );
    let mut chunk = OriginalContent {
        span_id: span,
        artifact_id: artifact,
        release_id: id(row.try_get("release_id")?)?,
        source_digest: digest,
        path: row.try_get("path")?,
        source_kind: row.try_get("source_kind")?,
        alignment: serde_json::from_value(json!(row.try_get::<String, _>("alignment")?))
            .map_err(|_| corrupt("release alignment"))?,
        provenance: row.try_get("provenance")?,
        span_start: start,
        span_end: end,
        chunk_start: start + offset,
        chunk_end: start + offset,
        text: None,
        bytes_base64: None,
        complete: false,
    };
    let metadata = serde_json::to_vec(&result)
        .map_err(|_| corrupt("evidence encoding"))?
        .len()
        + serde_json::to_vec(&chunk)
            .map_err(|_| corrupt("evidence encoding"))?
            .len()
        + 2048;
    let remaining = budget
        .checked_sub(metadata)
        .filter(|n| *n >= 24)
        .ok_or_else(|| refused("evidence metadata budget"))?;
    // JSON can expand a single source byte to six bytes. Reserve that worst case before I/O.
    let count = ((remaining / 6) as i64).min(end - start - offset);
    let body:Vec<u8>=sqlx::query_scalar("SELECT substring(body FROM $3::int FOR $4::int) FROM lctx_serving.catalog_artifacts WHERE generation_digest=$1 AND artifact_id=$2")
        .bind(g.id.0.as_slice()).bind(artifact.0.as_slice()).bind(start+offset+1).bind(count).fetch_one(&mut *conn).await?;
    let mut consumed = body.len();
    match std::str::from_utf8(&body) {
        Ok(s) => chunk.text = Some(s.into()),
        Err(e) if e.error_len().is_none() && e.valid_up_to() > 0 => {
            consumed = e.valid_up_to();
            chunk.text = Some(
                std::str::from_utf8(&body[..consumed])
                    .map_err(|_| corrupt("UTF8 boundary"))?
                    .into(),
            );
        }
        Err(_) => {
            chunk.bytes_base64 = Some(base64::engine::general_purpose::STANDARD.encode(&body))
        }
    }
    chunk.chunk_end += consumed as i64;
    chunk.complete = offset == 0 && chunk.chunk_end == end;
    if chunk.chunk_end < end {
        result.next_cursor = Some(cursor(
            g,
            &target,
            position,
            offset + consumed as i64,
            expanded,
        )?);
    } else if position + 1 < spans.len() as u64 {
        result.next_cursor = Some(cursor(g, &target, position + 1, 0, expanded)?);
    }
    result.content.push(chunk);
    if serde_json::to_vec(&result)
        .map_err(|_| corrupt("evidence encoding"))?
        .len()
        + 32
        > budget
    {
        return Err(refused("evidence page budget").into());
    }
    Ok(result)
}

/// Bound the complete operation after typed assembly, even without evidence associations.
/// `reserve` accounts for the enclosing wire discriminator or pending evidence metadata.
pub(crate) fn check_operation_packet(
    result: &Value,
    expanded: bool,
    reserve: usize,
) -> Result<(), Error> {
    let bytes = serde_json::to_vec(result)
        .map_err(|_| corrupt("operation packet encoding"))?
        .len();
    let budget = if expanded { 256 * 1024 } else { 32 * 1024 };
    if bytes.saturating_add(reserve) > budget {
        let catalog_bytes = serde_json::to_vec(&result["catalog"])
            .map_err(|_| corrupt("catalog packet encoding"))?
            .len();
        let mode = if expanded { "expanded" } else { "default" };
        let action = if expanded {
            "read original evidence through get_evidence using a returned retrieval unit"
        } else {
            "request expanded=true"
        };
        return Err(refused(format!(
            "operation packet exceeds {mode} byte budget (operation={bytes}, catalog={catalog_bytes}, limit={budget}); {action}"
        )).into());
    }
    Ok(())
}

pub(crate) async fn enrich(
    conn: &mut PgConnection,
    g: &PinnedGeneration,
    result: &mut Value,
    options: &EvidenceOptions,
) -> Result<(), Error> {
    let Some(member) = result["catalog"]["member"]["member_id"].as_str() else {
        return Ok(());
    };
    let member = Id::from_hex(member).ok_or_else(|| corrupt("member ID"))?;
    let target = format!("operation:{}", member.hex());
    let (offset, byte_offset) = position(g, &target, options.cursor.as_deref(), options.expanded)?;
    if byte_offset != 0 {
        return Err(Error::Request("operation cursor has a byte offset".into()));
    }
    let limit = if options.limit == 0 {
        20
    } else {
        options.limit
    };
    if limit > 50 {
        return Err(Error::Request("evidence limit exceeds 50".into()));
    }
    let counts=sqlx::query("SELECT count(*) AS total,count(*) FILTER(WHERE intent='demonstration' AND evidence_kind='scenario') AS demonstrations,count(*) FILTER(WHERE intent IN ('expected_failure','skip_xfail')) AS negative,count(*) FILTER(WHERE evidence_kind='scenario' AND evidence_id IN (SELECT scenario_id FROM lctx_serving.catalog_scenarios WHERE generation_digest=$1 AND detail::jsonb->>'context'='context_dependent')) AS contextual FROM lctx_serving.catalog_associations WHERE generation_digest=$1 AND (member_id=$2 OR release_id IS NOT NULL)")
        .bind(g.id.0.as_slice()).bind(member.0.as_slice()).fetch_one(&mut *conn).await?;
    let total = counts.try_get::<i64, _>("total")? as u64;
    if offset > total {
        return Err(Error::Request("evidence cursor exceeds population".into()));
    }
    let records=sqlx::query("SELECT association_id,member_id,release_id,evidence_id,evidence_kind,role,basis,intent,site_id,support FROM lctx_serving.catalog_associations WHERE generation_digest=$1 AND (member_id=$2 OR release_id IS NOT NULL) ORDER BY CASE WHEN evidence_kind='scenario' AND intent='demonstration' THEN 0 WHEN intent IN ('expected_failure','skip_xfail') THEN 1 WHEN evidence_kind='span' THEN 2 ELSE 3 END,association_id LIMIT $3 OFFSET $4")
        .bind(g.id.0.as_slice()).bind(member.0.as_slice()).bind(i64::from(limit)).bind(offset as i64).fetch_all(&mut *conn).await?;
    let mut page = EvidencePage {
        total,
        demonstrations: counts.try_get::<i64, _>("demonstrations")? as u64,
        negative: counts.try_get::<i64, _>("negative")? as u64,
        context_dependent: counts.try_get::<i64, _>("contextual")? as u64,
        ..EvidencePage::default()
    };
    let budget = if options.expanded {
        256 * 1024
    } else {
        32 * 1024
    };
    for row in records {
        let subject = match (
            row.try_get::<Option<Vec<u8>>, _>("member_id")?,
            row.try_get::<Option<Vec<u8>>, _>("release_id")?,
        ) {
            (Some(member), None) => cpg_schema::wire::AssociationSubject::Member {
                member_id: id(member)?,
            },
            (None, Some(release)) => cpg_schema::wire::AssociationSubject::Release {
                release_id: id(release)?,
            },
            _ => return Err(corrupt("association subject").into()),
        };
        let item = EvidenceAssociation {
            subject,
            association_id: id(row.try_get("association_id")?)?,
            evidence: EvidenceRef::new(
                kind(&row.try_get::<String, _>("evidence_kind")?)?,
                id(row.try_get("evidence_id")?)?,
            ),
            role: row.try_get("role")?,
            basis: row.try_get("basis")?,
            intent: serde_json::from_value(json!(row.try_get::<String, _>("intent")?))
                .map_err(|_| corrupt("scenario intent"))?,
            site_id: row
                .try_get::<Option<Vec<u8>>, _>("site_id")?
                .map(id)
                .transpose()?,
            support: serde_json::from_str(&row.try_get::<String, _>("support")?)
                .map_err(|_| corrupt("association support"))?,
        };
        page.items.push(item);
        result["catalog"]["evidence_page"] = json!(page);
        if serde_json::to_vec(result)
            .map_err(|_| corrupt("packet encoding"))?
            .len()
            + 2048
            > budget
        {
            page.items.pop();
            break;
        }
    }
    if offset + (page.items.len() as u64) < total {
        page.next_cursor = Some(cursor(
            g,
            &target,
            offset + page.items.len() as u64,
            0,
            options.expanded,
        )?);
    }
    if page.items.is_empty() && offset < total {
        // `result` still contains the first association which could not fit. Refuse the
        // assembled packet explicitly; neither signatures nor behavioral fates are truncated.
        return check_operation_packet(result, options.expanded, 2048);
    }
    result["catalog"]["evidence_page"] = json!(page);
    let mut demonstrated = std::collections::BTreeSet::new();
    for item in page.items.iter().filter(|i| {
        i.evidence.kind() == EvidenceKind::Scenario
            && i.intent == cpg_schema::evidence::Intent::Demonstration
            && i.basis == "resolved_target"
    }) {
        if demonstrated.len() == 2 {
            break;
        }
        if !demonstrated.insert(item.evidence.id()) {
            continue;
        }
        let size = serde_json::to_vec(result)
            .map_err(|_| corrupt("packet encoding"))?
            .len();
        let Some(remaining) = budget.checked_sub(size + 2048) else {
            break;
        };
        if remaining < 4096 {
            break;
        }
        let demo = match read(
            conn,
            g,
            item.evidence.clone(),
            None,
            options.expanded,
            remaining.min(8192),
        )
        .await
        {
            Ok(demo) => demo,
            Err(Error::Projection(error))
                if error.kind == cpg_schema::serving_projection::FailureKind::ResourceRefused =>
            {
                continue;
            }
            Err(error) => return Err(error),
        };
        result["catalog"]["demonstrations"]
            .as_array_mut()
            .ok_or_else(|| corrupt("demonstration list"))?
            .push(json!(demo));
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitCursor {
    generation: String,
    unit: Id,
    position: usize,
    inner: Option<String>,
    expanded: bool,
}
impl ServingStore {
    pub async fn get_retrieval_unit(
        &self,
        g: &PinnedGeneration,
        snapshot: &str,
        unit: cpg_schema::wire::RetrievalUnitId,
        raw_cursor: Option<&str>,
        expanded: bool,
    ) -> Result<Value, Error> {
        use cpg_schema::{
            retrieval::{Anchor, Unit},
            wire::{RetrievalEvidenceResult, RetrievalUnitHeader},
        };
        g.check_snapshot(snapshot)?;
        let mut cursor = UnitCursor {
            generation: g.generation(),
            unit: unit.storage(),
            position: 0,
            inner: None,
            expanded,
        };
        if let Some(raw) = raw_cursor {
            if raw.len() > 2048 {
                return Err(Error::Request("unit cursor budget".into()));
            }
            cursor = serde_json::from_slice(
                &URL_SAFE_NO_PAD
                    .decode(raw)
                    .map_err(|_| Error::Request("unit cursor".into()))?,
            )
            .map_err(|_| Error::Request("unit cursor".into()))?;
            if cursor.generation != g.generation()
                || cursor.unit != unit.storage()
                || cursor.expanded != expanded
            {
                return Err(Error::Request(
                    "unit cursor belongs to another generation or target".into(),
                ));
            }
        }
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let generation_id = g.id;
        let unit_id = unit.storage();
        let detail = sqlx::query_file_scalar!(
            "queries/unit_detail.sql",
            generation_id.0.as_slice(),
            unit_id.0.as_slice()
        )
        .fetch_optional(&mut *lease.connection)
        .await?;
        let unit: Unit = serde_json::from_str(
            &detail.ok_or_else(|| Error::Request("retrieval unit not in generation".into()))?,
        )
        .map_err(|_| corrupt("retrieval unit contract"))?;
        let fragments = cpg_schema::retrieval::fragments(&unit, 4096)?;
        if cursor.position > fragments.len() + unit.anchors.len() {
            return Err(Error::Request("unit cursor position".into()));
        }
        let mut result = RetrievalEvidenceResult {
            snapshot_id: cpg_schema::wire::SnapshotId::parse(snapshot)?,
            generation: cpg_schema::wire::GenerationDigest::parse(&g.generation())?,
            unit: RetrievalUnitHeader::from(&unit),
            fragment: None,
            original: None,
            next_cursor: None,
        };
        if let Some(fragment) = fragments.get(cursor.position) {
            result.fragment = Some(fragment.clone());
            cursor.position += 1;
        } else {
            while let Some(anchor) = unit.anchors.get(cursor.position - fragments.len()) {
                let reference = match anchor {
                    Anchor::Original { evidence } => Some(evidence.clone()),
                    Anchor::Catalog { evidence } => {
                        let evidence_id = evidence.storage();
                        let spans = sqlx::query_file_scalar!(
                            "queries/unit_original_span.sql",
                            generation_id.0.as_slice(),
                            evidence_id.0.as_slice()
                        )
                        .fetch_all(&mut *lease.connection)
                        .await?;
                        if spans.is_empty() {
                            return Err(
                                corrupt("catalog unit original span closure missing").into()
                            );
                        }
                        Some(EvidenceRef::Span(cpg_schema::wire::SpanId::from_storage(
                            id(spans[0].clone())?,
                        )))
                    }
                    Anchor::Declaration { .. } => None,
                };
                if let Some(reference) = reference {
                    let original = read(
                        &mut lease.connection,
                        g,
                        reference,
                        cursor.inner.as_deref(),
                        expanded,
                        if expanded { 240 * 1024 } else { 24 * 1024 },
                    )
                    .await?;
                    cursor.inner = original.next_cursor.clone();
                    result.original = Some(original);
                    if cursor.inner.is_none() {
                        cursor.position += 1;
                    }
                    break;
                }
                cursor.position += 1;
            }
        }
        if cursor.position < fragments.len() + unit.anchors.len() {
            result.next_cursor =
                Some(URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&cursor).map_err(|_| corrupt("unit cursor encoding"))?,
                ));
        }
        let value = serde_json::to_value(result).map_err(|_| corrupt("unit expansion encode"))?;
        if serde_json::to_vec(&value)
            .map_err(|_| corrupt("unit expansion bytes"))?
            .len()
            > if expanded { 256 * 1024 } else { 32 * 1024 }
        {
            return Err(refused("unit expansion budget").into());
        }
        lease.complete();
        Ok(value)
    }
}

#[cfg(test)]
mod packet_budget_tests {
    use super::*;

    #[test]
    fn operation_packets_without_associations_obey_both_wire_limits() {
        for (expanded, limit) in [(false, 32 * 1024), (true, 256 * 1024)] {
            let mut packet = json!({"catalog":{}, "parameters":[], "payload":""});
            let overhead = serde_json::to_vec(&packet).unwrap().len() + 32;
            packet["payload"] = json!("x".repeat(limit - overhead));
            assert!(check_operation_packet(&packet, expanded, 32).is_ok());
            packet["payload"] = json!("x".repeat(limit - overhead + 1));
            let error = check_operation_packet(&packet, expanded, 32)
                .unwrap_err()
                .to_string();
            assert!(error.contains("operation packet exceeds"));
            assert!(error.contains("catalog=2"));
            assert_eq!(error.contains("request expanded=true"), !expanded);
            assert_eq!(error.contains("get_evidence"), expanded);
            assert_eq!(
                packet["payload"].as_str().unwrap().len(),
                limit - overhead + 1
            );
        }
    }
}
