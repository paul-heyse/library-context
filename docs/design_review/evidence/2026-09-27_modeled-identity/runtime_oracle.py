"""Independent generated lexical/model identity controls; no analyzer fixture is executed."""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

CASES = {
    "framed": ("try:\n        return cast(object, value)\n    finally:\n        pass", True),
    "nested_frames": (
        "try:\n        try:\n            return cast(object, value)\n"
        "        finally:\n            pass\n    finally:\n        pass",
        True,
    ),
    "keyword": (
        "try:\n        return cast(typ=object, val=value)\n    finally:\n        pass",
        True,
    ),
    "rebound": (
        "value = 7\n    try:\n        return cast(object, value)\n    finally:\n        pass",
        False,
    ),
    "deleted": (
        "del value\n    try:\n        return cast(object, value)\n    finally:\n        pass",
        False,
    ),
    "overridden": (
        "try:\n        return cast(object, value)\n    finally:\n        return 7",
        False,
    ),
    "nested_mutation": (
        "def mutate():\n        nonlocal value\n        value = 7\n"
        "    mutate()\n    try:\n        return cast(object, value)\n    finally:\n        pass",
        False,
    ),
}
WORKER = r"""
import hashlib, json, resource, socket, sys, typing
resource.setrlimit(resource.RLIMIT_CPU, (2, 2))
resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
socket.socket = lambda *a, **k: (_ for _ in ()).throw(RuntimeError("network forbidden"))
assert sys.version_info[:3] == (3, 14, 7)
source = open(sys.argv[1], encoding="utf-8").read()
namespace = {}
exec(compile(source, sys.argv[1], "exec"), namespace)
value = object()
try:
    result = namespace["run"](value)
    outcome = "return"
    same = result is value
except BaseException as error:
    outcome = type(error).__name__
    same = False
print(json.dumps({"same_identity": same, "outcome": outcome,
    "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
    "typing_sha256": hashlib.sha256(open(typing.__file__, "rb").read()).hexdigest()}))
"""


def main() -> None:
    outcomes = {}
    with tempfile.TemporaryDirectory(prefix="lctx-model-identity-oracle-") as temp:
        for name, (body, expected) in CASES.items():
            source = f"from typing import cast\ndef run(value):\n    {body}\n"
            path = Path(temp) / f"{name}.py"
            path.write_text(source, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, "-I", "-c", WORKER, str(path)],
                cwd=temp,
                capture_output=True,
                text=True,
                timeout=5,
                env={"PATH": os.environ.get("PATH", "")},
                check=True,
            )
            observed = json.loads(result.stdout)
            assert observed["same_identity"] == expected, (name, observed)
            outcomes[name] = observed
    print(
        json.dumps(
            {
                "outcome": "passed",
                "python": sys.version.split()[0],
                "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                "cases": outcomes,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
