"""Compile and serve both PR3 profiles in disposable PG with editable native modules."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path.cwd()
sys.path[:0] = [str(ROOT / "scripts")]
from postgres_test_support import database
from postgres_expand import write_secret

OUTPUT = ROOT / sys.argv[1]
TASKS = ROOT / (sys.argv[2] if len(sys.argv) > 2 else "build/pr3-task-checks-final")


def main():
    OUTPUT.mkdir(exist_ok=False)
    cli = str(ROOT / "target/release/lctx")
    results = []
    with tempfile.TemporaryDirectory(prefix="lctx-pr3-pilots-") as tmp:
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
            for embedder in ["vllm"]:
                for profile in ["catalog", "behavioral"]:
                    name = f"{profile}-{embedder}"
                    command = [cli, "--database-config", str(runtime), "compile", "fastmcp", "--store", str(OUTPUT / name / "store"), "--embedder", embedder, "--profile", profile, "--generations", str(OUTPUT / name / "generations")]
                    for task in ["programmatic", "cli"]:
                        command.extend(["--evidence-observations", str(TASKS / f"{task}.json")])
                    started = time.monotonic()
                    log = OUTPUT / f"{name}.log"
                    with log.open("x") as stream:
                        result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT)
                    if result.returncode:
                        raise RuntimeError(f"{name} compile failed; see {log}")
                    generation = Path(next(line.split()[1] for line in log.read_text().splitlines() if line.startswith("generation /")))
                    manifest = json.loads((generation / "MANIFEST.json").read_text())
                    call([cli, "serving", "--importer-config", str(importer), "import-bundle", "--bundle", str(generation), "--artifacts", str(OUTPUT / "artifacts")])
                    call([cli, "serving", "--importer-config", str(importer), "select", "--library", "fastmcp", "--generation", manifest["projection_generation"]])
                    smoke = subprocess.run([sys.executable, "-m", "lctx_mcp.smoke", str(generation), "--embedder", embedder, "--config", str(serving)], capture_output=True, text=True)
                    (OUTPUT / f"{name}-smoke.log").write_text(smoke.stdout + smoke.stderr)
                    if smoke.returncode:
                        raise RuntimeError(f"{name} smoke failed")
                    receipt = {"profile": profile, "embedder": embedder, "outcome": "passed", "generation": str(generation), "projection_generation": manifest["projection_generation"], "seconds": time.monotonic() - started, "capabilities": manifest["capabilities"]}
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
