"""Launch the operator-controlled vLLM service from the hashed embedding spec."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

SPEC = Path(__file__).resolve().parents[1] / "specs/embedding/qwen3-embedding-8b.json"
PROJECT = Path(__file__).resolve().parents[1] / "services/vllm"


def launch_command(spec: dict, port: int) -> list[str]:
    if spec["server"] != "vllm 0.30.0":
        raise ValueError(f"unsupported serving engine: {spec['server']}")
    if spec["pooling"] != "last-token, L2-normalized (the model's sentence-transformers config; E1)":
        raise ValueError("the launch recipe does not support this pooling contract")
    if not 1 <= port <= 65535:
        raise ValueError("port must be between 1 and 65535")
    return [
        "uv", "run", "--project", str(PROJECT), "--frozen", "vllm", "serve",
        spec["model"],
        "--revision", spec["revision"],
        "--tokenizer-revision", spec["tokenizer_revision"],
        "--served-model-name", spec["model"],
        "--runner", "pooling",
        "--max-model-len", "8192",
        "--dtype", spec["served_dtype"],
        "--gpu-memory-utilization", "0.80",
        "--port", str(port),
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--print-command", action="store_true")
    args = parser.parse_args()
    command = launch_command(json.loads(SPEC.read_text(encoding="utf-8")), args.port)
    if args.print_command:
        print(json.dumps(command))
    else:
        os.execvp(command[0], command)


if __name__ == "__main__":
    main()
