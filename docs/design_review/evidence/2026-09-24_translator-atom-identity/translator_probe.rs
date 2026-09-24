//! The external-review assessment's probe P1, as it ran (2026-09-24): a temporary test placed at
//! `crates/cpg-flow/tests/zz_review_probe.rs`, run with
//! `cargo test -p cpg-flow --test zz_review_probe -- --nocapture`, then deleted. It prints the
//! region condition of every `emit()` statement and the definitions reaching every `return`.
//! `shapes.py` in this folder is its `SRC` (plus a `two_suppress` shape, inconclusive and dropped).
use cpg_flow::{Input, RuntimeContext};

const SRC: &str = include_str!("shapes.py");

#[test]
fn probe() {
    let out = cpg_flow::index(
        &[Input { path: "m.py".to_owned(), text: SRC.to_owned() }],
        &RuntimeContext { python_version: (3, 14, 7), platform: "linux".to_owned() },
    );
    let m = &out[0];
    assert_eq!(m.error, None);
    let line = |b: u32| 1 + SRC.as_bytes()[..b as usize].iter().filter(|&&c| c == b'\n').count();
    for r in &m.regions {
        let t = &SRC[r.span.start as usize..r.span.end as usize];
        if t.starts_with("emit()") {
            println!("region L{} emit() if {}", line(r.span.start), r.condition.encode());
        }
    }
    for (i, u) in m.uses.iter().enumerate() {
        let t = &SRC[u.span.start as usize..u.span.end as usize];
        if SRC[..u.span.start as usize].ends_with("return ") {
            for rr in m.reaching.iter().filter(|r| r.use_ix as usize == i) {
                let d = rr.def_ix.and_then(|d| m.defs[d as usize].value)
                    .map(|v| SRC[v.start as usize..v.end as usize].to_owned());
                println!("reach L{} {t} <- {:?} if {}", line(u.span.start), d, rr.condition.encode());
            }
        }
    }
}
