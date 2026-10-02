//! Actual pinned compiler, canonical generation receipts and shared native request grant.
#[path = "fixtures/serving_support.rs"]
mod support;
use lctx_model::domain::{
    self,
    calls::SignatureParameter,
    native_requests::{Assumptions, ExactOutcome, ExactScalar},
    normalized::entities::ParameterEntityLink,
    resources::ResourceBudget,
    serving::*,
    *,
};
use lctx_postgres::{
    generations::{CatalogService, Error, GenerationId, GenerationService, GenerationStore},
    testing::DisposableDatabase,
};
use std::sync::Arc;
use support::{command, input_source, write};

const SOURCE:&[u8]=b"__all__ = ['api', 'defaulted', 'computed', 'relay']\ndef api(value):\n    \"\"\"Return the provided value with explicit None handling.\"\"\"\n    if value is None:\n        return value\n    return value\ndef defaulted(value=None):\n    if value is None:\n        return value\n    return value\ndef computed(value):\n    return value + 1\ndef relay(value):\n    return api(value)\ndef _pair(a, b, c, d, e, f, g, h):\n    return a\ndef _grouped(value):\n    _pair(value, None, value, None, value, None, value, None)\n    return _pair(None, value, None, value, None, value, None, value)\n";

#[tokio::test]
async fn canonical_native_restriction_both_profiles_and_request_binding() {
    let db = DisposableDatabase::start().await;
    db.migrate().await;
    let model = Arc::new(domain::model().unwrap());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    input_source(dir.path(), SOURCE);
    db.write_configs(dir.path()).unwrap();
    write(
        &dir.path().join("libraries/demo/analytics.toml"),
        "version = 1\n[subsystem]\nmodule_prefixes = ['demo']\npublic_roots = ['demo']\n[seeds]\nprimary = ['demo.api']\ndistractors = []\n[pass_a]\nmax_depth = 4\nmax_vertices = 256\nmax_edges = 1024\nmax_witnesses = 4\n[briefs]\nbudget = 1\n",
    );
    let mut transport_failures = Vec::new();
    for profile in ["catalog", "behavioral"] {
        let output = command(
            dir.path(),
            &dir.path().join("postgres.json"),
            "catalog",
            profile,
        )
        .output()
        .unwrap();
        assert!(
            output.status.success(),
            "{profile}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let generation = GenerationId::from_hex(report["generation"].as_str().unwrap()).unwrap();
        let config =
            lctx_postgres::roles::RoleConfig::load(&dir.path().join("postgres-serving.json"))
                .unwrap();
        let service = GenerationService::admit(model.clone(), &config, Some(generation))
            .await
            .unwrap();
        let catalog = CatalogService::prepare(service.clone()).await.unwrap();
        let execution = service.execution().await.unwrap();
        let found = catalog
            .find(
                &execution,
                &FindOperationsRequest {
                    library: Name::new("demo").unwrap(),
                    selection: SelectionInput::default(),
                    page: PageRequest {
                        size: 100,
                        ..Default::default()
                    },
                },
            )
            .await
            .unwrap();
        let candidate = found
            .supported
            .items
            .iter()
            .chain(&found.unresolved.items)
            .find(|r| r.name.as_str().ends_with("api"))
            .expect("actual published api");
        drop(execution);
        let mut lease = store
            .pin(
                &db.reader,
                generation,
                ResourceBudget::fixed(128 << 20).unwrap(),
            )
            .await
            .unwrap();
        let parameters = lease.read::<SignatureParameter>().await.unwrap();
        let shapes = lease.read::<calls::ParameterShape>().await.unwrap();
        let links = lease.read::<ParameterEntityLink>().await.unwrap();
        let declarations = lease
            .read::<declarations::ParameterDeclaration>()
            .await
            .unwrap();
        let occurrences = lease.read::<source::Occurrence>().await.unwrap();
        let artifacts = lease.read::<source::SourceArtifact>().await.unwrap();
        // Independent ordering witness: canonical receipts use ID order, while the shared
        // call validator requires each call's argument ordinals to remain grouped.
        let arguments = lease.read::<calls::CallArgument>().await.unwrap();
        assert!(arguments.rows().len() >= 16);
        let mut by_id = arguments.rows().iter().map(Record::id).collect::<Vec<_>>();
        by_id.sort();
        let mut grouped = arguments.rows().iter().collect::<Vec<_>>();
        grouped.sort_by_key(|row| (row.call, row.ordinal, row.id()));
        assert_ne!(
            by_id,
            grouped.iter().map(|row| row.id()).collect::<Vec<_>>(),
            "fixture must challenge both orderings"
        );
        let briefs = lease.read::<synthesis::briefs::Brief>().await.unwrap();
        let capability = briefs
            .rows()
            .first()
            .expect("actual seeded producer publishes a capability")
            .id();
        let artifact = artifacts
            .rows()
            .iter()
            .find(|r| r.path.ends_with("__init__.py"))
            .unwrap()
            .id();
        let owner_formal = |function: &str| {
            let needle = format!("def {function}(");
            let offset = std::str::from_utf8(SOURCE).unwrap().find(&needle).unwrap() as i64;
            let owner = occurrences
                .rows()
                .iter()
                .find(|r| {
                    r.start == offset
                        && r.syntax_kind == source::SyntaxKind::StmtFunctionDef
                        && artifacts
                            .rows()
                            .iter()
                            .any(|a| a.id() == r.source && a.path.ends_with("__init__.py"))
                })
                .unwrap();
            let declaration = declarations
                .rows()
                .iter()
                .find(|d| {
                    occurrences.rows().iter().any(|p| {
                        p.id() == d.declaration
                            && p.source == owner.source
                            && p.start >= owner.start
                            && p.end <= owner.end
                            && p.structural_path.starts_with(&owner.structural_path)
                    })
                })
                .unwrap();
            normalized::entities::ParameterEntity::Source {
                declaration: declaration.declaration,
            }
            .id()
        };
        let formal = owner_formal("api");
        assert!(links.rows().iter().any(|r| r.entity == formal
            && parameters.rows().iter().any(|p| {
                p.id() == r.parameter
                    && shapes.rows().iter().any(|s| {
                        s.id() == p.shape && s.name.as_ref().is_some_and(|n| n.as_str() == "value")
                    })
            })));
        let default_formal = owner_formal("defaulted");
        let computed_formal = owner_formal("computed");
        let relay_formal = owner_formal("relay");
        drop(lease);
        let native = service.prepare_native().await.unwrap();
        let mut request = InspectValuePathsRequest {
            member: candidate.member,
            analysis: candidate.analysis,
            inputs: vec![ExactInputBinding {
                formal,
                value: ExactScalar::String {
                    value: "ready".into(),
                },
            }],
            assumptions: Assumptions::default(),
            page: PageRequest {
                size: 100,
                ..Default::default()
            },
        };
        let other_service = GenerationService::admit(model.clone(), &config, Some(generation))
            .await
            .unwrap();
        let execution = service.execution().await.unwrap();
        let result = native.inspect(&execution, request.clone()).await.unwrap();
        assert_eq!(result.generation.0, *generation.bytes());
        let other_execution = other_service.execution().await.unwrap();
        assert!(
            matches!(
                native.inspect(&other_execution, request.clone()).await,
                Err(Error::Contract)
            ),
            "same generation from a replacement guard cannot supply the original grant"
        );
        drop(other_execution);
        other_service.shutdown().await.unwrap();
        if profile == "catalog" {
            assert!(matches!(
                result.paths.availability,
                Availability::NotRequested {}
            ));
            assert!(result.paths.items.is_empty());
        } else {
            assert!(!result.paths.items.is_empty(), "real admitted finite paths");
            assert!(
                result
                    .paths
                    .items
                    .iter()
                    .any(|p| p.exact == ExactOutcome::RefutedPathUnderModel
                        && p.verdict == obligation::Verdict::RefutedUnderModel),
                "{:#?}",
                result.paths.items
            );
            assert!(result.paths.items.iter().all(|p| !p.proof.is_empty()));
            assert!(
                result
                    .paths
                    .items
                    .iter()
                    .any(|p| p.exact == ExactOutcome::CompatibleUnderMayModel)
            );
            // Independent origin control: the same original observation/condition can justify
            // distinct canonical finite paths. Request restriction identities must preserve both.
            let siblings=result.paths.items.iter().enumerate().find_map(|(index,left)|result.paths.items[index+1..].iter().find_map(|right|{
                let shared_observation=left.proof.iter().any(|reference|reference.relation.as_str()==flow::FlowTestLeafObservation::NAME && right.proof.contains(reference));
                (left.exact==ExactOutcome::RefutedPathUnderModel && right.exact==ExactOutcome::RefutedPathUnderModel
                    && left.original_condition==right.original_condition && left.path!=right.path && shared_observation).then_some((left,right))
            })).unwrap_or_else(||panic!("actual producer preserves distinct finite origins sharing an original observation: {:#?}",result.paths.items));
            assert_ne!(
                siblings.0.restricted_result, siblings.1.restricted_result,
                "restricted result identity retains each canonical origin"
            );
            for (name, formal, reason) in [
                (
                    "defaulted",
                    default_formal,
                    obligation::ObligationKind::EntryValueUnknown,
                ),
                (
                    "computed",
                    computed_formal,
                    obligation::ObligationKind::ConditionTransferUnsupported,
                ),
            ] {
                let candidate = found
                    .supported
                    .items
                    .iter()
                    .chain(&found.unresolved.items)
                    .find(|r| r.name.as_str().ends_with(name))
                    .unwrap();
                let boundary = native
                    .inspect(
                        &execution,
                        InspectValuePathsRequest {
                            member: candidate.member,
                            analysis: candidate.analysis,
                            inputs: vec![ExactInputBinding {
                                formal,
                                value: ExactScalar::Integer {
                                    decimal: "1".into(),
                                },
                            }],
                            assumptions: Assumptions::default(),
                            page: PageRequest {
                                size: 100,
                                ..Default::default()
                            },
                        },
                    )
                    .await
                    .unwrap();
                assert!(
                    !boundary.paths.items.is_empty(),
                    "actual canonical {name} paths remain visible"
                );
                assert!(
                    boundary
                        .paths
                        .items
                        .iter()
                        .all(|p| p.verdict == obligation::Verdict::Unknown
                            && p.reason.0 == Some(reason)
                            && p.restricted_result.0.is_none()),
                    "{name}: {:#?}",
                    boundary.paths.items
                );
            }
            let relay = found
                .supported
                .items
                .iter()
                .chain(&found.unresolved.items)
                .find(|r| r.name.as_str().ends_with("relay"))
                .unwrap();
            let relayed = native
                .inspect(
                    &execution,
                    InspectValuePathsRequest {
                        member: relay.member,
                        analysis: relay.analysis,
                        inputs: vec![ExactInputBinding {
                            formal: relay_formal,
                            value: ExactScalar::String {
                                value: "ready".into(),
                            },
                        }],
                        assumptions: Assumptions::default(),
                        page: PageRequest {
                            size: 100,
                            ..Default::default()
                        },
                    },
                )
                .await
                .unwrap();
            assert!(
                !relayed.paths.items.is_empty(),
                "finite published caller summaries/continuations"
            );
            // Independent physical lookup of every returned original proof and condition.
            // SQL identifiers are the canonical generation schema and model-declared relation; values are bound.
            for packet in &result.paths.items {
                let references = packet.proof.iter().chain(std::iter::once(&packet.path));
                for reference in references {
                    assert!(
                        model
                            .relations()
                            .iter()
                            .any(|r| r.name() == reference.relation.as_str())
                    );
                    let sql = format!(
                        "SELECT EXISTS(SELECT 1 FROM \"{}\".\"{}\" WHERE generation_id=$1 AND id=$2)",
                        generation.schema(),
                        reference.relation.as_str()
                    );
                    let exists: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
                        .bind(generation.bytes().as_slice())
                        .bind(reference.row.as_slice())
                        .fetch_one(&db.superuser)
                        .await
                        .unwrap();
                    assert!(
                        exists,
                        "original proof row exists in the same generation: {reference:?}"
                    );
                }
                let sql = format!(
                    "SELECT EXISTS(SELECT 1 FROM \"{}\".\"conditions\" WHERE generation_id=$1 AND id=$2)",
                    generation.schema()
                );
                let exists: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
                    .bind(generation.bytes().as_slice())
                    .bind(packet.original_condition.bytes().as_slice())
                    .fetch_one(&db.superuser)
                    .await
                    .unwrap();
                assert!(exists, "restriction retained the original stored condition");
            }
            request.page.size = 1;
            let page = native.inspect(&execution, request.clone()).await.unwrap();
            if let Some(cursor) = page.paths.continuation.0 {
                request.page.cursor = Optional::supplied(cursor);
                let next = native.inspect(&execution, request.clone()).await.unwrap();
                assert_ne!(page.paths.items[0].path, next.paths.items[0].path);
                request.inputs[0].value = ExactScalar::None {};
                assert!(
                    native.inspect(&execution, request.clone()).await.is_err(),
                    "cursor binds exact input"
                );
            }
        }
        request.analysis = serde_json::from_value(serde_json::json!(vec![255u8; 16])).unwrap();
        assert!(matches!(
            native.inspect(&execution, request).await,
            Err(Error::Contract)
        ));
        let capability_result = service
            .capability(
                &execution,
                &GetCapabilityRequest {
                    capability,
                    page: PageRequest {
                        expanded: true,
                        ..Default::default()
                    },
                },
            )
            .await
            .unwrap();
        assert!(
            !capability_result.capability.rendered.as_str().is_empty(),
            "actual authored rendering and assertions hydrate"
        );
        drop(execution);
        // Execute the current Python transport against this still-live, actual compiler generation.
        let python_root = std::env::var_os("LCTX_T0_PYTHON_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        let python_paths = std::env::join_paths([
            python_root.join("python/lctx_mcp/src"),
            python_root.join("python/lctx_storage/python"),
            python_root.join("python/lctx_semantics/python"),
        ])
        .unwrap();
        let transport=std::process::Command::new("uv").current_dir(&python_root)
            .args(["run","--no-sync","pytest","-q","python/lctx_mcp/tests/current_transport.py"])
            .env("PYTHONPATH",python_paths)
            .env("LCTX_SERVING_TEST_CONFIG",dir.path().join("postgres-serving.json"))
            .env("LCTX_SERVING_TEST_GENERATION",generation.hex())
            .env("LCTX_SERVING_TEST_LIBRARY","demo").env("LCTX_SERVING_TEST_PROFILE",profile)
            .env("LCTX_SERVING_TEST_CAPABILITY",capability.hex())
            .env("LCTX_SERVING_TEST_ARTIFACT",serde_json::to_string(&artifact).unwrap())
            .env("LCTX_SERVING_TEST_NATIVE",serde_json::json!({"member":candidate.member,"analysis":candidate.analysis,"formal":formal}).to_string()).output().unwrap();
        println!(
            "{profile} transport: {}",
            String::from_utf8_lossy(&transport.stdout)
        );
        if !transport.status.success() {
            transport_failures.push(format!(
                "{profile} transport: {}\n{}",
                String::from_utf8_lossy(&transport.stdout),
                String::from_utf8_lossy(&transport.stderr)
            ));
        }
        if profile == "behavioral" {
            // The identifier is a declared relation and the canonical generation schema.
            let sql = format!(
                "UPDATE \"{}\".\"{}\" SET operand=NULL",
                generation.schema(),
                flow::FlowTestLeafObservation::NAME
            );
            assert!(
                sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                    .execute(&db.superuser)
                    .await
                    .unwrap()
                    .rows_affected()
                    > 0
            );
            assert!(
                service.prepare_native().await.is_err(),
                "canonical content receipt rejects a changed original operand before reuse"
            );
        }
        drop(native);
        drop(catalog);
        service.shutdown().await.unwrap();
        store.retire(generation).await.unwrap();
    }
    assert!(
        transport_failures.is_empty(),
        "{}",
        transport_failures.join("\n")
    );
}
