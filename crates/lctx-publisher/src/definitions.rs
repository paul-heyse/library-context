//! Immutable named executable epochs. Other publications may install additional names
//! without changing this publication's realization identity.
use lctx_model::domain::{ContentHash, Key, KeySink, ModelError};
use lctx_surrealdb::{
    Loader,
    surrealdb::types::{Object, ToSql, Value},
};
use std::collections::BTreeSet;
use surrealdb_sql::{
    Expr, TopLevelExpr,
    statements::{DefineStatement, define::DefineKind},
};

pub(crate) fn epoch_identity(blueprint: &str) -> ContentHash {
    let mut key = KeySink::new("native-definition-epoch/v1");
    key.part(b"blueprint", blueprint.as_bytes());
    key.part(b"view-relative-library", b"indexed-library-root-adapter/v1");
    key.part(b"vector-policy", b"exact-eligible-cosine/v1");
    key.finish()
}
pub(crate) fn function_name(epoch: ContentHash, name: &str) -> String {
    format!(
        "lctx_e{}_{}",
        epoch.hex(),
        name.strip_prefix("lctx_").unwrap_or(name)
    )
}

pub(crate) fn expected(blueprint: &str) -> Result<(Vec<String>, Vec<String>), ModelError> {
    lctx_surrealdb::materialization::validate_native_definitions(blueprint)?;
    let epoch = epoch_identity(blueprint);
    let mut base = Vec::new();
    let mut functions = Vec::new();
    for statement in surrealdb_syn::parse(blueprint)
        .map_err(ModelError::codec)?
        .expressions
    {
        match statement {
            TopLevelExpr::Expr(Expr::Define(mut definition)) => match definition.as_mut() {
                DefineStatement::Function(function) => {
                    function.name = function_name(epoch, function.name.as_str()).into();
                    function.kind = DefineKind::Default;
                    let sql = definition.to_sql();
                    functions.push(normalize(&sql)?);
                }
                _ => base.push(normalize(&definition.to_sql())?),
            },
            _ => {
                return Err(ModelError::Schema(
                    "publication definition blueprint grammar",
                ));
            }
        }
    }
    Ok((base, functions))
}
pub(crate) fn normalize(sql: &str) -> Result<String, ModelError> {
    let parsed = surrealdb_syn::parse(sql).map_err(ModelError::codec)?;
    if parsed.expressions.len() != 1 {
        return Err(ModelError::Schema("single definition inventory"));
    }
    Ok(parsed.expressions[0].to_sql())
}
async fn info(loader: &Loader, sql: String) -> Result<Object, ModelError> {
    loader.check_read_admission()?;
    let mut response = loader
        .client()
        .query(sql)
        .await
        .map_err(ModelError::codec)?
        .check()
        .map_err(ModelError::codec)?;
    let value: Value = response.take(0).map_err(ModelError::codec)?;
    let Value::Object(object) = value else {
        return Err(ModelError::Schema("definition inventory object"));
    };
    Ok(object)
}
fn add_group(
    inventory: &mut BTreeSet<String>,
    object: &Object,
    group: &str,
) -> Result<(), ModelError> {
    let Some(Value::Object(definitions)) = object.get(group) else {
        return Err(ModelError::Schema("definition inventory group"));
    };
    for value in definitions.values() {
        let Value::String(sql) = value else {
            return Err(ModelError::Schema("definition inventory text"));
        };
        inventory.insert(normalize(sql)?);
    }
    Ok(())
}
async fn inventory(loader: &Loader) -> Result<BTreeSet<String>, ModelError> {
    let db = info(loader, "INFO FOR DB".into()).await?;
    let mut result = BTreeSet::new();
    for group in ["functions", "analyzers", "tables"] {
        add_group(&mut result, &db, group)?;
    }
    let Some(Value::Object(tables)) = db.get("tables") else {
        return Err(ModelError::Schema("definition table inventory"));
    };
    for name in tables.keys() {
        let escaped = name.replace('`', "\\`");
        let table = info(loader, format!("INFO FOR TABLE `{escaped}`")).await?;
        for group in ["fields", "indexes"] {
            add_group(&mut result, &table, group)?;
        }
    }
    Ok(result)
}
pub(crate) async fn install_epoch(
    loader: &Loader,
    blueprint: &str,
) -> Result<ContentHash, ModelError> {
    let (base, functions) = expected(blueprint)?;
    loader.check_read_admission()?;
    let mut catalog = loader.installation_catalog().await?;
    let actual = catalog.definitions();
    if base.iter().any(|definition| !actual.contains(definition)) {
        return Err(ModelError::Conflict(
            "installed native search/schema definitions",
        ));
    }
    catalog.apply(&(functions.join(";\n") + ";"), "immutable executable epoch").await?;
    loader.check_read_admission()?;
    let version = loader.client().version().await.map_err(ModelError::codec)?.to_string();
    verify_expected(blueprint, &version, &catalog.definitions(), &base, &functions)
}
pub(crate) async fn verify_epoch(
    loader: &Loader,
    blueprint: &str,
) -> Result<ContentHash, ModelError> {
    let actual = inventory(loader).await?;
    loader.check_read_admission()?;
    let version = loader
        .client()
        .version()
        .await
        .map_err(ModelError::codec)?
        .to_string();
    verify_inventory(blueprint, &version, &actual)
}
pub(crate) fn verify_inventory(blueprint: &str, version: &str, actual: &BTreeSet<String>) -> Result<ContentHash, ModelError> {
    let (base, functions) = expected(blueprint)?;
    verify_expected(blueprint, version, actual, &base, &functions)
}
fn verify_expected(blueprint: &str, version: &str, actual: &BTreeSet<String>, base: &[String], functions: &[String]) -> Result<ContentHash, ModelError> {
    let mut key = KeySink::new("native-view-realization/v1");
    if !version.starts_with("3.3.") {
        return Err(ModelError::Conflict("reviewed native engine family"));
    }
    key.part(b"engine", version.as_bytes());
    epoch_identity(blueprint).encode(&mut key);
    lctx_surrealdb::schema::realization_identity(blueprint).encode(&mut key);
    for definition in base.iter().chain(functions.iter()) {
        if !actual.contains(definition) {
            return Err(ModelError::Conflict("pinned executable definition epoch"));
        }
        key.part(b"actual-definition", definition.as_bytes());
    }
    Ok(key.finish())
}
pub(crate) use verify_epoch as verify_realization;
