from __future__ import annotations

import json
from pathlib import Path

import library_scan as scan
import library_semantic as sem


def hit(file: str, path: str, lines=(1,), package="pkg", version="1.0.0", via="path") -> sem.Hit:
    return sem.Hit(file, package, version, path, via, tuple(lines))


def workspace(tmp_path: Path, files: dict[str, str]) -> sem.Roles:
    for name, text in files.items():
        (tmp_path / name).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / name).write_text(text)
    meta = {
        "packages": [
            {"name": "a", "manifest_path": str(tmp_path / "crates" / "a" / "Cargo.toml")},
            {"name": scan.HACK, "manifest_path": str(tmp_path / "crates" / "hack" / "Cargo.toml")},
        ]
    }
    return sem.Roles(tmp_path, meta)


def test_parse_hits_reads_the_resolver_output() -> None:
    line = json.dumps(
        {
            "file": "f.rs",
            "package": "p",
            "version": "1",
            "path": "p::X",
            "via": "path",
            "lines": [3],
        }
    )
    assert sem.parse_hits(line + "\n\n") == [sem.Hit("f.rs", "p", "1", "p::X", "path", (3,))]


def test_roles_come_from_location_and_from_test_items(tmp_path: Path) -> None:
    src = "fn f() { x(); }\n#[cfg(test)]\nmod tests {\n    fn t() { x(); }\n}\n"
    roles = workspace(
        tmp_path,
        {"crates/a/src/lib.rs": src, "crates/a/tests/it.rs": "fn t() {}\n"},
    )
    assert roles.package("crates/a/src/lib.rs") == "a"
    assert roles.role("crates/a/src/lib.rs", (1,)) == "src"
    assert roles.role("crates/a/src/lib.rs", (1, 4)) == "src"  # one production line: src
    assert roles.role("crates/a/src/lib.rs", (4,)) == "test"
    assert roles.role("crates/a/tests/it.rs", (1,)) == "test"


def test_augment_refs_adds_only_files_the_lexical_scan_missed(tmp_path: Path) -> None:
    roles = workspace(tmp_path, {"crates/a/src/x.rs": "\n", "crates/a/src/y.rs": "\n"})
    lexical = {"lib": [scan.Ref("a", "crates/a/src/x.rs", "src", 1)]}
    keys = {("pkg", "1.0.0"): "lib", ("pkg", "0.9.0"): "old"}
    hits = [
        hit("crates/a/src/x.rs", "pkg::A"),
        hit("crates/a/src/y.rs", "pkg::B", lines=(1, 1)),
        hit("crates/a/src/y.rs", "other::C", package="other"),
    ]
    out = sem.augment_refs(lexical, hits, roles, keys)
    assert [r.path for r in out["lib"]] == ["crates/a/src/x.rs", "crates/a/src/y.rs"]
    assert "old" not in out and len(lexical["lib"]) == 1  # the input is not mutated


def test_evidence_normalize_uses_own_files_family_and_owner() -> None:
    hits = [
        hit("f1.rs", "sqlx_core::transaction::Transaction::commit", package="sqlx-core"),
        hit("f1.rs", "sqlx_core::query::query", package="sqlx-core"),
        hit("f2.rs", "other_core::query::query", package="other-core"),
        hit("f2.rs", "tracing::macros::info", package="tracing"),
        hit("f3.rs", "a::x::Dup", package="a"),
        hit("f3.rs", "a::y::Dup", package="a"),
    ]
    by_path, by_file, by_leaf = {}, {}, {}
    for h in hits:
        by_path.setdefault(h.path, []).append(h)
        by_file.setdefault(h.file, []).append(h)
        by_leaf.setdefault(h.path.rsplit("::", 1)[-1], []).append(h)
    items = [
        "sqlx::Transaction::commit",
        "sqlx::query",
        "tracing::info!",
        "a::Dup",
        "a::Missing",
        "sqlx_core::query::query",
    ]
    new, replaced, notes = sem.evidence_normalize(items, ["f1.rs"], by_path, by_file, by_leaf)
    assert new == [
        "sqlx_core::transaction::Transaction::commit",
        "sqlx_core::query::query",  # the wrong-family `other_core` hit is not a candidate
        "tracing::macros::info!",  # found repository-wide, and the macro `!` is kept
        "a::Dup",
        "a::Missing",
    ]
    assert replaced == ["sqlx::Transaction::commit", "sqlx::query", "tracing::info!"]
    assert notes == ["a::Dup (ambiguous)", "a::Missing (not resolved)"]


def test_merge_files_keeps_indirect_entries_and_orders_by_evidence(tmp_path: Path) -> None:
    roles = workspace(
        tmp_path,
        {
            "crates/a/src/impl.rs": "\n",
            "crates/a/src/gone.rs": "\n",
            "crates/a/src/wrapped.rs": "\n",
            "crates/a/src/new_big.rs": "\n",
            "crates/a/src/new_small.rs": "\n",
            "crates/a/tests/t.rs": "\n",
            "rules/r.yml": "\n",
        },
    )
    hits = {
        "p::X": [
            hit("crates/a/src/impl.rs", "p::X"),
            hit("crates/a/src/new_big.rs", "p::X", lines=(1, 2, 3)),
            hit("crates/a/src/new_small.rs", "p::X"),
            hit("crates/a/tests/t.rs", "p::X"),
        ]
    }
    existing = [
        {"path": "crates/a/src/impl.rs", "role": "impl"},
        {"path": "crates/a/src/gone.rs", "role": "impl"},
        {"path": "crates/a/src/wrapped.rs", "role": "consumer"},
        {"path": "rules/r.yml", "role": "impl"},
        {"path": "crates/a/tests/t.rs", "role": "impl"},
    ]
    files, dropped = sem.merge_files(existing, ["p::X"], hits, roles)
    assert [(f["path"].split("/")[-1], f["role"]) for f in files] == [
        ("impl.rs", "impl"),  # a recorded role survives on a production file
        ("wrapped.rs", "consumer"),  # indirect: kept without evidence
        ("r.yml", "impl"),  # not Rust: kept
        ("t.rs", "test"),  # evidence says test: the recorded `impl` yields
        ("new_big.rs", "consumer"),
        ("new_small.rs", "consumer"),
    ]
    assert dropped == ["crates/a/src/gone.rs (impl)"]


def test_apply_regenerates_stays_stable_and_keeps_records_with_no_evidence(tmp_path: Path) -> None:
    roles = workspace(tmp_path, {"crates/a/src/x.rs": "\n", "crates/a/src/macro.rs": "\n"})
    caps = [
        {
            "kind": "capability",
            "id": "p/one",
            "lib": "p",
            "items": ["p::X"],
            "files": [{"path": "crates/a/src/macro.rs", "role": "macro-expansion"}],
            "n_files": 9,
            "status": "used",
            "verified": "old",
        },
        {
            "kind": "capability",
            "id": "p/two",
            "lib": "p",
            "items": ["p::Never"],
            "files": [{"path": "crates/a/src/macro.rs", "role": "impl"}],
            "n_files": 1,
            "status": "used",
            "verified": "old",
        },
    ]
    hits = [hit("crates/a/src/x.rs", "p::X"), hit("crates/a/src/x.rs", "p::Other", lines=(1, 2))]
    out, drift, report = sem.apply(caps, hits, roles, "S")
    assert [f["path"].split("/")[-1] for f in out[0]["files"]] == ["macro.rs", "x.rs"]
    assert out[0]["n_files"] == 2 and out[0]["verified"] == "S"
    assert out[1]["files"] == caps[1]["files"] and out[1]["verified"] == "old"
    assert (
        len(drift) == 1
        and "p/two: no item resolved; files kept as recorded"
        in report["capability files dropped (no resolution)"]
    )
    assert report["resolved library items no capability records"] == []  # p::Other: one file
    again, drift2, _ = sem.apply(out, hits, roles, "T")
    assert drift2 == [] and again == out


def test_uncatalogued_reports_items_used_in_two_files_or_more_per_package() -> None:
    hits = [
        hit("a.rs", "p::Seen", package="p"),
        hit("a.rs", "p::New", package="p"),
        hit("b.rs", "p::New", package="p"),
        hit("a.rs", "q::Once", package="q"),
    ]
    assert sem._uncatalogued(hits, {"p::Seen"}) == ["p: p::New (2 files)"]
