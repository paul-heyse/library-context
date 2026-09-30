"""Run one or all current-model FastMCP qualification pilots; never select a generation."""

import argparse
import json
import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
CLI = ROOT / "target/release/lctx"
CONFIG = Path.home() / ".config/library-context/postgres.json"
RAW = HERE / "raw"
CASES = {
    "facts-catalog": ("facts", "catalog"),
    "normalized-catalog": ("normalized", "catalog"),
    "facts-behavioral": ("facts", "behavioral"),
    "normalized-behavioral": ("normalized", "behavioral"),
    "normalized-behavioral-repeat": ("normalized", "behavioral"),
}


def sql(query: str) -> str:
    return subprocess.check_output(
        ["psql", "-h", "127.0.0.1", "-U", "lctx_superuser", "-d", "lctx", "-X", "-A", "-t", "-v", "ON_ERROR_STOP=1", "-c", query],
        text=True,
    ).strip()


def run(label: str) -> None:
    frontier, profile = CASES[label]
    args = [str(CLI), "--database", str(CONFIG), "compile", "fastmcp", "--through", frontier, "--profile", profile]
    started = time.monotonic()
    env = {**os.environ, "LCTX_LOG": "warn,cpg_core::facts=info,cpg_core::normalize=info"}
    samples = []
    with (RAW / f"{label}.stdout").open("w") as output, (RAW / f"{label}.stderr").open("w") as errors:
        process = subprocess.Popen(["/usr/bin/time", "-v", *args], cwd=ROOT, env=env, stdout=output, stderr=errors)
        while process.poll() is None:
            samples.append({"elapsed_seconds": time.monotonic() - started, "connections": json.loads(sql("SELECT coalesce(json_agg(t),'[]'::json) FROM (SELECT usename,state,application_name,count(*) AS connections FROM pg_stat_activity WHERE datname='lctx' AND pid<>pg_backend_pid() GROUP BY usename,state,application_name ORDER BY usename,state,application_name) t"))})
            time.sleep(2)
    (RAW / f"{label}-connections.json").write_text(json.dumps(samples, indent=2) + "\n")
    receipt_path = HERE / "pilot-receipt.json"
    receipt = json.loads(receipt_path.read_text()) if receipt_path.exists() else {}
    receipt[label] = {"command": ["target/release/lctx", "--database", "~/.config/library-context/postgres.json", *args[3:]], "exit": process.returncode, "elapsed_seconds": time.monotonic() - started, "outcome": "passed" if process.returncode == 0 else "failed"}
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"label": label, **receipt[label]}), flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
    result = json.loads((RAW / f"{label}.stdout").read_text())
    assert result["frontier"] == frontier and result["profile"] == profile and not result["selected"]
    assert len(result["stage_measurements"]) == (5 if profile == "catalog" else 6) + (7 if frontier == "normalized" else 0)
    generation = result["generation"]
    assert len(generation) == 32 and all(c in "0123456789abcdef" for c in generation)
    details = subprocess.check_output([str(CLI), "generation", "show", generation], cwd=ROOT)
    (RAW / f"{label}-generation.json").write_bytes(details)
    relations = sql(f"SELECT json_agg(t ORDER BY stage_name,relation_name) FROM (SELECT r.stage_name,r.relation_name,r.row_count,encode(r.content_digest,'hex') AS content_digest,pg_total_relation_size(c.oid) AS storage_bytes FROM lctx_model_store.stage_receipts r JOIN pg_namespace n ON n.nspname='lctx_g{generation}' JOIN pg_class c ON c.relnamespace=n.oid AND c.relname=r.relation_name WHERE r.generation_id=decode('{generation}','hex')) t")
    (RAW / f"{label}-relations.json").write_text(relations + "\n")
    if label == "normalized-behavioral-repeat":
        first = json.loads((RAW / "normalized-behavioral.stdout").read_text())
        assert result["content_digest"] == first["content_digest"]
        receipt["normalized_behavioral_repeat_content"] = "passed"
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("label", choices=[*CASES, "all"])
    label = parser.parse_args().label
    for name in CASES if label == "all" else [label]:
        run(name)


if __name__ == "__main__":
    main()
