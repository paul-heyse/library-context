//! Review F3 and F5 oracles: absence is never implicit (DESIGN §3.7, §4.2.2, §4.2.3).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{cell, column, run};
use cpg_extract::ExtractOutput;

/// module name → [(family, status, reason)] with codebook codes.
fn coverage(out: &ExtractOutput) -> BTreeMap<String, Vec<(String, String, String)>> {
    let files = out.table("source_files").unwrap();
    let names: BTreeMap<String, String> = (0..files.num_rows())
        .map(|r| {
            (
                cell(files, "module_node_id", r),
                cell(files, "module_name", r),
            )
        })
        .collect();
    let cov = out.table("coverage").unwrap();
    let mut by_module: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    for r in 0..cov.num_rows() {
        by_module
            .entry(names[&cell(cov, "scope_node_id", r)].clone())
            .or_default()
            .push((
                cell(cov, "fact_family", r),
                cell(cov, "status", r),
                cell(cov, "reason", r),
            ));
    }
    by_module
}

// Codebook codes used below.
const EXPORTS: &str = "1";
const COMPLETE: &str = "0";
const PARTIAL: &str = "1";
const UNAVAILABLE: &str = "3";
const OUTSIDE_PROVIDER_MODEL: &str = "10";
const SYNTAX_ERROR: &str = "11";
const UNDECODABLE: &str = "12";

#[test]
fn dunder_all_forms_hand_written_expectations() {
    let (_dir, out) = run("dunder_all");
    let cov = coverage(&out);
    let exports = |m: &str| {
        cov[m]
            .iter()
            .find(|(f, _, _)| f == EXPORTS)
            .unwrap()
            .1
            .clone()
    };
    // Literal `__all__` forms (`=`, `+=`, `.append`) are complete; the others are flagged.
    assert_eq!(exports("dunder.lit"), COMPLETE);
    assert_eq!(exports("dunder.sub"), COMPLETE);
    assert_eq!(
        exports("dunder"),
        PARTIAL,
        "sub.__all__ + [...] is not literal"
    );
    assert_eq!(
        exports("dunder.dyn"),
        PARTIAL,
        "__all__ = _names() is not literal"
    );

    // Hand-written runtime `__all__` for the literal module: the independent oracle (DM-53).
    let public: BTreeSet<String> = column(out.table("public_names").unwrap(), "access_path")
        .into_iter()
        .collect();
    for name in ["dunder.lit.exported", "dunder.lit.also", "dunder.lit.third"] {
        assert!(public.contains(name), "{name} is public");
    }
    assert!(!public.contains("dunder.lit.hidden"));
    // Where Pyrefly's reading differs from the runtime `__all__`, the module is `partial` (above).
    // The runtime `dunder.__all__` is ["alpha", "beta", "top"]; Pyrefly keeps only "top". The
    // runtime `dunder.dyn.__all__` is ["dyn_public", "dyn_other"]; Pyrefly falls back to every
    // non-underscore name, so `alpha_call` is over-reported.
    let under = |prefix: &str| -> BTreeSet<String> {
        public
            .iter()
            .filter(|p| {
                p.strip_prefix(prefix)
                    .is_some_and(|rest| !rest.contains('.'))
            })
            .cloned()
            .collect()
    };
    assert_eq!(under("dunder."), BTreeSet::from(["dunder.top".to_owned()]));
    assert_eq!(
        under("dunder.dyn."),
        BTreeSet::from(
            ["alpha_call", "dyn_other", "dyn_public"].map(|n| format!("dunder.dyn.{n}"))
        )
    );

    let b = out.table("boundaries").unwrap();
    let flagged = (0..b.num_rows())
        .filter(|&r| {
            cell(b, "reason", r) == OUTSIDE_PROVIDER_MODEL && cell(b, "fact_family", r) == EXPORTS
        })
        .count();
    assert_eq!(flagged, 2, "one boundary per non-literal __all__ statement");
}

#[test]
fn undecodable_and_broken_modules_are_never_silent() {
    let (_dir, out) = run("_invalid/undecodable");
    let cov = coverage(&out);
    for (family, status, reason) in &cov["badpkg.latin1"] {
        assert_eq!(
            (status.as_str(), reason.as_str()),
            (UNAVAILABLE, UNDECODABLE),
            "family {family}"
        );
    }
    for (family, status, reason) in &cov["badpkg.broken"] {
        assert_eq!(
            (status.as_str(), reason.as_str()),
            (PARTIAL, SYNTAX_ERROR),
            "family {family}"
        );
    }
    let files = out.table("source_files").unwrap();
    let utf8: BTreeMap<String, String> = (0..files.num_rows())
        .map(|r| (cell(files, "module_name", r), cell(files, "utf8", r)))
        .collect();
    assert_eq!(utf8["badpkg.latin1"], "false");
    // Nothing from the undecodable module is asserted.
    let decls = column(out.table("declarations").unwrap(), "qualified_name");
    assert!(decls.iter().all(|d| !d.starts_with("badpkg.latin1")));
    let pysa = column(out.table("pysa_functions").unwrap(), "module_name");
    assert!(pysa.iter().all(|m| m != "badpkg.latin1"));
    // The recovered module still yields facts.
    assert!(decls.contains(&"badpkg.broken.after".to_owned()));
}

#[test]
fn unicode_bom_crlf_calls_all_join_on_the_call_range() {
    let (_dir, out) = run("unicode_bom");
    // Offsets index the acquired bytes, BOM included (§3.4): every declaration's name span slices
    // its name out of the file as stored (review O6).
    let files = out.table("source_files").unwrap();
    let path: BTreeMap<String, String> = (0..files.num_rows())
        .map(|r| (cell(files, "module_node_id", r), cell(files, "path", r)))
        .collect();
    let decls = out.table("declarations").unwrap();
    assert!(decls.num_rows() > 0);
    for r in 0..decls.num_rows() {
        let bytes = std::fs::read(
            common::fixture("unicode_bom").join(&path[&cell(decls, "module_node_id", r)]),
        )
        .unwrap();
        let (s, e): (usize, usize) = (
            cell(decls, "name_start_byte", r).parse().unwrap(),
            cell(decls, "name_end_byte", r).parse().unwrap(),
        );
        assert_eq!(bytes[s..e], *cell(decls, "name", r).as_bytes());
    }
    let b = out.table("boundaries").unwrap();
    assert_eq!(
        b.num_rows(),
        0,
        "every call matched; nothing outside the model"
    );
    let cov = coverage(&out);
    for (m, rows) in cov {
        for (family, status, _) in rows {
            assert_eq!(status, COMPLETE, "{m} family {family}");
        }
    }
}
