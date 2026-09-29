from __future__ import annotations

from pathlib import Path

import library_scan as scan

IDENTS = {"petgraph": "petgraph", "arrow_array": "arrow-array", "ast_ty": "ruff_python_ast_ty"}


def test_mask_blanks_comments_and_literals_but_keeps_length_and_lifetimes() -> None:
    src = (
        'let a = "petgraph::x"; // petgraph::y\n'
        '/* outer /* petgraph::z */ still */ let b = r#"petgraph::"q"#;\n'
        "fn f<'a>(c: char) -> &'a str { let d = '}'; petgraph::real }\n"
    )
    masked = scan.mask(src)
    assert len(masked) == len(src) and masked.count("\n") == src.count("\n")
    assert masked.count("petgraph") == 1  # only the real path
    assert "'a" in masked and "'}'" not in masked


def refs(src: str) -> dict[str, list[bool]]:
    return dict(scan.scan_text(src, IDENTS))


def test_paths_use_statements_and_macro_reexports_count_but_local_paths_do_not() -> None:
    src = """
use petgraph::Graph;
pub use arrow_array;
use ruff_x::y;
fn f() { let g = ::petgraph::algo::tarjan_scc(&x); crate::petgraph::not_this(); }
macro_rules! m { () => { $crate::__private::arrow_array::RecordBatch } }
use ast_ty as ast;
"""
    found = refs(src)
    assert len(found["petgraph"]) == 2  # `use petgraph::` and `::petgraph::algo`
    assert len(found["arrow-array"]) == 2  # `pub use arrow_array;` and the `__private::` path
    assert len(found["ruff_python_ast_ty"]) == 1
    assert "ruff_x" not in found


def test_cfg_test_and_test_items_are_flagged_and_others_are_not() -> None:
    src = """
use petgraph::Graph;
#[cfg(test)]
mod tests {
    use petgraph::visit::Bfs;
    fn helper() { let _ = { 1 }; }
}
#[cfg(not(test))]
fn real() { petgraph::x(); }
#[tokio::test]
async fn t() { petgraph::y(); }
#[test]
#[ignore]
fn u() { petgraph::z(); }
fn tail() { petgraph::w(); }
"""
    assert refs(src)["petgraph"] == [False, True, False, True, True, False]
    assert scan.test_regions("#![cfg(test)]\nfn a() {}") == [(0, len("#![cfg(test)]\nfn a() {}"))]


def test_a_cfg_test_module_declaration_without_a_body_ends_at_the_semicolon() -> None:
    src = "#[cfg(test)]\nmod tests;\nfn f() { petgraph::x(); }\n"
    assert refs(src)["petgraph"] == [False]


def test_roles_come_from_location_then_from_where_the_references_sit() -> None:
    assert scan.path_role(Path("tests/dormant/a.rs")) == "dormant"
    assert scan.path_role(Path("tests/a.rs")) == "test"
    assert scan.path_role(Path("examples/p.rs")) == "test"
    assert scan.path_role(Path("src/tests.rs")) == "test"
    assert scan.path_role(Path("build.rs")) == "build"
    assert scan.path_role(Path("src/a.rs")) == "src"
    assert scan.file_role("src", [True, True]) == "test"
    assert scan.file_role("src", [True, False]) == "src"
    assert scan.file_role("dormant", [True]) == "dormant"


def ref(role: str, package: str = "a", path: str = "p.rs") -> scan.Ref:
    return scan.Ref(package, path, role, 1)


def test_status_is_the_strongest_role_and_used_in_counts_files_by_package() -> None:
    assert scan.status_of([]) == "declared-unused"
    assert scan.status_of([ref("dormant")]) == "dormant-only"
    assert scan.status_of([ref("dormant"), ref("test")]) == "test-only"
    assert scan.status_of([ref("test"), ref("build")]) == "used"
    counted = scan.used_in([ref("src", "a", "1"), ref("build", "a", "2"), ref("test", "b", "3")])
    assert counted == {"a": {"src": 2}, "b": {"test": 1}}


def test_scan_workspace_scopes_identifiers_to_each_packages_own_dependencies(
    tmp_path: Path,
) -> None:
    def package(name: str, deps: list[dict]) -> dict:
        (tmp_path / "crates" / name / "src").mkdir(parents=True)
        return {
            "name": name,
            "manifest_path": str(tmp_path / "crates" / name / "Cargo.toml"),
            "dependencies": deps,
        }

    def dep(name: str) -> dict:
        return {"name": name, "rename": None, "path": None}

    meta = {"packages": [package("a", [dep("petgraph")]), package("b", [])]}
    for name in ("a", "b"):
        (tmp_path / "crates" / name / "src" / "lib.rs").write_text("fn f() { petgraph::x(); }\n")
    (tmp_path / "crates" / "a" / "src" / "tests.rs").write_text("use petgraph::y;\n")
    found = scan.scan_workspace(tmp_path, meta)
    assert sorted((r.package, r.path, r.role) for r in found["petgraph"]) == [
        ("a", "crates/a/src/lib.rs", "src"),
        ("a", "crates/a/src/tests.rs", "test"),
    ]
