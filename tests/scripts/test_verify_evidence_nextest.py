"""Small native controls and independent malformed/attempt fixtures for Nextest capture."""

from __future__ import annotations

import json
import os
import signal
import subprocess
import time
import tomllib
from pathlib import Path

import pytest

from verify_evidence import config_helper, nextest
from verify_evidence.contracts import CaptureUnsupported

ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def native_project(tmp_path):
    project = tmp_path / "fixture"
    project.mkdir()
    (project / "Cargo.toml").write_text(
        '[package]\nname="evidence-native"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[lib]\npath="lib.rs"\n'
    )
    (project / "lib.rs").write_text("""
#[test] fn passes() { println!("passing stdout"); eprintln!("passing stderr"); }
#[test] fn fails() { println!("failing stdout"); panic!("deliberate native failure"); }
#[test] fn flaky() {
    if std::env::var("NEXTEST_ATTEMPT").unwrap_or_default() == "1" {
        panic!("deliberate first attempt");
    }
    println!("successful retry");
}
#[test] #[ignore = "deliberate ignored control"] fn ignored() {}
#[test] fn output_bytes() {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(b"head\\x00\\xff").unwrap();
    stdout.write_all(&vec![b'x'; 8192]).unwrap();
    stdout.write_all(b"tail").unwrap();
}
#[test] fn interruptible() { std::thread::sleep(std::time::Duration::from_secs(30)); }
""")
    repo = tmp_path / "repo.toml"
    repo.write_text("""[profile.default]
fail-fast = false
retries = 1
flaky-result = "fail"
[[profile.default.overrides]]
filter = "test(flaky)"
retries = 1
[profile.default.overrides.junit]
store-success-output = false
""")
    user = tmp_path / "user.toml"
    user.write_text("""[ui]
show-progress = "none"
[record]
enabled = false
max-age = "1s"
[[overrides]]
platform = "cfg(all())"
[overrides.record]
enabled = false
max-output-size = "100B"
max-total-size = "1B"
""")
    return project, repo, user


def prepared(tmp_path, native_project, *extra):
    project, repo, user = native_project
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
        *extra,
    ]
    env = dict(os.environ)
    env["CARGO_TERM_COLOR"] = "never"
    return nextest.prepare(argv, env, tmp_path / "capture", ROOT)


def execute(preparation):
    with Path(preparation["stdout"]).open("wb") as output:
        return subprocess.run(
            preparation["argv"],
            cwd=ROOT,
            env=preparation["env"],
            stdout=output,
            stderr=subprocess.PIPE,
        )


def test_toml_composition_preserves_policy_and_field_precedence(tmp_path):
    original_repo = """[store]
dir = "target/custom"
[profile.default]
retries = 7
test-threads = "num-cpus"
slow-timeout = { period = "60s", terminate-after = 6 }
[[profile.default.overrides]]
filter = "test(a)"
retries = 4
[profile.ci]
inherits = "default"
fail-fast = true
"""
    original_user = """[ui]
show-progress = "bar"
[[overrides]]
platform = "cfg(all())"
[overrides.ui]
show-progress = "counter"
[overrides.record]
enabled = false
max-total-size = "1B"
"""
    result = config_helper.compose(
        {
            "repo_text": original_repo,
            "user_text": original_user,
            "profile": "ci",
            "junit": str(tmp_path / "junit.xml"),
            "directory": str(tmp_path),
        }
    )
    repo = tomllib.loads(Path(result["repo"]).read_text())
    user = tomllib.loads(Path(result["user"]).read_text())
    original = tomllib.loads(original_repo)
    assert repo["profile"]["default"] == original["profile"]["default"]
    assert repo["store"] == original["store"]
    assert repo["profile"]["ci"]["inherits"] == "default"
    assert repo["profile"]["ci"]["fail-fast"] is True
    assert repo["profile"]["ci"]["overrides"][0] == {
        "filter": "all()",
        "junit": config_helper.JUNIT,
    }
    assert user["overrides"][1:] == tomllib.loads(original_user)["overrides"]
    assert user["ui"] == {"show-progress": "bar"}
    assert user["overrides"][0]["record"] == config_helper.RECORD


def test_prepare_is_immutable_and_preserves_argv_grammar(tmp_path, native_project):
    project, repo, user = native_project
    argv = [
        "cargo",
        "nextest",
        "run",
        "--config-file=" + str(repo),
        "--user-config-file=" + str(user),
        "--manifest-path",
        str(project / "Cargo.toml"),
        "--message-format=human",
        "--cargo-message-format",
        "human",
        "--",
        "--exact",
        "passes",
    ]
    env = dict(os.environ) | {"NEXTEST_STATE_DIR": "/do/not/use", "NEXTEST_RETRIES": "3"}
    before_argv, before_env = list(argv), dict(env)
    result = nextest.prepare(argv, env, tmp_path / "capture", ROOT)
    assert argv == before_argv and env == before_env
    assert result["argv"][-3:] == ["--", "--exact", "passes"]
    assert result["env"]["NEXTEST_RETRIES"] == "3"
    assert result["env"]["NEXTEST_STATE_DIR"] == str(tmp_path / "capture/nextest-state")
    assert result["argv"].count("--cargo-message-format") == 1
    assert "--message-format=human" not in result["argv"]
    with pytest.raises(CaptureUnsupported, match="already been used"):
        nextest.prepare(argv, env, tmp_path / "capture", ROOT)


@pytest.mark.parametrize(
    "argument", ["--no-run", "--no-capture", "--stress-count=2", "--debugger=gdb"]
)
def test_unsupported_semantics_are_refused_before_creation(tmp_path, argument):
    with pytest.raises(CaptureUnsupported):
        nextest.prepare(["cargo", "nextest", "run", argument], {}, tmp_path / "capture", ROOT)
    assert not (tmp_path / "capture").exists()


def test_junit_native_attempt_order_flaky_failure_and_distinct_binaries(tmp_path):
    (tmp_path / "junit.xml").write_text("""<testsuites uuid="4b4ec130-d5d7-4db4-9237-4dd6b7d6303c">
<testsuite name="pkg::a"><testcase name="same" classname="pkg::a" time="0.25">
<failure type="test failure">first</failure><rerunFailure time="0.5">second</rerunFailure>
<system-out>first output</system-out></testcase></testsuite>
<testsuite name="pkg::b"><testcase name="same" classname="pkg::b">
<failure type="flaky failure" message="policy">successful last attempt</failure>
<flakyFailure>failed first attempt</flakyFailure><system-out>last output</system-out>
</testcase><testcase name="skip" classname="pkg::b"><skipped message="ignored"/></testcase>
</testsuite></testsuites>""")
    records = list(nextest.records(tmp_path))
    assert [(r["test"], r["attempt"], r["outcome"]) for r in records] == [
        ("same", 1, "failed"),
        ("same", 2, "failed"),
        ("same", 1, "failed"),
        ("same", 2, "passed"),
        ("skip", 1, "skipped"),
    ]
    assert records[0]["case_key"] != records[2]["case_key"]
    assert [row["duration"] for row in records[:2]] == [0.25, 0.5]
    assert "first output" in records[0]["payload"]
    assert "second" in records[1]["payload"]
    assert "failed first attempt" in records[2]["payload"]
    assert records[3]["case_outcome"] == "failed"


def test_libtest_prefix_exposes_identity_and_retry_information_loss(tmp_path):
    records = [
        {
            "type": "suite",
            "event": "started",
            "nextest": {"crate": "p", "test_binary": "a", "kind": "test"},
        },
        {"type": "test", "event": "started", "name": "p::a$same"},
        {
            "type": "test",
            "event": "failed",
            "name": "p::a$same#3",
            "stdout": "last failure",
            "exec_time": 0.75,
        },
    ]
    (tmp_path / "nextest-events.jsonl").write_bytes(
        ("\n".join(map(json.dumps, records)) + '\n{"type":"test"').encode()
    )
    observed = list(nextest.records(tmp_path))
    assert len(observed) == 2
    assert observed[0]["outcome"] == "started"
    assert observed[0]["duration"] is None
    assert observed[1]["duration"] == 0.75
    assert observed[1]["attempt"] == 3
    assert observed[1]["binary_id"] is None
    assert observed[1]["native"]["binary_identity"] == "unavailable"


def test_json_append_starts_at_watermark_and_honors_snapshot_end(tmp_path):
    path = tmp_path / "nextest-events.jsonl"
    first = json.dumps({"type": "test", "event": "started", "name": "p::a$one"}) + "\n"
    second = json.dumps({"type": "test", "event": "ok", "name": "p::a$one"}) + "\n"
    third = json.dumps({"type": "test", "event": "ok", "name": "p::a$two"}) + "\n"
    path.write_text(first + second + third)
    initial = list(nextest.records(tmp_path, ends={path.name: len(first)}))
    appended = list(
        nextest.records(
            tmp_path, watermarks={path.name: len(first)}, ends={path.name: len(first + second)}
        )
    )
    assert len(initial) == len(appended) == 1
    assert initial[0]["case_key"] == appended[0]["case_key"]
    assert appended[0]["source_start"] == len(first)
    assert appended[0]["source_end"] == len(first + second)


def test_native_uuid_mismatch_preserves_junit_observation(tmp_path, native_project, monkeypatch):
    preparation = prepared(tmp_path, native_project)
    (tmp_path / "capture/junit.xml").write_text(
        '<testsuites uuid="4b4ec130-d5d7-4db4-9237-4dd6b7d6303c"/>'
    )
    responses = iter(
        ["1 recorded run:\n  11223344  row\n", "Run 11223344-5566-4788-9922-334455667788\n"]
    )
    monkeypatch.setattr(nextest, "_native", lambda *args: next(responses))
    result = nextest.finalize(tmp_path / "capture", preparation["env"], ROOT)
    assert result["channels"]["junit"]["status"] == "available"
    assert result["channels"]["native-recording"]["status"] == "failed"
    assert "disagrees" in result["channels"]["native-recording"]["reason"]


def test_malformed_completed_xml_is_not_reinterpreted_as_empty_success(tmp_path):
    (tmp_path / "junit.xml").write_text(
        '<testsuites uuid="4b4ec130-d5d7-4db4-9237-4dd6b7d6303c"><testcase'
    )
    with pytest.raises(nextest.ET.ParseError):
        list(nextest.records(tmp_path))


def test_native_exact_filterset_grammar(tmp_path, native_project):
    preparation = prepared(
        tmp_path, native_project, "-E", "binary_id(=evidence-native) & test(=passes)"
    )
    completed = execute(preparation)
    assert completed.returncode == 0, completed.stderr.decode()
    observations = [
        r
        for r in nextest.records(tmp_path / "capture")
        if r["event_kind"] == "nextest-junit-attempt"
    ]
    assert [r["test"] for r in observations if r["outcome"] != "skipped"] == ["passes"]


def test_native_literal_filterset_metacharacters_parse(tmp_path, native_project):
    # Opening parentheses are ordinary characters; closing parentheses, commas and
    # backslashes use the native equality matcher's documented escapes.
    preparation = prepared(tmp_path, native_project, "-E", r"test(=a(b\)c\,d\\e)")
    completed = execute(preparation)
    assert completed.returncode == 4, completed.stderr.decode()


def test_native_complete_recording_exports_and_matches_junit(tmp_path, native_project):
    preparation = prepared(tmp_path, native_project, "-E", "not test(interruptible)")
    completed = execute(preparation)
    assert completed.returncode == 100, completed.stderr.decode()
    result = nextest.finalize(tmp_path / "capture", preparation["env"], ROOT)
    assert result["status"] == "available", result
    assert result["channels"]["junit"]["uuid"] == result["native_uuid"]
    assert Path(result["channels"]["native-recording"]["path"]).stat().st_size > 0
    assert result["channels"]["chrome-trace"]["status"] == "available"
    observations = [
        r
        for r in nextest.records(tmp_path / "capture")
        if r["event_kind"] == "nextest-junit-attempt"
    ]
    flaky = [r for r in observations if r["test"] == "flaky"]
    assert [(r["attempt"], r["outcome"], r["case_outcome"]) for r in flaky] == [
        (1, "failed", "failed"),
        (2, "passed", "failed"),
    ]
    assert "successful retry" in flaky[1]["payload"]
    assert all(r["binary_id"] == "evidence-native" for r in observations)
    assert any("passing stdout" in r["payload"] for r in observations)
    assert any(r["outcome"] == "skipped" for r in observations)
    # The retained private candidate survives ordinary finalization despite an operator
    # override that disables recording and otherwise self-prunes at a single byte.
    effective_user = (tmp_path / "capture/nextest-user.toml").read_text()
    assert 'max-total-size = "18446744073709551615"' in effective_user


def test_native_self_pruning_is_revealed_without_managed_retention(tmp_path, native_project):
    preparation = prepared(tmp_path, native_project, "-E", "test(passes)")
    config = tmp_path / "capture/nextest-user.toml"
    config.write_text(
        config.read_text().replace(
            'max-total-size = "18446744073709551615"', 'max-total-size = "1B"'
        )
    )
    completed = execute(preparation)
    assert completed.returncode == 0, completed.stderr.decode()
    result = nextest.finalize(tmp_path / "capture", preparation["env"], ROOT)
    assert result["channels"]["junit"]["status"] == "available"
    assert result["channels"]["native-recording"]["status"] == "failed"
    assert "0 candidates" in result["channels"]["native-recording"]["reason"]


def test_native_truncation_and_xml_normalization_have_distinct_coverage(tmp_path, native_project):
    preparation = prepared(tmp_path, native_project, "-E", "test(output_bytes)")
    config = tmp_path / "capture/nextest-user.toml"
    config.write_text(
        config.read_text().replace('max-output-size = "256MiB"', 'max-output-size = "1KiB"')
    )
    completed = execute(preparation)
    assert completed.returncode == 0, completed.stderr.decode()
    result = nextest.finalize(tmp_path / "capture", preparation["env"], ROOT)
    assert result["channels"]["native-recording"]["status"] == "available", result
    observations = list(nextest.records(tmp_path / "capture"))
    actual = next(r for r in observations if r.get("test") == "output_bytes")
    assert "x" * 8192 in actual["payload"]
    assert "\ufffd" in actual["payload"] and "\0" not in actual["payload"]
    replay = subprocess.run(
        [
            "cargo",
            "nextest",
            "--no-pager",
            "replay",
            "--run-id",
            result["channels"]["native-recording"]["path"],
            "--success-output",
            "immediate",
            "--manifest-path",
            str(native_project[0] / "Cargo.toml"),
            "--config-file",
            str(tmp_path / "capture/nextest-repo.toml"),
            "--user-config-file",
            str(config),
        ],
        cwd=ROOT,
        env=preparation["env"],
        capture_output=True,
    )
    assert replay.returncode == 0, replay.stderr.decode(errors="replace")
    rendered = replay.stdout + replay.stderr
    assert b"truncated" in rendered.lower()
    assert b"x" * 8192 not in rendered


def test_native_interrupt_survives_and_exports(tmp_path, native_project):
    preparation = prepared(tmp_path, native_project, "-E", "test(interruptible)")
    with Path(preparation["stdout"]).open("wb") as output:
        child = subprocess.Popen(
            preparation["argv"],
            cwd=ROOT,
            env=preparation["env"],
            stdout=output,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        try:
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                if b'"event":"started","name":' in Path(preparation["stdout"]).read_bytes():
                    break
                assert child.poll() is None
                time.sleep(0.02)
            else:
                pytest.fail("native fixture did not start")
            child.send_signal(signal.SIGINT)
            _, stderr = child.communicate(timeout=15)
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
    assert child.returncode != 0, stderr.decode()
    result = nextest.finalize(tmp_path / "capture", preparation["env"], ROOT)
    assert result["channels"]["native-recording"]["status"] == "available", result
    assert result["native_uuid"] is not None
