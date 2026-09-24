mod ours;

use std::collections::BTreeSet;
use std::time::Instant;

use bit_set::BitSet;
use fixedbitset::FixedBitSet;
use odis::algorithms::{CanonicalBasis, Titanic, NextClosure};
use odis::traits::{ConceptEnumerator, IcebergConceptEnumerator, ImplicationEngine};
use odis::FormalContext;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Incidence as (object, attribute) pairs.
type Inc = Vec<(usize, usize)>;

fn cxt(g: usize, m: usize, inc: &Inc) -> FormalContext<String> {
    let mut s = format!("B\n\n{g}\n{m}\n\n");
    for i in 0..g { s += &format!("g{i}\n"); }
    for j in 0..m { s += &format!("m{j}\n"); }
    let set: BTreeSet<(usize, usize)> = inc.iter().copied().collect();
    for i in 0..g {
        for j in 0..m { s.push(if set.contains(&(i, j)) { 'X' } else { '.' }); }
        s.push('\n');
    }
    FormalContext::<String>::from(s.as_bytes()).unwrap()
}

fn fb(s: &FixedBitSet) -> Vec<usize> { s.ones().collect() }
fn bs(s: &BitSet) -> Vec<usize> { s.iter().collect() }

fn main() {
    // 1. Correctness: ours vs odis on random contexts.
    let mut mismatches = 0;
    let mut checked = 0;
    for seed in 1..40u64 {
        let mut r = Rng(seed * 0x9E37_79B9_7F4A_7C15 + 1);
        let (g, m) = (8 + (seed % 5) as usize, 6 + (seed % 4) as usize);
        let inc: Inc = (0..g).flat_map(|i| (0..m).map(move |j| (i, j)))
            .filter(|_| r.next() % 3 == 0).collect::<Vec<_>>();
        let ctx = ours::Context::new(g, m, &inc);
        let octx = cxt(g, m, &inc);
        let basis_full: Vec<(BitSet, BitSet)> = CanonicalBasis.compute_basis(&octx);
        for s in 0..=4usize {
            checked += 1;
            let lat = ours::analyse(&ctx, s, 1_000_000);
            let our_c: BTreeSet<(Vec<usize>, Vec<usize>)> =
                lat.concepts.iter().map(|(e, i)| (fb(e), fb(i))).collect();
            let ice = Titanic.enumerate(&octx, s as u32);
            let their_c: BTreeSet<(Vec<usize>, Vec<usize>)> =
                ice.poset.nodes.iter().map(|(e, i)| (bs(e), bs(i))).collect();
            // odis's NextClosure (all concepts), filtered by support
            let nc: BTreeSet<(Vec<usize>, Vec<usize>)> = NextClosure
                .enumerate_concepts(&octx)
                .filter(|(e, _)| e.len() >= s)
                .map(|(e, i)| (bs(&e), bs(&i))).collect();
            let our_b: BTreeSet<(Vec<usize>, Vec<usize>)> = lat.implications.iter()
                .map(|x| (fb(&x.premise), fb(&x.conclusion))).collect();
            let their_b: BTreeSet<(Vec<usize>, Vec<usize>)> = basis_full.iter()
                .filter(|(p, _)| octx.index_extent(p).len() >= s)
                .map(|(p, c)| {
                    let concl: Vec<usize> = c.iter().filter(|x| !p.contains(*x)).collect();
                    (bs(p), concl)
                }).collect();
            let ok = our_c == their_c && our_c == nc && our_b == their_b;
            if !ok {
                mismatches += 1;
                if mismatches <= 3 {
                    println!("MISMATCH seed {seed} s {s}: concepts ours {} titanic {} nc {}; basis ours {} odis {}",
                        our_c.len(), their_c.len(), nc.len(), our_b.len(), their_b.len());
                }
            }
        }
    }
    println!("correctness: {checked} cases, {mismatches} mismatches");

    // 2. Timing at pilot shape: 51 objects; attributes with skewed frequencies.
    for (g, m, per) in [(51usize, 193usize, 14usize), (51, 363, 24)] {
        let mut r = Rng(7);
        let mut inc: Inc = Vec::new();
        for i in 0..g {
            let mut picked = BTreeSet::new();
            while picked.len() < per {
                // Zipf-ish: small attribute indices far more likely
                let u = (r.next() % 10_000) as f64 / 10_000.0;
                let j = ((m as f64) * u * u * u) as usize % m;
                picked.insert(j);
            }
            inc.extend(picked.into_iter().map(|j| (i, j)));
        }
        let ctx = ours::Context::new(g, m, &inc);
        let octx = cxt(g, m, &inc);
        let t = Instant::now();
        let lat = ours::analyse(&ctx, 2, 20_000);
        println!("{g}x{m}: ours {} concepts {} implications {} examined in {:?} (budget {})",
            lat.concepts.len(), lat.implications.len(), lat.examined, t.elapsed(), lat.budget_reached);
        let t = Instant::now();
        let ice = Titanic.enumerate(&octx, 2);
        println!("{g}x{m}: odis Titanic {} iceberg concepts in {:?}", ice.poset.nodes.len(), t.elapsed());
        let t = Instant::now();
        let bo = odis::algorithms::canonical_basis::index_canonical_basis_optimised(&octx);
        let fo = bo.iter().filter(|(p, _)| octx.index_extent(p).len() >= 2).count();
        println!("{g}x{m}: odis optimised basis {} implications ({} frequent) in {:?}", bo.len(), fo, t.elapsed());
        // determinism: two Titanic runs give the same node order?
        let a1: Vec<_> = Titanic.enumerate(&octx, 2).poset.nodes.iter().map(|(e,i)| (bs(e), bs(i))).collect();
        let a2: Vec<_> = Titanic.enumerate(&octx, 2).poset.nodes.iter().map(|(e,i)| (bs(e), bs(i))).collect();
        println!("{g}x{m}: Titanic node order identical across runs: {}", a1 == a2);
        if m > 200 { continue; }
        let t = Instant::now();
        let b = CanonicalBasis.compute_basis(&octx);
        let freq = b.iter().filter(|(p, _)| octx.index_extent(p).len() >= 2).count();
        println!("{g}x{m}: odis full canonical basis {} implications ({} frequent) in {:?}", b.len(), freq, t.elapsed());
    }
}
