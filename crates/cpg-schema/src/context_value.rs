//! Identity of a context entry result, independent of the manager and raw provider transfer.
//! The source producer proves lexical binding. Completion separately proves that entry and
//! assignment happened and that the return survives cleanup.
use crate::context_protocol::{SourceContextSitesRow,SourceContextArgumentsRow};
use crate::id::{Id, IdHasher, recipe::SummaryFlowProofStep};
use crate::codebook::SummaryFlowStepKind as K;
use crate::table::table;

table!(
    SourceContextValueIdentities, SourceContextValueIdentitiesRow = "source_context_value_identities",
    family = Findings,
    key = [snapshot_id, identity_id],
    checks = [("span_order", "return_start_byte >= 0 AND end_byte > start_byte AND start_byte >= return_start_byte")],
    {
        snapshot_id: Id,
        identity_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        parameter_name: String,
        source_flow_fact_id: Id,
        source_origin_id: Id,
        condition_id: Id,
        return_site_fact_id: Id,
        return_region_fact_id: Id,
        return_condition_id: Id,
        return_start_byte: i64,
        context_site_id: Id,
        argument_fact_id: Id,
        argument_expression_fact_id: Id,
        argument_reference_fact_id: Id,
        argument_resolution_fact_id: Id,
        parameter_binding_fact_id: Id,
        parameter_fact_id: Id,
        target_binding_fact_id: Id,
        expression_fact_id: Id,
        reference_fact_id: Id,
        resolution_fact_id: Id,
        scope_fact_id: Id,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
    }
);

pub fn identity(row:&SourceContextValueIdentitiesRow)->Id {
    IdHasher::new("source-context-value-identity")
        .id(row.function_node_id).id(row.parameter_node_id).str(&row.parameter_name)
        .id(row.source_flow_fact_id).id(row.source_origin_id).id(row.condition_id)
        .id(row.return_site_fact_id).id(row.return_region_fact_id).id(row.return_condition_id)
        .i64(row.return_start_byte).id(row.context_site_id).id(row.argument_fact_id)
        .id(row.argument_expression_fact_id).id(row.argument_reference_fact_id)
        .id(row.argument_resolution_fact_id).id(row.parameter_binding_fact_id).id(row.parameter_fact_id)
        .id(row.target_binding_fact_id).id(row.expression_fact_id).id(row.reference_fact_id)
        .id(row.resolution_fact_id).id(row.scope_fact_id).id(row.module_node_id)
        .i64(row.start_byte).i64(row.end_byte).finish_id()
}

/// The immutable certificate is source-reconstructed before publication. Consumers must also
/// match the source origin, actual entry argument and successful lifecycle in this return proof.
pub fn admits(row:&SourceContextValueIdentitiesRow, site:&SourceContextSitesRow, argument:&SourceContextArgumentsRow,
    function:Id,parameter:Id,source:Id,origin:Id,condition:Id,return_site:Id,
    proof:&[SummaryFlowProofStep])->bool {
    let positions=|kind,evidence|proof.iter().enumerate().filter_map(|(i,s)|
        (s.kind==kind && s.evidence_id==evidence).then_some(i)).collect::<Vec<_>>();
    let entry=positions(K::ContextEntry,site.site_id);
    let value=positions(K::ContextEntryValueIdentity,row.identity_id);
    let exit=positions(K::ContextExit,site.site_id);
    let anchors=positions(K::ReturnExit,row.return_site_fact_id);
    let [anchor]=anchors.as_slice() else {return false;};
    let ([entry],[value],[exit])=(entry.as_slice(),value.as_slice(),exit.as_slice()) else {return false;};
    row.identity_id==identity(row) && row.function_node_id==function && row.parameter_node_id==parameter
        && row.source_flow_fact_id==source && row.source_origin_id==origin && row.condition_id==condition
        && row.return_site_fact_id==return_site && row.context_site_id==site.site_id
        && site.function_node_id==function && site.constructor_valid
        && site.entry_argument_fact_id==Some(row.argument_fact_id)
        && argument.snapshot_id==row.snapshot_id && argument.site_id==site.site_id
        && argument.argument_fact_id==row.argument_fact_id && argument.expression_fact_id==row.argument_expression_fact_id
        && row.snapshot_id==site.snapshot_id
        && row.start_byte>=row.return_start_byte && row.end_byte>row.start_byte && row.return_start_byte>=0
        && !row.parameter_name.is_empty()
        && entry<value && value<exit && exit<anchor && proof[*anchor].condition_id==row.return_condition_id
        && proof[*value].condition_id==condition
        && proof[entry+1..*value].iter().any(|s|s.kind==K::LocalAssignmentBinding && s.evidence_id==row.target_binding_fact_id)
}


#[cfg(test)]
mod tests {
    use super::*;
    fn id(n:u8)->Id {Id([n;16])}
    #[test]
    fn entry_value_requires_the_active_site_and_its_return_scope() {
        // Site/argument admission is a separate prerequisite; this tests the value witness
        // against those admitted views, including the previously completed sibling trap.
        let site=SourceContextSitesRow {snapshot_id:id(1),site_id:id(30),function_node_id:id(2),
            with_node_id:id(31),with_fact_id:id(32),item_node_id:id(33),item_fact_id:id(34),item_ordinal:0,
            call_node_id:id(35),call_fact_id:id(36),expression_fact_id:id(37),protocol_id:id(38),
            model_id:id(39),class_node_id:id(40),reference_fact_id:id(41),resolution_fact_id:id(42),
            import_binding_fact_id:id(43),import_region_fact_id:id(44),import_condition_id:id(45),
            export_fact_id:id(46),allocation_call_fact_id:id(47),initialization_call_fact_id:id(48),
            constructor_valid:true,entry_argument_fact_id:Some(id(12))};
        let argument=SourceContextArgumentsRow {snapshot_id:id(1),site_id:id(30),ordinal:0,
            argument_fact_id:id(12),expression_fact_id:id(13),parameter_fact_id:Some(id(49)),
            exception_class_node_id:None,exception_class_fact_id:None,exception_module_fact_id:None,
            reference_fact_id:None,resolution_fact_id:None};
        let mut row=SourceContextValueIdentitiesRow {snapshot_id:id(1),identity_id:Id::ZERO,
            function_node_id:id(2),parameter_node_id:id(3),parameter_name:"value".into(),
            source_flow_fact_id:id(4),source_origin_id:id(5),condition_id:id(6),return_site_fact_id:id(7),
            return_region_fact_id:id(8),return_condition_id:id(9),return_start_byte:100,context_site_id:id(30),
            argument_fact_id:id(12),argument_expression_fact_id:id(13),argument_reference_fact_id:id(14),
            argument_resolution_fact_id:id(15),parameter_binding_fact_id:id(16),parameter_fact_id:id(17),
            target_binding_fact_id:id(18),expression_fact_id:id(19),reference_fact_id:id(20),
            resolution_fact_id:id(21),scope_fact_id:id(22),module_node_id:id(23),start_byte:107,end_byte:113};
        row.identity_id=identity(&row);
        let step=|kind,evidence_id,condition_id|SummaryFlowProofStep {kind,evidence_id,condition_id};
        let proof=vec![step(K::ContextEntry,site.site_id,id(6)),step(K::LocalAssignmentBinding,id(18),id(6)),
            step(K::ContextEntryValueIdentity,row.identity_id,id(6)),step(K::ContextExit,site.site_id,id(9)),
            step(K::ReturnExit,id(7),id(9))];
        let check=|proof:&[_]|admits(&row,&site,&argument,id(2),id(3),id(4),id(5),id(6),id(7),proof);
        assert!(check(&proof));
        let mut completed=proof.clone();completed.swap(2,3);assert!(!check(&completed));
        let mut unassigned=proof.clone();unassigned.swap(1,2);assert!(!check(&unassigned));
        let mut wrong=proof.clone();wrong[4].condition_id=id(6);assert!(!check(&wrong));
        let mut missing=proof.clone();missing.remove(2);assert!(!check(&missing));
        let mut twice=proof.clone();twice.insert(2,proof[2].clone());assert!(!check(&twice));
        let mut foreign=argument.clone();foreign.expression_fact_id=id(50);
        assert!(!admits(&row,&site,&foreign,id(2),id(3),id(4),id(5),id(6),id(7),&proof));
    }
}
