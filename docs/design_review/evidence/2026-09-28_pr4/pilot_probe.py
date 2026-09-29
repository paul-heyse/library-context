"""Compile and serve both PR4 profiles in disposable PG with editable native modules."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path.cwd()
sys.path[:0] = [str(ROOT / "scripts")]
from postgres_test_support import database
from postgres_bootstrap import write_secret

OUTPUT = ROOT / sys.argv[1]
TASKS = ROOT / (sys.argv[2] if len(sys.argv) > 2 else "build/pr4-task-checks-final")


def main():
    OUTPUT.mkdir(exist_ok=True)
    cli = str(ROOT / "target/release/lctx")
    results = []
    with tempfile.TemporaryDirectory(prefix="lctx-pr4-pilots-") as tmp:
        config = Path(tmp)
        with database(config) as (serving, _, call, role, _):
            runtime = config / "postgres.json"
            settings = json.loads(runtime.read_text())
            settings["statement_timeout_seconds"] = 300
            settings["acquire_timeout_seconds"] = 30
            runtime.write_text(json.dumps(settings))
            serving.write_text(json.dumps({**role, "statement_timeout_seconds": 60}))
            importer = config / "postgres-importer.json"
            write_secret(importer, {**role, "role": "importer", "url": role["url"].replace("lctx_serving:", "lctx_importer:"), "statement_timeout_seconds": 300})
            for embedder in ["none"]:
                for profile in ["catalog", "behavioral"]:
                    name = f"{profile}-{embedder}"
                    command = [cli, "--database-config", str(runtime), "compile", "fastmcp", "--store", str(OUTPUT / name / "store"), "--embedder", embedder, "--profile", profile, "--generations", str(OUTPUT / name / "generations")]
                    for task in ["programmatic", "cli"]:
                        command.extend(["--evidence-observations", str(TASKS / f"{task}.json")])
                    started = time.monotonic()
                    log = OUTPUT / f"{name}.log"
                    compile_inputs = {"cli_sha256": hashlib.sha256(Path(cli).read_bytes()).hexdigest(), "profile": profile, "embedder": embedder, "tasks": {task: hashlib.sha256((TASKS / f"{task}.json").read_bytes()).hexdigest() for task in ["programmatic", "cli"]}}
                    input_receipt = OUTPUT / f"{name}-compile-inputs.json"
                    recorded_inputs = json.loads(input_receipt.read_text()) if input_receipt.exists() else None
                    same_inputs = recorded_inputs is not None and all(recorded_inputs[key] == compile_inputs[key] for key in ["profile", "embedder", "tasks"])
                    # Explicit serving-only requalification: canonical producer sources must be
                    # unchanged. Import still validates the entire current bundle and artifacts.
                    same_producer = recorded_inputs is not None and (recorded_inputs["cli_sha256"] == compile_inputs["cli_sha256"] or "--reuse-published" in sys.argv)
                    reused = same_inputs and same_producer and log.exists() and any(line.startswith("generation /") for line in log.read_text().splitlines())
                    if not reused:
                        with log.open("w") as stream:
                            result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT)
                        if result.returncode:
                            raise RuntimeError(f"{name} compile failed; see {log}")
                        input_receipt.write_text(json.dumps(compile_inputs, indent=2) + "\n")
                    generation = Path(next(line.split()[1] for line in log.read_text().splitlines() if line.startswith("generation /")))
                    manifest = json.loads((generation / "MANIFEST.json").read_text())
                    call([cli, "serving", "--importer-config", str(importer), "import-bundle", "--bundle", str(generation), "--artifacts", str(OUTPUT / "artifacts")])
                    call([cli, "serving", "--importer-config", str(importer), "select", "--library", "fastmcp", "--generation", manifest["projection_generation"]])
                    smoke = subprocess.run([sys.executable, "-m", "lctx_mcp.smoke", str(generation), "--embedder", embedder, "--config", str(serving)], capture_output=True, text=True)
                    (OUTPUT / f"{name}-smoke.log").write_text(smoke.stdout + smoke.stderr)
                    if smoke.returncode:
                        raise RuntimeError(f"{name} smoke failed")
                    from retrieval_probe import check
                    import asyncio
                    retrieval = asyncio.run(check(generation, serving))
                    receipt = {"profile": profile, "embedder": embedder, "outcome": "passed", "generation": str(generation), "projection_generation": manifest["projection_generation"], "seconds": time.monotonic() - started, "reused_canonical_compile": reused, "canonical_cli_sha256": (recorded_inputs if reused else compile_inputs)["cli_sha256"], "serving_cli_sha256": compile_inputs["cli_sha256"], "capabilities": manifest["capabilities"], "retrieval": retrieval}
                    results.append(receipt)
                    (OUTPUT / "receipt.json").write_text(json.dumps(results, indent=2) + "\n")
                    print(json.dumps(receipt), flush=True)
            backup = OUTPUT / "both-profiles.dump"
            call([sys.executable, str(ROOT / "scripts/postgres_backup.py"), "backup", str(backup), "--config", str(runtime)])
            restore = call([sys.executable, str(ROOT / "scripts/postgres_backup.py"), "restore-drill", str(backup)])
            (OUTPUT / "restore.json").write_text(restore.stdout)
            print(restore.stdout, flush=True)


if __name__ == "__main__":
    main()
