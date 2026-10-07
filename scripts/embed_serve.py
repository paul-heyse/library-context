"""Launch the operator-controlled vLLM service from the hashed embedding spec."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path

SPEC = Path(__file__).resolve().parents[1] / "specs/embedding/qwen3-embedding-8b.json"
PROJECT = Path(__file__).resolve().parents[1] / "services/vllm"
CHECKPOINT = Path("/home/paul/wheelhouse/gpu-stack/models/Qwen3-Embedding-8B-NVFP4-r2")
SERVER = "vllm 0.30.1rc1.dev286+g3d5f4d4cd.sm120.r2"


def verify_checkpoint(spec: dict, checkpoint: Path = CHECKPOINT) -> None:
    """Bind both weights and tokenizer to the published manifest and verify its files."""
    manifest = (checkpoint / "SHA256SUMS").read_bytes()
    revision = "sha256:" + hashlib.sha256(manifest).hexdigest()
    if spec["revision"] != revision or spec["tokenizer_revision"] != revision:
        raise ValueError("checkpoint manifest does not match the embedding spec")
    for line in manifest.decode().splitlines():
        expected, name = line.split(maxsplit=1)
        path = checkpoint / name.removeprefix("*")
        if not path.resolve().is_relative_to(checkpoint.resolve()):
            raise ValueError("checkpoint manifest path escapes its directory")
        with path.open("rb") as stream:
            actual = hashlib.file_digest(stream, "sha256").hexdigest()
        if actual != expected:
            raise ValueError(f"checkpoint checksum mismatch: {path.name}")


def launch_command(spec: dict, port: int) -> list[str]:
    if spec["server"] != SERVER:
        raise ValueError(f"unsupported serving engine: {spec['server']}")
    if (
        spec["pooling"]
        != "last-token, L2-normalized (the model's sentence-transformers config; E1)"
    ):
        raise ValueError("the launch recipe does not support this pooling contract")
    if not 1 <= port <= 65535:
        raise ValueError("port must be between 1 and 65535")
    admission = spec["admission"]
    if (
        spec["format"] != 3
        or spec["source_dimensions"] != 4096
        or spec["reduction"] != "none"
        or spec["dimensions"] != 4096
        or spec["normalization"] != "l2"
        or admission
        != {
            "is_matryoshka": True,
            "matryoshka_dimensions": [4096],
            "use_activation": True,
        }
    ):
        raise ValueError("incompatible embedding reduction/admission")
    return [
        "uv",
        "run",
        "--project",
        str(PROJECT),
        "--frozen",
        "vllm",
        "serve",
        str(CHECKPOINT),
        "--served-model-name",
        spec["model"],
        "--runner",
        "pooling",
        "--max-model-len",
        "8192",
        "--dtype",
        spec["served_dtype"],
        "--gpu-memory-utilization",
        "0.85",
        "--max-num-batched-tokens",
        "16384",
        "--no-enable-prefix-caching",
        "--host",
        "127.0.0.1",
        "--hf-overrides",
        json.dumps(
            {k: admission[k] for k in ("is_matryoshka", "matryoshka_dimensions")},
            separators=(",", ":"),
        ),
        "--pooler-config",
        json.dumps(
            {
                "seq_pooling_type": "LAST",
                "use_activation": admission["use_activation"],
                "dimensions": spec["dimensions"],
            },
            separators=(",", ":"),
        ),
        "--port",
        str(port),
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--print-command", action="store_true")
    args = parser.parse_args()
    spec = json.loads(SPEC.read_text(encoding="utf-8"))
    command = launch_command(spec, args.port)
    if args.print_command:
        print(json.dumps(command))
    else:
        verify_checkpoint(spec)
        os.environ.update(
            CUDA_HOME="/usr/local/cuda-13.4",
            CUDA_CACHE_MAXSIZE="4294967296",
            VLLM_NO_USAGE_STATS="1",
        )
        # Role E is non-BI; an inherited reranker environment must not change it.
        os.environ.pop("VLLM_BATCH_INVARIANT", None)
        os.environ.pop("VLLM_BATCH_INVARIANT_ALLOW_AOT", None)
        os.execvp(command[0], command)


if __name__ == "__main__":
    main()
