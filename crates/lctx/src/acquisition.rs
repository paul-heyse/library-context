//! Original acquired-input admission. Frozen capture does not retain this lease.
use anyhow::Context as _;
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

pub struct Lease {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    pub environment: PathBuf,
    pub source: Option<PathBuf>,
}

impl Lease {
    pub fn open(
        library: &Path,
        environment: &Path,
        sources: &Path,
        synchronize: bool,
        reinstall: bool,
        include_source: bool,
    ) -> anyhow::Result<Self> {
        let helper =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/storage_acquisition.py");
        let source = if include_source {
            cpg_extract::library::source(library)?
        } else {
            None
        };
        let request = serde_json::json!({"library":library,"environment_root":environment,"source_root":sources,
            "synchronize":synchronize,"reinstall":reinstall,
            "source":source.map(|s|serde_json::json!({"repository":s.repository,"commit":s.commit}))});
        let mut command = Command::new("uv");
        command
            .args([
                "run",
                "--no-project",
                "--offline",
                "--no-python-downloads",
                "--python",
                "3.14.7",
                "python",
            ])
            .arg(helper)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("UV_") || key == "VIRTUAL_ENV" {
                command.env_remove(key);
            }
        }
        let mut child = command
            .spawn()
            .context("blocked: pinned acquisition owner helper could not start")?;
        let mut input = child
            .stdin
            .take()
            .context("acquisition owner stdin unavailable")?;
        let output = child
            .stdout
            .take()
            .context("acquisition owner stdout unavailable")?;
        let mut output = BufReader::new(output);
        writeln!(input, "{request}")?;
        input.flush()?;
        let mut line = String::new();
        output.read_line(&mut line)?;
        let reply: serde_json::Value = serde_json::from_str(&line)
            .context("acquisition owner returned no valid readiness receipt")?;
        if reply["outcome"] != "passed" {
            drop(input);
            let _ = child.wait();
            anyhow::bail!("blocked: acquisition owner: {}", reply["reason"]);
        }
        Ok(Self {
            child,
            input: Some(input),
            output,
            environment: PathBuf::from(
                reply["environment"]
                    .as_str()
                    .context("acquisition environment missing")?,
            ),
            source: reply["source"].as_str().map(PathBuf::from),
        })
    }

    pub fn finish(&mut self) -> anyhow::Result<()> {
        let Some(mut input) = self.input.take() else {
            return Ok(());
        };
        writeln!(input, "{{\"action\":\"finish\"}}")?;
        input.flush()?;
        let mut line = String::new();
        self.output.read_line(&mut line)?;
        drop(input);
        let status = self.child.wait()?;
        let reply: serde_json::Value = serde_json::from_str(&line)
            .context("acquisition reader finish acknowledgement missing")?;
        anyhow::ensure!(
            status.success() && reply["finished"] == true,
            "acquisition reader cleanup unresolved"
        );
        Ok(())
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("acquisition reader retained: {error}");
        }
    }
}
