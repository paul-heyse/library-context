//! Explicit ANN qualification against an externally captured independent cosine reference.
use crate::{
    Error,
    import::{digest, lock, verify_locations},
    profiles::{Policy, Route, hex},
    repository::PinnedGeneration,
    retrieval::{RankedEntity, rank_on, validate_query},
    serving::{ImportStore, QueryLease, ServingStore},
};
use cpg_schema::id::{Digest, Id};
use cpg_schema::serving_projection::{Manifest, corrupt, refused};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::{Connection, PgConnection};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationPack {
    pub format: u32,
    pub generation: String,
    pub spec: String,
    pub maximum_ann_p95_ms: f64,
    pub cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub name: String,
    pub stratum: String,
    pub operations: bool,
    pub vector: Vec<f32>,
    pub eligible: Vec<String>,
    pub promoted: Vec<String>,
    pub lexical: Vec<String>,
    pub reference: BTreeMap<String, Vec<Reference>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub id: String,
    pub score: f64,
}
#[derive(Default, Serialize)]
struct Stratum {
    queries: usize,
    recall_at_10: f64,
    fused_recall_at_10: f64,
}
impl ImportStore {
    pub async fn build_hnsw(&self, generation: Digest) -> Result<(), Error> {
        let mut lease = QueryLease::acquire(&self.pool).await?;
        sqlx::query("SELECT lctx_serving.build_hnsw($1)")
            .bind(generation.0.as_slice())
            .execute(&mut *lease.connection)
            .await?;
        lease.complete();
        Ok(())
    }
    pub async fn qualify_hnsw(&self, reader: &ServingStore, raw: &[u8]) -> Result<Value, Error> {
        if raw.len() > 128 * 1024 * 1024 {
            return Err(refused("qualification pack byte budget").into());
        }
        let pack: QualificationPack = serde_json::from_slice(raw)
            .map_err(|_| Error::Request("invalid qualification pack".into()))?;
        if pack.format != 1
            || pack.cases.is_empty()
            || pack.cases.len() > 64
            || !pack.maximum_ann_p95_ms.is_finite()
            || !(0.0..=30000.0).contains(&pack.maximum_ann_p95_ms)
        {
            return Err(Error::Request("invalid qualification bounds".into()));
        }
        let generation = digest(&pack.generation)?;
        let mut lease = QueryLease::acquire(&self.pool).await?;
        let mut guard = lock(&generation).acquire(&mut *lease.connection).await?;
        let doc:String=sqlx::query_scalar("SELECT canonical_manifest FROM lctx_serving.generations WHERE generation_digest=$1 AND state='ready'").bind(generation.0.as_slice()).fetch_one(guard.as_mut()).await?;
        let manifest: Manifest =
            serde_json::from_str(&doc).map_err(|_| corrupt("qualification manifest"))?;
        manifest.validate()?;
        if manifest.spec_hash.as_deref() != Some(&pack.spec)
            || manifest.generation()? != pack.generation
        {
            return Err(Error::Request(
                "qualification generation/spec mismatch".into(),
            ));
        }
        let artifacts = verify_locations(guard.as_mut(), &generation, &manifest).await?;
        let pinned = PinnedGeneration {
            id: generation,
            manifest,
            artifacts,
            profile: digest(&Policy::hnsw().digest()?)?,
            policy: Policy::hnsw(),
        };
        let mut strata: BTreeMap<String, Stratum> = BTreeMap::new();
        let mut cases = Vec::new();
        let mut exact_pass = true;
        let mut plans_pass = true;
        let mut latencies = Vec::new();
        let mut names = BTreeSet::new();
        let mut read_lease = QueryLease::acquire(&reader.pool).await?;
        for case in pack.cases {
            if !names.insert(case.name.clone()) {
                return Err(Error::Request("duplicate qualification case".into()));
            }
            validate_query(&case.vector)?;
            let ids = ids(&case.eligible)?;
            let universe = pinned.manifest.relations[if case.operations {
                "operations"
            } else {
                "briefs"
            }]
            .rows as usize;
            let ratio = ids.len() as f64 / universe.max(1) as f64;
            if ids.is_empty()
                || ids.len() > universe
                || !matches!(case.stratum.as_str(), "unfiltered" | "broad" | "selective")
                || case.stratum == "unfiltered" && ids.len() != universe
                || case.stratum == "broad" && !(0.1 < ratio && ratio < 1.0)
                || case.stratum == "selective" && ratio > 0.1
            {
                return Err(Error::Request(
                    "misclassified qualification filter stratum".into(),
                ));
            }
            let (table, key) = if case.operations {
                ("operations", "node_id")
            } else {
                ("briefs", "brief_id")
            };
            let count:i64=sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($2)"))).bind(generation.0.as_slice()).bind(&ids).fetch_one(&mut *read_lease.connection).await?;
            if count as usize != ids.len() {
                return Err(Error::Request(
                    "qualification identities outside pinned consumer".into(),
                ));
            }
            let started = Instant::now();
            let (exact, _) = rank_on(
                &mut read_lease.connection,
                &pinned,
                &case.vector,
                case.operations,
                &ids,
                10,
                Route::Exact,
            )
            .await?;
            let exact_ms = started.elapsed().as_secs_f64() * 1000.0;
            let started = Instant::now();
            let (ann, metadata) = rank_on(
                &mut read_lease.connection,
                &pinned,
                &case.vector,
                case.operations,
                &ids,
                10,
                Route::Hnsw,
            )
            .await?;
            let ann_ms = started.elapsed().as_secs_f64() * 1000.0;
            latencies.push(ann_ms);
            let mut reference_ok = true;
            let mut plans_ok = true;
            let mut view_recalls = Vec::new();
            let mut plan_checks = Vec::new();
            let expected_views = if case.operations {
                vec!["signature_doc", "source_body"]
            } else {
                vec!["brief"]
            };
            if case
                .reference
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != expected_views.iter().copied().collect()
            {
                return Err(Error::Request(
                    "qualification view inventory mismatch".into(),
                ));
            }
            for view in expected_views {
                let actual: Vec<_> = exact.iter().filter(|r| r.view == view).collect();
                let reference = &case.reference[view];
                reference_ok &= actual.len() == reference.len()
                    && actual.iter().zip(reference).all(|(a, b)| {
                        hex(&a.id) == b.id
                            && b.score.is_finite()
                            && (a.score - b.score).abs() <= 1e-5
                    });
                let wanted: Vec<_> = actual.iter().take(10).map(|r| hex(&r.id)).collect();
                let candidates: Vec<_> = ann
                    .iter()
                    .filter(|r| r.view == view)
                    .take(10)
                    .map(|r| hex(&r.id))
                    .collect();
                reference_ok &= !actual.is_empty();
                view_recalls.push((view, overlap(&wanted, &candidates)));
                if !actual.is_empty() {
                    let checked = plans(
                        &mut read_lease.connection,
                        &pinned,
                        &case.vector,
                        case.operations,
                        view,
                        &ids,
                    )
                    .await?;
                    plans_ok &= checked["passed"] == true;
                    plan_checks.push(json!({"view":view,"checks":checked}));
                }
            }
            let allowed: BTreeSet<_> = case.eligible.iter().collect();
            if case
                .lexical
                .iter()
                .chain(&case.promoted)
                .any(|id| !allowed.contains(id))
                || case.lexical.iter().collect::<BTreeSet<_>>().len() != case.lexical.len()
            {
                return Err(Error::Request(
                    "qualification lexical eligibility/uniqueness".into(),
                ));
            }
            let exact_fused = fused(&case.lexical, &case.promoted, &exact);
            let ann_fused = fused(&case.lexical, &case.promoted, &ann);
            let fused_recall = overlap(&exact_fused, &ann_fused);
            for (view, recall) in &view_recalls {
                let name = format!(
                    "{}:{}:{view}",
                    if case.operations {
                        "operations"
                    } else {
                        "briefs"
                    },
                    case.stratum
                );
                let stratum = strata.entry(name).or_default();
                stratum.queries += 1;
                stratum.recall_at_10 += recall;
                stratum.fused_recall_at_10 += fused_recall;
            }
            exact_pass &= reference_ok;
            plans_pass &= plans_ok && metadata.actual_route == Route::Hnsw;
            cases.push(json!({"name":case.name,"stratum":case.stratum,"exact_reference_passed":reference_ok,"plans_passed":plans_ok,"plans":plan_checks,"recall_by_view":view_recalls,"fused_recall_at_10":fused_recall,"exact_ms":exact_ms,"ann_ms":ann_ms,"routing":metadata}));
        }
        for result in strata.values_mut() {
            result.recall_at_10 /= result.queries as f64;
            result.fused_recall_at_10 /= result.queries as f64;
        }
        latencies.sort_by(f64::total_cmp);
        let p95 = latencies[(latencies.len() * 95).div_ceil(100) - 1];
        let latency_pass = p95 <= pack.maximum_ann_p95_ms;
        let required = [
            "briefs:unfiltered:brief",
            "operations:unfiltered:signature_doc",
            "operations:unfiltered:source_body",
            "operations:broad:signature_doc",
            "operations:broad:source_body",
            "operations:selective:signature_doc",
            "operations:selective:source_body",
        ];
        let passed = exact_pass
            && plans_pass
            && latency_pass
            && strata.len() == required.len()
            && required.iter().all(|name| strata.contains_key(*name))
            && strata
                .values()
                .all(|s| s.recall_at_10 >= 0.99 && s.fused_recall_at_10 >= 0.99);
        let strata:Vec<_>=strata.into_iter().map(|(name,s)|json!({"name":name,"queries":s.queries,"recall_at_10":s.recall_at_10,"fused_recall_at_10":s.fused_recall_at_10})).collect();
        let result = json!({"runner":1,"generation":pack.generation,"profile":pinned.profile.hex(),"pack_sha256":hex(Sha256::digest(raw)),"passed":passed,"exact_reference_passed":exact_pass,"plans_passed":plans_pass,"latency_passed":latency_pass,"ann_p95_ms":p95,"maximum_ann_p95_ms":pack.maximum_ann_p95_ms,"strata":strata,"cases":cases});
        sqlx::query("SELECT lctx_serving.record_profile($1,$2,$3)")
            .bind(generation.0.as_slice())
            .bind(Policy::hnsw().canonical()?)
            .bind(&result)
            .execute(guard.as_mut())
            .await?;
        read_lease.complete();
        guard.release_now().await?;
        lease.complete();
        Ok(result)
    }
}
fn ids(values: &[String]) -> Result<Vec<Vec<u8>>, Error> {
    if values.len() > 200000 {
        return Err(refused("qualification entity budget").into());
    }
    let ids = values
        .iter()
        .map(|s| {
            Id::from_hex(s)
                .map(|id| id.0.to_vec())
                .ok_or_else(|| Error::Request("invalid reference identity".into()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err(Error::Request("duplicate reference identity".into()));
    }
    Ok(ids)
}
fn overlap(wanted: &[String], actual: &[String]) -> f64 {
    if wanted.is_empty() {
        return f64::from(actual.is_empty());
    }
    wanted.iter().filter(|id| actual.contains(id)).count() as f64 / wanted.len() as f64
}
// Qualification-only independent RRF oracle. Online fusion remains solely Python-owned.
fn fused(lexical: &[String], promoted: &[String], rows: &[RankedEntity]) -> Vec<String> {
    let mut scores: BTreeMap<String, f64> = BTreeMap::new();
    for (i, id) in lexical.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 1.0 / (61 + i) as f64;
    }
    for row in rows {
        *scores.entry(hex(&row.id)).or_default() += 1.0 / (60 + row.rank) as f64;
    }
    for id in promoted {
        *scores.entry(id.clone()).or_default() += 1.0;
    }
    let mut rows: Vec<_> = scores.into_iter().collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    rows.into_iter().take(10).map(|(id, _)| id).collect()
}
async fn plans(
    conn: &mut PgConnection,
    generation: &PinnedGeneration,
    query: &[f32],
    operations: bool,
    view: &str,
    ids: &[Vec<u8>],
) -> Result<Value, Error> {
    let (table, key) = if operations {
        ("operation_vectors", "node_id")
    } else {
        ("vectors", "brief_id")
    };
    let sql = format!(
        "PREPARE lctx_ann_plan(bytea,lctx_ext.vector,bytea[],text,bigint) AS SELECT {key} FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($3) {} ORDER BY vector OPERATOR(lctx_ext.<=>) $2 LIMIT $5",
        if operations {
            "AND embedding_view=$4"
        } else {
            "AND $4='brief'"
        }
    );
    let vector = format!(
        "[{}]",
        query
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    let ids = ids
        .iter()
        .map(|id| format!("decode('{}','hex')", hex(id)))
        .collect::<Vec<_>>()
        .join(",");
    // All SQL literals below are finite floats, validated binary identities or fixed view names.
    let explain = format!(
        "EXPLAIN (ANALYZE,FORMAT JSON) EXECUTE lctx_ann_plan(decode('{}','hex'),'{}'::lctx_ext.vector,ARRAY[{}]::bytea[],'{}',200)",
        generation.id.hex(),
        vector,
        ids,
        view
    );
    let expected = format!(
        "{}{}_ann",
        if operations { "o_" } else { "v_" },
        &generation.id.hex()[..48]
    );
    let expected = if operations {
        expected.replace(
            "_ann",
            if view == "signature_doc" {
                "_d_ann"
            } else {
                "_s_ann"
            },
        )
    } else {
        expected
    };
    let mut passed = true;
    let mut checks = Vec::new();
    for mode in ["force_custom_plan", "force_generic_plan"] {
        let mut tx = conn.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("SET TRANSACTION READ ONLY; SET LOCAL plan_cache_mode='{mode}'; SET LOCAL hnsw.ef_search=100; SET LOCAL hnsw.iterative_scan='strict_order'; SET LOCAL hnsw.max_scan_tuples=20000; SET LOCAL work_mem='8MB'; SET LOCAL hnsw.scan_mem_multiplier=2; {sql}"))).execute(&mut *tx).await?;
        let plan: Value = sqlx::query_scalar(sqlx::AssertSqlSafe(explain.clone()))
            .fetch_one(&mut *tx)
            .await?;
        fn used(plan: &Value, index: &str) -> bool {
            match plan {
                Value::Object(v) => {
                    v.get("Index Name").and_then(Value::as_str) == Some(index)
                        && v.get("Actual Loops")
                            .and_then(Value::as_f64)
                            .is_some_and(|v| v > 0.0)
                        || v.values().any(|p| used(p, index))
                }
                Value::Array(v) => v.iter().any(|p| used(p, index)),
                _ => false,
            }
        }
        fn scans(plan: &Value, out: &mut Vec<Value>) {
            match plan {
                Value::Object(v) => {
                    if v.contains_key("Relation Name") {
                        out.push(json!({"relation":v["Relation Name"],"index":v.get("Index Name"),"loops":v.get("Actual Loops"),"rows":v.get("Actual Rows")}));
                    }
                    for value in v.values() {
                        scans(value, out);
                    }
                }
                Value::Array(v) => {
                    for value in v {
                        scans(value, out);
                    }
                }
                _ => {}
            }
        }
        let mut scanned = Vec::new();
        scans(&plan, &mut scanned);
        let leaf = expected.trim_end_matches("_ann");
        let pruned = scanned.iter().all(|s| {
            s["loops"].as_f64().unwrap_or(0.0) == 0.0
                || s["relation"].as_str().is_none_or(|name| {
                    !name.starts_with("v_") && !name.starts_with("o_") || name == leaf
                })
        });
        let indexed = used(&plan, &expected);
        passed &= indexed && pruned;
        checks.push(json!({"mode":mode,"expected_index":expected,"index_used":indexed,"partition_pruning_passed":pruned,"scans":scanned,"planning_ms":plan[0]["Planning Time"],"execution_ms":plan[0]["Execution Time"]}));
        sqlx::raw_sql("DEALLOCATE lctx_ann_plan")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(json!({"passed":passed,"prepared_plans":checks}))
}
