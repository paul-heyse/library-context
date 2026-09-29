//! Source and attributed syntax facts. Identity never depends on a normalized L1 entity.
use crate::{Domain, DomainCode, DomainSum};
use super::{ContentHash, EvidenceBytes, Id, ModelError, Record};
use super::input::{InputRevision, Release};
use super::attribution::ProviderRun;

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "source_artifacts", validate = validate_source)]
pub struct SourceArtifact {
    #[model(key)] pub input: Id<InputRevision>,
    #[model(key)] pub path: String,
    #[model(key)] pub content: ContentHash,
    pub byte_len: i64,
    pub body: EvidenceBytes,
}
impl SourceArtifact {
    pub fn new(input: Id<InputRevision>, path: String, body: Vec<u8>) -> Result<Self, ModelError> {
        let row = Self { input, path, content: ContentHash::of(&body),
            byte_len: i64::try_from(body.len()).map_err(|_| ModelError::Invalid("artifact too large".into()))?, body: EvidenceBytes(body) };
        row.validate()?;
        Ok(row)
    }
    pub fn text(&self) -> Result<&str, std::str::Utf8Error> { std::str::from_utf8(&self.body.0) }
    pub fn is_stub(&self) -> bool { self.path.ends_with(".pyi") }
    pub fn is_package(&self) -> bool { matches!(self.path.rsplit('/').next(), Some("__init__.py" | "__init__.pyi")) }
    pub fn manifest_entry(&self) -> super::input::ManifestEntry {
        super::input::ManifestEntry { path: self.path.clone(), content: self.content, byte_len: self.byte_len }
    }
}
fn validate_source(row: &SourceArtifact) -> Result<(), ModelError> {
    super::input::validate_path(&row.path)?;
    if usize::try_from(row.byte_len).ok() != Some(row.body.0.len()) || row.content != ContentHash::of(&row.body.0) {
        return Err(ModelError::Invalid("artifact bytes differ from length or content digest".into()));
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
#[model(name = "occurrences", validate = validate_occurrence, invariants = occurrence_invariants)]
pub struct Occurrence {
    #[model(key)] pub source: Id<SourceArtifact>,
    #[model(key)] pub start: i64,
    #[model(key)] pub end: i64,
    #[model(key)] pub syntax_kind: SyntaxKind,
    /// Structural role distinguishes events sharing a span (e.g. individual with-items).
    #[model(key)] pub role: OccurrenceRole,
    #[model(key)] pub structural_path: Vec<i32>,
}
fn validate_occurrence(row: &Occurrence) -> Result<(), ModelError> {
    if row.start < 0 || row.end < row.start || row.structural_path.iter().any(|index| *index < 0) {
        return Err(ModelError::Invalid("invalid occurrence span/kind/role".into()));
    }
    Ok(())
}
/// A source event's role is semantic data, not a caller-constructed string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum OccurrenceRole {
    Syntax = 0, Declaration = 1, Parameter = 2, Binding = 3, Read = 4,
    Call = 5, Argument = 6, Predicate = 7, WithItem = 8, Decorator = 9,
    Return = 10, Yield = 11, Raise = 12,
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
    #[model(code = 2)] Input { input: Id<InputRevision> },
    #[model(code = 3)] Artifact { artifact: Id<SourceArtifact> },
}


fn occurrence_invariants() -> Vec<super::Invariant> {
    vec![super::Invariant { name: "occurrence_source_bounds", inputs: vec![
        super::ValidationInput::of::<SourceArtifact>(&["id"]),
        super::ValidationInput::of::<Occurrence>(&["source", "start", "end"]),
    ], create: || Box::new(OccurrenceBounds { lengths: Default::default() }) }]
}
struct OccurrenceBounds { lengths: std::collections::BTreeMap<Id<SourceArtifact>, i64> }
impl super::InvariantCheck for OccurrenceBounds {
    fn visit(&mut self, relation: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if relation == SourceArtifact::NAME {
            for source in SourceArtifact::decode(batch)? {
                if self.lengths.len() >= 1_000_000 { return Err(ModelError::Invalid("source validation cardinality budget exceeded".into())); }
                if self.lengths.insert(source.id(), source.byte_len).is_some() { return Err(ModelError::Conflict(SourceArtifact::NAME)); }
            }
        } else if relation == Occurrence::NAME {
            for occurrence in Occurrence::decode(batch)? {
                if !self.lengths.get(&occurrence.source).is_some_and(|length| occurrence.end <= *length) {
                    return Err(ModelError::Invalid("occurrence is outside its source bytes".into()));
                }
            }
        } else { return Err(ModelError::Invalid("undeclared span validation input".into())); }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> { Ok(()) }
}
