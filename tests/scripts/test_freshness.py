"""Read-only freshness classes over fake runners: nothing is regenerated or executed for real."""

from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path

import pytest

import freshness
from freshness import Ran


class Fake:
    """Answers by the first argv words; records every call."""

    def __init__(self, answers: dict[str, Ran]) -> None:
        self.answers = answers
        self.calls: list[tuple[str, ...]] = []

    def __call__(self, argv: Sequence[str], **kwargs) -> Ran:
        self.calls.append(tuple(argv))
        joined = " ".join(argv)
        for key, ran in self.answers.items():
            if key in joined:
                return ran
        raise AssertionError(f"unexpected command {joined}")


CLEAN = Ran(0, "", "")


def test_hakari_clean_stale_and_undecided(tmp_path: Path) -> None:
    (tmp_path / "Cargo.toml").write_text("[workspace]\nmembers=[]\n")
    (tmp_path / "Cargo.lock").write_text("version = 4\n")
    fake = Fake({"generate --diff": CLEAN, "manage-deps --dry-run": CLEAN})
    assert freshness.hakari(fake, tmp_path).state == "clean"
    assert {c[2] for c in fake.calls} == {"generate", "manage-deps"}

    diff = Ran(1, '--- original\n+++ modified\n-serde = { version = "1" }\n', "")
    stale = freshness.hakari(Fake({"generate": diff, "manage-deps": CLEAN}), tmp_path)
    assert stale.state == "stale" and stale.regenerate == "just build-features"
    assert stale.details == ['-serde = { version = "1" }']

    deps = Ran(1, "add workspace-hack to crates/x\n", "")
    assert freshness.hakari(Fake({"generate": CLEAN, "manage-deps": deps}), tmp_path).state == (
        "stale"
    )
    missing = Ran(127, "", "No such file: cargo")
    undecided = freshness.hakari(Fake({"generate": missing, "manage-deps": CLEAN}), tmp_path)
    assert undecided.state == "not_run" and "exit 127" in undecided.reason


def test_hakari_capture_owns_mutations_and_detects_concurrent_inputs(tmp_path):
    (tmp_path / "Cargo.toml").write_text("[workspace]\nmembers=[]\n")
    (tmp_path / "Cargo.lock").write_text("version = 4\n# stale lock\n")
    original = {name: (tmp_path / name).read_bytes() for name in ("Cargo.toml", "Cargo.lock")}
    for directory in ("target", "build", ".venv"):
        (tmp_path / directory).mkdir()
        (tmp_path / directory / "Cargo.toml").write_text("must not copy")

    def mutate_copy(argv, *, cwd, environment):
        assert cwd != tmp_path and environment["CARGO_NET_OFFLINE"] == "true"
        assert not any((cwd / directory).exists() for directory in ("target", "build", ".venv"))
        assert not any(path.is_symlink() for path in cwd.rglob("*"))
        (cwd / "Cargo.lock").write_text("normalized lock")
        (cwd / "Cargo.toml").write_text("edited only in capture")
        return CLEAN

    freshness.hakari(mutate_copy, tmp_path)
    assert original == {name: (tmp_path / name).read_bytes() for name in original}

    def concurrent_edit(argv, **kwargs):
        (tmp_path / "Cargo.toml").write_text("concurrent edit")
        return CLEAN

    result = freshness.hakari(concurrent_edit, tmp_path)
    assert result.state == "stale" and "inconclusive" in result.reason
    assert (tmp_path / "Cargo.toml").read_text() == "concurrent edit"


def test_hakari_missing_offline_resolution_never_certifies_clean(tmp_path):
    (tmp_path / "Cargo.toml").write_text("[workspace]\nmembers=[]\n")
    (tmp_path / "Cargo.lock").write_text("version=4\n")
    result = freshness.hakari(
        Fake({"cargo": Ran(1, "", "error: no matching package found offline")}), tmp_path
    )
    assert result.state == "not_run"


def test_hakari_actual_stale_lock_metadata_preserves_live_bytes(tmp_path):
    import shutil

    if not shutil.which("cargo") or not shutil.which("cargo-hakari"):
        pytest.skip("installed Cargo/Hakari required for native metadata control")
    (tmp_path / "Cargo.toml").write_text('[workspace]\nresolver="2"\nmembers=["app", "hack"]\n')
    for name in ("app", "hack"):
        source = tmp_path / name / "src"
        source.mkdir(parents=True)
        (source / "lib.rs").write_text("")
        (source.parent / "Cargo.toml").write_text(
            f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2021"\n'
            + ("\n### BEGIN HAKARI SECTION\n### END HAKARI SECTION\n" if name == "hack" else "")
        )
    (tmp_path / ".config").mkdir()
    (tmp_path / ".config/hakari.toml").write_text('hakari-package="hack"\nresolver="2"\n')
    lock = tmp_path / "Cargo.lock"
    lock.write_text("version=4\n# deliberately stale: no package entries\n")
    before = freshness.metadata_inputs(tmp_path)
    observed = []

    def native(argv, **kwargs):
        result = freshness.run(argv, **kwargs)
        observed.append((kwargs["cwd"] / "Cargo.lock").read_bytes())
        return result

    result = freshness.hakari(native, tmp_path)
    assert result.state in {"clean", "stale"}, result
    assert observed and observed[0] != before[lock]
    assert freshness.metadata_inputs(tmp_path) == before
    assert not (tmp_path / "target").exists()


def test_docs_observation_uses_publisher_owner_only(tmp_path, monkeypatch):
    import docs

    monkeypatch.setattr(docs, "observe_publication", lambda root: ("stale", "changed source"))
    result = freshness.docs(Fake({}), tmp_path)
    assert result.state == "stale" and result.regenerate == "just docs-check"


def test_formatting_reports_files_and_never_formats(tmp_path: Path) -> None:
    rust = Ran(1, f"{tmp_path}/crates/a/src/lib.rs\n{tmp_path}/crates/a/src/../b.rs\n", "")
    fake = Fake({"cargo fmt": rust})
    result = freshness.rust_fmt(fake, tmp_path)
    assert result.state == "stale" and result.files == ["crates/a/b.rs", "crates/a/src/lib.rs"]
    assert fake.calls == [("cargo", "fmt", "--check", "--message-format", "short")]

    found = [{"filename": f"{tmp_path}/scripts/x.py", "code": "unformatted"}]
    python = freshness.python_fmt(Fake({"ruff format": Ran(1, json.dumps(found), "")}), tmp_path)
    assert python.state == "stale" and python.files == ["scripts/x.py"]
    assert freshness.python_fmt(Fake({"ruff format": CLEAN}), tmp_path).state == "clean"
    broken = freshness.python_fmt(Fake({"ruff format": Ran(2, "", "parse error")}), tmp_path)
    assert broken.state == "not_run"
    for call in fake.calls:
        assert "--check" in call


def test_gold_without_its_skill_is_not_run_and_runs_nothing(tmp_path: Path) -> None:
    fake = Fake({})
    result = freshness.gold(fake, tmp_path, skill=tmp_path / "absent")
    assert result.state == "not_run" and "not installed" in result.reason
    assert fake.calls == []


def test_gold_states_follow_check_gold(tmp_path: Path) -> None:
    skill = tmp_path / "manifests"
    skill.mkdir()
    (skill / "fastmcp.json").write_text("{}")
    ok = Ran(0, "gold: ok (agree)\n", "")
    assert freshness.gold(Fake({"check_gold": ok}), tmp_path, skill).state == "clean"
    said = Ran(0, "gold: not_run (the fastmcp skill is not installed)\n", "")
    assert freshness.gold(Fake({"check_gold": said}), tmp_path, skill).state == "not_run"
    drift = Ran(1, "gold: fastmcp: skill 4.0.4, uv.lock 4.0.5\n", "")
    stale = freshness.gold(Fake({"check_gold": drift}), tmp_path, skill)
    assert stale.state == "stale" and stale.details == ["gold: fastmcp: skill 4.0.4, uv.lock 4.0.5"]


def test_skills_states(tmp_path: Path) -> None:
    assert freshness.skills(Fake({"library_skills": CLEAN}), tmp_path).state == "clean"
    differ = Ran(1, "link fastmcp: missing\n", "")
    assert freshness.skills(Fake({"library_skills": differ}), tmp_path).state == "stale"
    absent = Ran(1, "", "Shared library skills are not installed at /x")
    assert freshness.skills(Fake({"library_skills": absent}), tmp_path).state == "not_run"


def test_adr_index_compares_without_writing(tmp_path: Path, monkeypatch) -> None:
    import adr

    monkeypatch.setattr(adr, "load_all", lambda root: [])
    monkeypatch.setattr(adr, "render_index", lambda root, records: "index\n")
    monkeypatch.setattr(adr, "adr_dir", lambda root: root)
    index = tmp_path / "README.md"
    index.write_text("index\n")
    assert freshness.adr_index(Fake({}), tmp_path).state == "clean"
    index.write_text("old\n")
    stale = freshness.adr_index(Fake({}), tmp_path)
    assert stale.state == "stale" and stale.regenerate == "just adr-index"
    assert index.read_text() == "old\n"


def test_insta_is_not_run_with_orphan_hint_and_pending(tmp_path: Path) -> None:
    crate = tmp_path / "crates" / "a"
    (crate / "tests" / "snapshots").mkdir(parents=True)
    (crate / "Cargo.toml").write_text("")
    (crate / "tests" / "live.rs").write_text("")
    live = "crates/a/tests/snapshots/live__x.snap"
    gone = "crates/a/tests/snapshots/gone__x.snap"
    (tmp_path / live).write_text("---\nsource: crates/a/tests/live.rs\n---\nbody\n")
    (tmp_path / gone).write_text("---\nsource: tests/gone.rs\nexpression: x\n---\nbody\n")
    pending = "crates/a/tests/snapshots/live__x.snap.new"
    listing = Ran(0, f"{live}\n{gone}\n{pending}\n", "")
    insta, hint = freshness.insta(Fake({"ls-files": listing}), tmp_path)
    assert insta.state == "not_run" and freshness.INSTA_NOT_RUN in insta.reason
    assert insta.files == [pending]
    assert hint.state == "heuristic" and hint.files == [f"{gone} (source: tests/gone.rs)"]

    (tmp_path / gone).unlink()
    _, quiet = freshness.insta(Fake({"ls-files": Ran(0, f"{live}\n", "")}), tmp_path)
    assert quiet.state == "heuristic" and quiet.files == []


def test_selection_exit_status_and_unknown_outputs(monkeypatch, capsys) -> None:
    assert freshness.select(["fmt", "rust-fmt"]) == ["rust-fmt", "python-fmt"]
    assert freshness.select([]) == list(freshness.CHECKS)
    with pytest.raises(KeyError):
        freshness.select(["nope"])

    def fake_check(names, runner=None):
        return [
            freshness.Freshness("gold", "not_run", "x", "absent"),
            freshness.Freshness("insta-sources", "heuristic", "x", "hint"),
        ]

    monkeypatch.setattr(freshness, "check", fake_check)
    assert freshness.main([]) == 0  # not_run and heuristic alone do not fail
    capsys.readouterr()
    monkeypatch.setattr(
        freshness, "check", lambda names, runner=None: [freshness.Freshness("a", "stale", "x", "y")]
    )
    assert freshness.main(["--json"]) == 1
    assert json.loads(capsys.readouterr().out)[0]["state"] == "stale"


def test_unknown_output_is_a_usage_error(capsys) -> None:
    assert freshness.main(["nope"]) == 2
    assert "unknown output" in capsys.readouterr().err
