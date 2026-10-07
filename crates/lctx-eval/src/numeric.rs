//! Stored-value numerical diagnostics. Bounded blocks, F64 scalar cosine on F32 inputs,
//! prefix norm in F64 and one F32 rounding. These are not semantic relevance judgments.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub full_dimensions: usize,
    pub projection_dimensions: usize,
    pub block_rows: usize,
    pub k: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoredVector {
    pub id: String,
    pub values: Vec<f32>,
    pub eligible: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub policy: Policy,
    pub query: Vec<f32>,
    pub vectors: Vec<StoredVector>,
    pub nominated_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Hit {
    pub id: String,
    pub score: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResultRows {
    pub full: Vec<Hit>,
    pub projected: Vec<Hit>,
    pub candidate_union_rescored: Vec<Hit>,
    pub missing_after_projection: Vec<String>,
    pub missing_from_union: Vec<String>,
    pub eligible_rows: usize,
    pub precision: String,
    pub ties: String,
    pub supplied_population_complete: bool,
    pub reference_domain: String,
    pub nomination_basis: String,
}
fn norm(values: &[f32]) -> Result<f64, String> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return Err("empty/nonfinite stored vector".into());
    }
    let norm = values
        .iter()
        .fold(0.0_f64, |sum, v| sum + f64::from(*v) * f64::from(*v))
        .sqrt();
    if !norm.is_finite() || norm == 0.0 {
        return Err("zero/nonfinite vector norm".into());
    }
    Ok(norm)
}
pub fn project(values: &[f32], dimensions: usize) -> Result<Vec<f32>, String> {
    let prefix = values
        .get(..dimensions)
        .ok_or("projection exceeds full vector")?;
    let length = norm(prefix)?;
    Ok(prefix
        .iter()
        .map(|v| (f64::from(*v) / length) as f32)
        .collect())
}
fn cosine(left: &[f32], right: &[f32]) -> Result<f64, String> {
    if left.len() != right.len() {
        return Err("stored dimension mismatch".into());
    }
    let denominator = norm(left)? * norm(right)?;
    let dot = left
        .iter()
        .zip(right)
        .fold(0.0_f64, |sum, (a, b)| sum + f64::from(*a) * f64::from(*b));
    Ok(dot / denominator)
}
fn retain(top: &mut Vec<Hit>, item: Hit, k: usize) {
    top.push(item);
    top.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.id.cmp(&b.id)));
    top.truncate(k);
}
pub fn evaluate(input: &Input) -> Result<ResultRows, String> {
    let p = &input.policy;
    if p.full_dimensions == 0
        || p.full_dimensions > 4096
        || p.projection_dimensions == 0
        || p.projection_dimensions > p.full_dimensions
        || p.block_rows == 0
        || p.block_rows > 64
        || p.k == 0
        || p.k > 64
        || input.vectors.len() > 256
    {
        return Err("unsupported bounded numeric policy".into());
    }
    if input.query.len() != p.full_dimensions {
        return Err("query dimension mismatch".into());
    }
    norm(&input.query)?;
    let projected_query = project(&input.query, p.projection_dimensions)?;
    let ids: BTreeSet<_> = input.vectors.iter().map(|row| row.id.as_str()).collect();
    if ids.len() != input.vectors.len()
        || ids.contains("")
        || input
            .nominated_ids
            .iter()
            .any(|id| !ids.contains(id.as_str()))
    {
        return Err("duplicate/empty/foreign candidate identity".into());
    }
    let nominated: BTreeSet<_> = input.nominated_ids.iter().collect();
    let mut full = vec![];
    let mut projected = vec![];
    let mut union = vec![];
    let mut eligible_rows = 0;
    for block in input.vectors.chunks(p.block_rows) {
        for row in block {
            if row.values.len() != p.full_dimensions {
                return Err("stored dimension mismatch".into());
            }
            norm(&row.values)?;
            if !row.eligible {
                if nominated.contains(&row.id) {
                    return Err("nominated foreign-scope vector".into());
                }
                continue;
            }
            eligible_rows += 1;
            let score = cosine(&input.query, &row.values)?;
            retain(
                &mut full,
                Hit {
                    id: row.id.clone(),
                    score,
                },
                p.k,
            );
            let projection = project(&row.values, p.projection_dimensions)?;
            retain(
                &mut projected,
                Hit {
                    id: row.id.clone(),
                    score: cosine(&projected_query, &projection)?,
                },
                p.k,
            );
            if nominated.contains(&row.id) {
                retain(
                    &mut union,
                    Hit {
                        id: row.id.clone(),
                        score,
                    },
                    p.k,
                );
            }
        }
    }
    if eligible_rows == 0 {
        return Err("empty eligible numeric inventory is not a successful reference".into());
    }
    let missing_after_projection = full
        .iter()
        .filter(|hit| !projected.iter().any(|other| other.id == hit.id))
        .map(|hit| hit.id.clone())
        .collect();
    let missing_from_union = full
        .iter()
        .filter(|hit| !nominated.contains(&hit.id))
        .map(|hit| hit.id.clone())
        .collect();
    Ok(ResultRows {
        full,
        projected,
        candidate_union_rescored: union,
        missing_after_projection,
        missing_from_union,
        eligible_rows,
        precision: "F64 scalar cosine over stored F32; prefix F64 norm then F32 rounding".into(),
        ties: "exact F64 score then canonical ID ascending; no tolerance tie merging".into(),
        supplied_population_complete: true,
        reference_domain:
            "supplied finite stored rows only; external inventory completeness not inferred".into(),
        nomination_basis: "caller-supplied diagnostic union; no native ANN readiness claim".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full4096_projection1024_and_union_separate_losses_and_ties() {
        let mut query = vec![0.0; 4096];
        query[0] = 1.0;
        query[1024] = 1.0;
        let mut a = query.clone();
        a[0] = 1.0;
        let mut b = vec![0.0; 4096];
        b[0] = 1.0;
        b[1024] = -1.0;
        let input = Input {
            policy: Policy {
                full_dimensions: 4096,
                projection_dimensions: 1024,
                block_rows: 1,
                k: 1,
            },
            query,
            vectors: vec![
                StoredVector {
                    id: "z-best".into(),
                    values: a,
                    eligible: true,
                },
                StoredVector {
                    id: "a-prefix-tie".into(),
                    values: b,
                    eligible: true,
                },
            ],
            nominated_ids: vec!["a-prefix-tie".into()],
        };
        let rows = evaluate(&input).unwrap();
        assert_eq!(rows.full[0].id, "z-best");
        assert!((rows.full[0].score - 1.0).abs() < 1e-15);
        assert_eq!(rows.projected[0].id, "a-prefix-tie");
        assert_eq!(rows.missing_after_projection, vec!["z-best"]);
        assert_eq!(rows.missing_from_union, vec!["z-best"]);
        assert_eq!(rows.candidate_union_rescored[0].score, 0.0);
    }
    #[test]
    fn independent_scalar_f64_reference_and_block_shapes_agree() {
        let query = vec![3.0, 4.0, 2.0];
        let data = [
            vec![2.0, 1.0, 4.0],
            vec![1.0, 0.0, 1.0],
            vec![3.0, 4.0, 2.0],
        ];
        for block_rows in [1, 2, 64] {
            let input = Input {
                policy: Policy {
                    full_dimensions: 3,
                    projection_dimensions: 2,
                    block_rows,
                    k: 3,
                },
                query: query.clone(),
                vectors: data
                    .iter()
                    .enumerate()
                    .map(|(i, v)| StoredVector {
                        id: i.to_string(),
                        values: v.clone(),
                        eligible: true,
                    })
                    .collect(),
                nominated_ids: vec!["0".into(), "1".into(), "2".into()],
            };
            let rows = evaluate(&input).unwrap();
            for hit in rows.full {
                let v = &data[hit.id.parse::<usize>().unwrap()];
                // Separate hand-sized direct F64 arithmetic, no kernel helper.
                let dot = f64::from(query[0]) * f64::from(v[0])
                    + f64::from(query[1]) * f64::from(v[1])
                    + f64::from(query[2]) * f64::from(v[2]);
                let length =
                    (f64::from(v[0]).powi(2) + f64::from(v[1]).powi(2) + f64::from(v[2]).powi(2))
                        .sqrt();
                assert!((hit.score - dot / (29.0_f64.sqrt() * length)).abs() < 1e-15);
            }
            for hit in rows.projected {
                let v = &data[hit.id.parse::<usize>().unwrap()];
                let length = (f64::from(v[0]).powi(2) + f64::from(v[1]).powi(2)).sqrt();
                let q0 = f64::from((3.0_f64 / 5.0) as f32);
                let q1 = f64::from((4.0_f64 / 5.0) as f32);
                let v0 = f64::from((f64::from(v[0]) / length) as f32);
                let v1 = f64::from((f64::from(v[1]) / length) as f32);
                let expected =
                    (q0 * v0 + q1 * v1) / ((q0 * q0 + q1 * q1) * (v0 * v0 + v1 * v1)).sqrt();
                assert!((hit.score - expected).abs() < 1e-15);
            }
        }
    }
    #[test]
    fn invalid_values_empty_domains_and_foreign_scope_refuse() {
        let mut input = Input {
            policy: Policy {
                full_dimensions: 2,
                projection_dimensions: 1,
                block_rows: 1,
                k: 1,
            },
            query: vec![1.0, 0.0],
            vectors: vec![StoredVector {
                id: "x".into(),
                values: vec![1.0, 0.0],
                eligible: true,
            }],
            nominated_ids: vec!["x".into()],
        };
        input.vectors[0].eligible = false;
        assert!(evaluate(&input).is_err());
        input.vectors[0].eligible = true;
        input.vectors[0].values[0] = f32::NAN;
        assert!(evaluate(&input).is_err());
        input.vectors[0].values = vec![0.0, 1.0];
        assert!(evaluate(&input).is_err());
        input.vectors.clear();
        input.nominated_ids.clear();
        assert!(evaluate(&input).is_err());
    }
}
