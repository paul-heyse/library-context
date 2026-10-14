"""Invocation-owned capture. Capture bookkeeping never changes a native command outcome."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib
import json
import os
import shlex
import sys
import tomllib
import uuid
from pathlib import Path
from typing import Any

import storage_owners
from executable_observation import observe as observe_executable
from harness import hold_lock, read_json, write_json_atomic
from storage_lifecycle import admission

from .context import SourceObserver, digest_file, project_root
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

REQUEST = "LCTX_CAPTURE_NATIVE"
PREPARED = "LCTX_EVIDENCE_PREPARED"
INVOCATION = "LCTX_EVIDENCE_INVOCATION"


def _marker(argv: Any, cwd: Path) -> str:
    # A marker is consumed only by hooks in the preparing process. An inherited
    # marker cannot suppress a nested launch by a child, even with identical argv.
    return hashlib.sha256(
        json.dumps([os.getpid(), list(argv), str(cwd.resolve())]).encode()
    ).hexdigest()


def native_tool(argv: Any) -> str:
    """Recognize actual argv only; opaque wrappers are never shell-parsed."""
    if not argv:
        return "other"
    name = Path(argv[0]).name
    if name == "cargo":
        return "nextest" if list(argv[1:3]) == ["nextest", "run"] else "cargo"
    if name == "pytest" or (name.startswith("python") and list(argv[1:3]) == ["-m", "pytest"]):
        return "pytest"
    if name == "uv" and len(argv) > 2 and argv[1] == "run":
        # Only the qualified transparent launcher prefix, not arbitrary uv
        # option/value grammar or a script containing a command.
        position = 2
        while position < len(argv) and argv[position] in {
            "--no-sync",
            "--locked",
            "--frozen",
            "--offline",
        }:
            position += 1
        if position < len(argv):
            return native_tool(argv[position:])
    return "other"


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
            argv=self.row["effective_argv"],
            env=self.prepared["env"],
            log=Path(self.row["log"]),
            live=live,
            cwd=Path(self.row["cwd"]),
            stdout=Path(self.prepared["stdout"]) if self.prepared.get("stdout") else None,
            observer=self.observe,
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
                    self.directory, self.prepared["env"], Path(self.row["cwd"])
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
                    except OSError, ValueError, KeyError, TypeError:
                        pass
                    binaries.append(
                        {
                            **observe_executable(path),
                            "package_id": event.get("package_id"),
                            "package_name": package_name,
                            "target": event.get("target"),
                            "features": event.get("features"),
                            "profile": event.get("profile"),
                        }
                    )
            self.save(binaries=binaries)


class CaptureOwner:
    def __init__(self, run_dir: Path, root: Path) -> None:
        self.run_dir, self.root = run_dir.resolve(), root.resolve()
        self.raw = self.run_dir / "verify-evidence"
        self.receipt = self.run_dir / "verify-evidence-receipt"
        self.manifest_path = self.receipt / "manifest.json"
        self.source = SourceObserver(root)
        self.invocations: list[Invocation] = []
        self.session_id: str | None = None
        self.warned = False
        owner = {"kind": "run", "path": str(self.run_dir)}
        consumer = f"run:{run_dir.name}"
        with admission([self.raw, self.receipt, self.run_dir / "verify-evidence.json"]):
            self.raw.mkdir(parents=True, exist_ok=True)
            self.receipt.mkdir(parents=True, exist_ok=True)
            descriptor = hold_lock(self.receipt / "registration.lock")
            try:
                if self.manifest_path.exists():
                    self._manifest()
                else:
                    link = self.run_dir / "verify-evidence.json"
                    if link.exists() or link.is_symlink():
                        raise ValueError("unrecognized existing evidence manifest; preserved")
                    manifest = {
                        "schema": SCHEMA,
                        "run": run_dir.name,
                        "invocations": [],
                        "storage": {
                            "raw": storage_owners.enroll(
                                self.raw, "run-log", owner, consumer, temporary_days=30
                            ),
                            "receipt": storage_owners.enroll(
                                self.receipt, "run-receipt", owner, consumer, temporary_days=90
                            ),
                        },
                    }
                    write_json_atomic(self.manifest_path, manifest)
                link = self.run_dir / "verify-evidence.json"
                if not link.exists() and not link.is_symlink():
                    link.symlink_to("verify-evidence-receipt/manifest.json")
                if link.resolve() != self.manifest_path:
                    raise ValueError("evidence manifest binding changed; preserved")
            finally:
                os.close(descriptor)

    def _manifest(self) -> dict[str, Any]:
        manifest = read_json(self.manifest_path)
        if (
            not manifest
            or manifest.get("schema") != SCHEMA
            or manifest.get("run") != self.run_dir.name
        ):
            raise ValueError("existing evidence manifest identity/schema mismatch")
        return manifest

    @property
    def manifest(self) -> dict[str, Any]:
        return self._manifest()

    def register(self, entry: dict[str, Any]) -> None:
        """Append one immutable UUID registration under a short lock, never a cached manifest."""
        descriptor = hold_lock(self.receipt / "registration.lock")
        try:
            manifest = self._manifest()
            existing = next(
                (row for row in manifest["invocations"] if row["id"] == entry["id"]), None
            )
            if existing is not None:
                if existing != entry:
                    raise ValueError("invocation registration changed")
                return
            manifest["invocations"].append(entry)
            write_json_atomic(self.manifest_path, manifest)
        finally:
            os.close(descriptor)

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
        self,
        boundary: str | None,
        step: Any,
        argv: Any,
        env: Any,
        fixture: Any,
        selection: Any,
        log: Path,
        *,
        cwd: Path | None = None,
        session: str | None = None,
        ident: str | None = None,
    ) -> Invocation:
        import runs

        ident = ident or uuid.uuid4().hex
        cwd = (cwd or self.root).resolve()
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
            "cwd": str(cwd),
            "session": session,
            "prerequisites": "declared-boundary"
            if boundary is not None
            else "unsupported: ad hoc command has no declared prerequisites",
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
        self.register({"id": ident, "boundary": boundary, "step": step.name, "session": session})
        env = dict(env)
        env[INVOCATION] = ident
        if env.get(REQUEST):
            env[REQUEST] = "1"
        prepared: dict[str, Any]
        try:
            if step.tool in ("nextest", "pytest"):
                module = "pytest_adapter" if step.tool == "pytest" else "nextest"
                invocation.adapter = importlib.import_module(f"verify_evidence.{module}")
                prepared = invocation.adapter.prepare(list(argv), dict(env), directory, cwd)
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
                raise ValueError(
                    "no qualified native capture adapter for this argv; "
                    "request remains available to nested launch owners"
                )
            prepared["env"][PREPARED] = _marker(prepared["argv"], cwd)
            invocation.prepared = prepared
            invocation.save(
                effective_argv=list(prepared["argv"]),
                artifacts=prepared.get("artifacts", {}),
                capture={"status": "effective", "channels": prepared.get("channels")},
                tool_version=prepared.get("tool_version"),
                parent_log=str(log),
                log=str(directory / "native.log"),
                effective_config=(
                    {"selected": prepared["configuration"]}
                    if prepared.get("configuration")
                    else configuration(directory, cwd, argv, step.tool, env)
                ),
                selection_options=prepared.get("selection_options"),
            )
        except Exception as error:
            invocation.adapter = None
            invocation.prepared = {"env": {**env, PREPARED: _marker(argv, cwd)}}
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


class CapturedLaunch:
    """One actual launch writer; an inherited request is not an invocation owner."""

    def __init__(
        self, argv: Any, env: Any, cwd: Path, log: Path, invocation: Invocation | None = None
    ) -> None:
        self.invocation = invocation
        self.argv = list(argv) if invocation is None else invocation.row["effective_argv"]
        self.env = dict(env) if invocation is None else invocation.prepared["env"]
        self.cwd = cwd
        self.log = Path(invocation.row["log"]) if invocation else log
        self.stdout = (
            Path(invocation.prepared["stdout"])
            if invocation and invocation.prepared.get("stdout")
            else None
        )

    def observe(self, event: dict[str, Any]) -> None:
        if self.invocation:
            self.invocation.observe(event)

    def finish(self, exit_code: int | None, launch_error: Any = None, cleanup: Any = None) -> None:
        if self.invocation:
            from types import SimpleNamespace

            try:
                self.invocation.command_finished(
                    SimpleNamespace(exit=exit_code, launch_error=launch_error)
                )
                if cleanup is not None:
                    self.invocation.save(
                        host_cleanup=cleanup,
                        host_cleanup_scope=(
                            "direct launched command group; enclosing registered host cleanup "
                            "is reported by the run owner"
                        ),
                    )
            except Exception as error:
                self.invocation.owner.warn(error)


@contextlib.contextmanager
def capture_launch(argv: Any, env: Any, cwd: Path, log: Path, fixture: Any = None):
    """Claim a root capture request at an actual owner launch, preserving unsupported argv."""
    cwd, log = Path(cwd).resolve(), Path(log)
    if (
        not env.get(REQUEST)
        or env.get(PREPARED) == _marker(argv, cwd)
        or not env.get("LCTX_RUN_DIR")
    ):
        yield CapturedLaunch(argv, env, cwd, log)
        return
    from types import SimpleNamespace

    with optional_owner(Path(env["LCTX_RUN_DIR"]), project_root(cwd)) as owner:
        if owner is None:
            yield CapturedLaunch(argv, env, cwd, log)
            return
        step = SimpleNamespace(
            name="native", tool=native_tool(argv), consumes=(), produces=(), restart_after=False
        )
        # Host writer registration precedes preparation exposure/launch and shares
        # T01 admission through final delegated host cleanup, independent of native finality.
        import run_delegation

        ident = uuid.uuid4().hex
        with run_delegation.writer_owner("native-capture", context={"invocation": ident}, env=env):
            try:
                invocation = owner.start(
                    None, step, argv, env, fixture, None, log, cwd=cwd, ident=ident
                )
            except Exception as error:
                owner.warn(error)
                invocation = None
            yield CapturedLaunch(argv, env, cwd, log, invocation)


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
        import run_delegation

        with run_delegation.writer_owner(
            "verification-evidence", context={"component": "verify-evidence"}
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
