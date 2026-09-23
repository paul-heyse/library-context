//! Stage C and Stage D (DESIGN §3.2, §4.1): tables DataFusion computes from one snapshot's written
//! tables, stored through the same write path as raw tables.
//!
//! A derived row carries keys, the `fact_id`s of the rows it joins, and what the join decides. It
//! never copies a raw payload column, so the raw table stays the one authority, and it is
//! rebuildable from Delta (DM-23). A row the join cannot map is kept with a null node, and a
//! reason where one applies; no inner join drops it.

use crate::codebook::{
    BoundaryReason, Codebook, DeclarationKind, PysaCalleeKind, PysaSiteKind, PysaTargetKind,
    PysaUnresolvedReason, ResolutionDomain, ResolutionStatus, SignatureForm,
};
use crate::id::Id;
use crate::table::{Table, table};

/// A table computed by one query over the snapshot's tables, each registered under its own name
/// and already filtered to the snapshot.
pub trait Derived: Table {
    /// The query. It yields every declared column except `snapshot_id`, which the derive step
    /// adds, and is run through the read-only SQL helper.
    fn sql() -> String;
}

fn c(v: impl Codebook) -> i16 {
    v.code()
}

table!(
    /// Stage C: Pysa function keys → declaration nodes, joined on the name span within one file
    /// (DESIGN §4.1 C). The key is unique per snapshot. An unmapped key keeps a null node and a
    /// reason: `no_source_declaration` when Pysa gives no name span (a synthesized member such as
    /// a dataclass `__init__`, or a callable class field), `provider_disagreement` when no `def`
    /// sits at the span Pysa gives.
    ProviderNodeMap, ProviderNodeMapRow = "provider_node_map",
    family = Signatures,
    key = [snapshot_id, module_node_id, function_key],
    checks = [],
    {
        snapshot_id: Id,
        module_node_id: Id,
        function_key: String,
        node_id: Option<Id>,
        pysa_fact_id: Id,
        declaration_fact_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for ProviderNodeMap {
    fn sql() -> String {
        format!(
            "SELECT f.module_node_id, f.function_key, d.node_id AS node_id, \
                    f.fact_id AS pysa_fact_id, d.fact_id AS declaration_fact_id, \
                    CAST(CASE WHEN d.node_id IS NOT NULL THEN NULL \
                              WHEN f.name_start_byte IS NULL THEN {synthesized} \
                              ELSE {disagreement} END AS SMALLINT) AS reason \
             FROM pysa_functions f \
             LEFT JOIN declarations d \
               ON d.module_node_id = f.module_node_id \
              AND d.name_start_byte = f.name_start_byte \
              AND d.name_end_byte = f.name_end_byte",
            synthesized = c(BoundaryReason::NoSourceDeclaration),
            disagreement = c(BoundaryReason::ProviderDisagreement),
        )
    }
}

table!(
    /// Public access path → the declaration Pass A seeds from (DESIGN §9.1). Among the origin
    /// file's declarations of that name: an implementation before an `@overload` stub, then one
    /// Pysa describes (Stage C, so Pyrefly's binding choice under the context decides between
    /// `sys.version_info` or `TYPE_CHECKING` branches), then the last in source order. Null when
    /// the origin is not a `def` or `class` of the release: a variable, or a dependency. One row
    /// per `public_names` row, so a `.py`/`.pyi` pair gives an access path two rows, one per file
    /// (`source_files.is_stub` tells them apart).
    Exports, ExportsRow = "exports",
    family = Exports,
    key = [snapshot_id, access_path, public_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        access_path: String,
        declaration_node_id: Option<Id>,
        public_fact_id: Id,
        declaration_fact_id: Option<Id>,
    }
);

impl Derived for Exports {
    fn sql() -> String {
        "WITH keyed AS ( \
           SELECT DISTINCT node_id FROM provider_node_map WHERE node_id IS NOT NULL), \
         ranked AS ( \
           SELECT d.node_id, d.fact_id, d.module_node_id, d.qualified_name, \
                  row_number() OVER (PARTITION BY d.module_node_id, d.qualified_name \
                                     ORDER BY d.is_overload, k.node_id IS NULL, \
                                              d.start_byte DESC, d.node_id) AS pick \
           FROM declarations d LEFT JOIN keyed k ON k.node_id = d.node_id) \
         SELECT p.access_path, r.node_id AS declaration_node_id, \
                p.fact_id AS public_fact_id, r.fact_id AS declaration_fact_id \
         FROM public_names p \
         LEFT JOIN ranked r \
           ON r.pick = 1 \
          AND r.module_node_id = p.origin_module_node_id \
          AND r.qualified_name = p.origin_path"
            .to_owned()
    }
}

table!(
    /// One row per function `def` statement (DESIGN §3.4.1 overloads). Its callable is itself,
    /// except that an `@overload` stub rolls up to the first later `def` of its name that is an
    /// implementation or that Pysa describes (Pysa keys a stub-only group by its last stub), else
    /// to the group's last stub. Pysa folds overloads into the callable's function and gives one
    /// undecorated signature per stub, in source order, and none for an implementation;
    /// `signature_index` places each stub in that list. A callable Pysa does not describe is
    /// `unreachable_in_context` when a same-name `def` of the file is described (Pyrefly's
    /// context never binds it), else `missing_evidence`; counts that do not line up are
    /// `provider_disagreement`.
    Signatures, SignaturesRow = "signatures",
    family = Signatures,
    key = [snapshot_id, signature_node_id],
    checks = [(
        "signature_index_nonnegative",
        "signature_index IS NULL OR signature_index >= 0"
    )],
    {
        snapshot_id: Id,
        signature_node_id: Id,
        callable_node_id: Id,
        module_node_id: Id,
        /// The Pysa function of the callable.
        function_key: Option<String>,
        signature_index: Option<i64>,
        form: Option<SignatureForm>,
        declaration_fact_id: Id,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for Signatures {
    fn sql() -> String {
        format!(
            "WITH keyed AS ( \
               SELECT node_id, MIN(function_key) AS function_key \
               FROM provider_node_map WHERE node_id IS NOT NULL GROUP BY node_id), \
             fn AS ( \
               SELECT d.node_id, d.fact_id, d.module_node_id, d.qualified_name, d.is_overload, \
                      d.start_byte, k.function_key \
               FROM declarations d LEFT JOIN keyed k ON k.node_id = d.node_id \
               WHERE d.kind IN ({function}, {async_function})), \
             next_start AS ( \
               SELECT s.node_id, MIN(i.start_byte) AS start_byte \
               FROM fn s JOIN fn i \
                 ON i.module_node_id = s.module_node_id AND i.qualified_name = s.qualified_name \
                AND i.start_byte > s.start_byte \
                AND (NOT i.is_overload OR i.function_key IS NOT NULL) \
               WHERE s.is_overload AND s.function_key IS NULL GROUP BY s.node_id), \
             last_stub AS ( \
               SELECT module_node_id, qualified_name, MAX(start_byte) AS start_byte \
               FROM fn WHERE is_overload GROUP BY module_node_id, qualified_name), \
             owned AS ( \
               SELECT s.node_id, s.fact_id, s.module_node_id, s.qualified_name, s.is_overload, \
                      s.start_byte, \
                      CASE WHEN s.is_overload AND s.function_key IS NULL \
                           THEN COALESCE(n.start_byte, l.start_byte) \
                           ELSE s.start_byte END AS callable_start \
               FROM fn s \
               LEFT JOIN next_start n ON n.node_id = s.node_id \
               LEFT JOIN last_stub l \
                 ON l.module_node_id = s.module_node_id AND l.qualified_name = s.qualified_name), \
             placed AS ( \
               SELECT o.node_id, o.fact_id, o.module_node_id, o.qualified_name, o.is_overload, \
                      k.node_id AS callable_node_id, k.function_key, \
                      row_number() OVER (PARTITION BY o.module_node_id, o.qualified_name, \
                                         o.callable_start, o.is_overload \
                                         ORDER BY o.start_byte) - 1 \
                        AS stub_ordinal, \
                      SUM(CASE WHEN o.is_overload THEN 1 ELSE 0 END) \
                        OVER (PARTITION BY o.module_node_id, o.qualified_name, o.callable_start) \
                        AS stubs \
               FROM owned o \
               JOIN fn k \
                 ON k.module_node_id = o.module_node_id AND k.qualified_name = o.qualified_name \
                AND k.start_byte = o.callable_start), \
             described AS ( \
               SELECT DISTINCT module_node_id, qualified_name FROM fn \
               WHERE function_key IS NOT NULL), \
             sig AS ( \
               SELECT p.node_id, p.fact_id, p.module_node_id, p.callable_node_id, p.function_key, \
                      CASE WHEN p.function_key IS NULL THEN NULL \
                           WHEN p.is_overload AND p.stub_ordinal < f.signature_count \
                             THEN p.stub_ordinal \
                           WHEN NOT p.is_overload AND p.stubs = 0 AND f.signature_count > 0 \
                             THEN 0 END AS signature_index, \
                      CASE WHEN p.function_key IS NULL AND g.qualified_name IS NOT NULL \
                             THEN {unreachable} \
                           WHEN p.function_key IS NULL THEN {missing} \
                           WHEN p.is_overload AND p.stub_ordinal >= f.signature_count \
                             THEN {disagreement} \
                           WHEN p.node_id = p.callable_node_id AND p.stubs > 0 \
                            AND p.stubs <> f.signature_count THEN {disagreement} \
                           WHEN NOT p.is_overload AND p.stubs = 0 \
                            AND f.signature_count <> 1 THEN {disagreement} END AS reason \
               FROM placed p \
               LEFT JOIN described g \
                 ON g.module_node_id = p.module_node_id AND g.qualified_name = p.qualified_name \
               LEFT JOIN pysa_functions f \
                 ON f.module_node_id = p.module_node_id AND f.function_key = p.function_key), \
             forms AS ( \
               SELECT module_node_id, function_key, signature_index, MIN(form) AS form \
               FROM parameter_semantics WHERE ordinal IS NULL \
               GROUP BY module_node_id, function_key, signature_index) \
             SELECT s.node_id AS signature_node_id, s.callable_node_id, s.module_node_id, \
                    s.function_key, s.signature_index, \
                    CAST(CASE WHEN s.signature_index IS NULL THEN NULL \
                              ELSE COALESCE(m.form, {list}) END AS SMALLINT) AS form, \
                    s.fact_id AS declaration_fact_id, CAST(s.reason AS SMALLINT) AS reason \
             FROM sig s \
             LEFT JOIN forms m \
               ON m.module_node_id = s.module_node_id AND m.function_key = s.function_key \
              AND m.signature_index = s.signature_index",
            function = c(DeclarationKind::Function),
            async_function = c(DeclarationKind::AsyncFunction),
            unreachable = c(BoundaryReason::UnreachableInContext),
            missing = c(BoundaryReason::MissingEvidence),
            disagreement = c(BoundaryReason::ProviderDisagreement),
            list = c(SignatureForm::List),
        )
    }
}

table!(
    /// Parameters per signature: Ruff's row and Pysa's row joined on the ordinal, keeping either
    /// side alone. A missing side (where Pysa gives a parameter list), a differing name or a
    /// differing kind is `provider_disagreement`.
    Parameters, ParametersRow = "parameters",
    family = Signatures,
    key = [snapshot_id, signature_node_id, ordinal],
    checks = [("ordinal_nonnegative", "ordinal >= 0")],
    {
        snapshot_id: Id,
        signature_node_id: Id,
        ordinal: i64,
        syntax_fact_id: Option<Id>,
        semantics_fact_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for Parameters {
    fn sql() -> String {
        format!(
            "WITH syn AS ( \
               SELECT s.signature_node_id, p.ordinal, p.fact_id, p.name, p.kind \
               FROM signatures s JOIN parameter_syntax p ON p.function_node_id = s.signature_node_id), \
             sem AS ( \
               SELECT s.signature_node_id, q.ordinal, q.fact_id, q.name, q.kind \
               FROM signatures s JOIN parameter_semantics q \
                 ON q.module_node_id = s.module_node_id AND q.function_key = s.function_key \
                AND q.signature_index = s.signature_index AND q.ordinal IS NOT NULL), \
             joined AS ( \
               SELECT COALESCE(a.signature_node_id, b.signature_node_id) AS signature_node_id, \
                      COALESCE(a.ordinal, b.ordinal) AS ordinal, \
                      a.fact_id AS syntax_fact_id, b.fact_id AS semantics_fact_id, \
                      a.name AS syntax_name, b.name AS semantics_name, \
                      a.kind AS syntax_kind, b.kind AS semantics_kind \
               FROM syn a FULL OUTER JOIN sem b \
                 ON a.signature_node_id = b.signature_node_id AND a.ordinal = b.ordinal) \
             SELECT j.signature_node_id, j.ordinal, j.syntax_fact_id, j.semantics_fact_id, \
                    CAST(CASE WHEN j.syntax_fact_id IS NULL \
                                OR (j.semantics_fact_id IS NULL AND s.form = {list}) \
                                OR j.syntax_name <> j.semantics_name \
                                OR j.syntax_kind <> j.semantics_kind \
                              THEN {disagreement} END AS SMALLINT) AS reason \
             FROM joined j JOIN signatures s ON s.signature_node_id = j.signature_node_id",
            list = c(SignatureForm::List),
            disagreement = c(BoundaryReason::ProviderDisagreement),
        )
    }
}

table!(
    /// One resolution set per call site (DESIGN §3.6); the call sites are the `call_syntax` rows.
    /// Pysa's regular call records at the full call range give the targets (higher-order
    /// argument targets excluded). With no record the status is `not_attempted`: inside an
    /// annotation `outside_provider_model`, else `missing_evidence`.
    Resolutions, ResolutionsRow = "resolutions",
    family = Calls,
    key = [snapshot_id, call_site_node_id],
    checks = [("target_count_nonnegative", "target_count >= 0")],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        status: ResolutionStatus,
        domain: ResolutionDomain,
        has_unresolved_remainder: bool,
        /// False when an `Overrides` target leaves the set open, or anything is unresolved.
        candidate_set_complete_under_model: bool,
        unresolved_reason: Option<PysaUnresolvedReason>,
        target_count: i64,
        call_fact_id: Id,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for Resolutions {
    fn sql() -> String {
        format!(
            "WITH hits AS ( \
               SELECT c.node_id, p.target_kind, p.unresolved_reason \
               FROM call_syntax c JOIN pysa_calls p \
                 ON p.module_node_id = c.module_node_id AND p.start_byte = c.start_byte \
                AND p.end_byte = c.end_byte AND p.site_kind = {regular} \
                AND p.callee_kind = {call} AND p.higher_order_index IS NULL), \
             agg AS ( \
               SELECT c.node_id, c.fact_id, c.in_annotation, \
                      COUNT(h.target_kind) AS pysa_rows, \
                      SUM(CASE WHEN h.target_kind <> {unresolved} THEN 1 ELSE 0 END) AS targets, \
                      SUM(CASE WHEN h.target_kind = {unresolved} THEN 1 ELSE 0 END) AS remainders, \
                      SUM(CASE WHEN h.target_kind = {overrides} THEN 1 ELSE 0 END) AS overrides, \
                      MIN(h.unresolved_reason) AS unresolved_reason \
               FROM call_syntax c LEFT JOIN hits h ON h.node_id = c.node_id \
               GROUP BY c.node_id, c.fact_id, c.in_annotation) \
             SELECT node_id AS call_site_node_id, \
                    CAST(CASE WHEN pysa_rows = 0 THEN {not_attempted} \
                              WHEN targets = 0 THEN {status_unresolved} \
                              WHEN remainders > 0 THEN {partial} \
                              ELSE {resolved} END AS SMALLINT) AS status, \
                    CAST({domain} AS SMALLINT) AS domain, \
                    NOT (targets > 0 AND remainders = 0) AS has_unresolved_remainder, \
                    (targets > 0 AND remainders = 0 AND overrides = 0) \
                      AS candidate_set_complete_under_model, \
                    unresolved_reason, \
                    CAST(targets AS BIGINT) AS target_count, \
                    fact_id AS call_fact_id, \
                    CAST(CASE WHEN pysa_rows = 0 AND in_annotation THEN {outside} \
                              WHEN pysa_rows = 0 THEN {missing} END AS SMALLINT) AS reason \
             FROM agg",
            regular = c(PysaSiteKind::Regular),
            call = c(PysaCalleeKind::Call),
            unresolved = c(PysaTargetKind::Unresolved),
            overrides = c(PysaTargetKind::Overrides),
            not_attempted = c(ResolutionStatus::NotAttempted),
            status_unresolved = c(ResolutionStatus::Unresolved),
            partial = c(ResolutionStatus::Partial),
            resolved = c(ResolutionStatus::Resolved),
            domain = c(ResolutionDomain::Call),
            outside = c(BoundaryReason::OutsideProviderModel),
            missing = c(BoundaryReason::MissingEvidence),
        )
    }
}

table!(
    /// Each Pysa target of a call site (higher-order argument targets included), with the
    /// declaration it names when the target is a function of the release. A release target with
    /// no declaration carries the Stage-C reason (`missing_evidence` if Pysa has no such key);
    /// a null node with a null reason means a target outside the release. Modality, origin and
    /// phase stay on the Pysa row and its `facts` row.
    CallTargets, CallTargetsRow = "call_targets",
    family = Calls,
    key = [snapshot_id, call_site_node_id, pysa_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        pysa_fact_id: Id,
        target_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for CallTargets {
    fn sql() -> String {
        format!(
            "SELECT c.node_id AS call_site_node_id, p.fact_id AS pysa_fact_id, \
                    m.node_id AS target_node_id, \
                    CAST(CASE WHEN f.module_node_id IS NULL THEN NULL \
                              WHEN m.function_key IS NULL THEN {missing} \
                              ELSE m.reason END AS SMALLINT) AS reason \
             FROM call_syntax c \
             JOIN pysa_calls p \
               ON p.module_node_id = c.module_node_id AND p.start_byte = c.start_byte \
              AND p.end_byte = c.end_byte AND p.site_kind = {regular} \
              AND p.callee_kind = {call} AND p.target_kind <> {unresolved} \
             LEFT JOIN source_files f ON p.target_module = '@' || f.path \
             LEFT JOIN provider_node_map m \
               ON m.module_node_id = f.module_node_id AND m.function_key = p.target_key",
            regular = c(PysaSiteKind::Regular),
            call = c(PysaCalleeKind::Call),
            unresolved = c(PysaTargetKind::Unresolved),
            missing = c(BoundaryReason::MissingEvidence),
        )
    }
}

/// Invoke `$mac!(Table, …)` with every derived table, in dependency order: each query reads only
/// raw tables and the derived tables before it.
#[macro_export]
macro_rules! for_each_derived_table {
    ($mac:ident) => {
        $mac!(
            $crate::derived::ProviderNodeMap,
            $crate::derived::Exports,
            $crate::derived::Signatures,
            $crate::derived::Parameters,
            $crate::derived::Resolutions,
            $crate::derived::CallTargets
        )
    };
}

/// Every derivation's query, in dependency order (snapshot-tested with the contracts).
pub fn derivations() -> Vec<(&'static str, String)> {
    macro_rules! all {
        ($($t:ty),+) => { vec![$((<$t as Table>::NAME, <$t as Derived>::sql())),+] };
    }
    crate::for_each_derived_table!(all)
}
