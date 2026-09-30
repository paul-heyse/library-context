"""The operator transition (cutover plan P1.12/P1.13) on a disposable PG18 carrying a legacy
migration history. Opt in with LCTX_POSTGRES_TEST=1; needs Docker, the pinned image and a release
`target/release/lctx`."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from postgres_backup import connection_env, fingerprints  # noqa: E402
from postgres_bootstrap import bootstrap_sql, configurations, provision_sql, write_secret  # noqa: E402

pytestmark = pytest.mark.skipif(os.environ.get("LCTX_POSTGRES_TEST") != "1", reason="set LCTX_POSTGRES_TEST=1 (Docker, pinned PG18)")
LCTX = ROOT / "target/release/lctx"
TRANSITION = ROOT / "scripts/postgres_transition.py"
RETAINED = ("lctx_cache.specs", "lctx_cache.embedding_values", "lctx_ops.attempts", "lctx_ops.events")

# The retired service migrations 0001 and 0002 (Git a594e8a), which the operator database carries.
LEGACY = """
CREATE TABLE public._sqlx_migrations (version BIGINT PRIMARY KEY, description TEXT NOT NULL,
  installed_on TIMESTAMPTZ NOT NULL DEFAULT now(), success BOOLEAN NOT NULL, checksum BYTEA NOT NULL, execution_time BIGINT NOT NULL);
INSERT INTO public._sqlx_migrations(version, description, success, checksum, execution_time)
  SELECT v, 'legacy', true, '\\x00', 1 FROM unnest(ARRAY[202609270001,202609270002,202609270003,202609270004,202609280005,202609280006,
  202609280007,202609280008,202609280009,202609280010,202609280011,202609280012,202609300013]) AS v;
CREATE SCHEMA lctx_cache; CREATE SCHEMA lctx_ops; CREATE SCHEMA lctx_serving;
CREATE TABLE lctx_serving.generations (generation_digest bytea PRIMARY KEY);
CREATE TABLE lctx_cache.specs (spec_hash bytea PRIMARY KEY CHECK (octet_length(spec_hash) = 32), canonical_spec text NOT NULL,
  dimensions integer NOT NULL CHECK (dimensions > 0 AND dimensions <= 65536));
CREATE TABLE lctx_cache.embedding_values (spec_hash bytea NOT NULL REFERENCES lctx_cache.specs(spec_hash),
  input_hash bytea NOT NULL CHECK (octet_length(input_hash) = 32), codec smallint NOT NULL CHECK (codec = 1),
  dimensions integer NOT NULL CHECK (dimensions > 0 AND dimensions <= 65536), vector_bytes bytea NOT NULL CHECK (octet_length(vector_bytes) = dimensions * 4),
  value_digest bytea NOT NULL CHECK (octet_length(value_digest) = 32), admitted_tokens integer NOT NULL CHECK (admitted_tokens >= 0),
  PRIMARY KEY (spec_hash, input_hash));
CREATE TABLE lctx_ops.attempts (attempt_id bytea PRIMARY KEY CHECK (octet_length(attempt_id) = 16),
  compiler_digest bytea NOT NULL CHECK (octet_length(compiler_digest) = 32), library text NOT NULL CHECK (length(library) BETWEEN 1 AND 1024),
  store_path text NOT NULL, started_at timestamptz NOT NULL DEFAULT clock_timestamp());
CREATE TABLE lctx_ops.events (attempt_id bytea NOT NULL REFERENCES lctx_ops.attempts(attempt_id),
  event_key text NOT NULL CHECK (length(event_key) BETWEEN 1 AND 256),
  kind text NOT NULL CHECK (kind IN ('started', 'stage', 'published', 'generated', 'failed', 'interrupted')),
  detail text NOT NULL CHECK (octet_length(detail) <= 4096), recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(), PRIMARY KEY (attempt_id, event_key));
CREATE TABLE lctx_ops.snapshots (store_path text NOT NULL, snapshot_id bytea NOT NULL, PRIMARY KEY (store_path, snapshot_id));
ALTER TABLE lctx_ops.attempts ALTER COLUMN library DROP NOT NULL;
ALTER TABLE lctx_ops.attempts ALTER COLUMN started_at DROP NOT NULL;
ALTER TABLE lctx_ops.attempts ADD COLUMN registration text NOT NULL DEFAULT 'started' CHECK (registration IN ('started', 'reconciled'));
ALTER TABLE lctx_ops.attempts ADD COLUMN observed_at timestamptz NOT NULL DEFAULT clock_timestamp();
INSERT INTO lctx_cache.specs VALUES (decode(repeat('11', 32), 'hex'), '{"format":2}', 2);
INSERT INTO lctx_cache.embedding_values VALUES (decode(repeat('11', 32), 'hex'), decode(repeat('21', 32), 'hex'), 1, 2,
  decode('0000803f00000000', 'hex'), decode(repeat('31', 32), 'hex'), 3);
INSERT INTO lctx_ops.attempts(attempt_id, compiler_digest, library, store_path) VALUES (decode(repeat('01', 16), 'hex'), decode(repeat('02', 32), 'hex'), 'fastmcp', 'build/store');
INSERT INTO lctx_ops.attempts(attempt_id, compiler_digest, library, store_path, started_at, registration)
  VALUES (decode(repeat('03', 16), 'hex'), decode(repeat('02', 32), 'hex'), NULL, 'build/store', NULL, 'reconciled');
INSERT INTO lctx_ops.events(attempt_id, event_key, kind, detail) VALUES (decode(repeat('01', 16), 'hex'), 'started', 'started', ''),
  (decode(repeat('01', 16), 'hex'), 'publish', 'published', 'snapshot'), (decode(repeat('03', 16), 'hex'), 'operator/interrupted', 'interrupted', '');
INSERT INTO lctx_ops.snapshots VALUES ('build/store', decode(repeat('04', 16), 'hex'));
"""


def run(args, **kw):
    return subprocess.run(args, text=True, capture_output=True, **kw)


@pytest.fixture
def legacy(tmp_path):
    image = (ROOT / "specs/postgres-vector-image.txt").read_text().strip()
    container = run(["docker", "run", "--rm", "-d", "-p", "127.0.0.1::5432", "-e", "POSTGRES_PASSWORD=fixture-only", image], check=True).stdout.strip()
    try:
        port = int(json.loads(run(["docker", "inspect", container], check=True).stdout)[0]["NetworkSettings"]["Ports"]["5432/tcp"][0]["HostPort"])
        for _ in range(120):
            if run(["docker", "exec", container, "pg_isready", "-h", "127.0.0.1"]).returncode == 0:
                break
            time.sleep(0.25)
        admin = ["docker", "exec", "-i", container, "psql", "-X", "-qAt", "-U", "postgres", "-v", "ON_ERROR_STOP=1"]
        passwords = {role: "ab" * 32 for role in ("lctx_app", "lctx_migrator", "lctx_importer", "lctx_serving")}
        run(admin, input=bootstrap_sql(passwords), check=True)
        run(admin + ["-d", "lctx"], input=provision_sql({r: passwords[r] for r in ("lctx_importer", "lctx_serving")}), check=True)
        for name, data in configurations(passwords, port).items():
            write_secret(tmp_path / name, data)
        write_secret(tmp_path / "admin.json", {"url": f"postgres://postgres:fixture-only@127.0.0.1:{port}/postgres?sslmode=disable"})
        owner = connection_env(f"postgres://lctx_migrator:{'ab' * 32}@127.0.0.1:{port}/lctx?sslmode=disable")
        run(["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"], input=LEGACY, env=owner, check=True)
        yield tmp_path, owner
    finally:
        run(["docker", "rm", "-f", container])


def transition(directory, *args):
    return run([sys.executable, str(TRANSITION), *args, "--config", str(directory / "postgres.json"), "--admin-config", str(directory / "admin.json")])


def lctx(directory, *args):
    env = {k: v for k, v in os.environ.items() if k != "LCTX_DATABASE_CONFIG"}
    return run([str(LCTX), "--database", str(directory / "postgres.json"), *args], env=env)


def fingerprint(env, database):
    env = dict(env, PGDATABASE=database)
    query = lambda sql: run(["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"], input=sql, env=env, check=True).stdout.strip()  # noqa: E731
    return fingerprints(query, RETAINED)


def test_the_legacy_database_moves_to_a_fresh_baseline_and_is_archived(legacy):
    directory, owner = legacy
    before = fingerprint(owner, "lctx")
    # The binary refuses the legacy history rather than upgrading it.
    assert lctx(directory, "store", "install").returncode == 1
    planned = transition(directory, "plan")
    assert planned.returncode == 0, planned.stderr
    plan = json.loads(planned.stdout)
    assert plan["legacy_history"] and plan["retained"] == before and "lctx_serving" in plan["left_behind"]
    prepared = transition(directory, "prepare")
    assert prepared.returncode == 0, prepared.stderr
    assert transition(directory, "prepare").returncode == 2, "a second prepare is refused while lctx_next exists"
    assert transition(directory, "switch").returncode == 2, "the switch needs its confirmation"
    switched = transition(directory, "switch", "--confirm-switch", "lctx")
    assert switched.returncode == 0, switched.stderr
    archive = json.loads(switched.stdout)["archive"]
    assert fingerprint(owner, "lctx") == before, "the retained rows are equal after the move"
    assert lctx(directory, "store", "check").returncode == 0
    runs = lctx(directory, "runs", "list")
    assert runs.returncode == 0 and "0101010101" in runs.stdout and "reconciled" in runs.stdout, runs.stdout
    # The archive is the untouched legacy database.
    archived = dict(owner, PGDATABASE=archive)
    schemas = run(["psql", "-X", "-qAt", "-c", "SELECT string_agg(nspname, ',' ORDER BY nspname) FROM pg_namespace WHERE nspname LIKE 'lctx%'"],
                  env=archived, check=True).stdout.strip()
    assert "lctx_serving" in schemas and fingerprint(owner, archive) == before
    assert transition(directory, "drop-retired", "--confirm-drop", "lctx").returncode == 2, "only an archive can be dropped"
    assert transition(directory, "drop-retired", "--confirm-drop", archive).returncode == 0
    assert run(["psql", "-X", "-qAt", "-c", "SELECT 1"], env=archived).returncode != 0, "the archive is gone"


def test_a_current_database_has_nothing_to_transition(legacy):
    directory, owner = legacy
    run(["psql", "-X", "-qAt", "-v", "ON_ERROR_STOP=1"], input="DELETE FROM public._sqlx_migrations;", env=owner, check=True)
    assert transition(directory, "prepare").returncode == 2
