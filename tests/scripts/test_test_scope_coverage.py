"""Coverage accounting rejects forgotten targets/files without running their tests."""

from types import SimpleNamespace

import pytest

from test_scope_coverage import audit


def command(tool, *argv):
    return SimpleNamespace(skipped=None, argv=argv, step=SimpleNamespace(tool=tool, argv=argv))


@pytest.fixture
def workspace(tmp_path):
    (tmp_path / "pyproject.toml").write_text('[tool.pytest.ini_options]\ntestpaths = ["tests"]\n')
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests/test_old.py").write_text("def test_old(): pass\n")
    (tmp_path / "crate/src").mkdir(parents=True)
    (tmp_path / "crate/Cargo.toml").write_text("")
    metadata = {
        "workspace_members": ["crate-id"],
        "packages": [
            {
                "id": "crate-id",
                "name": "crate",
                "manifest_path": str(tmp_path / "crate/Cargo.toml"),
                "features": {},
                "targets": [
                    {"name": "crate", "kind": ["lib"], "test": True},
                    {"name": "journey", "kind": ["test"], "test": True},
                ],
            }
        ],
    }
    return tmp_path, metadata


def inspect(workspace, *commands):
    root, metadata = workspace
    return audit([SimpleNamespace(steps=commands)], root=root, metadata=metadata)


def test_complete_package_and_python_directory_automatically_include_new_tests(workspace):
    root, _ = workspace
    (root / "tests/test_new.py").write_text("def test_new(): pass\n")
    report = inspect(
        workspace,
        command("nextest", "cargo", "nextest", "run", "-p", "crate"),
        command("pytest", "tests"),
    )
    assert report["complete"]
    assert report["rust_target_count"] == 2
    assert report["python_file_count"] == 2


def test_new_target_and_python_file_cannot_fall_out_of_named_selections(workspace):
    root, _ = workspace
    (root / "tests/test_new.py").write_text("")
    report = inspect(
        workspace,
        command("nextest", "cargo", "nextest", "run", "-p", "crate", "--lib"),
        command("pytest", "tests/test_old.py"),
    )
    assert not report["complete"]
    assert report["missing_rust_targets"] == ["crate:test:journey"]
    assert report["missing_python_files"] == ["tests/test_new.py"]


def test_new_workspace_member_needs_an_execution_owner(workspace):
    root, metadata = workspace
    metadata["workspace_members"].append("new-id")
    metadata["packages"].append(
        {
            "id": "new-id",
            "name": "new",
            "manifest_path": str(root / "new/Cargo.toml"),
            "targets": [{"name": "new", "kind": ["proc-macro"], "test": True}],
        }
    )
    report = inspect(workspace, command("nextest", "-p", "crate"), command("pytest", "tests"))
    assert report["missing_rust_targets"] == ["new:lib:new"]


def test_filtered_rust_command_does_not_establish_target_coverage(workspace):
    report = inspect(
        workspace,
        command("nextest", "-p", "crate", "-E", "test(one)"),
        command("pytest", "tests"),
    )
    assert not report["complete"]
    assert len(report["missing_rust_targets"]) == 2


def test_pytest_ignored_file_is_reported_as_unassigned(workspace):
    report = inspect(
        workspace,
        command("nextest", "--workspace"),
        command("pytest", "tests", "--ignore=tests/test_old.py"),
    )
    assert report["missing_python_files"] == ["tests/test_old.py"]


@pytest.mark.parametrize("ignored_tool", ["nextest", "cargo"])
def test_optional_feature_scope_and_ignored_lifecycle_controls_are_explicit(
    workspace, ignored_tool
):
    root, metadata = workspace
    metadata["packages"][0]["targets"].append(
        {
            "name": "optional",
            "kind": ["test"],
            "test": True,
            "required-features": ["gpu"],
        }
    )
    (root / "crate/src/lib.rs").write_text(
        '#[test]\n#[ignore = "requires exclusive maintenance"]\nfn disruptive() {}\n'
        '#[ignore = "Invoked in a joined process by the bounded parent test"]\nfn child() {}\n'
        '#[ignore = "installed fixture"]\nasync fn installed() {}\n'
    )
    ignored_command = (
        command("nextest", "-p", "crate", "--run-ignored", "all", "-E", "test(installed)")
        if ignored_tool == "nextest"
        else command(
            "cargo", "cargo", "test", "-p", "crate", "installed", "--", "--ignored", "--exact"
        )
    )
    report = inspect(
        workspace,
        command("nextest", "-p", "crate"),
        command("pytest", "tests"),
        ignored_command,
    )
    assert report["complete"]
    assert report["conditional_rust_targets"] == [
        {
            "target": "crate:test:optional",
            "required_features": ["gpu"],
        }
    ]
    dispositions = {row["function"]: row["disposition"] for row in report["ignored_controls"]}
    assert dispositions == {
        "disruptive": "excluded_by_default",
        "child": "delegated_to_parent",
        "installed": "explicitly_selected",
    }


def test_default_feature_transitive_target_is_required(workspace):
    _, metadata = workspace
    metadata["packages"][0]["features"] = {"default": ["a"], "a": ["b"], "b": []}
    metadata["packages"][0]["targets"].append(
        {
            "name": "enabled",
            "kind": ["test"],
            "test": True,
            "required-features": ["b"],
        }
    )
    report = inspect(
        workspace, command("nextest", "-p", "crate", "--lib"), command("pytest", "tests")
    )
    assert "crate:test:enabled" in report["missing_rust_targets"]
    assert not report["conditional_rust_targets"]
