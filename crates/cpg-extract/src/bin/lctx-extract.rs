//! `lctx-extract`: run one extraction and write each raw table as `<out>/<table>.arrow`, for
//! debugging the raw facts. Production compiles go through `lctx compile` (ADR-0013).
//!
//! Usage, an acquired library (`lctx acquire` built the environment):
//!   lctx-extract --library DIR --env DIR --snapshot HEX32 --out DIR
//! or a source tree compiled under a label:
//!   lctx-extract --release-root DIR --release-label TEXT --venv-root DIR --site-packages DIR
//!                [--site-packages DIR] --snapshot HEX32 --out DIR [--python 3.14.0] [--platform linux]

use std::path::PathBuf;
use std::process::ExitCode;

use cpg_extract::{ExtractInput, Release, extract, library, write_ipc};
use cpg_schema::id::Id;

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

#[derive(Default)]
struct Args {
    library: Option<PathBuf>,
    env: Option<PathBuf>,
    release_root: Option<PathBuf>,
    label: Option<String>,
    venv_root: Option<PathBuf>,
    site_packages: Vec<PathBuf>,
    snapshot: Option<Id>,
    out: Option<PathBuf>,
    python: Option<(u32, u32, u32)>,
    platform: Option<String>,
}

fn parse() -> Result<Args, String> {
    let mut a = Args::default();
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--library" => a.library = Some(PathBuf::from(value)),
            "--env" => a.env = Some(PathBuf::from(value)),
            "--release-root" => a.release_root = Some(PathBuf::from(value)),
            "--release-label" => a.label = Some(value),
            "--venv-root" => a.venv_root = Some(PathBuf::from(value)),
            "--site-packages" => a.site_packages.push(PathBuf::from(value)),
            "--snapshot" => {
                a.snapshot = Some(parse_hex_id(&value).ok_or("--snapshot takes 32 hex digits")?);
            }
            "--out" => a.out = Some(PathBuf::from(value)),
            "--platform" => a.platform = Some(value),
            "--python" => {
                let parts: Vec<u32> = value.split('.').filter_map(|p| p.parse().ok()).collect();
                let [x, y, z] = parts[..] else {
                    return Err("--python takes MAJOR.MINOR.MICRO".to_owned());
                };
                a.python = Some((x, y, z));
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(a)
}

fn input(a: Args, snapshot: Id) -> Result<ExtractInput, String> {
    if let (Some(library_dir), Some(env)) = (a.library, a.env) {
        return library::acquired(&library_dir, &env, snapshot).map_err(|e| e.to_string());
    }
    let (Some(root), Some(label), Some(venv_root)) = (a.release_root, a.label, a.venv_root) else {
        return Err(
            "give --library and --env, or --release-root, --release-label and --venv-root"
                .to_owned(),
        );
    };
    Ok(ExtractInput {
        release: Release::from_tree(root, &label).map_err(|e| e.to_string())?,
        venv_root,
        site_packages: a.site_packages,
        python_version: a.python.unwrap_or((3, 14, 0)),
        python_platform: a.platform.unwrap_or_else(|| "linux".to_owned()),
        snapshot_id: snapshot,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: Default::default(),
    })
}

fn main() -> ExitCode {
    let run = || -> Result<(), String> {
        let mut a = parse()?;
        let snapshot = a.snapshot.take().ok_or("--snapshot is required")?;
        let out = a.out.take().ok_or("--out is required")?;
        let input = input(a, snapshot)?;
        let output = extract(&input).map_err(|e| e.to_string())?;
        write_ipc(&out, &output).map_err(|e| e.to_string())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lctx-extract: {e}");
            ExitCode::from(1)
        }
    }
}
