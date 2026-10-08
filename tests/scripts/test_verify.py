"""The resolved command plan and its execution over fakes; nothing here builds or starts servers."""

from __future__ import annotations

import contextlib
import dataclasses
import json
from pathlib import Path
from typing import Any

import pytest

import verify
from verify import Boundary, Observation, Options, PlanError, Runtime, Selection, Step


def plan_of(*args: str, options: Options | None = None):
    namespace = verify.parse_args(list(args))
    return verify.resolve(namespace.selections, options or verify.options_from(namespace))


def argv_of(planned, step: str) -> tuple[str, ...]:
    return next(s.argv for s in planned.steps if s.step.name == step)


# ---------------------------------------------------------------------------------------------
# Static plan


def test_drill_selection_is_one_static_plan_with_each_filter_at_its_owner(monkeypatch, capsys):
    """D2 drill: four boundaries, four owners, one --print, no build or fixture."""

    def forbidden(*args, **kwargs):
        raise AssertionError("--print must not launch anything")

    monkeypatch.setattr(verify.subprocess, "run", forbidden)
    monkeypatch.setattr(verify.subprocess, "Popen", forbidden)
    monkeypatch.setattr(verify, "real_fixture", forbidden)
    monkeypatch.setattr(
        verify,
        "observe_requirements",
        lambda required, static=False: {
            r: Observation(r, None if r == "native-python" else True, f"{r}: observed")
            for r in required
        },
    )
    code = verify.main(
        [
            "--select", "compiler:producer", "--nextest-args", "--test graph_artifact",
            "--select", "compiler:cli", "--nextest-args", "--test compile_artifact",
            "--select", "store", "--nextest-args", "--test publication",
            "--select", "serving:mcp", "--pytest-args", "-k native_session",
            "--print", "--json",
        ]
    )  # fmt: skip
    assert code == 0
    data = json.loads(capsys.readouterr().out)
    steps = {
        (row["boundary"], step["name"]): step["argv"]
        for row in data["boundaries"]
        for step in row["steps"]
    }
    assert steps[("compiler:producer", "nextest")][-4:] == [
        "-p", "cpg-core", "--test", "graph_artifact"
    ]  # fmt: skip
    assert steps[("compiler:cli", "nextest")][-4:] == ["-p", "lctx", "--test", "compile_artifact"]
    # The store's two-package set narrows to the package that has the named test.
    assert steps[("store:rust", "nextest")][-4:] == [
        "-p", "lctx-publisher", "--test", "publication"
    ]  # fmt: skip
    assert steps[("serving:mcp", "pytest")][-2:] == ["-k", "native_session"]
    assert "native_session" not in " ".join(steps[("serving:mcp", "native_journey")])
    assert all("--no-tests=fail" in argv for (b, s), argv in steps.items() if s == "nextest")
    fixture_env = data["boundaries"][0]["steps"][0]["env"]
    assert (
        "LCTX_SURREAL_TEST_CONFIG" in fixture_env and "LCTX_COMPILER_RUNTIME_CONFIG" in fixture_env
    )
    assert data["readiness"]["native-python"]["ready"] is None


def test_shared_tool_arguments_reach_every_owner_and_scoped_ones_stay_scoped():
    plan = plan_of(
        "--nextest-args", "-E 'test(x)'",
        "--select", "model", "--select", "analytics", "--nextest-args", "--lib",
    )  # fmt: skip
    model, analytics = plan
    assert argv_of(model, "nextest")[-2:] == ("-E", "test(x)")
    assert argv_of(analytics, "nextest")[-3:] == ("-E", "test(x)", "--lib")


def test_invalid_target_is_named_against_the_actual_package_set():
    with pytest.raises(PlanError) as error:
        plan_of("--select", "store", "--nextest-args", "--test nope")
    message = str(error.value)
    assert "lctx-surrealdb, lctx-publisher" in message and "publication" in message


def test_explicit_targets_replace_defaults_but_filters_keep_them():
    (filtered,) = plan_of("--select", "compiler:producer", "--nextest-args", "-E 'test(a)'")
    assert argv_of(filtered, "nextest")[-6:] == (
        "-p",
        "cpg-core",
        "--lib",
        "--tests",
        "-E",
        "test(a)",
    )
    (targeted,) = plan_of("--select", "compiler:producer", "--nextest-args", "--lib")
    argv = argv_of(targeted, "nextest")
    assert "--tests" not in argv and argv[-3:] == ("-p", "cpg-core", "--lib")


def test_tool_arguments_without_an_owner_are_refused():
    with pytest.raises(PlanError, match="no pytest step"):
        plan_of("--select", "model", "--pytest-args", "-k x")
    with pytest.raises(PlanError, match="empty required selections"):
        plan_of("--select", "model", "--nextest-args", "--no-tests=pass")


def test_family_shortcuts_route_to_the_primary_tool():
    assert verify.legacy(["model", "-E", "test(ids)"]) == [
        "--select", "model", "--nextest-args", "-E 'test(ids)'"
    ]  # fmt: skip
    assert verify.legacy(["serving", "--command", "mcp", "--", "-k", "session"]) == [
        "--select", "serving:mcp", "--pytest-args", "-k session"
    ]  # fmt: skip
    with pytest.raises(PlanError, match="choose --command"):
        verify.legacy(["serving", "-k", "x"])
    assert verify.legacy(["qualify"]) == ["--qualify"]


def test_cargo_configuration_reaches_cargo_and_nextest_only():
    options = Options(cargo_config=("k=v",))
    (planned,) = plan_of("--select", "oracles", options=options)
    assert argv_of(planned, "build")[:3] == ("cargo", "--config", "k=v")
    assert "--config" not in argv_of(planned, "pytest")
    (rust,) = plan_of("--select", "model", options=options)
    assert argv_of(rust, "nextest")[:5] == ("cargo", "nextest", "run", "--config", "k=v")


def test_adding_a_boundary_changes_only_its_definition(monkeypatch, capsys):
    added = Boundary(
        "model", "extra", "a new boundary", (Step("nextest", "nextest", packages=("lctx-model",)),)
    )
    monkeypatch.setattr(verify, "BOUNDARIES", (*verify.BOUNDARIES, added))
    # Selection, plan, help and execution all see it without any other edit.
    assert [p.boundary.id for p in plan_of("--select", "model")] == ["model:rust", "model:extra"]
    assert "model:extra" in verify.help_epilog()
    fake = Fake()
    results = verify.execute(
        plan_of("--select", "model:extra"), fake.runtime(), Options(), fake.logs
    )
    assert [(r["boundary"], r["outcome"]) for r in results] == [("model:extra", "passed")]


# ---------------------------------------------------------------------------------------------
# Execution over fakes


class FakeServer:
    def __init__(self) -> None:
        self.id = "fx1"
        self.kind = "run"
        self.ended: str | None = None

    def exited(self) -> str | None:
        return self.ended


class FakeAttachment:
    def __init__(self, scratch: Path) -> None:
        self.server = FakeServer()
        self.id = "att1"
        self.scratch = scratch
        self.extra_env: dict[str, str] = {}
        self.restarts = 0
        self.retained: list[str] = []

    def environment(self, source):
        env = dict(source)
        env["LCTX_SURREAL_TEST_CONFIG"] = "cfg"
        env.update(self.extra_env)
        return env

    def restart(self) -> None:
        self.restarts += 1

    def use_serving(self, name: str) -> dict[str, Any]:
        self.extra_env["LCTX_NATIVE_SERVING_CONFIG"] = f"/kept/{name}/viewer.json"
        return {"name": name, "content": {"semantic": "s1", "realization": "r1"}}

    def retain_serving(self, name: str) -> Path:
        return self.scratch / name / "viewer.json"

    def record_serving(self, name: str, command) -> dict[str, Any]:
        self.retained.append(name)
        return {"name": name, "content": {"semantic": "s2"}}


class Fake:
    def __init__(self, codes: dict[str, int] | None = None, ready: set[str] | None = None) -> None:
        import tempfile

        self.codes = codes or {}
        self.ready = ready
        self.commands: list[tuple[str, ...]] = []
        self.envs: list[dict[str, str]] = []
        self.progress: list[dict[str, Any]] = []
        self.root = Path(tempfile.mkdtemp())
        self.logs = self.root / "logs"
        self.attachments: list[FakeAttachment] = []
        self.on_run = None

    def run(self, argv, env, log, live) -> int:
        self.commands.append(tuple(argv))
        self.envs.append(dict(env))
        log.parent.mkdir(parents=True, exist_ok=True)
        with open(log, "a") as handle:
            handle.write("output\n")
        if self.on_run:
            self.on_run(argv)
        joined = " ".join(argv)
        return next((code for key, code in self.codes.items() if key in joined), 0)

    @contextlib.contextmanager
    def fixture(self, options):
        attachment = FakeAttachment(self.root / "scratch")
        self.attachments.append(attachment)
        yield attachment

    def observe(self, required):
        return {
            r: Observation(r, self.ready is None or r in self.ready, f"{r}: x", f"just sync {r}")
            for r in required
        }

    def runtime(self, stop=lambda: False) -> Runtime:
        return Runtime(
            observe=self.observe,
            run=self.run,
            fixture=self.fixture,
            owner=lambda required, report: contextlib.nullcontext(),
            progress=lambda **fields: self.progress.append(fields),
            report=lambda message: None,
            release_dir=Path("/release"),
            stop=stop,
        )


def test_a_failed_boundary_preserves_the_others_outcomes():
    fake = Fake({"lctx-model": 1})
    plan = plan_of("--select", "model", "--select", "analytics", "--select", "store")
    results = verify.execute(plan, fake.runtime(), Options(), fake.logs)
    assert [(r["boundary"], r["outcome"]) for r in results] == [
        ("model:rust", "failed"),
        ("analytics:rust", "passed"),
        ("store:rust", "passed"),
    ]
    assert results[0]["reason"] == "step nextest exited 1"
    assert results[2]["fixture"]["id"] == "fx1"
    assert any(p.get("current_command") == "analytics:rust" for p in fake.progress)


def test_blocked_needs_evidence_and_names_its_repair():
    fake = Fake(ready={"tools"})
    plan = plan_of("--select", "serving:rust", "--select", "model")
    results = verify.execute(plan, fake.runtime(), Options(), fake.logs)
    assert results[0]["outcome"] == "blocked"
    assert "native-serving: just sync native-serving" in results[0]["reason"]
    assert results[1]["outcome"] == "passed" and len(fake.commands) == 1

    launch = Fake({"lctx-model": 127})
    (result,) = verify.execute(
        plan_of("--select", "model"), launch.runtime(), Options(), launch.logs
    )
    assert result["outcome"] == "blocked" and "exit 127" in result["reason"]

    oom = Fake()
    oom.on_run = lambda argv: setattr(oom.attachments[-1].server, "ended", "oom-kill")
    (result,) = verify.execute(plan_of("--select", "store"), oom.runtime(), Options(), oom.logs)
    assert result["outcome"] == "blocked" and "oom-kill" in result["reason"]

    unexplained = Fake({"lctx-model": 75})
    (result,) = verify.execute(
        plan_of("--select", "model"), unexplained.runtime(), Options(), unexplained.logs
    )
    assert result["outcome"] == "failed"  # an exit code alone is not infrastructure evidence


def test_fixture_launch_failure_is_blocked_with_its_evidence(monkeypatch):
    import surrealdb_fixture

    fake = Fake()

    @contextlib.contextmanager
    def failing(options):
        raise surrealdb_fixture.FixtureBlocked("readiness", "server did not answer", "retry")
        yield

    runtime = fake.runtime()
    runtime.fixture = failing
    (result,) = verify.execute(plan_of("--select", "store"), runtime, Options(), fake.logs)
    assert result["outcome"] == "blocked"
    assert result["reason"] == "fixture readiness: server did not answer; repair: retry"


def test_interruption_is_a_termination_and_unstarted_boundaries_are_not_run():
    fake = Fake()
    stopping: list[int] = []
    fake.on_run = lambda argv: stopping.append(15)
    plan = plan_of("--select", "model", "--select", "analytics")
    results = verify.execute(plan, fake.runtime(stop=lambda: bool(stopping)), Options(), fake.logs)
    assert [r["outcome"] for r in results] == ["not_run", "not_run"]
    assert results[0]["reason"] == "interrupted"
    assert results[1]["reason"] == "interrupted before it started"


def test_mcp_journey_produces_restarts_then_runs_pytest_with_the_serving_config():
    fake = Fake()
    (result,) = verify.execute(
        plan_of("--select", "serving:mcp"), fake.runtime(), Options(), fake.logs
    )
    assert result["outcome"] == "passed"
    assert [s["name"] for s in result["steps"]] == ["build", "native_journey", "pytest"]
    serving = str(fake.root / "scratch" / "serving.json")
    assert fake.envs[1]["LCTX_RETAIN_NATIVE_FIXTURE_CONFIG"] == serving
    assert fake.envs[2]["LCTX_NATIVE_SERVING_CONFIG"] == serving
    assert fake.envs[2]["LCTX_REMEDIATION_CLI_BIN"] == "/release/lctx"
    assert fake.attachments[0].restarts == 1


def test_mcp_reuse_records_the_skipped_journey_as_not_run_with_identity():
    fake = Fake()
    options = Options(attach="kept1", serving="pilot")
    (result,) = verify.execute(
        plan_of("--select", "serving:mcp", options=options), fake.runtime(), options, fake.logs
    )
    assert result["outcome"] == "passed"
    journey = result["steps"][1]
    assert journey["outcome"] == "not_run"
    assert journey["identity"]["content"] == {"semantic": "s1", "realization": "r1"}
    assert not any("native_journey" in " ".join(c) for c in fake.commands)
    assert fake.envs[-1]["LCTX_NATIVE_SERVING_CONFIG"] == "/kept/pilot/viewer.json"
    assert fake.attachments[0].restarts == 0


def test_retaining_records_identity_after_the_producer_succeeds():
    fake = Fake()
    options = Options(attach="kept1", retain_serving="pilot")
    (result,) = verify.execute(
        plan_of("--select", "serving:mcp", options=options), fake.runtime(), options, fake.logs
    )
    assert result["fixture"]["serving_retained"]["name"] == "pilot"
    assert fake.attachments[0].retained == ["pilot"]


def test_qualify_refuses_reuse_and_filters_and_selects_everything():
    for options in (
        Options(qualify=True, attach="k"),
        Options(qualify=True, serving="n", attach="k"),
    ):
        with pytest.raises(PlanError, match="refuses reuse"):
            verify.resolve([], options)
    with pytest.raises(PlanError, match="no filters"):
        verify.resolve([Selection(None, ["-E", "x"])], Options(qualify=True))
    plan = verify.resolve([], Options(qualify=True))
    assert [p.boundary.id for p in plan] == [b.id for b in verify.BOUNDARIES]
    assert any(p.boundary.family == "leaf" for p in plan)
    with pytest.raises(PlanError, match="kept fixture"):
        plan_of("--select", "serving:mcp", "--serving", "x")


def test_rerun_repeats_only_failed_blocked_and_unreached_boundaries(tmp_path, monkeypatch):
    monkeypatch.setenv("LCTX_RUNS_ROOT", str(tmp_path))
    run_dir = tmp_path / "20261008T000000.000Z-abc123"
    run_dir.mkdir()
    fake = Fake({"lctx-model": 1})
    plan = plan_of("--select", "model", "--nextest-args", "-E 'test(ids)'", "--select", "analytics")
    summary = verify.Summary(run_dir / "summary.json", plan, Options(cli=True))
    results = verify.execute(plan, fake.runtime(), Options(), fake.logs, on_result=summary.update)
    summary.finish(results, "completed")
    # A boundary the run never reached is also repeated.
    data = json.loads((run_dir / "summary.json").read_text())
    data["plan"].append({"select": "providers:flow", "nextest_args": [], "pytest_args": []})
    (run_dir / "summary.json").write_text(json.dumps(data))

    selections, options = verify.rerun_selection("abc123")
    assert [(s.select, s.nextest_args) for s in selections] == [
        ("model:rust", ["-E", "test(ids)"]),
        ("providers:flow", []),
    ]
    assert options.cli is True
    again = Fake()
    verify.execute(verify.resolve(selections, options), again.runtime(), options, again.logs)
    assert len(again.commands) == 2 and not any("lctx-analytics" in c for c in again.commands)


def test_pure_boundaries_take_no_ownership_and_python_ones_share(tmp_path, monkeypatch):
    import workspace_env

    monkeypatch.setenv("XDG_RUNTIME_DIR", str(tmp_path))
    monkeypatch.delenv("LCTX_ENV_OWNERSHIP", raising=False)
    for boundary in ("model:rust", "providers:flow", "tooling:docs"):
        with verify.environment_owner(verify.boundaries()[boundary].requirements) as owned:
            assert owned.resources == ()
    with verify.environment_owner(verify.boundaries()["serving:mcp"].requirements) as owned:
        assert [r.kind for r in owned.acquired] == ["environment", "extension"]
        assert owned.mode == "shared"
        assert owned.resources[0].path == workspace_env.environment_path()


def test_readiness_observes_native_substrate_and_never_synchronizes():
    calls = []

    @dataclasses.dataclass
    class Seen:
        ready: bool
        repair: str = "route"

        def message(self):
            return "seen"

    observed = verify.observe_requirements(
        {"tools", "native-python", "native-store"},
        static=True,
        python=lambda r: calls.append(("python", r)) or Seen(True),
        native=lambda r: calls.append(("native", r)) or Seen(False),
    )
    assert calls == [("native", "native-store"), ("python", "tools")]
    assert observed["native-python"].ready is None
    assert observed["native-store"].ready is False


def test_every_named_default_target_exists():
    layout = verify.workspace_packages()
    for boundary in verify.BOUNDARIES:
        for step in boundary.steps:
            if step.tool == "nextest":
                for option, name in verify.target_options(step.targets):
                    if name is not None:
                        owners = [
                            p
                            for p in step.packages
                            if name
                            in getattr(
                                verify.package_targets(p, layout[p]), verify.NAMED_TARGETS[option]
                            )
                        ]
                        assert owners, (boundary.id, option, name)
