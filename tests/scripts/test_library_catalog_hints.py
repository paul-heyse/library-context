from __future__ import annotations

import json
import os
from pathlib import Path
from types import SimpleNamespace

import library_catalog_db as db
import library_catalog_hints as hints
import library_names as names
import library_semantic as sem

LIBS = [
    {
        "kind": "library",
        "lib": "sqlx",
        "skill": "sqlx-postgres",
        "status": "used",
        "pin": "=0.9.0",
        "wrappers": [{"path": "crates/pg/src/lib.rs", "symbol": "Store"}],
        "resolved_features": ["macros", "postgres"],
        "linked_by": ["pg"],
        "skill_pin_delta": None,
    },
    {
        "kind": "library",
        "lib": "pgpq",
        "skill": "sqlx-postgres",
        "status": "test-only",
        "wrappers": [],
    },
    {
        "kind": "library",
        "lib": "oxidd",
        "skill": "rust-reasoning",
        "status": "not-used",
        "note": "Skill-covered; no direct dependency, so no local precedent.",
        "wrappers": [],
    },
    {
        "kind": "library",
        "lib": "ruff_db",
        "skill": "ty-flow",
        "status": "used",
        "skill_pin_delta": "pyrefly-ruff indexes 0.0.13; we resolve 0.0.14",
        "wrappers": [],
    },
]
CAPS = [
    {
        "kind": "capability",
        "id": "sqlx/runtime-query-bind",
        "lib": "sqlx",
        "feature": "Runtime queries with binds",
        "use": "Bind parameters and fetch rows",
        "items": ["sqlx_core::query::query", "sqlx_core::query_as::query_as", "sqlx::query_file!"],
        "as_written": ["sqlx::query_as"],
        "files": [{"path": "crates/pg/src/a.rs", "role": "impl"}],
        "wrapper": {"path": "crates/pg/src/lib.rs", "symbol": "Store"},
        "status": "used",
    },
    {
        "kind": "capability",
        "id": "pgpq/batch-copy",
        "lib": "pgpq",
        "feature": "Binary COPY staging",
        "use": "Encode Arrow batches",
        "items": ["pgpq::ArrowToPostgresBinaryEncoder::write_batch"],
        "files": [{"path": "crates/pg/src/copy.rs", "role": "impl"}],
        "wrapper": None,
        "status": "used",
    },
]


def hit(file: str, package: str, version: str, path: str, lines=(1,), kind="fn") -> sem.Hit:
    return sem.Hit(file, package, version, path, "path", tuple(lines), kind)


HITS = [
    hit("crates/pg/src/a.rs", "sqlx-core", "0.9.0", "sqlx_core::query::query", (3, 9)),
    hit("crates/pg/src/a.rs", "sqlx-postgres", "0.9.0", "sqlx_postgres::PgPool", (4,), "adt"),
    hit("crates/pg/src/b.rs", "sqlx-core", "0.9.0", "sqlx_core::query::query", (7,)),
    hit(
        "crates/pg/src/copy.rs", "pgpq", "0.12.0", "pgpq::ArrowToPostgresBinaryEncoder", (2,), "adt"
    ),
]


def build(tmp_path: Path, hits: list[sem.Hit] | None = HITS, graph: dict | None = None):
    src = tmp_path / "crates" / "pg" / "src"
    src.mkdir(parents=True)
    for name in ("a.rs", "b.rs", "copy.rs", "lib.rs", "dormant_like.rs"):
        (src / name).write_text("fn f() {}\n")
    (tmp_path / "Cargo.toml").write_text(
        "[workspace]\n[workspace.dependencies]\n"
        'sqlx = "=0.9.0"\nserde = "1"\nsea-query = "=1.0.2"\n'
    )
    catalog = tmp_path / "catalog.jsonl"
    catalog.write_text("".join(json.dumps(r) + "\n" for r in LIBS + CAPS))
    index = tmp_path / "build" / "usage.sqlite"
    if hits is not None:
        meta = {
            "packages": [{"name": "pg", "manifest_path": str(tmp_path / "crates/pg/Cargo.toml")}]
        }
        (tmp_path / "crates" / "pg" / "src" / "dormant_like.rs").unlink()  # not in the index later
        run = SimpleNamespace(
            root=tmp_path,
            meta=meta,
            hits=hits,
            packages=[
                ("sqlx", "sqlx", "0.9.0"),
                ("sqlx", "sqlx-core", None),
                ("sqlx", "sqlx-postgres", None),
                ("pgpq", "pgpq", "0.12.0"),
            ],
            stamp="2026-09-29@abc1234",
        )
        db.write_index(index, run, catalog.read_text(), sem.Roles(tmp_path, meta))
        (src / "dormant_like.rs").write_text("fn added_later() {}\n")  # new since the index
    store = db.Store(catalog, index)
    loader = lambda _root: (graph, "" if graph else "cargo not found")  # noqa: E731
    ctx = hints.Context(
        root=tmp_path,
        store=store,
        graph=hints.GraphCache(tmp_path, loader),
        index=names.Index(
            symbols={"sqlx_core::query_as::query_as": "function"},
            aliases={"sqlx::query_as": ["sqlx_core::query_as::query_as"]},
            methods=set(),
            by_name={("sqlx_core", "query_as"): ["sqlx_core::query_as::query_as"]},
            crates={"sqlx_core", "sqlx"},
        ),
        _index_loaded=True,
    )
    return ctx


def unit(pkg_id: str, features: list[str], deps=()) -> dict:
    return {"pkg_id": pkg_id, "features": features, "dependencies": [{"index": i} for i in deps]}


def graph_for(tmp_path: Path, features: list[str]) -> dict:
    return {
        "units": [
            unit("registry+r#sqlx@0.9.0", features),
            unit(f"path+file://{tmp_path.resolve()}/crates/pg#0.1.0", [], [0]),
        ]
    }


def test_a_found_library_lists_everything_and_says_nothing_about_freshness_when_nothing_differs(
    tmp_path,
) -> None:
    ctx = build(tmp_path, graph=graph_for(tmp_path, ["macros", "postgres"]))
    answer = hints.get_library(ctx, "sqlx")
    assert answer["state"] == "found" and [c["id"] for c in answer["capabilities"]] == [
        "sqlx/runtime-query-bind"
    ]
    assert answer["usage"]["rows"] == 3 and answer["usage"]["items"] == 2
    assert "validation" not in answer  # nothing differs: silence
    text = json.dumps(answer).lower()
    assert not any(w in text for w in ("stale", "out of date", "outdated", "refresh"))
    assert any("reuse the local abstraction: Store" in h for h in answer["hints"])
    assert answer["skill"]["skill"] == "sqlx-postgres" and answer["skill"]["linked"] is False
    assert hints.get_library(ctx, "SQLX")["library"]["lib"] == "sqlx"  # case and hyphen tolerant


def test_state_and_status_hints_for_unused_test_only_and_pin_delta_libraries(tmp_path) -> None:
    ctx = build(tmp_path)
    unused = hints.get_library(ctx, "oxidd")
    assert unused["state"] == "not_used" and "no local precedent" in unused["hints"][0]
    test_only = hints.get_library(ctx, "pgpq")
    assert any("referenced only from test code" in h for h in test_only["hints"])
    delta = hints.get_library(ctx, "ruff_db")
    assert any("pyrefly-ruff indexes 0.0.13" in h and "docs/pins.md" in h for h in delta["hints"])
    assert any("no curated capability" in h for h in delta["hints"])


def test_an_unknown_name_is_distinguished_from_a_dependency_without_a_record(tmp_path) -> None:
    ctx = build(tmp_path)
    gap = hints.get_library(ctx, "sea-query")
    assert (
        gap["state"] == "dependency_without_record"
        and "not evidence the library is unused" in gap["hints"][0]
    )
    unknown = hints.get_library(ctx, "sqlxx")
    assert unknown["state"] == "unknown" and "sqlx" in unknown["candidates"]
    assert "says nothing about whether the library could serve a need" in unknown["hints"][0]


def test_validation_names_only_the_cited_files_that_changed(tmp_path) -> None:
    ctx = build(tmp_path)
    before = hints.find_item(ctx, "sqlx_core::query::query")
    assert "validation" not in before
    os.utime(tmp_path / "crates/pg/src/b.rs", ns=(1, 2))
    (tmp_path / "crates/pg/src/a.rs").unlink()
    after = hints.find_item(ctx, "sqlx_core::query::query")
    changes = {c["file"]: c["change"] for c in after["validation"]["files"]}
    assert changes == {
        "crates/pg/src/a.rs": "deleted since the usage index was built",
        "crates/pg/src/b.rs": "modified since the usage index was built",
    }


def test_dependency_facts_are_compared_with_the_live_graph_and_named_when_they_differ(
    tmp_path,
) -> None:
    same = build(tmp_path / "same", graph=graph_for(tmp_path / "same", ["macros", "postgres"]))
    assert "validation" not in hints.get_library(same, "sqlx")
    differ = build(
        tmp_path / "diff", graph=graph_for(tmp_path / "diff", ["macros", "postgres", "json"])
    )
    facts = hints.get_library(differ, "sqlx")["validation"]["dependency_facts"]
    assert (
        facts["resolved_features"]["added"] == ["json"]
        and facts["resolved_features"]["removed"] == []
    )
    none = build(tmp_path / "none", graph=None)
    assert hints.get_library(none, "sqlx")["validation"]["dependency_facts"] == {
        "unavailable": "cargo not found"
    }


def test_find_item_normalises_spellings_and_falls_back_to_the_leaf_without_capping(
    tmp_path,
) -> None:
    ctx = build(tmp_path)
    facade = hints.find_item(ctx, "sqlx::query_as")
    assert facade["normalized"]["canonical"] == "sqlx_core::query_as::query_as"
    assert [c["id"] for c in facade["capabilities"]] == ["sqlx/runtime-query-bind"]
    macro = hints.find_item(ctx, "sqlx::query_file!")
    assert [c["id"] for c in macro["capabilities"]] == ["sqlx/runtime-query-bind"]
    exact = hints.find_item(ctx, "sqlx_core::query::query")
    assert exact["match"] == "path" and exact["totals"]["files"] == 2
    assert [f["lines"] for f in exact["usage"][0]["files"]] == [[3, 9], [7]]
    leaf = hints.find_item(ctx, "query")
    assert leaf["state"] == "found" and leaf["match"] == "leaf"
    assert [u["path"] for u in leaf["usage"]] == ["sqlx_core::query::query"]


def test_an_absence_is_answered_from_the_index_with_blind_spots_never_from_the_catalog(
    tmp_path,
) -> None:
    ctx = build(tmp_path)
    answer = hints.find_item(ctx, "nothing_here::Widget")
    assert answer["state"] == "no_reference_found"
    assert answer["searched"]["usage_rows"] == 4 and answer["searched"]["files"] == 4
    assert answer["blind_spots"] == list(db.BLIND_SPOTS)
    assert "validation" not in answer  # no changed file mentions it
    added = tmp_path / "crates/pg/src/dormant_like.rs"
    added.write_text("fn use_it() { let _ = Widget; }\n")
    again = hints.find_item(ctx, "nothing_here::Widget")
    assert again["validation"]["changed_files_mentioning"] == [
        {"file": "crates/pg/src/dormant_like.rs", "change": "new since the usage index was built"}
    ]


def test_without_a_usage_index_the_answer_says_so_and_never_claims_non_use(tmp_path) -> None:
    ctx = build(tmp_path, hits=None)
    answer = hints.find_item(ctx, "nothing_here::Widget")
    assert (
        answer["state"] == "no_usage_index"
        and "no usage index has been built" in answer["hints"][0]
    )
    assert "does not show anything is unused" in answer["hints"][0]
    assert hints.library_usage(ctx, "sqlx")["state"] == "no_usage_index"
    assert hints.catalog_status(ctx)["usage_index"] is None
    listed = hints.find_item(ctx, "sqlx::query_file!")
    assert listed["state"] == "found" and [c["id"] for c in listed["capabilities"]]


def test_library_usage_groups_by_item_or_file_and_includes_the_whole_family(tmp_path) -> None:
    ctx = build(tmp_path)
    by_item = hints.library_usage(ctx, "sqlx")
    assert by_item["totals"] == {
        "rows": 3,
        "items": 2,
        "files": 2,
        "packages": ["sqlx-core", "sqlx-postgres"],
    }
    assert [u["path"] for u in by_item["usage"]] == [
        "sqlx_core::query::query",
        "sqlx_postgres::PgPool",
    ]
    by_file = hints.library_usage(ctx, "sqlx", group_by="file")
    assert [f["file"] for f in by_file["usage"]] == ["crates/pg/src/a.rs", "crates/pg/src/b.rs"]
    unused = hints.library_usage(ctx, "oxidd")
    assert unused["state"] == "no_reference_found" and unused["blind_spots"]


def test_find_by_file_distinguishes_indexed_new_missing_and_unindexed_files(tmp_path) -> None:
    ctx = build(tmp_path)
    found = hints.find_by_file(ctx, "crates/pg/src/a.rs")
    assert found["state"] == "found" and [c["id"] for c in found["capabilities"]] == [
        "sqlx/runtime-query-bind"
    ]
    assert [i["path"] for i in found["files"][0]["items"]] == [
        "sqlx_core::query::query",
        "sqlx_postgres::PgPool",
    ]
    absolute = hints.find_by_file(ctx, str(tmp_path / "crates/pg/src/a.rs"))
    assert absolute["requested"] == "crates/pg/src/a.rs"
    assert hints.find_by_file(ctx, "crates/pg/src")["state"] == "found"
    lib = hints.find_by_file(ctx, "crates/pg/src/lib.rs")
    assert lib["state"] == "found" and lib["capabilities"]  # the wrapper file matches
    assert hints.find_by_file(ctx, "crates/pg/src/dormant_like.rs")["state"] == "not_in_usage_index"
    assert hints.find_by_file(ctx, "crates/pg/src/none.rs")["state"] == "no_such_file"


def test_search_returns_every_match_and_only_caps_when_the_caller_asks(tmp_path) -> None:
    ctx = build(tmp_path)
    everything = hints.search_capabilities(ctx, "")
    assert everything["total"] == 2 and "omitted" not in everything
    limited = hints.search_capabilities(ctx, "", limit=1)
    assert limited["total"] == 2 and len(limited["results"]) == 1 and limited["omitted"] == 1
    assert hints.search_capabilities(ctx, "copy", skill="sqlx-postgres")["total"] == 1
    assert hints.search_capabilities(ctx, "copy", lib="sqlx")["total"] == 0
    assert hints.search_capabilities(ctx, "", role="impl")["total"] == 2
    miss = hints.search_capabilities(ctx, "graphs")
    assert (
        miss["state"] == "no_curated_match"
        and "does not show the workspace lacks" in miss["hints"][0]
    )


def test_capabilities_by_library_groups_every_capability_under_its_library(tmp_path) -> None:
    ctx = build(tmp_path)
    answer = hints.capabilities_by_library(ctx)
    assert set(answer) == {"libraries"}
    everything = ctx.store.capabilities()
    grouped = answer["libraries"]
    assert list(grouped) == sorted({c["lib"] for c in everything})
    total = 0
    for body in grouped.values():
        assert set(body) == {"capabilities"}
        for cap in body["capabilities"]:
            assert "kind" not in cap and "lib" not in cap
            total += 1
    assert total == len(everything)
    stored = {c["id"]: c for c in everything}
    for body in grouped.values():
        for cap in body["capabilities"]:
            source = stored[cap["id"]]
            assert cap == {k: v for k, v in source.items() if k not in ("kind", "lib")}


def test_gaps_and_status(tmp_path) -> None:
    ctx = build(tmp_path)
    assert [r["lib"] for r in hints.catalog_gaps(ctx, "not_used")["rows"]] == ["oxidd"]
    assert [r["lib"] for r in hints.catalog_gaps(ctx, "pin_delta")["rows"]] == ["ruff_db"]
    assert [r["id"] for r in hints.catalog_gaps(ctx, "no_wrapper")["rows"]] == ["pgpq/batch-copy"]
    assert [
        r["dependency"] for r in hints.catalog_gaps(ctx, "dependency_without_record")["rows"]
    ] == [
        "sea-query",
        "serde",
    ]
    assert hints.catalog_gaps(ctx, "bogus")["state"] == "unknown_kind"
    status = hints.catalog_status(ctx)
    listing = status["libraries"]
    assert {lib: body["status"] for lib, body in listing.items()} == {
        r["lib"]: r["status"] for r in ctx.store.libraries()
    }
    everything = ctx.store.capabilities()
    assert {lib: [c["id"] for c in body["capabilities"]] for lib, body in listing.items()} == {
        lib: [c["id"] for c in everything if c["lib"] == lib] for lib in listing
    }
    assert all(set(c) == {"id", "feature"} for b in listing.values() for c in b["capabilities"])
    assert status["skills"] and status["verified_stamps"] is not None
    index = status["usage_index"]
    assert index["blind_spots"] and index["packages"] and "usage_rows" not in index
    assert "files_scanned" not in index


def test_the_skill_reader_is_detected_from_the_skill_and_a_missing_link_is_a_fact(tmp_path) -> None:
    ctx = build(tmp_path)
    reader = tmp_path / ".claude/skills/sqlx-postgres/scripts"
    reader.mkdir(parents=True)
    (reader / "reference.py").write_text('sub.add_parser("find")\nsub.add_parser("show")\n')
    info = hints.skill_info(ctx, "sqlx-postgres")
    assert info is not None and info["linked"] and info["subcommands"] == ["find", "show"]
    bare = tmp_path / ".claude/skills/no-scripts"
    bare.mkdir()
    fallback = hints.skill_info(ctx, "no-scripts")
    assert (
        fallback is not None
        and fallback["reader"] is None
        and "content/index" in fallback["fallback"]
    )
    absent = hints.skill_info(ctx, "absent")
    assert absent is not None
    assert absent["fact"] == ".claude/skills/absent is not linked in this checkout"
    assert hints.skill_info(ctx, None) is None


def test_the_graph_is_reread_only_when_a_manifest_changes(tmp_path) -> None:
    calls = []

    def loader(root):
        calls.append(1)
        return graph_for(tmp_path, ["macros"]), ""

    (tmp_path / "Cargo.toml").write_text("[workspace]\n")
    cache = hints.GraphCache(tmp_path, loader)
    cache.summary()
    cache.summary()
    assert len(calls) == 1
    os.utime(tmp_path / "Cargo.toml", ns=(5, 6))
    cache.summary()
    assert len(calls) == 2


def test_a_capability_that_records_more_files_than_it_lists_says_so_in_every_lookup(
    tmp_path,
) -> None:
    ctx = build(tmp_path)
    ctx.store.conn.execute(
        "UPDATE capabilities SET record = json_set(record, '$.n_files', 26) "
        "WHERE id = 'sqlx/runtime-query-bind'"
    )
    fact = "sqlx/runtime-query-bind records 26 files but lists 1"
    for answer in (
        hints.get_library(ctx, "sqlx"),
        hints.find_item(ctx, "sqlx::query_file!"),
        hints.find_by_file(ctx, "crates/pg/src/a.rs"),
        hints.search_capabilities(ctx, "bind"),
    ):
        assert any(fact in h for h in answer["hints"]), answer["state"]
    assert not any("records" in h for h in hints.get_library(ctx, "pgpq")["hints"])
