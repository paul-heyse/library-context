"""Owned RocksDB engine experiments. Never invokes the service installer or fixture owner.

Run with repository Python 3.14.7. The destination must be new. Credentials are generated
in memory and never printed. All SQL touches only the fresh campaign's single database.
"""
from __future__ import annotations

import argparse
import base64
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
from storage_lifecycle import admission

EXPECTED = "710229df33346fdf9733ef86d46e31874f0cacc2691529327e19e6534477b8ed"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--grpc-probe", type=Path)
    parser.add_argument("--grpc-only", action="store_true")
    args = parser.parse_args()
    destination = args.destination.resolve()
    with admission([destination, args.binary.parent]):
        destination.mkdir(mode=0o700, parents=True, exist_ok=False)
        if run_dir := os.environ.get("LCTX_RUN_DIR"):
            import storage_owners
            storage_owners.enroll(destination, "run-log", {"kind": "run", "path": run_dir},
                                  "citation:efficiency-design-decisions")
        binary = destination / "surreal"
        with args.binary.open("rb") as source:
            assert hashlib.file_digest(source, "sha256").hexdigest() == EXPECTED
        shutil.copyfile(args.binary, binary)
        binary.chmod(0o500)
        with binary.open("rb") as copied:
            assert hashlib.file_digest(copied, "sha256").hexdigest() == EXPECTED
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        username, password = "campaign", secrets.token_urlsafe(32)
        env = {**os.environ, "SURREAL_USER": username, "SURREAL_PASS": password,
               "SURREAL_GRPC_MAX_MESSAGE_SIZE": "128MiB", "SURREAL_EXPORT_BATCH_SIZE": "1"}
        (destination / "tmp").mkdir()
        server_log = (destination / "server.log").open("wb")
        server = subprocess.Popen([str(binary), "start", "--no-banner", "--log=warn",
                                   "--bind", f"127.0.0.1:{port}", "--query-timeout", "20s",
                                   "--transaction-timeout", "10s", "--temporary-directory",
                                   str(destination / "tmp"), f"rocksdb://{destination / 'store'}"],
                                  env=env, stdout=server_log, stderr=subprocess.STDOUT,
                                  start_new_session=True)
        result: dict = {"server_sha256": EXPECTED, "port": port, "cases": {}}
        auth = "Basic " + base64.b64encode(f"{username}:{password}".encode()).decode()
        def query(sql: str):
            request = urllib.request.Request(f"http://127.0.0.1:{port}/sql", data=sql.encode(),
                                             headers={"Authorization": auth, "Accept": "application/json",
                                                      "Surreal-NS": "efficiency_campaign", "Surreal-DB": "decisions"})
            with urllib.request.urlopen(request, timeout=25) as response:
                return json.load(response)
        try:
            deadline = time.monotonic() + 40
            while True:
                if server.poll() is not None:
                    raise RuntimeError("owned server exited before ready")
                try:
                    query("RETURN 1;")
                    break
                except OSError:
                    if time.monotonic() > deadline:
                        raise
                    time.sleep(0.05)
            print("campaign ready", flush=True)
            initialized = query("DEFINE NAMESPACE efficiency_campaign; USE NS efficiency_campaign; DEFINE DATABASE decisions; USE DB decisions; DEFINE TABLE guard SCHEMALESS; DEFINE TABLE effects SCHEMALESS; DEFINE TABLE holds SCHEMALESS; DEFINE TABLE stripe SCHEMALESS;")
            assert all(row["status"] == "OK" for row in initialized), initialized
            def race(name: str, setup: str, slow: str, fast: str, inspect: str):
                setup_result = query(setup)
                assert all(row["status"] == "OK" for row in setup_result), setup_result
                with concurrent.futures.ThreadPoolExecutor() as pool:
                    started = time.monotonic()
                    a = pool.submit(query, slow)
                    time.sleep(0.25)
                    b = query(fast)
                    b_done = time.monotonic() - started
                    a_result = a.result()
                    a_done = time.monotonic() - started
                row = {"slow": a_result, "fast": b, "fast_finished_seconds": b_done,
                       "slow_finished_seconds": a_done, "final": query(inspect)}
                result["cases"][name] = row
                print(json.dumps({"case": name, **row}), flush=True)
            if args.grpc_only:
                grpc_cases(query, args.grpc_probe, env, port, destination, result)
                result["health"] = query("RETURN 1;")
                return
            race("existing_write_guard", "UPSERT guard:existing SET revision=0; DELETE effects;",
                 "BEGIN; UPDATE guard:existing SET revision+=1; SLEEP 2s; CREATE effects:slow; COMMIT;",
                 "BEGIN; UPDATE guard:existing SET revision+=1; CREATE effects:fast; COMMIT;",
                 "SELECT * FROM guard:existing; SELECT * FROM effects;")
            race("absent_guard_upsert", "DELETE guard:absent; DELETE effects;",
                 "BEGIN; UPSERT guard:absent SET revision=1; SLEEP 2s; CREATE effects:slow; COMMIT;",
                 "BEGIN; UPSERT guard:absent SET revision=2; CREATE effects:fast; COMMIT;",
                 "SELECT * FROM guard:absent; SELECT * FROM effects;")
            race("range_phantom", "DELETE holds; DELETE effects; UPSERT guard:range SET revision=0;",
                 "BEGIN; LET $absent = SELECT * FROM holds WHERE target='x'; SLEEP 2s; CREATE effects:retire SET seen=$absent; COMMIT;",
                 "BEGIN; CREATE holds:new SET target='x'; COMMIT;",
                 "SELECT * FROM holds; SELECT * FROM effects;")
            race("guarded_reference_predicate", "DELETE holds; DELETE effects; UPSERT guard:target SET revision=0;",
                 "BEGIN; UPDATE guard:target SET revision+=1; LET $absent = SELECT * FROM holds WHERE target='x'; SLEEP 2s; CREATE effects:retire SET seen=$absent; COMMIT;",
                 "BEGIN; UPDATE guard:target SET revision+=1; CREATE holds:new SET target='x'; COMMIT;",
                 "SELECT * FROM holds; SELECT * FROM effects;")
            race("named_read_guard", "UPSERT guard:read SET revision=0; DELETE effects;",
                 "BEGIN; LET $old = SELECT * FROM ONLY guard:read FOR UPDATE; SLEEP 2s; CREATE effects:slow SET seen=$old.revision; COMMIT;",
                 "BEGIN; UPDATE guard:read SET revision=1; CREATE effects:fast; COMMIT;",
                 "SELECT * FROM guard:read; SELECT * FROM effects;")
            race("era_all_stripes", "UPSERT stripe:zero SET revision=0; UPSERT stripe:one SET revision=0; DELETE effects;",
                 "BEGIN; UPDATE stripe:one SET revision+=1; SLEEP 2s; CREATE effects:ordinary; COMMIT;",
                 "BEGIN; UPDATE stripe:zero SET revision+=1; UPDATE stripe:one SET revision+=1; CREATE effects:cut; COMMIT;",
                 "SELECT * FROM stripe; SELECT * FROM effects;")
            race("independent_stripes", "UPSERT stripe:zero SET revision=0; UPSERT stripe:one SET revision=0; DELETE effects;",
                 "BEGIN; UPDATE stripe:zero SET revision+=1; SLEEP 2s; CREATE effects:zero; COMMIT;",
                 "BEGIN; UPDATE stripe:one SET revision+=1; CREATE effects:one; COMMIT;",
                 "SELECT * FROM stripe; SELECT * FROM effects;")
            result["cases"]["throw_rollback"] = query(
                "BEGIN; CREATE effects:rollback; UPDATE stripe:zero SET revision+=100; THROW 'injected'; COMMIT;")
            result["cases"]["throw_rollback_final"] = query("SELECT * FROM effects:rollback; SELECT * FROM stripe:zero;")
            result["cases"]["range_for_update_refused"] = query("BEGIN; SELECT * FROM holds WHERE target='x' FOR UPDATE; COMMIT;")
            validate_engine(result)
            if args.grpc_probe:
                grpc_cases(query, args.grpc_probe, env, port, destination, result)
            result["health"] = query("RETURN 1;")
        finally:
            os.killpg(server.pid, signal.SIGTERM)
            try:
                server.wait(timeout=60)
                result["cleanup"] = {"child_exit": server.returncode, "wait_confirmed": True}
            finally:
                server_log.close()
                (destination / "observations.json").write_text(json.dumps(result, indent=2) + "\n")
                print(json.dumps({"cleanup": result.get("cleanup")}), flush=True)


def validate_engine(result):
    cases = result["cases"]
    for name in ["existing_write_guard", "absent_guard_upsert", "guarded_reference_predicate",
                 "named_read_guard", "era_all_stripes"]:
        row = cases[name]
        assert any(x.get("details", {}).get("kind") == "TransactionConflict" for x in row["slow"]), name
        assert all(x["status"] == "OK" for x in row["fast"]), name
        assert row["fast_finished_seconds"] < row["slow_finished_seconds"], name
        assert not any(x.get("id") in ("effects:slow", "effects:retire", "effects:ordinary")
                       for population in row["final"] for x in population.get("result", []) or []), name
    for name in ["range_phantom", "independent_stripes"]:
        assert all(x["status"] == "OK" for x in cases[name]["slow"] + cases[name]["fast"]), name
    assert cases["range_phantom"]["final"][1]["result"] == [{"id": "effects:retire", "seen": []}]
    assert cases["throw_rollback_final"][0]["result"] == []
    assert cases["throw_rollback_final"][1]["result"][0]["revision"] == 1
    assert cases["range_for_update_refused"][1]["status"] == "ERR"
    result["engine_controls"] = "passed"


def grpc_cases(query, binary, env, port, destination, result):
    assert binary is not None
    setup = query("DEFINE NAMESPACE efficiency; USE NS efficiency; DEFINE DATABASE disconnect; USE DB disconnect; DEFINE TABLE grpc_probe SCHEMALESS;")
    assert all(row["status"] == "OK" for row in setup), setup
    cases = [
        ("drain_transaction", "BEGIN; CREATE grpc_probe:drained; SLEEP 2s; COMMIT;", 0),
        ("drop_transaction", "BEGIN; CREATE grpc_probe:uncommitted; SLEEP 2s; COMMIT;", 1),
        ("drop_autocommit", "CREATE grpc_probe:before; SLEEP 2s; CREATE grpc_probe:after;", 2),
    ]
    result["grpc"] = {}
    for name, sql, drop_after in cases:
        child = subprocess.run([str(binary), "--nocapture"], env={**env,
            "EFFICIENCY_GRPC_ENDPOINT": f"http://127.0.0.1:{port}",
            "EFFICIENCY_GRPC_USER": env["SURREAL_USER"],
            "EFFICIENCY_GRPC_PASSWORD": env["SURREAL_PASS"],
            "EFFICIENCY_GRPC_SQL": sql,
            "EFFICIENCY_GRPC_DROP_AFTER": str(drop_after),
            "EFFICIENCY_GRPC_DROP_DELAY_MS": "250"},
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=40)
        (destination / (name + ".log")).write_bytes(child.stdout)
        result["grpc"][name] = {"exit": child.returncode,
            "final": query("USE NS efficiency; USE DB disconnect; SELECT * FROM grpc_probe;")}
        print(name, child.stdout.decode(), result["grpc"][name], flush=True)


if __name__ == "__main__":
    main()
