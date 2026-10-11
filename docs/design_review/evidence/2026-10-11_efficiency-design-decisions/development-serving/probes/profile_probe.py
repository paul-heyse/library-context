"""Build a tiny independent workspace under the owned worktree; observe actual rustc policy."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    p=argparse.ArgumentParser();p.add_argument("--repository",type=Path,required=True);p.add_argument("--scratch",type=Path,required=True);p.add_argument("--output",type=Path,required=True);args=p.parse_args()
    manifest=(args.repository/"Cargo.toml").read_text()
    profiles=manifest[manifest.index("[profile.dev]"):manifest.index("# Backport the upstream")]
    args.scratch.mkdir(parents=True,exist_ok=True)
    output={"date":"2026-10-11","scope":"tiny pure Rust profile policy, not product functional qualification","profiles_source":profiles,"commands":[]}
    with tempfile.TemporaryDirectory(prefix="profiles-",dir=args.scratch) as tmp:
        root=Path(tmp);(root/"src").mkdir();(root/"dep/src").mkdir(parents=True)
        (root/"Cargo.toml").write_text('[package]\nname="policy-probe"\nversion="0.1.0"\nedition="2024"\n[dependencies]\npolicy-dependency={path="dep"}\n[workspace]\nexclude=["dep"]\n'+profiles)
        (root/"dep/Cargo.toml").write_text('[package]\nname="policy-dependency"\nversion="0.1.0"\nedition="2024"\n')
        (root/"dep/src/lib.rs").write_text('pub fn imported() -> bool { cfg!(debug_assertions) }\n')
        (root/"src/lib.rs").write_text('''pub fn workspace() -> bool { cfg!(debug_assertions) }
#[cfg(test)] mod tests {
 #[test] fn assertion_policy() { assert_eq!(super::workspace(), cfg!(debug_assertions)); assert_eq!(policy_dependency::imported(), cfg!(debug_assertions)); }
 #[test] fn overflow_policy() { let max=std::hint::black_box(u8::MAX); let one=std::hint::black_box(1u8); let result=std::panic::catch_unwind(||max+one); assert_eq!(result.is_err(),cfg!(debug_assertions)); }
}
''')
        env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(root/"target"),CARGO_BUILD_BUILD_DIR=str(root/"intermediates"),CARGO_NET_OFFLINE="true",RUSTUP_AUTO_INSTALL="0")
        for command,profile in [("build","dev"),("test","test"),("test","release")]:
            argv=["cargo",command,"--offline","--profile",profile,"-vv"]
            done=subprocess.run(argv,cwd=root,env=env,capture_output=True,text=True)
            output["commands"].append({"argv":argv,"outcome":"passed" if done.returncode==0 else "failed","stdout":done.stdout.replace(str(root),"<ROOT>"),"stderr":done.stderr.replace(str(root),"<ROOT>")})
    args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(output,indent=2)+"\n")
    print(json.dumps({"outcomes":[r["outcome"] for r in output["commands"]],"output":str(args.output)}))
    raise SystemExit(any(r["outcome"]!="passed" for r in output["commands"]))

if __name__=="__main__":main()
