//! Review F3 and F5 oracles: absence is never implicit (DESIGN §3.7, §4.2.2, §4.2.3).

mod common;

use std::collections::BTreeMap;

use common::{cell, run};
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

    // The public sets, against the runtime `__all__`s, are `typed_symbols`'.

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
    // Nothing from the undecodable module is stated (`typed_conformance`: no occurrence, and its
    // families unavailable).
}

#[test]
fn unicode_bom_crlf_calls_all_join_on_the_call_range() {
    let (_dir, out) = run("unicode_bom");
    // Byte spans over the BOM and CRLF lines are the typed syntax's (`typed_syntax_shapes`).
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
