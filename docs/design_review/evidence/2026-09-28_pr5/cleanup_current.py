"""Retire obsolete local epochs only after the current operator replacement is verified."""
import hashlib
import json
import re
from pathlib import Path
import shutil
import sys

ROOT = Path.cwd().resolve()
BUILD = ROOT / "build"


def remove(path):
    assert path.parent == BUILD or BUILD in path.parents
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.is_dir():
        shutil.rmtree(path)


def main():
    pilots = BUILD / "pr5-qualified"
    operator = BUILD / "pr5-operator-current"
    deployed = json.loads((operator / "receipt.json").read_text())
    assert deployed["outcome"] == deployed["reconstruction"]["outcome"] == "passed"
    assert deployed["current_generations"] == 2 and deployed["obsolete_generations"] == 0
    rows = json.loads((pilots / "receipt.json").read_text())
    assert {r["profile"] for r in rows} == {"catalog", "behavioral"}
    artifacts = BUILD / "serving-artifacts"
    keep = set()
    for row in rows:
        generation = Path(row["generation"])
        assert generation.is_relative_to(pilots) and generation.is_dir()
        manifest = json.loads((generation / "MANIFEST.json").read_text())
        for name, artifact in manifest["projection"]["artifacts"].items():
            assert Path(name).name == name
            path = artifacts / artifact["sha256"] / name
            assert not path.is_symlink() and path.is_file()
            assert path.stat().st_size == artifact["bytes"]
            with path.open("rb") as stream:
                assert hashlib.file_digest(stream, "sha256").hexdigest() == artifact["sha256"]
            keep.add(path)
    # Explicit obsolete task epochs; acquisition inputs, fixtures, target and caches are excluded.
    # Exact replaced runtime epochs only. No benchmarks, build intermediates or source inputs.
    retired = [BUILD / name for name in ["pr4-qualified-pilots", "pr4-operator-current", "store", "generations", "last_snapshot", "live-generation"]]
    retired += [BUILD / "embedding-adoption" / profile for profile in ["catalog", "behavioral"]]
    retired += [pilots / "artifacts"]
    retired = [p for p in retired if p.exists() or p.is_symlink()]
    benchmark_roots = [BUILD / "perf", BUILD / "ablation", BUILD / "cargo-adoption"]
    def inventory():
        return {str(p): (p.stat().st_size, p.stat().st_mtime_ns)
                for root in benchmark_roots if root.exists() for p in root.rglob("*") if p.is_file()}
    benchmark_before = inventory()
    assert not any(p == pilots or p in pilots.parents or p == operator or p in operator.parents for p in retired)
    directories = list(artifacts.iterdir())
    assert all(not d.is_symlink() and d.is_dir() and re.fullmatch(r"[0-9a-f]{64}", d.name) for d in directories)
    obsolete_artifacts = [p for directory in directories for p in directory.iterdir() if p not in keep]
    temporary = [p for parent, prefix in [(pilots, "both-profiles.dump"), (operator, "current.dump")]
                 for p in parent.glob(prefix + "*")]
    fixture_obsolete = []
    py_fixture = BUILD / "py-fixture"
    py_current = py_fixture / (py_fixture / "CURRENT").read_text().strip()
    assert py_current.parent == py_fixture and (py_current / "MANIFEST.json").is_file()
    fixture_obsolete.extend(p for p in py_fixture.iterdir() if p.is_dir() and p != py_current)
    catalog_fixture = BUILD / "catalog-fixture"
    catalog_current = catalog_fixture / (catalog_fixture / "CURRENT").read_text().strip()
    catalog_store = Path((catalog_fixture / "STORE").read_text().strip())
    assert catalog_current.is_relative_to(catalog_fixture) and (catalog_current / "MANIFEST.json").is_file()
    assert catalog_store.parent.parent == catalog_fixture and (catalog_store / "snapshots").is_dir()
    fixture_keep = {catalog_fixture / "CURRENT", catalog_fixture / "STORE", catalog_store.parent, catalog_current.parent}
    fixture_obsolete.extend(p for p in catalog_fixture.iterdir() if p not in fixture_keep)
    fixture_obsolete.extend(p for p in catalog_current.parent.iterdir() if p.is_dir() and p != catalog_current)
    if "--apply" not in sys.argv:
        print(json.dumps({"obsolete_task_paths": len(retired), "obsolete_artifact_files": len(obsolete_artifacts), "current_artifact_files": len(keep), "temporary_reconstruction_paths": len(temporary), "obsolete_fixture_paths": len(fixture_obsolete)}))
        return
    for path in retired + obsolete_artifacts + temporary + fixture_obsolete:
        remove(path)
    for directory in artifacts.iterdir():
        if not any(directory.iterdir()):
            directory.rmdir()
    selected = next(Path(r["generation"]) for r in rows if r["profile"] == "behavioral")
    (BUILD / "generations").symlink_to(selected.parent.relative_to(BUILD), target_is_directory=True)
    (BUILD / "store").symlink_to((selected.parent.parent / "store").relative_to(BUILD), target_is_directory=True)
    assert {p for directory in artifacts.iterdir() for p in directory.iterdir()} == keep
    assert inventory() == benchmark_before, "benchmark artifacts changed during cleanup"
    adoption = BUILD / "embedding-adoption/status.json"
    adoption.write_text(json.dumps({"state":"current_only_cutover_complete", "current_profiles": rows,
                                   "accuracy_assessment":"not_run"}, indent=2) + "\n")
    receipt = {"outcome": "passed", "benchmarks_preserved": True, "benchmark_files": len(benchmark_before), "obsolete_task_paths_removed": len(retired), "obsolete_artifact_files_removed": len(obsolete_artifacts), "current_artifact_files": len(keep), "temporary_reconstruction_paths_removed": len(temporary), "selected_generation": str(selected), "default_store": str((BUILD / "store").resolve()), "prior_epochs_retained": False, "obsolete_fixture_paths_removed": len(fixture_obsolete)}
    (operator / "cleanup.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))


if __name__ == "__main__":
    main()
