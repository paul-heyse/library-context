# Plan: the behavioral model, going forward

**Status:** Active; the sole product execution plan. **Revised:** 2026-09-25 (documentation
migration: the reasoning reviews reconciled and earlier plans internalized).
**Authority.** The [architecture map](../design/README.md), DESIGN and its section owners own
contracts and accepted targets; current ADRs own rationale. This plan owns execution order, the
current disposition of scheduled findings (§6) and deferred triggers. A change to a §B decision
lands its ADR and a design/target review first (binding under ADR-0040).

## 1. Current state and qualification boundary

Production checkpoint `acbcee5` on `main` (no `crates/` or `python/` change since). Increment 4
(Stages 2.9–3) is in progress. Stages 0–2.9 are built: the whole-surface operation catalog, the
`flow` family from ty, the condition language, five verdicts and Stage 2's behaviors. **Stage 3 is
functionally incomplete and integrated qualification is outstanding.**

| Scope | State (2026-09-25) | Remaining boundary |
|---|---|---|
| L1/L1.5 conditions | **Implemented; focused tests passed.** Bounded BDD catalogs, exact entry-value links, narrow primitive refutation, per-path native work accounting | Operation-wide compatibility; decision/support/aggregate budget defects (W6); membership and integer equality (W11) |
| L4 pinned models | **Implemented; focused tests passed.** Typed transfer/effect/callback/resource/exception assertions and exact source applications. `typing.cast`/`typing.assert_type` assert total normal return. A Pydantic `TypeAdapter.validate_python` potential transform is dormant | Pure/helper, I/O, async/context, pydantic and HTTP/server families; a dynamic-schema representation before any validation effect |
| L2 fates | **Implemented and Tested in focused cases.** Handler clauses, pinned class/MRO candidates, direct handler `return None`, modeled-exception paths, ordered literal `finally: pass` chains | Effectful finalizers, context-manager exits, handler propagation, callback fate and resource pairing remain unknown |
| L3 summaries | **Implemented; focused tests passed.** Direct, exact modeled, unique-assignment and unconditional acyclic wrapper value paths with canonical ordered proofs; petgraph SCC schedule; recursive members withheld; an earlier call is ignored only when its region is BDD-incompatible with the return | Normal-completion witnesses for compatible predecessors, recursive composition, effect/exception/role summaries, path-specific boundary identity, `call_transfer` discharge |
| L7 serving | **Partially implemented.** FORMAT 7 structural condition/summary data; native `inspect_value_paths` gives path-local inspection only | Typed operation-wide compatibility/effect/role filters, full proof spans, verdict/citation fidelity (W2, W3), clean-wheel query |
| Integrated acceptance | **Not established.** A 2026-09-25 `just test-all` attempt passed 313/313 release Rust tests and 110/112 Python tests (two stale tool-list expectations, since corrected: `test_server.py` 12/12 passed). The rerun was stopped at operator direction; the operator's expectation of a pass is an assumption, not an outcome | `just test-all`, fresh `just pilot`, Q01/Q03/Q05/Q09, structured evaluation, clean-wheel query and the increment-end review are `not_run` |

**Accepted schema migrations at this checkpoint:** `value_flow_contributions` keys include the
local/upstream transfer flags (extractor output version 74); multi-release validation selects one
distinct snapshot; MRO validation compares distinct ancestor facts with provenance.

**Next dependency:** W1 (ARC-01). Derive native proof-kind admission from the schema codebook and
verify a real finalizer-bearing generation through the native executor. Then W4 (access
normalization) and W5 (explicit summary inputs and typed refusals, ARC-02/03) before any new SCC
or limit policy.

**Measurement baseline** (pilot snapshot `162bda5a0fe39cc1cedadbfaa8efd1f9`, 2026-09-24, fake
embedder; Measured): 1,534 public declarations; 15,415 behaviors (4,276 established, 3,554
conditional, 19 refuted); unknown by reason: **6,137 `call_transfer`**, 1,347 `override_dispatch`,
42 `budget_reached`, 39 `abstract_body`, 1 `runtime_unreachable`; pilot compile 40.4 s. Stage 3
reports its exit against these counts. No reduction is claimed before that run.

## 2. Product target and principles

The product (ADR-0021) is an **evidence-carrying behavioral model of a pinned library's whole
public surface**. An agent asks which operations accept X, pass it to Y, under which
configuration, raising what, needing which lifecycle, and receives exhaustive answers where the
analysis is complete, ranked candidates where only discovery applies, and a named unknown where the
analysis stopped. Each claim carries evidence and a derivation; briefs remain one rendering.
[§3.9](../design/sections/behavior-model.md) owns conditions, places and verdicts;
[§9.9](../design/sections/behavioral-analysis.md) owns models, summaries and the registry.

| # | Principle | Without it |
|---|---|---|
| T1 | The whole public surface is the universe; analysis runs per callable, bottom-up | Silence about most operations |
| T2 | Relations first, sentences last: typed, persisted relations with in-row provenance | Knowledge locked in templates |
| T3 | The flow IR is our declared runtime model (§B5), whichever provider builds it | Checker views relabelled as runtime flow |
| T4 | Meaning comes from models, propagation from summaries; never "calls X, so can X" | Unsupported transitive claims |
| T5 | Conditions are Boolean functions over evaluation atoms in a bounded decision-diagram kernel, plus a typed theory for primitive places; no theory solver (§B10) | Merged evaluations drop feasible paths; budget cuts lose structure |
| T6 | Five verdicts, never a null; a negative verdict only inside complete coverage | Unsupported negatives |
| T7 | Materialize source facts at compile time; run bounded semantic selections in the pinned Rust executor | A second, unproven serve-time semantics |
| T8 | Discovery nominates, definitions decide; FCA, RCA, communities and embeddings never write membership | Statistical membership |
| T9 | Pre-registered behavioral question sets, plus mechanical known-answer fixtures | Circular evaluation |
| T10 | Providers observe; our semantic layer concludes under stated, tested rules | Provider artefacts become claims |
| T11 | Execution checks admission: within the stated model every observed execution is admitted; a violation is a counterexample; a finite pass is evidence, not proof | Translation defects stay invisible |

Kept unchanged: programmatic synthesis with no generative model in the pipeline or query path
(§B11); immutable, byte-identical generations (§B7, §B12); declared dependency families
(ADR-0002); the gold under `.claude/skills/` is never a compiler input; licence is never a
criterion.

| Layer | What | State |
|---|---|---|
| L0 | The CPG | Built |
| L1 | Flow facts (`flow` family, ty through `cpg-flow`) | Built |
| L1.5 | The condition kernel (bounded BDDs, typed primitive theory) | Built in part |
| L2 | Behavior relations, including handlers, callbacks, resources and exits | Built in part |
| L3 | Transfer summaries resolving `call_transfer` | Built in part |
| L4 | Models catalog | Built in part |
| L5 | Capability registry and membership | Stage 4 |
| L6 | Discovery (FCA, RCA, communities, views) | Built; consumers in Stage 4 |
| L7 | Serving (FastMCP tools, native executor) | Built in part |
| L8 | Structured evaluation | Built; stage exit rules continue |
| L9 | Validation lane (runtime soundness oracle, CrossHair, Pysa) | Built in part; order 8 extends it |

## 3. Stage 3 execution queue

Every intermediate outcome is a cited positive candidate or `unknown`. No absence becomes
`refuted_under_model` before the model channels and relevant source paths are closed. A slice
uses focused compilation, one positive and one withholding fixture, its schema/rule snapshots and
the shared publication validator; the last column names a targeted check, not integrated
acceptance. Findings are identified in §6; their priority follows the follow-up review's §12.

### 3.1 Contract repairs before extension

| Order | Work (§6 item) | Targeted acceptance |
|---|---|---|
| A1 | Native proof-kind admission from the schema contract; one owned typed decoder for generation tables (W1: ARC-01, RF/F13) | A finalizer-bearing generation loads and queries through the real native executor; an unknown kind is rejected; a schema drift test between generation and native reader |
| A2 | One resolved attribute-access contract (W4: RFU/F05, RF/F09) | Bare, qualified and aliased builtin access, computed names and shadowed builtins under the real provider; qualified/aliased literal reads withhold no-read claims; one corrected premise traced through publication and serving |
| A3 | Served claim fidelity (W2: RFU/F02; W3: RFU/F03) | A facet with established and unknown values keeps both in lookup and agrees with filtering; a finding-only claim is followed to its model and source span in the served generation, identically in structured and Markdown forms |
| A4 | Explicit summary inputs and typed refusals (W5: ARC-02, ARC-03) with the next normal-completion witness | The production transformation runs without extraction, embedding, Delta or server setup; an actual depth/BDD cap and an unsupported case keep specific causes through producer, snapshot and native response |
| A5 | `Model::reach` fixed point with a work bound (W7: RF/F05) | The query-order reproduction gives the same contributions A-first and B-first; the acyclic and finite-base controls hold; a dense SCC hits its cap with a boundary; shuffled input is byte-identical |
| A6 | Kernel decisions, effective support and aggregate preparation bounds (W6: RF/F01, RF/F02, RFU/F01) | The tests named in W6; a bounded refusal stays unknown |
| A7 | Analyzer input identity and canonical cache fill (W8: RFU/F07; W9: RFU/F06) | The helper edit/addition and cache race/over-cap controls named in W8 and W9 |

A1–A5 precede further proof-kind, native or SCC extension. A6 precedes raising catalog limits or
extending summary catalogs. A7 precedes a hermetic corpus/run-identity or cache-conformance claim;
it is independent of A1–A6.

### 3.2 Functional completion

| Order | Build and proof boundary | Targeted acceptance |
|---|---|---|
| 1 | **Call-site and predecessor evaluation.** For each argument and preceding statement on a proposed return path, distinguish an evaluated direct value, a cited normal outcome, a possible raise and an unresolved expression, tied to Ruff syntax, ty use/region facts and the call/return sites. Preserve evaluation order; a modeled target's normal return starts after its arguments. A true return region does not prove that an earlier call returned | A direct formal and a raising sibling argument diverge; a terminating branch before recursion and an unconditional self-call before return diverge; the shared validator rejects a forged completion row |
| 2 | **Exit and L2 fate.** Nested `try`/`finally` and `with` frame order, normal/exceptional completion and suppression; callback stored/invoked/forwarded/registered and resource acquire/release only with an execution and exit witness. Generators/coroutines stay at the Stage 5 deferred-execution boundary. ty's `end_of_scope_reachability` may add one append-only implicit-exit kind | Nesting, early-return, re-raise, suppressor, callback-not-run and unreleased-resource cases; lexical containment never substitutes for execution |
| 3 | **Proof identity for composition.** The canonical ordered proof identity (ADR-0045) extends to the call/model/flow/exit steps order 6 composes, retaining parallel paths without duplicating prefixes | Two calls sharing endpoints, reordered inputs, a forged or missing step |
| 4 | **Positive modeled results.** Admit an exact whole-return identity path only when the target set is closed and sole, modalities are definite, all arguments reach the call, the pinned target asserts normal return, the condition is admitted and the return exit is proven. An assignment predecessor follows only when its reaching definition is unambiguous and condition-compatible; otherwise a specific `summary_boundaries` reason | Direct, shadowed, nested-call, fallback, alternate-target, raising-argument and assignment cases; a forged summary is rejected |
| 5 | **Model families** (§3.3). Record each target's exact pin, formal mapping, independent channel coverage, invocation phase and a normal-return assertion only when source-backed. A family without an in-scope consumer stays a candidate model | Per-family positive/withholding fixtures; model-catalog equality; CrossHair for pure models in an isolated worker (a timeout or unexplored path is inconclusive) |
| 6 | **Finite interprocedural summaries.** Replace the recursive-member refusal with bounded SCC composition that admits a cited finite base path; compose value/transform/effect/exception/role paths with BDD conjunction and declared modality. Depth, node, pair-work and iteration caps write explicit boundaries; an open override blocks negative closure. The engine is chosen by the W12 comparison; SCC routine and ownership by W13 | Acyclic, terminating base, self- and mutual recursion, parallel edge, open override and cap fixtures; shuffled input byte-identical; a cap's specific cause reaches `summary_boundaries` |
| 7 | **Claim discharge.** Reconstruct `summary_flows`, `summary_effects` and `summary_boundaries` through shared validators; `call_transfer` becomes established/conditional only with the matching proof; a refutation requires complete source/model/handler coverage | `behavior_shapes` part 2: logging-only and disabled-option wrappers; unrelated or ambiguous candidates stay unknown |
| 8 | **Independent challenge** (W11's independent controls). CrossHair `diffbehavior` for pure models; Pysa TITO with a real source→sink rule and the pinned Pyrefly binary; CPython 3.14 `sys.monitoring` with Hypothesis for flow, exception and exit admission, including an ignored predecessor followed by the cited return. Disagreement becomes a fixture or an explicit boundary | Targeted oracle cases while implementing; the preregistered full comparison at the integrated end. Oracles never write facts; the gold never enters analysis |
| 9 | **FORMAT 7 native serving.** Validated structural condition/proof/summary tables, one immutable PyO3 executor per process, typed compatibility plus effect/role filters. Resolve the operation/formal and exact primitive origin before BDD evaluation; string membership and integer equality (W11) are needed for Q09. Unsupported proof, caps and cursor exhaustion return explicit unknown/truncated with work accounting | Editable-import and native query cases, then one clean-wheel generation-pinned query after functionality lands |
| 10 | **Integrated exit.** `just fmt`, `just test-all`, fresh-store `just pilot`, Q01/Q03/Q05/Q09 and the structured Stage 3 evaluation, the clean-wheel query and the increment-end design/target review | Each command `passed`/`failed`/`blocked`/`not_run`; `call_transfer` before/after, condition budget counts, compile time/RSS and serving bounds. A material fix repeats only the affected gate plus invalidated acceptance |

**Stage 3 exit:** `behavior_shapes` part 2 passes and Stage 3's pre-registered exit rule (Q01, Q03,
Q05, Q09) passes; then increment 4's assembled design/target review.

**Carry-forward constraints.**
- Keep exact source argument/target counts and the shared reconstruction validator for proofs.
  No generic `return x` positive after an unproved call.
- The ordered nested pass-finalizer proof does not generalize: effectful finalizers and `with`
  stay unknown until frame actions and suppression are cited.
- A Pydantic `TypeAdapter` selects its schema at runtime. Do not populate the named-schema
  `validate(schema)` action with the adapter class name; a typed dynamic-schema case, with a source
  witness where one can be proved, precedes any validation-effect or negative-coverage claim.
- The BDD-incompatibility screen for earlier calls is interim; order 1's condition-compatible
  predecessor witness replaces it. Missing, approximate or capped evidence stays an explicit unknown.
- `inspect_value_paths` is path-local inspection, never an operation-wide verdict.
- Accepted snapshots are reviewed migrations; the post-fix complete gate and fresh pilot are
  outstanding.

### 3.3 Models catalog v1

Authored from pinned library source and official docs, append-only model ids, typed data with the
catalog digest in `compiler_digest`; no model cites `.claude/skills/`.

| Family | Targets |
|---|---|
| Pure identity/value | `typing.cast`, `typing.assert_type` (built); `str`, `dict`, `list`, `tuple`; `functools.partial`, `functools.wraps` |
| I/O and serialization | `open`/`io`/`pathlib`, `json`, `gzip`/`zlib`, logging |
| Async, timeouts, context | asyncio and anyio (`fail_after`, `move_on_after`, `to_thread`, task groups); `contextvars`; `contextlib` |
| Validation and settings | pydantic `BaseModel`, `Field`, `TypeAdapter.validate_python` (dynamic schema, above); pydantic-settings `BaseSettings` (environment prefix) |
| HTTP and servers | httpx timeouts; the starlette and uvicorn entry points |

## 4. Stages 4 and 5

### Stage 4: the capability registry

1. **Registry** in `cpg-schema` (TOML with `deny_unknown_fields`, compiled to Arrow): append-only
   concept ids, labels with their sources, `broader`/`related`, scope notes, facets. Definitions
   are a Rust enum AST compiled to DataFusion SQL and digested; definitions over conditions use
   the kernel's compatibility and implication.
2. **Integrity:** `broader` is acyclic (DataFusion `WITH RECURSIVE` with `UNION`); `related` is
   disjoint from the `broader` closure. No per-language label rule.
3. **Vocabulary** seeded by a one-off script whose output the operator reviews: Stack Overflow tag
   synonyms (candidates only; they merge opposites), Wikidata and EDAM `closeMatch`, method
   stereotypes. Author 20–40 concepts.
4. **Membership:** `concept_members` materialized with role bindings, condition, verdict and
   witness; every member cites a definition digest and a witness; a test shows FCA never writes
   membership.
5. **`lookup_concepts`** lists every concept while the catalog is small; ranked lookup (bm25s,
   PyStemmer, vectors, RRF) waits until it outgrows one page.
6. **`explain`** returns the stored witness chain: rule id, premises and spans; each derived row
   stores its rule id and proof height.
7. **FCA over behavioral attributes** per structural scope, as candidate facets. RCA only if the
   structured evaluation shows value, and only after W10 gives attributes typed meaning.
8. **Query schema authority** (schemars → JSON Schema → pydantic) only if Rust needs the request types.

**Exit:** the structured evaluation of concept queries, with targets pre-registered in
`eval/behavior/` before this stage's output is read.

### Stage 5: frameworks, protocols, lifecycle

1. **Framework models:** registries dispatched by name; middleware `call_next` chains; lifespan;
   ContextVar places and state; function-object metadata such as `__fastmcp__`.
2. **Deferred execution:** a coroutine or generator's creation is distinct from its execution,
   suspension and completion; the read phase distinguishes "when called" from "when it runs"
   (Q07, Q08, Q11).
3. **Protocol operations**, on their trigger: operators, truthiness and formatting lowered to
   semantic operations with possible implicit calls, when an evaluation item turns on a dunder.
4. **Protocols as partial orders** mined from official usage by generalizing Pass C, labelled
   *observed pattern*, never *enforced protocol*.
5. **Rules parameterized by user code** (Q02, Q11): conditional records keyed on predicates about
   the user's annotations, coroutines and generators.
6. **Graph-FCA offline** on a bounded projection, judged by the structured evaluation; never in
   the pipeline.

**Exit:** Q02, Q06, Q07, Q08, Q11 and Q12 under Stage 5's exit rule. **Then, to close
increment 5:** unseal `eval/heldout/` and verify its manifest; write each task's target before
running anything; run the structured packet on the final generation and assess it in the same
rubric; decide the §B11 generative-model trigger; an assembled design/target review.

## 5. Evaluation and tooling

| Stage | Pre-registered questions (`eval/behavior/fastmcp-4.0.5.toml`) | Exit rule |
|---|---|---|
| 3 | Q01 (`tool` options), Q03 (`tasks=`, strict validation), Q05 (error masking), Q09 (transport option conflicts) | No positive item incorrect or misleading; no negative claimed; at least half the positives present or partial |
| 4 | Concept queries, written before Stage 4's output is read | Added then |
| 5 | Q02, Q06, Q07, Q08, Q11, Q12 | As Stage 3 |
| End of increment 5 | `eval/heldout/` | The same rubric |

The packet is `just structured-eval <generation> <stage> [embed_url]`; the author assesses it with
the rubric present/partial/absent/incorrect/misleading, per item, never as a percentage; no API
agents evaluate content. The structured evaluation asks whether answers are useful and correct;
the validation lane asks whether the analysis admits what really executes.

| Library | Decision | Trigger or qualification |
|---|---|---|
| biodivine-lib-bdd 0.6.3 | **Expand** (W6, W11): bounded decisions, effective-support normalization, cube restriction | Keep structural ids and one union vocabulary; `check_binary_op`'s `None` is unknown |
| Ascent 0.8.1, datafrog 2.0.1, a small SCC worklist | **Compare** at order 6 after W5 (W12); none is adopted | Same semantic state and refusal contract for all three; timeouts are not deterministic budgets; measure compile/run cost after parity |
| OxiDD 0.12 | **Deferred migration** | A measured production case (pilot evidence that biodivine is too slow or large), compared on capacity, GC, retained memory and stable-id adapters |
| z3 0.21.1 on Z3 5.1.0 | **Deferred** | A registered query whose correctly linked theory constraints are impractical or inexpressible in the bounded finite lowering; it would sit behind `primitive_theory`, never become the kernel |
| fcars 0.2.2 | **Dev-only oracle** | Add a sparse context above 64 attributes if bitset coverage grows; concept-set agreement does not test attribute fidelity (W10) |
| rustworkx-core | **Keep bespoke** keyed ordering (W13) | A consumer for centralities beyond the current ones |
| Pysa, CrossHair, Hypothesis + `sys.monitoring` | **Oracles** (order 8) | Pysa's call graph is Pyrefly's, so only its TITO result is independent |
| PyStemmer, DataFusion `WITH RECURSIVE` (compile time), schemars | Stage 4 adopt | schemars only if Rust needs request types |
| Graph-FCA | Stage 5 offline spike | Never in the pipeline |

Avoid: crepe, DDlog, cozo, differential-dataflow; LinkML, OWL/RDF stacks, taxonomy induction,
BERTopic or UMAP in the pipeline; rust-bert, ort, fastembed.

## 6. Findings disposition

This table is the **single current disposition owner** for the findings below. Source reviews
keep their dated evidence and link here; STATUS links here. Qualified ids name the source:
**ARC** the retired 2026-09-25 core-3.0 calibration reviews (`design_review_v3_stage3_architecture_calibration_2026-09-25.md`
F01–F03 → ARC-01–03; `design_review_v3_nested_finalizers_calibration_2026-09-25.md` F01 → ARC-01;
`git show f54b09d:docs/design_review/reviews/<file>`); **RF** the
[reasoning review](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md);
**RFU** its [follow-up](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md),
whose §7.1 qualifications govern where the two disagree. Evidence is Interface-checked or Tested
as stated in the source; corrections are **Proposed**. A deferred item keeps its trigger; closure
needs the named evidence, never an accepted ADR or a moved row.

| Item | Component | Consequence and intended correction | Disposition · dependency | Closure check |
|---|---|---|---|---|
| <a id="ARC-01"></a>**W1 · ARC-01**, [RF/F13](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F13) | `cpg-schema` (`SummaryFlowStepKind`), `lctx_semantics` loader | The native loader re-whitelists proof kinds and decodes positional string tuples, so the compiler's finalizer proof kind is rejected and every new kind or column edits both sides. Derive admission from the schema contract and centralize one typed projection decoder; choose transport (Arrow IPC or other) by total conversion cost. IPC bytes alone do not make decoders agree | **Open** · A1; precedes further proof-kind or native acceptance | A1's acceptance; an unknown-kind rejection control; a generation/reader schema drift test |
| **W2 · [RFU/F02](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F02)** | Python operation response (`operations.py`) | Lookup discards each facet value's verdict, so an `unknown` value reads like an established one. Return typed `{value, verdict}` entries, keeping separate completeness | **Open** · A3; before treating lookup facets as behavioral facts | A mixed established/unknown facet agrees between lookup and filtering; the generated response schema shows the verdict |
| **W3 · [RFU/F03](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F03)** | Bundle support projection, Python rendering | Finding-backed claims lose their evidence/model closure; Markdown drops the citation. Export a bounded canonical support closure and render the same support edges everywhere; at minimum keep support ids and report unavailable resolution | **Open** · A3; before a serving evidence-closure acceptance claim; ADR/DESIGN if the projection contract changes | A finding-only coordinates claim resolves to its model and source span; structured and resource forms agree; missing support is rejected or unknown |
| **W4 · [RFU/F05](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F05)**, [RF/F09](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F09) | `cpg-flow` / `cpg-core::flow_model` access boundary | Qualified and aliased builtin `getattr` produce no read or dynamic boundary and a true no-read premise (Tested); a quote-prefix test also treats computed names as literals. One resolved access contract with literal/dynamic status, receiver provenance and unsupported outcome; a literal-node fix alone does not close the defect | **Open** · A2; before relying on field/global no-read claims | Real-provider controls for all three spellings, computed names and shadowed builtins; a genuinely unread field stays distinguishable; one premise traced through publication and serving |
| <a id="ARC-02"></a>**W5 · ARC-02** | `lctx-analytics::summaries`, `cpg-core` | Summary policy acquires its own session, so it cannot be tested or composed without acquisition. `cpg-core` prepares relations; the analytics owner exposes finite composition over explicit inputs and outcomes, without a new crate or provider framework | **Open** · A4; with the next normal-completion witness, before SCC rules | An admitted case and an independent withholding control run the production transformation with no extraction, embedding, Delta or server |
| <a id="ARC-03"></a>**W5 · ARC-03** | Summary producer, boundary publication | Refusal causes are discarded and later inferred generically from an absent flow. The producer owns typed admitted/refused outcomes and publication preserves the cause; coordinate with path-specific boundary identity; ADR/DESIGN if persisted meaning changes | **Open** · A4; before new SCC budgets | An actual depth/BDD cap and a distinct unsupported case keep specific causes through producer, snapshot and native response; neither becomes a negative; any codebook change is append-only |
| **W6 · [RF/F01](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F01), [RF/F02](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F02), [RFU/F01](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F01)** | `cpg-schema::condition_kernel`, native generation preparation | F01: decisions build the full result under a node cap, so decidable compatibility/implication answer `NodeLimit` (Tested on actual source). F02: declared support never shrinks, so admission depends on construction history (Tested). RFU/F01: catalog caps count shared storage while hydration retains per-root expansions (256 roots → 17,152 owned nodes, Tested). Decide with bounded non-allocating checks while keeping transfer/support admission; normalize effective support; add an aggregate preparation/retention budget with deterministic refusal | **Open** · A6; before raising catalog limits or extending summary catalogs | F01: an admitted pair over 50k result nodes decides; an over-preflight contradiction decides; a task-limit control stays unknown. F02: redundant composition stays under the limit and live support equals hydrated support. RFU/F01: shared-tail and disjoint controls refuse before large allocation; native diagnostics distinguish stored from retained counts |
| **W7 · [RF/F05](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F05)** | `cpg-core::flow_model` | `Model::reach` is not a fixed point at a cycle head: a query-order reproduction loses a transfer variant, with no work bound (Tested). Missing contributions affect seed and boundary coverage; the review's broader negative-premise consequence was not established. SCC fixed point with a work cap writing `BudgetReached` | **Open** · A5; before engine selection | A5's acceptance |
| **W8 · [RFU/F07](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F07)** | `cpg-extract` acquisition/context | The corpus run's import root is outside context identity: an unselected helper imported by an example can change facts without changing any identity. Ordinary site-packages are already hashed; do not generalize. Include every analyzer-readable root's content and membership, or enforce an immutable verified tree; ADR if acquisition policy changes | **Open** · A7; before a hermetic corpus/run-identity claim | Editing and adding an unselected helper changes identity or is refused before extraction; relocation invariance holds; facts equal a clean recomputation |
| **W9 · [RFU/F06](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F06)** | `cpg-core::embed` | Two cache-fill entry points apply different admission, and one returns locally computed vectors instead of the committed winner. One cache-fill operation owns token admission, batching, insert-only merge and versioned readback | **Open** · A7; before embedding-cache conformance or kNN replay claims | Two attempts with distinct valid vectors return exactly the committed values; an over-cap embedder is rejected through both entry points before insertion. Fake-embedder doubles qualify cache semantics only |
| **W10 · [RFU/F04](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F04)** | Attribute schema, `lctx-analytics::concepts`, Stage F | FCA/RCA attributes are English labels; candidate calls render as "calls X". Typed attribute keys with modality and evidence, rendered after analysis; an interim fix keeps may-call meaning in one owned rendering and rejects unknown forms | **Deferred** · trigger: before claiming candidate-derived `+rca` behavior or extending FCA/RCA (Stage 4.7) | Candidate-only and definite controls keep distinct meaning in shared-signature and implication output; relabeling does not change membership |
| **W11 · [RF/F03](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F03), [RF/F04](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F04), [RF/F12](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F12)** | `condition_kernel`, `primitive_theory`, validation lane | F04: `MemberOf`/`Equals{Int}` return unknown, blocking Q09; add exact string membership and non-bool integer equality (keep `True in {1}` unknown); this alone does not complete operation-wide Q09. F03: `given` still nominates from DNF; use cube restriction as a candidate generator while keeping `factor ∧ quotient == original`; restriction is not factor division; link minimization is an optional improvement with a total work budget. F12: add truth-table kernel tests, Python-lowering controls and served round trips | **Open** · F04 at order 9; F03 and F12 with A6 and order 8 | F04: a `transport.py`-shaped fixture with the bool/int control. F03: an over-budget DNF factor now factors with the equality gate; existing theory fixtures unchanged. F12: truth tables over ≤10 atoms in kernel tests; order 8 records disagreements as fixtures |
| **W12 · [RF/F06](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F06)** | `lctx-analytics::summaries` | The engine decision was sequenced after bespoke loops. Compare Ascent, datafrog and the smallest SCC worklist at the first genuinely shared recursive contract, after W5, on one semantic state and refusal contract; no unconditional adoption | **Open** · order 6; ADR if an engine is adopted | Order 6's fixtures under the chosen engine; shuffled-input identity; a cap cause in `summary_boundaries`; measured compile-time delta |
| **W13 · [RF/F07](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F07), [RF/F08](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F08)** | `cpg-schema` recursive CTEs; `lctx-analytics::summaries` SCC | F07: type-term walks use `UNION ALL` and rely on extraction's depth cap; F08: summaries use recursive `tarjan_scc` while earlier records named `kosaraju_scc`. Resolve closure row identity/provenance and SCC ownership before substituting; no blind `UNION` or routine swap. The analytics owner records the current routine and the open stack-safety question | **Open** · order 6 | Cyclic type-term fixture terminates with unchanged identity; existing SCC and component-order tests; a recorded stack-safety decision |
| **W14 · [RF/F10](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F10), [RF/F11](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F11), [RF/F14](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F14), [RF/F15](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F15)** | `cpg-extract`, `cpg-flow`, `lctx-analytics` Pass B, `cpg-core` derivation | F10: abstract/stub status from decorator text; use Pyrefly's resolved flags (a schema migration), keeping Protocol semantics. F11: ty runs with empty program settings; construct them from the run context. F14: Pass B's suppression-sensitive property is absent from its visited key. F15: SQL-only tables round-trip through Rust rows | **Open**: F10, F11, F14 at the next edit of their owner. **Deferred**: F15 until the next edit of `attempt.rs` table construction | F10: aliased-decorator and Protocol fixtures. F11: a provider operation actually affected by settings (e.g. resolution). F14: a fixture where the clean path is found second. F15: identical snapshots; independent rederivations kept |
| **W15 · [RF/F16](../design_review/reviews/design_review_library-fit-reasoning_2026-09-25.md#F16)** | DESIGN §6.3; `cpg-core::delta` | §6.3 previously claimed merge migrations no code performs; `verify` rejects drift. [ADR-0048](../adr/0048-schema-rebuild-policy.md) chooses a fresh rebuild of the current library from pinned inputs for schema changes. Historical snapshot reads and retaining old binaries are not required; the old store may be held temporarily for rollback. Reuse the global embedding cache only after independent contract verification | **Policy decided; Interface-checked.** Strict-verification documentation mismatch closed; fresh rebuild after the `function_implementations` migration is pending integrated acceptance. Revisit only for a real historical-read consumer or unavailable pinned input | A fresh compile produces a current snapshot and `embedding_specs` with the new schema; old analysis tables are not copied; `verify` still refuses drift |
| **W16 · [RFU/F08](../design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#F08)** | Embedding spec, `just embed-serve`, Rust/Python clients | The launch recipe repeats revision/dtype from the hashed spec, and clients accept an endpoint by model name, so the running deployment is not established by spec-hash equality. Derive controlled launch arguments from the spec and declare the supported endpoint contract, or limit the claim to the operator-controlled launch; ADR/DESIGN if endpoint guarantees change | **Deferred** · trigger: the next model, tokenizer or serving upgrade, or accepting another endpoint | A revision change moves launch configuration and cache identity together; a mismatched endpoint identity is refused or explicitly unsupported |

Both reviews' tool-placement conclusions are in §5. W15's policy is decided by ADR-0048; the
fresh-store execution is still pending. No other review finding is closed by documentation cleanup.

## 7. Deferred, each with a trigger

| Item | Trigger |
|---|---|
| Points-to sets, memory versions, strong and weak updates | A Stage 3 summary whose field effect needs aliasing |
| Per-claim assumption records | A served claim misread because of an assumption the `approximated` flag does not show |
| A per-iteration unrolled loop model | An evaluation item graded `partial` for a loop condition |
| Pyrefly-typed receivers for dynamic access; per-run largest-scope and residue reports | A refutation on a field read through a typed non-`self` receiver, or a report consumer |
| The strongest-transfer filter; decorators and lambdas as value sources | Stage 3's summaries touch transfers |
| An unmapped argument at a multi-callee site; a lambda read's phase; inherited methods counted as field reads | A pilot or fixture row appears |
| ty type inference as a second type provider; ty patches; the CPython bytecode CFG (`bytecode` 0.19) | A question Pyrefly's types cannot answer; a measurably degraded scope; an effectful-frame case runtime inputs cannot exercise (order 2) |
| Analytics technique retention (ADR-0020's keep-rule outcome); deleting FCA, RCA, communities or kNN variants | Stage 4 gives them consumers; decide on that evidence |
| Stage F brief-builder restructuring | A rendering consumer needs it; briefs remain one rendering |
| Graph-FCA in the pipeline | An offline experiment yields templates the evaluation rates useful |
| On-demand RCA at serve time | Materialized membership proves too coarse |
| Lance / LanceDB | More than ~10⁵ vectors at 4,096-d, or filtered ANN with managed FTS |
| spaCy in compile | Regex directive tagging misses conditions the evaluation needs |
| A neural reranker | An ADR under §B10 after a measured need |
| Native-extension bodies | A pilot question needs one |

## 8. Risks

| Risk | Mitigation |
|---|---|
| Per-site atoms lose simplification | Sound by construction; the typed theory restores sharing for proven-stable primitive places; rendered sizes measured on the pilot |
| BDD blow-up on unstructured conditions | Per-operation node and pair-work caps widening to `unknown`; W6's aggregate preparation budget; deterministic order |
| The validation lane executes code | Generated programs only, outside `fixtures/python/`; isolated worker, no network, timeouts; the analyzed library is never executed |
| Summaries lose precision or fail to terminate | Finite domains with explicit caps and typed refusals (W5); `unknown` rates reported |
| ty churn and salsa skew | Exact pins; parity tests on upgrade |
| Scope creep | Stage exit rules; §7's triggers; nothing starts before its stage |
| Disk and GPU | One pilot store; live GPU legs only with ≥30 GB free and vLLM stopped afterwards |

## 9. Standing conventions

- Small commits to `main`, each naming its stage/item and any ADR and stating the observed check
  outcome; a preliminary gate cannot certify the Stage 3 end. Never push unasked, force-push or
  `reset --hard`.
- Judgment calls go in the commit message; a scope change updates this plan; an architectural
  change gets an ADR.
- Snapshot changes: read the `.snap.new`, then `cargo insta accept --workspace`; a schema snapshot
  change is a declared migration. Never `cargo insta review`. A schema migration needs a fresh
  store rebuilt from pinned inputs (ADR-0048). Moving `build/store` aside is a temporary rollback
  measure, not a historical-read promise. The new compile produces `embedding_specs`; reuse the
  global `embedding_cache` only if its separate contract verifies. Do not copy old analysis tables.
- Codebooks are append-only (`verdict`, `boundary_reason`, `condition_atom`, effect kinds, concept
  ids, model ids). `libraries/fastmcp/analytics.toml` is frozen; an edit needs an ADR.
- `fixtures/python/` is never executed; `eval/heldout/` stays sealed until increment 5's end.
- Pitfalls already hit: `just test-all` stops at its first failure, so rerun before reporting;
  edits made through Python or shell bypass the format hook, so run `just fmt` first; the Python
  suite runs against the `analysis_shapes` fixture generation, and the analysis diff snapshot
  counts its behaviors, so attribution changes move both; `pkill -f` can match its own command
  line (use `[v]llm`).
- Operator housekeeping: the moved-aside stores `build/store-pre-*` and `build/store-stage3-*` are
  kept until the operator deletes them.
