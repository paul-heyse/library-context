"""The controlled service launch and cache spec select the same model revisions."""

from __future__ import annotations

import hashlib
import json
import runpy
from pathlib import Path

import pytest


def test_launch_uses_local_checkpoint_and_declared_served_identity() -> None:
    root = Path(__file__).resolve().parents[3]
    launch = runpy.run_path(str(root / "scripts/embed_serve.py"))["launch_command"]
    spec = json.loads((root / "specs/embedding/qwen3-embedding-8b.json").read_text())
    command = launch(spec, 8123)
    assert command[command.index("serve") + 1].endswith("Qwen3-Embedding-8B-NVFP4-r2")
    assert "--revision" not in command
    assert "--tokenizer-revision" not in command
    assert command[command.index("--dtype") + 1] == spec["served_dtype"]
    assert command[command.index("--served-model-name") + 1] == spec["model"]
    assert command[-2:] == ["--port", "8123"]

    assert command[command.index("--gpu-memory-utilization") + 1] == "0.85"
    assert command[command.index("--max-num-batched-tokens") + 1] == "16384"
    assert "--no-enable-prefix-caching" in command


def test_checkpoint_refuses_mismatched_manifest_and_changed_files(tmp_path: Path) -> None:
    root = Path(__file__).resolve().parents[3]
    verify = runpy.run_path(str(root / "scripts/embed_serve.py"))["verify_checkpoint"]
    payload = b"checkpoint"
    (tmp_path / "model.safetensors").write_bytes(payload)
    manifest = f"{hashlib.sha256(payload).hexdigest()}  model.safetensors\n".encode()
    (tmp_path / "SHA256SUMS").write_bytes(manifest)
    revision = "sha256:" + hashlib.sha256(manifest).hexdigest()
    spec = {"revision": revision, "tokenizer_revision": revision}
    verify(spec, tmp_path)
    with pytest.raises(ValueError, match="manifest"):
        verify(spec | {"tokenizer_revision": "wrong"}, tmp_path)
    (tmp_path / "model.safetensors").write_bytes(b"changed weights")
    with pytest.raises(ValueError, match="checksum mismatch"):
        verify(spec, tmp_path)


def test_mrl_launch_is_derived_from_the_standard_spec() -> None:
    root = Path(__file__).resolve().parents[3]
    launch = runpy.run_path(str(root / "scripts/embed_serve.py"))["launch_command"]
    spec = json.loads((root / "specs/embedding/qwen3-embedding-8b.json").read_text())
    command = launch(spec, 8123)
    assert json.loads(command[command.index("--hf-overrides") + 1]) == {
        "is_matryoshka": True,
        "matryoshka_dimensions": [1024],
    }
    assert json.loads(command[command.index("--pooler-config") + 1]) == {
        "seq_pooling_type": "LAST",
        "use_activation": True,
        "dimensions": 1024,
    }
    with pytest.raises(ValueError, match="reduction/admission"):
        launch(spec | {"dimensions": 4096}, 8123)
