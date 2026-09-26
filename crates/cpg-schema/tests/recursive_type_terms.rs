//! Recursive type-term projections preserve set identity even with a cyclic term edge.

use cpg_schema::codebook::{Codebook, DeclarationKind, ParameterKind, TypeRole, TypeTermKind};
use cpg_schema::id::Id;
use datafusion::arrow::array::StringArray;
use datafusion::prelude::SessionContext;

async fn shared_type_rows(cycle: bool) -> Vec<datafusion::arrow::record_batch::RecordBatch> {
    let ctx = SessionContext::new();
    let links = if cycle {
        "SELECT 'root' AS parent_node_id, 'child' AS child_node_id \
         UNION ALL SELECT 'child', 'root'"
    } else {
        "SELECT 'root' AS parent_node_id, 'child' AS child_node_id"
    };
    for statement in [
        "CREATE TABLE parameter_syntax AS SELECT 'p' AS node_id, 'f' AS function_node_id, \
         0 AS ordinal, 0 AS kind".to_owned(),
        "CREATE TABLE provider_node_map AS SELECT 'none' AS node_id, 'm' AS module_node_id, \
         'k' AS function_key WHERE false".to_owned(),
        "CREATE TABLE pysa_functions AS SELECT 'm' AS module_node_id, 'k' AS function_key, \
         CAST(NULL AS VARCHAR) AS defining_class, false AS is_staticmethod WHERE false".to_owned(),
        format!("CREATE TABLE type_observations AS SELECT 'p' AS subject_node_id, \
         'root' AS term_node_id, {} AS role, true AS declared", TypeRole::Parameter.code()),
        format!("CREATE TABLE type_term_args AS {links}"),
        "CREATE TABLE type_class_targets AS SELECT 'child' AS term_node_id, \
         'class' AS class_node_id".to_owned(),
        "CREATE TABLE declarations AS SELECT 'class' AS node_id".to_owned(),
    ] {
        ctx.sql(&statement).await.unwrap().collect().await.unwrap();
    }
    ctx.sql(&cpg_schema::communities::shared_types_sql())
        .await.unwrap().collect().await.unwrap()
}

#[tokio::test]
async fn shared_type_closure_terminates_on_a_cycle_without_changing_membership() {
    let acyclic = shared_type_rows(false).await;
    let cyclic = shared_type_rows(true).await;
    assert_eq!(acyclic, cyclic);
    assert_eq!(cyclic.iter().map(|batch| batch.num_rows()).sum::<usize>(), 1);
    let target = cyclic[0].column(0).as_any().downcast_ref::<StringArray>().unwrap();
    let class = cyclic[0].column(1).as_any().downcast_ref::<StringArray>().unwrap();
    assert_eq!((target.value(0), class.value(0)), ("f", "class"));
}

async fn attribute_rows(cycle: bool) -> Vec<datafusion::arrow::record_batch::RecordBatch> {
    let ctx = SessionContext::new();
    let function = Id([1; 16]);
    let terms = if cycle {
        "SELECT 'root' AS parent_node_id, 'child' AS child_node_id \
         UNION ALL SELECT 'child', 'root'"
    } else {
        "SELECT 'root' AS parent_node_id, 'child' AS child_node_id"
    };
    for statement in [
        format!("CREATE TABLE declarations AS SELECT X'{}' AS node_id, {} AS kind, \
         make_array('deco') AS decorators", function.hex(), DeclarationKind::Function.code()),
        format!("CREATE TABLE parameter_syntax AS SELECT 'p' AS node_id, \
         X'{}' AS function_node_id, 'value' AS name, {} AS kind, 0 AS ordinal",
            function.hex(), ParameterKind::PositionalOrKeyword.code()),
        format!("CREATE TABLE type_terms AS SELECT 'root' AS node_id, {} AS kind, \
         'Root' AS display, '' AS detail UNION ALL SELECT 'child', {}, 'Unknown', 'implicit'",
            TypeTermKind::ClassInstance.code(), TypeTermKind::Any.code()),
        format!("CREATE TABLE type_term_args AS {terms}"),
        format!("CREATE TABLE type_observations AS SELECT 'p' AS subject_node_id, \
         'root' AS term_node_id, {} AS role, true AS declared",
            TypeRole::Parameter.code()),
        "CREATE TABLE provider_node_map AS SELECT 'none' AS node_id, 'm' AS module_node_id, \
         'k' AS function_key WHERE false".to_owned(),
        "CREATE TABLE pysa_functions AS SELECT 'm' AS module_node_id, 'k' AS function_key, \
         CAST(NULL AS VARCHAR) AS defining_class, false AS is_staticmethod WHERE false".to_owned(),
        format!("CREATE TABLE syntax_nodes AS SELECT 'none' AS node_id, \
         X'{}' AS owner_node_id WHERE false", function.hex()),
    ] {
        ctx.sql(&statement).await.unwrap().collect().await.unwrap();
    }
    ctx.sql(&cpg_schema::concepts::attributes_sql(&[function]))
        .await.unwrap().collect().await.unwrap()
}

#[tokio::test]
async fn unknown_type_ancestor_closure_terminates_on_a_cycle_without_relabeling() {
    let acyclic = attribute_rows(false).await;
    let cyclic = attribute_rows(true).await;
    assert_eq!(acyclic, cyclic);
    let labels: Vec<_> = cyclic.iter().flat_map(|batch| {
        let array = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        (0..batch.num_rows()).map(|row| array.value(row).to_owned()).collect::<Vec<_>>()
    }).collect();
    assert_eq!(labels, ["decorator deco", "parameter value"]);
}
