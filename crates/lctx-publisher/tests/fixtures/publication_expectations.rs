//! Private end-to-end CN02 controls. Run only on the coordinator's wholly owned
//! lifecycle campaign under its live maintenance lease. Database names grant no authority.
use super::{Admission, Publication};
use cpg_core::{
    artifact, compilation,
    workspace::{Workspace, WorkspaceOptions},
};
use futures::FutureExt;
use lctx_model::domain::{
    admission::Frontier,
    completion::{Completion, complete},
    stages::Profile,
    *,
};
use lctx_native::phase::{Phase, Terminal};
use lctx_native::{
    Loader, RuntimeConfig,
    surrealdb::types::{Value, Variables},
};
use std::{panic::AssertUnwindSafe, sync::Arc};
use tracing::instrument::WithSubscriber;

#[derive(Clone, Copy)]
enum Mutation {
    Search,
    Lexical,
    Analyzer,
}

async fn observed<T>(
    name: &'static str,
    operation: impl std::future::Future<Output = Result<T, ModelError>>,
) -> Result<T, ModelError> {
    let phase = Phase::begin(name);
    let result = operation.await;
    phase.finish_result(&result);
    result
}

fn require_campaign_maintenance(config: &RuntimeConfig) -> Result<(), ModelError> {
    // Reuse the service owner's exact descriptor and live host-lock/process checks.
    // Reimplementing those checks here would create a second maintenance authority.
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let proof = std::process::Command::new(repository.join(".venv/bin/python"))
        .env("PYTHONPATH", repository.join("scripts"))
        .args(["-c", r#"
import json
import os
from pathlib import Path
import sys
import surrealdb_service as service

selected = Path(os.environ["LCTX_SURREAL_SERVICE_CONFIG"]).resolve()
installation = service.Installation.load(ready=False)
record = installation.record
assert selected == (installation.directory / "installation.json").resolve(), "selected campaign descriptor path"
assert record.get("kind") == "campaign", "CN02 requires an owned campaign"
assert record.get("purpose") == "surrealdb-lifecycle", "CN02 requires lifecycle campaign purpose"
assert installation.id == os.environ["LCTX_SURREAL_INSTALLATION_ID"], "campaign attachment installation identity"
endpoint, namespace, generation, database = sys.argv[1:]
assert installation.grpc_endpoint == endpoint, "campaign runtime endpoint"
assert record["namespace"] == namespace, "campaign runtime namespace"
assert bytes(record["service_generation"]).hex() == generation, "campaign runtime generation"
assert os.environ["LCTX_SURREAL_GENERATION"] == generation, "campaign attachment generation"
assert database in record["databases"], "campaign runtime database scope"
assert Path(os.environ["LCTX_SURREAL_INSTALLER_CONFIG"]).resolve() == installation.runtime_path(database, installer=True).resolve(), "campaign installer path"
assert service.maintenance_owned(installation), "CN02 requires live matching host maintenance ownership"
runtime = json.loads(Path(os.environ["LCTX_COMPILER_RUNTIME_CONFIG"]).read_text())
assert (runtime["endpoint"], runtime["namespace"], bytes(runtime["service_generation"]).hex(), runtime["database"]) == (endpoint, namespace, generation, database), "selected compiler runtime correspondence"
"#])
        .arg(&config.endpoint)
        .arg(config.namespace.as_str())
        .arg(config.service_generation.hex())
        .arg(config.database.as_str())
        .output().map_err(ModelError::codec)?;
    if !proof.status.success() {
        return Err(ModelError::Invalid(format!(
            "CN02 owned campaign maintenance proof refused: {}",
            String::from_utf8_lossy(&proof.stderr)
        )));
    }
    Ok(())
}

async fn control(mutation: Mutation) -> Result<(), ModelError> {
    let config = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").expect("owned campaign runtime"),
    ))?;
    require_campaign_maintenance(&config)?;
    let installer = RuntimeConfig::read(std::path::Path::new(
        &std::env::var_os("LCTX_SURREAL_INSTALLER_CONFIG").expect("owned campaign installer"),
    ))?;
    if installer.endpoint != config.endpoint
        || installer.namespace != config.namespace
        || installer.database != config.database
        || installer.service_generation != config.service_generation
        || installer.authentication != lctx_native::AuthenticationScope::Root
    {
        return Err(ModelError::Conflict(
            "CN02 exact campaign installer correspondence",
        ));
    }
    let model = Arc::new(model()?);
    let native = lctx_compiler_store::compiler::NativeCompilerStore::begin(
        &config,
        Frontier::Catalog,
        &lctx_compiler_store::compiler::recovery_factory(
            &config,
            (lctx_model::domain::resources::ResourceBudget::fixed(
                lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
            )
            .expect("owned native control recovery budget"))
            .clone(),
        )
        .expect("owned native control factory"),
    )
    .await?;
    let workspace = match Workspace::new(model, WorkspaceOptions::default(), native.clone()) {
        Ok(workspace) => workspace,
        Err(error) => return complete(Err(error), native.abandon_finality_report().await),
    };
    let mut terminal = Completion::default();
    let mut expected_refusal = None;
    let result = AssertUnwindSafe(async {
        let scratch = tempfile::tempdir().map_err(ModelError::codec)?;
        let module = format!("cn02_{}", native.attempt().hex());
        let path = format!("{module}.py");
        std::fs::write(scratch.path().join(&path), b"def discover(option: str = 'value') -> str:\n    \"\"\"Discover the requested option.\"\"\"\n    return option\n").map_err(ModelError::codec)?;
        let captured = Arc::new(cpg_extract::bundle::CapturedInputs::new(
            vec![cpg_extract::acquisition::AcquiredInput::tree(
                cpg_extract::capture::CapturedInput::capture(scratch.path(), &[path], workspace.budget()).map_err(ModelError::codec)?,
                &module,
            )],
            cpg_extract::native_context::NativeContextConfig::committed(Profile::Catalog, workspace.budget()).map_err(ModelError::codec)?,
        ));
        let settings = ContentHash::of(module.as_bytes());
        let analytics = analysis::settings::AnalyticsConfiguration {
            module_prefixes: vec![module.clone()], public_roots: vec![module], configured_seeds: vec![],
            depth: 2, vertices: 512, arcs: 2048, witnesses: 3, brief_budget: 8,
            communities: false, pagerank: false, fca: false, knn: false, rca: false,
            type_layer: false, mention_layer: false, knn_layer: false,
        };
        let prepared = compilation::PreparedCompilation::new(Frontier::Catalog, analytics, captured.config().catalog(), None, workspace.budget()).map(std::sync::Arc::new)?;
        observed("cn02_compile", compilation::compile(&workspace, captured.clone(), Profile::Catalog, settings, Frontier::Catalog, Some(prepared.clone()), None, None)).await?;
        drop(prepared);
        let admitted = observed("cn02_admit", artifact::admit(&workspace, &captured, Frontier::Catalog, Profile::Catalog, settings)).await?;
        let definitions = lctx_native::materialization::native_definitions();
        let client = lctx_native::reader::connect(&installer.endpoint, &installer.writer_credentials(), installer.namespace.as_str(), installer.database.as_str()).await?;
        let root = Loader::new(client.clone()).with_budget(workspace.budget());
        let created = AssertUnwindSafe(observed("cn02_publication_prepare", Publication::new(Admission::Completed(&admitted), &config, &definitions))).catch_unwind().await;
        let mut publication = match created {
            Ok(Ok(publication)) => publication,
            failed => {
                terminal.step("CN02 installer setup session invalidation", root.client().invalidate().await.map_err(ModelError::codec));
                match failed {
                    Ok(Err(error)) => return Err(error),
                    Err(panic) => std::panic::resume_unwind(panic),
                    Ok(Ok(_)) => unreachable!(),
                }
            }
        };
        let mut original = None;
        let exercised = AssertUnwindSafe(async {
            observed("cn02_materialize_references", publication.materialize_references()).await?;
            observed("cn02_materialize_search", publication.materialize_search()).await?;
            if matches!(mutation, Mutation::Lexical) {
                // Test the actual retained route before any native mutation. Equal
                // metadata and a shared staging root never grant a foreign Loader use.
                let expected = super::publication_handle(admitted.manifest(), &publication.bindings, native.database(), &config, publication.loader.derivation()?, publication.definition_epoch, publication.eligibility)?;
                let staging = publication.loader.staging().ok_or(ModelError::Schema("CN02 actual staging owner"))?.clone();
                let foreign = Loader::for_attempt_views(publication.loader.shared_client(), native.attempt(), publication.loader.view_ids().ok_or(ModelError::Schema("CN02 exact views"))?.to_vec())
                    .with_budget(workspace.budget()).with_derivation(publication.loader.derivation()?).with_staging(staging.clone())?.prepare_completed_reader().await?;
                let rejected = staging.publish_prepared(&foreign, &expected, admitted.manifest(), &publication.bindings, workspace.budget(), publication.search.as_ref().ok_or(ModelError::Schema("CN02 retained search"))?).await;
                let foreign_closed = foreign.close_reader().await;
                let error = match rejected { Ok(_) => return Err(ModelError::Conflict("CN02 foreign Loader borrowed preparation")), Err(error) => error };
                foreign_closed?;
                if !error.to_string().contains("prepared search protected owner/context changed") {
                    return Err(ModelError::Invalid(format!("CN02 foreign owner refused before expected capability: {error}")));
                }
                // A changed pool on the very same nominal Loader also refuses. Restore
                // the ordinary owner's pool before its final independent actual check.
                let foreign_budget = resources::ResourceBudget::fixed(workspace.budget().limit())?;
                let placeholder = Loader::new(publication.loader.shared_client());
                publication.loader = std::mem::replace(&mut publication.loader, placeholder).with_budget(&foreign_budget);
                let rejected = staging.publish_prepared(&publication.loader, &expected, admitted.manifest(), &publication.bindings, workspace.budget(), publication.search.as_ref().ok_or(ModelError::Schema("CN02 retained search"))?).await;
                let placeholder = Loader::new(publication.loader.shared_client());
                publication.loader = std::mem::replace(&mut publication.loader, placeholder).with_budget(workspace.budget());
                if rejected.is_ok() { return Err(ModelError::Conflict("CN02 changed pool borrowed preparation")); }
            }
            // The cold route has no access to Publication's retained expectations.
            observed("cn02_cold_search_reconcile", lctx_native::derived_search::reconcile_search(publication.loader())).await?;
            observed("cn02_mutate", async {
            let reader = publication.loader().reader();
            if matches!(mutation, Mutation::Analyzer) {
                client.query("DEFINE ANALYZER OVERWRITE lctx_discovery TOKENIZERS blank FILTERS lowercase;").await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
                // Bypass Publication::seal's whole definition verification deliberately:
                // the publicly reachable native retained boundary must independently refuse.
                let expected = super::publication_handle(admitted.manifest(), &publication.bindings, native.database(), &config, publication.loader.derivation()?, publication.definition_epoch, publication.eligibility)?;
                let staging = publication.loader.staging().ok_or(ModelError::Schema("CN02 direct retained staging"))?;
                let result = staging.publish_prepared(publication.loader(), &expected, admitted.manifest(), &publication.bindings, workspace.budget(), publication.search.as_ref().ok_or(ModelError::Schema("CN02 direct retained expectations"))?).await;
                let error = match result { Ok(_) => return Err(ModelError::Conflict("CN02 direct native retained route accepted changed analyzer")), Err(error) => error };
                if !error.to_string().contains("prepared lexical actual analyzer changed") || error.has_committed_effect() {
                    return Err(ModelError::Invalid(format!("CN02 direct native analyzer refusal missing: {error}")));
                }
            } else {
                let (sql, mut vars) = if matches!(mutation, Mutation::Lexical) {
                    let mut vars = Variables::new(); vars.insert("scope", lctx_native::lexical_stats::scope_identity(&reader)?.hex());
                    ("SELECT * FROM lexical_corpus WITH INDEX scoped_corpus WHERE scope=$scope ORDER BY id LIMIT 1", vars)
                } else {
                    let mut actual = lctx_native::derived_search::actual_search_rows(&reader, workspace.budget())?;
                    let selected = async {
                        while let Some(value) = actual.next().await? {
                            if matches!(value.as_object().and_then(|row| row.get("id")), Some(Value::RecordId(id)) if id.table.as_str().starts_with("search_")) {
                                return Ok(value);
                            }
                        }
                        Err(ModelError::Conflict("CN02 compiled fixture needs a real search document"))
                    }.await;
                    let mut terminal = Completion::default(); terminal.step("CN02 document selection drainage", actual.drain_transport().await);
                    let value = complete(selected, terminal)?;
                    let mut vars = Variables::new(); vars.insert("selected", value.as_object().and_then(|row| row.get("id")).cloned().ok_or(ModelError::Schema("CN02 selected document id"))?);
                    ("SELECT * FROM $selected", vars)
                };
                let rows: Vec<Value> = reader.query_native(sql, std::mem::take(&mut vars)).await?;
                let [Value::Object(row)] = rows.as_slice() else { return Err(ModelError::Schema("CN02 mutation original cardinality")); };
                let Some(Value::RecordId(id)) = row.get("id") else { return Err(ModelError::Schema("CN02 mutation original id")); };
                original = Some((id.clone(), Value::Object(row.clone())));
                let mut vars = Variables::new(); vars.insert("id", id.clone());
                let sql = if matches!(mutation, Mutation::Lexical) { "UPDATE $id SET total_length+=1 RETURN NONE" } else { "UPDATE $id SET text=string::concat(text,' changed after materialization') RETURN NONE" };
                client.query(sql).bind(vars).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
            }
            drop(reader);
            Ok(())
            }).await?;
            let outcome = observed("cn02_seal", publication.seal(&config, &definitions)).await;
            let error = match outcome { Ok(_) => return Err(ModelError::Conflict("CN02 mutated state was published")), Err(error) => error };
            if error.has_committed_effect() { return Err(ModelError::Conflict("CN02 refusal acknowledged a publication")); }
            let correct_refusal = match mutation {
                Mutation::Search => matches!(error.primary(), Some(ModelError::Serving(lctx_model::domain::serving::FailureKind::Corrupt))),
                Mutation::Lexical => matches!(error.primary(), Some(ModelError::Conflict("frozen lexical statistics differ"))),
                Mutation::Analyzer => matches!(error.primary(), Some(ModelError::Conflict("pinned executable definition epoch"))),
            };
            if !correct_refusal { return Err(ModelError::Invalid(format!("CN02 unexpected seal refusal: {error:#?}"))); }
            expected_refusal = Some(error);
            observed("cn02_publication_invalidate", publication.invalidate(expected_refusal.as_ref().unwrap())).await?;
            Ok(())
        }).catch_unwind().await;
        // Restore exactly this test's mutation before attempting logical cleanup.
        let restored = observed("cn02_restore", async { if matches!(mutation, Mutation::Analyzer) {
            client.query("DEFINE ANALYZER OVERWRITE lctx_discovery TOKENIZERS class,camel FILTERS lowercase;").await.map_err(ModelError::codec).and_then(|response| response.check().map(drop).map_err(ModelError::codec))
        } else if let Some((id, value)) = original {
            let mut vars = Variables::new(); vars.insert("id", id); vars.insert("row", value);
            client.query("UPDATE $id CONTENT $row RETURN NONE").bind(vars).await.map_err(ModelError::codec).and_then(|response| response.check().map(drop).map_err(ModelError::codec))
        } else { Ok(()) } }).await;
        terminal.step("CN02 original mutation restoration", restored);
        match &exercised {
            Ok(Err(error)) => terminal.step("CN02 publication invalidation", observed("cn02_publication_invalidate", publication.invalidate(error)).await),
            Err(_) => terminal.step("CN02 panicked publication invalidation", observed("cn02_publication_invalidate", publication.invalidate(&ModelError::Invalid("CN02 control panicked".into()))).await),
            Ok(Ok(())) => {},
        }
        terminal.step("CN02 installer session invalidation", observed("cn02_installer_session_close", async { root.client().invalidate().await.map_err(ModelError::codec) }).await);
        match exercised {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }).catch_unwind().await;
    let phase = Phase::begin("cn02_workspace_drain");
    let drained = workspace.drain_owned_report().await;
    phase.finish(Terminal::Returned);
    terminal.merge(drained);
    // Abandonment itself refuses unknown drainage. Its actual report must precede
    // the workspace signal; historical negative-control errors are diagnostics.
    let phase = Phase::begin("cn02_native_abandon");
    let abandoned = native.abandon_finality_report().await;
    phase.finish(Terminal::Returned);
    terminal.merge(abandoned);
    let signalled = observed(
        "cn02_workspace_signal",
        workspace.complete_signal(&terminal),
    )
    .await;
    terminal.step("CN02 workspace signal finality", signalled);
    let mut diagnostics = Completion::default();
    workspace.append_operation_failures(&mut diagnostics);
    match result {
        Ok(Ok(())) => {
            complete(Ok(()), terminal)?;
            let expected = expected_refusal
                .as_ref()
                .ok_or(ModelError::Schema("CN02 expected seal refusal"))?;
            if diagnostics.failures.is_empty()
                || diagnostics.failures.iter().any(|failure| {
                    failure.step != "native final seal" || !same_refusal(&failure.error, expected)
                })
            {
                return Err(ModelError::Invalid(format!(
                    "CN02 unexpected operation diagnostics: {diagnostics:#?}"
                )));
            }
            eprintln!(
                "CN02 expected seal refusal retained after actual finality: {diagnostics:#?}"
            );
            Ok(())
        }
        Ok(Err(error)) => {
            terminal.merge(diagnostics);
            complete(Err(error), terminal)
        }
        Err(panic) => {
            terminal.merge(diagnostics);
            if let Err(error) = complete(Ok(()), terminal) {
                eprintln!("CN02 panic cleanup: {error}");
            }
            std::panic::resume_unwind(panic)
        }
    }
}

// Native lease refusal and sticky diagnostics retain the exact same shared cause.
fn same_refusal(actual: &ModelError, expected: &ModelError) -> bool {
    let mut actual = actual;
    while let ModelError::SharedCause(source) = actual {
        let mut expected = expected;
        while let ModelError::SharedCause(other) = expected {
            if Arc::ptr_eq(source, other) {
                return true;
            }
            expected = other;
        }
        actual = source;
    }
    false
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires coordinator-owned disposable maintenance campaign"]
async fn materialize_search_then_mutate_document_seal_refuses() {
    control(Mutation::Search)
        .with_subscriber(cpg_extract::logging::dispatch())
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires coordinator-owned disposable maintenance campaign"]
async fn materialize_search_then_mutate_lexical_seal_refuses() {
    control(Mutation::Lexical)
        .with_subscriber(cpg_extract::logging::dispatch())
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires exclusive coordinator-owned disposable maintenance campaign"]
async fn materialize_search_then_change_actual_analyzer_seal_refuses() {
    control(Mutation::Analyzer)
        .with_subscriber(cpg_extract::logging::dispatch())
        .await
        .unwrap();
}
