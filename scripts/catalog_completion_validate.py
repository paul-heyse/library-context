"""Run the remaining Catalog completion scope under existing run and campaign owners.

Preview: uv run --no-sync python scripts/catalog_completion_validate.py --print
Execute: just run --background --label catalog-completion-validation -- \
    uv run --no-sync python scripts/catalog_completion_validate.py

The enclosing run retains summary.json/summary.md and every child receipt/log. Each disposable
batch gets a fresh lifecycle campaign; FastMCP target B and its private publication are retained.
This is the selected completion scope, not test-all/qualify or an operator activation route.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import shutil
import signal
import sys
import time
import uuid
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import runs
from build_environment import normalized_env
from harness import SpawnGuard, observe_exit, read_json, spawn_group, write_json_atomic
from storage_lifecycle import Storage, admission

ROOT = Path(__file__).resolve().parents[1]
STATE = Path("/home/paul/.local/state/library-context")
RETAINED = STATE / "catalog-completion-retained-fastmcp-20261014-b/installation.json"
RETAINED_ID = "a4e56c59-5bc6-446b-a611-3c2c39dd655f"
SERVER = STATE / (
    "surrealdb.tools/surreal/"
    "405f6c02ddfe12c004b04deab6bd91fd1458ed1c3ba247c05baa4a5396d5e206/record.json"
)
TARGET = Path(normalized_env(dict(os.environ), ROOT).get("CARGO_TARGET_DIR", ROOT / "target"))
LCTX = TARGET / "release/lctx"
COMPOSITION = (
    "test(scoped_summary::actual_summary_matches_complete_finite_whole_operation) | "
    "test(scoped_summary::actual_recursive_summary_preserves_scc_delta_answers_and_exhaustion) | "
    "test(summary_publication::catalog_summaries_are_explicitly_not_requested) | "
    "test(=current_catalog_analysis_links_public_native_operations_and_original_evidence)"
)
COLD = "test(=facts_backup_cold_concurrent_restores_preserve_originals_and_fresh_ownership)"
MUTATIONS = (
    "test(materialize_search_then_mutate_document_seal_refuses) | "
    "test(materialize_search_then_mutate_lexical_seal_refuses)"
)
ANALYZER = "test(materialize_search_then_change_actual_analyzer_seal_refuses)"
EPOCH = "test(boundary_final_live_hold_refuses_epoch_drift_after_local_context_reuse)"


def nextest(
    packages: Sequence[str], targets: Sequence[str], selection: str, *, ignored: bool = False
) -> list[str]:
    command = [
        "cargo",
        "nextest",
        "run",
        "--release",
        "--locked",
        "--no-fail-fast",
        "--no-tests=fail",
    ]
    for package in packages:
        command.extend(["-p", package])
    command.extend(targets)
    if ignored:
        command.extend(["--run-ignored", "only"])
    return [*command, "-E", selection]


@dataclass(frozen=True)
class Batch:
    name: str
    command: list[str]
    maintenance: bool = False


BATCHES = (
    Batch(
        "composition",
        nextest(
            ["cpg-core", "lctx-serving"],
            ["--test", "compiler_artifacts", "--test", "compact_analysis_native"],
            COMPOSITION,
        ),
    ),
    Batch("cold", nextest(["lctx-publisher"], ["--test", "publication"], COLD)),
    Batch(
        "document-lexical",
        nextest(["lctx-publisher"], ["--test", "publication"], MUTATIONS, ignored=True),
        True,
    ),
    Batch(
        "analyzer",
        nextest(["lctx-publisher"], ["--test", "publication"], ANALYZER, ignored=True),
        True,
    ),
    Batch("epoch", nextest(["lctx-native"], ["--lib"], EPOCH, ignored=True), True),
)
QUALITY = (
    (
        "workspace-check",
        ["cargo", "check", "--release", "--workspace", "--all-targets", "--locked", "--keep-going"],
    ),
    (
        "clippy",
        [
            "cargo",
            "clippy",
            "--release",
            "--workspace",
            "--all-targets",
            "--locked",
            "--keep-going",
            "--",
            "-D",
            "warnings",
        ],
    ),
    ("ruff", ["uv", "run", "--no-sync", "ruff", "check"]),
    (
        "pyrefly",
        ["uv", "run", "--no-sync", "pyrefly", "check", "--summary=none", "--min-severity", "warn"],
    ),
    *(
        (name, ["just", name])
        for name in ("deps", "docs-check", "adr-lint", "fixtures-check", "rules-scan", "rules-test")
    ),
    ("rust-fmt", ["cargo", "fmt", "--all", "--", "--check"]),
    ("python-fmt", ["uv", "run", "--no-sync", "ruff", "format", "--check"]),
)


def fixture_command(batch: Batch) -> list[str]:
    command = ["just", "fixture", "--requires", "none", "--", *batch.command]
    return (
        ["just", "service", "maintenance", "--native-clients", "--", *command]
        if batch.maintenance
        else command
    )


def install_command() -> list[str]:
    return [
        "just",
        "service",
        "install",
        "--campaign",
        "--purpose",
        "surrealdb-lifecycle",
        "--installer",
        str(LCTX),
        "--server-generation",
        str(SERVER),
    ]


def pilot_command(mode: str, output: Path) -> list[str]:
    return [
        "just",
        "fixture",
        "--requires",
        "native-python",
        "--retain-serving" if mode == "produce" else "--serving",
        "fastmcp-v03",
        "--",
        "uv",
        "run",
        "--no-sync",
        "python",
        "scripts/catalog_pilot.py",
        mode,
        "--lctx",
        str(LCTX),
        "--output",
        str(output),
    ]


def plan(run: Path) -> list[dict[str, Any]]:
    steps = [
        {"name": "build-cli", "command": ["cargo", "build", "--release", "--locked", "-p", "lctx"]},
        {
            "name": "build-controls",
            "command": [
                "cargo",
                "nextest",
                "run",
                "--release",
                "--locked",
                "--no-run",
                "-p",
                "cpg-core",
                "-p",
                "lctx-serving",
                "-p",
                "lctx-publisher",
                "-p",
                "lctx-native",
                "--lib",
                "--test",
                "compiler_artifacts",
                "--test",
                "compact_analysis_native",
                "--test",
                "publication",
            ],
        },
        {"name": "ready", "command": ["just", "ready"]},
    ]
    for batch in BATCHES:
        steps.extend(batch_plan(batch, run))
    for mode in ("produce", "check"):
        steps.append(
            {
                "name": f"pilot-{mode}",
                "command": pilot_command(mode, run / "fastmcp-v03"),
                "service_config": str(RETAINED),
            }
        )
    steps.extend({"name": name, "command": command} for name, command in QUALITY)
    return steps


def batch_plan(batch: Batch, run: Path) -> list[dict[str, Any]]:
    config = STATE / f"catalog-completion-validation-{run.name}-{batch.name}/installation.json"
    return [
        {
            "name": f"{batch.name}-install",
            "command": install_command(),
            "service_config": str(config),
        },
        {"name": batch.name, "command": fixture_command(batch), "service_config": str(config)},
        {
            "name": f"{batch.name}-dispose",
            "command": [
                "just",
                "service",
                "dispose-campaign",
                "<EXACT_INSTALLATION_UUID>",
                "--participant-run",
                "<EXACT_INSTALL_RUN>",
                "--participant-run",
                "<EXACT_BATCH_RUN>",
            ],
            "service_config": str(config),
        },
    ]


def outcome(exit_code: int | None, cleanup: dict[str, Any], terminal: str | None) -> str:
    if cleanup.get("status") != "confirmed" or terminal != "completed":
        return "failed"
    return "passed" if exit_code == 0 else "blocked" if exit_code == 75 else "failed"


def child_environment(source: dict[str, str], config: str | None) -> dict[str, str]:
    environment = normalized_env(source, ROOT)
    for key in list(environment):
        if key.startswith(("LCTX_SURREAL_", "LCTX_FIXTURE_", "LCTX_NATIVE_")) or key in {
            "LCTX_COMPILER_RUNTIME_CONFIG",
            "LCTX_RETAIN_NATIVE_FIXTURE_CONFIG",
            "LCTX_PD02_RECOVERY_DIRECTORY",
        }:
            environment.pop(key)
    environment["INSTA_UPDATE"] = "no"
    if config:
        environment["LCTX_SURREAL_SERVICE_CONFIG"] = config
        environment.setdefault("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
        environment.setdefault(
            "DBUS_SESSION_BUS_ADDRESS", f"unix:path={environment['XDG_RUNTIME_DIR']}/bus"
        )
    return environment


def campaign_identity(config: Path) -> str:
    """Consume only our fresh installation descriptor; never adopt or dispose target B."""
    value = read_json(config)
    if (
        config.resolve() == RETAINED.resolve()
        or not isinstance(value, dict)
        or value.get("kind") != "campaign"
        or value.get("purpose") != "surrealdb-lifecycle"
        or value.get("state_root") != str(config.parent)
        or value.get("binary", {}).get("generation", {}).get("identity") != SERVER.parent.name
    ):
        raise ValueError("fresh campaign identity unavailable or incompatible; target retained")
    identity = str(uuid.UUID(value["installation_id"]))
    if identity == RETAINED_ID:
        raise ValueError("target B is retained and cannot be disposed")
    return identity


class Validation:
    def __init__(self, run: Path):
        self.run = run
        self.consumer = f"catalog-completion-validation:{run.name}"
        self.steps = [{**step, "outcome": "not_run", "reason": "not reached"} for step in plan(run)]
        self.stopped = False
        self.error: str | None = None
        self.save()

    def save(self) -> None:
        result = {
            "scope": "remaining Catalog completion validation",
            "run": str(self.run),
            "consumer": self.consumer,
            "retained_target": str(RETAINED),
            "stopped": self.stopped,
            "error": self.error,
            "steps": self.steps,
        }
        write_json_atomic(self.run / "summary.json", result)
        lines = [
            "Catalog completion validation",
            "",
            f"Run: {self.run}",
            f"Retained FastMCP target: {RETAINED} (never disposed)",
            "",
            "Test bodies, fixture exit, host cleanup and native finality remain separate.",
            "Checked disposal settles only owned target retention, never a failed test.",
            "",
        ]
        for step in self.steps:
            lines.append(f"- {step['outcome']}: {step['name']} — {step.get('reason', '')}")
            if step.get("run"):
                lines.append(f"  Run/log: {step['run']} / {step['log']}")
        if self.error:
            lines.extend(["", self.error])
        (self.run / "summary.md").write_text("\n".join(lines) + "\n")

    def row(self, name: str) -> dict[str, Any]:
        return next(step for step in self.steps if step["name"] == name)

    def blocked(self, name: str, reason: str) -> bool:
        self.row(name).update(outcome="blocked", reason=reason)
        self.save()
        return False

    def execute(self, name: str, command: list[str] | None = None, *, junit: bool = False) -> bool:
        if self.stopped:
            raise InterruptedError("validation was cancelled; no further commands launched")
        try:
            return self._execute(name, command, junit=junit)
        except InterruptedError:
            self.stopped = True
            raise
        except Exception as error:
            self.row(name).update(outcome="failed", reason=f"{type(error).__name__}: {error}")
            self.save()
            print(f"{name}: failed: {error}", flush=True)
            return False

    def _execute(self, name: str, command: list[str] | None = None, *, junit: bool = False) -> bool:
        child_run = runs.new_run_dir()
        with admission([child_run]):
            return self._execute_owned(name, command, child_run, junit=junit)

    def _execute_owned(
        self, name: str, command: list[str] | None, child_run: Path, *, junit: bool
    ) -> bool:
        step = self.row(name)
        command = step["command"] if command is None else command
        step.update(
            command=command,
            run=str(child_run),
            log=str(child_run / "output.log"),
            outcome="running",
            reason="",
        )
        self.save()
        runs.set_progress(current_command=name)
        print(f"{name}: {shlex.join(command)}\n  run/log: {child_run}", flush=True)
        config = step.get("service_config")
        environment = child_environment(dict(os.environ), config)
        attachment_root = Path(config).parent / "attachments" if config else None
        before = set(attachment_root.glob("*/record.json")) if attachment_root else set()
        target = Path(environment.get("CARGO_TARGET_DIR", str(ROOT / "target")))
        junit_source = target / "nextest/default/junit.xml"
        previous_junit = junit_source.stat() if junit and junit_source.exists() else None
        guard = None
        try:
            with (child_run / "launcher.log").open("ab") as log:
                child = spawn_group(
                    [
                        sys.executable,
                        str(ROOT / "scripts/runs.py"),
                        "_own",
                        "--dir",
                        str(child_run),
                        "--label",
                        f"catalog-completion-{name}",
                        "--",
                        *command,
                    ],
                    cwd=ROOT,
                    env=environment,
                    stdout=log,
                    stderr=log,
                    death_signal=signal.SIGTERM,
                )
                guard = SpawnGuard(child)
                with guard:
                    try:
                        while observe_exit(child) is None:
                            time.sleep(0.1)
                    except InterruptedError:
                        self.stopped = True
                        step["cancellation"] = runs.cancel(child_run)
                        raise
        except InterruptedError:
            self.stopped = True
            if step.get("cancellation", {}).get("host_cleanup", {}).get("status") != "confirmed":
                step["cancellation_recovery"] = runs.cancel(child_run)
            raise
        except OSError as error:
            step["reason"] = f"command launch/observation failed: {error}"
        finally:
            record = runs.load_record(child_run) or {}
            cleanup = runs.host_cleanup(child_run, record)
            step.update(
                exit=record.get("exit"),
                cleanup=cleanup,
                launcher_exit=guard.returncode if guard else None,
                launcher_cleanup=guard.cleanup if guard else {"status": "unknown"},
                termination=record.get("termination"),
            )
            step["outcome"] = outcome(
                record.get("exit", {}).get("code") if record.get("exit") else None,
                cleanup,
                record.get("termination"),
            )
            if step["outcome"] == "blocked":
                step["reason"] = "command reported a missing prerequisite (exit 75); see log"
            if guard is None or guard.cleanup.get("status") != "confirmed":
                step["outcome"] = "failed"
            elif step["outcome"] == "passed" and guard.returncode != 0:
                step.update(outcome="failed", reason="run launcher failed after command completion")
            for receipt in record.get("storage", {}).values():
                Storage().retain(receipt, self.consumer)
            step["attachments"] = []
            if attachment_root:
                from surrealdb_fixture import native_completion

                for path in sorted(set(attachment_root.glob("*/record.json")) - before):
                    attachment = read_json(path) or {}
                    step["attachments"].append(
                        {
                            "record": str(path),
                            **{
                                key: attachment.get(key)
                                for key in ("id", "released", "child_exit_code", "command_cleanup")
                            },
                            "native_completion": native_completion(path.parent, attachment),
                        }
                    )
                needs_native = name in {batch.name for batch in BATCHES} or name in {
                    "pilot-produce",
                    "pilot-check",
                }
                native_confirmed = bool(step["attachments"]) and all(
                    row.get("released") is True
                    and (row.get("command_cleanup") or {}).get("status") == "confirmed"
                    and row["native_completion"].get("status") == "confirmed"
                    for row in step["attachments"]
                )
                if needs_native:
                    step["native_finality"] = "confirmed" if native_confirmed else "unconfirmed"
                if needs_native and not native_confirmed and step["outcome"] == "passed":
                    step.update(outcome="failed", reason="fixture/native finality unconfirmed")
            if junit:
                current = junit_source.stat() if junit_source.exists() else None
                if current is not None and current != previous_junit:
                    destination = child_run / "junit.xml"
                    shutil.copyfile(junit_source, destination)
                    step["junit"] = str(destination)
                else:
                    step["junit"] = None
                    step["junit_reason"] = "no fresh report; previous batch report was not reused"
                    if step["outcome"] == "passed":
                        step["outcome"] = "failed"
            self.save()
            print(
                f"{name}: {step['outcome']} exit={step['exit']} "
                f"host_cleanup={cleanup.get('status')}",
                flush=True,
            )
        if step["termination"] in {"cancelled", "interrupted"}:
            self.stopped = True
            raise InterruptedError(f"{name} was {step['termination']}")
        return step["outcome"] == "passed"

    def batch(self, batch: Batch, ready: bool) -> bool:
        install_name, dispose_name = f"{batch.name}-install", f"{batch.name}-dispose"
        if not ready:
            for name in (install_name, batch.name, dispose_name):
                self.blocked(name, "current CLI, control build and guard-free just ready required")
            return False
        config = Path(self.row(install_name)["service_config"])
        if config.parent.exists():
            for name in (install_name, batch.name, dispose_name):
                self.blocked(name, f"campaign path exists; refusing reuse: {config.parent}")
            return False
        participants = []
        passed = False
        try:
            installed = self.execute(install_name)
            if participant := self.row(install_name).get("run"):
                participants.append(participant)
            if installed:
                try:
                    campaign_identity(config)
                except (OSError, ValueError, KeyError, TypeError) as error:
                    self.blocked(batch.name, f"campaign identity refused: {error}")
                else:
                    passed = self.execute(batch.name, junit=True)
                    if participant := self.row(batch.name).get("run"):
                        participants.append(participant)
            else:
                self.blocked(batch.name, "fresh campaign installation did not pass")
        finally:
            if self.stopped:
                self.row(dispose_name).update(
                    reason="cancelled; exact campaign retained for checked disposal"
                )
                self.save()
            else:
                try:
                    identity = campaign_identity(config)
                except (OSError, ValueError, KeyError, TypeError) as error:
                    self.blocked(dispose_name, str(error))
                else:
                    command = ["just", "service", "dispose-campaign", identity]
                    for participant in participants:
                        command.extend(["--participant-run", participant])
                    disposed = self.execute(dispose_name, command)
                    passed = passed and disposed
        return passed

    def validate(self) -> None:
        built = self.execute("build-cli")
        controls = self.execute("build-controls")
        ready = self.execute("ready")
        prerequisites = built and controls and ready
        results = [self.batch(batch, prerequisites) for batch in BATCHES]
        if prerequisites and all(results):
            retained = read_json(RETAINED) or {}
            if (
                not isinstance(retained, dict)
                or retained.get("installation_id") != RETAINED_ID
                or retained.get("phase") != "ready"
            ):
                self.blocked("pilot-produce", "exact deliberately retained target B is unavailable")
                self.blocked("pilot-check", "pilot production prerequisite blocked")
            elif self.execute("pilot-produce"):
                self.execute("pilot-check")
            else:
                self.blocked("pilot-check", "pilot production, fixture and host cleanup must pass")
        else:
            for name in ("pilot-produce", "pilot-check"):
                self.blocked(name, "V01/V02 functional batches and checked disposal must pass")
        for name, _ in QUALITY:
            self.execute(name)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--print", action="store_true", dest="preview")
    parser.add_argument("--json", action="store_true", help="JSON preview (with --print)")
    args = parser.parse_args(argv)
    if args.preview:
        steps = plan(Path("<MANAGED_RUN>"))
        if args.json:
            print(json.dumps(steps, indent=2))
        else:
            for step in steps:
                binding = (
                    f"LCTX_SURREAL_SERVICE_CONFIG={step['service_config']} "
                    if step.get("service_config")
                    else ""
                )
                print(f"{step['name']}: {binding}{shlex.join(step['command'])}")
            print("Aggregate: build/runs/RUN/summary.json and summary.md; logs/JUnit retained.")
            print("Pilot requires V01/V02 passes. Fresh disposable targets; B is never disposed.")
        return 0
    run = runs.current_run()
    if run is None:
        parser.error(
            "execution requires just run --background --label catalog-completion-validation -- …"
        )
    if any(
        os.environ.get(key)
        for key in (
            "LCTX_COMPILER_RUNTIME_CONFIG",
            "LCTX_SURREAL_TEST_CONFIG",
            "LCTX_SURREAL_MAINTENANCE_TOKEN",
        )
    ):
        parser.error("start outside a fixture/maintenance lease; just ready must be guard-free")
    validation = Validation(run)

    def interrupted(number: int, _frame: object) -> None:
        validation.stopped = True
        raise InterruptedError(f"validation interrupted by signal {number}")

    previous = {
        sig: signal.signal(sig, interrupted)
        for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)
    }
    try:
        with admission([run]):
            receipt = (runs.load_record(run) or {})["storage"]["receipt"]
            Storage().retain(receipt, validation.consumer)
            validation.validate()
    except (Exception, KeyboardInterrupt) as error:
        validation.error = f"{type(error).__name__}: {error}"
        if isinstance(error, (InterruptedError, KeyboardInterrupt)):
            validation.stopped = True
    finally:
        validation.save()
        runs.set_progress(current_command=None)
        for sig, handler in previous.items():
            signal.signal(sig, handler)
    print((run / "summary.md").read_text())
    passed = not validation.error and all(s["outcome"] == "passed" for s in validation.steps)
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
