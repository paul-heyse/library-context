use super::{
    build::{digest, invalid},
    *,
};
use crate::domain::{normalized::entities::EntityRef, resources::ResourceBudget};
use std::collections::{BTreeMap, BTreeSet};
pub fn publish(
    p: &communities::Partition,
    result: &TechniqueResult,
    public: &BTreeSet<Id<EntityRef>>,
    out: &mut Output,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let _reservation = b.reserve(
        "analytic-partition-presentation",
        p.vertices()
            .len()
            .checked_mul(1024)
            .ok_or_else(|| invalid("partition size overflow"))?,
    )?;
    for run in p.runs() {
        let row = CommunityRun {
            result: result.id(),
            resolution: run.resolution,
            seed: run.seed as i64,
            iterations: run.iterations as i64,
            converged: run.converged,
            history: digest("leiden-quality-history", &run.quality),
        };
        let mut representatives = BTreeMap::new();
        for (entity, label) in p.vertices().iter().zip(&run.labels) {
            representatives.entry(*label).or_insert(*entity);
        }
        for (entity, label) in p.vertices().iter().zip(&run.labels) {
            out.partitions.insert(PartitionMember {
                run: row.id(),
                entity: *entity,
                representative: representatives[label],
            })?;
        }
        for (ordinal, value) in run.quality.iter().enumerate() {
            out.quality_steps.insert(QualityStep {
                run: row.id(),
                ordinal: ordinal as i64,
                value: *value,
            })?;
        }
        out.runs.insert(row)?;
    }
    for profile in p.profiles() {
        out.profiles.insert(CommunityProfile {
            result: result.id(),
            resolution: profile.resolution,
            mean_ari: profile.mean_ari,
            sd_ari: profile.sd_ari,
            mean_nmi: profile.mean_nmi,
            min_nmi: profile.min_nmi,
            largest: profile.largest as i64,
            communities: profile.communities as i64,
            degenerate: profile.degenerate,
        })?;
    }
    if p.chosen() {
        let chosen = p
            .runs()
            .iter()
            .filter(|r| r.resolution.get() == 1.0)
            .collect::<Vec<_>>();
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        for (i, label) in chosen[0].labels.iter().enumerate() {
            if public.contains(&p.vertices()[i]) {
                groups.entry(*label).or_default().push(i);
            }
        }
        for members in groups.into_values() {
            if members.len() < 2 {
                continue;
            }
            let pairs = members
                .len()
                .checked_mul(members.len() - 1)
                .ok_or_else(|| invalid("community agreement overflow"))?
                / 2;
            let mut sum = 0.0;
            for run in &chosen[1..] {
                let mut together = 0;
                for (i, a) in members.iter().enumerate() {
                    for other in &members[i + 1..] {
                        if run.labels[*a] == run.labels[*other] {
                            together += 1;
                        }
                    }
                }
                sum += together as f64 / pairs as f64;
            }
            let agreement = if chosen.len() == 1 {
                1.0
            } else {
                sum / (chosen.len() - 1) as f64
            };
            if agreement < 0.5 {
                continue;
            }
            let representative = p.vertices()[members[0]];
            let row = Community {
                result: result.id(),
                representative,
                agreement: FiniteF64::new(agreement)?,
            };
            for index in members {
                out.community_members.insert(CommunityMember {
                    community: row.id(),
                    entity: p.vertices()[index],
                })?;
            }
            out.communities.insert(row)?;
        }
    }
    Ok(())
}
