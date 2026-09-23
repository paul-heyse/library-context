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
    }
);

codebook!(
    ScopeKind = "scope_kind" {
        Release = 0 => "release",
        Module = 1 => "module",
        Callable = 2 => "callable",
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
