from __future__ import annotations

import json
from pathlib import Path

import library_names as names
import library_scan as scan
import library_utilization as lu


def dep(name: str, req: str = "=1.0.0", **kw) -> dict:
    return {
        "name": name,
        "req": req,
        "source": kw.get("source", "registry+https://github.com/rust-lang/crates.io-index"),
        "kind": kw.get("kind"),
        "rename": kw.get("rename"),
        "uses_default_features": kw.get("default", True),
        "features": kw.get("features", []),
        "path": kw.get("path"),
    }


def meta(**crates: list[dict]) -> dict:
    return {"packages": [{"name": n, "dependencies": d} for n, d in crates.items()]}


def group(skill: str, name: str, crates: dict, label: str | None = None) -> lu.Group:
    single = next(iter(crates)) if len(crates) == 1 else None
    return lu.Group(skill, name, label or single or name, crates)


def test_direct_deps_skip_members_path_deps_and_the_hakari_stub() -> None:
    m = meta(
        a=[
            dep("b", path="/x"),
            dep("petgraph"),
            dep("ruff_python_ast", rename="ast_ty", kind="dev"),
        ],
        b=[],
        **{lu.HACK: [dep("petgraph", "^0.8")]},
    )
    deps = lu.direct_deps(m)
    assert set(deps) == {"petgraph", "ast_ty"}
    assert deps["ast_ty"].package == "ruff_python_ast"
    assert deps["ast_ty"].dependents == {"a (dev)"}


def test_pin_is_the_requirement_or_a_short_git_ref() -> None:
    git = lu.Dep(
        "py", "pyrefly", reqs={"*"}, sources={"git+https://github.com/o/pyrefly?rev=abcdef0123"}
    )
    assert lu.pin_of(git, ["1.3.1"]) == "git o/pyrefly rev abcdef0"
    assert lu.pin_of(lu.Dep("x", "x", reqs={"=2.0.1"}, sources={""}), ["2.0.1"]) == "=2.0.1"


def test_skill_choice_prefers_the_one_indexing_our_version_and_reports_the_others() -> None:
    groups = [
        group("old", "lib", {"ruff_db": "0.0.13"}),
        group("flow", "line", {"ruff_db": "0.0.14"}),
    ]
    cover = lu.covering("ruff_db", groups)
    assert lu.choose_skill(None, cover, ["0.0.14"]) == "flow"
    assert lu.pin_delta(None, cover, ["0.0.14"]) == "old indexes 0.0.13; we resolve 0.0.14"
    assert lu.pin_delta("hand text", cover, ["0.0.14"]) == "hand text"
    assert lu.pin_delta(None, cover, ["0.0.13"]) == "flow indexes 0.0.14; we resolve 0.0.13"


def test_build_generates_status_used_in_not_used_and_keeps_hand_fields() -> None:
    m = meta(a=[dep("petgraph", "=0.8.3", features=["x"], default=False), dep("serde")])
    lock = {"petgraph": ["0.8.3"], "z3": ["0.21.1"]}
    groups = [
        group("graphs", "petgraph", {"petgraph": "0.8.3"}),
        group("logic", "z3", {"z3": None, "z3-sys": None}, label="z3"),
    ]
    existing = [
        {
            "kind": "library",
            "lib": "petgraph",
            "status": "declared-unused",
            "wrappers": [{"path": "p"}],
            "note": "keep",
        },
        {"kind": "capability", "id": "gone/x", "lib": "gone"},
    ]
    refs = {"petgraph": [scan.Ref("a", "crates/a/src/x.rs", "src", 2)]}
    records, report = lu.build(existing, m, lock, groups, refs)
    by = {r["lib"]: r for r in records}
    assert by["petgraph"]["status"] == "used" and by["petgraph"]["note"] == "keep"
    assert by["petgraph"]["used_in"] == {"a": {"src": 1}}
    assert by["petgraph"]["wrappers"] == [{"path": "p"}]
    assert by["petgraph"]["default_features"] is False and by["petgraph"]["features"] == ["x"]
    assert by["z3"]["status"] == "not-used" and by["z3"]["crates"] == ["z3", "z3-sys"]
    assert by["z3"]["pin"] == "0.21.1 (transitive)"
    assert "serde" not in by and report["unlisted"] == ["serde (a)"]
    assert report["orphan capabilities"] == ["gone/x"]


def test_a_library_that_becomes_a_dependency_takes_its_status_from_the_scan() -> None:
    m = meta(a=[dep("oxidd")])
    existing = [{"kind": "library", "lib": "oxidd", "status": "not-used", "wrappers": []}]
    args = (existing, m, {"oxidd": ["1.0.0"]}, [group("s", "oxidd", {"oxidd": "1.0.0"})])
    records, _ = lu.build(*args, {})
    assert records[0]["status"] == "declared-unused" and records[0]["used_in"] == {}
    records, _ = lu.build(*args, {"oxidd": [scan.Ref("a", "crates/a/tests/t.rs", "test", 1)]})
    assert records[0]["status"] == "test-only"


def test_only_changed_records_are_restamped_and_render_is_stable(tmp_path: Path) -> None:
    old = {"petgraph": {"kind": "library", "lib": "petgraph", "status": "used", "verified": "old"}}
    same = dict(old["petgraph"])
    changed = dict(old["petgraph"], pin="=1")
    new = dict(old["petgraph"], lib="new", status="unknown")
    assert lu.stamp_changed([same], old, "S") == []
    assert same["verified"] == "old"
    drift = lu.stamp_changed([changed, new], old, "S")
    assert [line[0] for line in drift] == ["~", "+"] and changed["verified"] == new[
        "verified"
    ] == "S"
    text = lu.render([new, changed, {"kind": "capability", "id": "petgraph/a", "lib": "petgraph"}])
    assert [json.loads(line).get("id", json.loads(line)["lib"]) for line in text.splitlines()] == [
        "new",
        "petgraph",
        "petgraph/a",
    ]


def test_normalize_capabilities_rewrites_items_once_and_keeps_the_spellings() -> None:
    index = names.Index(
        symbols={"lib::a::Type": "struct"},
        aliases={"lib::Type": "lib::a::Type"},
        methods={("lib::a::Type", "go")},
        by_name={("lib", "Type"): ["lib::a::Type"]},
        crates={"lib"},
    )
    cap = {
        "kind": "capability",
        "id": "lib/x",
        "lib": "lib",
        "items": ["lib::Type", "lib::Type::go", "lib::a::Type", "lib::Gone", "other::Thing"],
        "verified": "old",
    }
    libs = [{"lib": "lib", "skill_pin_delta": "skill indexes 2"}]
    out, drift, report = lu.normalize_capabilities([cap], index, libs, "S")
    assert out[0]["items"] == ["lib::a::Type", "lib::a::Type::go", "lib::Gone", "other::Thing"]
    assert out[0]["as_written"] == ["lib::Type", "lib::Type::go"]
    assert out[0]["verified"] == "S" and len(drift) == 1
    assert report["items not in the skill index"] == ["lib/x: lib::Gone (absent) [skill indexes 2]"]
    assert report["items in crates no skill indexes"] == ["other: 1"]
    again, drift2, _ = lu.normalize_capabilities(out, index, libs, "T")
    assert drift2 == [] and again[0]["verified"] == "S"
