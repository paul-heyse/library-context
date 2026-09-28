"""Explicit operator schema011 cutover after PR3 qualification; preserves all prior generations."""
import asyncio
import json
from pathlib import Path
import subprocess
import sys

from evidence_probe import check

ROOT = Path.cwd()


def main():
    pilots = ROOT / sys.argv[1]
    output = ROOT / sys.argv[2]
    rows = json.loads((pilots / "receipt.json").read_text())
    assert {r["profile"] for r in rows} == {"catalog", "behavioral"}
    assert all(r["outcome"] == "passed" for r in rows)
    output.mkdir(exist_ok=False)
    config = Path.home() / ".config/library-context"
    cli = [str(ROOT / "target/release/lctx"), "--database-config", str(config / "postgres.json")]
    publisher = [*cli, "serving", "--importer-config", str(config / "postgres-importer.json")]
    serving = config / "postgres-serving.json"

    def call(label, command):
        result = subprocess.run(command, capture_output=True, text=True)
        (output / (label + ".log")).write_text(result.stdout + result.stderr)
        if result.returncode:
            raise RuntimeError(f"{label} failed; see protected/local cutover output")
        return result.stdout

    call("migrate", [*cli, "db", "migrate"])
    call("check", [*cli, "db", "check"])
    evidence = []
    for row in rows:
        generation = Path(row["generation"])
        name = row["profile"]
        call(name + "-import", [*publisher, "import-bundle", "--bundle", str(generation)])
        call(name + "-reconcile", [*publisher, "reconcile", "--generation", row["projection_generation"]])
        call(name + "-smoke", [sys.executable, "-m", "lctx_mcp.smoke", str(generation), "--embedder", "vllm", "--config", str(serving)])
        evidence.append(asyncio.run(check(generation, serving)))
        call(name + "-status", [*publisher, "status", "--generation", row["projection_generation"], "--verify-artifacts"])
    selected = next(r["projection_generation"] for r in rows if r["profile"] == "behavioral")
    selection = json.loads(call("select", [*publisher, "select", "--library", "fastmcp", "--generation", selected]))
    assert selection["selected"] and selection["generation"] == selected
    call("selected-status", [*publisher, "status", "--generation", selected, "--verify-artifacts"])
    archive = output / "current.dump"
    backup = [sys.executable, str(ROOT / "scripts/postgres_backup.py")]
    call("backup", [*backup, "backup", str(archive), "--config", str(config / "postgres.json")])
    restored = json.loads(call("restore", [*backup, "restore-drill", str(archive)]))
    baseline = json.loads((ROOT / "build/pr3-baseline/pre-cutover.dump.json").read_text())
    current = json.loads(archive.with_suffix(".dump.json").read_text())
    prior = {r["generation"] for r in baseline["inventory"]["generations"]}
    present = {r["generation"] for r in current["inventory"]["generations"]}
    assert prior | {r["projection_generation"] for r in rows} <= present
    assert restored["outcome"] == "passed" and restored["all_generations_preserved"]
    assert selected in restored["serving"]["selected_generations"]
    receipt = {"outcome": "passed", "selection": selection, "generations": len(present), "retained": len(prior), "restore": restored, "evidence": evidence}
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
