//! Projection declarations (DESIGN §15.10).
//!
//! A projection is a typed declaration, generated from role declarations and a call policy: its
//! universe (separate from its selector), its arc source, direction, parallel-arc and unresolved
//! policies. It serves topology analyses only; relational questions stay relational. The runtime
//! (a shared dense index, both-direction adjacency, arc ids as graph weights) lands in cutover
//! phase 4.

use crate::calls::CallPolicy;
use crate::decl::codebook::Codebook;
use crate::id::{Digest, IdHasher, IdKind};
use crate::vocab::EntityKind;

/// Which way arcs are traversed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
    Both,
}

/// What happens to several arcs between one ordered pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParallelArcs {
    /// Every arc is its own edge, weighted by its arc id.
    Keep,
    /// One edge per pair, keeping the arc-id mapping for lineage.
    Collapse,
}

/// How an unresolved target appears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// As an explicit unresolved node, so paths through it stay visible.
    Node,
    /// Left out, and counted in the projection's coverage.
    Omit,
}

/// One projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionDecl {
    pub name: &'static str,
    /// The node universe: every entity of these kinds, arcs or not (isolates survive).
    pub universe: &'static [EntityKind],
    /// The arc source: the call targets the policy admits.
    pub arcs: CallPolicy,
    pub direction: Direction,
    pub parallel: ParallelArcs,
    pub unresolved: Unresolved,
}

impl ProjectionDecl {
    /// The digest every analysis invocation over this projection records.
    pub fn digest(&self) -> Digest {
        let mut h = IdHasher::v2(IdKind::Projection);
        h.str(self.name);
        h.strs(self.universe.iter().map(|k| k.text()));
        h.str(self.arcs.name()).str(&self.arcs.sql());
        h.str(&format!("{:?}/{:?}/{:?}", self.direction, self.parallel, self.unresolved));
        h.finish_digest()
    }
}
