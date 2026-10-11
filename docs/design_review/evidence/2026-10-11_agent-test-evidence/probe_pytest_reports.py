"""Compare native report semantics without installing plugins or running project tests.

Run using the checkout's Python/pytest. Only --output persists; all synthetic inputs,
downloaded plugin code and raw reports live in a temporary directory owned by this probe.
"""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import urllib.request
import xml.etree.ElementTree as ET
import zipfile

import pytest


URL = (
    "https://files.pythonhosted.org/packages/05/70/"
    "807cbdac584629623c1c2f76b7a39a343e3989510cd52385f1f40581e963/"
    "pytest_reportlog-1.0.0-py3-none-any.whl"
)
SHA256 = "3fc837ef3be6e50f33b52aaf99f88bfbb2ec525febbc3990389d24bbfba28753"
NORMAL = '''import logging, os, pytest, warnings
@pytest.fixture
def broken_setup():
    try:
        raise KeyError("cause-key")
    except KeyError as cause:
        raise ValueError("setup-value") from cause
@pytest.fixture
def broken_teardown():
    yield
    raise RuntimeError("teardown-value")
def test_pass():
    print("passing-stdout")
    os.write(2, b"passing-fd-stderr\\n")
    logging.warning("passing-log")
def test_setup(broken_setup):
    assert False, "unreachable"
def test_call_and_teardown(broken_teardown):
    assert 1 == 2, "call-value"
@pytest.mark.skip(reason="skip-reason")
def test_skip(): pass
@pytest.mark.xfail(reason="xfail-reason")
def test_xfail(): assert False
@pytest.mark.xfail(strict=True, reason="strict-reason")
def test_xpass(): pass
def test_warning(): warnings.warn("warning-value")
'''
INTERRUPT = '''def test_a_pass(): pass
def test_b_interrupt(): raise KeyboardInterrupt()
'''
KILLED = '''import os, pathlib, time
def test_a_pass(): print("retained-passing-output")
def test_b_inflight():
    print("unfinished-call-output", flush=True)
    pathlib.Path(os.environ["PROBE_READY"]).write_text("ready")
    time.sleep(60)
'''


def run_case(root, plugin, name, source, kill=False):
    directory = root / name
    directory.mkdir()
    (directory / "pytest.ini").write_text("[pytest]\n")
    (directory / "test_case.py").write_text(source)
    report = directory / "events.jsonl"
    junit = directory / "junit.xml"
    ready = directory / "ready"
    env = dict(os.environ, PYTEST_DISABLE_PLUGIN_AUTOLOAD="1", PYTHONPATH=str(plugin),
               PROBE_READY=str(ready))
    env.pop("PYTEST_ADDOPTS", None)
    command = [sys.executable, "-m", "pytest", "-c", str(directory / "pytest.ini"),
               "--confcutdir", str(directory), "-p", "pytest_reportlog.plugin",
               "--report-log", str(report), "--junitxml", str(junit),
               "-o", "junit_logging=all", "-o", "log_level=INFO", "--tb=long", "-q",
               str(directory / "test_case.py")]
    with (directory / "console.log").open("wb") as log:
        child = subprocess.Popen(command, cwd=directory, env=env,
                                 stdout=log, stderr=subprocess.STDOUT)
        try:
            if kill:
                deadline = time.monotonic() + 10
                while not ready.exists() and child.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.01)
                assert ready.exists(), "synthetic child never reached its inflight call"
                child.terminate()
            exit_code = child.wait(timeout=15)
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()
    events = [json.loads(line) for line in report.read_text().splitlines()]
    rows = ET.parse(junit).findall(".//testcase") if junit.exists() else []
    cases = [event for event in events if event.get("$report_type") == "TestReport"]
    return {
        "exit": exit_code,
        "event_types": sorted({event["$report_type"] for event in events}),
        "phase_outcomes": [{key: event.get(key) for key in ("nodeid", "when", "outcome", "wasxfail")}
                           for event in cases],
        "session_finished": any(event["$report_type"] == "SessionFinish" for event in events),
        "junit_exists": junit.exists(),
        "junit_names": [row.get("name") for row in rows],
        "junit_failures": len([row for row in rows if row.find("failure") is not None]),
        "junit_errors": len([row for row in rows if row.find("error") is not None]),
        "passing_sections": [section[0] for event in cases if "::test_pass" in event["nodeid"]
                             for section in event.get("sections", [])],
        "exception_chain_retained": all(text in json.dumps(events)
                                        for text in ("cause-key", "setup-value")),
        "unfinished_output_retained": "unfinished-call-output" in json.dumps(events),
        "has_test_start_event": any("start" in event["$report_type"].lower()
                                    and event["$report_type"] != "SessionStart" for event in events),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    wheel = urllib.request.urlopen(URL).read()
    assert hashlib.sha256(wheel).hexdigest() == SHA256
    with tempfile.TemporaryDirectory(prefix="lctx-test-evidence-probe-") as scratch:
        root = Path(scratch)
        plugin = root / "plugin"
        with zipfile.ZipFile(io.BytesIO(wheel)) as archive:
            archive.extractall(plugin)
        cases = {
            "normal": run_case(root, plugin, "normal", NORMAL),
            "collection": run_case(root, plugin, "collection", "def test_invalid(: pass\n"),
            "interrupt": run_case(root, plugin, "interrupt", INTERRUPT),
            "killed": run_case(root, plugin, "killed", KILLED, kill=True),
        }
    assert cases["normal"]["exit"] == 1
    assert cases["normal"]["junit_names"].count("test_call_and_teardown") == 2
    assert cases["normal"]["exception_chain_retained"]
    assert "Captured stderr call" in cases["normal"]["passing_sections"]
    assert cases["collection"]["exit"] == 2
    assert cases["interrupt"]["exit"] == 2 and cases["interrupt"]["session_finished"]
    assert cases["killed"]["exit"] == -15
    assert not cases["killed"]["session_finished"] and not cases["killed"]["junit_exists"]
    assert not cases["killed"]["unfinished_output_retained"]
    result = {"python": sys.version.split()[0], "pytest": pytest.__version__,
              "reportlog": "1.0.0", "wheel_sha256": SHA256,
              "scope": "synthetic report-format capability controls; no project tests", "cases": cases}
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"outcome": "passed", "scenarios": list(cases), "output": str(args.output)}))


if __name__ == "__main__":
    main()
