"""Read-only coverage accounting for the verification runner's complete test scopes.

Cargo owns Rust targets; pytest's configured roots own Python test files. This
compares those inputs with the runner's existing boundary plan, without compiling,
importing tests or creating fixtures. It does not claim test execution or cover
every optional-feature combination. Rust ignore annotations are source inventory,
not a substitute for the runner's per-test results.
"""

from __future__ import annotations

import fnmatch
import json
import re
import subprocess
import tomllib
from collections.abc import Sequence
from pathlib import Path
from typing import Any

LIBRARY_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}
IGNORE = re.compile(
    r'^\s*#\[ignore(?:\s*=\s*"(?P<reason>[^"\n]*)")?\s*\]\s*'
    r"(?:#\[[^\n]*\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+(?P<name>\w+)",
    re.MULTILINE,
)


def _values(argv: Sequence[str], *flags: str) -> list[str]:
    values = []
    for index, argument in enumerate(argv):
        option, equals, value = argument.partition("=")
        if option in flags:
            if equals:
                values.append(value)
            elif index + 1 < len(argv):
                values.append(argv[index + 1])
    return values


def _target_selected(argv: Sequence[str], kind: str, name: str) -> bool:
    named = {"lib": "--lib", "bin": "--bin", "test": "--test", "example": "--example"}
    plural = {"bin": "--bins", "test": "--tests", "example": "--examples"}
    selectors = {*named.values(), *plural.values(), "--all-targets"}
    if not any(argument.partition("=")[0] in selectors for argument in argv):
        return True
    if "--all-targets" in argv or ("--tests" in argv and kind in {"lib", "bin", "test"}):
        return True
    if kind == "lib":
        return "--lib" in argv
    return plural[kind] in argv or name in _values(argv, named[kind])


def _default_features(package: dict[str, Any]) -> set[str]:
    features = package.get("features", {})
    pending = ["default"]
    enabled = set()
    while pending:
        feature = pending.pop()
        if feature in enabled:
            continue
        enabled.add(feature)
        pending.extend(value for value in features.get(feature, []) if value in features)
    return enabled


def _python_files(root: Path, paths: Sequence[str], patterns: Sequence[str]) -> set[str]:
    found = set()
    for value in paths:
        path = root / value.split("::", 1)[0]
        candidates = path.rglob("*.py") if path.is_dir() else [path]
        for candidate in candidates:
            if candidate.is_file() and any(fnmatch.fnmatch(candidate.name, p) for p in patterns):
                found.add(candidate.relative_to(root).as_posix())
    return found


def audit(
    plan: Sequence[Any],
    *,
    root: Path,
    metadata: dict[str, Any] | None = None,
    excluded_plan: Sequence[Any] = (),
) -> dict[str, Any]:
    """Account for default-feature Rust test targets and configured Python files.

    `plan` is the existing verification plan; this module owns no parallel list of
    packages, test paths or prerequisite declarations. Filtered Rust commands do
    not establish complete coverage of their target.
    """
    if metadata is None:
        try:
            done = subprocess.run(
                ("cargo", "metadata", "--no-deps", "--offline", "--format-version", "1"),
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            )
        except (OSError, subprocess.CalledProcessError) as error:
            raise ValueError(
                f"test coverage needs Cargo's workspace target metadata: {error}"
            ) from error
        metadata = json.loads(done.stdout)
    members = set(metadata["workspace_members"])
    packages = [package for package in metadata["packages"] if package["id"] in members]
    commands = [step for boundary in plan for step in boundary.steps if step.skipped is None]
    rust = [step.argv for step in commands if step.step.tool == "nextest"]
    ignored_commands = [
        step.argv
        for step in commands
        if (step.step.tool == "nextest" and "--run-ignored" in step.argv)
        or (step.step.tool == "cargo" and "test" in step.argv and "--ignored" in step.argv)
    ]
    unrestricted = [
        argv
        for argv in rust
        if not any(
            arg.partition("=")[0] in {"-E", "--filter-expr", "--filterset", "--filter"}
            or arg.startswith("-E")
            for arg in argv
        )
    ]
    missing_rust = []
    conditional = []
    rust_count = 0
    for package in packages:
        for target in package["targets"]:
            kinds = set(target["kind"])
            kind = "lib" if kinds & LIBRARY_KINDS else next(iter(kinds), "")
            if not target.get("test", True) or kind not in {"lib", "bin", "test", "example"}:
                continue
            identity = f"{package['name']}:{kind}:{target['name']}"
            required = set(target.get("required-features", []))
            if not required <= _default_features(package):
                conditional.append({"target": identity, "required_features": sorted(required)})
                continue
            rust_count += 1
            if not any(
                ("--workspace" in argv or package["name"] in _values(argv, "-p", "--package"))
                and _target_selected(argv, kind, target["name"])
                for argv in unrestricted
            ):
                missing_rust.append(identity)
    settings = tomllib.loads((root / "pyproject.toml").read_text())["tool"]["pytest"]["ini_options"]
    patterns = settings.get("python_files", ["test_*.py", "*_test.py"])
    if isinstance(patterns, str):
        patterns = patterns.split()
    testpaths = settings["testpaths"]
    if isinstance(testpaths, str):
        testpaths = testpaths.split()
    missing_roots = [value for value in testpaths if not (root / value).exists()]
    expected_python = _python_files(root, testpaths, patterns)
    covered_python: set[str] = set()
    for step in commands:
        if step.step.tool != "pytest":
            continue
        arguments = step.step.argv
        paths = [
            value for value in arguments if not value.startswith("-") and (root / value).exists()
        ]
        selected = _python_files(root, paths, patterns)
        ignored = _python_files(root, _values(arguments, "--ignore"), patterns)
        covered_python.update(selected - ignored)
    ignored_controls = []
    for package in packages:
        directory = Path(package["manifest_path"]).parent
        for folder in (directory / "src", directory / "tests"):
            for path in sorted(folder.rglob("*.rs")):
                source = path.read_text()
                for match in IGNORE.finditer(source):
                    reason = match.group("reason") or "Rust #[ignore]"
                    name = match.group("name")
                    disposition = "excluded_by_default"
                    if "joined process" in reason:
                        disposition = "delegated_to_parent"
                    elif any(any(name in arg for arg in argv) for argv in ignored_commands):
                        disposition = "explicitly_selected"
                    ignored_controls.append(
                        {
                            "package": package["name"],
                            "source": path.relative_to(root).as_posix(),
                            "line": source.count("\n", 0, match.start()) + 1,
                            "function": name,
                            "reason": reason,
                            "disposition": disposition,
                        }
                    )
    missing_python = sorted(expected_python - covered_python)
    excluded_rust, excluded_python = [], []
    unexpected_rust, unexpected_python = missing_rust, missing_python
    if excluded_plan:
        omitted = audit(excluded_plan, root=root, metadata=metadata)
        excluded_rust = sorted(set(missing_rust) - set(omitted["missing_rust_targets"]))
        excluded_python = sorted(set(missing_python) - set(omitted["missing_python_files"]))
        unexpected_rust = sorted(set(missing_rust) - set(excluded_rust))
        unexpected_python = sorted(set(missing_python) - set(excluded_python))
    return {
        "basis": (
            "default-feature Cargo test targets and pytest-configured test files; "
            "execution is reported separately"
        ),
        "rust_target_count": rust_count,
        "python_file_count": len(expected_python),
        "missing_rust_targets": sorted(missing_rust),
        "missing_python_files": missing_python,
        "missing_python_roots": missing_roots,
        "conditional_rust_targets": conditional,
        "ignored_controls": ignored_controls,
        "complete": not (missing_rust or missing_python or missing_roots),
        "selected_complete": not (unexpected_rust or unexpected_python or missing_roots),
        "excluded_rust_targets": excluded_rust,
        "excluded_python_files": excluded_python,
        "unexpected_missing_rust_targets": unexpected_rust,
        "unexpected_missing_python_files": unexpected_python,
    }
