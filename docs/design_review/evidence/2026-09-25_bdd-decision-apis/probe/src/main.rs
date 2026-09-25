use biodivine_lib_bdd::{Bdd, BddVariableSet, op_function};
fn main() {
    // 20 pairs, separated order -> large results (B009 shape)
    let n = 16;
    let names: Vec<String> = (0..2*n).map(|i| format!("v{i:03}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let ctx = BddVariableSet::new(&refs);
    let v = ctx.variables();
    let mut left = ctx.mk_false();
    for i in 0..n { left = left.or(&ctx.mk_var(v[i]).and(&ctx.mk_var(v[i+n]))); }
    let right = ctx.mk_var(v[0]).not().or(&ctx.mk_var(v[1]));
    let full = Bdd::binary_op_with_limit(usize::MAX, &left, &right, op_function::and).unwrap();
    println!("sizes left={} right={} and={}", left.size(), right.size(), full.size());
    println!("limit1 compatible (sat) -> {:?}", Bdd::binary_op_with_limit(1, &left, &right, op_function::and).map(|b| b.is_false()));
    println!("limit 1000 -> {:?}", Bdd::binary_op_with_limit(1000, &left, &right, op_function::and).map(|b| b.size()));
    let contra = left.and(&left.not());
    println!("limit1 unsat -> {:?}", Bdd::binary_op_with_limit(1, &left, &left.not(), op_function::and).map(|b| b.is_false()));
    let _ = contra;
    println!("check_binary_op sat -> {:?}", Bdd::check_binary_op(usize::MAX, &left, &right, op_function::and));
    println!("check_binary_op unsat -> {:?}", Bdd::check_binary_op(usize::MAX, &left, &left.not(), op_function::and));
    println!("check_binary_op limit 10 -> {:?}", Bdd::check_binary_op(10, &left, &right, op_function::and));
    println!("pair product = {}", left.size()*right.size());
    // implies via and_not
    let a = ctx.mk_var(v[0]).and(&ctx.mk_var(v[n]));
    println!("implies a=>left limit1 -> {:?}", Bdd::binary_op_with_limit(1, &a, &left, op_function::and_not).map(|b| b.is_false()));
    println!("implies left=>a limit1 -> {:?}", Bdd::binary_op_with_limit(1, &left, &a, op_function::and_not).map(|b| b.is_false()));
    // support shrink
    let x = ctx.mk_var(v[2]);
    let s = x.and(&x.not()).or(&ctx.mk_var(v[3]));
    println!("support_set after x&!x | y = {:?} num_vars={}", s.support_set().len(), s.num_vars());
    // restrict size bound
    let r = left.restrict(&[(v[0], true), (v[n], false)]);
    println!("restrict size {} <= {}", r.size(), left.size());
    println!("is_clause a={} left={}", a.is_clause(), left.is_clause());
    println!("most_free_clause left={:?}", left.most_free_clause().map(|c| c.to_values().len()));
}
