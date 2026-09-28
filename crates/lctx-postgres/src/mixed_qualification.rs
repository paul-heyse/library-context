//! Frozen calibration and independent repeated confirmation; never activates a selection.
use crate::{
    Error, admission,
    import::{digest, lock, verify_locations},
    profiles::{Policy, Route, hex},
    qualification::{Case, fused, ids, overlap, plans},
    repository::PinnedGeneration,
    retrieval::{RankMode, RankedEntity, rank_on, validate_query},
    serving::{ImportStore, QueryLease, ServingStore},
};
use cpg_schema::serving_projection::{Manifest, corrupt};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use sqlx::PgConnection;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    format: u32,
    generation: String,
    spec: String,
    maximum_ann_p95_ms: f64,
    phase: String,
    policy: Option<Policy>,
    calibration_sha256: Option<String>,
    cases: Vec<Case>,
}
fn vector_digest(c: &Case) -> String {
    let mut h = Sha256::new();
    for v in &c.vector {
        h.update(v.to_bits().to_le_bytes());
    }
    hex(h.finalize())
}
fn p95(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::MAX;
    }
    v.sort_by(f64::total_cmp);
    v[(v.len() * 95).div_ceil(100) - 1]
}
fn reference(c: &Case, rows: &[RankedEntity]) -> Value {
    let views: BTreeSet<_> = if c.operations {
        BTreeSet::from(["signature_doc", "source_body"])
    } else {
        BTreeSet::from(["brief"])
    };
    let mut passed = views == c.reference.keys().map(String::as_str).collect()
        && rows.iter().all(|r| views.contains(r.view.as_str()));
    let mut diagnostics = Vec::new();
    let mut reference_rows = Vec::new();
    for (view, wanted) in &c.reference {
        let actual: Vec<_> = rows.iter().filter(|r| &r.view == view).collect();
        let expected: BTreeMap<_, _> = wanted.iter().map(|r| (r.id.as_str(), r.score)).collect();
        let mut maximum_error = 0.0f64;
        let mut displaced = 0usize;
        let mut examples = Vec::new();
        let identities: BTreeSet<_> = actual.iter().map(|r| hex(&r.id)).collect();
        let inventories = actual.len() == wanted.len()
            && identities.len() == actual.len()
            && expected.len() == wanted.len()
            && identities
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == expected.keys().copied().collect();
        let ordered = actual
            .windows(2)
            .all(|w| w[0].score > w[1].score || (w[0].score == w[1].score && w[0].id < w[1].id))
            && actual
                .iter()
                .enumerate()
                .all(|(i, r)| r.rank as usize == i + 1);
        let numeric = actual.iter().all(|r| {
            expected.get(hex(&r.id).as_str()).is_some_and(|score| {
                let error = (r.score - score).abs();
                maximum_error = maximum_error.max(error);
                r.score.is_finite() && score.is_finite() && error <= 1e-5
            })
        });
        let reference_ordered = wanted
            .windows(2)
            .all(|w| w[0].score > w[1].score || (w[0].score == w[1].score && w[0].id < w[1].id));
        passed &= inventories && ordered && numeric && reference_ordered;
        for (i, (a, b)) in actual.iter().zip(wanted).enumerate() {
            if hex(&a.id) != b.id {
                displaced += 1;
                if examples.len() < 8 {
                    examples.push(json!({"rank":i+1,"postgres_id":hex(&a.id),"float64_id":b.id}));
                }
            }
        }
        for (i, r) in wanted.iter().enumerate() {
            if let Some(id) = cpg_schema::Id::from_hex(&r.id) {
                reference_rows.push(RankedEntity {
                    id: id.0.to_vec(),
                    view: view.clone(),
                    rank: (i + 1) as u32,
                    score: r.score,
                });
            } else {
                passed = false;
            }
        }
        diagnostics.push(json!({"view":view,"inventory_passed":inventories,"postgres_order_passed":ordered,"float64_order_passed":reference_ordered,"numeric_passed":numeric,"maximum_score_error":maximum_error,"float64_ordinal_equal":inventories && displaced==0,"displaced_ranks":displaced,"crossing_examples":examples,"float64_top10_equal":actual.iter().take(10).map(|r|hex(&r.id)).collect::<Vec<_>>()==wanted.iter().take(10).map(|r|r.id.clone()).collect::<Vec<_>>() }));
    }
    json!({"comparison_revision":2,"passed":passed,"views":diagnostics,"float64_fused_top10_equal":fused(&c.lexical,&c.promoted,rows)==fused(&c.lexical,&c.promoted,&reference_rows)})
}
#[derive(Default)]
struct Samples {
    exact: Vec<f64>,
    ann: Vec<f64>,
    recall: BTreeMap<String, Vec<f64>>,
    fused: Vec<f64>,
    plans: bool,
    route: bool,
}

async fn measure(
    conn: &mut PgConnection,
    pinned: &PinnedGeneration,
    pack: &Pack,
) -> Result<Value, Error> {
    let mut samples: BTreeMap<String, [Samples; 2]> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut controls = Vec::new();
    let mut all_exact = true;
    for case in &pack.cases {
        let eligible = ids(&case.eligible)?;
        validate_query(&case.vector)?;
        let (table, key) = if case.operations {
            ("operations", "node_id")
        } else {
            ("briefs", "brief_id")
        };
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM lctx_serving.{table} WHERE generation_digest=$1 AND {key}=ANY($2)"
        )))
        .bind(pinned.id.0.as_slice())
        .bind(&eligible)
        .fetch_one(&mut *conn)
        .await?;
        if count as usize != eligible.len() {
            return Err(Error::Request(
                "qualification identities outside pinned consumer".into(),
            ));
        }
        let allowed: BTreeSet<_> = case.eligible.iter().collect();
        if case
            .lexical
            .iter()
            .chain(&case.promoted)
            .any(|id| !allowed.contains(id))
            || case.lexical.iter().collect::<BTreeSet<_>>().len() != case.lexical.len()
        {
            return Err(Error::Request("qualification lexical eligibility".into()));
        }
        let (e, u) = admission::population(
            conn,
            pinned.id,
            case.operations,
            &eligible,
            pinned.vector_population,
        )
        .await?;
        let (route, class) = pinned.policy.choose(case.operations, e, u);
        let (exact, _) = rank_on(
            conn,
            pinned,
            &case.vector,
            case.operations,
            &eligible,
            10,
            RankMode::Qualification(Route::Exact),
        )
        .await?;
        let reference_check = reference(case, &exact);
        let exact_ok = reference_check["passed"] == true;
        all_exact &= exact_ok;
        if route == Route::Exact {
            controls.push(json!({"name":case.name,"route":"exact","reason":class,"eligible":e,"universe":u,"reference_passed":exact_ok,"reference":reference_check}));
            continue;
        }
        *counts.entry(class.into()).or_default() += 1;
        let entry = samples.entry(class.into()).or_insert_with(|| {
            std::array::from_fn(|_| Samples {
                plans: true,
                route: true,
                ..Default::default()
            })
        });
        let mut plan_evidence = Vec::new();
        for view in case
            .reference
            .keys()
            .filter(|v| !case.reference[*v].is_empty())
        {
            let p = plans(conn, pinned, &case.vector, case.operations, view, &eligible).await?;
            for s in entry.iter_mut() {
                s.plans &= p["passed"] == true;
            }
            plan_evidence.push(json!({"view":view,"checks":p}));
        }
        for (run, s) in entry.iter_mut().enumerate() {
            for round in 0..13 {
                let mut outputs = Vec::new();
                let order = if (run + round) % 2 == 0 {
                    [Route::Exact, Route::Hnsw]
                } else {
                    [Route::Hnsw, Route::Exact]
                };
                for r in order {
                    let start = Instant::now();
                    let (rows, meta) = rank_on(
                        conn,
                        pinned,
                        &case.vector,
                        case.operations,
                        &eligible,
                        10,
                        RankMode::Qualification(if r == Route::Hnsw { Route::Mixed } else { r }),
                    )
                    .await?;
                    // Materialize the same compact IPC as online ranking inside the timed stage.
                    let _ipc = crate::retrieval::encode(&rows)?;
                    let ms = start.elapsed().as_secs_f64() * 1000.;
                    if round >= 3 {
                        if r == Route::Exact {
                            s.exact.push(ms);
                        } else {
                            s.ann.push(ms);
                            s.route &= meta.actual_route == Route::Hnsw && meta.fallback.is_none();
                        }
                    }
                    outputs.push((r, rows));
                }
                let exact = &outputs.iter().find(|(r, _)| *r == Route::Exact).unwrap().1;
                let ann = &outputs.iter().find(|(r, _)| *r == Route::Hnsw).unwrap().1;
                all_exact &= reference(case, exact)["passed"] == true;
                if round >= 3 {
                    for view in case
                        .reference
                        .keys()
                        .filter(|v| !case.reference[*v].is_empty())
                    {
                        let a = exact
                            .iter()
                            .filter(|r| &r.view == view)
                            .take(10)
                            .map(|r| hex(&r.id))
                            .collect::<Vec<_>>();
                        let b = ann
                            .iter()
                            .filter(|r| &r.view == view)
                            .take(10)
                            .map(|r| hex(&r.id))
                            .collect::<Vec<_>>();
                        s.recall
                            .entry(view.clone())
                            .or_default()
                            .push(overlap(&a, &b));
                    }
                    s.fused.push(overlap(
                        &fused(&case.lexical, &case.promoted, exact),
                        &fused(&case.lexical, &case.promoted, ann),
                    ));
                }
            }
        }
        controls.push(json!({"name":case.name,"route":"hnsw","class":class,"eligible":e,"universe":u,"reference_passed":exact_ok,"reference":reference_check,"plans":plan_evidence}));
    }
    let mut classes = Vec::new();
    for class in &pinned.policy.routing.as_ref().unwrap().classes {
        let mut runs = Vec::new();
        if let Some(values) = samples.get_mut(class) {
            for s in values {
                let recall: BTreeMap<_, _> = s
                    .recall
                    .iter()
                    .map(|(v, x)| (v, x.iter().sum::<f64>() / x.len() as f64))
                    .collect();
                let minimum = recall.values().copied().reduce(f64::min).unwrap_or(0.);
                let fused = if s.fused.is_empty() {
                    0.
                } else {
                    s.fused.iter().sum::<f64>() / s.fused.len() as f64
                };
                let exact = p95(&mut s.exact);
                let ann = p95(&mut s.ann);
                let passed = minimum >= 0.99
                    && fused >= 0.99
                    && s.plans
                    && s.route
                    && ann <= pack.maximum_ann_p95_ms
                    && ann <= exact * 0.8;
                runs.push(json!({"passed":passed,"plans_passed":s.plans,"ann_execution_passed":s.route,"recall_by_view":recall,"recall_at_10":minimum,"fused_recall_at_10":fused,"exact_p95_ms":exact,"ann_p95_ms":ann,"samples":s.ann.len()}));
            }
        }
        let queries = counts.get(class).copied().unwrap_or(0);
        classes.push(json!({"class":class,"queries":queries,"passed":queries>=8 && runs.len()==2 && runs.iter().all(|r|r["passed"]==true),"runs":runs}));
    }
    Ok(
        json!({"passed":all_exact && classes.iter().all(|c|c["passed"]==true),"exact_reference_passed":all_exact,"classes":classes,"cases":controls}),
    )
}

pub(crate) async fn qualify(
    writer: &ImportStore,
    reader: &ServingStore,
    raw: &[u8],
) -> Result<Value, Error> {
    let pack: Pack = serde_json::from_slice(raw)
        .map_err(|_| Error::Request("invalid mixed qualification pack".into()))?;
    if pack.format != 2
        || pack.cases.is_empty()
        || pack.cases.len() > 64
        || !pack.maximum_ann_p95_ms.is_finite()
        || !(0.0 < pack.maximum_ann_p95_ms && pack.maximum_ann_p95_ms <= 250.)
        || !matches!(pack.phase.as_str(), "calibration" | "confirmation")
    {
        return Err(Error::Request("invalid mixed qualification bounds".into()));
    }
    let names: BTreeSet<_> = pack.cases.iter().map(|c| &c.name).collect();
    if names.len() != pack.cases.len() {
        return Err(Error::Request("duplicate qualification case".into()));
    }
    let vectors: BTreeSet<_> = pack.cases.iter().map(vector_digest).collect();
    if vectors.len() != pack.cases.len() {
        return Err(Error::Request(
            "qualification requests must have distinct vectors".into(),
        ));
    }
    let id = digest(&pack.generation)?;
    let mut lease = QueryLease::acquire(&reader.pool).await?;
    let mut guard = lock(&id).acquire(&mut *lease.connection).await?;
    let doc:String=sqlx::query_scalar("SELECT canonical_manifest FROM lctx_serving.generations WHERE generation_digest=$1 AND state='ready'").bind(id.0.as_slice()).fetch_one(guard.as_mut()).await?;
    let manifest: Manifest =
        serde_json::from_str(&doc).map_err(|_| corrupt("qualification manifest"))?;
    manifest.validate()?;
    if manifest.generation()? != pack.generation
        || manifest.spec_hash.as_deref() != Some(&pack.spec)
    {
        return Err(Error::Request("qualification identity mismatch".into()));
    }
    let artifacts = verify_locations(guard.as_mut(), &id, &manifest).await?;
    let physical = admission::realization(guard.as_mut(), id).await?;
    if physical["valid"] != true {
        return Err(Error::Admission("build valid indexes before qualification"));
    }
    let policies = if pack.phase == "calibration" {
        if pack.policy.is_some() || pack.calibration_sha256.is_some() {
            return Err(Error::Request("calibration cannot supply a policy".into()));
        }
        vec![
            Policy::mixed(1024, vec!["broad".into(), "unfiltered".into()]),
            Policy::mixed(4096, vec!["broad".into(), "unfiltered".into()]),
        ]
    } else {
        let p = pack
            .policy
            .clone()
            .ok_or_else(|| Error::Request("confirmation requires frozen policy".into()))?;
        p.validate()?;
        if p.route != Route::Mixed {
            return Err(Error::Request("confirmation requires mixed policy".into()));
        }
        let previous:Option<Value>=sqlx::query_scalar("SELECT qualification FROM lctx_serving.profile_attempts WHERE generation_digest=$1 AND qualification->>'phase'='calibration' AND qualification->>'pack_sha256'=$2 AND qualification->'chosen_policy'=$3 ORDER BY attempt_id DESC LIMIT 1")
            .bind(id.0.as_slice()).bind(&pack.calibration_sha256).bind(serde_json::to_value(&p).unwrap()).fetch_optional(guard.as_mut()).await?;
        let prev =
            previous.ok_or_else(|| Error::Request("matching calibration is required".into()))?;
        if prev["realization"] != physical
            || prev["vector_digests"].as_array().is_none_or(|v| {
                v.iter()
                    .any(|x| x.as_str().is_some_and(|s| vectors.contains(s)))
            })
        {
            return Err(Error::Request(
                "confirmation overlaps calibration or index realization changed".into(),
            ));
        }
        vec![p]
    };
    let vector_population = Some(admission::universe(guard.as_mut(), id).await?);
    let mut candidates = Vec::new();
    let mut chosen = None;
    for policy in &policies {
        let pinned = PinnedGeneration {
            id,
            manifest: manifest.clone(),
            artifacts: artifacts.clone(),
            profile: digest(&policy.digest()?)?,
            policy: policy.clone(),
            vector_population,
        };
        let mut r = measure(guard.as_mut(), &pinned, &pack).await?;
        r["policy"] = serde_json::to_value(policy).unwrap();
        if pack.phase == "calibration" && chosen.is_none() && r["exact_reference_passed"] == true {
            let classes = r["classes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["passed"] == true)
                .map(|c| c["class"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            if !classes.is_empty() {
                chosen = Some(Policy::mixed(
                    policy.routing.as_ref().unwrap().count_floor,
                    classes,
                ));
            }
        }
        candidates.push(r);
    }
    let policy = if pack.phase == "confirmation" {
        policies[0].clone()
    } else {
        chosen.clone().unwrap_or_else(|| policies[0].clone())
    };
    let mut result = if pack.phase == "confirmation" {
        candidates[0].clone()
    } else {
        json!({"passed":false,"candidates":candidates,"chosen_policy":chosen})
    };
    for (k,v) in json!({"runner":2,"comparison_revision":2,"phase":pack.phase,"generation":pack.generation,"profile":policy.digest()?,"policy":policy,"pack_sha256":hex(Sha256::digest(raw)),"calibration_sha256":pack.calibration_sha256,"realization":physical,"vector_digests":vectors,"warmups":3,"repetitions":10,"paired_runs":2,"maximum_ann_p95_ms":pack.maximum_ann_p95_ms}).as_object().unwrap(){result[k]=v.clone();}
    // Release the measured serving-role connection before the writer's atomic
    // realization recheck/admission. No importer RLS/planner behavior enters timing.
    guard.release_now().await?;
    lease.complete();
    let mut write = QueryLease::acquire(&writer.pool).await?;
    sqlx::query("SELECT lctx_serving.record_mixed_profile($1,$2,$3)")
        .bind(id.0.as_slice())
        .bind(policy.canonical()?)
        .bind(&result)
        .execute(&mut *write.connection)
        .await?;
    write.complete();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_checks_numerics_and_declared_order_without_fuzzy_ties() {
        let mut case = Case {
            name: "crossing".into(),
            stratum: "unfiltered".into(),
            operations: false,
            vector: vec![],
            eligible: vec![],
            promoted: vec![],
            lexical: vec![],
            reference: BTreeMap::from([(
                "brief".into(),
                vec![
                    crate::qualification::Reference {
                        id: "01".repeat(16),
                        score: 0.5,
                    },
                    crate::qualification::Reference {
                        id: "02".repeat(16),
                        score: 0.49999999,
                    },
                ],
            )]),
        };
        let mut rows = vec![
            RankedEntity {
                id: vec![2; 16],
                view: "brief".into(),
                score: 0.50000001,
                rank: 1,
            },
            RankedEntity {
                id: vec![1; 16],
                view: "brief".into(),
                score: 0.5,
                rank: 2,
            },
        ];
        let checked = reference(&case, &rows);
        assert_eq!(checked["passed"], true);
        assert_eq!(checked["views"][0]["float64_ordinal_equal"], false);
        assert_eq!(checked["views"][0]["displaced_ranks"], 2);
        case.reference.get_mut("brief").unwrap().swap(0, 1);
        assert_eq!(
            reference(&case, &rows)["passed"],
            false,
            "reference order must match its own scores"
        );
        case.reference.get_mut("brief").unwrap().swap(0, 1);
        rows[0].score = 0.5;
        assert_eq!(
            reference(&case, &rows)["passed"],
            false,
            "equal PG scores must use ID order"
        );
        rows[0].score = 0.6;
        assert_eq!(
            reference(&case, &rows)["passed"],
            false,
            "numeric error cannot pass"
        );
        rows[0].score = 0.50000001;
        rows[1].rank = 3;
        assert_eq!(
            reference(&case, &rows)["passed"],
            false,
            "ranks must be contiguous"
        );
        rows[1].rank = 2;
        case.reference.get_mut("brief").unwrap()[0].score = f64::NAN;
        assert_eq!(
            reference(&case, &rows)["passed"],
            false,
            "nonfinite reference cannot pass"
        );
    }
    #[test]
    fn percentile_uses_all_samples() {
        assert_eq!(p95(&mut (1..=100).map(f64::from).collect::<Vec<_>>()), 95.);
    }
}
