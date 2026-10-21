"""Resolve retained central FastMCP evaluation data, independent of agent discovery."""

import os
from pathlib import Path


def reference_root() -> Path:
    """Existing data only: do not create, synchronize or rebuild this source."""
    store = Path(
        os.environ.get("LIBRARY_SKILLS_ROOT", "~/.local/share/library-skills")
    ).expanduser()
    return store / "skills" / "fastmcp"
