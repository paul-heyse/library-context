//! The store, generation and query commands driven through the binary against a disposable real
//! PostgreSQL 18 with protected (0600) configurations (cutover plan P1.11). Exit status: 0 ok,
//! 1 error, 2 refused, 3 unavailable.
use lctx_model::domain::{
    Record, input::Package, model, stages::Profile, transfer::local::TransferKey,
};
use lctx_postgres::generations::GenerationStore;
use lctx_postgres::testing::{
    DisposableDatabase, Harness,
    fixtures::{Facts, budget},
};
use std::{path::Path, process::Command, sync::Arc};

struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}
fn lctx(args: &[&str], config: Option<&Path>, discovered: Option<&Path>) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lctx"));
    command
        .env_remove("LCTX_DATABASE_CONFIG")
        .env("HOME", "/nonexistent");
    if let Some(config) = config {
        command.arg("--database").arg(config);
    }
    if let Some(discovered) = discovered {
        command.env("LCTX_DATABASE_CONFIG", discovered);
    }
    let output = command.args(args).output().expect("the lctx binary runs");
    Run {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}
fn expect(run: Run, status: i32, what: &str) -> Run {
    assert_eq!(
        run.status, status,
        "{what}: stdout {} stderr {}",
        run.stdout, run.stderr
    );
    run
}

#[tokio::test]
async fn store_generation_and_query_commands() {
    let db = DisposableDatabase::start().await;
    let dir = tempfile::tempdir().unwrap();
    db.write_configs(dir.path()).unwrap();
    let config = dir.path().join("postgres.json");
    let cfg = Some(config.as_path());
    // Install is explicit and idempotent; check is clean after it and refuses before it.
    expect(
        lctx(&["store", "check"], cfg, None),
        2,
        "check before install",
    );
    expect(lctx(&["store", "install"], cfg, None), 0, "install");
    expect(
        lctx(&["store", "install"], cfg, None),
        0,
        "repeated install",
    );
    expect(
        lctx(&["store", "check"], cfg, None),
        0,
        "check after install",
    );
    // Generations come from attempts (P2's compile drives the same library path).
    let store = GenerationStore::open(db.owner.clone(), Arc::new(model().unwrap()))
        .await
        .unwrap();
    let facts = Facts::new();
    let published = facts.published(&store, db.writer.clone()).await;
    let mut conformance = Harness::begin_empty_conformance(
        &store,
        db.writer.clone(),
        Profile::Catalog,
        budget(),
        vec![Package {
            name: "example".into(),
        }],
    )
    .await
    .unwrap();
    conformance.seal().await.unwrap();
    conformance.validate(&budget()).await.unwrap();
    conformance.publish().await.unwrap();
    let conformance = conformance.generation();
    // The reader lists and shows.
    let listed: serde_json::Value =
        serde_json::from_str(&expect(lctx(&["generation", "list"], cfg, None), 0, "list").stdout)
            .unwrap();
    let ids: Vec<&str> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap())
        .collect();
    assert!(
        ids.contains(&published.hex().as_str()) && ids.contains(&conformance.hex().as_str()),
        "{listed}"
    );
    let facts_only: serde_json::Value = serde_json::from_str(
        &expect(
            lctx(&["generation", "list", "--frontier", "facts"], cfg, None),
            0,
            "filtered list",
        )
        .stdout,
    )
    .unwrap();
    assert_eq!(facts_only.as_array().unwrap().len(), 1);
    let shown: serde_json::Value = serde_json::from_str(
        &expect(
            lctx(&["generation", "show", &published.hex()], cfg, None),
            0,
            "show",
        )
        .stdout,
    )
    .unwrap();
    assert_eq!(
        (shown["state"].as_str(), shown["frontier"].as_str()),
        (Some("published"), Some("facts"))
    );
    assert!(
        shown["admission"]["families"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["family"] == "Flow" && f["availability"] == "NotRequested"),
        "{shown}"
    );
    expect(
        lctx(&["generation", "show", &"0".repeat(32)], cfg, None),
        2,
        "show an absent generation",
    );
    expect(
        lctx(&["generation", "show", "not-hex"], cfg, None),
        2,
        "an invalid id is a usage error",
    );
    // The owner selects facts only, and a selected generation cannot be retired.
    let refused = expect(
        lctx(&["generation", "select", &conformance.hex()], cfg, None),
        2,
        "select conformance",
    );
    assert!(refused.stderr.contains("frontier"), "{}", refused.stderr);
    expect(
        lctx(&["generation", "select", &published.hex()], cfg, None),
        0,
        "select facts",
    );
    expect(
        lctx(&["generation", "retire", &published.hex()], cfg, None),
        2,
        "retire the selected generation",
    );
    // Read-only SQL over a leased generation.
    let counted = expect(
        lctx(
            &[
                "query",
                "--generation",
                &published.hex(),
                "SELECT count(*) AS n FROM provider_coverage",
            ],
            cfg,
            None,
        ),
        0,
        "query",
    );
    assert!(counted.stdout.contains("| 3 "), "{}", counted.stdout);
    expect(
        lctx(
            &[
                "query",
                "--generation",
                &published.hex(),
                "CREATE TABLE leak AS SELECT 1",
            ],
            cfg,
            None,
        ),
        2,
        "DDL is refused",
    );
    let above = expect(
        lctx(
            &[
                "query",
                "--generation",
                &published.hex(),
                &format!("SELECT count(*) FROM {}", TransferKey::NAME),
            ],
            cfg,
            None,
        ),
        2,
        "above the frontier",
    );
    assert!(above.stderr.contains("frontier"), "{}", above.stderr);
    expect(
        lctx(
            &["query", "--generation", &published.hex(), "SELEC nonsense"],
            cfg,
            None,
        ),
        1,
        "a malformed query is an error, not a refusal",
    );
    // Discovery through the environment, and the retained services.
    expect(
        lctx(&["generation", "list"], None, cfg),
        0,
        "discovery via LCTX_DATABASE_CONFIG",
    );
    expect(lctx(&["runs", "list"], cfg, None), 0, "runs");
    expect(
        lctx(&["generation", "clear-selection"], cfg, None),
        0,
        "clear selection",
    );
    expect(
        lctx(&["generation", "retire", &published.hex()], cfg, None),
        0,
        "retire",
    );
    expect(
        lctx(&["compile", "fastmcp", "--through", "serving"], cfg, None),
        3,
        "serving is unavailable",
    );
    // Reset: a dry run by default, then confirmed by the database name.
    let dry = expect(lctx(&["store", "reset"], cfg, None), 2, "dry-run reset");
    assert!(dry.stdout.contains(&conformance.schema()), "{}", dry.stdout);
    expect(
        lctx(&["store", "reset", "--confirm", "postgres"], cfg, None),
        2,
        "a wrong confirmation",
    );
    expect(
        lctx(&["store", "reset", "--confirm", "lctx"], cfg, None),
        0,
        "reset",
    );
    let empty: serde_json::Value = serde_json::from_str(
        &expect(
            lctx(&["generation", "list"], cfg, None),
            0,
            "list after reset",
        )
        .stdout,
    )
    .unwrap();
    assert_eq!(empty.as_array().unwrap().len(), 0);
    expect(lctx(&["store", "check"], cfg, None), 0, "check after reset");
}

#[test]
fn model_describe_needs_no_database() {
    let json = expect(
        lctx(&["model", "describe", "--format", "json"], None, None),
        0,
        "json",
    );
    let described: serde_json::Value = serde_json::from_str(&json.stdout).unwrap();
    let relations = described["relations"].as_array().unwrap();
    assert!(
        relations
            .iter()
            .any(|r| r["name"] == "packages" && r["facts"] == true)
    );
    assert!(
        relations
            .iter()
            .any(|r| r["name"] == TransferKey::NAME && r["facts"] == false)
    );
    let text = expect(lctx(&["model", "describe"], None, None), 0, "text");
    assert!(
        text.stdout.starts_with("model ") && text.stdout.contains("packages  [facts]"),
        "{}",
        &text.stdout[..200.min(text.stdout.len())]
    );
}
