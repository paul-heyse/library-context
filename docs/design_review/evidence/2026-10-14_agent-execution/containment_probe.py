"""Owned inert I03 comparison. Run only under an existing managed run."""

from __future__ import annotations

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))

from harness import ProcessIdentity, SpawnGuard, spawn_group, write_json_atomic


def until(predicate, seconds: float = 10.0) -> None:
    end = time.monotonic() + seconds
    while not predicate():
        if time.monotonic() >= end:
            raise TimeoutError("owned containment observation did not arrive")
        time.sleep(0.02)


def actor(directory: Path) -> None:
    sampler = """
from pathlib import Path
import signal,sys,time
d=Path(sys.argv[1])
def stop(*_):
    (d/'sampler-stopping').write_text('owned')
    time.sleep(4)
    (d/'sampler-finished').write_text('owned')
    raise SystemExit(0)
signal.signal(signal.SIGTERM,stop)
(d/'sampler-ready').write_text('owned')
time.sleep(120)
"""
    children = {
        "new_session_writer": subprocess.Popen(
            [sys.executable, "-c", "import time; time.sleep(120)"], start_new_session=True
        ),
        "new_shared_daemon_candidate": subprocess.Popen(
            [sys.executable, "-c", "import time; time.sleep(120)"], start_new_session=True
        ),
        "observer": subprocess.Popen([sys.executable, "-c", sampler, str(directory)]),
    }
    until(lambda: (directory / "sampler-ready").exists())
    write_json_atomic(
        directory / "children.json",
        {name: ProcessIdentity.of(child.pid).to_json() for name, child in children.items()},
    )
    # Exit deliberately: ExitType=cgroup must still retain the descendants.


def live(identity: dict) -> bool:
    owner = ProcessIdentity.from_json(identity)
    if not owner.alive():
        return False
    try:
        return Path(f"/proc/{owner.pid}/stat").read_text().rsplit(")", 1)[1].split()[0] not in {"Z", "X"}
    except FileNotFoundError:
        return False


def main() -> None:
    import runs
    import storage_owners
    from storage_lifecycle import admission

    run = runs.current_run()
    if run is None:
        raise RuntimeError("I03 requires a managed run before creating any output/process")
    directory = run / "containment"
    consumer = "citation:catalog-completion-agent-effectiveness"
    with admission([directory]):
        directory.mkdir()
        storage_owners.enroll(
            directory, "run-log", {"kind": "run", "path": str(run)}, consumer,
            temporary_days=30,
        )
        uid = os.getuid()
        runtime = Path("/run/user") / str(uid)
        bus = runtime / "bus"
        if runtime.stat().st_uid != uid or bus.stat().st_uid != uid or not bus.is_socket():
            raise RuntimeError("owned user-manager connection unavailable")
        env = {**os.environ, "XDG_RUNTIME_DIR": str(runtime),
               "DBUS_SESSION_BUS_ADDRESS": f"unix:path={bus}"}
        nonce = uuid.uuid4().hex
        unit = f"lctx-agent-probe-{nonce}.service"
        description = f"library-context I03 {run.name} {nonce}"
        write_json_atomic(directory / "intent.json", {
            "run": str(run), "unit": unit, "description": description,
            "consumer": consumer, "purpose": "inert-lifecycle-comparison",
        })

        def properties() -> dict[str, str]:
            result = subprocess.run(
                ["systemctl", "--user", "show", unit, "--property=Description,InvocationID,ActiveState,MainPID"],
                env=env, capture_output=True, text=True, check=False,
            )
            return dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)

        def stop_owned() -> None:
            observed = properties()
            if observed.get("Description") == description and observed.get("InvocationID"):
                subprocess.run(["systemctl", "--user", "stop", unit], env=env,
                               check=True, timeout=15)

        sentinel = spawn_group([sys.executable, "-c", "import time; time.sleep(120)"])
        with SpawnGuard(sentinel, grace=1):
            sentinel_identity = ProcessIdentity.of(sentinel.pid).to_json()
            launcher = None
            try:
                with (directory / "unit.log").open("wb") as output:
                    launcher = subprocess.Popen([
                        "systemd-run", "--user", "--quiet", "--wait", "--pipe", "--collect",
                        f"--unit={unit}", f"--description={description}",
                        "--property=Type=exec", "--property=ExitType=cgroup",
                        "--property=KillMode=control-group", "--property=TimeoutStopSec=2s",
                        sys.executable, str(Path(__file__).resolve()), "--actor", str(directory),
                    ], env=env, stdout=output, stderr=subprocess.STDOUT)
                    until(lambda: (directory / "children.json").exists() or launcher.poll() is not None)
                    if not (directory / "children.json").exists():
                        raise RuntimeError("transient unit failed before owned child publication")
                    children = json.loads((directory / "children.json").read_text())
                    until(lambda: properties().get("MainPID") == "0")
                    observed = properties()
                    assert observed["Description"] == description and observed["InvocationID"]
                    assert observed["ActiveState"] == "active"
                    assert all(live(identity) for identity in children.values())
                    write_json_atomic(directory / "observed.json", observed)
                    stop_owned()
                    launcher.wait(timeout=15)
                    until(lambda: not any(live(identity) for identity in children.values()))
                    assert live(sentinel_identity)
                    result = {
                        "nested_session_contained": True,
                        "exited_leader_keeps_cgroup_active": True,
                        "outside_owned_sentinel_preserved": True,
                        "new_shared_daemon_survived_unit_stop": live(children["new_shared_daemon_candidate"]),
                        "observer_started_finalization": (directory / "sampler-stopping").exists(),
                        "observer_finished_finalization": (directory / "sampler-finished").exists(),
                        "candidate_qualified": False,
                        "selection": "explicit-delegated-owner-composition",
                        "reason": "one disposable cgroup flattens shared-daemon and observer lifetimes",
                    }
                    assert not result["new_shared_daemon_survived_unit_stop"]
                    assert result["observer_started_finalization"]
                    assert not result["observer_finished_finalization"]
                    write_json_atomic(directory / "result.json", result)
                    print(json.dumps(result, sort_keys=True), flush=True)
            finally:
                stop_owned()
                if launcher is not None:
                    launcher.wait(timeout=15)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--actor":
        actor(Path(sys.argv[2]))
    else:
        main()
