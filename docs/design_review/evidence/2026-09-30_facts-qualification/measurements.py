"""Summarize the full facts-pilot resource envelope and inspect published input sizes."""

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
CLI = ROOT / "target/release/lctx"


def main() -> None:
    measured = {}
    for label in ["catalog", "behavioral", "behavioral-repeat"]:
        published = json.loads((HERE / f"raw/{label}.stdout").read_text())
        expected_profile = "catalog" if label == "catalog" else "behavioral"
        assert published["profile"] == expected_profile
        families = {row["family"]: row["availability"] for row in published["families"]}
        assert (families["Flow"] == "NotRequested") == (label == "catalog")
        assert len(published["stage_measurements"]) == (5 if label == "catalog" else 6)
        errors = (HERE / f"raw/{label}.stderr").read_text()
        match = re.search(r"Maximum resident set size \(kbytes\): (\d+)", errors)
        assert match is not None
        measured[label] = {
            "generation": published["generation"],
            "content_digest": published["content_digest"],
            "peak_reservation_bytes": published["peak_reservation_bytes"],
            "process_peak_rss_bytes": int(match.group(1)) * 1024,
            "rss_sampling_interval_ms": published["rss_sampling_interval_ms"],
            "stage_measurements": published["stage_measurements"],
            "families": published["families"],
            "profile_correct_flow": "passed",
        }
        for query_label, sql in [
            (
                "artifact-count",
                "SELECT COUNT(*) AS artifacts, SUM(byte_len) AS captured_bytes "
                "FROM source_artifacts",
            ),
            (
                "largest-python",
                "SELECT path, byte_len FROM source_artifacts "
                "WHERE path LIKE '%.py' OR path LIKE '%.pyi' "
                "ORDER BY byte_len DESC, path LIMIT 5",
            ),
            (
                "bound-residuals",
                "SELECT child.kind, child.other_variant, COUNT(*) AS bound_methods "
                "FROM type_terms parent JOIN type_terms child "
                "ON parent.boundmethod_function = child.id "
                "WHERE parent.kind = 8 AND child.kind IN (26, 27) "
                "GROUP BY child.kind, child.other_variant",
            ),
        ]:
            result = subprocess.run(
                [str(CLI), "query", "--generation", published["generation"], sql],
                cwd=ROOT,
                capture_output=True,
                check=False,
            )
            (HERE / f"raw/{label}-{query_label}.stdout").write_bytes(result.stdout)
            (HERE / f"raw/{label}-{query_label}.stderr").write_bytes(result.stderr)
            assert result.returncode == 0, result.stderr.decode()
            measured[label][query_label] = "passed"
        detail = subprocess.check_output(
            [str(CLI), "generation", "show", published["generation"]],
            cwd=ROOT,
        )
        (HERE / f"raw/{label}-generation.json").write_bytes(detail)
    measured["scope"] = (
        "Shared-host elapsed/RSS evidence, not a performance comparison or total-RSS cap. "
        "Native parse/solver heaps, allocator retention, transient native adapter state and "
        "SQLx within-operation buffers remain external allowances; Arrow aliases are charged "
        "conservatively and B-tree/nested-growth calibration has a named P3 trigger."
    )
    (HERE / "measurement-receipt.json").write_text(json.dumps(measured, indent=2) + "\n")
    print(json.dumps({"outcome": "passed", "pilots": len(measured) - 1}), flush=True)


if __name__ == "__main__":
    main()
