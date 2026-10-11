# Efficiency alignment across the codebase: evidence

**Question.** What material opportunities exist to bring library-context's physical
realization into line with FP-07/A4 and the *Heuristics for Efficient Architecture* (H1–H28)?
The scope is the product plus the development loop, at design/target tier.

**Consumer.** The principal review,
[`design_review_efficiency-alignment-codebase_2026-10-11.md`](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It owns the combined judgment, finding IDs, verdicts and rule impacts. Everything in this
folder is supporting evidence. Local labels such as `A-01`, `V03` and `R5` are not finding IDs.

## Method

1. **Plan.** The operator approved the review plan on 2026-10-11 (`plan-design-review`) and
   decided:
   - **Scope:** the product plus the development loop, excluding docs publication tooling.
   - **Evidence:** static review plus light probes only, meaning no build, service, fixture or
     SurrealDB query.
   - **Challenge stage:** none built in; the operator will run a separate audit.
2. **Operation map.** One read-only `code-mapper` (Opus override) produced
   [operation-map.md](operation-map.md). It records facts only, and it contains one known error,
   correction V02.
3. **Inquiries.** Eleven supporting inquiries ran against the [inquiry briefs](inquiry-briefs.md):
   - nine `design-reviewer` inquiries: A, B, C, D, E, F1, F2, G and H;
   - two `library-research` inquiries: L1 and L2.

   All were read-only. Each was published verbatim under a provenance header.
4. **Coordinator verification.** The coordinator ran checks on the decisive source (below),
   [light probes P1–P9](probes/README.md) and the reconciliation (below). One correction was
   sent to inquiry F1 while it was running (V03).
5. **Principal review.** A fresh `design-reviewer`, uninvolved in steps 2–4, re-read the decisive
   evidence and wrote the principal review.

**Baseline.** `main` at `88f33bc0dd8bc67ef9291115c3a487b5f54bcb57`, 2026-10-11. Production
source is clean at that commit. Uncommitted edits from another session were in:
- `STATUS.md`;
- `crates/lctx-publisher/tests/{publication,stream_search}.rs`;
- `docs/plans/persisted-graph-execution-plan_2026-10-07.md`.

Inquiries D and E also observed uncommitted serving-side edits in
`crates/lctx-surrealdb/src/{derived_search,reader,selection}.rs` and in
`docs/design/sections/synthesis-and-serving.md`. They read the HEAD content.

## Index

| Document | Role |
|---|---|
| [inquiry-briefs.md](inquiry-briefs.md) | Common brief, calibration safeguards, eleven inquiry briefs |
| [operation-map.md](operation-map.md) | Facts-only trace: cold compile, warm recompile, one serving sequence, one fixture-backed verify boundary |
| [A: acquisition, providers and producer streaming](acquisition-providers-and-producer-streaming.md) | `cpg-extract`, `cpg-flow`, acquisition, producer write path |
| [B: compiler read path and catalog-profile stages](compiler-read-path-and-catalog-stages.md) | `cpg-core` workspace/read loop, DataFusion provider, stage units, admission |
| [C: analyses, reuse/invalidation and embedding](analyses-reuse-and-embedding.md) | Upper analyses, product reuse layers, embedding realization |
| [D: native persistence, publication and state](native-persistence-publication-and-state.md) | Effect protocol, publication/seal, attach, lifecycle |
| [E: serving, retrieval and agent interface](serving-retrieval-and-agent-interface.md) | `lctx-serving`, PyO3, MCP, search channels |
| [F1: Rust build and test composition](rust-build-and-test-composition.md) | Profiles, crate closures, build scripts, test structure |
| [F2: verification, evidence and environment tooling](verification-evidence-and-environment-tooling.md) | `verify`, fixture, evidence capture, storage lifecycle |
| [G: assurance, identity and observation](assurance-identity-and-observation.md) | Checks, digests, readbacks across components |
| [H: capacity, concurrency and live working set](capacity-concurrency-and-working-set.md) | Runtimes, admission, memory envelope, conflict exposure |
| [L1: data, persistence and boundary libraries](data-persistence-and-boundary-libraries.md) | SurrealDB 3.3, DataFusion/Arrow, serde_arrow, PyO3, moka, HTTP |
| [L2: analysis and compute libraries](analysis-and-compute-libraries.md) | Pyrefly, Ruff, ty, salsa, graph and reasoning kernels |
| [probes/](probes/README.md) | Light probes P1–P9 (Measured or historical-receipt strength as labelled) |
| [audit-handoff.md](audit-handoff.md) | Context for the operator's separate independent audit: facts that are easy to get wrong, the coordinator's view of where the review is weakest, unexamined breadth. These are leads, not conclusions. |

## Coordinator checks

These checks verified decisive source on 2026-10-11 at the baseline. Each is labelled with the
evidence vocabulary of principles §D.

| Check | Candidate(s) | Source read | Result |
|---|---|---|---|
| V01 | L1-01, H-01, D-01 | `lctx-surrealdb/src/control/authorization.rs::ERA_GUARD`, used in `begin_attempt_in`, effect intent and payload; `control.rs::run_transaction`; pinned `surrealdb-kvs-rocksdb-3.3.0/src/{owned_tx.rs, lib.rs, cnf.rs}` | **Interface-checked.** Every effect transaction reads `native_installation:current` with `FOR UPDATE` and writes `control_revision+=1`. The engine is an `OptimisticTransactionDB`, and `Busy` becomes `TransactionConflict`. The default `SyncMode::Every` applies. Retries are unbounded, with only `yield_now`. How often conflicts occur is unmeasured. |
| V02 | Operation map §1.3 | `cpg-extract/src/ty_flow.rs` contract; `cpg-core/src/facts.rs` profile filter | **Correction.** TyFlow declares `profiles: [Behavioral]`, so five providers run under catalog, not six. |
| V03 | Probe P4 | `build/runs/20261011T071240.860Z-5e33c9/summary.json`, `junit.xml` | **Correction.** That run was `interrupted`. Its 32 `providers:extract` cases were aborted with signal 15, so the uniform ~280 s is the instant of cancellation. P4's per-process-preparation hypothesis is withdrawn, and F1 was told while running. F1's independent F1-01 rests on test source, not on P4. |
| V04 | B-01 | `lctx-surrealdb/src/compiler_provider.rs::SupportedFilter::driver`, `atomic_scope_field` | **Interface-checked.** Only atomic scope fields become drivers; `id` predicates lower to residual SQL. That full-run iteration follows for SQL predicates comes from B's reading of `scan_bound_rows_inner`, which was not independently re-read. |
| V05 | L2-01, L2-03 | `cpg-extract/src/diagnostic_records.rs` (`check(Noqa::Enabled)` and `check(Noqa::Disabled)`, each calling `check_path`); `ruff_context.rs` (`observe_parsed`); `pyrefly_stage.rs` dependency loop | **Interface-checked.** There are two `check_path` traversals plus one observation traversal. Dependency waves call `transaction.run(…, None)`, which uses no shared pool. |
| V06 | E-06 | Pinned `surrealdb-core-3.3.0/src/exec/physical_expr/subquery.rs::ScalarSubquery`; `rg` for hoisting in `src/exec` | **Interface-checked, with a limit.** In the 3.3 exec engine the subquery plan runs per outer row, and no hoisting was found. Whether these serving statements use the exec engine or the legacy compute path (`exec/plan_or_compute.rs`) is not settled. |
| V07 | A-01 | `lctx-surrealdb/src/compiler.rs::{write_batch_inner, persist_original_chunk, ensure_immutable_rows}` | **Interface-checked.** Original persistence calls `ensure_immutable_rows` once per artifact chunk, which means once per file of up to 1 MiB, not once per 128-row window. |
| V08 | A-02 premise | `contributes:` declarations; facts provider `uses!`/inputs | **Interface-checked.** Four facts providers contribute to `assembly::vocabulary()`. No facts provider consumes a vocabulary relation, and normalization and upper stages contribute nothing. |
| V09 | C-05, C-06, C-07 | `scripts/surrealdb_service.py` (`"reuse"` config); `product_cache.rs` (`math::sum(bytes)` scans); `compilation_product.rs::ProductSection { bytes: Vec<u8> }` | **Interface-checked.** The per-root cache is attached by default, insertion re-sums every entry, and section bytes have no bytes adapter. |
| V10 | D-04 | `lctx-publisher/src/native_publication.rs::Publication::seal`; `derived_search::reconcile_search` | **Interface-checked.** The seal recomputes the completed state, runs a full `Loader::reconcile`, then `reconcile_search`. `reconcile_search` is the same `lower` that `materialize_search` already ran and compared ("independent cold lowering"), plus `lexical_stats::reconcile`. |
| V11 | C-01, A-04, D-09 | `compiler.rs` attach lookup (`contribution.attempt.admitted=true`); `mark_attempt_admitted` | **Interface-checked.** Reuse is admitted-only, and facts providers have no reuse path. |
| V12 | G-01 | `rg` for `__request`, `.request`, `"request"` over `crates/{lctx-surrealdb,lctx-publisher,cpg-core}/src`, tests excluded | **Interface-checked within that search scope.** The production comparisons of `request` (intent `$__prior`, payload `$__operation`, reconciliation) all compare the stored value with the same process's `$__request`. No independent comparator was found. `history.rs` binds retained targets, but its callers are tests (per G). |
| V13 | Review F38 (P-E1, run after the principal review at its request) | Pinned `surrealdb-core-3.3.0/src/exec/plan_or_compute.rs`; `rg -i subquer` over `src/legacy` | **Interface-checked, still limited.** Expressions are planned by the exec engine, and fall back to legacy `Expr::compute` only on `PlannerUnimplemented`/`PlannerUnsupported`. The exec engine's `ScalarSubquery` runs per outer row (V06). No subquery caching or hoisting was found in the legacy path. Per-row evaluation is therefore the best-supported reading on either path, but `EXPLAIN ANALYZE` remains the settling control. This does not change the review's F38 judgment. |
| V14 | Review F16 premise (L2 probe 3, after the review) | Pinned Pyrefly `63cda07` tests | **Interface-checked.** Upstream `pyrefly/lib/test/state.rs` and `pyrefly_lsp_test` construct `State` with `TEST_THREAD_COUNT = NumThreads(3)` (`pyrefly_util/src/thread_pool.rs`) against fixed expectations. This is weak support for thread-count neutrality of Pyrefly's outputs. Set-determinism of the lctx facts emitted from them still needs a replay control. |
| V15 | Review §10.4 item 2 (D's P-D1 log) | `build/runs/20261008T225907.834Z-dd7ee2/verify/store-rust.log` | **Resolved as deliberate controls.** The two `Transaction conflict: Resource busy` matches are panics in `cache::tests::rocksdb_grpc_commit_conflict_has_the_source_confirmed_retryable_reason` and `cache::tests::lost_acknowledgement_reconciles_actual_committed_winner_without_replay` (2026-10-08). They are conflict controls, not escalated production contention. F01's exposure is unchanged: retries are silent. |

## Reconciliation

These are the coordinator's notes on where inquiries agree, disagree or depend on each other.
They point the principal reviewer at evidence; they are not verdicts. Where inquiries
disagree, both positions and the deciding evidence are kept.

- **R1. The installation fence.** Covers L1-01, H-01, D-01, the A-03/A-05/D-02 seams, and
  H-03.
  - All three inquiries agree on the mechanism, which V01 confirms.
  - **Correction to L1-01.** L1 said one compile's eight-way native concurrency collapses.
    H-01 and D-01 independently established that effects *within one compile* are strictly
    sequential: providers run in order, each batch waits for its acknowledgement, and no join
    runs outside tests. `NativeCalls`' `Semaphore(8)` never binds on the facts path (A-05). The
    contention is therefore *cross-process*:
    - concurrent fixture tests in the shared `validation` database (S10);
    - concurrent compiles (S02);
    - reader-pin acquisition and release during publication (S05, S06).
  - **Corroboration.** D cites an HS6 receipt in which a control failed because another control
    moved the global `control_revision`.
  - **Remedies.** Two alternatives:
    - L1 proposes a striped fence of K lanes, where closure writes every lane.
    - H and D converge independently on read-only fencing for ordinary effects plus a two-step
      fence or cut, with lane striping only where a scan cannot be split.

    Both must re-derive every predicate-scan consumer's phantom partner. H names the
    counterexample: a backup hold committing just after a retirement.
  - **Complementary remedies.** These are not alternatives to the fence change:
    - one guarded effect per window (A-03, D-02);
    - set-based payloads (L1-02);
    - a single-transaction effect protocol (D-03);
    - bounded backoff with a retry counter (H-03).

    D notes that the larger window transactions widen conflict windows unless the fence is
    changed first.
- **R2. Facts reuse.** A-04 and C-01 describe the same cause.
  - L2 settles the library premise: there is no library-owned cross-process incrementality at
    the pins. Pyrefly's `State` cannot be serialized, and persisting ty/salsa is not viable.
    Reuse must therefore be owned by lctx and keyed on the captured binding plus the provider
    build digest.
  - C-02 and C-03 add over-keying from identity composition and profile (C's RC-01 and
    RC-03).
  - A suggests a further refinement: one contribution per (provider, input).
- **R3. Vector route and HNSW.** Covers L1-04, E-04 and E-05.
  - L1-04's claim that HNSW has no consumer is **corrected** by P5: the evaluation worker
    `scripts/programmatic_native.py` runs a native ANN check.
  - E argues that approximate HNSW results on the *shared* `vector` table depend on unrelated
    publications' inserts. That conflicts with §11.2 frozen ranking and CI-13, so brute-force
    exactness is justified.
  - L1-04's option (b), adopting KNN, therefore conflicts with that guarantee unless a
    per-publication index existed, and none is available per table.
  - **Shared diagnosis.** The design text contradicts itself. The index is maintained with no
    serving reader. The evaluation control qualifies a route that serving does not use.
  - **Efficiency route.** E-04's score-ordered lazy eligibility.
- **R4. Constant identities.** E-01 and F1-05 describe the same defect: `operation_definition`
  re-hashes about 9 MB of embedded source and regenerates schemas on every key and page. The
  remedy is the same for both.
- **R5. Multi-owner explicit-key lookup.** L1-05 and B-02 describe the same diagnosis.
  - **Their remedies conflict.**
    - L1 proposes a multi-get of the owners × keys deterministic IDs. That is the owner×key
      Cartesian expansion that native-efficiency F01 rejected.
    - B proposes merging against the cached membership run, or owner-restricted batched probes,
      with no Cartesian expansion.
  - B adds retained-history coupling: the `member_keys` index returns every contribution that
    ever held the key.
- **R6. Selective reads.** B-01 (`id` predicates iterate the whole membership run per grain)
  and B-07 (id-only reads re-fetch payloads). Checked as V04.
- **R7. Unrequested profile work.** B-03 (stage units and per-scope NotRequested coverage) is
  consistent with C, which found kernel preparation already skipped. B-04 reports the expected
  domain rebuilt about 15 times.
- **R8. Identity and admission scans.** B-05 (row-by-row completed-state hash) and B-06
  (O(graph) lookup). P8 receipts show `artifact_manifest` at 65–89 s on recent fixture
  journeys, without attribution.
- **R9. Streams held across effects under fixed deadlines.** H-02 covers `seal_staging`,
  attach and completion. P-H1 adds weak support: one streamed query exceeded the 20 s timeout
  on 2026-10-10. D-06 covers the same `seal_staging` stream from the ownership side.
- **R10. External sorter.** L1-03 reports a 1 MiB run cap, pairwise merge and an eager
  tempdir. E uses `SortedRows` several times per family per request.
- **R11. Serving.** E-01 to E-10. E-06 depends on V06's unresolved engine path.
- **R12. Development-loop timeouts (S10) have three candidate causes.**
  - Repetition: F1-01, about 29 tests each repeat one fixture compile.
  - CPU oversubscription: F1-02 and H-04.
  - Store serialization: H-01 and D-01.

  Their split is unresolved (F1's P-d needs a fixture run).
  - **Rule pressure.** F1's RC-F1-a (per-test weighting), H's RC-H2 (a capacity-derived memory
    envelope) and F2's conditional RC-2 all press on the "never constrain test parallelism"
    rule.
  - **Other dev-loop candidates:** F2-01 to F2-05; F1-03 and F1-04 (crate coupling, with P9
    confirming the single `cpg-extract → cpg-core` edge); F1-06 (two optimized profiles). F1's revised report also refutes the "second non-fresh copies" question as distinct oracle-dependency versions under `odis`, not a feature-unification failure.
- **R13. Analyzer extraction.** L2-01 to L2-05. A's parse counts agree.
- **R14. Originals.** A-01 (per-chunk effects, V07), A-07 (revision-scoped storage), A-09
  (verification placement), A-10 and A-11.
- **R15. Cancellation.** H-06: no signal reaches the compile's drain, so attempts are left
  open (S07).
- **R16. Attach by reference.** C-04 and D-07 are the same cause; D-08 (view re-registration)
  is its companion.
- **R17. Seal re-verification.** D-04, checked as V10. B-05 overlaps on the completed-state
  hash, and G owns the independence judgment.
- **R18. Republishing unchanged content.** D-05. It needs a derivation-implementation identity
  in the handle.
- **R19. Ownership transfer.** D-06.
- **R20. Recovery.** D-09. Admitted-only reuse contradicts ADR-0145's recovery sentence.
- **R21. Reclamation.** D-10. Content released by unadmitted attempts has no reclamation route.
- **R22. Same-process seal and cold validation on the ordinary path.**
  - These overlap:
    - G-05 and D-04 make the same diagnosis (V10);
    - G-04 (backing validation runs twice) and B-05 (completed-state hash) describe one mechanism from two sides;
    - G-06 (durable checked premises) is the reuse counterpart.
  - Combined, the remedy keeps full independent reconstruction at the cold trust boundaries (import, restore, export, audit). It carries admission's identities through same-process publication, with one post-write readback per derived family.
  - **Preservation constraint.** D and G both stress keeping the originals byte readback on the first publication. G notes that the admission invariant `artifact_chunk_content` already reads those bytes, so the seal's rehash is the duplicate.
- **R23. Write-path assurance.** G-01: the effect request digest is computed twice per effect, and production has no independent comparator. V12 is a search over `crates/{lctx-surrealdb,lctx-publisher,cpg-core}/src` excluding tests: every reader compares the stored `request` with the same process's `$__request`. G-02: the intent transaction ships the full payload bindings, so each payload crosses gRPC twice. G-03 and A-08: the store re-establishes `Batch<R>`'s canonical form and decodes rows again.
- **R24. Capture verification placement.** G-08 and A-09 make the same diagnosis.
- **R25. Observation volume.** G-07: per-root phase events are logged at INFO. A historical receipt shows 91% of `lctx_phase` lines are `borrowed_cpu`.
- **R26. Tokenizer TOCTOU.** G-09: tokenizer assets are verified from one read and loaded from another.
- **Constant identities (R4)** also appear in G §4, and fixture digests (F2-01) appear there too.

## Coverage limits

**No end-to-end measurement was made.** Every magnitude is either a static derivation (round
trips and passes counted from code) or a historical receipt attributed to its run. No speedup
is claimed.

**Not run, because they need a build, the service or a fixture:**
- the per-inquiry probes that need a run: F1 P-d, D P-D3, E P-E1 `EXPLAIN ANALYZE`, and B's
  planning-only control;
- any real-library (FastMCP) compile;
- any SurrealDB `EXPLAIN`.

Unexamined breadth recorded by the inquiries themselves is listed in each report's section 1.
