"""Disposable PostgreSQL reference for synthetic data: database w1_probe, one schema per snapshot,
tables typed like the generation's physical tables (bytea ids/digests)."""
import json
import journeys

DB = "w1_probe"
PGT = {"id": "bytea", "digest": "bytea", "binary": "bytea", "int16": "smallint", "int32": "integer",
       "int64": "bigint", "bool": "boolean", "text": "text", "finite_f64": "double precision"}


def create_db():
    journeys.psql("postgres", f"DROP DATABASE IF EXISTS {DB}")
    journeys.psql("postgres", f"CREATE DATABASE {DB}")


def ensure_db():
    if journeys.psql("postgres", f"SELECT 1 FROM pg_database WHERE datname = '{DB}'").strip() != "1":
        journeys.psql("postgres", f"CREATE DATABASE {DB}")


def drop_db():
    journeys.psql("postgres", f"DROP DATABASE IF EXISTS {DB}")


def val(f, v):
    if v is None:
        return "NULL"
    if f["list"]:
        inner = [val(dict(f, list=False), x) for x in v]
        return f"ARRAY[{', '.join(inner)}]::{PGT[f['type']]}[]"
    t = f["type"]
    if t in ("id", "digest"):
        return f"decode('{v}','hex')"
    if t == "binary":
        return f"decode('{v.hex()}','hex')"
    if t == "text":
        return "'" + v.replace("'", "''") + "'"
    if t == "bool":
        return "true" if v else "false"
    return repr(v)


def load(model, schema, rows):
    stmts = [f"DROP SCHEMA IF EXISTS {schema} CASCADE", f"CREATE SCHEMA {schema}"]
    for name in model.rel:
        fields = model.rel[name]["fields"]
        cols = ["id bytea PRIMARY KEY"] + [f'"{f["name"]}" {PGT[f["type"]]}{"[]" if f["list"] else ""}' for f in fields]
        stmts.append(f"CREATE TABLE {schema}.{name} ({', '.join(cols)})")
        for r in rows.get(name, []):
            vals = [val({"type": "id", "list": False}, r["id"])] + [val(f, r.get(f["name"])) for f in fields]
            stmts.append(f"INSERT INTO {schema}.{name} VALUES ({', '.join(vals)})")
    journeys.psql(DB, ";\n".join(stmts) + ";")
