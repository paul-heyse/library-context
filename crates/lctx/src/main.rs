//! `lctx`: the one production path from a pinned Python library to a published snapshot (DESIGN
//! §4.0, ADR-0013). Every analyzed library is a committed uv project under `libraries/<name>/`.
//!
//! ```text
//! lctx library init <name> --requirement REQ [--python 3.14.7]   write, lock, acquire, propose release
//! lctx acquire <name> [--reinstall]                              uv sync --frozen into build/envs/<name>
//! lctx compile <name> --store DIR                                acquire, Stage A, extract, derive,
//!                                                                validate, publish
//! lctx query --store DIR --snapshot HEX "SQL"                    read-only SQL over a published
//!                                                                snapshot (its tables by name)
//! ```
//! Common options: `--libraries DIR` (default `libraries`), `--envs DIR` (default `build/envs`).
//! Upgrading a library: edit its pin, `uv lock --project libraries/<name> --upgrade-package <dist>`,
//! then `lctx compile <name>`.

mod propose;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use cpg_extract::{extract, library};
use cpg_schema::id::Id;

struct Options {
    command: Vec<String>,
    libraries: PathBuf,
    envs: PathBuf,
    store: Option<PathBuf>,
    requirement: Option<String>,
    python: String,
    reinstall: bool,
    snapshot: Option<String>,
}

fn absolute(p: &Path) -> Result<PathBuf, String> {
    std::path::absolute(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn parse() -> Result<Options, String> {
    let mut o = Options {
        command: Vec::new(),
        libraries: PathBuf::from("libraries"),
        envs: PathBuf::from("build/envs"),
        store: None,
        requirement: None,
        python: "3.14.7".to_owned(),
        reinstall: false,
        snapshot: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--libraries" => o.libraries = PathBuf::from(value()?),
            "--envs" => o.envs = PathBuf::from(value()?),
            "--store" => o.store = Some(PathBuf::from(value()?)),
            "--requirement" => o.requirement = Some(value()?),
            "--python" => o.python = value()?,
            "--reinstall" => o.reinstall = true,
            "--snapshot" => o.snapshot = Some(value()?),
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ => o.command.push(arg),
        }
    }
    o.libraries = absolute(&o.libraries)?;
    o.envs = absolute(&o.envs)?;
    Ok(o)
}

/// Run uv for one library, with the environment pinned to `env_dir` and every other `UV_*`
/// setting and `VIRTUAL_ENV` removed, so nothing ambient steers resolution or installation.
fn uv(args: &[&str], env_dir: &Path) -> Result<(), String> {
    let mut command = Command::new("uv");
    command.args(args);
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy().into_owned();
        if key.starts_with("UV_") || key == "VIRTUAL_ENV" {
            command.env_remove(key);
        }
    }
    command.env("UV_PROJECT_ENVIRONMENT", env_dir);
    let status = command
        .status()
        .map_err(|e| format!("blocked: `uv` could not run ({e})"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("uv {} failed ({status})", args.join(" ")))
    }
}

/// `uv sync --frozen` into the library's environment, ignoring user and system uv configuration
/// (`--no-config`), on the interpreter `.python-version` pins, and copying files rather than
/// hard-linking them from the uv cache, which other environments share (ADR-0013 review F3).
/// `reinstall` rebuilds every package: the remedy when Stage A finds a changed file.
fn acquire(library_dir: &Path, env_dir: &Path, reinstall: bool) -> Result<(), String> {
    if !library_dir.join("uv.lock").exists() {
        return Err(format!(
            "{} has no uv.lock; run `lctx library init` or `uv lock --project {}`",
            library_dir.display(),
            library_dir.display()
        ));
    }
    let python = std::fs::read_to_string(library_dir.join(".python-version"))
        .map_err(|e| format!("{}/.python-version: {e}", library_dir.display()))?;
    let project = library_dir.to_string_lossy();
    let mut args = vec![
        "sync",
        "--project",
        &project,
        "--frozen",
        "--no-install-project",
        "--no-config",
        "--python",
        python.trim(),
        "--link-mode",
        "copy",
    ];
    if reinstall {
        args.push("--reinstall");
    }
    uv(&args, env_dir)
}

fn random_id() -> Result<Id, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| format!("randomness: {e}"))?;
    Ok(Id(bytes))
}

fn compile(library_dir: &Path, env_dir: &Path, store: &Path) -> Result<(), String> {
    let started = Instant::now();
    acquire(library_dir, env_dir, false)?;
    let acquired = started.elapsed();
    let snapshot = random_id()?;
    let input = library::acquired(library_dir, env_dir, snapshot).map_err(|e| e.to_string())?;
    let staged = started.elapsed() - acquired;
    println!(
        "release {} ({} modules)",
        input.release.release_id.hex(),
        input.release.files.len()
    );
    let output = extract(&input).map_err(|e| e.to_string())?;
    let extracted = started.elapsed();
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let published = runtime
        .block_on(cpg_core::attempt::compile(store, snapshot, &output.tables))
        .map_err(|e| e.to_string())?;
    println!("snapshot {} published", published.snapshot_id.hex());
    println!("content  {}", published.content_digest.hex());
    for (name, rows) in &published.rows {
        println!(
            "  {name:<20} {rows:>7} rows  v{}",
            published.versions[*name]
        );
    }
    println!("stages (wall time, peak RSS so far):");
    println!(
        "  {:<52} {:>7.2}s",
        "acquire (uv sync --frozen)",
        acquired.as_secs_f64()
    );
    println!(
        "  {:<52} {:>7.2}s",
        "Stage A (verify RECORDs)",
        staged.as_secs_f64()
    );
    for stage in output.stages.iter().chain(&published.stages) {
        println!(
            "  {:<52} {:>7.2}s  {:>6} MiB",
            stage.name,
            stage.seconds,
            stage
                .peak_rss_bytes
                .map_or("?".to_owned(), |b| (b >> 20).to_string())
        );
    }
    println!(
        "extract {:.1}s, total {:.1}s",
        extracted.as_secs_f64(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Write `libraries/<name>/`, lock it, acquire it, and propose `[tool.lctx] release` as the
/// requested distribution plus every installed distribution sharing a source repository with it.
fn init(name: &str, o: &Options) -> Result<(), String> {
    let requirement = o
        .requirement
        .as_deref()
        .ok_or("library init needs --requirement")?;
    let dist = library::requirement_name(requirement);
    let (major, minor) = {
        let mut parts = o.python.split('.');
        (parts.next().unwrap_or("3"), parts.next().unwrap_or("14"))
    };
    let library_dir = o.libraries.join(name);
    if library_dir.exists() {
        return Err(format!("{} already exists", library_dir.display()));
    }
    std::fs::create_dir_all(&library_dir).map_err(|e| e.to_string())?;
    let write = |release: &[String]| {
        let release = release
            .iter()
            .map(|r| format!("\"{r}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            library_dir.join("pyproject.toml"),
            format!(
                "# {name} as an analyzed library (DESIGN §4.0, ADR-0013), written by `lctx library \
                 init`.\n[project]\nname = \"lctx-library-{name}\"\nversion = \"0\"\n\
                 requires-python = \"=={major}.{minor}.*\"\ndependencies = [\"{requirement}\"]\n\n\
                 [tool.uv]\npackage = false\n\n[tool.lctx]\n# The first-party distributions whose \
                 code is compiled. Proposed from shared source\n# repositories: review before \
                 compiling.\nrelease = [{release}]\n"
            ),
        )
        .map_err(|e| e.to_string())
    };
    write(std::slice::from_ref(&dist))?;
    std::fs::write(
        library_dir.join(".python-version"),
        format!("{}\n", o.python),
    )
    .map_err(|e| e.to_string())?;
    let env_dir = o.envs.join(name);
    uv(
        &["lock", "--project", &library_dir.to_string_lossy()],
        &env_dir,
    )?;
    acquire(&library_dir, &env_dir, false)?;
    let site = std::fs::read_dir(env_dir.join("lib"))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path().join("site-packages")))
        .find(|p| p.is_dir())
        .ok_or("the acquired environment has no site-packages")?;
    let installed: Vec<(String, String)> = std::fs::read_dir(&site)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter_map(|p| {
            let stem = p
                .file_name()?
                .to_str()?
                .strip_suffix(".dist-info")?
                .to_owned();
            let metadata = std::fs::read_to_string(p.join("METADATA")).unwrap_or_default();
            Some((library::normalize(stem.rsplit_once('-')?.0), metadata))
        })
        .collect();
    let (release, found) = propose::propose(&dist, &installed);
    write(&release)?;
    println!(
        "{} written and locked; proposed release = {release:?}{}. Review it, then commit the \
         directory (pyproject.toml, .python-version, uv.lock).",
        library_dir.display(),
        if found {
            ""
        } else {
            " (the distribution names no source repository, so it is proposed alone)"
        }
    );
    Ok(())
}

/// Read-only SQL over one published snapshot: every table registered under its own name, at its
/// recorded version, filtered to the snapshot (DESIGN §6.2).
fn query(store: &Path, hex: &str, sql: &str) -> Result<(), String> {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("zz"), 16))
        .collect::<Result<_, _>>()
        .map_err(|_| format!("--snapshot {hex}: not hex"))?;
    let id = Id(bytes
        .try_into()
        .map_err(|_| format!("--snapshot {hex}: not 16 bytes"))?);
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let (_, ctx) = cpg_core::snapshot::published(store, id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("snapshot {hex} is not published in {}", store.display()))?;
        let text = cpg_core::sql::render(&ctx, sql)
            .await
            .map_err(|e| e.to_string())?;
        println!("{text}");
        Ok(())
    })
}

fn run() -> Result<(), String> {
    let o = parse()?;
    let words: Vec<&str> = o.command.iter().map(String::as_str).collect();
    match words[..] {
        ["library", "init", name] => init(name, &o),
        ["acquire", name] => acquire(&o.libraries.join(name), &o.envs.join(name), o.reinstall),
        ["query", sql] => {
            let store = absolute(o.store.as_deref().ok_or("query needs --store DIR")?)?;
            let hex = o.snapshot.as_deref().ok_or("query needs --snapshot HEX")?;
            query(&store, hex, sql)
        }
        ["compile", name] => {
            let store = absolute(o.store.as_deref().ok_or("compile needs --store DIR")?)?;
            compile(&o.libraries.join(name), &o.envs.join(name), &store)
        }
        _ => Err(
            "usage: lctx library init <name> --requirement REQ | lctx acquire <name> | \
                  lctx compile <name> --store DIR | \
                  lctx query --store DIR --snapshot HEX \"SQL\""
                .to_owned(),
        ),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lctx: {e}");
            ExitCode::from(1)
        }
    }
}
