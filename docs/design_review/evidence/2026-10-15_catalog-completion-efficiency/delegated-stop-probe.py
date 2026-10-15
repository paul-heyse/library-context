"""Reveal delegated-stop selection with no processes signalled or services accessed.

This is a pure branch control of the actual request_stop implementation. Process
liveness and signalling are substituted; temporary receipts are owned by this
invocation and removed by TemporaryDirectory. It establishes no host drainage.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))

import harness  # noqa: E402 — the standalone probe adds the repository's scripts directory above.
import run_delegation as delegated  # noqa: E402


def case(*, completed_first: bool) -> dict:
    with TemporaryDirectory(prefix="lctx-review-delegated-stop-") as scratch:
        run = Path(scratch) / "review-run"
        entries = run / delegated.ENTRIES
        entries.mkdir(parents=True)
        parent = harness.ProcessIdentity.of().to_json()
        other = {**parent, "pid": parent["pid"] + 10_000_000}
        record = {
            "owner": parent,
            "delegation": {"schema": 1, "path": delegated.SCOPE},
        }
        (run / delegated.SCOPE).write_text(
            json.dumps({"schema": 1, "run": run.name, "owner": parent, "accepting": True})
        )
        order = [("a", True), ("b", False)] if completed_first else [("a", False), ("b", True)]
        live = None
        for name, completed in order:
            directory = entries / name
            directory.mkdir()
            row = {
                "schema": 1,
                "id": name,
                "run": run.name,
                "run_owner": parent,
                "owner": other,
                "kind": "verification-evidence",
                "child": None,
                "spawning": False,
                "ended": "completed-before-stop" if completed else None,
                "cleanup": {"status": "confirmed" if completed else "unknown"},
            }
            (directory / "record.json").write_text(json.dumps(row))
            if not completed:
                live = directory
        assert live is not None
        signals = []

        def signal(owner, lock, signum):
            signals.append(signum.name)
            return True

        with (
            patch.object(harness.ProcessIdentity, "alive", return_value=True),
            patch.object(delegated, "lock_held", side_effect=lambda path: path.parent == live),
            patch.object(delegated, "_signal_owner", side_effect=signal),
        ):
            for now in (100.0, 110.0, 120.0):
                with patch.object(delegated.time, "monotonic", return_value=now):
                    delegated.request_stop(run, record, grace=5.0)
        return {
            "completed_first": completed_first,
            "requests_over_three_polls": signals,
            "live_request_persisted": (live / "recovery.json").exists(),
        }


if __name__ == "__main__":
    blocked = case(completed_first=True)
    control = case(completed_first=False)
    assert blocked["requests_over_three_polls"] == []
    assert blocked["live_request_persisted"] is False
    assert control["requests_over_three_polls"][0] == "SIGTERM"
    assert control["live_request_persisted"] is True
    print(
        json.dumps({"completed_before_live": blocked, "live_before_completed": control}, indent=2)
    )
