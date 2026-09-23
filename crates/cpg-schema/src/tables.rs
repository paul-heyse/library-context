//! The tables slice 1 emits (DESIGN §3.2; ADR-0008 declares only what is emitted).
//!
//! Every row carries `snapshot_id`. Raw extraction rows carry a `fact_id` whose provenance
//! (run, origin, extraction mode, modality, fidelity, model) is one `facts` row (§B6).

use crate::codebook::{
    AncestryRelation, ArgumentKind, BoundaryReason, CoverageStatus, DeclarationKind,
    DefinitionKind, ExportSyntaxKind, ExtractionMode, FactFamily, Fidelity, ImplicitReceiver,
    InvocationPhase, Modality, ModuleOrigin, Origin, ParameterKind, PysaCalleeKind, PysaSiteKind,
    PysaTargetKind, PysaUnresolvedReason, ScopeKind, SignatureForm,
};
use crate::id::{Digest, Id};
use crate::table::table;

// ---------------------------------------------------------------- provenance

table!(
    /// One row per raw fact: its run and provenance (§B6).
    Facts, FactsRow = "facts",
    family = Provenance,
    key = [snapshot_id, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        run_id: Id,
        /// The table holding the fact's payload.
        table_name: String,
        origin: Origin,
        extraction_mode: ExtractionMode,
        modality: Modality,
        fidelity: Fidelity,
        /// `<producer_id hex>/<surface>`, validated against `producers`.
        model_id: String,
    }
);

table!(
    /// One producer applied to one context for a declared set of families (§4.0).
    Runs, RunsRow = "runs",
    family = Provenance,
    key = [snapshot_id, run_id],
    checks = [],
    {
        snapshot_id: Id,
        run_id: Id,
        release_id: Id,
        context_id: Id,
        producer_id: Id,
        /// Declared fact families, sorted.
        families: Vec<String>,
        config_digest: Digest,
    }
);

table!(
    /// The analysis context: everything that can change an analyzer's answer (§4.0).
    Contexts, ContextsRow = "contexts",
    family = Provenance,
    key = [snapshot_id, context_id],
    checks = [],
    {
        snapshot_id: Id,
        context_id: Id,
        python_version: String,
        python_platform: String,
        /// Ordered search path, relative to the release root.
        search_path: Vec<String>,
        /// Site-package path, relative to the analysis-venv root.
        site_package_path: Vec<String>,
        /// Digest of the canonical configured analyzer configuration.
        config_digest: Digest,
        /// Digest of the dependency environment: each installed distribution's name, version and
        /// `RECORD` digest, plus the content of every top-level entry no `RECORD` owns (DESIGN
        /// §4.0, ADR-0013).
        environment_digest: Digest,
        /// Digest of the library's `uv.lock`; null for a source tree.
        lock_digest: Option<Digest>,
    }
);

table!(
    Producers, ProducersRow = "producers",
    family = Provenance,
    key = [snapshot_id, producer_id],
    checks = [],
    {
        snapshot_id: Id,
        producer_id: Id,
        tool: String,
        /// Tool revision: for the extractor, the Pyrefly fork revision, patch digest and ruff line.
        revision: String,
        build_digest: Digest,
    }
);

table!(
    /// The release an extraction analyzed (DESIGN §4.0, ADR-0013): an acquired library, or a
    /// source tree compiled under a label.
    Releases, ReleasesRow = "releases",
    family = Provenance,
    key = [snapshot_id, release_id],
    checks = [],
    {
        snapshot_id: Id,
        release_id: Id,
        /// The library definition's name (`libraries/<name>`); null for a source tree.
        library: Option<String>,
        /// The pinned requirement the library's lock resolves.
        requirement: Option<String>,
        lock_digest: Option<Digest>,
        /// The release distributions as `name==version`, sorted; empty for a source tree.
        distributions: Vec<String>,
        /// The installer that built the environment (`uv 0.12.18`, from `pyvenv.cfg`).
        installer: Option<String>,
        /// The label a source tree was compiled under; null for an acquired library.
        label: Option<String>,
    }
);

table!(
    /// Every distribution installed in the analysis environment of a context (ADR-0013); the
    /// release's own are listed in `releases.distributions`.
    Distributions, DistributionsRow = "distributions",
    family = Provenance,
    key = [snapshot_id, context_id, name],
    checks = [],
    {
        snapshot_id: Id,
        context_id: Id,
        /// Normalized (PEP 503) name.
        name: String,
        version: String,
        /// sha256 (hex) of every artifact `uv.lock` records for this version, sorted.
        artifact_sha256: Vec<String>,
        /// Digest of the installed `RECORD`.
        record_digest: Digest,
    }
);

table!(
    /// One analyzed module of the release.
    SourceFiles, SourceFilesRow = "source_files",
    family = Provenance,
    key = [snapshot_id, module_name, path, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        release_id: Id,
        module_name: String,
        /// Release-relative path.
        path: String,
        is_package: bool,
        is_stub: bool,
        content_digest: Digest,
        byte_len: i64,
        /// The acquired bytes decoded as UTF-8 (`false` makes every family `unavailable`).
        utf8: bool,
        /// The release distribution whose `RECORD` lists the file (ADR-0013's module →
        /// distribution link); null for a source tree.
        distribution: Option<String>,
    }
);

table!(
    /// Every dependency module a fact of the run references (DESIGN §3.2, §3.8; ADR-0014): the
    /// `external_module` nodes. Its node id is the owning distribution and version (or Pyrefly's
    /// bundled typeshed and revision) and the module name, so it survives a moved environment.
    ContextModules, ContextModulesRow = "context_modules",
    family = Provenance,
    key = [snapshot_id, module_node_id, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        module_name: String,
        origin: ModuleOrigin,
        /// Site-relative (site-packages, search path) or bundle-relative path; null in memory.
        path: Option<String>,
        /// The installed distribution whose `RECORD` lists the file.
        distribution: Option<String>,
        version: Option<String>,
    }
);

table!(
    /// Pysa's definitions, in the dependency modules of `context_modules`, of every function and
    /// class a fact of the run references, and of every name a public export traces to (DESIGN
    /// §3.8): the `external_symbol` nodes. They come from Pyrefly's own collectors over those
    /// modules, independent of the columns that reference them.
    ContextDefinitions, ContextDefinitionsRow = "context_definitions",
    family = Provenance,
    key = [snapshot_id, symbol_node_id, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        symbol_node_id: Id,
        /// The `context_modules` node.
        module_node_id: Id,
        module_name: String,
        kind: DefinitionKind,
        /// Pysa's `FunctionId` or `ClassId`, unique within the module and kind.
        key: String,
        name: String,
        /// The enclosing classes and the name (`<locals>` marks a function scope).
        qualified_name: String,
        /// Defined at module level (what an export can trace to).
        is_top_level: bool,
    }
);

// ---------------------------------------------------------------- exports

table!(
    /// `def` and `class` statements from the Ruff walk (`ruff-ast`).
    Declarations, DeclarationsRow = "declarations",
    family = Exports,
    key = [snapshot_id, module_node_id, start_byte, end_byte, node_id],
    checks = [
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        ("name_span_order", "name_start_byte >= start_byte AND name_end_byte >= name_start_byte"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        module_node_id: Id,
        /// The enclosing declaration; null at module level.
        parent_node_id: Option<Id>,
        qualified_name: String,
        name: String,
        kind: DeclarationKind,
        start_byte: i64,
        end_byte: i64,
        name_start_byte: i64,
        name_end_byte: i64,
        docstring: Option<String>,
        docstring_start_byte: Option<i64>,
        docstring_end_byte: Option<i64>,
        is_overload: bool,
        /// Trailing names of the decorators, in source order.
        decorators: Vec<String>,
    }
);

table!(
    /// Import aliases and `__all__` statements (`ruff-ast`): syntax evidence, not "public".
    ExportSyntax, ExportSyntaxRow = "export_syntax",
    family = Exports,
    key = [snapshot_id, module_node_id, start_byte, end_byte, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        module_node_id: Id,
        kind: ExportSyntaxKind,
        /// `from <module> import …` or `import <module>`; null for `__all__`.
        imported_module: Option<String>,
        imported_name: Option<String>,
        alias: Option<String>,
        /// Relative-import level; 0 for absolute imports and `__all__`.
        level: i64,
        start_byte: i64,
        end_byte: i64,
        /// For `__all__`: whether it is a literal list or tuple of strings (F3 detector input).
        dunder_all_literal: Option<bool>,
    }
);

table!(
    /// Public access paths and their origins, by Pyrefly's definition of public (`pyrefly-public`).
    PublicNames, PublicNamesRow = "public_names",
    family = Exports,
    key = [snapshot_id, access_path, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        access_path: String,
        access_module: String,
        name: String,
        /// The defining module's path to the name; null when Pyrefly cannot trace it.
        origin_path: Option<String>,
        /// The file Pyrefly traced the origin to, when it is a file of the release (a `.py` and
        /// its `.pyi` share `origin_path`).
        origin_module_node_id: Option<Id>,
        via_dunder_all: bool,
        /// `origin_path` as a typed pair: the defining module's name and the name in it.
        origin_module: Option<String>,
        origin_name: Option<String>,
    }
);

// ---------------------------------------------------------------- signatures

table!(
    /// Parameters as written (`ruff-ast`).
    ParameterSyntax, ParameterSyntaxRow = "parameter_syntax",
    family = Signatures,
    key = [snapshot_id, function_node_id, ordinal, fact_id],
    checks = [
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        ("ordinal_nonnegative", "ordinal >= 0"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        function_node_id: Id,
        ordinal: i64,
        name: String,
        kind: ParameterKind,
        default_text: Option<String>,
        default_start_byte: Option<i64>,
        default_end_byte: Option<i64>,
        annotation_text: Option<String>,
        start_byte: i64,
        end_byte: i64,
    }
);

table!(
    /// Pysa's function definitions (`pyrefly-pysa`): the bridge from Pysa function keys to spans.
    PysaFunctions, PysaFunctionsRow = "pysa_functions",
    family = Signatures,
    key = [snapshot_id, module_node_id, function_key, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// The defining file (`source_files`): a `.py` and its `.pyi` share a module name.
        module_node_id: Id,
        module_name: String,
        /// Pysa's `FunctionId` (`F:3`, `MTL`, `CF:1:2`, …), unique within a file.
        function_key: String,
        name: String,
        name_start_byte: Option<i64>,
        name_end_byte: Option<i64>,
        is_overload: bool,
        is_staticmethod: bool,
        is_classmethod: bool,
        is_property_getter: bool,
        is_property_setter: bool,
        is_stub: bool,
        is_def_statement: bool,
        /// Class reference (§4.2.3) of the class defining this method.
        defining_class: Option<String>,
        /// `<module ref>::<function_key>` of the method this one overrides (§4.2.3).
        overridden_base: Option<String>,
        /// `defining_class` and `overridden_base` as typed (module ref, key) pairs, so joins never
        /// parse the strings.
        defining_class_module: Option<String>,
        defining_class_key: Option<String>,
        overridden_module: Option<String>,
        overridden_key: Option<String>,
        /// How many undecorated signatures Pysa gives (one per `@overload`, else one). A
        /// signature without parameters has no `parameter_semantics` row, so this is its trace.
        signature_count: i64,
    }
);

table!(
    /// Pysa's undecorated signatures (`pyrefly-pysa`): one row per parameter, or one row for an
    /// `...`/`ParamSpec` form.
    ParameterSemantics, ParameterSemanticsRow = "parameter_semantics",
    family = Signatures,
    key = [snapshot_id, module_node_id, function_key, signature_index, ordinal, fact_id],
    checks = [("signature_index_nonnegative", "signature_index >= 0")],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        module_name: String,
        function_key: String,
        signature_index: i64,
        form: SignatureForm,
        ordinal: Option<i64>,
        kind: Option<ParameterKind>,
        name: Option<String>,
        required: Option<bool>,
        /// Pysa's display string for the annotation.
        annotation: Option<String>,
        /// Class references (§4.2.3) Pysa extracted from the annotation, sorted.
        annotation_classes: Vec<String>,
        annotation_classes_exhaustive: Option<bool>,
        /// Pysa's scalar properties that hold (`bool`, `int`, `float`, `enum`).
        annotation_scalar: Vec<String>,
    }
);

table!(
    /// Pysa's class bases and reported MRO (ancestors, excluding the class and `object`, §3.5.1).
    ClassAncestry, ClassAncestryRow = "class_ancestry",
    family = Signatures,
    key = [snapshot_id, module_node_id, class_key, relation, ordinal, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        module_name: String,
        /// Pysa's `ClassId`, unique within a file.
        class_key: String,
        class_name: String,
        name_start_byte: Option<i64>,
        name_end_byte: Option<i64>,
        relation: AncestryRelation,
        /// Null only for a cyclic MRO's single marker row.
        ordinal: Option<i64>,
        /// Class reference (§4.2.3).
        ancestor: Option<String>,
        mro_cyclic: bool,
        /// `ancestor` as a typed (module ref, class key) pair.
        ancestor_module: Option<String>,
        ancestor_key: Option<String>,
    }
);

table!(
    /// Pysa's class definitions (`pyrefly-pysa`), one row per class: the key Stage C maps to a
    /// class declaration. `class_ancestry` has no row for a class without bases.
    PysaClasses, PysaClassesRow = "pysa_classes",
    family = Signatures,
    key = [snapshot_id, module_node_id, class_key, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        module_name: String,
        /// Pysa's `ClassId`, unique within a file.
        class_key: String,
        class_name: String,
        name_start_byte: i64,
        name_end_byte: i64,
        /// Synthesized (a functional `namedtuple`, …), not a `class` statement.
        is_synthesized: bool,
        is_dataclass: bool,
        is_named_tuple: bool,
        is_typed_dict: bool,
    }
);

// ---------------------------------------------------------------- calls

table!(
    /// Call expressions as written (`ruff-ast`).
    CallSyntax, CallSyntaxRow = "call_syntax",
    family = Calls,
    key = [snapshot_id, module_node_id, start_byte, end_byte, node_id],
    checks = [
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        ("callee_span_order", "callee_start_byte >= start_byte AND callee_end_byte >= callee_start_byte"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        module_node_id: Id,
        /// The enclosing declaration; null at module level.
        owner_node_id: Option<Id>,
        start_byte: i64,
        end_byte: i64,
        callee_start_byte: i64,
        callee_end_byte: i64,
        in_annotation: bool,
        positional_count: i64,
        keyword_count: i64,
    }
);

table!(
    Arguments, ArgumentsRow = "arguments",
    family = Calls,
    key = [snapshot_id, call_node_id, ordinal, fact_id],
    checks = [
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        ("ordinal_nonnegative", "ordinal >= 0"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// A role in the call, not the expression: `H(argument, call node, ordinal)` (§3.4.1), so
        /// `f(g(x))`'s argument is not the inner call site.
        node_id: Id,
        call_node_id: Id,
        ordinal: i64,
        kind: ArgumentKind,
        keyword: Option<String>,
        start_byte: i64,
        end_byte: i64,
    }
);

table!(
    /// Pysa's call graph (`pyrefly-pysa`), one row per target or unresolved remainder (§4.2.3).
    PysaCalls, PysaCallsRow = "pysa_calls",
    family = Calls,
    key = [snapshot_id, module_node_id, start_byte, end_byte, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// The row's run-independent payload digest (`pysa-call` over every other column): an
        /// edge discriminator stable across runs (§3.4.1).
        payload_id: Id,
        module_node_id: Id,
        module_name: String,
        caller_key: String,
        site_kind: PysaSiteKind,
        callee_kind: PysaCalleeKind,
        /// The artificial call's `OriginKind`, or the identifier's name.
        site_detail: Option<String>,
        start_byte: i64,
        end_byte: i64,
        phase: InvocationPhase,
        higher_order_index: Option<i64>,
        target_kind: PysaTargetKind,
        /// Module reference (§4.2.3) of the target's file.
        target_module: Option<String>,
        target_key: Option<String>,
        target_name: Option<String>,
        /// Class reference (§4.2.3).
        receiver_class: Option<String>,
        /// `receiver_class` as a typed (module ref, class key) pair.
        receiver_module: Option<String>,
        receiver_key: Option<String>,
        implicit_receiver: Option<ImplicitReceiver>,
        implicit_dunder_call: Option<bool>,
        is_class_method: Option<bool>,
        is_static_method: Option<bool>,
        unresolved_reason: Option<PysaUnresolvedReason>,
        /// For an attribute access: Pysa's `is_attribute`, some flow reads a plain attribute, so
        /// a property row is at most `candidate` (review F1).
        is_attribute: Option<bool>,
    }
);

// ---------------------------------------------------------------- coverage

table!(
    /// One row per declared family and module in scope; absence is never implicit (§3.7).
    Coverage, CoverageRow = "coverage",
    family = Coverage,
    key = [snapshot_id, run_id, scope_node_id, fact_family],
    checks = [],
    {
        snapshot_id: Id,
        run_id: Id,
        scope_kind: ScopeKind,
        scope_node_id: Id,
        fact_family: FactFamily,
        status: CoverageStatus,
        reason: Option<BoundaryReason>,
        detail: Option<String>,
    }
);

table!(
    /// Resolution issues and analysis stops, one row per stop (§3.7).
    Boundaries, BoundariesRow = "boundaries",
    family = Coverage,
    key = [snapshot_id, module_node_id, fact_family, start_byte, fact_id],
    checks = [(
        "span_order",
        "(start_byte IS NULL AND end_byte IS NULL) OR (start_byte >= 0 AND end_byte >= start_byte)"
    )],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        subject_node_id: Option<Id>,
        fact_family: FactFamily,
        reason: BoundaryReason,
        start_byte: Option<i64>,
        end_byte: Option<i64>,
        detail: Option<String>,
    }
);

// ---------------------------------------------------------------- publication

table!(
    /// The publication act: one row per table, appended in **one** commit after validation
    /// passes (DESIGN §6.1). A reader resolves the snapshot's table versions here.
    Snapshots, SnapshotsRow = "snapshots",
    family = Publication,
    key = [snapshot_id, table_name],
    checks = [
        ("version_nonnegative", "table_version >= 0"),
        ("row_count_nonnegative", "row_count >= 0"),
    ],
    {
        snapshot_id: Id,
        /// §3.4.1: compares reruns.
        content_digest: Digest,
        table_name: String,
        /// The Delta version the attempt's rows are visible at.
        table_version: i64,
        /// `cpg-schema`'s canonical schema digest of the table's declared contract.
        schema_digest: Digest,
        /// The compiler that derived, validated and published the snapshot (§3.4.1).
        compiler_digest: Digest,
        /// Rows the attempt wrote to the table.
        row_count: i64,
    }
);

/// Invoke `$mac!(Table, …)` with every raw table the extractor emits, in declaration order.
#[macro_export]
macro_rules! for_each_table {
    ($mac:ident) => {
        $mac!(
            $crate::tables::Facts,
            $crate::tables::Runs,
            $crate::tables::Contexts,
            $crate::tables::Producers,
            $crate::tables::Releases,
            $crate::tables::Distributions,
            $crate::tables::SourceFiles,
            $crate::tables::ContextModules,
            $crate::tables::ContextDefinitions,
            $crate::tables::Declarations,
            $crate::tables::ExportSyntax,
            $crate::tables::PublicNames,
            $crate::tables::ParameterSyntax,
            $crate::tables::PysaFunctions,
            $crate::tables::ParameterSemantics,
            $crate::tables::ClassAncestry,
            $crate::tables::PysaClasses,
            $crate::tables::CallSyntax,
            $crate::tables::Arguments,
            $crate::tables::PysaCalls,
            $crate::tables::Coverage,
            $crate::tables::Boundaries
        )
    };
}

/// Every stored table's contract text: raw, derived, then `snapshots` (snapshot-tested).
pub fn contracts() -> Vec<(&'static str, String)> {
    use crate::table::{Table, contract};
    macro_rules! all {
        ($($t:ty),+) => { vec![$((<$t as Table>::NAME, contract::<$t>())),+] };
    }
    let mut out = crate::for_each_table!(all);
    out.extend(crate::for_each_derived_table!(all));
    out.push((Snapshots::NAME, contract::<Snapshots>()));
    out
}
