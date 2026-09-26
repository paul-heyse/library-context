"""The controlled service launch and cache spec select the same model revisions."""

from __future__ import annotations

import json
import runpy
from pathlib import Path


def test_launch_tracks_hashed_model_and_tokenizer_revisions() -> None:
    root = Path(__file__).resolve().parents[3]
    launch = runpy.run_path(str(root / "scripts/embed_serve.py"))["launch_command"]
    spec = json.loads((root / "specs/embedding/qwen3-embedding-8b.json").read_text())
    command = launch(spec, 8123)
    assert command[command.index("--revision") + 1] == spec["revision"]
    assert command[command.index("--tokenizer-revision") + 1] == spec["tokenizer_revision"]
    assert command[command.index("--dtype") + 1] == spec["served_dtype"]
    assert command[command.index("--served-model-name") + 1] == spec["model"]
    assert command[-2:] == ["--port", "8123"]

    changed = spec | {"revision": "another-model-revision", "tokenizer_revision": "another-tokenizer"}
    changed_command = launch(changed, 8123)
    assert changed_command != command
    assert changed_command[changed_command.index("--revision") + 1] == changed["revision"]
    assert changed_command[changed_command.index("--tokenizer-revision") + 1] == changed["tokenizer_revision"]
