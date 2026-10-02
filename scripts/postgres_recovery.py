"""Bounded retained-service backup; semantic generations are rebuilt from pinned inputs."""

from __future__ import annotations

import hashlib
import json
import os
import queue
import re
import subprocess
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
            "NOT (n.nspname='lctx_cache' AND c.relname IN ('serving_vectors','serving_vector_artifacts')) AND "
            "(n.nspname IN ('lctx_cache','lctx_ops') OR "
            "(n.nspname='public' AND c.relname='_sqlx_migrations'));"
        )
    )
    from postgres_backup import TABLES

    if set(tables) != set(TABLES) or any(
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


def backup(config, archive):
    from postgres_backup import TABLES, connection_env, protected

    protected(config)
    settings = json.loads(config.read_text())
    if "migration_url" not in settings:
        config = config.with_name("postgres-admin.json")
        protected(config)
        settings = json.loads(config.read_text())
    env = connection_env(settings["migration_url"])
    env["PGOPTIONS"] = "-c statement_timeout=900000 -c lock_timeout=5000 -c default_transaction_read_only=on"
    archive.parent.mkdir(parents=True, exist_ok=True)
    receipt = archive.with_suffix(archive.suffix + ".json")
    if archive.exists() or receipt.exists():
        raise RuntimeError("refusing to overwrite a backup")
    archive.touch(mode=0o600, exist_ok=False)
    session = Session(env)
    started = time.monotonic()
    try:
        snapshot = session.one("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; SELECT pg_export_snapshot();")
        tables = roots(session)
        state = fingerprints(session, tables)
        run(["pg_dump", "--format=custom", "--snapshot", snapshot,
             *[f"--table={table}" for table in TABLES],
             "--file", str(archive)], env=env, capture_output=True)
        list(session.lines("COMMIT;"))
        with archive.open("rb") as stream:
            checksum = hashlib.file_digest(stream, "sha256").hexdigest()
            os.fsync(stream.fileno())
        result = dict(format=4, scope="retained-services", fingerprint_algorithm="sha256-sorted-row-sha256-v2",
                      sha256=checksum, tables=state,
                      image=(ROOT / "specs/postgres-vector-image.txt").read_text().strip())
        pending = receipt.with_suffix(receipt.suffix + ".incomplete")
        secret(pending, result)
        with pending.open("rb") as stream:
            os.fsync(stream.fileno())
        pending.rename(receipt)
        sync_directory(receipt.parent)
        print(json.dumps(dict(outcome="passed", receipt_format=4, tables=len(state), seconds=time.monotonic()-started)))
    finally:
        session.close()


def restore(archive):
    from postgres_backup import TABLES, connection_env, protected

    protected(archive)
    receipt = archive.with_suffix(archive.suffix + ".json")
    protected(receipt)
    expected = json.loads(receipt.read_text())
    image = (ROOT / "specs/postgres-vector-image.txt").read_text().strip()
    if (expected.get("format") != 4 or expected.get("scope") != "retained-services"
        or expected.get("fingerprint_algorithm") != "sha256-sorted-row-sha256-v2"
        or set(expected.get("tables", {})) != set(TABLES) or expected.get("image") != image):
        raise RuntimeError("unsupported retained-service recovery receipt; rebuild semantic generations from pinned inputs")
    with archive.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != expected["sha256"]:
            raise RuntimeError("backup checksum mismatch")
    container = run(["docker", "run", "--rm", "--detach", "--publish", "127.0.0.1::5432",
                     "--env", "POSTGRES_PASSWORD=fixture-only", image], capture_output=True).stdout.strip()
    try:
        for _ in range(120):
            if subprocess.run(["docker", "exec", container, "pg_isready", "-h", "127.0.0.1", "-U", "postgres"], capture_output=True).returncode == 0:
                break
            time.sleep(0.25)
        else:
            raise RuntimeError("recovery database did not become ready")
        port = json.loads(run(["docker", "inspect", container], capture_output=True).stdout)[0]["NetworkSettings"]["Ports"]["5432/tcp"][0]["HostPort"]
        command = ["docker", "exec", "-i", container, "psql", "-X", "-qAt", "-U", "postgres", "-v", "ON_ERROR_STOP=1"]
        run(command, input=" ".join(f"CREATE ROLE {r} LOGIN PASSWORD 'fixture-only';" for r in ("lctx_app", "lctx_migrator", "lctx_importer", "lctx_serving")) + " CREATE DATABASE lctx OWNER lctx_migrator;", capture_output=True)
        # Table-only dumps deliberately exclude semantic schemas and do not create namespaces.
        run([*command,"-d","lctx"], input="CREATE SCHEMA lctx_cache AUTHORIZATION lctx_migrator; CREATE SCHEMA lctx_ops AUTHORIZATION lctx_migrator; REVOKE ALL ON SCHEMA lctx_cache,lctx_ops FROM PUBLIC; GRANT USAGE ON SCHEMA lctx_cache,lctx_ops TO lctx_app;", capture_output=True)
        run(["docker", "cp", str(archive), container+":/tmp/lctx.dump"], capture_output=True)
        run(["docker", "exec", container, "pg_restore", "-U", "postgres", "--dbname=lctx", "--exit-on-error", "/tmp/lctx.dump"], capture_output=True)
        session = Session(connection_env(f"postgres://postgres:fixture-only@127.0.0.1:{port}/lctx"))
        try:
            actual = fingerprints(session, roots(session))
        finally:
            session.close()
        if actual != expected["tables"]:
            raise RuntimeError("restored retained-service fingerprints differ")
        attempt = uuid.uuid4().hex
        run([*command,"-d","lctx"], input="SET ROLE lctx_app; INSERT INTO lctx_ops.attempts(attempt_id,compiler_digest,library,store_path) " + f"VALUES(decode('{attempt}','hex'),decode(repeat('00',32),'hex'),'recovery-probe','disposable'); " + "INSERT INTO lctx_ops.events(attempt_id,event_key,kind,detail) " + f"VALUES(decode('{attempt}','hex'),'recovery-probe','started','recovered writer verified'); RESET ROLE;", capture_output=True)
        print(json.dumps(dict(outcome="passed", scope="retained-services", tables=len(actual), semantic_generations="rebuild_from_pinned_inputs")))
    finally:
        run(["docker", "rm", "--force", container], stdout=subprocess.DEVNULL)
