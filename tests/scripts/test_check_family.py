from __future__ import annotations

from copy import deepcopy
from pathlib import Path

import pytest

import check_family

LOCK = """version = 4

[[package]]
name = "arrow-array"
version = "59.3.0"

[[package]]
name = "datafusion"
version = "55.1.0"

[[package]]
name = "syn"
version = "1.0.0"

[[package]]
name = "syn"
version = "2.0.0"
"""


@pytest.fixture(autouse=True)
def _no_repository_manifest(tmp_path_factory: pytest.TempPathFactory, monkeypatch) -> None:
    """`main` reads ROOT's manifests; family tests use an empty one."""
    root = tmp_path_factory.mktemp("root")
    (root / "Cargo.toml").write_text("[workspace.dependencies]\n")
    monkeypatch.setattr(check_family, "ROOT", root)


def test_single_versions_pass_and_unrelated_duplicates_are_ignored(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK)
    assert check_family.duplicates(lock) == {}
    assert check_family.main([str(lock)]) == 0


def test_second_arrow_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + '\n[[package]]\nname = "arrow-array"\nversion = "58.0.0"\n')
    assert check_family.duplicates(lock) == {"arrow-array": ["58.0.0", "59.3.0"]}
    assert check_family.main([str(lock)]) == 1


def test_missing_lockfile_is_not_run_not_failure(tmp_path: Path) -> None:
    assert check_family.main([str(tmp_path / "absent.lock")]) == 0


MANIFEST = """[workspace.dependencies]
{dep}
"""


@pytest.mark.parametrize(
    ("dep", "problems"),
    [
        # A caret or range on a registry dependency fails.
        ('tokio = "1.53.1"', ["tokio: '1.53.1' is not an exact pin (=x.y.z)"]),
        (
            'tokio = { version = ">=1.53", features = ["macros"] }',
            ["tokio: '>=1.53' is not an exact pin (=x.y.z)"],
        ),
        # An exact pin or a git rev passes without a docs/pins.md row.
        ('blake3 = "=1.8.6"', []),
        ('tokio = { version = "=1.53.2", features = ["macros"] }', []),
        ('pyrefly = { git = "https://example.invalid/pyrefly", rev = "abc" }', []),
        (
            'pyrefly = { git = "https://example.invalid/pyrefly", branch = "main" }',
            ["pyrefly: git dependency without a rev"],
        ),
    ],
)
def test_declared_dependencies_must_be_exact(tmp_path: Path, dep: str, problems: list) -> None:
    manifest = tmp_path / "Cargo.toml"
    manifest.write_text(MANIFEST.format(dep=dep))
    assert check_family.unpinned(manifest) == problems


def test_member_tables_are_checked_and_path_or_workspace_entries_exempt(tmp_path: Path) -> None:
    manifest = tmp_path / "Cargo.toml"
    manifest.write_text(
        '[dependencies]\ntonic = "0.14"\nanyhow = { workspace = true }\n'
        'lctx-model = { path = "../lctx-model" }\n'
        '[dev-dependencies]\ninsta = "=1.49.0"\n'
    )
    assert check_family.unpinned(manifest) == ["tonic: '0.14' is not an exact pin (=x.y.z)"]


def test_main_fails_on_a_caret_in_the_root_manifest(tmp_path: Path) -> None:
    (check_family.ROOT / "Cargo.toml").write_text('[workspace.dependencies]\nhex = "0.4.3"\n')
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK)
    assert check_family.main([str(lock)]) == 1


def test_second_ruff_line_fails(tmp_path: Path) -> None:
    """Undeclared Ruff lines cannot substitute for either admitted nominal family."""
    lock = tmp_path / "Cargo.lock"
    ruff = '\n[[package]]\nname = "ruff_python_ast"\nversion = "{}"\n'
    lock.write_text(LOCK + ruff.format("0.0.11") + ruff.format("0.0.13"))
    assert check_family.duplicates(lock) == {"ruff_python_ast": ["0.0.11", "0.0.13"]}
    assert check_family.main([str(lock)]) == 1


EXTRA = f"""
[[package]]
name = "ruff_python_ast"
version = "0.0.14"
source = "{check_family.REGISTRY}"

[[package]]
name = "ruff_python_ast"
version = "0.0.16"
source = "{check_family.RUFF_SOURCE}"

[[package]]
name = "ty_python_core"
version = "0.0.16"
source = "{check_family.RUFF_SOURCE}"
dependencies = ["ruff_python_ast 0.0.16", "salsa"]

[[package]]
name = "salsa"
version = "0.28.5"

[[package]]
name = "cpg-flow"
version = "0.1.0"
dependencies = ["ty_python_core", "ruff_python_ast 0.0.16"]

[[package]]
name = "pyrefly"
version = "1.4.0-dev.3"
dependencies = ["ruff_python_ast 0.0.14"]
"""


def test_declared_source_families_are_allowed_in_their_scopes(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA)
    assert check_family.duplicates(lock) == {}
    assert check_family.extra_scope(lock) == []


def test_latest_family_outside_its_scope_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    stray = '\n[[package]]\nname = "cpg-core"\nversion = "0.1.0"\n'
    stray += 'dependencies = ["ruff_python_ast 0.0.16"]\n'
    lock.write_text(LOCK + EXTRA + stray)
    assert any("cpg-core" in p and "outside latest" in p for p in check_family.extra_scope(lock))


def test_salsa_patch_release_fails(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA.replace('version = "0.28.5"', 'version = "0.28.4"'))
    assert any("salsa resolves to 0.28.4" in p for p in check_family.extra_scope(lock))


def test_equal_version_from_another_source_cannot_split_the_latest_family(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    wrong = (
        '\n[[package]]\nname = "ruff_python_ast"\nversion = "0.0.16"\n'
        f'source = "{check_family.REGISTRY}"\n'
    )
    lock.write_text(LOCK + EXTRA + wrong)
    problems = check_family.extra_scope(lock)
    assert any("undeclared analyzer source" in p for p in problems)
    assert any("ambiguous dependency ruff_python_ast 0.0.16" in p for p in problems)
    qualified = EXTRA.replace(
        '"ruff_python_ast 0.0.16"', f'"ruff_python_ast 0.0.16 ({check_family.RUFF_SOURCE})"'
    )
    lock.write_text(LOCK + qualified + wrong)
    assert not any("ambiguous" in p for p in check_family.extra_scope(lock))
    assert any("undeclared" in p for p in check_family.extra_scope(lock))


def test_bare_dependency_is_not_expanded_to_all_versions(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(LOCK + EXTRA.replace('"ruff_python_ast 0.0.16"', '"ruff_python_ast"'))
    assert any("ambiguous dependency ruff_python_ast" in p for p in check_family.extra_scope(lock))


def test_embedded_family_cannot_enter_ty(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    lock.write_text(
        LOCK
        + EXTRA.replace(
            'dependencies = ["ruff_python_ast 0.0.16", "salsa"]',
            'dependencies = ["ruff_python_ast 0.0.14", "salsa"]',
        )
    )
    assert any("outside Pyrefly embedded Ruff" in p for p in check_family.extra_scope(lock))


@pytest.mark.parametrize("name", ["ty_ide", "ty_project"])
def test_unpublished_ty_versions_need_exact_latest_source(tmp_path: Path, name: str) -> None:
    lock = tmp_path / "Cargo.lock"
    unpublished = (
        f'\n[[package]]\nname = "{name}"\nversion = "0.0.0"\n'
        f'source = "{check_family.RUFF_SOURCE}"\n'
    )
    lock.write_text(LOCK + EXTRA + unpublished)
    assert check_family.extra_scope(lock) == []
    lock.write_text(
        LOCK + EXTRA + unpublished.replace(check_family.RUFF_SOURCE, check_family.REGISTRY)
    )
    assert any("undeclared analyzer source" in p for p in check_family.extra_scope(lock))


def test_feature_unifier_may_depend_on_latest_family_only(tmp_path: Path) -> None:
    lock = tmp_path / "Cargo.lock"
    hack = (
        '\n[[package]]\nname = "lctx-workspace-hack"\nversion = "0.1.0"\n'
        'dependencies = ["ruff_python_ast 0.0.16"]\n'
    )
    lock.write_text(LOCK + EXTRA + hack)
    assert check_family.extra_scope(lock) == []
    lock.write_text(LOCK + EXTRA + hack.replace("ruff_python_ast 0.0.16", "ruff_python_ast 0.0.14"))
    assert any("outside Pyrefly embedded Ruff" in p for p in check_family.extra_scope(lock))


def oracle_fixture() -> tuple[list[check_family.LockPackage], check_family.CargoMetadata]:
    identities: list[check_family.PackageIdentity] = [
        ("lctx-analytics", "0.1.0", None),
        *check_family.ORACLE_CHAIN,
        ("petgraph", "0.8.3", check_family.REGISTRY),
        ("lctx-workspace-hack", "0.1.0", None),
    ]
    ids = [f"package-{i}" for i in range(len(identities))]
    packages: list[check_family.LockPackage] = []
    metadata_packages: list[check_family.MetadataPackage] = []
    nodes: list[check_family.MetadataNode] = []
    for package_id, (name, version, source) in zip(ids, identities, strict=True):
        package: check_family.LockPackage = {"name": name, "version": version}
        if source is not None:
            package["source"] = source
        packages.append(package)
        metadata_packages.append(
            {"id": package_id, "name": name, "version": version, "source": source}
        )
        nodes.append({"id": package_id, "deps": []})
    packages[0]["dependencies"] = ["odis", "petgraph 0.8.3", "lctx-workspace-hack"]
    packages[1]["dependencies"] = ["rust-sugiyama"]
    packages[2]["dependencies"] = ["petgraph 0.6.5"]
    nodes[0]["deps"] = [
        {"pkg": ids[1], "dep_kinds": [{"kind": "dev"}]},
        {"pkg": ids[4], "dep_kinds": [{"kind": None}]},
        {"pkg": ids[5], "dep_kinds": [{"kind": None}]},
    ]
    nodes[1]["deps"] = [{"pkg": ids[2], "dep_kinds": [{"kind": None}]}]
    nodes[2]["deps"] = [{"pkg": ids[3], "dep_kinds": [{"kind": None}]}]
    return packages, {
        "packages": metadata_packages,
        "workspace_members": [ids[0], ids[5]],
        "resolve": {"nodes": nodes},
    }


def write_packages(path: Path, packages: list[check_family.LockPackage]) -> None:
    lines = ["version = 4"]
    for package in packages:
        lines.extend(
            [
                "",
                "[[package]]",
                f'name = "{package["name"]}"',
                f'version = "{package["version"]}"',
            ]
        )
        if (source := package.get("source")) is not None:
            lines.append(f'source = "{source}"')
        if dependencies := package.get("dependencies"):
            lines.append("dependencies = [" + ", ".join(f'"{dep}"' for dep in dependencies) + "]")
    path.write_text("\n".join(lines))


def test_only_proven_oracle_graph_is_exempt_and_other_family_stays_strict(tmp_path: Path) -> None:
    packages, metadata = oracle_fixture()
    development, problems = check_family.oracle_scope(packages, metadata)
    assert problems == []
    assert development == frozenset({check_family.ORACLE_CHAIN[-1]})
    lock = tmp_path / "Cargo.lock"
    write_packages(lock, packages)
    assert check_family.duplicates(lock) == {"petgraph": ["0.6.5", "0.8.3"]}
    assert check_family.duplicates(lock, development=development) == {}
    packages.extend(
        [
            {"name": "arrow-array", "version": "58.0.0"},
            {"name": "arrow-array", "version": "59.3.0"},
        ]
    )
    write_packages(lock, packages)
    assert check_family.duplicates(lock, development=development) == {
        "arrow-array": ["58.0.0", "59.3.0"]
    }


def test_oracle_without_metadata_proof_fails_closed(tmp_path: Path) -> None:
    packages, _ = oracle_fixture()
    development, problems = check_family.oracle_scope(packages, None)
    assert not development
    assert any("lacks matching Cargo metadata proof" in p for p in problems)
    lock = tmp_path / "Cargo.lock"
    write_packages(lock, packages)
    # No manifest means no fresh Cargo proof can be acquired.
    assert check_family.main([str(lock)]) == 1


@pytest.mark.parametrize("kind", [None, "build"])
@pytest.mark.parametrize("through_unifier", [False, True])
def test_oracle_normal_or_build_contamination_fails(
    kind: str | None, through_unifier: bool
) -> None:
    packages, metadata = oracle_fixture()
    nodes = metadata["resolve"]["nodes"]
    if through_unifier:
        nodes[5]["deps"].append({"pkg": nodes[3]["id"], "dep_kinds": [{"kind": kind}]})
    else:
        nodes[0]["deps"][0]["dep_kinds"] = [{"kind": kind}]
    development, problems = check_family.oracle_scope(packages, metadata)
    assert not development
    assert any("normal/build dependency closure" in p for p in problems)


@pytest.mark.parametrize("target", [1, 2, 3])
def test_oracle_other_dev_ingress_fails(target: int) -> None:
    packages, metadata = oracle_fixture()
    nodes = metadata["resolve"]["nodes"]
    nodes[5]["deps"].append({"pkg": nodes[target]["id"], "dep_kinds": [{"kind": "dev"}]})
    development, problems = check_family.oracle_scope(packages, metadata)
    assert not development
    assert any("sole pinned oracle dependency path" in p for p in problems)


@pytest.mark.parametrize("index", [1, 2, 3])
def test_oracle_wrong_source_even_if_lock_matches_fails(index: int) -> None:
    packages, metadata = oracle_fixture()
    packages[index]["source"] = "git+https://example.invalid/oracle#wrong"
    metadata["packages"][index]["source"] = packages[index]["source"]
    development, problems = check_family.oracle_scope(packages, metadata)
    assert not development
    assert any("wrong source/version" in p for p in problems)


def test_oracle_incomplete_graph_or_stale_lock_fails() -> None:
    packages, metadata = oracle_fixture()
    stale = deepcopy(metadata)
    stale["packages"][4]["version"] = "0.8.2"
    incomplete = deepcopy(metadata)
    incomplete["resolve"]["nodes"].pop()
    missing_kinds = deepcopy(metadata)
    missing_kinds["resolve"]["nodes"][0]["deps"][0]["dep_kinds"] = []
    for invalid in [stale, incomplete, missing_kinds]:
        development, problems = check_family.oracle_scope(packages, invalid)
        assert not development
        assert any("incomplete or mismatched" in p for p in problems)
    packages[1]["dependencies"] = []
    development, problems = check_family.oracle_scope(packages, metadata)
    assert not development
    assert any("matching pinned lockfile path" in p for p in problems)
