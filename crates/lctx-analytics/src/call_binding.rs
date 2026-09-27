//! Source-call binding and fresh definition-time default certificates. Pure source inputs;
//! no default expression is re-evaluated at invocation and no argument is fabricated.
use std::collections::{BTreeMap, HashMap};
use cpg_schema::behavior::{ExpressionEvaluationsRow, StatementCompletionsRow, StatementCompletionStepsRow};
use cpg_schema::codebook::{BoundaryReason as R, BindingKind, CompletionKind, DeclarationKind,
    ParameterKind, SummaryFlowStepKind as K, SyntaxField as F, SyntaxKind as S};
use cpg_schema::id::Id;
use cpg_schema::summary_contract::{LocalCallArgument, SignatureParameter, bind_arguments};
use cpg_schema::tables::{ArgumentsRow, BindingsRow, DeclarationsRow, ParameterSyntaxRow,
    ReferencesRow, ReferenceResolutionsRow, SyntaxNodesRow};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Request { pub snapshot:Id, pub caller:Id, pub call_node:Id, pub call_fact:Id, pub callee:Id }

#[derive(Clone, Debug, Default)]
pub struct BoundCall {
    pub defaults: Vec<DefaultValue>,
    pub proof: Vec<(K,Id)>,
}

#[derive(Clone, Debug)]
pub struct DefaultValue { pub formal_node_id:Id, pub boolean_value:Option<bool> }

#[derive(Clone, Debug)]
pub struct SourceCallBinding {
    pub call_fact_id:Id,
    pub callee_node_id:Id,
    pub result:Result<BoundCall,R>,
}

pub struct Inputs<'a> {
    pub requests:&'a [Request],
    pub mappings:&'a [LocalCallArgument],
    pub syntax:&'a [SyntaxNodesRow],
    pub declarations:&'a [DeclarationsRow],
    pub parameters:&'a [ParameterSyntaxRow],
    pub arguments:&'a [ArgumentsRow],
    pub bindings:&'a [BindingsRow],
    pub references:&'a [ReferencesRow],
    pub resolutions:&'a [ReferenceResolutionsRow],
    pub statements:&'a [StatementCompletionsRow],
    pub statement_steps:&'a [StatementCompletionStepsRow],
    pub expressions:&'a [ExpressionEvaluationsRow],
}

type Groups<'a,T> = HashMap<(Id,Id),Vec<&'a T>>;
fn grouped<T>(rows:&[T],key:impl Fn(&T)->(Id,Id))->Groups<'_,T> {
    let mut out:Groups<'_,T>=HashMap::new();
    for row in rows {out.entry(key(row)).or_default().push(row);}
    out
}
fn one<'a,T>(rows:Option<&Vec<&'a T>>)->Result<&'a T,R> {
    match rows.map(Vec::as_slice) {Some([row])=>Ok(*row),_=>Err(R::MissingEvidence)}
}

pub fn bind(inputs:Inputs<'_>)->Vec<SourceCallBinding> {
    let nodes=grouped(inputs.syntax,|r|(r.snapshot_id,r.node_id));
    let children=grouped(inputs.syntax,|r|(r.snapshot_id,r.parent_node_id));
    let owned=grouped(inputs.syntax,|r|(r.snapshot_id,r.owner_node_id.unwrap_or(r.module_node_id)));
    let declarations=grouped(inputs.declarations,|r|(r.snapshot_id,r.node_id));
    let parameters=grouped(inputs.parameters,|r|(r.snapshot_id,r.function_node_id));
    let arguments=grouped(inputs.arguments,|r|(r.snapshot_id,r.call_node_id));
    let bindings=grouped(inputs.bindings,|r|(r.snapshot_id,r.site_node_id));
    let references=grouped(inputs.references,|r|(r.snapshot_id,r.node_id));
    let resolutions=grouped(inputs.resolutions,|r|(r.snapshot_id,r.binding_id.unwrap_or(r.reference_id)));
    let statements=grouped(inputs.statements,|r|(r.snapshot_id,r.source_fact_id));
    let steps=grouped(inputs.statement_steps,|r|(r.snapshot_id,r.source_fact_id));
    let expressions=grouped(inputs.expressions,|r|(r.snapshot_id,r.syntax_fact_id));
    let mappings=grouped(inputs.mappings,|r|(r.call_fact_id,r.callee_node_id));
    let mut requests:BTreeMap<_,Vec<_>>=BTreeMap::new();
    for request in inputs.requests {
        requests.entry((request.snapshot,request.call_fact,request.callee)).or_default().push(request);
    }
    requests.into_values().map(|requests| {
        let req=*requests[0];
        let result=(|| {
            if requests.iter().any(|other|**other!=req) {return Err(R::MissingEvidence);}
            let params=parameters.get(&(req.snapshot,req.callee)).map_or(&[][..],Vec::as_slice);
            let signatures:Vec<_>=params.iter().map(|p|SignatureParameter {evidence_id:p.fact_id,
                ordinal:p.ordinal,name:p.name.clone(),kind:p.kind,required:p.default_text.is_none()
                    && !matches!(p.kind,ParameterKind::VarPositional|ParameterKind::VarKeyword)}).collect();
            let args:Vec<_>=arguments.get(&(req.snapshot,req.call_node)).into_iter().flatten().map(|a|(*a).clone()).collect();
            let bound=bind_arguments(&signatures,&args)?;
            let mapped=mappings.get(&(req.call_fact,req.callee)).map_or(&[][..],Vec::as_slice);
            if mapped.len()!=bound.explicit.len() {return Err(R::MissingEvidence);}
            let mut out=BoundCall::default();
            for binding in &bound.explicit {
                let formal=params.iter().find(|p|p.fact_id==binding.parameter_fact_id).ok_or(R::MissingEvidence)?;
                let rows:Vec<_>=mapped.iter().filter(|a|a.argument_fact_id==binding.argument_fact_id).collect();
                if rows.len()!=1 || rows[0].formal_node_id!=Some(formal.node_id) {return Err(R::MissingEvidence);}
                out.proof.push((K::ParameterBinding,formal.fact_id));
            }
            if bound.defaults.is_empty() {return Ok(out);}
            // An immediate sole direct call to a fresh nested function is the initial closed
            // stability domain. Module functions, aliases, decorators, recursion and intervening
            // actions do not acquire a default promise from their source signature.
            let declaration=one(declarations.get(&(req.snapshot,req.callee)))?;
            if declaration.kind!=DeclarationKind::Function || declaration.parent_node_id!=Some(req.caller)
                || !declaration.decorators.is_empty() {return Err(R::DefaultStabilityUnknown);}
            let header=one(nodes.get(&(req.snapshot,req.callee)))?;
            let call=one(nodes.get(&(req.snapshot,req.call_node)))?;
            let returned=one(nodes.get(&(req.snapshot,call.parent_node_id)))?;
            if header.kind!=S::StmtFunctionDef || header.parent_node_id!=req.caller || header.field!=F::Body
                || header.owner_node_id!=Some(req.caller) || returned.kind!=S::StmtReturn
                || returned.parent_node_id!=req.caller || returned.field!=F::Body
                || returned.ordinal!=header.ordinal+1 || call.field!=F::Value
                || call.owner_node_id!=Some(req.caller) || call.kind!=S::ExprCall {
                return Err(R::DefaultStabilityUnknown);
            }
            let binding=one(bindings.get(&(req.snapshot,req.callee)))?;
            if binding.kind!=BindingKind::FunctionDef {return Err(R::DefaultStabilityUnknown);}
            let resolution=one(resolutions.get(&(req.snapshot,binding.node_id)))
                .map_err(|_|R::DefaultStabilityUnknown)?;
            let reference=one(references.get(&(req.snapshot,resolution.reference_id)))?;
            if resolution.reason.is_some() || reference.parent_node_id!=req.call_node || reference.field!=F::Callee {
                return Err(R::DefaultStabilityUnknown);
            }
            // Normal expression evaluation alone does not imply no effects. Refuse any nested
            // call or assignment in the immediate invocation's arguments, even a total call.
            if owned.get(&(req.snapshot,req.caller)).into_iter().flatten().any(|n|
                n.start_byte>=call.start_byte && n.end_byte<=call.end_byte && n.node_id!=call.node_id
                    && matches!(n.kind,S::ExprCall|S::ExprNamed)) {return Err(R::DefaultStabilityUnknown);}
            let completed=one(statements.get(&(req.snapshot,header.fact_id)))?;
            if completed.kind!=CompletionKind::Normal || completed.reason.is_some() {
                return Err(completed.reason.filter(|r|matches!(r,R::ExpressionDepthLimit|R::ExpressionWorkLimit
                    |R::CompletionDepthLimit|R::CompletionWorkLimit)).unwrap_or(R::DefaultUnavailable));
            }
            let mut header_steps=steps.get(&(req.snapshot,header.fact_id)).cloned().ok_or(R::MissingEvidence)?;
            header_steps.sort_by_key(|s|s.ordinal);
            if header_steps.is_empty() || header_steps.iter().enumerate().any(|(i,s)|s.ordinal!=i as i64) {
                return Err(R::MissingEvidence);
            }
            // Reuse evidence of creation as availability, never as another invocation-time
            // expression evaluation. Ordered evaluation already belongs to the entry proof.
            out.proof.extend(header_steps.iter().map(|s|(K::DefaultAvailabilityEvidence,s.evidence_id)));
            for evidence in [header.fact_id,returned.fact_id,call.fact_id,binding.fact_id,reference.fact_id,resolution.fact_id] {
                out.proof.push((K::DefaultStabilityEvidence,evidence));
            }
            for required in bound.defaults {
                let formal=params.iter().find(|p|p.fact_id==required.parameter_fact_id).ok_or(R::MissingEvidence)?;
                let defaults:Vec<_>=children.get(&(req.snapshot,header.node_id)).into_iter().flatten()
                    .filter(|n|n.field==F::Default && Some(n.start_byte)==formal.default_start_byte
                        && Some(n.end_byte)==formal.default_end_byte).copied().collect();
                let [default]=defaults.as_slice() else {return Err(R::DefaultUnavailable);};
                let value=one(expressions.get(&(req.snapshot,default.fact_id)))?;
                if !value.normal {return Err(value.reason.unwrap_or(R::DefaultUnavailable));}
                out.defaults.push(DefaultValue {formal_node_id:formal.node_id,boolean_value:value.boolean_value});
                out.proof.extend([(K::DefaultAvailabilityEvidence,formal.fact_id),(K::DefaultValueEvidence,default.fact_id)]);
            }
            Ok(out)
        })();
        SourceCallBinding {call_fact_id:req.call_fact,callee_node_id:req.callee,result}
    }).collect()
}
