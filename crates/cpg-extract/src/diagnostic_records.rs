//! Selected native diagnostic payloads retained from the live parse/session.
use lctx_model::domain::{*,assertion::{AssertionQualification,Evidence,EvidenceSourceSpanId},diagnostics::*,charged::StateCharge,resources::ResourceBudget,source::SourceArtifact};
use ruff_db::diagnostic::{Diagnostic,Span,Severity};
use ruff_linter::{Locator,directives,settings::{LinterSettings,TargetVersion,flags::Noqa,rule_table::RuleTable},registry::Rule,source_kind::SourceKind,suppression::Suppressions,package::PackageRoot};
use ruff_python_ast_latest::{ModModule,PySourceType};
use ruff_python_parser_latest::Parsed;
use ruff_python_codegen_latest::Stylist;
use ruff_python_index_latest::Indexer;
use std::collections::BTreeMap;
pub struct Records {pub ruff:Vec<RuffDiagnosticObservation>,pub pyrefly:Vec<PyreflyDiagnosticObservation>,pub subjects:Vec<DiagnosticSubject>,pub annotations:Vec<DiagnosticAnnotation>,pub evidence:Vec<Evidence>,charge:StateCharge}
impl Records {
    pub fn new(budget:&ResourceBudget)->Self {Self {ruff:vec![],pyrefly:vec![],subjects:vec![],annotations:vec![],evidence:vec![],charge:StateCharge::new(budget,"native-selected-diagnostics")}}
    pub(crate) fn hold<R:Record+HeapSize>(&mut self,row:&R)->Result<(),ModelError>{self.charge.grow(size_of::<R>().saturating_mul(4).saturating_add(row.heap_bytes()))}
    pub(crate) fn span(&mut self,artifact:&SourceArtifact,source:&str,start:u32,end:u32)->Result<Option<EvidenceSourceSpanId>,ModelError>{
        let (start,end)=(start as usize,end as usize);if end<start||end>source.len()||!source.is_char_boundary(start)||!source.is_char_boundary(end){return Ok(None);}
        let row=Evidence::SourceSpan {source:artifact.id(),start:start as i64,end:end as i64};let id=EvidenceSourceSpanId::of(&row).unwrap();self.hold(&row)?;self.evidence.push(row);Ok(Some(id))
    }
    fn ruff_span(&mut self,span:&Span,artifact:&SourceArtifact,source:&str,path:&std::path::Path)->Result<Option<EvidenceSourceSpanId>,ModelError>{
        let Some(file)=span.as_ruff_file() else{return Ok(None)};if file.source_text()!=source||std::path::Path::new(file.name())!=path{return Ok(None)}
        let Some(range)=span.range() else{return Ok(None)};self.span(artifact,source,range.start().to_u32(),range.end().to_u32())
    }
}
pub(crate) fn location(span:Option<EvidenceSourceSpanId>)->DiagnosticLocation {if span.is_some(){DiagnosticLocation::Available}else{DiagnosticLocation::Unavailable}}
#[allow(clippy::too_many_arguments)]
pub fn ruff(artifact:&SourceArtifact,source:&str,parsed:&Parsed<ModModule>,native:&ruff_linter::semantic_facts::Settings,q:&AssertionQualification,budget:&ResourceBudget)->Result<Records,ModelError>{
    let mut out=Records::new(budget);
    // All inputs are captured. Never call Default/for_rules, which read Ruff's cached cwd.
    let mut settings=LinterSettings::new_with_src(std::path::Path::new("/captured"),vec!["/captured".into()]);
    settings.rules=RuleTable::from_iter([Rule::UndefinedName,Rule::UnusedImport,Rule::UnusedVariable]);settings.builtins=native.custom_builtins.clone();settings.typing_modules=native.typing_modules.clone();settings.unresolved_target_version=TargetVersion(Some(native.python_version));
    let digest=ContentHash::of(settings.to_string().as_bytes());
    let allowance=source.len().checked_mul(96).and_then(|n|n.checked_add(65536)).ok_or_else(||ModelError::Invalid("selected Ruff diagnostic allowance overflow".into()))?;
    let _native=budget.reserve("selected-ruff-diagnostic-native",allowance)?;
    let locator=Locator::new(source);let stylist=Stylist::from_tokens(parsed.tokens(),source);let indexer=Indexer::from_tokens(parsed.tokens(),source);
    let directives=directives::extract_directives(parsed.tokens(),directives::Flags::from_settings(&settings),&locator,&indexer);
    let suppressions=Suppressions::from_tokens(source,parsed.tokens(),&indexer,&settings);
    let kind=SourceKind::Python {code:source.to_owned(),is_stub:native.source_type==PySourceType::Stub};
    let check=|noqa|ruff_linter::linter::check_path(&native.path,native.package_root.as_deref().map(PackageRoot::root),&locator,&stylist,&indexer,&directives,&settings,noqa,&kind,native.source_type,parsed,settings.unresolved_target_version,&suppressions);
    let emitted=check(Noqa::Enabled);let unsuppressed=check(Noqa::Disabled);
    fn key(d:&Diagnostic)->(String,String,Option<(u32,u32)>,String){(d.id().as_str().into(),d.secondary_code_or_id().into(),d.primary_span().and_then(|s|s.range().map(|r|(r.start().to_u32(),r.end().to_u32()))),d.headline_message().into())}
    let mut remaining=BTreeMap::new();for d in &emitted {*remaining.entry(key(d)).or_insert(0usize)+=1;}
    for d in &unsuppressed {
        let channel=match remaining.get_mut(&key(d)) {Some(count) if *count>0=>{*count-=1;DiagnosticChannel::Emitted},_=>DiagnosticChannel::RuffNoqaSuppressed};
        let rule=match d.secondary_code_or_id(){"F821"=>SelectedRuffRule::UndefinedName,"F401"=>SelectedRuffRule::UnusedImport,"F841"=>SelectedRuffRule::UnusedVariable,_=>continue};
        let primary=match d.primary_span(){Some(span)=>out.ruff_span(&span,artifact,source,&native.path)?,None=>None};
        let severity=match d.severity(){Severity::Info=>DiagnosticSeverity::Info,Severity::Warning=>DiagnosticSeverity::Warning,Severity::Error=>DiagnosticSeverity::Error,Severity::Fatal=>DiagnosticSeverity::Fatal};
        let row=RuffDiagnosticObservation {qualification:q.id(),artifact:artifact.id(),ordinal:out.ruff.len() as i64,primary,location:location(primary),rule,native_id:d.id().as_str().into(),native_code:d.secondary_code_or_id().into(),severity,channel,message:d.headline_message().into(),settings:digest};row.validate()?;out.hold(&row)?;
        let subject=DiagnosticSubject::Ruff {observation:row.id()};out.hold(&subject)?;
        for (ordinal,annotation) in d.secondary_annotations().enumerate(){let span=out.ruff_span(annotation.get_span(),artifact,source,&native.path)?;let a=DiagnosticAnnotation {diagnostic:subject.id(),ordinal:ordinal as i64,span,location:location(span),label:annotation.get_message().map(str::to_owned)};a.validate()?;out.hold(&a)?;out.annotations.push(a);}
        out.subjects.push(subject);out.ruff.push(row);
    }
    // A surviving unmatched enabled result would mean the pair answered different questions.
    if remaining.values().any(|count|*count!=0){return Err(ModelError::Invalid("selected Ruff noqa passes disagree on diagnostic identity".into()));}
    Ok(out)
}
// The public transaction returns private native error types. Expand in its caller so every
// native method is used while the exact live result remains inferred, without a fork exposure.
macro_rules! pyrefly_diagnostics {
    ($artifact:expr,$source:expr,$errors:expr,$module:expr,$q:expr,$budget:expr)=>{{
        use lctx_model::domain::{diagnostics::*,Record};
        use pyrefly_config::error_kind::Severity as S;
        use ruff_text_size::Ranged;
        let (artifact,source,errors,module,q,budget)=($artifact,$source,$errors,$module,$q,$budget);
        let mut out=$crate::diagnostic_records::Records::new(budget);
        for (channel,errors) in [(DiagnosticChannel::Emitted,&errors.ordinary),(DiagnosticChannel::PyreflyDirective,&errors.directives),(DiagnosticChannel::PyreflySuppressed,&errors.suppressed),(DiagnosticChannel::PyreflyDisabled,&errors.disabled),(DiagnosticChannel::PyreflyBaseline,&errors.baseline)]{
            for error in errors {
                let same=module.is_some_and(|module|module.path()==error.module().path() && module.name()==error.module().name()) && error.lined_buffer().contents().as_str()==source;
                let primary=if same {out.span(artifact,source,error.range().start().to_u32(),error.range().end().to_u32())?}else{None};
                let severity=match error.severity(){S::Ignore=>DiagnosticSeverity::Ignore,S::Info=>DiagnosticSeverity::Info,S::Warn=>DiagnosticSeverity::Warning,S::Error=>DiagnosticSeverity::Error};
                // Public native projection distinguishes no configured baseline and a match;
                // false deliberately retains the unresolvable NotCompared/Unmatched distinction.
                let baseline=match error.baseline_status().legacy_baselined_flag(){None=>NativeBaselineStatus::NotConfigured,Some(true)=>NativeBaselineStatus::Matched,Some(false)=>NativeBaselineStatus::NotComparedOrUnmatched};
                let row=PyreflyDiagnosticObservation {qualification:q.id(),artifact:artifact.id(),ordinal:out.pyrefly.len() as i64,primary,location:$crate::diagnostic_records::location(primary),category:error.error_kind().to_name().into(),severity,channel,baseline,header:error.msg_header().into(),details:error.msg_details().map(str::to_owned)};row.validate()?;out.hold(&row)?;
                let subject=DiagnosticSubject::Pyrefly {observation:row.id()};out.hold(&subject)?;
                for (ordinal,annotation) in error.secondary_annotations().iter().enumerate(){let span=if same {out.span(artifact,source,annotation.range.start().to_u32(),annotation.range.end().to_u32())?}else{None};let a=DiagnosticAnnotation {diagnostic:subject.id(),ordinal:ordinal as i64,span,location:$crate::diagnostic_records::location(span),label:Some(annotation.label.to_string())};a.validate()?;out.hold(&a)?;out.annotations.push(a);}
                out.subjects.push(subject);out.pyrefly.push(row);
            }
        }
        out
    }};
}
pub(crate) use pyrefly_diagnostics;
