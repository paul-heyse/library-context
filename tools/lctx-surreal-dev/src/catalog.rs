//! Independent installed-definition comparison, shared by Sync and tester postflight.
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use surrealdb::{
    Surreal,
    engine::any::Any,
    types::{Object, ToSql, Value},
};
use surrealdb_sql::{
    Expr, TopLevelExpr,
    statements::{DefineStatement, define::DefineKind},
};

/// Normalize only the application modifier. Bodies, permissions, kinds, indexes and nested
/// statements remain part of equality. The parser is pinned to the actual server family.
pub fn definitions(sql: &str) -> Result<BTreeMap<String, String>> {
    surrealdb_syn::parse(sql)?
        .expressions
        .into_iter()
        .map(|mut expression| {
            let TopLevelExpr::Expr(Expr::Define(definition)) = &mut expression else {
                bail!("schema inventory contains a non-definition");
            };
            let key = match definition.as_mut() {
                DefineStatement::Table(value) => {
                    value.kind = DefineKind::Default;
                    format!("table:{}", value.name.to_sql())
                }
                DefineStatement::Field(value) => {
                    value.kind = DefineKind::Default;
                    format!("field:{}:{}", value.what.to_sql(), value.name.to_sql())
                }
                DefineStatement::Index(value) => {
                    value.kind = DefineKind::Default;
                    format!("index:{}:{}", value.what.to_sql(), value.name.to_sql())
                }
                DefineStatement::Analyzer(value) => {
                    value.kind = DefineKind::Default;
                    format!("analyzer:{}", value.name.to_sql())
                }
                DefineStatement::Function(value) => {
                    value.kind = DefineKind::Default;
                    format!("function:{}", value.name.as_str())
                }
                _ => bail!("unsupported native definition family"),
            };
            Ok((key, expression.to_sql()))
        })
        .collect()
}

async fn info(db: &Surreal<Any>, sql: &str) -> Result<Object> {
    let mut response = db.query(sql).await?.check()?;
    let value: Value = response.take(0)?;
    let Value::Object(object) = value else {
        bail!("installed definition inventory is not an object");
    };
    Ok(object)
}

fn ingest(object: &Object, groups: &[&str], into: &mut BTreeMap<String, String>) -> Result<()> {
    for group in groups {
        let Some(Value::Object(entries)) = object.get(*group) else {
            bail!("missing installed definition group: {group}");
        };
        for value in entries.values() {
            let Value::String(sql) = value else {
                bail!("installed definition is not SQL");
            };
            into.extend(definitions(sql)?);
        }
    }
    Ok(())
}

/// Compare every selected native definition, ignoring only unrelated beta bookkeeping and
/// test-fixture objects. This observation grants no publication/admission authority.
pub async fn check_required(db: &Surreal<Any>, requested: &[String]) -> Result<usize> {
    let selected = crate::selected_modules(requested)?;
    let mut expected = BTreeMap::new();
    for module in selected {
        let (_, files) = crate::embedded_modules()
            .iter()
            .find(|(name, _)| *name == module)
            .context("selected embedded inventory")?;
        for file in *files {
            expected.extend(definitions(file.sql)?);
        }
    }
    let database = info(db, "INFO FOR DB").await?;
    let mut actual = BTreeMap::new();
    ingest(
        &database,
        &["functions", "analyzers", "tables"],
        &mut actual,
    )?;
    let Some(Value::Object(tables)) = database.get("tables") else {
        bail!("installed table inventory missing");
    };
    for table in tables.keys() {
        // Beta bookkeeping is outside native definition authority.
        if ["__entity", "__rollout", "__seed"].contains(&table.as_str()) {
            continue;
        }
        let escaped = table.replace('`', "\\`");
        let observed = info(db, &format!("INFO FOR TABLE `{escaped}`")).await?;
        ingest(&observed, &["fields", "indexes"], &mut actual)?;
    }
    for (key, definition) in &expected {
        match actual.get(key) {
            None => bail!("required native definition missing after beta sync: {key}"),
            Some(observed) if observed != definition => bail!(
                "required native definition changed after beta sync: {key}\nexpected: {definition}\nactual: {observed}"
            ),
            Some(_) => {}
        }
    }
    Ok(expected.len())
}
