"""Compile unchanged repository modules against cached release dependencies; no Cargo mutation."""
from pathlib import Path
import hashlib
import subprocess
import tempfile

here = Path(__file__).resolve().parent
repo = here.parents[3]
deps = repo / "target/release/deps"
cmd = ["rustc", "--edition=2024", "--crate-name", "followup_kernel", str(here / "probe.rs"),
       "-C", "opt-level=2", "-C", "linker=clang", "-C", "link-arg=-fuse-ld=mold",
       "-L", f"dependency={deps}"]
for name in ("biodivine_lib_bdd", "blake3", "serde_json"):
    candidates = sorted(deps.glob(f"lib{name}-*.rlib"), key=lambda p: p.stat().st_mtime_ns)
    if not candidates:
        raise SystemExit(f"blocked: cached release dependency {name} missing")
    dep = candidates[-1]
    print(f"dependency {name}={dep.name}", flush=True)
    cmd += ["--extern", f"{name}={dep}"]
for name in ("condition_kernel", "condition", "codebook", "id"):
    path = repo / f"crates/cpg-schema/src/{name}.rs"
    print(f"sha256 {path.relative_to(repo)} {hashlib.sha256(path.read_bytes()).hexdigest()}", flush=True)
subprocess.run(["rustc", "--version"], cwd=repo, check=True)
with tempfile.TemporaryDirectory(prefix="lctx-followup-kernel-") as tmp:
    binary = Path(tmp) / "probe"
    subprocess.run(cmd + ["-o", str(binary)], cwd=repo, check=True)
    subprocess.run([str(binary)], cwd=repo, check=True)
