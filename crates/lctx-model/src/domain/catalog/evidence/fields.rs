//! Sourced field candidates require owning declaration identity and exact attribute member syntax.
use super::{*,build::{EvidenceData,EvidenceOutput,need,qualification,exact}};
use crate::domain::{source::SyntaxKind,lexical::SyntaxField};
pub(super) fn derive(data:&EvidenceData,out:&mut EvidenceOutput)->Result<(),ModelError> {
 for option in data.catalog.options.iter() {
  let Some(CatalogOptionSubject::Field {field})=data.catalog.subjects.get(option.subject) else {continue};let field=need(&data.core.fields,*field)?;
  let Some(ClassEntity::Source {declaration:class})=data.core.source_classes.get(field.class) else {continue};
  for owner in data.core.ownership.iter() {
   let EntityRef::Callable {callable}=need(&data.core.refs,owner.entity)? else {continue};let Some(CallableEntity::Source {declaration,..})=data.core.source_callables.get(*callable) else {continue};
   for method in data.core.declarations.iter().filter(|r|r.declaration==*declaration && r.parent==Some(*class)) {
    let q=qualification(data,method.qualification)?;let member=need(&data.catalog.members,option.member)?;if !data.facts.core_links.iter().any(|link|link.member==option.member && data.facts.core_invocations.get(link.invocation).is_some_and(|i|i.input==member.input && i.context==q.context)) {continue;}let attribute=need(&data.core.occurrences,owner.occurrence)?;if attribute.syntax_kind!=SyntaxKind::ExprAttribute {continue;}
    if !exact(q,q.context) {continue;}
    let constructor=data.catalog.constructors.iter().find(|r|data.catalog.callables.get(r.callable).and_then(|c|data.core.assessments.get(c.assessment)).is_some_and(|a|a.callable==*callable && a.context==q.context) && data.catalog.classes.get(r.class).is_some_and(|c|c.class==field.class)).map(Record::id);
    let mut named=false;for placement in data.core.placements.iter().filter(|r|r.parent==Some(attribute.id())) {if !exact(qualification(data,placement.qualification)?,q.context) || need(&data.core.occurrences,placement.occurrence)?.syntax_kind!=SyntaxKind::Identifier {continue;}
     for spelling in data.facts.spellings.iter().filter(|r|r.occurrence==placement.occurrence) {if exact(qualification(data,spelling.qualification)?,q.context) && spelling.spelling.as_str()==&*field.name {named=true;}}
    }if !named {continue;}
    let kind=access_kind(data,attribute.id(),q.context)?;let phase=if constructor.is_some() && matches!(kind,FieldAccessKind::Write|FieldAccessKind::Augment) {calls::CallPhase::Init}else {calls::CallPhase::Call};let mut found=false;
    for reference in data.core.references.iter().filter(|r|r.parent==attribute.id() && r.field==SyntaxField::Value) {if !exact(qualification(data,reference.qualification)?,q.context) {continue;}for assessment in data.core.reference_assessments.iter().filter(|r|r.reference==reference.id()) {for candidate in data.core.reference_candidates.iter().filter(|r|r.assessment==assessment.id()) {let raw=need(&data.core.lexical_resolutions,candidate.resolution)?;if qualification(data,raw.qualification)?.context!=q.context {continue;}out.accesses.insert(FieldAccessAssessment {option:option.id(),occurrence:attribute.id(),qualification:reference.qualification,owner:owner.id(),receiver:Some(candidate.id()),field:field.id(),constructor,kind,phase,basis:AssociationBasis::SourceFieldCandidate,applicability:Knowledge::Unknown})?;found=true;}}}
    if !found {out.accesses.insert(FieldAccessAssessment {option:option.id(),occurrence:attribute.id(),qualification:method.qualification,owner:owner.id(),receiver:None,field:field.id(),constructor,kind,phase,basis:AssociationBasis::SourceFieldCandidate,applicability:Knowledge::Unknown})?;}
   }
  }
 }Ok(())
}
fn access_kind(data:&EvidenceData,site:Id<source::Occurrence>,context:Id<attribution::AnalysisContext>)->Result<FieldAccessKind,ModelError> {
 let mut current=site;for _ in 0..256 {let mut placements=data.core.placements.iter().filter(|r|r.occurrence==current && data.core.qualifications.get(r.qualification).is_some_and(|q|q.context==context));let Some(placement)=placements.next() else {break};if placements.next().is_some() {break;}let Some(parent)=placement.parent else {break};let kind=need(&data.core.occurrences,parent)?.syntax_kind;
  if placement.field==SyntaxField::Target {return Ok(match kind {SyntaxKind::StmtAssign|SyntaxKind::StmtAnnAssign=>FieldAccessKind::Write,SyntaxKind::StmtAugAssign=>FieldAccessKind::Augment,SyntaxKind::StmtDelete=>FieldAccessKind::Delete,_=>FieldAccessKind::Read});}
  if kind!=SyntaxKind::ExprTuple && kind!=SyntaxKind::ExprList {break;}current=parent;
 }Ok(FieldAccessKind::Read)
}
