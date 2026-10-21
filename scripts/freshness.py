"""Read-only freshness of enumerated generated outputs (plan P6, AE-11; capability review F08).

Standard library only. `just fresh [output…]` launches it as
`uv run --no-project --offline --no-python-downloads python scripts/freshness.py …`.

Every output reports exactly one state, its regenerate command (never invoked) and a reason:

- `clean`: the owning check ran and found no drift;
- `stale`: the owning check found drift; the command exits 1;
- `heuristic`: a hint that neither proves drift nor freshness;
- `not_run`: freshness was not established (a prerequisite is missing, a check could not run,
  or only a test run can decide).

Nothing here regenerates, formats or synchronizes. Uncommitted work is reported as found.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Callable, Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, dataclass, field
from pathlib import Path

from gold_reference import reference_root

ROOT = Path(__file__).resolve().parent.parent
STATES = ("clean", "stale", "heuristic", "not_run")
GOLD_SKILL = reference_root() / "build" / "manifests"
INSTA_NOT_RUN = "confirmed only by running the owning tests with INSTA_UPDATE=no"


@dataclass
class Freshness:
    output: str
    state: str
    regenerate: str
    reason: str
    files: list[str] = field(default_factory=list)
    details: list[str] = field(default_factory=list)

    def __post_init__(self) -> None:
        assert self.state in STATES, self.state


@dataclass
class Ran:
    returncode: int
    stdout: str
    stderr: str


Runner = Callable[..., Ran]


def run(argv: Sequence[str], *, cwd: Path = ROOT, environment: dict | None = None) -> Ran:
    """Run a read-only check from the checkout root; a missing tool is exit 127."""
    env = dict(os.environ)
    env.update(GIT_OPTIONAL_LOCKS="0", CARGO_TERM_COLOR="never", NO_COLOR="1")
    env.update(environment or {})
    try:
        done = subprocess.run(
            list(argv),
            cwd=cwd,
            env=env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return Ran(127, "", str(error))
    return Ran(done.returncode, done.stdout, done.stderr)


def _failed(output: str, regenerate: str, argv: Sequence[str], ran: Ran) -> Freshness:
    tail = (ran.stderr or ran.stdout).strip().splitlines()[-3:]
    return Freshness(
        output,
        "not_run",
        regenerate,
        f"`{' '.join(argv)}` could not decide (exit {ran.returncode})",
        details=tail,
    )


def _relative(path: str, root: Path) -> str:
    try:
        return str(Path(path).resolve().relative_to(root))
    except ValueError:
        return path


# ---------------------------------------------------------------------------------------------
# Outputs


class MetadataInputs(dict[Path, bytes]):
    """Metadata-operation capture, never a source/producer semantic fingerprint."""

    def __init__(self):
        super().__init__()
        self.directories: set[Path] = set()
        self.symlinks: dict[Path, str] = {}

    def __eq__(self, other):
        return (
            isinstance(other, MetadataInputs)
            and dict.__eq__(self, other)
            and self.directories == other.directories
            and self.symlinks == other.symlinks
        )

    def __ne__(self, other):
        return not self == other


class UnsupportedShape(ValueError):
    """A conservative capture plus an actual Cargo oracle may qualify this shape."""


def metadata_inputs(
    root: Path, *, source_bodies: bool = False, conservative: bool = False
) -> MetadataInputs:
    """Capture Cargo's declarations and complete target discovery shape.

    Cargo owns target discovery, including arbitrary explicit suffixes and empty glob
    directories. Only source bodies are placeholders; non-source/declaration bytes
    retain their meaning. Unsupported layouts refuse instead of certifying clean.
    """
    root = root.resolve()
    inputs = MetadataInputs()
    excluded = {".git", ".venv", "target", "build", "__pycache__", "node_modules"}
    omitted = set()
    for directory, dirs, files in os.walk(root, followlinks=False):
        omitted.update(
            Path(directory) / name
            for name in dirs
            if name in excluded and (conservative or Path(directory) == root)
        )
        dirs[:] = [
            name
            for name in dirs
            if name not in excluded or (not conservative and Path(directory) != root)
        ]
        inputs.directories.add(Path(directory))
        for name in list(dirs):
            path = Path(directory) / name
            if path.is_symlink():
                if not conservative:
                    raise UnsupportedShape(f"unsupported symlink directory: {path}")
                inputs.symlinks[path] = os.readlink(path)
                inputs.directories.add(path)
                dirs.remove(name)
        for name in files:
            if name == ".git" and Path(directory) == root:
                continue
            path = Path(directory) / name
            declaration = name in {
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain",
                "rust-toolchain.toml",
                "config",
                "config.toml",
                "hakari.toml",
            }
            if path.is_symlink():
                if not conservative:
                    raise UnsupportedShape(f"unsupported symlink input: {path}")
                inputs.symlinks[path] = os.readlink(path)
                if not declaration and path.suffix != ".rs":
                    inputs[path] = b""
                    continue
            if not path.resolve().is_relative_to(root):
                raise ValueError(f"unsupported escaping input: {path}")
            inputs[path] = (
                b""
                if (path.suffix == ".rs" and not source_bodies)
                or (conservative and not declaration and path.suffix != ".rs")
                else path.read_bytes()
            )
    declarations = {
        path
        for path in inputs
        if path.name
        in {
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain",
            "rust-toolchain.toml",
            "config",
            "config.toml",
            "hakari.toml",
        }
    }
    for path, data in list(inputs.items()):
        if path.name != "Cargo.toml":
            continue

        def paths(value):
            if isinstance(value, dict):
                for key, child in value.items():
                    if key in {"path", "workspace"} and isinstance(child, str):
                        yield child
                    elif key in {"members", "default-members", "exclude"} and isinstance(
                        child, list
                    ):
                        yield from child
                    else:
                        yield from paths(child)
            elif isinstance(value, list):
                for child in value:
                    yield from paths(child)

        manifest = tomllib.loads(data.decode())
        for declaration in paths(manifest):
            source = (path.parent / declaration).resolve()
            if Path(declaration).is_absolute() or not source.resolve().is_relative_to(root):
                raise ValueError(f"unsupported escaping path in {path}: {declaration}")
            if any(source == skipped or source.is_relative_to(skipped) for skipped in omitted):
                raise ValueError(
                    f"declaration enters excluded capture tree in {path}: {declaration}"
                )
            for match in glob.glob(str(source), recursive=True, include_hidden=True):
                if any(
                    Path(match).resolve() == skipped
                    or Path(match).resolve().is_relative_to(skipped)
                    for skipped in omitted
                ):
                    raise ValueError(
                        f"declaration glob enters excluded capture tree in {path}: {declaration}"
                    )
        for package in (
            manifest.get("package", {}),
            manifest.get("workspace", {}).get("package", {}),
        ):
            for key in ("readme", "license-file"):
                name = package.get(key)
                if isinstance(name, str):
                    declared = (path.parent / name).resolve()
                    if Path(name).is_absolute() or not declared.resolve().is_relative_to(root):
                        raise ValueError(f"unsupported escaping {key} in {path}: {name}")
                    if declared.is_file():
                        declarations.add(declared)
                        inputs[declared] = declared.read_bytes()
        sources = []
        for key in ("lib", "bin", "test", "example", "bench"):
            targets = manifest.get(key, [])
            if isinstance(targets, dict):
                targets = [targets]
            sources.extend(target["path"] for target in targets if "path" in target)
        build = manifest.get("package", {}).get("build")
        if isinstance(build, str):
            sources.append(build)
        for name in sources:
            source = (path.parent / name).resolve()
            if source.is_file() and source not in declarations:
                inputs[path.parent / name] = source.read_bytes() if source_bodies else b""
    for parent in [root, *root.parents]:
        for name in ("config", "config.toml"):
            path = parent / ".cargo" / name
            if path.is_symlink():
                raise ValueError(f"unsupported symlink Cargo configuration: {path}")
            if path.is_file():
                inputs[path] = path.read_bytes()
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    if not cargo_home.is_absolute():
        raise ValueError("relative CARGO_HOME cannot be relocated faithfully")
    for name in ("config", "config.toml"):
        path = cargo_home / name
        if path.is_symlink():
            raise ValueError(f"unsupported symlink Cargo-home configuration: {path}")
        if path.is_file():
            inputs[path.resolve()] = path.read_bytes()
    for path, data in inputs.items():
        if path.parent.name == ".cargo" and path.name in {"config", "config.toml"}:
            config = tomllib.loads(data.decode())
            if any(key in config for key in ("include", "paths", "patch")) or any(
                any(key in source for key in ("directory", "local-registry"))
                for source in config.get("source", {}).values()
            ):
                raise ValueError(f"unsupported path-dependent Cargo configuration: {path}")
            target = config.get("build", {}).get("target", [])
            targets = [target] if isinstance(target, str) else target
            if any(
                value.endswith(".json") or "/" in value or "\\" in value for value in targets
            ) or any(
                isinstance(value, dict) and value.get("relative")
                for value in config.get("env", {}).values()
            ):
                raise ValueError(f"unsupported target/env path Cargo configuration: {path}")
    target = os.environ.get("CARGO_BUILD_TARGET", "")
    if target.endswith(".json") or "/" in target or "\\" in target:
        raise ValueError("path-dependent CARGO_BUILD_TARGET cannot be relocated faithfully")
    if root / "Cargo.toml" not in inputs or root / "Cargo.lock" not in inputs:
        raise ValueError("Cargo.toml or Cargo.lock missing; prepare the workspace explicitly")
    return inputs


def hakari(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just build-features"
    generate = ("cargo", "hakari", "generate", "--diff")
    manage = ("cargo", "hakari", "manage-deps", "--dry-run")
    root = root.resolve()
    conservative = False
    try:
        try:
            captured = metadata_inputs(root)
        except UnsupportedShape:
            conservative = True
            captured = metadata_inputs(root, source_bodies=True, conservative=True)
        with tempfile.TemporaryDirectory(prefix="lctx-fresh-hakari-") as temporary:
            owned = Path(temporary)
            workspace = owned / root.relative_to(root.anchor)
            for directory in captured.directories:
                (owned / directory.relative_to(directory.anchor)).mkdir(parents=True, exist_ok=True)
            cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).resolve()
            home_configs = {cargo_home / "config", cargo_home / "config.toml"}
            for source, data in captured.items():
                # Cargo still reads its real home configuration. Mirroring that same
                # file into a capture ancestor would apply array settings twice.
                if source in home_configs:
                    continue
                destination = owned / source.relative_to(source.anchor)
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
            intended_configs = {
                owned / source.relative_to(source.anchor)
                for source in captured
                if source.parent.name == ".cargo"
                and source.name in {"config", "config.toml"}
                and source not in home_configs
            }
            for parent in [workspace, *workspace.parents]:
                for name in ("config", "config.toml"):
                    config = parent / ".cargo" / name
                    if (
                        config.exists()
                        and config not in intended_configs
                        and config not in home_configs
                    ):
                        raise ValueError(
                            f"capture inherits additional Cargo configuration: {config}"
                        )
            environment = {
                "CARGO_NET_OFFLINE": "true",
                "RUSTUP_AUTO_INSTALL": "0",
                "CARGO_TARGET_DIR": str(owned / "target"),
                "CARGO_BUILD_BUILD_DIR": str(owned / "intermediates"),
            }
            metadata = ("cargo", "metadata", "--offline", "--locked", "--format-version", "1")

            def observe_metadata(cwd):
                ran = runner(metadata, cwd=cwd, environment=environment)
                if ran.returncode:
                    raise ValueError(
                        f"conservative Cargo metadata oracle unavailable (exit {ran.returncode}): "
                        f"{ran.stderr.strip()}"
                    )
                return json.loads(
                    ran.stdout.replace(cwd.as_uri(), "<WORKSPACE_URI>").replace(
                        str(cwd), "<WORKSPACE>"
                    )
                )

            original_metadata = observe_metadata(root) if conservative else None
            if conservative and observe_metadata(workspace) != original_metadata:
                raise ValueError(
                    "conservative capture differs from actual Cargo metadata; "
                    "unsupported input layout"
                )
            diff = runner(generate, cwd=workspace, environment=environment)
            deps = runner(manage, cwd=workspace, environment=environment)
            metadata_unchanged = not conservative or observe_metadata(root) == original_metadata
            normalized_lock = (workspace / "Cargo.lock").read_bytes() != captured[
                root / "Cargo.lock"
            ]
    except (OSError, ValueError, TypeError, KeyError) as error:
        return Freshness("hakari", "not_run", regenerate, f"metadata capture unavailable: {error}")
    try:
        unchanged = (
            metadata_unchanged
            and metadata_inputs(root, source_bodies=conservative, conservative=conservative)
            == captured
        )
    except OSError, ValueError, TypeError, KeyError:
        unchanged = False
    if not unchanged:
        return Freshness(
            "hakari", "stale", regenerate, "inputs changed during observation; inconclusive"
        )
    details = []
    for argv, ran in ((generate, diff), (manage, deps)):
        # Hakari uses 1 for drift AND metadata errors; only an actual diff/edit
        # observation establishes stale. Offline resolution failure is a non-result.
        if ran.returncode not in (0, 1) or (
            ran.returncode == 1
            and ("error:" in ran.stderr.lower() or "error:" in ran.stdout.lower())
        ):
            result = _failed("hakari", regenerate, argv, ran)
            result.reason += (
                "; prepare the pinned tools/locked offline dependencies explicitly, "
                "then rerun just fresh hakari"
            )
            result.details = (ran.stderr or ran.stdout).strip().splitlines()[:8]
            return result
    if diff.returncode == 0 and deps.returncode == 0 and not normalized_lock:
        return Freshness("hakari", "clean", regenerate, "workspace-hack matches", details=details)
    reasons = []
    if normalized_lock:
        reasons.append("Cargo metadata normalized the captured lockfile (live file unchanged)")
    if diff.returncode == 1:
        changed = [
            line
            for line in (diff.stdout + diff.stderr).splitlines()
            if line[:1] in "+-" and not line.startswith(("+++", "---")) and "=" in line
        ]
        reasons.append(f"workspace-hack Cargo.toml differs ({len(changed)} dependency lines)")
        details += changed
    if deps.returncode == 1:
        reasons.append("workspace crates' workspace-hack dependencies need changes")
        details += [line for line in (deps.stdout + deps.stderr).splitlines() if line.strip()]
    return Freshness(
        "hakari",
        "stale",
        regenerate,
        "; ".join(reasons),
        files=["crates/lctx-workspace-hack/Cargo.toml"] if diff.returncode == 1 else [],
        details=details,
    )


def docs(runner: Runner, root: Path = ROOT) -> Freshness:
    import docs as publisher

    state, reason = publisher.observe_publication(root)
    return Freshness("docs", state, "just docs-check", reason)


def adr_index(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just adr-index"
    try:
        sys.path.insert(0, str(root / "scripts"))
        import adr

        expected = adr.render_index(root, adr.load_all(root))
    except Exception as error:  # an unreadable record set cannot be judged here
        return Freshness("adr-index", "not_run", regenerate, f"could not render: {error}")
    index = adr.adr_dir(root) / "README.md"
    if index.exists() and index.read_text() == expected:
        return Freshness("adr-index", "clean", regenerate, "docs/adr/README.md matches")
    return Freshness(
        "adr-index",
        "stale",
        regenerate,
        "docs/adr/README.md differs from the records",
        files=["docs/adr/README.md"],
    )


def rust_fmt(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just fmt"
    argv = ("cargo", "fmt", "--check", "--message-format", "short")
    ran = runner(argv)
    if ran.returncode == 0:
        return Freshness("rust-fmt", "clean", regenerate, "cargo fmt --check passes")
    files = sorted({_relative(line.strip(), root) for line in ran.stdout.splitlines() if line})
    if ran.returncode == 1 and files:
        return Freshness(
            "rust-fmt", "stale", regenerate, f"{len(files)} file(s) unformatted", files=files
        )
    return _failed("rust-fmt", regenerate, argv, ran)


def python_fmt(runner: Runner, root: Path = ROOT) -> Freshness:
    regenerate = "just fmt"
    argv = ("uv", "run", "--no-sync", "ruff", "format", "--check", "--output-format", "json")
    ran = runner(argv)
    if ran.returncode == 0:
        return Freshness("python-fmt", "clean", regenerate, "ruff format --check passes")
    try:
        found = json.loads(ran.stdout)
    except ValueError:
        found = None
    if ran.returncode == 1 and isinstance(found, list) and found:
        files = sorted({_relative(item["filename"], root) for item in found})
        return Freshness(
            "python-fmt", "stale", regenerate, f"{len(files)} file(s) unformatted", files=files
        )
    return _failed("python-fmt", regenerate, argv, ran)


def gold(runner: Runner, root: Path = ROOT, skill: Path = GOLD_SKILL) -> Freshness:
    regenerate = (
        "uv run --no-project --offline --no-python-downloads python scripts/gold_extract.py "
        "for the extract; a pin mismatch needs libraries/fastmcp and the central FastMCP "
        "evaluation reference aligned"
    )
    if not (skill / "fastmcp.json").exists():
        return Freshness(
            "gold", "not_run", regenerate, "the central FastMCP evaluation reference is unavailable"
        )
    argv = (sys.executable, str(root / "scripts" / "check_gold.py"))
    ran = runner(argv)
    lines = [line for line in ran.stdout.splitlines() if line.strip()]
    if "not_run" in ran.stdout:
        return Freshness("gold", "not_run", regenerate, "check_gold.py reported not_run", lines)
    if ran.returncode == 0:
        return Freshness("gold", "clean", regenerate, "gold and libraries/fastmcp agree")
    if ran.returncode == 1:
        return Freshness("gold", "stale", regenerate, "check_gold.py found drift", details=lines)
    return _failed("gold", regenerate, argv, ran)


def _snapshot_source(snapshot: Path) -> str | None:
    try:
        with snapshot.open(encoding="utf-8") as handle:
            if handle.readline().strip() != "---":
                return None
            for line in handle:
                if line.strip() == "---":
                    return None
                if line.startswith("source:"):
                    return line.split(":", 1)[1].strip().strip('"')
    except OSError:
        return None
    return None


def _crate_dir(path: Path, root: Path) -> Path:
    for parent in path.parents:
        if (parent / "Cargo.toml").exists() or parent == root:
            return parent
    return root


def insta(runner: Runner, root: Path = ROOT) -> list[Freshness]:
    """Snapshots: `not_run` (only tests decide) plus a heuristic orphaned-source hint."""
    regenerate = (
        "run the owning tests with INSTA_UPDATE=no; read each .snap.new, then `cargo insta accept`"
    )
    argv = ("git", "ls-files", "-co", "--exclude-standard", "--", "*.snap", "*.snap.new")
    ran = runner(argv)
    if ran.returncode != 0:
        return [_failed("insta", regenerate, argv, ran)]
    paths = [line for line in ran.stdout.splitlines() if line]
    snaps = [p for p in paths if p.endswith(".snap")]
    pending = sorted(p for p in paths if p.endswith(".snap.new"))
    orphans = []
    for relative in snaps:
        source = _snapshot_source(root / relative)
        if source is None:
            continue
        candidates = (root / source, _crate_dir(root / relative, root) / source)
        if not any(candidate.exists() for candidate in candidates):
            orphans.append(f"{relative} (source: {source})")
    pending_note = f"; {len(pending)} pending .snap.new" if pending else ""
    results = [
        Freshness(
            "insta",
            "not_run",
            regenerate,
            f"{len(snaps)} snapshot(s), {INSTA_NOT_RUN}{pending_note}",
            files=pending,
        )
    ]
    reason = (
        f"{len(orphans)} snapshot(s) name a source file that no longer exists"
        if orphans
        else "every snapshot's source file exists (this does not show the content is current)"
    )
    results.append(
        Freshness(
            "insta-sources",
            "heuristic",
            "delete an orphaned snapshot, or rerun its moved test",
            reason,
            files=orphans,
        )
    )
    return results


CHECKS: dict[str, Callable[..., Freshness | list[Freshness]]] = {
    "hakari": hakari,
    "docs": docs,
    "adr-index": adr_index,
    "rust-fmt": rust_fmt,
    "python-fmt": python_fmt,
    "gold": gold,
    "insta": insta,
}
ALIASES = {"fmt": ("rust-fmt", "python-fmt"), "insta-sources": ("insta",)}


def select(names: Sequence[str]) -> list[str]:
    if not names:
        return list(CHECKS)
    chosen: list[str] = []
    for name in names:
        expanded = ALIASES.get(name, (name,))
        for item in expanded:
            if item not in CHECKS:
                raise KeyError(name)
            if item not in chosen:
                chosen.append(item)
    return chosen


def check(names: Sequence[str], runner: Runner = run) -> list[Freshness]:
    chosen = select(names)
    with ThreadPoolExecutor(max_workers=len(chosen)) as pool:
        futures = [pool.submit(CHECKS[name], runner) for name in chosen]
        results: list[Freshness] = []
        for future in futures:
            outcome = future.result()
            results.extend(outcome if isinstance(outcome, list) else [outcome])
    return results


def render(results: Sequence[Freshness], *, limit: int = 20) -> str:
    lines = []
    for result in results:
        lines.append(f"{result.output:<14}{result.state:<10}{result.reason}")
        shown = result.files[:limit]
        lines += [f"{'':<24}{path}" for path in shown]
        if len(result.files) > limit:
            lines.append(f"{'':<24}… {len(result.files) - limit} more (--all or --json)")
        if result.state != "clean":
            details = result.details[:5]
            lines += [f"{'':<24}| {detail}" for detail in details]
            if len(result.details) > 5:
                lines.append(f"{'':<24}| … {len(result.details) - 5} more (--json)")
            lines.append(f"{'':<24}regenerate: {result.regenerate}")
    stale = [r.output for r in results if r.state == "stale"]
    lines.append(f"fresh: {len(stale)} stale" + (f" ({', '.join(stale)})" if stale else ""))
    return "\n".join(lines)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="fresh", description=__doc__.split("\n\n")[0])
    parser.add_argument("outputs", nargs="*", help=f"any of {', '.join([*CHECKS, *ALIASES])}")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--all", action="store_true", help="list every file")
    args = parser.parse_args(argv)
    try:
        results = check(args.outputs)
    except KeyError as error:
        print(f"fresh: unknown output {error}; choose from {', '.join(CHECKS)}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps([asdict(r) for r in results], indent=2))
    else:
        print(render(results, limit=10**9 if args.all else 20))
    return 1 if any(r.state == "stale" for r in results) else 0


if __name__ == "__main__":
    sys.exit(main())
