"""The resolved command plan and its execution over fakes; nothing here builds or starts servers."""

from __future__ import annotations

import contextlib
import dataclasses
import json
from pathlib import Path
from typing import Any

import pytest

import verify
from verify import Boundary, Options, PlanError, Runtime, Selection, Step
from workspace_env import Readiness


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

    # The only command a static plan runs is Cargo's read-only target report; take it first.
    verify.cargo_targets()
    monkeypatch.setattr(verify.subprocess, "run", forbidden)
    monkeypatch.setattr(verify.subprocess, "Popen", forbidden)
    monkeypatch.setattr(verify, "real_fixture", forbidden)
    monkeypatch.setattr(
        verify,
        "observe_requirements",
        lambda required, static=False: {
            r: Readiness(r, "tools", True, (), "") for r in required if r != "native-python"
        },
    )
    code = verify.main(
        [
            "--select", "compiler:producer", "--nextest-args",
            "--test compiler_artifacts -E 'test(graph_artifact::)'",
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
    assert steps[("compiler:producer", "nextest")][-6:] == [
        "-p", "cpg-core", "--test", "compiler_artifacts", "-E", "test(graph_artifact::)"
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
    assert set(verify.fixture_variables()) <= set(fixture_env)  # named by the fixture module
    assert "native-python" not in data["readiness"]
    assert data["unobserved"] == ["native-python"]


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
    assert verify.legacy(["model", "--", "-E", "test(ids)"]) == [
        "--select", "model", "--nextest-args", "-E 'test(ids)'"
    ]  # fmt: skip
    assert verify.legacy(["serving", "--command", "mcp", "--", "-k", "session"]) == [
        "--select", "serving:mcp", "--pytest-args", "-k session"
    ]  # fmt: skip
    with pytest.raises(PlanError, match="choose --command"):
        verify.legacy(["serving", "--", "-k", "x"])
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
        env["LCTX_COMPILER_RUNTIME_CONFIG"] = "compiler-cfg"
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


class FakeOwnership:
    def __init__(self, required) -> None:
        self.required = required

    def environment(self, base):
        return {**base, "LCTX_ENV_OWNERSHIP": "token:" + ",".join(sorted(self.required))}


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
            r: Readiness(r, r, self.ready is None or r in self.ready, (), "x") for r in required
        }

    def runtime(self, stop=lambda: False) -> Runtime:
        return Runtime(
            observe=self.observe,
            run=self.run,
            fixture=self.fixture,
            owner=lambda required, report: contextlib.nullcontext(FakeOwnership(required)),
            progress=lambda **fields: self.progress.append(fields),
            report=lambda message: None,
            target_dir=Path("/"),
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


def test_flow_oracle_owns_native_configuration_and_tools():
    fake = Fake()
    (result,) = verify.execute(
        plan_of("--select", "oracles:flow"), fake.runtime(), Options(), fake.logs
    )
    assert result["outcome"] == "passed"
    assert len(fake.attachments) == 1
    assert all(env.get("LCTX_COMPILER_RUNTIME_CONFIG") for env in fake.envs)


def test_unconfigured_tooling_excludes_native_consumers():
    (planned,) = plan_of("--select", "tooling:python")
    assert not planned.boundary.fixture
    command = argv_of(planned, "pytest")
    assert "--ignore=tests/scripts/test_programmatic_native.py" in command
    assert "--ignore=tests/scripts/test_programmatic_native_numeric.py" in command
    (serving,) = plan_of("--select", "serving:mcp")
    assert "tests/scripts/test_programmatic_native.py" in argv_of(serving, "pytest")
    assert "tests/scripts/test_programmatic_native_numeric.py" in argv_of(serving, "pytest")


def test_failed_native_publication_keeps_dependent_evaluator_not_run():
    fake = Fake({"native_journey": 1})
    (result,) = verify.execute(
        plan_of("--select", "serving:mcp"), fake.runtime(), Options(), fake.logs
    )
    assert result["outcome"] == "failed"
    assert len(fake.commands) == 2
    assert result["steps"][-1]["name"] == "pytest"
    assert result["steps"][-1]["outcome"] == "blocked"
    assert "native serving production" in result["steps"][-1]["reason"]


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
    for boundary in ("compiler:producer", "providers:flow", "tooling:docs"):
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
    assert "native-python" not in observed
    assert observed["native-store"].ready is False


def test_every_named_default_target_exists():
    """Against Cargo's own report (cargo metadata, read-only)."""
    known = verify.cargo_targets()
    for boundary in verify.BOUNDARIES:
        for step in boundary.steps:
            for option, name in verify.target_options(step.targets):
                if name is not None:
                    kind = verify.NAMED_TARGETS[option]
                    assert any(name in known[p].get(kind, set()) for p in step.packages), (
                        boundary.id,
                        option,
                        name,
                    )


# ---------------------------------------------------------------------------------------------
# Review findings (2026-10-08): F02 shortcut options, F03 --cli, F04 evidence, F07, F08, targets

TARGETS = {
    "lctx-surrealdb": {"lib": {"lctx_surrealdb"}, "test": {"native", "cache"}},
    "lctx-publisher": {"lib": {"lctx_publisher"}, "test": {"publication"}},
}


def shortcut(*args: str):
    namespace = verify.parse_args(verify.legacy(list(args)))
    return namespace, verify.options_from(namespace)


def test_shortcut_options_reach_the_plan_parser_and_only_the_remainder_passes_through():
    namespace, options = shortcut(
        "store", "--cargo-config", "profile.release.debug=0", "--print", "--", "-E", "test(x)"
    )
    assert options.cargo_config == ("profile.release.debug=0",) and namespace.print
    (planned,) = verify.resolve(namespace.selections, options)
    argv = argv_of(planned, "nextest")
    assert argv[:5] == ("cargo", "nextest", "run", "--config", "profile.release.debug=0")
    assert argv[-2:] == ("-E", "test(x)") and "--cargo-config" not in argv

    namespace, options = shortcut(
        "serving", "--command", "mcp", "--attach", "abc", "--serving", "pilot", "--print"
    )
    assert (options.attach, options.serving) == ("abc", "pilot")
    (planned,) = verify.resolve(namespace.selections, options)
    assert [s.skipped is not None for s in planned.steps] == [False, True, False]
    assert "--attach" not in argv_of(planned, "pytest")

    namespace, options = shortcut("compiler", "--command", "producer", "--nextest-args", "--lib")
    (planned,) = verify.resolve(namespace.selections, options)
    assert argv_of(planned, "nextest")[-3:] == ("-p", "cpg-core", "--lib")

    namespace, options = shortcut("store", "--cli", "--live", "--retain-serving", "x")
    assert options.cli and namespace.live and options.retain_serving == "x"


def test_shortcut_refuses_unknown_options_before_the_separator():
    with pytest.raises(PlanError, match="tool arguments go after `--`"):
        verify.legacy(["model", "-E", "test(ids)"])
    for option in ("--select", "--rerun"):
        with pytest.raises(PlanError, match="not a family-shortcut option"):
            verify.legacy(["model", option, "x"])


def test_cli_exercises_the_cli_in_the_steps_that_read_it():
    options = Options(cli=True)
    fake = Fake()
    verify.execute(
        plan_of("--select", "store", options=options), fake.runtime(), options, fake.logs
    )
    assert fake.commands[0][:2] == ("cargo", "build")
    assert fake.envs[1]["LCTX_REMEDIATION_CLI_BIN"] == "/release/lctx"
    plain = Fake()
    verify.execute(plan_of("--select", "store"), plain.runtime(), Options(), plain.logs)
    assert len(plain.commands) == 1 and "LCTX_REMEDIATION_CLI_BIN" not in plain.envs[0]


def test_a_server_exit_without_oom_evidence_is_a_failure():
    fake = Fake()
    fake.on_run = lambda argv: setattr(fake.attachments[-1].server, "ended", "exit-code")
    (result,) = verify.execute(plan_of("--select", "store"), fake.runtime(), Options(), fake.logs)
    assert result["outcome"] == "failed"
    assert result["reason"] == "fixture server exited (exit-code) during the step"
    assert result["steps"][-1]["outcome"] == "failed"


def test_requirements_follow_what_boundaries_import():
    declared = {b.id: b.requirements for b in verify.BOUNDARIES}
    assert "tools" in declared["model:rust"]  # presentation.rs: uv run --no-sync python
    assert "tools" in declared["providers:extract"]  # harness.rs: uv run --no-sync pyrefly
    for pure in ("compiler:producer", "compiler:cli", "store:rust", "serving:rust"):
        assert declared[pure] & {"tools", "native-python"} == set(), pure


def test_step_children_receive_the_ownership_token():
    fake = Fake()
    verify.execute(plan_of("--select", "model"), fake.runtime(), Options(), fake.logs)
    assert fake.envs[0]["LCTX_ENV_OWNERSHIP"] == "token:tools"


def test_named_target_narrows_and_unknown_names_the_package_set():
    store = verify.boundaries()["store:rust"]
    step = store.steps[-1]
    assert verify.narrow(store, step, ["--test", "publication"], TARGETS) == (
        ("lctx-publisher",),
        (),
    )
    assert verify.narrow(store, step, ["--lib"], TARGETS)[0] == step.packages
    with pytest.raises(PlanError) as error:
        verify.narrow(store, step, ["--test=nope"], TARGETS)
    assert "(lctx-surrealdb, lctx-publisher)" in str(error.value)
    assert "lctx-publisher: publication" in str(error.value)


# BC3/BC4: command planning and receipt controls only; no Rust execution or qualification.


def test_cargo_profiles_are_shared_or_scoped_and_nextest_runner_profile_is_independent():
    model, analytics = plan_of(
        "--cargo-profile",
        "local-test-candidate",
        "--select",
        "model",
        "--select",
        "analytics",
        "--cargo-profile",
        "test",
        "--nextest-args",
        "--profile ci -E 'test(ids)'",
    )
    assert model.selection["cargo_profile"] == "local-test-candidate"
    assert analytics.selection["cargo_profile"] == "test"
    for planned, profile in ((model, "local-test-candidate"), (analytics, "test")):
        argv = argv_of(planned, "nextest")
        assert argv[argv.index("--cargo-profile") + 1] == profile
        assert "--release" not in argv
    assert argv_of(analytics, "nextest")[-4:] == ("--profile", "ci", "-E", "test(ids)")
    (default,) = plan_of("--select", "model")
    assert default.selection["cargo_profile"] == "release"
    namespace, _ = shortcut("model", "--cargo-profile", "test")
    assert namespace.selections[0].cargo_profile == "test"


def test_profile_arguments_cannot_bypass_the_resolved_owner():
    for argument in (
        "--release",
        "-r",
        "-rEtest(ids)",
        "--cargo-profile=test",
        "--cargo-profile test",
    ):
        with pytest.raises(PlanError, match="verify --cargo-profile"):
            plan_of("--select", "model", "--nextest-args", argument)
    for profile in ("../release", "", "debug", "test/other"):
        with pytest.raises(PlanError, match="invalid Cargo profile"):
            plan_of("--select", "model", "--cargo-profile", profile)
    for argument in ("--target-dir /other", "--config profile.release.opt-level=0"):
        with pytest.raises(PlanError, match="owned by verify"):
            plan_of("--select", "model", "--nextest-args", argument)


def test_local_profiles_drive_build_test_doc_and_actual_cli_environment():
    options = Options(cli=True)
    store, mcp, docs = plan_of(
        "--cargo-profile",
        "local-test-candidate",
        "--select",
        "store",
        "--select",
        "serving:mcp",
        "--select",
        "tooling:docs",
        options=options,
    )
    for planned in (store, mcp, docs):
        for step in planned.steps:
            if step.step.tool == "cargo":
                assert step.argv[step.argv.index("--profile") + 1] == "local-test-candidate"
                assert "--release" not in step.argv
    assert "--doc" in argv_of(docs, "doctest")
    fake = Fake()
    fake_runtime = fake.runtime()
    fake_runtime.target_dir = Path("/owned/artifacts")
    results = verify.execute([store, mcp, docs], fake_runtime, options, fake.logs)
    assert all(result["outcome"] == "passed" for result in results)
    assert fake.envs[1]["LCTX_REMEDIATION_CLI_BIN"] == "/owned/artifacts/local-test-candidate/lctx"
    assert fake.envs[4]["LCTX_EVAL_WORKER"] == "/owned/artifacts/local-test-candidate/lctx-eval"
    assert fake.envs[4]["LCTX_REMEDIATION_CLI_BIN"] == "/owned/artifacts/local-test-candidate/lctx"
    assert results[0]["steps"][0]["cargo_profile"] == "local-test-candidate"
    assert results[0]["steps"][0]["artifact_dir"] == "/owned/artifacts/local-test-candidate"
    # Both Cargo's test and dev profiles use debug artifacts. The build itself still proves
    # the selected profile; a pre-existing debug/lctx file is never treated as freshness.
    test_plan = plan_of("--select", "store", "--cargo-profile", "test", options=options)
    test_fake = Fake()
    verify.execute(test_plan, test_fake.runtime(), options, test_fake.logs)
    assert test_fake.envs[1]["LCTX_REMEDIATION_CLI_BIN"] == "/debug/lctx"
    assert test_fake.commands[0][test_fake.commands[0].index("--profile") + 1] == "test"


def test_target_directory_uses_effective_override_and_profile_mapping(tmp_path, monkeypatch):
    monkeypatch.setattr(verify, "ROOT", tmp_path)
    (tmp_path / ".cargo").mkdir()
    (tmp_path / ".cargo/config.toml").write_text('[build]\ntarget-dir = "configured"\n')
    config_file = tmp_path / ".cargo/override.toml"
    config_file.write_text('[build]\ntarget-dir = "from-file"\n')
    assert verify.target_directory({}) == tmp_path / "configured"
    assert (
        verify.target_directory({"CARGO_BUILD_TARGET_DIR": "build-env"}) == tmp_path / "build-env"
    )
    assert verify.target_directory({"CARGO_TARGET_DIR": "/intentional"}) == Path("/intentional")
    assert (
        verify.target_directory({"CARGO_TARGET_DIR": "/intentional"}, (str(config_file),))
        == tmp_path / "from-file"
    )
    target = verify.target_directory({}, ('build.target-dir="command-line"',))
    assert target == tmp_path / "command-line"
    assert verify.artifact_directory(target, "test") == target / "debug"
    assert verify.artifact_directory(target, "dev") == target / "debug"
    assert verify.artifact_directory(target, "release") == target / "release"
    assert verify.artifact_directory(target, "bench") == target / "release"
    assert (
        verify.artifact_directory(target, "local-test-candidate") == target / "local-test-candidate"
    )


def test_print_and_list_use_the_same_resolved_profile_without_real_builds(monkeypatch, capsys):
    plans = plan_of("--select", "model", "--cargo-profile", "test")
    monkeypatch.setattr(verify, "base_environment", lambda: {"CARGO_TARGET_DIR": "/intentional"})
    description = verify.describe(plans, {}, Options())
    assert description["boundaries"][0]["selection"]["cargo_profile"] == "test"
    step = description["boundaries"][0]["steps"][0]
    assert step["cargo_profile"] == "test" and step["artifact_dir"] == "/intentional/debug"
    verify.print_plan(description)
    assert "--cargo-profile test" in capsys.readouterr().out
    calls = []

    def listing(argv, **kwargs):
        calls.append(tuple(argv))
        return type(
            "Listed",
            (),
            {
                "returncode": 0,
                "stdout": (
                    '{"rust-suites": {"model": {"testcases": '
                    '{"ids": {"filter-match": {"status": "matches"}}}}}}'
                ),
                "stderr": "",
            },
        )()

    monkeypatch.setattr(verify.subprocess, "run", listing)
    assert verify.list_plan(plans, Options(), {}) == 0
    assert calls[0][:3] == ("cargo", "nextest", "list")
    assert calls[0][calls[0].index("--cargo-profile") + 1] == "test"
    assert "--release" not in calls[0]


def test_qualify_remains_release_after_local_default_changes_and_refuses_weakening(
    monkeypatch, tmp_path
):
    monkeypatch.setattr(verify, "DEFAULT_CARGO_PROFILE", "test")
    # Test the owner transition without claiming that the future profile is qualified.
    (ordinary,) = plan_of("--select", "model")
    assert ordinary.selection["cargo_profile"] == "test"
    qualified = verify.resolve([], Options(qualify=True))
    assert all(p.selection["cargo_profile"] == "release" for p in qualified)
    assert all(s.cargo_profile in (None, "release") for p in qualified for s in p.steps)
    for profile in ("test", "local-test-candidate"):
        with pytest.raises(PlanError, match="requires Cargo profile release"):
            plan_of("--qualify", "--cargo-profile", profile)
    config = tmp_path / "weaken.toml"
    config.write_text("[profile.release]\nopt-level=0\n")
    for override in (
        "profile.release.opt-level=0",
        'env.CARGO_PROFILE_RELEASE_OPT_LEVEL="0"',
        str(config),
    ):
        with pytest.raises(PlanError, match="profile configuration overrides"):
            verify.resolve([], Options(qualify=True, cargo_config=(override,)))
    monkeypatch.setenv("CARGO_PROFILE_RELEASE_LTO", "off")
    with pytest.raises(PlanError, match="inherited CARGO_PROFILE"):
        verify.resolve([], Options(qualify=True))


def test_production_only_oracle_and_native_extension_are_explicit(monkeypatch):
    oracle, mcp = plan_of(
        "--cargo-profile", "local-test-candidate", "--select", "oracles", "--select", "serving:mcp"
    )
    assert oracle.selection["cargo_profile"] == "release"
    assert all(s.cargo_profile == "release" for s in oracle.steps)
    assert argv_of(oracle, "build")[argv_of(oracle, "build").index("--target-dir") + 1] == str(
        verify.ROOT / "target"
    )
    assert mcp.selection["cargo_profile"] == "local-test-candidate"
    assert mcp.selection["production_only"] == ["native-python preparation: release"]


def test_rerun_preserves_new_profiles_and_historical_release_when_default_changes(
    tmp_path, monkeypatch
):
    run = tmp_path / "run"
    run.mkdir()
    monkeypatch.setattr(verify.runs, "resolve", lambda _: run)
    plans = plan_of(
        "--select",
        "model",
        "--cargo-profile",
        "local-test-candidate",
        "--select",
        "analytics",
        "--cargo-profile",
        "test",
    )
    summary = verify.Summary(run / "summary.json", plans, Options())
    assert summary.data["plan"][0]["steps"][0]["cargo_profile"] == "local-test-candidate"
    assert summary.data["plan"][0]["steps"][0]["artifact_dir"].endswith("/local-test-candidate")
    fake = Fake({"lctx-model": 1})
    results = verify.execute(plans, fake.runtime(), Options(), fake.logs)
    # Preserve a candidate failed result; the second boundary never reached the old runner.
    summary.finish(results[:1], "interrupted")
    monkeypatch.setattr(verify, "DEFAULT_CARGO_PROFILE", "test")
    selections, options = verify.rerun_selection("prior")
    assert [s.cargo_profile for s in selections] == ["local-test-candidate", "test"]
    assert [p.selection["cargo_profile"] for p in verify.resolve(selections, options)] == [
        "local-test-candidate",
        "test",
    ]
    data = json.loads((run / "summary.json").read_text())
    del data["boundaries"][0]["selection"]["cargo_profile"]
    del data["plan"][1]["cargo_profile"]
    (run / "summary.json").write_text(json.dumps(data))
    selections, options = verify.rerun_selection("historical")
    assert [p.selection["cargo_profile"] for p in verify.resolve(selections, options)] == [
        "release",
        "release",
    ]


def test_provider_group_scope_is_mandatory_and_user_filters_intersect_even_on_target_override():
    (provider,) = plan_of("--select", "providers:extract")
    argv = argv_of(provider, "nextest")
    assert verify.target_options(argv) == [
        ("--lib", None),
        ("--test", "acquisition"),
        ("--test", "bundle"),
        ("--test", "extraction_contracts"),
        ("--test", "extraction_types"),
        ("--test", "extraction_calls"),
        ("--test", "extraction_syntax"),
    ]
    predicate = argv[argv.index("-E") + 1]
    assert "binary(=extraction_normalized)" not in predicate
    for case in (
        "harness",
        "typed_conformance",
        "typed_flow",
        "typed_calls",
        "native_overload_origins",
        "typed_ruff_context",
    ):
        assert f"test(/^{case}::/)" in predicate
    assert (
        "kind(lib)" in predicate
        and "binary(=acquisition)" in predicate
        and "binary(=bundle)" in predicate
    )
    (provider,) = plan_of(
        "--select",
        "providers:extract",
        "--nextest-args",
        "--tests -E 'test(flow)' --filter-expr='test(calls)'",
    )
    argv = argv_of(provider, "nextest")
    assert argv.count("-E") == 1
    selected_scope = verify.PROVIDER_FILTER
    assert (
        argv[argv.index("-E") + 1]
        == f"(package(=cpg-extract)) & ({selected_scope}) & ((test(flow)) | (test(calls)))"
    )
    assert "--filter-expr=test(calls)" not in argv
    assert "kind(lib)" in argv[argv.index("-E") + 1]
    assert "--no-tests=fail" in argv
    # Unrestricted compiler producer still selects all of its lib/integration targets.
    (compiler,) = plan_of("--select", "compiler:producer")
    assert "--lib" in argv_of(compiler, "nextest") and "--tests" in argv_of(compiler, "nextest")
    assert "-E" not in argv_of(compiler, "nextest")


def test_provider_discovery_has_same_mandatory_intersection_as_execution(monkeypatch):
    plans = plan_of("--select", "providers:extract", "--nextest-args", "-E 'test(harness::)' --lib")
    calls = []
    monkeypatch.setattr(
        verify.subprocess,
        "run",
        lambda argv, **kw: (
            calls.append(tuple(argv))
            or type(
                "Listed",
                (),
                {
                    "returncode": 0,
                    "stdout": (
                        '{"rust-suites": {"extract": {"testcases": '
                        '{"harness": {"filter-match": {"status": "matches"}}}}}}'
                    ),
                    "stderr": "",
                },
            )()
        ),
    )
    assert verify.list_plan(plans, Options(), {}) == 0
    run = argv_of(plans[0], "nextest")
    assert calls[0][calls[0].index("-E") + 1] == run[run.index("-E") + 1]
    assert run[run.index("-E") + 1] == "(package(=cpg-extract)) & (kind(lib)) & ((test(harness::)))"
    assert calls[0][calls[0].index("--cargo-profile") + 1] == "release"


def test_qualifier_focused_reruns_keep_release_acceptance_guards_and_provenance(
    tmp_path, monkeypatch
):
    run = tmp_path / "run"
    run.mkdir()
    monkeypatch.setattr(verify.runs, "resolve", lambda _: run)
    # Historical qualification receipts predate the explicit provenance field.
    saved = {
        "options": {"qualify": True},
        "plan": [{"select": "model:rust", "nextest_args": [], "pytest_args": []}],
        "boundaries": [],
    }
    (run / "summary.json").write_text(json.dumps(saved))
    selections, options = verify.rerun_selection("qualified")
    assert not options.qualify and options.required_release
    monkeypatch.setattr(verify, "DEFAULT_CARGO_PROFILE", "test")
    plan = verify.resolve(selections, options)
    assert [p.boundary.id for p in plan] == ["model:rust"]
    assert plan[0].selection["cargo_profile"] == "release"
    monkeypatch.setenv("CARGO_PROFILE_RELEASE_OPT_LEVEL", "0")
    with pytest.raises(PlanError, match="inherited CARGO_PROFILE"):
        verify.resolve(selections, options)
    monkeypatch.delenv("CARGO_PROFILE_RELEASE_OPT_LEVEL")
    with pytest.raises(PlanError, match="profile configuration overrides"):
        verify.resolve(
            selections, dataclasses.replace(options, cargo_config=("profile.release.opt-level=0",))
        )
    with pytest.raises(PlanError, match="requires Cargo profile release"):
        verify.resolve([Selection("model:rust", cargo_profile="test")], options)
    # A failed focused rerun must carry the constraint into the next generation too.
    summary = verify.Summary(run / "summary.json", plan, options)
    assert summary.data["options"]["required_release"] is True
    assert summary.data["options"]["qualify"] is False
    summary.finish([], "interrupted")
    again, again_options = verify.rerun_selection("focused")
    assert again_options.required_release and not again_options.qualify
    assert [p.boundary.id for p in verify.resolve(again, again_options)] == ["model:rust"]


def test_explicit_provider_target_keeps_only_its_exact_scope_or_empty_failure():
    for target, expected in (
        ("--lib", "kind(lib)"),
        ("--test extraction_calls", verify.PROVIDER_TARGET_FILTERS[5][2]),
        ("--test extraction_normalized", "none()"),
    ):
        (provider,) = plan_of("--select", "providers:extract", "--nextest-args", target)
        argv = argv_of(provider, "nextest")
        assert argv[argv.index("-E") + 1] == f"(package(=cpg-extract)) & ({expected})"
        assert "--no-tests=fail" in argv


def test_provider_tests_target_includes_library_and_allowed_integrations():
    (provider,) = plan_of("--select", "providers:extract", "--nextest-args", "--tests")
    argv = argv_of(provider, "nextest")
    assert "--tests" in argv
    predicate = argv[argv.index("-E") + 1]
    assert "kind(lib)" in predicate
    assert "binary(=acquisition)" in predicate and "binary(=bundle)" in predicate
    for case in (
        "harness",
        "typed_conformance",
        "typed_flow",
        "typed_calls",
        "native_overload_origins",
        "typed_ruff_context",
    ):
        assert f"test(/^{case}::/)" in predicate
    (specific,) = plan_of(
        "--select", "providers:extract", "--nextest-args", "--test extraction_calls"
    )
    specific_argv = argv_of(specific, "nextest")
    specific_predicate = specific_argv[specific_argv.index("-E") + 1]
    assert "kind(lib)" not in specific_predicate
    assert "binary(=extraction_calls)" in specific_predicate
    assert "binary(=extraction_contracts)" not in specific_predicate


def test_discovery_empty_required_selection_fails_without_rust_execution(monkeypatch):
    monkeypatch.setattr(
        verify.subprocess,
        "run",
        lambda *a, **kw: type(
            "Listed", (), {"returncode": 0, "stdout": '{"rust-suites": {}}', "stderr": ""}
        )(),
    )
    assert verify.list_plan(plan_of("--select", "model"), Options(), {}) == 1


def test_rerun_refuses_profile_overrides_instead_of_silently_ignoring_them(capsys):
    assert verify.main(["--rerun", "prior", "--cargo-profile", "test", "--print"]) == 2
    assert "prior selection and profile" in capsys.readouterr().err
