from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

import docs
from repo_paths import local_skill


@pytest.fixture
def root(tmp_path):
    for name, body in {
        "README.md": "# Home\n",
        "docs/design/DESIGN.md": "# Design\n## §1 One\n",
        "docs/design_review/design_principles/core.md": "# Core\n",
        "docs/design_review/design_principles/template.md": "# Template\n",
        "docs/design_review/design_principles/binding.md": "# Binding\n",
        "docs/design_review/design_principles/standard.toml": (
            'profiles=[]\n[core]\nprinciples="core.md"\ntemplate="template.md"\n'
            '[binding]\npath="binding.md"\n'
        ),
    }.items():
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body)
    return tmp_path


@pytest.fixture
def settings():
    return {
        "publication": {
            "entrypoints": ["README.md"],
            "current_work": [],
            "exclude": [],
            "assets": [],
        },
        "collections": [
            {
                "title": "Docs",
                "include": [
                    "README.md",
                    "docs/design/**/*.md",
                    "docs/design_review/design_principles/*.md",
                ],
                "scope": "Reference",
            }
        ],
        "site": {"repository": "https://example.test/repo", "branch": "main", "base_url": "/"},
        "tools": {"mdbook": "0.5.4"},
    }


@pytest.fixture
def publication(root, settings, monkeypatch):
    """Real capture/annotation/Git with disposable identifiable renderer stand-ins."""
    settings["tools"] = {"mdbook": "0.5.4", "pagefind": "1.5.2", "lychee": "0.24.2"}
    site = ["[site]"] + [f"{k}={json.dumps(v)}" for k, v in settings["site"].items()]
    site += ["[tools]"] + [f"{k}={json.dumps(v)}" for k, v in settings["tools"].items()]
    site += ["[publication]"] + [f"{k}={json.dumps(v)}" for k, v in settings["publication"].items()]
    for collection in settings["collections"]:
        site += ["[[collections]]"] + [f"{k}={json.dumps(v)}" for k, v in collection.items()]
    (root / "docs/site.toml").write_text("\n".join(site))
    (root / "docs/book.toml").write_text('[book]\ntitle="Test"\n[output.html]\n')
    (root / "docs/theme").mkdir()
    (root / "docs/theme/search.js").write_text("// captured theme")
    (root / ".gitignore").write_text("/build/\n/.tools/\n")
    for args in [
        ("init", "-q"),
        ("config", "user.name", "Test"),
        ("config", "user.email", "test@example.invalid"),
        ("add", "."),
        ("commit", "-qm", "fixture"),
    ]:
        subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True)
    binaries = root / ".tools"
    binaries.mkdir()
    for name, version in settings["tools"].items():
        path = binaries / name
        path.write_text(f"#!/bin/sh\necho '{name} {version}'\n")
        path.chmod(0o755)
    monkeypatch.setattr(
        docs.shutil,
        "which",
        lambda name: str(binaries / name) if (binaries / name).exists() else None,
    )
    original_run = docs.run

    def renderer(argv, **kwargs):
        name = Path(argv[0]).name
        if name == "git" or "--version" in argv:
            return original_run(argv, **kwargs)
        if name == "mdbook":
            candidate, destination = Path(argv[2]), Path(argv[-1])
            destination.mkdir()
            for source in (candidate / "src").rglob("*.md"):
                if source.name == "SUMMARY.md":
                    continue
                path = destination / source.relative_to(candidate / "src").with_suffix(".html")
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(
                    '<html lang="en"><head></head><main>' + source.read_text() + "</main></html>"
                )
        return subprocess.CompletedProcess(argv, 0, "")

    monkeypatch.setattr(docs, "run", renderer)
    return root, settings, renderer


def test_discovery_add_delete_deterministic_and_standard_scope(root, settings):
    first = docs.discover(root, settings)
    path = root / "docs/design/new.md"
    path.write_text("```md\n# False\n```\n   # Real\n")
    second = docs.discover(root, settings)
    assert second == docs.discover(root, settings)
    assert next(p for p in second if p.path.name == "new.md").title == "Real"
    assert next(p for p in second if p.path.name == "core.md").scope == "Current"
    path.unlink()
    assert first == docs.discover(root, settings)


def test_explicit_title_and_missing_entry_and_duplicate(root, settings):
    (root / "README.md").write_text("No H1")
    with pytest.raises(ValueError, match="missing title"):
        docs.discover(root, settings)
    settings["titles"] = {"README.md": "Override"}
    assert docs.discover(root, settings)[0].title == "Override"
    settings["publication"]["current_work"] = ["absent.md"]
    with pytest.raises(ValueError, match="not published"):
        docs.discover(root, settings)
    settings["publication"]["current_work"] = []
    settings["collections"].append(settings["collections"][0])
    with pytest.raises(ValueError, match="duplicate"):
        docs.discover(root, settings)


def test_lifecycle_scopes_and_evidence_exclusion(root, settings):
    adr = root / "docs/adr"
    adr.mkdir()
    for n, status in enumerate(["accepted", "proposed", "superseded", "rejected"]):
        (adr / f"{n:04d}-test.md").write_text(
            f"---\nstatus: {status}\ntitle: Record\n---\n# Record\n"
        )
    evidence = root / "docs/evidence/probe"
    evidence.mkdir(parents=True)
    (evidence / "README.md").write_text("# Evidence\n")
    (evidence / "raw.md").write_text("# Raw\n")
    settings["collections"] += [
        {"title": "Decisions", "include": ["docs/adr/*.md"], "scope": "Reference"},
        {"title": "Evidence", "include": ["docs/evidence/**/README.md"], "scope": "History"},
    ]
    pages = docs.discover(root, settings)
    assert [p.scope for p in pages if p.collection == "Decisions"] == [
        "Current",
        "Reference",
        "History",
        "History",
    ]
    assert not any(p.path.name == "raw.md" for p in pages)
    assert docs.scope_options(pages) == ["Current", "Reference", "History", "Everything"]


def test_a_scope_without_pages_is_not_offered(root, settings):
    pages = docs.discover(root, settings)
    assert {p.scope for p in pages} == {"Current", "Reference"}
    assert docs.scope_options(pages) == ["Current", "Reference", "Everything"]


def rewrite(root, settings, text, tracked=()):
    site = root / "site"
    site.mkdir(exist_ok=True)
    page = docs.Page(Path("docs/page.md"), "Page", "", "Docs", "Current")
    return docs.rewrite_links(
        root, site, page, text, {Path("README.html")}, set(tracked), settings, "abc123"
    )


def test_links_nested_source_and_local_new_source_and_invalid(root, settings):
    (root / "code.rs").write_text("code")
    result = rewrite(root, settings, '<a href="../code.rs#L1">code</a>', ["code.rs"])
    assert "https://example.test/repo/blob/abc123/code.rs#L1" in result
    result = rewrite(root, settings, '<a href="../code.rs">code</a>')
    assert "uncommitted local source" in result and "href=" not in result
    assert rewrite(root, settings, '<a href="../README.html#home">home</a>').startswith(
        '<a href="../README.html#home">'
    )
    with pytest.raises(ValueError, match="unresolved"):
        rewrite(root, settings, '<a href="missing.html">missing</a>')
    with pytest.raises(ValueError, match="escapes"):
        rewrite(root, settings, '<a href="../../secret">missing</a>')


def test_images_copy_but_evidence_download_stays_remote(root, settings):
    (root / "docs/image.svg").write_text("<svg/>")
    result = rewrite(root, settings, '<img src="image.svg">', ["docs/image.svg"])
    assert 'src="image.svg"' in result and (root / "site/docs/image.svg").exists()
    folder = root / "docs/evidence"
    folder.mkdir()
    (folder / "raw.svg").write_text("<svg/>")
    result = rewrite(
        root, settings, '<a href="evidence/raw.svg">raw</a>', ["docs/evidence/raw.svg"]
    )
    assert "/blob/abc123/docs/evidence/raw.svg" in result
    assert not (root / "site/docs/evidence/raw.svg").exists()


def test_clean_checkout_optional_skills_are_declared_and_ignored(root, settings):
    subprocess.run(["git", "init", "-q", str(root)], check=True)
    skill = root / ".claude/skills"
    skill.mkdir(parents=True)
    (skill / "README.md").write_text("[example](example/SKILL.md)\n[process](adr/SKILL.md)")
    (root / ".gitignore").write_text(
        "/.claude/skills/*\n!/.claude/skills/README.md\n!/.claude/skills/adr/\n"
    )
    assert local_skill(root, skill / "example/SKILL.md")
    assert not local_skill(root, skill / "typo/SKILL.md")
    assert not local_skill(root, skill / "adr/SKILL.md")
    result = rewrite(
        root, settings, '<a href="../.claude/skills/example/SKILL.html">capability</a>'
    )
    assert "[local reference]" in result and "href=" not in result


def test_shared_skill_symlink_stays_a_local_reference(root, settings, tmp_path):
    subprocess.run(["git", "init", "-q", str(root)], check=True)
    skills = root / ".claude/skills"
    skills.mkdir(parents=True)
    (skills / "README.md").write_text("[shared](example/SKILL.md)")
    (root / ".gitignore").write_text("/.claude/skills/*\n!/.claude/skills/README.md\n")
    outside = tmp_path.parent / (tmp_path.name + "-shared")
    outside.mkdir(exist_ok=True)
    (outside / "SKILL.md").write_text("Not publication input")
    (skills / "example").symlink_to(outside, target_is_directory=True)
    result = rewrite(root, settings, '<a href="../.claude/skills/example/SKILL.html">shared</a>')
    assert "[local reference]" in result and "href=" not in result
    (root / "escape").symlink_to(outside, target_is_directory=True)
    with pytest.raises(ValueError, match="escapes"):
        rewrite(root, settings, '<a href="../escape/SKILL.md">secret</a>')


@pytest.mark.parametrize("failure", ["missing", "wrong"])
def test_tools_fail_actionably(monkeypatch, settings, failure):
    def fake(*args, **kwargs):
        if failure == "missing":
            raise FileNotFoundError()
        return subprocess.CompletedProcess([], 0, "mdbook v9.9.9")

    monkeypatch.setattr(docs, "run", fake)
    with pytest.raises(ValueError, match="bootstrap-docs"):
        docs.check_tools(settings)


def test_an_unversioned_tool_needs_only_presence(monkeypatch, settings):
    settings["tools"] = {"mdbook": ""}
    monkeypatch.setattr(docs, "run", lambda *a, **k: subprocess.CompletedProcess([], 0, "v9.9.9"))
    docs.check_tools(settings)

    def missing(*args, **kwargs):
        raise FileNotFoundError()

    monkeypatch.setattr(docs, "run", missing)
    with pytest.raises(ValueError, match="bootstrap-docs"):
        docs.check_tools(settings)


def test_bootstrap_installs_a_pin_exactly_and_an_unversioned_tool_at_the_latest():
    assert docs.install_command("mdbook", "0.5.4")[-3:] == ["--version", "0.5.4", "mdbook"]
    assert "--version" not in docs.install_command("mdbook", "")


@pytest.mark.parametrize("tool", ["mdbook", "pagefind", "lychee"])
def test_pipeline_failure_preserves_previous_artifact(publication, monkeypatch, tool):
    root, settings, renderer = publication
    destination = root / "build/docs/site"
    destination.mkdir(parents=True)
    (destination / "old.html").write_text("last successful artifact")

    def fake(argv, **kwargs):
        if Path(argv[0]).name == tool and "--version" not in argv:
            raise subprocess.CalledProcessError(1, argv)
        return renderer(argv, **kwargs)

    monkeypatch.setattr(docs, "run", fake)
    with pytest.raises(subprocess.CalledProcessError):
        docs.build(root, settings)
    assert (destination / "old.html").read_text() == "last successful artifact"
    assert list((root / "build/docs").iterdir()) == [destination]


def test_receipt_matches_capture_and_observation_does_not_render(publication, monkeypatch):
    root, settings, renderer = publication
    site = docs.build(root, settings)
    receipt = json.loads((site / docs.RECEIPT).read_text())
    assert receipt["files"]["README.md"] == docs.digest((root / "README.md").read_bytes())
    assert receipt["tools"]["mdbook"]["version"] == "mdbook 0.5.4"

    def read_only(argv, **kwargs):
        assert argv[0] == "git", "freshness must not execute a renderer or tool binary"
        return renderer(argv, **kwargs)

    monkeypatch.setattr(docs, "run", read_only)
    assert docs.observe_publication(root)[0] == "clean"
    assert json.loads((site / docs.RECEIPT).read_text()) == receipt


@pytest.mark.parametrize(
    "change",
    [
        "page",
        "membership",
        "book",
        "config",
        "theme",
        "tool",
        "missing-tool",
        "git",
        "publisher",
        "selection",
        "environment",
    ],
)
def test_receipt_stale_for_output_affecting_inputs(publication, monkeypatch, change):
    root, settings, _ = publication
    docs.build(root, settings)
    if change == "page":
        (root / "README.md").write_text("# Changed\n")
    elif change == "membership":
        (root / "docs/design/new.md").write_text("# Added page\n")
    elif change == "book":
        (root / "docs/book.toml").write_text("# changed config")
    elif change == "config":
        with (root / "docs/site.toml").open("a") as stream:
            stream.write("\n# changed selection declaration\n")
    elif change == "theme":
        (root / "docs/theme/search.js").write_text("// newer theme")
    elif change == "tool":
        # Same declared/printed version, different actual executable bytes.
        with (root / ".tools/mdbook").open("a") as stream:
            stream.write("# changed binary\n")
    elif change == "missing-tool":
        (root / ".tools/mdbook").unlink()
    elif change == "git":
        subprocess.run(
            ["git", "-C", str(root), "commit", "--allow-empty", "-qm", "new revision"], check=True
        )
    elif change == "publisher":
        monkeypatch.setattr(docs, "publisher_identity", lambda: {"docs.py": "changed"})
    elif change == "environment":
        monkeypatch.setenv("MDBOOK_BOOK__TITLE", "Different rendered title")
    else:
        receipt = root / "build/docs/site" / docs.RECEIPT
        data = json.loads(receipt.read_text())
        data["selection"]["collections"] = []
        receipt.write_text(json.dumps(data))
    assert docs.observe_publication(root)[0] == ("not_run" if change == "selection" else "stale")


@pytest.mark.parametrize("change", ["page", "theme", "tool", "git"])
def test_drift_during_render_refuses_publication_preserving_prior_site(
    publication, monkeypatch, change
):
    root, settings, renderer = publication
    site = docs.build(root, settings)
    before = {p.relative_to(site): p.read_bytes() for p in site.rglob("*") if p.is_file()}

    def edit_during_render(argv, **kwargs):
        result = renderer(argv, **kwargs)
        if Path(argv[0]).name == "mdbook" and "--version" not in argv:
            if change == "page":
                (root / "README.md").write_text("# Concurrent newer source\n")
                assert "Concurrent newer" not in (Path(argv[-1]) / "README.html").read_text()
            elif change == "theme":
                (root / "docs/theme/search.js").write_text("// concurrent newer theme")
            elif change == "tool":
                with (root / ".tools/pagefind").open("a") as stream:
                    stream.write("# concurrent replacement\n")
            else:
                subprocess.run(
                    ["git", "-C", str(root), "commit", "--allow-empty", "-qm", "concurrent"],
                    check=True,
                )
        return result

    monkeypatch.setattr(docs, "run", edit_during_render)
    with pytest.raises(ValueError, match="inputs changed"):
        docs.build(root, settings)
    assert before == {p.relative_to(site): p.read_bytes() for p in site.rglob("*") if p.is_file()}
    assert list(site.parent.iterdir()) == [site]
    assert docs.observe_publication(root)[0] == "stale"


def test_missing_partial_receipt_never_clean(root):
    assert docs.observe_publication(root)[0] == "not_run"
    site = root / "build/docs/site"
    site.mkdir(parents=True)
    (site / docs.RECEIPT).write_text('{"schema": 1, "files": {}}')
    assert docs.observe_publication(root)[0] == "not_run"


def test_partial_publication_cannot_certify_full_selection(publication):
    root, settings, _ = publication
    (root / "docs/design/extra.md").write_text("# Extra page\n")
    settings["publication"]["exclude"] = ["docs/design/extra.md"]
    site = docs.build(root, settings)
    assert not (site / "docs/design/extra.html").exists()
    assert docs.observe_publication(root)[0] == "stale"


def test_referenced_asset_bytes_are_captured_and_observed(publication):
    root, settings, _ = publication
    asset = root / "docs/image.svg"
    asset.write_text("<svg>first</svg>")
    (root / "README.md").write_text('# Home\n<img src="docs/image.svg">\n')
    site = docs.build(root, settings)
    assert (site / "docs/image.svg").read_bytes() == asset.read_bytes()
    assert docs.observe_publication(root)[0] == "clean"
    asset.write_text("<svg>second</svg>")
    assert docs.observe_publication(root)[0] == "stale"


def test_omitted_dynamic_inventory_cannot_report_clean(publication):
    root, settings, _ = publication
    asset = root / "docs/image.svg"
    asset.write_text("<svg>first</svg>")
    (root / "README.md").write_text('# Home\n<img src="docs/image.svg">\n')
    site = docs.build(root, settings)
    path = site / docs.RECEIPT
    receipt = json.loads(path.read_text())
    receipt["files"].pop("docs/image.svg")
    receipt["references"].pop("docs/image.svg")
    path.write_text(json.dumps(receipt))
    asset.write_text("<svg>newer</svg>")
    state, reason = docs.observe_publication(root)
    assert state == "not_run" and "integrity" in reason


def test_replacement_removes_deleted_pages(tmp_path):
    old = tmp_path / "site"
    old.mkdir()
    (old / "deleted.html").touch()
    new = tmp_path / "candidate"
    new.mkdir()
    (new / "new.html").touch()
    docs.replace_site(new, old)
    assert [p.name for p in old.iterdir()] == ["new.html"]


def test_isolated_command_uses_the_locked_pytest_and_no_project():
    command = docs.isolated(
        docs.ROOT, "--with", f"pytest=={docs.pytest_version(docs.ROOT)}", "pytest"
    )
    assert (
        "--no-project" in command and "--offline" in command and "--no-python-downloads" in command
    )
    assert "lctx" not in " ".join(command)


def test_renderer_annotation_titles_and_duplicate_alias(root, settings):
    for args in [
        ("init", "-q"),
        ("config", "user.name", "Test"),
        ("config", "user.email", "test@example.invalid"),
        ("add", "."),
        ("commit", "-qm", "fixture"),
    ]:
        subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True)
    theme = root / "docs/theme"
    theme.mkdir()
    (theme / "search.js").write_text("// test")
    site = root / "site"
    site.mkdir()
    (site / "README.html").write_text(
        '<html lang="en"><head></head><body><h1>Book title</h1>'
        "<main><h1>Owner</h1></main></body></html>"
    )
    (site / "404.html").write_text('<html lang="en"><main>Not found</main></html>')
    page = docs.Page(
        Path("README.md"), "Actual page, with comma", "", "Docs", "Current", "accepted"
    )
    docs.annotate(root, site, [page], settings)
    canonical = (site / "README.html").read_text()
    assert 'data-doc-title="Actual page, with comma"' in canonical
    assert "title[data-doc-title]" in canonical
    assert "data-pagefind-body" not in (site / "index.html").read_text()
    assert 'data-pagefind-ignore="all"' in (site / "404.html").read_text()


def test_invalid_asset_declaration(root, settings):
    settings["publication"]["assets"] = ["docs/missing.png"]
    with pytest.raises(ValueError, match="missing declared asset"):
        docs.discover(root, settings)


def test_architecture_frontmatter_preserves_stable_anchor_destinations(root, settings):
    (root / "docs/design/DESIGN.md").write_text(
        "---\ntitle: Design\n---\n# Design\n## §1 First\nOne\n## §2 Second\nTwo\n"
    )
    (root / "docs/book.toml").write_text('[book]\ntitle="Test"\n[output.html]\n')
    candidate = root / "candidate"
    candidate.mkdir()
    docs.stage(root, candidate, docs.discover(root, settings), settings)
    body = (candidate / "src/docs/design/DESIGN.md").read_text()
    assert '<a id="section-1"></a>\n\n## §1 First' in body
    assert '<a id="section-2"></a>\n\n## §2 Second' in body
