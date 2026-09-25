# Plan: the behavioral model, going forward

**Supersedes:** [`behavioral-model-pivot-plan_2026-09-24.md`](behavioral-model-pivot-plan_2026-09-24.md).
Its Stages 0–2 are built. This plan carries forward their open follow-ups and the rest of its
scope (Stages 3–5).

**Integrates:**
- the external review [`docs/full_cpg_pipeline_external_review.md`](../full_cpg_pipeline_external_review.md),
  as assessed in
  [`design_review_external-review-assessment_2026-09-24.md`](../design_review/reviews/design_review_external-review-assessment_2026-09-24.md)
  (decision **Revise**; findings X1–X6; recommendations E1–E25);
- the Stage 2 end review,
  [`design_review_stage2-end_2026-09-24.md`](../design_review/reviews/design_review_stage2-end_2026-09-24.md)
  (**Revise**; R1–R11, dispositioned in its §12).

**Date:** 2026-09-24. **Status:** Active. The operator approved the direction on 2026-09-24: keep
ty as the flow provider and Pysa as the cross-check (deviation B21), and adopt the assessment's
recommendations, including decisions D-10 to D-13 (§11; deviation B23).

**Authority.** This plan is not authority. `docs/design/DESIGN.md` and the ADRs are. A stage that
changes a §B decision lands its ADR, and a `standard` review of it, first.

---

## 1. Purpose and target

**The product** (unchanged, ADR-0021) is an **evidence-carrying behavioral model of a pinned
library's whole public surface**. An agent asks which operations accept X, pass it to Y, under
which configuration, raising what, needing which lifecycle. It gets:
- **exhaustive answers** where the analysis is complete;
- **ranked candidates** where only discovery applies;
- **a named unknown** where the analysis stopped.

Each claim carries evidence and a derivation. Briefs stay, as one rendering.

**What changes in the target.** The external review and its assessment showed that the weak point
is no longer which provider to use. It is **the translation from provider facts into our
conditions**. An atom is identified by its text, so distinct evaluations merge, and feasible paths
are dropped. Eight shapes reproduce this, and CPython executes every "impossible" statement
(assessment §5). The target therefore adds three things:

1. **A sound semantic bridge.**
   - A condition atom is an **evaluation**: its kind, with the Python operator preserved, its
     operands, and its site.
   - Atoms are shared across sites only where the observed value is provably stable.
   - The runtime view is decided by **resolved bindings**, not spelling.
2. **A condition kernel.** Conditions are Boolean functions in a **BDD** over those atoms,
   persisted losslessly. DNF becomes a bounded rendering. Implication and compatibility become
   exact over the atoms, with a typed theory for primitive places.
3. **An independent execution oracle.** Generated and curated programs run on the pinned CPython
   under `sys.monitoring`. **Every observed execution must be admitted by the model.**

**Kept, unchanged:**
- programmatic synthesis; no generative model in the pipeline or the query path (§B11);
- immutable, byte-identical generations (§B7, §B12);
- declared dependency families (ADR-0002 amendment);
- structured, qualitative evaluation with pre-registered targets; no API-agent evaluation;
- the gold (`.claude/skills/`) is never a compiler input;
- licence is never a criterion.

## 2. Where we start

**Built** (Stages 0–2 of the superseded plan): the whole-surface behavior relations, the operation
catalog, bundle `FORMAT` 5, the three operation tools, the `flow` family from ty, the condition
language, five verdicts, and Stage 2's behaviors. Evaluation v0 is pre-registered in
`eval/behavior/fastmcp-4.0.5.toml`.

**Baseline** (Measured, pilot snapshot `162bda5a0fe39cc1cedadbfaa8efd1f9`, 2026-09-24, fake
embedder, uncommitted Stage 2 end-review fixes):

| Quantity | Value |
|---|---|
| Public declarations | 1,534. Operation status: 636 established, 535 unknown, 363 not analyzed (classes) |
| Behaviors | 15,415 |
| Behaviors by verdict | 4,276 established; 3,554 conditional; 19 refuted |
| Unknown behaviors, by reason | **6,137 `call_transfer`**; 1,347 `override_dispatch`; 42 `budget_reached`; 39 `abstract_body`; 1 `runtime_unreachable` |
| Flow facts | 66,954 uses; 69,010 reaching rows; 53,466 value sources; 6,148 tests; 13,764 attribute loads; 11,184 conditions |
| Behavior tables | 22,565 value flows; 52 ambient reads; 790 raise sites (35 may be caught) |
| Pilot compile | 40.4 s; the flow model takes 1.73 s |

**Open follow-ups from what is built:**

| Item | State | Where it lands |
|---|---|---|
| The Stage 2 end review's fixes (R1–R11, O5): new rules, probes, ADR-0022 and DESIGN amendments | Implemented and tested (nextest 262/262; `just pilot` passed), **uncommitted**. Part of it is in the operator's commit `3129c7c` | 2.9.1 |
| Test attribution follows identity and derived transfers only, not call transfers | Proposed; the edit was not applied | 2.9.1 |
| The Stage 2 exit rule, re-applied to the final Stage 2 build | Passed on `85304572`, before the end review's fixes | 2.9.8 |
| ADR-0022's acceptance | Proposed; blocked on the re-review | 2.9.9 |
| `STATUS.md` | Stale (last written at `90eadeb`) | 2.9.10 |
| Operator reviews | Pending: the Stage 1 and Stage 2 evaluations; deviations B1–B22; the moved-aside stores `build/store-pre-*` (about 11 GB) | §11 |

## 3. Principles

T1–T9 carry over from the superseded plan. T5 is restated, and T10–T11 are new.

| # | Principle | Without it |
|---|---|---|
| T1 | The whole public surface is the universe; analysis runs per callable, bottom-up | Silence about most operations |
| T2 | Relations first, sentences last: typed, persisted relations with in-row provenance | Knowledge locked in templates |
| T3 | Our abstraction, stated: the flow IR is our declared runtime model (§B5) | Checker views relabelled as runtime flow |
| T4 | Meaning comes from models, propagation from summaries | "Calls X, so can X" |
| **T5** | **Conditions are Boolean functions over evaluation atoms, held in a decision-diagram kernel.** Propositional reasoning, plus a typed theory for primitive places, is allowed. **No theory solver** (§B10) | Merged evaluations drop feasible paths (X1); budget cuts lose structure (X4) |
| T6 | Five verdicts, never a null; a negative verdict only inside complete coverage | Unsupported negatives |
| T7 | Materialize source facts at compile time; run bounded semantic selections in the pinned Rust executor | A second, unproven serve-time condition semantics in Python |
| T8 | Discovery nominates, definitions decide | Statistical membership |
| T9 | Pre-registered behavioral question sets, plus mechanical known-answer fixtures | Circular evaluation |
| **T10** | **Providers observe; our semantic layer concludes.** Provider facts are observations under the provider's model. What they justify is decided by our rules, which are stated and tested | Provider artefacts become claims |
| **T11** | **Execution checks admission.** Within the stated model, every observed execution is admitted by the analysis. A violation is a counterexample to the analysis. A finite pass is evidence, not proof | Translation defects stay invisible (X6) |

## 4. Target architecture

| Layer | What | State | Changes in this plan |
|---|---|---|---|
| L0 | The CPG | Built | — |
| **L1** | Flow facts (`flow` family, ty through `cpg-flow`) | Built | Evaluation atoms; the runtime view by resolution; lossless conditions; counted skips (Stage 2.9, 3.0) |
| **L1.5** | **The condition kernel** | New | A BDD over atoms with a typed theory; implication and compatibility; bounded rendering (Stage 3.0) |
| **L2** | Behavior relations | Built in part | Adds `handlers`, `callbacks`, `resources` and exits (Stage 3.2) |
| **L3** | Transfer summaries | Planned | Resolves `call_transfer` (Stage 3.3) |
| **L4** | Models catalog | Planned | CrossHair as its oracle (Stage 3.1) |
| **L5** | Capability registry and membership | Planned | Definitions use the kernel's compatibility (Stage 4) |
| L6 | Discovery (FCA, RCA, communities, views) | Built | New consumers in Stage 4 |
| **L7** | Serving | `FORMAT` 5; three tools | Kernel-backed filters (Stage 3.6); `lookup_concepts`, `explain` (Stage 4) |
| L8 | Structured evaluation | Built | Stage exit rules continue |
| **L9** | **The validation lane** | New | Runtime soundness oracle (Stage 2.9); CrossHair model checks (Stage 3.1); Pysa differential (Stage 3.4) |

### 4.1 The condition model (the target; ADR-0022, amended in Stage 2.9; new ADR in Stage 3.0)

**Atoms are evaluations.**
- An atom is a kind, with the Python operator kept (`is`, `==`, `in`, truthiness, `isinstance`),
  its operands, and the **evaluation site** of the test that produced it.
- Its source text is a label, never its identity.
- `x == None` is equality, not `is None`. `x is True` is an identity atom (a codebook append), not
  `== True`.

**Sharing across sites requires a proof of the same runtime value.** Stage 2.9 keeps all source
tests per site. Its first definition-set rule was superseded by the compact review's executable
counterexample: a nested call can rebind a `nonlocal` without changing the use-def set. Stage 3.0
may share equality, membership or truthiness only when it also proves no intervening write or
effect can change the name's value; a Pyrefly builtin immutable-scalar type at each use is
necessary for the typed theory but insufficient for cross-site sharing.

These remain per site unless that proof is added:
- calls and other opaque tests;
- attribute places (a call may mutate them);
- object state such as a container's truthiness;
- the synthetic predicates (non-empty iterable, context-manager suppression, finally, undecided).

`place@line` is retired. Reaching-definition sets remain flow facts, but are not atom identities
without an effect-stability proof.

**The runtime view** decides a test only where our reference resolution binds the name:
- the `TYPE_CHECKING` sentinel, where the name resolves to `typing.TYPE_CHECKING` (aliases
  included);
- `sys.version_info`, `sys.platform` and `os.name`, where `sys` or `os` resolve to the stdlib
  modules.

Elsewhere the name is an ordinary atom. Version tuples compare as Python compares them: the
five-field `sys.version_info` is greater than an equal prefix of up to three fields. A longer
literal stays undecided until release level and serial are modeled. The rename stays, only to
stop ty deciding the name by spelling.

**The kernel** (Stage 3.0) is biodivine-lib-bdd 0.6.3.
- **Variable order:** deterministic, with atoms sorted by identity.
- **Operations:** composition, restriction, existential projection, and exact implication and
  compatibility over atoms.
- **Typed theory:** constraints between shared atoms on one stable value (`x == "a"` excludes
  `x == "b"`).
- **Persistence:** lossless, as a node table over the atom table, with content ids from the
  canonical BDD. The DNF encoding is a rendering with a display budget. A node limit, not a DNF
  budget, is the only point where information is cut, and that is `unknown` (`budget_reached`).
- **Normal-path factoring** nominates a quotient with the bounded Stage 2 rule and accepts it only
  after checking `F == N ∧ G` with the bounded diagram kernel. If the check cannot establish that
  equality, F remains authoritative with `not_factored` (ADR-0024).

**The approximation direction** is stated in ADR-0022 §Verdicts and in the tool descriptions:
- positive verdicts are may-behavior the model admits (AMBIGUOUS read as true; calls assumed to
  return; operators, f-strings and containers over primitive operands; context managers other
  than `suppress` assumed not to suppress);
- a negative verdict holds under a complete may-analysis.

An `approximated` flag on regions and reaching rows whose diagram path crossed AMBIGUOUS or an
assumed return lands with the kernel.

### 4.2 The validation lane (L9)

The lane has three instruments, each with one purpose.

| Instrument | Checks | Where it runs |
|---|---|---|
| **Runtime soundness oracle** | Every executed statement's region is admitted (not `false`); every observed reaching definition is present; from Stage 3.5, every observed value flow is admitted | Programs from the two reviews' shapes, plus Hypothesis-generated ones from a grammar (tests, rebindings, loops, `try`/`with`, calls, mutation), run on the pinned CPython 3.14.7 under `sys.monitoring`, with inputs driving each branch. They live outside `fixtures/python/`, which is never executed, and run in an isolated worker. A pytest, in `just check` |
| **CrossHair** (`diffbehavior`) | A model of a pure callable in the catalog (Stage 3.1) agrees with the real function; a canonical-form assumption holds | An isolated worker, with timeouts; replayed concretely before a counterexample becomes a fixture |
| **Pysa** (pyre-check 0.10.0, `--pyrefly-binary` pinned) | `summary_flows` against Pysa's taint-in-taint-out (TITO) models; each disagreement explained, or turned into a fixture; Pysa's obscure-callee default never copied | Offline, on the pinned library (Stage 3.4) |

Outputs are test results and findings. They never change a static conclusion directly (T10).

## 5. Stages

During design and implementation, use focused checks and small commits that state the check
outcome (`not_run` where appropriate). Run `just test-all` and `just pilot` once at the end of the
integrated Stage 3 scope, with stage timings and peak RSS. Keep the planned reviews and handoffs;
where a stage review coincides with an increment's end, one review at the increment's depth covers
both. This cadence supersedes the earlier per-stage full-gate wording (operator direction,
2026-09-24).

**Increments** (DESIGN §1.2):
- increment 4 is Stages 2.9–3, ending with a `compact` review;
- increment 5 is Stages 4–5 and the held-out evaluation, ending with a `deep` review.

### Stage 2.9: harden the semantic bridge, and close Stage 2

1. **Commit the Stage 2 end review's fixes.** First decide and apply the test-attribution rule:
   only identity and derived transfers name a parameter, and a call transfer is the callee's
   question. Then run `just test-all` and commit, naming R1–R11 and deviations B21–B22, with the
   migrations declared (`flow_tests`, `flow_attribute_loads`, `raise_sites.escapes`,
   `abstract_body`, `EXTRACTOR_OUTPUT_VERSION` 22).
2. **Evaluation identity (X1, E3, E4, E8).**
   - Atom identity as §4.1: operators preserved; the `is <literal>` atom (a `condition_atom`
     append); per-site synthetic predicates; definition-set versions; the stable-sharing rule for
     names.
   - `flow_shapes` cases for each of the assessment's P1 shapes, and for same-line bindings and
     merged definitions.
   - `EXTRACTOR_OUTPUT_VERSION` is bumped, and the snapshots move as declared migrations.
3. **The runtime view by resolution (X2, E5, E7).** `cpg-extract` passes, per module, the spans
   our resolution binds to `typing.TYPE_CHECKING`, the stdlib `sys` and `os`. The translator
   decides only those. Version tuples compare with Python's semantics. Cases: a `TYPE_CHECKING`
   parameter, an alias, `config.TYPE_CHECKING`, and the four comparison operators against a
   two-part tuple.
4. **Counted skips (X3).** Each module's coverage row records how many reaching rows and value
   sources were skipped as `false`, and why: ty's own always-false, the runtime view, or a
   stable-atom contradiction.
5. **The runtime soundness oracle (X6, E21).** Build the §4.2 oracle for regions and reaching
   definitions:
   - a small `lctx flow` developer command (flow facts of one file as JSON);
   - a pytest driver with Hypothesis (a new Python dev dependency) and `sys.monitoring`;
   - the reviews' shapes as its seed corpus.

   It runs in `just check`.
6. **The stated model (X5, E18).** ADR-0022 §Verdicts states the approximation direction and its
   assumptions, and the tool descriptions say what `established` means.
7. **ADR-0022, amended in place:**
   - §Conditions: atoms, operators, sharing, skips;
   - §Places: definition-set versions, mutation, impure calls;
   - §Composed layers: the runtime view by resolution and version semantics;
   - §Verdicts: the direction.

   DESIGN §3.9 follows.
8. **Re-apply Stage 2's pre-registered exit rule** to the hardened generation (Q04, Q10, Q21–Q24;
   live vectors for Q23 and Q24 when the GPU has ≥30 GB free, and vLLM is stopped afterwards).
   The assessment goes in the Stage 2 evaluation document as a dated addendum. A failure is fixed
   here.
9. **A `compact` re-review** of R1–R8 and X1–X3 and X6 (the `design-reviewer` subagent). Then
   **accept ADR-0022**.
10. **Handoff** (`STATUS.md`).

**Exit:**
- `flow_shapes` (including the P1 shapes), `behavior_shapes` and its probes, and the runtime oracle
  all pass;
- Stage 2's exit rule passes on the hardened build;
- ADR-0022 is accepted.

**Closes:** X1, X2, X3, X5, X6; R1–R11.

### Stage 3: the condition kernel, models and summaries

**0. The condition kernel (X4, E10, E12).**
- **A new ADR:** conditions as Boolean functions in a decision-diagram kernel over evaluation
  atoms. It amends §B10's "not a solver" wording: propositional diagrams and a typed theory for
  primitive places are allowed; theory solvers stay excluded. It also amends DESIGN §3.9 and
  §9.9. A `standard` review follows.
- **A spike, with exit tests:**
  - canonical ids reproduce the `conditions.rs` known answers under a fixed variable order;
  - normal-path factoring by equivalence reproduces Stage 2's `given` results;
  - determinism under shuffled input;
  - a node-limit case;
  - on the pilot, `budget_reached` falls, and the node limit's hits and compile time are reported.
- **Then, in the product:**
  - construct and compose BDDs in `cpg-flow` before the Stage 2 DNF limit. The read-only
    2026-09-24 pilot survey converted all 13,775 stated condition rows but found one
    `over_budget` sentinel shared by 237 regions and 846 reaching facts across modules;
    post-extraction conversion cannot recover those separate expressions. Measure any
    reduction in the 66 recorded budget claims on a product pilot. Migrate the flow-model
    consumer with the same root/node schema;
  - lossless conditions (a node table in the `flow` family), with the DNF encoding as a bounded
    rendering;
  - an attributed test-use/type-term observation and the typed theory: only exact builtin
    runtime values under the stated type-trust model, with stable-value evidence across sites;
    broad annotations and possible subclasses stay undecided;
  - the kernel's `implies`/`compatible` API, which Stage 3.6's filters and Stage 4's definitions
    use;
  - the `approximated` flag.

**Stage 3.0 implementation order after design/probe sign-off:**
1. In `cpg-schema::condition_kernel`, add direct construction from a typed evaluation atom and
   an explicit boundary-bearing result. The BDD is the producer's Boolean authority; a capped
   DNF rendering is output only. Test `false`/`true` plus a budget boundary, 17 independent OR
   sites, source-order shuffles, `given` equivalence and corrupt-node refusal.
2. Replace `cpg-flow`'s use of Stage 2 `Condition::and`/`or`/`not` at predicate creation and
   region/value/reaching composition. Preserve ADR-0022's per-evaluation identity. Carry a
   boundary reason and `approximated` status; never turn a failed apply into `false` or a
   negative claim. The 17-site case must stay stated before any persistence work starts.
3. Migrate `conditions` to structural root/condition ids and add the lossless node relation in
   the `flow` family. Update `cpg-extract::flow` deduplication, the extractor output version,
   Arrow contract snapshots and the flow consumer in `cpg-core::flow_model` together. The
   consumer must read the validated root/node closure for decisions, not parse display text.
   Old DNF ids never silently join new ids.
4. Add one shared graph validator used by publication and native generation load. It checks
   format, root closure, terminal and Merkle ids, atom order/support, reduction and budgets.
   Tamper a published-node fixture through the Delta path; a malformed generation is rejected
   before a query can run. Keep the node-format handshake in the manifest.
5. Emit `flow_test_leaves` per provider predicate and atom before `flow_tests`' span-only dedup.
   Include provider predicate identity, test span, condition root and leaf atom/span so separate
   `match` arms sharing a subject span remain distinct. Then produce `flow_test_types` against
   that leaf row and the exact `flow_uses` operand; join Pyrefly's trace by operand-use span.
   Freeze the Arrow row and uniqueness contract in ADR-0024 first: snapshot/module/scope,
   provider predicate key, test/root, full encoded atom plus atom id and leaf span. Validate
   root support, recomputed source-module key, `Site` or `Synthetic` identity and exact-one
   use-row match before any proof row is admitted.
   Add only the closed exact-runtime-origin cases and effect-stability witnesses; an
   absent/ambiguous mapping leaves atoms independent. The later `flow_test_value_links` bridge
   cites the same leaf row and requires its own justified subject-use mapping for synthetic
   pattern predicates. Pin compound Boolean and two-arm `match` cases, plus sibling-operand,
   annotation-use and call/write counterexamples, before pilot measurement.
   The read-only [test-leaf join probe](../design_review/evidence/2026-09-24_test-leaf-proof-joins/README.md)
   establishes the source identity gap; it does not validate the proposed new relation.
6. Run focused source counterexamples first, then the product pilot and the registered Stage 3
   condition questions. Report the old 66 `budget_reached` claims against the new count, node
   and work-limit hits, condition roots/nodes, compile time and RSS. The read-only conversion
   survey's 23,760 unique nodes and 53-node maximum are a sizing baseline, not an exit result.

**1. Models catalog v1** (the superseded plan's D-7 list, unchanged).
- **Stdlib and dependencies:** `open`/`io`/`pathlib`, `json`, `gzip`/`zlib`, logging; asyncio and
  anyio (`fail_after`, `move_on_after`, `to_thread`, task groups); contextvars;
  `functools.partial`/`wraps`; `contextlib`; pydantic `BaseModel`, `Field`,
  `TypeAdapter.validate_python`; pydantic-settings `BaseSettings`; httpx timeouts; the starlette
  and uvicorn entry points.
- **Added:** `typing.cast` (identity) and the builtins that carry a value (`str`, `dict`, `list`,
  `tuple`).
- **Guards:** the catalog's digest joins `compiler_digest`; no model cites `.claude/skills`.
- **CrossHair** checks each model of a pure callable in an isolated worker (E22).

**2. The remaining L2 relations** (Stage 2's unbuilt part):
- `handlers`: caught types, and the action (re-raise, convert, swallow, value);
- `callbacks`: stored, invoked, forwarded or registered;
- `resources`: acquire and release;
- exits: normal, exceptional, `finally` (E19).

**3. The summary kernel** `lctx_analytics::summaries` (L3).
- **Call results as intermediate values:** a callee's summary (formal → `ReturnValue`, field
  writes, `Raise[T]`) resolves each `call_transfer` claim to `established`, `conditional`,
  `refuted_under_model` (a complete summary with no transfer), or `unknown` with its boundary.
- **Composition:** SCC bottom-up (`tarjan_scc`, callees first). A fixpoint per SCC, bounded by
  path depth k and the kernel's node limit, widening to `unknown`. Joins over override-open
  candidates. Exception conversion through `handlers`. Invocation rows record budgets.
- **Tables:** `summary_flows`, `summary_effects`, `summary_boundaries`.
- **Reported:** the `call_transfer` count, against 6,137 today.

**4. Ascent spike**, only on its trigger (three or more recursive rule families repeating the
worklist shape). ascent 0.8.1 must match the hand kernel on `behavior_shapes`, with timings. The
boundary is DataFusion for relations, Rust or Ascent for recursion, Arrow for contracts (E14).

**5. The Pysa oracle** (§4.2).

**6. `behavior_shapes` part 2:**
- the "used only for logging" case;
- a wrapper that disables an option of its callee;
- the runtime oracle, extended to observed value flows.

**7. Tools:**
- summarized flows and effects in `get_operation`;
- effect and role filters in `find_operations`;
- a compatibility filter ("fates compatible with `transport == 'sse'`"), because Q09 needs one.
- Its input is anchored to the operation's entry formal; a checked entry-to-test value/stability
  bridge is required before comparing with a persisted predicate. Missing bridges return
  `unknown`, and compatibility means may-model non-refutation, not concrete feasibility.
- The focused [entry-value bridge probe](../design_review/evidence/2026-09-24_entry-value-bridge/README.md)
  fixes the first producer cases: direct formal reach may link after exact operand and effect
  checks; explicit rebind, unknown source or an unmodeled call/write withholds the link. A
  post-call reaching row that still names the formal does not prove stability.
- Supersede ADR-0010's lookup-only materialized executor. The Python FastMCP layer loads an
  in-process Rust/PyO3 semantic executor against one immutable generation. Typed compatibility,
  implication, effect/role filtering and bounded witness traversal may run at query time; row,
  node, pair-work and depth limits produce explicit `unknown` and `truncated`. Direct lookups and
  ranked retrieval keep their existing materialized routes. A `standard` review covers the §B13
  pivot (ADR-0025) before acceptance.
- **Design-phase loop:** use focused kernel/translator tests, editable `uv run` import and
  targeted probes while the node relation, proof contracts and served API are being designed.
  Run the broader repository and pilot gates at the integrated Stage 3 end. At Stage 3.6 acceptance,
  after the generation format and served API settle, build and clean-install one wheel and
  exercise a generation-pinned FastMCP/native query; repeat that check for release.

**Exit:** `behavior_shapes` part 2 passes, and Stage 3's pre-registered exit rule (Q01, Q03, Q05,
Q09) passes. Then increment 4's `compact` review.

**Closes:** X4; the superseded plan's F4 (the models part) and F8 (the data-file part).

#### Stage 3 execution contract (Proposed, 2026-09-24)

This section resolves the implementation order and representation boundaries of items 0–7.
It does not widen their exit rule. The first direct entry-formal → test-use relation is
**Implemented and Tested**; its 2026-09-24 pilot published 102 positive links. No typed
contradiction or served compatibility result follows from that count.

1. **Finish identity before theory.** Keep `flow_test_types` as Pyrefly observation and
   `flow_test_value_links` as a separate positive proof. Extend the link origin codebook only
   for a transfer whose entry formal, operand use, condition and intervening effects have cited
   witnesses. A missing, ambiguous or capped join stays `unknown`. Do not infer identity from
   parameter spelling, type annotation, enclosing span alone, or a post-call reaching row.
   Preserve the current direct origin's exact no-effect contract as a stable baseline.
2. **Give exact runtime origins their own relation.** Admit a literal value or a runtime
   `type(value) is <builtin>` guard only with exact operand attribution, a cited branch
   condition, and a same-value witness from the guard's use to the later use. `isinstance`, a
   narrowed Pyrefly type, an annotation, and a protocol are not exact class origins. Extend the
   closed atom language with a typed exact-type test if the guard cannot be represented without
   opaque text; append its codebook entry and migrate the condition format explicitly. The
   origin row cites the test leaf, operand use, condition root, runtime class/value, and the
   value-stability link. Keep an unproved origin absent, never a broad fallback.
   Resolve both `type` and its class operand to the intended builtins using lexical/reference
   facts; the spelling alone is insufficient. The trusted one-argument builtin `type(value)`
   call is effect-free for the argument value; document that exemption in the same effect-rule
   digest as the link producer. A later use across this guard needs a path-specific stability
   witness instead of the current blanket prior-predicate barrier. A default literal is not an
   exact entry-formal value when callers may pass an argument; a query-supplied literal is a
   separate exact query origin.
3. **Use the existing BDD for the primitive theory.** Derive bounded constraints only for
   atoms proved to concern the same stable entry value. Safe initial exclusions are `is_none`
   against a proved non-`None` singleton, and unequal string equality tests when the value is
   proved to be exact builtin `str`. Do not treat `==` like `is`, or assume distinct integer
   and Boolean literals compare unequal (`1 == True`). A query about an operation's formal
   introduces a separate query atom; checked links and exact-origin witnesses provide the
   equivalences or exclusions that connect it to source evaluation atoms. Conjoin these
   constraints with the existing diagram through the kernel's node- and pair-work-capped apply.
   A cap, unproved link or unproved exact origin returns `unknown` with a boundary; it never
   becomes `false`. This uses biodivine-lib-bdd 0.6.3's bounded Boolean operations instead of
   introducing Z3 for a small, closed primitive theory. Add Z3 only if a registered question
   requires arithmetic, order or another theory that the Boolean constraints cannot express.
4. **Compile models as typed data, not a third string interpreter.** Parse committed TOML with
   serde's unknown-field rejection into tagged input/output path, transfer, effect, callback,
   resource and exception variants. One canonical renderer emits the DESIGN §9.9 access-path
   spelling for Arrow and display; model authors do not hand-author another free-text grammar.
   Model identity includes file bytes, target callable identity, target library pin and model
   revision. The catalog digest joins `compiler_digest`; malformed or unresolved targets fail
   before publication. Distinguish a model of a dependency callable from an observation of the
   analyzed release, and keep `synthetic_model` provenance on its derived effects. Author the
   Stage 3.1 list in small families (pure identity/value constructors; I/O and serialization;
   async/timeouts/context; pydantic and HTTP), using pinned library source/docs. CrossHair
   checks pure models in an isolated worker; a timeout or unexplored path is inconclusive.
5. **Derive the remaining L2 relations before summaries.** `handlers` names caught types and
   the action on each normal and exceptional exit; `callbacks` distinguishes storage,
   registration, forwarding and invocation; `resources` names acquire/release and the exit path;
   `exits` distinguishes normal return, raise and `finally`. Keep candidate/open dispatch and
   suppressing context managers as boundaries until a model proves an action. Use Ruff syntax
   identity, ty flow and DataFusion joins for local relations; do not infer a handled exception
   or invoked callback from syntactic containment. Each relation gets a schema, codebook where
   needed, publication validator and one positive plus one withholding case.
6. **Compose finite summaries with existing graph and condition kernels.** Resolve call targets
   relationally, use petgraph's SCC decomposition with callees first, then a monotone worklist
   within each SCC. Canonical summary keys contain callable, input/output resolved paths,
   effect or transfer kind, condition root and cited call/model facts. Cap path depth, BDD nodes,
   pair work and SCC iterations; exhaustion writes `summary_boundaries` and makes dependent
   verdicts `unknown`. A complete summary may refute a transfer only when all candidate calls,
   handlers and modeled effects are closed. Keep DataFusion for joins and petgraph for SCCs;
   add Ascent only on the plan's repeated-recursive-rule trigger. Record the initial 6,137
   `call_transfer` claims and the exact number discharged or still unknown on the end pilot.
7. **Check independent claims with the matching oracle.** CrossHair `diffbehavior` checks pure
   model semantics; only exhausted paths support an equivalence claim. Pysa runs on the pinned
   source with a rule that actually joins source and sink and with its pyrefly binary specified;
   compare TITO sets, treating `obscure` and silence as inconclusive. Hypothesis drives small
   generated Python programs under CPython 3.14 `sys.monitoring` for observed flow/exits; it
   uses a private tool id, explicit profile, bounded examples, temporary storage and a timeout.
   The generated programs are isolated; analyzed libraries and `fixtures/python/` are never
   executed. These oracles disagree independently with the Rust producer; none writes facts.
8. **Serve one validated FORMAT 7 generation.** Add structural condition ids, proof rows and
   summary rows to the serving projection and manifest, all tied to one snapshot and checked
   before the native executor admits them. PyO3 owns one immutable indexed executor per process;
   the Python FastMCP layer only validates requests and shapes structured results. A
   compatibility request names a real public operation/formal and a typed primitive predicate.
   The result carries the verdict, model revision, proof ids and source spans, plus explicit
   `unknown`, `truncated`, work count and cursor fields. Missing proof is `unknown` even if raw
   BDD atoms happen to be compatible. Row, node, pair-work and depth bounds are checked before
   allocation/traversal. Search and direct lookup continue using their materialized routes.

**Design-phase verification policy (operator direction, 2026-09-24):** use focused compile and
counterexample probes while these contracts are settled. Do not run `just test-all` or the full
`just pilot` after each slice. At the end of the integrated Stage 3 implementation, run
`just fmt`, `just test-all`, the fresh-store `just pilot` and Stage 3 structured evaluation,
then the clean wheel/generation-pinned FastMCP-native query and increment-end review. Release
profile Rust artifacts are cached across ordinary test invocations; a test's data store may be
fresh, reused or empty according to that test's purpose. The opt-in build-measurement campaign
continues to measure cold compilation and is not an acceptance-test recipe.

### Stage 4: the capability registry (unchanged in scope; the superseded plan's §15 applies)

1. **The registry** in `cpg-schema` (TOML, `deny_unknown_fields`, compiled to Arrow): append-only
   concept ids, labels with their sources, `broader`/`related`, scope notes, facets. Definitions
   are a Rust enum AST compiled to DataFusion SQL, digested. **Definitions over conditions use the
   kernel's compatibility and implication.**
2. **Integrity:** `broader` is acyclic (DataFusion `WITH RECURSIVE` with `UNION`); `related` is
   disjoint from the `broader` closure. There is no per-language rule.
3. **The vocabulary**, seeded by a one-off script whose output the operator reviews. Its
   candidates: Stack Overflow tag synonyms, Wikidata and EDAM `closeMatch`, method stereotypes.
   20–40 concepts are authored.
4. **Membership:** `concept_members` materialized, with role bindings, condition, verdict and
   witness. Every member cites a definition digest and a witness; FCA never writes membership.
5. **`lookup_concepts`** lists every concept while the catalog is small. Ranked lookup (bm25s,
   PyStemmer, vectors, RRF) waits until the catalog outgrows one page.
6. **`explain`:** a witness chain of rule id, premises and spans. Each derived row stores its rule
   id and proof height.
7. **FCA over behavioral attributes** per structural scope, as candidate facets. RCA only if the
   structured evaluation shows value.
8. **Query schema authority** (schemars → JSON Schema → pydantic), only if Rust needs the request
   types.

**Exit:** the structured evaluation of concept queries, with targets pre-registered before this
stage's output is read.

**Closes:** the superseded plan's F7 and F12.

### Stage 5: frameworks, protocols, lifecycle

1. **Framework models:** registries dispatched by name; middleware `call_next` chains; lifespan;
   ContextVar places and state (moved here by the superseded plan's §15); function-object metadata
   such as `__fastmcp__`.
2. **Deferred execution** (E19): a coroutine or generator's creation is distinct from its
   execution, suspension and completion. The read phase distinguishes "when called" from "when it
   runs", because Q07, Q08 and Q11 need it.
3. **Protocol operations** (E18), on their trigger: operators, truthiness and formatting lowered
   to semantic operations with possible implicit calls, where an evaluation item turns on a dunder
   method.
4. **Protocols as partial orders** mined from official usage by generalizing Pass C. They are
   labelled *observed pattern*.
5. **Rules parameterized by user code** (Q02, Q11): conditional records keyed on predicates about
   the user's annotations, coroutines and generators.
6. **Graph-FCA offline** on a bounded projection, judged by the structured evaluation; it never
   enters the pipeline.

**Exit:** Q02, Q06, Q07, Q08, Q11 and Q12 answered under Stage 5's exit rule.

**Then, to close increment 5:**
- unseal `eval/heldout/` and verify its manifest;
- write each task's target return before running anything;
- run the structured packet on the final generation;
- assess it in the same rubric;
- decide on the §B11 LLM trigger (its ADR stays `proposed` until then);
- a `deep` review.

## 6. Evaluation

| Stage | Pre-registered questions | Exit rule |
|---|---|---|
| 2.9 (re-applied) | Q04, Q10, Q21–Q24 | Stage 2's, in `eval/behavior/fastmcp-4.0.5.toml` |
| 3 | Q01 (`tool` options), Q03 (`tasks=`, strict validation), Q05 (error masking), Q09 (transport option conflicts) | Stage 3's |
| 4 | Concept queries, written before Stage 4's output is read | Added then |
| 5 | Q02, Q06, Q07, Q08, Q11, Q12 | Stage 5's |
| End of increment 5 | `eval/heldout/` | The same rubric |

**The evaluation and the validation lane answer different questions.**
- The structured evaluation asks whether answers are useful and correct on real questions.
- The lane (L9) asks whether the analysis admits what really executes.

## 7. Tooling by stage

| Stage | Adopt | Spike, with its exit test | Defer (trigger) | Avoid |
|---|---|---|---|---|
| 2.9 | Hypothesis 6.168.x (Python dev); `sys.monitoring` (stdlib, 3.14) | — | CPython bytecode CFG, via `dis` or `bytecode` 0.19 (a region defect runtime inputs cannot exercise) | A ty patch for runtime-aware indexing, while resolution suffices (E6) |
| 3 | biodivine-lib-bdd 0.6.3 (after its spike); PyO3 (pinned in-process semantic executor); CrossHair 0.0.110 (validation lane); pyre-check 0.10.0 / Pysa (oracle; Pyrefly binary pinned); heck, aho-corasick, regex, strsim for identifier splitting | ascent 0.8.1 (on its trigger) | OxiDD 0.12 (biodivine measured too slow or too large); z3 0.21 on the system libz3 (a query the kernel plus typed theory cannot decide); Soufflé or Nemo as rule oracles | crepe, DDlog, cozo, differential-dataflow |
| 4 | PyStemmer; DataFusion `WITH RECURSIVE` (compile time only); schemars (if needed) | — | oxttl SKOS export; FCA oracles (dev-only) | LinkML; OWL/RDF stacks; taxonomy induction; BERTopic or UMAP in the pipeline |
| 5 | — | Graph-FCA offline | Qwen3-Reranker (a §B10 ADR) | rust-bert, ort, fastembed |
| Any | — | — | Lance / LanceDB (more than ~10⁵ vectors at 4,096-d, or filtered ANN); ty type inference as a second type provider (a question Pyrefly's types cannot answer, E2); ty saturation exposure (a scope measurably degraded, E11); `mypy.stubtest` (a native-extension library, E23) | Scalpel as a component (E24) |

## 8. Verification and measurement

| Claim | Check | Expected | Stage |
|---|---|---|---|
| No feasible path is dropped by the translation | `flow_shapes` P1 cases; the runtime soundness oracle | Every executed statement admitted | 2.9 |
| The runtime view follows resolution and Python's semantics | `flow_shapes` cases; the oracle, on the pinned interpreter | Pass | 2.9 |
| Skipped rows are accounted for | Coverage counts asserted in `flow_shapes` | Counts by cause | 2.9 |
| One condition, one id, whatever its construction | Kernel known answers; shuffle determinism | Identical ids | 3.0 |
| Structure is not lost at extraction | Pilot `budget_reached` count, and the node limit's hits | Falls from the current 66; hits reported | 3.0 |
| Summaries resolve call transfers | Pilot `call_transfer` count; `behavior_shapes` part 2; the Pysa differential | Falls from 6,137; disagreements explained | 3 |
| Models agree with the functions they model | CrossHair `diffbehavior` in an isolated worker | No counterexample, or a fixture | 3.1 |
| No negative claim outside complete coverage | `semantic:refuted-needs-complete-region`, `refuted-not-overridden`, `premise-no-attribute-load` | Violations rejected | continuing |
| Membership comes only from definitions | A rule on `concept_members`; a test that FCA writes none | Pass | 4 |
| Determinism | Shuffle and relocate over every new table | Byte-identical | 2.9–5 |
| Cost | `just pilot` stage timings and peak RSS; serving p50/p95 | Reported; **no target claimed** | 2.9–5 |
| Agents get better answers | The structured packet against pre-registered targets | Rubric per item; operator review | each stage |

## 9. Findings traceability (open items only)

| Finding | Closed by | Oracle |
|---|---|---|
| X1: atoms merge distinct evaluations | 2.9.2 | P1 cases; runtime oracle |
| X2: the runtime view by spelling; version tuples | 2.9.3 | Cases; runtime oracle |
| X3: silent skips | 2.9.4 | Coverage counts; runtime oracle |
| X4: DNF-only conditions | 3.0 | Kernel known answers; pilot counts |
| X5: approximation direction | 2.9.6 (prose); 3.0 (the flag) | Prose; a flag case |
| X6: no execution oracle | 2.9.5 | The oracle itself |
| R1–R11 (Stage 2 end review) | 2.9.1 and 2.9.9 (re-review) | Its §12 |
| Superseded plan F4 (models), F8 (data files) | 3.1 | `behavior_shapes` part 2; digest tests |
| Superseded plan F7, F12 | 4 | Membership rule; FCA test |
| Superseded plan F10 (exit criteria) | This plan's staging | Stage exit rules |

## 10. Risks

| Risk | Mitigation |
|---|---|
| **Per-site atoms lose simplification** (fewer contradictions found, longer rendered conditions) | Sound by construction; the typed theory (3.0) restores sharing for primitive places; rendered-condition sizes measured on the pilot |
| **BDD blow-up** on unstructured conditions (a random 32×8 over 40 atoms did not finish in 10 minutes in the probe) | A node limit per operation (`binary_op_with_limit`), widening to `unknown`; a deterministic order; the spike measures pilot conditions, which are structured |
| **The oracle executes code** | Generated programs only, outside `fixtures/python/`; an isolated worker; no network; timeouts. The library under analysis is never executed by the oracle |
| **The typed theory depends on Pyrefly's types and value stability** | A builtin immutable scalar at the use is necessary, and cross-site sharing also needs an effect-stability witness. An unknown type or intervening effect keeps sites independent |
| ty churn and salsa skew | Exact pins; `--precise` locks; parity tests on upgrade |
| Summaries lose precision or fail to terminate | A finite domain with widening; `unknown` rates reported per stage |
| Scope creep | Exit criteria per stage; §12's deferred list; nothing starts before its stage |
| Disk and GPU | One pilot store; moved-aside stores kept only for the operator's review; live legs only with ≥30 GB free, with vLLM stopped afterwards |

## 11. Operator decisions and reviews

**New decisions, approved by the operator on 2026-09-24** as recommended (the assessment's §11):

| # | Decision | Approved |
|---|---|---|
| D-10 | Evaluation identity for condition atoms | Adopt §4.1, in ADR-0022 before acceptance |
| D-11 | A decision-diagram condition kernel, amending §B10's wording | Adopt biodivine-lib-bdd 0.6.3 after Stage 3.0's spike; theory solvers stay excluded |
| D-12 | The validation lane executes generated programs | Allowed: isolated, generated inputs only, never the analyzed library, never `fixtures/python/` |
| D-13 | A typed theory from Pyrefly's types | Adopt for builtin immutable scalars only |

**Outstanding reviews for the operator:**
- the Stage 1 and Stage 2 structured evaluations;
- deviations B1–B22;
- the moved-aside stores (`build/store-pre-*`, about 11 GB) for deletion;
- this plan.

## 12. Deferred, each with a trigger

| Item | Trigger |
|---|---|
| Points-to sets, memory versions, strong and weak updates (E17) | A Stage 3 summary whose field effect needs aliasing |
| Per-claim assumption records (X5) | A served claim misread because of an assumption the flag does not show |
| A per-iteration unrolled loop model | An evaluation item graded `partial` for a loop condition |
| Pyrefly-typed receivers for dynamic access; per-run largest-scope and residue reports (R10) | A refutation on a field read through a typed non-`self` receiver, or a report consumer |
| The strongest-transfer filter; decorators and lambdas as value sources (Stage 2 end review O3, O4) | Stage 3's summaries touch transfers |
| An unmapped argument at a multi-callee site (O6); a lambda read's phase (O1); inherited methods counted as field reads (O2) | A pilot or fixture row appears |
| z3 (E15), OxiDD (E13), ty type inference (E2), ty patches (E6, E11), the bytecode CFG (E20) | See §7 |
| Graph-FCA in the pipeline | An offline experiment yields templates the structured evaluation rates useful |
| On-demand RCA at serve time | Materialized membership proves too coarse |
| Lance / LanceDB | More than ~10⁵ vectors at 4,096-d, or filtered ANN with managed FTS |
| spaCy in compile | Regex directive tagging misses conditions the evaluation needs |
| A neural reranker | An ADR under §B10 after a measured need |
| Native-extension bodies | A pilot question needs one |

**Carried from the holistic plan** (unchanged):
- Phase 4's deletion exit stays paused, because FCA, RCA, communities and kNN get consumers in
  Stage 4.
- Phase 5 B3/B6 (the Stage F builders) stays paused.
- ADR-0020 stays `proposed`.

## 13. Standing conventions

- Small commits to `main`, each naming its stage and ADR and stating the focused check outcome;
  mark the full integrated gate `not_run` until the Stage 3 end.
  Never push, force-push or `reset --hard`.
- Snapshot changes: read the `.snap.new`, then `cargo insta accept`; a schema snapshot change is a
  declared migration. Never run `cargo insta review`.
- Codebooks are append-only (`verdict`, `boundary_reason`, `condition_atom`, the effect kinds,
  concept ids, model ids).
- Report every check as `passed`, `failed`, `blocked` (naming the prerequisite) or `not_run`,
  with its command. A mocked provider is never a pass. Label every design claim with a principles §D
  label, and date it.
- The gold is never a compiler input. `eval/heldout/` stays sealed until increment 5's end. No
  API agents evaluate content.
- `fixtures/python/` is never executed; the validation lane uses its own generated programs.
- Licence is never a criterion. Extra dependency families are allowed when declared and isolated.
- `analytics.toml` is frozen; an edit needs an amendment to ADR-0021.
- Live GPU legs only with ≥30 GB free, and vLLM is stopped afterwards
  (`pkill -f "[v]llm serve Qwen"`).
- Judgment calls go to the deviation log (`deviations_behavioral-model_2026-09-24.md`), from B23
  onward.
