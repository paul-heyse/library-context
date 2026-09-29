//! Source and attributed syntax facts. Identity never depends on a normalized L1 entity.
use crate::{Domain, DomainCode, DomainSum};
use super::{ContentHash, Id, ModelError, Relation, ValidatedModel};

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "packages")]
pub struct Package {
    #[model(key)] pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "releases")]
pub struct Release {
    #[model(key)] pub package: Id<Package>,
    #[model(key)] pub version: String,
    #[model(key)] pub lock_digest: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "source_artifacts", validate = validate_source)]
pub struct SourceArtifact {
    #[model(key)] pub release: Id<Release>,
    #[model(key)] pub path: String,
    #[model(key)] pub content: ContentHash,
}
fn validate_source(row: &SourceArtifact) -> Result<(), ModelError> {
    if row.path.is_empty() || row.path.starts_with('/') || row.path.split('/').any(|p| p == "." || p == ".." || p.is_empty()) {
        return Err(ModelError::Invalid("source path must be relative and normalized".into()));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "modules")]
pub struct Module {
    #[model(key)] pub source: Id<SourceArtifact>,
    #[model(key)] pub qualified_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "occurrences", validate = validate_occurrence)]
pub struct Occurrence {
    #[model(key)] pub source: Id<SourceArtifact>,
    #[model(key)] pub start: i64,
    #[model(key)] pub end: i64,
    #[model(key)] pub syntax_kind: SyntaxKind,
    /// Structural role distinguishes events sharing a span (e.g. individual with-items).
    #[model(key)] pub role: String,
}
fn validate_occurrence(row: &Occurrence) -> Result<(), ModelError> {
    if row.start < 0 || row.end < row.start || row.role.is_empty() {
        return Err(ModelError::Invalid("invalid occurrence span/kind/role".into()));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "providers")]
pub struct Provider {
    #[model(key)] pub tool: String,
    #[model(key)] pub revision: String,
    #[model(key)] pub build_digest: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "analysis_contexts")]
pub struct AnalysisContext {
    #[model(key)] pub python_version: String,
    #[model(key)] pub python_platform: String,
    #[model(key)] pub search_path: Vec<String>,
    #[model(key)] pub site_package_path: Vec<String>,
    #[model(key)] pub config_digest: ContentHash,
    #[model(key)] pub environment_digest: ContentHash,
    #[model(key)] pub lock_digest: Option<ContentHash>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "provider_runs")]
pub struct ProviderRun {
    #[model(key, provenance)] pub provider: Id<Provider>,
    #[model(key, provenance)] pub context: Id<AnalysisContext>,
    #[model(key)] pub release: Id<Release>,
    #[model(key)] pub configuration: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "syntax_observations")]
pub struct SyntaxObservation {
    #[model(key)] pub occurrence: Id<Occurrence>,
    #[model(key)] pub spelling: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "syntax_supports")]
pub struct SyntaxSupport {
    #[model(key)] pub assertion: Id<SyntaxObservation>,
    #[model(key, provenance)] pub run: Id<ProviderRun>,
    #[model(key, provenance)] pub surface: String,
}

/// One membership manifest. Lowerings never maintain their own inventory.
pub fn model() -> Result<ValidatedModel, ModelError> {
    ValidatedModel::validate(vec![
        Relation::of::<Package>(), Relation::of::<Release>(), Relation::of::<SourceArtifact>(),
        Relation::of::<Module>(), Relation::of::<Occurrence>(), Relation::of::<Provider>(),
        Relation::of::<AnalysisContext>(), Relation::of::<ProviderRun>(),
        Relation::of::<SyntaxObservation>(), Relation::of::<SyntaxSupport>(),
        Relation::of::<CoverageScope>(), Relation::of::<ProviderCoverage>(),
    ])
}

/// Ruff syntax kinds; existing codes retained from the extraction contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum SyntaxKind {
    ModModule = 0,
    ModExpression = 1,
    StmtFunctionDef = 2,
    StmtClassDef = 3,
    StmtReturn = 4,
    StmtDelete = 5,
    StmtTypeAlias = 6,
    StmtAssign = 7,
    StmtAugAssign = 8,
    StmtAnnAssign = 9,
    StmtFor = 10,
    StmtWhile = 11,
    StmtIf = 12,
    StmtWith = 13,
    StmtMatch = 14,
    StmtRaise = 15,
    StmtTry = 16,
    StmtAssert = 17,
    StmtImport = 18,
    StmtImportFrom = 19,
    StmtGlobal = 20,
    StmtNonlocal = 21,
    StmtExpr = 22,
    StmtPass = 23,
    StmtBreak = 24,
    StmtContinue = 25,
    StmtIpyEscapeCommand = 26,
    ExprBoolOp = 27,
    ExprNamed = 28,
    ExprBinOp = 29,
    ExprUnaryOp = 30,
    ExprLambda = 31,
    ExprIf = 32,
    ExprDict = 33,
    ExprSet = 34,
    ExprListComp = 35,
    ExprSetComp = 36,
    ExprDictComp = 37,
    ExprGenerator = 38,
    ExprAwait = 39,
    ExprYield = 40,
    ExprYieldFrom = 41,
    ExprCompare = 42,
    ExprCall = 43,
    ExprFString = 44,
    ExprTString = 45,
    ExprStringLiteral = 46,
    ExprBytesLiteral = 47,
    ExprNumberLiteral = 48,
    ExprBooleanLiteral = 49,
    ExprNoneLiteral = 50,
    ExprEllipsisLiteral = 51,
    ExprAttribute = 52,
    ExprSubscript = 53,
    ExprStarred = 54,
    ExprName = 55,
    ExprList = 56,
    ExprTuple = 57,
    ExprSlice = 58,
    ExprIpyEscapeCommand = 59,
    ExceptHandlerExceptHandler = 60,
    InterpolatedElement = 61,
    InterpolatedStringLiteralElement = 62,
    PatternMatchValue = 63,
    PatternMatchSingleton = 64,
    PatternMatchSequence = 65,
    PatternMatchMapping = 66,
    PatternMatchClass = 67,
    PatternMatchStar = 68,
    PatternMatchAs = 69,
    PatternMatchOr = 70,
    TypeParamTypeVar = 71,
    TypeParamTypeVarTuple = 72,
    TypeParamParamSpec = 73,
    InterpolatedStringFormatSpec = 74,
    PatternArguments = 75,
    PatternKeyword = 76,
    Comprehension = 77,
    Arguments = 78,
    Parameters = 79,
    Parameter = 80,
    ParameterWithDefault = 81,
    Keyword = 82,
    Alias = 83,
    WithItem = 84,
    MatchCase = 85,
    Decorator = 86,
    ElifElseClause = 87,
    TypeParams = 88,
    FString = 89,
    TString = 90,
    StringLiteral = 91,
    BytesLiteral = 92,
    Identifier = 93,
}

/// Coverage attaches to an explicit scope rather than an untyped entity ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "coverage_scopes")]
pub enum CoverageScope {
    #[model(code = 0)] Release { release: Id<Release> },
    #[model(code = 1)] Module { module: Id<Module> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum CoverageStatus {
    CompleteUnderStatedModel = 0,
    Partial = 1,
    NotRequested = 2,
    Unavailable = 3,
    Failed = 4,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "provider_coverage", validate = validate_coverage)]
pub struct ProviderCoverage {
    #[model(key)] pub scope: Id<CoverageScope>,
    #[model(key, provenance)] pub provider: Id<Provider>,
    #[model(key, provenance)] pub context: Id<AnalysisContext>,
    #[model(key)] pub family: String,
    #[model(key, provenance)] pub run: Option<Id<ProviderRun>>,
    pub status: CoverageStatus,
    pub reason: Option<String>,
}
fn validate_coverage(row: &ProviderCoverage) -> Result<(), ModelError> {
    if row.family.is_empty() { return Err(ModelError::Invalid("coverage needs a family".into())); }
    match row.status {
        CoverageStatus::CompleteUnderStatedModel | CoverageStatus::Partial if row.run.is_none() =>
            Err(ModelError::Invalid("attempted coverage needs a run".into())),
        CoverageStatus::NotRequested if row.run.is_some() =>
            Err(ModelError::Invalid("not-requested coverage cannot name a run".into())),
        CoverageStatus::Partial | CoverageStatus::Unavailable | CoverageStatus::Failed if row.reason.as_ref().is_none_or(String::is_empty) =>
            Err(ModelError::Invalid("incomplete coverage needs a reason".into())),
        _ => Ok(()),
    }
}
