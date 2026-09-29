//! The stage table (DESIGN §15.11; cutover plan §3.3).
//!
//! Every stage declares its input and output relations, the transient in-memory values it hands
//! on, its declared context, its effect class, the profiles it runs in and its code identity. The
//! schedule is derived ([`plan`]): a Kahn order with the declaration order as tie-break, refusing a
//! published relation without exactly one writer in the profile, a read of something nothing
//! writes, and cycles. A stage the profile skips leaves its published outputs empty.

use std::collections::{BTreeMap, BTreeSet};

/// What a stage reads or writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RelRef {
    /// A declared model relation, by name (`crate::relations`).
    Model(&'static str),
    /// A legacy relation, by name, during the cutover.
    Legacy(&'static str),
    /// An in-memory handoff between stages of one attempt; never published.
    Transient(&'static str),
}

impl RelRef {
    pub fn name(self) -> &'static str {
        match self {
            Self::Model(n) | Self::Legacy(n) | Self::Transient(n) => n,
        }
    }

    pub fn published(self) -> bool {
        !matches!(self, Self::Transient(_))
    }
}

/// What a stage does beyond computing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// A pure function of its inputs and context.
    Pure,
    /// Runs an analyzer over the library.
    Extraction,
    /// Reads or writes the store.
    Store,
    /// Uses the one embedding-session capability.
    Embedding,
}

/// The compile profiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Profile {
    Catalog,
    Behavioral,
}

/// A set of profiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Profiles(u8);

impl Profiles {
    pub const CATALOG: Self = Self(1);
    pub const BEHAVIORAL: Self = Self(2);
    pub const ALL: Self = Self(3);

    pub fn contains(self, profile: Profile) -> bool {
        let bit = match profile {
            Profile::Catalog => 1,
            Profile::Behavioral => 2,
        };
        self.0 & bit != 0
    }
}

/// One stage.
#[derive(Clone, Copy, Debug)]
pub struct StageDecl {
    pub id: &'static str,
    pub inputs: &'static [RelRef],
    pub outputs: &'static [RelRef],
    /// Declared, digested non-relation inputs (compile inputs, the analysis configuration, …).
    pub context: &'static [&'static str],
    pub effect: Effect,
    pub profiles: Profiles,
    /// The source groups whose code decides the stage's outputs.
    pub code: &'static [&'static str],
    /// A legacy stage, deleted in the phase that replaces it.
    pub legacy: bool,
}

/// A stage table: its stages and the relations given to the attempt from outside (extraction).
#[derive(Clone, Copy, Debug)]
pub struct StageTable {
    pub stages: &'static [StageDecl],
    pub external: &'static [RelRef],
}

/// A table defect, found by [`plan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageError {
    DuplicateStage(&'static str),
    /// A published relation has no writer in the profile and no skipped writer either.
    MissingWriter { relation: RelRef, reader: &'static str },
    /// More than one active stage writes it.
    DoubleWriter { relation: RelRef, writers: Vec<&'static str> },
    /// A transient is read but written by no active stage.
    TransientUnwritten { relation: RelRef, reader: &'static str },
    /// These stages depend on each other.
    Cycle(Vec<&'static str>),
    /// An external relation is also written by a stage.
    ExternalWritten(RelRef),
}

/// The derived schedule for one profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub profile: Profile,
    /// Stage ids in execution order.
    pub order: Vec<&'static str>,
    /// Published relations whose only writers the profile skips: present, and empty.
    pub empty: Vec<RelRef>,
}

/// Derive the schedule for `profile`, or every defect found.
pub fn plan(table: &StageTable, profile: Profile) -> Result<Schedule, Vec<StageError>> {
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    for s in table.stages {
        if !ids.insert(s.id) {
            errors.push(StageError::DuplicateStage(s.id));
        }
    }
    let active: Vec<(usize, &StageDecl)> = table
        .stages
        .iter()
        .enumerate()
        .filter(|(_, s)| s.profiles.contains(profile))
        .collect();
    let mut writers: BTreeMap<RelRef, Vec<usize>> = BTreeMap::new();
    for (i, s) in &active {
        for out in s.outputs {
            writers.entry(*out).or_default().push(*i);
        }
    }
    let external: BTreeSet<RelRef> = table.external.iter().copied().collect();
    for (relation, ws) in &writers {
        if ws.len() > 1 {
            errors.push(StageError::DoubleWriter {
                relation: *relation,
                writers: ws.iter().map(|i| table.stages[*i].id).collect(),
            });
        }
        if external.contains(relation) {
            errors.push(StageError::ExternalWritten(*relation));
        }
    }
    let skipped_outputs: BTreeSet<RelRef> = table
        .stages
        .iter()
        .filter(|s| !s.profiles.contains(profile))
        .flat_map(|s| s.outputs.iter().copied())
        .filter(|r| r.published() && !writers.contains_key(r))
        .collect();
    // Edges: writer → reader, over the active stages.
    let mut indegree: BTreeMap<usize, usize> = active.iter().map(|(i, _)| (*i, 0)).collect();
    let mut edges: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for (i, s) in &active {
        for input in s.inputs {
            match writers.get(input) {
                Some(ws) => {
                    for w in ws {
                        if w != i && edges.entry(*w).or_default().insert(*i) {
                            *indegree.get_mut(i).expect("active") += 1;
                        }
                    }
                }
                None if external.contains(input) => {}
                None if input.published() && skipped_outputs.contains(input) => {}
                None if input.published() => errors.push(StageError::MissingWriter {
                    relation: *input,
                    reader: s.id,
                }),
                None => errors.push(StageError::TransientUnwritten {
                    relation: *input,
                    reader: s.id,
                }),
            }
        }
    }
    // Kahn, releasing the lowest declaration index first.
    let mut ready: BTreeSet<usize> = indegree.iter().filter(|(_, d)| **d == 0).map(|(i, _)| *i).collect();
    let mut order = Vec::new();
    while let Some(next) = ready.pop_first() {
        order.push(next);
        for reader in edges.get(&next).into_iter().flatten() {
            let d = indegree.get_mut(reader).expect("active");
            *d -= 1;
            if *d == 0 {
                ready.insert(*reader);
            }
        }
    }
    if order.len() != active.len() {
        let placed: BTreeSet<usize> = order.iter().copied().collect();
        errors.push(StageError::Cycle(
            active
                .iter()
                .filter(|(i, _)| !placed.contains(i))
                .map(|(_, s)| s.id)
                .collect(),
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Schedule {
        profile,
        order: order.into_iter().map(|i| table.stages[i].id).collect(),
        empty: skipped_outputs.into_iter().collect(),
    })
}

/// Every published relation any stage writes or the attempt receives, in name order: what one
/// generation holds.
pub fn published(table: &StageTable) -> BTreeSet<RelRef> {
    table
        .stages
        .iter()
        .flat_map(|s| s.outputs.iter().copied())
        .chain(table.external.iter().copied())
        .filter(|r| r.published())
        .collect()
}
