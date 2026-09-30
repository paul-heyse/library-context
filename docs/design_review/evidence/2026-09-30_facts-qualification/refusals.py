"""Challenge facts budgets on the pinned pilot without changing published generations."""

import json
import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
CLI = ROOT / "target/release/lctx"


def inventory() -> bytes:
    return subprocess.check_output([str(CLI), "generation", "list"], cwd=ROOT)


def main() -> None:
    before = inventory()
    (HERE / "raw/refusal-generations-before.json").write_bytes(before)
    receipt = {}
    for label, memory in [("capture-refusal", 65536), ("stage-refusal", 268435456)]:
        args = [
            str(CLI),
            "compile",
            "fastmcp",
            "--through",
            "facts",
            "--profile",
            "catalog",
            "--memory-bytes",
            str(memory),
        ]
        env = {**os.environ, "LCTX_LOG": "warn,cpg_core::facts=info"}
        started = time.monotonic()
        result = subprocess.run(
            ["/usr/bin/time", "-v", *args],
            cwd=ROOT,
            env=env,
            capture_output=True,
            timeout=7200,
            check=False,
        )
        (HERE / f"raw/{label}.stdout").write_bytes(result.stdout)
        (HERE / f"raw/{label}.stderr").write_bytes(result.stderr)
        assert result.returncode != 0, "the chosen budget unexpectedly admitted the full pilot"
        assert b"memory reservation refused" in result.stderr, result.stderr.decode()
        after = inventory()
        assert before == after, "a refusal changed generation registry state"
        subprocess.run([str(CLI), "store", "check"], cwd=ROOT, check=True, capture_output=True)
        row = {
            "command": ["target/release/lctx", *args[1:]],
            "exit": result.returncode,
            "elapsed_seconds": time.monotonic() - started,
            "typed_resource_refusal": "passed",
            "unchanged_generation_registry": "passed",
            "store_check": "passed",
        }
        if label == "stage-refusal":
            assert b'outcome="failed"' in result.stderr and b"facts stage" in result.stderr, (
                "no provider-stage refusal was observed"
            )
            row["provider_stage_refusal"] = "passed"
        receipt[label] = row
        (HERE / "refusal-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps({"label": label, **row}), flush=True)
    (HERE / "raw/refusal-generations-after.json").write_bytes(inventory())


if __name__ == "__main__":
    main()
