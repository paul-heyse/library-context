use biodivine_lib_bdd::{Bdd, BddVariable, BddVariableSet};
use std::time::Instant;

fn dnf(vars: &BddVariableSet, v: &[BddVariable], terms: &[&[(usize, bool)]]) -> Bdd {
    let mut out = vars.mk_false();
    for t in terms {
        let mut c = vars.mk_true();
        for &(i, pos) in *t {
            let l = vars.mk_literal(v[i], pos);
            c = c.and(&l);
        }
        out = out.or(&c);
    }
    out
}

// A tiny deterministic PRNG (xorshift) for synthetic conditions.
struct R(u64);
impl R {
    fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
}

fn main() {
    let vars = BddVariableSet::new_anonymous(4);
    let v = vars.variables();
    // R9's counterexample: `!b | !c & a & !d | a & b & c | !d & !a` in two conjunction orders.
    let (a, b, c, d) = (0, 1, 2, 3);
    let x = dnf(&vars, &v, &[&[(b, false)], &[(c, false), (a, true), (d, false)], &[(a, true), (b, true), (c, true)], &[(d, false), (a, false)]]);
    let y = dnf(&vars, &v, &[&[(d, false), (a, false)], &[(a, true), (b, true), (c, true)], &[(b, false)], &[(c, false), (a, true), (d, false)]]);
    println!("R9 canonical: equal={} size={}", x == y, x.size());
    println!("R9 rendering: {:?}", x.to_optimized_dnf().iter().map(|p| format!("{:?}", p)).collect::<Vec<_>>());
    // Site identity: two evaluations of `probe()` are two variables, so `p & !p'` stays satisfiable.
    let vars2 = BddVariableSet::new_anonymous(2);
    let v2 = vars2.variables();
    let inner = vars2.mk_literal(v2[0], true).and(&vars2.mk_literal(v2[1], false));
    let merged = vars2.mk_literal(v2[0], true).and(&vars2.mk_literal(v2[0], false));
    println!("two sites satisfiable={} one atom satisfiable={}", !inner.is_false(), !merged.is_false());
    // Implication and compatibility: N => F ? and (F & assumption) satisfiable?
    let n = dnf(&vars, &v, &[&[(a, true)], &[(b, true)]]);
    let f = dnf(&vars, &v, &[&[(a, true), (c, true)], &[(b, true), (c, true)]]);
    println!("F&N == c&N (factor out N): {}", f.and(&n) == vars.mk_literal(v[c], true).and(&n));
    println!("compatible(F, !a & !b) = {}", !f.and(&vars.mk_literal(v[a], false)).and(&vars.mk_literal(v[b], false)).is_false());
    // Size limit.
    let big = Bdd::binary_op_with_limit(1, &x, &n, biodivine_lib_bdd::op_function::and);
    println!("limit 1 -> {:?}", big.map(|b| b.size()));
    // Scale, like the pilot's conditions: 16 conjunctions of up to 8 literals over 20 atoms.
    for (nvars, terms, lits) in [(20usize, 16usize, 8usize), (40, 16, 8)] {
        let vs = BddVariableSet::new_anonymous(nvars as u16);
        let vv = vs.variables();
        let mut r = R(0x9E3779B97F4A7C15);
        let t0 = Instant::now();
        let mut conds = Vec::new();
        for _ in 0..500 {
            let mut out = vs.mk_false();
            for _ in 0..terms {
                let mut c = vs.mk_true();
                for _ in 0..(1 + r.next() as usize % lits) {
                    let i = (r.next() % nvars as u64) as usize;
                    c = c.and(&vs.mk_literal(vv[i], r.next() % 2 == 0));
                }
                out = out.or(&c);
            }
            conds.push(out);
        }
        let built = t0.elapsed();
        let t1 = Instant::now();
        let (mut max, mut over) = (0, 0);
        for w in conds.windows(2) {
            match Bdd::binary_op_with_limit(50_000, &w[0], &w[1], biodivine_lib_bdd::op_function::and) {
                Some(g) => max = max.max(g.size()),
                None => over += 1,
            }
        }
        let conj = t1.elapsed();
        let t2 = Instant::now();
        let rendered: usize = conds.iter().take(50).map(|c| c.to_optimized_dnf().len()).max().unwrap_or(0);
        println!(
            "{nvars} vars, {terms}x<={lits}: 500 built {:?}; 499 limited conjunctions {:?} (max nodes {max}, over the 50k limit {over}); 50 optimized DNFs {:?} (max terms {rendered})",
            built, conj, t2.elapsed()
        );
    }
    // Determinism: the same construction twice renders identically.
    let again = dnf(&vars, &v, &[&[(b, false)], &[(c, false), (a, true), (d, false)], &[(a, true), (b, true), (c, true)], &[(d, false), (a, false)]]);
    println!("deterministic rendering: {}", format!("{:?}", again.to_optimized_dnf()) == format!("{:?}", x.to_optimized_dnf()));
}
