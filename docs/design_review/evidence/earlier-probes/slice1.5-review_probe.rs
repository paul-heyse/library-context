//! Review probes for slice 1.5 (scratch only; never committed).

use std::path::Path;

use arrow_array::RecordBatch;
use cpg_core::attempt::compile_analyzed;
use cpg_core::snapshot::published;
use cpg_core::sql;
use cpg_extract::{ExtractInput, TestHooks, extract};
use cpg_schema::id::Id;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::SessionContext;
use lctx_analytics::config::AnalyticsConfig;

fn config(witnesses: u32) -> String {
    format!(
        r#"
version = 1
[subsystem]
module_prefixes = ["pkg.server", "pkg.helpers", "pkg.handlers", "pkg.boot"]
public_roots = ["pkg"]
[seeds]
primary = ["pkg.Server.tool"]
distractors = ["pkg.helper"]
[pass_a]
max_depth = 2
max_vertices = 128
max_edges = 512
max_witnesses = {witnesses}
[briefs]
budget = 2
serve_unreviewed = true
"#
    )
}

fn copy(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let p = e.unwrap().path();
        let t = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy(&p, &t);
        } else {
            std::fs::copy(&p, &t).unwrap();
        }
    }
}

async fn analyzed(sub: &str, reverse: bool, witnesses: u32) -> (SessionContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join(sub);
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/python/analysis_shapes");
    copy(&fixture.join("release"), &base.join("release"));
    copy(&fixture.join("site"), &base.join("venv/site-packages"));
    let s = Id([7; 16]);
    let out = extract(&ExtractInput {
        release: cpg_extract::Release::from_tree(
            std::fs::canonicalize(base.join("release")).unwrap(),
            "analysis_shapes",
        )
        .unwrap(),
        venv_root: std::fs::canonicalize(base.join("venv")).unwrap(),
        site_packages: vec![std::fs::canonicalize(base.join("venv/site-packages")).unwrap()],
        python_version: (3, 14, 0),
        python_platform: "linux".to_owned(),
        snapshot_id: s,
        corpus: None,
        keep_pysa_json: false,
        test_hooks: TestHooks {
            reverse_module_order: reverse,
            ..Default::default()
        },
    })
    .unwrap();
    let analysis = cpg_core::analyze::Analysis {
        config: AnalyticsConfig::parse(&config(witnesses)).unwrap(),
    };
    let store = dir.path().join("store");
    compile_analyzed(&store, s, &out.tables, Some(&analysis))
        .await
        .unwrap();
    let (_, ctx) = published(&store, s).await.unwrap().unwrap();
    (ctx, dir)
}

async fn batches(ctx: &SessionContext, statement: &str) -> Vec<RecordBatch> {
    sql::query(ctx, statement)
        .await
        .unwrap()
        .collect()
        .await
        .unwrap()
}

async fn text(ctx: &SessionContext, statement: &str) -> String {
    pretty_format_batches(&batches(ctx, statement).await)
        .unwrap()
        .to_string()
}

/// P1: the synthesis tables are identical across module order and location.
#[tokio::test(flavor = "multi_thread")]
async fn probe_synthesis_determinism() {
    let (a, _da) = analyzed("one", false, 3).await;
    let (b, _db) = analyzed("elsewhere", true, 3).await;
    for table in [
        "evidence",
        "assertions",
        "assertion_support",
        "briefs",
        "brief_assertions",
        "brief_members",
        "brief_documents",
        "assertion_policy",
    ] {
        let q = format!("SELECT * EXCLUDE (snapshot_id) FROM {table} ORDER BY 1, 2, 3");
        let (x, y) = (text(&a, &q).await, text(&b, &q).await);
        println!("P1 {table}: identical = {}", x == y);
        assert_eq!(x, y, "{table}");
    }
}

/// P2: with a witness budget of 1, the direct delegation to `helper` (two call sites) reads.
#[tokio::test(flavor = "multi_thread")]
async fn probe_witness_cap_in_call_site_count() {
    let (ctx, _d) = analyzed("one", false, 1).await;
    println!(
        "P2\n{}",
        text(
            &ctx,
            "SELECT f.witnesses_omitted, a.text FROM assertions a \
             JOIN assertion_support s ON s.assertion_id = a.assertion_id \
             JOIN findings f ON f.finding_id = s.finding_id \
             WHERE a.assertion_kind = 2 ORDER BY a.text"
        )
        .await
    );
    println!(
        "P2 validate: {:?}",
        cpg_core::validate::validate(&ctx).await.unwrap()
    );
}

/// P3: a parameter assertion relabelled `documented` (fact evidence only), and every assertion's
/// supports removed: which rules fire?
#[tokio::test(flavor = "multi_thread")]
async fn probe_status_ceiling() {
    let (ctx, _d) = analyzed("one", false, 3).await;
    for t in ["assertions", "assertion_support"] {
        let p = ctx.table(t).await.unwrap();
        ctx.register_table(format!("{t}_pub").as_str(), p.into_view())
            .unwrap();
    }
    let doctored = sql::query(
        &ctx,
        "SELECT snapshot_id, assertion_id, run_id, model_id, extraction_mode, assertion_kind, \
                subject_node_id, applicable_case, \
                CASE WHEN assertion_kind = 3 THEN CAST(1 AS SMALLINT) ELSE evidence_status END \
                  AS evidence_status, \
                text, conditions, limitations, template_version FROM assertions_pub",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("assertions").unwrap();
    ctx.register_table("assertions", doctored).unwrap();
    println!(
        "P3a parameter as documented: {:?}",
        cpg_core::validate::validate(&ctx).await.unwrap()
    );
    let original = ctx.table("assertions_pub").await.unwrap().into_view();
    ctx.deregister_table("assertions").unwrap();
    ctx.register_table("assertions", original).unwrap();
    // Remove the supports of every non-outcome assertion that cites only evidence (parameters).
    let doctored = sql::query(
        &ctx,
        "SELECT * FROM assertion_support_pub WHERE evidence_id IS NULL",
    )
    .await
    .unwrap()
    .into_view();
    ctx.deregister_table("assertion_support").unwrap();
    ctx.register_table("assertion_support", doctored).unwrap();
    println!(
        "P3b evidence supports removed: {:?}",
        cpg_core::validate::validate(&ctx).await.unwrap()
    );
    println!(
        "P3c resolved assertions with no support on the published fixture: {}",
        text(
            &ctx,
            "SELECT a.assertion_kind, a.evidence_status, a.text FROM assertions_pub a \
             LEFT ANTI JOIN assertion_support_pub s ON s.assertion_id = a.assertion_id"
        )
        .await
    );
}

/// P4: the lead sentence the passage leg would pick for each pilot passage holding an exact
/// mention, and whether that sentence contains the mention.
#[test]
fn probe_lead_sentence_on_pilot_passages() {
    let data = std::fs::read_to_string(
        "/tmp/claude-1000/-home-paul-library-context/32eabd30-fa2f-4969-9f50-f6f3537cdcb7/scratchpad/passages.tsv",
    )
    .unwrap();
    let (mut none, mut inside, mut outside) = (0, 0, 0);
    let mut shown = 0;
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let (target, path, ordinal, rel, hex) = (f[0], f[1], f[2], f[3], f[4]);
        if hex.len() >= 65535 {
            println!("P4 skipped (printer-truncated) {target} @ {path}#{ordinal}");
            continue;
        }
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let passage = String::from_utf8(bytes).unwrap();
        let rel: usize = rel.parse().unwrap();
        match cpg_core::synth::lead_sentence(&passage) {
            None => none += 1,
            Some((a, z)) => {
                if a <= rel && rel < z {
                    inside += 1;
                } else {
                    outside += 1;
                    if shown < 8 {
                        shown += 1;
                        println!(
                            "P4 OUTSIDE {target} @ {path}#{ordinal}: {:?}",
                            &passage[a..z]
                        );
                    }
                }
            }
        }
    }
    println!("P4 none={none} inside={inside} outside={outside}");
}

/// P5: a first sentence hard-wrapped across two lines, and how many pilot lead sentences end at a
/// line break without terminal punctuation.
#[test]
fn probe_wrapped_lead_sentence() {
    let p = "Azure AD B2C uses different endpoints, scope URIs, and\ntoken issuers than Entra ID. More.\n";
    let (a, z) = cpg_core::synth::lead_sentence(p).unwrap();
    println!("P5 wrapped: {:?}", &p[a..z]);
    let data = std::fs::read_to_string(
        "/tmp/claude-1000/-home-paul-library-context/32eabd30-fa2f-4969-9f50-f6f3537cdcb7/scratchpad/passages.tsv",
    )
    .unwrap();
    let (mut cut, mut total) = (0, 0);
    let mut seen = std::collections::BTreeSet::new();
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let hex = f[4];
        if hex.len() >= 65535 || !seen.insert((f[1].to_owned(), f[2].to_owned())) {
            continue;
        }
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let s = String::from_utf8(bytes).unwrap();
        if let Some((a, z)) = cpg_core::synth::lead_sentence(&s) {
            total += 1;
            let sentence = &s[a..z];
            let next = s[z..].trim_start_matches([' ', '\t']);
            if next.starts_with('\n') && !sentence.ends_with(['.', '!', '?', ':']) {
                cut += 1;
                println!("P5 cut at line end: {:?}", sentence);
            }
        }
    }
    println!("P5 distinct passages={total} cut_at_line_end={cut}");
}

/// P6: the lead sentence of the changelog's first passage (its first 3,000 characters).
#[test]
fn probe_changelog_lead() {
    let hex = std::fs::read_to_string(
        "/tmp/claude-1000/-home-paul-library-context/32eabd30-fa2f-4969-9f50-f6f3537cdcb7/scratchpad/changelog_head.hex",
    )
    .unwrap();
    let hex = hex.trim();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let s = String::from_utf8(bytes).unwrap();
    let (a, z) = cpg_core::synth::lead_sentence(&s).unwrap();
    println!("P6 changelog lead: {:?}", &s[a..z]);
}
