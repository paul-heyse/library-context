# Translator atom identity (P1, P2), 2026-09-24

**Question.** Does translating ty's predicates into our condition atoms drop feasible paths, as
the external review claimed (atoms identified by text, not by evaluation)?

**P1, the translator.** `translator_probe.rs` ran as a temporary `cpg-flow` test (see its header)
on `shapes.py`, context Python (3, 14, 7), at the tree of `3129c7c` plus the Stage 2 end review's
uncommitted fixes. Output:

```
region L8 emit() if false          impure: two calls of probe() became one atom
region L24 emit() if false         mutated: self.x mutated by a call between two tests
region L31 emit() if false         cleared: items.clear() between two tests
region L37 emit() if false         eq_vs_is: `== True` folded into `is True`
region L43 emit() if false         eq_none: `== None` folded into `is None`
region L48 emit() if true          version: `<= (3, 14)` decided true under 3.14.7 (wrong)
region L50 emit() if false         version: `> (3, 14)` decided false under 3.14.7 (wrong)
region L55 emit() if false         choose: a parameter named TYPE_CHECKING decided false
reach L17 found <- Some("None") if !opaque("<the iterable is non-empty>")
reach L17 found <- Some("j") if opaque("<the iterable is non-empty>")
                                   two_ranges: `found = i` missing (one atom for both loops)
```

**P2, CPython.** `uv run python monitor_probe.py` on CPython 3.14.7: `executed emit() lines:
[8, 24, 31, 37, 43, 50, 55] of [8, 24, 31, 37, 43, 48, 50, 55]`, and `two_ranges(2, 0)` returns 1.
Exactly the lines the translator decided `false` execute; the one it decided `true` does not.

**Conclusion.** Confirmed: eight defects, one cause. Fixes: forward plan Stage 2.9.2–2.9.4
(decision D-10). `shapes.py` seeds the Stage 2.9.5 runtime oracle and the `flow_shapes` cases.
