from __future__ import annotations

import json
import os
from pathlib import Path
from types import SimpleNamespace

import library_catalog_db as db
import library_semantic as sem

LIBS = [
    {"kind": "library", "lib": "sqlx", "skill": "sqlx-postgres", "status": "used", "pin": "=0.9.0"},
    {
        "kind": "library",
        "lib": "pgpq",
        "skill": "sqlx-postgres",
        "status": "used",
        "pin": "=0.12.0",
    },
    {"kind": "library", "lib": "zeta", "skill": None, "status": "not-used", "pin": None},
]


def capability(id_: str, lib: str, feature: str, use: str, items: list[str], **extra) -> dict:
    return {
        "kind": "capability",
        "id": id_,
        "lib": lib,
        "feature": feature,
        "use": use,
        "items": items,
        "files": extra.pop("files", []),
        "status": "used",
        **extra,
    }


CAPS = [
    capability(
        "sqlx/runtime-query-bind",
        "sqlx",
        "Runtime queries with binds",
        "Bind parameters and fetch rows",
        ["sqlx_core::query::query", "sqlx_core::query_as::query_as", "sqlx::query_file!"],
        as_written=["sqlx::query_as"],
        files=[
            {"path": "crates/pg/src/a_b.rs", "role": "impl"},
            {"path": "crates/pg/src/c.rs", "role": "consumer"},
        ],
        wrapper={"path": "crates/pg/src/lib.rs", "symbol": "Store"},
    ),
    capability(
        "pgpq/batch-copy",
        "pgpq",
        "Binary COPY staging",
        "Encode Arrow batches for COPY",
        ["pgpq::ArrowToPostgresBinaryEncoder::write_batch"],
        files=[{"path": "crates/pg/src/copy.rs", "role": "impl"}],
    ),
]


def write_catalog(tmp_path: Path) -> Path:
    path = tmp_path / "catalog.jsonl"
    path.write_text("".join(json.dumps(r) + "\n" for r in LIBS + CAPS))
    return path


def test_catalog_queries_return_every_match_and_search_ranks_without_a_cutoff(tmp_path) -> None:
    store = db.Store(write_catalog(tmp_path), tmp_path / "none.sqlite")
    assert [c["id"] for c in store.capabilities()] == ["pgpq/batch-copy", "sqlx/runtime-query-bind"]
    sqlx = store.library("sqlx")
    assert sqlx is not None and sqlx["pin"] == "=0.9.0" and store.library("missing") is None
    assert store.skills() == {"sqlx-postgres": ["pgpq", "sqlx"], "": ["zeta"]}
    hits = store.search("bind fetch rows")
    assert [c["id"] for c, _ in hits] == ["sqlx/runtime-query-bind"]
    assert [c["id"] for c, _ in store.search("copy")] == ["pgpq/batch-copy"]
    assert len(store.search("")) == 2  # an empty query matches everything, ordered
    both = {c["id"] for c, _ in store.search("bind copy")}
    assert both == {"sqlx/runtime-query-bind", "pgpq/batch-copy"}  # any token: recall first
    assert [c["id"] for c, _ in store.search("query_as")] == ["sqlx/runtime-query-bind"]
    by_skill = {c["id"] for c, _ in store.search("sqlx-postgres")}
    assert by_skill == {"pgpq/batch-copy", "sqlx/runtime-query-bind"}  # the skill name is indexed


def test_items_match_as_items_as_written_spellings_and_leaves_and_macros_keep_their_bang(
    tmp_path,
) -> None:
    store = db.Store(write_catalog(tmp_path), tmp_path / "none.sqlite")
    ids = lambda cs: [c["id"] for c in cs]  # noqa: E731
    assert ids(store.capabilities_with_items(["sqlx_core::query_as::query_as"])) == [
        "sqlx/runtime-query-bind"
    ]
    assert ids(store.capabilities_with_items(["sqlx::query_as"])) == ["sqlx/runtime-query-bind"]
    assert ids(store.capabilities_with_items(["sqlx::query_file"])) == ["sqlx/runtime-query-bind"]
    assert ids(store.capabilities_with_items([], leaf="QUERY_AS")) == ["sqlx/runtime-query-bind"]
    assert store.capabilities_with_items(["sqlx::nothing"]) == []
    assert store.capabilities_with_items([]) == []


def test_files_and_wrappers_match_exactly_and_by_directory_with_underscores_and_percents(
    tmp_path,
) -> None:
    store = db.Store(write_catalog(tmp_path), tmp_path / "none.sqlite")
    ids = lambda cs: sorted(c["id"] for c in cs)  # noqa: E731
    assert ids(store.capabilities_for_path("crates/pg/src/a_b.rs")) == ["sqlx/runtime-query-bind"]
    assert ids(store.capabilities_for_path("crates/pg/src/lib.rs")) == ["sqlx/runtime-query-bind"]
    assert ids(store.capabilities_for_path("crates/pg/src")) == [
        "pgpq/batch-copy",
        "sqlx/runtime-query-bind",
    ]
    assert store.capabilities_for_path("crates/pg/src/a%") == []  # % is not a wildcard here
    assert store.capabilities_for_path("crates/pg/src/a_") == []  # nor is _


def fake_run(tmp_path: Path, hits: list[sem.Hit]) -> SimpleNamespace:
    (tmp_path / "crates" / "pg" / "src").mkdir(parents=True)
    for name in ("a_b.rs", "c.rs", "copy.rs", "lib.rs"):
        (tmp_path / "crates" / "pg" / "src" / name).write_text("fn f() {}\n")
    meta = {
        "packages": [
            {"name": "pg", "manifest_path": str(tmp_path / "crates" / "pg" / "Cargo.toml")}
        ]
    }
    packages = [
        ("sqlx", "sqlx", "0.9.0"),
        ("sqlx", "sqlx-core", None),  # the family, any version
        ("sqlx", "sqlx-postgres", None),
        ("old", "ruff_python_ast", "0.0.11"),
        ("new", "ruff_python_ast", "0.0.14"),
    ]
    return SimpleNamespace(
        root=tmp_path, meta=meta, hits=hits, packages=packages, stamp="2026-09-29@abc1234"
    )


def hit(file: str, package: str, version: str, path: str, lines=(1,), kind="fn") -> sem.Hit:
    return sem.Hit(file, package, version, path, "path", tuple(lines), kind)


def test_the_usage_index_round_trips_and_attributes_packages_by_family_and_version(
    tmp_path,
) -> None:
    hits = [
        hit("crates/pg/src/a_b.rs", "sqlx-core", "0.9.0", "sqlx_core::query::query", (3, 9)),
        hit("crates/pg/src/c.rs", "sqlx-core", "0.9.0", "sqlx_core::query::query", (5,)),
        hit("crates/pg/src/c.rs", "sqlx-postgres", "0.9.0", "sqlx_postgres::PgPool", (6,), "adt"),
        hit("crates/pg/src/copy.rs", "ruff_python_ast", "0.0.11", "ruff_python_ast::Expr"),
        hit("crates/pg/src/copy.rs", "ruff_python_ast", "0.0.14", "ruff_python_ast::Stmt"),
    ]
    run = fake_run(tmp_path, hits)
    catalog = write_catalog(tmp_path)
    index = tmp_path / "build" / "usage.sqlite"
    db.write_index(index, run, catalog.read_text(), sem.Roles(tmp_path, run.meta))
    assert not list(index.parent.glob(".usage.sqlite.*"))  # no temporary file left behind
    store = db.Store(catalog, index)
    assert (
        store.has_usage and store.meta["usage_rows"] == "5" and store.meta["files_scanned"] == "4"
    )
    assert store.meta["run_stamp"] == "2026-09-29@abc1234"
    assert json.loads(store.meta["blind_spots"]) == list(db.BLIND_SPOTS)
    rows = store.usage_by_path("sqlx_core::query::query")
    assert [(r["file"], r["lines"], r["workspace_package"], r["role"]) for r in rows] == [
        ("crates/pg/src/a_b.rs", [3, 9], "pg", "src"),
        ("crates/pg/src/c.rs", [5], "pg", "src"),
    ]
    assert len(store.usage_by_leaf("QUERY")) == 2 and len(store.usage_by_leaf("query")) == 2
    assert {r["path"] for r in store.usage_by_library("sqlx")} == {
        "sqlx_core::query::query",
        "sqlx_postgres::PgPool",
    }
    assert [r["path"] for r in store.usage_by_library("old")] == ["ruff_python_ast::Expr"]
    assert [r["path"] for r in store.usage_by_library("new")] == ["ruff_python_ast::Stmt"]
    assert {r["path"] for r in store.usage_by_file("crates/pg/src")} == {r.path for r in hits}
    assert len(store.usage_by_file("crates/pg/src/c.rs")) == 2
    assert set(store.file_states()) == {
        "crates/pg/src/a_b.rs",
        "crates/pg/src/c.rs",
        "crates/pg/src/copy.rs",
        "crates/pg/src/lib.rs",
    }
    assert [p["package"] for p in store.usage_packages()][:2] == [
        "ruff_python_ast",
        "ruff_python_ast",
    ]


def test_without_an_index_usage_queries_are_empty_and_say_so_through_has_usage(tmp_path) -> None:
    store = db.Store(write_catalog(tmp_path), tmp_path / "missing.sqlite")
    assert not store.has_usage and store.usage_by_path("x") == [] and store.file_states() == {}
    assert store.usage_packages() == [] and store.meta == {}


def test_the_catalog_always_comes_from_the_current_jsonl_and_change_is_detected(tmp_path) -> None:
    run = fake_run(tmp_path, [hit("crates/pg/src/c.rs", "sqlx-core", "0.9.0", "sqlx_core::x::y")])
    catalog = write_catalog(tmp_path)
    index = tmp_path / "usage.sqlite"
    db.write_index(index, run, catalog.read_text(), sem.Roles(tmp_path, run.meta))
    store = db.Store(catalog, index)
    assert not store.stale
    edited = [*LIBS, capability("new/one", "sqlx", "Added later", "x", ["sqlx::new"])]
    catalog.write_text("".join(json.dumps(r) + "\n" for r in edited))
    os.utime(catalog, ns=(1, 2))
    assert store.stale
    fresh = db.Store(catalog, index)
    assert [c["id"] for c in fresh.capabilities()] == ["new/one"]  # the catalog is the edited file
    assert fresh.has_usage and len(fresh.usage_by_path("sqlx_core::x::y")) == 1  # usage still there
