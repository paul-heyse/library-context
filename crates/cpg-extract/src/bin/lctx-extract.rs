//! `lctx-extract`: run one extraction and write each raw table as `<out>/<table>.arrow`.
//!
//! Usage: lctx-extract --release-root DIR --venv-root DIR --site-packages DIR [--site-packages DIR]
//!                     --release-label TEXT --snapshot HEX32 --out DIR [--python 3.14.0] [--platform linux]

use std::path::PathBuf;
use std::process::ExitCode;

use cpg_extract::{ExtractInput, extract, write_ipc};
use cpg_schema::id::{Id, IdHasher, kind};

fn parse_hex_id(s: &str) -> Option<Id> {
    if s.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(Id(out))
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut release_root = None;
    let mut venv_root = None;
    let mut site_packages = Vec::new();
    let mut label = None;
    let mut snapshot = None;
    let mut out = None;
    let mut python = (3, 14, 0);
    let mut platform = "linux".to_owned();
    while let Some(flag) = args.next() {
        let value = args.next();
        match (flag.as_str(), value) {
            ("--release-root", Some(v)) => release_root = Some(PathBuf::from(v)),
            ("--venv-root", Some(v)) => venv_root = Some(PathBuf::from(v)),
            ("--site-packages", Some(v)) => site_packages.push(PathBuf::from(v)),
            ("--release-label", Some(v)) => label = Some(v),
            ("--snapshot", Some(v)) => snapshot = parse_hex_id(&v),
            ("--out", Some(v)) => out = Some(PathBuf::from(v)),
            ("--platform", Some(v)) => platform = v,
            ("--python", Some(v)) => {
                let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
                if let [a, b, c] = parts[..] {
                    python = (a, b, c);
                }
            }
            (other, _) => {
                eprintln!("lctx-extract: unknown or incomplete argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    let (Some(release_root), Some(venv_root), Some(label), Some(snapshot), Some(out)) =
        (release_root, venv_root, label, snapshot, out)
    else {
        eprintln!("lctx-extract: missing required arguments (see the module docs)");
        return ExitCode::from(2);
    };
    let input = ExtractInput {
        release_root,
        venv_root,
        site_packages,
        python_version: python,
        python_platform: platform,
        release_id: IdHasher::new(kind::RELEASE).str(&label).finish_id(),
        snapshot_id: snapshot,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    };
    match extract(&input).and_then(|output| write_ipc(&out, &output)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lctx-extract: {e}");
            ExitCode::from(1)
        }
    }
}
