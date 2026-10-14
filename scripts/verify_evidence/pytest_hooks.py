"""Small supplements to native reportlog, without another test-result reporter."""

from __future__ import annotations

import json
import os
import shlex
from pathlib import Path

import pytest


class Observations:
    def __init__(self, path: Path):
        self.stream = path.open("w", encoding="utf-8", buffering=1)
        self.deselected: set[str] = set()

    def emit(self, kind: str, **fields):
        self.stream.write(json.dumps({"$report_type": kind, **fields}, ensure_ascii=False) + "\n")
        self.stream.flush()

    def pytest_collection_finish(self, session):
        for item in session.items:
            self.emit(
                "CollectionMember", nodeid=item.nodeid, location=item.location, outcome="selected"
            )
        # session.testscollected is assigned *after* this hook in pytest9.1.1.
        # This observes completion of collection hooks, not collection success.
        self.emit(
            "CollectionFinished",
            selected=len(session.items),
            deselected=len(self.deselected),
            outcome="observed",
        )

    def pytest_deselected(self, items):
        for item in items:
            self.deselected.add(item.nodeid)
            self.emit(
                "Deselected", nodeid=item.nodeid, location=item.location, outcome="deselected"
            )

    def pytest_runtest_logstart(self, nodeid, location):
        self.emit("CaseStart", nodeid=nodeid, location=location, outcome="observed")

    def pytest_sessionfinish(self, exitstatus):
        self.emit("SessionFinish", exitstatus=int(exitstatus))


@pytest.hookimpl(tryfirst=True)
def pytest_configure(config):
    path = os.environ.get("LCTX_PYTEST_EVIDENCE_HOOKS")
    if not path:
        return
    unsupported = None
    if getattr(config.option, "numprocesses", None) or hasattr(config, "workerinput"):
        unsupported = "xdist"
    if getattr(config.option, "reruns", None) or getattr(config.option, "force_reruns", None):
        unsupported = "retry plugins"
    user_addopts = [*config.getini("addopts"), *shlex.split(os.environ.get("PYTEST_ADDOPTS", ""))]
    if any(arg == "--report-log" or arg.startswith("--report-log=") for arg in user_addopts):
        unsupported = "explicit report-log destination conflict"
    observer = Observations(Path(path))
    config._lctx_evidence_observer = observer
    if unsupported:
        # An unanticipated late configuration can reduce capture coverage, but
        # must not change the actual tests' result or execution policy.
        observer.emit("CaptureUnsupported", reason=unsupported)
        return
    args = [
        *config.getini("addopts"),
        *shlex.split(os.environ.get("PYTEST_ADDOPTS", "")),
        *config.invocation_params.args,
    ]
    if not any(arg == "--tb" or arg.startswith("--tb=") for arg in args):
        config.option.tbstyle = "long"
    # pytest owns fd capture, warnings and log attachment. Supply a useful log
    # threshold only when CLI/config did not choose one; preserve exclusion flags.
    if config.option.log_level is None and not config.getini("log_level"):
        config.option.log_level = "INFO"
    config.pluginmanager.register(observer, "lctx-evidence-observations")
    observer.emit(
        "CaptureConfiguration",
        cwd=str(config.invocation_params.dir),
        rootpath=str(config.rootpath),
        configfile=str(config.inipath) if config.inipath else None,
        capture=config.option.capture,
        traceback=config.option.tbstyle,
        log_level=config.option.log_level or config.getini("log_level"),
        passing_logs=not getattr(config.option, "report_log_exclude_logs_on_passed_tests", False),
        warnings_plugin=config.pluginmanager.hasplugin("warnings"),
        plugin_autoload_disabled=bool(os.environ.get("PYTEST_DISABLE_PLUGIN_AUTOLOAD"))
        or getattr(config.option, "disable_plugin_autoload", False),
    )


def pytest_unconfigure(config):
    observer = getattr(config, "_lctx_evidence_observer", None)
    if observer:
        observer.stream.close()
        if config.pluginmanager.is_registered(observer):
            config.pluginmanager.unregister(observer)
        del config._lctx_evidence_observer
