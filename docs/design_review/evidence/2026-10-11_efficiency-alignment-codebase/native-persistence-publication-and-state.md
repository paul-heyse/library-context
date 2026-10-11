# Inquiry D: Native persistence, publication and state lifecycle

**Provenance.** This is the verbatim final report of supporting inquiry D, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `D-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# Inquiry D: native persistence, publication and state lifecycle

**Baseline:** `main` at `88f33bc0`, read-only. `derived_search.rs`, `selection.rs` and `reader.rs` have uncommitted edits from another session, so I read the HEAD content and treat the diff (serving-side context eligibility) as a seam with inquiry E. I used `git show HEAD:` for the plan. Candidate labels are local (`D-nn`). "RT" means one gRPC request/response.

## 1. Scope examined and coverage limits

**Read in full or at the relevant symbols:**
- `lctx-surrealdb`:
  - `control.rs` and `control/{authorization,cleanup,retirement(head),history(collection)}`;
  - `control/schema.surql`;
  - `compiler.rs` at these symbols: `write_batch_inner`, `ensure_physical_rows`, `ensure_immutable_rows`, `persist_original_chunk`, `complete_contribution_inner`, `registered_or_install_views`, `mark_attempt_admitted`, `retain_product_identity`, `attach_retained_product`, `retain_attached_members`, `view_owners`, `completed_state_after_closure`, `state_identity_selected`;
  - `loader.rs` (`insert`, `ensure_window`, references, `NativeWindows`);
  - `reconciliation.rs` (`Loader::reconcile`, `actual_roles`);
  - HEAD `derived_search.rs` (`Expected`, `Batch`, `lower`, `materialize_search`, `reconcile_search`, `actual_search_rows`);
  - `lexical_stats.rs`;
  - `config.rs` (selection lock);
  - `upgrade.rs` and `upgrade/progress.rs` (structure only);
  - `reader.rs` (selection, `relation_bodies`).
- `lctx-publisher`: `lib.rs`, `native_publication.rs`, `definitions.rs`, `backup.rs` (backup, retire), `restore.rs::{restore, reconstruct, compare_derived_claims}`, and the head of `selected_backup.rs`.
- `cpg-core`: `artifact::admit` and `workspace.rs` (writer submission).
- `crates/lctx/src/newnative.rs`.

**SurrealDB semantics, settled at the pinned source (Interface-checked; I executed nothing):**
- `surrealdb-kvs-rocksdb-3.3.0` (`lib.rs`, `owned_tx.rs`, `transaction`, `getu`);
- `surrealdb-core-3.3.0` (`dbs/processor.rs`, `kvs/tests/select_for_update_test.rs`);
- `neo4j-surrealdb` skill SB033/SB091/SB092/SB118.

**Not read, or only skimmed:**
- retirement paging beyond `claim` and `commit_page`;
- the history-compaction internals;
- `selected_backup` export body, `backup_import`, `inspection.rs` beyond the reconcile calls;
- `product_cache.rs`;
- `prepare_canonical` internals;
- serving consumers;
- the server patches.

The SA-F01 upgrade migration was examined only structurally.

**Historical receipts used** (read-only reads of existing records; neither is a fresh run):

1. **Phase timings.** `lctx_phase` events from `build/runs/2026101[01]*/verify/*.log`. These are fixture-scale tests at mixed revisions on a shared host, with medians:
   - `publication_references` 16.6 s (n=27, p90 34.4 s);
   - `publication_seal` 16.3 s (p90 56.3 s), of which `publication_final_reconciliation` is 2.9 s, `publication_marker` 1.35 s and `publication_readback` 1.0 s;
   - `publication_search` 2.2 s;
   - `artifact_admission` 17.8 s.

   Command: grep `phase=… status="passed" elapsed_ms=` from those logs and take the median per phase. These numbers attribute no cost to a mechanism.
2. **HS6 inventory** of the `validation` database. `build/runs/20261011T004509.553Z-413c1e/hs6-history-inventory/manifest.json` (2026-10-11) shows, for 337 attempts:
   - `native_hold` 2,632,341;
   - `native_guard` 1,211,398;
   - `native_effect` 267,591;
   - `compiler_binding` 25,441;
   - `compiler_view` 20,519.

   Content tables are not in that inventory.

## 2. Complete operations examined

**Native effect** (`control/authorization.rs::{effect_for_owner, execute_owner}`). Every `effect`, `ensure_rows(_owned)`, `hold(_many)` and `guarded_effect` goes through it.

| RT | Statement | Writes to `native_installation:current` |
|---|---|---|
| 1 | Owner era read | — |
| 2 | Intent transaction | `ERA_GUARD` (FOR UPDATE + `control_revision+=1`) |
| 3 | Payload transaction | `ERA_GUARD` again |
| 4 | Outcome-ref release transaction | `control_revision+=1` again |

- `control::run_transaction` retries a retryable conflict in an unbounded loop with `yield_now`. Each retry re-sends the whole statement and its bindings, up to an 8 MiB window.
- Live state is small; the payload bindings are serialized and hashed twice (`ContentHash::of(serde_json::to_vec(&(sql,&bindings)))`).

**Typed batch write** (`NativeCompilerStore::write_batch_inner`): canonical Arrow → `codec::batch_bodies` → physical rows plus memberships. Each 128-row window then does:
- `ensure_rows_owned(anchors)`;
- `ensure_rows_owned(rows)`;
- `hold(contribution→ids)`;
- `hold_many(row→anchor/in/out/source)`.

Memberships repeat `ensure + hold`; originals do `ensure + hold + hold_many`. Inside `ensure_rows_owned`, each row does a guard FOR UPDATE, a guard UPSERT, an existence SELECT, then CREATE only if absent. `hold_many` upserts the object and owner guards again and checks that the target exists.

**Completion** (`complete_contribution_inner`): one streamed re-read of the contribution's memberships into `KeySink` digests, then a completion effect, then `registered_or_install_views`. That function re-enumerates all members of each view and ensures and holds `compiler_view_member` rows in 128-row windows, then holds `view→contribution`.

**Admission** (`cpg_core::artifact::admit`): freeze, invariants, `prepare_canonical`, a full entity/assertion scan into `FamilyHasher`, `artifact_manifest::populate` (which computes `completed_state`), then `mark_attempt_admitted`.

**Publication** (`lctx_publisher::seal_completed`). All steps are strictly sequential:

| Step | What it does |
|---|---|
| `Publication::new` | Session #4 |
| `install_definitions` | `verify_epoch`: `INFO FOR DB` plus `INFO FOR TABLE` per table |
| References | Second canonical scan; per 128 rows, `ensure("external")` (1 read + insert) and `insert(edges)` (`ensure_rows` + `hold_many(attempt→edge, edge→endpoints)`) |
| `materialize_search` | `lower` (vectors, family rows, second window stream, occurrences, each via `Loader::insert`), `Expected::reconcile` against actual selected rows, `lexical_stats::materialize` (server-side `search::analyze` of every document, write, `check`) |
| `seal` | `begin_finalization`, `completed_state_after_closure` (full state rehash with backing validation), `Loader::reconcile` (third canonical scan, physical re-derivation, roles, and **every original's bytes** re-read and rehashed), `reconcile_search` (second complete `lower` plus compare, then `lexical_stats::reconcile` = second analysis plus compare) |
| `seal_staging` | Second `verify_epoch`; `hold(publication→views)`; stream every attempt-owned hold (`ORDER BY id`) and re-hold each root under the publication in 128 windows; marker `ensure_rows`; `close_attempt` (deletes the attempt's holds page by page); readback session #5 (pin then release) |

Spillable `SortedRows` bound the live state.

**Retained-product hit** (`attach_retained_product`):
- lookup;
- `hold(attempt→source)`;
- `guarded_effect` copies the descriptor to a new per-attempt contribution;
- `hold_contribution_inputs`;
- every source membership is streamed and rewritten as a new per-attempt membership (`retain_attached_members`: ensure + hold_many per 128);
- completion effect;
- `registered_or_install_views`, which repeats the full member index again.

**Lifecycle:**
- `close_attempt`: for an admitted attempt, release the attempt's holds; for an unadmitted one, delete products, then release holds.
- Retirement exists only from publication roots (`lctx_publisher::backup::retire` → `retire_reachable`).
- The selection lock is a 10 ms `try_lock` poll with the request deadline (`RuntimeConfig::lock_selection`).
- Backup is one transaction snapshot over the selected closure (`backup_owned`).
- Restore is typed reconstruction, then the publication path (`restore.rs::restore`).

## 3. Candidate findings

### D-01: every native effect writes one global record, so all concurrent effects in a database conflict

**Cause.**
- `authorization.rs::ERA_GUARD` both reads `native_installation:current` FOR UPDATE and writes `control_revision+=1`. It appears in the intent and payload transactions of `execute_owner`, and the release transaction writes the record again.
- Installation-owned effects also write `admission_revision`.
- At the pinned engine, `surrealdb-kvs-rocksdb-3.3.0` uses `OptimisticTransactionDB` with `set_snapshot(true)`.
- FOR UPDATE maps to `getu` → `get_for_update_opt`. That only registers the key for commit-time validation; it does not block (`dbs/processor.rs`; upstream test `rocksdb_select_for_update_conflicts_with_concurrent_write`).
- A write to a key that another transaction committed after this one's snapshot fails with Busy, which surfaces as `TransactionConflict`.

So any two effect transactions whose server-side execution windows overlap conflict on that one key, whatever records their payloads touch, and the later committer is aborted.

**What the write protects.**
- The era cut (`history::cut_era`) checks "no unresolved effects, no open attempts" with predicate reads. Snapshot isolation does not conflict-register those reads (SB091/SB092).
- Writing the shared record in every effect makes the cut's own installation write conflict with any concurrently committed intent. This is a legitimate defence against write skew (DP-19, G5).
- The cost does not match that purpose. The cut is a maintenance-only operation run with admission closed, yet the materialized conflict taxes every ordinary effect.

**Critical section.** Only the overlapping execution windows of individual effect transactions serialize, typically one 128-row guarded loop. The whole compile is not one critical section.

**Consequences:**
- **S05, S06, S10.** Concurrent compiles, MCP pin acquisition (`pin_views` is three effects), publication and fixture tests in the shared `validation` database all abort each other's transactions. Each loser re-sends and re-executes its full payload with no backoff or bound (`control.rs::run_transaction`), so wasted server work and starvation are possible (H23, H24, H21).
- **Corroboration** (historical receipt, plan §9.1 HS6, 2026-10-11). A qualification control failed only because "global control_revision changed 12009→12010" through another control. The remedy was a separate exclusive maintenance admission per control (H25).
- **H22.** The counter forecloses the obvious remedy for the strictly sequential publication and write windows (§2): parallel windows would just conflict.
- Retries are invisible: no count or log (DP-21).

**Simpler conforming alternative (Proposed):**
- Make fencing read-only for ordinary effects: `ERA_GUARD` keeps `SELECT … FOR UPDATE` and the field checks, but drops the write.
- Turn the cut into a two-step owner protocol. First a transaction writes a durable "cutting" state on installation. Then a later transaction runs the predicate check and the era advance.
- Any intent with an earlier snapshot that commits after the first step fails on its registered read; any later one sees "cutting" and refuses. The second transaction's predicate therefore sees every effect that can ever commit in that era.
- Move `admission_revision` epoch allocation to its own sequence record, so installation-owned effects stop aborting attempt effects.
- Give `run_transaction` a bounded, jittered retry with a counted outcome.

**Preservation.** No effect of a closed era may commit after the cut. Pending effects must remain discoverable by the cut. Outcome-ref/compaction ordering, epoch allocation and the HS6 "rollback leaves the marker unchanged" assertions must still hold, re-expressed without the global counter.

**Uncertainty:**
- I found no production reader of `control_revision` beyond upgrade validation and test snapshots (searched `crates`, `scripts`, `python`, the `.surql` files and docs).
- The history-compaction predicates may rely on outcome-ref writes conflicting; I did not trace them fully.
- The conflict rate is unmeasured. Within one compile, most effects are issued sequentially (the bridge blocks per batch).

**Evidence:** Implemented (structure). The conflict semantics are Interface-checked at the pinned source. The test interference is a historical receipt.

**Closure:**
- The source shows no ordinary-effect installation write.
- A concurrency control: two attempts and one pin acquisition issue overlapping effects with zero retries.
- An adversarial era-cut control: an intent committed between the two cut steps is refused or seen.
- Retry counts are observable.

### D-02: one logical window write is several independent effects, with repeated guard writes

**Cause.**
- `compiler.rs::ensure_physical_rows`, `ensure_immutable_rows` and `persist_original_chunk`, and `loader.rs::Loader::insert`, issue `ensure_rows_owned`, `hold` and `hold_many` as separate effects per 128-row window.
- Each effect upserts the object's `native_guard` again, so one node row's guard is written in up to 3 transactions.
- `registered_or_install_views` adds `ensure + hold` per member window, and `retain_attached_members` adds `ensure + hold_many`.

The result is 8–16 sequential RTs per node window and about 8 per membership, view-member or derived window (operation map M2; I confirmed it in source).

**Consequences:**
- **S01, S02, S03.** Write latency is the window count times 2–4 sequential effects, in strictly sequential loops (H2, H8).
- **H18.** Each guard is written several times to protect one admission.
- Between `ensure` and `hold`, an object exists unheld. A concurrent retirement can claim shared content in that gap, and the attempt then fails on its hold (fail-closed but avoidable).
- The historical `publication_references` median of 16.6 s at fixture scale is consistent with RT-bound write loops, though not attributed.

**Alternative (Proposed).** One guarded window effect per window that, in a single transaction:
- upserts each guard once;
- creates the missing rows;
- inserts the owner hold and the reference holds;
- checks existence.

That is one protocol instance instead of 3–4, and the row and its ownership commit atomically. The window bounds (128 rows, 8 MiB) are kept.

**Preservation:**
- the full-value collision check;
- the retiring and epoch fences per guard;
- the admitted-canonical fence;
- the contribution fence;
- byte bounds.

**Uncertainty.** Larger transactions widen the D-01 conflict window unless D-01 is fixed first. The two are sequenced, not bundled. Server per-row cost is unmeasured.

**Evidence:** Implemented.

**Closure:** a source trace showing one effect per window for node, membership, view-member and derived windows, and a concurrent-retirement control on shared content.

### D-03: the per-effect protocol uses three transactions and two client hashes where one transaction could carry the same fencing

**Cause.**
- `execute_owner` commits an intent record (`native_effect` with `committed=false`) and `native_outcome_ref`.
- A payload transaction then marks it committed.
- A third transaction deletes the outcome ref.
- `effect_for_owner` also does a separate owner-era read.
- The bindings are serialized and BLAKE3-hashed twice.

**Consequence.** Every effect costs 4 RTs of control. For small effects (holds, markers, pins) the control cost dominates. Under S08, durable `native_effect` rows (267,591 in the HS6 receipt) persist until maintenance compaction (H25, H26).

**Alternative (Proposed).**
- Create the `native_effect` record with `committed=true` inside the payload transaction, reading the owner era there.
- Reconciliation of an unknown acknowledgement still works. `reconcile_request` creates the fencing record if it is absent, and a late payload's `CREATE` of the same id then fails.
- Keep explicit outcome refs only where the caller retains them (`retain=true`).

**Preservation:** exact identity of the original operation across retries, refusal of a late duplicate, closed-era refusal, and caller-retained outcomes.

**Uncertainty.** The intent record may serve recovery consumers I did not trace (drained-maintenance `drain_effects`, cleanup successors). This needs the ADR-0145 owner's confirmation. Lower priority than D-01 and D-02.

**Evidence:** Implemented (structure); the remedy is Proposed.

**Closure:** lost-acknowledgement, delayed-payload and era-cut controls pass with the single-transaction protocol.

### D-04: seal repeats the publication's own derivations and admission's checks over frozen immutable content

**Cause.** `native_publication.rs::Publication::seal` runs, after `materialize_search` has already done so:
1. `completed_state_after_closure`, a full rehash of the state tables with backing-model validation. Admission already computed `manifest.completed_state` via `artifact_manifest::populate`, and the state is frozen by the content lane and the admitted fence.
2. `Loader::reconcile`, which rescans every entity and assertion. It re-derives the physical rows with the writer's own `adapter::physical_row` and `codec::*_views`, and recomputes the family hashes that admission computed from the same stored rows (`artifact::admit` → `FamilyHasher`). It also re-derives role edges with the writer's `codec::entity_references`.
3. `derived_search::reconcile_search`, which calls the same `lower` with `loader=None` and compares against the same `actual_search_rows` that `materialize_search` compared minutes earlier.
4. `lexical_stats::reconcile`, a second full `prepare` (server-side analysis of every document) plus `check`.
5. A second `definitions::verify_epoch` in `seal_staging` (SA-F03).

Items 1, 3 and 4 run the same code on the same frozen inputs against the same stored rows. They detect only a change between the two checks, or nondeterminism.

**The concurrency argument does not hold.** Foreign derived rows written by another publication after `materialize_search` would not be fenced by this re-check either: they can equally appear after seal, and serving eligibility is by dependency membership. The guarantee belongs at read time or in derived-row identity, which the uncommitted serving diff appears to address.

**Consequence:**
- **S01, S03, S05.** Seal costs roughly a second full lowering, a second lexical analysis, a third canonical scan and a second full state hash (H20, H19, H12).
- **Historical receipts** (plan §9.1): seal reconciliation hit a 10 s native query timeout on 2026-10-10. On 2026-10-11, `publication_search` refusals came from "actual-row reconciliation".
- The 2026-10-05 graph-store F14 pattern ("replay is not independent") survives in a new form.

**Alternative (Proposed):**
- For `Admission::Completed`, keep one reconciliation of the written derived rows: `materialize_search`'s actual-versus-expected comparison, the actual-role comparison, and the lexical `check` against the expected rows already in hand.
- Keep the originals byte readback (but see D-05).
- Carry the admission-computed family and state identities through the frozen-attempt lease instead of recomputing them.
- Run `reconcile_search` and the full `Loader::reconcile` re-derivation in `inspection::audit` (already wired) and for `Admission::Restored`/`Export`, where inputs are untrusted and the check is genuinely independent.
- Verify definitions once per publication under the finalization lease.

**Preservation:**
- an independent cold check for imported or restored content (ADR-0145, TF01);
- detection of missing or extra stored rows, through one actual-row comparison per derived family;
- byte-level original verification;
- DP-19's freeze-then-publish ordering.

**Challenge.** Process-local freezing is weaker than a native fence. The admitted fence covers canonical tables but not memberships or contributions (`ensure_rows_owned` fence list). The state rehash is cheap insurance only if the freeze could be bypassed; a native "frozen" fence on those tables would remove the doubt.

**Evidence:** Implemented. The duplication is established statically; magnitudes are unmeasured.

**Closure:** the seal path shows one lowering and one analysis per publication. Seeded missing, extra and changed derived rows still refuse. Restore and audit retain the cold lowering.

### D-05: republishing unchanged or mostly unchanged content repeats the complete publication, including all original bytes

**Cause:**
- `seal_completed` has no check for an existing sealed `publication:<handle>`. Search scope: `lctx-publisher/src`, `cpg-core/src/compilation.rs`, `lctx/src/compile.rs`; no hits.
- The handle (`publication_identity`) is computable before references are written: it depends on the manifest content, the realization, the view identity, the generation and the definition epoch.
- References, search and lexical rows are content-addressed, so `ensure_rows` skips creation but every RT and comparison still runs.
- `Loader::reconcile` re-reads and rehashes **every original's bytes** from the database at every publication. Those originals are immutable, collision-checked and guarded, and were already verified by an earlier seal.

**Consequence.** S03 (unchanged inputs, patch upgrade) and S05 pay O(all derived rows and all original bytes). Growth with dependency-closure size under S02 is linear per publication (H13, H14, H19).

**Alternative (Proposed):**
1. **Identical handle.** After admission, if `publication:<handle>` exists with identical marker bytes and is not retiring, take a guarded hold on it, close the attempt and return the handle.
2. **Partial change.** Skip the originals readback for `(original id, content, incarnation)` already verified under a sealed publication. The premise owner is the immutable guarded row plus a verification record keyed by incarnation and invalidated on retirement.

**Preservation and premise (safeguard 3).** The handle today binds the SurrealQL definitions (`schema::realization_identity`), not the Rust derivation implementation: `derived_search::lower`, `codec::entity_references`, `lexical_stats`. A short-circuit therefore needs a derivation-implementation identity added to the handle or the marker; otherwise a changed lowering would wrongly reuse old derived rows. (Today such a change fails closed at reconcile as "extra" rows; that is a seam for inquiries G and E.) The short-circuit must also guard against concurrent retirement through the publication's guard.

**Uncertainty.** Byte-identical manifests across attempts are inferred from the `canonical_bindings` logic and its test, not observed.

**Evidence:** Implemented (absence of a short-circuit). The remedy is Proposed.

**Closure:** a warm unchanged recompile performs zero derived writes and no original reads. A changed derivation implementation yields a new handle, and concurrent retirement refuses or retains.

### D-06: derived ownership is created under the attempt, re-created under the publication, then deleted

**Cause:**
- `Loader::insert` holds every derived row under the attempt (`hold_many(attempt→id)`).
- `seal_staging` streams every attempt-owned hold (`SELECT VALUE object … WHERE owner=$attempt ORDER BY id`). With index `owner_holds(owner,object)`, ordering by `id` likely forces a server-side sort and materialization; this is inferred.
- It then re-holds each root under the publication in 128-row effects.
- `close_attempt` → `CleanupHolds::step` deletes the attempt holds 128 at a time.

**Consequence.** Three ownership writes per derived row (plus guard upserts and the D-01 conflict exposure) on S01 and S03's critical path. Historical `publication_seal` minus its named sub-phases leaves roughly 11 s median unattributed, which covers close and state hashing (H16, H23, H27).

**Alternative (Proposed).** Hold derived rows from the start under one per-publication staging root, which the attempt holds. The marker transaction then adds a single `publication→staging root` hold. Alternatively, keep the closed attempt as a retained root that the publication holds, instead of transferring per row. Cleanup of an unsealed attempt releases the staging root as one object.

**Preservation.** Nothing becomes reachable from a marker before the marker commits. Retirement reachability is preserved transitively. Interrupted publication leaves discoverable cleanup.

**Uncertainty.** The retirement traversal cost of one high-degree staging root needs checking against HS-F03 paging.

**Evidence:** Implemented.

**Closure:** seal's ownership writes are O(1) in derived rows. Retire/publish and crash-before-marker controls pass.

### D-07: each attempt mints new physical identities for logically identical retained content, and they accumulate under shared views

**Cause:**
- `attach_retained_product` mints `attempt_contribution(attempt, spec)` and copies every source membership to new ids (`retain_attached_members`).
- `registered_or_install_views` adds a `view→contribution` hold for each attempt's physical contributor.
- A view's identity is logical, so the view is shared, but those holds are view-owned and never released by attempt cleanup (`cleanup.rs` deletes only holds owned by the attempt or its products).
- Every admitted attempt also leaves a `native_product:{request}_{attempt}` row (`retain_product_identity`). `attach_retained_product` filters these by `request` and follows `contribution.attempt` links, so the lookup set grows with admitted attempts.

**Consequence:**
- **S03.** A reuse "hit" still costs O(rows) membership writes plus the view-member work (D-08), close to a miss's write cost (H14).
- **S08.** Each admitted recompile of unchanged content permanently adds a contribution, a full membership copy and its holds, reachable from any live publication of the same view. Storage and retirement traversal grow with the number of attempts, not distinct content (H25, H27).

**Alternative (Proposed).** Attach by reference:
- map the logical contributor to the existing completed physical contribution (`remember_contributor(logical, source)`);
- keep the existing `hold(attempt→source)`;
- skip membership copying and the new `view→contribution` hold when the view is already held.

Keyed product rows by request plus logical contributor, deduplicated, keep the lookup O(1).

**Preservation.** The source must be completed and admitted (or see D-09). Its retention is guarded. Current provenance attribution (ADR-0145 "current attribution never silently becomes an old producer's") must be carried by the attempt binding, not the copied membership.

**Uncertainty.** Whether readers or audits depend on per-attempt membership rows (`portable_state_rows`, export) needs checking. The HS6 counts (holds 2.6 M across 337 attempts) are suggestive only.

**Evidence:** Implemented.

**Closure:** after N unchanged recompiles, the numbers of contributions, memberships and `view→contribution` holds for a view stay constant, and attribution and export controls pass.

### D-08: view registration re-materializes the member index of views that already exist

**Cause.** `registered_or_install_views` always runs `prepare_memberships`, an ordered candidate sort, and `ensure_rows` plus `hold` for every `compiler_view_member`, even when `compiler_view:<identity>` already exists. It does this per completion and per attach.

**Consequence.** S03 and repeated views pay O(view rows) reads and ensure/hold effects that rewrite identical rows (H12, H19).

**Why it is done.** It is crash safety. Members are written in windows after the view row, so an existing view row does not prove its members are complete.

**Alternative (Proposed).** Commit a `compiler_view_installed` marker in the final member window, after the cardinality check. An existing marker lets later attempts skip the enumeration and only add their ownership.

**Preservation:** the declared cardinality is checked at the actual end of the stream; partial installs are resumed by the next registrant.

**Evidence:** Implemented.

**Closure:** a warm registration of an existing view issues O(1) effects, and an interrupted install is completed by the next attempt.

### D-09: recovery after interruption recomputes all completed work because reuse requires an admitted attempt

**Cause.**
- `attach_retained_product` requires `contribution.attempt.admitted=true`.
- `close_attempt` on an unadmitted attempt deletes its `native_product` rows.
- A cancelled or crashed compile before admission (S07) therefore makes every completed contribution ineligible, and the rerun replays everything.

ADR-0145 says "Recovery retains compatible completed prerequisites and re-executes missing work". The implementation contradicts that.

**Consequence.** S07 replays the whole attempt rather than the missing unit (H24).

**Alternative (Proposed).** Treat completed contributions of terminal unadmitted attempts as attachable under the same `spec` and `request` equality. The new attempt's admission still covers all views, so semantic admission is unchanged. Retain the products of abandoned attempts under a recovery hold, with a declared lifetime.

**Challenge.** A contribution from an attempt that failed admission on an invariant will fail again with identical inputs, so it is wasted but sound. Producer bugs are no more trusted than with admitted reuse.

**Uncertainty.** The admitted-only rule may encode an assumption I did not find documented: that `completed` alone is insufficient (for example, poisoning after `fail()`). That is a candidate rule impact.

**Evidence:** Implemented.

**Closure:** interrupt after normalization, rerun, and the completed stages attach; admission controls are unchanged.

### D-10: content released by attempt cleanup is never reclaimed

**Cause.**
- `close_attempt` and `CleanupHolds` delete holds and products only.
- Production retirement starts only from publication roots (`backup.rs::retire` → `retire_reachable`).
- Rows written by attempts that never publish (failed, abandoned, `--artifact-only`) become unheld and have no reclamation route.
- Search scope: `lctx/src/{main,newnative}.rs` and `lctx-surrealdb/src`, for retire/collect/sweep/unheld/orphan; no collector found.

**Consequence.** S07 and S08: the store grows with failed and interrupted attempts, and with the validation-database churn of S10 (H25, H27). Under safeguard 5, nothing may be deleted by age, so today there is no safe deletion path at all.

**Alternative (Proposed).** Before deleting an unadmitted attempt's holds, `CleanupHolds` registers the released objects as `retire_reachable` roots. That traversal already rechecks holds and incarnations under guards, so shared surviving content is retained.

**Preservation:** shared content and pinned or held objects survive; cleanup interruption stays resumable.

**Uncertainty.** This may be an intended cache ("admitted products survive"), but unadmitted content has no consumer.

**Evidence:** Implemented.

**Closure:** after an abandoned attempt, rows that only it held are retired and shared rows remain.

## 4. Opportunities and uncertainties that are not findings

- **Membership re-read at completion** (`complete_contribution_inner`). One streamed read gives a stored-state digest that the writer cannot reproduce cheaply (it would need a sorted dedup). This is proportionate.
- **gRPC sessions and sign-in.** Each `reader::connect` signs in, so there are 4–5 sessions per compile and one per MCP start. This is minor; the server-side password-verification cost is unmeasured.
- **`relation_bodies` drains the relation after its limit.** This is required for late-conflict detection (SA integration). Inquiry B/E owns it.
- **Derived-search identity versus implementation.** A lowering change over identical content yields the same handle and refuses as "extra" rows (D-05 premise). The seam belongs to inquiries G and E; the uncommitted `eligible_context_rows` work touches it.
- **Selection lock.** The 10 ms poll with a deadline is adequate for a rarely contended operator lock. Note that retirement holds it across the whole bounded retirement pass.

## 5. Strengths, preservation constraints and adequate areas

- **No long transactions.** No transaction spans extraction, CPU work or external I/O. Every effect is one bounded request, so graph-store F01's cause is gone (H23).
- **Unknown acknowledgements are handled precisely.** Operation identity, `reconcile_request` fencing and committed-identity retention through seal (`lease.record_committed`) are rigorous. Keep this guarantee in any D-03 simplification.
- **Content-addressed canonical rows** with full-value collision checks (`ensure_rows_owned`) and incarnation guards make retirement safe for shared content.
- **Backup** is a single transaction snapshot over the exact selected closure (`backup_owned`). Native `EXPORT` cannot select a record closure, and native `IMPORT` executes SQL and DDL and inserts rather than upserts (SB118). The bespoke typed restore therefore fills a real contract gap (H4), so it is adequate.
- **Durable reader pins** are needed for cross-process retirement safety. Keep them, at a lower per-pin cost once D-01 and D-03 are addressed.
- **Upgrade** now has durable pass and cursor progress (`upgrade/progress.rs`). History collection is paged.
- **The originals byte readback** is the only check binding stored bytes to the captured content hash. Keep it for first publication.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| SA-F01 | Not examined (only the Preflight-before-Declarations structure seen) | `upgrade.rs::upgrade_in`, `progress::Pass` |
| SA-F02 | Residual / largely resolved structurally | `progress.rs` pass + `after` cursor; not run |
| SA-F03 | Confirmed still present | `verify_epoch` ×2 per publication; `INFO FOR TABLE` per table (`definitions.rs::inventory`) |
| SA-F04 | Resolved for unrelated membership; relation-local full drain remains by design | `reader.rs::relation_bodies` → `sparse_relation_rows` |
| SA-F05, SA-F06 | Not examined (inquiry E; WIP diff touches F06) | — |
| SA-F07 | Residual | Paged history; set reads of incarnations (`retirement.rs::incarnations`) |
| SA-integration F01 | Not examined (E/B) | — |
| HS-F01, HS-F02 | Not examined | — |
| HS-F03 | Not examined in depth | `commit_page` is bounded per window |
| HS-F04 | Resolved (Implemented, static) | `backup_owned` single snapshot, selected closure |
| HS-F05 | Residual | Effect rows persist until maintenance compaction; D-03 |
| HS-F06, HS-F08–F10 | Not examined (G/E) | — |
| HS-integrated F01–F03 | Not examined | — |
| Unified F01, F02, F07 | Not examined | — |
| Unified F03 | Residual (seam) | Publication is view-scoped, but derived occurrences are eligible by dependency, not by publication |
| Unified F04 | Residual | No payload replay; O(rows) membership copy + view re-registration (D-07, D-08) |
| Unified F05 | Resolved, at a cost | Effect protocol (D-01, D-03) |
| Unified F06 | Residual | Logical isolation exists, but the shared installation record couples tests (D-01; HS6 receipt) |
| Unified F08 | Resolved structurally; growth residual | Holds and guards (D-07, D-10) |
| Unified-plan TF01 | Resolved at Implemented level | `restore.rs::reconstruct` typed ingress |
| Assembled F01–F06 | Not examined (correctness); F04 retained-commit observed | `seal_staging` `record_committed` |
| Graph-store F01 | Resolved (cause removed) | No transaction spans computation |
| Graph-store F03, F14 | Moved | Seal re-verification (D-04) |
| Graph-store F07 | Residual, not examined | Sequential `FamilyHasher`/`KeySink` digests |
| Graph-store F10 | Resolved | PostgreSQL services gone |
| Graph-store F02, F04–F06, F08, F09, F11–F13 | Not examined (sibling scope) | — |

## 7. Candidate rule impacts

1. **ADR-0145 / storage-and-publication lifecycle** ("Native-control version 4", effect fencing). D-01 and D-03 change how the fence is realized: a read-only guard plus a two-step cut, and a single-transaction effect. If the rule is kept, D-01 reduces to bounded retry with counted contention.
2. **ADR-0145 publication paragraph** ("Validate and reconcile … independent cold admission"). D-04 restricts cold re-lowering to restore, export and audit. If the rule is kept, remove only the exact duplicates: `reconcile_search` after `materialize_search`, `lexical_stats::reconcile`, and the second `verify_epoch`.
3. **ADR-0145 "Checked attachment may reuse retained admitted content"** against its own "Recovery retains compatible completed prerequisites". D-09 proposes extending attachment to completed contributions of terminal unadmitted attempts. If the rule is kept, correct the recovery sentence: recovery replays the whole attempt.
4. **The publication handle contract** (storage-and-publication; `native-publication/v2`). D-05 adds a derivation-implementation identity.

## 8. Proposed light probes

**P-D1: contention signal in existing logs.**
- Question: did any escalated conflict reach a test log? (Retries themselves are silent.)
- Command: `rg -l -e 'TransactionConflict' -e 'Resource busy' build/runs/*/verify/*.log`.
- My scan matched one 2026-10-08 log, which I did not inspect.
- Discriminating outcome: escalations in concurrent fixture runs support D-01's exposure; none says nothing, because retries are unlogged.

**P-D2: publication phase share by run size.**
- Question: does `publication_references` time scale with the derived-row count?
- Command: extend the §1 phase aggregation by pairing each run's `publication_references` and `publication_seal` `elapsed_ms` with the test's fixture name from the same log.
- Discriminating outcome: a near-constant per-window time across fixture sizes points to RT-bound loops (D-02); large variance with concurrency points to D-01.

**P-D3: actual-plan check for the seal transfer.**
- Question: does `ORDER BY id` with `WITH INDEX owner_holds` sort server-side?
- This needs `EXPLAIN` on the service, so it is not a light probe. I am recording it for the coordinator's later functional scope. A `Sort` node in the plan confirms D-06's inferred sort.
