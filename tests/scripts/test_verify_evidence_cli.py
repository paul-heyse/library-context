"""Read-only experiment plans and portable native-evidence CLI controls."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest
import test_verify_evidence_nextest as native_controls

import runs
import storage_lifecycle
import verify
import workspace_env
from harness import ProcessIdentity, hold_lock, write_json_atomic
from verify_evidence import cli, nextest, projection, rerun
from verify_evidence.contracts import CaptureUnsupported

ROOT = Path(__file__).resolve().parents[2]
native_fixture = native_controls.native_project


def write_run(root: Path, *, tool="pytest", boundary="tooling:python", step="pytest"):
    directory = root / "verify-evidence" / "case123"
    directory.mkdir(parents=True)
    invocation = {
        "schema": 1,
        "id": "case123",
        "tool": tool,
        "boundary": boundary,
        "step": step,
        "cwd": str(ROOT),
        "argv": ["uv", "run", "--no-sync", "pytest", "tests/scripts"],
        "artifacts": {"report": "pytest-reportlog.jsonl"},
        "capture": {"status": "available"},
        "command": {"exit": 1},
        "spawn": {"status": "started"},
        "postprocessing": {"status": "failed"},
        "boundary_final": {"outcome": "failed"},
        "finalization": {"status": "available"},
    }
    write_json_atomic(directory / "invocation.json", invocation)
    write_json_atomic(
        root / "verify-evidence.json",
        {"schema": 1, "invocations": [{"id": "case123", "boundary": boundary, "step": step}]},
    )
    write_json_atomic(root / runs.SUMMARY, {"schema": 1, "options": {}, "results": []})
    write_json_atomic(root / runs.RECORD, {"schema": 2, "termination": "completed", "exit": 1})
    return directory, invocation


def report_line(directory, node, outcome="failed", payload="retained failure", phase="call"):
    with (directory / "pytest-reportlog.jsonl").open("a") as stream:
        stream.write(
            json.dumps(
                {
                    "$report_type": "TestReport",
                    "nodeid": node,
                    "when": phase,
                    "outcome": outcome,
                    "longrepr": payload,
                }
            )
            + "\n"
        )


def invoke(capsys, *arguments, expected=0):
    assert verify.main(list(map(str, arguments))) == expected
    captured = capsys.readouterr()
    if expected:
        return captured.err
    assert not captured.err
    return json.loads(captured.out)


@pytest.fixture
def no_execution(monkeypatch):
    def forbidden(*args, **kwargs):
        raise AssertionError("inspection/planning must not execute or observe readiness")

    monkeypatch.setattr(subprocess, "run", forbidden)
    monkeypatch.setattr(subprocess, "Popen", forbidden)
    monkeypatch.setattr(workspace_env, "observe", forbidden)
    monkeypatch.setattr(verify, "observe_requirements", forbidden)
    monkeypatch.setattr(
        cli.SourceObserver, "observe", lambda self: {"id": "current", "available": True}
    )


def test_nextest_selector_keeps_filter_union_hash_partition_and_native_grammar(no_execution):
    original = [
        "cargo",
        "nextest",
        "run",
        "--release",
        "-E",
        "test(first)",
        "--filterset=test(second)",
        "--partition",
        "hash:1/2",
        "--run-ignored",
        "all",
        "--ignore-default-filter",
        "--",
        "--exact",
        "same",
    ]
    before = list(original)
    result = rerun.nextest_case(original, {"binary_id": "pkg::binary", "test": r"a(b)c,d\e"})
    assert original == before
    assert result[-3:] == ["--", "--exact", "same"]
    expression = result[result.index("-E") + 1]
    assert "(test(first)) | (test(second))" in expression
    assert r"binary_id(=pkg::binary) & test(=a(b\)c\,d\\e)" in expression
    assert result[result.index("--partition") + 1] == "hash:1/2"
    assert "--run-ignored" in result and "--ignore-default-filter" in result
    assert result.count("-E") == 1 and not any(x.startswith("--filterset") for x in result)


@pytest.mark.parametrize("partition", ["count:1/2", "slice:1/2"])
def test_unstable_partition_and_missing_native_identity_are_refused(partition, no_execution):
    with pytest.raises(CaptureUnsupported, match="hash partition"):
        rerun.nextest_case(
            ["cargo", "nextest", "run", "--partition=" + partition],
            {"binary_id": "pkg", "test": "one"},
        )
    with pytest.raises(CaptureUnsupported, match="identity unavailable"):
        rerun.nextest_case(["cargo", "nextest", "run"], {"test": "one", "binary_id": None})


def test_pytest_parametrized_node_replaces_original_target_union_preserving_values(no_execution):
    original = [
        "uv",
        "run",
        "--no-sync",
        "pytest",
        "tests/first",
        "tests/second.py::old",
        "-k",
        "wanted",
        "-m",
        "slow",
        "-o",
        "addopts=-q",
        "--maxfail",
        "2",
        "--tb=long",
        "-p",
        "custom_plugin",
        "--",
        "tests/third",
    ]
    node = "tests/second.py::test_case[value with spaces, punctuation)]"
    result = rerun.pytest_case(original, node)
    assert result == [
        "uv",
        "run",
        "--no-sync",
        "pytest",
        "-k",
        "wanted",
        "-m",
        "slow",
        "-o",
        "addopts=-q",
        "--maxfail",
        "2",
        "--tb=long",
        "-p",
        "custom_plugin",
        "--",
        node,
    ]
    assert original[4:6] == ["tests/first", "tests/second.py::old"]


@pytest.mark.parametrize(
    "argv",
    [
        ["pytest", "tests", "--unknown-option", "value"],
        ["pytest", "tests", "--maxfail"],
        ["python", "-m", "unrecognized", "tests"],
    ],
)
def test_pytest_ambiguous_native_grammar_is_refused(argv, no_execution):
    with pytest.raises(CaptureUnsupported):
        rerun.pytest_case(argv, "tests/test_one.py::test_case")


@pytest.mark.parametrize("option,value", [("--show-capture", "all"), ("-r", "a")])
def test_pytest_report_options_keep_required_value(option, value, no_execution):
    result = rerun.pytest_case(["pytest", "tests", option, value], "tests/test_a.py::one")
    assert result == ["pytest", option, value, "--", "tests/test_a.py::one"]


def recorded(boundary, step, tool, argv, **fields):
    return {
        "id": step + "123",
        "boundary": boundary,
        "step": step,
        "tool": tool,
        "argv": argv,
        "settings": {"RUST_BACKTRACE": "full"},
        **fields,
    }


def test_cli_producer_dependency_is_conditional_and_unfiltered(no_execution):
    build = recorded("store:rust", "build-cli", "cargo", ["cargo", "build", "-p", "lctx"])
    target = recorded(
        "store:rust", "nextest", "nextest", ["cargo", "nextest", "run", "-p", "lctx-publisher"],
        binaries=[{"package_id": "path+file:///publisher#lctx-publisher@0.1.0", "package_name": "lctx-publisher", "target": {"kind": ["test"], "name": "publication"}}],
    )
    case = {"case_key": "case", "binary_id": "lctx-publisher::publication", "test": "one"}
    selected = rerun.plan(target, case, {"options": {"cli": True}}, [build, target])
    assert [row["step"] for row in selected["operations"]] == ["build-cli", "nextest"]
    assert selected["operations"][0]["argv"] == build["argv"]
    assert selected["operations"][0]["case_filter"] is False
    assert selected["operations"][1]["case_filter"] is True
    assert selected["executes"] is False and selected["fixture"] is True
    assert "native-store" in selected["requirements"]
    without = rerun.plan(target, case, {"options": {"cli": False}}, [target])
    assert [row["step"] for row in without["operations"]] == ["nextest"]


@pytest.mark.parametrize("kind,binary,selector", [
    ("lib", "pkg", ["--lib"]),
    ("proc-macro", "pkg", ["--lib"]),
    ("test", "pkg::chosen", ["--test", "chosen"]),
    ("bin", "pkg::bin/chosen", ["--bin", "chosen"]),
    ("example", "pkg::example/chosen", ["--example", "chosen"]),
    ("bench", "pkg::bench/chosen", ["--bench", "chosen"]),
])
def test_recorded_target_narrows_build_scope_preserves_policy(kind, binary, selector, no_execution):
    artifact = {"package_id": "path+file:///work/pkg#0.1.0", "package_name": "pkg",
        "target": {"kind": [kind], "name": "chosen"}, "features": ["default", "extra"]}
    argv = ["cargo", "nextest", "run", "--workspace", "--exclude", "other", "-pother",
        "--all-targets", "--tests", "--test=wide", "--cargo-profile", "release", "--profile", "ci",
        "--features", "extra", "--no-default-features", "--ignore-default-filter"]
    invocation = recorded("model:rust", "nextest", "nextest", argv, binaries=[artifact])
    case = {"binary_id": binary, "test": "nested::chosen"}
    exact = rerun.cargo_target(invocation, case)
    narrowed = rerun.nextest_case(rerun.narrow_cargo(argv, exact), case)
    assert "--workspace" not in narrowed and "--all-targets" not in narrowed
    assert "--tests" not in narrowed and "--test=wide" not in narrowed
    assert narrowed[narrowed.index("--package") + 1] == artifact["package_id"]
    at = narrowed.index(selector[0])
    assert narrowed[at:at + len(selector)] == selector
    for flag, value in (("--cargo-profile", "release"), ("--profile", "ci"), ("--features", "extra")):
        assert narrowed[narrowed.index(flag) + 1] == value
    assert "--no-default-features" in narrowed and "--ignore-default-filter" in narrowed
    assert f"binary_id(={binary}) & test(=nested::chosen)" in narrowed


def test_missing_or_ambiguous_recorded_target_refuses_without_discovery(no_execution):
    case = {"binary_id": "pkg::one", "test": "chosen"}
    with pytest.raises(CaptureUnsupported, match="missing or ambiguous"):
        rerun.cargo_target({}, case)
    rows = [{"package_name": "pkg", "package_id": identity, "target": {"kind": ["test"], "name": "one"}}
        for identity in ("path+file:///a#pkg@0.1", "path+file:///b#pkg@0.2")]
    with pytest.raises(CaptureUnsupported, match="2 matches"):
        rerun.cargo_target({"binaries": rows}, case)


def test_completion_retains_cargo_target_and_feature_identity(tmp_path):
    from types import SimpleNamespace
    from verify_evidence.capture import Invocation

    manifest = tmp_path / "Cargo.toml"
    manifest.write_text('[package]\nname="recorded-package"\nversion="0.1.0"\n')
    executable = tmp_path / "unit-test"
    executable.write_bytes(b"observed executable")
    event = {"reason": "compiler-artifact", "executable": str(executable),
        "manifest_path": str(manifest), "package_id": "path+file:///recorded#0.1.0",
        "target": {"kind": ["proc-macro"], "name": "recorded", "src_path": "source.rs"},
        "features": ["feature-one"], "profile": {"test": True, "opt_level": "2"}}
    (tmp_path / "nextest-events.jsonl").write_text(json.dumps(event) + "\n")
    owner = SimpleNamespace(context=lambda: "context", binary_cache={})
    invocation = Invocation(owner, tmp_path, {"tool": "nextest"})
    invocation.command_finished(SimpleNamespace(exit=0, launch_error=None))
    row = invocation.row["binaries"][0]
    assert row["package_name"] == "recorded-package"
    assert row["package_id"] == event["package_id"]
    assert row["target"] == event["target"]
    assert row["features"] == event["features"] and row["profile"] == event["profile"]
    assert rerun.cargo_target(invocation.row, {"binary_id": "recorded-package"}) == row


@pytest.mark.parametrize("kind", ["lib", "proc-macro", "test", "bin", "example", "bench"])
def test_narrowed_recorded_target_executes_exact_native_case(tmp_path, native_fixture, kind):
    project, repo, _ = native_fixture
    name = "chosen"
    if kind == "proc-macro":
        with (project / "Cargo.toml").open("a") as stream:
            stream.write("proc-macro=true\n")
    elif kind != "lib":
        with (project / "Cargo.toml").open("a") as stream:
            stream.write(f'\n[[{kind}]]\nname="{name}"\npath="chosen.rs"\n')
        (project / "chosen.rs").write_text('#[test] fn passes() {}\nfn main() {}\n')
    (project / "tests").mkdir()
    (project / "tests/unselected.rs").write_text("this sibling must never compile")
    metadata = subprocess.run(["cargo", "metadata", "--offline", "--no-deps", "--format-version", "1", "--manifest-path", str(project / "Cargo.toml")], cwd=ROOT, capture_output=True, text=True)
    assert metadata.returncode == 0, metadata.stderr
    package = json.loads(metadata.stdout)["packages"][0]
    cargo_target = next(target for target in package["targets"] if kind in target["kind"])
    artifact = {"package_id": package["id"], "package_name": package["name"], "target": cargo_target}
    binary = package["name"] if kind in {"lib", "proc-macro"} else f'{package["name"]}::{name}' if kind == "test" else f'{package["name"]}::{kind}/{name}'
    original = ["cargo", "nextest", "run", "--manifest-path", str(project / "Cargo.toml"), "--config-file", str(repo), "--user-config-file", "none", "--all-targets", "--message-format", "libtest-json-plus"]
    command = rerun.nextest_case(rerun.narrow_cargo(original, artifact), {"binary_id": binary, "test": "passes"})
    done = subprocess.run(command, cwd=ROOT, env=dict(os.environ) | {"NEXTEST_EXPERIMENTAL_LIBTEST_JSON": "1", "CARGO_TERM_COLOR": "never"}, capture_output=True, text=True)
    assert done.returncode == 0, done.stderr
    events = [json.loads(line) for line in done.stdout.splitlines()]
    finished = [event for event in events if event.get("type") == "test" and event.get("event") == "ok"]
    assert len(finished) == 1 and finished[0]["name"].endswith("$passes")
    assert "unselected.rs" not in done.stderr


def test_serving_prerequisites_compose_shared_build_and_restart_once(no_execution):
    build = recorded(
        "serving:mcp", "build", "cargo", ["cargo", "build", "-p", "lctx-eval", "-p", "lctx"]
    )
    serving = recorded(
        "serving:mcp",
        "native_journey",
        "cargo",
        ["cargo", "test", "--test", "native_journey"],
        restart_after=True,
    )
    target = recorded(
        "serving:mcp",
        "pytest",
        "pytest",
        ["uv", "run", "--no-sync", "pytest", "python/lctx_mcp/tests"],
    )
    case = {"case_key": "node", "nodeid": "python/lctx_mcp/tests/test_native_session.py::one"}
    result = rerun.plan(target, case, {"options": {}}, [build, serving, target])
    assert [(row.get("step"), row.get("operation")) for row in result["operations"]] == [
        ("build", None),
        ("native_journey", None),
        (None, "fixture-restart"),
        ("pytest", None),
    ]
    assert result["operations"][0]["argv"] == build["argv"]
    assert result["operations"][1]["argv"] == serving["argv"]
    assert result["requirements"] == ["native-python", "native-serving", "tools"]
    retained = rerun.plan(
        target, case, {"options": {"serving": "/retained/config"}}, [build, target]
    )
    assert [row.get("operation") for row in retained["operations"]] == [
        None,
        "admit-retained-serving",
        None,
    ]
    with pytest.raises(CaptureUnsupported, match="recorded command"):
        rerun.plan(target, case, {"options": {}}, [build, target])


def test_cli_report_case_and_evidence_pages_events_and_printed_plan(tmp_path, capsys, no_execution):
    root = tmp_path / "run"
    directory, _ = write_run(root)
    for index in range(3):
        report_line(directory, f"tests/test_a.py::case[{index}]", payload=f"failure-{index}")
    report = invoke(capsys, "report", root, "--json")
    assert report["observation_counts"] == {"failed": 3}
    assert report["failure_groups"][0]["count"] == 3
    assert report["invocations"][0]["command"]["exit"] == 1
    page = invoke(capsys, "cases", root, "--limit", "1", "--outcome", "failed")
    assert page["total"] == 3 and page["omitted"] == 2
    continuation = invoke(
        capsys, "cases", root, "--limit", "2", "--outcome", "failed", "--cursor", page["cursor"]
    )
    assert continuation["cursor"] is None
    identities = [row["case"] for row in [*page["cases"], *continuation["cases"]]]
    assert len(set(identities)) == 3
    selected = page["cases"][0]["case"]
    first = invoke(capsys, "evidence", root, "--case", selected, "--limit-bytes", "3")
    second = invoke(
        capsys,
        "evidence",
        root,
        "--case",
        selected,
        "--limit-bytes",
        "100",
        "--cursor",
        first["cursor"],
    )
    assert first["evidence"][0]["text"] + second["evidence"][0]["text"] == "failure-0"
    milestones = invoke(capsys, "events", root, "--step", "pytest", "--limit", "2")
    rest = invoke(
        capsys, "events", root, "--step", "pytest", "--limit", "5", "--cursor", milestones["cursor"]
    )
    event_rows = milestones["events"] + rest["events"]
    while rest["cursor"]:
        rest = invoke(
            capsys, "events", root, "--step", "pytest", "--limit", "5", "--cursor", rest["cursor"]
        )
        event_rows.extend(rest["events"])
    assert [row["event"] for row in event_rows[:5]] == [
        "spawn",
        "command",
        "postprocessing",
        "boundary_final",
        "finalization",
    ]
    assert [row["event"] for row in event_rows[5:]] == ["TestReport"] * 3
    plan = invoke(capsys, "rerun-plan", root, "--case", selected, "--print")
    assert plan["executes"] is False
    assert plan["operations"][-1]["argv"][-1] == "tests/test_a.py::case[0]"
    assert plan["remaining_obligations"]


def test_cli_legacy_coverage_and_context_comparison_are_explicit(tmp_path, capsys, no_execution):
    legacy = tmp_path / "legacy"
    legacy.mkdir()
    write_json_atomic(legacy / runs.SUMMARY, {"results": [{"outcome": "failed"}]})
    old = invoke(capsys, "report", legacy)
    assert old["coverage"] == "legacy boundary/log only"
    assert old["observation_counts"] == {} and old["invocations"] == []
    first, second = tmp_path / "first", tmp_path / "second"
    for root, ident, files in [
        (first, "before", {"tracked.rs": "same", "untracked.py": "old"}),
        (second, "after", {"tracked.rs": "same", "untracked.py": "new"}),
    ]:
        directory, row = write_run(root)
        (directory / "pytest-reportlog.jsonl").touch()
        row["context_before"] = ident
        row["settings"] = {"RUST_BACKTRACE": "full" if root == first else "1"}
        write_json_atomic(directory / "invocation.json", row)
        write_json_atomic(
            root / f"verify-evidence/contexts/{ident}.json", {"head": "same-head", "files": files}
        )
    compared = invoke(capsys, "compare", first, second)
    assert compared["differences"][0]["source_changed"] == ["untracked.py"]
    assert compared["differences"][0]["settings_changed"] is True
    assert compared["differences"][0]["context_available"] is True
    assert "unknown" in compared["source_to_binary"]


def test_event_cursor_pins_native_prefix_milestones_and_coverage(tmp_path, capsys, no_execution):
    root = tmp_path / "run"
    directory, invocation = write_run(root)
    report_line(directory, "tests/test_a.py::first")
    report_line(directory, "tests/test_a.py::second")
    first = invoke(capsys, "events", root, "--step", "pytest", "--limit", "1")
    report_line(directory, "tests/test_a.py::new_later")
    invocation["command"] = {"exit": 0, "changed_after_first_page": True}
    invocation["capture"] = {"status": "failed", "reason": "changed after first page"}
    write_json_atomic(directory / "invocation.json", invocation)
    later = invoke(
        capsys, "events", root, "--step", "pytest", "--limit", "100", "--cursor", first["cursor"]
    )
    assert later["total"] == first["total"] == 7
    all_events = first["events"] + later["events"]
    assert next(row for row in all_events if row["event"] == "command")["observation"] == {
        "exit": 1
    }
    assert not any("new_later" in row.get("case", "") for row in all_events)
    assert later["native_coverage"]["case123"]["capture"]["status"] == "available"
    assert invoke(capsys, "events", root, "--step", "pytest", "--limit", "100")["total"] == 8


def test_corrupt_invocation_reports_coverage_without_hiding_other_results(
    tmp_path, capsys, no_execution
):
    root = tmp_path / "run"
    directory, _ = write_run(root)
    report_line(directory, "tests/test_good.py::one")
    corrupt = root / "verify-evidence" / "bad123"
    corrupt.mkdir()
    write_json_atomic(
        corrupt / "invocation.json",
        {
            "schema": 1,
            "id": "bad123",
            "tool": "pytest",
            "boundary": "tooling:python",
            "step": "pytest",
            "command": {"exit": 1},
        },
    )
    (corrupt / "pytest-reportlog.jsonl").write_text('{"malformed":\n')
    manifest = json.loads((root / "verify-evidence.json").read_bytes())
    manifest["invocations"].append({"id": "bad123", "boundary": "tooling:python", "step": "pytest"})
    write_json_atomic(root / "verify-evidence.json", manifest)
    report = invoke(capsys, "report", root)
    assert report["observation_counts"] == {"failed": 1}
    failed_capture = next(row for row in report["invocations"] if row["id"] == "bad123")
    assert failed_capture["coverage"]["status"] == "failed"
    cases = invoke(capsys, "cases", root)
    assert cases["total"] == 1 and cases["cases"][0]["case_key"] == "tests/test_good.py::one"
    assert cases["coverage"]["bad123"]["status"] == "failed"


def test_compare_retains_unknown_context_and_external_config_tool_drift(
    tmp_path, capsys, no_execution
):
    first, second = tmp_path / "first", tmp_path / "second"
    for root, user_digest, tool_version in [
        (first, "old-user", "0.9.146"),
        (second, "new-user", "0.9.147"),
    ]:
        directory, invocation = write_run(root)
        (directory / "pytest-reportlog.jsonl").touch()
        invocation.update(
            effective_config={"repo": "same", "external-user": user_digest},
            tool_version=tool_version,
        )
        write_json_atomic(directory / "invocation.json", invocation)
    compared = invoke(capsys, "compare", first, second)
    assert len(compared["differences"]) == 1
    difference = compared["differences"][0]
    assert difference["context_available"] is False
    assert difference["effective_config_changed"] is True
    assert difference["effective_config_available"] is True
    assert difference["tool_version_changed"] is True


def test_nextest_cargo_failure_survives_without_completed_test_cases(
    tmp_path, capsys, no_execution
):
    root = tmp_path / "build-failure"
    directory, _ = write_run(root, tool="nextest", boundary="model:rust", step="nextest")
    (directory / "nextest-events.jsonl").write_text(
        json.dumps(
            {
                "reason": "compiler-message",
                "package_id": "path+file:///fixture#0.0.0",
                "target": {"name": "fixture", "kind": ["lib"]},
                "message": {
                    "level": "error",
                    "rendered": "error: deliberate fixture build failure",
                },
            }
        )
        + "\n"
    )
    report = invoke(capsys, "report", root)
    assert report["observation_counts"] == {"failed": 1}
    cases = invoke(capsys, "cases", root)
    assert cases["cases"][0]["phase"] == "build"
    assert cases["cases"][0]["event_kind"] == "compiler-message"
    assert cases["cases"][0].get("test") is None


def test_corrupt_committed_metadata_with_unchanged_original_reports_failed_coverage(
    tmp_path, capsys, no_execution
):
    root = tmp_path / "run"
    directory, _ = write_run(root)
    report_line(directory, "tests/test_good.py::one")
    assert invoke(capsys, "cases", root)["total"] == 1
    original = directory / "pytest-reportlog.jsonl"
    original_stat = original.stat()
    checkpoint = json.loads((directory / "projection/checkpoint.json").read_bytes())
    segment = (
        directory / "projection" / checkpoint["generation"] / checkpoint["segments"][0]["path"]
    )
    segment.write_bytes(segment.read_bytes().replace(b"tests/test_good.py", b"tests/test_fake.py"))
    assert original.stat() == original_stat
    report = invoke(capsys, "report", root)
    assert report["invocations"][0]["coverage"]["status"] == "failed"
    assert "metadata segment" in report["invocations"][0]["coverage"]["reason"]
    cases = invoke(capsys, "cases", root)
    assert cases["coverage"]["case123"]["status"] == "failed"
    assert "metadata segment" in cases["coverage"]["case123"]["reason"]


def test_rerun_refuses_hidden_addopts_targets_and_preserves_flags_only(no_execution):
    case = {"case_key": "one", "nodeid": "tests/test_one.py::one[param value]"}
    base = recorded("tooling:python", "pytest", "pytest", ["pytest", "tests/original"])
    for channel in ("configured_addopts", "environment_addopts"):
        invocation = {**base, "selection_options": {channel: ["-q", "tests/hidden.py"]}}
        with pytest.raises(CaptureUnsupported, match="positional targets"):
            rerun.plan(invocation, case, {"options": {}}, [invocation])
    for argv in (
        ["pytest", "tests/original", "-o", "addopts=-q tests/hidden.py"],
        ["pytest", "tests/original", "--override-ini=addopts=tests/hidden.py"],
        ["pytest", "tests/original", "-oaddopts=tests/hidden.py"],
    ):
        invocation = {**base, "argv": argv}
        with pytest.raises(CaptureUnsupported, match="positional targets"):
            rerun.plan(invocation, case, {"options": {}}, [invocation])
    invocation = {
        **base,
        "argv": ["pytest", "tests/original", "-o", "addopts=-q --tb=long"],
        "selection_options": {
            "configured_addopts": ["-k", "wanted"],
            "environment_addopts": ["--show-capture", "all"],
        },
    }
    result = rerun.plan(invocation, case, {"options": {}}, [invocation])
    assert result["executes"] is False
    assert result["operations"][-1]["argv"] == [
        "pytest",
        "-o",
        "addopts=-q --tb=long",
        "--",
        case["nodeid"],
    ]


def test_future_manifest_schema_is_refused_without_readiness_or_execution(
    tmp_path, capsys, no_execution
):
    root = tmp_path / "run"
    directory, _ = write_run(root)
    report_line(directory, "tests/test_one.py::one")
    manifest = json.loads((root / "verify-evidence.json").read_bytes())
    manifest["schema"] = 999
    write_json_atomic(root / "verify-evidence.json", manifest)
    for command in ("report", "cases", "events"):
        assert "unsupported verification evidence manifest schema" in invoke(
            capsys, command, root, expected=2
        )


@pytest.fixture
def export_owner(tmp_path, monkeypatch):
    repository = tmp_path / "registered"
    (repository / ".config").mkdir(parents=True)
    (repository / ".config/storage.toml").write_text(
        'schema=1\n[categories.run-log]\nowner="run"\ntemporary_days=30\n'
        '[categories.run-receipt]\nowner="run"\ntemporary_days=90\n'
        '[categories.environment]\nowner="environment"\ngrace_days=0\n'
    )
    subprocess.run(["git", "init", "-q", str(repository)], check=True)
    host = tmp_path / "host.toml"
    host.write_text(f"schema=1\nrepositories=[{json.dumps(str(repository))}]\n")
    monkeypatch.setenv("LCTX_STORAGE_CONFIG", str(host))
    storage = storage_lifecycle.Storage(root=repository)
    monkeypatch.setattr(cli, "Storage", lambda: storage)
    owner = repository / "build/runs/export-owner"
    owner.mkdir(parents=True)
    record = {
        "schema": 2,
        "id": owner.name,
        "owner": ProcessIdentity.of().to_json(),
        "child": None,
        "termination": None,
        "started": runs.now(),
    }
    write_json_atomic(owner / runs.RECORD, record)
    monkeypatch.setenv("LCTX_RUN_DIR", str(owner))
    lock = hold_lock(owner / runs.OWNER_LOCK)
    try:
        assert runs.current_run() == owner
        yield repository, storage
    finally:
        write_json_atomic(
            owner / runs.RECORD,
            {
                **record,
                "termination": "completed",
                "ended": runs.now(),
                "cleanup": {"status": "confirmed", "observed": runs.now()},
            },
        )
        os.close(lock)


def test_reports_and_exports_retain_original_completion_observation(export_owner, monkeypatch, capsys):
    from executable_observation import observe
    repository, _ = export_owner
    source = repository / "source-run"
    directory, invocation = write_run(source)
    report_line(directory, "tests/test_one.py::one")
    executable = repository / "mutable-executable"
    executable.write_bytes(b"original bytes")
    original = observe(executable)
    invocation["binaries"] = [original]
    write_json_atomic(directory / "invocation.json", invocation)
    executable.write_bytes(b"later bytes")
    actual_open = Path.open
    def guarded(path, *args, **kwargs):
        assert path != executable, "a report must not reopen the later executable path"
        return actual_open(path, *args, **kwargs)
    monkeypatch.setattr(Path, "open", guarded)
    assert invoke(capsys, "report", source)["invocations"][0]["binaries"] == [original]
    bundle = repository / "bundle"
    invoke(capsys, "export", source, "--output", bundle)
    assert invoke(capsys, "report", bundle)["invocations"][0]["binaries"] == [original]


def test_export_relocation_rebuild_and_imported_index_tampering(export_owner, tmp_path, capsys):
    repository, storage = export_owner
    source = repository / "source-run"
    directory, _ = write_run(source)
    report_line(directory, "tests/test_one.py::one", payload="portable output")
    original = invoke(capsys, "cases", source)["cases"][0]
    output = repository / "exported"
    exported = invoke(capsys, "export", source, "--output", output)
    assert storage.get(exported["storage_id"])["owner"]["kind"] == "run"
    obligation = storage.get(exported["storage_id"])["obligations"][exported["consumer"]]
    assert obligation["requires"] == "cited-report"
    moved = repository / "relocated"
    shutil.move(output, moved)
    shutil.rmtree(source)  # Only this control's disposable fixture originals.
    observed = invoke(capsys, "evidence", moved, "--case", original["reference"])
    assert observed["evidence"][0]["text"] == "portable output"
    index = moved / "verify-evidence/case123/projection"
    shutil.rmtree(index)
    assert invoke(capsys, "cases", moved)["cases"][0]["reference"] == original["reference"]
    # An imported checkpoint may carry valid-looking checksums and arbitrary paths.
    # Bundle admission must discard its authority and rebuild from native originals.
    checkpoint = index / "checkpoint.json"
    checkpoint.write_text('{"checksum":"fake","generation":"../../escape"}')
    assert invoke(capsys, "cases", moved)["cases"][0]["reference"] == original["reference"]
    assert list(index.glob("imported-*.json"))


def test_bundle_hmac_marker_cannot_mask_replaced_original(export_owner, capsys):
    repository, _ = export_owner
    source = repository / "source-run"
    directory, _ = write_run(source)
    report_line(directory, "tests/test_one.py::one", payload="original")
    output = repository / "bundle"
    invoke(capsys, "export", source, "--output", output)
    assert (output / ".inspection.json").is_file()
    report = output / "verify-evidence/case123/pytest-reportlog.jsonl"
    stat = report.stat()
    report.write_bytes(report.read_bytes().replace(b"original", b"tampered"))
    os.utime(report, ns=(stat.st_atime_ns, stat.st_mtime_ns))
    error = invoke(capsys, "cases", output, expected=2)
    assert "original digest mismatch" in error


def test_exact_plan_native_parser_reuses_fixture_binaries_without_build(
    tmp_path, native_fixture, monkeypatch
):
    project, repo, _ = native_fixture
    manifest = str(project / "Cargo.toml")
    cargo_metadata = tmp_path / "cargo-metadata.json"
    metadata = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--manifest-path", manifest],
        cwd=ROOT,
        capture_output=True,
    )
    assert metadata.returncode == 0, metadata.stderr.decode()
    cargo_metadata.write_bytes(metadata.stdout)
    binaries = tmp_path / "binaries-metadata.json"
    compiled = subprocess.run(
        [
            "cargo",
            "nextest",
            "list",
            "--manifest-path",
            manifest,
            "--config-file",
            str(repo),
            "--user-config-file",
            "none",
            "--message-format",
            "json",
            "--list-type",
            "binaries-only",
        ],
        cwd=ROOT,
        capture_output=True,
    )
    assert compiled.returncode == 0, compiled.stderr.decode()
    binaries.write_bytes(compiled.stdout)
    original = [
        "cargo",
        "nextest",
        "run",
        "--cargo-metadata",
        str(cargo_metadata),
        "--binaries-metadata",
        str(binaries),
        "--config-file",
        str(repo),
        "--user-config-file",
        "none",
        "--partition=hash:1/1",
        "-Etest(fails)",
        "--filterset",
        "test(passes)",
        "--message-format",
        "libtest-json-plus",
    ]

    def forbidden(*args, **kwargs):
        raise AssertionError("constructing an exact plan must not launch discovery or execution")

    with monkeypatch.context() as patch:
        patch.setattr(subprocess, "run", forbidden)
        planned = rerun.nextest_case(original, {"binary_id": "evidence-native", "test": "passes"})
        escaped = rerun.nextest_case(
            original, {"binary_id": "evidence-native", "test": r"a(b)c,d\e"}
        )
    env = dict(os.environ) | {"NEXTEST_EXPERIMENTAL_LIBTEST_JSON": "1", "CARGO_TERM_COLOR": "never"}
    completed = subprocess.run(planned, cwd=ROOT, env=env, capture_output=True)
    assert completed.returncode == 0, completed.stderr.decode()
    observed = [json.loads(line) for line in completed.stdout.splitlines()]
    finished = [row for row in observed if row.get("type") == "test" and row.get("event") == "ok"]
    assert [row["name"] for row in finished] == ["evidence-native::evidence_native$passes"]
    assert b"Compiling" not in completed.stderr
    no_match = subprocess.run(escaped, cwd=ROOT, env=env, capture_output=True)
    assert no_match.returncode == 4, no_match.stderr.decode()
    assert b"Compiling" not in no_match.stderr


def test_native_bundle_replay_after_source_and_private_store_removal(
    export_owner, native_fixture, capsys
):
    repository, _ = export_owner
    project, repo, user = native_fixture
    source = repository / "native-source"
    directory, invocation = write_run(source, tool="nextest", boundary="model:rust", step="nextest")
    argv = [
        "cargo",
        "nextest",
        "run",
        "--manifest-path",
        str(project / "Cargo.toml"),
        "--config-file",
        str(repo),
        "--user-config-file",
        str(user),
        "-E",
        "test(passes)",
    ]
    prepared = nextest.prepare(argv, dict(os.environ), directory, ROOT)
    with Path(prepared["stdout"]).open("wb") as output:
        completed = subprocess.run(
            prepared["argv"], cwd=ROOT, env=prepared["env"], stdout=output, stderr=subprocess.PIPE
        )
    assert completed.returncode == 0, completed.stderr.decode()
    finalized = nextest.finalize(directory, prepared["env"], ROOT)
    assert finalized["channels"]["native-recording"]["status"] == "available", finalized
    invocation.update(
        argv=argv,
        effective_argv=prepared["argv"],
        artifacts=prepared["artifacts"],
        command={"exit": 0},
        finalization=finalized,
    )
    write_json_atomic(directory / "invocation.json", invocation)
    original = next(
        row
        for row in invoke(capsys, "cases", source)["cases"]
        if row.get("event_kind") == "nextest-junit-attempt" and row["test"] == "passes"
    )
    output = repository / "native-bundle"
    invoke(capsys, "export", source, "--output", output)
    relocated = repository / "native-relocated"
    shutil.move(output, relocated)
    shutil.rmtree(source)  # Removes this control's private native store as well as source reports.
    shutil.rmtree(project)  # A replay must not depend on the original test crate or executable.
    shutil.rmtree(relocated / "verify-evidence/case123/projection")
    evidence = invoke(capsys, "evidence", relocated, "--case", original["reference"])
    assert "passing stdout" in evidence["evidence"][0]["text"]
    recording = relocated / "verify-evidence/case123/nextest-recording.zip"
    env = dict(os.environ) | {"NEXTEST_EXPERIMENTAL_RECORD": "1", "CARGO_TERM_COLOR": "never"}
    info = subprocess.run(
        ["cargo", "nextest", "--no-pager", "store", "info", str(recording)],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    assert info.returncode == 0, info.stderr.decode()
    assert finalized["native_uuid"].encode() in info.stdout
    replay = subprocess.run(
        [
            "cargo",
            "nextest",
            "--no-pager",
            "replay",
            "-R",
            str(recording),
            "--success-output",
            "immediate",
        ],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    assert replay.returncode == 0, replay.stderr.decode()
    assert b"passing stdout" in replay.stdout + replay.stderr


def test_completed_junit_does_not_retire_earlier_partial_json_references(tmp_path):
    directory = tmp_path / "invocation"
    directory.mkdir()
    write_json_atomic(directory / "invocation.json", {"id": "native123", "tool": "nextest"})
    (directory / "nextest-events.jsonl").write_text(
        json.dumps(
            {"type": "test", "event": "started", "name": "evidence-native::evidence_native$passes"}
        )
        + "\n"
    )
    earlier = projection.cases(directory)["cases"][0]
    (directory / "junit.xml").write_text("""<testsuites uuid="4b4ec130-d5d7-4db4-9237-4dd6b7d6303c">
<testsuite name="evidence-native"><testcase name="passes" classname="evidence-native">
<system-out>finished</system-out></testcase></testsuite></testsuites>""")
    now = projection.cases(directory)
    assert {row["event_kind"] for row in now["cases"]} == {
        "nextest-test-started",
        "nextest-junit-attempt",
    }
    assert earlier["reference"] in {row["reference"] for row in now["cases"]}
    observed = projection.evidence(directory, earlier["reference"])
    assert observed["evidence"][0]["outcome"] == "started"
