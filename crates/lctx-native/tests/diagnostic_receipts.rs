//! Deliberate request failure on an explicitly owned lifecycle campaign. Its native
//! failure remains protected until the service owner acknowledges whole-target discard.
use lctx_model::domain::{ContentHash, completion::Completion};
use lctx_native::{RuntimeConfig, control, signal::SignalRegistration};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use surrealdb::types::Variables;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "deliberate native failure; requires owned lifecycle campaign and subsequent whole-target disposal"]
async fn actual_native_request_failure_persists_through_host_observer() {
    let installation_path = PathBuf::from(
        std::env::var_os("LCTX_SURREAL_SERVICE_CONFIG").expect("explicit owned campaign"),
    );
    let installation: serde_json::Value =
        serde_json::from_slice(&std::fs::read(installation_path).unwrap()).unwrap();
    assert_eq!(
        installation["kind"], "campaign",
        "never run destructive/failed-owner control against shared storage"
    );
    assert_eq!(installation["purpose"], "surrealdb-lifecycle");
    let cfg = RuntimeConfig::read(Path::new(
        &std::env::var_os("LCTX_COMPILER_RUNTIME_CONFIG").unwrap(),
    ))
    .unwrap();
    let owner = SignalRegistration::retry(
        "native diagnostic request control",
        control::RetryCancellation::default(),
    )
    .unwrap();
    let client = lctx_compiler_store::compiler::check_installation(&cfg)
        .await
        .unwrap();
    let committed_sql = "RETURN NONE";
    let committed_bindings = Variables::new();
    let committed = control::NativeRequest::issue(
        &client,
        ContentHash::of(&serde_json::to_vec(&(committed_sql, &committed_bindings)).unwrap()),
    )
    .await
    .unwrap();
    control::execute_request(&client, committed, None, committed_sql, committed_bindings)
        .await
        .unwrap();
    let sql = "THROW 'lifecycle diagnostic request control'";
    let bindings = Variables::new();
    let request = control::NativeRequest::issue(
        &client,
        ContentHash::of(&serde_json::to_vec(&(sql, &bindings)).unwrap()),
    )
    .await
    .unwrap();
    let error = control::execute_request(&client, request, None, sql, bindings)
        .await
        .unwrap_err();
    let projected = lctx_native::diagnostic::failure("controlled native request", &error);
    assert!(projected.causes.iter().any(|cause| {
        cause
            .identities
            .iter()
            .any(|identity| identity.value == request.operation.hex())
    }));
    let mut completion = Completion::default();
    completion.committed("native request", committed.operation.hex());
    completion.step("controlled native request", Err(error));
    completion.step(
        "native diagnostic session invalidate",
        client
            .invalidate()
            .await
            .map_err(lctx_model::domain::ModelError::codec),
    );
    owner.complete(&completion).await.unwrap();

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let observed = Command::new(root.join(".venv/bin/python"))
        .env("PYTHONPATH", root.join("scripts"))
        .args(["-c", "import json,os; from pathlib import Path; import surrealdb_fixture as f; d=Path(os.environ['LCTX_NATIVE_DRAIN_DIRECTORY']).parent; print(json.dumps(f.native_completion(d,f.read_json(d/'record.json'))))"])
        .output().unwrap();
    assert!(
        observed.status.success(),
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(
        body["status"], "unknown",
        "failure cannot be rewritten as successful cleanup"
    );
    let owner = body["owners"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["owner"] == "native diagnostic request control")
        .unwrap();
    assert_eq!(
        owner["completion"]["committed"][0]["identity"],
        committed.operation.hex()
    );
    let diagnostic = &owner["failure_diagnostics"][0];
    assert_eq!(diagnostic["status"], "typed");
    let cause = diagnostic["diagnostic"]["causes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cause| cause["code"] == "native_request")
        .unwrap();
    assert_eq!(cause["identities"][0]["value"], request.operation.hex());
    assert_eq!(cause["identities"][1]["value"], request.request.hex());
    assert_eq!(
        cause["issuance"]["generation"],
        request.issuance.generation.hex()
    );
    assert_eq!(cause["issuance"]["era"], request.issuance.era);
    assert_eq!(cause["inspect"], "native_request");
}
