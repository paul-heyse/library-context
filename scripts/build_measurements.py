# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Paul Heyse
"""Capture one source state and measure isolated Rust build variants.

Run ``just bench-builds preflight``; then ``just bench-builds capture
build/perf/<name>``. Later, run ``just bench-builds run build/perf/<name>
--variant stable --phase screen`` and repeat for the other named variants.
``report`` summarizes retained receipts. Only ``run`` compiles the Rust
workspace. All outputs stay under the campaign; the original source and
target remain untouched.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
import tomllib
from datetime import UTC, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NIGHTLY = "nightly-2026-09-13"
MIN_FREE_GIB = 200
EDIT_FILE = Path("crates/cpg-flow/src/lib.rs")
ENCODED_SEPARATOR = "\x1f"
VARIANTS = {
    "stable": ("1.98.1", False, 32, 1),
    "stable-cache": ("1.98.1", True, 32, 1),
    "nightly-32x1": (NIGHTLY, True, 32, 1),
    "nightly-16x2": (NIGHTLY, True, 16, 2),
    "nightly-8x4": (NIGHTLY, True, 8, 4),
    "nightly-4x8": (NIGHTLY, True, 4, 8),
}
ENV_KEYS = (
    "CARGO_BUILD_JOBS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_INCREMENTAL",
    "CARGO_TARGET_DIR",
    "RUSTC_WRAPPER",
    "RUSTUP_TOOLCHAIN",
    "SCCACHE_CACHE_SIZE",
    "SCCACHE_DIR",
    "SCCACHE_SERVER_UDS",
    "PSE_LLVM_PREFIX",
    "CLANG_PATH",
    "LIBCLANG_PATH",
    "LLVM_CONFIG_PATH",
    "UV_PROJECT_ENVIRONMENT",
)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def capture_command(
    command: list[str], *, env: dict[str, str] | None = None, cwd: Path | None = None
) -> str:
    return subprocess.check_output(
        command, text=True, env=env, cwd=cwd, stderr=subprocess.PIPE
    ).strip()


def inventory(root: Path) -> list[dict[str, str | int]]:
    """Inventory tracked and nonignored untracked files, including current edits."""
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=root
    ).split(b"\0")
    rows: list[dict[str, str | int]] = []
    for raw in sorted(set(names) - {b""}):
        name = os.fsdecode(raw)
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError(f"unsafe source path: {name}")
        path = root / relative
        if not path.exists() and not path.is_symlink():
            continue  # tracked deletion; the snapshot records git status separately
        if path.is_symlink():
            content = os.fsencode(path.readlink())
            kind = "symlink"
        elif path.is_file():
            content = path.read_bytes()
            kind = "file"
        else:
            raise ValueError(f"source inventory contains a non-file: {name}")
        rows.append(
            {
                "path": name,
                "kind": kind,
                "mode": path.lstat().st_mode & 0o777,
                "sha256": hashlib.sha256(content).hexdigest(),
            }
        )
    return rows


def source_digest(rows: list[dict[str, str | int]]) -> str:
    return hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()


def snapshot(root: Path, campaign: Path) -> dict:
    if campaign.exists():
        raise ValueError(f"campaign already exists: {campaign}")
    rows = inventory(root)
    campaign.mkdir(parents=True)
    destination = campaign / "source"
    destination.mkdir()
    for row in rows:
        source = root / str(row["path"])
        target = destination / str(row["path"])
        target.parent.mkdir(parents=True, exist_ok=True)
        if row["kind"] == "symlink":
            target.symlink_to(source.readlink())
        else:
            shutil.copy2(source, target)
    if inventory(root) != rows or inventory_from_rows(destination, rows) != rows:
        raise ValueError("source changed during capture or snapshot bytes differ")
    manifest = {
        "captured_utc": datetime.now(UTC).isoformat(),
        "head": capture_command(["git", "rev-parse", "HEAD"], cwd=root),
        "status": capture_command(["git", "status", "--porcelain=v1", "-uall"], cwd=root),
        "source_sha256": source_digest(rows),
        "files": rows,
    }
    write_json(campaign / "snapshot.json", manifest)
    return manifest


def inventory_from_rows(root: Path, rows: list[dict[str, str | int]]) -> list[dict[str, str | int]]:
    observed = []
    for row in rows:
        path = root / str(row["path"])
        content = os.fsencode(path.readlink()) if path.is_symlink() else path.read_bytes()
        observed.append(
            {
                "path": row["path"],
                "kind": "symlink" if path.is_symlink() else "file",
                "mode": path.lstat().st_mode & 0o777,
                "sha256": hashlib.sha256(content).hexdigest(),
            }
        )
    return observed


def preflight(root: Path = ROOT, system_script: Path | None = None) -> dict:
    config = tomllib.loads((root / ".cargo/config.toml").read_text())
    linux = config["target"]['cfg(target_os = "linux")']
    flags = linux["rustflags"]
    if linux["linker"] != "clang" or flags != ["-C", "link-arg=-fuse-ld=mold"]:
        raise ValueError("expected the checked Clang driver and mold flags in .cargo/config.toml")
    tools = {
        name: shutil.which(name)
        for name in ("clang", "clang++", "llvm-config", "mold", "sccache", "cargo", "uv")
    }
    missing = [name for name, path in tools.items() if path is None]
    if missing:
        raise ValueError(f"missing required tools: {', '.join(missing)}")
    prefix = Path(capture_command(["llvm-config", "--prefix"]))
    resource = Path(capture_command(["clang", "-print-resource-dir"]))
    if not resource.is_relative_to(prefix):
        raise ValueError("Clang resource directory differs from selected LLVM prefix")
    link = subprocess.run(
        ["clang", "-###", "-fuse-ld=mold", "-x", "c", "/dev/null", "-o", "/dev/null"],
        text=True,
        capture_output=True,
        check=True,
    )
    if "ld.mold" not in link.stderr:
        raise ValueError("Clang did not select mold in its link command")
    report = {
        "checked_utc": datetime.now(UTC).isoformat(),
        "tools": {name: str(Path(path).resolve()) for name, path in tools.items() if path},
        "llvm_prefix": str(prefix.resolve()),
        "llvm_resource_dir": str(resource.resolve()),
        "llvm_version": capture_command(["llvm-config", "--version"]),
        "mold_version": capture_command(["mold", "--version"]),
        "stable_rustc": capture_command(["rustup", "run", "1.98.1", "rustc", "-Vv"]),
        "nightly_rustc": capture_command(["rustup", "run", NIGHTLY, "rustc", "-Vv"]),
        "link_driver": linux,
    }
    if system_script is not None:
        report["system_llvm_verification"] = json.loads(
            capture_command([sys.executable, str(system_script), "--verify"])
        )
    return report


def competing_builds() -> list[str]:
    """Reject timing while another Cargo/rustc process is active."""
    output = capture_command(["ps", "-eo", "pid=,comm="])
    return [
        line.strip()
        for line in output.splitlines()
        if len(line.split()) >= 2 and line.split()[1] in {"cargo", "rustc"}
    ]


def variant_env(campaign: Path, variant: str, trial: int) -> dict[str, str]:
    toolchain, cached, jobs, threads = VARIANTS[variant]
    env = os.environ.copy()
    for key in (
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTC_WRAPPER",
        "CARGO_INCREMENTAL",
        "UV_PROJECT_ENVIRONMENT",
    ):
        env.pop(key, None)
    env["RUSTUP_TOOLCHAIN"] = toolchain
    env["CARGO_BUILD_JOBS"] = str(jobs)
    env["CARGO_INCREMENTAL"] = "0"
    env["CARGO_TARGET_DIR"] = str(campaign / "targets" / variant / f"trial-{trial}")
    flags = ["-C", "link-arg=-fuse-ld=mold"]
    if toolchain == NIGHTLY:
        flags.append(f"-Zthreads={threads}")
    # The snapshot is under build/, beneath the live .cargo/config.toml. Explicit
    # flags prevent Cargo from concatenating the parent's mold flag twice.
    env["CARGO_ENCODED_RUSTFLAGS"] = ENCODED_SEPARATOR.join(flags)
    if cached:
        wrapper = shutil.which("sccache")
        if wrapper is None:
            raise ValueError("sccache is unavailable")
        env["RUSTC_WRAPPER"] = wrapper
        env["SCCACHE_DIR"] = str(campaign / "cache" / variant)
        env["SCCACHE_CACHE_SIZE"] = "32G"
        socket_id = hashlib.sha256(f"{campaign}:{variant}".encode()).hexdigest()[:16]
        env["SCCACHE_SERVER_UDS"] = f"/tmp/lctx-perf-{socket_id}.sock"
    else:
        # Override the repository's default compiler wrapper for uncached controls.
        env["RUSTC_WRAPPER"] = ""
        for key in ("SCCACHE_DIR", "SCCACHE_CACHE_SIZE", "SCCACHE_SERVER_UDS"):
            env.pop(key, None)
    return env


def cache_stats(env: dict[str, str]) -> dict:
    if not env.get("RUSTC_WRAPPER"):
        return {}
    output = capture_command([env["RUSTC_WRAPPER"], "--show-stats", "--stats-format=json"], env=env)
    return json.loads(output)


def counter_delta(before: dict, after: dict) -> dict:
    result: dict[str, object] = {}
    for key, new in after.items():
        old = before.get(key, {} if isinstance(new, dict) else 0)
        if isinstance(new, dict) and isinstance(old, dict):
            result[key] = counter_delta(old, new)
        elif type(new) is int and type(old) is int:
            result[key] = new - old
    return result


def process_tree_rss(pid: int) -> int:
    pending, seen, total = [pid], set(), 0
    while pending:
        child = pending.pop()
        if child in seen:
            continue
        seen.add(child)
        try:
            lines = Path(f"/proc/{child}/status").read_text().splitlines()
            total += next(
                (int(line.split()[1]) * 1024 for line in lines if line.startswith("VmRSS:")), 0
            )
            for task in Path(f"/proc/{child}/task").iterdir():
                pending.extend(int(value) for value in (task / "children").read_text().split())
        except FileNotFoundError, ProcessLookupError:
            continue
    return total


def cargo_artifacts(path: Path) -> dict:
    fresh = rebuilt = 0
    finished = None
    for line in path.read_text().splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact":
            if message["fresh"]:
                fresh += 1
            else:
                rebuilt += 1
        elif message.get("reason") == "build-finished":
            finished = message["success"]
    if finished is not True:
        raise ValueError("Cargo did not emit a successful build-finished record")
    return {"fresh_artifacts": fresh, "rebuilt_artifacts": rebuilt}


def sample(
    work: Path,
    output: Path,
    target: Path,
    env: dict[str, str],
    name: str,
    command: list[str],
    *,
    cargo_json: bool = True,
) -> dict:
    active = competing_builds()
    if active:
        raise ValueError(f"competing Cargo/rustc processes: {active}")
    if shutil.disk_usage(output.parent).free < MIN_FREE_GIB * 1024**3:
        raise ValueError(f"less than {MIN_FREE_GIB} GiB free before {name}")
    output.mkdir(parents=True, exist_ok=True)
    if (output / "result.json").exists():
        raise ValueError(f"sample already exists: {output}")
    before_cache = cache_stats(env)
    free_before = shutil.disk_usage(output).free
    peak = 0
    done = threading.Event()
    started = time.monotonic()
    with (output / "stdout.log").open("w") as stdout, (output / "stderr.log").open("w") as stderr:
        process = subprocess.Popen(
            command, cwd=work, env=env, stdout=stdout, stderr=stderr, start_new_session=True
        )

        def watch() -> None:
            nonlocal peak
            while not done.is_set():
                peak = max(peak, process_tree_rss(process.pid))
                done.wait(0.1)

        sampler = threading.Thread(target=watch, daemon=True)
        sampler.start()
        try:
            code = process.wait()
        except BaseException:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            raise
        finally:
            done.set()
            sampler.join()
    result = {
        "name": name,
        "command": command,
        "environment": {key: env.get(key) for key in ENV_KEYS},
        "exit_code": code,
        "wall_seconds": time.monotonic() - started,
        "sampled_process_tree_peak_rss_bytes": peak,
        "peak_rss_scope": "sampled /proc descendant RSS; independent cache server excluded",
        "disk_free_before": free_before,
        "disk_free_after": shutil.disk_usage(output).free,
        "cache_delta": counter_delta(before_cache, cache_stats(env)),
    }
    if cargo_json and code == 0:
        result.update(cargo_artifacts(output / "stdout.log"))
        timing = target / "cargo-timings/cargo-timing.html"
        if timing.exists():
            shutil.copy2(timing, output / "cargo-timing.html")
    write_json(output / "result.json", result)
    if code:
        raise RuntimeError(f"{name} failed with exit {code}; see {output}")
    return result


def cargo_command(*, release: bool = False, package: str | None = None) -> list[str]:
    command = ["cargo", "build" if release or package else "test", "--locked"]
    if release:
        command.append("--release")
    if package:
        command.extend(["-p", package])
    else:
        command.extend(["--workspace", "--no-run"])
    command.extend(["--timings", "--message-format=json"])
    return command


def run_campaign(campaign: Path, variant: str, trial: int, phase: str) -> dict:
    manifest = json.loads((campaign / "snapshot.json").read_text())
    source = campaign / "source"
    if inventory_from_rows(source, manifest["files"]) != manifest["files"]:
        raise ValueError("captured source was modified")
    free = shutil.disk_usage(campaign).free
    if free < MIN_FREE_GIB * 1024**3:
        raise ValueError(f"less than {MIN_FREE_GIB} GiB free; do not start another build")
    output = campaign / "results" / variant / f"trial-{trial}"
    if output.exists():
        raise ValueError(f"trial already exists: {output}")
    output.mkdir(parents=True)
    work = campaign / "work" / variant / f"trial-{trial}"
    shutil.copytree(source, work, symlinks=True)
    env = variant_env(campaign, variant, trial)
    target = Path(env["CARGO_TARGET_DIR"])
    if target.exists():
        raise ValueError(f"target already exists: {target}")
    write_json(output / "host.json", preflight())
    write_json(
        output / "selection.json",
        {
            "variant": variant,
            "trial": trial,
            "phase": phase,
            "source_sha256": manifest["source_sha256"],
        },
    )
    wrapper = env.get("RUSTC_WRAPPER")
    if wrapper:
        Path(env["SCCACHE_DIR"]).mkdir(parents=True, exist_ok=True)
        subprocess.run([wrapper, "--start-server"], env=env, check=True, stdout=subprocess.DEVNULL)
    try:
        receipts = {}

        def record(name: str, command: list[str], *, cargo_json: bool = True) -> None:
            try:
                receipts[name] = sample(
                    work, output / name, target, env, name, command, cargo_json=cargo_json
                )
            except RuntimeError:
                result = output / name / "result.json"
                if result.exists():
                    receipts[name] = json.loads(result.read_text())
                    write_json(output / "summary.json", receipts)
                raise
            else:
                write_json(output / "summary.json", receipts)

        tests = cargo_command()
        record("cold-tests", tests)
        repeats = 1 if phase == "screen" else 3
        for index in range(repeats):
            record(f"warm-tests-{index + 1}", tests)
        if phase == "full":
            edit = work / EDIT_FILE
            original = edit.read_bytes()
            try:
                for index in range(3):
                    edit.write_bytes(
                        original + f"\n// build measurement edit {index + 1}\n".encode()
                    )
                    record(f"edit-tests-{index + 1}", tests)
            finally:
                edit.write_bytes(original)
            record("restore-tests", tests)
            record(
                "execute-tests",
                ["cargo", "nextest", "run", "--workspace", "--no-tests=pass"],
                cargo_json=False,
            )
            record("release-lctx", cargo_command(release=True, package="lctx"))
            record("native-crate", cargo_command(package="lctx-semantics"))
            # uv installs workspace members as editable packages. This exercises
            # the native import in a private environment, with no durable wheel.
            env["UV_PROJECT_ENVIRONMENT"] = str(work / ".venv")
            record("native-editable-sync", ["uv", "sync", "--frozen"], cargo_json=False)
            record(
                "native-import",
                [
                    "uv",
                    "run",
                    "--no-sync",
                    "pytest",
                    "python/lctx_mcp/tests/test_native_semantics.py",
                    "-q",
                    "-k",
                    "test_native_condition_boundary_uses_the_rust_kernel",
                ],
                cargo_json=False,
            )
            # Retain only task-owned results; target recovery is the final workload.
            shutil.rmtree(target)
            record("target-recovery", tests)
        return receipts
    finally:
        if wrapper:
            subprocess.run(
                [wrapper, "--stop-server"],
                env=env,
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=15,
            )


def summarize(campaign: Path) -> dict:
    manifest = json.loads((campaign / "snapshot.json").read_text())
    trials: dict[str, dict[str, dict[str, list[float]]]] = {}
    failures = []
    for path in sorted((campaign / "results").glob("*/trial-*/summary.json")):
        variant = path.parent.parent.name
        trial = path.parent.name
        samples = json.loads(path.read_text())
        for name, result in samples.items():
            if result["exit_code"]:
                failures.append(f"{variant}/{trial}/{name}")
                continue
            scenario = (
                name.rsplit("-", 1)[0] if name.startswith(("warm-tests-", "edit-tests-")) else name
            )
            trials.setdefault(variant, {}).setdefault(trial, {}).setdefault(scenario, []).append(
                result["wall_seconds"]
            )
    per_trial = {
        variant: {
            trial: {name: statistics.median(values) for name, values in scenarios.items()}
            for trial, scenarios in cases.items()
        }
        for variant, cases in trials.items()
    }
    groups: dict[str, dict[str, list[float]]] = {}
    for variant, cases in per_trial.items():
        for scenarios in cases.values():
            for name, value in scenarios.items():
                groups.setdefault(variant, {}).setdefault(name, []).append(value)
    rows = {
        variant: {
            name: {
                "count": len(values),
                "median_seconds": statistics.median(values),
                "min_seconds": min(values),
                "max_seconds": max(values),
            }
            for name, values in samples.items()
        }
        for variant, samples in groups.items()
    }
    comparisons = {}
    for variant in per_trial:
        baseline = "stable" if variant == "stable-cache" else "stable-cache"
        if variant == "stable" or baseline not in per_trial:
            continue
        common_trials = set(per_trial[variant]) & set(per_trial[baseline])
        common_scenarios = set(groups[variant]) & set(groups[baseline])
        comparisons[variant] = {}
        for scenario in sorted(common_scenarios):
            pairs = [
                (per_trial[baseline][trial][scenario], per_trial[variant][trial][scenario])
                for trial in sorted(common_trials)
                if scenario in per_trial[baseline][trial] and scenario in per_trial[variant][trial]
            ]
            if pairs:
                comparisons[variant][scenario] = {
                    "baseline": baseline,
                    "paired_trials": len(pairs),
                    "median_wall_improvement_percent": statistics.median(
                        100 * (before - after) / before for before, after in pairs
                    ),
                    "every_pair_faster": all(after < before for before, after in pairs),
                }
    return {
        "source_sha256": manifest["source_sha256"],
        "measurements": rows,
        "comparisons": comparisons,
        "failures": failures,
        "decision": "not_run: comparisons require paired, uncontended trials and correctness gates",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    host = commands.add_parser("preflight", help="inspect Clang, mold and Rust without Rust builds")
    host.add_argument(
        "--llvm-system-script", type=Path, help="also run the applied installer in --verify mode"
    )
    capture = commands.add_parser("capture", help="copy exact current source into a new campaign")
    capture.add_argument("campaign", type=Path)
    run = commands.add_parser("run", help="execute one build variant in a captured campaign")
    run.add_argument("campaign", type=Path)
    run.add_argument("--variant", choices=tuple(VARIANTS), required=True)
    run.add_argument("--trial", type=int, default=1)
    run.add_argument("--phase", choices=("screen", "full"), default="screen")
    report = commands.add_parser("report", help="summarize existing trial receipts without builds")
    report.add_argument("campaign", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "preflight":
            print(json.dumps(preflight(system_script=args.llvm_system_script), indent=2))
        elif args.command == "capture":
            print(json.dumps(snapshot(ROOT, args.campaign.resolve()), indent=2))
        elif args.command == "report":
            print(json.dumps(summarize(args.campaign.resolve()), indent=2))
        else:
            if args.trial < 1:
                parser.error("--trial must be positive")
            results = run_campaign(args.campaign.resolve(), args.variant, args.trial, args.phase)
            print(json.dumps(results, indent=2))
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"build measurement failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
