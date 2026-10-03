//! Actual parameter definition answers; pytest fixture metadata never claims test execution.
use crate::syntax_records::{Records as Syntax,Spans};
use lctx_model::domain::{*,assertion::{AssertionQualification,Evidence,EvidenceSourceSpanId},diagnostics::*,charged::StateCharge,resources::ResourceBudget,source::SourceArtifact,obligation::ObligationKind};
use pyrefly::state::{state::Transaction,lsp::{DefinitionMetadata,FindPreference}};
use pyrefly_build::handle::Handle;
use pyrefly_python::symbol_kind::SymbolKind;
use ruff_text_size::Ranged;
pub struct Records {pub rows:Vec<NativeParameterDefinitionObservation>,pub evidence:Vec<Evidence>,charge:StateCharge}
fn symbol(kind:SymbolKind)->NativeDefinitionSymbolKind {match kind {SymbolKind::Module=>NativeDefinitionSymbolKind::Module,SymbolKind::Attribute=>NativeDefinitionSymbolKind::Attribute,SymbolKind::Variable=>NativeDefinitionSymbolKind::Variable,SymbolKind::Constant=>NativeDefinitionSymbolKind::Constant,SymbolKind::Parameter=>NativeDefinitionSymbolKind::Parameter,SymbolKind::TypeParameter=>NativeDefinitionSymbolKind::TypeParameter,SymbolKind::TypeAlias=>NativeDefinitionSymbolKind::TypeAlias,SymbolKind::Function=>NativeDefinitionSymbolKind::Function,SymbolKind::Method=>NativeDefinitionSymbolKind::Method,SymbolKind::Class=>NativeDefinitionSymbolKind::Class}}
#[allow(clippy::too_many_arguments)]
pub fn records(transaction:&Transaction<'_>,handle:&Handle,artifact:&SourceArtifact,source:&str,syntax:&Syntax,spans:&Spans,q:&AssertionQualification,native_reason:Option<ObligationKind>,budget:&ResourceBudget)->Result<Records,ModelError>{
    let mut out=Records {rows:vec![],evidence:vec![],charge:StateCharge::new(budget,"native-parameter-definitions")};
    let info=transaction.get_module_info(handle);let ast=transaction.get_ast(handle);
    for parameter in &syntax.parameters {
        let formal=syntax.formals.iter().find(|(wrapper,_)|*wrapper==parameter.parameter).map(|(_,formal)|*formal);
        let range=formal.and_then(|f|spans.range_of(f));
        let offset=range.map_or(0,|r|r.start().to_u32() as i64);
        let correspondence=native_reason.is_none() && info.as_ref().is_some_and(|m|m.contents().as_str()==source) && range.is_some_and(|range|ast.as_ref().is_some_and(|ast|pyrefly_python::ast::Ast::locate_node(ast,range.start()).iter().any(|n|matches!(n,ruff_python_ast::AnyNodeRef::Parameter(p) if p.range()==range))));
        let mut preference=FindPreference::default();preference.disable_style_fallback=true;
        let result=if correspondence {transaction.find_definition(handle,range.unwrap().start(),preference).ok().filter(|items|!items.is_empty())}else{None};
        let count=result.as_ref().map_or(0,|items|items.len());
        if let Some(items)=result {
            for (ordinal,item) in items.into_iter().enumerate(){
                let metadata=match &item.metadata {DefinitionMetadata::Attribute=>NativeDefinitionMetadata::Attribute,DefinitionMetadata::Module=>NativeDefinitionMetadata::Module,DefinitionMetadata::Variable(_)=>NativeDefinitionMetadata::Variable,DefinitionMetadata::VariableOrAttribute(_)=>NativeDefinitionMetadata::VariableOrAttribute};
                let kind=item.metadata.symbol_kind().map(symbol);
                let role=match (metadata,kind) {(NativeDefinitionMetadata::Variable,Some(NativeDefinitionSymbolKind::Function))=>NativeParameterRole::Fixture,(NativeDefinitionMetadata::Variable,Some(NativeDefinitionSymbolKind::Parameter))=>NativeParameterRole::Ordinary,_=>NativeParameterRole::Unknown};
                let r=item.definition_range;let start=r.start().to_usize();let end=r.end().to_usize();
                let target=if info.as_ref().is_some_and(|m|m.path()==item.module.path() && m.name()==item.module.name()) && item.module.contents().as_str()==source && start<=end && end<=source.len() && source.is_char_boundary(start) && source.is_char_boundary(end){let span=Evidence::SourceSpan {source:artifact.id(),start:start as i64,end:end as i64};out.charge.grow(size_of::<Evidence>().saturating_mul(4)+span.heap_bytes())?;let id=EvidenceSourceSpanId::of(&span).unwrap();out.evidence.push(span);Some(id)}else{None};
                let row=NativeParameterDefinitionObservation {qualification:q.id(),parameter:parameter.id(),subject:parameter.parameter,query_offset:offset,ordinal:ordinal as i64,answer_count:count as i64,answer:if count==1 {DefinitionAnswer::Known}else{DefinitionAnswer::Candidate},role,reason:if count==1 {None}else{Some(ObligationKind::MissingEvidence)},metadata:Some(metadata),symbol_kind:kind,target,target_location:if target.is_some(){DiagnosticLocation::Available}else{DiagnosticLocation::Unavailable},target_name:item.display_name};row.validate()?;out.charge.grow(size_of::<NativeParameterDefinitionObservation>().saturating_mul(4)+row.heap_bytes())?;out.rows.push(row);
            }
        } else {
            let row=NativeParameterDefinitionObservation {qualification:q.id(),parameter:parameter.id(),subject:parameter.parameter,query_offset:offset,ordinal:0,answer_count:0,answer:DefinitionAnswer::Unknown,role:NativeParameterRole::Unknown,reason:Some(native_reason.unwrap_or(ObligationKind::MissingEvidence)),metadata:None,symbol_kind:None,target:None,target_location:DiagnosticLocation::Unavailable,target_name:None};row.validate()?;out.charge.grow(size_of::<NativeParameterDefinitionObservation>().saturating_mul(4)+row.heap_bytes())?;out.rows.push(row);
        }
    }
    Ok(out)
}
