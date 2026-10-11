"""Compare actual full Cargo/Hakari output using today's capture owner, in owned copies only."""
import argparse
import gzip
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib


def main():
    p=argparse.ArgumentParser();p.add_argument("--repository",type=Path,required=True);p.add_argument("--scratch",type=Path,required=True);p.add_argument("--output",type=Path,required=True);args=p.parse_args()
    root=args.repository.resolve();sys.path.insert(0,str(root/"scripts"))
    from freshness import metadata_inputs
    captured=metadata_inputs(root)
    flags=tomllib.loads((root/".cargo/config.toml").read_text())["target"]["cfg(target_os = \"linux\")"]["rustflags"]
    results={"date":"2026-10-11","root":str(root),"scope":"actual full Cargo metadata and Hakari read-only commands in full-body and placeholder captures","inputs":len(captured),"full_bytes":sum(len(b) for b in captured.values()),"encoded_rustflags":flags,"isolation":"exact configured Linux rustflags bound through CARGO_ENCODED_RUSTFLAGS to avoid duplicate outer-worktree and relocated config array inheritance","commands":{}}
    args.scratch.mkdir(parents=True,exist_ok=True);args.output.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="hakari-",dir=args.scratch) as tmp:
        owner=Path(tmp)
        for variant in ["full","placeholder"]:
            owned=owner/variant;workspace=owned/root.relative_to(root.anchor)
            for source,data in captured.items():
                destination=owned/source.relative_to(source.anchor);destination.parent.mkdir(parents=True,exist_ok=True)
                destination.write_bytes(b"" if variant=="placeholder" and source.suffix==".rs" else data)
            env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(owned/"target"),CARGO_BUILD_BUILD_DIR=str(owned/"intermediates"),CARGO_NET_OFFLINE="true",RUSTUP_AUTO_INSTALL="0",CARGO_TERM_COLOR="never",NO_COLOR="1",CARGO_ENCODED_RUSTFLAGS="\x1f".join(flags))
            records=[]
            for label,argv in [("metadata",["cargo","metadata","--offline","--format-version","1"]),("generate",["cargo","hakari","generate","--diff"]),("manage",["cargo","hakari","manage-deps","--dry-run"]),("generate-drift",["cargo","hakari","generate","--diff"]),("manage-missing-cli",["cargo","hakari","manage-deps","--dry-run"])]:
                if label=="generate-drift":
                    path=workspace/"crates/lctx-workspace-hack/Cargo.toml";saved=path.read_bytes()
                    path.write_text("\n".join(line for line in saved.decode().splitlines() if not line.startswith("ahash ="))+"\n")
                elif label=="manage-missing-cli":
                    path=workspace/"crates/lctx/Cargo.toml";saved=path.read_bytes()
                    path.write_text("\n".join(line for line in saved.decode().splitlines() if not line.startswith("lctx-workspace-hack ="))+"\n")
                done=subprocess.run(argv,cwd=workspace,env=env,capture_output=True,text=True)
                if label in {"generate-drift","manage-missing-cli"}:path.write_bytes(saved)
                stdout=done.stdout.replace(str(owned),"<CAPTURE>");stderr=done.stderr.replace(str(owned),"<CAPTURE>")
                if label=="metadata" and done.returncode==0:
                    # Every package, resolve node/edge, feature, target and manifest is compared.
                    stdout=json.loads(stdout)
                record={"label":label,"argv":argv,"returncode":done.returncode,"stdout":stdout,"stderr":stderr}
                raw=args.output.parent/f"hakari-{variant}-{label}.json.gz"
                with gzip.open(raw,"wt") as handle:json.dump(record,handle,sort_keys=True)
                records.append(record)
            results["commands"][variant]=records
        results["stable_inputs"]=metadata_inputs(root)==captured
        comparisons=[]
        for full,placeholder in zip(results["commands"]["full"],results["commands"]["placeholder"]):
            label=full["label"]
            # Warnings/progress can differ on first offline resolution but semantic output cannot.
            same=full["returncode"]==placeholder["returncode"] and full["stdout"]==placeholder["stdout"] and full["stderr"]==placeholder["stderr"]
            valid=full["returncode"] in (0,1) and not (full["returncode"]==1 and "error:" in full["stderr"].lower())
            if label=="generate-drift":valid=valid and full["returncode"]==1 and "ahash" in str(full["stdout"])+full["stderr"]
            if label=="manage-missing-cli":valid=valid and (full["returncode"]==1 or "lctx" in full["stdout"])
            comparisons.append({"command":label,"equivalent_stdout_and_exit":same,"valid_result":valid,"full_returncode":full["returncode"],"placeholder_returncode":placeholder["returncode"],"full_stderr":full["stderr"],"placeholder_stderr":placeholder["stderr"]})
        results["comparisons"]=comparisons
        del results["commands"]
        results["outcome"]="passed" if results["stable_inputs"] and all(c["equivalent_stdout_and_exit"] and c["valid_result"] for c in comparisons) else "blocked"
    args.output.write_text(json.dumps(results,indent=2)+"\n")
    print(json.dumps({"outcome":results["outcome"],"inputs":len(captured),"comparison":comparisons,"output":str(args.output)}))
    raise SystemExit(results["outcome"]!="passed")

if __name__=="__main__":main()
