//! A narrow source/model proof of a bare return of a context entry value. Provider transfer
//! flags nominate an origin; they are neither rewritten nor treated as identity evidence.
use std::collections::{BTreeMap,HashMap,HashSet};
use cpg_schema::behavior::{ExitSitesRow,ValueFlowContributionsRow};
use cpg_schema::codebook::{BindingKind as B,ContextEntryKind,DeclarationKind,ExitSiteKind,
    FlowSink,LexicalScopeKind,SyntaxField as F,SyntaxKind as S};
use cpg_schema::context_protocol::{ModelContextProtocolsRow,SourceContextSitesRow,SourceContextArgumentsRow};
use cpg_schema::context_value::{SourceContextValueIdentitiesRow,identity};
use cpg_schema::id::Id;
use cpg_schema::tables::*;

pub struct Inputs<'a> {
    pub declarations:&'a [DeclarationsRow], pub parameters:&'a [ParameterSyntaxRow],
    pub syntax:&'a [SyntaxNodesRow], pub bindings:&'a [BindingsRow], pub scopes:&'a [ScopesRow],
    pub references:&'a [ReferencesRow], pub resolutions:&'a [ReferenceResolutionsRow],
    pub values:&'a [FlowValuesRow], pub contributions:&'a [ValueFlowContributionsRow],
    pub exits:&'a [ExitSitesRow], pub sites:&'a [SourceContextSitesRow],
    pub arguments:&'a [SourceContextArgumentsRow], pub protocols:&'a [ModelContextProtocolsRow],
}
fn one<T>(mut values:impl Iterator<Item=T>)->Option<T> {
    let value=values.next()?; values.next().is_none().then_some(value)
}

pub fn prove(input:Inputs<'_>)->Vec<SourceContextValueIdentitiesRow> {
    let mut nodes:HashMap<_,Vec<_>>=HashMap::new();
    for n in input.syntax {nodes.entry((n.snapshot_id,n.node_id)).or_default().push(n);}
    let node=|snapshot,id|match nodes.get(&(snapshot,id))?.as_slice() {[n]=>Some(*n),_=>None};
    let mut hazards=HashSet::new();
    for n in input.syntax.iter().filter(|n|matches!(n.kind,S::StmtDelete|S::StmtNonlocal|S::ExprYield|S::ExprYieldFrom)) {
        let mut owner=n.owner_node_id; let mut seen=HashSet::new();
        while let Some(id)=owner {
            if !seen.insert(id) {break;}
            hazards.insert((n.snapshot_id,id));
            owner=one(input.declarations.iter().filter(|d|d.snapshot_id==n.snapshot_id && d.node_id==id))
                .and_then(|d|d.parent_node_id);
        }
    }
    let mut out=BTreeMap::new();
    for contribution in input.contributions {
        let snapshot=contribution.snapshot_id; let function=contribution.function_node_id;
        if contribution.sink_function_node_id!=Some(function) || contribution.captured
            || hazards.contains(&(snapshot,function)) {continue;}
        let Some(parameter)=contribution.parameter_node_id else {continue;};
        let Some(declaration)=one(input.declarations.iter().filter(|d|d.snapshot_id==snapshot && d.node_id==function)) else {continue;};
        if declaration.kind!=DeclarationKind::Function || !declaration.decorators.is_empty() {continue;}
        let Some(formal)=one(input.parameters.iter().filter(|p|p.snapshot_id==snapshot && p.node_id==parameter && p.function_node_id==function)) else {continue;};
        let Some(value)=one(input.values.iter().filter(|v|v.snapshot_id==snapshot && v.fact_id==contribution.flow_value_fact_id)) else {continue;};
        if value.sink!=FlowSink::Return || value.use_id!=contribution.use_id || value.module_node_id!=declaration.module_node_id {continue;}
        let Some(expression)=one(input.syntax.iter().filter(|n|n.snapshot_id==snapshot && n.kind==S::ExprName
            && n.module_node_id==value.module_node_id && n.start_byte==value.sink_start_byte && n.end_byte==value.sink_end_byte)) else {continue;};
        if expression.field!=F::Value || expression.owner_node_id!=Some(function) {continue;}
        let Some(return_node)=node(snapshot,expression.parent_node_id) else {continue;};
        if return_node.kind!=S::StmtReturn || return_node.owner_node_id!=Some(function) {continue;}
        let Some(exit)=one(input.exits.iter().filter(|e|e.snapshot_id==snapshot && e.site_node_id==return_node.node_id
            && e.function_node_id==function && e.kind==ExitSiteKind::Return)) else {continue;};
        let resolve=|expression:&SyntaxNodesRow,kind:B| {
            let reference=one(input.references.iter().filter(|r|r.snapshot_id==snapshot && r.name_node_id==expression.node_id))?;
            let resolution=one(input.resolutions.iter().filter(|r|r.snapshot_id==snapshot && r.reference_id==reference.node_id))?;
            if resolution.captured || resolution.reason.is_some() || resolution.builtin_name.is_some() {return None;}
            let binding=one(input.bindings.iter().filter(|b|b.snapshot_id==snapshot && Some(b.node_id)==resolution.binding_id))?;
            let scope=one(input.scopes.iter().filter(|s|s.snapshot_id==snapshot && s.node_id==binding.scope_id))?;
            if binding.kind!=kind || reference.scope_id!=binding.scope_id || reference.name!=binding.name
                || expression.detail.as_deref()!=Some(&binding.name) || scope.kind!=LexicalScopeKind::Function
                || scope.owner_node_id!=function || scope.module_node_id!=value.module_node_id
                || binding.module_node_id!=value.module_node_id || reference.module_node_id!=value.module_node_id
                || reference.start_byte!=expression.start_byte || reference.end_byte!=expression.end_byte
                || input.bindings.iter().filter(|b|b.snapshot_id==snapshot && b.scope_id==binding.scope_id && b.name==binding.name).count()!=1 {
                return None;
            }
            Some((reference,resolution,binding,scope))
        };
        let Some((reference,resolution,binding,scope))=resolve(expression,B::WithTarget) else {continue;};
        let Some(target)=node(snapshot,binding.site_node_id) else {continue;};
        if target.kind!=S::ExprName || target.field!=F::Target || target.owner_node_id!=Some(function) {continue;}
        let Some(site)=one(input.sites.iter().filter(|s|s.snapshot_id==snapshot && s.function_node_id==function
            && s.item_node_id==target.parent_node_id)) else {continue;};
        if !site.constructor_valid {continue;}
        let Some(protocol)=one(input.protocols.iter().filter(|p|p.snapshot_id==snapshot
            && cpg_schema::context_protocol::binding_id(p)==site.protocol_id)) else {continue;};
        if protocol.entry!=ContextEntryKind::ArgumentOrNone {continue;}
        let Some(argument)=one(input.arguments.iter().filter(|a|a.snapshot_id==snapshot && a.site_id==site.site_id
            && Some(a.argument_fact_id)==site.entry_argument_fact_id)) else {continue;};
        let Some(argument_expression)=one(input.syntax.iter().filter(|n|n.snapshot_id==snapshot
            && n.fact_id==argument.expression_fact_id && n.kind==S::ExprName && n.owner_node_id==Some(function))) else {continue;};
        let Some((arg_ref,arg_resolution,param_binding,param_scope))=resolve(argument_expression,B::Parameter) else {continue;};
        if param_binding.site_node_id!=parameter || param_binding.name!=formal.name || param_scope.node_id!=scope.node_id {continue;}
        // The read occurs inside this exact entered body, never after a completed sibling
        // context or in a nested callable. Completion still owns whether the body is reached.
        let mut current=return_node;let mut seen=HashSet::new();let mut inside=false;
        for _ in 0..128 {
            if !seen.insert(current.node_id) || current.owner_node_id!=Some(function) {break;}
            if current.parent_node_id==site.with_node_id {inside=current.field==F::Body;break;}
            let Some(parent)=node(snapshot,current.parent_node_id) else {break;};current=parent;
        }
        if !inside {continue;}
        let mut certificate=SourceContextValueIdentitiesRow {
            snapshot_id:snapshot,identity_id:Id::ZERO,function_node_id:function,parameter_node_id:parameter,
            parameter_name:formal.name.clone(),source_flow_fact_id:value.fact_id,source_origin_id:contribution.origin_id,
            condition_id:contribution.condition_id,return_site_fact_id:exit.source_fact_id,
            return_region_fact_id:exit.region_fact_id,return_condition_id:exit.condition_id,return_start_byte:return_node.start_byte,
            context_site_id:site.site_id,argument_fact_id:argument.argument_fact_id,argument_expression_fact_id:argument_expression.fact_id,
            argument_reference_fact_id:arg_ref.fact_id,argument_resolution_fact_id:arg_resolution.fact_id,
            parameter_binding_fact_id:param_binding.fact_id,parameter_fact_id:formal.fact_id,target_binding_fact_id:binding.fact_id,
            expression_fact_id:expression.fact_id,reference_fact_id:reference.fact_id,resolution_fact_id:resolution.fact_id,
            scope_fact_id:scope.fact_id,module_node_id:value.module_node_id,start_byte:expression.start_byte,end_byte:expression.end_byte,
        };
        certificate.identity_id=identity(&certificate);out.insert(certificate.identity_id,certificate);
    }
    out.into_values().collect()
}
