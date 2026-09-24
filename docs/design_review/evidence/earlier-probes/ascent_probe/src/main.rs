use ascent::{ascent, ascent_par};
use ascent::lattice::set::Set;
use ascent::aggregators::count;
// Reaching-definitions-style may-analysis as a set lattice over (node, var) -> defs;
// plus a provenance column (rule id, premise) encoded by hand.
ascent! {
    struct Flow;
    relation edge(u32, u32);
    relation gen_(u32, &'static str, u32);   // node, var, def-site
    relation kill(u32, &'static str);
    lattice reach_out(u32, &'static str, Set<u32>);
    lattice reach_in(u32, &'static str, Set<u32>);
    reach_out(n, v, Set::singleton(*d)) <-- gen_(n, v, d);
    reach_in(m, v, s.clone()) <-- edge(n, m), reach_out(n, v, ?s);
    reach_out(n, v, s.clone()) <-- reach_in(n, v, ?s), !kill(n, v);
    // provenance: which edge carried a def into which node (rule id 2)
    relation why(u32, &'static str, u32, u32, u8);
    why(m, v, d, n, 2) <-- edge(n, m), reach_out(n, v, ?s), for d in s.iter();
    relation fanin(u32, usize);
    fanin(m, c) <-- edge(_, m), agg c = count() in edge(_, m);
}
ascent_par! { struct Tc; relation e(u32,u32); relation p(u32,u32); p(x,y) <-- e(x,y); p(x,z) <-- p(x,y), e(y,z); }
fn main() {
    let mut f = Flow::default();
    f.edge = vec![(1,2),(2,3),(3,2),(3,4)];
    f.gen_ = vec![(1,"x",1),(3,"x",3)];
    f.kill = vec![(3,"x")];
    f.run();
    let mut r: Vec<_> = f.reach_in.iter().map(|(n,v,s)| (*n,*v,s.iter().copied().collect::<Vec<_>>())).collect(); r.sort();
    println!("reach_in = {:?}", r);
    println!("why rows = {}", f.why.len());
    let mut t = Tc::default(); t.e = vec![(1,2),(2,3),(3,1),(3,4)].into_iter().collect(); t.run();
    println!("par closure = {}", t.p.len());
}
