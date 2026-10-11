"""P2: size and read cost of the storage-lifecycle object records (read-only).

Run with `python3 -I probes/p2_storage_records.py STATE_DIR [REPEATS]`. Mirrors the glob + JSON
parse of `scripts/storage_lifecycle.py::Storage.records`, without its validation or lock.
"""
import json
import sys
import time
from pathlib import Path

state = Path(sys.argv[1])
repeats = int(sys.argv[2]) if len(sys.argv) > 2 else 5
paths = sorted((state / "objects").glob("*.json"))
total = sum(path.stat().st_size for path in paths)
print(f"objects: {len(paths)} files, {total} bytes")
for name in ("paths", "scopes", "admission", "journals", "actions", "owners"):
    directory = state / name
    if directory.exists():
        print(f"{name}: {sum(1 for _ in directory.iterdir())} entries")
for i in range(repeats):
    start = time.perf_counter()
    rows = [json.loads(path.read_text()) for path in sorted((state / "objects").glob("*.json"))]
    print(f"pass {i}: {time.perf_counter() - start:.4f} s for {len(rows)} rows")
