//! An occurrence-specific source proof, independent of provider reaching approximation.
//! It proves only which parameter a bare return read denotes. Entry and exit completion
//! remain separate obligations, and raw provider flags remain unchanged.

use crate::id::{Id, IdHasher};
use crate::table::table;

table!(
    /// One bare return read resolved to the only binding of a parameter in its lexical scope.
    /// Source reconstruction additionally excludes deletion and nonlocal mutation. This is
    /// neither a general alias proof nor proof that the return is reached or survives its frames.
    SourceParameterIdentities, SourceParameterIdentitiesRow = "source_parameter_identities",
    family = Findings,
    key = [snapshot_id, identity_id],
    checks = [("span_order", "start_byte >= 0 AND end_byte >= start_byte")],
    {
        snapshot_id: Id,
        identity_id: Id,
        function_node_id: Id,
        parameter_node_id: Id,
        source_flow_fact_id: Id,
        source_origin_id: Id,
        condition_id: Id,
        return_site_fact_id: Id,
        expression_fact_id: Id,
        reference_fact_id: Id,
        resolution_fact_id: Id,
        binding_fact_id: Id,
        parameter_fact_id: Id,
        module_node_id: Id,
        start_byte: i64,
        end_byte: i64,
    }
);

pub fn identity(row: &SourceParameterIdentitiesRow) -> Id {
    IdHasher::new("source-parameter-identity")
        .id(row.function_node_id)
        .id(row.parameter_node_id)
        .id(row.source_flow_fact_id)
        .id(row.source_origin_id)
        .id(row.condition_id)
        .id(row.return_site_fact_id)
        .id(row.expression_fact_id)
        .id(row.reference_fact_id)
        .id(row.resolution_fact_id)
        .id(row.binding_fact_id)
        .id(row.parameter_fact_id)
        .id(row.module_node_id)
        .i64(row.start_byte)
        .i64(row.end_byte)
        .finish_id()
}

/// Shared source/native scope admission; the source producer/validator owns the lexical proof.
pub fn admits(
    row: &SourceParameterIdentitiesRow,
    function: Id,
    parameter: Id,
    source: Id,
    origin: Id,
    condition: Id,
    return_site: Id,
) -> bool {
    row.identity_id == identity(row)
        && row.function_node_id == function
        && row.parameter_node_id == parameter
        && row.source_flow_fact_id == source
        && row.source_origin_id == origin
        && row.condition_id == condition
        && row.return_site_fact_id == return_site
        && row.start_byte >= 0
        && row.end_byte > row.start_byte
}
