//! Stage C and Stage D (DESIGN §3.2, §4.1): tables DataFusion computes from one snapshot's written
//! tables, stored through the same write path as raw tables.
//!
//! A derived row carries keys, the `fact_id`s of the rows it joins, and what the join decides. It
//! never copies a raw payload column, so the raw table stays the one authority, and it is
//! rebuildable from Delta (DM-23). A row the join cannot map is kept with a null node, and a
//! reason where one applies; no inner join drops it.

use crate::codebook::{
    BindingKind, BoundaryReason, Codebook, DeclarationKind, DefinitionKind, ExportSyntaxKind,
    LexicalScopeKind, ModuleOrigin, PysaCalleeKind, PysaSiteKind, PysaTargetKind,
    PysaUnresolvedReason, ResolutionDomain, ResolutionStatus, SignatureForm, SymbolKind,
    SyntaxKind,
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
    /// Stage C for classes: Pysa class keys → class declarations, joined on the name span within
    /// one file. A synthesized class (a functional `namedtuple`) reads `no_source_declaration`; a
    /// class statement Pysa places where no declaration sits reads `provider_disagreement`.
    ProviderClassMap, ProviderClassMapRow = "provider_class_map",
    family = Signatures,
    key = [snapshot_id, module_node_id, class_key],
    checks = [],
    {
        snapshot_id: Id,
        module_node_id: Id,
        class_key: String,
        node_id: Option<Id>,
        pysa_fact_id: Id,
        declaration_fact_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for ProviderClassMap {
    fn sql() -> String {
        format!(
            "SELECT c.module_node_id, c.class_key, d.node_id AS node_id, \
                    c.fact_id AS pysa_fact_id, d.fact_id AS declaration_fact_id, \
                    CAST(CASE WHEN d.node_id IS NOT NULL THEN NULL \
                              WHEN c.is_synthesized THEN {synthesized} \
                              ELSE {disagreement} END AS SMALLINT) AS reason \
             FROM pysa_classes c \
             LEFT JOIN declarations d \
               ON d.module_node_id = c.module_node_id AND d.kind = {class} \
              AND d.name_start_byte = c.name_start_byte \
              AND d.name_end_byte = c.name_end_byte",
            synthesized = c(BoundaryReason::NoSourceDeclaration),
            disagreement = c(BoundaryReason::ProviderDisagreement),
            class = c(DeclarationKind::Class),
        )
    }
}

table!(
    /// The `synthetic_callable` nodes (DESIGN §3.8): a Pysa function with no `def` of its own (a
    /// dataclass `__init__`, a callable class field), so a call to one has a typed target.
    SyntheticCallables, SyntheticCallablesRow = "synthetic_callables",
    family = Signatures,
    key = [snapshot_id, node_id],
    checks = [],
    {
        snapshot_id: Id,
        node_id: Id,
        module_node_id: Id,
        function_key: String,
        pysa_fact_id: Id,
    }
);

impl Derived for SyntheticCallables {
    fn sql() -> String {
        format!(
            "SELECT lctx_id('synthetic_callable', module_node_id, function_key) AS node_id, \
                    module_node_id, function_key, pysa_fact_id \
             FROM provider_node_map WHERE node_id IS NULL AND reason = {synthesized}",
            synthesized = c(BoundaryReason::NoSourceDeclaration),
        )
    }
}

/// The seed rank (DESIGN §3.4.1): among one scope's declarations of a name, an implementation
/// before an `@overload` stub, then one Pysa describes (Stage C, so Pyrefly's binding choice
/// under the context decides between `sys.version_info` or `TYPE_CHECKING` branches), then the
/// last in source order. `exports` seeds with it and `stub_for` targets with it, so the two cannot
/// drift (review O1). `d` is the declarations alias; `keyed` must be in scope.
pub(crate) fn seed_rank(partition: &str) -> String {
    format!(
        "row_number() OVER (PARTITION BY {partition} \
                            ORDER BY d.is_overload, k.node_id IS NULL, d.start_byte DESC, \
                                     d.node_id)"
    )
}

/// The Stage-C-described declarations, for `seed_rank`.
pub(crate) const KEYED: &str =
    "keyed AS (SELECT DISTINCT node_id FROM provider_node_map WHERE node_id IS NOT NULL)";

table!(
    /// Public access path → what it names (DESIGN §3.2, §9.1). `declaration_node_id` is the
    /// declaration Pass A seeds from: the origin file's declaration of that name by the seed rank.
    /// `target_node_id` is the typed target (ADR-0014): that declaration, else the dependency
    /// definition the path re-exports, else the module it names (of the release, or a dependency
    /// module). One row per `public_names` row, so a `.py`/`.pyi` pair gives an access path two
    /// rows, one per access file (`public_names.access_module_node_id`).
    Exports, ExportsRow = "exports",
    family = Exports,
    key = [snapshot_id, access_path, public_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        access_path: String,
        /// The `export` node: `H(export, release, access path)` (§3.4.1).
        export_node_id: Id,
        declaration_node_id: Option<Id>,
        target_node_id: Option<Id>,
        public_fact_id: Id,
        declaration_fact_id: Option<Id>,
        /// Why there is no target, only where a provider says so: `variable_origin` when Pyrefly
        /// calls the origin a variable, attribute, constant, parameter, type parameter or type
        /// alias; `missing_evidence` when Pyrefly traces no origin or records no kind. A function,
        /// class, method or module we fail to find keeps a null reason, which `typed:exports`
        /// rejects (review F1).
        reason: Option<BoundaryReason>,
    }
);

impl Derived for Exports {
    fn sql() -> String {
        let variable_like = [
            SymbolKind::Attribute,
            SymbolKind::Variable,
            SymbolKind::Constant,
            SymbolKind::Parameter,
            SymbolKind::TypeParameter,
            SymbolKind::TypeAlias,
        ]
        .iter()
        .map(|k| c(*k).to_string())
        .collect::<Vec<_>>()
        .join(", ");
        format!(
            "WITH {KEYED}, \
             ranked AS ( \
               SELECT d.node_id, d.fact_id, d.module_node_id, d.qualified_name, \
                      {rank} AS pick \
               FROM declarations d LEFT JOIN keyed k ON k.node_id = d.node_id), \
             ext AS ( \
               SELECT m.module_name, d.name, d.symbol_node_id, \
                      row_number() OVER (PARTITION BY m.module_name, d.name \
                                         ORDER BY d.kind, d.key) AS pick \
               FROM context_definitions d \
               JOIN context_modules m ON m.module_node_id = d.module_node_id \
               WHERE d.is_top_level), \
             release_modules AS ( \
               SELECT module_name, module_node_id, \
                      row_number() OVER (PARTITION BY module_name ORDER BY is_stub, path) AS pick \
               FROM source_files), \
             dependency_modules AS ( \
               SELECT DISTINCT module_name, module_node_id FROM context_modules \
               WHERE origin <> {not_found}), \
             module_bindings AS ( \
               SELECT b.module_node_id, b.name, b.node_id, \
                      row_number() OVER (PARTITION BY b.module_node_id, b.name \
                                         ORDER BY b.ordinal DESC) AS pick \
               FROM bindings b JOIN scopes s ON s.node_id = b.scope_id AND s.kind = {module_scope} \
               WHERE b.kind NOT IN ({unbinding})), \
             release AS ( \
               SELECT f.fact_id, r.release_id FROM facts f JOIN runs r ON r.run_id = f.run_id \
               WHERE f.table_name = 'public_names'), \
             resolved AS ( \
               SELECT p.access_path, x.release_id, p.fact_id AS public_fact_id, p.origin_path, \
                      p.origin_symbol_kind, p.origin_module_node_id, \
                      r.node_id AS declaration_node_id, \
                      r.fact_id AS declaration_fact_id, \
                      COALESCE(r.node_id, \
                               CASE WHEN p.origin_module_node_id IS NULL \
                                    THEN e.symbol_node_id END, \
                               CASE WHEN p.origin_symbol_kind IN ({variable_like}) \
                                    THEN mb.node_id END, \
                               CASE WHEN p.origin_symbol_kind IS NULL \
                                      OR p.origin_symbol_kind = {module} \
                                    THEN COALESCE(sm.module_node_id, dm.module_node_id) END) \
                        AS target_node_id \
               FROM public_names p \
               JOIN release x ON x.fact_id = p.fact_id \
               LEFT JOIN ranked r \
                 ON r.pick = 1 \
                AND r.module_node_id = p.origin_module_node_id \
                AND r.qualified_name = p.origin_path \
               LEFT JOIN ext e \
                 ON e.pick = 1 AND p.origin_module_node_id IS NULL \
                AND e.module_name = p.origin_module AND e.name = p.origin_name \
               LEFT JOIN module_bindings mb \
                 ON mb.pick = 1 AND mb.module_node_id = p.origin_module_node_id \
                AND mb.name = p.origin_name \
               LEFT JOIN release_modules sm ON sm.pick = 1 AND sm.module_name = p.origin_path \
               LEFT JOIN dependency_modules dm \
                 ON sm.module_name IS NULL AND dm.module_name = p.origin_path) \
             SELECT access_path, lctx_id('export', release_id, access_path) AS export_node_id, \
                    declaration_node_id, target_node_id, public_fact_id, declaration_fact_id, \
                    CAST(CASE WHEN target_node_id IS NOT NULL THEN NULL \
                              WHEN origin_path IS NULL OR origin_symbol_kind IS NULL \
                                THEN {missing} \
                              WHEN origin_symbol_kind IN ({variable_like}) \
                               AND origin_module_node_id IS NULL THEN {variable} END \
                         AS SMALLINT) AS reason \
             FROM resolved",
            rank = seed_rank("d.module_node_id, d.qualified_name"),
            module = c(SymbolKind::Module),
            module_scope = c(LexicalScopeKind::Module),
            unbinding = [
                BindingKind::Del,
                BindingKind::Global,
                BindingKind::Nonlocal,
                BindingKind::StarImport,
            ]
            .iter()
            .map(|k| c(*k).to_string())
            .collect::<Vec<_>>()
            .join(", "),
            missing = c(BoundaryReason::MissingEvidence),
            variable = c(BoundaryReason::VariableOrigin),
            not_found = c(ModuleOrigin::NotFound),
        )
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
    /// One resolution per higher-order argument (DESIGN §3.6): the callables Pysa says an
    /// argument may be invoked as (`potential` targets), and its unresolved remainder, which the
    /// call's own resolution leaves out.
    ArgumentResolutions, ArgumentResolutionsRow = "argument_resolutions",
    family = Calls,
    key = [snapshot_id, argument_node_id],
    checks = [("target_count_nonnegative", "target_count >= 0")],
    {
        snapshot_id: Id,
        argument_node_id: Id,
        status: ResolutionStatus,
        has_unresolved_remainder: bool,
        unresolved_reason: Option<PysaUnresolvedReason>,
        target_count: i64,
    }
);

impl Derived for ArgumentResolutions {
    fn sql() -> String {
        format!(
            "WITH hits AS ( \
               SELECT c.node_id AS call_node_id, p.higher_order_index, p.target_kind, \
                      p.unresolved_reason \
               FROM call_syntax c JOIN pysa_calls p \
                 ON p.module_node_id = c.module_node_id AND p.start_byte = c.start_byte \
                AND p.end_byte = c.end_byte AND p.site_kind = {regular} \
                AND p.callee_kind = {call} AND p.higher_order_index IS NOT NULL), \
             agg AS ( \
               SELECT a.node_id, \
                      SUM(CASE WHEN h.target_kind <> {unresolved} THEN 1 ELSE 0 END) AS targets, \
                      SUM(CASE WHEN h.target_kind = {unresolved} THEN 1 ELSE 0 END) AS remainders, \
                      MIN(h.unresolved_reason) AS unresolved_reason \
               FROM hits h JOIN arguments a \
                 ON a.call_node_id = h.call_node_id AND a.ordinal = h.higher_order_index \
               GROUP BY a.node_id) \
             SELECT node_id AS argument_node_id, \
                    CAST(CASE WHEN targets = 0 THEN {status_unresolved} \
                              WHEN remainders > 0 THEN {partial} \
                              ELSE {resolved} END AS SMALLINT) AS status, \
                    remainders > 0 AS has_unresolved_remainder, unresolved_reason, \
                    CAST(targets AS BIGINT) AS target_count \
             FROM agg",
            regular = c(PysaSiteKind::Regular),
            call = c(PysaCalleeKind::Call),
            unresolved = c(PysaTargetKind::Unresolved),
            status_unresolved = c(ResolutionStatus::Unresolved),
            partial = c(ResolutionStatus::Partial),
            resolved = c(ResolutionStatus::Resolved),
        )
    }
}

/// Pysa function references resolved to nodes: a release function (Stage C), a synthetic callable,
/// or a dependency definition. `{module}` and `{key}` name the reference's typed pair columns.
fn function_target(alias: &str, module: &str, key: &str) -> String {
    format!(
        "LEFT JOIN source_files {alias}_f ON {module} = '@' || {alias}_f.path \
         LEFT JOIN provider_node_map {alias}_m \
           ON {alias}_m.module_node_id = {alias}_f.module_node_id AND {alias}_m.function_key = {key} \
         LEFT JOIN synthetic_callables {alias}_s \
           ON {alias}_s.module_node_id = {alias}_f.module_node_id AND {alias}_s.function_key = {key} \
         LEFT JOIN external_functions {alias}_e \
           ON {alias}_f.module_node_id IS NULL AND {alias}_e.module_name = {module} \
          AND {alias}_e.key = {key}"
    )
}

/// The node a `function_target` join found, and the reason when it found none.
fn function_target_node(alias: &str) -> String {
    format!("COALESCE({alias}_m.node_id, {alias}_s.node_id, {alias}_e.symbol_node_id)")
}

/// Only Stage C's own reason explains a missing node (a key Pysa describes with no `def` at its
/// span). Any other miss is our failure to find what Pysa referenced, so the reason stays null and
/// the `typed:*` rule rejects the snapshot (review F1).
fn function_target_reason(alias: &str) -> String {
    format!(
        "CASE WHEN {node} IS NOT NULL THEN NULL \
              WHEN {alias}_m.function_key IS NOT NULL THEN {alias}_m.reason END",
        node = function_target_node(alias),
    )
}

/// Dependency definitions by module name and key, for the typed-target joins.
fn external(kind: DefinitionKind) -> String {
    format!(
        "SELECT m.module_name, d.key, d.symbol_node_id FROM context_definitions d \
         JOIN context_modules m ON m.module_node_id = d.module_node_id WHERE d.kind = {}",
        c(kind)
    )
}

table!(
    /// Each Pysa target of a call site (higher-order argument targets included, with their
    /// argument): the node it names, which is always typed (DESIGN §3.8; ADR-0014). A function of
    /// the release is its declaration (Stage C) or a synthetic callable; any other is a dependency
    /// definition. A null target always carries a reason: Stage C's, or `missing_evidence` when
    /// no definition has the key. Modality, origin and phase stay on the Pysa row and its `facts`
    /// row.
    CallTargets, CallTargetsRow = "call_targets",
    family = Calls,
    key = [snapshot_id, call_site_node_id, pysa_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        call_site_node_id: Id,
        pysa_fact_id: Id,
        /// For a higher-order target: the argument whose value may be invoked.
        argument_node_id: Option<Id>,
        target_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for CallTargets {
    fn sql() -> String {
        format!(
            "WITH external_functions AS ({external}) \
             SELECT c.node_id AS call_site_node_id, p.fact_id AS pysa_fact_id, \
                    a.node_id AS argument_node_id, \
                    {node} AS target_node_id, CAST({reason} AS SMALLINT) AS reason \
             FROM call_syntax c \
             JOIN pysa_calls p \
               ON p.module_node_id = c.module_node_id AND p.start_byte = c.start_byte \
              AND p.end_byte = c.end_byte AND p.site_kind = {regular} \
              AND p.callee_kind = {call} AND p.target_kind <> {unresolved} \
             LEFT JOIN arguments a \
               ON a.call_node_id = c.node_id AND a.ordinal = p.higher_order_index \
             {target}",
            external = external(DefinitionKind::Function),
            node = function_target_node("t"),
            reason = function_target_reason("t"),
            target = function_target("t", "p.target_module", "p.target_key"),
            regular = c(PysaSiteKind::Regular),
            call = c(PysaCalleeKind::Call),
            unresolved = c(PysaTargetKind::Unresolved),
        )
    }
}

table!(
    /// Each base and MRO entry Pysa reports, resolved to nodes (DESIGN §3.5.1, §3.8): the class
    /// (Stage C for classes) and the ancestor (a class of the release, or a dependency
    /// definition). An end that does not resolve carries a reason.
    AncestryTargets, AncestryTargetsRow = "ancestry_targets",
    family = Signatures,
    key = [snapshot_id, ancestry_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        ancestry_fact_id: Id,
        class_node_id: Option<Id>,
        ancestor_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for AncestryTargets {
    fn sql() -> String {
        format!(
            "WITH external_classes AS ({external}) \
             SELECT a.fact_id AS ancestry_fact_id, s.node_id AS class_node_id, \
                    COALESCE(t.node_id, e.symbol_node_id) AS ancestor_node_id, \
                    CAST(CASE WHEN s.node_id IS NULL THEN s.reason \
                              WHEN COALESCE(t.node_id, e.symbol_node_id) IS NOT NULL THEN NULL \
                              WHEN t.class_key IS NOT NULL THEN t.reason END AS SMALLINT) \
                      AS reason \
             FROM class_ancestry a \
             JOIN provider_class_map s \
               ON s.module_node_id = a.module_node_id AND s.class_key = a.class_key \
             LEFT JOIN source_files f ON a.ancestor_module = '@' || f.path \
             LEFT JOIN provider_class_map t \
               ON t.module_node_id = f.module_node_id AND t.class_key = a.ancestor_key \
             LEFT JOIN external_classes e \
               ON f.module_node_id IS NULL AND e.module_name = a.ancestor_module \
              AND e.key = a.ancestor_key \
             WHERE a.ancestor_module IS NOT NULL",
            external = external(DefinitionKind::Class),
        )
    }
}

table!(
    /// Each type term's class resolved to a node (C4, DESIGN §3.8): a class of the release (Stage
    /// C for classes) or a dependency definition. An unresolved class carries a reason: Stage C's,
    /// or `missing_evidence` when no definition has the key.
    TypeClassTargets, TypeClassTargetsRow = "type_class_targets",
    family = Types,
    key = [snapshot_id, term_node_id],
    checks = [],
    {
        snapshot_id: Id,
        term_node_id: Id,
        term_fact_id: Id,
        class_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for TypeClassTargets {
    fn sql() -> String {
        format!(
            "WITH external_classes AS ({external}) \
             SELECT t.node_id AS term_node_id, t.fact_id AS term_fact_id, \
                    COALESCE(m.node_id, e.symbol_node_id) AS class_node_id, \
                    CAST(CASE WHEN COALESCE(m.node_id, e.symbol_node_id) IS NOT NULL THEN NULL \
                              WHEN m.class_key IS NOT NULL THEN m.reason \
                              ELSE {missing} END AS SMALLINT) AS reason \
             FROM type_terms t \
             LEFT JOIN source_files f ON t.class_module = '@' || f.path \
             LEFT JOIN provider_class_map m \
               ON m.module_node_id = f.module_node_id AND m.class_key = t.class_key \
             LEFT JOIN external_classes e \
               ON f.module_node_id IS NULL AND e.module_name = t.class_module \
              AND e.key = t.class_key \
             WHERE t.class_module IS NOT NULL",
            external = external(DefinitionKind::Class),
            missing = c(BoundaryReason::MissingEvidence),
        )
    }
}

table!(
    /// Each method Pysa says overrides a base method, resolved to nodes (DESIGN §3.8): the method
    /// and the overridden method (of the release, synthetic, or a dependency definition). An end
    /// that does not resolve carries a reason.
    OverrideTargets, OverrideTargetsRow = "override_targets",
    family = Signatures,
    key = [snapshot_id, function_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        function_fact_id: Id,
        function_node_id: Option<Id>,
        overridden_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for OverrideTargets {
    fn sql() -> String {
        format!(
            "WITH external_functions AS ({external}) \
             SELECT p.fact_id AS function_fact_id, \
                    COALESCE(m.node_id, s.node_id) AS function_node_id, \
                    {node} AS overridden_node_id, \
                    CAST(CASE WHEN COALESCE(m.node_id, s.node_id) IS NULL THEN m.reason \
                              ELSE {reason} END AS SMALLINT) AS reason \
             FROM pysa_functions p \
             JOIN provider_node_map m \
               ON m.module_node_id = p.module_node_id AND m.function_key = p.function_key \
             LEFT JOIN synthetic_callables s \
               ON s.module_node_id = p.module_node_id AND s.function_key = p.function_key \
             {target} \
             WHERE p.overridden_module IS NOT NULL",
            external = external(DefinitionKind::Function),
            node = function_target_node("t"),
            reason = function_target_reason("t"),
            target = function_target("t", "p.overridden_module", "p.overridden_key"),
        )
    }
}

table!(
    /// Pysa's records at attribute, artificial and format-string sites (CPG slice C2), each
    /// resolved to the syntax node at its range (the deepest placed node with exactly that span;
    /// a chained comparison's pairwise site, which no node spans, to the innermost comparison that
    /// contains it) and to its typed target. Every expression is placed, so a span with no node is our
    /// own failure: a null reason `typed:site_targets` rejects (C2 review F2). An unresolved record
    /// reads `unresolved_target`; otherwise a reason only from Stage C.
    SiteTargets, SiteTargetsRow = "site_targets",
    family = Syntax,
    key = [snapshot_id, pysa_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        pysa_fact_id: Id,
        site_node_id: Option<Id>,
        target_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for SiteTargets {
    fn sql() -> String {
        format!(
            "WITH external_functions AS ({external}), \
             sites AS ( \
               SELECT fact_id, module_node_id, start_byte, end_byte, target_kind, target_module, \
                      target_key \
               FROM pysa_calls \
               WHERE NOT (site_kind = {regular} AND callee_kind = {call}) \
                 AND callee_kind <> {identifier}), \
             exact AS ( \
               SELECT s.fact_id, n.node_id, n.parent_node_id FROM sites s \
               JOIN syntax_nodes n \
                 ON n.module_node_id = s.module_node_id AND n.start_byte = s.start_byte \
                AND n.end_byte = s.end_byte), \
             chained AS ( \
               SELECT s.fact_id, n.node_id, n.parent_node_id FROM sites s \
               LEFT ANTI JOIN exact e ON e.fact_id = s.fact_id \
               JOIN syntax_nodes n \
                 ON n.module_node_id = s.module_node_id AND n.kind = {compare} \
                AND n.start_byte <= s.start_byte AND n.end_byte >= s.end_byte), \
             candidates AS (SELECT * FROM exact UNION ALL SELECT * FROM chained), \
             deepest AS ( \
               SELECT c.fact_id, c.node_id, \
                      row_number() OVER (PARTITION BY c.fact_id ORDER BY c.node_id \
                                         ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS pick \
               FROM candidates c LEFT ANTI JOIN candidates d \
                 ON d.fact_id = c.fact_id AND d.parent_node_id = c.node_id) \
             SELECT s.fact_id AS pysa_fact_id, x.node_id AS site_node_id, \
                    CASE WHEN s.target_kind <> {unresolved} THEN {node} END AS target_node_id, \
                    CAST(CASE WHEN x.node_id IS NULL THEN NULL \
                              WHEN s.target_kind = {unresolved} THEN {unresolved_target} \
                              ELSE {reason} END AS SMALLINT) AS reason \
             FROM sites s \
             LEFT JOIN deepest x ON x.fact_id = s.fact_id AND x.pick = 1 \
             {target}",
            external = external(DefinitionKind::Function),
            regular = c(PysaSiteKind::Regular),
            call = c(PysaCalleeKind::Call),
            identifier = c(PysaCalleeKind::Identifier),
            unresolved = c(PysaTargetKind::Unresolved),
            node = function_target_node("t"),
            reason = function_target_reason("t"),
            unresolved_target = c(BoundaryReason::UnresolvedTarget),
            target = function_target("t", "s.target_module", "s.target_key"),
            compare = c(SyntaxKind::ExprCompare),
        )
    }
}

table!(
    /// Pysa's records at identifier sites (a callable value that may be invoked, `if_called`; CPG
    /// slice C3), each resolved to the reference at its exact span and to its typed target. A span
    /// with no reference is our own failure (a null reason `typed:identifier_targets` rejects); an
    /// unresolved record reads `unresolved_target`; otherwise a reason only from Stage C.
    IdentifierTargets, IdentifierTargetsRow = "identifier_targets",
    family = Lexical,
    key = [snapshot_id, pysa_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        pysa_fact_id: Id,
        reference_node_id: Option<Id>,
        target_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for IdentifierTargets {
    fn sql() -> String {
        format!(
            "WITH external_functions AS ({external}), \
             sites AS ( \
               SELECT fact_id, module_node_id, start_byte, end_byte, target_kind, target_module, \
                      target_key \
               FROM pysa_calls \
               WHERE NOT (site_kind = {regular} AND callee_kind = {call}) \
                 AND callee_kind = {identifier}), \
             hits AS ( \
               SELECT s.fact_id, r.node_id, \
                      row_number() OVER (PARTITION BY s.fact_id ORDER BY r.node_id \
                                         ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS pick \
               FROM sites s JOIN references r \
                 ON r.module_node_id = s.module_node_id AND r.start_byte = s.start_byte \
                AND r.end_byte = s.end_byte) \
             SELECT s.fact_id AS pysa_fact_id, h.node_id AS reference_node_id, \
                    CASE WHEN s.target_kind <> {unresolved} THEN {node} END AS target_node_id, \
                    CAST(CASE WHEN h.node_id IS NULL THEN NULL \
                              WHEN s.target_kind = {unresolved} THEN {unresolved_target} \
                              ELSE {reason} END AS SMALLINT) AS reason \
             FROM sites s \
             LEFT JOIN hits h ON h.fact_id = s.fact_id AND h.pick = 1 \
             {target}",
            external = external(DefinitionKind::Function),
            regular = c(PysaSiteKind::Regular),
            call = c(PysaCalleeKind::Call),
            identifier = c(PysaCalleeKind::Identifier),
            unresolved = c(PysaTargetKind::Unresolved),
            node = function_target_node("t"),
            reason = function_target_reason("t"),
            unresolved_target = c(BoundaryReason::UnresolvedTarget),
            target = function_target("t", "s.target_module", "s.target_key"),
        )
    }
}

table!(
    /// Each import (C3): the module it names (`export_syntax.resolved_module`), as a module of the
    /// release (the `.py` before the `.pyi`) or a dependency module. A module that does not
    /// resolve in the context (an optional dependency) reads `unresolved_target`.
    ImportTargets, ImportTargetsRow = "import_targets",
    family = Lexical,
    key = [snapshot_id, import_fact_id],
    checks = [],
    {
        snapshot_id: Id,
        import_fact_id: Id,
        /// The importing module.
        module_node_id: Id,
        target_node_id: Option<Id>,
        reason: Option<BoundaryReason>,
    }
);

impl Derived for ImportTargets {
    fn sql() -> String {
        format!(
            "WITH release_modules AS ( \
               SELECT module_name, module_node_id, \
                      row_number() OVER (PARTITION BY module_name ORDER BY is_stub, path) AS pick \
               FROM source_files), \
             dependency_modules AS ( \
               SELECT DISTINCT module_name, module_node_id, origin FROM context_modules) \
             SELECT x.fact_id AS import_fact_id, x.module_node_id, \
                    COALESCE(r.module_node_id, \
                             CASE WHEN d.origin <> {not_found} THEN d.module_node_id END) \
                      AS target_node_id, \
                    CAST(CASE WHEN r.module_node_id IS NOT NULL THEN NULL \
                              WHEN x.resolved_module IS NULL OR d.origin = {not_found} \
                                THEN {unresolved} END AS SMALLINT) AS reason \
             FROM export_syntax x \
             LEFT JOIN release_modules r ON r.pick = 1 AND r.module_name = x.resolved_module \
             LEFT JOIN dependency_modules d \
               ON r.module_name IS NULL AND d.module_name = x.resolved_module \
             WHERE x.kind IN ({import}, {import_from})",
            unresolved = c(BoundaryReason::UnresolvedTarget),
            not_found = c(ModuleOrigin::NotFound),
            import = c(ExportSyntaxKind::Import),
            import_from = c(ExportSyntaxKind::ImportFrom),
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
            $crate::derived::ProviderClassMap,
            $crate::derived::SyntheticCallables,
            $crate::derived::Exports,
            $crate::derived::Signatures,
            $crate::derived::Parameters,
            $crate::derived::Resolutions,
            $crate::derived::ArgumentResolutions,
            $crate::derived::CallTargets,
            $crate::derived::AncestryTargets,
            $crate::derived::OverrideTargets,
            $crate::derived::SiteTargets,
            $crate::derived::IdentifierTargets,
            $crate::derived::ImportTargets,
            $crate::derived::TypeClassTargets,
            $crate::graph::Nodes,
            $crate::graph::Edges,
            $crate::graph::GraphGaps,
            $crate::graph::EdgeKinds
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
