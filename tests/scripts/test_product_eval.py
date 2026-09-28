"""PR0 budget/freeze controls; mocked transport checks are not trial evidence."""
import argparse
import asyncio
import json
from pathlib import Path

import pytest
import product_eval as runner


def test_freeze_requires_every_frozen_input(tmp_path):
    root = runner.PROTOCOL.parent
    for name in ["protocol.json", "development.json", "freeze.json"]:
        (tmp_path / name).write_bytes((root / name).read_bytes())
    assert runner.check_freeze(tmp_path)["tasks"] == 24
    (tmp_path / "freeze.json").write_text('{"sha256":{}}')
    with pytest.raises(ValueError, match="required frozen"):
        runner.check_freeze(tmp_path)


def test_deployment_tasks_are_in_implementation(tmp_path):
    tasks = json.loads((runner.PROTOCOL.parent / "development.json").read_text())
    for task in tasks:
        task["deployment"] = task["stratum"] == "choice"
    path = tmp_path / "tasks.json"
    path.write_text(json.dumps(tasks))
    with pytest.raises(ValueError, match="deployment"):
        runner.load_tasks(path)


def test_unqualified_condition_has_persistent_blocked_receipt(tmp_path):
    args = argparse.Namespace(task="D01", condition="A", out=tmp_path / "trial",
                              condition_config=runner.PROTOCOL.parent / "condition-A.json")
    receipt = asyncio.run(runner.run_trial(args))
    assert receipt["outcome"] == "blocked"
    assert json.loads((args.out / "receipt.json").read_text()) == receipt
    assert not (args.out / "answer.txt").exists()


def test_arbitrary_cli_overrides_are_rejected(tmp_path):
    path = tmp_path / "condition.json"
    path.write_text(json.dumps({"condition": "A", "codex_config": ["model=other"]}))
    args = argparse.Namespace(task="D01", condition="A", out=tmp_path / "trial", condition_config=path)
    with pytest.raises(ValueError, match="condition must"):
        asyncio.run(runner.run_trial(args))


def test_receipt_cannot_be_overwritten(tmp_path):
    path = tmp_path / "receipt.json"
    runner.write_receipt(path, {"outcome": "blocked"})
    with pytest.raises(FileExistsError):
        runner.write_receipt(path, {"outcome": "completed"})
