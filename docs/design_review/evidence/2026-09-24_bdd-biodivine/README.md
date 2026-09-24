# biodivine-lib-bdd 0.6.3 as the condition kernel (P3), 2026-09-24

**Question.** Does biodivine-lib-bdd give canonical forms, exact factoring and compatibility, a
size limit and deterministic rendering, at a cost fit for our conditions?

**Run.** `RUSTUP_TOOLCHAIN=1.98.1 CARGO_TARGET_DIR=target cargo run --release` here (a standalone
crate; `Cargo.lock` is the resolved set). The synthetic stress case (random 32×8 over 40 atoms)
was removed after it did not finish in 10 minutes; `src/main.rs` holds the realistic-size version.

**Output (2026-09-24, this machine):**
```
R9 canonical: equal=true size=7
R9 rendering: [!d, !b, a & c]
two sites satisfiable=true one atom satisfiable=false
F&N == c&N (factor out N): true
compatible(F, !a & !b) = false
limit 1 -> None
20 vars, 16x<=8: 500 built 29.7 ms; 499 limited conjunctions 14.5 ms (max nodes 4,051, over the 50k limit 0); 50 optimized DNFs 77.4 ms (max terms 26)
40 vars, 16x<=8: 500 built 112.0 ms; 499 limited conjunctions 422.5 ms (max nodes 48,952, over the 50k limit 31); 50 optimized DNFs 991.6 ms (max terms 16)
deterministic rendering: true
```

**Conclusion.** Fit, with a node limit and a bounded rendering. Only `num-rational` is new to our
lockfile. Cited by decision D-11 and forward plan Stage 3.0.
