//! Operation-owned actual catalog. Callers provide maintenance exclusion; possession of a
//! capture is not authority to install, publish or admit data in another database/epoch.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use surrealdb_sql::{Expr, TopLevelExpr, statements::DefineStatement};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum DefinitionKey {
    Table(String), Field(String, String), Index(String, String), Analyzer(String), Function(String),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Group { Database, Table(String) }
impl DefinitionKey {
    fn group(&self) -> Group {
        match self {
            Self::Field(table, _) | Self::Index(table, _) => Group::Table(table.clone()),
            _ => Group::Database,
        }
    }
}
struct Declaration { key: DefinitionKey, sql: String, index: Option<InstallationIndex> }
fn prepare(sql: &str) -> Result<Vec<Declaration>, ModelError> {
    surrealdb_syn::parse(sql).map_err(ModelError::codec)?.expressions.into_iter().map(|mut expression| {
        let TopLevelExpr::Expr(Expr::Define(definition)) = &mut expression else {
            return Err(ModelError::Schema("installation declaration grammar"));
        };
        let (key, index) = match definition.as_mut() {
            DefineStatement::Table(value) => (DefinitionKey::Table(value.name.to_sql()), None),
            DefineStatement::Field(value) => (DefinitionKey::Field(value.what.to_sql(), value.name.to_sql()), None),
            DefineStatement::Index(value) => {
                value.concurrently = false;
                let key = DefinitionKey::Index(value.what.to_sql(), value.name.to_sql());
                let mut execution = value.clone(); execution.concurrently = true;
                let label = format!("{} ON {}", value.name.to_sql(), value.what.to_sql());
                (key, Some(InstallationIndex { execution: execution.to_sql(), information: format!("INFO FOR INDEX {label}"), label }))
            }
            DefineStatement::Analyzer(value) => (DefinitionKey::Analyzer(value.name.to_sql()), None),
            DefineStatement::Function(value) => (DefinitionKey::Function(value.name.to_string()), None),
            _ => return Err(ModelError::Schema("installation declaration grammar")),
        };
        Ok(Declaration { key, sql: expression.to_sql(), index })
    }).collect()
}
fn missing<'a>(actual: &BTreeMap<DefinitionKey, String>, desired: &'a [Declaration]) -> Result<Vec<&'a Declaration>, ModelError> {
    let mut result = Vec::new();
    let mut names = BTreeMap::new();
    for declaration in desired {
        if names.insert(&declaration.key, &declaration.sql).is_some_and(|old| old != &declaration.sql) {
            return Err(ModelError::Conflict("conflicting desired installation declarations"));
        }
        match actual.get(&declaration.key) {
            Some(value) if value != &declaration.sql => return Err(ModelError::Conflict("installed named declaration meaning")),
            Some(_) => {},
            None => result.push(declaration),
        }
    }
    Ok(result)
}
/// A checked catalog belongs to one retained native session and excluded installation.
/// Ordinary publication, backup and cold admission perform their own actual-state capture.
pub struct InstallationCatalog {
    client: Arc<Surreal<Client>>,
    actual: BTreeMap<DefinitionKey, String>,
}
impl InstallationCatalog {
    /// Comparison metadata from this operation's actual capture, never an admission grant.
    pub fn definitions(&self) -> BTreeSet<String> { self.actual.values().cloned().collect() }
    pub(crate) fn has_table(&self, name: &str) -> bool { self.actual.contains_key(&DefinitionKey::Table(name.to_owned())) }
    pub(crate) fn check(&self, schema: &str) -> Result<(), ModelError> {
        if !missing(&self.actual, &prepare(schema)?)?.is_empty() {
            return Err(ModelError::Conflict("required actual installation declarations"));
        }
        Ok(())
    }
    async fn info(&self, sql: String) -> Result<Object, ModelError> {
        let mut response = self.client.query(sql).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
        let value: Value = response.take(0).map_err(ModelError::codec)?;
        let Value::Object(object) = value else { return Err(ModelError::Schema("installation definition inventory")); };
        Ok(object)
    }
    fn ingest(&mut self, object: &Object, groups: &[&str]) -> Result<(), ModelError> {
        for group in groups {
            let Some(Value::Object(definitions)) = object.get(*group) else { return Err(ModelError::Schema("installation definition group")); };
            for value in definitions.values() {
                let Value::String(sql) = value else { return Err(ModelError::Schema("installation definition text")); };
                for declaration in prepare(sql)? { self.actual.insert(declaration.key, declaration.sql); }
            }
        }
        Ok(())
    }
    pub(super) async fn capture(client: Arc<Surreal<Client>>) -> Result<Self, ModelError> {
        let mut result = Self { client, actual: BTreeMap::new() };
        let db = result.info("INFO FOR DB".into()).await?;
        result.ingest(&db, &["functions", "analyzers", "tables"])?;
        let Some(Value::Object(tables)) = db.get("tables") else { return Err(ModelError::Schema("installation table inventory")); };
        for name in tables.keys() {
            let escaped = name.replace('`', "\\`");
            let table = result.info(format!("INFO FOR TABLE `{escaped}`")).await?;
            result.ingest(&table, &["fields", "indexes"])?;
        }
        Ok(result)
    }
    async fn refresh(&mut self, groups: &BTreeSet<Group>) -> Result<(), ModelError> {
        for group in groups {
            let (sql, names): (String, &[&str]) = match group {
                Group::Database => ("INFO FOR DB".into(), &["functions", "analyzers", "tables"]),
                Group::Table(name) => (format!("INFO FOR TABLE {name}"), &["fields", "indexes"]),
            };
            let actual = self.info(sql).await?;
            self.actual.retain(|key, _| key.group() != *group);
            self.ingest(&actual, names)?;
        }
        Ok(())
    }
    pub async fn install(&mut self, blueprint: &str) -> Result<(), ModelError> {
        let physical = physical_native_definitions(blueprint)?;
        self.apply(&crate::schema::canonical_schema(), "canonical schema").await?;
        self.apply(&physical, "native physical definitions").await
    }
    /// Apply missing declarations only. A conflicting existing name refuses before effects.
    /// This method never silently replaces pinned meaning during ordinary installation.
    pub async fn apply(&mut self, schema: &str, phase: &str) -> Result<(), ModelError> {
        self.apply_observed(schema, phase, std::future::ready, Ok).await
    }
    // Private observations surround real checked DDL and real readiness reads. Controls can
    // discard a local acknowledgement or supply one pending observation without replacing
    // the production installation/reconciliation algorithm or introducing a global policy.
    async fn apply_observed<C, F, R>(&mut self, schema: &str, phase: &str, mut completion: C, mut readiness: R) -> Result<(), ModelError>
    where
        C: FnMut(Result<(), ModelError>) -> F,
        F: std::future::Future<Output = Result<(), ModelError>>,
        R: FnMut(Value) -> Result<Value, ModelError>,
    {
        let declarations = prepare(schema)?;
        let missing = missing(&self.actual, &declarations)?;
        let mut affected = BTreeSet::new();
        for (window, chunk) in missing.chunks(32).enumerate() {
            let groups = chunk.iter().map(|item| item.key.group()).collect::<BTreeSet<_>>();
            affected.extend(groups.iter().cloned());
            let sql = chunk.iter().map(|item| item.index.as_ref().map_or(item.sql.as_str(), |index| index.execution.as_str())).collect::<Vec<_>>().join(";") + ";";
            let result = self.client.query(sql).await.and_then(|response| response.check()).map(|_| ()).map_err(write_failure);
            let result = completion(result).await;
            if let Err(error) = result {
                let primary = installation_failure(format!("{phase} declaration window {}", window + 1), error);
                // A lost acknowledgement is resolved against precisely the affected catalog
                // groups. Partial or differing application remains a failure, never replay.
                let mut completion = lctx_model::domain::completion::Completion::default();
                let refreshed = self.refresh(&groups).await;
                let reconciled = refreshed.is_ok() && chunk.iter().all(|item| self.actual.get(&item.key) == Some(&item.sql));
                completion.step("targeted declaration acknowledgement reconciliation", refreshed);
                if !reconciled { return lctx_model::domain::completion::complete(Err(primary), completion); }
            } else {
                for item in chunk { self.actual.insert(item.key.clone(), item.sql.clone()); }
            }
        }
        self.refresh(&affected).await?;
        if declarations.iter().any(|item| self.actual.get(&item.key) != Some(&item.sql)) {
            return Err(installation_failure(format!("{phase} declaration readback"), ModelError::Conflict("installed native declaration readback")));
        }
        let mut pending = declarations.into_iter().filter_map(|item| item.index).collect::<Vec<_>>();
        while !pending.is_empty() {
            let mut remaining = Vec::new();
            for index in pending {
                let mut response = self.client.query(index.information.clone()).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                let value: Value = response.take(0).map_err(ModelError::codec)?;
                let value = readiness(value)?;
                if !installation_index_ready(&value).map_err(|error| installation_failure(format!("{phase} index {} readiness", index.label), error))? { remaining.push(index); }
            }
            pending = remaining;
            if !pending.is_empty() { tokio::time::sleep(std::time::Duration::from_millis(100)).await; }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operation_catalog_refuses_named_drift_before_missing_effects() {
        let original = prepare("DEFINE TABLE t SCHEMAFULL; DEFINE FIELD x ON t TYPE int;").unwrap();
        let actual = original.into_iter().map(|item| (item.key, item.sql)).collect();
        let desired = prepare("DEFINE FIELD new ON t TYPE string; DEFINE FIELD x ON t TYPE string;").unwrap();
        assert!(missing(&actual, &desired).is_err());
        let equal = prepare("DEFINE TABLE t SCHEMAFULL; DEFINE FIELD x ON t TYPE int;").unwrap();
        assert!(missing(&actual, &equal).unwrap().is_empty());
    }
    #[test]
    fn operation_catalog_keeps_table_names_and_index_execution_distinct() {
        let original = prepare("DEFINE FIELD x ON a TYPE int; DEFINE FIELD x ON b TYPE int; DEFINE INDEX ix ON a FIELDS x CONCURRENTLY;").unwrap();
        assert_ne!(original[0].key, original[1].key);
        let index = original[2].index.as_ref().unwrap();
        assert!(index.execution.contains("CONCURRENTLY"));
        assert!(!original[2].sql.contains("CONCURRENTLY"));
        assert_eq!(original[2].key.group(), Group::Table("a".into()));
    }
    async fn failed_targeted_refresh_retains_discarded_acknowledgement(config: &crate::RuntimeConfig) -> Result<(), ModelError> {
        let table = format!("catalog_control_{}", crate::control::fresh_identity("catalog-refresh-failure-control")?.hex());
        // This session alone loses authentication. Cleanup obtains another canonical
        // maintenance session; the surrounding control's client remains authenticated.
        let client = crate::compiler::check_installation(config).await?;
        let result = async {
            let mut catalog = InstallationCatalog::capture(client.clone()).await?;
            catalog.apply(&format!("DEFINE TABLE {table} SCHEMAFULL;"), "catalog refresh failure setup").await?;
            let declaration = format!("DEFINE FIELD committed ON {table} TYPE option<int>;");
            let refused = catalog.apply_observed(&declaration, "controlled unauthenticated catalog refresh", |result| {
                let client = client.clone();
                async move {
                    result?;
                    client.invalidate().await.map_err(ModelError::codec)?;
                    Err(ModelError::Conflict("controlled invalidated-session DDL acknowledgement discarded"))
                }
            }, Ok).await;
            match refused {
                Err(ModelError::Completion(outcome))
                    if outcome.primary.as_deref().is_some_and(|error| match error {
                        ModelError::Cause(error) => error.downcast_ref::<InstallationFailure>().is_some_and(|failure|
                            failure.context == "controlled unauthenticated catalog refresh declaration window 1"
                            && matches!(&failure.cause, ModelError::Conflict("controlled invalidated-session DDL acknowledgement discarded"))),
                        _ => false,
                    })
                    && outcome.completion.failures.len() == 1
                    && outcome.completion.failures.iter().any(|failure|
                        failure.step == "targeted declaration acknowledgement reconciliation"
                        && failure.error.to_string().contains("Not enough permissions to perform this action")) => Ok(()),
                Err(error) => Err(error),
                Ok(()) => Err(ModelError::Conflict("catalog accepted unauthenticated reconciliation read")),
            }
        }.await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("catalog refresh failure session invalidation", client.invalidate().await.map_err(ModelError::codec));
        completion.step("catalog refresh failure nonce-owned table cleanup", async {
            let cleanup_client = crate::compiler::check_installation(config).await?;
            let cleanup = async {
                cleanup_client.query(format!("REMOVE TABLE IF EXISTS {table};")).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                Ok(())
            }.await;
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step("catalog refresh cleanup session invalidation", cleanup_client.invalidate().await.map_err(ModelError::codec));
            lctx_model::domain::completion::complete(cleanup, completion)
        }.await);
        lctx_model::domain::completion::complete(result, completion)
    }
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "requires explicit owned-service validation maintenance installer credentials"]
    async fn checked_catalog_reconciles_discarded_ddl_ack_and_polls_existing_index() {
        let path = std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG")
            .expect("explicit validation maintenance installer configuration");
        let config = crate::RuntimeConfig::read(std::path::Path::new(&path)).unwrap();
        assert_eq!(config.authentication, crate::AuthenticationScope::Root);
        assert_eq!(config.database.as_str(), "validation");
        let table = format!("catalog_control_{}", crate::control::fresh_identity("catalog-completion-control").unwrap().hex());
        let client = crate::compiler::check_installation(&config).await.unwrap();
        let result = async {
            let mut response = client.query("SELECT VALUE admission_open FROM ONLY native_installation:current")
                .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let admission: Value = response.take(0).map_err(ModelError::codec)?;
            if admission != Value::Bool(false) { return Err(ModelError::Conflict("catalog control requires closed validation maintenance")); }
            let mut catalog = InstallationCatalog::capture(client.clone()).await?;
            catalog.apply(&format!("DEFINE TABLE {table} SCHEMAFULL; DEFINE FIELD value ON {table} TYPE string;"), "catalog control setup").await?;

            // Real DDL commits first. This discards only its local acknowledgement;
            // the production failure branch must reconcile the affected table group.
            let acknowledged = format!("DEFINE FIELD acknowledged ON {table} TYPE option<int>;");
            let mut discarded = 0;
            catalog.apply_observed(&acknowledged, "controlled discarded DDL acknowledgement", |result| {
                std::future::ready(result.and_then(|_| {
                    discarded += 1;
                    Err(ModelError::Conflict("controlled local DDL acknowledgement discarded"))
                }))
            }, Ok).await?;
            if discarded != 1 { return Err(ModelError::Conflict("catalog control did not discard actual DDL acknowledgement")); }
            catalog.check(&acknowledged)?;

            // Controlled nonce-owned mutations make the actual affected group partial,
            // then different. Neither state may be credited as a reconciled commit.
            let partial = format!("DEFINE FIELD partial_a ON {table} TYPE option<int>; DEFINE FIELD partial_b ON {table} TYPE option<int>;");
            let refused = catalog.apply_observed(&partial, "controlled partial DDL group", |result| {
                let client = client.clone(); let sql = format!("REMOVE FIELD partial_b ON {table};");
                async move {
                    result?;
                    client.query(sql).await.map_err(write_failure)?.check().map_err(ModelError::codec)?;
                    Err(ModelError::Conflict("controlled partial-group acknowledgement discarded"))
                }
            }, Ok).await;
            match refused {
                Err(error) if error.to_string().contains("controlled partial-group acknowledgement discarded") => {},
                Err(error) => return Err(error),
                Ok(()) => return Err(ModelError::Conflict("catalog reconciled a partial actual group")),
            }
            catalog.check(&format!("DEFINE FIELD partial_a ON {table} TYPE option<int>;"))?;
            if catalog.check(&partial).is_ok() { return Err(ModelError::Conflict("catalog partial group was not refreshed")); }

            let differing = format!("DEFINE FIELD differing ON {table} TYPE option<int>;");
            let refused = catalog.apply_observed(&differing, "controlled differing DDL group", |result| {
                let client = client.clone(); let sql = format!("DEFINE FIELD OVERWRITE differing ON {table} TYPE option<string>;");
                async move {
                    result?;
                    client.query(sql).await.map_err(write_failure)?.check().map_err(ModelError::codec)?;
                    Err(ModelError::Conflict("controlled differing-group acknowledgement discarded"))
                }
            }, Ok).await;
            match refused {
                Err(error) if error.to_string().contains("controlled differing-group acknowledgement discarded") => {},
                Err(error) => return Err(error),
                Ok(()) => return Err(ModelError::Conflict("catalog reconciled a differing actual group")),
            }
            catalog.check(&format!("DEFINE FIELD differing ON {table} TYPE option<string>;"))?;

            let rows = (0..32).map(|key| {
                let mut row = Object::new();row.insert("id", RecordId::new(table.as_str(), key.to_string()));row.insert("value", "populated");Value::Object(row)
            }).collect::<Vec<_>>();
            client.query("INSERT $rows RETURN NONE").bind(("rows", rows)).await.map_err(write_failure)?.check().map_err(ModelError::codec)?;
            client.query(format!("DEFINE INDEX populated_value ON {table} FIELDS value CONCURRENTLY;"))
                .await.map_err(write_failure)?.check().map_err(ModelError::codec)?;
            // Capture immediately after concurrent submission: never await readiness before
            // the equal-definition retry. One controlled pending observation makes the poll
            // branch deterministic even if this tiny actual build has already completed.
            let indexed = format!("DEFINE INDEX populated_value ON {table} FIELDS value;");
            let mut catalog = InstallationCatalog::capture(client.clone()).await?;
            catalog.check(&indexed)?;
            let mut ddl_completions = 0;let mut observations = 0;let mut terminal_actual = false;
            catalog.apply_observed(&indexed, "catalog-present pending readiness control", |result| {
                ddl_completions += 1;std::future::ready(result)
            }, |value| {
                let actual_ready = installation_index_ready(&value)?;
                observations += 1;
                if observations == 1 {
                    let mut building = Object::new();building.insert("status", "indexing");
                    let mut pending = Object::new();pending.insert("building", building);Ok(Value::Object(pending))
                } else { terminal_actual = actual_ready;Ok(value) }
            }).await?;
            if ddl_completions != 0 || observations < 2 || !terminal_actual { return Err(ModelError::Conflict("catalog-present index skipped actual terminal readiness")); }
            let mut response = client.query(format!("SELECT VALUE id FROM {table} WITH INDEX populated_value WHERE value='populated';"))
                .await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            let selected: Vec<RecordId> = response.take(0).map_err(ModelError::codec)?;
            if selected.len() != 32 { return Err(ModelError::Conflict("catalog-present populated index incomplete")); }

            let mut failed_readiness = 0;
            let refused = catalog.apply_observed(&indexed, "controlled terminal index failure", |result| {
                ddl_completions += 1;std::future::ready(result)
            }, |_| {
                failed_readiness += 1;
                let mut building = Object::new();building.insert("status", "error");
                let mut failed = Object::new();failed.insert("building", building);Ok(Value::Object(failed))
            }).await;
            if ddl_completions != 0 || failed_readiness != 1
                || !refused.is_err_and(|error| error.to_string().contains("controlled terminal index failure")) {
                return Err(ModelError::Conflict("catalog accepted terminal index failure or replayed DDL"));
            }

            // The actual table disappears after its new field commits. Pinned 3.3 INFO
            // returns empty definition groups for an absent table, not an error. The
            // successful targeted read must clear stale entries and refuse reconciliation.
            let gone = format!("DEFINE FIELD gone ON {table} TYPE option<int>;");
            let refused = catalog.apply_observed(&gone, "controlled unavailable catalog group", |result| {
                let client = client.clone();let sql = format!("REMOVE TABLE {table};");
                async move {
                    result?;
                    client.query(sql).await.map_err(write_failure)?.check().map_err(ModelError::codec)?;
                    Err(ModelError::Conflict("controlled unavailable-group acknowledgement discarded"))
                }
            }, Ok).await;
            match refused {
                Err(ModelError::Cause(error))
                    if error.downcast_ref::<InstallationFailure>().is_some_and(|failure|
                        failure.context == "controlled unavailable catalog group declaration window 1"
                        && matches!(&failure.cause, ModelError::Conflict("controlled unavailable-group acknowledgement discarded"))) => {},
                Err(error) => return Err(error),
                Ok(()) => return Err(ModelError::Conflict("catalog accepted unavailable reconciliation group")),
            }
            if catalog.check(&gone).is_ok()
                || catalog.actual.keys().any(|key| key.group() == Group::Table(table.clone())) {
                return Err(ModelError::Conflict("catalog absent group retained stale declarations"));
            }
            failed_targeted_refresh_retains_discarded_acknowledgement(&config).await?;
            Ok::<(), ModelError>(())
        }.await;
        let mut completion = lctx_model::domain::completion::Completion::default();
        completion.step("catalog control nonce-owned table cleanup", async {
            client.query(format!("REMOVE TABLE IF EXISTS {table};")).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;Ok(())
        }.await);
        completion.step("catalog control session invalidation", client.invalidate().await.map_err(ModelError::codec));
        lctx_model::domain::completion::complete(result, completion).unwrap();
    }
}
