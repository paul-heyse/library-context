//! Analysis results (ADR-0019; DESIGN §3.2, §9, §10): what the passes and the statistical
//! analytics find, and later the assertions and briefs synthesized from it.
//!
//! These tables carry their provenance **in-row**, not as `facts` rows: an invocation names its
//! run (`lctx-compiler`), model, extraction mode, method, parameters and diagnostics, and every
//! result names its invocation. They cite the catalogs' node and edge ids and the extractor's
//! fact ids, and they are rebuildable from the snapshot, the analytics config and the compiler
//! digest. They are not in the `nodes`/`edges` catalogs, which Stage D derives before any
//! analysis runs.
//!
//! Every id here is content-derived and contains no analytics-config digest (§3.4.1), so an
//! unchanged result keeps its id when a parameter changes and an ablation diff is a join.

use crate::codebook::{
    AnalyticMethod, CoverageStatus, EvidenceStatus, ExtractionMode, FindingKind, InvocationPhase,
    MemberRole, Modality, StopReason,
};
use crate::id::{Digest, Id, IdHasher, kind};
use crate::table::table;

table!(
    /// One run of an analytic method (ADR-0019): its parameters as canonical JSON, the projection
    /// it read, the libraries it ran, and its diagnostics where the method has them (guidelines
    /// §8, §10). A method without a convergence diagnostic leaves it null ("unavailable").
    AnalysisInvocations, AnalysisInvocationsRow = "analysis_invocations",
    family = Findings,
    key = [snapshot_id, invocation_id],
    checks = [
        ("iterations_nonnegative", "iterations IS NULL OR iterations >= 0"),
        ("candidate_set_size_nonnegative", "candidate_set_size IS NULL OR candidate_set_size >= 0"),
        ("vertices_examined_nonnegative", "vertices_examined IS NULL OR vertices_examined >= 0"),
        ("arcs_examined_nonnegative", "arcs_examined IS NULL OR arcs_examined >= 0"),
    ],
    {
        snapshot_id: Id,
        invocation_id: Id,
        /// The `lctx-compiler` run.
        run_id: Id,
        /// `<producer_id hex>/<surface>`, like `facts.model_id`.
        model_id: String,
        extraction_mode: ExtractionMode,
        method: AnalyticMethod,
        /// The method's parameters as canonical JSON (fields in a fixed order, no whitespace).
        parameters: String,
        parameters_digest: Digest,
        /// The digest of the projection spec the method read, when it read one.
        projection_digest: Option<Digest>,
        /// `name version` of each library the method ran, sorted.
        library_versions: Vec<String>,
        /// The invocation's subject (Pass A: the seed declaration).
        subject_node_id: Option<Id>,
        seed: Option<i64>,
        iterations: Option<i64>,
        residual: Option<f64>,
        converged: Option<bool>,
        quality_history: Vec<f64>,
        candidate_set_size: Option<i64>,
        vertices_examined: Option<i64>,
        arcs_examined: Option<i64>,
        /// Whether the method ran to completion under its stated model, or stopped early.
        completion: CoverageStatus,
        stop_reason: Option<StopReason>,
    }
);

table!(
    /// A typed finding (DESIGN §10.1): never a sentence.
    ///
    /// **Identity** (ADR-0019, review F1): kind, subject, related node, condition node, status,
    /// depth, stop reason, `witnesses_omitted`, its witness steps and its members. **Lineage**
    /// (outside the id): the invocation and the score. A rooted analysis (Pass A) carries its root,
    /// the seed, as the subject.
    ///
    /// **Completion** (review F4): the depth bound and the subsystem, dependency and synthetic
    /// boundaries are the stated model, so a result inside them is complete under it; a vertex or
    /// arc budget is operational truncation, recorded on the invocation (`partial`); the witness
    /// cap is presentation only (`witnesses_omitted`).
    Findings, FindingsRow = "findings",
    family = Findings,
    key = [snapshot_id, finding_id],
    checks = [("depth_nonnegative", "depth IS NULL OR depth >= 0")],
    {
        snapshot_id: Id,
        finding_id: Id,
        invocation_id: Id,
        finding_kind: FindingKind,
        subject_node_id: Id,
        related_node_id: Option<Id>,
        evidence_status: EvidenceStatus,
        depth: Option<i64>,
        stop_reason: Option<StopReason>,
        /// More witness paths existed than the witness budget kept (presentation only; DESIGN
        /// §9.1's `omitted_paths`).
        witnesses_omitted: bool,
        score: Option<f64>,
        /// The guard a conditional finding depends on (Pass B).
        condition_node_id: Option<Id>,
    }
);

table!(
    /// The members of a finding that has several (a public alias's access paths; later a
    /// community's members, a ranking's entries, a concept's extent and intent), by role and
    /// ordinal. A member is a node, a cited fact, or both.
    FindingMembers, FindingMembersRow = "finding_members",
    family = Findings,
    key = [snapshot_id, finding_id, role, ordinal],
    checks = [("ordinal_nonnegative", "ordinal >= 0")],
    {
        snapshot_id: Id,
        finding_id: Id,
        role: MemberRole,
        ordinal: i64,
        node_id: Option<Id>,
        cited_fact_id: Option<Id>,
        label: Option<String>,
        weight: Option<f64>,
    }
);

table!(
    /// The ordered steps of a finding's witness paths (DESIGN §9.1): the persisted projection rows
    /// linking each arc to its evidence (§5). A step's identity is its call site and callee; its
    /// `edge_id` is lineage, because call edge ids are producer-scoped (ADR-0014 O6).
    Witnesses, WitnessesRow = "witnesses",
    family = Findings,
    key = [snapshot_id, finding_id, path, step],
    checks = [("path_nonnegative", "path >= 0"), ("step_nonnegative", "step >= 0")],
    {
        snapshot_id: Id,
        finding_id: Id,
        /// 0 is the first witness path; 1 and 2 the alternatives differing in the final arc.
        path: i64,
        step: i64,
        caller_node_id: Id,
        call_site_node_id: Id,
        callee_node_id: Id,
        edge_id: Id,
        modality: Modality,
        phase: InvocationPhase,
    }
);

/// Invoke `$mac!(Table, …)` with every analysis table, in declaration order (ADR-0019).
#[macro_export]
macro_rules! for_each_analysis_table {
    ($mac:ident) => {
        $mac!(
            $crate::findings::AnalysisInvocations,
            $crate::findings::Findings,
            $crate::findings::FindingMembers,
            $crate::findings::Witnesses
        )
    };
}

/// A witness step as its identity sees it: the call site, the callee, and the arc's modality and
/// phase (codebook values, so stable across producers); never the producer-scoped edge id
/// (review F9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StepKey {
    pub call_site: Id,
    pub callee: Id,
    pub modality: i16,
    pub phase: i16,
}

/// A member as its finding's identity sees it: every column but the weight (lineage).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemberKey {
    pub role: i16,
    pub ordinal: i64,
    pub node: Option<Id>,
    pub cited_fact: Option<Id>,
    pub label: Option<String>,
}

/// The evidence status each finding kind may carry (ADR-0019, review F6): a kernel never chooses
/// it freely, and the §10.2 status propagation reads it. Structural passes observe structure;
/// statistical methods (from increment 2) derive statistically.
pub const FINDING_STATUS: &[(FindingKind, EvidenceStatus)] = &[
    (
        FindingKind::PublicAlias,
        EvidenceStatus::StructurallyObserved,
    ),
    (
        FindingKind::DirectDelegation,
        EvidenceStatus::StructurallyObserved,
    ),
    (
        FindingKind::BoundedDelegationPath,
        EvidenceStatus::StructurallyObserved,
    ),
    (
        FindingKind::ImplementationBoundary,
        EvidenceStatus::StructurallyObserved,
    ),
    (
        FindingKind::IncompleteResolution,
        EvidenceStatus::StructurallyObserved,
    ),
];

/// The content-derived ids of analysis results (ADR-0019). Rust-only: no SQL recomputes them.
pub mod recipe {
    use super::{Digest, Id, IdHasher, MemberKey, StepKey, kind};

    /// An analytics config: its canonical JSON.
    pub fn config_digest(canonical_json: &str) -> Digest {
        IdHasher::new(kind::ANALYTICS_CONFIG)
            .str(canonical_json)
            .finish_digest()
    }

    /// An invocation: method, parameters, projection, subject and seed.
    pub fn invocation(
        method: i16,
        parameters: Digest,
        projection: Option<Digest>,
        subject: Option<Id>,
        seed: Option<i64>,
    ) -> Id {
        IdHasher::new(kind::INVOCATION)
            .i64(i64::from(method))
            .digest_field(parameters)
            .opt_digest(projection)
            .opt_id(subject)
            .opt_i64(seed)
            .finish_id()
    }

    /// A finding's identity: every identity column of its `findings` row (see [`super::Findings`])
    /// plus its child rows in key order: each witness path's steps, and its members. The
    /// invocation and the score are lineage, never hashed.
    pub struct FindingKey<'a> {
        pub finding_kind: i16,
        pub subject: Id,
        pub related: Option<Id>,
        pub condition: Option<Id>,
        pub evidence_status: i16,
        pub depth: Option<i64>,
        pub stop_reason: Option<i16>,
        pub witnesses_omitted: bool,
        pub paths: &'a [Vec<StepKey>],
        pub members: &'a [MemberKey],
    }

    impl FindingKey<'_> {
        pub fn id(&self) -> Id {
            let mut h = IdHasher::new(kind::FINDING);
            h.i64(i64::from(self.finding_kind))
                .id(self.subject)
                .opt_id(self.related)
                .opt_id(self.condition)
                .i64(i64::from(self.evidence_status))
                .opt_i64(self.depth)
                .opt_i64(self.stop_reason.map(i64::from))
                .bool(self.witnesses_omitted)
                .i64(self.paths.len() as i64);
            for path in self.paths {
                h.i64(path.len() as i64);
                for step in path {
                    h.id(step.call_site)
                        .id(step.callee)
                        .i64(i64::from(step.modality))
                        .i64(i64::from(step.phase));
                }
            }
            h.i64(self.members.len() as i64);
            for m in self.members {
                h.i64(i64::from(m.role))
                    .i64(m.ordinal)
                    .opt_id(m.node)
                    .opt_id(m.cited_fact)
                    .opt_str(m.label.as_deref());
            }
            h.finish_id()
        }
    }
}
