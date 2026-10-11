"""Independent Cargo oracle for a metadata-only capture candidate; no repository generator.

Run with Python 3.11+, --scratch under the owned decision worktree and --output evidence JSON.
Temporary input trees belong to this probe and are removed only by its TemporaryDirectory.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib


def write(root, name, content=""):
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def candidate(source, destination):
    # Preserve all directory/file names, not only .rs: explicit targets can use arbitrary suffixes.
    # Refuse symlinks and escaping declarations rather than inventing relocated path semantics.
    def declarations(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key in {"path", "workspace"} and isinstance(child, str): yield child
                elif key in {"members", "default-members", "exclude"} and isinstance(child, list): yield from child
                else: yield from declarations(child)
        elif isinstance(value, list):
            for child in value: yield from declarations(child)
    for manifest in source.rglob("Cargo.toml"):
        for declaration in declarations(tomllib.loads(manifest.read_text())):
            if Path(declaration).is_absolute() or not (manifest.parent / declaration).resolve().is_relative_to(source):
                raise ValueError("escaping Cargo declaration")
    for path in source.rglob("*"):
        if path.is_symlink():
            raise ValueError("symlink input")
        target = destination / path.relative_to(source)
        if path.is_dir():
            target.mkdir(parents=True, exist_ok=True)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            retained = path.name in {"Cargo.toml", "Cargo.lock", "config", "config.toml", "hakari.toml", "rust-toolchain.toml"}
            target.write_bytes(path.read_bytes() if retained else b"")


def metadata(root):
    env = os.environ.copy()
    env.update(CARGO_TARGET_DIR=str(root / "target"), CARGO_NET_OFFLINE="true", RUSTUP_AUTO_INSTALL="0")
    completed = subprocess.run(["cargo", "metadata", "--offline", "--no-deps", "--format-version", "1"], cwd=root, env=env, text=True, capture_output=True)
    value = json.loads(completed.stdout) if completed.returncode == 0 else completed.stderr
    return completed.returncode, json.loads(json.dumps(value).replace(str(root), "<ROOT>"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.scratch.mkdir(parents=True, exist_ok=True)
    base = '[package]\nname="capture-probe"\nversion="0.1.0"\nedition="2024"\n'
    cases = []
    def case(name, files, directories=(), symlinks=()):
        with tempfile.TemporaryDirectory(prefix="metadata-", dir=args.scratch) as tmp:
            root = Path(tmp)
            original, captured = root / "original", root / "captured"
            original.mkdir(); captured.mkdir()
            for name_, body in files.items():
                if name_ == "Cargo.toml" and "[workspace]" not in body:
                    body += "\n[workspace]\n"
                write(original, name_, body)
            for directory in directories: (original / directory).mkdir(parents=True, exist_ok=True)
            for link, target in symlinks: (original / link).symlink_to(target)
            expected = metadata(original)
            try:
                candidate(original, captured)
                actual = metadata(captured)
                equal = expected == actual
                row = {"case": name, "outcome": "passed" if equal else "failed", "oracle_returncode": expected[0], "equal_full_metadata_or_error": equal, "oracle": expected, "candidate": actual}
            except ValueError as error:
                row = {"case": name, "outcome": "passed", "candidate": "refused", "reason": str(error), "oracle_returncode": expected[0], "oracle": expected}
            cases.append(row)
    case("automatic-file-and-directory-targets", {"Cargo.toml":base, "src/lib.rs":"INVALID RUST CONTENT", "src/main.rs":"fn main() {}", "src/bin/one.rs":"", "src/bin/two/main.rs":"", "tests/a.rs":"", "tests/b/main.rs":"", "examples/a.rs":"", "examples/b/main.rs":"", "benches/a.rs":"", "benches/b/main.rs":"", "build.rs":""})
    case("automatic-disabled-explicit-arbitrary-source", {"Cargo.toml":base+'autolib=false\nautobins=false\nautotests=false\nautoexamples=false\nautobenches=false\nbuild="custom/build.input"\n[[bin]]\nname="chosen"\npath="custom/source.input"\n', "src/lib.rs":"", "src/main.rs":"", "tests/a.rs":"", "custom/source.input":"non rust content", "custom/build.input":""})
    case("explicit-name-infers-auto-path", {"Cargo.toml":base+'[[bin]]\nname="one"\ntest=false\n', "src/lib.rs":"", "src/bin/one.rs":""})
    case("missing-explicit-root-valid-metadata", {"Cargo.toml":base+'[lib]\npath="missing/source.rs"\n'})
    case("directory-without-main-is-not-a-target", {"Cargo.toml":base,"src/lib.rs":"","tests/helpers/module.rs":""}, ["src/bin/ghost", "tests/empty"])
    case("missing-lib-and-bin-invalid-package", {"Cargo.toml":base}, ["src/bin/empty"])
    case("workspace-member-globs-and-exclusions", {"Cargo.toml":'[workspace]\nmembers=["packages/*"]\nexclude=["packages/excluded"]\nresolver="3"\n', "packages/one/Cargo.toml":base.replace("capture-probe","member-one"),"packages/one/src/lib.rs":"", "packages/excluded/Cargo.toml":base.replace("capture-probe","excluded"),"packages/excluded/src/lib.rs":""})
    case("workspace-glob-matches-invalid-directory", {"Cargo.toml":'[workspace]\nmembers=["packages/*"]\nresolver="3"\n'}, ["packages/empty"])
    case("target-cfg-optional-path-dependencies", {"Cargo.toml":base+'[target.\'cfg(unix)\'.dependencies]\nhelper={path="helper",optional=true}\n[workspace]\n',"src/lib.rs":"", "helper/Cargo.toml":base.replace("capture-probe","helper"),"helper/src/lib.rs":""})
    case("cargo-config-discovery-and-hakari-bytes", {"Cargo.toml":base, "src/lib.rs":"", ".cargo/config.toml":'[build]\ntarget-dir="other-target"\n', ".config/hakari.toml":'resolver="3"\n'})
    case("symlink-source-is-refused", {"Cargo.toml":base, "actual.rs":""}, ["src"], [("src/lib.rs", "../actual.rs")])
    case("symlink-directory-is-refused", {"Cargo.toml":base, "actual/lib.rs":""}, [], [("src", "actual")])
    case("escaping-valid-dependency-is-refused", {"Cargo.toml":base+'[dependencies]\nexternal={path="../external"}\n', "src/lib.rs":"", "../external/Cargo.toml":base.replace("capture-probe","external")+'[workspace]\n', "../external/src/lib.rs":""})
    case("escaping-missing-explicit-target-is-refused", {"Cargo.toml":base+'[lib]\npath="../outside.input"\n'})
    case("same-name-file-and-directory-discovery-error", {"Cargo.toml":base,"src/lib.rs":"","tests/one.rs":"","tests/one/main.rs":""})
    # Membership and declaration bytes are stable premises; changes invalidate capture even if
    # Cargo would ignore one particular added file. Body-only edits need not invalidate metadata.
    with tempfile.TemporaryDirectory(prefix="mutation-", dir=args.scratch) as tmp:
        source=Path(tmp); write(source,"Cargo.toml",base);write(source,"src/lib.rs","body1")
        def signature():
            return sorted((str(p.relative_to(source)), "directory" if p.is_dir() else p.read_text() if p.name=="Cargo.toml" else "file") for p in source.rglob("*"))
        before=signature();write(source,"src/lib.rs","body2")
        cases.append({"case":"body-only-edit-does-not-change-metadata-key","outcome":"passed" if signature()==before else "failed"})
        write(source,"tests/new.rs","")
        cases.append({"case":"concurrent-target-addition-invalidates","outcome":"passed" if signature()!=before else "failed"})
        before=signature();(source/"tests/new.rs").unlink()
        cases.append({"case":"concurrent-target-removal-invalidates","outcome":"passed" if signature()!=before else "failed"})
        before=signature();write(source,"Cargo.toml",base+'autotests=false\n')
        cases.append({"case":"concurrent-declaration-edit-invalidates","outcome":"passed" if signature()!=before else "failed"})
    version=subprocess.run(["cargo","--version"],capture_output=True,text=True,check=True).stdout.strip()
    result={"date":"2026-10-11","cargo":version,"scope":"actual Cargo metadata, no-deps local synthetic workspaces; not actual Hakari generation or whole-repository support","cases":cases}
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({"cases":len(cases),"failed":[r["case"] for r in cases if r["outcome"]!="passed"],"output":str(args.output)}))
    raise SystemExit(any(r["outcome"]!="passed" for r in cases))

if __name__ == "__main__": main()
