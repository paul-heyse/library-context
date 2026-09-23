from __future__ import annotations

import json
from pathlib import Path

import check_gold as gold

REQ = "fastmcp[tasks]==4.0.5"


def write(tmp: Path, install: str, slim: str) -> tuple[Path, Path]:
    library = tmp / "library"
    library.mkdir()
    (library / "pyproject.toml").write_text(
        f'[project]\nname = "x"\nversion = "0"\ndependencies = ["{REQ}"]\n'
        '[tool.lctx]\nrelease = ["fastmcp", "fastmcp-slim"]\n'
    )
    (library / "uv.lock").write_text(
        'version = 1\n[[package]]\nname = "fastmcp"\nversion = "4.0.5"\n'
        '[[package]]\nname = "fastmcp-slim"\nversion = "4.0.5"\n'
    )
    manifests = tmp / "manifests"
    manifests.mkdir()
    (manifests / "fastmcp.json").write_text(json.dumps({"environment": {"install": [install]}}))
    (manifests / "resolved.json").write_text(json.dumps({"fastmcp": "4.0.5", "fastmcp-slim": slim}))
    return library, manifests


def test_one_fastmcp_passes(tmp_path: Path) -> None:
    assert gold.problems(*write(tmp_path, REQ, "4.0.5")) == []


def test_a_different_install_line_or_version_fails(tmp_path: Path) -> None:
    found = gold.problems(*write(tmp_path, "fastmcp[tasks]==4.0.3", "4.0.3"))
    assert any("skill installs" in p for p in found)
    assert any(p.startswith("fastmcp-slim: skill 4.0.3") for p in found)


def test_the_repository_library_reads() -> None:
    requirement, release, locked = gold.library_pins(gold.LIBRARY)
    assert requirement.startswith("fastmcp[") and requirement.endswith("==4.0.5")
    assert all(locked[d] == "4.0.5" for d in release)
