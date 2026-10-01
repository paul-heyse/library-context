//! Observed test intent follows exact stored target identities and typed execution boundaries.
use super::{*,build::{EvidenceData,need,qualification,exact}};
use crate::domain::{source::SyntaxKind,lexical::SyntaxField,calls::ProviderModule};
fn target(data:&EvidenceData,site:Id<source::Occurrence>,context:Id<attribution::AnalysisContext>,names:&[&str])->Result<bool,ModelError> {
 for raw in data.facts.raw_targets.iter().filter(|r|r.site==site) {if !exact(qualification(data,raw.qualification)?,context) {continue;}let Some(symbol)=need(&data.facts.destinations,raw.destination)?.symbol() else {continue};let symbol=need(&data.core.symbols,symbol)?;if symbol.context!=context || !names.contains(&symbol.name.as_str()) {continue;}
  let module=match need(&data.core.provider_modules,symbol.module)? {ProviderModule::Bundled {provider,name,..} if *provider==symbol.provider=>Some(name.as_str()),ProviderModule::Acquired {module}=>data.core.modules.get(*module).map(|r|r.qualified_name.as_str()),_=>None};
  let qualified=match module {
   Some("pytest"|"_pytest.python_api"|"_pytest.mark.structures")=>matches!(symbol.name.as_str(),"raises"|"skip"|"skipif"|"xfail"),
   Some("unittest"|"unittest.case") if matches!(symbol.name.as_str(),"assertRaises"|"assertRaisesRegex")=>data.facts.symbol_observations.iter().any(|observation|observation.symbol==symbol.id() && data.core.qualifications.get(observation.qualification).is_some_and(|q|exact(q,context)) && observation.parent.is_some_and(|parent|data.core.symbols.get(parent).is_some_and(|class|class.context==context && class.provider==symbol.provider && class.module==symbol.module && class.name=="TestCase" && class.kind==calls::SymbolKind::Class) && data.core.traits.iter().any(|r|r.symbol==symbol.id() && r.defining_class==Some(parent) && data.core.qualifications.get(r.qualification).is_some_and(|q|exact(q,context))))),
   _=>false,
  };
  if qualified && data.core.resolutions.iter().any(|r|r.symbol==symbol.id() && r.context==context && r.status==ResolutionStatus::Resolved && r.entity.is_some()) {return Ok(true);}
 }Ok(false)
}
pub(super) fn classify(data:&EvidenceData,event:&NormalizedCallEvent,base:Intent)->Result<Intent,ModelError> {
 let context=event.context;let mut current=event.site;
 for _ in 0..256 {let mut placements=data.core.placements.iter().filter(|r|r.occurrence==current && data.core.qualifications.get(r.qualification).is_some_and(|q|q.context==context));let Some(edge)=placements.next() else {break};if placements.next().is_some() {return Ok(Intent::Unknown);}let Some(parent)=edge.parent else {break};let parent_kind=need(&data.core.occurrences,parent)?.syntax_kind;
  if parent_kind==SyntaxKind::StmtWith && edge.field==SyntaxField::Body {for item in data.core.placements.iter().filter(|r|r.parent==Some(parent) && r.field==SyntaxField::Item && data.core.qualifications.get(r.qualification).is_some_and(|q|exact(q,context))) {for value in data.core.placements.iter().filter(|r|r.parent==Some(item.occurrence) && r.field==SyntaxField::Value && data.core.qualifications.get(r.qualification).is_some_and(|q|exact(q,context))) {if target(data,value.occurrence,context,&["raises","assertRaises","assertRaisesRegex"])? {return Ok(Intent::ExpectedFailure);}}}}
  if parent_kind==SyntaxKind::StmtFunctionDef && edge.field==SyntaxField::Body {for observation in data.facts.decorators.iter().filter(|r|r.declaration==parent && data.core.qualifications.get(r.qualification).is_some_and(|q|exact(q,context))) {if target(data,observation.decorator,context,&["skip","skipif","xfail"])? {return Ok(Intent::SkipXfail);}for child in data.core.placements.iter().filter(|r|r.parent==Some(observation.decorator)) {if target(data,child.occurrence,context,&["skip","skipif","xfail"])? {return Ok(Intent::SkipXfail);}}}break;}
  if parent_kind==SyntaxKind::ExprLambda && edge.field==SyntaxField::Value {break;}
  if parent_kind==SyntaxKind::ExprGenerator {
   // Only the first comprehension's iterable is eager. Its typed Item/Iter route is required.
   if edge.field!=SyntaxField::Item {break;}let comprehension=edge;
   let first=data.core.placements.iter().filter(|r|r.parent==Some(parent) && r.field==SyntaxField::Item && data.core.qualifications.get(r.qualification).is_some_and(|q|exact(q,context))).min_by_key(|r|r.ordinal);if first.map(Record::id)!=Some(comprehension.id()) {break;}
   // When crossing the clause, current records which clause child contained this site.
   let eager=data.core.placements.iter().any(|r|r.parent==Some(current) && r.field==SyntaxField::Iter && data.core.qualifications.get(r.qualification).is_some_and(|q|exact(q,context)) && contains(data,r.occurrence,event.site).unwrap_or(false));if !eager {break;}
  }
  current=parent;
 }Ok(base)
}
fn contains(data:&EvidenceData,outer:Id<source::Occurrence>,inner:Id<source::Occurrence>)->Result<bool,ModelError> {let outer=need(&data.core.occurrences,outer)?;let inner=need(&data.core.occurrences,inner)?;Ok(outer.source==inner.source && outer.start<=inner.start && inner.end<=outer.end)}
