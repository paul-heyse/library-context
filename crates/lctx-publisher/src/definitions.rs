//! Immutable named executable epochs. Other publications may install additional names
//! without changing this publication's realization identity.
use lctx_model::domain::{ContentHash, Key, KeySink, ModelError};
use lctx_native::{
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
    let expected = prepare_expected(blueprint)?;
    Ok((expected.base, expected.functions))
}
struct DefinitionRequirements {
    base: Vec<String>,
    functions: Vec<String>,
    tables: BTreeSet<String>,
}
fn prepare_expected(blueprint: &str) -> Result<DefinitionRequirements, ModelError> {
    lctx_native::materialization::validate_native_definitions(blueprint)?;
    let epoch = epoch_identity(blueprint);
    let mut base = Vec::new();
    let mut functions = Vec::new();
    let mut tables = BTreeSet::new();
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
                _ => {
                    match definition.as_ref() {
                        DefineStatement::Table(value) => {
                            tables.insert(value.name.to_sql());
                        }
                        DefineStatement::Field(value) => {
                            tables.insert(value.what.to_sql());
                        }
                        DefineStatement::Index(value) => {
                            tables.insert(value.what.to_sql());
                        }
                        _ => {}
                    }
                    base.push(normalize(&definition.to_sql())?);
                }
            },
            _ => {
                return Err(ModelError::Schema(
                    "publication definition blueprint grammar",
                ));
            }
        }
    }
    Ok(DefinitionRequirements {
        base,
        functions,
        tables,
    })
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
async fn inventory(
    loader: &Loader,
    required_tables: &BTreeSet<String>,
) -> Result<BTreeSet<String>, ModelError> {
    let db = info(loader, "INFO FOR DB".into()).await?;
    let mut result = BTreeSet::new();
    for group in ["functions", "analyzers", "tables"] {
        add_group(&mut result, &db, group)?;
    }
    for table in required_tables {
        let table = info(loader, format!("INFO FOR TABLE {table}")).await?;
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
    catalog
        .apply(&(functions.join(";\n") + ";"), "immutable executable epoch")
        .await?;
    loader.check_read_admission()?;
    let version = loader
        .client()
        .version()
        .await
        .map_err(ModelError::codec)?
        .to_string();
    verify_expected(
        blueprint,
        &version,
        &catalog.definitions(),
        &base,
        &functions,
    )
}
/// Operation-owned interpretation only. Actual mutable definitions are observed again at seal;
/// neither these strings nor a hash confer maintenance exclusion or installed authority.
pub(crate) struct PreparedDefinitions {
    blueprint: String,
    base: Vec<String>,
    functions: Vec<String>,
    tables: BTreeSet<String>,
    _charge: Box<dyn lctx_model::domain::resources::Reservation>,
}
impl PreparedDefinitions {
    pub(crate) fn new(loader: &Loader, blueprint: &str) -> Result<Self, ModelError> {
        let charge = loader.read_budget()?.reserve(
            "publication required definition preparation",
            blueprint.len().saturating_mul(24).saturating_add(2048),
        )?;
        let DefinitionRequirements {
            base,
            functions,
            tables,
        } = prepare_expected(blueprint)?;
        Ok(Self {
            blueprint: blueprint.to_owned(),
            base,
            functions,
            tables,
            _charge: charge,
        })
    }
    pub(crate) fn require_blueprint(&self, blueprint: &str) -> Result<(), ModelError> {
        if self.blueprint != blueprint {
            return Err(ModelError::Conflict(
                "publication executable blueprint changed",
            ));
        }
        Ok(())
    }
    pub(crate) async fn verify(&self, loader: &Loader) -> Result<ContentHash, ModelError> {
        let actual = inventory(loader, &self.tables).await?;
        loader.check_read_admission()?;
        let version = loader
            .client()
            .version()
            .await
            .map_err(ModelError::codec)?
            .to_string();
        verify_expected(
            &self.blueprint,
            &version,
            &actual,
            &self.base,
            &self.functions,
        )
    }
}
pub(crate) async fn verify_epoch(
    loader: &Loader,
    blueprint: &str,
) -> Result<ContentHash, ModelError> {
    PreparedDefinitions::new(loader, blueprint)?
        .verify(loader)
        .await
}
pub(crate) fn verify_inventory(
    blueprint: &str,
    version: &str,
    actual: &BTreeSet<String>,
) -> Result<ContentHash, ModelError> {
    let (base, functions) = expected(blueprint)?;
    verify_expected(blueprint, version, actual, &base, &functions)
}
fn verify_expected(
    blueprint: &str,
    version: &str,
    actual: &BTreeSet<String>,
    base: &[String],
    functions: &[String],
) -> Result<ContentHash, ModelError> {
    for definition in base.iter().chain(functions.iter()) {
        if !actual.contains(definition) {
            return Err(ModelError::Conflict("pinned executable definition epoch"));
        }
    }
    let mut key = KeySink::new("native-view-realization/v2");
    lctx_native::derivation::DerivationIdentity::current(
        implementation_identity()?,
        epoch_identity(blueprint),
    )?
    .identity()?
    .encode(&mut key);
    if !version.starts_with("3.3.") {
        return Err(ModelError::Conflict("reviewed native engine family"));
    }
    key.part(b"engine", version.as_bytes());
    epoch_identity(blueprint).encode(&mut key);
    lctx_native::schema::realization_identity(blueprint).encode(&mut key);
    for definition in base.iter().chain(functions.iter()) {
        key.part(b"actual-definition", definition.as_bytes());
    }
    Ok(key.finish())
}
pub(crate) use verify_epoch as verify_realization;

fn implementation_identity() -> Result<ContentHash, ModelError> {
    Ok(ContentHash(
        hex::decode(env!("LCTX_PUBLICATION_SOURCE_DIGEST"))
            .map_err(ModelError::codec)?
            .try_into()
            .map_err(|_| ModelError::Schema("publication implementation digest width"))?,
    ))
}

#[cfg(test)]
mod controls {
    use super::*;
    #[test]
    fn required_definition_details_are_scoped_and_required_drift_refuses() {
        let blueprint = lctx_native::materialization::native_definitions();
        let DefinitionRequirements {
            base,
            functions,
            tables,
        } = prepare_expected(&blueprint).unwrap();
        assert!(tables.contains("search_source"));
        assert!(tables.contains("vector"));
        assert!(!tables.contains("compiler_record"));
        assert!(!tables.contains("unrelated"));
        let mut actual = base
            .iter()
            .chain(&functions)
            .cloned()
            .collect::<BTreeSet<_>>();
        let identity = verify_inventory(&blueprint, "3.3.0", &actual).unwrap();
        actual.insert("DEFINE TABLE unrelated SCHEMAFULL".into());
        assert_eq!(
            verify_inventory(&blueprint, "3.3.0", &actual).unwrap(),
            identity
        );
        actual.remove(&base[0]);
        assert!(verify_inventory(&blueprint, "3.3.0", &actual).is_err());
    }
}
