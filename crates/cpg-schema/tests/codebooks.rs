//! Codebooks are append-only (DESIGN §3.5): the registry is snapshot-tested, so renumbering,
//! reordering or removal shows up as a reviewed snapshot diff; codes must stay dense from 0.

use cpg_schema::codebook::{self, Codebook, PysaUnresolvedReason, code_range};

#[test]
fn registry_snapshot() {
    let text: String = codebook::registry()
        .iter()
        .map(|c| {
            let values: Vec<String> = c.values.iter().map(|(n, t)| format!("  {n} {t}")).collect();
            format!("{}\n{}\n", c.name, values.join("\n"))
        })
        .collect();
    insta::assert_snapshot!(text);
}

#[test]
fn codes_are_dense_from_zero_and_names_unique() {
    let reg = codebook::registry();
    let mut names: Vec<&str> = reg.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), reg.len(), "codebook names are unique");
    for c in &reg {
        for (i, (code, _)) in c.values.iter().enumerate() {
            assert_eq!(*code as usize, i, "{}: codes are dense from 0", c.name);
        }
        assert_eq!(code_range(c.name), Some((0, c.values.len() as i16 - 1)));
    }
}

#[test]
fn from_code_round_trips_and_rejects_unknown_codes() {
    for r in PysaUnresolvedReason::all() {
        assert_eq!(PysaUnresolvedReason::from_code(r.code()), Some(*r));
    }
    assert_eq!(
        PysaUnresolvedReason::all().len(),
        14,
        "Pysa's 14 reasons at the pin"
    );
    assert_eq!(PysaUnresolvedReason::from_code(14), None);
    assert_eq!(PysaUnresolvedReason::from_code(-1), None);
}
