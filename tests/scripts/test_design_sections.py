from pathlib import Path

import pytest

import design_sections as ds


def put(root: Path, name: str, body: str) -> Path:
    path = root / "docs/design" / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)
    return path


def test_move_retains_logical_parent_and_ignores_stub_and_fences(tmp_path):
    put(
        tmp_path,
        "DESIGN.md",
        "## §9 Analysis\n> Decision: ADR-0001\n### §9.9 Detail\n\n"
        "<!-- relocated-section -->\n[owner](sections/detail.md)\n```md\n## §99 Fake\n```\n",
    )
    owner = put(tmp_path, "sections/detail.md", "# §9.9 Detail\nMeaning\n~~~md\n# §99 Fake\n~~~\n")
    sections = ds.resolve(tmp_path)
    assert set(sections) == {"9", "9.9"}
    assert sections["9.9"].path == owner
    assert [s.ident for s in ds.governing_sections("9.9", sections)] == ["9.9", "9"]


def test_duplicate_owner_is_error(tmp_path):
    put(tmp_path, "DESIGN.md", "# §4 Original\n")
    put(tmp_path, "sections/duplicate.md", "# §4 Duplicate\n")
    with pytest.raises(ValueError, match="duplicate section §4"):
        ds.resolve(tmp_path)


def test_sibling_decisions_do_not_govern(tmp_path):
    path = put(
        tmp_path, "DESIGN.md", "# §4 Parent\n## §4.1 First\n> Decision: ADR-0001\n## §4.2 Second\n"
    )
    owners = ds.resolve(tmp_path)
    governing = "\n".join(
        line
        for section in ds.governing_sections("4.2", owners)
        for line in ds.own_lines(section, ds.prose_lines(path.read_text()))
    )
    assert "ADR-0001" not in governing
