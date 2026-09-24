# Design review: the external review's recommendations, against the target design

**Date:** 2026-09-24 · **Depth:** standard (several recommendations would change §B decisions:
the condition engine, a solver, a Python validation lane) · **Mode:** document plus code.

**Target.**
- **The external review:** `docs/full_cpg_pipeline_external_review.md`, a source-and-design review
  of `main` at `3129c7c` ("flow graph mostly implemented").
- **What it is assessed against:**
  - DESIGN §B1, §B10, §3.2, §3.9 and §9.9;
  - ADR-0022 (`proposed`);
  - the plan `docs/plans/behavioral-model-pivot-plan_2026-09-24.md`, Stages 3–5.
- **The code the external review cites,** read at the current tree: `3129c7c` plus the
  uncommitted fixes for the Stage 2 end review.
  - `crates/cpg-flow/src/{lib,predicate}.rs`;
  - `crates/cpg-schema/src/condition.rs`;
  - `crates/cpg-core/src/flow_model.rs`.

**Reviewer:** the session executing the plan, at the operator's request. The external review
is evidence for this review, not authority. Every source claim it makes was re-checked here, and
its library proposals were probed.

---

## 1. Decision and scope

**Question.** Which of the external review's recommendations should change the target design and
the plan, and how?

**Where things stand.**
- The provider boundary is Tested and holds: ty and Pyrefly through `cpg-flow`, spans, place text
  and conditions.
- The translation from ty's predicates into our condition atoms is the weak point.
  - The Stage 2 end review found part of it: R4, versioning; R6, guards.
  - The external review finds a deeper cause: an atom's identity is its **text**, not the
    **evaluation** it stands for.

**Observable outcome of the change:** no feasible path is dropped by the translation, conditions
compose without a budget cut, and an independent execution oracle checks it.

**In scope:** every recommendation in the external review's §1–§6.

**Not in scope:**
- the Stage 2 end review's disposition, which is in progress and uncommitted;
- Stage 4's registry.

### Method and coverage

**Read in full:**
- the external review;
- ADR-0022;
- DESIGN §B10, and §3.9's condition and place text;
- `predicate.rs`'s translator and its runtime view (`:301-362`);
- `cpg-flow/src/lib.rs`'s program setup (`:247`) and its skips (`:601`, `:697`);
- the plan's Stages 3–5.

**Verified by probes** (2026-09-24; all outside the repository except one temporary test file,
deleted afterwards):

| Probe | What ran | Result |
|---|---|---|
| **P1: translator** | A temporary `cpg-flow` test feeding 9 shapes to `cpg_flow::index` under the context (3, 14, 7). One shape, two `suppress` blocks, was inconclusive, because its second body always rebinds | Eight of eight claimed defects reproduced (§5) |
| **P2: runtime** | The eight conclusive shapes executed under CPython 3.14.7's `sys.monitoring` (LINE events), with inputs taking the "impossible" paths | Exactly the `emit()` lines the translator decided `false` executed (lines 8, 24, 31, 37, 43, 50, 55). The one it decided `true` (line 48) did not |
| **P3: BDD** | biodivine-lib-bdd 0.6.3 in a scratch crate, Rust 1.98.1, release build | Canonical forms, exact factoring and compatibility, a size limit, deterministic rendering; timings in §9 |
| **P4: CrossHair** | CrossHair 0.0.110 on Python 3.14.7, `diffbehavior` | It found `x=1` for `== True` against `is True`, and a class with `__eq__` for `== None` against `is None` |
| **Registry and docs** | crates.io and PyPI metadata; Context7 for OxiDD, Ascent and CrossHair; docs.rs for biodivine-lib-bdd; GitHub for Pysa (earlier) | §3's table |

**Not verified:**
- the z3 crate (not built; a system `libz3.so.4` is present);
- OxiDD (docs only);
- Ascent beyond its docs (it is already a planned spike);
- the `bytecode` package, `stubtest` and Scalpel (registry facts only);
- ty's own reachability documentation, which the external review cites: asserted.

**Guarantees not attacked:**
- ty's use-def map beyond the probe shapes;
- the determinism of biodivine-lib-bdd across platforms;
- CrossHair's isolation.

## 2. Authority and lifecycle map (compressed)

| Fact | Authority today | Problem |
|---|---|---|
| **What a test observes** (its truth at one evaluation) | ty's predicate, one per expression node (`PredicateNode::Expression(x)`) | Our atom merges every predicate with the same text, and one line version (X1) |
| **Whether a name is `typing.TYPE_CHECKING`** or the stdlib's `sys` or `os` | Our reference resolution (`references`, `bindings`, `export_syntax`) | The translator decides by spelling (`predicate.rs:304-305, 308, 327-329`) (X2) |
| **A condition's structure** | ty's diagram | Flattened at extraction to DNF within 16×8. Past it, `OverBudget` loses the structure for every consumer (X4) |

## 3. The recommendations, one by one

| # | Recommendation (external §) | Verdict | Evidence | Where it lands |
|---|---|---|---|---|
| E1 | Keep Pyrefly + Ruff + ty, with separate provider contracts (§1) | **Accept** (already the design) | ADR-0012 amendment; ADR-0022 §The flow provider; `cpg-flow`'s boundary | — |
| E2 | Consume ty's type inference later, as guarded type observations (§1) | **Defer** | We link `ty_python_core` (the semantic index) only; types come from Pyrefly (§B1). A second type provider has no consumer | Trigger: a question needing a narrowed type at a use site that Pyrefly's `type_observations` cannot answer |
| E3 | Predicate identity is an evaluation, not text (§2A) | **Accept: the review's top finding** | P1 and P2: impure calls, shared synthetic atoms, attribute and container mutation between tests (X1) | ADR-0022 §Conditions and §Places, amended before acceptance |
| E4 | Versions by definition identity and memory versions, not `place@line` (§2A) | **Accept, modified.** Definition-set identity for names now; memory versions only as far as the per-site rule of X1 needs | Same-line bindings share a line; mutation changes a value with no rebinding (P1 `mutated`, `cleared`) | X1 |
| E5 | Runtime special cases by resolved bindings (§2B) | **Accept** | P1 `choose(TYPE_CHECKING)`; `predicate.rs:304-305` | X2 |
| E6 | A narrow ty patch for runtime-aware indexing, instead of source rewriting (§2B) | **Reject for now** | The rename is only there to stop ty deciding the name itself; deciding by our own resolution needs no second fork | Trigger: a runtime-view case our resolution cannot express |
| E7 | Version tuples with Python's tuple semantics (§2B) | **Accept** | `predicate.rs:315-316` truncates to a common prefix. P1 and P2: `<= (3, 14)` is decided true, `> (3, 14)` false, under 3.14.7; CPython does the opposite | X2 |
| E8 | Preserve Python operators: identity, equality, membership, truthiness, isinstance (§2C) | **Accept** | P1 `eq_vs_is`, `eq_none`; P4 CrossHair counterexamples; ADR-0022's canonical forms (`== None` is `is_none`; `is True` is `equals`) | X1 (a codebook append for an identity atom) |
| E9 | Keep "not excluded" apart from "a feasible execution is established": approximation direction and assumptions per claim (§2D) | **Accept, modified.** The ADR states the direction now; an `approximated` flag later; per-claim assumption records deferred | ADR-0022 declares may-behavior globally; `established` reads as feasibility in served answers | X5 |
| E10 | Keep the formula structure and precision-loss information; DNF only for display (§2E) | **Accept, in Stage 3** | 42 pilot claims are `budget_reached` (snapshot `162bda5a…`); an over-budget condition is lost at extraction | X4 |
| E11 | Expose ty's diagram saturation (a patch) (§2E) | **Defer** | Saturation gives AMBIGUOUS, which we read as `true`, the sound direction: precision is lost, not paths | Trigger: a pilot scope's conditions measurably degraded by saturation |
| E12 | biodivine-lib-bdd as the condition engine (§3) | **Accept for Stage 3 entry**, after E3 | P3 (§9). Its dependencies are already locked except `num-rational`. MIT | New ADR, amending §B10's wording |
| E13 | OxiDD as the alternative (§3) | **Defer** | About ten sub-crates, fixed-capacity managers, multithreaded by default; nothing here needs ternary or shared-root management yet | Trigger: E12's single-owner BDDs measured too slow or too large on the pilot |
| E14 | Ascent for recursive relations (§3) | **Keep as planned** (a Stage 3 spike behind its trigger) | DESIGN L325 and L3481 already say so, and the external review's boundary (DataFusion relational, Rust or Ascent for recursion, Arrow contracts) matches ADR-0011 | — |
| E15 | Rust `z3` for targeted feasibility (§3) | **Defer** (§B10 excludes a solver) | A system `libz3` exists, so it could link; no query needs it yet | Trigger: a query that E12's propositional kernel plus the typed atom theory cannot decide (for example, integer bounds across two conditions) |
| E16 | petgraph, with explicit control points (§3) | **Accept as it stands** | Already the substrate | — |
| E17 | Object identity, aliasing, memory effects, strong and weak updates (§4) | **Defer; a minimal rule now** | Soundness now needs only X1's per-site rule for state-dependent atoms. Points-to sets serve Stage 3's field effects | Trigger: a Stage 3 summary whose field effect needs aliasing |
| E18 | Implicit calls: operators, truthiness, formatting as protocol operations (§4) | **Defer; state the assumption now** | ADR-0022: "an operator, an f-string or a container is computed from its operands directly". That is a primitive-operand assumption and should say so | Stage 5 (protocols); trigger: an evaluation item turning on a dunder method |
| E19 | Exceptional and deferred execution: `finally`, coroutine creation against execution (§4) | **Defer to Stage 3 (exceptions) and Stage 5 (lifecycle)** | The read phase is lexical; Q05, Q07 and Q08 are the consumers | Plan Stage 3 (handlers) and Stage 5 (phases) |
| E20 | CPython bytecode as a control-flow cross-check (§5A) | **Defer** | P2's runtime oracle checks the invariant that matters (executed ⇒ admitted) more directly, without exception-table interpretation | Trigger: a `try`/`finally`/`with` region defect that runtime inputs cannot exercise |
| E21 | Hypothesis plus `sys.monitoring`: observed ⊆ admitted (§5B) | **Accept, narrowly: the translator's soundness oracle** | P2 caught all eight defects on its first run; Hypothesis 6.168.1 supports 3.14 | X6: plan Stage 2.9 |
| E22 | CrossHair for selected models (§5C) | **Accept, in Stage 3** (the models catalog's oracle) | P4 works on 3.14.7. It executes code, so it runs isolated and outside `fixtures/` | Plan Stage 3.1 |
| E23 | `mypy.stubtest` at native boundaries (§5D) | **Reject** | The pilot is pure Python; importing executes code; no consumer | — |
| E24 | Scalpel and CodeQL (§5) | **Reject as components**; CodeQL's control-flow model as a design reference | No consumer | — |
| E25 | Sequence: harden the bridge first (§6) | **Accept** | It matches this review's X1–X3 | Plan Stage 2.9 |

## 4. Derivation: the translation, as a transformation (compressed)

**What happens now.**
- ty gives one predicate per test expression, and a diagram over predicate ids.
- `diagram()` maps each predicate to an atom (`predicate.rs:365+`). An atom is keyed by kind,
  spelled place and literal, or by opaque text.
- Two predicates whose atoms encode alike become one Boolean variable.

**Where it is unsound.**
- **The merge is sound only when the two evaluations observe the same stable value.** That holds
  for an identity or isinstance test on a name at the same definition set.
- **It fails for** calls, synthetic predicates, attribute places across calls, object state
  (truthiness, `==`, `in`) across mutation, and different operators folded into one atom.

**Where the loss goes unseen.** A reaching row or value source whose condition becomes `false` is
skipped at extraction (`lib.rs:601`, `:697`), so no rule sees the loss (X3).

## 5. Journeys

**P1 and P2 together** (translator against CPython 3.14.7):

| Shape | Translator (P1) | CPython (P2) | Cause |
|---|---|---|---|
| `if probe(): if not probe(): emit()` | `emit()` under `false` | executes (`probe` returns True, then False) | One atom for two calls |
| `for i in range(n): found = i` then `for j in range(m): found = j`; `return found` | no `found = i` reaching row | `two_ranges(2, 0)` returns 1 | `<the iterable is non-empty>` shared by both loops |
| `if self.x is None: self.reset(); if self.x is not None: emit()` | `false` | executes | Mutation between tests |
| `if items: items.clear(); if not items: emit()` | `false` | executes | Object state between tests |
| `if x == True: if x is not True: emit()` | `false` | executes (`x=1`) | `is True` folded into `equals` |
| `if x == None: if x is not None: emit()` | `false` | executes (`__eq__` returns True) | `== None` folded into `is_none` |
| `if sys.version_info > (3, 14): emit()` | `false` | executes on 3.14.7 | Prefix truncation |
| `if sys.version_info <= (3, 14): emit()` | `true` | does not execute | Prefix truncation |
| `def choose(TYPE_CHECKING): if TYPE_CHECKING: emit()` | `false` | executes (`True` passed) | Decided by spelling |

**Ordinary extension** (a new test form): with text identity, every new opaque form risks another
merge. With evaluation identity, the new form is sound by default, and merging is an explicit
per-kind rule.

**Boundary** (a callee's condition restated in a caller, in Stage 3): with DNF and a budget, a
composed condition is cut and lost. With a BDD over evaluation atoms, it composes, and only its
rendering is bounded.

## 6. Acceptance gates (the target design as it stands, challenged by the external review)

| Gate | Result | Evidence | Required action |
|---|---|---|---|
| G1 Authority | **fail** (narrow) | Whether a name is `typing.TYPE_CHECKING`, `sys` or `os` is decided by spelling (`predicate.rs:304-308`), not by our reference resolution | X2 |
| G2 Semantic fidelity | **fail** | Feasible paths are dropped (P1, P2); operators are reinterpreted (`== None` as `is None`) | X1, X2 |
| G3 Validity | **fail** | A translation-induced `false` is skipped at extraction before any rule (`lib.rs:601, 697`); `semantic:condition-not-false` sees only served rows | X3 |
| G4 Hidden behavior | **pass** | Providers are pinned, and the program settings are explicit (`ProgramSettings::empty`, `lib.rs:247`). The proposed Python lane executes code and must stay isolated (E21, E22) | — |
| G5 Consistency and recovery | **pass** | Publication is unchanged | — |
| G6 Transformation and reuse | **fail** | The predicate → atom translation merges distinct evaluations without an equivalence policy | X1 |
| G7 Truthful capability claims | **fail** | ADR-0022 §Places: "a spelled place names a value… two tests with the same spelling on one path read one value within an iteration" is false under mutation and impure calls (P1 rows 3–4); the runtime view is claimed for `TYPE_CHECKING`, but applied to any name so spelled | X1, X2 |

## 7. Findings

Ranked by cause: correctness (X1–X3), then extension and representation (X4), then claims and
oracles (X5, X6).

| # | Finding | Principle IDs | Evidence | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| **X1** | **A condition atom is identified by its text (plus a line version), so distinct evaluations merge, and feasible paths are dropped as contradictions** | DM-24, DM-42, DM-07 · G2, G6, G7 | `Atom` encodes kind, place and literal, or opaque text (`condition.rs`); synthetic predicates share fixed text (`predicate.rs` `NON_EMPTY`, `SUPPRESSES`, `FINALLY`, `UNDECIDED`); canonical forms fold `== None` into `is_none` and `is True` into `equals` (ADR-0022 §Conditions); versions are lines (`predicate.rs::version`). P1 and P2, rows 1–6 | A behavior that runs is served as impossible, or silently absent: a flow into a sink after two `range` loops, an `emit()` after a mutation. A negative answer can then rest on a dropped path | **Evaluation identity:** an atom is its kind, with the Python operator preserved, plus its operands, plus the evaluation site; the text is a label. Atoms may be shared across sites only when the observed value is provably stable: an identity (`is None`, `is <literal>`) or `isinstance` test on a **name**, at the same **definition set** (definition ids, not lines). Truthiness, `==`, `in`, attribute places, calls, opaque tests and every synthetic predicate are per site. A new atom kind for `is <literal>` (codebook append); `== None` becomes `equals(p, None)`. Normal-path factoring still works, because a guard and the fates it gates come from one predicate site | `flow_shapes` cases for P1's rows 1–6, plus X6's oracle |
| **X2** | **The runtime view decides by spelling, and compares version tuples by truncated prefix** | DM-02, DM-24 · G1, G2 | `predicate.rs:304-305`: any `TYPE_CHECKING` name, or dotted `X.TYPE_CHECKING`, is false; `:308, 327-329`: `sys`, `os` by spelling; `:315-316`: prefix comparison. P1 and P2, rows 7–9 | A parameter or unrelated attribute named `TYPE_CHECKING` has its branch removed; `sys.version_info > (3, 14)` is decided false on 3.14.7; an alias (`TC`) is never decided | Keep the rename only to stop ty deciding the name itself. Decide the sentinel false, and `sys`/`os` tests at all, only where our resolution binds the name to `typing.TYPE_CHECKING` or the stdlib module; otherwise it is an ordinary atom. Aliases are then decided too. Compare version tuples as Python does: a proper prefix is less | `flow_shapes` cases (`choose`, an alias, `config.TYPE_CHECKING`, the four version operators), plus X6's oracle, which runs on the pinned interpreter itself |
| **X3** | **A translation-induced `false` is dropped at extraction, where no rule can see it** | DM-08, DM-43 · G3 | `lib.rs:601`: `if condition.is_never() { continue; }` (reaching rows); `:697` (value sources). The Stage 2 end review's `semantic:condition-not-false` covers only served rows | X1's losses leave no trace in any table: an absence nothing distinguishes from "no flow" | After X1, `false` can come only from ty (a diagram's always-false), the runtime view, or a contradiction between stable atoms. Keep the skip, but count skipped rows per module in coverage, and let X6 check it against executions | X6's oracle; a coverage count asserted in `flow_shapes` |
| **X4** | **DNF with a 16×8 budget is the only condition representation, so structure past the budget is lost at extraction, and implication and compatibility can only be approximated** | DM-08, DM-40 · G2 (latent) | `condition.rs` (`MAX_CONJUNCTIONS`, `MAX_LITERALS`); 42 `budget_reached` claims on the pilot (`162bda5a…`); ADR-0022 defers compatibility to Stage 3's Q9, and the Stage 2 end review's R9 needed sorting to become order-independent | Stage 3 composes conditions across calls, and Q09 asks "ignored under `transport='sse'`", which needs exact compatibility. DNF composition hits the budget exactly there | A **BDD kernel** (biodivine-lib-bdd 0.6.3) over X1's atoms, with a deterministic variable order (atoms sorted by identity). Persist conditions losslessly (a node table over the atom table; ids from the canonical BDD); the DNF encoding becomes a bounded rendering (`to_optimized_dnf`, a budget on display only). Amend §B10: propositional decision diagrams are allowed; theory solvers stay excluded. Lands at Stage 3 entry, **after X1** | `conditions.rs` known answers, re-derived from the BDD; pilot `budget_reached` measured; a node-limit case |
| **X5** | **"Established" reads as a feasible execution, but the model admits may-behavior** | DM-43, DM-59 · G7 (documentary) | ADR-0022: AMBIGUOUS is `true`; calls are assumed to return; operators are primitive (E18); context managers do not suppress, except `suppress(...)` | An agent reads "established: `y` reaches `sink`" as "there is an execution where it does". It is "the model has not excluded it, and no boundary interrupted the derivation" | Say so in ADR-0022 §Verdicts and in the served tool descriptions: positive verdicts over-approximate, and negative verdicts hold under a complete may-analysis. Later, an `approximated` flag on regions and reaching rows whose diagram path crossed AMBIGUOUS or an assumed return. Per-claim assumption records are deferred | Prose now; a `flow_shapes` case for the flag when it lands |
| **X6** | **No oracle checks the translator against real executions** | DM-60 · G3 | The `flow_shapes` known answers are hand-written; the two-way parity checks names and spans, not truth. P2 found in one run what two design reviews had missed | Every future translation rule (X1's sharing policy, new atom kinds) can drop paths unseen | **A runtime soundness oracle** in the Python lane: small programs (the reviews' shapes, plus Hypothesis-generated ones from a grammar of tests, rebindings, loops, `try`/`with`, calls and mutation) run on the pinned CPython under `sys.monitoring`, with inputs driving each branch. Invariant: every executed statement's region, and every observed reaching definition, is admitted (not `false`, present). Generated programs live outside `fixtures/python` (those are never executed), in an isolated worker. Hypothesis becomes a Python dev dependency | The oracle itself, a pytest in `just check` |

**Observations** (not findings):
- **O1.** The external review's §2B proposes a ty patch. We already maintain one fork (Pyrefly); a
  second doubles the pin-check surface. E6's alternative uses facts we already have.
- **O2.** The external review describes biodivine-lib-bdd as thread-safe with owned memory. Both are
  confirmed by its docs; its canonicity is not stated there, but P3 confirmed it for a fixed
  variable set.
- **O3.** Two of the external review's framings are already true of the design: the provider
  boundary (§1), and DataFusion for relations with Rust for recursion (§3's Ascent boundary).

**Applicability.**
- **Groups that bore:** 2 (absence and validity: X1, X3); 5 (derivation: X1, X4); 9 (providers:
  X2, E6); 12 (claims: X5); 1 (authority: X2).
- **Did not bear:** 6 and 7 (publication and caches are unchanged); 8 (performance is measured
  only for the proposal, in §9).

## 8. Alternatives

| Alternative | Soundness | Cost | Verdict |
|---|---|---|---|
| **Baseline:** patch P1's nine shapes one by one | The class remains; the next shape drops paths again | Small now, recurring | Rejected |
| **The external review's full program** at once: identity, BDD, memory model, implicit calls, Ascent, z3, the Python lane | Strong | Large, and it outruns its consumers (§B10, ADR-0001) | Rejected as a batch |
| **Simpler viable alternative:** X1 (evaluation identity, operators kept), X2 (resolution and tuple semantics), X3 (skips counted), and X6 (the runtime oracle) now, as Stage 2.9. X4 (the BDD kernel) at Stage 3's entry, where composition and Q09 need it. Everything else deferred with triggers | Sound under the stated model, and checked by execution | Small: one translator change, one codebook append, one oracle; the kernel in Stage 3 | **Recommended** |

## 9. Verification and measurement

| Claim | Label | Check |
|---|---|---|
| Eight translation defects reproduce | **Tested** (P1, a temporary test; 2026-09-24) | To become `flow_shapes` cases |
| The runtime oracle catches them | **Tested** (P2: CPython 3.14.7, `sys.monitoring` LINE events; 2026-09-24) | X6 |
| biodivine-lib-bdd: canonical forms, factoring, compatibility, limits, deterministic rendering | **Tested** (P3; 2026-09-24) | The Stage 3 spike's exit tests |
| biodivine-lib-bdd cost | **Measured** (P3: release build, Rust 1.98.1, this machine; 2026-09-24). 500 conditions of ≤16×8 over 20 atoms built in 30 ms; 499 conjunctions in 15 ms; 50 optimized DNF renderings in 77 ms. Over 40 atoms: 499 conjunctions in 423 ms, 31 of them past a 50,000-node limit; renderings about 20 ms each. A random 32×8 over 40 atoms did not finish in 10 minutes | A node limit and a bounded rendering are required. Pilot conditions are structured, not random: measure them in the spike |
| CrossHair finds operator differences on 3.14 | **Tested** (P4; 2026-09-24) | Stage 3.1 models |
| OxiDD, z3, Ascent | **Interface-checked** (docs and registry) | Deferred or planned |

## 10. Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| E2 ty type observations | No consumer; Pyrefly types | A question needing narrowed types that Pyrefly cannot answer |
| E6 ty runtime-aware patch | Our own resolution suffices (X2) | A runtime-view case resolution cannot express |
| E11 ty saturation exposure | Sound direction | A pilot scope measurably degraded by saturation |
| E13 OxiDD | Heavier; nothing needs ternary or multi-root management | E12 measured too slow or too large |
| E15 z3 | §B10; no query needs it | A query the BDD plus typed atoms cannot decide |
| E17 points-to and memory versions | X1's per-site rule gives soundness | A Stage 3 field effect needing aliasing |
| E18 implicit protocol calls | Primitive-operand assumption, stated (X5) | An evaluation item turning on a dunder method |
| E19 deferred execution phases | Stage 5's lifecycle questions | Stage 5 |
| E20 bytecode CFG | X6 is more direct | A region defect runtime inputs cannot reach |
| X5's per-claim assumption records | The flag first | A served claim misread because of an assumption |

## 11. Decision and implementation changes

**Decision: Revise the target design** before ADR-0022 is accepted. G1, G2, G3, G6 and G7 fail on
the translation, and the external review's core diagnosis (E3) is confirmed by execution.

**Changes to the target design:**
1. **ADR-0022, amended in place** (still `proposed`):
   - §Conditions: evaluation identity and the stable-sharing rule (X1); operators preserved, with
     an `is <literal>` atom (codebook append); synthetic predicates per site; counted skips (X3).
   - §Places: definition-set versions replace `place@line`; mutation and impure calls are named in
     the rule.
   - §Composed layers: the runtime view is decided by resolution, and version tuples follow
     Python's semantics (X2).
   - §Verdicts: the approximation direction (X5).
2. **A new ADR at Stage 3 entry:** the BDD condition kernel (X4). It amends §B10's "not a solver"
   wording, allowing propositional decision diagrams while keeping theory solvers out, with E12's
   spike exit tests.
3. **DESIGN** §3.9 and §9.9 follow both.

**Changes to the plan:**

| Priority | Change | Where |
|---|---|---|
| P1 | **Stage 2.9, semantic-bridge hardening:** X1, X2, X3; `flow_shapes` cases for P1's shapes; X6's runtime oracle. Then the compact re-review of the Stage 2 end review, and ADR-0022's acceptance | New, before Stage 3 |
| P2 | **Stage 3.0, the condition kernel:** a biodivine-lib-bdd spike with exit tests (canonical ids equal the `conditions.rs` known answers; pilot `budget_reached` down; the node limit reported); lossless persisted conditions | Stage 3, before summaries |
| P2 | **Stage 3.1 addition:** CrossHair as the models catalog's oracle, isolated | Stage 3.1 |
| P3 | Stage 3's handlers carry exceptional exits (E19); Stage 5 adds deferred-execution phases and protocol operations (E18, E19) | Stages 3 and 5 |
| — | Ascent stays behind its trigger; Pysa stays the cross-check (B21); z3, OxiDD and the ty patches are deferred (§10) | — |

**Final check.**
- **Claims against evidence:** matched. Every accepted source claim of the external review was
  reproduced (P1, P2); every library verdict cites a probe or its docs.
- **Where the design is unsafe today:** conditions over repeated calls, loops over `range`,
  mutated state, `==` against `is`, version checks against a two-part tuple, and any name spelled
  `TYPE_CHECKING`.
