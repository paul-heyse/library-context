"""A frozen independent reference exercises the real qualification and plan evidence path."""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def test_small_fixture_reports_failed_ann_plan_gate_without_selection(
    generation, pg_serving, tmp_path
):
    requests = tmp_path / "requests.json"
    requests.write_text(
        json.dumps(
            [
                dict(name="brief", query="register tool", operations=False, stratum="unfiltered"),
                dict(name="all", query="register tool", operations=True, stratum="unfiltered"),
                dict(
                    name="broad",
                    query="remove component",
                    operations=True,
                    stratum="broad",
                    where={"kind": "method"},
                ),
                dict(
                    name="selective",
                    query="strict value",
                    operations=True,
                    stratum="selective",
                    where={"path_prefix": "pkg.controls.strict"},
                ),
            ]
        )
    )
    pack = tmp_path / "pack.json"
    pg_serving.run(
        [
            sys.executable,
            str(ROOT / "scripts/postgres_qualification_pack.py"),
            str(generation),
            str(requests),
            "--out",
            str(pack),
            "--config",
            str(pg_serving.config),
            "--embedder",
            "fake",
        ]
    )
    manifest = json.loads(pack.read_text())
    assert len(manifest["cases"]) == 4 and manifest["generation"] == pg_serving.generation
    command = [
        str(ROOT / "target/release/lctx"),
        "serving",
        "--importer-config",
        str(pg_serving.importer),
    ]
    pg_serving.run([*command, "build-hnsw", "--generation", pg_serving.generation])
    measured = subprocess.run(
        [*command, "qualify-hnsw", "--pack", str(pack), "--serving-config", str(pg_serving.config)],
        capture_output=True,
        text=True,
    )
    report = json.loads(measured.stdout)
    # A tiny relation should use its cheaper sequential plan. This is a failed ANN gate,
    # never labelled qualified merely because an index was successfully installed.
    assert measured.returncode != 0 and not report["passed"] and not report["plans_passed"]
    assert report["exact_reference_passed"]
    assert len(report["strata"]) == 7 and len(report["cases"]) == 4
    assert all(c["plans"] for c in report["cases"])
    assert all(c["routing"]["requested_route"] == "hnsw" for c in report["cases"])
    assert pg_serving.load().descriptor["policy"]["route"] == "exact"
