# Handoff: independent audit of the 2026-10-11 codebase efficiency-alignment review

**Audience.** An independent auditor (the operator plans a Codex audit) who will audit the
[efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
The audit is expected to produce a comparative analysis of that review:
- **corrections** to what it states;
- **improvements** to what it specifies: diagnoses, corrective contracts, priorities, order and
  rule impacts;
- **additional efficiency items** it missed.

**What this is.** Context for planning your own review. It does not tell you what to conclude.
Each statement is either a fact about how the review was produced or an explicitly labelled
coordinator opinion. Treat the opinions as leads: the review may be right where this note doubts
it, or wrong where it trusts it.

**Author.** The coordinator of the original review, 2026-10-11. The coordinator wrote the
briefs, ran the light probes and reconciled the inquiries, but did not write the principal
review's judgment.

---

## 1. What exists and where

| Artifact | Path | Notes |
|---|---|---|
| Principal review | `docs/design_review/reviews/design_review_efficiency-alignment-codebase_2026-10-11.md` | Written by a fresh `design-reviewer` that took no part in the inquiries. It owns finding IDs, verdicts and rule impacts. |
| Evidence index | `docs/design_review/evidence/2026-10-11_efficiency-alignment-codebase/README.md` | Method, coordinator checks V01–V15, reconciliation R1–R26, coverage limits |
| Inquiry briefs | `…/inquiry-briefs.md` | What each inquiry was asked; use it to see what was *never* asked |
| Operation map | `…/operation-map.md` | Facts-only trace. One claim is known to be wrong (V02: TyFlow does not run under catalog). |
| Eleven inquiry reports | `…/{acquisition-providers-and-producer-streaming, compiler-read-path-and-catalog-stages, analyses-reuse-and-embedding, native-persistence-publication-and-state, serving-retrieval-and-agent-interface, rust-build-and-test-composition, verification-evidence-and-environment-tooling, assurance-identity-and-observation, capacity-concurrency-and-working-set, data-persistence-and-boundary-libraries, analysis-and-compute-libraries}.md` | Verbatim outputs under provenance headers. Local labels (`A-01`, …) are not final IDs. F1 is its *revised* report, issued after a mid-run correction. |
| Probes | `…/probes/` | P1–P9; see the probe README for each one's strength and limits |
| Standard | `docs/design_review/design_principles/` (`standard.toml`: core 3.3, heuristics 1.0, code-intelligence 1.5, `binding/library-context.md`) | The only judging criteria, together with the functional target (DESIGN §1.1, product §14) |

## 2. How the review was produced

### 2.1 Operator decisions (2026-10-11)

- **Full re-assessment.** About 33 topical efficiency reviews since 2026-10-05 left roughly 96
  disposition rows, mostly in `docs/plans/persisted-graph-execution-plan_2026-10-07.md` §8. Every
  "implemented", "focused Tested" or "closed" disposition was treated as a lead and as contrary
  evidence, never as a conclusion.
- **Scope.** The product, meaning all crates, `python/lctx_semantics`, `python/lctx_mcp`, the
  SurrealDB service integration, the embedding client and programmatic evaluation; plus the
  development loop. Docs publication tooling was excluded.
- **Evidence mode.** Static source, documents and library skills, plus light probes that need no
  build, service, fixture or SurrealDB query. SA9 qualification was in progress on the shared
  stable service at the time, so nothing could touch it.
- **No built-in challenge stage.** The skill's fresh principal reviewer is the independence step,
  and this audit is the separate challenge.

### 2.2 Grounding

The review was judged against the standard and the functional target. It was **not** judged
against ADRs, §B decisions, plans, AGENTS.md policies, pins or operator memory, which are the
current design. That includes the choice of SurrealDB itself and the "never constrain test
parallelism" rule. Every change the review would make to such a rule is a `review#RCnn` rule
impact, effective only after the operator confirms it in plan creation. The disposition owner is
"not yet scheduled".

### 2.3 Pipeline (13 agents)

1. **Operation map.** One read-only `code-mapper` with an Opus override.
2. **Inquiries.** Eleven parallel, read-only inquiries against a common brief with 12
   calibration safeguards drawn from the pse-arrow audit:
   - nine `design-reviewer` inquiries (A, B, C, D, E, F1, F2, G, H);
   - two `library-research` inquiries (L1, L2).

   Boundaries followed responsibilities. G (assurance and identity) and H (capacity and
   concurrency) deliberately cut across components.
3. **Coordinator verification.** Checks V01–V12, probes P1–P9 and reconciliation R1–R26. One
   correction (V03) was sent to F1 mid-run.
4. **Principal review.** A fresh `design-reviewer` re-read the decisive evidence.
5. **Publication.** The coordinator published the review and evidence without changing the
   principal's judgment. The coordinator then ran the principal's optional follow-ups (V13–V15).
   None changes a verdict, and they are recorded only in the evidence README.

### 2.4 What was actually measured

| Probe | What | Result | Strength |
|---|---|---|---|
| P1 | SHA-256 of the 368.7 MB server executable | 0.155–0.161 s per pass | Measured, warm cache |
| P2 | Storage-lifecycle registry parse | 700 objects, about 6 ms | Measured |
| P3 | Boundary time outside steps (historical summaries) | Median 1.1–2.8 s for fixture boundaries; the steps take minutes | Historical receipt |
| P4 | JUnit decomposition | `model:rust` 128.6 s step against a 2.7 s suite. The ~280 s `providers:extract` durations were **SIGTERM aborts** (V03). | Historical receipt; corrected |
| P5 | KNN consumers, conflict text, the parquet path | KNN only in `scripts/programmatic_native.py`; parquet only via the DataFusion feature | Static |
| P6 | Server journal | One 20 s streamed-query timeout (2026-10-10) | Read-only; short journal |
| P7 | JSON integer-array expansion | ×3.31 on one sample | Measured |
| P8 | `lctx_phase` receipts across run logs | Single-test small-fixture journeys: `artifact_admission` 109–162 s, `artifact_manifest` 65–89 s, `native_batch_write` 67–76 s, `publication_seal` about 63 s | Historical receipt; unattributed |
| P9 | `pyrefly` path to `lctx-publisher` | Single `cpg-extract → cpg-core` edge | Static |

No real-library (FastMCP) compile ran, no SurrealDB `EXPLAIN` ran, and no end-to-end
measurement was made. **No speedup is claimed anywhere.**

## 3. Facts that are easy to get wrong

Check these before relying on the opposite.

- **The catalog profile runs five facts providers, not six.** TyFlow is behavioral-only
  (`ty_flow.rs` contract; `facts.rs` filter). The operation map says six.
- **Native effects inside one compile are strictly sequential.** Providers run in order, every
  batch write waits for its acknowledgement, and there is no join outside tests.
  `NativeCalls`' `Semaphore(8)` never binds on the facts path. The `native_installation:current`
  contention is therefore *cross-process*: concurrent fixture tests, concurrent compiles, and
  reader pins during publication. Inquiry L1 initially claimed a compile's own 8-way concurrency
  collapses, which is wrong.
- **SurrealDB 3.3 on RocksDB is optimistic.** `FOR UPDATE` registers a key for commit-time
  conflict detection and does not block. Writes to a key committed after a transaction's
  snapshot abort it with `TransactionConflict`. The repository's retry loop is unbounded with
  only `yield_now`. Retries are silent, so log absence proves nothing.
- **The ~280 s per-test durations in run `20261011T071240` are cancellation instants.** That run
  was interrupted, and 32 cases show `signal 15`. F1-01, the repeated fixture compile per test
  process, rests on the test source, not on those durations.
- **HNSW has a consumer**: the programmatic evaluation worker (`<|k,128|>`). Serving uses exact
  brute-force cosine. Inquiry E argues that exactness is *required*: approximate results on the
  shared `vector` table would depend on unrelated publications' inserts, which breaks the §11.2
  frozen-ranking guarantee. The design text contradicts itself on HNSW versus exact.
- **`reconcile_search` is labelled "independent cold lowering" but calls the same `lower`**
  that `materialize_search` ran and compared minutes earlier. It repeats the producer; it is not
  an independent oracle. The originals byte readback in `Loader::reconcile` is a genuine check,
  but admission's `artifact_chunk_content` invariant already performs it once.
- **The per-root portable product cache is on by default.** The installed runtime config always
  carries `reuse` (`scripts/surrealdb_service.py`).
- **The installed service uses an owned generation binary.** The six-digests-per-boundary path in
  the fixture tooling therefore applies. Each digest takes about 0.16 s, so this is small next to
  multi-minute steps.
- **Where the time goes.** Tooling overhead outside test steps is about 1–3 s per fixture-backed
  boundary. The minutes go to Cargo compilation and linking and to the tests themselves. The
  focused `store:rust` loop showed cargo taking 34 s to 5 min 54 s against a 1–24 s suite.
- **The analytics "second non-fresh copies"** (petgraph 0.6.5, fixedbitset 0.4.2, aws-lc-sys)
  are distinct versions pulled in by the `odis` oracle dev-dependency, not a feature-unification
  failure.
- **Admitted-only reuse.** Reuse requires `contribution.attempt.admitted=true`, and unadmitted
  attempts' products are deleted at close. That contradicts ADR-0145's recovery sentence
  (D-09).
- **Concurrent sessions.** Uncommitted serving-side edits (`eligible_context_rows` work in
  `lctx-surrealdb/src/{derived_search,reader,selection}.rs` and
  `docs/design/sections/synthesis-and-serving.md`) were in the tree during the review. The
  inquiries read HEAD. Those edits touch the E-03/E-04 seams.

## 4. Where the coordinator thinks the review is weakest

The review's decision is **Revise**: 50 findings (F01–F50) and 18 rule impacts (RC01–RC18).
These notes are leads written by the coordinator, who designed the briefs. Some are therefore
the coordinator's own blind spots.

### 4.1 Magnitudes behind priorities are unmeasured

Every diagnosis is structural.
- No FastMCP compile ran, so every "per 128-row window" or "per root" multiplier lacks a real
  volume.
- The P8 phase receipts come from single-test, small-fixture journeys at mixed revisions.

Priorities that rest most on assumed magnitude:

| Finding | What the priority assumes |
|---|---|
| F03, F04 | Round-trip latency × the effect count per window or per file |
| F21 | Quadratic grain seeding, which needs root counts |
| F32 | Per-seed universe rebuild, which needs seed and vertex counts |
| F35, F36 | Per-call fixed costs and eligible-domain size |
| F09, F10 | Attribution of the 65–89 s `artifact_manifest` receipts and the about 63 s seal |

The decision-relevant settling evidence (review §10.3–§10.4):
- a retry counter on a concurrent boundary;
- a matched-load `providers:extract` run with and without `threads-required`;
- `EXPLAIN ANALYZE` for F38 and for `seal_staging`'s `ORDER BY id` (F06/F12);
- one authorized real-library compile with `lctx_phase` spans and `/usr/bin/time -v`.

### 4.2 Remedy soundness, as opposed to diagnosis

Each item below is open; its tradeoff, risk or condition is unresolved.

1. **F01 / RC01, the fence realization.** The review prefers read-only fencing for ordinary
   effects plus two-phase fences, over L1's striped lanes. Its correctness depends on a complete
   per-consumer phantom inventory, which was not built. Check each predicate-scan consumer:
   - era cut;
   - history inventory;
   - retirement's backup-hold scan, H's counterexample;
   - history compaction, which D did not trace fully;
   - upgrade publication.

   Check each against SurrealDB 3.3's optimistic semantics. Range and index scans are not
   conflict-tracked.
2. **F10 / RC06, seal reduction.** The review trusts the process-local freeze plus the admitted
   fence. D notes that the admitted fence covers canonical tables, not memberships or
   contributions. Whether an O(1) attempt-revision witness truly covers late mutation by another
   process holding the same attempt ID needs a precise argument.
3. **F11 / RC09, the republish short-circuit.** It requires a derivation-implementation identity
   that does not exist yet. Concurrent retirement of the existing publication must also be
   excluded.
4. **F16 / RC02, facts reuse.** It rests on Pyrefly and the providers being set-deterministic for
   equal inputs (V14 is weak support), and on `captured_binding` covering every provider input,
   including the native context configuration. C flagged both as unchecked.
5. **F17, attach by reference.** It assumes readers, export and `portable_state_rows` do not
   require attempt-owned membership rows. C and D both flagged this as not traced.
6. **F22, the multi-owner lookup.** Two remedies conflict (R5). L1's owner×key multi-get
   reintroduces the Cartesian expansion that native-efficiency F01 rejected. B's merge-or-probe
   needs SurrealDB 3.3's `IN` behaviour on a trailing index column, which was not settled.
7. **F36 / RC11, the vector reorder.** It needs a clarified reading of "eligibility precedes
   ranking". The cohort may also hold many vectors with no eligible occurrence (retained
   realizations), which shifts work to the server.
8. **F41 / RC12, runner weighting and a memory envelope.** These press directly on "never
   constrain test parallelism". The review defers this until the R12 evidence arrives.
9. **F24 / RC08, domain-total `NotRequested`.** This changes a model coverage contract. No
   consumer of per-scope `NotRequested` rows outside serving was checked (`lctx-eval` was not
   checked).

### 4.3 Claims resting on inquiry searches the principal did not repeat

The review marks "relies on" per finding. The most consequential are these absence claims:
- no facts-provider reuse path (F16; searched `facts.rs`, `bundle.rs`, `pyrefly_stage.rs`,
  `ty_flow.rs`);
- no signal handling in `lctx`/`cpg-core` (F42);
- no reclamation route for content released by unadmitted attempts (F13b);
- no production comparator for the effect request digest (F05, V12);
- no serving reader of the HNSW `neighbor` index (F37; P5 found only the evaluation worker);
- F45's per-test fixture-compile counts (11, 13 and 5 tests across three case modules).

### 4.4 Verdict calls that could reasonably go the other way

- **G4 fails** on the tokenizer TOCTOU (F44). That is a narrow hidden-input path, and someone
  could call it a hygiene issue rather than a gate failure.
- **G7 fails** on the HNSW text, because §11.2 already states the exact route. A reader may
  instead see stale text in a non-governing section.
- **G1 is unresolved** (F38, plus F20's two accounting authorities). Neither divergence was shown
  to produce disagreement.
- **FP-03 and A3 are satisfied.** Per-kind physical expansion (F23, F24) was judged under A4, not
  as a composition failure.

### 4.5 Grain of the findings

Several findings bundle distinct mechanisms with separate closure obligations:
- F35 covers (a) constant identities, (b) domain rediscovery, (c) membership re-proofs and (d)
  page recomputation.
- F05 covers (a) intent bindings, (b) the double digest and (c) the durable intent record.
- F13 covers (a) recovery reuse and (b) reclamation.
- F20 covers (a) per-root crossings, (b) `math::sum` accounting and (c) JSON byte expansion.
- F48 covers (a) server identity re-proofs and (b) binary digests.

Consider whether any should be split for disposition, and whether any merge hides a distinct
obligation.

## 5. Breadth that was not examined

The authoritative lists are each inquiry's section 1, "Not read". Coordinator summary:
- Behavioral production kernels (summary worklist, transfer), the synthesis kernels, and
  `lexical_stats::prepare`/`derived_search::lower` internals.
- `restore.rs`/`backup_import`/`selected_backup` bodies, the `upgrade/*` migrations (SA-F01) and
  history compaction internals.
- `lctx-eval` internals beyond its dependency list.
- Python: `scripts/programmatic_eval.py` internals and `python/lctx_mcp` beyond the boundary
  modules.
- vLLM server-side batching.
- `compile_profile*.py`, `worktree.py`, `freshness.py`, `storage_archive.py`.
- Most of `scripts/surrealdb_service.py` (install, upgrade, backup, handoff).
- Server-side gRPC request scheduling.
- Real row volumes per relation for FastMCP: every "per W window" multiplier is unquantified.
- Docs publication tooling (excluded by scope).

Unexamined breadth is not a defect, but it prevents product-wide passes.

## 6. Working constraints and practical notes

- **Do not disturb SA9 or the shared stable SurrealDB service.** Another session owns them.
  Fixture-backed probes need coordination.
- **`just` recipes have side effects.** They enroll runs and storage records. For strictly
  read-only probes, run interpreters directly (`python3 -I …`) and say that you did.
- **Run records** under `build/runs/` are historical receipts at mixed revisions and load.
  Attribute them with run ID and date.
- **Library evidence route.** Use the library skills first: `neo4j-surrealdb`, `datafusion`,
  `serde-arrow`, `pyo3`, `python-analyzers`, `salsa`, `rust-graphs`, `rust-reasoning`, `fastmcp`.
  Then Context7, as a lead only. Then pinned or patched source under `~/.cargo/registry/src/`,
  `~/.cargo/git/checkouts/` and `third_party/`. Check versions against `Cargo.lock`, `uv.lock`
  and `docs/pins.md`.
- **Operator preferences relevant to judgment.**
  - Prefer native and library capability.
  - Aim at the best-in-class target and record conflicting rules as rule impacts.
  - Judgment over ceremony: no mandated probes or checklists.
  - Hard cutover with no legacy.
  - Qualitative heuristics, with no speed claims unless measured.
  - "Never constrain test parallelism" is a current rule, and several findings press on it.

## 7. Suggested shape for the comparative analysis

This is a suggestion only.
- Publish the audit as its own principal document, for example
  `docs/design_review/reviews/design_review_efficiency-alignment-audit_<date>.md`, with any
  evidence under `docs/design_review/evidence/<date>_efficiency-alignment-audit/`. Do not edit the
  original review.
- For each original finding, record its position: confirmed, corrected, improved, merged, split,
  re-scoped or withdrawn, with the decisive evidence.
- For each `RCnn`, record agree, modify or reject.
- Give additional findings new IDs (`A01…`) under the same finding standard.
- Report corrections to remedy compatibility and prerequisite order.
- If host capacity allows, the decision-relevant probes the review lists are where a small amount
  of execution would most strengthen the result.
