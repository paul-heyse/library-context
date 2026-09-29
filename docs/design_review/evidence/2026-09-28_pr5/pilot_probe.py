"""Fresh PR5 publications and bounded MCP journeys; no accuracy evaluation."""
import asyncio
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / "scripts"))
from postgres_bootstrap import write_secret
from postgres_test_support import database
from packet_probe import check


def main():
    output = (ROOT / sys.argv[1]).resolve()
    tasks = (ROOT / sys.argv[2]).resolve()
    output.mkdir(parents=True, exist_ok=True)
    cli = str(ROOT / "target/release/lctx")
    operator = Path.home() / ".config/library-context/postgres.json"
    results = []
    # Use the existing full-spec cache for compilation; serving is independently imported.
    for profile in ["catalog", "behavioral"]:
        inputs = {"cli_sha256": hashlib.file_digest(open(cli, "rb"), "sha256").hexdigest(),
                  "profile": profile, "embedder": "vllm",
                  "tasks": {t: hashlib.sha256((tasks / f"{t}.json").read_bytes()).hexdigest()
                            for t in ["programmatic", "cli"]}}
        record = output / f"{profile}-inputs.json"
        log = output / f"{profile}.log"
        recorded = json.loads(record.read_text()) if record.exists() else None
        same_input = recorded is not None and all(recorded[k] == inputs[k] for k in ["profile", "embedder", "tasks"])
        # Explicit bounded serving repair only: the canonical compiler sources must be unchanged.
        same_compiler = recorded == inputs or "--reuse-published" in sys.argv
        reusable = same_input and same_compiler and log.exists() and any(
            line.startswith("generation /") for line in log.read_text().splitlines())
        if not reusable:
            command = [cli, "--database-config", str(operator), "compile", "fastmcp",
                       "--store", str(output / profile / "store"), "--generations",
                       str(output / profile / "generations"), "--profile", profile, "--embedder", "vllm"]
            for task in inputs["tasks"]:
                command.extend(["--evidence-observations", str(tasks / f"{task}.json")])
            with log.open("w") as stream:
                result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT)
            if result.returncode:
                raise RuntimeError(f"{profile} compile failed; inspect {log}")
            record.write_text(json.dumps(inputs, indent=2) + "\n")
        generation = Path(next(line.split()[1] for line in log.read_text().splitlines()
                               if line.startswith("generation /")))
        manifest = json.loads((generation / "MANIFEST.json").read_text())
        results.append({"profile": profile, "generation": str(generation),
                        "projection_generation": manifest["projection_generation"],
                        "canonical_cli_sha256": (recorded if reusable else inputs)["cli_sha256"],
                        "serving_cli_sha256": inputs["cli_sha256"],
                        "capabilities": manifest["capabilities"]})
        print(f"{profile}: compiled", flush=True)
    with tempfile.TemporaryDirectory(prefix="lctx-pr5-journeys-") as tmp:
        temp = Path(tmp)
        with database(temp) as (serving, _, call, role, _):
            runtime = temp / "postgres.json"
            runtime.write_text(json.dumps({**json.loads(runtime.read_text()), "statement_timeout_seconds": 300,
                                   "acquire_timeout_seconds": 30}))
            serving.write_text(json.dumps({**role, "statement_timeout_seconds": 120, "acquire_timeout_seconds": 30}))
            importer = temp / "postgres-importer.json"
            write_secret(importer, {**role, "role": "importer", "statement_timeout_seconds": 300,
                                    "url": role["url"].replace("lctx_serving:", "lctx_importer:")})
            for row in results:
                generation = Path(row["generation"])
                publisher = [cli, "serving", "--importer-config", str(importer)]
                call([*publisher, "import-bundle", "--bundle", str(generation), "--artifacts", str(output / "artifacts")])
                call([*publisher, "select", "--library", "fastmcp", "--generation", row["projection_generation"]])
                row["journeys"] = [asyncio.run(check(generation, serving, route)) for route in ["none", "vllm"]]
                row["outcome"] = "passed"
                (output / "receipt.json").write_text(json.dumps(results, indent=2) + "\n")
                print(f"{row['profile']}: both MCP routes passed", flush=True)
            # Reconstruction is tested using a temporary archive, then that archive is deleted.
            archive = temp / "current.dump"
            call([sys.executable, "scripts/postgres_backup.py", "backup", str(archive), "--config", str(runtime)])
            restored = call([sys.executable, "scripts/postgres_backup.py", "restore-drill", str(archive)])
            (output / "restore.json").write_text(restored.stdout)
            assert json.loads(restored.stdout)["outcome"] == "passed"
            print("both profiles: reconstruction passed", flush=True)


if __name__ == "__main__":
    main()
