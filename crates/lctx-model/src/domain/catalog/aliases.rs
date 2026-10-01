//! Source aliases use an owned module binding and normalized value references, never symbol spelling identity.
use super::{*,build::{CatalogData,CatalogOutput}};
use crate::domain::{resources::ResourceBudget,normalized::Rows,lexical::{BindingEventKind,LexicalScopeKind},assertion::{AssertionQualification,Approximation},attribution::Modality};
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,ModelError> {rows.get(id).ok_or_else(||ModelError::Invalid("catalog alias premise absent".into()))}
fn exact(q:&AssertionQualification,context:Id<attribution::AnalysisContext>)->bool {q.context==context && q.modality==Modality::Definite && q.approximation==Approximation::Exact && q.condition==conditions::Diagram::always().id()}
pub(super) fn expand(data:&CatalogData,out:&mut CatalogOutput,budget:&ResourceBudget)->Result<(),ModelError> {
    // Freeze original roots so derived source links never become new identity authority.
    let mut roots=Rows::new(budget);for row in out.exposures.iter() {roots.insert(row.clone())?;}
    for parent in roots.iter() {
        let public=need(&data.exposures,parent.exposure)?;let name=need(&data.names,public.observation)?;
        let (module,slot)=match data.export_origins.get(name.origin) {
            Some(symbols::ExportOrigin::Traced {module,name:slot,..})=>{let Some(calls::ProviderModule::Acquired {module})=data.provider_modules.get(*module) else {continue};(*module,slot.as_str())},
            Some(symbols::ExportOrigin::Untraced)|None=>(public.access,name.name.as_str()),
        };
        for binding in data.bindings.iter().filter(|r|r.kind==BindingEventKind::Assignment && r.static_branch.is_none()) {
            if !exact(need(&data.qualifications,binding.qualification)?,public.context) {continue;}
            let event=need(&data.binding_events,binding.event)?;if event.name!=slot {continue;}
            let scope=need(&data.lexical_scopes,binding.scope)?;if scope.kind!=LexicalScopeKind::Module {continue;}
            let Some(ownership)=data.ownership.iter().find(|r|r.occurrence==event.site && r.owner==scope.owner && data.refs.get(r.entity)==Some(&EntityRef::Module {module})) else {continue};
            let Some(value)=binding.value else {continue};
            for reference in data.references.iter().filter(|r|r.read==value) {
                if reference.scope!=binding.scope || !exact(need(&data.qualifications,reference.qualification)?,public.context) {continue;}
                for assessment in data.reference_assessments.iter().filter(|r|r.reference==reference.id()) {
                    for candidate in data.reference_candidates.iter().filter(|r|r.assessment==assessment.id()) {
                        let q=need(&data.qualifications,need(&data.lexical_resolutions,candidate.resolution)?.qualification)?;if q.context!=public.context || q.approximation!=Approximation::Exact {continue;}
                        let ReferenceEntityTarget::Binding {entity,..}=need(&data.reference_targets,candidate.target)? else {continue};
                        let alias=out.aliases.insert(CatalogAlias {parent:parent.id(),binding:binding.id(),ownership:ownership.id(),reference:candidate.id(),entity:*entity,basis:CatalogContractBasis::SourceOnlyAlias})?;
                        let derived=out.candidates.insert(CatalogCandidate {exposure:parent.id(),candidate:None,entity:None,path:None,alias:Some(alias)})?;
                        match need(&data.refs,*entity)? {
                            EntityRef::Callable {callable}=>build::callable(data,out,parent.member,derived,*callable,public.context)?,
                            EntityRef::Class {class}=>{let owner=out.classes.insert(CatalogClass {member:parent.member,candidate:derived,class:*class})?;build::constructors(data,out,owner,public.context)?;build::fields(data,out,parent.member,*class,public.context)?;},
                            _=>{}
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
