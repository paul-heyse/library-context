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
    AnalyticMethod, AssertionKind, BriefSection, CoverageStatus, EvidenceKind, EvidenceStatus,
    ExtractionMode, FindingKind, InvocationPhase, MemberRole, Modality, ReviewState, StopReason,
    SupportRole,
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

table!(
    /// One cited thing (DESIGN §3.2; ADR-0019): a fact with its source span, a source span, a
    /// passage span, an example or a fixture run, with the **resolved text** the serving bundle
    /// carries. The text is the exact bytes of its span (review F8).
    ///
    /// **Identity:** kind, node, module, span and the digest of the resolved text. **Lineage:**
    /// the cited `fact_id`, which is run-scoped (review F7).
    Evidence, EvidenceRow = "evidence",
    family = Findings,
    key = [snapshot_id, evidence_id],
    checks = [
        ("span_order", "start_byte IS NULL OR end_byte IS NULL OR start_byte <= end_byte"),
        ("start_nonnegative", "start_byte IS NULL OR start_byte >= 0"),
    ],
    {
        snapshot_id: Id,
        evidence_id: Id,
        evidence_kind: EvidenceKind,
        cited_fact_id: Option<Id>,
        /// The cited node: a declaration, a parameter, a passage, a module.
        node_id: Option<Id>,
        /// The module or document whose bytes the span indexes.
        module_node_id: Option<Id>,
        start_byte: Option<i64>,
        end_byte: Option<i64>,
        text: Option<String>,
    }
);

table!(
    /// An atomic assertion (DESIGN §10.2), filled by its kind's template from findings and
    /// evidence. Its status is derived from its supports, never chosen; an `unresolved`
    /// assertion has no text. Its section is its kind's (`ASSERTION_POLICY`).
    ///
    /// **Identity:** kind, subject, applicable case, status, text, conditions, limitations and
    /// its supports. **Lineage:** run, model, extraction mode, template version.
    Assertions, AssertionsRow = "assertions",
    family = Findings,
    key = [snapshot_id, assertion_id],
    checks = [("template_version_positive", "template_version > 0")],
    {
        snapshot_id: Id,
        assertion_id: Id,
        run_id: Id,
        model_id: String,
        extraction_mode: ExtractionMode,
        assertion_kind: AssertionKind,
        subject_node_id: Id,
        applicable_case: Option<String>,
        evidence_status: EvidenceStatus,
        text: Option<String>,
        conditions: Option<String>,
        limitations: Option<String>,
        template_version: i64,
    }
);

table!(
    /// An assertion's supporting findings and evidence, each with its role (DESIGN §10.2).
    AssertionSupport, AssertionSupportRow = "assertion_support",
    family = Findings,
    key = [snapshot_id, assertion_id, role, ordinal],
    checks = [("ordinal_nonnegative", "ordinal >= 0")],
    {
        snapshot_id: Id,
        assertion_id: Id,
        role: SupportRole,
        ordinal: i64,
        finding_id: Option<Id>,
        evidence_id: Option<Id>,
    }
);

table!(
    /// One brief: one outcome anchored to one public operation (DESIGN §10.3). `capability_id` is
    /// `brief_id`. `review_state` is outside the id (ADR-0020).
    Briefs, BriefsRow = "briefs",
    family = Findings,
    key = [snapshot_id, brief_id],
    checks = [],
    {
        snapshot_id: Id,
        brief_id: Id,
        run_id: Id,
        model_id: String,
        /// The public operation's declaration.
        seed_node_id: Id,
        /// The configured access path of the seed.
        access_path: String,
        title: String,
        applicable_case: Option<String>,
        /// No analytic finding supports it: it is honestly documentation alone (§1.5).
        documentation_only: bool,
        review_state: ReviewState,
    }
);

table!(
    /// A brief's assertions in presentation order (sections in `brief_section` order).
    BriefAssertions, BriefAssertionsRow = "brief_assertions",
    family = Findings,
    key = [snapshot_id, brief_id, ordinal],
    checks = [("ordinal_nonnegative", "ordinal >= 0")],
    {
        snapshot_id: Id,
        brief_id: Id,
        ordinal: i64,
        assertion_id: Id,
    }
);

table!(
    /// The public access paths a brief is about: `symbol_map` (§6.4) is built from them.
    BriefMembers, BriefMembersRow = "brief_members",
    family = Findings,
    key = [snapshot_id, brief_id, access_path],
    checks = [],
    {
        snapshot_id: Id,
        brief_id: Id,
        access_path: String,
        /// The export node whose access path the member's path extends.
        export_node_id: Id,
        declaration_node_id: Id,
    }
);

table!(
    /// A brief's embedding projection (DESIGN §11.1), chunked: the text the embedder receives,
    /// and its cache key once embedded (slice 1.6).
    BriefDocuments, BriefDocumentsRow = "brief_documents",
    family = Findings,
    key = [snapshot_id, brief_id, chunk],
    checks = [("chunk_nonnegative", "chunk >= 0")],
    {
        snapshot_id: Id,
        brief_id: Id,
        chunk: i64,
        text: String,
        input_hash: Option<Digest>,
    }
);

table!(
    /// The kind policy (DESIGN §10.2): each assertion kind's section and permitted statuses,
    /// published per snapshot from `ASSERTION_POLICY`.
    AssertionPolicy, AssertionPolicyRow = "assertion_policy",
    family = Findings,
    key = [snapshot_id, assertion_kind, evidence_status],
    checks = [],
    {
        snapshot_id: Id,
        assertion_kind: AssertionKind,
        brief_section: BriefSection,
        evidence_status: EvidenceStatus,
    }
);

/// Each assertion kind's brief section and permitted statuses (DESIGN §10.2; ADR-0005).
/// Statistical output may set titles, grouping, seeds, ordering, Related entries and doc links,
/// never a control, a limit or a behavioural claim.
pub const ASSERTION_POLICY: &[(AssertionKind, BriefSection, &[EvidenceStatus])] = &[
    (
        AssertionKind::Outcome,
        BriefSection::Outcome,
        &[EvidenceStatus::Documented, EvidenceStatus::Unresolved],
    ),
    (
        AssertionKind::PublicAccess,
        BriefSection::PublicAccess,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::Coordinates,
        BriefSection::PublicAccess,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::Parameter,
        BriefSection::Controls,
        &[
            EvidenceStatus::StructurallyObserved,
            EvidenceStatus::Documented,
        ],
    ),
    (
        AssertionKind::AnalysisBoundary,
        BriefSection::Limits,
        &[EvidenceStatus::StructurallyObserved],
    ),
    (
        AssertionKind::Related,
        BriefSection::Related,
        &[EvidenceStatus::StatisticallyDerived],
    ),
];

/// An assertion kind's section.
pub fn section_of(kind: AssertionKind) -> BriefSection {
    ASSERTION_POLICY
        .iter()
        .find(|(k, _, _)| *k == kind)
        .map(|(_, s, _)| *s)
        .expect("every assertion kind has a policy row")
}

/// Invoke `$mac!(Table, …)` with every analysis table, in declaration order (ADR-0019).
#[macro_export]
macro_rules! for_each_analysis_table {
    ($mac:ident) => {
        $mac!(
            $crate::findings::AnalysisInvocations,
            $crate::findings::Findings,
            $crate::findings::FindingMembers,
            $crate::findings::Witnesses,
            $crate::findings::Evidence,
            $crate::findings::Assertions,
            $crate::findings::AssertionSupport,
            $crate::findings::Briefs,
            $crate::findings::BriefAssertions,
            $crate::findings::BriefMembers,
            $crate::findings::BriefDocuments,
            $crate::findings::AssertionPolicy
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

    /// An evidence row's identity: kind, node, module, span and the digest of its resolved text.
    /// The cited fact is lineage (review F7).
    pub fn evidence(
        kind_code: i16,
        node: Option<Id>,
        module: Option<Id>,
        span: Option<(i64, i64)>,
        text: Option<&str>,
    ) -> Id {
        IdHasher::new(kind::EVIDENCE)
            .i64(i64::from(kind_code))
            .opt_id(node)
            .opt_id(module)
            .opt_i64(span.map(|s| s.0))
            .opt_i64(span.map(|s| s.1))
            .opt_digest(text.map(|t| crate::id::content_digest(t.as_bytes())))
            .finish_id()
    }

    /// An assertion's identity: its content and its supports `(role, finding, evidence)` in order.
    pub struct AssertionKey<'a> {
        pub kind: i16,
        pub subject: Id,
        pub applicable_case: Option<&'a str>,
        pub status: i16,
        pub text: Option<&'a str>,
        pub conditions: Option<&'a str>,
        pub limitations: Option<&'a str>,
        pub supports: &'a [(i16, Option<Id>, Option<Id>)],
    }

    impl AssertionKey<'_> {
        pub fn id(&self) -> Id {
            let mut h = IdHasher::new(kind::ASSERTION);
            h.i64(i64::from(self.kind))
                .id(self.subject)
                .opt_str(self.applicable_case)
                .i64(i64::from(self.status))
                .opt_str(self.text)
                .opt_str(self.conditions)
                .opt_str(self.limitations)
                .i64(self.supports.len() as i64);
            for (role, finding, evidence) in self.supports {
                h.i64(i64::from(*role)).opt_id(*finding).opt_id(*evidence);
            }
            h.finish_id()
        }
    }

    /// A brief's identity: its seed, applicable case and its assertions in presentation order.
    /// A changed brief is a new brief (ADR-0020).
    pub fn brief(seed: Id, applicable_case: Option<&str>, assertions: &[Id]) -> Id {
        let mut h = IdHasher::new(kind::BRIEF);
        h.id(seed)
            .opt_str(applicable_case)
            .i64(assertions.len() as i64);
        for a in assertions {
            h.id(*a);
        }
        h.finish_id()
    }
}
