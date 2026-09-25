from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

import adr

REPO = Path(__file__).resolve().parents[2]

DESIGN = """# Design
## §2 Binding decisions
### §B1 Front ends
## §4 Pipeline
### §4.2 Identity
"""


@pytest.fixture
def root(tmp_path: Path) -> Path:
    (tmp_path / "docs" / "adr").mkdir(parents=True)
    (tmp_path / "docs" / "design").mkdir(parents=True)
    (tmp_path / "docs" / "design" / "DESIGN.md").write_text(DESIGN)
    template = (REPO / "docs" / "adr" / "TEMPLATE.md").read_text()
    (tmp_path / "docs" / "adr" / "TEMPLATE.md").write_text(template)
    return tmp_path


def new(root: Path, slug: str, design: str = "[§B1]", decide: bool = True) -> Path:
    path = adr.create(root, slug, slug, [])
    adr.set_field(path, "design", design)
    if decide:
        # DESIGN names the record in each section it lists, as the lint requires.
        record = f"ADR-{path.name[:4]}"
        text = (root / "docs" / "design" / "DESIGN.md").read_text()
        for ref in design.strip("[]").split(","):
            heading = next(
                line for line in text.splitlines() if line.split(" ", 1)[-1].startswith(ref.strip())
            )
            text = text.replace(heading, f"{heading}\n> Decision: {record}", 1)
        (root / "docs" / "design" / "DESIGN.md").write_text(text)
    return path


def run(root: Path, *argv: str) -> int:
    return adr.main(["--root", str(root), *argv])


def test_new_numbers_sequentially_and_index_makes_lint_pass(root: Path) -> None:
    first = new(root, "first")
    second = new(root, "second", "[§4.2]")
    assert (first.name, second.name) == ("0001-first.md", "0002-second.md")
    assert adr.lint(root, check_git=False) == ["docs/adr/README.md is stale: run `just adr index`"]
    run(root, "index")
    assert adr.lint(root, check_git=False) == []


def test_unresolved_design_ref_is_reported(root: Path) -> None:
    new(root, "bad-ref", "[§9.9]", decide=False)
    run(root, "index")
    problems = adr.lint(root, check_git=False)
    assert any("§9.9 does not resolve" in p for p in problems)


def test_supersede_links_both_sides(root: Path) -> None:
    new(root, "original")
    assert run(root, "supersede", "ADR-0001", "replacement") == 0
    replacement = root / "docs" / "adr" / "0002-replacement.md"
    adr.set_field(replacement, "design", "[§B1]")
    decide(root, "ADR-0002")
    records = {a.id: a for a in adr.load_all(root)}
    assert records["ADR-0001"].status == "superseded"
    assert records["ADR-0001"].refs("superseded-by") == ["ADR-0002"]
    assert records["ADR-0002"].refs("supersedes") == ["ADR-0001"]
    run(root, "index")
    assert adr.lint(root, check_git=False) == []


def test_one_sided_supersession_is_reported(root: Path) -> None:
    path = new(root, "orphan")
    adr.set_field(path, "status", "superseded")
    adr.set_field(path, "superseded-by", "ADR-0099")
    run(root, "index")
    assert any("ADR-0099 does not exist" in p for p in adr.lint(root, check_git=False))


def test_invalid_evidence_label_is_reported(root: Path) -> None:
    path = new(root, "evidence")
    adr.set_field(path, "evidence", "passed")
    run(root, "index")
    assert any("not a principles §D label" in p for p in adr.lint(root, check_git=False))


def decide(root: Path, record: str, heading: str = "### §B1 Front ends") -> None:
    """Name `record` in `heading`'s section, as the lint requires of a listed section."""
    design = root / "docs" / "design" / "DESIGN.md"
    design.write_text(design.read_text().replace(heading, f"{heading}\n> Decision: {record}", 1))


def git(root: Path, *argv: str) -> None:
    subprocess.run(["git", "-C", str(root), *argv], check=True, capture_output=True)


def test_accepted_record_is_immutable_except_status_fields(root: Path) -> None:
    git(root, "init", "-q")
    git(root, "config", "user.email", "t@example.com")
    git(root, "config", "user.name", "t")
    path = new(root, "stable")
    adr.set_field(path, "status", "accepted")
    run(root, "index")
    git(root, "add", "-A")
    git(root, "commit", "-qm", "accept")

    # Superseding (status fields only) is allowed.
    run(root, "supersede", "ADR-0001", "next")
    adr.set_field(root / "docs" / "adr" / "0002-next.md", "design", "[§B1]")
    decide(root, "ADR-0002")
    run(root, "index")
    assert adr.lint(root) == []

    # Editing the decision text is not.
    path.write_text(path.read_text().replace("## Decision", "## Decision\n\nChanged."))
    assert any("accepted record edited" in p for p in adr.lint(root))


def test_amendments_may_be_appended_but_not_rewritten(root: Path) -> None:
    git(root, "init", "-q")
    git(root, "config", "user.email", "t@example.com")
    git(root, "config", "user.name", "t")
    path = new(root, "amended")
    adr.set_field(path, "status", "accepted")
    run(root, "index")
    path.write_text(path.read_text() + "\n## Amendments\n\n- 2026-09-22: first.\n")
    git(root, "add", "-A")
    git(root, "commit", "-qm", "accept")

    path.write_text(path.read_text() + "- 2026-09-23: second.\n")
    assert adr.lint(root) == []

    path.write_text(path.read_text().replace("first.", "rewritten."))
    assert any("accepted record edited" in p for p in adr.lint(root))


def test_a_listed_section_must_name_its_record(root: Path) -> None:
    new(root, "undecided", decide=False)
    run(root, "index")
    problems = adr.lint(root, check_git=False)
    assert any("§B1: no `> Decision:` line names ADR-0001" in p for p in problems)


def test_an_enclosing_sections_decision_line_counts(root: Path) -> None:
    new(root, "parent", "[§4]")
    path = new(root, "child", "[§4.2]", decide=False)
    text = (root / "docs" / "design" / "DESIGN.md").read_text()
    (root / "docs" / "design" / "DESIGN.md").write_text(
        text.replace("> Decision: ADR-0001", f"> Decision: ADR-0001, ADR-{path.name[:4]}")
    )
    run(root, "index")
    assert adr.lint(root, check_git=False) == []


def test_a_cited_record_must_exist_unless_to_be_written(root: Path) -> None:
    new(root, "real")
    run(root, "index")
    design = root / "docs" / "design" / "DESIGN.md"
    design.write_text(design.read_text() + "Verdicts (ADR-0042, to be written) and ADR-0043.\n")
    problems = adr.lint(root, check_git=False)
    assert problems == ["DESIGN.md cites ADR-0043, which does not exist"]


def test_a_status_claim_must_match_the_record(root: Path) -> None:
    new(root, "open")
    run(root, "index")
    design = root / "docs" / "design" / "DESIGN.md"
    design.write_text(
        design.read_text()
        + "The spike passed and ADR-0001 accepted with it.\n"
        + "| 2026-09-22 | ADR-0001 accepted then |\n"
    )
    problems = adr.lint(root, check_git=False)
    assert len(problems) == 1 and "says ADR-0001 is accepted, but it is proposed" in problems[0]


def test_moved_section_inherits_only_its_logical_parent(root: Path) -> None:
    new(root, "moved", "[§4.2]", decide=False)
    design = root / "docs/design/DESIGN.md"
    design.write_text(
        "# Design\n## §4 Pipeline\n> Decision: ADR-0001\n"
        "### §4.2 Identity\n\n<!-- relocated-section -->\n[owner](sections/identity.md)\n"
    )
    sections = root / "docs/design/sections"
    sections.mkdir()
    owner = sections / "identity.md"
    owner.write_text("# §4.2 Identity\nMeaning.\n```md\nADR-9999\n```\n")
    run(root, "index")
    assert adr.lint(root, check_git=False) == []
    design.write_text(design.read_text().replace("> Decision: ADR-0001\n", ""))
    assert any("no `> Decision:`" in problem for problem in adr.lint(root, check_git=False))
    owner.unlink()
    assert any("does not resolve" in problem for problem in adr.lint(root, check_git=False))


def commit_all(root: Path, message: str) -> None:
    git(root, "add", "-A")
    git(root, "commit", "-qm", message)


def init_repo(root: Path) -> None:
    git(root, "init", "-q")
    git(root, "config", "user.email", "t@example.com")
    git(root, "config", "user.name", "t")


def test_a_retired_predecessor_is_history_but_a_missing_governing_record_fails(
    root: Path,
) -> None:
    new(root, "old")
    run(root, "supersede", "ADR-0001", "replacement")
    replacement = root / "docs" / "adr" / "0002-replacement.md"
    adr.set_field(replacement, "design", "[§B1]")
    adr.set_field(replacement, "status", "accepted")
    decide(root, "ADR-0002")
    (root / "docs" / "adr" / "0001-old.md").unlink()
    design = root / "docs" / "design" / "DESIGN.md"
    design.write_text(design.read_text().replace("> Decision: ADR-0001\n", ""))
    run(root, "index")
    assert adr.lint(root, check_git=False) == []
    index = (root / "docs" / "adr" / "README.md").read_text()
    assert "| ADR-0001 |" in index and "(0001-old.md)" not in index
    assert "historical recovery" in index
    design.write_text(design.read_text() + "Governed by ADR-0001.\n")
    assert adr.lint(root, check_git=False) == ["DESIGN.md cites ADR-0001, which does not exist"]


def test_a_retained_superseded_record_needs_a_retained_successor(root: Path) -> None:
    new(root, "old")
    run(root, "supersede", "ADR-0001", "replacement")
    (root / "docs" / "adr" / "0002-replacement.md").unlink()
    run(root, "index")
    problems = adr.lint(root, check_git=False)
    assert any("superseded-by ADR-0002 does not exist" in p for p in problems)


def test_only_an_accepted_record_may_replace_a_retired_one(root: Path) -> None:
    new(root, "old")
    run(root, "supersede", "ADR-0001", "replacement")
    replacement = root / "docs" / "adr" / "0002-replacement.md"
    adr.set_field(replacement, "design", "[§B1]")
    decide(root, "ADR-0002")
    (root / "docs" / "adr" / "0001-old.md").unlink()
    design = root / "docs" / "design" / "DESIGN.md"
    design.write_text(design.read_text().replace("> Decision: ADR-0001\n", ""))
    run(root, "index")
    assert any(
        "supersedes retired ADR-0001 but is proposed" in p for p in adr.lint(root, check_git=False)
    )
    adr.set_field(replacement, "status", "accepted")
    run(root, "index")
    assert adr.lint(root, check_git=False) == []


def test_supersede_rejects_a_missing_live_predecessor(root: Path) -> None:
    with pytest.raises(SystemExit, match="no such ADR"):
        run(root, "supersede", "ADR-0007", "replacement")


def test_new_numbers_above_retired_records_in_history(root: Path) -> None:
    init_repo(root)
    new(root, "first")
    second = new(root, "second")
    commit_all(root, "two records")
    second.unlink()
    commit_all(root, "retire the second")
    assert adr.create(root, "third", "third", []).name == "0003-third.md"


def test_new_refuses_a_shallow_history_but_lint_works(
    root: Path, tmp_path_factory: pytest.TempPathFactory
) -> None:
    init_repo(root)
    first = new(root, "first")
    new(root, "second", "[§4.2]")
    commit_all(root, "two")
    first.unlink()
    design = root / "docs" / "design" / "DESIGN.md"
    design.write_text(design.read_text().replace("> Decision: ADR-0001\n", ""))
    run(root, "index")
    commit_all(root, "retire the first")
    clone = tmp_path_factory.mktemp("shallow") / "clone"
    subprocess.run(
        ["git", "clone", "-q", "--depth", "1", f"file://{root}", str(clone)],
        check=True,
        capture_output=True,
    )
    with pytest.raises(SystemExit, match="git fetch --unshallow"):
        adr.next_number(clone)
    assert adr.lint(clone) == []
