//! Declared graph projections (DESIGN §5; guidelines §3): what an analysis reads from the
//! catalogs, stated as data before any algorithm runs.
//!
//! A projection names its vertex universe (node kinds, isolates included) separately from the
//! arcs, the edge kinds it selects, the modalities, origins and fidelities it accepts, its
//! candidate- and unknown-target policies, and its weight policy. Its SQL is generated here, and
//! its digest joins the parameters of every invocation that reads it (ADR-0019).

use crate::codebook::{
    ArcKind, Codebook, EdgeKind, Fidelity, InvocationPhase, Modality, NodeKind, Origin,
};
use crate::id::{Digest, IdHasher};

/// A declared projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionSpec {
    pub name: &'static str,
    /// The vertex universe, selected separately from the arcs, so an isolate survives.
    pub vertex_kinds: &'static [NodeKind],
    /// The edge kinds the arcs come from.
    pub edge_kinds: &'static [EdgeKind],
    /// The modalities an arc may carry.
    pub modalities: &'static [Modality],
    /// The origins an arc's evidence may have.
    pub origins: &'static [Origin],
    /// The fidelities an arc's evidence may have.
    pub fidelities: &'static [Fidelity],
    /// What a candidate target means for a traversal.
    pub candidate_policy: &'static str,
    /// What happens to a call site with no target or an unresolved remainder.
    pub unknown_target_policy: &'static str,
    /// What an arc's weight is.
    pub weight_policy: &'static str,
    /// Queries yielding the vertices, the arcs and the unresolved sites, each totally ordered.
    pub vertices_sql: String,
    pub arcs_sql: String,
    pub unresolved_sql: String,
}

impl ProjectionSpec {
    /// The spec's identity: every declared field and query.
    pub fn digest(&self) -> Digest {
        let codes = |v: &mut IdHasher, xs: &[i16]| {
            v.i64(xs.len() as i64);
            for x in xs {
                v.i64(i64::from(*x));
            }
        };
        let mut h = IdHasher::new("projection");
        h.str(self.name);
        codes(
            &mut h,
            &self
                .vertex_kinds
                .iter()
                .map(|k| k.code())
                .collect::<Vec<_>>(),
        );
        codes(
            &mut h,
            &self.edge_kinds.iter().map(|k| k.code()).collect::<Vec<_>>(),
        );
        codes(
            &mut h,
            &self.modalities.iter().map(|k| k.code()).collect::<Vec<_>>(),
        );
        codes(
            &mut h,
            &self.origins.iter().map(|k| k.code()).collect::<Vec<_>>(),
        );
        codes(
            &mut h,
            &self.fidelities.iter().map(|k| k.code()).collect::<Vec<_>>(),
        );
        h.str(self.candidate_policy)
            .str(self.unknown_target_policy)
            .str(self.weight_policy)
            .str(&self.vertices_sql)
            .str(&self.arcs_sql)
            .str(&self.unresolved_sql);
        h.finish_digest()
    }
}

fn list(xs: &[i16]) -> String {
    xs.iter().map(i16::to_string).collect::<Vec<_>>().join(", ")
}

/// The invocation projection (DESIGN §5, v1): who may call whom, per call site.
///
/// - **Vertices:** every module, class, function, synthetic callable and dependency symbol, with
///   its module's name and role (null outside the analyzed releases), ordered by `node_id`.
/// - **Arcs:** `encloses_call` ⋈ `call_target` (the caller is the call's owner: its function, or
///   the class or module whose body holds it), plus the `site_target` arcs of property getters and
///   setters, whose site's owner is read from `syntax_nodes`. Each carries its call site (also
///   the resolution's id, §3.6), `edge_id`, phase, modality and whether its resolution has an
///   unresolved remainder, ordered by `(src, dst, call_site, edge_id)`, a total order because
///   `edge_id` is unique. Parallel call sites stay distinct arcs.
/// - **Definition arcs** (slice 1.4 review F1): the `declares` edges from a function to a function
///   it defines, so a decorator factory's nested callable, which the caller invokes later, is
///   reached. Their site is the nested declaration, and they carry no phase.
/// - **Excluded:** `potential` targets (every `higher_order_target`) and `synthetic_model`
///   evidence. Candidate (`Overrides`) arcs stay, with their modality.
/// - **Unknown targets:** no arc to a placeholder; a site with no target or an unresolved
///   remainder is a row of the unresolved-sites query.
pub fn invocation() -> ProjectionSpec {
    const VERTICES: &[NodeKind] = &[
        NodeKind::Module,
        NodeKind::Class,
        NodeKind::Function,
        NodeKind::SyntheticCallable,
        NodeKind::ExternalSymbol,
    ];
    const EDGES: &[EdgeKind] = &[
        EdgeKind::EnclosesCall,
        EdgeKind::CallTarget,
        EdgeKind::SiteTarget,
        EdgeKind::Declares,
    ];
    const MODALITIES: &[Modality] = &[Modality::Definite, Modality::Candidate];
    const ORIGINS: &[Origin] = &[Origin::AnalyzerAssertion];
    const FIDELITIES: &[Fidelity] = &[Fidelity::ReportProjection];
    let kinds = list(&VERTICES.iter().map(|k| k.code()).collect::<Vec<_>>());
    let modalities = list(&MODALITIES.iter().map(|k| k.code()).collect::<Vec<_>>());
    let origins = list(&ORIGINS.iter().map(|k| k.code()).collect::<Vec<_>>());
    let fidelities = list(&FIDELITIES.iter().map(|k| k.code()).collect::<Vec<_>>());
    let encloses = EdgeKind::EnclosesCall.code();
    let call_target = EdgeKind::CallTarget.code();
    let site_target = EdgeKind::SiteTarget.code();
    let property = list(&[
        InvocationPhase::PropertyGet.code(),
        InvocationPhase::PropertySet.code(),
    ]);
    let vertices_sql = format!(
        "SELECT n.node_id, n.node_kind, s.module_name, s.role \
         FROM nodes n LEFT JOIN source_files s ON s.module_node_id = n.module_node_id \
         WHERE n.node_kind IN ({kinds}) \
         ORDER BY n.node_id"
    );
    let accepted = format!(
        "f.modality IN ({modalities}) AND f.origin IN ({origins}) AND f.fidelity IN ({fidelities})"
    );
    let declares = EdgeKind::Declares.code();
    let function = NodeKind::Function.code();
    let (call, definition) = (ArcKind::Call.code(), ArcKind::Definition.code());
    let definite = Modality::Definite.code();
    let owner = crate::graph::owner_of("sn");
    let arcs_sql = format!(
        "SELECT caller AS src_node_id, callee AS dst_node_id, call_site_node_id, edge_id, \
                phase, modality, has_unresolved_remainder, arc_kind FROM ( \
           SELECT ec.src_node_id AS caller, ct.dst_node_id AS callee, \
                  ct.src_node_id AS call_site_node_id, ct.edge_id, p.phase, f.modality, \
                  r.has_unresolved_remainder, CAST({call} AS SMALLINT) AS arc_kind \
           FROM edges ct \
           JOIN edges ec ON ec.dst_node_id = ct.src_node_id AND ec.edge_kind = {encloses} \
           JOIN pysa_calls p ON p.fact_id = ct.evidence_fact_id \
           JOIN facts f ON f.fact_id = ct.evidence_fact_id \
           JOIN resolutions r ON r.call_site_node_id = ct.src_node_id \
           WHERE ct.edge_kind = {call_target} AND {accepted} \
           UNION ALL \
           SELECT {owner} AS caller, \
                  st.dst_node_id AS callee, st.src_node_id AS call_site_node_id, st.edge_id, \
                  p.phase, f.modality, false AS has_unresolved_remainder, \
                  CAST({call} AS SMALLINT) AS arc_kind \
           FROM edges st \
           JOIN syntax_nodes sn ON sn.node_id = st.src_node_id \
           JOIN pysa_calls p ON p.fact_id = st.evidence_fact_id \
           JOIN facts f ON f.fact_id = st.evidence_fact_id \
           WHERE st.edge_kind = {site_target} AND p.phase IN ({property}) AND {accepted} \
           UNION ALL \
           SELECT d.src_node_id AS caller, d.dst_node_id AS callee, \
                  d.dst_node_id AS call_site_node_id, d.edge_id, CAST(NULL AS SMALLINT) AS phase, \
                  CAST({definite} AS SMALLINT) AS modality, false AS has_unresolved_remainder, \
                  CAST({definition} AS SMALLINT) AS arc_kind \
           FROM edges d \
           WHERE d.edge_kind = {declares} AND d.src_kind = {function} \
             AND d.dst_kind = {function}) a \
         ORDER BY src_node_id, dst_node_id, call_site_node_id, edge_id"
    );
    let unresolved_sql = format!(
        "SELECT ec.src_node_id AS caller_node_id, r.call_site_node_id, r.target_count, \
                r.has_unresolved_remainder, r.unresolved_reason, r.reason \
         FROM resolutions r \
         JOIN edges ec ON ec.dst_node_id = r.call_site_node_id AND ec.edge_kind = {encloses} \
         WHERE r.target_count = 0 OR r.has_unresolved_remainder \
         ORDER BY caller_node_id, r.call_site_node_id"
    );
    ProjectionSpec {
        name: "invocation",
        vertex_kinds: VERTICES,
        edge_kinds: EDGES,
        modalities: MODALITIES,
        origins: ORIGINS,
        fidelities: FIDELITIES,
        candidate_policy: "kept with modality `candidate`: a traversal may follow it, never as a \
                           direct delegation (DESIGN §3.6); a definition arc is followed but is \
                           never a call",
        unknown_target_policy: "no arc to a placeholder; a call site with no target or an \
                                unresolved remainder is listed in the unresolved-sites relation",
        weight_policy: "none: each arc is one call site's resolution to one target",
        vertices_sql,
        arcs_sql,
        unresolved_sql,
    }
}

/// Every declared projection, for the compiler digest.
pub fn projections() -> Vec<ProjectionSpec> {
    vec![invocation()]
}

/// The declared output schemas of a projection's three queries: each result is cast to its schema
/// (strictly) before an adapter reads it, as a derived table's is (§4.3).
pub mod schemas {
    use std::sync::Arc;

    use arrow_schema::{DataType, Field, Schema, SchemaRef};

    fn id(name: &str, nullable: bool) -> Field {
        Field::new(name, DataType::FixedSizeBinary(16), nullable)
    }

    pub fn vertices() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("node_id", false),
            Field::new("node_kind", DataType::Int16, false),
            Field::new("module_name", DataType::Utf8, true),
            Field::new("role", DataType::Int16, true),
        ]))
    }

    pub fn arcs() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("src_node_id", false),
            id("dst_node_id", false),
            id("call_site_node_id", false),
            id("edge_id", false),
            Field::new("phase", DataType::Int16, true),
            Field::new("modality", DataType::Int16, false),
            Field::new("has_unresolved_remainder", DataType::Boolean, false),
            Field::new("arc_kind", DataType::Int16, false),
        ]))
    }

    pub fn unresolved() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("caller_node_id", false),
            id("call_site_node_id", false),
            Field::new("target_count", DataType::Int64, false),
            Field::new("has_unresolved_remainder", DataType::Boolean, false),
            Field::new("unresolved_reason", DataType::Int16, true),
            Field::new("reason", DataType::Int16, true),
        ]))
    }
}
