"""Challenge published normalized scan admission and a measured insufficient compile budget."""

import json
import os
import subprocess
import time

from pilots import CLI, HERE, RAW, ROOT, sql


def inventory() -> bytes:
    return subprocess.check_output([str(CLI), "generation", "list"], cwd=ROOT)


def main() -> None:
    before = inventory()
    selection = sql("SELECT encode(generation_id,'hex') FROM lctx_model_store.selection")
    (RAW / "refusal-generations-before.json").write_bytes(before)
    pilot = json.loads((RAW / "normalized-catalog.stdout").read_text())
    # Choose a budget below the observed peak, capped to exercise an early provider refusal.
    memory = min(256 << 20, pilot["peak_reservation_bytes"] // 2)
    scan_sql = "SELECT a.id,b.id,c.id FROM callable_entities a JOIN callable_entities b ON a.id=b.id JOIN callable_entities c ON b.id=c.id LIMIT 1"
    cases = {
        "scan-refusal": ["query", "--generation", pilot["generation"], scan_sql],
        "memory-refusal": ["compile", "fastmcp", "--through", "normalized", "--profile", "catalog", "--memory-bytes", str(memory)],
    }
    receipt = {"observed_normalized_peak_bytes": pilot["peak_reservation_bytes"], "refusal_budget_bytes": memory}
    for label, args in cases.items():
        started = time.monotonic()
        result = subprocess.run([str(CLI), *args], cwd=ROOT, env={**os.environ, "LCTX_LOG": "warn,cpg_core::facts=info,cpg_core::normalize=info"}, capture_output=True, check=False)
        (RAW / f"{label}.stdout").write_bytes(result.stdout)
        (RAW / f"{label}.stderr").write_bytes(result.stderr)
        assert result.returncode != 0, result.stdout.decode()
        detail = result.stderr.decode()
        assert ("memory reservation refused" in detail) if label == "memory-refusal" else ("scan" in detail and ("budget" in detail or "demand" in detail or "slot" in detail)), detail
        assert before == inventory(), "a refusal changed the generation registry"
        assert selection == sql("SELECT encode(generation_id,'hex') FROM lctx_model_store.selection")
        check = subprocess.run([str(CLI), "store", "check"], cwd=ROOT, capture_output=True, check=False)
        assert check.returncode == 0, check.stderr.decode()
        receipt[label] = {"command": ["target/release/lctx", *args], "exit": result.returncode, "elapsed_seconds": time.monotonic() - started, "expected_refusal": "passed", "unchanged_registry_and_selection": "passed", "store_check": "passed"}
        (HERE / "refusal-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps({"label": label, **receipt[label]}), flush=True)
    (RAW / "refusal-generations-after.json").write_bytes(inventory())


if __name__ == "__main__":
    main()
