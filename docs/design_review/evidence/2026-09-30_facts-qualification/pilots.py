"""Execute Q's pinned facts pilots; no selection and no analyzed-code execution."""

import json
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
CLI = ROOT / "target/release/lctx"
RAW = HERE / "raw"
RAW.mkdir(exist_ok=True)


def run(label: str, arguments: list[str]) -> dict:
    started = time.monotonic()
    with (RAW / f"{label}.stdout").open("w") as output:
        with (RAW / f"{label}.stderr").open("w") as errors:
            result = subprocess.run(
                ["/usr/bin/time", "-v", str(CLI), *arguments],
                cwd=ROOT,
                stdout=output,
                stderr=errors,
                timeout=7200,
                check=False,
            )
    row = {
        "command": [str(CLI.relative_to(ROOT)), *arguments],
        "exit": result.returncode,
        "elapsed_seconds": time.monotonic() - started,
        "outcome": "passed" if result.returncode == 0 else "failed",
    }
    print(json.dumps({"label": label, **row}), flush=True)
    return row


def main() -> None:
    receipt = {}
    for label, args in [
        ("reset", ["store", "reset", "--confirm", "lctx"]),
        ("check-before", ["store", "check"]),
        ("catalog", ["compile", "fastmcp", "--through", "facts", "--profile", "catalog"]),
        (
            "behavioral",
            ["compile", "fastmcp", "--through", "facts", "--profile", "behavioral"],
        ),
        (
            "behavioral-repeat",
            ["compile", "fastmcp", "--through", "facts", "--profile", "behavioral"],
        ),
        ("check-after", ["store", "check"]),
        ("generations", ["generation", "list"]),
    ]:
        receipt[label] = run(label, args)
        (HERE / "pilot-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        if receipt[label]["exit"]:
            raise SystemExit(receipt[label]["exit"])
    catalog = json.loads((RAW / "catalog.stdout").read_text())
    first = json.loads((RAW / "behavioral.stdout").read_text())
    repeat = json.loads((RAW / "behavioral-repeat.stdout").read_text())
    assert all(row["frontier"] == "facts" and not row["selected"] for row in [catalog, first, repeat])
    assert first["content_digest"] == repeat["content_digest"]
    assert catalog["content_digest"] != first["content_digest"]
    receipt["content_equality"] = "passed"
    receipt["no_automatic_selection"] = "passed"
    receipt["scope"] = "FastMCP 4.0.5 full locked closure and declared pinned corpus; facts only"
    (HERE / "pilot-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
