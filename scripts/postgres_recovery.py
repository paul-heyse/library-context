"Bounded libpq subprocess transport and complete serving recovery (no second query owner)."

from __future__ import annotations

import hashlib
import json
import os
import queue
import re
import shutil
import subprocess
import tempfile
import threading
import time
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, timeout=900, **kwargs)


class Session:
    "One exported snapshot; bounded producer queue and deadline on every response."

    def __init__(self, env):
        self.process = subprocess.Popen(
            ["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"],
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        assert self.process.stdin is not None and self.process.stdout is not None
        self.input = self.process.stdin
        stdout = self.process.stdout
        self.queue = queue.Queue(maxsize=128)
        self.closed = threading.Event()

        def read():
            while not self.closed.is_set():
                line = stdout.readline(32 * 1024 * 1024 + 1)
                item = line.rstrip("\n") if line and len(line) <= 32 * 1024 * 1024 else None
                while not self.closed.is_set():
                    try:
                        self.queue.put(item, timeout=0.1)
                        break
                    except queue.Full:
                        pass
                if item is None:
                    return

        self.thread = threading.Thread(target=read, daemon=True)
        self.thread.start()
        list(
            self.lines(
                "SET TimeZone='UTC'; SET DateStyle='ISO, YMD'; SET IntervalStyle='p"
                "ostgres'; SET extra_float_digits=3; SET bytea_output='hex';"
            )
        )

    def lines(self, sql):
        marker = "lctx_end_" + uuid.uuid4().hex
        self.input.write(sql + "\n\\echo " + marker + "\n")
        self.input.flush()
        deadline = time.monotonic() + 900
        while True:
            try:
                item = self.queue.get(timeout=max(0.001, deadline - time.monotonic()))
            except queue.Empty:
                raise RuntimeError("backup query deadline exceeded") from None
            if item is None:
                raise RuntimeError("backup query failed or exceeded its output bound")
            if item == marker:
                return
            yield item

    def one(self, sql):
        lines = list(self.lines(sql))
        if len(lines) != 1:
            raise RuntimeError("unexpected backup query response")
        return lines[0]

    def close(self):
        self.closed.set()
        if self.process.poll() is None:
            self.process.terminate()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=10)
        self.thread.join(timeout=1)


def roots(session):
    tables = json.loads(
        session.one(
            "SELECT coalesce(json_agg(n.nspname||'.'||c.relname "
            "ORDER BY n.nspname,c.relname),'[]') "
            "FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace "
            "WHERE c.relkind IN ('r','p') AND NOT c.relispartition AND "
            "(n.nspname IN ('lctx_cache','lctx_ops','lctx_serving') OR "
            "(n.nspname='public' AND c.relname='_sqlx_migrations'));"
        )
    )
    if len(tables) > 512 or any(
        not re.fullmatch(r"[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*", t) for t in tables
    ):
        raise RuntimeError("backup relation inventory refused")
    return tables


def fingerprints(session, tables):
    result = {}
    for table in tables:
        if not re.fullmatch(r"[a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*", table):
            raise RuntimeError("invalid recovery relation")
        digest = hashlib.sha256()
        count = 0
        for row in session.lines(
            "COPY (SELECT encode(sha256(convert_to(to_jsonb(t)::text,'UTF8')),'hex') "
            'COLLATE "C" AS h '
            f"FROM {table} t ORDER BY h) TO STDOUT;"
        ):
            if not re.fullmatch(r"[0-9a-f]{64}", row):
                raise RuntimeError("invalid streamed recovery hash")
            digest.update(bytes.fromhex(row))
            count += 1
        result[table] = {"rows": count, "digest": digest.hexdigest()}
    return result


def secret(path, value):
    with open(path, "x", opener=lambda p, f: os.open(p, f, 0o600)) as out:
        json.dump(value, out, indent=2)
        out.write("\n")


def sync_directory(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def artifact_target(root, artifact):
    name, digest = artifact["name"], artifact["sha256"]
    if Path(name).name != name or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise RuntimeError("invalid recovery artifact identity")
    return root / digest / name


def verify_file(path, artifact):
    if path.is_symlink() or not path.is_file() or path.stat().st_size != artifact["bytes"]:
        raise RuntimeError("recovery artifact missing or size mismatch")
    with path.open("rb") as source:
        if hashlib.file_digest(source, "sha256").hexdigest() != artifact["sha256"]:
            raise RuntimeError("recovery artifact checksum mismatch")


def backup(config, archive):
    from postgres_backup import connection_env, protected

    protected(config)
    settings = json.loads(config.read_text())
    if "migration_url" not in settings:
        config = config.with_name("postgres-admin.json")
        protected(config)
        settings = json.loads(config.read_text())
    env = connection_env(settings["migration_url"])
    env["PGOPTIONS"] = (
        "-c statement_timeout=900000 -c lock_timeout=5000 -c default_transaction_read_only=on"
    )
    archive.parent.mkdir(parents=True, exist_ok=True)
    receipt = archive.with_suffix(archive.suffix + ".json")
    artifact_root = archive.with_suffix(archive.suffix + ".artifacts")
    if archive.exists() or receipt.exists() or artifact_root.exists():
        raise RuntimeError("refusing to overwrite a backup")
    archive.touch(mode=0o600, exist_ok=False)
    artifact_root.mkdir(mode=0o700)
    session = Session(env)
    started = time.monotonic()
    try:
        snapshot = session.one(
            "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; SELECT pg_export_snapshot();"
        )
        tables = roots(session)
        state = fingerprints(session, tables)
        inventory: dict = {"format": 2, "generations": []}
        if "lctx_serving.generations" in tables and int(
            session.one("SELECT count(*) FROM lctx_serving.generations WHERE state='ready';")
        ):
            raw = run(
                [
                    str(ROOT / "target/release/lctx"),
                    "serving",
                    "--importer-config",
                    str(config.with_name("postgres-importer.json")),
                    "recovery-inventory",
                    "--snapshot",
                    snapshot,
                ],
                capture_output=True,
            ).stdout
            if len(raw) > 32 * 1024 * 1024:
                raise RuntimeError("recovery inventory byte budget")
            inventory = json.loads(raw)
        selections = (
            json.loads(
                session.one(
                    "SELECT coalesce(json_agg(json_build_object('library',library,'gene"
                    "ration',encode(generation_digest,'hex'),'profile',encode(profile_d"
                    "igest,'hex')) ORDER BY library),'[]') FROM lctx_serving.selections"
                    ";"
                )
            )
            if "lctx_serving.selections" in tables
            else []
        )
        versions = json.loads(
            session.one(
                "SELECT json_build_object('server',current_setting('server_version_"
                "num'),'extension',(SELECT extversion FROM pg_extension WHERE extna"
                "me='vector'));"
            )
        )
        fingerprint_seconds = time.monotonic() - started
        dump_started = time.monotonic()
        run(
            ["pg_dump", "--format=custom", "--snapshot", snapshot, "--file", str(archive)],
            env=env,
            capture_output=True,
        )
        dump_seconds = time.monotonic() - dump_started
        list(session.lines("COMMIT;"))
        artifact_started = time.monotonic()
        for generation in inventory["generations"]:
            for artifact in generation["artifacts"]:
                target = artifact_target(artifact_root, artifact)
                target.parent.mkdir(mode=0o700, exist_ok=True)
                if not target.exists():
                    source = Path(artifact["location"])
                    verify_file(source, artifact)
                    with source.open("rb") as inp, target.open("xb") as out:
                        target.chmod(0o600)
                        shutil.copyfileobj(inp, out, 1024 * 1024)
                        out.flush()
                        os.fsync(out.fileno())
                verify_file(target, artifact)
        with archive.open("rb") as source:
            checksum = hashlib.file_digest(source, "sha256").hexdigest()
        result = dict(
            format=3,
            fingerprint_algorithm="sha256-sorted-row-sha256-v2",
            sha256=checksum,
            tables=state,
            inventory=inventory,
            selections=selections,
            versions=versions,
            image=(ROOT / "specs/postgres-vector-image.txt").read_text().strip(),
            timings=dict(
                fingerprints_seconds=fingerprint_seconds,
                dump_seconds=dump_seconds,
                artifact_seconds=time.monotonic() - artifact_started,
                total_seconds=time.monotonic() - started,
            ),
        )
        # Completion becomes visible only after durable dump/artifact bytes are verified.
        for path in [archive, *artifact_root.rglob("*")]:
            if path.is_file():
                with path.open("rb") as stream:
                    os.fsync(stream.fileno())
        temp = receipt.with_suffix(receipt.suffix + ".incomplete")
        for directory in artifact_root.iterdir():
            sync_directory(directory)
        sync_directory(artifact_root)
        secret(temp, result)
        with temp.open("rb") as stream:
            os.fsync(stream.fileno())
        temp.rename(receipt)
        sync_directory(receipt.parent)
        print(
            json.dumps(
                {
                    "outcome": "passed",
                    "receipt_format": 3,
                    "ready_generations": len(inventory["generations"]),
                    "tables": len(state),
                    "seconds": time.monotonic() - started,
                }
            )
        )
    finally:
        session.close()


def restore(archive):
    started = time.monotonic()
    from postgres_backup import connection_env, protected

    protected(archive)
    receipt = archive.with_suffix(archive.suffix + ".json")
    protected(receipt)
    expected = json.loads(receipt.read_text())
    if (
        expected.get("format") != 3
        or expected.get("inventory", {}).get("format") != 2
        or expected["fingerprint_algorithm"] != "sha256-sorted-row-sha256-v2"
    ):
        raise RuntimeError(
            "unsupported recovery receipt; regenerate obsolete state with the current runtime"
        )
    image = (ROOT / "specs/postgres-vector-image.txt").read_text().strip()
    if expected["image"] != image:
        raise RuntimeError("recovery image pin mismatch")
    with archive.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != expected["sha256"]:
            raise RuntimeError("backup checksum mismatch")
    artifact_root = archive.with_suffix(archive.suffix + ".artifacts")
    for gen in expected["inventory"]["generations"]:
        raw = gen["canonical_manifest"]
        if (
            hashlib.sha256(raw.encode()).hexdigest() != gen["generation"]
            or json.loads(raw) != gen["manifest"]
        ):
            raise RuntimeError("recovery canonical manifest identity mismatch")
        for artifact in gen["artifacts"]:
            verify_file(artifact_target(artifact_root, artifact), artifact)
    container = run(
        [
            "docker",
            "run",
            "--rm",
            "--detach",
            "--publish",
            "127.0.0.1::5432",
            "--env",
            "POSTGRES_PASSWORD=fixture-only",
            image,
        ],
        capture_output=True,
    ).stdout.strip()
    try:
        for _ in range(120):
            if (
                subprocess.run(
                    [
                        "docker",
                        "exec",
                        container,
                        "pg_isready",
                        "-h",
                        "127.0.0.1",
                        "-U",
                        "postgres",
                    ],
                    capture_output=True,
                    timeout=10,
                ).returncode
                == 0
            ):
                break
            time.sleep(0.25)
        else:
            raise RuntimeError("recovery database did not become ready")
        port = json.loads(run(["docker", "inspect", container], capture_output=True).stdout)[0][
            "NetworkSettings"
        ]["Ports"]["5432/tcp"][0]["HostPort"]
        command = [
            "docker",
            "exec",
            "-i",
            container,
            "psql",
            "-X",
            "-qAt",
            "-U",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
        ]
        run(
            command,
            input=" ".join(
                f"CREATE ROLE {r} LOGIN PASSWORD 'fixture-only';"
                for r in ("lctx_app", "lctx_migrator", "lctx_importer", "lctx_serving")
            )
            + " CREATE DATABASE lctx OWNER lctx_migrator;",
            capture_output=True,
        )
        command += ["-d", "lctx"]
        run(["docker", "cp", str(archive), container + ":/tmp/lctx.dump"], capture_output=True)
        run(
            [
                "docker",
                "exec",
                container,
                "pg_restore",
                "-U",
                "postgres",
                "--dbname=lctx",
                "--exit-on-error",
                "/tmp/lctx.dump",
            ],
            capture_output=True,
        )
        dump_seconds = time.monotonic() - started
        session = Session(connection_env(f"postgres://postgres:fixture-only@127.0.0.1:{port}/lctx"))
        try:
            actual = fingerprints(session, roots(session))
            restored = json.loads(
                session.one(
                    "SELECT coalesce(json_agg(json_build_object('generation',encode(gen"
                    "eration_digest,'hex'),'manifest',canonical_manifest::json) ORDER B"
                    "Y generation_digest),'[]') FROM lctx_serving.generations WHERE sta"
                    "te='ready';"
                )
            )
            inventory = expected["inventory"]["generations"]
            if restored != [
                {"generation": g["generation"], "manifest": g["manifest"]} for g in inventory
            ]:
                raise RuntimeError("recovery inventory differs from restored ready generations")
            for generation in inventory:
                artifacts = {
                    a["name"]: {k: a[k] for k in ("sha256", "bytes", "format")}
                    for a in generation["artifacts"]
                }
                if (
                    len(artifacts) != len(generation["artifacts"])
                    or artifacts != generation["manifest"]["artifacts"]
                ):
                    raise RuntimeError("recovery artifact closure differs from manifest")
            selected = json.loads(
                session.one(
                    "SELECT coalesce(json_agg(json_build_object('library',library,'gene"
                    "ration',encode(generation_digest,'hex'),'profile',encode(profile_d"
                    "igest,'hex')) ORDER BY library),'[]') FROM lctx_serving.selections"
                    ";"
                )
            )
            if selected != expected["selections"]:
                raise RuntimeError("recovery selection inventory differs from restored rows")
        finally:
            session.close()
        if actual != expected["tables"]:
            raise RuntimeError("restored logical table fingerprints differ")
        if "lctx_serving.import_attempts" in actual:
            run(
                command,
                input=(
                    "DO $$ BEGIN IF nextval(pg_get_serial_sequence("
                    "'lctx_serving.import_attempts','attempt_id')) <= "
                    "(SELECT coalesce(max(attempt_id),0) FROM lctx_serving.import_attempts) "
                    "THEN RAISE EXCEPTION 'recovered sequence is behind'; END IF; END $$;"
                ),
                capture_output=True,
            )
        attempt = uuid.uuid4().hex
        run(
            command,
            input=(
                "SET ROLE lctx_app; INSERT INTO lctx_ops.attempts"
                "(attempt_id,compiler_digest,library,store_path) "
                f"VALUES(decode('{attempt}','hex'),decode(repeat('00',32),'hex'),"
                "'recovery-probe','disposable'); INSERT INTO lctx_ops.events"
                "(attempt_id,event_key,kind,detail) "
                f"VALUES(decode('{attempt}','hex'),'recovery-probe','started',"
                "'recovered writer verified'); RESET ROLE;"
            ),
            capture_output=True,
        )
        # Only the disposable clone loses old locators. Source artifacts remain untouched.
        if expected["inventory"]["generations"]:
            run(
                command,
                input=(
                    "UPDATE lctx_serving.artifact_locations SET location='/nonexistent/"
                    "lctx-recovery/'||encode(generation_digest,'hex')||'/'||name||'/'||"
                    "encode(sha256(convert_to(location,'UTF8')),'hex');"
                ),
                capture_output=True,
            )
        with tempfile.TemporaryDirectory(prefix="lctx-recovery-") as tmp:
            config_root = Path(tmp)
            for role in ("importer", "serving"):
                secret(
                    config_root / f"postgres-{role}.json",
                    dict(
                        format=1,
                        role=role,
                        url=f"postgres://lctx_{role}:fixture-only@127.0.0.1:{port}/lctx",
                        max_connections=10 if role == "importer" else 2,
                        provider_connections=8 if role == "importer" else 0,
                        acquire_timeout_seconds=5,
                        statement_timeout_seconds=30,
                        lock_timeout_seconds=5,
                    ),
                )
            run(
                command,
                input=(
                    "REVOKE TEMP ON DATABASE lctx FROM PUBLIC; GRANT TEMP ON DATABASE l"
                    "ctx TO lctx_importer; ALTER ROLE lctx_serving SET default_transact"
                    "ion_read_only=on;"
                ),
                capture_output=True,
            )
            cli = [
                str(ROOT / "target/release/lctx"),
                "serving",
                "--importer-config",
                str(config_root / "postgres-importer.json"),
            ]
            for gen in expected["inventory"]["generations"]:
                run(
                    [
                        *cli,
                        "relocate-artifacts",
                        "--generation",
                        gen["generation"],
                        "--artifacts",
                        str(artifact_root.resolve()),
                    ],
                    capture_output=True,
                )
                run([*cli, "reconcile", "--generation", gen["generation"]], capture_output=True)
            for selected in expected["selections"]:
                run(
                    [
                        *cli,
                        "select",
                        "--library",
                        selected["library"],
                        "--generation",
                        selected["generation"],
                    ],
                    capture_output=True,
                )
            result = run(
                [
                    str(ROOT / ".venv/bin/python"),
                    str(ROOT / "scripts/postgres_recovery_probe.py"),
                    str(config_root / "postgres-serving.json"),
                    str(receipt),
                ],
                capture_output=True,
            )
            probe = json.loads(result.stdout)
        elapsed = time.monotonic() - started
        if elapsed > 900:
            raise RuntimeError("local restore exceeded the 15-minute objective")
        print(
            json.dumps(
                {
                    "outcome": "passed",
                    "tables": len(actual),
                    "serving": probe,
                    "all_generations_preserved": True,
                    "dump_restore_seconds": dump_seconds,
                    "restore_seconds": elapsed,
                    "rto_seconds": 900,
                    "rto_passed": True,
                    "ann": "requires_new_physical_admission",
                }
            )
        )
    finally:
        run(["docker", "rm", "--force", container], stdout=subprocess.DEVNULL)
