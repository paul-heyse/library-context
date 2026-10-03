//! Selected native source diagnostics and parameter-definition answers, never execution outcomes.
use super::{*, assertion::{AssertionQualification,EvidenceSourceSpanId}, attribution::FactFamily, source::{SourceArtifact,Occurrence}, syntax::ParameterSyntaxObservation, obligation::ObligationKind};
use crate::{Assertion,Domain,DomainCode,DomainSum};
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum DiagnosticSeverity { Ignore=0, Info=1, Warning=2, Error=3, Fatal=4 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum DiagnosticChannel { Emitted=0, RuffNoqaSuppressed=1, PyreflyDirective=2, PyreflySuppressed=3, PyreflyDisabled=4, PyreflyBaseline=5 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum DiagnosticLocation { Available=0, Unavailable=1 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum SelectedRuffRule { UndefinedName=0, UnusedImport=1, UnusedVariable=2 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum NativeBaselineStatus { NotConfigured=0, NotCompared=1, Unmatched=2, Matched=3, NotComparedOrUnmatched=4 }
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name="ruff_diagnostic_observations",validate=validate_ruff)]
#[assertion(support=RuffDiagnosticSupport,name="ruff_diagnostic_supports",family=FactFamily::Lexical,subjects(artifact,primary))]
pub struct RuffDiagnosticObservation {
    #[model(key)] pub qualification:Id<AssertionQualification>,
    #[model(key)] pub artifact:Id<SourceArtifact>,
    #[model(key)] pub ordinal:i64,
    #[model(key)] pub primary:Option<EvidenceSourceSpanId>,
    #[model(key)] pub location:DiagnosticLocation,
    #[model(key)] pub rule:SelectedRuffRule,
    #[model(key)] pub native_id:String,
    #[model(key)] pub native_code:String,
    #[model(key)] pub severity:DiagnosticSeverity,
    #[model(key)] pub channel:DiagnosticChannel,
    #[model(key)] pub message:String,
    /// Digest of actual selected settings and both native noqa passes over the retained parse.
    #[model(key)] pub settings:ContentHash,
}
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name="pyrefly_diagnostic_observations",validate=validate_pyrefly)]
#[assertion(support=PyreflyDiagnosticSupport,name="pyrefly_diagnostic_supports",family=FactFamily::Types,subjects(artifact,primary))]
pub struct PyreflyDiagnosticObservation {
    #[model(key)] pub qualification:Id<AssertionQualification>,
    #[model(key)] pub artifact:Id<SourceArtifact>,
    #[model(key)] pub ordinal:i64,
    #[model(key)] pub primary:Option<EvidenceSourceSpanId>,
    #[model(key)] pub location:DiagnosticLocation,
    /// Exact native ErrorKind::to_name(), rather than a collapsed catalog category.
    #[model(key)] pub category:String,
    #[model(key)] pub severity:DiagnosticSeverity,
    #[model(key)] pub channel:DiagnosticChannel,
    #[model(key)] pub baseline:NativeBaselineStatus,
    #[model(key)] pub header:String,
    #[model(key)] pub details:Option<String>,
}
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="native_diagnostic_subjects")]
pub enum DiagnosticSubject {
    #[model(code=0)] Ruff {observation:Id<RuffDiagnosticObservation>},
    #[model(code=1)] Pyrefly {observation:Id<PyreflyDiagnosticObservation>},
}
/// Secondary annotations inherit their exact native parent; they grant no additional source bytes.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="native_diagnostic_annotations",validate=validate_annotation)]
pub struct DiagnosticAnnotation {
    #[model(key)] pub diagnostic:Id<DiagnosticSubject>,
    #[model(key)] pub ordinal:i64,
    #[model(key)] pub span:Option<EvidenceSourceSpanId>,
    #[model(key)] pub location:DiagnosticLocation,
    #[model(key)] pub label:Option<String>,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum DefinitionAnswer { Known=0, Candidate=1, Unknown=2 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum NativeParameterRole {
    /// Native parameter lookup selected local fixture-function metadata; no execution claim.
    Fixture=0,
    /// Native Variable(Parameter) answer, including fallback when no fixture was retained.
    /// This does not establish that the containing function is outside pytest.
    Ordinary=1,
    Unknown=2,
}
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum NativeDefinitionMetadata { Attribute=0, Module=1, Variable=2, VariableOrAttribute=3 }
#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash,DomainCode)]
#[repr(i16)]
pub enum NativeDefinitionSymbolKind { Module=0, Attribute=1, Variable=2, Constant=3, Parameter=4, TypeParameter=5, TypeAlias=6, Function=7, Method=8, Class=9 }
#[derive(Debug,Clone,PartialEq,Eq,Domain,Assertion)]
#[model(name="native_parameter_definition_observations",validate=validate_parameter)]
#[assertion(support=NativeParameterDefinitionSupport,name="native_parameter_definition_supports",family=FactFamily::Types,subjects(subject,target))]
pub struct NativeParameterDefinitionObservation {
    #[model(key)] pub qualification:Id<AssertionQualification>,
    #[model(key)] pub parameter:Id<ParameterSyntaxObservation>,
    #[model(key)] pub subject:Id<Occurrence>,
    #[model(key)] pub query_offset:i64,
    #[model(key)] pub ordinal:i64,
    #[model(key)] pub answer_count:i64,
    #[model(key)] pub answer:DefinitionAnswer,
    #[model(key)] pub role:NativeParameterRole,
    #[model(key)] pub reason:Option<ObligationKind>,
    #[model(key)] pub metadata:Option<NativeDefinitionMetadata>,
    #[model(key)] pub symbol_kind:Option<NativeDefinitionSymbolKind>,
    #[model(key)] pub target:Option<EvidenceSourceSpanId>,
    #[model(key)] pub target_location:DiagnosticLocation,
    #[model(key)] pub target_name:Option<String>,
}
fn location(span:Option<EvidenceSourceSpanId>,location:DiagnosticLocation)->bool {(location==DiagnosticLocation::Available)==span.is_some()}
fn invalid()->ModelError {ModelError::Invalid("native diagnostic/definition payload differs from its actual availability or channel".into())}
fn validate_ruff(row:&RuffDiagnosticObservation)->Result<(),ModelError>{
    if row.ordinal<0 || !location(row.primary,row.location) || row.native_id.is_empty() || row.native_code.is_empty()
        || !matches!(row.channel,DiagnosticChannel::Emitted|DiagnosticChannel::RuffNoqaSuppressed) || row.severity==DiagnosticSeverity::Ignore {return Err(invalid());} Ok(())
}
fn validate_pyrefly(row:&PyreflyDiagnosticObservation)->Result<(),ModelError>{
    if row.ordinal<0 || !location(row.primary,row.location) || row.category.is_empty() || row.channel==DiagnosticChannel::RuffNoqaSuppressed || row.severity==DiagnosticSeverity::Fatal {return Err(invalid());} Ok(())
}
fn validate_annotation(row:&DiagnosticAnnotation)->Result<(),ModelError>{if row.ordinal<0 || !location(row.span,row.location){return Err(invalid());}Ok(())}
fn validate_parameter(row:&NativeParameterDefinitionObservation)->Result<(),ModelError>{
    if row.ordinal<0 || row.query_offset<0 || row.answer_count<0 || !location(row.target,row.target_location)
        || (row.answer==DefinitionAnswer::Unknown && (row.role!=NativeParameterRole::Unknown || row.reason.is_none() || row.metadata.is_some()))
        || (row.answer==DefinitionAnswer::Known && (row.answer_count!=1 || row.reason.is_some()))
        || (row.answer==DefinitionAnswer::Candidate && (row.answer_count<2 || row.reason.is_none()))
        || (row.role==NativeParameterRole::Fixture && (row.metadata!=Some(NativeDefinitionMetadata::Variable) || row.symbol_kind!=Some(NativeDefinitionSymbolKind::Function)))
        || (row.role==NativeParameterRole::Ordinary && (row.metadata!=Some(NativeDefinitionMetadata::Variable) || row.symbol_kind!=Some(NativeDefinitionSymbolKind::Parameter))) {return Err(invalid());} Ok(())
}
