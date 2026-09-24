//! The tables slice 1 emits (DESIGN §3.2; ADR-0008 declares only what is emitted).
//!
//! Every row carries `snapshot_id`. Raw extraction rows carry a `fact_id` whose provenance
//! (run, origin, extraction mode, modality, fidelity, model) is one `facts` row (§B6).

use crate::codebook::{
    AncestryRelation, ArgumentKind, AttributeValueKind, BindingKind, BoundaryReason, ComponentForm,
    ConditionAtom, CoverageStatus, DeclarationKind, DefinitionKind, ExportSyntaxKind,
    ExtractionMode, FactFamily, Fidelity, FlowSink, ImplicitReceiver, InvocationPhase,
    LexicalScopeKind, MentionClass, MentionSource, Modality, ModuleOrigin, Origin, ParameterKind,
    PysaCalleeKind, PysaSiteKind, PysaTargetKind, PysaUnresolvedReason, RecordKind, ScopeKind,
    SignatureForm, SourceRole, StaticBranch, SymbolKind, SyntaxField, SyntaxKind, TypeArgRole,
    TypeRole, TypeTermKind,
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
        /// What the module is to its library: the release, or an example, test or doc block of
        /// its corpus (ADR-0015).
        role: SourceRole,
        /// The module's text, the bytes every span of the snapshot indexes, so a snippet is read
        /// from Delta alone (ADR-0015); null exactly when the bytes are not UTF-8.
        text: Option<String>,
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
        /// The imported module as an absolute name, relative levels resolved against this module
        /// (C3); null for `__all__` or a relative import that climbs past the top package.
        resolved_module: Option<String>,
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
        /// The release file the access path is read from (a `.py` and its `.pyi` are two).
        access_module_node_id: Id,
        /// Pyrefly's kind for the origin's export; null when Pyrefly records none.
        origin_symbol_kind: Option<SymbolKind>,
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
    /// Parameter documentation from a function's docstring (`pyrefly-docstring`: Pyrefly's
    /// `parse_parameter_documentation`, Sphinx `:param x:` and Google `Args:`; increment 2 slice
    /// 2.1). One row per documented parameter of the signature, its text normalized as Pyrefly
    /// returns it, its span the description's verbatim bytes in the source (the evidence a
    /// documented control cites). A description whose bytes cannot be located is not emitted.
    ParameterDocs, ParameterDocsRow = "parameter_docs",
    family = Signatures,
    key = [snapshot_id, function_node_id, name],
    checks = [("span_order", "start_byte >= 0 AND end_byte > start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        function_node_id: Id,
        module_node_id: Id,
        name: String,
        text: String,
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

// ---------------------------------------------------------------- syntax

table!(
    /// Placed syntax nodes (`ruff-ast`; CPG slice C2, DESIGN §3.2): every statement, the clause
    /// nodes, the expressions Pysa reports sites at, and the full expression subtree under each
    /// `test`, `exc`, `cause`, `guard` and `msg`. A `def`, a `class` and a call are placed under
    /// their existing declaration and call-site ids (so one syntax node has one id), which gives
    /// every placed node its parent, field and ordinal. Nothing inside an annotation is placed.
    SyntaxNodes, SyntaxNodesRow = "syntax_nodes",
    family = Syntax,
    key = [snapshot_id, module_node_id, start_byte, end_byte, node_id],
    checks = [
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        ("ordinal_nonnegative", "ordinal >= 0"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// The structural syntax id; for a `def`/`class` the declaration id, for a call the
        /// call-site id.
        node_id: Id,
        module_node_id: Id,
        /// The innermost enclosing declaration; null at module level.
        owner_node_id: Option<Id>,
        /// The nearest placed ancestor: a syntax node, a declaration, a call site or the module.
        parent_node_id: Id,
        kind: SyntaxKind,
        field: SyntaxField,
        /// Position among the parent's placed children in the same field, in source order (a
        /// statement's index in its block).
        ordinal: i64,
        start_byte: i64,
        end_byte: i64,
        /// A name, an attribute, an operator, a literal as written, a handler's bound name.
        detail: Option<String>,
    }
);

// ---------------------------------------------------------------- lexical

table!(
    /// Lexical scopes (our recognizer, `lctx-lexical`; CPG slice C3, DESIGN §3.2): the module, each
    /// class, function, lambda and comprehension. Node id `H(scope, owner)`.
    Scopes, ScopesRow = "scopes",
    family = Lexical,
    key = [snapshot_id, node_id, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        module_node_id: Id,
        kind: LexicalScopeKind,
        /// The node that opens the scope: the module, a declaration, a lambda or a comprehension.
        owner_node_id: Id,
        /// The enclosing scope; null for the module.
        parent_scope_id: Option<Id>,
        start_byte: i64,
        end_byte: i64,
    }
);

table!(
    /// Binding events, with shadowed history (C3): every binding of a name in a scope, in source
    /// order, including those in branches Pyrefly decides statically (marked). Node id
    /// `H(binding, site, name)`.
    Bindings, BindingsRow = "bindings",
    family = Lexical,
    key = [snapshot_id, scope_id, ordinal, node_id, fact_id],
    checks = [
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        ("ordinal_nonnegative", "ordinal >= 0"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        scope_id: Id,
        module_node_id: Id,
        name: String,
        kind: BindingKind,
        /// Position among the scope's binding events, in source order.
        ordinal: i64,
        /// The binding site: a declaration, a parameter, or the syntax id of the name, alias,
        /// handler or statement that binds.
        site_node_id: Id,
        /// The span of the bound name (or of the site when the name has no node of its own).
        start_byte: i64,
        end_byte: i64,
        /// The assigned value's span (assignments, augmented assignments, walrus, `for`
        /// iterables, `with` context managers), joinable to `syntax_nodes` and `call_syntax`.
        value_start_byte: Option<i64>,
        value_end_byte: Option<i64>,
        /// The innermost branch Pyrefly decides statically that the binding sits in (the kind of
        /// the deciding test), and whether Pyrefly analyzes that branch (`true`) or prunes it
        /// (`false`), clause by clause as `SysInfo::pruned_if_branches` decides (H1 C1).
        static_branch: Option<StaticBranch>,
        static_polarity: Option<bool>,
    }
);

table!(
    /// Name loads (C3): each a role of its name's syntax node, `H(reference, name node)`, with its
    /// scope and its place under the nearest placed syntax node.
    References, ReferencesRow = "references",
    family = Lexical,
    key = [snapshot_id, module_node_id, start_byte, node_id, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        /// The name's structural syntax id.
        name_node_id: Id,
        scope_id: Id,
        module_node_id: Id,
        name: String,
        /// The nearest placed ancestor (a syntax node, declaration, call site or the module), and
        /// the field the name sits in there (its role: callee, argument, attribute value, …).
        parent_node_id: Id,
        field: SyntaxField,
        start_byte: i64,
        end_byte: i64,
    }
);

table!(
    /// The recognizer's name resolution (C3), full Python scoping: the scope's own bindings of the
    /// name, else the nearest enclosing function scope (class scopes skipped), else the module,
    /// else a builtin, honoring `global`/`nonlocal`; comprehension, lambda and class scopes. One
    /// row per candidate binding (flow-insensitive: `candidate` when more than one); a builtin
    /// names its builtin; otherwise a reason. Bindings in statically decided branches are kept.
    ReferenceResolutions, ReferenceResolutionsRow = "reference_resolutions",
    family = Lexical,
    key = [snapshot_id, reference_id, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        reference_id: Id,
        binding_id: Option<Id>,
        /// The binding is in an enclosing function scope (a closure).
        captured: bool,
        /// The builtin the name resolves to, when no scope binds it.
        builtin_name: Option<String>,
        /// Why there is no binding: `unresolved_target` (nothing binds the name), or
        /// `variable_origin` (a builtin that is not a function or class).
        reason: Option<BoundaryReason>,
    }
);

// ---------------------------------------------------------------- flow

table!(
    /// A place load ty records (ADR-0022 §The flow provider; `cpg-flow`): a name, an attribute
    /// chain or a literal subscript read, an augmented assignment's target, a `del` target. Id
    /// `H(flow_use, module, start, end)`. `annotation`: inside an annotation, which `references`
    /// does not model as a read (the declared parity residue).
    FlowUses, FlowUsesRow = "flow_uses",
    family = Flow,
    key = [snapshot_id, module_node_id, start_byte, end_byte, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        use_id: Id,
        module_node_id: Id,
        /// The place as ty spells it (`timeout`, `self._x`, `settings.host`, `d["k"]`).
        place: String,
        scope_kind: LexicalScopeKind,
        /// The span naming the scope: a function's or class's name, a lambda's or
        /// comprehension's expression; null for the module.
        scope_start_byte: Option<i64>,
        scope_end_byte: Option<i64>,
        start_byte: i64,
        end_byte: i64,
        annotation: bool,
    }
);

table!(
    /// A binding ty records, normalized (ADR-0022): loop headers become the bindings they stand
    /// for; an import alias's target is the bound name. Id `H(flow_definition, module, kind,
    /// start, end, place)`.
    FlowDefinitions, FlowDefinitionsRow = "flow_definitions",
    family = Flow,
    key = [snapshot_id, module_node_id, start_byte, end_byte, definition_id, fact_id],
    checks = [
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        (
            "value_span_order",
            "(value_start_byte IS NULL AND value_end_byte IS NULL) \
             OR (value_start_byte >= 0 AND value_end_byte >= value_start_byte)"
        ),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        definition_id: Id,
        module_node_id: Id,
        place: String,
        kind: BindingKind,
        scope_kind: LexicalScopeKind,
        scope_start_byte: Option<i64>,
        scope_end_byte: Option<i64>,
        /// The bound target.
        start_byte: i64,
        end_byte: i64,
        /// The value it takes (see `flow_values` for what the value reads).
        value_start_byte: Option<i64>,
        value_end_byte: Option<i64>,
    }
);

table!(
    /// A definition reaching a use, under the condition of its reachability at the use (a path
    /// condition from the scope's entry). A null definition: the place may be unbound there.
    FlowReaching, FlowReachingRow = "flow_reaching",
    family = Flow,
    key = [snapshot_id, use_id, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        use_id: Id,
        definition_id: Option<Id>,
        condition_id: Id,
        /// The admitted may-path crossed a provider ambiguity or declared assumption.
        approximated: bool,
        /// Reached around a loop's back edge.
        loop_carried: bool,
    }
);

table!(
    /// What a value reads: per sink (a definition's value, a call argument, a `return`, a
    /// `yield`, a `raise`), each use inside it, **identity** (the value passes unchanged) or
    /// derived, under the condition inside the expression that selects it. A derived use inside a
    /// call (its callee, receiver or an argument) is `through_call`: it reaches the value only if
    /// the callee's result carries it (ADR-0022 §Verdicts).
    FlowValues, FlowValuesRow = "flow_values",
    family = Flow,
    key = [snapshot_id, module_node_id, sink_start_byte, sink_end_byte, use_id, fact_id],
    checks = [
        ("sink_span_order", "sink_start_byte >= 0 AND sink_end_byte >= sink_start_byte"),
        ("identity_not_through_call", "NOT (identity AND through_call)"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        sink: FlowSink,
        sink_start_byte: i64,
        sink_end_byte: i64,
        use_id: Id,
        identity: bool,
        through_call: bool,
        condition_id: Id,
        approximated: bool,
    }
);

table!(
    /// A statement's reachability condition, relative to its scope's entry: a statement inside a
    /// function is reached under this **and** its `def` statement's region.
    FlowRegions, FlowRegionsRow = "flow_regions",
    family = Flow,
    key = [snapshot_id, module_node_id, start_byte, end_byte, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        scope_kind: LexicalScopeKind,
        scope_start_byte: Option<i64>,
        scope_end_byte: Option<i64>,
        start_byte: i64,
        end_byte: i64,
        condition_id: Id,
        approximated: bool,
    }
);

table!(
    /// A test the flow provider records as a predicate (an `if`/`elif`/`while`/`assert` test, a
    /// conditional expression's or boolean operand's test, a `match` subject with its pattern):
    /// its span and its condition. The flow uses inside the span are what the test reads, so which
    /// parameters a test reads comes from the flow IR (the Stage 2 end review's R8).
    FlowTests, FlowTestsRow = "flow_tests",
    family = Flow,
    key = [snapshot_id, module_node_id, start_byte, end_byte, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        scope_kind: LexicalScopeKind,
        scope_start_byte: Option<i64>,
        scope_end_byte: Option<i64>,
        start_byte: i64,
        end_byte: i64,
        condition_id: Id,
    }
);

table!(
    /// One evaluation atom of one provider predicate, emitted before span-only test deduplication.
    /// Its proof identity is (snapshot, module, predicate_key, atom_id).
    FlowTestLeaves, FlowTestLeavesRow = "flow_test_leaves",
    family = Flow,
    key = [snapshot_id, module_node_id, predicate_key, atom_id, fact_id],
    checks = [
        ("test_span_order", "test_start_byte >= 0 AND test_end_byte >= test_start_byte"),
        ("leaf_span_order", "leaf_start_byte >= 0 AND leaf_end_byte >= leaf_start_byte"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        scope_kind: LexicalScopeKind,
        scope_start_byte: Option<i64>,
        scope_end_byte: Option<i64>,
        predicate_key: String,
        test_start_byte: i64,
        test_end_byte: i64,
        condition_id: Id,
        atom_id: Id,
        atom: String,
        leaf_start_byte: i64,
        leaf_end_byte: i64,
    }
);

table!(
    /// An attribute load by name on any receiver, a place or not (`get_server()._worker`,
    /// `x[k].f`), or a `getattr`/`hasattr` with a literal name; outside annotations. The field and
    /// global premises count these (ADR-0022 §Verdicts; the Stage 2 end review's R7).
    FlowAttributeLoads, FlowAttributeLoadsRow = "flow_attribute_loads",
    family = Flow,
    key = [snapshot_id, module_node_id, start_byte, end_byte, name, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
        name: String,
    }
);

table!(
    /// A condition's diagram root and bounded display (ADR-0024). The root/node closure is the
    /// authority; `encoding` is presentation only. An unstated condition has a named boundary.
    Conditions, ConditionsRow = "conditions",
    family = Flow,
    key = [snapshot_id, condition_id, fact_id],
    checks = [
        ("root_iff_stated", "(stated AND root_id IS NOT NULL AND boundary_reason IS NULL) OR (NOT stated AND root_id IS NULL AND boundary_reason IS NOT NULL)"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        condition_id: Id,
        root_id: Option<Id>,
        encoding: String,
        stated: bool,
        display_truncated: bool,
        boundary_reason: Option<String>,
    }
);

table!(
    /// Lossless content-addressed decision nodes. Terminal ids are fixed and implicit.
    ConditionNodes, ConditionNodesRow = "condition_nodes",
    family = Flow,
    key = [snapshot_id, node_id, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        atom: String,
        low_id: Id,
        high_id: Id,
    }
);

table!(
    /// A condition's literals: conjunction by conjunction, in the normal form's order.
    ConditionLiterals, ConditionLiteralsRow = "condition_literals",
    family = Flow,
    key = [snapshot_id, condition_id, conjunction, ordinal, fact_id],
    checks = [("ordinal_nonnegative", "conjunction >= 0 AND ordinal >= 0")],
    {
        snapshot_id: Id,
        fact_id: Id,
        condition_id: Id,
        conjunction: i64,
        ordinal: i64,
        atom: ConditionAtom,
        positive: bool,
        /// The place tested; null for an opaque atom.
        place: Option<String>,
        /// The value, value set, class or opaque text, as encoded.
        argument: Option<String>,
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

// ---------------------------------------------------------------- types

table!(
    /// Distinct type terms from Pyrefly's native types (`pyrefly-types`; CPG slice C4, DESIGN
    /// §3.2). The id is `H(type, kind, detail, class, children…)`: a Merkle id over Pyrefly's own
    /// structure and identities (`type_term_args`), so one structure is one term; the display is a
    /// label, not identity. A class is a (module ref, class key) pair, never an expansion, and a
    /// recursive alias is a reference to its name, so every term is finite. A type variable's id is
    /// Pyrefly's own identity (`QuantifiedIdentity`), so two unrelated `T`s are two terms and one
    /// variable is one term wherever it is observed (C4 review F3); its bound, constraints and
    /// default are its children.
    TypeTerms, TypeTermsRow = "type_terms",
    family = Types,
    key = [snapshot_id, node_id, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        node_id: Id,
        kind: TypeTermKind,
        /// Pyrefly's display of the term.
        display: String,
        /// What the kind leaves open: a literal as displayed, a module or alias name, a `def`'s
        /// name, a type variable's name, the `Any` or `Never` style, the variant for `other`.
        detail: Option<String>,
        /// The term's class as a typed (module ref, class key) pair (§4.2.3): an instance, a class
        /// object, a `TypedDict`, `Self`.
        class_module: Option<String>,
        class_key: Option<String>,
        /// A type variable's identity as Pyrefly keys it: `<module>:<start>-<end>#<index>/<origin>`
        /// (its scope anchor; deterministic, from source positions).
        variable: Option<String>,
        /// A source-anchored type variable's scope anchor (Pyrefly's): the module and byte span
        /// Stage D finds the binder at (`type_binders`). Null for a synthesized variable.
        anchor_module: Option<String>,
        anchor_start: Option<i64>,
        anchor_end: Option<i64>,
    }
);

table!(
    /// The structure of a type term (C4): each child term at its role and ordinal. A callable
    /// parameter carries its name, kind and whether it is required.
    TypeTermArgs, TypeTermArgsRow = "type_term_args",
    family = Types,
    key = [snapshot_id, parent_node_id, role, ordinal, fact_id],
    checks = [("ordinal_nonnegative", "ordinal >= 0")],
    {
        snapshot_id: Id,
        fact_id: Id,
        parent_node_id: Id,
        role: TypeArgRole,
        ordinal: i64,
        child_node_id: Id,
        name: Option<String>,
        parameter_kind: Option<ParameterKind>,
        required: Option<bool>,
    }
);

table!(
    /// What Pyrefly says a program element's type is (C4; DESIGN §3.5.1): a parameter, a
    /// function's return, a call's result, an argument's value, a raised exception. `declared`
    /// types are Pyrefly's reading of the annotation as written, never a computed type relabelled.
    TypeObservations, TypeObservationsRow = "type_observations",
    family = Types,
    key = [snapshot_id, subject_node_id, role, fact_id],
    checks = [],
    {
        snapshot_id: Id,
        fact_id: Id,
        module_node_id: Id,
        subject_node_id: Id,
        role: TypeRole,
        /// The subject has an annotation and this is its type; false: Pyrefly computed it.
        declared: bool,
        term_node_id: Id,
    }
);

table!(
    /// The fields a class declares under a record model (C4): dataclass, attrs, pydantic,
    /// `TypedDict`, `NamedTuple`. One row per field the class itself declares (an inherited field
    /// is its base's row), with the flags as the field states them. The constructor they imply is
    /// Pyrefly's synthesized `__init__`, a `synthetic_callable` with its `parameter_semantics`.
    RecordFields, RecordFieldsRow = "record_fields",
    family = Types,
    key = [snapshot_id, class_node_id, ordinal, fact_id],
    checks = [
        ("ordinal_nonnegative", "ordinal >= 0"),
        ("span_order", "start_byte IS NULL OR (start_byte >= 0 AND end_byte >= start_byte)"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// `H(field, class_node_id, name)`.
        node_id: Id,
        class_node_id: Id,
        module_node_id: Id,
        record_kind: RecordKind,
        name: String,
        /// Position in Pyrefly's field order for the class (inherited fields first).
        ordinal: i64,
        term_node_id: Id,
        /// The field has an explicit annotation.
        declared: bool,
        /// The field's declaration.
        start_byte: Option<i64>,
        end_byte: Option<i64>,
        /// Dataclass-like and `NamedTuple` fields: a default (or factory) is given.
        has_default: Option<bool>,
        /// Dataclass-like fields: the field is a parameter of the synthesized `__init__`.
        init: Option<bool>,
        /// Dataclass-like fields: the `__init__` parameter's name, when it is an alias.
        alias: Option<String>,
        /// Dataclass-like fields: `kw_only` as the field sets it; null when the field leaves it to
        /// the class.
        kw_only: Option<bool>,
        /// `TypedDict` fields.
        required: Option<bool>,
        read_only: Option<bool>,
    }
);

// ---------------------------------------------------------------- docs

table!(
    /// The documents of a corpus release (CPG slice C5, DESIGN §3.2 `docs`): each selected file of
    /// the upstream tree at its pinned commit. A document that does not parse is still a document;
    /// its coverage row says why it has no passages.
    Documents, DocumentsRow = "documents",
    family = Docs,
    key = [snapshot_id, node_id, fact_id],
    checks = [("byte_len_nonnegative", "byte_len >= 0")],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// `H(document, release, path)`.
        node_id: Id,
        release_id: Id,
        /// Release-relative (the tree's root).
        path: String,
        content_digest: Digest,
        byte_len: i64,
        /// The frontmatter's `title`, as written.
        title: Option<String>,
        /// markdown-rs parsed it (MDX constructs, frontmatter).
        parsed: bool,
    }
);

table!(
    /// A document's sections (`markdown-rs`; C5): each top-level heading opens one, which runs to
    /// the next top-level heading, so a document's passages partition it; the text before the
    /// first heading is passage 0 at level 0. The heading path is the enclosing headings' text.
    Passages, PassagesRow = "passages",
    family = Docs,
    key = [snapshot_id, document_node_id, ordinal, fact_id],
    checks = [
        ("ordinal_nonnegative", "ordinal >= 0"),
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// `H(passage, document, ordinal)`.
        node_id: Id,
        document_node_id: Id,
        ordinal: i64,
        /// The heading's depth (1–6); 0 before the first heading.
        level: i64,
        heading: Option<String>,
        heading_path: Vec<String>,
        start_byte: i64,
        end_byte: i64,
        /// The passage's source text, as written.
        text: String,
    }
);

table!(
    /// Fenced code blocks (`markdown-rs`; C5), at any depth (inside MDX components too), in their
    /// document's order.
    CodeBlocks, CodeBlocksRow = "code_blocks",
    family = Docs,
    key = [snapshot_id, document_node_id, ordinal, fact_id],
    checks = [
        ("ordinal_nonnegative", "ordinal >= 0"),
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        /// `H(code_block, document, ordinal)`.
        node_id: Id,
        document_node_id: Id,
        passage_node_id: Id,
        ordinal: i64,
        language: Option<String>,
        meta: Option<String>,
        /// The fence's span in the document.
        start_byte: i64,
        end_byte: i64,
        code: String,
        content_digest: Digest,
        /// A Python block's materialized module in the usage run (C5b), release-relative; null for
        /// any other language.
        module_path: Option<String>,
    }
);

table!(
    /// Links in passages (`markdown-rs`; C5): the URL and its text, as written.
    DocLinks, DocLinksRow = "doc_links",
    family = Docs,
    key = [snapshot_id, passage_node_id, ordinal, fact_id],
    checks = [
        ("ordinal_nonnegative", "ordinal >= 0"),
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        passage_node_id: Id,
        ordinal: i64,
        url: String,
        title: Option<String>,
        text: String,
        start_byte: i64,
        end_byte: i64,
    }
);

table!(
    /// Mentions of the library's API in passages (`lctx-docs`, our recognizer; C5). `exact`: the
    /// text is a public access path or a public class's member; `lexical`: a bare public name, one
    /// row per access path it could be (`candidate`). The two classes are never merged, and a
    /// mention by embedding similarity is not one (§9.7).
    Mentions, MentionsRow = "mentions",
    family = Docs,
    key = [snapshot_id, passage_node_id, start_byte, fact_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        fact_id: Id,
        passage_node_id: Id,
        class: MentionClass,
        source: MentionSource,
        /// The text as written.
        form: String,
        /// The public access path it names (an export), or
        access_path: Option<String>,
        /// the release declaration it names (a public class's member).
        qualified_name: Option<String>,
        start_byte: i64,
        end_byte: i64,
    }
);

table!(
    /// The MDX JSX components of a document (`markdown-rs`'s mdast; the holistic assessment's A3):
    /// each element in pre-order, with its parent, name and form, its span, the span of its
    /// children (null when self-closing) and of its first direct paragraph. A component lies in one
    /// passage (passages are cut only at top-level headings). A span fact, not a graph node; a
    /// fragment has no name.
    DocComponents, DocComponentsRow = "doc_components",
    family = Docs,
    key = [snapshot_id, document_node_id, ordinal, fact_id],
    checks = [
        ("ordinal_nonnegative", "ordinal >= 0 AND depth >= 0"),
        ("span_order", "start_byte >= 0 AND end_byte >= start_byte"),
        ("parent_before", "parent_ordinal IS NULL OR (parent_ordinal >= 0 AND parent_ordinal < ordinal)"),
        (
            "inner_within",
            "inner_start IS NULL OR (inner_start >= start_byte AND inner_start <= inner_end AND inner_end <= end_byte)"
        ),
        (
            "lead_within",
            "lead_start IS NULL OR (lead_start >= start_byte AND lead_start <= lead_end AND lead_end <= end_byte)"
        ),
    ],
    {
        snapshot_id: Id,
        fact_id: Id,
        document_node_id: Id,
        passage_node_id: Id,
        /// Pre-order over the document's components.
        ordinal: i64,
        /// The enclosing component's ordinal; null at the top level.
        parent_ordinal: Option<i64>,
        /// Enclosing components.
        depth: i64,
        name: Option<String>,
        form: ComponentForm,
        start_byte: i64,
        end_byte: i64,
        /// From the first child's start to the last child's end.
        inner_start: Option<i64>,
        inner_end: Option<i64>,
        /// The first direct paragraph child: a component's lead text.
        lead_start: Option<i64>,
        lead_end: Option<i64>,
    }
);

table!(
    /// A component's attributes in order (A3): a literal value as written; an expression's or a
    /// spread's source text, never read as a literal.
    DocComponentAttributes, DocComponentAttributesRow = "doc_component_attributes",
    family = Docs,
    key = [snapshot_id, document_node_id, component_ordinal, ordinal, fact_id],
    checks = [("ordinal_nonnegative", "component_ordinal >= 0 AND ordinal >= 0")],
    {
        snapshot_id: Id,
        fact_id: Id,
        document_node_id: Id,
        component_ordinal: i64,
        ordinal: i64,
        /// Null for a spread.
        name: Option<String>,
        /// Null for a bare name.
        value: Option<String>,
        value_kind: AttributeValueKind,
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
            $crate::tables::ParameterDocs,
            $crate::tables::PysaFunctions,
            $crate::tables::ParameterSemantics,
            $crate::tables::ClassAncestry,
            $crate::tables::PysaClasses,
            $crate::tables::CallSyntax,
            $crate::tables::Arguments,
            $crate::tables::SyntaxNodes,
            $crate::tables::Scopes,
            $crate::tables::Bindings,
            $crate::tables::References,
            $crate::tables::ReferenceResolutions,
            $crate::tables::TypeTerms,
            $crate::tables::TypeTermArgs,
            $crate::tables::TypeObservations,
            $crate::tables::RecordFields,
            $crate::tables::Documents,
            $crate::tables::Passages,
            $crate::tables::CodeBlocks,
            $crate::tables::DocLinks,
            $crate::tables::Mentions,
            $crate::tables::DocComponents,
            $crate::tables::DocComponentAttributes,
            $crate::tables::PysaCalls,
            $crate::tables::FlowUses,
            $crate::tables::FlowDefinitions,
            $crate::tables::FlowReaching,
            $crate::tables::FlowValues,
            $crate::tables::FlowRegions,
            $crate::tables::FlowTests,
            $crate::tables::FlowTestLeaves,
            $crate::tables::FlowAttributeLoads,
            $crate::tables::Conditions,
            $crate::tables::ConditionNodes,
            $crate::tables::ConditionLiterals,
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
    out.extend(crate::for_each_analysis_table!(all));
    out.extend(crate::for_each_global_table!(all));
    out.push((Snapshots::NAME, contract::<Snapshots>()));
    out
}
