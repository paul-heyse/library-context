//! `public_paths`' refusal past an unresolved base (the holistic assessment's A1): no snapshot the
//! compiler validates holds one (the pilot has none; a named base Stage C cannot map is the only
//! source), so the relation runs over a hand-built session holding only the columns it reads.

use datafusion::common::ScalarValue;
use datafusion::prelude::SessionContext;

use cpg_schema::codebook::{
    AncestryRelation, BindingKind, Codebook, DeclarationKind, LexicalScopeKind,
};

async fn session() -> SessionContext {
    let ctx = SessionContext::new();
    let class = DeclarationKind::Class.code();
    let function = DeclarationKind::Function.code();
    let mro = AncestryRelation::Mro.code();
    let statements = [
        "CREATE TABLE exports AS SELECT * FROM (VALUES \
           ('s', 'pub.B', 'b', 'xb'), ('s', 'pub.C', 'c', 'xc'), ('s', 'pub.D', 'd', 'xd')) \
         AS t(snapshot_id, access_path, declaration_node_id, export_node_id)"
            .to_owned(),
        format!(
            "CREATE TABLE declarations AS SELECT * FROM (VALUES \
               ('b', 'm', {class}, CAST(NULL AS VARCHAR), 'B', false, 0), \
               ('c', 'm', {class}, NULL, 'C', false, 10), \
               ('d', 'm', {class}, NULL, 'D', false, 20), \
               ('b.run', 'm', {function}, 'b', 'run', false, 1), \
               ('c.own', 'm', {function}, 'c', 'own', false, 11)) \
             AS t(node_id, module_node_id, kind, parent_node_id, name, is_overload, start_byte)"
        ),
        "CREATE TABLE source_files AS SELECT * FROM (VALUES ('m', false)) \
         AS t(module_node_id, is_stub)"
            .to_owned(),
        format!(
            "CREATE TABLE class_ancestry AS SELECT * FROM (VALUES \
               ('a0', {mro}, 0), ('a1', {mro}, 1), ('a2', {mro}, 0)) \
             AS t(fact_id, relation, ordinal)"
        ),
        // `C`'s MRO: an unresolved base, then `B`. `D`'s: `B`.
        "CREATE TABLE ancestry_targets AS SELECT * FROM (VALUES \
           ('a0', 'c', CAST(NULL AS VARCHAR)), ('a1', 'c', 'b'), ('a2', 'd', 'b')) \
         AS t(ancestry_fact_id, class_node_id, ancestor_node_id)"
            .to_owned(),
        format!(
            "CREATE TABLE scopes AS SELECT * FROM (VALUES ('scope', 'nobody', {})) \
             AS t(node_id, owner_node_id, kind)",
            LexicalScopeKind::Class.code()
        ),
        format!(
            "CREATE TABLE bindings AS SELECT * FROM (VALUES ('scope', 'x', {})) \
             AS t(scope_id, name, kind)",
            BindingKind::FunctionDef.code()
        ),
        "CREATE TABLE provider_node_map AS SELECT * FROM (VALUES ('none')) AS t(node_id)"
            .to_owned(),
    ];
    for statement in statements {
        ctx.sql(&statement).await.unwrap().collect().await.unwrap();
    }
    ctx
}

#[tokio::test]
async fn nothing_is_inherited_past_an_unresolved_base() {
    let ctx = session().await;
    let roots = ScalarValue::List(ScalarValue::new_list_nullable(
        &[ScalarValue::Utf8(Some("pub".to_owned()))],
        &datafusion::arrow::datatypes::DataType::Utf8,
    ));
    let batches = ctx
        .sql(&cpg_schema::public::public_paths().sql)
        .await
        .unwrap()
        .with_param_values(vec![("roots", roots)])
        .unwrap()
        .collect()
        .await
        .unwrap();
    let text = datafusion::arrow::util::pretty::pretty_format_batches(&batches)
        .unwrap()
        .to_string();
    // `C` keeps its own method and inherits nothing past the unresolved base; `D` inherits `run`.
    assert!(text.contains("pub.C.own"), "{text}");
    assert!(!text.contains("pub.C.run"), "{text}");
    assert!(text.contains("pub.D.run"), "{text}");
}
