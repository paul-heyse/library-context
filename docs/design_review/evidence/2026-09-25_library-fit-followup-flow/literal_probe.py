"""Link the focused real-provider probe against the current cached release crates."""
from pathlib import Path
import json
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
deps = ROOT / "target/release/deps"
fingerprints = ROOT / "target/release/.fingerprint"
core = max(deps.glob("libcpg_core-*.rlib"), key=lambda p:p.stat().st_mtime)
core_hash = core.stem.removeprefix("libcpg_core-")
core_deps = json.loads((fingerprints / f"cpg-core-{core_hash}" / "lib-cpg_core.json").read_text())["deps"]
def dependency(name):
    wanted = next(d[3] for d in core_deps if d[1] == name)
    for fingerprint in fingerprints.glob(f"{name.replace('_','-')}-*/lib-{name}"):
        if int.from_bytes(bytes.fromhex(fingerprint.read_text().strip()),"little") == wanted:
            return deps / f"lib{name}-{fingerprint.parent.name.rsplit('-',1)[1]}.rlib"
    raise RuntimeError(f"no cached release artifact for {name}")
with tempfile.TemporaryDirectory(prefix="lctx-literal-review-") as temp:
    binary = Path(temp) / "literal"
    command = ["rustc", "--edition=2024", "-C", "linker=clang", "-C", "link-arg=-fuse-ld=mold", str(HERE / "literal_probe.rs"), "-L", f"dependency={deps}", "-o", str(binary)]
    for name in ("cpg_core", "cpg_extract", "cpg_schema", "tokio", "tempfile"):
        rlib = dependency(name) if name in ("cpg_schema", "tokio") else max(deps.glob(f"lib{name}-*.rlib"), key=lambda p:p.stat().st_mtime)
        print(name,rlib.name,flush=True)
        command.extend(["--extern",f"{name}={rlib}"])
    subprocess.run(command,cwd=ROOT,check=True)
    subprocess.run([str(binary)],cwd=ROOT,check=True)
