//! `lctx parity`: the cutover's parity harness (cutover plan §3.6). Reports are JSON and Markdown,
//! archived in the phase's evidence folder.
use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the harness's own controls: identical relations pass, an injected difference fails,
    /// declaring it passes, a changed multiplicity fails, and a corrupted legacy-id mapping fails.
    SelfTest {
        /// The directory the report is written to.
        #[arg(long)]
        out: PathBuf,
    },
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Command::SelfTest { out } => self_test(&out),
        }
    }
}

fn self_test(out: &Path) -> anyhow::Result<()> {
    let cases = tokio::runtime::Runtime::new()?.block_on(cpg_core::parity::self_test())?;
    fs_err::create_dir_all(out)?;
    let mut markdown = String::from("# Parity harness self-test\n\n| Case | Must pass | Passed | Behaved |\n|---|---|---|---|\n");
    let mut json = Vec::new();
    for case in &cases {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            case.name,
            case.must_pass,
            case.report.passed(),
            if case.behaved() { "yes" } else { "**no**" }
        ));
        json.push(serde_json::json!({
            "case": case.name,
            "must_pass": case.must_pass,
            "behaved": case.behaved(),
            "report": case.report,
        }));
    }
    fs_err::write(out.join("self-test.json"), serde_json::to_string_pretty(&json)?)?;
    fs_err::write(out.join("self-test.md"), &markdown)?;
    print!("{markdown}");
    let failed = cases.iter().filter(|c| !c.behaved()).count();
    if failed > 0 {
        anyhow::bail!("{failed} parity self-test case(s) misbehaved");
    }
    Ok(())
}
