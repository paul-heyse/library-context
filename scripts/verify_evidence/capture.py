"""Invocation-owned capture. Capture bookkeeping never changes a native command outcome."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib
import json
import shlex
import sys
import tomllib
import uuid
from pathlib import Path
from typing import Any

import storage_owners
from executable_observation import observe as observe_executable
from harness import write_json_atomic
from storage_lifecycle import admission

from .context import SourceObserver, digest_file
from .contracts import SCHEMA, LaunchRequest

SAFE_ENV = (
    "INSTA_UPDATE",
    "UV_NO_SYNC",
    "RUST_BACKTRACE",
    "RUST_LIB_BACKTRACE",
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "NEXTEST_PROFILE",
    "PYTEST_ADDOPTS",
)


def configuration(directory: Path, root: Path, argv: Any, tool: str, env: Any) -> dict[str, Any]:
    """Bind effective policy, normalizing only invocation-owned evidence destinations."""
    observed = {}
    if tool == "nextest":
        paths = [
            ("repo", directory / "nextest-repo.toml"),
            ("user", directory / "nextest-user.toml"),
        ]
        for label, path in paths:
            with path.open("rb") as source:
                content = tomllib.load(source)
            for profile in content.get("profile", {}).values():
                if isinstance(profile.get("junit"), dict):
                    profile["junit"]["path"] = "<invocation-junit>"
            observed[label] = hashlib.sha256(
                json.dumps(content, sort_keys=True, default=str).encode()
            ).hexdigest()
        for index, arg in enumerate(argv):
            value = (
                argv[index + 1]
                if arg == "--tool-config-file" and index + 1 < len(argv)
                else arg.split("=", 1)[1]
                if arg.startswith("--tool-config-file=")
                else None
            )
            if value:
                tool_name, separator, path = value.partition(":")
                if separator:
                    observed[f"tool:{index}:{tool_name}"] = digest_file(Path(path))
    elif tool == "pytest":
        paths = []
        combined = [*shlex.split(env.get("PYTEST_ADDOPTS", "")), *argv]
        for index, arg in enumerate(combined):
            if arg in ("-c", "--config-file") and index + 1 < len(combined):
                paths = [("explicit", root / combined[index + 1])]
            elif arg.startswith("--config-file="):
                paths = [("explicit", root / arg.split("=", 1)[1])]
            elif arg.startswith("-c") and len(arg) > 2 and not arg.startswith("--"):
                paths = [("explicit", root / arg[2:])]
        if not paths:
            paths = [
                (name, root / name)
                for name in (
                    "pytest.toml",
                    ".pytest.toml",
                    "pytest.ini",
                    ".pytest.ini",
                    "pyproject.toml",
                    "tox.ini",
                    "setup.cfg",
                )
                if (root / name).is_file()
            ]
        for label, path in paths:
            observed[label] = digest_file(path)
    return observed


class Invocation:
    def __init__(self, owner: CaptureOwner, directory: Path, row: dict[str, Any]) -> None:
        self.owner, self.directory, self.row = owner, directory, row
        self.prepared: dict[str, Any] = {}
        self.adapter: Any = None

    def save(self, **values: Any) -> None:
        self.row.update(values)
        try:
            write_json_atomic(self.directory / "invocation.json", self.row)
        except OSError as error:
            self.owner.warn(error)

    def observe(self, event: dict[str, Any]) -> None:
        self.save(**event)

    def request(self, log: Path, live: bool) -> LaunchRequest:
        return LaunchRequest(
            self.row["effective_argv"],
            self.prepared["env"],
            log,
            live,
            Path(self.prepared["stdout"]) if self.prepared.get("stdout") else None,
            self.observe,
        )

    def command_finished(self, result: Any, *, duration: float | None = None) -> None:
        import runs

        # Persist before export, context re-observation, serving registration or restart.
        self.save(
            command={
                "exit": result.exit,
                "launch_error": result.launch_error,
                "duration": duration,
                "completed": runs.now(),
            }
        )
        if self.adapter and hasattr(self.adapter, "finalize"):
            try:
                capture = self.adapter.finalize(
                    self.directory, self.prepared["env"], self.owner.root
                )
                self.save(finalization=capture)
            except Exception as error:
                self.save(finalization={"status": "failed", "reason": str(error)})
        self.save(context_after=self.owner.context())
        # Artifact bytes observed here are distinct from source closure or proof that
        # the runner executed those same bytes during a concurrent replacement.
        binaries = []
        source = self.directory / (
            "nextest-events.jsonl" if self.row["tool"] == "nextest" else "cargo.jsonl"
        )
        if source.is_file():
            with source.open("rb") as stream:
                for line in stream:
                    try:
                        event = json.loads(line)
                    except ValueError:
                        continue
                    if (
                        not isinstance(event, dict)
                        or event.get("reason") != "compiler-artifact"
                        or not event.get("executable")
                    ):
                        continue
                    path = Path(event["executable"])
                    package_name = None
                    try:
                        manifest = Path(event["manifest_path"]).read_bytes()
                        package_name = tomllib.loads(manifest.decode())["package"]["name"]
                    except (OSError, ValueError, KeyError, TypeError):
                        pass
                    binaries.append({
                        **observe_executable(path),
                        "package_id": event.get("package_id"),
                        "package_name": package_name,
                        "target": event.get("target"),
                        "features": event.get("features"),
                        "profile": event.get("profile"),
                    })
            self.save(binaries=binaries)


class CaptureOwner:
    def __init__(self, run_dir: Path, root: Path) -> None:
        self.run_dir, self.root = run_dir.resolve(), root.resolve()
        self.raw = self.run_dir / "verify-evidence"
        self.receipt = self.run_dir / "verify-evidence-receipt"
        self.manifest_path = self.receipt / "manifest.json"
        self.source = SourceObserver(root)
        self.invocations: list[Invocation] = []
        self.warned = False
        self.manifest: dict[str, Any] = {"schema": SCHEMA, "run": run_dir.name, "invocations": []}
        if self.manifest_path.exists() or (self.run_dir / "verify-evidence.json").exists():
            raise ValueError("this run already owns verification evidence; start a separate run")
        owner = {"kind": "run", "path": str(self.run_dir)}
        consumer = f"run:{run_dir.name}"
        with admission([self.raw, self.receipt, self.run_dir / "verify-evidence.json"]):
            self.raw.mkdir(parents=True, exist_ok=True)
            self.receipt.mkdir(parents=True, exist_ok=True)
            self.manifest["storage"] = {
                "raw": storage_owners.enroll(
                    self.raw, "run-log", owner, consumer, temporary_days=30
                ),
            }
            write_json_atomic(self.manifest_path, self.manifest)
            self.manifest["storage"]["receipt"] = storage_owners.enroll(
                self.receipt, "run-receipt", owner, consumer, temporary_days=90
            )
            write_json_atomic(self.manifest_path, self.manifest)
            (self.run_dir / "verify-evidence.json").symlink_to(
                "verify-evidence-receipt/manifest.json"
            )

    def warn(self, error: Exception) -> None:
        if not self.warned:
            print(
                f"verify: evidence capture gap ({type(error).__name__}); native result unchanged",
                file=sys.stderr,
            )
            self.warned = True

    def context(self) -> str | None:
        context = self.source.observe()
        if not context.get("available"):
            return None
        ident = context["id"]
        path = self.raw / "contexts" / f"{ident}.json"
        if not path.exists():
            write_json_atomic(path, context)
        return ident

    def start(
        self, boundary: str, step: Any, argv: Any, env: Any, fixture: Any, selection: Any, log: Path
    ) -> Invocation:
        import runs

        ident = uuid.uuid4().hex
        directory = self.raw / ident
        directory.mkdir()
        row: dict[str, Any] = {
            "schema": SCHEMA,
            "id": ident,
            "boundary": boundary,
            "step": step.name,
            "tool": step.tool,
            "argv": list(argv),
            "effective_argv": list(argv),
            "cwd": str(self.root),
            "started": runs.now(),
            "selection": selection,
            "fixture": copy.deepcopy(fixture),
            "log": str(log),
            "context_before": self.context(),
            "settings": {key: env[key] for key in SAFE_ENV if key in env},
            "environment_coverage": "allowlisted non-secret diagnostics; credentials omitted",
            "dependencies": {"consumes": list(step.consumes), "produces": list(step.produces)},
            "restart_after": step.restart_after,
            "capture": {"status": "requested"},
            "artifacts": {},
        }
        invocation = Invocation(self, directory, row)
        invocation.save()
        self.invocations.append(invocation)
        self.manifest["invocations"].append({"id": ident, "boundary": boundary, "step": step.name})
        write_json_atomic(self.manifest_path, self.manifest)
        try:
            if step.tool in ("nextest", "pytest"):
                module = "pytest_adapter" if step.tool == "pytest" else "nextest"
                invocation.adapter = importlib.import_module(f"verify_evidence.{module}")
                prepared = invocation.adapter.prepare(list(argv), dict(env), directory, self.root)
            elif (
                step.tool == "cargo"
                and len(argv) > 1
                and argv[1] in ("check", "build", "test", "clippy", "rustc")
            ):
                if any(arg.startswith("--message-format") for arg in argv):
                    raise ValueError("explicit Cargo message format; native command preserved")
                effective = list(argv)
                at = effective.index("--") if "--" in effective else len(effective)
                effective.insert(at, "--message-format=json-render-diagnostics")
                prepared = {
                    "argv": effective,
                    "env": dict(env),
                    "stdout": str(directory / "cargo.jsonl"),
                    "artifacts": {"cargo": str(directory / "cargo.jsonl")},
                    "channels": {"cargo": "requested", "human": "boundary log"},
                }
            else:
                prepared = {
                    "argv": list(argv),
                    "env": dict(env),
                    "artifacts": {},
                    "channels": {"human": "boundary log"},
                }
            invocation.prepared = prepared
            invocation.save(
                effective_argv=list(prepared["argv"]),
                artifacts=prepared.get("artifacts", {}),
                capture={"status": "effective", "channels": prepared.get("channels")},
                tool_version=prepared.get("tool_version"),
                effective_config=(
                    {"selected": prepared["configuration"]}
                    if prepared.get("configuration")
                    else configuration(directory, self.root, argv, step.tool, env)
                ),
                selection_options=prepared.get("selection_options"),
            )
        except Exception as error:
            invocation.adapter = None
            invocation.prepared = {"env": dict(env)}
            invocation.save(
                capture={
                    "status": "unsupported",
                    "reason": str(error),
                    "repair": "just sync tools; inspect capture support",
                }
            )
        return invocation

    def close_boundary(
        self, boundary: str, result: dict[str, Any] | None, error: BaseException | None = None
    ) -> None:
        for invocation in self.invocations:
            if invocation.row["boundary"] == boundary:
                invocation.save(
                    boundary_final={
                        "outcome": result.get("outcome") if result else None,
                        "reason": result.get("reason") if result else None,
                        "error": type(error).__name__ if error else None,
                        "fixture_finalization": result.get("fixture_finalization")
                        if result
                        else {"status": "failed", "error": type(error).__name__},
                    }
                )


@contextlib.contextmanager
def optional_owner(run_dir: Path | None, root: Path):
    """Hold admission through all command and fixture finalization effects."""
    if run_dir is None:
        yield None
        return
    with admission(
        [
            run_dir / "verify-evidence",
            run_dir / "verify-evidence-receipt",
            run_dir / "verify-evidence.json",
        ]
    ):
        try:
            owner = CaptureOwner(run_dir, root)
        except Exception as error:
            print(
                f"verify: evidence unavailable ({type(error).__name__}); command unchanged",
                file=sys.stderr,
            )
            owner = None
        yield owner
