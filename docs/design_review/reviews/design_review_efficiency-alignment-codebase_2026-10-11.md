---
title: Efficiency alignment across the codebase
date: 2026-10-11
tier: design
purpose: target
status: review
standard: Core 3.3
heuristics: Efficient Architecture 1.0
profile: Code Intelligence 1.5
baseline: 88f33bc0dd8bc67ef9291115c3a487b5f54bcb57
---

# Efficiency alignment across the codebase

This is the principal design review for the codebase-wide efficiency-alignment question. It
covers the product and the development loop, excluding docs publication tooling. The question:
where does library-context's physical realization fail to fit its supported workload (FP-07, A4),
judged with the *Heuristics for Efficient Architecture* (H1–H28) and the independent correctness
and fidelity gates?

The review is a fresh, independent judgment. The [evidence folder](../evidence/2026-10-11_efficiency-alignment-codebase/README.md)
holds the supporting material:
- the operation map;
- eleven bounded inquiries (A–H, L1, L2);
- coordinator checks V01–V12 and reconciliation notes R1–R26;
- light probes P1–P9.

These are leads and evidence, not conclusions. The finding IDs, verdicts and rule impacts below
are this review's own. Local inquiry labels (`A-01`, `R5`, `V03`) appear only as provenance.

**Reviewer and date.** Fresh principal `design-reviewer`, 2026-10-11.

**Evidence.**
- Static review at baseline `88f33bc0`, plus the probes the coordinator ran. No build, test,
  service, fixture or SurrealDB query was run for this review.
- Files with another session's uncommitted edits were read at HEAD (`git show HEAD:<path>`).
- Every finding's decisive source was re-read at `path::symbol` grain by this reviewer. Where a
  count or a search comes from an inquiry instead, the finding says so.

---

## Decision and integrated synthesis

**Decision: Revise.**
- A4 is violated for several supported workloads.
- A1 and A2 are violated in bounded places.
- G4 and G7 fail narrowly, and G1 is unresolved.
- No correctness failure of published output was found.
- The semantic architecture is strong and largely worth keeping.
- The physical realization repeats, re-proves and serializes work that the semantics do not
  require.

### What the system is for, and what that demands physically

The functional target ([DESIGN §1.1](../../design/DESIGN.md), [§14](../../design/sections/api-and-evidence-product.md))
is a pinned API and evidence catalog. A coding agent must be able to:
- discover a built-in feature;
- select its exact invocation;
- inspect its evidence, with exhaustive answers that name their coverage and unknowns that stay
  local.

§14.10 states the rebuild contract: work should follow what changed. Changed policies rebuild only
their consumers, and "unchanged source facts and units need no reinterpretation". §14.12 makes
compile/index cost and p50/p95 latency part of the qualification report.

The workload premises S01–S10 (the [common brief](../evidence/2026-10-11_efficiency-alignment-codebase/inquiry-briefs.md))
add four conditions:
- a second library 5–20× larger, with many releases retained in one store;
- warm and patch-release recompiles;
- concurrent agents against one pinned publication;
- a development loop of concurrent fixture-backed tests on one shared SurrealDB 3.3 service.

### The strongest structural causes

**1. The native write path materializes one global conflict point and splits every logical
write into several fully fenced effects (F01–F07).**
- Every effect transaction reads *and writes* `native_installation:current`. That is three
  transactions per effect, on SurrealDB 3.3's optimistic RocksDB engine.
- Every 128-row window is three or four such effects.
- Original bytes are windowed per artifact chunk, so each small file costs three effects.
- Within one compile this costs sequential round trips and durable commits, not conflicts.
  Across processes it turns every concurrent writer into an abort-and-retry competitor:
  - fixture tests;
  - concurrent compiles;
  - reader-pin acquisition.
- The retry loop is unbounded, immediate and invisible.
- The same protocol forecloses the producer overlap and parallel windows that would otherwise
  hide round-trip latency.

**2. Same-process publication treats freshly admitted content like untrusted import (F09–F12).**
- Seal recomputes the whole completed-state identity, including cold backing validation. Admission
  already computed it with the same code.
- It re-derives every canonical row and every original's bytes, and re-runs the same `lower`
  function that `materialize_search` ran minutes earlier. That second run is labelled
  "independent", but it is the producer again.
- An unchanged republish repeats all of this. Derived ownership is written three times per row.

**3. Reuse stops before the most expensive work, and a reuse hit still costs O(rows) of writes
(F13, F16–F20).**
- Facts providers have no reuse unit, although their complete dependency key exists before they
  run.
- Downstream keys chain provenance through view identities and couple sibling outputs, so no value
  cutoff exists. The profile is keyed into everything.
- A hit copies every membership row and mints new physical identities that accumulate under shared
  views.
- The per-root portable cache crosses the native and file boundaries per root on every cold compile.

**4. The verified membership run is not used as an access path for key-only demand (F21–F25).**
- `id IN (…)` grain seeds iterate the whole view's membership run per grain. That is quadratic in
  root count.
- Multi-owner key lookups are per key and scan every retained contribution that ever held the key.
- Unrequested behavioural methods materialize per-scope `NotRequested` coverage as full stage units.

**5. Serving has no owner for facts that are fixed for the life of a pin, and enumerates the
eligible vector domain before it scores (F35–F40).**
- Each call re-hashes about 8.3 MB of embedded source and regenerates the wire schemas.
- Each call rediscovers the library domain to build its own cache key.
- Each call re-proves view membership per window, channel and family.
- Each call hydrates every eligible vector occurrence before computing scores.

**6. The development loop multiplies the above (F41, F45–F50).**
- Fixture tests repeat a full compile per test process.
- Every test process sizes CPU pools and a 64 GiB memory budget as if it owned the host.
- One crate edge puts the whole analyzer stack into publisher and serving test closures.
- Each boundary re-proves the server executable's identity up to six times.

Several of these causes compound. The S10 timeouts have three candidate causes that are not
mutually exclusive (§10, R12):
- repeated fixture compiles (F45);
- independent per-process admission (F41);
- installation-row serialization (F01).

Only a counted-retry observation (F02) plus a matched-load run can apportion them.

### What the design gets right, and must keep

These are preservation constraints on every remedy below (details in [Strengths](#strengths-and-preservation-constraints)).
- **No long transactions.** Every effect is short and bounded.
- **Content addressing with full-value collision checks**, and guard-based retirement that
  understands optimistic concurrency.
- **Rigorous unknown-acknowledgement reconciliation.**
- **Pinned readers whose continuations never rediscover.**
- **Exact, frozen ranking.** Exact vector scoring is required, not a missed optimization (F37).
- **Explicit `NotRequested`, `Partial` and refusal states.**
- **Premise-memoized invariant admission within a workspace.**
- **Wake-driven bridging**, with charged budgets.
- **Independent oracles and evaluators** (`lctx-eval`, the RECORD SHA-256 check, cold
  audit/restore admission).
- **A sound split** between incremental workspace crates and O3 imported crates.

### Finding index

Priority follows consequence (§12). The prerequisite order is separate and is given after the
index. Mechanism codes are the heuristics (H), foundations (FP), supporting rules (DP) and
profile principles (CI).

| ID | Finding | Priority | Main scenarios | Provenance |
|---|---|---|---|---|
| [F01](#f01) | Every native effect transaction writes the one installation record | High | S02, S05, S06, S10 | L1-01 (corrected), H-01, D-01; V01; R1 |
| [F02](#f02) | Conflict retry is unbounded, immediate and unobserved | Medium | S10, S02 | H-03, D-01, L1-01 |
| [F03](#f03) | One logical window write is 3–4 separately fenced effects | High | S01–S03 | A-03, D-02; L1-02 seam |
| [F04](#f04) | Original bytes are windowed per artifact chunk, not per batch | High | S01, S03 | A-01; V07 |
| [F05](#f05) | Per-effect control machinery costs the same whatever the effect needs | Medium | S01, S02, S08 | D-03, G-01, G-02; V12 |
| [F06](#f06) | Streamed statements stay open across per-window effects under fixed deadlines | High | S02, S03 | H-02, D-06; P6; R9 |
| [F07](#f07) | The facts phase awaits every batch and every provider in sequence | Medium | S01, S02 | A-05, A-06, H-05 |
| [F08](#f08) | The store re-establishes a canonical form the `Batch` type already guarantees | Low | S01, S02 | A-08, G-03 |
| [F09](#f09) | The completed-state identity re-serializes the whole state, with cold validation, twice per compile | High | S01–S03 | B-05, G-04; R8, R22 |
| [F10](#f10) | Same-process seal repeats publication's derivations and admission's checks | High | S01, S03, S05 | D-04, G-05; V10; R17, R22 |
| [F11](#f11) | Unchanged content republishes in full; the handle omits the derivation implementation | Medium | S03, S05 | D-05; R18 |
| [F12](#f12) | Derived ownership is created per row, re-held per row, then deleted | Medium | S01, S03 | D-06; R19 |
| [F13](#f13) | Unadmitted attempts' completed work has neither a reuse nor a reclamation route | Medium | S07, S08 | D-09, D-10; R20, R21 |
| [F14](#f14) | Original bytes are stored per revision-scoped identity | Medium | S02, S03, S08 | A-07 |
| [F15](#f15) | Admission validity is carried only within one workspace | Low–Medium | S03 | G-06 |
| [F16](#f16) | Facts providers have no reuse unit | High | S03, S04, S07 | A-04, C-01; V11; R2 |
| [F17](#f17) | Reuse hits copy memberships, mint per-attempt identities and re-index existing views | High | S03, S08 | C-04, D-07, D-08; R16 |
| [F18](#f18) | Reuse keys chain provenance and couple sibling outputs; there is no value cutoff | Medium | S03 | C-02 |
| [F19](#f19) | The profile is keyed into every contribution | Medium | S04 | C-03 |
| [F20](#f20) | The per-root portable cache is crossed per root on every compile and amplifies its own bytes and accounting | Medium | S01, S02, S10 | C-05, C-06, C-07; V09; P7 |
| [F21](#f21) | Key-only demand is served by payload windows over the whole membership run | High | S01, S02 | B-01, B-07; V04; R6 |
| [F22](#f22) | Multi-owner explicit-key lookup runs per key and scans retained history | Medium | S02, S08 | B-02, L1-05; R5 |
| [F23](#f23) | Each contributing provider re-forms and re-installs union views of every vocabulary relation | Medium | S01, S02, S09 | A-02; V08 |
| [F24](#f24) | Unrequested behavioural methods run as full stage units and materialize per-scope `NotRequested` coverage | Medium | S01, S02 | B-03; R7; P8 |
| [F25](#f25) | The expected coverage domain is rebuilt by every analysis owner | Low–Medium | S01, S02 | B-04; R7 |
| [F26](#f26) | Per-module canonical work is repeated inside the Pyrefly provider | Medium | S01–S03 | L2-01, L2-02; V05; R13 |
| [F27](#f27) | Dependency-context analysis waves bypass the shared analysis pool | Medium | S01, S02 | L2-03; V05 |
| [F28](#f28) | Artifact-to-use lookups in provider loops are quadratic | Low–Medium | S02 | A-10 |
| [F29](#f29) | Capture verification sits where no uncertainty enters | Low | S01, S03 | A-09, G-08; R24 |
| [F30](#f30) | Corpus revision attribution checks only `HEAD` | Low | S01 | A §4 (promoted) |
| [F31](#f31) | Flow conditions are tree-copied and lowered through fresh variable sets | Low | S04 | L2-05 |
| [F32](#f32) | Structural delegation rebuilds whole-universe indexes per seed on the default catalog path | Medium–High | S02 | C-09 |
| [F33](#f33) | Optional analytics expand candidates quadratically under one global work cap | Low | S02 | C-10, C-11 |
| [F34](#f34) | Retrieval embedding effects are batched per rendered root | Low | S01, S02 | C-08 |
| [F35](#f35) | Serving re-derives pin-fixed facts per call: constant identities, domains, membership proofs and pages | High | S06, S02 | E-01, F1-05, E-02, E-03, E-08; R4, R11 |
| [F36](#f36) | The vector channel hydrates the whole eligible domain before scoring, and its ranking kernels scan linearly | High | S02, S06 | E-04, E-07; R3 |
| [F37](#f37) | The vector contract is stated two ways, and the shared HNSW index has no production reader | Medium | S01, S05, S08 | E-05, L1-04 (corrected); P5; R3 |
| [F38](#f38) | Serving SQL re-defines view membership in uncorrelated subqueries | Medium | S06 | E-06; V06 |
| [F39](#f39) | MCP admission bounds the embedding call with the native-capacity slot | Low | S06 | E-09 |
| [F40](#f40) | The external sorter caps runs at 1 MiB, merges pairwise and creates a tempdir eagerly | Medium | S01, S02, S06 | L1-03; R10 |
| [F41](#f41) | Per-process resource admission ignores co-located processes | Medium | S10, S02 | H-04, F1-02; R12 |
| [F42](#f42) | No external cancellation reaches the compile's owned drain | Medium | S07, S10 | H-06; R15 |
| [F43](#f43) | Per-root phase events are emitted at INFO by default | Low | S02 | G-07; R25 |
| [F44](#f44) | Tokenizer assets are verified on one read and loaded from another | Low | S01 | G-09; R26 |
| [F45](#f45) | Fixture-driven provider tests recompile one fixture per test process | Medium | S10 | F1-01; R12 |
| [F46](#f46) | Crate boundaries couple unrelated rebuild and identity closures | Medium | S10, S03 | F1-03, F1-04; P9 |
| [F47](#f47) | Two near-identical optimized profiles keep separate build trees | Low | S10 | F1-06 |
| [F48](#f48) | Boundary tooling re-proves fixed premises on every invocation | Low–Medium | S10 | F2-01, F2-02; P1 |
| [F49](#f49) | Focused and rerun selections never narrow the build | Low–Medium | S10 | F2-04 |
| [F50](#f50) | Owner-to-object completion scans and validates the whole host registry | Low | S10, S08 | F2-03; P2 |

**Grain.** There are 50 findings, which is more than the brief anticipated. The scope covers the
whole product plus the development loop. Manifestations of one cause were grouped, for example
F35, F13, F17 and F41; distinct closure obligations were kept inside each finding, not split
into new IDs. Items judged not to be findings are listed in [§7.10](#79-observations-that-are-not-findings).

### Prerequisite order (distinct from priority)

1. **Fence and protocol semantics come first: F01, then F02.**
   - Capability: concurrent effect transactions conflict only when they touch the same object, and
     contention is countable.
   - Until then:
     - fusing effects (F03) lengthens every transaction that a short intent or release from
       another process can abort;
     - write-behind or concurrent providers (F07) turn a compile's own effects into conflicts;
     - parallel publication windows (F12, F03) do the same.
   - F04's window filling and F05(a) (an intent without payload bindings) do not lengthen the
     fenced section, so they can land before F01.
2. **The attach-by-reference contract (F17) comes before reuse expansion:** facts reuse (F16),
   recovery attachment (F13a), value-cutoff attachment under a refreshed specification (F18), and
   pointer-based per-root products (F20 alternative).
   - Capability: a hit's cost is O(1) in product rows.
   - Without it, the largest relations (facts) would gain a hit path that still copies every
     membership.
3. **Carry admission's identities (F09) before reducing seal (F10).**
   - Capability: seal can compare a witness instead of recomputing.
   - The order holds whichever of F09's remedies is chosen.
4. **Add a derivation-implementation identity to the handle (F11 premise) before the republish
   short-circuit.**
   - Capability: an identical handle then implies identical derived rows.
5. **Decouple sibling outputs (F18 step 1) before profile-neutral keys (F19).**
   - Capability: a profile-independent stage's key no longer moves when a profile-dependent
     sibling's content changes.
6. **Decide the vector contract (F37) before the vector reorder (F36).**
   - Capability: one owner states that exactness and frozen ranking govern, so the reorder is
     judged against that contract rather than against HNSW text.
7. **Add the pin-lifetime prepared state (F35b) before page-local continuation (F35d).**
8. **Add counted retries (F02) and the matched-load run (§10) before choosing between the S10
   rule change (RC12) and isolation alternatives.**
9. **F06 has no prerequisite** and removes a predictable failure. It can go first.

---

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | All crates (`cpg-extract`, `cpg-flow`, `cpg-core`, `lctx-model`, `lctx-surrealdb`, `lctx-publisher`, `lctx-serving`, `lctx-embed`, `lctx`, `lctx-eval`, `lctx-analytics`); `python/lctx_semantics`; `python/lctx_mcp`; the SurrealDB service integration; the embedding client; programmatic evaluation; development loop tooling (`scripts/verify*.py`, fixture, service, runs, storage lifecycle, readiness/sync). Excluded: docs publication tooling. Baseline `88f33bc0`. Uncommitted edits read at HEAD: `STATUS.md`, two publisher test files, the coordinator plan, `crates/lctx-surrealdb/src/{derived_search,reader,selection}.rs`, `docs/design/sections/synthesis-and-serving.md`. |
| Standard | Core principles and template 3.3; Heuristics 1.0; code-intelligence profile 1.5; library-context binding (Q17, §3 scenarios) |
| Tier · purpose | Design · target |
| Reviewer · date | Fresh principal `design-reviewer` · 2026-10-11 |
| Maturity and outcome | Design phase after the graph-native pivot (ADR-0138) and the native-control (ADR-0145) work. The decision should enable plan creation to schedule physical corrections in the right prerequisite order, with rule impacts presented to the operator. |
| Supported scope | Catalog compile through publication and sealing; behavioral profile (S04); native serving through MCP; programmatic evaluation; the development loop. Workload premises are S01–S10 (common brief). Host: 32 logical CPUs and about 188 GiB, shared. SurrealDB 3.3 runs as a remote RocksDB server over the patched gRPC SDK. vLLM serves Qwen3-Embedding-8B. |
| Expected changes | A second, larger library; warm and patch recompiles; policy changes (§14.10); a behavioral request over a catalog compile; publish/select/retire while serving; concurrent agents; interruption and recovery; history growth; analyzer upgrade or fact-family addition; the focused development loop. |
| Baseline | Implemented graph-native compiler with native contributions, admission, sealed publication and pinned readers. Most earlier dispositions sit at "Implemented / focused Tested, enclosing acceptance open". |
| Method and coverage | Static reading of decisive source at `path::symbol` grain by this reviewer, including pinned library source (`surrealdb-kvs-rocksdb-3.3.0`, `surrealdb-core-3.3.0`, `surrealdb-rpc-3.3.0`, the pyrefly fork `63cda07`). Inquiry reports, coordinator checks and probes are used as leads; any reliance on an inquiry's count or search is stated per finding. **Not examined by anyone:** real-library volumes, actual SurrealDB plans (`EXPLAIN`), end-to-end timings and peak memory. **Examined only through inquiries:** the internals of the behavioral kernels, synthesis kernels, `restore`/`backup_import`, `delivery`/`capability`/`inspection` serving modules, and `compile_profile*`/`worktree`/`maintenance` tooling. |

**Profile scope.**

Fact families in scope:
- capture/originals;
- Ruff syntax and lexical facts;
- Pyrefly typing, calls and Pysa;
- ty flow (behavioral);
- documents and deployment;
- normalized relations;
- coverage and outcomes;
- catalog members, evidence and selection;
- retrieval windows, vectors and lexical occurrences.

Analyses in scope:
- structural delegation;
- coverage frontier;
- optional neighbours, communities and FCA;
- BDD conditions;
- SCC summaries.

Served answers in scope: search, browse, find, get_operation/evidence, continuations and resources.

---

## 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Allowed dependencies / direction | Expected reason for change |
|---|---|---|---|---|
| `lctx-model` | Relations, identities (`KeySink`, `ContentHash`, `Id`), invariants, kernels, completed-view and contribution identity, resources | Typed records, `Batch`, invariants, kernel functions, `CompletedView`/`ContributionSpec` | Depends on no store; every crate depends on it | New semantic kinds or invariants; identity composition (F18, F19) |
| `cpg-extract` / `cpg-flow` | Capture, the five catalog providers (six with behavioral), analyzer adapters | `ProviderStage`, `StageContext`, provider contracts | Analyzer forks; `lctx-model` | Analyzer upgrade; provider reuse (F16) |
| `cpg-core` | Workspace, producer outputs, facts and stage orchestration, read sessions, admission, product reuse | `compile_driver`, `ProducerOutput`, `CompletedInputs`, `artifact::admit` | `cpg-extract` (**normal edge**: F46), `lctx-surrealdb`, `lctx-model` | Stage scheduling, reuse eligibility, admission placement |
| `lctx-surrealdb` | Native control protocol, compiler store, `NativeTable`, loader and reconciliation, derived search, reader and selection, product cache | `control::{effect, ensure_rows_owned, hold, hold_many}`, `NativeCompilerStore`, `NativeReader` | `lctx-model`; SurrealDB SDK (patched) | Fence protocol (F01), effect granularity (F03), access paths (F21, F22), crate split (F46) |
| `lctx-publisher` | Publication, seal, backup and retire, restore, audit | `seal_completed`, `publish`, `retire` | `cpg-core`, `lctx-surrealdb` | Seal assurance placement (F10), republish (F11) |
| `lctx-serving` | Pinned service, operations, search channels, preparation cache, pagination | `NativeService`, wire tools | `lctx-surrealdb`, `lctx-model` | Pin-lifetime prepared state (F35), vector route (F36) |
| `python/lctx_semantics`, `python/lctx_mcp` | PyO3 session; MCP transport, admission, embedder | Tool registration and envelopes from Rust | Rust owns meaning; Python owns transport | Admission scope (F39), deployment shape (§7.10) |
| Tooling (`scripts/`) | Verify boundaries, evidence capture, fixture/service, runs, storage lifecycle | `just verify`, `just fixture`, receipts | Python 3.14.7; no product imports | Re-proof placement (F48), narrowing (F49) |

**Semantic ownership.** Meaning is authored in `lctx-model` and consumed by orchestration and
adapters. That holds well across facts, coverage, outcomes, identities and invariants.

Two consumer-side re-expressions were found:
- **F38.** Serving SQL defines view membership locally, without the alias and nominal-conflict
  semantics that `SelectedPayloads::sparse_membership` owns.
- **C-06 (part of F20).** The product cache keeps two accounting authorities for one total: the
  `product_quota:current.used` counter and a `math::sum(bytes)` scan.

The design text also carries two incompatible vector contracts (F37).

A further identity-scope gap: the publication handle (`native_publication::publication_identity`) binds:
- the SurrealQL definitions;
- the serving and model implementation digests, through `fn::lctx_operation_definition`.

It does not bind the Rust derivation code that produced the derived rows:
`lctx_surrealdb::derived_search::lower`, `codec::entity_references` and `lexical_stats`.
`lctx-serving/build.rs::capture` digests only `lctx-serving/src` plus the manifests (F11).

### Fact and fidelity table (profile addition)

Fidelity, coverage and identity are as implemented; this review found no fidelity regressions
caused by the physical choices. The table records where each family's *physical* realization
matters to the findings.

| Fact family or relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers / efficiency findings |
|---|---|---|---|---|---|
| Captured originals and artifacts | Acquisition (capture + RECORD) | Extracted | Per-input closure manifest; RECORD mismatch refuses | `SourceArtifact(input, path, content)` | Admission chunk invariant; seal readback; serving evidence (F04, F14, F29, F30) |
| Canonical syntax and lexical facts | Latest Ruff 0.0.16 (fork `9080ee7`) | Extracted | Per-module parse refusal is explicit | Content-checked against the artifact | Typed syntax, diagnostics, observation (F26) |
| Typing, calls, Pysa | Pyrefly fork `63cda07` (embedded Ruff 0.0.14) | Resolved / inferred | Dependency context at `Exports`, roots at `Everything`; unresolved explicit | Provider build digest | Normalization, catalog (F26, F27, F16) |
| Flow facts and conditions | ty 0.0.16 crates (behavioral only) | Derived (runtime flow view) | `Refused` beyond the 4,096-node cap is explicit | Condition diagram identity | Behavioral stages (F31) |
| Vocabulary (Evidence, Literal, SyntaxDetail, …) | Pyrefly, Documents, Deployment (+ TyFlow) | Extracted | Union across contributors | View = relation + contributor set | Normalization inputs (F23) |
| Coverage, invocations, outcomes | Analysis owners | Derived | `NotRequested`, `Partial`, `Completed` explicit per invocation and scope | Invocation and scope identity | Frontier, manifest, serving availability (F24, F25) |
| Retrieval vectors / lexical occurrences | Retrieval and publication lowering | Derived; vectors from vLLM (observed) | Eligibility by view dependencies | Content-addressed rows; spec and projection identities | Serving channels (F36, F37, F35) |
| Optional analytics | Leiden, FCA, ranking, neighbours | Heuristic / statistically derived | `Partial/WorkLimit` explicit | Run-local labels | Not served as facts (F33) |

---

## 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs and expectations | Invariants and enforcement | Effects / lifecycle / failure | Substitution | Isolated verification |
|---|---|---|---|---|---|
| Native effect (`control/authorization.rs::{effect_for_owner, execute_owner}`) | SQL plus bindings; owner; exactly-once outcome under unknown acknowledgement | Era/generation fence (`ERA_GUARD`); owner predicate `FOR UPDATE`; request identity | Three transactions; durable `native_effect`; reconciliation on failure; **unbounded retry** | Single fence implementation | `tests/native_control.rs` (store) |
| Immutable ingress (`control::ensure_rows_owned`, `hold`, `hold_many`) | Rows plus owner; idempotent identical replay | Per-row guard, full-value collision, admitted-canonical fence | One effect each; holds create retention | — | Store family |
| Typed batch (`lctx_model::domain::record::Batch::with_reservation`) | Validated, sorted, deduplicated rows plus Arrow | Construction enforces canonical form | Charged reservation | Typed only | Model family (no store) |
| Completion (`NativeCompilerStore::complete_contribution_inner`) | Outputs; views as relation plus contributor set | Membership readback digest | View installation per output | — | Store family |
| Admission (`cpg_core::artifact::admit`) | Frozen attempt → manifest (families, completed state, originals) | Invariants with premise memo; graph admission | Marks the attempt admitted | — | Fixture family |
| Seal (`lctx_publisher::native_publication::Publication::seal`) | Manifest, loader, definitions → handle | State equality; reconcile; search reconcile; definition epoch | Hold transfer; marker; close; readback | Same path for Completed, Export and Restored admissions | Publisher family |
| Product reuse (`NativeCompilerStore::attach_retained_product`) | `ProductRequest` → attached contribution | Spec equality; admitted attempt | Copies memberships; installs views | — | Store and core families |
| Serving (`lctx_serving::service::NativeService::run`) | Request plus optional vector → bounded response | Definition check once; vector spec and projection per call; budgets | Read-only, snapshot reads | Pinned for the process | Serving and MCP families |

Absence states are explicit throughout (`NotRequested`, `Partial`, refusal, `unavailable`).
Equality claims are exact or content-addressed. The testing-boundary observation that matters
here belongs to F45 and F46: local semantic checks of providers require a native fixture
compile per test process, and store or serving test targets link the analyzer stack.

---

## 4. Composition and execution

| Operation | Semantic inputs → outputs | Owning mechanism | Reuse boundary | Effects / publication | Physical shape found (static) |
|---|---|---|---|---|---|
| Capture | Library closure → frozen copy plus receipts | `CapturedInput::capture_with`, `capture_receipts` | None across compiles | Private tempdir | 4–5 full closure reads (F29) |
| Facts | Captures → five provider contributions (catalog) | `facts::compile_facts`, `bundle::run_provider` | **None** (F16) | Native writes | Sequential providers; synchronous per-batch writes (F07); 3–4 effects per window (F03); 3 effects per file of originals (F04) |
| Completion | Contribution → views | `complete_contribution_inner`, `registered_or_install_views` | View identity | View member rows, holds | Union views re-formed per contributor (F23); streams held across effects (F06) |
| Normalization (9 stages) | Facts → normalized relations | `compile_driver`, `normalize::*` | Whole-stage `native_product` + per-root portable cache | Native writes | Per-root cache crossings (F20); `id IN` per grain (F21) |
| Upper stages (21 bindings under catalog) | Normalized → catalog, analyses, retrieval | `compile_driver`, `UpperStage` | Whole-stage attach | Native writes | Unrequested methods as full units (F24); domain rebuilt per owner (F25); structural per-seed index (F32) |
| Admission | Frozen attempt → manifest | `artifact::admit`, `artifact_manifest::populate` | In-workspace premise memo only (F15) | `mark_attempt_admitted` | Whole-state reserialization with cold validation (F09) |
| Publication | Manifest → derived rows → sealed handle | `seal_completed`, `materialize_search`, `Publication::seal` | None (F11) | Holds, marker, close | Second `lower`, third canonical scan, second state hash (F10); per-row ownership transfer (F12) |
| Serving request | Request → ranked packets / evidence | `NativeService::run`, `operations::*`, `search::*` | `PreparedCache` keyed by discovered roots | None | Per-call rediscovery and re-proof (F35); eligible-domain-first vector (F36) |
| Fixture boundary | Selection → test outcomes plus evidence | `verify.py::_run_boundary`, `surrealdb_fixture` | Per-invocation stat caches | Records, captures | Six executable digests; per-step binary digests (F48); no build narrowing (F49) |

**Analysis record columns (profile addition), for the analyses whose execution this review judges.**

| Question answered | Projection | Method and settings | Exact / conservative / heuristic | Budgets and partial behaviour | Output and evidence linkage | Finding |
|---|---|---|---|---|---|---|
| Delegation from a public seed | CallableInvocation + DefinitionContainment, selected subsystem | Bounded BFS per seed | Exact under bounds | `Bounds`; per-seed reservation sized on the **whole universe** | Traversal witnesses | F32 |
| Coverage frontier | Expected domain per method and frame | `CoverageAdmission`, `coverage::admit` | Exact | Explicit status per scope | Coverage rows per scope, including `NotRequested` | F24, F25 |
| Vector ranking | Eligible `vec_occurs` in the pinned views | Exact cosine at 1,024 dims, rescored at 4,096 | Exact; frozen | Tiers 128–1024; 30 s deadline | Witness occurrences | F36, F37 |
| BM25 lexical | FULLTEXT `@OR@` nomination; frozen publication statistics | Bespoke exact-view BM25 | Exact over nomination | Frontier ≤ 1,024 | Witnesses | (adequate; §7.10) |
| Neighbours / co-use communities | All window pairs / owner cliques | Exact all-pairs; Leiden | Heuristic | One global `MAX_WORK` → whole-relation `Partial` | Run-local | F33 |

---

## 5. Change and failure scenarios

| Scenario (kind) | Owning component | Contract change | Affected consumers: expected → observed | Independent edits / hidden knowledge / test setup | Evidence |
|---|---|---|---|---|---|
| S01 cold catalog compile (workload) | Whole compile path | — | Work proportional to output → protocol and re-verification passes dominate: 3–4 effects per window, 3 per original file, second lowering, second state hash | — | F03, F04, F09, F10; receipts P8 (attributed only) |
| S02 a library 5–20× larger, many releases retained (growth/skew) | Read path, serving, analyses | — | Linear in output → quadratic in roots (F21, F28, F32), retained-history coupling (F22), streams past fixed deadlines (F06) | — | Static structure |
| S03 warm, unchanged (reuse) | Reuse and publication | — | Near-zero work → full extraction (F16), O(rows) attach (F17), full republish (F11), admission re-run (F15) | — | Static structure |
| S03 patch release (instance) | Reuse, originals | — | Proportional to the change → one changed byte re-keys every artifact (F14) and every downstream key (F18) | — | Static |
| S03 changed analysis, rendering or embedding policy (policy) | Reuse keys | §14.10 table | Only dependents → Configuration sibling coupling invalidates catalog and retrieval (F18) | — | Static |
| S03 changed model declaration (domain concept) | Model digest in every key | — | Conservative full rebuild; consistent with the ADR-0131 cutover | — | Not a finding (§7.10) |
| S04 behavioral over catalog (policy) | Profile keys | — | Reuse of profile-independent work → nothing reused (F19) | — | Static |
| S05 publish and select while serving (lifecycle) | Seal, pins | — | Serving unaffected at the KV layer (adequate); pin acquisition conflicts with publication effects (F01) | — | Pinned engine semantics |
| S06 concurrent agents, high degree, cold start (concurrency) | Serving | — | Per-call work bounded by the answer → per-call rediscovery and whole eligible domain (F35, F36); per-row subqueries (F38, unresolved) | — | Static |
| S07 cancel, crash, restart (failure) | Lifecycle | — | Replay the missing unit → whole attempt replayed (F13); cancelled attempt left open (F42) | — | Static |
| S08 history growth (growth) | Retention | — | Ordinary work independent of history → multi-owner key lookups (F22) and the cache SUM (F20) scan history; unadmitted content never reclaimed (F13); per-attempt identities accumulate (F17) | — | Static; HS6 inventory receipt |
| S09 add a fact family (domain extension) | Provider + model | One declaration plus provider code | Additions follow one owner (adequate semantically); physically every vocabulary contributor re-forms union views (F23), and each new kind adds per-kind stage units (F24) | One model declaration, one provider; no duplicated meaning | Static |
| S09 analyzer upgrade (mechanism substitution) | Provider adapter | Fork revision and build digest | Adapter-local (adequate); all facts and downstream products invalidate through the build digest (sound), and nothing below extraction is reused (F16, F18) | Fork bump only | Static |
| S09 replace the fence protocol (mechanism substitution) | `control/authorization.rs` | Fence realization | Control-local, if every predicate-scan consumer's phantom partner is re-derived (F01) | Phantom inventory needed | Pinned source |
| S10 focused loop, concurrent fixtures (development) | Tooling, tests, build | — | Build of the edited closure plus focused tests → analyzer stack in store/serving closures (F46), per-test compiles (F45), independent per-process admission (F41), six identity proofs (F48) | Fixture service shared | Historical receipts (F1, F2) |

**Code-intelligence journeys.**
- **Trace a served claim** (CI-13): continuation pins are honoured (adequate, E §5.1). F11 shows
  the handle's identity scope omits derivation code.
- **Grow content or semantic kinds**: F21, F23, F24, F32.
- **High-degree node or concurrent compile/serve**: F01, F36, F38.
- **Publish or replace a realization**: F10, F12.
- **An evaluation run**: no leakage found. The numeric reference qualifies an unused route (F37).

---

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | **Unresolved** | Serving SQL re-defines view membership without alias and nominal-conflict semantics (`selection.rs::{library_packages_sql, member_candidates_sql}`, `evidence.rs::derivations` vs `SelectedPayloads::sparse_membership`): whether the two can disagree for these entity families is not settled. The product cache keeps two accounting authorities (`product_quota.used` vs `math::sum(bytes)`). | F38, F20(b) |
| G2 Semantic fidelity | **Pass (sampled)** | `NotRequested`/`Partial`/refusal explicit; ranking exact and frozen; Ruff noqa reconciliation by key multiset is count-exact for identical keys. Sampled, not product-wide. | — |
| G3 Validity | **Pass (sampled)** | Construction validates `Batch`; store collision and fence checks; admission invariants. | — |
| G4 Hidden behaviour | **Fail (narrow)** | `retrieval_tokenizer::LocalTokenizer::load_verified` verifies `tokenizer.json`/`config.json` from one read and `load` re-reads them from disk: an ambient change between the reads alters the loaded tokenizer without changing the recorded identity (CI-10). | F44 |
| G5 Consistency and recovery | **Pass (sampled)** | Seal freezes before reconciling; pinned readers; idempotent retries; explicit partial states. The unbounded retry (F02) and the open attempts after cancellation (F42) are DP-20 and operational concerns, not inconsistent outputs. | F02, F42 (A4, not G5) |
| G6 Transformation and reuse | **Pass (sampled)**, with one identity-scope gap | Reuse requires spec equality and an admitted attempt; keys are conservative. The handle omits derivation implementation (F11); today a changed lowering fails closed at actual-row reconciliation rather than reusing stale rows. | F11 premise before any short-circuit |
| G7 Truthful capability claims | **Fail** | §14.4/§14.8/§14.11 (`api-and-evidence-product.md`), ADR-0131 and `docs/surrealdb.md` claim native HNSW candidate ranking. The implemented and governing route is exact (`synthesis-and-serving.md` §11.2 at HEAD; `search.rs::vector_selection_sql`), and no serving or compile path queries `neighbor`. | F37 |
| G8 Library leverage | **Pass** | Bespoke sorter, ranking, FCA and BDD bounding each have stated contract reasons; their defects are physical parameters (F40), not missing libraries. | — |
| CI-G1 Fidelity | **Pass (sampled)** | No relabelled relation or absent-as-unknown found in the examined paths. The narrow attribution-label gap F30 does not let an unknown read as absent. | F30 |
| CI-G2 Evidence closure | **Pass (sampled; not re-examined in depth)** | Pinned continuation binding and original-range evidence re-read; synthesis closure not re-traced. | — |
| CI-G3 Evaluation integrity | **Pass (sampled)** | `lctx-eval` has no production dependencies; no path from gold or heldout into production was found in scope. The numeric reference's relevance defect is F37, not leakage. | — |

---

## 7. Findings

Each finding states the following:
- cause, with `path::symbol` evidence;
- consequence under named scenarios, with its mechanism;
- the corrective contract: what the corrected operation consumes, decides and produces;
- preservation constraints, and the counterexample checked;
- uncertainty;
- evidence labels;
- closure evidence;
- provenance.

On labels: diagnoses are Interface-checked or Implemented. Remedies are Proposed. Measured
applies only to a probe's narrow quantity. "Principal re-read" names what this reviewer read
directly; "relies on" names an inquiry's count or search that this reviewer did not repeat.

### 7.1 Native write and control protocol

<a id="f01"></a>
#### F01 — Every native effect transaction writes the one installation record

**Cause.**
- `lctx_surrealdb::control::authorization::ERA_GUARD` runs `SELECT * FROM ONLY native_installation:current FOR UPDATE` and then `UPDATE native_installation:current SET control_revision=…+1`. Its own comment reads: "Every mutation conflicts with era closure, even if its payload writes disjoint records."
- `execute_owner` runs it in both the intent transaction and the payload transaction.
- The outcome-reference release transaction writes `control_revision` again.
- Installation-owned effects also write `admission_revision`.
- At the pinned engine, `surrealdb-kvs-rocksdb-3.3.0/src/lib.rs` opens an `OptimisticTransactionDB` with `set_snapshot(true)`.
  - `getu` calls `get_for_update_opt`, which registers the key for commit-time conflict detection and takes no lock.
  - `Busy` and `TryAgain` map to `TransactionConflict`.
- So any two effect transactions whose snapshot-to-commit intervals overlap cannot both commit, whatever records their payloads touch.
- **Critical section counted (safeguard 9).** It is each transaction's server-side interval, three per effect. Within one compile the effects are strictly sequential (F07), so a compile does not conflict with itself. The contention is between processes.

**Consequence.**
- **S10.** Up to 32 nextest processes run fixture tests on one `validation` database. Every effect transaction from every process contends on one key, and short intent and release transactions abort long 128-row payload transactions, which then re-execute in full (H23, H24).
- **S02.** Concurrent compiles behave the same way.
- **S05, S06.** MCP pin acquisition (`control::pin_views` plus holds: three effects) retries against publication and compile traffic.
- **Future concurrency.** The protocol forecloses parallelism within a compile: F07 write-behind, concurrent providers and parallel publication windows would all convert into conflicts.
- Mechanisms: H21, H22, H23, H24; FP-07; A4.
- Corroboration, as a historical receipt attributed by inquiry D: an HS6 qualification control failed because another control moved `control_revision` (persisted-graph plan §9.1, 2026-10-11).
- Magnitude unmeasured.

**What the write protects.**
- Era closure (`control/history.rs`, the effect at the `era closure` site) and history inventory check "no unresolved effects, open attempts, live pins, active backup holds" with predicate scans. Optimistic RocksDB transactions do not conflict-track those scans.
- The shared write materializes the phantom. That is a legitimate defence against write skew (DP-19), placed on the frequent side.
- Era closure already requires `admission_open=false`, so the protocol has a first phase. Ordinary attempt- and cleanup-owned effects do not check that flag.

**Corrective contract (Proposed).**
- An ordinary effect consumes the installation fence as a read: `FOR UPDATE` plus the era, generation, closed-through and new *cutting* checks. It writes only its owner, guards and payload.
- Each rare fence (era closure, history inventory, upgrade publication, maintenance drain) becomes two transactions:
  1. a commit that writes the fence state;
  2. a scan and transition in a transaction that starts after that commit.
- The interleavings are then covered:
  - An effect whose snapshot predates the fence write and commits after it aborts on its registered read.
  - An effect that commits before the fence write is visible to the later scan.
  - An effect whose snapshot follows the fence write sees the state and refuses.
- Allocation of `admission_revision` moves to its own sequence record.
- Lane striping (L1's alternative) is kept for any predicate-scan consumer whose scan cannot be split.

**The two remedy alternatives stay explicit (R1).**
- **L1: striped fence.** Effects write one of K lanes; fences write all lanes. This preserves bidirectional conflict without restructuring the fences, but every effect still pays a durable write and contends at 1/K.
- **H and D: read-only fence plus two-phase fences.** This removes ordinary-effect writes entirely, but requires a per-consumer phantom inventory.
- This reviewer prefers the second, because the fences are rare maintenance operations (H25: put the cost on the rare side). The choice is settled by that inventory, not by preference.

**Preservation and counterexample.**
- No effect of a closed era may commit after the cut, and pending effects must stay discoverable by the cut.
- Delayed or retried requests stay fenced by era and closed-through.
- Outcome references are released after commit.
- The HS6 assertions must be re-expressed without the global counter.
- **Counterexample checked (H's):** retirement's `native_backup_hold` scan relies on backup-hold creation and retirement both writing the installation. A backup hold that commits just after a retirement which no longer writes the installation must still be excluded, so that consumer needs its own guard-object fence or a lane.

**Uncertainty.**
- The conflict rate is unmeasured: retries are silent (F02).
- The full phantom inventory has not been built. Inquiry D did not trace history-compaction predicates fully.

**Labels.** Diagnosis Interface-checked (pinned engine) and Implemented (protocol). Remedy Proposed.

**Closure evidence.**
- A per-site inventory of the installation writers and their phantom partners.
- Source showing ordinary effects with no installation write.
- Adversarial interleaving controls in both orders for each fence.
- A concurrent fixture boundary whose retry counter (F02) shows no cross-process conflicts for disjoint effects.

**Principal re-read.** `control/authorization.rs` in full (`ERA_GUARD`, `begin_attempt_in`, `effect_for_owner`, `execute_owner`, release and reconciliation SQL); `control.rs::run_transaction`; `control/history.rs` era-closure SQL; pinned `surrealdb-kvs-rocksdb-3.3.0/src/{lib.rs,owned_tx.rs}`.

**Provenance.** L1-01 (corrected: the "8-way collapse within one compile" is wrong; R1), H-01, D-01; V01; R1.

<a id="f02"></a>
#### F02 — Conflict retry is unbounded, immediate and unobserved

**Cause.**
- `control.rs::run_transaction` loops on `retryable_transaction` with only `tokio::task::yield_now()`.
- Each try re-sends the whole query and its bindings.
- There is no bound, backoff, jitter, cancellation check or counter.

**Consequence.**
- **S10, S02.** Immediate resends maximize overlap and so the abort rate under F01, and a long payload can starve against a stream of short commits (H24).
- Because nothing counts retries, F01's share of the S10 timeouts cannot be attributed (R12). DP-21 asks that lifecycle decisions, including retries, be observable.

**Corrective contract (Proposed).**
- A bounded, jittered exponential backoff that keeps the request handle and bytes unchanged.
- A per-transaction-kind conflict count, emitted through the existing `Phase`/`lctx_phase` observation.
- A typed contention outcome when the bound is reached.

**Preservation.**
- Exact-message retryability matching.
- The identity-retaining retry.
- Idempotent payload short-circuit (`$__operation.resolved`).

**Counterexample.** A bound that is too low would fail valid work under transient contention. The bound must therefore be generous, and its exhaustion must be classified as contention, not failure of meaning.

**Uncertainty.** None on structure.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Retry counts appear in a concurrent boundary's evidence. A forced-conflict control terminates with a typed contention outcome.

**Principal re-read.** `control.rs::run_transaction`, `retryable_transaction`.

**Provenance.** H-03; D-01 and L1-01 (secondary parts).

<a id="f03"></a>
#### F03 — One logical window write is 3–4 separately fenced effects

**Cause.** Each of these runs its parts as separate effects (each an F05 protocol instance: an owner read plus three transactions):
- `lctx_surrealdb::compiler::NativeCompilerStore::ensure_physical_rows` runs, per 128-row `NativeWindows` window, `ensure_rows_owned(anchors)`, `ensure_rows_owned(rows)`, `hold(ids)` and `hold_many(references)`.
- `ensure_immutable_rows` (membership, aliases, original headers) runs `ensure_rows_owned` plus `hold`.
- `loader.rs::Loader::insert` (publication references and derived search) runs `ensure_rows` ×2 plus `hold_many`.
- `registered_or_install_views` and `retain_attached_members` add their own ensure and hold pairs.

Each effect upserts the object's guard again, so one row's guard is written in up to three transactions.

**Consequence.**
- **S01–S03.** Protocol round trips and durable commits dominate small-row writes, because they grow as windows × 3–4 effects × 4 RTs.
- Between `ensure` and `hold`, a row exists unheld, so a concurrent retirement can claim shared content and fail the attempt on its hold (fail-closed but avoidable).
- Mechanisms: H2 (one physical unit per semantic sub-operation), H8, H23; FP-07; A4.
- Inquiry D's attributed historical median of 16.6 s for `publication_references` at fixture scale is consistent with this, but not attributed to it.

**Corrective contract (Proposed).**
- One guarded window effect consumes a bounded window: rows, anchors, owner holds and reference holds. In one transaction it:
  - upserts each guard once;
  - checks fences, retiring state and collisions;
  - creates the absent rows;
  - inserts all holds.
- It produces one committed effect identity per window.
- Row and ownership become visible atomically, and the 128-row and 8 MiB window bounds stay.

**Preservation.**
- The full-value collision check.
- Per-guard retiring and epoch fences.
- The admitted-canonical and contribution fences.
- Byte bounds.
- All-or-nothing per window.

**Counterexample.** A larger transaction widens the window that F01's short transactions can abort, so this lands after F01. Within one compile it is safe either way, because effects are sequential.

**Uncertainty.** Server per-row execution cost is unmeasured. The set-based form of the per-row loop (L1-02) is not established as cheaper (§7.10).

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Source showing one effect per window for node, membership, view-member and derived windows; a concurrent-retirement control on shared content.

**Principal re-read.** `compiler.rs::{write_batch_inner, ensure_physical_rows, ensure_immutable_rows}`, `loader.rs::Loader::insert`.

**Provenance.** A-03, D-02 (L1-02 is the per-row seam).

<a id="f04"></a>
#### F04 — Original bytes are windowed per artifact chunk, not per batch

**Cause.**
- `NativeCompilerStore::write_batch_inner` runs `for row in &original_chunks { self.persist_original_chunk(row).await? }`.
- `persist_original_chunk` splits one `ArtifactChunk` (≤ 1 MiB) into 64 KiB rows and calls `ensure_immutable_rows("original_chunk", …)` for that chunk alone.
- That issues `ensure_rows_owned`, `hold` and `hold_many`: three effects per chunk, with windows that never fill.
- Every closure file under 1 MiB is one chunk.

**Consequence.**
- **S01.** Inquiry A counted 7,935 non-empty files in the legacy fastmcp environment; this reviewer relies on that count. On that basis original bytes alone cost about three effects (≈12 round trips) per file, with the corpus and derived documents on top. That is the largest round-trip term of the acquisition provider.
- **S03.** All of it repeats on every unchanged recompile (F16).
- Mechanisms: H2, H8; DP-10.

**Corrective contract (Proposed).** Gather every chunk row (and its source→chunk reference) of the batch into one row vector, then pass it once through `NativeWindows`. That gives three effects per 128 slices, and one under F03.

**Preservation.**
- Every row is validated before any effect.
- Immutable-collision refusal.
- Header and chunk ownership holds.
- The 8 MiB transfer cap.

**Counterexample.** None that rejects valid work: the window semantics are unchanged.

**Uncertainty.** File counts for the current generation; round-trip latency.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** An effect-count control for N small files shows ⌈slices/128⌉ windows.

**Principal re-read.** `compiler.rs::{write_batch_inner, persist_original_chunk, ensure_immutable_rows}`.

**Provenance.** A-01; V07 (corrects the operation map's "12 per window").

<a id="f05"></a>
#### F05 — Per-effect control machinery costs the same whatever the effect needs

**Cause.** One structural choice: every effect carries the complete retained-retry request protocol. It has three obligations.
- **(a) The intent ships the payload.** `execute_owner` runs the intent transaction with `bindings.clone()`, and its SQL references only `$__*` control variables. The SDK sends every bound variable, so each payload (up to 128 rows and 8 MiB) crosses gRPC twice.
- **(b) The request digest is computed twice and compared with itself.**
  - `effect_for_owner` computes `ContentHash::of(serde_json::to_vec(&(sql, &bindings)))`, and `execute_owner` recomputes it to compare.
  - `surrealdb-types-3.3.0` `Bytes` serializes through `serialize_bytes`, which `serde_json` writes as an integer array. P7 measured ×3.31 for one sample.
  - The only caller with a separately retained request, `execute_request`, has no production callers. The search covered `crates/` excluding `/tests/`: its callers and `history::retain_outcome`'s are tests.
  - So every production `request` comparison checks a value written from the same in-memory request (V12). It challenges nothing.
- **(c) Three transactions plus durable rows.** Intent, payload and release are three transactions, plus an owner read. Durable `native_effect` rows persist until maintenance compaction.

**Consequence.**
- **S01, S02.** Two whole-payload JSON serializations and BLAKE3 passes, plus a doubled transfer, for every window.
- **S08.** `native_effect` history grows: inquiry D's HS6 inventory receipt counts 267,591 rows over 337 attempts.
- Small effects (holds, markers, pins) pay four round trips of control.
- Mechanisms: H16, H20, H26; DP-23 (repeating the producer is not an oracle).

**Corrective contract (Proposed).**
- (a) Bind only control variables to the intent.
- (b) Compute a request digest only where a retry handle is retained. Otherwise key the receipt by operation identity, or digest the content-addressed row IDs plus the SQL.
- (c) Investigate creating the effect record committed inside the payload transaction. `reconcile_request` already creates a fencing record when it is absent, and a late payload's `CREATE` of the same id fails. Keep explicit outcome references only where the caller retains them.

**Preservation.**
- Refusal of a changed payload under a genuinely retained retry.
- Disposition of an ambiguous failure through operation identity.
- Era and owner fencing.
- Refusal of a late duplicate.

**Counterexample.** For (c), recovery consumers that read `committed=false` intents (drained-maintenance `drain_effects`, cleanup successors) may depend on the intent record. (c) is unresolved until the ADR-0145 owner confirms them. (a) and (b) are safe now.

**Uncertainty.** The magnitude of (a) and (b) relative to round-trip latency is unmeasured.

**Labels.** Implemented; the SDK binding behaviour relies on G's reading of `method/query.rs`. Remedy Proposed.

**Closure evidence.**
- Intent bindings contain only `__*` keys.
- At most one digest per effect.
- Retry-payload-change refusal is kept for the retained flow.
- For (c), lost-acknowledgement, delayed-payload and era-cut controls.

**Principal re-read.** `control/authorization.rs::{effect_for_owner, execute_owner, execute_request}`; `rg 'execute_request\(|NativeRequest::issue|retain_outcome\('` over `crates`, excluding `/tests/`.

**Provenance.** D-03, G-01, G-02; V12; P7.

<a id="f06"></a>
#### F06 — Streamed statements stay open across per-window effects under fixed deadlines

**Cause.** Three operations keep one streamed `SELECT` open while they issue effects per 128 rows:
- `lctx_publisher::native_publication::seal_staging` streams `SELECT VALUE object FROM native_hold … WHERE owner=$attempt ORDER BY id` and runs `control::hold` per 128 roots;
- `NativeCompilerStore::attach_retained_product` streams every source membership while `retain_attached_members` writes windows;
- `complete_contribution_inner` streams contribution memberships while calling `new_membership_keys` per 128 keys.

At the pins:
- the server's `query_timeout` covers producing every row of a streamed query (`surrealdb-core-3.3.0/src/rpc/protocol.rs`);
- production is consumer-paced (`surrealdb-rpc-3.3.0::QUERY_STREAM_BUFFER = 2`);
- the service runs with `--query-timeout 20s --transaction-timeout 10s` (`scripts/surrealdb_service.py::unit_text`).

**Consequence.**
- **S02, and S03 warm attach of a large product.** Stream duration grows as rows/128 × the effect round trips, up to a fixed deadline: a predictable failure of supported work, not a protective refusal (H11, H23). The open read also pins a RocksDB snapshot.
- P6 found one "streaming query did not stop within the grace period … timeout=20s" in the journal for 2026-10-10. It is not attributed to a call site.

**Corrective contract (Proposed).** Each operation consumes its immutable premise and pages it by keyset into short statements, as `control::keyset_source` and `release_pin` already do. Alternatively it drains into the existing disk-backed `SortedRows` before issuing effects. Each output is unchanged. The premises are immutable for the operation:
- the frozen attempt for seal;
- the completed, admitted source for attach;
- the closed producing scope for completion.

**Preservation.** The `(relation, semantic_key)` order that feeds `KeySink` digests; complete coverage of the set; cancellation and drainage.

**Counterexample.** Keyset paging over a mutable set could miss rows. All three sets are immutable during the operation, so it does not.

**Uncertainty.**
- Whether `owner_holds` serves `owner=$attempt AND id>$after ORDER BY id LIMIT 128` without a sort at 3.3.
- Per-site volumes.
- That the transaction timeout also binds bare streamed statements relies on H's reading of `dbs/executor.rs`. The query timeout alone is enough for the finding.

**Labels.** Interface-checked (pinned server) and Implemented (three sites). Remedy Proposed.

**Closure evidence.** No statement whose duration scales with attempt or library size; a control holding more than 20 s of roots at a known latency.

**Principal re-read.** `native_publication.rs::seal_staging`, `compiler.rs::{attach_retained_product, complete_contribution_inner}`, `surrealdb_service.py` unit flags, the pinned `protocol.rs` and `stream.rs`.

**Provenance.** H-02, D-06 (stream side); P6; R9.

<a id="f07"></a>
#### F07 — The facts phase awaits every batch and every provider in sequence

**Cause.** One structural cause, the facts phase as one sequential await chain, with two closure obligations.
- **(a) Synchronous per-batch acknowledgement.**
  - `cpg_core::workspace::Writer::write_batch` calls `NativeBridge::call_boxed`, which is `futures::executor::block_on`, so each batch blocks the provider thread until acknowledged.
  - `ProducerOutput::push` awaits `submit_batch_write`.
  - The `mpsc(2)` channel and `NativeCalls`' `Semaphore(8)` therefore never bind.
- **(b) Sequential providers.** `cpg_core::facts::compile_facts` iterates `schedule.stages()` and awaits each provider and its completion. Declared inputs order only Deployment after Acquisition, and Documents (and TyFlow) after Pyrefly. Acquisition (I/O- and write-bound) and Pyrefly (CPU on the AllThreads pool) are independent.

**Consequence.**
- **S01, S02.** Elapsed time is compute plus write latency instead of their maximum, and Acquisition's writes add to Pyrefly's analysis (H22).
- The live set stays at one batch, which is a strength.

**Corrective contract (Proposed).**
- (a) A bounded write-behind depth per producer (two to K), charged to the budget. Failure is observed before completion through `ProducingScope::close_and_wait`.
- (b) Ready stages run concurrently, with completions serialized by the existing `completion_gate`.

**Preservation.**
- A failed write fails the contribution before completion.
- Cancellation drains submitted work.
- Memory charges.
- Exact input binding for dependent stages.
- Deterministic union views.

**Counterexample.**
- Both obligations turn a compile's own effects into installation conflicts until F01 lands. That is a hard prerequisite.
- (b) raises the live set: Pyrefly's `State` and Acquisition's buffers coexist (H17).

**Uncertainty.** Whether compute or round trips dominate. After F03 and F04, (b) may not be worth its live-set cost; reassess then.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Late-error, cancel-in-flight and budget controls; a phase trace showing overlap.

**Principal re-read.** `facts.rs::compile_facts`; `workspace.rs::{Writer::write_batch, owned_batch_write, submit_batch_write}`; `native_bridge.rs::call_boxed`; `native_calls.rs`.

**Provenance.** A-05, H-05, A-06.

<a id="f08"></a>
#### F08 — The store re-establishes a canonical form the `Batch` type already guarantees

**Cause.**
- `lctx_model::domain::record::Batch::with_reservation` validates, sorts by `Record::id`, deduplicates and encodes.
- The single production write leaf, `cpg_core::workspace::owned_batch_write`, then passes `batch.arrow()` as an erased `&RecordBatch`.
- `write_batch_inner` calls `adapter.relation.canonical`, which is `model.rs::canonical`: `R::decode`, `sort_by_key(Record::id)` (recomputing ids per comparison), a duplicate scan, and `R::encode`.
- G further reports per-row re-derivations inside `adapter::{select, physical_row}`: decoding the canonical JSON back to recover an id the caller holds, a duplicate `content()`, and `validate_native_row` twice. This reviewer relies on G for those.

**Consequence.** One full decode-sort-encode per batch on every write (S01–S03), plus extra live representations (H16, H17, H19).

**Corrective contract (Proposed).** The write leaf consumes a canonical token constructible only by `Batch` (or `Batch::read`). The store skips `canonical` for it, keeps a checked constructor for untyped or test callers, and decodes graph rows once.

**Preservation.** Refusal of non-canonical input from any untyped entry point.

**Counterexample.** Tests that call `write_batch` with raw `R::encode` must still be canonicalized; the checked constructor keeps that.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** A trace with one decode per batch; duplicate and conflict controls still refuse.

**Principal re-read.** `record.rs::Batch::with_reservation`, `model.rs::canonical`, `workspace.rs::owned_batch_write`, `compiler.rs::write_batch_inner`.

**Provenance.** A-08, G-03.

### 7.2 Admission, publication and lifecycle

<a id="f09"></a>
#### F09 — The completed-state identity re-serializes the whole state, with cold validation, twice per compile

**Cause.**
- `cpg_core::artifact_manifest::populate` calls `NativeCompilerStore::completed_state`.
- `completed_state_inner` calls `state_identity_inner(true, …)`, then `state_identity_selected`.
- That function walks all six `STATE_TABLES` with `rows.next_batch(1)`, runs `validate_state_row` and `serde_json::to_vec` on every row into one `KeySink`, and, for `compiler_record`, runs `validate_backing_batch`: an Arrow re-projection, canonicalization, re-hash and `physical_row` comparison.
- Seal repeats the same call (`completed_state_after_closure`).
- `state_identity_inner` also begins with the global `wait_scans()` barrier.
- The import path deliberately passes `validate_backing = false` "without repeating model reconstruction" (comment at `state_identity_inner`). So cold backing validation, placed at cold exposure by the persisted-graph F01 correction, also runs twice on the ordinary same-process path.
- Completion has already produced exact per-contribution digests (`complete_contribution_inner` hashes `(semantic_key, content)` per output), and view identities derive from them.

**Consequence.**
- **S01–S03.** Whole-state, row-at-a-time reserialization grows with every membership of every stage, including F24's `NotRequested` coverage, and repeats in full for unchanged content (H14, H19, H20, H26).
- Historical receipts show a single `artifact_manifest` phase at 65–98 s on 2026-10-11 fixture journeys (G, B). P8 shows up to 190 s summed over two occurrences in one run. They are unattributed and show only that the phase can dominate.

**Corrective contract (Proposed).** One of two directions, both keeping the backing check where backing rows enter.
- **(a) Compositional identity.** The completed-state identity is composed from completion-time contribution digests plus view and binding descriptors. Admission produces it in O(contributions). Seal checks an attempt-revision or era witness (`bind_inner` already increments `$__attempt.revision`) instead of recomputing.
- **(b) Keep the physical-walk identity** (`native-completed-state/v3`), computed once per attempt in batched windows with `validate_backing = false` on the ordinary path. Seal compares the witness.
- In both, the cold reconstruction stays at import, export, restore and `inspection::audit`.

**Preservation.**
- Detection of any mutation between admission and seal, including another writer. That needs a witness covering attempt-owned rows (see the F10 challenge).
- Import equality.
- The backing-fidelity check from the persisted-graph implementation F01, at entry or completion, and at cold exposure.

**Counterexample.** An external process or direct SQL mutates attempt rows after admission. The witness must detect it, or seal keeps one recompute. Out-of-protocol mutation otherwise belongs to the cold audit.

**Uncertainty.** How much of `artifact_manifest` is this walk rather than `producer_inventory`, `semantic_outcomes` and the other `populate` steps. Not separated.

**Labels.** Implemented; remedy Proposed. Rule impact RC07 for (a).

**Closure evidence.** No `validate_backing_batch` on the ordinary path; one state computation per attempt; tamper, late-mutation and import-mismatch controls still refuse.

**Principal re-read.** `compiler.rs::{completed_state, completed_state_after_closure, completed_state_inner, state_identity_inner, state_identity_selected, export_state_inner}`; `artifact_manifest.rs::populate`.

**Provenance.** B-05, G-04; R8, R22.

<a id="f10"></a>
#### F10 — Same-process seal repeats publication's derivations and admission's checks

**Cause.** `lctx_publisher::native_publication::Publication::seal` treats every admission alike. After `begin_finalization` it runs:
1. `completed_state_after_closure` (F09);
2. `Loader::reconcile`, which:
   - decodes every entity and assertion a third time;
   - rebuilds `physical_row` with the writer's own adapters;
   - recomputes family hashes that `artifact::admit` computed from the same rows;
   - rebuilds expected edges with the writer's `codec::entity_references`;
   - re-reads every original: one header plus at least two paged chunk queries each, the last returning empty;
3. `derived_search::reconcile_search`, which calls the same `lower` with `loader=None`, labelled "independent cold lowering". It is the producer again, compared against the same actual rows that `materialize_search` compared minutes earlier, followed by `lexical_stats::reconcile`;
4. a second `definitions::verify_epoch` in `seal_staging`.

The admission invariant `artifact_chunk_content` (`lctx_model::domain::artifact::content_invariants`) has already hashed every original's stored bytes against the captured digest. Its `ArtifactChunk` body projection hydrates from `original_chunk` (`compiler.rs` projection `__original_chunks`).

**Consequence.**
- **S01, S03, S05.** The publication critical path re-reads and re-derives the whole artifact two or three times, plus a second lowering and a second lexical analysis (H19, H20: repeating the producer is not an independent oracle).
- Historical receipts (persisted-graph plan §9.1, attributed by D) record a seal reconciliation hitting a 10 s native timeout on 2026-10-10. P8 shows `publication_seal` at 41–65 s in 2026-10-10 fixture journeys. These are unattributed.
- **Concurrency argument rejected (with D).** Derived rows written by another publication after `materialize_search` can equally appear after seal. Eligibility at read time owns that guarantee, not a second same-code comparison.

**Corrective contract (Proposed).** For `Admission::Completed`, seal consumes the admitted manifest and the frozen attempt and decides only what can have changed since admission. It produces the same handle. Concretely:
- Premise checks: frozen, admitted, no pending effects, finalization lease held, and the F09 witness.
- Exactly one post-write actual-versus-expected comparison per derived family. This is the comparison `materialize_search` already performs, and it catches extra eligible rows that collision checks cannot.
- The lexical statistics `check` against expected rows already in hand.
- Seal's originals byte rehash replaced by metadata equality against the manifest. The admission invariant has proven the bytes under the same freeze.
- `verify_epoch` once per publication under the lease.
- The full cold path (second lowering, full `Loader::reconcile`, state recompute) kept for `publish(VerifiedExport)`, restore and `inspection::audit`.

**Preservation.**
- An independent cold check for imported or restored content.
- Detection of missing, extra or changed derived rows.
- Byte-level original verification at least once per first publication.
- DP-19's freeze-then-publish ordering.
- The reader-open readback.

**Counterexample.**
- The admitted fence in `ensure_rows_owned` covers canonical tables, not memberships or contributions (D's challenge). A process-local freeze is therefore weaker than a native fence for those tables.
- Until the F09 witness covers attempt-owned memberships, one state recomputation stays at seal.
- The originals readback is dropped only if `artifact_chunk_content` is confirmed to read `original_chunk` under the same freeze. This reviewer confirmed the projection; the freeze ordering is `artifact::admit`'s (freeze before `admit_semantics`).

**Uncertainty.** Each re-derivation's share of seal time is unmeasured.

**Labels.** Implemented; remedy Proposed. Rule impact RC06.

**Closure evidence.** A seal trace with one lowering, one analysis and no state recompute on the ordinary path; seeded missing, extra and changed derived rows still refuse; restore and audit keep the cold path.

**Principal re-read.** `native_publication.rs::{Publication::seal, seal_staging}`; HEAD `derived_search.rs::{materialize_search, reconcile_search}`; `reconciliation.rs::Loader::reconcile` (originals loop, `table_count`); `lctx_model::domain::artifact::content_invariants` and `ArtifactContents::visit`; the `compiler.rs` `ArtifactChunk` body projection.

**Provenance.** D-04, G-05; V10; R17, R22; SA-F03 (second `verify_epoch`).

<a id="f11"></a>
#### F11 — Unchanged content republishes in full; the handle omits the derivation implementation

**Cause.**
- `lctx_publisher::seal_completed` has no check for an existing sealed `publication:<handle>`. The search coverage is the operation map §6 plus D's search of `lctx-publisher/src`, `cpg-core/src/compilation.rs` and `lctx/src/compile.rs`. This reviewer's `rg` over `lctx-publisher/src` and `lctx/src/compile.rs` also found none.
- Content-addressed references, search and lexical rows make `ensure_rows` skip creation, but every round trip and comparison still runs, and `Loader::reconcile` re-reads every original's bytes.
- `native_publication::publication_identity` binds semantic content, realization, view, service generation and definition epoch. The definition epoch includes `fn::lctx_operation_definition`, which carries the serving and model implementation digests. `lctx-serving/build.rs::capture` digests only `lctx-serving/src` and the manifests. So the Rust derivation code in `lctx-surrealdb` is outside the handle: `derived_search::lower`, `codec::entity_references` and `lexical_stats`.

**Consequence.**
- **S03 unchanged, S05.** Publishing pays O(all derived rows + all original bytes) for nothing (H13, H14).
- **Identity scope (DP-04).** A changed lowering over identical content yields the same handle. Today that fails closed: old derived rows appear as "extra" at actual-row reconciliation. But it means a correct short-circuit cannot be built on the current handle, and republishing after a lowering change cannot succeed.

**Corrective contract (Proposed).**
1. Add a derivation-implementation identity to the handle or marker, or to the realization identity.
2. After admission, if `publication:<handle>` exists with identical marker bytes and is not retiring, take a guarded hold on it, close the attempt and return the handle.
3. Optionally skip the originals readback for `(original id, content, incarnation)` already verified under a sealed publication, invalidated on retirement.

**Preservation.**
- Guard against concurrent retirement through the publication guard.
- Pins and selection semantics unchanged.
- A changed derivation implementation yields a new handle.

**Counterexample.** Two attempts racing to publish the same handle must converge on one marker; the guarded hold plus the marker's `ensure_rows` collision check give that.

**Uncertainty.** Byte-identical manifests across attempts are inferred from the canonical bindings, not observed.

**Labels.** Implemented (absence) and Interface-checked (identity scope). Remedy Proposed. Rule impact RC09.

**Closure evidence.** A warm unchanged recompile performs zero derived writes and no original reads; a changed lowering yields a new handle and publishes.

**Principal re-read.** `native_publication.rs::publication_identity`, `seal_staging`; `lctx-serving/src/lib.rs::{native_definitions, operation_definition}`; `lctx-serving/build.rs::capture`.

**Provenance.** D-05; R18.

<a id="f12"></a>
#### F12 — Derived ownership is created per row, re-held per row, then deleted

**Cause.**
- `Loader::insert` holds every derived row under the attempt (`hold_many`).
- `seal_staging` streams every attempt-owned hold and re-holds each root under the publication in 128-row effects (F06 covers the stream).
- `close_attempt` then deletes the attempt holds 128 at a time (`control/cleanup.rs::CleanupHolds`).

**Consequence.** **S01, S03.** Three ownership writes per derived row, each a fenced effect window, plus guard upserts and F01 exposure, all on the critical path (H16, H23, H27).

**Corrective contract (Proposed).** Derived rows are held from the start by one per-publication staging root, which the attempt holds. The marker transaction adds a single `publication → staging root` hold. Cleanup of an unsealed attempt releases the staging root as one object.

**Preservation.**
- Nothing becomes reachable from a marker before the marker commits.
- Retirement reachability is transitive.
- An interrupted publication leaves discoverable cleanup.

**Counterexample.** One high-degree staging root makes retirement traversal fan out. That needs checking against HS-F03's paging, which bounds candidates rather than the whole effect.

**Uncertainty.** Whether `ORDER BY id` with `owner_holds` sorts server-side (D's P-D3 needs `EXPLAIN`).

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Seal's ownership writes are O(1) in derived rows; retire/publish and crash-before-marker controls pass.

**Principal re-read.** `native_publication.rs::seal_staging`, `control/cleanup.rs` (hold deletion SQL), `loader.rs::Loader::insert`.

**Provenance.** D-06; R19.

<a id="f13"></a>
#### F13 — Unadmitted attempts' completed work has neither a reuse nor a reclamation route

**Cause.** One lifecycle gap with two obligations, which must be designed together because their naive remedies conflict.
- **(a) Reuse.** `NativeCompilerStore::attach_retained_product` requires `contribution.attempt.admitted=true`, and cleanup of an unadmitted attempt deletes its `native_product` rows (`control/cleanup.rs::CleanupHolds`). So a compile cancelled or crashed before admission (S07) replays everything. ADR-0145 states the opposite: "Recovery retains compatible completed prerequisites and re-executes missing work".
- **(b) Reclamation.**
  - `close_attempt` and cleanup delete holds and products only.
  - Production retirement starts only from publication roots: `lctx_publisher::backup::retire` calls `retire_reachable`. Search coverage: `rg` over `crates` for `retire(` callers; the only production caller is `lctx/src/newnative.rs::retire`.
  - So rows written by attempts that never publish (failed, abandoned, `--artifact-only`) become unheld and unreclaimable. Safeguard 5 rightly forbids deleting them by age.

**Consequence.**
- **S07.** Recovery replays the whole attempt (H24).
- **S08, S10.** The store grows with failed and abandoned attempts, including validation-database churn (H25, H27).

**Corrective contract (Proposed).**
- An unadmitted terminal attempt's *completed* contributions move under a recovery hold with a declared lifetime and stay attachable under the same `spec` and `request` equality.
- When the hold ends, or for content that no completed contribution owns, cleanup registers the released objects as `retire_reachable` roots. That traversal re-checks holds and incarnations under guards, so shared surviving content is retained.
- Semantic admission still covers every view of the new attempt.

**Preservation.** Shared and pinned content survives; cleanup interruption stays resumable; reuse never trusts an outcome more than admitted reuse does.

**Counterexample.**
- A contribution from an attempt that failed admission on an invariant fails again with identical inputs: wasted but sound.
- A poisoned contribution (written after `fail()`) must not be attachable. Only contributions with `completed=true` and an outcome qualify, and this assumption needs the ADR-0145 owner's confirmation.

**Uncertainty.** Whether "admitted-only" encodes an undocumented poisoning premise.

**Labels.** Implemented; remedy Proposed. Rule impact RC03.

**Closure evidence.** Interrupt after normalization, rerun, and the completed stages attach. After an abandoned attempt, rows only it held are retired and shared rows remain.

**Principal re-read.** `compiler.rs::attach_retained_product` lookup SQL; `control/cleanup.rs`; `control/retirement.rs::retire_reachable` callers.

**Provenance.** D-09, D-10; R20, R21.

<a id="f14"></a>
#### F14 — Original bytes are stored per revision-scoped identity

**Cause.** `lctx_model::domain::source::SourceArtifact` keys on `(input: Id<InputRevision>, path, content)`, and `InputRevision.manifest` digests the whole closure. `original` and `original_chunk` ids derive from `EntityId::of(SourceArtifact id)` (`compiler.rs::persist_original_chunk`).

**Consequence.**
- **S03 patch release.** One changed file re-keys every artifact, so all environment bytes (about 58 MB in inquiry A's measurement of the legacy environment) are re-stored, re-held and re-read at seal.
- **S02, S08.** Shared dependencies are stored once per library per retained revision (H15, H16, H27).

**Corrective contract (Proposed).** Keep the semantic `SourceArtifact` and per-artifact `original` header. Store chunk bytes by content (`original_blob:<content>`, chunked) and reference them from the header.

**Preservation.**
- Evidence retrieval by artifact.
- Seal readback against `(content, byte_len)`.
- Retirement only when no holder remains.

**Counterexample.** A shared blob retired while another revision references it. Hold-governed retirement prevents that, and it needs a retirement control with a shared blob.

**Labels.** Implemented; remedy Proposed. Rule impact RC16.

**Closure evidence.** A patch-upgrade trace writes only changed blobs; a shared-blob retirement control.

**Principal re-read.** `source.rs::SourceArtifact` keys; `compiler.rs::persist_original_chunk`.

**Provenance.** A-07.

<a id="f15"></a>
#### F15 — Admission validity is carried only within one workspace

**Cause.**
- `cpg_core::workspace::Workspace::validation_premise` keys each invariant result on the model digest, `Invariant::digest`, the profile, the selectors and exact view and snapshot identities. None of these is attempt-scoped.
- The result is held in memory in `checked_premises`, so a new compile re-runs every invariant even when the view identities are equal.

**Consequence.** **S03 unchanged.** Full invariant admission repeats (H12, H19). NE-F02-era receipts of invariant admission were near 900 s; that figure is historical and not re-measured.

**Corrective contract (Proposed).** Durable checked-premise records owned by the admitted attempt, keyed as now plus `implementation_digest()` and the compiler source digest. `Invariant::digest` covers name, purpose and inputs, not code. A new attempt consumes a record only when every key part is equal.

**Preservation.** Independent cold admission at import, restore and audit.

**Counterexample.** An invariant that reads outside its declared inputs would be carried wrongly. The in-workspace memo already relies on the same premise.

**Uncertainty.** Value once F16 and F17 make warm recompiles cheap elsewhere. This is lower priority than they are.

**Labels.** Implemented (memo scope); remedy Proposed. Rule impact RC17. If that rule is kept, this finding is withdrawn.

**Closure evidence.** An unchanged recompile shows zero invariant re-execution with a matching key, and a changed implementation forces re-execution.

**Principal re-read.** `workspace.rs::validation_premise`.

**Provenance.** G-06.

### 7.3 Reuse and invalidation

<a id="f16"></a>
#### F16 — Facts providers have no reuse unit

**Cause.**
- `cpg_core::facts::compile_facts` runs every offered provider unconditionally.
- `rg 'reuse|attach_retained|retain_product|product_request'` over `facts.rs`, `cpg-extract/src/{bundle,pyrefly_stage}.rs` returns nothing; tests were not searched.
- `UpperStage::reuse` returns `Fresh("external effects retain their existing owner")` for every non-`Pure` effect (`compilation.rs`).
- Yet every result-affecting input of a facts producer is known before it runs and already sits in `ContributionSpec`:
  - the captured binding (`InputRevision.manifest` over every closure byte);
  - provider implementation (`bundle::build_digest` over the source closure, `Cargo.lock` and fork patches);
  - configuration;
  - profile;
  - model.
- `refuse_ambient` plus explicit configuration (§B8) remove ambient inputs.
- L2 settled that no library-owned cross-process incrementality exists at the pins (R2): Pyrefly's `State` cannot be serialized, and ty's queries are not `persist`-annotated.

**Consequence.**
- **S03 unchanged, or a changed analysis, rendering or embedding policy.** Pyrefly analysis, all emission, all writes (F03, F04) and completion repeat. §14.10's "Association policy with unchanged facts" row presumes facts are reused.
- **S04, S07.** The same applies (H12, H13, H14).
- The P4 figure cited by inquiry C was withdrawn (V03) and carries no weight here.

**Corrective contract (Proposed).**
- Each provider gets a `ProductRequest` keyed on its existing spec.
- An `Extraction` or `Acquisition` stage whose only inputs are the frozen capture becomes eligible for checked attachment of an admitted contribution with an equal spec.
- The proof of "unchanged" is then the capture digest: one closure pass, not a full analysis.
- Refinements:
  - one contribution per (provider, input), so a corpus-only change keeps the installed-input analysis;
  - a hash-only capture on a full facts hit.

**Preservation.**
- CI-10 hermeticity.
- Admitted-only, or F13's recovery rule.
- Coverage receipts naming providers.
- Current attribution: the attempt binds the attached contribution.

**Counterexample.** A provider whose output depends on thread scheduling would reuse rows a fresh run might emit differently. That is acceptable only if outputs are set-deterministic, which is assumed and not established for Pyrefly's AllThreads run (L2 probe 3). A replay control must test it.

**Uncertainty.**
- The share of extraction in a compile.
- Whether `captured_binding` covers the native context configuration (a `contracts::composition` check).

**Labels.** Implemented (absence, with the search scope above); remedy Proposed. Rule impact RC02.

**Prerequisite.** F17. Without attach-by-reference, a facts hit copies the largest relations' memberships row by row.

**Closure evidence.** A same-binary recompile attaches providers without running analyzers; a changed captured byte, configuration or build misses; an unadmitted prior attempt is never attached (unless F13).

**Principal re-read.** `facts.rs::compile_facts`; `compilation.rs::UpperStage::reuse`; `workspace_products.rs::product_request`; `completed.rs::ContributionSpec::identity`.

**Provenance.** A-04, C-01; V11; R2.

<a id="f17"></a>
#### F17 — Reuse hits copy memberships, mint per-attempt identities and re-index existing views

**Cause.** One cause: retained content is attached by copying, not by reference. It has three obligations.
- **(a) Copy on hit.** `NativeCompilerStore::attach_retained_product`:
  - mints `attempt_contribution(attempt, spec)`;
  - copies the descriptor (`guarded_effect`);
  - streams every source `compiler_membership` row and rewrites it under the new contribution (`retain_attached_members`: `ensure_rows_owned` plus `hold_many`, two holds per member);
  - completes it;
  - runs `registered_or_install_views`.

  For checked stages, the attach (with its writes) happens before the receiver, event and binding predicates run, so a rejection fails the attempt instead of falling through.
- **(b) Accumulation.** Each attach adds a `view → contribution` hold that attempt cleanup never releases, and a `native_product:{request}_{attempt}` row. Storage and the lookup set therefore grow with attempts rather than with distinct content.
- **(c) View re-indexing.** `registered_or_install_views` always runs `prepare_memberships` plus `ensure` and `hold` for every `compiler_view_member`, even when `compiler_view:<identity>` exists. That is crash safety, because members are written after the view row.

**Consequence.**
- **S03.** A hit costs O(product rows) of fenced writes, close to a miss's write cost. Proving "unchanged" repeats the work avoided (H14).
- **S08.** Each admitted recompile of unchanged content permanently adds a contribution, a membership copy and holds (H25, H27). Inquiry D's HS6 inventory receipt (2,632,341 `native_hold` rows over 337 attempts) is suggestive only.

**Corrective contract (Proposed).**
- (a) Attach by reference. Map the logical contributor to the existing completed physical contribution (`remember_contributor(logical, source)` already does this). Keep `hold(attempt → source)`. Make attempt-scoped enumeration follow `attempt → contribution` holds. Run checked predicates on `scan_contribution_batches(source)` *before* attaching.
- (b) Key product rows by request plus logical contributor, deduplicated.
- (c) Commit a `compiler_view_installed` marker in the final member window after the cardinality check, so later registrants skip enumeration and only add ownership. A partial install is resumed by the next registrant.

**Preservation.**
- Pinned readers and export closure include referenced contributions.
- Retirement stays hold-governed.
- View identity is unchanged.
- Current attribution is carried by the attempt binding.

**Counterexample.** Readers, audits or export that enumerate attempt-owned membership rows (`portable_state_rows`, `state_owners`) would miss referenced contributions. Their enumeration must follow holds, which is the uncertainty to settle first.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** After N unchanged recompiles, contributions, memberships and `view → contribution` holds for a view stay constant; a checked rejection falls through to fresh production; attribution and export controls pass.

**Principal re-read.** `compiler.rs::{attach_retained_product, retain_attached_members, registered_or_install_views}`.

**Provenance.** C-04, D-07, D-08; R16; unified-persistent F04 (residual).

<a id="f18"></a>
#### F18 — Reuse keys chain provenance and couple sibling outputs; there is no value cutoff

**Cause.**
- `ProducerOutput::product_request` keys inputs as `DependencyKind::ExactView` with `source.snapshot.identity()`.
- `CompletedView::new` hashes relation plus contributor identities.
- `CompletedContribution::identity` hashes `spec.identity()` (which contains input snapshot identities), the outcome, and **every** output's `(rows, content)`.
- So:
  - a view changes when any upstream spec changes, even with identical rows (no cutoff);
  - a view changes when a *sibling* output of the same contribution changes. `analysis_prepare::configuration` writes ModelCatalog, AuthoredModel, MethodParameters, AnalysisDefinition, ProjectionDefinition, AnalyticsConfiguration and RetrievalDefinition in one contribution.
- `compilation/reuse.rs::DependencyGraph` documents a pure-value cutoff, but its `affected` result feeds only a debug count (`compilation.rs`).

**Consequence.**
- **S03 changed policy.** Toggling one optional technique or retrieval definition misses every stage that reads any configuration relation, contradicting §14.10's per-change rows.
- **S03 patch.** Every downstream key moves (H13; DP-09 qualitative over-invalidation).

**Corrective contract (Proposed).**
1. Per-relation dependency tokens derived from that relation's own `OutputContent` plus declared stage parameters. This removes sibling coupling.
2. For consumers whose outputs do not embed input provenance, content-keyed dependencies, with a hit attaching under a refreshed spec that records current inputs (after F17).
3. Either make `DependencyGraph` the cutoff owner or remove it.

**Preservation.** The content digest covers additions and removals. Evidence-only or moved-span changes still miss, because spans are content. Admission still covers new views.

**Counterexample.** A stage that reads view metadata (counts, producer identity) rather than rows would hit wrongly under content keys, so it must declare `ExactView`. Stages embedding `SourceSnapshot` receipts (`analytic.rs::produce`, `local_semantics::run`) stay exact-view unless receipts cite content identity. That is a model decision (RC04).

**Uncertainty.** Which upper stages read which configuration relation (not traced).

**Labels.** Implemented; remedy Proposed. Rule impact RC04.

**Closure evidence.** A changed `AnalyticsConfiguration` with techniques off hits CatalogCore and Retrieval; a recomputed stage with identical rows lets consumers hit; a changed span still misses.

**Principal re-read.** `completed.rs::{CompletedView::new, CompletedContribution::identity, ContributionSpec::identity}`; `workspace_products.rs::product_request`; `compilation.rs` `products.affected` use.

**Provenance.** C-02.

<a id="f19"></a>
#### F19 — The profile is keyed into every contribution

**Cause.**
- `ProducerOutput::product_request` sets `profile`, and the spec identity encodes it.
- Per-root requests in `kernel_products` inherit it.
- Some declarations are profile-independent (`entity_normalization::stage()`; the `normalize::entities` body does not read the profile).

**Consequence.** **S04.** A behavioral compile over an existing catalog compile repeats the entire catalog work: extraction, normalization and the catalog stages (DP-09 over-keying, H13).

**Corrective contract (Proposed).** Key on profile only where the declaration or code is profile-dependent, using the model's existing `stage(profile)` versus `stage()` distinction. Profile identity stays explicit on the attempt, the manifest and the publication.

**Preservation.** Profile-specific stages keep the profile (Local's `Catalog → NotRequested`). An attached catalog contribution never asserts a behavioral outcome.

**Counterexample.** If behavioral adds relations to a shared contribution, sibling coupling (F18 step 1) still blocks reuse. That makes F18 step 1 a prerequisite.

**Labels.** Implemented; remedy Proposed. Rule impact RC05.

**Closure evidence.** A behavioral compile after a catalog compile hits the profile-independent normalization stages.

**Principal re-read.** `workspace_products.rs::product_request` (`profile` field).

**Provenance.** C-03.

<a id="f20"></a>
#### F20 — The per-root portable cache is crossed per root on every compile and amplifies its own bytes and accounting

**Cause.** One owner (`lctx_surrealdb::product_cache::NativeProductCache`), attached by default, realized as a second byte store with its own lifecycle. It has three obligations.
- **(a) Per-root crossings.**
  - The installed runtime config always carries `reuse` (`scripts/surrealdb_service.py`), so `compile` always attaches the cache.
  - In `normalize/mod.rs`, each selected root awaits `kernel_products::{entity,callable,aspect}_hit`, which calls `NativeProductCache::lookup`: flocks plus two RTs.
  - Each miss awaits `retain_*`, which calls `insert`. Inquiry C reckons about 13 RTs, roughly 8 flocks and 2 fsyncs per miss; this reviewer relies on that count.
  - Keys include the whole compiler source digest and model digest, so every code edit makes every entry a miss.
- **(b) Two accounting authorities.**
  - `reserve_stage` runs `SELECT VALUE math::sum(bytes)` over `product_entry` and `product_stage` on every insert, under the exclusive lock.
  - The fenced `product_quota:current.used` counter carries the same total.
  - Entries from superseded keys are evicted only by capacity.
- **(c) Byte expansion.** `compilation_product::ProductSection { bytes: Vec<u8> }` has no bytes adapter, so `serde_json` writes integer arrays (P7: ×3.31 on one sample). The hit path then decodes through JSON → `Value` → Arrow → `hash_rows`, whose digest is discarded, → `Value` → typed rows.

**Consequence.**
- **S01, S02.** Serialized RTs, flocks and fsyncs per root on every cold compile. Thousands of callables in the pilot alone are multiplied by three callable kernels.
- **S10.** Pure cost, because code changes miss everything.
- **S08.** Each insert costs O(retained entries).
- Mechanisms: H8, H14, H16, H25; DP-10; G1 smell in (b).

**Corrective contract (Proposed).**
- (a) Batch per root window: one lease, one `SELECT * FROM $ids`, one staged multi-product insert. Or defer insertion to a bounded owned writer off the critical path. Or attach only when a prior generation exists.
- (b) Read `product_quota.used` and index `created` for eviction.
- (c) Encode sections as one opaque byte string and keep the one typed canonical check.
- A pointer-based per-root product (domain token → retained memberships, protected by holds, after F17) would unify this layer with whole-stage reuse. That route is plausible but unsettled.

**Preservation.**
- Reuse-contract F01's fences and acknowledgement semantics.
- Complete-domain keys with Present, Absent and Virtual outcomes. This is the only value cutoff in the system, so keep it.
- Exact readback before visibility.
- Resource-refusal fall-through.

**Counterexample.** Deferring insertion loses entries on crash. That is acceptable, because insertion is optional acceleration and the whole-stage layer is unaffected.

**Uncertainty.** Root counts; round-trip latency.

**Labels.** Implemented; remedy Proposed. Rule impact RC19 (conditional).

**Closure evidence.** Per-window lookup and insert counts independent of root count; insert cost independent of entry count; stored size close to canonical body bytes; F04 (reuse-contract) controls pass.

**Principal re-read.** `scripts/surrealdb_service.py` `reuse` config; `product_cache.rs::reserve_stage` and the quota SQL; `compilation_product.rs::ProductSection`; `normalize/mod.rs` hit and retain call sites.

**Provenance.** C-05, C-06, C-07; V09; P7.

### 7.4 Compiler read path and stage units

<a id="f21"></a>
#### F21 — Key-only demand is served by payload windows over the whole membership run

**Cause.** The verified membership run is cached per view (`prepare_memberships`), but it is not used as an access path for key-only questions.
- **(a) `id` predicates.**
  - `compiler_provider::SupportedFilter::driver` yields a driver only for `atomic_scope_field`, and `id` is not one, so `id IN (…)` lowers to `NativePredicate::Sql` (V04).
  - `NativeCompilerStore::scan_bound_rows_inner` treats `Sql` as non-selective: `SelectionKeys::Verified` over the whole cached run, then `SelectedRows::start_payload` issues one payload query per 128 candidates of the entire view, filtering server-side.
  - Production callers:
    - `catalog_core::produce` builds `id IN (32 literals)` per grain through `PreparedEdges::grain`;
    - `consumed_rows::PreparedEdges::batch_inner` runs `… WHERE id IN (…)` for every Physical-root batch of the normalization callable loops;
    - `scoped_inventory` runs `grain` per root through `scoped_admission::root_predicate`.
- **(b) id-only projections.** `start_payload` always queries `$__compiler_nodes` even when only `semantic_key AS id` is projected and the run already holds verified keys. This hits:
  - every root enumeration;
  - every `validate_scope` emptiness probe;
  - every reference-join target side.

**Consequence.**
- **S01, S02.** Seeding or presence-checking g grains over N roots costs about g·⌈N/128⌉ RTs and g·N node reads. With g = N/32 that is **quadratic in root count**: a CI "bounded answer, amplified work" case (H5, H7, H8, H10; FP-07; CI-08; A4).
- (b) adds about N/128 RTs per enumeration of keys already local.
- DataFusion's coercion and unwrap of `X'…'` to `FixedSizeBinary` relies on B's reading of the pinned `datafusion-*-55.1.0` source. Either way the work is O(N) per grain: if the filter were not pushed, the full id column would transfer instead.

**Corrective contract (Proposed).**
- (a) `NativeTable::scan` recognizes a mandatory `id = lit` / `id IN (lits)` conjunct as an explicit-key driver (`NativePredicate::KeysSql` with the residual), mirroring the atomic-field rule (OR supplies no driver). Presence-only demand merge-intersects sorted requested keys with the cached run, as `row_tokens_inner` already does.
- (b) Unpredicated id-only projections stream from the verified run without a payload query.

**Preservation.**
- Exact present and absent outcomes, including virtual roots.
- Sorted distinct keys.
- Foreign and duplicate backing detection.
- Empty demand still checks the exact registered view.
- Budget charges.

**Counterexample.**
- `id IN (…) OR other` must remain residual.
- The payload query also re-confirms each node's `semantic_type`. If any closure relies on that per read (persisted-graph F01's backing fidelity), keep it at completion or seal rather than on every read.
- Multi-owner views degrade to per-key lookups (F22), so (a) must land with F22's remedy, or use the run merge.

**Uncertainty.** Row counts; whether a residual `CAST` survives in some forms (pinned source read, not executed).

**Labels.** Diagnosis Implemented; coercion Interface-checked (relies on B). Remedy Proposed.

**Closure evidence.**
- A planning-only control: `WHERE id IN (X'…')` yields an explicit-key partition.
- A native control: payload RTs scale with demand, not with view size.
- Absent and virtual-root controls pass.

**Principal re-read.** `compiler_provider.rs::SupportedFilter::{interpret, driver}`; `compiler.rs::{scan_bound_rows_inner, SelectedRows::start_payload}`; `catalog_core.rs` grain loop; `consumed_rows.rs::{batch_inner, grain, grain_roots}`; `scoped_admission.rs::root_predicate`.

**Provenance.** B-01, B-07; V04; R6.

<a id="f22"></a>
#### F22 — Multi-owner explicit-key lookup runs per key and scans retained history

**Cause.**
- `compiler.rs::MembershipLookup` uses deterministic point IDs only when `owners.len() == 1`. Otherwise it issues `MEMBERSHIP_KEY_QUERY` (`… WITH INDEX member_keys WHERE relation=$relation AND semantic_key=$key`) once per key.
- `member_keys` is `(relation, semantic_key, contribution)`, so each probe returns every contribution in the store that ever held the key, across attempts, libraries and releases, and `decode_membership(…, allow_foreign)` discards the foreign ones client-side.
- Vocabulary relations are multi-owner by construction.

**Consequence.**
- **S02, S08.** Per-key round trips, plus examined rows that grow with retained history. Ordinary work becomes coupled to history (H8, H10, H25; A4).

**Remedy: R5 stays unresolved.**
- **L1's alternative.** A multi-get of the deterministic `membership_id(owner, relation, key)` over owners × keys. That is the Cartesian expansion the native-execution-efficiency review's F01 rejected, because work falls on absent combinations when owners are many and matches sparse. That review also noted that SurrealDB 3.3's planner expands compound `IN` unions as Cartesian equality values.
- **B's alternatives.**
  - (a) Merge against the cached, owner-filtered membership run when demand is a large fraction of the view.
  - (b) Owner-restricted batched probes (`… AND contribution IN $owners`) within one multi-statement request.
- **Deciding premise:** the owner count of multi-owner views on the catalog path. In facts that is at most three or four contributors (F23), which would make a bounded Cartesian cheap and history-independent. Upper-stage views were not counted. The other deciding fact is the actual selected plan for (b), which needs `EXPLAIN`.
- This reviewer's position:
  - the retained-history coupling is a defect regardless of remedy;
  - (a) is preferred where the run exists;
  - the choice between L1's multi-get and B's (b) for sparse demand stays open until the owner-count inventory and plan are in hand.

**Preservation.**
- No unbounded owner × key enumeration.
- Exact collision detection (`selected nominal backing collision`).
- Independent physical-ID checks.

**Labels.** Implemented; remedy Proposed (alternatives unresolved).

**Closure evidence.**
- A count of multi-owner views in a representative attempt.
- A control in which a key present in an unrelated retained attempt does not increase examined rows.

**Principal re-read.** `compiler.rs::{MembershipLookup, MEMBERSHIP_KEY_QUERY}`, the `member_keys` DDL; the native-execution-efficiency review F01 text.

**Provenance.** B-02, L1-05; R5.

<a id="f23"></a>
#### F23 — Each contributing provider re-forms and re-installs union views of every vocabulary relation

**Cause.**
- `Workspace::producer` sets `allowed = outputs ∪ contributes`.
- `ProducerOutput::complete` pads pending outputs with every allowed relation, even with zero rows written.
- `complete_contribution_inner` forms, for every output, `CompletedView::new(relation, previous ∪ {logical}, count)` and calls `registered_or_install_views`, which runs `prepare_memberships` over the union and writes member rows and holds.
- Under catalog, Pyrefly, Documents and Deployment all declare `contributes: assembly::vocabulary()` (TyFlow does too, under behavioral).
- No facts provider consumes a vocabulary relation, and normalization contributes nothing (V08). So the intermediate unions have no consumer before `freeze_completed_inputs`, which already binds epoch-shared relations once.

**Consequence.**
- **S01.** Vocabulary membership is materialized about (1 + later contributors) times.
- **S02, S09.** Each added contributor or vocabulary kind multiplies it (H2, H6, H13).

**Corrective contract (Proposed).** A contribution forms no new view for a relation it wrote no rows to. More generally, epoch-shared contributions register at completion, and the union view is installed and bound once, at the consuming boundary.

**Preservation.** Exact contributor sets and counts in the final view; conflict detection across contributors; completion atomicity per producer; final views identical to today's.

**Counterexample.** A future facts-phase consumer of a vocabulary relation would need its union at its own boundary. Demand-driven installation covers that.

**Uncertainty.** Vocabulary row counts.

**Labels.** Implemented (relies on V08 and A for the absence of facts-phase consumers); remedy Proposed.

**Closure evidence.** Each vocabulary relation is installed once per compile; final views and counts are equal to today's.

**Principal re-read.** `workspace.rs::{producer, ProducerOutput::complete}`; `compiler.rs::{complete_contribution_inner, registered_or_install_views}`; the `contributes:` declarations in `cpg-extract/src`.

**Provenance.** A-02; V08.

<a id="f24"></a>
#### F24 — Unrequested behavioural methods run as full stage units and materialize per-scope `NotRequested` coverage

**Cause.**
- `compile_driver` runs all 21 upper bindings under catalog. Local, Base, Completion, SourceCalls, Enriched, Models and Summary run as full stage units ending in `NotRequested`.
- `local_semantics::run` builds `CoverageAdmission`, then for every expected scope pushes the `CoverageRequirement`, required rows, observation sources, an assessed coverage row and premises. Only then does it push the invocation and an `AnalysisOutcome { status: NotRequested }`.
- Scopes are per Python source artifact × method × frame (`analysis::expected::contract`).
- These rows go through native writes, completion, retention, their own freeze boundaries, admission (`final_coverage`, `admit_frontier`, `semantic_outcomes`), the completed-state walk (F09) and publication.
- Serving derives availability from invocation outcomes only (`lctx_serving::behavior`, per B).

**Consequence.** **S01, S02.** Catalog compile and admission work grows with Python artifacts × seven methods, for answers that are uniformly "not requested" (H2, H6, H16; A4). P8's historical receipts show the nine unrequested phases at 32–33 s against 33–81 s for the five catalog product stages. They are unattributed.

**Corrective contract (Proposed).**
- `NotRequested` becomes an invocation-level, domain-total outcome: the invocation plus one outcome per method and frame, with no per-scope coverage rows.
- The coverage-frontier invariant derives scope status from the invocation for the method's declared contract.
- Physically, all unrequested methods' invocations and outcomes are emitted from one contribution.

**Preservation.**
- Explicit per-method, per-frame `NotRequested` (CI-04).
- Frontier totality.
- S04 never reuses `NotRequested` products, because the profile stays in profile-dependent keys (F19).
- Model-owned semantics: the invariant changes in `lctx-model`.

**Counterexample.** A partial request (some scopes requested) is `Partial` with per-scope rows, so the distinction survives.

**Uncertainty.** Whether `lctx-eval` or inspection reads per-scope `NotRequested` coverage (not checked by B or by this reviewer).

**Labels.** Implemented; remedy Proposed. Rule impact RC08.

**Closure evidence.** Unrequested methods emit only invocations and outcomes; frontier and serving controls still report `NotRequested`; a behavioral compile is unchanged.

**Principal re-read.** `local_semantics.rs::run` (status selection and the per-scope push loop).

**Provenance.** B-03; R7; P8.

<a id="f25"></a>
#### F25 — The expected coverage domain is rebuilt by every analysis owner

**Cause.**
- `CoverageAdmission::new` is constructed independently across `cpg-core/src`: in `local_semantics`, `semantic_execution` (×5), `semantic_models` (×2), `semantic_summaries` (×2), `structural`, `analytic`, `analytic_embedding`, `catalog_core`, `catalog_evidence` (×3), `catalog_selection`, `synthesis` and `retrieval`. This reviewer's `rg -c` gives about 20 construction sites.
- Each streams `analysis::expected::inputs(method)` with `SELECT *` from immutable post-normalization relations.

**Consequence.** **S01, S02.** The same domain is read, decoded and indexed per owner, linear in artifacts and scopes each time (H12, H19; DP-10).

**Corrective contract (Proposed).** One immutable `FrontierIndex` per exact expected-domain view identities and profile, prepared after normalized admission and lent to stages. Per-method contracts stay per call.

**Preservation.**
- `CapturedSources::accepts(permit)` binds to the identities the shared index was built from; mismatches rebuild.
- The publication callback's independent recomputation (`coverage::admit`) remains at admission.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** One index build per compile; coverage-admission controls pass.

**Principal re-read.** The `rg -c` construction-site count; `local_semantics.rs` admission construction.

**Provenance.** B-04; R7.

### 7.5 Providers and extraction

<a id="f26"></a>
#### F26 — Per-module canonical work is repeated inside the Pyrefly provider

**Cause.** One provider loop (`cpg_extract::pyrefly_stage::session`) repeats canonical work per module, with two obligations.
- **(a) Three Ruff Checker traversals per root.**
  - `diagnostic_records` calls `ruff_linter::linter::check_path` twice: `check(Noqa::Enabled)` and `check(Noqa::Disabled)`.
  - `ruff_context::CanonicalSyntax` then calls `observe_parsed` → `check_ast_with_observation` a third time with an empty rule table.
  - In the fork, `check_ast` is `check_ast_with_observation(…, None)`, and `check_noqa` already computes the ignored indices. This relies on L2's reading of the fork `9080ee7`.
- **(b) Re-parses in `types`.**
  - `pyrefly_stage::types` creates `let mut cached = BTreeMap::new()` inside each per-root call.
  - Its `field_occurrence` closure linearly scans `roots` with a `PathBuf` join per comparison.
  - On a miss it runs `CanonicalSyntax::parse` plus `typed_syntax::emit`.
  - The record-field path (`type_records.rs`) has no own-module shortcut, per L2.
  - So every root with class fields is parsed at least twice, and ancestor modules once per inheriting root.

**Consequence.**
- **S01–S03.** Three semantic-model builds where one suffices. Repeated parses scale with root count × inheritance fan-in: 166 of 257 fastmcp modules contain classes (L2's count).
- Mechanisms: H2, H10, H12, H28.

**Corrective contract (Proposed).**
- (a) One fork entry point that runs the three rules with the observation sink and returns the unsuppressed diagnostics plus `check_noqa`'s ignored set, so that emitted = unsuppressed − ignored. This is a fork revision bump (ADR-0117/0118 already govern it).
- (b) Reuse in-scope `spans` for the own module; a provider-session `Spans` cache keyed by artifact id, charged; and a path → artifact map built once.

**Preservation.**
- Observation rows independent of enabled rules.
- Separate `RuffNoqaSuppressed` and `Emitted` channels.
- Identical `Spans`.
- Supplier digests change with the fork.

**Counterexample.** A Checker path whose deferred visits depend on enabled rules would change the observation stream. A parity control over the Ruff fixture corpus settles it.

**Uncertainty.** The share of these costs in the provider phase.

**Labels.** Implemented; (a) relies on L2 for fork internals. Remedy Proposed.

**Closure evidence.** Byte-identical observation and diagnostic channels between the three-pass and one-pass forms, including a case with two identical diagnostics; one canonical parse per root (debug counter).

**Principal re-read.** `diagnostic_records.rs` (`check_path` ×2); `ruff_context.rs::observe_parsed` call; `pyrefly_stage.rs::types` (cache scope and linear `roots` scan).

**Provenance.** L2-01, L2-02; V05; R13.

<a id="f27"></a>
#### F27 — Dependency-context analysis waves bypass the shared analysis pool

**Cause.**
- The root run passes `Some(analysis_pool())` (`pyrefly_stage.rs`).
- The dependency-context loop calls `transaction.run(&dependencies, Require::Everything, None)`.
- The pinned fork's `Transaction::run_step` falls back to `custom_thread_pool.unwrap_or(&self.data.state.threads)`, and the `State` was created with `ThreadCount::Inline`.
- So each wave runs on the provider thread while the 32 pool workers idle.

**Consequence.** **S01, S02 heavy dependency context.** Waves that upgrade dependency modules to `Everything` run serially (H9, H21, H22).

**Corrective contract (Proposed).** Pass `Some(analysis_pool())`. The pool already joins before extraction inspects answers.

**Preservation.**
- Deterministic outputs. The root run already relies on thread-count neutrality.
- Stack headroom: pool threads have 10 MiB against 512 MiB on the provider thread. Pyrefly's expression-depth guard bounds this, per L2.

**Counterexample.** Inline may have been kept deliberately for stack depth. No comment or commit says so (L2).

**Labels.** Implemented and Interface-checked (pinned fork); remedy Proposed.

**Closure evidence.** Dependency-context controls unchanged; facts digests equal across `NumThreads(1)` and `AllThreads`.

**Principal re-read.** `pyrefly_stage.rs` (`State::new(…, ThreadCount::Inline)`, both `transaction.run` sites); pinned `pyrefly/lib/state/state.rs` fallback lines.

**Provenance.** L2-03; V05.

<a id="f28"></a>
#### F28 — Artifact-to-use lookups in provider loops are quadratic

**Cause.**
- `cpg_extract::acquisition::AcquiredInput::uses` runs, per artifact, `derivations().find(…)` and `library.files.iter().filter(|f| f.path == artifact.path)`.
- `uses()` is called from `Acquire::run`, `pyrefly_stage` (`analysis_roots`), `document_parser` and `deployment` (twice).
- Per A, `deployment.rs` and `pyrefly_stage::session` add further linear finds per artifact or root.

**Consequence.** **S02.** Quadratic growth in closure size, about 67M comparisons per call at 8.2k files (A's arithmetic, unmeasured) (H9).

**Corrective contract (Proposed).** Build path and id maps once per input, as `admission::analysis_roots` already does with `by_id`, and memoize `uses()`.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** No per-artifact linear scan in these loops.

**Principal re-read.** `acquisition.rs::AcquiredInput::uses` and its call sites.

**Provenance.** A-10.

<a id="f29"></a>
#### F29 — Capture verification sits where no uncertainty enters

**Cause.**
- `facts::compile_facts` calls `input.captured().verify()` before the providers. This is a re-read of a `CapturedInput` built moments earlier by `capture_with`, which digested while copying into a private tempdir.
- `Acquire::run` calls `frozen.verify()` immediately after `emit_chunks`, whose `ArtifactCapture::finish` already verified.
- `capture_receipts` re-reads each file for SHA-256 after `copy_digest` streamed the same bytes.
- Pyrefly and ty then read the frozen tree, and no verification follows them. The documented placement ("after a provider finishes, before its rows can publish") matches neither call site.

**Consequence.** **S01, S03.** Two or three redundant full closure passes (H18–H20). The byte cost at pilot scale is small (P1 measured SHA-256 throughput on another file). The defect is assurance placement.

**Corrective contract (Proposed).**
- Compute SHA-256 inside `copy_digest` for RECORD-listed files.
- Drop the two redundant `verify()` calls.
- If analyzer-read integrity matters, run one verification after the last tree-reading provider.

**Preservation.** The RECORD check; refusal of tampered frozen content (`tests/capture.rs` exercises `verify()`); point-of-use checks in `CanonicalSyntax::parse` and `ty_flow`.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** One digest pass at capture, plus one bracket after analysis if chosen; tamper controls still refuse.

**Principal re-read.** `facts.rs::compile_facts`; `acquisition.rs` `frozen.verify()` site.

**Provenance.** A-09, G-08; R24.

<a id="f30"></a>
#### F30 — Corpus revision attribution checks only `HEAD`

**Cause.**
- `scripts/storage_acquisition.py::check_source` checks only `git rev-parse HEAD == source["commit"]`.
- The worktree may differ from the commit, and capture then attributes those bytes to `InputOrigin::Corpus{revision: commit}`.
- Content identity stays exact through `InputRevision`.

**Consequence.** A provenance label can overstate what was verified: a commit is named for bytes that are not that commit's (CI-01). This is not an efficiency defect; it was encountered in scope and is reported because focus does not hide it.

**Corrective contract (Proposed).** Verify a clean tree (`git status --porcelain` or a tree-hash comparison) before reuse, or record the origin as "commit plus local modifications".

**Labels.** Implemented.

**Closure evidence.** A modified corpus tree refuses or is attributed as modified.

**Principal re-read.** `storage_acquisition.py::check_source`.

**Provenance.** A §4 note, promoted by this review.

<a id="f31"></a>
#### F31 — Flow conditions are tree-copied and lowered through fresh variable sets

**Cause.**
- `lctx_model::domain::conditions::graph::CondGraph::join` clones both operands' nodes on every `and`/`or`, which turns DAGs into trees capped at `MAX_GRAPH_NODES = 4096`.
- `Diagram::from_graph` lowers each node with a fresh `BddVariableSet` and support set. This relies on L2's reading of `kernel.rs`.
- `Diagram::from_expr` already shows the one-support form.

**Consequence.** **S04 only.** Per-fact re-preparation (H9, H10, H12), plus avoidable `Refused` (Unknown) outcomes when the tree inflates past the cap. That is CI-04-safe but loses precision.

**Corrective contract (Proposed).** Lower each graph with one support and one variable set; memoize per distinct graph within a writer; optionally hash-cons in `join`.

**Preservation.** Canonical diagram identity; the existing pair-work and node refusals.

**Labels.** Implemented (partially re-read); remedy Proposed.

**Closure evidence.** Identical condition ids over the flow fixtures; a refusal count before and after.

**Principal re-read.** `conditions/graph.rs::{join, MAX_GRAPH_NODES, from_graph}`; `kernel.rs::from_expr`.

**Provenance.** L2-05.

### 7.6 Analyses

<a id="f32"></a>
#### F32 — Structural delegation rebuilds whole-universe indexes per seed on the default catalog path

**Cause.** `lctx_model::domain::structural::build::produce` calls `analysis::delegation::Inputs::traverse` per selected seed. Each call:
- reserves a budget sized on the whole universe: vertices × 1024 + arcs × (10·Step + 512) + paths…;
- rebuilds the `selected` set and an `unresolved` index by scanning all `event_assessments`;
- rebuilds `call_nodes` and `definition_nodes` `BTreeMap`s over all nodes of both graphs;
- only then runs the bounded BFS.

Per C, `produce` also uses nested `iter().filter/find` joins.

**Consequence.** **S02.** Work grows as O(seeds × V log V) plus O(n·m) joins, quadratic in library size, while each answer is bounded (H9, H10, H12). The per-seed reservation can refuse legitimate bounded searches on large graphs (H11).

**Corrective contract (Proposed).** Prepare the delegation inputs once per frame: node maps or the graph's own index, the unresolved index and the subsystem set. Size per-seed charges by `Bounds`. Replace the nested filters with keyed maps.

**Preservation.** `Bounds` semantics with explicit `Stop` and partial outcomes; deterministic order; boundary classification.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Per-seed work is independent of universe size beyond the bounded search; known-answer delegation controls give identical traversals.

**Principal re-read.** `delegation.rs::traverse` (reservation, `unresolved`, node maps); `structural/build.rs` call site.

**Provenance.** C-09.

<a id="f33"></a>
#### F33 — Optional analytics expand candidates quadratically under one global work cap

**Cause.**
- `analytics::neighbours::Prepared::nearest` iterates queries × targets × windows².
- `analytics::build` co-use emits n² pairs per owner.
- Both are capped by a single `build::MAX_WORK`, which turns the whole relation `Partial/WorkLimit`.

**Consequence.** **S02.** These optional analytics predictably become `Partial` for everything, and one high-degree owner disables communities for every frame (H10, H11). This is honest under CI-04 but not a feasible route. They are off by default.

**Corrective contract (Proposed).** Prefilter per subject block, or nominate candidates through the exact vector cohort with exact rescoring. Weight co-use through the bipartite owner–target graph, or apply per-owner caps reported locally. Leiden itself is appropriate.

**Labels.** Implemented (relies on C for kernel loops; `MAX_WORK` sites re-read); remedy Proposed.

**Closure evidence.** A local, per-owner partial on a skewed fixture rather than a global one.

**Provenance.** C-10, C-11.

<a id="f34"></a>
#### F34 — Retrieval embedding effects are batched per rendered root

**Cause.** `cpg_core::retrieval::produce` renders 32-root windows, then calls `realize` (`Session::prepare`) per partition. Each call makes one cache read and, for misses, one vLLM POST. The analytic embedding stage already batches 64 windows per `prepare`.

**Consequence.** **S01, S02 with embeddings.** Many small sequential requests starve vLLM batching (H8, H21).

**Corrective contract (Proposed).** Per render window, collect every ready text across partitions, call `prepare` once, then run `realize` per root from the session index.

**Preservation.** Per-root `verify_uses` and publication order; per-text refusals; budgeted chunking.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Service batch counts follow windows/64 in a fake-embedder control.

**Principal re-read.** `retrieval.rs` render-window loop and `realize`.

**Provenance.** C-08.

### 7.7 Serving

<a id="f35"></a>
#### F35 — Serving re-derives pin-fixed facts per call

**Cause.** Serving has no owner for facts that are fixed for the lifetime of a pinned process. Four obligations follow.
- **(a) Constant identities.**
  - `lctx_serving::operation_definition` hashes the serving blob, `lctx_model::domain::implementation_digest()` (`ContentHash::of(include_bytes!(model-implementation.bin))`, 8,259,597 bytes observed in both build directories), `native_requests::definition`, `mappings::identity` and `wire_identity`.
  - `lctx_model::domain::serving::dispatch::wire_identity` regenerates every tool's JSON schemas.
  - None is memoized. This reviewer's search for `OnceLock|LazyLock|once_cell` covered `lctx-serving/src`, `lctx-model/src/domain/serving` and `model.rs`; the only hits are unrelated `mappings` statics.
  - `preparation.rs::PreparedCache::key` embeds `operation_definition()` in every exact key; `available_key` builds layout and closure keys per hydrate.
  - `pagination::binding` calls `wire_identity()` per page section and continuation.
- **(b) Domain rediscovery.**
  - `library::resolve_inner` runs `lctx_surrealdb::scope::library_roots`, a multi-hop membership-checked walk, before consulting the cache keyed by those roots.
  - `selection::members` runs per request.
  - Classification and library preparations are rebuilt from cached batches per request.
  - `EmbeddingSpec` and `ProjectionDefinition` are point-read per vector search.
- **(c) Membership re-proof.**
  - `derived_search::hydrate_occurrence_nominations` calls `NativeReader::selected_membership` per window.
  - `SelectedPayloads::sparse_membership` queries `compiler_view_member WITH INDEX view_nodes` per 128 nodes × per view, plus ancestry and `validate_nominal_membership`, with no memo across windows, channels, the four families or requests.
- **(d) Key continuations.** `pagination::page` sorts the full `values` vector and applies `key > after`. Callers (`browse`, `FindOperations`, evidence sections) rebuild the complete domain and every packet per page.

**Consequence.**
- **S06.** Every call pays whole-blob hashing and schema generation, including the ranked continuation that makes no store round trip.
- **S02.** Rediscovery grows with library size. The packages scan without a library name grows with retained libraries (S08).
- Membership re-proof repeats across about nine pipelines per search.
- Enumerating a library by pages is quadratic in pages (H6, H12, H14, H19, H26).

**Corrective contract (Proposed).** A pin-lifetime prepared state owned by the pinned reader and service. It consumes the pin and request parameters, and produces immutable, charged products:
- (a) process constants computed once (`LazyLock`, or build-script-emitted digests);
- (b) prepared products keyed by request parameters under the pin: `(library | none)` → domains plus roots; `(library, selector)` → members; `(member set, selection inputs)` → prepared classification; `(spec, projection)` → validated policy;
- (c) a charged, bounded memo of validated per-node membership witnesses (direct plus ancestry provenance);
- (d) page-local packet construction once members are retained.

The existing moka retention, `Flight` single-flight and charging hold these.

**Preservation.**
- CI-13 binding of cursors, keys and the stored `fn::lctx_operation_definition`. The values are identical.
- The first-use definition check.
- The per-call query-vector input-hash check, which crosses a trust boundary.
- Eviction recomputes and never returns absent (CI-08).
- Charges and close-time draining.

**Counterexample.**
- A memo that recorded only "selected = true" would erase nominal-variant conflict refusal, so it must hold the validated witness set.
- A selection change (S05) cannot reach a pinned process. A shared-process deployment would add the handle to every key, which `PreparedCache::pin` already does.

**Uncertainty.** The relative cost of (a) against (b) and (c).

**Labels.** Implemented and Interface-checked; remedy Proposed.

**Closure evidence.** One computation of each constant per process; a second identical request makes no discovery round trips; each dependency's membership is queried once per reader across the four families; a page-N request performs no member discovery; tamper and definition-mismatch controls still refuse.

**Principal re-read.** `lctx-serving/src/lib.rs::operation_definition`; `model.rs::implementation_digest`; `dispatch.rs::wire_identity`; `preparation.rs::{key, available_key}`; `pagination.rs::{binding, page}`; `library.rs::resolve_inner`; the `operations.rs` call sites; HEAD `derived_search.rs::hydrate_occurrence_nominations`; HEAD `selection.rs::sparse_membership`.

**Provenance.** E-01, F1-05, E-02, E-03, E-08; R4, R11.

<a id="f36"></a>
#### F36 — The vector channel hydrates the whole eligible domain before scoring, and its ranking kernels scan linearly

**Cause.**
- **(a) Eligibility before score order.**
  - `search::vector_scoped` calls `selected_frontier`, then `derived_search::selected_occurrences(vec_occurs, ELIGIBLE)`, then `indexed_occurrence_rows` (`WITH INDEX eligible_input`).
  - That streams every eligible occurrence row for the input, including rows of retained realizations, then proves membership, hydrates, sorts (possibly spilling), validates and re-sorts.
  - Only then does `score_documents` compute exact cosine over all eligible documents, 128 per round trip (`vector_selection_sql`). `rank_occurrences` uses at most the top 1,024.
- **(b) Linear kernels.**
  - `rank_occurrences` does `documents.iter().take(tier).find(…)` per occurrence across up to four rewinds.
  - `score_documents` re-sorts after every push.
  - `insert_witness` scans the map with `max_by` per eviction.

**Consequence.** **S02, S06.** Per-call work is proportional to the eligible corpus (times differing retained realizations) rather than to the frontier the answer uses. Under concurrency the 30 s deadline turns that into refusals (H5, H7, H10, H11; the CI calibration row "Bounded answer, amplified work"). (b) adds O(occurrences × tier) CPU inside the request's permit.

**Corrective contract (Proposed).**
- (a) Score the cohort server-side in score order through `vector WITH INDEX cohort` (`encoder_hash, policy_key, family, library_input IN $input_keys ORDER BY score DESC, id ASC`), in tiers. Nominate and prove eligibility only for returned documents, counting toward the tier only documents with at least one eligible occurrence, and expand until the cap or cohort exhaustion. The frontier, order and ties are identical to today's.
- (b) Build a per-tier `HashMap`, a bounded heap ordered by `(score desc, id asc)`, and an ordered witness frontier.

**Preservation.**
- Exactness: the EXPLAIN control `!plan.contains("KnnScan")`.
- Deterministic ties.
- Eligibility before caps.
- Independent exact-match and zero-score branches.
- CI-04 empty-cohort reporting.

**Counterexample.**
- In member mode with narrow pairs, most top documents may be ineligible, and the lazy route then examines more documents, never more than today's full pass.
- If the cohort holds many vectors with no eligible occurrence (retained realizations, other libraries sharing an input), server-side scoring grows while client hydration shrinks.

**Uncertainty.** Cohort composition at S02; the lexical analogue for broad `@OR@` (not established).

**Labels.** Implemented and Interface-checked (pinned `knn_topk`/`topk_pushdown` per E); remedy Proposed. Rule impact RC11.

**Closure evidence.** An equivalence control (old against new) with ineligible top documents, tier-boundary ties and retained rows; hydrated occurrences bounded by the examined frontier; existing ranking unit controls pass.

**Principal re-read.** `search.rs::{vector_scoped, selected_frontier, score_documents, insert_witness, rank_occurrences, vector_selection_sql}`; HEAD `derived_search.rs::{selected_occurrences, indexed_occurrence_rows}`.

**Provenance.** E-04, E-07; R3.

<a id="f37"></a>
#### F37 — The vector contract is stated two ways, and the shared HNSW index has no production reader

**Cause.**
- `api-and-evidence-product.md` §14.8 ("native HNSW candidate ranks", "scoped HNSW candidate paths remain selected") and §14.11 ("Native SurrealDB BM25/HNSW are selected"), ADR-0131 ("declared approximate HNSW") and `docs/surrealdb.md` ("shared-vector HNSW candidates") state an approximate HNSW route.
- `synthesis-and-serving.md` §11.2 at HEAD states "an explicitly identified exact eligible-vector route" and that "inserting unrelated content cannot change an older publication's ranking". `search.rs::vector` and `vector_selection_sql` implement exact scoring.
- `lctx_surrealdb::materialization::native_definitions` defines `INDEX neighbor … HNSW DIMENSION 1024`.
- The only `<|k,ef|>` user is `scripts/programmatic_native.py::native_numeric_reference`, an evaluation worker. This reviewer's `rg` covered the repository excluding `docs/`, `third_party/`, `target/` and `build/`.

**R3 settled.**
- At pinned 3.3.0, HNSW KNN is approximate (ef-bounded), and its graph is per index over the **shared** `vector` table, so unrelated publications' inserts change results. This relies on E and L1 reading `surrealdb-idx-3.3.0`.
- The covered-predicate prefilter is exact only under a 2,000-document allow-list threshold, so exactness would change with scale.
- Both contradict §11.2's frozen ranking and CI-13. Brute-force exactness is required; L1-04's option (b) (adopt KNN) is rejected unless a per-publication index existed, and none does per table.
- L1-04's "no consumer" is corrected by P5: the evaluation worker consumes it.

**Consequence.**
- **S01 with embeddings, S05, S08.** Every vector insert pays HNSW maintenance and storage with no serving reader (H27; DP-16).
- The numeric evaluation control qualifies a route the product does not serve, so it says nothing about the served route.
- A reader of §14 may "restore" HNSW and break frozen ranking.
- G7 fails: the design claims a capability route the product does not use.

**Corrective contract (Proposed).**
- One owner (§11.2) declares the exact eligible-vector route as the sole serving contract.
- Retire `neighbor`, or keep it under a named consumer.
- Point `native_numeric_reference` at the served exact SQL.

**Preservation.** The separate analytical exact-neighbour contracts.

**Labels.** Interface-checked; the absence is stated with its search scope. Remedy Proposed. Rule impact RC10.

**Closure evidence.** A single owner statement; the index exists only beside a reader; the evaluation control exercises production SQL.

**Principal re-read.** `materialization.rs` DDL; `programmatic_native.py::native_numeric_reference`; the `rg` for `<|`, `vector::distance::knn` and `WITH INDEX neighbor`; the HNSW statements in the design text; HEAD §11.2.

**Provenance.** E-05, L1-04 (corrected); P5; R3.

<a id="f38"></a>
#### F38 — Serving SQL re-defines view membership in uncorrelated subqueries

**Cause.**
- Six serving statements filter with `… IN (SELECT VALUE node FROM compiler_view_member WHERE view IN $lctx_views)`:
  - `selection.rs::{library_packages_sql, library_releases_sql, library_inputs_sql, member_candidates_sql}`;
  - `evidence.rs::derivations` (twice).
- `member_candidates_sql`'s `$path` branch also scans `compiler_view_member … AND node.anchor IN $access`, a record-link dereference with no index.
- These are SQL-local definitions of view membership without the alias-ancestry and nominal-conflict semantics that `SelectedPayloads::sparse_membership` owns (DP-01).
- In pinned `surrealdb-core-3.3.0`, `exec/physical_expr/subquery.rs::ScalarSubquery` evaluates "each row" and no hoisting was found. Which engine runs these statements (exec or legacy compute) is unsettled (V06).

**Consequence.**
- **Authority (G1 unresolved).** It is unsettled whether the two definitions can disagree for packages, releases, inputs and catalog members, for example under alias membership.
- **S06.** If per-row evaluation holds, each outer row re-materializes the whole view membership: O(rows × |view|), worst at high-degree evidence roots (H10). Unresolved.

**Corrective contract (Proposed).** Nominate first, then check through `NativeReader::selected_candidate_ids` (the owned sparse route). Or use an index-backed correlated point probe on `view_nodes`. Resolve the access module's node first for path predicates.

**Preservation.** Exact multi-view, alias and nominal-conflict semantics, now from one owner.

**Uncertainty.**
- The cost half needs probe P-E1, a static read of `exec/plan_or_compute.rs`, or `EXPLAIN ANALYZE`.
- The authority half needs a check of whether alias members can appear in these families.

**Labels.** Implemented (duplicate definition); cost Interface-checked with a limit. Remedy Proposed.

**Closure evidence.** One membership owner for these queries; an alias-member control; high-degree `get_evidence` examined rows bounded.

**Principal re-read.** The HEAD and working-tree subquery texts in `selection.rs` and `evidence.rs`.

**Provenance.** E-06; V06.

<a id="f39"></a>
#### F39 — MCP admission bounds the embedding call with the native-capacity slot

**Cause.**
- `lctx_mcp.native.NativeExecutor` takes `asyncio.Semaphore(2)` (a 1 s wait, then `resource_refused`) and holds it, through `_finished`, until the worker ends, including `HttpEmbedder.embed`.
- Rust `RequestAdmission` independently bounds native capacity (`query_connections = 2`, `cpu_jobs = 2`).

**Consequence.** **S06.** Two searches waiting on vLLM refuse every other call after 1 s, including `get_evidence` with no embedding, while Rust capacity is idle (H21).
- Inquiry H judged the duplication acceptable for one agent issuing sequential calls.
- This reviewer agrees it is low for that deployment. The scope mismatch (an external service held inside a native-capacity slot) is still a real coordination defect for concurrent calls.

**Corrective contract (Proposed).** Python bounds what it owns: pending workers plus a separate embedding-concurrency limit derived from vLLM capacity. Rust owns native capacity, with its limits exposed through `NativeSession`.

**Preservation.** Ownership of cancelled workers until drain; the threaded end-to-end deadline.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** With two hanging embeddings, a third non-search call is admitted.

**Principal re-read.** `native.py::NativeExecutor.{execute, _finished, _execute}`.

**Provenance.** E-09 (H §4 counterpoint).

<a id="f40"></a>
#### F40 — The external sorter caps runs at 1 MiB, merges pairwise and creates a tempdir eagerly

**Cause.** `lctx_surrealdb::ordered_rows`:
- `RUN_BYTES = 1 MiB`, and `Sorter::new` rejects larger limits whatever the budget;
- "binary carry merging" with a two-way `merge`;
- `tempfile::tempdir()` on every construction.

Consumers include membership preparation, reconciliation, backup and restore, and serving search (five `SortedRows::with_budget` sites in `search.rs`, per family per request).

**Consequence.**
- **S01, S02.** Any set over about 1 MiB is rewritten about log₂(size / 1 MiB) times.
- **S06.** Temporary directories are created and removed per family per request on the hot path.
- The live-set bound is set far below the configured budget (H16, H17; A4).

**Corrective contract (Proposed).** Derive the run size from the reservation, merge with a single k-way heap, and create the directory on first spill. DataFusion's `ExternalSorter` is an alternative, but it lacks the equal-key byte-identity conflict semantics, so the bespoke owner is justified (G8 pass).

**Preservation.** Charged reservations; deterministic order; duplicate-conflict refusal; owned spill directory; cancellation and poisoning.

**Uncertainty.** Real set sizes; if most are under 1 MiB, only the tempdir cost remains.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Spill counts and bytes rewritten for one compile and one search; no directory without a spill.

**Principal re-read.** `ordered_rows.rs` (`RUN_BYTES`, `merge`, `Sorter::new`); the count of `search.rs` sites.

**Provenance.** L1-03; R10.

### 7.8 Capacity, cancellation and observation

<a id="f41"></a>
#### F41 — Per-process resource admission ignores co-located processes

**Cause.** Each process sizes resources as if it owned the host. Two obligations follow.
- **(a) CPU.**
  - Pyrefly's pool is `ThreadCount::AllThreads` (`pyrefly_stage.rs::analysis_pool`), which is correct for one production compile.
  - Fixture tests create default multi-thread Tokio runtimes.
  - nextest admits up to its default slots, and `.config/nextest.toml` declares no `threads-required` or `test-groups`.
- **(b) Memory.** `lctx_model::domain::resources::DEFAULT_MEMORY_BYTES = 64 GiB` is the default for `WorkspaceOptions`, CLI flags and fixtures. `cpg_core::workspace::Workspace::new` uses it as the budget ceiling and as DataFusion's `with_memory_limit`, the spill threshold.

**Consequence.**
- **S10.** Thirty-two concurrent fixture processes each admit host-sized CPU pools and a nominal 64 GiB, on a 188 GiB shared host.
- **S02.** Concurrent compiles do the same.
- DataFusion spills only after 64 GiB per process, so memory pressure and swap precede spill (H17, H21; DP-20).
- Historical receipts (F1, persisted plan §9.1): qualify runs with 21–24 `providers:extract` timeouts against focused runs finishing in under 4 minutes, a bimodal signature; one recovery observed 178 of 188 GiB used with swap. Both are unattributed.

**Corrective contract (Proposed).**
- (a) The runner owns admission: nextest `threads-required` on heavy fixture filters, plus an optional test group around SurrealDB-heavy tests. Production pools stay unchanged.
- (b) A capacity-derived envelope: cgroup `memory.max` or host-available memory at start, or an explicit envelope from the run owner. Per-request and per-stage reservations stay unchanged.

**Preservation.**
- Production parallelism.
- Light tests at full concurrency.
- Concurrency defects still fixed in code.
- Timeouts kept so a hang still fails.
- Explicit refusal.

**Counterexample.**
- Weighting can hide a concurrency defect that shows only at full overlap, so keep a deliberately concurrent control.
- A small fixed memory ceiling would refuse valid S02 work, so the envelope must be capacity-derived.

**Uncertainty (R12).** CPU oversubscription, store serialization (F01) and repetition (F45) are not apportioned.

**Labels.** Implemented; remedy Proposed. Rule impact RC12.

**Closure evidence.** A matched-load comparison of the same selection with and without weighting (F1's P-4), with F02's retry counts; stage peaks (`ResourcePool::stage_peak`) against the derived envelope; spill observed under pressure rather than swap.

**Principal re-read.** `resources.rs::DEFAULT_MEMORY_BYTES`; `workspace.rs::Workspace::new` (`with_memory_limit`); `pyrefly_stage.rs::analysis_pool`; `.config/nextest.toml`.

**Provenance.** H-04, F1-02; R12.

<a id="f42"></a>
#### F42 — No external cancellation reaches the compile's owned drain

**Cause.**
- `rg 'tokio::signal|ctrl_c|SIGTERM|SIGINT|signal_hook'` over `crates/{lctx,cpg-core,lctx-publisher}/src` returns nothing.
- `lctx/src/compile.rs` calls `store.abandon()` only on error paths.
- Run cancellation is SIGTERM.

**Consequence.**
- **S07, and S10 through nextest `terminate-after`.** A cancelled compile or test leaves its attempt open, with holds and possibly an unresolved effect. Only drained maintenance resolves them.
- Work with no consumer is not drained (H21), and recovery is broader than the cancelled unit (H24).

**Corrective contract (Proposed).** Translate SIGINT and SIGTERM into the existing `Cancellation`. The existing owners (`NativeCalls` `CallGuard`, the bridge drain, `drain_provider`) reach terminal results, then `abandon()` runs. Crash recovery through maintenance remains for SIGKILL.

**Preservation.** Committed effects keep their identity; unknown outcomes stay protected.

**Labels.** Implemented (absence, search scope stated); remedy Proposed.

**Closure evidence.** A SIGTERM control leaves the attempt abandoned with no open holds.

**Principal re-read.** The `rg` above; `compile.rs` abandon sites.

**Provenance.** H-06; R15.

<a id="f43"></a>
#### F43 — Per-root phase events are emitted at INFO by default

**Cause.**
- `cpg_core::stage_runtime::borrowed_cpu` creates an `info_span` and calls `Phase::begin`/`finish` per selected root.
- `cpg_extract::logging::DEFAULT_LOG_FILTER = "warn,lctx_phase=info"`, and `lctx/src/main.rs` installs it through `init_logging`.
- This corrects L1's §4 "no subscriber in the `lctx` CLI".

**Consequence.** **S02.** Observation volume grows with root count across stages (H26). A historical receipt (G, run `20261009T214543…`) shows 91% of `lctx_phase` lines are `borrowed_cpu`.

**Corrective contract (Proposed).** Aggregate per stage (count, total, maximum) in the coarse phase, and move per-root events to `debug`.

**Preservation.** Coarse begin and terminal semantics; cancellation and drain visibility.

**Labels.** Implemented; remedy Proposed.

**Closure evidence.** Per-root events are absent at the default filter, and stage aggregates are present.

**Principal re-read.** `logging.rs`; `stage_runtime.rs::borrowed_cpu`; the `main.rs` install.

**Provenance.** G-07; R25.

<a id="f44"></a>
#### F44 — Tokenizer assets are verified on one read and loaded from another

**Cause.** `cpg_core::retrieval_tokenizer::LocalTokenizer::load_verified` hashes `tokenizer.json`, `tokenizer_config.json` and `config.json` against `SHA256SUMS`, then `load` reads `tokenizer.json` and `tokenizer_config.json` again from disk.

**Consequence.** The verified bytes are not necessarily the bytes in use: an ambient input between the reads can change tokenization without changing the recorded identity (G4, CI-10). There is also an extra read.

**Corrective contract (Proposed).** Construct the tokenizer from the verified buffers (`Tokenizer::from_bytes`).

**Labels.** Implemented.

**Closure evidence.** The tokenizer is built from the verified buffers.

**Principal re-read.** `retrieval_tokenizer.rs::{load_verified, load}`.

**Provenance.** G-09; R26.

### 7.9 Development loop

<a id="f45"></a>
#### F45 — Fixture-driven provider tests recompile one fixture per test process

**Cause.**
- Three case modules wrap a full behavioral compile in a shared helper that every test awaits, with no memo:
  - `crates/cpg-extract/tests/cases/execution_channels.rs::data`;
  - `local_semantics.rs::fixture`;
  - `entry_value_witnesses.rs::fixture`.
- Each calls `typed_driver::run_behavioral` with complete observation.
- nextest runs a process per test, so 29 tests repeat 3 fixture compiles: native attempt, providers including Pyrefly and ty, persistence and full readback.

**Consequence.** **S10, `test-all`, `qualify`.** Work grows with assertions, not distinct fixtures, and runs concurrently, which compounds F41 and F01 (H12). The interrupted run `20261011T071240` shows only that these processes were in flight at cancellation (V03).

**Corrective contract (Proposed).** One test per (fixture, profile) compiles once and runs the existing checks, collecting each named failure. Alternatively a nextest setup script produces a run-owned observed-tables artifact keyed by fixture bytes and the producer digest. That is experimental at the pinned nextest and needs a storage owner.

**Preservation.**
- Independent, individually reported assertions.
- Tests that vary limits, order, profile or abandonment keep their own compiles.
- Determinism checks keep two compiles.

**Counterexample.** One panicking check masks later ones unless failures are collected. A stale shared artifact would feed old facts unless the key includes the producer digest and fixture bytes.

**Labels.** Implemented (cause); consequence Proposed.

**Closure evidence.** One compile per (fixture, profile); assertion inventory unchanged; per-test durations in a receipt.

**Principal re-read.** `cases/execution_channels.rs::data`; `run_behavioral` call sites; test counts.

**Provenance.** F1-01 (revised); R12.

<a id="f46"></a>
#### F46 — Crate boundaries couple unrelated rebuild and identity closures

**Cause.** Crate boundaries do not follow ownership (DP-17). Two obligations follow.
- **(a)** `crates/cpg-core/Cargo.toml` depends normally on `cpg-extract`, the only path from `pyrefly` to `lctx-publisher` (P9).
  - Per F1's offline `cargo metadata`, `lctx-publisher` and `lctx-embed` (791 and 793 packages) link Pyrefly, Ruff, salsa and tokenizers while using only the artifact, workspace and embedding types.
  - The `lctx-serving` dev closure does the same.
- **(b)** `lctx-surrealdb` holds both the compiler store with its DataFusion provider and the serving reader.
  - So serving and the Python binding link DataFusion and compile the compiler store.
  - Per F1, `producer_fingerprint::capture` walks all of `lctx-surrealdb/src`, so a reader-only edit changes the compiler source digest and invalidates all compiler reuse.

**Consequence.**
- **S10.** A provider edit rebuilds publisher, embed, `lctx` and every serving and publisher test target. A compiler-store edit rebuilds serving and the extension.
- **S03.** Serving-only edits invalidate compiler products (sound, but caused by the crate boundary).
- Historical receipts (F1, F2): focused `store:rust` builds took 34 s to 5 min 54 s against 1–24 s suites.
- Mechanisms: H5, H13, H27; A1.

**Corrective contract (Proposed).**
- (a) Move the provider contract types below `cpg-core` (into a contract crate or `lctx-model`) and let the composition owner (the `lctx` binary, the facts test drivers) construct providers. `compile_facts` already takes `Vec<Box<dyn ProviderStage>>`. Move `Embedder` and `LocalTokenizer` below `cpg-core`, or invert `lctx-embed`.
- (b) Split compiler store, provider and `projected_arrow` from a shared native core (codec, control, config), or feature-gate them. The provenance closure follows automatically because capture is crate-rooted.

**Preservation.** ADR-0118 adapter isolation; provider identity entering admission unchanged; one codec and control authority; no new provider framework.

**Uncertainty.** How much of `facts.rs` and `compilation.rs` needs concrete provider statics.

**Labels.** Interface-checked (relies on F1's closure counts; P9 confirms the edge). Remedy Proposed.

**Closure evidence.** `cargo tree -i pyrefly` shows no publisher or embed path; a reader-only edit leaves the compiler source digest unchanged.

**Principal re-read.** The `cpg-core/Cargo.toml` dependency line; P9.

**Provenance.** F1-03, F1-04; P9.

<a id="f47"></a>
#### F47 — Two near-identical optimized profiles keep separate build trees

**Cause.**
- `Cargo.toml` `dev` is O2 incremental with O3 imports. `release` is O2 incremental with O3 imports; the two differ mainly in codegen units and assertions.
- `scripts/verify.py::DEFAULT_CARGO_PROFILE = "release"`.
- AGENTS.md routes compile checks to `cargo check`/`cargo build`, which resolve to `dev`.
- F1 observed 63 GB (debug) and 48 GB (release) build trees.

**Consequence.** **S10.** An edit validated by bare cargo and by verify compiles twice at nearly the same optimization, and every dependency move compiles its O3 subtree once per tree (H12, H27).

**Corrective contract (Proposed).** One qualified local profile (`dev`/`test`) for bare cargo and ordinary verification, `release` for acceptance, and `cargo check` named as the compile check.

**Labels.** Implemented; remedy Proposed. Rule impact RC14.

**Closure evidence.** One local build tree in ordinary use.

**Principal re-read.** The `Cargo.toml` profiles; `verify.py::DEFAULT_CARGO_PROFILE`.

**Provenance.** F1-06; compilation-cost F03 (extended).

<a id="f48"></a>
#### F48 — Boundary tooling re-proves fixed premises on every invocation

**Cause.** Two obligations, both proving a property per invocation rather than where its premise can change.
- **(a) The server executable's identity.**
  - With an owned generation (F2 settled that the installed service uses one), each `scripts/surrealdb_service.py::_installed_binary` call validates the descriptor: an admission flock, a full SHA-256 of the 368.7 MB executable and a `surreal version` child.
  - F2 counts six call sites per fixture boundary. Two are pre-borrow readiness checks proving a property before the ownership that protects it, and `_server_identity` repeats `check_daemon` immediately.
  - `Installation.check` queries both databases although the borrower uses only `validation`.
- **(b) Test executable digests.** `scripts/verify_evidence/capture.py::Invocation.command_finished` SHA-256s every `compiler-artifact` executable after every step. The stat-keyed cache lives for one invocation. The digest has no comparing consumer: `cli.compare` does not diff `binaries`, and ADR-0147 says the digest establishes neither the source closure nor the executed bytes.

**Consequence.** **S10.**
- (a) P1 measured 0.155–0.161 s per pass for that file, on the shared host, warm.
- (b) For `model:rust` (87 executables, 6.75 GB), F2 infers about 4.3 s outside the step from historical receipts (H18–H20, H26).
- P3 bounds the median fixed overhead per fixture boundary at 1.1–2.8 s, small against multi-minute steps, but material in a warm `just fixture --` loop.

**Corrective contract (Proposed).**
- (a) Prove identity once, inside `borrow`, and carry the validated installation into later steps. Readiness in execution mode is the borrow itself. Query only the borrowed database. One digest per service-start epoch, or per invocation keyed by the daemon's `/proc/<pid>/exe` stat identity.
- (b) Record cargo's artifact identity plus stat at completion, and digest lazily on export or citation, or when `fresh: false`.

**Preservation.**
- Identity established before any administrative query.
- Refusal of a replaced binary.
- Maintenance exclusion.
- Honest "unknown" labels for source-to-binary and executed bytes.
- Exported bundles that claim bytes digest at export.

**Counterexample.** A stat tuple is not content (safeguard 3). The property carried here is "the same file object whose bytes were digested in this boundary", and an in-place rewrite changes ctime. If same-user tampering within a boundary matters, keep one digest per boundary rather than six.

**Labels.** Interface-checked (relies on F2's call-site count); P1 Measured for one pass. Remedy Proposed. Rule impact RC15 for (b).

**Closure evidence.** At most one executable digest per boundary and no version child after borrow; a swapped executable is still refused; a warm repeated selection performs no binary digests and export still verifies.

**Principal re-read.** `surrealdb_service.py::{file_sha256, _owned_server_generation, _installed_binary}`; `capture.py` binary cache.

**Provenance.** F2-01, F2-02; P1, P3.

<a id="f49"></a>
#### F49 — Focused and rerun selections never narrow the build

**Cause.**
- `verify.narrow` narrows packages and targets only for explicit cargo target options.
- `verify_evidence/rerun.py::nextest_case` knows the exact `binary_id` but emits only `-E binary_id(…) & test(…)` with the original `-p` list.
- So an exact rerun builds every test binary of the boundary's packages.

**Consequence.** **S10.** Cargo build and link time, not the suite, dominates focused loops (historical receipts in F2) (H5, H6).

**Corrective contract (Proposed).** Project `package::name` to `-p package --test name`, and `package` to `--lib`, in `rerun.nextest_case`, and optionally when every user expression is binary-qualified. Keep the boundary's required filter.

**Counterexample.** A name-only filter cannot be narrowed without listing, which builds everything, so it is left as is.

**Labels.** Interface-checked; remedy Proposed.

**Closure evidence.** `rerun-plan … --print` emits `--test`/`--lib`, and the run compiles only that target.

**Principal re-read.** `rerun.py` (`binary_id` handling).

**Provenance.** F2-04.

<a id="f50"></a>
#### F50 — Owner-to-object completion scans and validates the whole host registry

**Cause.**
- `scripts/storage_lifecycle.py::Storage.records` globs and validates every `objects/*.json`, raising on the first invalid descriptor.
- Run finalization (`runs.py`), cancel, fixture recovery (`surrealdb_fixture.py`) and `cli.retention` call it for objects whose IDs or paths are already known.

**Consequence.**
- Cost is negligible today: P2 measured 700 objects in about 6 ms.
- The defect is failure coupling: one invalid foreign descriptor makes every later run's completion "unresolved" and keeps its raw evidence from expiring (H25; S08, S10).

**Corrective contract (Proposed).** Complete by the manifest's IDs (`Storage.get`) and `_path_row`; reserve whole-registry scans for sweep, plan, trim and inventory.

**Labels.** Interface-checked; P2 Measured.

**Closure evidence.** With one corrupt foreign descriptor present, a run still completes its own components.

**Principal re-read.** `storage_lifecycle.py::Storage.records`; its call sites.

**Provenance.** F2-03; P2.

<a id="79-observations-that-are-not-findings"></a>
### 7.10 Observations that are not findings

| Observation | Why not a finding | Source |
|---|---|---|
| `ensure_rows_owned` runs about five statements per row in a SurrealQL `FOR` loop; set-based `UPSERT`, `INSERT` and multi-get forms exist | Runs server-side inside one transaction, with no crossing. Whether 3.3 plans the in-loop statements once or per iteration is unestablished, and `INSERT IGNORE` is not a substitute for the collision check. Revisit with a measured window latency after F03. | L1-02 |
| Serial per-root extraction after the joined Pyrefly run | Plausible H22, but its share is unmeasured, and the remedy raises the live set and needs `Natives` merge semantics. Measure the provider-phase split first (L2 probe 1). | L2-04 |
| Admission's in-memory `Lookup` grows with the graph | Charged at 192 B per entry, with explicit refusal; magnitude unknown. Revisit at S02 with measured graph sizes. | B-06 |
| Provider-side cross-batch `seen` index | About 130 B per row, charged; a second enforcement of store uniqueness, but it gives an early, typed conflict. Low value. | A-11 |
| DataFusion `parquet` feature | Compile-unit cost only; the codecs may arrive through other paths. Housekeeping. | L1-06 |
| Process per stdio MCP client (no shared pin, cache or admission) | The deployment shape (how many agents per server) is unknown. Decide it with a capacity owner before treating it as a defect. | E-10 |
| Sequential families and channels in `operations::scores` | Concurrency would cut latency, not work, and multiply server streams; remove work first (F35, F36). | E §4 |
| New `httpx2.AsyncClient` per search call | One local connection setup per call; small. | E §4, L1 §4 |
| Pyrefly residency in a long-lived compiler worker | A process-model change; lctx-owned facts reuse (F16) is the recommended default. | L2 §7 |
| A changed model declaration rebuilds everything | Conservative and consistent with the design-phase cutover; DP-09 permits it. | B §4, C §4 |
| Native EXPORT/IMPORT versus the bespoke backup and restore | Native export cannot select a closure, and IMPORT bypasses admission; the bespoke route fills a real gap. | D §5, L1 §4 |
| Failed-only reruns through nextest `-R` | Its experimental status at 0.9.146 is unsettled; `rerun-plan` already gives exact cases. | F2-05 |
| `lctx-model` mixes declarations and kernels (196k lines) | Credible H3 separation, but its value needs measurement (F1 P-5). | F1 §4 |
| Effect payload JSON comparisons on cold paths expand `Bytes` | Cold paths only. | G §4 |
| Four to five gRPC sessions per compile, each signing in | Constant per compile. | D §4 |

---

## 8. Library fit and total complexity

| Capability and owner | Consumer or scenario | Candidates | Semantic fit and gaps at the resolved version | Burden | Choice and reason |
|---|---|---|---|---|---|
| Transactional fencing (`lctx-surrealdb::control`) | Every effect (F01) | SurrealDB 3.3 RocksDB optimistic transactions; guard records; lanes | Only optimistic transactions exist (`owned_tx.rs::Db`); `FOR UPDATE` registers keys and takes no lock; predicate scans are not conflict-tracked, so explicit materialization of conflicts is necessary | Phantom inventory per fence | Keep library semantics; move the materialized write to the rare side (F01) |
| Columnar results over gRPC | Read path (`projected_arrow`) | `surrealdb-protocol` `ArrowBatch` | "Not served yet" at 0.13.1, and the patched SDK refuses it (L1) | — | Bespoke `Value → Arrow` remains necessary |
| External ordering (`ordered_rows`) | Membership, reconciliation, serving | DataFusion `ExternalSorter`, `arrow-row` | No byte-identity duplicate semantics | Post-sort check needed | Keep bespoke; fix its parameters (F40) |
| Exact top-k vectors | Serving (F36) | `vector::similarity` `ORDER BY … LIMIT`; `<\|k,COSINE\|>` brute force; exec `KnnTopK` | Exact; ties by scan order unless ordered explicitly (E) | — | Keep exact scoring; reorder (F36) |
| Approximate ANN | — | HNSW `<\|k,ef\|>` | Approximate over a shared graph, so it violates frozen ranking (F37) | Index maintenance with no reader | Retire or name a consumer (F37) |
| Analyzer incrementality | S03 (F16) | Pyrefly `State`/`Transaction`; salsa `persistence`; ty queries | Residency only; no serialization; ty queries not `persist`; salsa persistence would need deep fork work (L2) | — | lctx owns facts reuse (F16) |
| Ruff semantic traversal | Provider (F26) | Fork `check_ast_with_observation` | One traversal can serve rules and observation (L2) | Fork bump | Use the fork entry point (F26a) |
| Graph and condition kernels | Analyses | petgraph, leiden-rs, biodivine, fixedbitset, fcars and odis (oracles) | petgraph `page_rank` is unweighted and O(V²·deg) per iteration; fcars lacks implications | — | Bespoke ranking and FCA justified (G8 pass) |
| Test admission | S10 (F41, F45) | nextest `threads-required`, test groups, setup scripts, `-R` | Weighting is stable; setup scripts and `-R` recording are experimental at 0.9.146 (unsettled) | Runner config | Weighting (F41a); in-test grouping preferred to setup scripts (F45) |
| Preparation coalescing | Serving (F35) | moka `get_with` | Re-runs `init` in another waiter on abort; lacks charge retention (L1) | — | moka as retention only, with the bespoke flight (adequate) |
| Byte serialization | Product cache, effect digest (F20c, F05b) | `serde_bytes` / raw bytes | Integer-array JSON at ×3.3 (P7) | Trivial | Adopt a bytes encoding |
| Tokenizer | Embedding (F44) | `tokenizers::Tokenizer::from_bytes` | Exact | Trivial | Load from verified buffers |
| Native backup | Backup and restore | Native `EXPORT`/`IMPORT` | No closure selection; `IMPORT` bypasses admission | — | Bespoke backup and restore justified |

Composed capability: the decisive mismatches are not library choices but compositions. A good
library operation is issued per sub-operation (F03, F04), per root (F20, F34), per window × view
(F35c), or before the optimizer-visible score order (F36). H8: keep the useful unit of intent
visible to the responsible mechanism.

---

## 9. Alternatives and tradeoffs

| Alternative | Change propagation and local reasoning | Semantic authority and composition | Test and substitution boundary | Total machinery and operational risk | Evidence, decision, revisit |
|---|---|---|---|---|---|
| Current baseline | Semantic changes stay local; physical costs propagate through effects, identities and crate closures (A1 violated in bounded places) | Strong model authority; two contract statements for vectors; SQL-local membership | Fixture compiles per test; analyzer stack in store and serving tests | Protocol fixed costs per sub-operation; global conflict point; re-verification passes | Revise |
| **Proposed direction** | Physical units chosen per execution need (H2): read-only fencing with two-phase fences; window effects; attach by reference; facts reuse; carried admission identities; pin-lifetime serving state; keyset paging | Unchanged semantic owners; one membership owner; one vector contract | Runner-owned test weighting; crate edges follow ownership | Removes write amplification, rewrites and repeated proofs; adds a phantom inventory, a recovery-hold lifetime and witness checks | Proposed; settle with the prerequisite order (§12) |
| Library-owned alternative | Striped lanes instead of a two-phase fence (L1); DataFusion sorter; nextest setup scripts; long-lived Pyrefly worker | Lanes keep bidirectional conflict without restructuring fences | Setup scripts need a storage owner and are experimental | Lanes keep one durable write per effect; residency changes the process model | Lanes stay acceptable where a scan cannot be split; residency is not recommended |
| Simplest viable alternative | Bounded backoff plus counter (F02); intent without payload (F05a); window filling (F04); `LazyLock` constants (F35a); drop the second `lower` and second state hash at seal (F10 narrow); keyset paging (F06) | No contract changes | No test-boundary change | Small; leaves F01 contention, F16 and F17 in place | A legitimate first slice; does not close A4 for S02, S03 or S10 |

What becomes harder under the proposed direction:
- fence operations need a two-step protocol and an inventory;
- attach by reference makes enumeration follow holds;
- carried identities need a witness covering memberships;
- per-relation dependency tokens add key structure.

These are the tradeoffs for removing O(rows) repeated work.

---

## 10. Verification and uncertainty

### 10.1 Evidence labels used

| Claim or scenario | Label / date | Basis | Conditions | Outcome or gap |
|---|---|---|---|---|
| Engine conflict semantics (F01) | Interface-checked, 2026-10-11 | Pinned `surrealdb-kvs-rocksdb-3.3.0` | Official 3.3.0 server plus timeout patch (provenance per H and L1) | Rate unmeasured |
| Effect and window counts (F03–F05) | Implemented | Source read | — | Round-trip latency unmeasured |
| SHA-256 pass on the server executable | Measured (P1), 2026-10-11 | `hashlib.file_digest`, 5 passes | Warm cache, load about 6 | Not a per-boundary time |
| Registry parse (F50) | Measured (P2) | 700 objects | Excludes locks and validation | — |
| JSON byte expansion (F05, F20) | Measured (P7) | One 6,420-byte sample, ×3.31 | Distribution-dependent | — |
| Phase durations (F09, F10, F24) | Historical receipts (P8, B, D, G), 2026-10-09 to 2026-10-11 | `lctx_phase` lines | Mixed revisions, fixtures and load; phases nest | Unattributed |
| S10 timeouts (F41, F45, F01) | Historical receipts (F1, F2) | Qualify and focused runs, 2026-10-08 to 2026-10-11 | Mixed revisions and load | Causes not apportioned (R12) |
| Every remedy | Proposed | — | — | — |

### 10.2 Corrections to the supporting material

- **V02.** TyFlow does not run under catalog: five providers, not six. Applied throughout.
- **V03.** The ~280 s durations in P4 are the SIGTERM instant. H-01's "consistent with a shared
  serializing resource" and C-01's P4 citation carry no weight here.
- **R1.** Effects within one compile are sequential. L1-01's "8-way collapse within S01" is wrong;
  the contention is cross-process.
- **R3.** L1-04's "no KNN consumer" is wrong; the evaluation worker consumes it.
- **R5.** L1's owners × keys multi-get is the Cartesian that the native-execution-efficiency
  review's F01 rejected. It stays an alternative pending the owner-count premise (F22).
- **New: L1 §4 "no subscriber in the `lctx` CLI".** Wrong. `lctx/src/main.rs` calls
  `cpg_extract::logging::init_logging()` with `DEFAULT_LOG_FILTER = "warn,lctx_phase=info"` (F43).
- **New: F2 §4's "analytics:rust second copies … different feature sets".** Refuted by F1's
  revised report: they are distinct versions under the `odis` oracle dev-dependency, not a
  feature-unification failure.
- **New: the operation map's M2 "12 RTs per window" for originals.** The unit is per chunk (V07,
  F04).
- **New: the `ensure_immutable_rows` comment** in `compiler.rs` ("insert new rows in one
  statement … without per-row transactions") no longer describes its body, which issues
  `ensure_rows_owned` plus `hold` (plus `hold_many`) effects per window. That is a stale comment,
  noted for the F03 owner.

### 10.3 Unresolved, kept explicit

- **R1, the fence remedy.** Read-only plus two-phase fences, or striped lanes. Diagnosis settled;
  remedy settled by the phantom-partner inventory (F01).
- **R3.** Settled in favour of exact scoring (F37). The efficiency route is F36's reorder.
- **R5, the multi-owner lookup remedy.** Unresolved; deciding evidence is the owner-count inventory
  and the selected plan (F22).
- **R12, the S10 timeout causes.** Repetition (F45), CPU and memory admission (F41) and store
  serialization (F01) are unapportioned. Settling evidence:
  - F02's retry counter on a concurrent boundary;
  - a matched-load run with and without `threads-required` (F1's P-4);
  - per-test phase extraction for one `extraction_calls` test (H's P-H2).

  The rule change RC12 should wait for this evidence.
- **E-06 cost (F38).** Exec versus legacy engine (V06); probe P-E1 or `EXPLAIN ANALYZE`.
- **Facts determinism (F16).** Whether Pyrefly's AllThreads run is set-deterministic (L2 probe 3,
  then a replay control).
- **Peak memory and volumes.** Real-library row counts, peak RSS and phase splits need an
  authorized pilot. They affect priorities, not diagnoses.

### 10.4 Light probes that could sharpen this review (coordinator's decision)

1. **P-E1 (static).** Read `surrealdb-core-3.3.0/src/exec/plan_or_compute.rs` and the select
   planner's lowering of `Expr::Select` in a `WHERE` operand. Discriminates F38's cost half:
   evaluation per row versus once per statement.
2. **P-D1 follow-up (read-only).** Inspect the one 2026-10-08 run log that matched
   `TransactionConflict`/`Resource busy` (D's scan) for whether it is a deliberate control or an
   escalated contention. Weakly informs F01's exposure.
3. **L2 probe 3 (static).** `rg 'TEST_THREAD_COUNT|ThreadCount::'` in the pinned Pyrefly tests.
   Informs F16's determinism premise and F27.

None of these would change the decision (Revise) or any gate verdict, except that P-E1 could
lower F38 to its authority half.

---

## 11. Authority changes and dispositions

**Disposition owner: not yet scheduled.** Plan creation chooses between the persisted-graph
coordinator §8 and a new plan. This review edits no plan, status or rule. Rule changes take
effect only once the operator confirms them during plan creation.

| ID | Rule and where recorded | Proposed change | Dependent findings | If the rule is kept |
|---|---|---|---|---|
| <a id="rc01"></a>RC01 | ADR-0145 §Issuance ("Keep installation coordination initially, restricted to short actual-effect transactions"); the `control.rs` module header; the SA8 Coordination row (`surrealdb-architecture-and-capability-leverage-plan` §6; persisted-graph plan §7.5) | Ordinary effects read the installation fence; rare fences become two-phase; lanes only where a scan cannot split; `admission_revision` gets its own sequence | F01; enables F03, F07 and parallel windows | F01 becomes a declared support limit: concurrent effect-writing processes in one database contend on one record. F02's backoff and counter still apply, with a revisit trigger on measured retry share. F03 and F07 then remain sequenced behind contention. |
| <a id="rc02"></a>RC02 | `UpperStage::reuse` "external effects retain their existing owner"; ADR-0145 checked attachment applied to pure stages only (GR plan §4) | Captured-input `Extraction`/`Acquisition` stages become attachable under CI-10 | F16 | S03 policy changes and unchanged recompiles always re-extract; F03 and F04 only mitigate. |
| <a id="rc03"></a>RC03 | ADR-0145 "checked attachment may reuse retained *admitted* content" versus its "Recovery retains compatible completed prerequisites" | Completed contributions of terminal unadmitted attempts are attachable under a recovery hold with a declared lifetime, then reclaimed | F13 | Correct the recovery sentence: recovery replays the whole attempt. F13(b) reclamation still applies. |
| <a id="rc04"></a>RC04 | Exact-view reuse dependency and identity composition (`lctx-model::domain::completed::{CompletedView, CompletedContribution, ContributionSpec}`; `SourceSnapshot`; GR plan) | Per-relation content tokens; refreshed-spec attachment for value-only consumers; receipts may cite content identity | F18, F19 | F18 shrinks to splitting the Configuration contribution per relation. |
| <a id="rc05"></a>RC05 | §14.10 "Catalog and behavioral profiles have explicit identity", as realized per contribution | Profile identity at attempt, manifest, publication and profile-dependent stages only | F19 | S04 reuses nothing. |
| <a id="rc06"></a>RC06 | ADR-0145 publication ("Validate and reconcile … independent cold admission"); `storage-and-publication.md` physical lowering ("complete independent expected-row reconstruction"); `synthesis-and-serving.md` ("Publication reconstructs and reconciles derived statistics") | Full independent reconstruction at cold trust boundaries (import, restore, export, audit); same-process publication reconciles derived rows once and carries admission identities | F10 | Remove only exact duplicates: the second `lower`, the second state hash, the second lexical analysis and the second `verify_epoch`. Keep one `Loader::reconcile`. |
| <a id="rc07"></a>RC07 | Completed-state identity format `native-completed-state/v3` (`STATE_FORMAT_VERSION`; storage-and-publication owner) | A compositional digest over completion digests, plus an attempt witness to seal | F09 (option a) | F09 option (b): batched, once per attempt, ordinary path without cold backing validation, seal compares a witness. |
| <a id="rc08"></a>RC08 | `semantic-model.md` coverage and frontier contract; the `lctx-model` `coverage_frontier` invariant (per-scope coverage for every invocation) | `NotRequested` is invocation-level and domain-total, with no per-scope rows | F24 | F24 narrows to fusing the stage units physically; the write volume stays. |
| <a id="rc09"></a>RC09 | Publication handle contract `native-publication/v2` (`publication_identity`) | Add a derivation-implementation identity | F11 | No safe republish short-circuit; a lowering change over identical content cannot republish. |
| <a id="rc10"></a>RC10 | HNSW statements: ADR-0131 ("declared approximate HNSW"); `api-and-evidence-product.md` §14.8 and §14.11; `docs/surrealdb.md` retrieval paragraph | The exact eligible-vector route (§11.2) is the sole contract; retire `neighbor` or name its consumer | F37, F36 | Kept as stated, approximate shared-table ANN contradicts §11.2 frozen ranking; only a per-publication immutable index would satisfy both, and none is available per table. |
| <a id="rc11"></a>RC11 | "Eligibility precedes ranking" / "Eligibility checks those dependencies before ranking limits" (§14.8, §11.2) | Eligibility precedes caps and tier counting, not the evaluation order of scores | F36(a) | Kept as a work-order rule, it forbids the reorder; F36 reduces to (b) plus F35(c). |
| <a id="rc12"></a>RC12 | "Do not impose job, worker or thread caps" (AGENTS.md Testing and Cargo setup); ADR-0137 "tests use their ordinary worker defaults"; `docs/surrealdb.md` "no … fixed memory cap is selected"; operator memory "Never constrain test parallelism" | Allow runner-declared per-test weight (`threads-required`, test groups) that states real demand, and a capacity-derived memory envelope; parallelism is otherwise unchanged | F41 | F45 still removes repeated work. Independent per-process admission remains, and the 64 GiB constant should at least be documented as not bounding host memory under concurrency. **Recommended to wait for R12's settling evidence.** |
| <a id="rc13"></a>RC13 | `docs/surrealdb.md` "No database per attempt or test" for fixtures | — (not recommended) | F01 (alternative) | Listed only because physical isolation per attachment is the alternative if RC01 is refused. |
| <a id="rc14"></a>RC14 | ADR-0137 profile set (dev, a separate qualified test profile, release); AGENTS.md "Compile checks (`cargo check`/`cargo build`)"; DESIGN build paragraph | One qualified local profile as `dev`/`test`; `release` for acceptance; `cargo check` as the compile check | F47 | Edits validated through both bare cargo and verify compile twice. |
| <a id="rc15"></a>RC15 | Agent-test-evidence plan "Capture available Cargo/native artifact identities…"; README.md (Tests) on binary digests | Artifact identity is cargo identity plus stat; digest at export or citation, or for non-fresh artifacts | F48(b) | A host-level stat-keyed digest cache, so only rebuilt binaries are hashed. |
| <a id="rc16"></a>RC16 | Physical lowering of originals keyed per artifact-in-revision (`storage-and-publication.md`; `schema.rs` DDL) | Content-keyed chunk storage referenced by per-artifact headers | F14 | Patch upgrades and retained releases store full closure copies; F04 still reduces round trips. |
| <a id="rc17"></a>RC17 | `storage-and-publication.md` "Current provenance, exact dependencies and independent cold admission remain mandatory" for attached content | Cold admission may be satisfied by a durable checked-premise record keyed by exact views and implementation digests | F15 | F15 is withdrawn; the in-workspace memo remains the only carry. |
| <a id="rc18"></a>RC18 | Installed runtime configuration always enables `reuse` (`scripts/surrealdb_service.py`) | Conditional: if F20(a–c) are not fixed, make the per-root cache opt-in | F20 | No change needed once F20 lands. |

No rule impact is proposed for §B1 (provider-local parsing), §B12/§B13 (SurrealDB as the store and
pinned serving), the 20 s and 10 s server deadlines (F06 conforms to them), or the durability mode
(`SyncMode::Every`).

---

## 12. Architectural judgment and decision

### Foundation verdicts

| Foundation | Verdict | Decisive reason |
|---|---|---|
| FP-01 Separation of concerns | **Violated (bounded)** | Semantic owners are coherent, but crate boundaries couple unrelated rebuild and identity closures (F46), and the handle's identity scope excludes derivation code that lives in another crate (F11) |
| FP-02 Stable contracts | **Violated (bounded)** | The vector contract is stated two incompatible ways (F37); the publication handle does not bind its derived-row implementation (F11) |
| FP-03 Composition | **Satisfied** | Workflows compose owned operations; lowerings *may* fuse, and the failures are physical choices under FP-07, not reimplemented rules. The exception is F38's SQL-local membership, judged under FP-04 |
| FP-04 Domain model and semantic authority | **Violated (narrow)** | The model is adequate and governs behaviour broadly; serving SQL re-expresses view membership outside its owner (F38) |
| FP-05 Explicit structure | **Satisfied (sampled)** | Lifecycle, effects, fences and outcomes are explicit; the retry policy (F02) and cancellation route (F42) are the explicit gaps |
| FP-06 Local reasoning | **Violated (bounded)** | Local provider semantics need a native fixture compile per test process (F45); store and serving tests link the analyzer stack (F46) |
| FP-07 Execution fits the workload | **Violated** | Structural amplification on S01–S03, S06, S08 and S10 (F01–F12, F16–F25, F32, F35, F36, F40, F41) |

### Architectural judgments

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | **Violated (bounded)** | A provider edit rebuilds publisher, embed and serving tests; a serving edit invalidates compiler reuse (F46); one configuration toggle invalidates unrelated consumers (F18). Semantic changes do follow their owners (S09). | F46, F18 |
| A2 Encode domain meaning explicitly | **Violated (narrow)** | Model adequacy and authority hold for facts, coverage, outcomes, identity and invariants. Serving reconstructs view membership locally (F38), and the vector contract has two owners in text (F37). | F38, F37 |
| A3 Extend through composition | **Satisfied** | Adding a fact family or analyzer is one declaration plus provider code (S09). The physical per-kind expansion (F23, F24) is an A4 matter. | — |
| A4 Fit execution to the supported workload | **Violated** | Per S01–S10: protocol units per sub-operation and a global conflict point (F01–F05); deadlines bound to growing streams (F06); re-verification at seal (F09, F10); no facts reuse and O(rows) hits (F16, F17); quadratic key seeding (F21) and per-seed universes (F32); per-call rediscovery and eligible-domain-first scoring in serving (F35, F36); per-process admission in the development loop (F41). Honest refusal and bounded output do not establish fit (H11). | Priority table below |

**Bounded decision: Revise.**
- **Enclosing architecture:** needs revision for execution fit. The semantic architecture is
  accepted for the scenarios examined, at Interface-checked/Implemented strength. This is a review
  decision, distinct from release or real-library qualification.
- **Gates:** G4 and G7 fail narrowly (F44, F37); G1 is unresolved (F38, F20b). The remaining core
  and profile gates pass on sampled evidence; sampled positives are not product-wide passes.

### Priority (by consequence) and prerequisite order

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Fence realization and retry observability (`lctx-surrealdb::control`) | F01, F02 | Phantom inventory; interleaving controls; retry counts on a concurrent boundary |
| 1 | Keyset paging for size-scaled streams (`seal_staging`, attach, completion) | F06 | No statement scales with attempt size |
| 1 | Window-level effects, chunk windowing, intent without payload (`compiler.rs`, `loader.rs`, `authorization.rs`) | F03, F04, F05(a,b) | One effect per window; effect counts per file; intent bindings |
| 1 | Carried admission identities; seal reconciles once (`compiler.rs`, `native_publication.rs`) | F09, F10 | Seal trace; tamper and late-mutation controls |
| 1 | Attach by reference, then facts reuse (`compiler.rs`, `facts.rs`) | F17, F16 | Constant identity counts after N recompiles; providers attach |
| 1 | Key-only access through the membership run (`compiler_provider.rs`, `compiler.rs`) | F21 | Payload RTs proportional to demand |
| 1 | Pin-lifetime serving state; eligibility in score order (`lctx-serving`) | F35, F36 | No rediscovery on repeat requests; equivalence control |
| 2 | Republish short-circuit with derivation identity; ownership by staging root | F11, F12 | Zero derived writes for an unchanged republish |
| 2 | Value cutoff and profile-neutral keys | F18, F19 | Policy-change and S04 hit controls |
| 2 | Unadmitted lifecycle: recovery hold plus reclamation | F13 | Interrupt-and-rerun; abandoned-attempt retirement |
| 2 | Vector contract owner; HNSW retirement | F37 | One owner statement |
| 2 | Union views once; `NotRequested` domain-total; shared frontier index | F23, F24, F25 | One installation per relation; invocation-only outcomes |
| 2 | Multi-owner lookup (after the owner-count premise) | F22 | History-independent examined rows |
| 2 | Provider extraction repetition and pool use | F26, F27 | Parity controls; digest equality across thread counts |
| 2 | Structural per-frame preparation | F32 | Per-seed work independent of universe |
| 2 | Sorter parameters; SQL-local membership | F40, F38 | Spill metrics; one membership owner |
| 2 | Development loop: test grouping, crate edges, runner weighting (after R12 evidence) | F45, F46, F41 | One compile per fixture; dependency closures; matched-load run |
| 2 | Cancellation route; durable effect simplification (after the ADR-0145 owner confirms) | F42, F05(c) | SIGTERM control; recovery controls |
| 3 | Remaining low items | F08, F14, F15, F20, F28–F31, F33, F34, F39, F43, F44, F47–F50 | Per finding |

The prerequisite order is given in the [synthesis](#prerequisite-order-distinct-from-priority).
In short:
- F01 (and F02) precede F03 and F07, because fused or overlapping effects need effects that do
  not all conflict.
- F17 precedes F16, F13(a), F18(2) and F20's pointer alternative, because hits must be O(1).
- F09 precedes F10, because seal needs a witness to compare.
- F11's identity precedes its short-circuit.
- F18 step 1 precedes F19.
- F37 precedes F36.
- F35(b) precedes F35(d).
- R12's evidence precedes RC12.

### Next consequential decision

The fence-protocol decision (RC01), together with the per-consumer phantom inventory it needs, is
the most consequential. It gates the protocol and producer-overlap corrections and decides S10's
store share.

The attach-by-reference contract (F17) is the second, because every reuse expansion depends on it.

Both belong to the native-persistence owner. They are routed through plan creation, which also
chooses the disposition owner (coordinator §8 or a new plan) and presents RC01–RC18 to the
operator.

---

## Positions on prior leads

Inquiries supplied these positions. This review reconciles them; where inquiries differed, the
reconciled position and its reason are given. "Not examined" means no inquiry and not this review.

| Prior review (lead block) | Position | Evidence and current home |
|---|---|---|
| catalog-compilation-speed F01 (selected grains lose selectivity) | **Residual / moved** | The IPC route is gone; `id` predicates still iterate the whole run (F21) |
| catalog-compilation-speed F02 (prefix rebuild at completion) | **Resolved (moved)** | Completion re-reads only its own contribution |
| catalog-compilation-speed F03 (stable validity discarded) | **Residual** | Premise memo within a workspace; nested epochs, whole-state hash (F09), no durable carry (F15) |
| catalog-compilation-speed F04 (unrequested analytics prepare inputs) | **Resolved** for kernel scopes; **residual** for expected domain and coverage | F24, F25 |
| catalog-compilation-speed F05 (one native crossing per occurrence) | **Resolved** in its owner; **recurred** in the originals path | F04 |
| catalog-compilation-speed D01–D04 | Not examined | — |
| native-execution-efficiency F01 (Cartesian owner × key) | **Moved** | Cartesian removed; replaced by per-key scalar lookups with history coupling (F22) |
| native-execution-efficiency F02 (support admission per grain) | **Residual** | `validate_grain` plans per grain (G) |
| native-execution-efficiency F03 (overlapping content tokens) | **Resolved** | Tokens fetched once per union batch (C) |
| native-execution-efficiency F04 (warm product's rich representations) | **Residual** | Decode chain on per-root hits (F20c) |
| native-execution-efficiency F05 (fixed-interval polling) | **Resolved** for the bridge; residual file-lock polling immaterial | A, H |
| native-execution-efficiency F06 (completion waits for every scan) | **Resolved** for ordinary producers; **residual** `wait_scans` in the state identity | `state_identity_inner` (F09) |
| native-execution-efficiency F07 (moka generation split) | **Resolved** | H, L1 |
| native-execution-efficiency F08 (extraction inspection reconstructs inventory) | Not examined | — |
| persisted-execution-correction-causes F03 (planner-sensitive lowering in consumers) | **Residual** | SQL-authored `id` predicates (F21) |
| persisted-execution-correction-causes F04 (whole match arrays) | **Resolved** | Spillable runs and windows |
| persisted-execution-correction-causes F05, F06 | Not examined | — |
| unified-persistent-surrealdb F01, F02, F07 | Not examined | — |
| unified-persistent-surrealdb F03 (database-wide publication and pinning) | **Residual (seam)** | Derived occurrences are eligible by dependency; the in-progress serving diff touches this |
| unified-persistent-surrealdb F04 (reuse is value replay) | **Residual** | No payload replay; O(rows) membership copy (F17); no facts reuse (F16) |
| unified-persistent-surrealdb F05 (durable effect fencing) | **Resolved, at a cost** | F01, F05 |
| unified-persistent-surrealdb F06 (logical isolation for concurrent verification) | **Residual** | The shared installation record couples tests (F01) |
| unified-persistent-surrealdb F08 (retention in the content contract) | **Resolved structurally; growth residual** | F13, F17 |
| unified-persistent-surrealdb plan TF01 | **Resolved (Implemented)** | Typed restore ingress |
| populated-journey-execution-amplification F01 | Not examined | — |
| populated-journey-execution-amplification F02 (repeated canonical preparation) | **Residual / moved** | One shared preparation; admission and publication each traverse (F10) |
| populated-journey-execution-amplification F03 (attempt-wide barrier) | **Moved** | Resolved for producers; residual in the state identity (F09) |
| populated-journey-execution-plan F01 (admission certifies the post-freeze state) | **Confirmed implemented** | Freeze before `admit_semantics` |
| graph-compilation-kernels-and-hashing F01 (per-root planning and hydration) | **Residual** | Per-grain seeding (F21); SQL planned per grain × relation (B §4) |
| graph-compilation-kernels-and-hashing F02 and hashing guidance | Hashing guidance **conformant** (G); F02 not examined | — |
| assembled-graph-scope-compilation VS-F01, C-F01, C-F03–F05, GK-F01, GK-F02 | Not examined | — |
| assembled-graph-scope-compilation C-F02 (optional discovery caches) | **Resolved** | H |
| assembled-graph-scope-compilation GK-F03 (reservations beyond allocation) | **Resolved** for the diagnosed case; residual envelope | F41(b) |
| persisted-execution-corrections-integrated F01–F04 | Not examined | — |
| compiled-product-reuse-contracts F01 (unknown native completion) | **Resolved in source; not re-tested** | Product-cache fences (C) |
| compiled-product-reuse-contracts F02 (detached wait owners) | **Resolved** | `PreparedCache` flights (H) |
| compiled-product-reuse-contracts F03 (private/public binding) | **Resolved by removal; cost moved** | Behavioral private owners are always fresh (C §4) |
| compiled-product-reuse-contracts F04 (canonical decoding) | **Resolved; cost noted** | F20(c) |
| graph-compilation-and-reuse-plan F01 (viewer retirement) | Not examined | — |
| holistic-state-management HS-F01, HS-F02, HS-F09, HS-F10 | Not examined | (HS-F02 is related to F36 as a moved residual, not examined directly) |
| holistic-state-management HS-F03 (retirement bounds candidates) | Not examined in depth | Bears on F12's staging-root counterexample |
| holistic-state-management HS-F04 (backup locality) | **Resolved** | Single-snapshot selected backup (D) |
| holistic-state-management HS-F05 (history growth lifecycle) | **Residual** | Effect rows until compaction (F05c, F13) |
| holistic-state-management HS-F06 (cold audit repeats capture) | **Resolved** for capture; **residual** for validation | G |
| holistic-state-management HS-F07 (preparation lifetimes) | **Resolved** | `preparation::Needs` (C) |
| holistic-state-management HS-F08 (ranked retention and resources) | **Resolved** (source-read) | E, H |
| holistic-state-management-integrated F01 (sparse reads hydrate the universe) | **Resolved; residual moved** | F36 |
| holistic-state-management-integrated F02 | Not examined | — |
| holistic-state-management-integrated F03 (selection bypasses budget) | **Resolved** (source-read) | E, H |
| graph-native-compiler-stage F01 (scalar effects) | **Residual** | Retrieval per root (F34) |
| graph-native-compiler-stage F02 (retrieval replays its producer) | **Resolved** | C |
| surrealdb-architecture-and-capability-leverage SA-F01 | Not examined (structure only) | — |
| SA-F02 | **Residual / largely resolved** | Durable pass cursor (D) |
| SA-F03 (definition verification twice) | **Confirmed** | F10 |
| SA-F04 (limited read prepares the whole membership) | **Resolved** for unrelated membership; **residual** for multi-owner keys | F22 |
| SA-F05 (lexical abandons selective nomination) | **Resolved** | E |
| SA-F06 (occurrence eligibility per occurrence) | **Residual** | Window batching done; repetition remains (F35c) |
| SA-F07 (bounded lifecycle crossings) | **Residual** | Ingress loops per row (§7.10); paged history |
| SA8 coordination ("keep installation ordering initially") | **Confirmed; new consequence** | F01, RC01 |
| surrealdb-capability-integration F01 | Not examined | — |
| surrealdb-capabilities F01 (native fusion) | **Resolved** | E |
| surrealdb-capabilities F02 (DEFINE API) | **Refuted for current scope** | E |
| surrealdb-capabilities F03 (mutable executable serving behaviour) | **Resolved**; cost moved | F35(a) |
| assembled-populated-execution-correction F01–F06 | Not examined (correctness); F04's retained commit observed | — |
| surrealdb-graph-store F01 (long transactions) | **Resolved** | No transaction spans computation |
| surrealdb-graph-store F03, F14 (replay is not independent) | **Moved** | F10 |
| surrealdb-graph-store F07 (sequential digests) | **Residual, not examined** | — |
| surrealdb-graph-store F10 | **Resolved** | PostgreSQL services gone |
| surrealdb-graph-store F02, F04–F06, F08, F09, F11–F13 | Not examined | Obsolete PostgreSQL-era items; no owner |
| rust-compilation-costs F01 (record-specific futures) | **Residual** | CU6 qualification open (F1) |
| rust-compilation-costs F02 (provider provenance) | **Residual** | Owner-rooted; `lctx-surrealdb` reader included (F46b) |
| rust-compilation-costs F03 (no qualified local loop) | **Confirmed** | F47 |
| rust-compilation-costs F04 (repeated drivers) | **Residual** | F45 is the runtime counterpart |
| rust-compilation-costs RC01, RC02 | **Resolved as rule changes**; implementation pending | ADR-0137 |
| rust-coroutine-compilation-amplification F01 | **Residual** | CU6 open |
| build-storage F01 | **Residual** | Reports retained without `until` |
| build-storage F02 | **Resolved (Implemented, not exercised)** | F2 |
| build-storage F03 | Not examined | — |
| storage-lifecycle-plan F01 | **Resolved in mechanism** | F2 |
| storage-lifecycle-plan F02 | Not examined | — |
| agent-effectiveness-followup F02, F04 | **Resolved** | F2 |
| agent-effectiveness-followup F03 (readiness detached from ownership) | **Resolved** for Python environments; **residual** for native-store | F48(a) |
| agent-effectiveness-followup F01, F05 | Not examined | — |
| agent-test-evidence-and-causal-analysis F01, F04 | **Resolved** as examined | F2 |
| agent-test-evidence-and-causal-analysis F02 (exact comparison) | **Resolved; source of cost** | F48(b) |
| agent-test-evidence-and-causal-analysis F03 (coarse reruns) | **Residual** | F49 |
| agent-test-evidence-integrated F02, F05 | **Confirmed implemented** | F2 |
| agent-test-evidence-integrated F01, F03, F04 | Not examined | — |
| agent-workspace-effectiveness F01, F02 | **Resolved** | F2 |
| agent-workspace-effectiveness F03–F08 | Not examined | — |
| agent-effectiveness-integrated F01–F02 | Not examined | — |
| persisted-graph-implementation F01 (cold backing disagreement) | **Resolved** (correctness); **moved** (cost onto the ordinary path) | F09 |

---

## Strengths and preservation constraints

These were checked and found adequate. Each is a constraint on the remedies above.

- **Transaction scope.** No transaction spans extraction, computation or external I/O. Every effect
  is one bounded request (graph-store F01's cause is gone).
- **Unknown acknowledgement.** Operation identity, `reconcile_request` fencing and
  committed-identity retention through seal (`lease.record_committed`) are rigorous. Any F05(c)
  change must keep them.
- **Content addressing is not equality.** `ensure_rows_owned` compares full values under guards,
  and xxh3 is used only for buckets. Guards rather than scans protect retirement under optimistic
  concurrency.
- **Pinned serving.** `RankedResults` binds session, digest, request, policy, wire, channels and
  snapshot; evicted entries refuse without rediscovery; original pages are range-bounded. Serving
  writes nothing, so publication cannot abort it at the KV layer.
- **Exact, frozen ranking.** Frozen BM25 statistics; independent exact-match branches; exact vector
  scoring with full-vector rescoring.
- **Explicit states.** `NotRequested`, `Partial/WorkLimit`, `Refused` and refusal outcomes, never
  silent truncation.
- **Assurance placement done right.**
  - Premise-memoized invariant admission within a workspace.
  - Separation of admission from diagnostic replay (`InvariantPurpose::DiagnosticReplay`).
  - Point-of-use content checks in `CanonicalSyntax::parse` and `ty_flow`.
  - The membership-completion readback.
  - Independent checks at real trust boundaries: RECORD SHA-256, `verify_export`/`verify_restored`,
    `inspection::audit`, the per-call query-vector input hash.
- **Independent evaluation.** `lctx-eval` has no production dependencies. The dev-only oracles
  (fcars, odis) are kept.
- **Bounded live state.** One batch in flight per producer; spillable runs; charged budgets and
  shared-budget composition in serving; wake-driven bridge; preparation flights with charged owners.
- **Library use.** Pyrefly's in-process use matches its intended pattern. Kosaraju SCC,
  biodivine's bounded apply with pair-product preflight, and FixedBitSet FCA are adequate. The
  bespoke ranking beats petgraph's `page_rank` for this contract.
- **Build.** Exact pins and workspace feature unification; owner-rooted provenance (relocation-safe,
  deletion-sensitive); narrow verify boundary filters; one resolved profile per verify run.
- **Tooling.** Readiness observes and never synchronizes; storage publication is bounded with short
  exclusive sections; cleanup authority is independent of exit status; log tails are bounded;
  evidence projection is lazy; outcome labels are honest.

---

## Closing summary

- **Scope.** The whole product (all crates, `python/lctx_semantics`, `python/lctx_mcp`, the
  SurrealDB service integration, the embedding client, programmatic evaluation) and the development
  loop, excluding docs publication tooling. Baseline `88f33bc0`; design tier, target purpose;
  static evidence plus the coordinator's light probes.
- **A1–A4.**
  - A1 violated (bounded: F46, F18).
  - A2 violated (narrow: F38, F37).
  - A3 satisfied.
  - A4 violated (F01–F12, F16–F25, F32, F35, F36, F40, F41).
- **Gates.**
  - G1 unresolved (F38, F20b).
  - G4 fails narrowly (F44).
  - G7 fails (F37).
  - G2, G3, G5, G6 and G8 pass on sampled evidence; G6 has one identity-scope gap that fails closed
    (F11).
  - CI-G1, CI-G2 and CI-G3 pass on sampled evidence.
- **Material findings.**
  - The global installation write and per-sub-operation effects (F01, F03–F05).
  - Size-scaled streams under fixed deadlines (F06).
  - Seal re-derivation and whole-state reserialization (F09, F10).
  - No facts reuse, and O(rows) hits (F16, F17).
  - Quadratic key seeding (F21).
  - Per-call rediscovery and eligible-domain-first vector scoring (F35, F36).
  - Per-process admission in the development loop (F41).
  - Fifty findings in all.
- **Rule impacts.** RC01–RC18; RC13 is listed but not recommended, and RC18 is conditional.
- **Bounded decision.** Revise.
- **Enclosing architectural status.** Needs revision for execution fit. The semantic architecture
  is accepted for the examined scenarios at Interface-checked/Implemented strength. Not a release
  or real-library qualification.
- **Path.** The fence-protocol decision (RC01, with its phantom inventory), then the
  attach-by-reference contract (F17). Plan creation chooses the disposition owner and presents the
  rule impacts to the operator.
