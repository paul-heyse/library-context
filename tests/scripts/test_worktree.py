"""Optional checkouts (D3): carry, removal guards and integration, in a throwaway repository."""

from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path

import pytest

import workspace_env
import worktree
from build_environment import BUILD_DIR_SELECTION, explain, normalized_env, shell_changes


def run(cwd: Path, *args: str) -> str:
    return subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout


@pytest.fixture
def repo(tmp_path, monkeypatch):
    for key, value in {
        "GIT_AUTHOR_NAME": "t",
        "GIT_AUTHOR_EMAIL": "t@example.invalid",
        "GIT_COMMITTER_NAME": "t",
        "GIT_COMMITTER_EMAIL": "t@example.invalid",
        "GIT_CONFIG_GLOBAL": "/dev/null",
        "GIT_CONFIG_NOSYSTEM": "1",
    }.items():
        monkeypatch.setenv(key, value)
    monkeypatch.setenv("CARGO_HOME", str(tmp_path / "cargo"))
    root = tmp_path / "main"
    root.mkdir()
    run(root, "git", "init", "-q", "-b", "main")
    (root / ".gitignore").write_text(".venv/\nbuild/\n/.dev/\n")
    (root / "src").mkdir()
    for name in ("tracked.txt", "staged.txt", "paused.rs"):
        (root / "src" / name).write_text(f"{name} original\n")
    (root / "src" / "blob.bin").write_bytes(bytes(range(256)) * 4)
    run(root, "git", "add", "-A")
    run(root, "git", "commit", "-qm", "base")
    return root


def snapshot(root: Path) -> dict[str, str]:
    """Every file's bytes plus the index, so 'source unchanged' covers staging too."""
    state = {
        path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(root.rglob("*"))
        if path.is_file() and ".git" not in path.relative_to(root).parts
    }
    state["<index>"] = hashlib.sha256((root / ".git" / "index").read_bytes()).hexdigest()
    state["<status>"] = run(root, "git", "status", "--porcelain=v1", "--untracked-files=all")
    return state


def scratch_changes(root: Path) -> None:
    src = root / "src"
    (src / "tracked.txt").write_text("tracked edit, unstaged\n")
    (src / "staged.txt").write_text("staged edit\n")
    run(root, "git", "add", "src/staged.txt")
    (src / "staged.txt").write_text("staged edit\nthen an unstaged edit\n")
    (src / "blob.bin").write_bytes(bytes(reversed(range(256))) * 4 + b"\0\xff")
    run(root, "git", "add", "src/blob.bin")
    (src / "new.txt").write_text("untracked\n")
    (src / "new.bin").write_bytes(b"\x00\x01\x02binary\xff")
    (src / "paused.rs").write_text("another agent's uncommitted work\n")
    (root / ".env.local").write_text("SECRET=1\n")


def test_carry_arrives_intact_and_source_is_unchanged(repo, tmp_path):
    scratch_changes(repo)
    before = snapshot(repo)
    named = ["src/tracked.txt", "src/staged.txt", "src/blob.bin", "src/new.txt", "src/new.bin"]
    lines: list[str] = []
    code = worktree.create(
        "carry", carry=named, root=repo, base=tmp_path / "wt", prepare=False, report=lines.append
    )
    assert code == 0, lines
    assert snapshot(repo) == before
    target = tmp_path / "wt" / "carry"
    for name in named:
        assert (target / name).read_bytes() == (repo / name).read_bytes(), name
    # Staging is preserved: the same staged diff and the same unstaged diff.
    for args in (("diff", "--cached", "--binary"), ("diff", "--binary")):
        assert run(target, "git", *args, "--", *named) == run(repo, "git", *args, "--", *named)
    status = run(target, "git", "status", "--porcelain=v1", "--untracked-files=all")
    assert "MM src/staged.txt" in status and "M  src/blob.bin" in status
    assert " M src/tracked.txt" in status and "?? src/new.bin" in status
    # Never carried implicitly: another agent's file and credential material.
    assert (target / "src/paused.rs").read_text() == "paused.rs original\n"
    assert not (target / ".env.local").exists()
    report = "\n".join(lines)
    assert "staged changes arrive staged" in report and "source checkout was only read" in report
    assert worktree.remove("carry", force=True, root=repo, base=tmp_path / "wt") == 0


def test_carry_needs_named_safe_paths(repo, tmp_path):
    scratch_changes(repo)
    base = tmp_path / "wt"
    with pytest.raises(ValueError, match=r"changed here:(.|\n)*src/paused\.rs"):
        worktree.create("x", carry=[], root=repo, base=base, prepare=False)
    for path, message in (
        (".env.local", "credential"),
        (".venv/lib/x", "environment"),
        ("build/out", "local build"),
        ("src/missing.txt", "no carriable changes"),
    ):
        with pytest.raises(ValueError, match=message):
            worktree.create("x", carry=[path], root=repo, base=base, prepare=False)
    assert not base.exists()
    # A directory carries its untracked files only; tracked changes under it must be named.
    plan = worktree.carry_plan(repo, ["src"])
    assert plan.untracked == ["src/new.bin", "src/new.txt"]
    assert plan.staged == plan.unstaged == []
    assert any(item.startswith("src/paused.rs") for item in plan.skipped)


def test_removal_reports_unintegrated_work_and_cherry_pick_integrates_it(repo, tmp_path):
    base = tmp_path / "wt"
    assert (
        worktree.create("feature", root=repo, base=base, prepare=False, report=lambda _: None) == 0
    )
    target = base / "feature"
    (target / "src" / "feature.txt").write_text("feature\n")
    lines: list[str] = []
    assert worktree.remove("feature", root=repo, base=base, report=lines.append) == 1
    assert any(line.startswith("dirty: ?? src/feature.txt") for line in lines)
    run(target, "git", "add", "src/feature.txt")
    run(target, "git", "commit", "-qm", "feature work")
    lines.clear()
    assert worktree.remove("feature", root=repo, base=base, report=lines.append) == 1
    assert any("not integrated into main" in line and "feature work" in line for line in lines)
    assert target.exists()
    # Work returns by cherry-pick; patch-equivalent commits count as integrated.
    (repo / "src" / "main-only.txt").write_text("main moved on\n")
    run(repo, "git", "add", "src/main-only.txt")
    run(repo, "git", "commit", "-qm", "main work")
    run(repo, "git", "cherry-pick", "wt/feature")
    assert (repo / "src" / "feature.txt").read_text() == "feature\n"
    lines.clear()
    assert worktree.remove("feature", root=repo, base=base, report=lines.append) == 0, lines
    assert not target.exists()
    assert run(repo, "git", "branch", "--list", "wt/feature") == ""


def test_own_build_dir_is_selected_shown_and_removed_with_the_worktree(repo, tmp_path):
    base = tmp_path / "wt"
    lines: list[str] = []
    worktree.create(
        "own", build_dir="own", root=repo, base=base, prepare=False, report=lines.append
    )
    target = base / "own"
    own = worktree.own_build_dir("own")
    assert own == tmp_path / "cargo" / "build" / "library-context-wt-own"
    assert (target / BUILD_DIR_SELECTION).read_text().strip() == str(own)
    assert normalized_env({}, target)["CARGO_BUILD_BUILD_DIR"] == str(own)
    assert "CARGO_BUILD_BUILD_DIR" not in normalized_env({}, repo)
    assert f"export CARGO_BUILD_BUILD_DIR={own}" in shell_changes({}, normalized_env({}, target))
    shown = next(line for line in explain({}, target) if line.startswith("cargo build dir:"))
    assert str(own) in shown and "own: .dev/build-dir" in shown
    assert any(line.startswith(f"build dir: {own} (own") for line in lines)
    own.mkdir(parents=True)
    (own / "unit").write_text("built")
    lines.clear()
    assert worktree.remove("own", root=repo, base=base, report=lines.append) == 0
    assert not own.exists() and any("removed own build dir" in line for line in lines)


def test_ready_selects_the_checkout_environment_over_an_inherited_absolute_one(tmp_path):
    checkout = tmp_path / "wt" / "probe"
    inherited = {"UV_PROJECT_ENVIRONMENT": "/main/.venv", "VIRTUAL_ENV": "/main/.venv"}
    changes, notes = workspace_env.selection(checkout, inherited)
    assert changes == {"UV_PROJECT_ENVIRONMENT": str(checkout / ".venv"), "VIRTUAL_ENV": None}
    assert any("replaced inherited UV_PROJECT_ENVIRONMENT=/main/.venv" in note for note in notes)
    selected = {"UV_PROJECT_ENVIRONMENT": str(changes["UV_PROJECT_ENVIRONMENT"])}
    (environment, extension) = workspace_env.resources_for("native", checkout, selected)
    assert environment.path == checkout / ".venv"
    assert extension.path.is_relative_to(checkout)
    unchanged, notes = workspace_env.selection(checkout, {})
    assert unchanged == {"UV_PROJECT_ENVIRONMENT": str(checkout / ".venv")}
    assert notes == [f"selected environment {checkout / '.venv'}"]


def test_names_are_validated(repo, tmp_path):
    for name in ("../escape", "", "a/b", "-flag"):
        with pytest.raises(ValueError, match="worktree names"):
            worktree.create(name, root=repo, base=tmp_path / "wt", prepare=False)


def test_worktree_ready_never_inherits_another_checkouts_environment(tmp_path):
    """Even a ref whose `ready` predates selection prepares the worktree's own environment."""
    target = tmp_path / "wt" / "old-ref"
    source = {
        "UV_PROJECT_ENVIRONMENT": "/main/.venv",
        "VIRTUAL_ENV": "/main/.venv",
        "LCTX_ENV_OWNERSHIP": "[]",
        "KEEP": "1",
    }
    env, notes = worktree.ready_environment(target, source)
    assert env == {"UV_PROJECT_ENVIRONMENT": str(target / ".venv"), "KEEP": "1"}
    assert "replaced inherited UV_PROJECT_ENVIRONMENT=/main/.venv" in notes[0]
    assert worktree.ready_environment(target, {})[1] == []


def _live_owner(tmp_path: Path):
    """A live process standing in for a launcher, plus its identity record."""
    import harness

    process = subprocess.Popen(["sleep", "60"])
    return process, harness.ProcessIdentity.of(process.pid).to_json()


def test_removal_refuses_live_fixture_and_running_run_naming_them(repo, tmp_path, monkeypatch):
    """Simulated records read through the real fixture and runs listings."""
    import json

    base = tmp_path / "wt"
    worktree.create("live", root=repo, base=base, prepare=False, report=lambda _: None)
    target = base / "live"
    process, owner = _live_owner(tmp_path)
    try:
        fixture = target / "build" / "fixtures" / "f00dfeed01"
        fixture.mkdir(parents=True)
        (fixture / "record.json").write_text(
            json.dumps(
                {
                    "schema": 1,
                    "id": "f00dfeed01",
                    "kind": "run",
                    "checkout": str(target),
                    "unit": "lctx-fixture-f00dfeed01.scope",
                    "port": 41234,
                    "memory_max": 1 << 30,
                    "created": "2026-10-08T00:00:00+00:00",
                    "owner": owner,
                }
            )
        )
        run = target / "build" / "runs" / "20261008T000000Z-ab12cd"
        run.mkdir(parents=True)
        (run / "record.json").write_text(
            json.dumps(
                {
                    "schema": 1,
                    "id": run.name,
                    "argv": ["just", "verify-serving"],
                    "label": None,
                    "owner": owner,
                    "child": None,
                    "started": "2026-10-08T00:00:00+00:00",
                    "termination": None,
                }
            )
        )
        lines: list[str] = []
        assert worktree.remove("live", root=repo, base=base, report=lines.append) == 1
        assert any("live: run-owned fixture f00dfeed01" in line for line in lines), lines
        assert any(f"live: running run {run.name}: just verify-serving" in line for line in lines)
        assert (fixture / "record.json").is_file() and (run / "record.json").is_file()
        # A run-owned fixture has no stop route here: even --force refuses rather than orphan it.
        lines.clear()
        stops: list[tuple] = []
        real = worktree._harness

        def harness(script, args, key, directory):
            if args[0] in ("cancel", "--stop"):  # record the stop route, don't signal sleep
                stops.append((script, *args))
                return subprocess.CompletedProcess(args, 0, "", "")
            return real(script, args, key, directory)

        monkeypatch.setattr(worktree, "_harness", harness)
        assert worktree.remove("live", force=True, root=repo, base=base, report=lines.append) == 1
        monkeypatch.setattr(worktree, "_harness", real)
        assert stops == [("runs.py", "cancel", run.name)]
        assert any("still live: run-owned fixture f00dfeed01" in line for line in lines)
        assert target.exists() and (fixture / "record.json").is_file()
    finally:
        process.kill()
        process.wait()
    # A dead owner with no child identity leaves cleanup unknown; force cannot orphan it.
    assert worktree.remove("live", force=True, root=repo, base=base, report=lambda _: None) == 1
    assert target.exists()
    record = json.loads((run / "record.json").read_text())
    record.update(schema=2, termination="interrupted", cleanup={"status": "confirmed"})
    (run / "record.json").write_text(json.dumps(record))
    assert worktree.remove("live", force=True, root=repo, base=base, report=lambda _: None) == 0


def test_force_stops_kept_fixtures_through_their_route_before_removal(repo, tmp_path, monkeypatch):
    base = tmp_path / "wt"
    worktree.create("kept", root=repo, base=base, prepare=False, report=lambda _: None)
    target = base / "kept"
    rows = {
        "surrealdb_fixture.py": [
            {
                "id": "k1",
                "kind": "kept",
                "unit": "lctx-fixture-k1.service",
                "state": "active/success",
                "port": 40001,
                "attachments": "0/1",
            },
            {
                "id": "k2",
                "kind": "kept",
                "unit": "lctx-fixture-k2.service",
                "state": "gone",
                "port": 40002,
                "attachments": "0/0",
            },
            {
                "id": None,
                "kind": "unrecorded",
                "unit": "lctx-fixture-other.service",
                "state": "loaded active running lctx fixture other",
            },
        ],
        "runs.py": [{"id": "r1", "state": "completed", "argv": ["true"], "protected": False}],
    }
    calls: list[tuple] = []

    def listing(script, args, key, directory):
        assert directory.is_relative_to(target)
        return rows[script]

    def harness(script, args, key, directory):
        calls.append((script, *args))
        assert (target / "build").parent.exists()  # the tree still exists while stopping
        rows["surrealdb_fixture.py"][0]["state"] = "gone"
        return subprocess.CompletedProcess(args, 0, "fixture k1: stopped\n", "")

    monkeypatch.setattr(worktree, "_listing", listing)
    monkeypatch.setattr(worktree, "_harness", harness)
    monkeypatch.setattr(worktree, "_unit_description", lambda unit: "checkout=/elsewhere")
    lines: list[str] = []
    assert worktree.remove("kept", root=repo, base=base, report=lines.append) == 1
    assert [line for line in lines if line.startswith("live:")] == [
        "live: kept fixture k1: lctx-fixture-k1.service on port 40001, attachments 0/1"
    ]
    rows["surrealdb_fixture.py"][0]["state"] = "active/success"
    lines.clear()
    assert worktree.remove("kept", force=True, root=repo, base=base, report=lines.append) == 0
    assert calls == [("surrealdb_fixture.py", "--stop", "k1", "--force", "--no-sweep")]
    assert "stopped kept fixture k1: fixture k1: stopped" in lines
    assert not target.exists()


def test_unknown_fixture_state_is_not_absent(repo, tmp_path, monkeypatch):
    base = tmp_path / "wt"
    worktree.create("unknown", root=repo, base=base, prepare=False, report=lambda _: None)
    monkeypatch.setattr(
        worktree, "_harness", lambda *a: subprocess.CompletedProcess(a, 1, "", "systemctl: no bus")
    )
    with pytest.raises(RuntimeError, match=r"cannot inspect .*no bus"):
        worktree.remove("unknown", root=repo, base=base, report=lambda _: None)
    assert (base / "unknown").exists()


@pytest.mark.parametrize("kind", ["run", "kept", "incomplete"])
def test_ended_fixture_with_unresolved_cleanup_protects_worktree(repo, tmp_path, monkeypatch, kind):
    base = tmp_path / "wt"
    worktree.create("protected", root=repo, base=base, prepare=False, report=lambda _: None)
    target = base / "protected"
    fixtures = [
        {
            "id": "pending",
            "kind": kind,
            "unit": "fixture.scope",
            "state": "gone",
            "owner_alive": False,
            "protected": True,
            "cleanup": {"a": "unknown"},
        }
    ]
    monkeypatch.setattr(
        worktree,
        "_listing",
        lambda script, *args: fixtures if script == "surrealdb_fixture.py" else [],
    )
    attempted = []

    def refuse(script, args, *unused):
        attempted.append((script, *args))
        return subprocess.CompletedProcess(args, 1, "", "cleanup identity unresolved")

    monkeypatch.setattr(worktree, "_harness", refuse)
    assert (
        worktree.remove("protected", force=True, root=repo, base=base, report=lambda _: None) == 1
    )
    assert target.exists()
    assert bool(attempted) is (kind != "incomplete")
    fixtures.clear()  # only confirmed owner recovery permits removal
    assert (
        worktree.remove("protected", force=True, root=repo, base=base, report=lambda _: None) == 0
    )
