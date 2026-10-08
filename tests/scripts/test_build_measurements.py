from __future__ import annotations

import json
from pathlib import Path

import pytest

import build_measurements as bm


def write_receipt(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value) + "\n")


def captured_bytes(campaign: Path) -> dict[Path, bytes]:
    return {
        path.relative_to(campaign): path.read_bytes()
        for path in campaign.rglob("*")
        if path.is_file()
    }


def test_report_reads_historical_trials_without_changing_captures(tmp_path: Path) -> None:
    write_receipt(tmp_path / "snapshot.json", {"source_sha256": "abc"})
    write_receipt(
        tmp_path / "results/stable/trial-1/summary.json",
        {"cold-tests": {"wall_seconds": 10.0, "exit_code": 0}},
    )
    write_receipt(
        tmp_path / "results/stable/trial-2/summary.json",
        {"cold-tests": {"wall_seconds": 12.0, "exit_code": 0}},
    )
    source = tmp_path / "source/src/main.rs"
    source.parent.mkdir(parents=True)
    source.write_text("fn main() {}\n")
    before = captured_bytes(tmp_path)

    report = bm.summarize(tmp_path)

    assert report["source_sha256"] == "abc"
    assert report["measurements"]["stable"]["cold-tests"] == {
        "count": 2,
        "median_seconds": 11.0,
        "min_seconds": 10.0,
        "max_seconds": 12.0,
    }
    assert report["decision"].startswith("not_run")
    assert captured_bytes(tmp_path) == before


def test_report_preserves_paired_trial_medians_and_failure_exclusion(tmp_path: Path) -> None:
    write_receipt(tmp_path / "snapshot.json", {"source_sha256": "abc"})
    write_receipt(
        tmp_path / "results/stable/trial-1/summary.json",
        {
            "warm-tests-1": {"wall_seconds": 10.0, "exit_code": 0},
            "warm-tests-2": {"wall_seconds": 30.0, "exit_code": 0},
            "release-lctx": {"wall_seconds": 90.0, "exit_code": 1},
        },
    )
    write_receipt(
        tmp_path / "results/stable-cache/trial-1/summary.json",
        {"warm-tests-1": {"wall_seconds": 10.0, "exit_code": 0}},
    )
    write_receipt(
        tmp_path / "results/stable-cache/trial-2/summary.json",
        {"warm-tests-1": {"wall_seconds": 40.0, "exit_code": 0}},
    )

    report = bm.summarize(tmp_path)

    assert report["measurements"]["stable"]["warm-tests"]["median_seconds"] == 20.0
    assert report["measurements"]["stable-cache"]["warm-tests"]["median_seconds"] == 25.0
    assert report["comparisons"]["stable-cache"]["warm-tests"] == {
        "baseline": "stable",
        "paired_trials": 1,
        "median_wall_improvement_percent": 50.0,
        "every_pair_faster": True,
    }
    assert report["failures"] == ["stable/trial-1/release-lctx"]
    assert "release-lctx" not in report["measurements"]["stable"]


def test_report_cli_emits_historical_json(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    write_receipt(tmp_path / "snapshot.json", {"source_sha256": "abc"})
    before = captured_bytes(tmp_path)

    assert bm.main(["report", str(tmp_path)]) == 0

    output = capsys.readouterr()
    assert json.loads(output.out) == bm.summarize(tmp_path)
    assert output.err == ""
    assert captured_bytes(tmp_path) == before


@pytest.mark.parametrize(
    ("arguments", "route"),
    [
        (["run", "missing-campaign", "--variant", "obsolete", "--phase", "full"], "record"),
        (["capture", "missing-campaign"], "record"),
        (["preflight"], "doctor"),
    ],
)
def test_retired_commands_refuse_before_reading_or_creating_campaigns(
    arguments: list[str],
    route: str,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.chdir(tmp_path)

    assert bm.main(arguments) == 1

    output = capsys.readouterr()
    assert output.out == ""
    assert f"bench-builds {arguments[0]} is retired" in output.err
    assert f"just compile-profile {route}" in output.err
    assert list(tmp_path.iterdir()) == []


def test_missing_report_returns_explicit_failure(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert bm.main(["report", str(tmp_path)]) == 1

    output = capsys.readouterr()
    assert output.out == ""
    assert "build measurement report failed" in output.err
