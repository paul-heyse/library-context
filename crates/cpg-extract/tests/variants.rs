//! Review F2 oracle: every Pysa variant is placed by the mapper (DESIGN §4.2.3). The snapshot is
//! the reviewed variant table as applied to `fixtures/python/pysa_variants`.

mod common;

use common::{cell, provenance, run};

#[test]
fn pysa_variant_table_snapshot() {
    let (_dir, out) = run("pysa_variants");
    let prov = provenance(&out);
    let calls = out.table("pysa_calls").unwrap();
    let mut lines: Vec<String> = (0..calls.num_rows())
        .map(|r| {
            let (origin, modality) = &prov[&cell(calls, "fact_id", r)];
            format!(
                "{}-{} site={} callee={} phase={} ho={} target={}:{} recv={} unresolved={} | origin={} modality={}",
                cell(calls, "start_byte", r),
                cell(calls, "end_byte", r),
                cell(calls, "site_kind", r),
                cell(calls, "callee_kind", r),
                cell(calls, "phase", r),
                cell(calls, "higher_order_index", r),
                cell(calls, "target_kind", r),
                cell(calls, "target_name", r),
                cell(calls, "receiver_class", r),
                cell(calls, "unresolved_reason", r),
                origin,
                modality,
            )
        })
        .collect();
    lines.sort();
    let boundaries = out.table("boundaries").unwrap();
    for r in 0..boundaries.num_rows() {
        lines.push(format!(
            "boundary family={} reason={} {}-{} {}",
            cell(boundaries, "fact_family", r),
            cell(boundaries, "reason", r),
            cell(boundaries, "start_byte", r),
            cell(boundaries, "end_byte", r),
            cell(boundaries, "detail", r),
        ));
    }
    insta::assert_snapshot!(lines.join("\n"));
}

#[test]
fn overrides_are_never_definite_and_if_called_is_potential() {
    let (_dir, out) = run("pysa_variants");
    let prov = provenance(&out);
    let calls = out.table("pysa_calls").unwrap();
    for r in 0..calls.num_rows() {
        let (_, modality) = &prov[&cell(calls, "fact_id", r)];
        // Codebook codes: target_kind overrides = 1; modality definite = 0, potential = 2;
        // callee_kind identifier = 1, attribute_access = 2; phase call = 0.
        if cell(calls, "target_kind", r) == "1" {
            assert_ne!(
                modality, "0",
                "an Overrides target is a dispatch set (§3.6)"
            );
        }
        let callee = cell(calls, "callee_kind", r);
        if (callee == "1" || callee == "2") && cell(calls, "phase", r) == "0" {
            assert_eq!(modality, "2", "if_called targets are potential");
        }
        if !cell(calls, "higher_order_index", r).is_empty() {
            assert_eq!(modality, "2", "higher-order targets are potential");
        }
    }
}
