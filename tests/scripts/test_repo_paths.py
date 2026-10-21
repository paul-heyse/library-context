import subprocess
from pathlib import Path

from repo_paths import local_skill


def test_inactive_references_resolve_without_discovery_links(tmp_path: Path) -> None:
    subprocess.run(["git", "init", "-q", str(tmp_path)], check=True)
    skills = tmp_path / ".claude/skills"
    skills.mkdir(parents=True)
    (skills / "README.md").write_text("[old](example/SKILL.md)\n[process](adr/SKILL.md)\n")
    (tmp_path / ".gitignore").write_text(
        "/.claude/skills/*\n!/.claude/skills/README.md\n!/.claude/skills/adr/\n"
    )
    assert local_skill(tmp_path, skills / "example/SKILL.md")
    assert local_skill(tmp_path, skills / "example/references/details.md")
    assert not local_skill(tmp_path, skills / "typo/SKILL.md")
    assert not local_skill(tmp_path, skills / "adr/SKILL.md")
    assert not local_skill(tmp_path, tmp_path / "docs/example/SKILL.md")
    central = tmp_path / "central"
    central.mkdir()
    (skills / "example").symlink_to(central, target_is_directory=True)
    assert local_skill(tmp_path, skills / "example/SKILL.md")
