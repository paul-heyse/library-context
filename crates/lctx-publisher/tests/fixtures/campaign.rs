//! Shared test proof for an explicitly owned disposable maintenance campaign.
use lctx_model::domain::ModelError;
use lctx_native::RuntimeConfig;

pub(crate) fn require_campaign_maintenance(config: &RuntimeConfig) -> Result<(), ModelError> {
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
assert record.get("kind") == "campaign", "publication control requires an owned campaign"
assert record.get("purpose") == "surrealdb-lifecycle", "publication control requires lifecycle campaign purpose"
assert installation.id == os.environ["LCTX_SURREAL_INSTALLATION_ID"], "campaign attachment installation identity"
endpoint, namespace, generation, database = sys.argv[1:]
assert installation.grpc_endpoint == endpoint, "campaign runtime endpoint"
assert record["namespace"] == namespace, "campaign runtime namespace"
assert bytes(record["service_generation"]).hex() == generation, "campaign runtime generation"
assert os.environ["LCTX_SURREAL_GENERATION"] == generation, "campaign attachment generation"
assert database in record["databases"], "campaign runtime database scope"
assert Path(os.environ["LCTX_SURREAL_INSTALLER_CONFIG"]).resolve() == installation.runtime_path(database, installer=True).resolve(), "campaign installer path"
assert service.maintenance_owned(installation), "publication control requires live matching host maintenance ownership"
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
            "publication control owned campaign maintenance proof refused: {}",
            String::from_utf8_lossy(&proof.stderr)
        )));
    }
    Ok(())
}
