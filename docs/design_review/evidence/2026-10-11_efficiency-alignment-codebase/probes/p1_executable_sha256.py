"""P1: wall time of one full SHA-256 pass over the installed SurrealDB server executable.

Read-only. Run with `python3 -I probes/p1_executable_sha256.py PATH [REPEATS]`. It mirrors the
whole-file digest the fixture tooling computes (storage_service.digest), not that function itself.
"""
import hashlib
import sys
import time

path = sys.argv[1]
repeats = int(sys.argv[2]) if len(sys.argv) > 2 else 5
for i in range(repeats):
    start = time.perf_counter()
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    elapsed = time.perf_counter() - start
    print(f"pass {i}: {elapsed:.3f} s  {digest.hexdigest()[:16]}")
