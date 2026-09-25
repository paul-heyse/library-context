//! Append-only `Int16` codebooks (DESIGN §3.5).
//!
//! Each codebook is declared once here. Codes are dense from 0 and **never renumbered, reordered or
//! removed**; a new value is appended with the next code. `tests/codebooks.rs` snapshots the whole
//! registry, so any change to an existing code shows up as a reviewed snapshot diff.

/// A closed category stored as `Int16` and validated against its codebook.
pub trait Codebook: Copy + Eq + std::fmt::Debug + 'static {
    /// The codebook's name, carried in Arrow field metadata (`lctx.codebook`).
    const NAME: &'static str;
    /// Every value, in code order.
    fn all() -> &'static [Self];
    fn code(self) -> i16;
    fn text(self) -> &'static str;
    fn from_code(code: i16) -> Option<Self>;
}

/// One codebook as data: its name and `(code, text)` pairs in code order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodebookEntry {
    pub name: &'static str,
    pub values: Vec<(i16, &'static str)>,
}

impl CodebookEntry {
    fn of<C: Codebook>() -> Self {
        Self {
            name: C::NAME,
            values: C::all().iter().map(|c| (c.code(), c.text())).collect(),
        }
    }
}

macro_rules! codebook {
    ($(#[$meta:meta])* $ty:ident = $name:literal { $($(#[$vmeta:meta])* $variant:ident = $code:literal => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(i16)]
        pub enum $ty { $($(#[$vmeta])* $variant = $code),+ }

        impl Codebook for $ty {
            const NAME: &'static str = $name;
            fn all() -> &'static [Self] { &[$($ty::$variant),+] }
            fn code(self) -> i16 { self as i16 }
            fn text(self) -> &'static str { match self { $($ty::$variant => $text),+ } }
            fn from_code(code: i16) -> Option<Self> {
                match code { $($code => Some($ty::$variant),)+ _ => None }
            }
        }
    };
}

codebook!(
    /// Where an assertion comes from.
    Origin = "origin" {
        InputContext = 0 => "input_context",
        SourceObservation = 1 => "source_observation",
        AnalyzerAssertion = 2 => "analyzer_assertion",
        DerivedAnalysis = 3 => "derived_analysis",
        SyntheticModel = 4 => "synthetic_model",
    }
);

codebook!(
    /// What structure a fact guarantees (DESIGN §3.5 definitions); a row takes its weakest field's.
    Fidelity = "fidelity" {
        Raw = 0 => "raw",
        NativeStructural = 1 => "native_structural",
        NormalizedStructural = 2 => "normalized_structural",
        ReportProjection = 3 => "report_projection",
        DisplayOnly = 4 => "display_only",
    }
);

codebook!(
    ExtractionMode = "extraction_mode" {
        NativeTraversal = 0 => "native_traversal",
        ReportDecode = 1 => "report_decode",
        RelationalDerivation = 2 => "relational_derivation",
        GraphAnalysis = 3 => "graph_analysis",
        Recognizer = 4 => "recognizer",
        StatisticalAnalysis = 5 => "statistical_analysis",
        TemplateSynthesis = 6 => "template_synthesis",
        FixtureExecution = 7 => "fixture_execution",
        ManualReview = 8 => "manual_review",
    }
);

codebook!(
    Modality = "modality" {
        Definite = 0 => "definite",
        Candidate = 1 => "candidate",
        Potential = 2 => "potential",
    }
);

codebook!(
    CoverageStatus = "coverage_status" {
        CompleteUnderStatedModel = 0 => "complete_under_stated_model",
        Partial = 1 => "partial",
        NotRequested = 2 => "not_requested",
        Unavailable = 3 => "unavailable",
        Failed = 4 => "failed",
    }
);

codebook!(
    ResolutionStatus = "resolution_status" {
        Resolved = 0 => "resolved",
        Partial = 1 => "partial",
        Unresolved = 2 => "unresolved",
        NotAttempted = 3 => "not_attempted",
    }
);

codebook!(
    ResolutionDomain = "resolution_domain" {
        Call = 0 => "call",
        Attribute = 1 => "attribute",
        Import = 2 => "import",
        Name = 3 => "name",
        Type = 4 => "type",
    }
);

codebook!(
    InvocationPhase = "invocation_phase" {
        Call = 0 => "call",
        New = 1 => "new",
        Init = 2 => "init",
        Decorator = 3 => "decorator",
        PropertyGet = 4 => "property_get",
        PropertySet = 5 => "property_set",
    }
);

codebook!(
    BoundaryReason = "boundary_reason" {
        NativeUnavailable = 0 => "native_unavailable",
        UnresolvedTarget = 1 => "unresolved_target",
        UnsupportedUnpacking = 2 => "unsupported_unpacking",
        AmbiguousBinding = 3 => "ambiguous_binding",
        UnsupportedControlFlow = 4 => "unsupported_control_flow",
        ScopeBoundary = 5 => "scope_boundary",
        BudgetReached = 6 => "budget_reached",
        MissingEvidence = 7 => "missing_evidence",
        NotRequested = 8 => "not_requested",
        ProviderDisagreement = 9 => "provider_disagreement",
        OutsideProviderModel = 10 => "outside_provider_model",
        /// The module parsed with errors; facts come from a recovered tree.
        SyntaxError = 11 => "syntax_error",
        /// The module's bytes are not UTF-8; nothing was analyzed.
        UndecodableSource = 12 => "undecodable_source",
        /// A function the provider describes that has no `def` of its own: a synthesized member
        /// (a dataclass `__init__`) or a callable class field (`fn = staticmethod(f)`).
        NoSourceDeclaration = 13 => "no_source_declaration",
        /// A definition the analyzer's context never binds: a branch it decides statically
        /// (`sys.version_info`, `TYPE_CHECKING`) while a same-name definition is bound.
        UnreachableInContext = 14 => "unreachable_in_context",
        /// An export whose origin is a variable, not a `def` or `class` (until the lexical family
        /// gives it a binding node; DESIGN §3.2).
        VariableOrigin = 15 => "variable_origin",
        /// A name- or string-driven access (`getattr` by a non-literal name, `vars()`,
        /// `__dict__`, `importlib`, `exec`/`eval`, a module `__getattr__`) that could reach a place
        /// a negative claim names (ADR-0022 §Verdicts).
        DynamicAccess = 16 => "dynamic_access",
        /// A `self.m(...)` call a subclass may override (ADR-0022 §Verdicts).
        OverrideDispatch = 17 => "override_dispatch",
        /// An operation whose seed declaration the runtime view cannot reach (ADR-0022 §Composed
        /// layers); `unreachable_in_context` is the checker's never-bound.
        RuntimeUnreachable = 18 => "runtime_unreachable",
        /// A value that reaches a sink only inside a call (its callee, receiver or an argument):
        /// whether the callee's result carries it is a summary's question, Stage 3's (ADR-0022
        /// §Verdicts).
        CallTransfer = 19 => "call_transfer",
        /// A body the operation's callers may not run: abstract, a stub (only `pass`, `...` or a
        /// docstring), or one that only raises, so an override or caller supplies the behavior
        /// and "never read" cannot be refuted there (ADR-0022 §Verdicts; the Stage 2 end
        /// review's R1).
        AbstractBody = 20 => "abstract_body",
    }
);

codebook!(
    /// Pysa's unresolved-call reasons at the pinned Pyrefly, spelled as Pysa spells them.
    PysaUnresolvedReason = "pysa_unresolved_reason" {
        LambdaArgument = 0 => "LambdaArgument",
        UnexpectedPyreflyTarget = 1 => "UnexpectedPyreflyTarget",
        EmptyPyreflyCallTarget = 2 => "EmptyPyreflyCallTarget",
        UnknownClassField = 3 => "UnknownClassField",
        ClassFieldOnlyExistInObject = 4 => "ClassFieldOnlyExistInObject",
        UnsupportedFunctionTarget = 5 => "UnsupportedFunctionTarget",
        UnexpectedDefiningClass = 6 => "UnexpectedDefiningClass",
        UnexpectedInitMethod = 7 => "UnexpectedInitMethod",
        UnexpectedNewMethod = 8 => "UnexpectedNewMethod",
        UnexpectedCalleeExpression = 9 => "UnexpectedCalleeExpression",
        UnresolvedMagicDunderAttr = 10 => "UnresolvedMagicDunderAttr",
        UnresolvedMagicDunderAttrDueToNoBase = 11 => "UnresolvedMagicDunderAttrDueToNoBase",
        UnresolvedMagicDunderAttrDueToNoAttribute = 12 => "UnresolvedMagicDunderAttrDueToNoAttribute",
        Mixed = 13 => "Mixed",
    }
);

codebook!(
    EvidenceStatus = "evidence_status" {
        StructurallyObserved = 0 => "structurally_observed",
        Documented = 1 => "documented",
        StatisticallyDerived = 2 => "statistically_derived",
        FixtureChecked = 3 => "fixture_checked",
        Unresolved = 4 => "unresolved",
    }
);

codebook!(
    /// Fact families: the Delta table group, the coverage unit and what an extractor declares.
    FactFamily = "fact_family" {
        Provenance = 0 => "provenance",
        Exports = 1 => "exports",
        Signatures = 2 => "signatures",
        Calls = 3 => "calls",
        Coverage = 4 => "coverage",
        /// The `snapshots` table: the publication act, not a fact family (DESIGN §6.1).
        Publication = 5 => "publication",
        /// The derived `nodes`/`edges` catalogs (DESIGN §3.8): not a coverage unit.
        Graph = 6 => "graph",
        /// C2: syntax nodes the passes read (DESIGN §3.2).
        Syntax = 7 => "syntax",
        /// C3: scopes, bindings, references and their resolution (DESIGN §3.2).
        Lexical = 8 => "lexical",
        /// C4: type terms, their structure, type observations and record fields (DESIGN §3.2).
        Types = 9 => "types",
        /// C5: documents, passages, code blocks, links and mentions (DESIGN §3.2).
        Docs = 10 => "docs",
        /// Analysis results: invocations, findings, witnesses, evidence, assertions and briefs
        /// (ADR-0019). Not a coverage unit: no run declares it.
        Findings = 11 => "findings",
        /// The global embedding cache (DESIGN §3.2, §11.1): not snapshot-qualified.
        EmbeddingCache = 12 => "embedding_cache",
        /// The flow IR (ADR-0022 §The flow provider): definitions, uses, reaching definitions,
        /// statement regions, value sources and their conditions, from `cpg-flow`.
        Flow = 13 => "flow",
    }
);

codebook!(
    ScopeKind = "scope_kind" {
        Release = 0 => "release",
        Module = 1 => "module",
        Callable = 2 => "callable",
        /// C5: a document of a corpus release (the `docs` family's unit).
        Document = 3 => "document",
    }
);

codebook!(
    DeclarationKind = "declaration_kind" {
        Function = 0 => "function",
        AsyncFunction = 1 => "async_function",
        Class = 2 => "class",
    }
);

codebook!(
    ExportSyntaxKind = "export_syntax_kind" {
        Import = 0 => "import",
        ImportFrom = 1 => "import_from",
        DunderAll = 2 => "dunder_all",
    }
);

codebook!(
    ParameterKind = "parameter_kind" {
        PositionalOnly = 0 => "positional_only",
        PositionalOrKeyword = 1 => "positional_or_keyword",
        VarPositional = 2 => "var_positional",
        KeywordOnly = 3 => "keyword_only",
        VarKeyword = 4 => "var_keyword",
    }
);

codebook!(
    /// The shape of a Pysa undecorated signature's parameter list.
    SignatureForm = "signature_form" {
        List = 0 => "list",
        Ellipsis = 1 => "ellipsis",
        ParamSpec = 2 => "param_spec",
    }
);

codebook!(
    ArgumentKind = "argument_kind" {
        Positional = 0 => "positional",
        Starred = 1 => "starred",
        Keyword = 2 => "keyword",
        DoubleStarred = 3 => "double_starred",
    }
);

codebook!(
    /// Pysa's expression identifier kind for a call-graph site.
    PysaSiteKind = "pysa_site_kind" {
        Regular = 0 => "regular",
        ArtificialCall = 1 => "artificial_call",
        ArtificialAttributeAccess = 2 => "artificial_attribute_access",
        Identifier = 3 => "identifier",
        FormatStringArtificial = 4 => "format_string_artificial",
        FormatStringStringify = 5 => "format_string_stringify",
    }
);

codebook!(
    /// What a Pysa call-graph row targets. `Overrides` is a dispatch set, never one callee.
    PysaTargetKind = "pysa_target_kind" {
        Function = 0 => "function",
        Overrides = 1 => "overrides",
        Unresolved = 2 => "unresolved",
    }
);

codebook!(
    /// Which Pysa callee record a `pysa_calls` row came from.
    PysaCalleeKind = "pysa_callee_kind" {
        Call = 0 => "call",
        Identifier = 1 => "identifier",
        AttributeAccess = 2 => "attribute_access",
        FormatStringArtificial = 3 => "format_string_artificial",
        FormatStringStringify = 4 => "format_string_stringify",
    }
);

codebook!(
    ImplicitReceiver = "implicit_receiver" {
        False = 0 => "false",
        TrueWithClassReceiver = 1 => "true_with_class_receiver",
        TrueWithObjectReceiver = 2 => "true_with_object_receiver",
    }
);

codebook!(
    AncestryRelation = "ancestry_relation" {
        Base = 0 => "base",
        Mro = 1 => "mro",
    }
);

codebook!(
    /// Where a dependency module Pyrefly resolved comes from (`context_modules`, DESIGN §3.2).
    ModuleOrigin = "module_origin" {
        /// A file under a site-packages directory of the context.
        SitePackages = 0 => "site_packages",
        /// A file elsewhere on the search path.
        SearchPath = 1 => "search_path",
        Namespace = 2 => "namespace",
        Memory = 3 => "memory",
        BundledTypeshed = 4 => "bundled_typeshed",
        BundledTypeshedThirdParty = 5 => "bundled_typeshed_third_party",
        BundledThirdParty = 6 => "bundled_third_party",
        /// An import names it and Pyrefly's finder cannot find it (C3 review F2): no file and no
        /// node, only the provider's answer.
        NotFound = 7 => "not_found",
    }
);

codebook!(
    /// What a Pysa definition in a dependency module defines (`context_definitions`).
    DefinitionKind = "definition_kind" {
        Function = 0 => "function",
        Class = 1 => "class",
    }
);

codebook!(
    /// The v1 node kinds (DESIGN §3.1, §3.8), appended by CPG slice.
    NodeKind = "node_kind" {
        Module = 0 => "module",
        Class = 1 => "class",
        /// Every `def`, overload stubs included.
        Function = 2 => "function",
        Parameter = 3 => "parameter",
        CallSite = 4 => "call_site",
        Argument = 5 => "argument",
        /// A public access path.
        Export = 6 => "export",
        ExternalModule = 7 => "external_module",
        ExternalSymbol = 8 => "external_symbol",
        SyntheticCallable = 9 => "synthetic_callable",
        /// C2: a placed syntax node that is not a declaration or a call site.
        SyntaxNode = 10 => "syntax_node",
        // C3
        Scope = 11 => "scope",
        Binding = 12 => "binding",
        Reference = 13 => "reference",
        // C4
        /// A distinct type term (`type_terms`).
        Type = 14 => "type",
        /// A record field of a dataclass, attrs or pydantic class, `TypedDict` or `NamedTuple`.
        Field = 15 => "field",
        // C5
        /// A document of the source corpus (`documents`).
        Document = 16 => "document",
        /// A section of a document under its heading (`passages`).
        Passage = 17 => "passage",
        /// A fenced code block (`code_blocks`).
        CodeBlock = 18 => "code_block",
    }
);

codebook!(
    /// The edge kinds of the registry (DESIGN §3.8), appended by CPG slice.
    EdgeKind = "edge_kind" {
        Declares = 0 => "declares",
        OverloadOf = 1 => "overload_of",
        StubFor = 2 => "stub_for",
        HasParameter = 3 => "has_parameter",
        Exports = 4 => "exports",
        EnclosesCall = 5 => "encloses_call",
        HasArgument = 6 => "has_argument",
        CallTarget = 7 => "call_target",
        HigherOrderTarget = 8 => "higher_order_target",
        BaseClass = 9 => "base_class",
        MroEntry = 10 => "mro_entry",
        Overrides = 11 => "overrides",
        DeclaredIn = 12 => "declared_in",
        // C2
        AstChild = 13 => "ast_child",
        ArgumentValue = 14 => "argument_value",
        SiteTarget = 15 => "site_target",
        // C3
        OwnsScope = 16 => "owns_scope",
        LexicalParent = 17 => "lexical_parent",
        Binds = 18 => "binds",
        Introduces = 19 => "introduces",
        ReadsBinding = 20 => "reads_binding",
        Captures = 21 => "captures",
        ReadsBuiltin = 22 => "reads_builtin",
        Shadows = 23 => "shadows",
        PotentialTarget = 24 => "potential_target",
        ImportsModule = 25 => "imports_module",
        // C4
        HasType = 26 => "has_type",
        TypeArg = 27 => "type_arg",
        TypeClass = 28 => "type_class",
        HasField = 29 => "has_field",
        FieldType = 30 => "field_type",
        // C5
        ContainsPassage = 31 => "contains_passage",
        ContainsBlock = 32 => "contains_block",
        Mentions = 33 => "mentions",
        BlockModule = 34 => "block_module",
        /// Retired by the C5 compact review before any snapshot relied on it: the corpus names
        /// the release's installed files by their `@path`, so a usage call's target is the
        /// release's own node and no link is needed.
        UsageLink = 35 => "usage_link",
    }
);

codebook!(
    /// Pyrefly's kind for an exported symbol (`public_names.origin_symbol_kind`, ADR-0014):
    /// what decides whether an unmapped export origin is a variable or our own failure.
    SymbolKind = "symbol_kind" {
        Module = 0 => "module",
        Attribute = 1 => "attribute",
        Variable = 2 => "variable",
        Constant = 3 => "constant",
        Parameter = 4 => "parameter",
        TypeParameter = 5 => "type_parameter",
        TypeAlias = 6 => "type_alias",
        Function = 7 => "function",
        Method = 8 => "method",
        Class = 9 => "class",
    }
);

codebook!(
    /// How an edge kind is asserted (DESIGN §3.8, `edge_kinds`).
    DerivationClass = "derivation_class" {
        /// One provider row states it.
        Extracted = 0 => "extracted",
        /// A provider's own resolution states it (Pysa, Pyrefly's public names).
        Analyzer = 1 => "analyzer",
        /// A Stage-C/D join decides it.
        Joined = 2 => "joined",
        /// Our own analysis decides it.
        Recognizer = 3 => "recognizer",
    }
);

codebook!(
    /// A syntax node's kind (DESIGN §3.2, C2): Ruff's `NodeKind` at the pinned ruff line, in its
    /// declaration order. A Ruff bump that adds a variant fails the exhaustive match in the
    /// extractor and is appended here.
    SyntaxKind = "syntax_kind" {
        ModModule = 0 => "mod_module",
        ModExpression = 1 => "mod_expression",
        StmtFunctionDef = 2 => "stmt_function_def",
        StmtClassDef = 3 => "stmt_class_def",
        StmtReturn = 4 => "stmt_return",
        StmtDelete = 5 => "stmt_delete",
        StmtTypeAlias = 6 => "stmt_type_alias",
        StmtAssign = 7 => "stmt_assign",
        StmtAugAssign = 8 => "stmt_aug_assign",
        StmtAnnAssign = 9 => "stmt_ann_assign",
        StmtFor = 10 => "stmt_for",
        StmtWhile = 11 => "stmt_while",
        StmtIf = 12 => "stmt_if",
        StmtWith = 13 => "stmt_with",
        StmtMatch = 14 => "stmt_match",
        StmtRaise = 15 => "stmt_raise",
        StmtTry = 16 => "stmt_try",
        StmtAssert = 17 => "stmt_assert",
        StmtImport = 18 => "stmt_import",
        StmtImportFrom = 19 => "stmt_import_from",
        StmtGlobal = 20 => "stmt_global",
        StmtNonlocal = 21 => "stmt_nonlocal",
        StmtExpr = 22 => "stmt_expr",
        StmtPass = 23 => "stmt_pass",
        StmtBreak = 24 => "stmt_break",
        StmtContinue = 25 => "stmt_continue",
        StmtIpyEscapeCommand = 26 => "stmt_ipy_escape_command",
        ExprBoolOp = 27 => "expr_bool_op",
        ExprNamed = 28 => "expr_named",
        ExprBinOp = 29 => "expr_bin_op",
        ExprUnaryOp = 30 => "expr_unary_op",
        ExprLambda = 31 => "expr_lambda",
        ExprIf = 32 => "expr_if",
        ExprDict = 33 => "expr_dict",
        ExprSet = 34 => "expr_set",
        ExprListComp = 35 => "expr_list_comp",
        ExprSetComp = 36 => "expr_set_comp",
        ExprDictComp = 37 => "expr_dict_comp",
        ExprGenerator = 38 => "expr_generator",
        ExprAwait = 39 => "expr_await",
        ExprYield = 40 => "expr_yield",
        ExprYieldFrom = 41 => "expr_yield_from",
        ExprCompare = 42 => "expr_compare",
        ExprCall = 43 => "expr_call",
        ExprFString = 44 => "expr_f_string",
        ExprTString = 45 => "expr_t_string",
        ExprStringLiteral = 46 => "expr_string_literal",
        ExprBytesLiteral = 47 => "expr_bytes_literal",
        ExprNumberLiteral = 48 => "expr_number_literal",
        ExprBooleanLiteral = 49 => "expr_boolean_literal",
        ExprNoneLiteral = 50 => "expr_none_literal",
        ExprEllipsisLiteral = 51 => "expr_ellipsis_literal",
        ExprAttribute = 52 => "expr_attribute",
        ExprSubscript = 53 => "expr_subscript",
        ExprStarred = 54 => "expr_starred",
        ExprName = 55 => "expr_name",
        ExprList = 56 => "expr_list",
        ExprTuple = 57 => "expr_tuple",
        ExprSlice = 58 => "expr_slice",
        ExprIpyEscapeCommand = 59 => "expr_ipy_escape_command",
        ExceptHandlerExceptHandler = 60 => "except_handler_except_handler",
        InterpolatedElement = 61 => "interpolated_element",
        InterpolatedStringLiteralElement = 62 => "interpolated_string_literal_element",
        PatternMatchValue = 63 => "pattern_match_value",
        PatternMatchSingleton = 64 => "pattern_match_singleton",
        PatternMatchSequence = 65 => "pattern_match_sequence",
        PatternMatchMapping = 66 => "pattern_match_mapping",
        PatternMatchClass = 67 => "pattern_match_class",
        PatternMatchStar = 68 => "pattern_match_star",
        PatternMatchAs = 69 => "pattern_match_as",
        PatternMatchOr = 70 => "pattern_match_or",
        TypeParamTypeVar = 71 => "type_param_type_var",
        TypeParamTypeVarTuple = 72 => "type_param_type_var_tuple",
        TypeParamParamSpec = 73 => "type_param_param_spec",
        InterpolatedStringFormatSpec = 74 => "interpolated_string_format_spec",
        PatternArguments = 75 => "pattern_arguments",
        PatternKeyword = 76 => "pattern_keyword",
        Comprehension = 77 => "comprehension",
        Arguments = 78 => "arguments",
        Parameters = 79 => "parameters",
        Parameter = 80 => "parameter",
        ParameterWithDefault = 81 => "parameter_with_default",
        Keyword = 82 => "keyword",
        Alias = 83 => "alias",
        WithItem = 84 => "with_item",
        MatchCase = 85 => "match_case",
        Decorator = 86 => "decorator",
        ElifElseClause = 87 => "elif_else_clause",
        TypeParams = 88 => "type_params",
        FString = 89 => "f_string",
        TString = 90 => "t_string",
        StringLiteral = 91 => "string_literal",
        BytesLiteral = 92 => "bytes_literal",
        Identifier = 93 => "identifier",
    }
);

codebook!(
    /// The field of its placed parent a syntax node sits in (C2).
    SyntaxField = "syntax_field" {
        Body = 0 => "body",
        Orelse = 1 => "orelse",
        Test = 2 => "test",
        Handler = 3 => "handler",
        Finalbody = 4 => "finalbody",
        Target = 5 => "target",
        Value = 6 => "value",
        Exc = 7 => "exc",
        Cause = 8 => "cause",
        Msg = 9 => "msg",
        Subject = 10 => "subject",
        Case = 11 => "case",
        Guard = 12 => "guard",
        Iter = 13 => "iter",
        Item = 14 => "item",
        Annotation = 15 => "annotation",
        Left = 16 => "left",
        Right = 17 => "right",
        Operand = 18 => "operand",
        Slice = 19 => "slice",
        Callee = 20 => "callee",
        Argument = 21 => "argument",
        Decorator = 22 => "decorator",
        Element = 23 => "element",
        Child = 24 => "child",
        /// A parameter's default value (C2 review: every expression is placed).
        Default = 25 => "default",
    }
);

codebook!(
    /// A lexical scope's kind (C3; separate from the coverage `scope_kind`).
    LexicalScopeKind = "lexical_scope_kind" {
        Module = 0 => "module",
        Class = 1 => "class",
        Function = 2 => "function",
        Lambda = 3 => "lambda",
        Comprehension = 4 => "comprehension",
    }
);

codebook!(
    /// How a binding event binds its name (C3): the subset of Ruff's `BindingKind` our recognizer
    /// emits, plus the unbinding (`del`) and the scope declarations.
    BindingKind = "binding_kind" {
        FunctionDef = 0 => "function_def",
        ClassDef = 1 => "class_def",
        Parameter = 2 => "parameter",
        Assignment = 3 => "assignment",
        AugAssignment = 4 => "aug_assignment",
        AnnotationOnly = 5 => "annotation_only",
        ForTarget = 6 => "for_target",
        WithTarget = 7 => "with_target",
        ExceptHandler = 8 => "except_handler",
        Import = 9 => "import",
        FromImport = 10 => "from_import",
        StarImport = 11 => "star_import",
        Walrus = 12 => "walrus",
        ComprehensionTarget = 13 => "comprehension_target",
        MatchCapture = 14 => "match_capture",
        Del = 15 => "del",
        Global = 16 => "global",
        Nonlocal = 17 => "nonlocal",
        TypeAlias = 18 => "type_alias",
        TypeParam = 19 => "type_param",
        /// A name the runtime binds with no statement (C3 review F1): a module's implicit globals
        /// (Pyrefly's `ImplicitGlobal` set), a method's `__class__` cell.
        Implicit = 20 => "implicit",
    }
);

codebook!(
    /// A branch the analyzer decides statically (C3): Pyrefly drops the branch it decides
    /// against, the recognizer keeps both.
    StaticBranch = "static_branch" {
        TypeChecking = 0 => "type_checking",
        VersionInfo = 1 => "version_info",
        /// `sys.platform`, or `os.name`.
        Platform = 2 => "platform",
        /// A literal Pyrefly decides (`if False:`, `if 0:`), naming none of the above (H1 C1).
        Constant = 3 => "constant",
        /// A test mixing two of the above (`sys.version_info >= (3, 10) and TYPE_CHECKING`).
        Combined = 4 => "combined",
    }
);

codebook!(
    /// What a type term is (C4): Pyrefly's `Type` variants, mapped by an exhaustive match.
    TypeTermKind = "type_term_kind" {
        /// An instance of a class, with its type arguments (`list[int]`).
        ClassInstance = 0 => "class_instance",
        /// A class itself, as a value (the name `list` in an expression).
        ClassObject = 1 => "class_object",
        /// `type[X]`.
        TypeOf = 2 => "type_of",
        /// A `TypedDict` instance (`detail` = `partial` for its update form).
        TypedDict = 3 => "typed_dict",
        Union = 4 => "union",
        Intersection = 5 => "intersection",
        /// A callable signature; a `def`'s type has its name as `detail`.
        Callable = 6 => "callable",
        Overload = 7 => "overload",
        BoundMethod = 8 => "bound_method",
        /// A generic callable or alias with its own type parameters (Pyrefly's `Forall`).
        Generic = 9 => "generic",
        Tuple = 10 => "tuple",
        Literal = 11 => "literal",
        TypeVar = 12 => "type_var",
        ParamSpec = 13 => "param_spec",
        TypeVarTuple = 14 => "type_var_tuple",
        Module = 15 => "module",
        /// `detail`: `explicit` (written), `implicit` (nothing written) or `error`.
        Any = 16 => "any",
        Never = 17 => "never",
        None = 18 => "none",
        TypeAlias = 19 => "type_alias",
        SelfType = 20 => "self_type",
        Annotated = 21 => "annotated",
        Unpack = 22 => "unpack",
        /// `TypeGuard[X]` or `TypeIs[X]` (`detail`).
        TypeGuard = 23 => "type_guard",
        /// A parameter list standing alone (a `ParamSpec`'s value, `Concatenate[...]`).
        ParamList = 24 => "param_list",
        /// A special form, or a type-variable declaration used as a value.
        SpecialForm = 25 => "special_form",
        /// A variant outside the stated model (solver-internal or experimental): display only.
        Other = 26 => "other",
        /// Cut at the depth cap: display only, no children.
        Truncated = 27 => "truncated",
    }
);

codebook!(
    /// A child's position in its parent type term (C4, `type_term_args`).
    TypeArgRole = "type_arg_role" {
        /// A class, `TypedDict` or alias type argument.
        Argument = 0 => "argument",
        /// A union or intersection member.
        Member = 1 => "member",
        /// A callable parameter (name, kind and requiredness on the row).
        Parameter = 2 => "parameter",
        Return = 3 => "return",
        /// A tuple element at a fixed position.
        Element = 4 => "element",
        /// The repeated element of `tuple[X, ...]`, or the unpacked middle of a tuple.
        Variadic = 5 => "variadic",
        /// One signature of an overload.
        Signature = 6 => "signature",
        /// The object a method is bound to.
        Receiver = 7 => "receiver",
        /// The function a method binds.
        Function = 8 => "function",
        /// A type parameter a generic term binds.
        TypeParameter = 9 => "type_parameter",
        /// The inner type of a wrapper (`type[X]`, `Annotated`, `Unpack`, a guard, an alias's value,
        /// a generic's body).
        Target = 10 => "target",
        /// The `ParamSpec` standing for a callable's remaining parameters.
        ParamSpec = 11 => "param_spec",
        /// A type variable's upper bound (C4 review F5).
        Bound = 12 => "bound",
        /// One of a type variable's constraints.
        Constraint = 13 => "constraint",
        /// A type variable's default.
        Default = 14 => "default",
    }
);

codebook!(
    /// What a type observation types (C4, `type_observations`; DESIGN §3.5.1): the role follows
    /// from the subject.
    TypeRole = "type_role" {
        /// A parameter's type.
        Parameter = 0 => "parameter",
        /// A function's return type.
        Return = 1 => "return",
        /// The value a call site evaluates to.
        CallResult = 2 => "call_result",
        /// The value passed as an argument.
        Argument = 3 => "argument",
        /// The exception a `raise` statement raises.
        Raised = 4 => "raised",
    }
);

codebook!(
    /// Which record model a class's fields come from (C4, `record_fields`).
    RecordKind = "record_kind" {
        Dataclass = 0 => "dataclass",
        Attrs = 1 => "attrs",
        Pydantic = 2 => "pydantic",
        TypedDict = 3 => "typed_dict",
        NamedTuple = 4 => "named_tuple",
    }
);

codebook!(
    /// What a test/operand type row warrants. A trace observation alone cannot prove the exact
    /// runtime class or standard equality of the value it describes.
    TestTypeOrigin = "test_type_origin" {
        PyreflyTrace = 0 => "pyrefly_trace",
    }
);

codebook!(
    /// The cited reason an operation's entry formal is the value read by a test leaf.
    /// Further origins are appended only with an effect and identity proof.
    TestValueLinkOrigin = "test_value_link_origin" {
        DirectParameterReachNoEffect = 0 => "direct_parameter_reach_no_effect",
        /// The only intervening call is this leaf's resolved one-argument builtin `type(x)`.
        ResolvedBuiltinTypeOperand = 1 => "resolved_builtin_type_operand",
        /// A later use on the true branch of a cited, effect-free exact-type guard.
        StableAfterExactTypeGuard = 2 => "stable_after_exact_type_guard",
    }
);

codebook!(
    /// An exact entry-value assertion and the condition under which it holds.
    ExactValueOrigin = "exact_value_origin" {
        ResolvedBuiltinTypeGuard = 0 => "resolved_builtin_type_guard",
    }
);

codebook!(
    /// How a mention names an API (C5, `mentions`; DESIGN §3.2 `docs`): the two classes are never
    /// merged.
    MentionClass = "mention_class" {
        /// The text is a public access path, or a public class's member (`FastMCP.tool`).
        Exact = 0 => "exact",
        /// The text is a bare public name: a candidate of every access path with that name.
        Lexical = 1 => "lexical",
    }
);

codebook!(
    /// Where in a passage a mention's text sits (C5, `mentions`).
    MentionSource = "mention_source" {
        InlineCode = 0 => "inline_code",
        Prose = 1 => "prose",
    }
);

codebook!(
    /// How an MDX JSX component is written (`doc_components.form`; the holistic assessment's
    /// A3): as a block of its own, or inside a paragraph's text.
    ComponentForm = "component_form" {
        Flow = 0 => "flow",
        Text = 1 => "text",
    }
);

codebook!(
    /// What an MDX JSX attribute's value is (`doc_component_attributes.value_kind`; A3). Only a
    /// literal is ever read as a value; an expression's source text is kept, never evaluated.
    AttributeValueKind = "attribute_value_kind" {
        /// `b="c"`: the value as written.
        Literal = 0 => "literal",
        /// `b={c}`: the expression's source text.
        Expression = 1 => "expression",
        /// `b`: a name with no value.
        Bare = 2 => "bare",
        /// `{...b}`: a spread, no name; the expression's source text.
        Spread = 3 => "spread",
    }
);

codebook!(
    /// What an analyzed module is to its library (ADR-0015, `source_files.role`): the release
    /// itself, or corpus code using it, by the `[tool.lctx.source]` key that selected it.
    SourceRole = "source_role" {
        /// A module of the analyzed release (a library or a source tree).
        Release = 0 => "release",
        /// Selected by `examples`: official example code.
        Example = 1 => "example",
        /// Selected by `tests`: the library's own tests.
        Test = 2 => "test",
        /// A Python code block of a selected document, materialized as a module.
        DocBlock = 3 => "doc_block",
    }
);

codebook!(
    /// The analytic method an `analysis_invocations` row ran (ADR-0019; DESIGN §9).
    AnalyticMethod = "analytic_method" {
        /// Pass A: a bounded breadth-first search with parent pointers over the invocation
        /// projection (§9.1).
        PassABfs = 0 => "pass_a_bfs",
        /// Pass B: a bounded worklist over argument flows and parameter guards (§9.2).
        PassBFlows = 1 => "pass_b_flows",
        /// Pass C: producer → consumer handoffs in the official usage code (§9.3).
        PassCHandoffs = 2 => "pass_c_handoffs",
        /// One Leiden run (leiden-rs, RBER) at one resolution and seed over the community layers
        /// (§9.4).
        Leiden = 3 => "leiden",
        /// The consensus over the Leiden runs: the pre-registered resolution choice, seed
        /// stability and per-community agreement (§9.4). Its communities cite it.
        CommunityConsensus = 4 => "community_consensus",
        /// Our weighted power iteration over the usage projection (§9.5).
        PageRank = 5 => "pagerank",
        /// Formal concept analysis of one structural scope: our own NextClosure, concepts and the
        /// Duquenne–Guigues basis over a support threshold (§9.6).
        Fca = 6 => "fca_next_closure",
        /// Seed selection within the brief budget (§9.4, §9.5; slice 2.6; the increment-2 review's
        /// U1): usage ranks, communities cap. Its diagnostics name the configured, selected and
        /// dropped seeds.
        SeedSelection = 7 => "seed_selection",
        /// Exact nearest neighbours over cached embeddings: doc links and community labels
        /// (§9.7; slice 3.1).
        Knn = 8 => "knn",
        /// Each public API's direct official-usage calls, counted over the usage projection
        /// (§9.5; the increment-2 review's U1).
        UsageCount = 9 => "usage_count",
        /// Pass B's worklist over every public callable of the release (the behavioral-model plan,
        /// Stage 1; ADR-0021): the control fates of the `behaviors` table, never brief findings.
        PassBSurface = 10 => "pass_b_surface",
    }
);

codebook!(
    /// What a finding states (DESIGN §9, §10.1). Never a sentence.
    FindingKind = "finding_kind" {
        /// A public access path names the seed's declaration (§9.1).
        PublicAlias = 0 => "public_alias",
        /// The seed calls the target directly, through a definite arc inside the subsystem.
        DirectDelegation = 1 => "direct_delegation",
        /// The seed reaches the target through a bounded path, or through a candidate
        /// (override-open) arc, inside the subsystem.
        BoundedDelegationPath = 2 => "bounded_delegation_path",
        /// A reached arc leaves what the pass analyzes: the subsystem, the release (a dependency
        /// or bundled definition) or a body in source (a synthetic callable).
        ImplementationBoundary = 3 => "implementation_boundary",
        /// A reached call site has no target, or an unresolved remainder.
        IncompleteResolution = 4 => "incomplete_resolution",
        /// The traversal stopped with arcs or sites not followed: at its depth bound, or at a
        /// vertex or arc budget (the stop reason says which; slice 1.5 review F1).
        TraversalStop = 5 => "traversal_stop",
        /// A seed parameter reaches a callee's formal unchanged, directly or through one
        /// identity alias (Pass B, §9.2). The related node is the formal.
        Forwarding = 6 => "forwarding",
        /// The seed supplies a callee's formal with a literal (Pass B). The related node is the
        /// formal; the value is a member.
        TransformedArgument = 7 => "transformed_argument",
        /// A reached callable raises in the branch of an `if` that tests a parameter the seed's
        /// parameter reaches (Pass B). The related node is the `raise`, the condition the test.
        ConditionalRaise = 8 => "conditional_raise",
        /// Official usage code passes what one public callable returns directly to another (Pass
        /// C, §9.3): `x = producer(...); consumer(x)` in one straight-line block, or nested. The
        /// subject is the seed, the related node the other callable; the score counts occurrences.
        Handoff = 9 => "handoff",
        /// A seed parameter reaches a call's argument in a form Pass B does not follow: after the
        /// name is rebound, inside an expression, or unpacked or taken by no single formal (slice
        /// 2.1 review F4). The related node is the callee; a `reason` member says which.
        UnfollowedArgument = 10 => "unfollowed_argument",
        /// Subsystem callables Leiden places together, stably across seeds (§9.4). The members
        /// are its public APIs; the subject its strongest public member; the score its seed
        /// agreement. Statistical: it never states a control or a limit.
        Community = 11 => "community",
        /// A public API's rank in the usage projection (§9.5): the score is its PageRank.
        /// Statistical: in the `+pagerank` variant it orders seeds and Related entries, never
        /// states behaviour.
        Centrality = 12 => "centrality",
        /// A formal concept of one structural scope (§9.6): public APIs (its extent) sharing
        /// attributes (its intent). The subject is the scope; the score its extent's size. A brief
        /// states it as a shared signature (`AssertionKind::SharedSignature`), not as an
        /// applicable case (the increment-2 review's U2); the kind's name is historical.
        ApplicableCase = 13 => "applicable_case",
        /// An implication of the scope's Duquenne–Guigues basis, with confidence 1 over the support
        /// threshold: every API with the premise's attributes has the conclusion's. The score is
        /// its support.
        Implication = 14 => "implication",
        /// A corpus passage near a public API by embedding similarity (§9.7): the subject is the
        /// API, the related node the passage, the score the cosine. Statistical: a link, never an
        /// Outcome or a claim.
        DocLink = 15 => "doc_link",
        /// The heading of the passage nearest a community's centroid (§9.7): the subject is the
        /// community's subject, the related node the passage, the score the cosine.
        CommunityLabel = 16 => "community_label",
        /// A public API's direct official-usage calls (§9.5): definite call arcs from a function or
        /// module of an example, test or doc block, one per call site. The score is the count.
        /// It orders seeds and Related entries, never states behaviour.
        DirectUsage = 17 => "direct_usage",
    }
);

codebook!(
    /// Why an analysis stopped where it did (a finding's or an invocation's).
    StopReason = "stop_reason" {
        /// The depth budget.
        DepthLimit = 0 => "depth_limit",
        /// The per-seed vertex budget.
        VertexBudget = 1 => "vertex_budget",
        /// The per-seed arc budget.
        EdgeBudget = 2 => "edge_budget",
        /// Reserved: the witness cap is `witnesses_omitted`, never a stop (slice 1.4 review O1).
        WitnessLimit = 3 => "witness_limit",
        /// The target is a release callable outside the subsystem.
        SubsystemBoundary = 4 => "subsystem_boundary",
        /// The target is outside the release: a dependency or bundled definition.
        ExternalBoundary = 5 => "external_boundary",
        /// The target has no body in source (a synthetic callable).
        SyntheticBoundary = 6 => "synthetic_boundary",
        /// The call site has no target, or an unresolved remainder.
        UnresolvedSite = 7 => "unresolved_site",
        /// FCA's closed-set budget: more concepts and implications may exist.
        ConceptBudget = 8 => "concept_budget",
    }
);

codebook!(
    /// The role of a `finding_members` row.
    MemberRole = "member_role" {
        /// A public access path (an `export` node) of a `public_alias` finding.
        AccessPath = 0 => "access_path",
        /// The seed parameter a Pass B finding starts from.
        SourceParameter = 1 => "source_parameter",
        /// A literal a Pass B finding records, as written.
        Value = 2 => "value",
        /// The alias a forwarded value passes through.
        Alias = 3 => "alias",
        /// The callee formal a Pass B finding's guard tests, or a handoff's consumer formal.
        Formal = 4 => "formal",
        /// A handoff occurrence's producer call site (its module path as label).
        ProducerSite = 5 => "producer_site",
        /// A handoff occurrence's consumer call site.
        ConsumerSite = 6 => "consumer_site",
        /// A call site on a Pass B path that its caller makes only on some paths (inside a
        /// conditional construct; slice 2.1 review F1).
        ConditionalCall = 7 => "conditional_call",
        /// Why Pass B does not follow a value (`rebound`, `computed`, `unmapped`).
        Reason = 8 => "reason",
        /// A public API in a community (its access path as label, its strength in the community
        /// as weight).
        CommunityMember = 9 => "community_member",
        /// A call site or usage scope behind one of a community's strongest pairs: the lineage an
        /// aggregated pair keeps (H1 review F9). The label names the layer.
        SupportingSite = 10 => "supporting_site",
        /// A concept's object: a public API (its access path as label).
        ExtentMember = 11 => "extent_member",
        /// A concept's attribute (the attribute as label).
        IntentAttribute = 12 => "intent_attribute",
        /// An implication's premise attribute.
        Premise = 13 => "premise",
        /// An implication's conclusion attribute.
        Conclusion = 14 => "conclusion",
        /// A passage's heading (or its document's path), as a doc link or community label shows it.
        Label = 15 => "label",
    }
);

codebook!(
    /// How a projection arc joins its ends (DESIGN §5; slice 1.4 review F1).
    ArcKind = "arc_kind" {
        /// The caller may invoke the callee at a call (or property) site.
        Call = 0 => "call",
        /// The caller defines the callee in its body (a nested function it returns or
        /// registers, as a decorator factory does).
        Definition = 1 => "definition",
    }
);

codebook!(
    /// What an assertion states (DESIGN §10.2): each kind has one brief section and a set of
    /// permitted statuses (`findings::ASSERTION_POLICY`). Increment-1 kinds first.
    AssertionKind = "assertion_kind" {
        /// What the caller can accomplish: a docstring summary or an explicit doc mention.
        Outcome = 0 => "outcome",
        /// The public access paths of the operation.
        PublicAccess = 1 => "public_access",
        /// What the operation already coordinates (Pass A's delegations).
        Coordinates = 2 => "coordinates",
        /// One parameter: name, kind, default, requiredness, annotation.
        Parameter = 3 => "parameter",
        /// Where the analysis stopped, and why.
        AnalysisBoundary = 4 => "analysis_boundary",
        /// Other briefs in the same community (increment 2).
        Related = 5 => "related",
        /// A parameter the operation passes on to a callee (Pass B `forwarding`).
        Control = 6 => "control",
        /// A callee formal the operation fixes to a literal (Pass B `transformed_argument`).
        TransformedControl = 7 => "transformed_control",
        /// A branch in which the implementation raises (Pass B `conditional_raise`); a public
        /// precondition only with documented support.
        Restriction = 8 => "restriction",
        /// A verbatim statement subset of an official example, test or doc block that uses the
        /// operation, with its setup (§10.5).
        UsagePattern = 9 => "usage_pattern",
        /// A direct handoff official usage code shows (Pass C `handoff`).
        Handoff = 10 => "handoff",
        /// A parameter that reaches a callee in a form the analysis does not follow (Pass B
        /// `unfollowed_argument`).
        UnfollowedControl = 11 => "unfollowed_control",
        /// An input or mode the operation applies to (§10.3). Reserved: no v1 source states one.
        /// Slice 2.5 filled it from an FCA concept; the increment-2 review's U2 moved that to
        /// `SharedSignature`, so the slot stays absent and the gap metric sees it.
        ApplicableCase = 12 => "applicable_case",
        /// An implication of the seed's scope that the seed satisfies (FCA `implication`).
        Implication = 13 => "implication",
        /// Documentation near the operation by embedding similarity (kNN `doc_link`).
        DocLink = 14 => "doc_link",
        /// The public APIs of the seed's own scope that share its signature: the FCA concept
        /// holding it with the most (other API, shared attribute) pairs (FCA `applicable_case`).
        SharedSignature = 15 => "shared_signature",
        /// A `<Warning>` component of a doc passage that exactly mentions the operation, verbatim
        /// (§10.3 Limits; slice 3.4).
        DocumentedWarning = 16 => "documented_warning",
    }
);

codebook!(
    /// A brief's sections, in their presentation order (DESIGN §10.3).
    BriefSection = "brief_section" {
        Outcome = 0 => "outcome",
        PublicAccess = 1 => "public_access",
        ApplicableCase = 2 => "applicable_case",
        Controls = 3 => "controls",
        UsagePattern = 4 => "usage_pattern",
        Limits = 5 => "limits",
        Evidence = 6 => "evidence",
        Related = 7 => "related",
    }
);

codebook!(
    /// How a finding or evidence row supports an assertion (DESIGN §10.2).
    SupportRole = "support_role" {
        /// It supports the claim.
        Support = 0 => "support",
        /// It defined the claim's scope (a statistical scope makes the claim statistical).
        Scope = 1 => "scope",
    }
);

codebook!(
    /// What an evidence row cites (DESIGN §3.2).
    EvidenceKind = "evidence_kind" {
        /// An extracted fact and its source span.
        Fact = 0 => "fact",
        /// A byte span of a source file.
        Span = 1 => "span",
        /// A byte span of a document passage.
        Passage = 2 => "passage",
        /// An official example module.
        Example = 3 => "example",
        /// An executed fixture run.
        FixtureRun = 4 => "fixture_run",
    }
);

codebook!(
    /// A brief's manual review (DESIGN §10.4; ADR-0020, slice 3.5). Outside `brief_id`.
    ReviewState = "review_state" {
        Unreviewed = 0 => "unreviewed",
        Accepted = 1 => "accepted",
        Rejected = 2 => "rejected",
        NeedsChanges = 3 => "needs_changes",
    }
);

impl FactFamily {
    /// Whether a run may declare the family and so owe a coverage row per module or document:
    /// the extraction families. Publication, the catalogs, analysis results and the embedding
    /// cache are not coverage units (ADR-0019 review O1).
    pub fn is_coverage_unit(self) -> bool {
        !matches!(
            self,
            FactFamily::Publication
                | FactFamily::Graph
                | FactFamily::Findings
                | FactFamily::EmbeddingCache
        )
    }
}

codebook!(
    /// Why Pass B does not follow a parameter read (§9.2; the holistic assessment's A2(d)): kept
    /// as its text in `finding_members.label` (role `reason`), read back through this codebook so
    /// every reason is matched by name, never by a default.
    UnfollowedReason = "unfollowed_reason" {
        /// The caller rebinds the parameter before the call.
        Rebound = 0 => "rebound",
        /// The value is computed from the parameter (an expression).
        Computed = 1 => "computed",
        /// The value is unpacked, or no single formal takes it.
        Unmapped = 2 => "unmapped",
    }
);

codebook!(
    /// How an argument's value arises in Pass B's argument flows (§9.2; `cpg_schema::flows`),
    /// persisted in the `argument_flows` table (the behavioral-model plan, Stage 1). The codes are
    /// `flows::value_class`'s.
    ValueClass = "value_class" {
        /// A name bound, once in its scope, by a parameter of the caller.
        Parameter = 0 => "parameter",
        /// A name bound once, directly in the caller's body, from such a parameter.
        Alias = 1 => "alias",
        /// A string, number, boolean or `None` literal, as written.
        Literal = 2 => "literal",
        /// Anything else, never followed.
        Other = 3 => "other",
    }
);

codebook!(
    /// What a `behaviors` row states about a public operation (ADR-0021, ADR-0022; DESIGN §3.2).
    BehaviorKind = "behavior_kind" {
        /// A parameter reaches a callee's formal unchanged, directly or through one identity alias.
        Forwards = 0 => "forwards",
        /// The operation supplies a callee's formal with a literal.
        SuppliesLiteral = 1 => "supplies_literal",
        /// A reached callable raises in a branch that tests a formal the parameter reaches.
        RaisesWhen = 2 => "raises_when",
        /// A parameter is read at a call in a form the analysis does not follow.
        Unfollowed = 3 => "unfollowed",
        /// The operation calls a release function (a depth-1 call arc).
        Delegates = 4 => "delegates",
        /// Official usage passes the operation's result directly to another public operation.
        HandsOffTo = 5 => "hands_off_to",
        /// Official usage passes another public operation's result directly to this one.
        TakesFrom = 6 => "takes_from",
        /// A parameter's value is computed into a call's argument (the flow IR's derived value
        /// source; Stage 2): the callee named as written when it is outside the release.
        Derives = 7 => "derives",
        /// A parameter is stored: to a field (`self.f`) or a dict entry (`d["k"]`).
        Stores = 8 => "stores",
        /// The operation reads a setting of a module-global singleton, in a read phase.
        ReadsSetting = 9 => "reads_setting",
        /// The claim "the parameter is read": established when read, refuted under the model when
        /// its premise holds (ADR-0022 §Verdicts), unknown otherwise.
        IsRead = 10 => "is_read",
        /// A parameter is returned (identity or derived).
        Returns = 11 => "returns",
        /// A parameter decides a branch of the operation: the literals over it, as the flow IR's
        /// region conditions test them.
        Tests = 12 => "tests",
    }
);

codebook!(
    /// The verdict every behavioral answer carries (ADR-0022; DESIGN §3.9): never a null.
    Verdict = "verdict" {
        /// Derived under the stated model, with no boundary in the region the predicate reads.
        Established = 0 => "established",
        /// Established under a stated condition.
        Conditional = 1 => "conditional",
        /// Only in a region complete under the stated model with no boundary of the kinds the
        /// predicate names (a rule rejects it anywhere else).
        RefutedUnderModel = 2 => "refuted_under_model",
        /// A boundary intervenes; its reason is named.
        Unknown = 3 => "unknown",
        /// Out of scope, not requested, or cut by a budget.
        NotAnalyzed = 4 => "not_analyzed",
    }
);

codebook!(
    /// A facet of a public operation, for `find_operations` (ADR-0021; DESIGN §11.3).
    OperationFacet = "operation_facet" {
        /// A parameter's name (`*NAME`, `**NAME` for the catch-alls), the receiver aside.
        Parameter = 0 => "parameter",
        /// A parameter's declared type, as Pyrefly displays it.
        ParameterType = 1 => "parameter_type",
        /// The declared return type.
        Returns = 2 => "returns",
        /// An exception class a `raise` directly in the body raises.
        Raises = 3 => "raises",
        /// A decorator, as written (its trailing name).
        Decorator = 4 => "decorator",
        /// `true` for an `async def`.
        Async = 5 => "async",
        /// A release callable the operation calls directly (its preferred public path, else its
        /// qualified name).
        DelegatesTo = 6 => "delegates_to",
        /// A callee a parameter of the operation reaches unchanged.
        ForwardsTo = 7 => "forwards_to",
        /// A public operation official usage passes this one's result to.
        HandsOffTo = 8 => "hands_off_to",
        /// A public operation whose result official usage passes to this one.
        TakesFrom = 9 => "takes_from",
        /// The module that declares the operation.
        Module = 10 => "module",
        /// The operation's kind: `function`, `method` or `class`.
        Kind = 11 => "kind",
        /// A setting the operation reads in its own body (`module.global.field`; Stage 2).
        ReadsSetting = 12 => "reads_setting",
    }
);

codebook!(
    /// A view of a public operation embedded for `search_operations` (ADR-0010 amendment,
    /// 2026-09-24): the view is a column, and every view shares one spec (one vector space).
    EmbeddingView = "embedding_view" {
        /// The preferred path with its parameters, then the docstring (§9.7's API text).
        SignatureDoc = 0 => "signature_doc",
        /// The declaration's source, cut into windows at line ends.
        SourceBody = 1 => "source_body",
    }
);

codebook!(
    /// The atoms of the closed condition language (ADR-0022 §Conditions; `cpg_schema::condition`).
    ConditionAtom = "condition_atom" {
        /// `p is None` (negated: `p is not None`).
        IsNone = 0 => "is_none",
        /// `p == v` for a literal `v`.
        Equals = 1 => "equals",
        /// `p in {v, …}` for literals.
        MemberOf = 2 => "member_of",
        /// The truthiness of `p`.
        Truthy = 3 => "truthy",
        /// `isinstance(p, C)`, `C` as written.
        IsInstance = 4 => "isinstance",
        /// Any other test, as its source text.
        Opaque = 5 => "opaque",
        /// `p is v` for a literal `v` (negated: `p is not v`).
        IsValue = 6 => "is_value",
        /// Resolved builtin `type(p) is C`; C is an exact builtin class.
        TypeIs = 7 => "type_is",
    }
);

codebook!(
    /// What a flow value source's sink is (ADR-0022 §The flow provider; `cpg_flow::Sink`).
    FlowSink = "flow_sink" {
        /// A definition's value: an assignment's right-hand side, a walrus's value, an
        /// augmented assignment's operand, a `for` or comprehension iterable, a `with` context.
        Definition = 0 => "definition",
        /// A call argument's value.
        Argument = 1 => "argument",
        Return = 2 => "return",
        Yield = 3 => "yield",
        /// A `raise`'s exception or its cause.
        Raise = 4 => "raise",
    }
);

codebook!(
    /// When a setting is read (ADR-0022 §Verdicts, the Stage 2 review's F8): decided by the
    /// reading site's scope.
    ReadPhase = "read_phase" {
        /// A module or class body.
        Import = 0 => "import",
        /// `__init__` or `__post_init__`.
        Construction = 1 => "construction",
        /// At construction, and the value is stored to a field.
        Snapshot = 2 => "snapshot",
        /// Any other function.
        PerCall = 3 => "per_call",
    }
);

codebook!(
    /// A name- or string-driven access (ADR-0022 §Verdicts, `dynamic_access`).
    DynamicKind = "dynamic_kind" {
        Getattr = 0 => "getattr",
        Setattr = 1 => "setattr",
        Hasattr = 2 => "hasattr",
        Delattr = 3 => "delattr",
        Vars = 4 => "vars",
        Dict = 5 => "__dict__",
        ImportModule = 6 => "import_module",
        DunderImport = 7 => "__import__",
        Exec = 8 => "exec",
        Eval = 9 => "eval",
    }
);

codebook!(
    /// The place kind a negative claim's premise is about (ADR-0022 §Verdicts).
    PremiseKind = "premise_kind" {
        Parameter = 0 => "parameter",
        Field = 1 => "field",
        Global = 2 => "global",
        ForwardChain = 3 => "forward_chain",
    }
);

/// Every codebook, in declaration order: the snapshot-tested registry.
pub fn registry() -> Vec<CodebookEntry> {
    vec![
        CodebookEntry::of::<Origin>(),
        CodebookEntry::of::<Fidelity>(),
        CodebookEntry::of::<ExtractionMode>(),
        CodebookEntry::of::<Modality>(),
        CodebookEntry::of::<CoverageStatus>(),
        CodebookEntry::of::<ResolutionStatus>(),
        CodebookEntry::of::<ResolutionDomain>(),
        CodebookEntry::of::<InvocationPhase>(),
        CodebookEntry::of::<BoundaryReason>(),
        CodebookEntry::of::<PysaUnresolvedReason>(),
        CodebookEntry::of::<EvidenceStatus>(),
        CodebookEntry::of::<FactFamily>(),
        CodebookEntry::of::<ScopeKind>(),
        CodebookEntry::of::<DeclarationKind>(),
        CodebookEntry::of::<ExportSyntaxKind>(),
        CodebookEntry::of::<ParameterKind>(),
        CodebookEntry::of::<SignatureForm>(),
        CodebookEntry::of::<ArgumentKind>(),
        CodebookEntry::of::<PysaSiteKind>(),
        CodebookEntry::of::<PysaTargetKind>(),
        CodebookEntry::of::<PysaCalleeKind>(),
        CodebookEntry::of::<ImplicitReceiver>(),
        CodebookEntry::of::<AncestryRelation>(),
        CodebookEntry::of::<ModuleOrigin>(),
        CodebookEntry::of::<DefinitionKind>(),
        CodebookEntry::of::<NodeKind>(),
        CodebookEntry::of::<EdgeKind>(),
        CodebookEntry::of::<SymbolKind>(),
        CodebookEntry::of::<DerivationClass>(),
        CodebookEntry::of::<SyntaxKind>(),
        CodebookEntry::of::<SyntaxField>(),
        CodebookEntry::of::<LexicalScopeKind>(),
        CodebookEntry::of::<BindingKind>(),
        CodebookEntry::of::<StaticBranch>(),
        CodebookEntry::of::<TypeTermKind>(),
        CodebookEntry::of::<TypeArgRole>(),
        CodebookEntry::of::<TypeRole>(),
        CodebookEntry::of::<TestTypeOrigin>(),
        CodebookEntry::of::<TestValueLinkOrigin>(),
        CodebookEntry::of::<ExactValueOrigin>(),
        CodebookEntry::of::<RecordKind>(),
        CodebookEntry::of::<MentionClass>(),
        CodebookEntry::of::<MentionSource>(),
        CodebookEntry::of::<SourceRole>(),
        CodebookEntry::of::<AnalyticMethod>(),
        CodebookEntry::of::<FindingKind>(),
        CodebookEntry::of::<StopReason>(),
        CodebookEntry::of::<MemberRole>(),
        CodebookEntry::of::<AssertionKind>(),
        CodebookEntry::of::<BriefSection>(),
        CodebookEntry::of::<SupportRole>(),
        CodebookEntry::of::<EvidenceKind>(),
        CodebookEntry::of::<ReviewState>(),
        CodebookEntry::of::<ArcKind>(),
        CodebookEntry::of::<ComponentForm>(),
        CodebookEntry::of::<AttributeValueKind>(),
        CodebookEntry::of::<UnfollowedReason>(),
        CodebookEntry::of::<ValueClass>(),
        CodebookEntry::of::<BehaviorKind>(),
        CodebookEntry::of::<Verdict>(),
        CodebookEntry::of::<OperationFacet>(),
        CodebookEntry::of::<EmbeddingView>(),
        CodebookEntry::of::<ConditionAtom>(),
        CodebookEntry::of::<FlowSink>(),
        CodebookEntry::of::<ReadPhase>(),
        CodebookEntry::of::<DynamicKind>(),
        CodebookEntry::of::<PremiseKind>(),
    ]
}

/// The inclusive code range of a codebook, for local validation (never a Delta CHECK: codebooks
/// grow, DESIGN §8).
pub fn code_range(name: &str) -> Option<(i16, i16)> {
    registry().into_iter().find(|c| c.name == name).map(|c| {
        (
            c.values.first().map_or(0, |v| v.0),
            c.values.last().map_or(-1, |v| v.0),
        )
    })
}
