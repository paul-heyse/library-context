# Design review: library fit, with the symbolic-reasoning libraries in depth (2026-09-25)

A design/target review of the Rust workspace and Python serving code, judged by one question:
does each generic capability sit in the library whose semantics fit it, and does the code use the
pinned libraries' relevant surface? The operator asked for the most depth on the libraries of the
`rust-reasoning` skill: biodivine-lib-bdd, OxiDD, z3, ascent, datafrog and fcars. The other pinned
libraries were reviewed through the same lens.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Code at `92eca2e` (clean tree): `crates/*`, `python/lctx_semantics`, `python/lctx_mcp`. Documents: DESIGN §B4, §B5, §B10, §3.9, §6.3, §9; ADR-0011, 0024, 0032, 0039; the [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md) §7, §9.2, §12 and its execution queue |
| Standard | Core 3.0, code-intelligence profile 1.1, the library-context binding (`standard.toml`) |
| Tier · purpose | Design · target. It covers library placement across the assembled Stage 3 code and its planned consumers. It is not a Stage 3 exit review |
| Reviewer · date | Claude (design-review skill; not the author of the code under review) · 2026-09-25. Five read-only investigations covered conditions, fixpoints, analytics, relational code, and extraction/serving; the reviewer re-read each finding's cited source. [Probe](../evidence/2026-09-25_bdd-decision-apis/README.md) rerun by the reviewer |
| Maturity and outcome | Stage 3 in progress. This review should settle which reasoning libraries to adopt, expand, defer or reject before plan orders 1, 6 and 9. It should also surface bespoke code that a pinned library already covers |
| Supported scope | Condition kernel and primitive theory; flow model and summary composition; Pass A/B/C; FCA/RCA, communities and ranking; DataFusion/Delta relational work; ty/Pyrefly/Ruff provider use; the independent oracles; the PyO3/FastMCP serving boundary |
| Expected changes | S1: operation-wide compatibility for Q09. S2: recursive SCC summary composition (order 6), then effect, exception and role families. S3: a new condition-atom kind or theory. S4: an analyzer upgrade (ty/Pyrefly). S5: FORMAT 7 native serving (order 9). S6: an independent challenge of a published claim (order 8) |
| Baseline | biodivine-lib-bdd is linked in `cpg-schema`. fcars is a dev-only oracle. Deferred with triggers: ascent (three recursive families), OxiDD (biodivine too slow or large), z3 (a query the kernel cannot decide) (plan §7, §12) |
| Method and coverage | Source reading at file:line, the pinned skill indexes (`reference.py`), the vendored library source and one executed probe. Nothing is built or tested in-repo: every in-repo test outcome is `not_run`. The two 2026-09-23 library-leverage reviews were read, and their settled items are re-opened only where a trigger fired. Not examined: CI-G2 evidence closure of served claims, Stage F synthesis templates, embedding clients |

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility (hidden decisions) | Consumer contract | Dependencies | Expected reason for change |
|---|---|---|---|---|
| `cpg-schema::condition_kernel` | Boolean condition algebra over atom ids: canonical diagrams, bounded apply, Merkle ids, hydration (variable order, budgets, biodivine handles) | `Diagram`/`BoundedCondition`: `and/or/not/given/implies/compatible`, `Result<_, KernelBoundary>` | biodivine-lib-bdd | New decision queries (S1), budgets, a theory (S3) |
| `cpg-schema::primitive_theory` | Evaluating an exact query value against source atoms through value links | Refutation/compatibility with cited links, `TheoryWork` | kernel | New atom kinds (S3), Q09 membership (S1) |
| `cpg-core::flow_model` | Within-function value sources (`Model::reach`), dynamic-access and abstract/stub premises, exit sites | `value_flows`, `value_flow_contributions`, boundaries | Ruff syntax facts, ty flow facts, kernel | Recursion/loop semantics, new premises |
| `cpg-core::summaries` → `lctx-analytics::summaries` | Call-graph SCC schedule and finite composition (ARC-02 moves the policy into analytics) | `summary_flows`, `summary_flow_steps`, `summary_boundaries` | petgraph, kernel, DataFusion | Order 6 SCC fixpoint (S2) and further families |
| `lctx-analytics` passes/FCA/communities/ranking | Topology and heuristic analytics over declared projections | Arrow in, Arrow out; `analysis_invocations` | petgraph, leiden-rs, fixedbitset | New analytics with consumers (§9.8) |
| `cpg-schema` SQL + `cpg-core` attempt/validate | Relational construction and validation | Arrow schemas, one query per rule | DataFusion, delta-rs | New fact families |
| `cpg-flow` | ty semantic index → flow facts, joined by byte range | `flow` rows | ty 0.0.14, salsa 0.28.2 | Analyzer upgrade (S4) |
| `lctx_semantics` (PyO3) + `lctx_mcp` | One immutable generation per process and native condition/path queries | FastMCP tools with structured results | cpg-schema, FastMCP | FORMAT 7 (S5) |

| Concept | Authority | Derived forms and consumers |
|---|---|---|
| Condition function | Persisted root plus nodes over sorted atom ids (ADR-0024/0032) | In-memory `Diagram`, rendered DNF (display), native executor |
| Decision "compatible / implies / unknown" | `condition_kernel.rs:638-664` | Predecessor screen, summary seeds, native `compatible` |
| Recursion schedule | petgraph SCCs plus a keyed sort (`lctx-analytics/src/summaries.rs:29-120`) | `summary_components`, composition order |
| Proof-step kinds | `cpg-schema` `SummaryFlowStepKind`, with a native re-whitelist (ARC-01) | PyO3 loader |
| Choice of analysis engine | ADR-0011 amendments, DESIGN §B4, plan §7 | This review |

**CI fact and fidelity table (in-scope relations only).**

| Relation | Provider | Fidelity | Coverage and unknowns | Consumers |
|---|---|---|---|---|
| Flow reachability/regions | ty 0.0.14 index | resolved | ty's settings are `ProgramSettings::empty`, i.e. Python 3.10 on the default platform, not the recorded 3.14.7 (F11) | conditions, exits |
| Value sources | our `Model::reach` | derived | loop-carried contributions may be lost at an SCC head (F05) | summaries, negative premises |
| Dynamic-access boundary | our `flow_model` | derived | a computed name that starts with a quote counts as a literal (F09) | negative premises |
| Abstract/stub body | our text heuristic | heuristic | Pyrefly's resolved `FuncFlags` are unused (F10) | negative premises |
| Condition decisions | kernel | exact under the Boolean model, else a typed boundary | some decidable questions answer `NodeLimit` (F01) | predecessor screen, serving |

## 3. Contracts, constraints and testing boundaries

| Contract | Consumer expectation | Enforcement | Failure | Isolated verification |
|---|---|---|---|---|
| Kernel decision | `Ok(bool)` is exact; `Err(boundary)` is unknown with a cause | Preflight plus `binary_op_with_limit(50_000)` | Correct direction, but over-refuses (F01, F02); its atom limit depends on history (F02) | Unit tests with no store: good. **No property oracle independent of biodivine** (F12) |
| Primitive theory | Refute only on a definite contradiction; cite links | 32-assignment/1M budget | Cited links are an order-dependent prefix, not minimal (F03); `MemberOf`/`Equals{Int}` return unknown (F04) | Unit tests |
| `Model::reach` | All may-sources with transfer and condition | Memo plus a lowlink cut | No work bound or exhaustion outcome (F05) | Needs the loop fixture in F05 |
| Recursive SQL walks | Distinct reachable terms | Consumers apply `DISTINCT` afterwards | Row blow-up per path; termination rests on extraction's depth 32 (F07) | Read-only probe |
| Native loader | Typed generation tables | Positional string tuples, re-whitelisted (ARC-01) | Drift between the Rust schema and the native side (F13) | Python tests |

## 4. Composition and execution (analysis records)

| Question | Projection | Method | Exact/conservative/heuristic | Budgets and partial result | Output linkage |
|---|---|---|---|---|---|
| Are two conditions compatible / does one imply the other? | Union atom vocabulary | BDD apply, 50k result nodes, 1M pair preflight | Exact under the Boolean model | Refusal is `KernelBoundary` and never false. It is **over-conservative** where the refusal itself decides (F01) | Cited roots |
| Which sources reach a use? | Use→def graph per function | Memoized DFS with a lowlink cut | Claimed complete may-analysis | **None declared** (F05) | `value_flow_contributions` |
| Which recursive summaries hold? | Call graph, SCCs | Acyclic walk, depth 8; recursive members withheld | Conservative | Refusals lose their cause (ARC-03) | `summary_flows`/steps |
| Pass B forwarding | Flows | BFS worklist with a depth bound | Conservative | `DepthLimit` → `BudgetReached` | Findings; the first path decides suppression (F14) |
| FCA/RCA | Declared context | In-house NextClosure; fcars oracle | Exact | Closed-set budget 20k | `concepts` |
| Type-term closure | `type_term_args` | `WITH RECURSIVE … UNION ALL` | Exact after `DISTINCT` | Implicit depth 32 (F07) | Concepts, communities |

## 5. Change scenarios

| Scenario | Owner | Contract change | Expected vs observed consumers | Hidden knowledge / test setup | Evidence |
|---|---|---|---|---|---|
| **S1: Q09 "fates compatible with `transport == 'sse'`"** | kernel + theory | None: `MemberOf`/`Equals{Int}` evaluation and exact decisions | Expected: theory plus kernel only. Observed: the same; route is local | None; unit-testable | Interface-checked. FastMCP `transport.py:91,99,417` tests `in {...}` |
| **S2: SCC composition, then effect/exception/role families** | ARC-02's `lctx-analytics::summaries` | New relations and typed refusals (ARC-03) | Expected: one rule set per family on shared recursion semantics. Observed: plan order 6 prescribes a hand-written monotone worklist per SCC, and "trigger the Ascent spike only after three recursive rule families repeat the worklist shape" | By its own wording the trigger fires only after bespoke loops exist (F06). `Model::reach` and Pass B already count as two such families | Proposed |
| **S3: a two-place relation or ordered comparison between atoms** | theory | New atom kind | Expected: a solver seam in the theory only. Observed: the kernel is theory-blind by design (B012); the theory owns the escalation point | Clear; z3 would sit behind `primitive_theory` | Proposed; trigger in §8 |
| **S4: ty upgrade** | `cpg-flow` | None | Expected: `cpg-flow` only. Observed: also needs `ProgramSettings` from the run context (F11) | Python version and platform are implicit | Interface-checked |
| **S5: FORMAT 7 native serving** | `lctx_semantics` | Generation table schemas | Expected: decode via `cpg-schema` row types. Observed: positional tuples plus a re-whitelist, so each new column or kind edits both sides (F13, ARC-01) | Tuple indices | Interface-checked |
| **S6: challenge a published negative/ignore claim** | test lane | None | Expected: an oracle that falsifies claims. Observed: `sys.monitoring` checks raw flow only, over 10 fixed programs (F12) | — | Interface-checked |

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence | Required action |
|---|---|---|---|
| G1 Authority | fail (carried) | ARC-01's native re-whitelist of proof kinds, already owned by plan §9.2. DESIGN §B4/ADR-0011 name `kosaraju_scc` for condensation, while summaries use `tarjan_scc` under a separate amendment (F08) | ARC-01; F13; record F08 |
| G2 Semantic fidelity | unresolved | ty analyses as Python 3.10 while the run context records 3.14.7 (F11). Dynamic-access literal test by text (F09) | F09, F11 |
| G3 Validity | pass (scope) | Kernel hydration validates order, reduction, closure and Merkle ids (`condition_kernel.rs:164-228`) | — |
| G4 Hidden behavior | pass (scope) | No ambient analyzer config found. F11 is a constant, not an ambient read | — |
| G5 Consistency and recovery | unresolved | `Model::reach` declares no work bound or exhaustion outcome (DP-12). Recursive CTE termination depends on an external cap (F07) | F05, F07 |
| G6 Transformation and reuse | unresolved | `AtomLimit` depends on construction history, not the function (F02). Pass B's suppression depends on which path is found first; Pass B/C do not check input order (F14) | F02, F14 |
| G7 Truthful capability claims | fail (minor) | DESIGN §6.3 claims additive migrations via `SchemaMode::Merge`; no code uses it and `delta.rs` `verify` rejects any schema difference (F16) | F16 |
| G8 Library leverage | fail | Pinned built-ins left unused: biodivine limit-1 decisions, `check_binary_op`, `support_set`, `restrict` (F01–F03). Pyrefly `FuncFlags` (F10). Ruff literal node kinds (F09). DataFusion `UNION` (F07). A recursion engine is scheduled only after bespoke loops (F06) | F01–F03, F06, F07, F09, F10 |
| CI-G1 Fidelity | unresolved | F05 (possible narrowed may-conditions feeding summaries), F09 and F10 (negative premises on heuristic evidence) | F05, F09, F10 |
| CI-G2 Evidence closure | not assessed | Outside this library-fit scope | — |
| CI-G3 Evaluation integrity | pass (scope) | Oracles and gold stay outside compiler inputs. Pysa's call graph comes from Pyrefly, so it is not independent for call edges; only its TITO taint result is (F12) | — |

## 7. Findings

Evidence labels are core §D, dated 2026-09-25 unless stated otherwise.

<a id="F01"></a>**F01: the kernel reports decidable compatibility and implication as unknown, and bounds work by a product estimate.**
- **Principles:** DP-14, DP-11, CI-04 · G8.
- **Evidence:**
  - `compatible` and `implies` (`condition_kernel.rs:638-664`) build the whole `and`/`and_not` result under a 50k-node cap and map an over-cap result to a boundary.
  - In biodivine 0.6.3, `binary_op_with_limit(1, …)` returns `None` only for a non-false result. That makes `None` a proof of compatibility, and of non-implication for `and_not`. Upstream `Bdd::cmp_implies` uses this idiom (`_impl_sort.rs:37-56`).
  - `Bdd::check_binary_op(limit, …)` returns `(non_empty, tasks)` without allocating a result. It needs at most as many tasks as the `|a|×|b|` preflight estimate, and on the probe shape used 131,070 tasks against about 1.7·10¹⁰.
  - [Probe](../evidence/2026-09-25_bdd-decision-apis/README.md), **Tested** (standalone crate, not the kernel).
- **Consequence:** the ADR-0039 predecessor screen, the summary seeds and native `compatible` withhold answers the library decides. The 2026-09-24 spike had 31 of 499 conjunctions over the cap. Pilot frequency has not been measured.
- **Correction (`condition_kernel`):** decide `compatible`/`implies` with `check_binary_op(MAX_PAIR_WORK, …)`, using the pair-product preflight only as a cheap early accept. Alternatively, use `binary_op_with_limit(1, …)` plus the existing preflight. Report real tasks in `TheoryWork`. `KernelBoundary` is unchanged.
- **Closure:** in-repo tests covering a satisfiable pair with a result over 50k (`Ok(true)`), a contradiction whose pair product exceeds `MAX_PAIR_WORK` but which `check_binary_op` decides, and a task-limit control.

<a id="F02"></a>**F02: declared support never shrinks, so `AtomLimit` depends on construction history.**
- **Principles:** DP-04, DP-11 · G6.
- **Evidence:** `union()` keeps `self.support ∪ other.support` (`condition_kernel.rs:550-570`), while hydration derives support from the nodes. `support_set()` of `(x∧¬x)∨y` is one variable (probe).
- **Consequence:** a compile-time fold (`flow_model.rs:899`) can persist an `AtomLimit` boundary for a function that would pass once re-hydrated. The boundary becomes a property of the evaluation path rather than of the condition.
- **Correction:** after each operation, transfer into `support_set()` when it is smaller. This is linear, and order is preserved (B006).
- **Closure:** a test that repeatedly composes redundant atoms and stays under the limit, with in-memory support equal to hydrated support.

<a id="F03"></a>**F03: restriction and cofactoring are re-implemented, factoring is still DNF-bound, and refutations cite a non-minimal prefix.**
- **Principles:** DP-14, DP-16 · G8.
- **Evidence:**
  - `BoundedCondition::given` returns `self` whenever the legacy DNF is over budget (`condition_kernel.rs:769-790`), which is exactly the case ADR-0024 adopted BDDs for.
  - `primitive_theory.rs:230-252` conjoins one literal diagram per assignment through union, transfer and apply.
  - biodivine offers `is_clause`, `restrict` and `restrict_valuation`, linear and bounded by input size (probe). The skill brief `reason.bdd-implication` prescribes `restrict` for `given`.
- **Consequence:** conditions lose factoring precisely where they are large. Refutation links follow BTreeMap order rather than being an irredundant set, which weakens the served explanation.
- **Correction:**
  - Use `restrict` for cube factors and keep DNF nomination for non-cubes.
  - Make theory assignment a single `restrict`, then minimize the links by deletion (at most 32 restricts). This is the BDD analogue of an unsat core, with no solver.
  - Return the residual condition for display.
- **Closure:** a test where a factor that is over budget in DNF now factors, with `factor ∧ q == f` still gating. A test where two links are cited but one is irrelevant cites only the needed one. Existing theory fixtures are unchanged.

<a id="F04"></a>**F04 (functional gap blocking S1, no library needed): `MemberOf` and `Equals{Int}` are not evaluated.**
- **Evidence:** `primitive_theory.rs:281-315` returns `None` for them. Q09.a–d (`eval/behavior/fastmcp-4.0.5.toml:438-487`) target `transport in {...}` branches.
- **Consequence:** the scenario that motivated considering z3 is blocked by this gap, not by a missing theory.
- **Correction:** add string membership over all-string literal sets, and integer equality for non-bool ints. Keep mixed bool/number sets unknown (`True in {1}`).
- **Closure:** a fixture mirroring `transport.py:91-99`, with the control `True in {1}` giving unknown.

<a id="F05"></a>**F05: `Model::reach` is not a fixpoint at a cycle head and has no work bound.**
- **Principles:** CI-07, DP-12, CI-08 · CI-G1, G5.
- **Evidence:** `flow_model.rs:705-826` memoizes the node where a cycle closes after one pass (`low >= depth`). Sources are keyed by `(Origin, Transfer)`, so a contribution that exists only around the loop (e.g. `x = p; while k(): x = h(x)` reaching `h`'s argument through the call) is cut. Members of the SCC other than the head are recomputed along every path, by native recursion. **Interface-checked**; not reproduced.
- **Consequence:** narrowed may-sources and conditions in `value_flow_contributions` can feed summary positives and negative premises. Dense loops have unbounded work and no boundary.
- **Correction (`flow_model`):** petgraph SCCs over the use→use graph, iterating each SCC to a fixpoint with a work cap that writes `BudgetReached`. This is `reason.fixpoint-choice` rung 1: one relation over one graph. Move it to the F06 engine only if one is adopted.
- **Closure:** the loop fixture above, run before and after the fix; a dense-SCC fixture hitting the cap; byte-identical output on shuffled input.

<a id="F06"></a>**F06: the recursion-engine decision is sequenced to fire only after bespoke loops exist.**
- **Principles:** DP-13, DP-16, CI-07 · A3, G8.
- **Evidence:**
  - Plan order 6 and §7 schedule a hand-written monotone worklist per SCC for value composition, then effect, exception and role families.
  - The ascent spike triggers only "after three recursive rule families repeat the worklist shape". Two already exist (`Model::reach`, Pass B), and order 6 adds up to four.
  - `reason.fixpoint-choice` recommends ascent exactly when "several recursive families share a worklist shape", with stratified negation for "no admitted path → unknown" (B020, C001), lattices for widening (B019), and deterministic single-threaded hashing.
  - Gaps, from the pinned source and briefs and not probed here:
    - there is no deterministic whole-run budget (`run_timeout` is wall-clock), so bounds must be depth columns, bounded-set lattices, and a per-SCC preflight;
    - there is no derivation provenance, but ADR-0034's content-addressed proof ids make witnesses data;
    - composing BDD roots inside rules needs fallible pure helpers or a between-runs step;
    - macro compile cost is not measured.
- **Consequence:** as written, the plan produces two to four hand-written fixpoints before the library comparison, and the spike then competes against sunk code. ARC-03's typed refusal causes map naturally onto rules (`refused(f, p, site, DepthCap)`), whereas a worklist re-derives them per family.
- **Correction:**
  - Re-sequence, without an ADR yet. After ARC-02/03, implement order-6 **value** composition as the ascent spike over the isolated transform, with the hand-written per-SCC loop as the fallback if the spike stalls on BDD composition.
  - Decide adopt or reject before the effect family starts, and record the outcome as an ADR-0011 amendment.
  - datafrog is not the candidate: its value is run-time rule assembly, which nothing here needs (C009 first-element joins add friction).
- **Closure:**
  - order-6 fixtures (acyclic, terminating base, self- and mutual recursion, parallel edge, open override, cap) pass under the chosen engine;
  - shuffled-input byte identity;
  - a cap fixture whose specific cause reaches `summary_boundaries`;
  - measured `lctx-analytics` compile-time delta.

<a id="F07"></a>**F07: recursive CTEs over shared type DAGs use `UNION ALL`, and their adoption superseded a recorded rejection without a record.**
- **Principles:** CI-08, DP-12, DP-24 · G5, G8.
- **Evidence:**
  - `cpg-schema/src/concepts.rs:36-40` and `communities.rs:39-51`.
  - A read-only probe on the pilot store gave 51,246 rows for 29,871 distinct, and 10,029 for 1,418 distinct. **Tested** by the relational investigation against snapshot `ecf8b9cd…`; not rerun by the reviewer.
  - Termination rests on extraction's depth cap of 32.
  - The 2026-09-23 leverage review rejected recursive CTEs; they were added afterwards in `5ccd38f`, `ad4eb53` and `9c23705`.
- **Correction:**
  - Use `UNION` in both walks: set semantics, with termination on cycles.
  - Record the superseded rejection in ADR-0011's amendment log, or in the plan's Stage 4 "Adopt DataFusion `WITH RECURSIVE`" row, which already exists.
  - Keep the depth-guarded `behavior.rs` climbs.
- **Closure:** rule snapshots unchanged; a cyclic type-term fixture terminates.

<a id="F08"></a>**F08: SCC routine and keyed topological sort vs the decision record.**
- **Principles:** DP-15, DP-12 · G1 (records).
- **Evidence:**
  - `lctx-analytics/src/summaries.rs:15,56` uses `tarjan_scc`, which recurses (petgraph 0.8.3 `tarjan_scc.rs:53`).
  - ADR-0011:96-97 and DESIGN §B4 prefer `kosaraju_scc` because Tarjan recurses, while the 2026-09-24 amendment (ADR-0011:204, DESIGN:348, 3417) says `tarjan_scc`.
  - The hand-written keyed topological sort (`summaries.rs:98-120`) is exactly ADR-0011's rustworkx-core trigger. `lexicographical_topological_sort` exists at the pinned version.
- **Consequence:** there is a latent stack-depth risk on a large library's call graph on a tokio worker thread (not measured), and two statements in the decision record contradict each other.
- **Correction:**
  - Switch to `kosaraju_scc`. The order is re-sorted anyway, so output is unchanged.
  - Record the rustworkx-core trigger as evaluated and kept bespoke. The reason is about 22 tested lines against about 7 crates and a condensation build.
- **Closure:** the existing SCC tests and component-order test.

<a id="F09"></a>**F09: dynamic-access names are judged literal by leading quote.**
- **Principles:** CI-04, DP-02, DP-14 · CI-G1, G8.
- **Evidence:** `flow_model.rs:1963-1968`. `"a" + x`, `"{}".format(x)` and `"".join(p)` all pass as literals, so the call is skipped at `:2001` and no boundary is written. Latent for FastMCP (no such call).
- **Correction:** use Ruff's `ExprStringLiteral` node kind from `syntax_nodes`.
- **Closure:** a fixture with all three forms writes boundaries; the plain literal control does not.

<a id="F10"></a>**F10: abstract/stub status comes from decorator text rather than Pyrefly's resolved flags.**
- **Principles:** DP-13, CI-02 · G8, CI-G1.
- **Evidence:**
  - `flow_model.rs:1238-1282` compares the last dotted segment with `abstractmethod`. That misses `abstractproperty`, aliases and Protocol members.
  - Pyrefly's `FuncFlags.is_abstract_method` and `body_kind` are available in fork `a07b7ba` (**Interface-checked** by the extraction investigation).
- **Correction:** persist Pyrefly's flag in `cpg-extract` (append-only column, a schema migration) and take the union with the body-shape rule.
- **Closure:** fixtures covering an aliased decorator and a Protocol member.

<a id="F11"></a>**F11: ty runs with empty program settings.**
- **Principles:** CI-10, DP-02 · G2.
- **Evidence:** `cpg-flow/src/lib.rs:327-329` calls `ProgramSettings::empty`. The Python version and platform differ from the recorded run context. No pilot effect today (no star imports).
- **Correction:** build the settings from `RuntimeContext`, about 3 lines.
- **Closure:** a version-gated fixture whose flow facts change with the configured version.

<a id="F12"></a>**F12: independent challenge is thinner than the claims it would challenge.**
- **Principles:** DP-23, FP-04 · (gate verdicts above).
- **Evidence:**
  - Kernel tests are example-based. No property test compares the kernel with an independent semantics, although `proptest` is pinned.
  - The `sys.monitoring` oracle (`tests/scripts/test_flow_soundness.py`) checks raw flow over 10 programs, with Hypothesis varying one integer.
  - No oracle targets ADR-0039's "ignored because disjoint" claim, raise escapes, or future `refuted_under_model`.
  - Pysa's call graph is Pyrefly's own, so only its TITO result is independent.
- **Correction:**
  - (a) A proptest truth-table oracle for `and/or/not/implies/compatible/given/restrict` over at most 10 atoms. It is independent of biodivine, so no z3 is needed.
  - (b) Plan order 8: a monitoring falsifier over generated packages that compiles through summaries and executes the ignored call then the cited return.
- **Closure:** (a) runs in the kernel tests. (b) Plan order 8 records disagreements as fixtures.

<a id="F13"></a>**F13: the PyO3 boundary carries positional string tuples.**
- **Principles:** DP-10, FP-04, DP-02 · G1 (via ARC-01).
- **Evidence:** the loader passes 13 positional hex/codebook-text tuple arguments (`generation.py:473-547`, `lctx_semantics/src/lib.rs:207-221`). Results are decoded by index.
- **Correction:** hand each digest-verified generation table to Rust as Arrow IPC bytes. Decode it with the pinned `arrow-ipc` 59.3 and `cpg-schema`'s row types, which check codebooks. This closes ARC-01 by construction and adds no crate; `arrow-pyarrow` is a heavier alternative.
- **Closure:** ARC-01's closure check, plus a schema drift test between the generation and the native reader.

<a id="F14"></a>**F14: order dependence in Pass B/C.**
- **Principles:** DP-11 · G6.
- **Evidence:**
  - `pass_b::Flows::build` and `pass_c::Handoffs::build` do not refuse unsorted input, unlike `graph.rs:157,195`.
  - Pass B's raise suppression reads the first BFS path to each state (`pass_b.rs:380-409`).
- **Correction:** add the same `windows(2)` order check, and a "clean path" bit in the visited key.
- **Closure:** a shuffled-input test, and a fixture where a clean path is discovered second.

<a id="F15"></a>**F15 (lower priority): SQL-only analysis tables round-trip through Rust rows.**
- **Principles:** DP-16, DP-23.
- **Evidence:**
  - About 21 tables in `attempt.rs:651-1000` are queried, materialized as Rust structs, then rebuilt. `derive.rs:22-45` already handles SQL → strict cast → canonical sort.
  - Their validators re-run the same SQL over the same inputs. That checks write fidelity, not independent semantics.
- **Correction:** move pure-SQL tables to the derive path and keep the genuinely independent re-derivations (`finite_flows`, `predecessor_compatibility`, `call_components`, the model catalog, contributions).
- **Closure:** identical snapshots, and fewer validator rules re-expressing SQL.

<a id="F16"></a>**F16: §6.3 claims `SchemaMode::Merge` migrations that no code performs.**
- **Principles:** DP-22 · G7.
- **Evidence:** DESIGN.md:2176; no `SchemaMode` anywhere in `crates/`; `delta.rs:113` `verify` rejects any difference.
- **Correction:** narrow §6.3 to "a schema change means a fresh store" (the practice).
- **Closure:** DESIGN amendment.

**Observations (not actionable findings):**
- **FCA:** keep fcars as the oracle, not the engine. It gives no implications, support threshold or cover relation, and no concept order (B024). The in-house NextClosure is sound, but its fcars oracle never exceeds 9 attributes against a pilot of 193–363, so add one case above 64 attributes.
- **odis:** re-record its rejection on merit (a non-optional `reqwest` in a hermetic compiler, low adoption) rather than on licence.
- **leiden-rs:** `converged = iterations < max_iterations` misreports convergence on the last permitted iteration.
- **Minor:**
  - `ranking.rs:3-4` says "definite calls" where the policy is definite or candidate;
  - hex ids are spliced into SQL in four places, contrary to `query.rs:9-11`;
  - `entry_links.rs:324-343` keeps the first duplicate after a non-total `ORDER BY`, so either assert that duplicates are identical or make the order total.
- **Kernel design holds:** it avoids `to_optimized_dnf`, never persists `BddVariable`, and always transfers into one union set.
- **Serving and storage, no change:**
  - FastMCP fits §11.3.
  - delta-rs CDF, generated columns and deletion vectors have no consumer.
  - datafusion-tracing stays deferred.
  - The DataFusion memory pool stays deferred, but record the validation-stage peak at the next pilot.

## 8. Library fit and total complexity

### 8.1 The `rust-reasoning` libraries

| Library | Where it could serve | Pinned semantic fit and gaps | Burden | Decision |
|---|---|---|---|---|
| **biodivine-lib-bdd 0.6.3** (in use) | Kernel decisions, factoring, theory, work accounting, witnesses | Unused built-ins fit exactly: limit-1 decisions, `check_binary_op`, `support_set`, `restrict`/`restrict_valuation`, `is_clause`; `sat_witness`/`most_free_clause` for display. The fixed variable order stays: it is part of the persisted format; revisit on pilot `node_limit` hits (B009) | None new | **Expand** (F01–F03). `sat_witness` for a compatibility explanation: defer until a served consumer asks |
| **OxiDD 0.12.0** | A shared manager; ZBDD/MTBDD | Content-addressed node rows already share structure. A fixed-capacity manager fails as a whole (B010), where the kernel relies on per-operation caps. ZBDD/MTBDD have no consumer. Feature traps (C005, C010). The plan §7 trigger is unmet: 13,775 rows in 0.97 s | New family, `AllocResult` throughout | **Reject** for this workload; reopen only on pilot evidence that biodivine is too slow or large |
| **z3 0.21.1 on Z3 5.1.0** | Theory reasoning beyond one place vs literals | The atom language (`condition.rs:65-105`) has no order, arithmetic or two-place relation. S1 is finite-domain (F04). Source-source relations on one stable value encode as BDD constraints (brief `reason.smt-escalation`), and minimal refutations come from F03 rather than unsat cores. Costs: timeouts give `Unknown` (B013), seeds pick models (B014), version-dependent compilation (C013), a C++ runtime in the PyO3 wheel. `libz3.so.5.1` is present under `/usr/local` (verified) | High for a narrow gain | **Keep deferred, with a sharper trigger:** a registered question needs an atom that relates two places, or an ordered/length/arithmetic comparison a finite partition cannot express. It would sit behind `primitive_theory` (S3) and never become the kernel |
| **ascent 0.8.1** | Recursive summary families (S2), possibly `Model::reach` | Declarative rules; compile-time stratified negation (C001, B020); lattice widening (B019); deterministic single-threaded runs. No budget or provenance, so encode bounds in the rules and treat content-addressed ids as witnesses | One crate in `lctx-analytics`, about 5 small transitive crates, macro compile time (unmeasured) | **Spike now as order 6's value composition** (F06); adopt if the effect family reuses its rules |
| **datafrog 2.0.1** | The same | Manual semi-naive loop; joins keyed on the first element (C009); its advantage is run-time rules | Minimal | **Reject** in favour of ascent; no consumer for run-time rules |
| **fcars 0.2.2** (dev) | FCA oracle | Concepts only; no order (B024); unconditional rayon | Dev-only | **Keep as oracle**; widen its test contexts (§7 observations) |

### 8.2 Other pinned and candidate libraries

| Capability | Candidate | Decision |
|---|---|---|
| SCC schedule | petgraph `kosaraju_scc` (pinned) | Adopt (F08) |
| Keyed toposort, centralities | rustworkx-core | Keep bespoke; its trigger fired, is recorded and was declined (F08). No consumer for new centralities |
| Personalized PageRank | graphops, graphina | Defer (no consumer; graphops' petgraph feature is 0.6, C004) |
| Recursive relational closure | DataFusion `WITH RECURSIVE … UNION` | Expand (F07) |
| Pure-SQL analysis tables | `derive.rs` path | Expand (F15) |
| Bag difference in validators | DataFusion `EXCEPT ALL` | Reject: in DataFusion 55.1 it behaves as a left-anti join (probe in the relational investigation) |
| Abstract/stub facts | Pyrefly `FuncFlags` | Adopt (F10) |
| Literal detection | Ruff node kinds | Adopt (F09) |
| Implicit end-of-function exit | ty `end_of_scope_reachability` | Expand at plan order 2/7 (one append-only exit kind) |
| Predecessor authority | ty `IsNonTerminalCall` | Reject as authority: statement-level calls only. At most a parity check |
| Narrowing constraints | ty | Defer (no failing consumer) |
| Generation transfer | `arrow-ipc` 59.3 (pinned) | Adopt with ARC-01 (F13) |
| Runtime oracle | `sys.monitoring` + Hypothesis | Expand (F12 b) |
| Kernel oracle | proptest truth tables (pinned) | Adopt (F12 a) |
| Compiled-control-flow oracle | `bytecode` 0.19 | Defer to order 2's effectful frames |
| Delta CDF, generated columns, `SchemaMode::Merge` | delta-rs | No consumer; narrow the DESIGN claim instead (F16) |

## 9. Alternatives for the two structural choices

| Alternative | Change propagation | Authority and composition | Test boundary | Machinery and risk | Decision |
|---|---|---|---|---|---|
| **Conditions.** Baseline: biodivine used as a bare apply engine | Local | One kernel | Unit tests | Over-refusal and history-dependent limits | Revise |
| Expanded biodivine (F01–F03) | Local to `condition_kernel`/`primitive_theory` | Same authority | Same, plus a truth-table oracle | No new crate | **Chosen** |
| z3 behind the theory | Theory seam only | Theory-blind kernel kept | Needs its own oracle discipline | C++ runtime, `Unknown`, seeds | Deferred (trigger in §8.1) |
| OxiDD kernel | Whole kernel and persisted format | Same | Re-validation of the format | Manager lifecycle | Rejected |
| **Recursion.** Baseline plan: a hand-written worklist per family | Each family re-derives bounds, refusals and schedule | Several independent semantic loops | Per-family setup | Grows with families | Revise sequencing |
| ascent rule sets on the ARC-02 transform | New family = new rules | One engine owns recursion; refusals are relations | Pure Rust inputs → sorted outputs | Macro compile time; BDD composition in rules | **Spike first** |
| Simplest: petgraph SCCs + a small per-SCC loop | Acceptable for one family | Fine for rung 1 | Same | Least now | Fallback, and the fix for F05 |

## 10. Verification and uncertainty

| Claim | Label/date | Basis | Gap |
|---|---|---|---|
| Limit-1 decisions, `check_binary_op`, `support_set`, `restrict` behave as stated | Tested 2026-09-25 (standalone probe) | [Evidence](../evidence/2026-09-25_bdd-decision-apis/README.md) | Not yet in the kernel; pilot frequency unmeasured |
| `Model::reach` loses loop-carried contributions | Interface-checked | Source at `flow_model.rs:705-826` | The fixture in F05 |
| `UNION ALL` row multiplication | Tested (investigation, snapshot `ecf8b9cd…`) | Read-only `lctx query` | Not rerun by the reviewer |
| ascent fits order 6 | Proposed | Skill briefs B019/B020/C001; an uncompiled sketch in the investigation notes | The spike |
| z3 unnecessary for S1 | Interface-checked | Atom grammar and Q09 sources | Holds until a relational atom kind is registered |
| In-repo tests | `not_run` | No build was run by this review | — |

## 11. Authority changes and dispositions

**Transferred 2026-09-25.** F01–F16 are scheduled in the forward plan; [its §6](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition) is their single current disposition owner. The table below is this review's dated recommendation, not current status.

None of these findings is transferred to the active plan yet. Until the operator schedules them,
this review owns them as **open**, per binding §4. Once scheduled, the plan's findings table becomes
the owner and this section keeps only the link.

| Required change | Route and owner | Findings | Closure or revisit trigger |
|---|---|---|---|
| Kernel decision/work/support/restrict expansion | Implementation within ADR-0024 (no ADR; the contract is unchanged) · `cpg-schema` | F01–F03, F12a | Tests in F01–F03 and F12 |
| `MemberOf`/`Equals{Int}` | Implementation · `primitive_theory` | F04 | Q09 fixture |
| `Model::reach` SCC fixpoint with work cap | Implementation · `cpg-core::flow_model` | F05 | F05 fixtures |
| Order-6 engine spike; ADR-0011 amendment with the outcome | Plan re-sequencing, then ADR-0011 amendment · `lctx-analytics::summaries` | F06 | Decision recorded before the effect family starts |
| `UNION`; record superseded CTE rejection | Implementation + ADR-0011/plan note · `cpg-schema` | F07 | Cyclic fixture |
| `kosaraju_scc`; record rustworkx-core trigger | Implementation + ADR-0011 amendment | F08 | SCC tests |
| Ruff literal kind; Pyrefly `FuncFlags`; ty settings | Implementation (F10 is a schema migration) · `cpg-core`, `cpg-extract`, `cpg-flow` | F09–F11 | Fixtures in each |
| Arrow IPC generation transfer | Folded into ARC-01 · `lctx_semantics` | F13 | ARC-01 closure |
| Pass B/C order and suppression | Implementation · `lctx-analytics` | F14 | Shuffle test |
| Derive-path consolidation | Implementation · `cpg-core` | F15 | Deferred: next edit to `attempt.rs` table construction |
| §6.3 narrowed | DESIGN amendment | F16 | — |
| z3 trigger sharpened; OxiDD/datafrog rejections recorded | Plan §7/§12 edit | §8.1 | As stated in §8.1 |

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence | Action |
|---|---|---|---|
| A1 Localize change | satisfied for S1 and S3; unresolved for S5 | Kernel and theory changes stay behind `Diagram`/`primitive_theory` (F01–F04). A z3 seam would sit in the theory alone. Native serving edits both sides of a positional boundary (F13) | F13 with ARC-01 |
| A2 Encode meaning structurally | violated (records), unresolved (flow) | The SCC decision record contradicts itself (F08). The CTE rejection was superseded silently (F07). The ty configuration is implicit (F11) | F07, F08, F11 |
| A3 Extend through composition | unresolved | S2: each recursive family is planned as its own loop, and the engine decision comes only after those loops exist (F06). This depends on ARC-02/03 | F06 after ARC-02/03 |

**Bounded decision: Revise.** G8 fails for concrete, pinned, local built-ins (F01–F03, F09, F10).
CI-G1, G5 and G6 are unresolved for F05, F07, F02 and F14. None of these needs a new crate. The only
new library this review recommends is **ascent**, as a spike tied to order 6, with adoption decided
on the spike's evidence. **z3 and OxiDD are not justified by any current or planned consumer.** z3's
trigger is sharpened, and the scenario that seemed to need it (Q09) is blocked by F04 instead.

**Enclosing architecture:** not certified by this review. It remains governed by the
Stage 3 calibration reviews (ARC-01–03, plan §9.2). Nothing here is release qualification.

| Priority | Change and component | Findings | Closure or trigger |
|---|---|---|---|
| 1 | `Model::reach` fixpoint plus work cap (`flow_model`); dynamic-access literal kind | F05, F09 | Fixtures |
| 2 | Kernel expansion plus truth-table oracle (`condition_kernel`, `primitive_theory`), then `MemberOf`/`Equals{Int}` | F01–F04, F12a | Tests; Q09 fixture |
| 3 | Order-6 ascent spike after ARC-02/03 (`lctx-analytics::summaries`) | F06 | ADR-0011 amendment |
| 4 | Small placements: `UNION`, `kosaraju_scc`, ty settings, Pyrefly flags, Pass B/C order | F07, F08, F10, F11, F14 | Per finding |
| 5 | Arrow IPC with ARC-01; derive consolidation; §6.3 | F13, F15, F16 | ARC-01; next table edit |

Next step: the operator decides whether to transfer F01–F16 into the plan's findings table and
re-sequence order 6 (F06). The first implementation step is F05's fixture in `cpg-core::flow_model`.
