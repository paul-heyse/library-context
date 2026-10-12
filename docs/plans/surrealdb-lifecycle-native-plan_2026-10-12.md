# SurrealDB lifecycle: native access and completion

**Proposed, 2026-10-12.** Companion to the [coordinator](surrealdb-lifecycle-plan_2026-10-12.md), which owns sequence, dispositions and receipts. Baseline `1e26996d`; source/interface assessment only. This document develops [source-review](../design_review/reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md) F01/F02/F03/F05. No current-source runtime acceptance is claimed.

## 1. Foundations and intended behavior

`NativeReader::for_installed_views` already acquires a pin and validates exact view/realization/incarnation/completion inventory. Native `SelectedPayloads` prepares selected members and contributor-owned aliases. These are reusable authorities. Serving's logical `view_bindings` SQL bypasses them; adding a `view` column would preserve the wrong interpretation.

`installed_view/audit.rs` independently reconstructs complete output counts/digests but performs `LIMIT 1` member reads and scalar backing/guard/hold/endpoint checks. `originals::validate_blob` reads all physical pieces through EOF. Each typed-original audit repeats that full scan. `install_live_view` also reconstructs overlapping closures before recognizing a sealed realization. The compiler's identical installed-view cache is real counterevidence to a claim that every lookup re-audits; it does not eliminate these repeated construction paths.

`complete_blob` captures an active guard revision, reads the blob, then requires that revision unchanged. A valid peer completion or equal immutable replay can change it. Current general transaction retries do not recapture this application audit. `native_guard` and `native_predicate` are distinct: replacing the former with the latter without a writer map would not establish validity.

The target keeps complete independent observation, exact selected physical copies, immutable bytes/layout associations and current semantic admission. It changes physical access granularity and repeated preparation. No generic query engine, durable audit registry, global compile lock or concurrency cap is required.

Relevant source entries: [native selection](../../crates/lctx-native/src/selection.rs), [reader](../../crates/lctx-native/src/reader.rs), [installed-view audit](../../crates/lctx-native/src/installed_view/audit.rs), [originals](../../crates/lctx-native/src/originals.rs), [compiler preparation](../../crates/lctx-compiler-store/src/compiler.rs), and [serving evidence](../../crates/lctx-serving/src/evidence.rs).

## 2. LN01 — protected selection used by every serving consumer

The native access owner receives a held completed/publication reader and a typed selection intent. Serving continues to own library/member/path interpretation and operation semantics; native owns translation to physical realizations, protected member pointers, role/alias access and streamed rows. Prefer extending the existing selected-access surface rather than exposing raw realization SQL or making serving deserialize an admission capability.

The operation supplies exact requested views, optional scope/member/path intent and caller budget/cancellation. The native owner resolves all membership and relevant aliases from the held inventory, executes indexed bounded access and returns scoped candidates or rows through actual terminality. The caller cannot supply arbitrary logical IDs as authority. Keep the existing plain/published transaction contexts distinct where their finality contracts differ.

Migrate all production consumers in serving selection, operation choice/get/search/compare, nested module-path lookup and evidence derivation. Search beyond the review's examples for `compiler_view_member`, `view_bindings` and logical-copy fallback. Explicitly distinguish harmless logical metadata bindings from bindings used to grant canonical access. Remove obsolete SQL and helpers after their last consumer moves; do not remove metadata APIs simply because their names appear in a search.

Empty demands and empty selected views still validate scope/authority. A foreign physical copy with the same logical or nominal identity cannot enlarge the result. Aliases are admitted only through selected contributor ownership. Evidence relationships, source windows and continuations retain the publication pin and definition epoch for the full journey. Read failure/uncertainty is not a successful empty answer.

**Completion:** a fresh small pinned-input native fixture reaches operation selection, get/search/compare and evidence derivation with populated members. Add a foreign same-nominal physical copy and an alias held only by the selected contributor; independently specified expectations identify which results are eligible. Show empty selection behavior, source-abandonment pin protection and cancellation/terminal refusal. Record actual empty/refusal behavior rather than guessing the old bug's presentation. Source inspection confirms every affected production path moved. This closes F01 only after the journey works, not when the API is agreed.

## 3. LN00 — define the protected audit premise

Before narrowing completion checks, map current header/piece creation, equal replay, completion, holds, deletion/retirement, incarnation recreation and privileged maintenance writes. Use current source and fresh interleavings, not an old failure capture. Identify which guard or predicate each mutation changes and what immutability/hold protection prevents it.

The delivered contract states the installation generation/validity epoch, exact blob identity/content/length, active incarnation, layout ownership, piece continuity/count and mutation fence under which observed completeness remains true. Changing an unrelated predicate cannot invalidate unchanged bytes by accident; changing a premise that matters cannot leave a witness usable. Do not assume a blob guard revision already covers every piece-set mutation.

For ordinary operation-scoped reuse, keep exact protected owners alive and recheck their necessary frozen premises at the consuming/final-effect boundary. If the existing protocol cannot supply a valid scope, add the smallest explicit protection or continue independent readback for that case. Narrower cross-operation witnesses are LN05, not a prerequisite to fixing F01 or deduplicating safely inside one scope.

**Decision evidence:** a source writer/retirement map plus revealing same-byte replay, changed-piece, retire/recreate and cancellation scenarios. The coordinator and native owner settle the validity contract, with focused independent design advice for a changed fence. The result enables LN03/LN04 and bounds LN05; it does not itself deliver a working completion implementation.

## 4. LN02 — bounded set-oriented independent audit

Read ordered membership windows using the contribution index and stable keyset continuation. Stream through `NativeRows`/the existing physical-terminal owner rather than accumulating arbitrary results through `query_native`. Row count and actual charged bytes both constrain a window. Existing transfer-window/maximum-row policies remain authority; a preferred page size is not a new semantic input-size limit.

For each admitted window, collect exact node and endpoint keys, then fetch expected backing, guards, owner holds and endpoint metadata in bounded indexed set queries. Validate missing, duplicate, extra and foreign results by requested key and owner; a count alone is insufficient. Reconstruct independent canonical payload/role/range expectations using existing library validators, not the producer's success flag. Preserve complete declared-output counts/digests, explicit empty outputs, undeclared-output refusal and ordering across page boundaries.

Charge key sets, returned native values, decoding, endpoint expansion and retained output state to the caller's budget. A large supported item may need its own window or streamed subordinate chunks; high-degree endpoints are partitioned without dropping correspondence checks. An indivisible item beyond actual supported limits returns the existing explicit resource refusal. Do not make tests pass by imposing worker/job caps or by increasing allowances to hide scalar work.

Separate transport granularity from assurance granularity: one fetched batch can be checked item by item locally without making a server request for each item. Preserve final active-owner comparisons after all provisional rows and physical EOF. A late statement/trailing transport error invalidates the audit and is drained/reported by its actual owner. No elapsed-time threshold is a completion criterion.

**Completion:** source shows set-oriented crossings, not a batch wrapper around scalar calls. A fresh multibatch case includes a large-but-supported row and high-degree relation; corruption cases remove/duplicate/substitute backing, holds or endpoints, alter ordering/digest and add undeclared output. Empty outputs stay checked. Inject a late terminal error on this route. Exact expected membership is independently specified; a second run of the same transformation is not the oracle. F02 remains open if necessary full checks were simply skipped.

## 5. LN03 — reuse unchanged observation within a protected scope

Introduce only the internal operation/preparation context needed to share verified observations under live owners. It owns its reader/cancellation/budget and exact held references; it cannot outlive their validity or become a portable eligibility claim.

Deduplicate whole-blob observation by exact blob identity and protected incarnation/premise. On first use, independently read pieces to EOF and check continuity, count, length and content. Later typed chunks still validate their source association, exact layout, range/hash and relevant holds, but consume the already verified whole-blob result. Same bytes with different source identities/layouts retain both associations. Empty blobs also have explicit verified completeness.

For overlapping dependency roots, share observations of the same exact physical contribution/immutable backing under this context. Each root still validates its own descriptor, selected mapping, aliases, holds, complete output inventory and final completion. Reusing a contributor's bytes does not reuse a different root's admission or omit unrequested outputs required by its closure. Preserve existing identical-view caching and do not mistake it for cross-root proof.

Prefer an early valid sealed-realization reuse branch only when its exact protected completion is sufficient for the property being consumed. Cold import, mismatched definitions, missing ownership or changed incarnation re-establish affected checks. Do not let the early branch trust a completed flag or hash in place of observation. If a frozen premise changes while the context lives, invalidate the affected observation and follow the owned revalidation path; unrelated observations may remain reusable.

**Completion:** source plus a minimal observation-count control demonstrates one whole-blob verification per valid protected scope across many chunks and overlapping roots. Counts expose removed duplicate work, not a throughput claim. Distinct layouts/ranges remain independently checked. Missing/corrupt pieces, changed association, extra output, cold reconstruction and ABA still refuse or perform fresh validation. This mandatory scoped correction closes the repeated-work portion of F03; LN05 must separately decide the review's broader reuse direction.

## 6. LN04 — equal-peer completion converges safely

Use recapture and independent revalidation of a changed same-incarnation audit premise as the default correction. Do not require a new retained witness merely to accept two equal peers. Classify audit-premise change as a typed internal retry condition; do not parse a human SurrealQL error string or retry all semantic conflicts indiscriminately.

An attempt captures the active incarnation/premise and exact layout, validates bytes, and submits its final effect through the existing native request owner. If the premise is conclusively changed but the operation is still permitted and the same incarnation remains active, recapture and revalidate the current layout/bytes before another final effect. The now-valid completed blob converges without rewriting its immutable declaration. A different incarnation, changed immutable content, missing layout hold or retirement is a refusal, not a new object to adopt.

Never issue a new final effect while the prior issued effect is remotely uncertain. Resolve it through its original request identity and owner. Retry remains subject to existing operation cancellation/deadline/conflict policy; a continuously changing premise reports that condition explicitly rather than looping outside ownership. This is internal operation retry, not a scheduler or replay of abandoned work. A canceled waiter drains its submitted owners and cannot cancel a successful peer's independent ownership.

The final implementation may avoid redundant readback after equal completion only if LN00 establishes an exact independently earned frozen witness for that branch. Such an optimization is not required for convergence and cannot delay the simpler correct recapture route. Merely moving the completed check before revision comparison is prohibited: it accepts mutations the earlier audit did not observe.

**Completion:** barrier-controlled actual native peers audit the same valid blob, one commits, and the other converges. Cover equal header/piece replay, different source layouts, zero-length blob, invalid piece mutation, retirement/ABA and cancellation during recapture/finalization. No test serializes all compiles or caps their normal concurrency. Confirm uncertain submitted effects retain their identities. This closes F05 independently of whether LN03 reduces its frequency.

## 7. LN05 — broader completeness witness decision

Mandatory decision, optional mechanism: determine whether operation-scoped reuse leaves a real repeated-work consumer needing a retained physical-completeness witness. Existing completed-view/admission mechanisms are the starting point; do not invent a second proof cache.

Compare current held completions plus scoped observation against carrying independently verified physical completeness across compatible owners. An admissible witness binds exact selected physical mappings, contribution descriptors, complete inventories, blob/layout identities, installation generation, definition/codec/validity epoch, incarnations and the mutation assumptions actually needed. Current semantic eligibility remains separately earned. Portable import/restore cannot deserialize authority into existence.

Source inspection settles writer coverage where possible. If concurrency/exclusion remains uncertain, use LD02's owned target for the minimal relevant interleaving. The result is one of: use an existing equivalent completion; implement a narrower retained witness with complete invalidation/consumer migration; or retain scoped revalidation because no useful safe consumer justifies more machinery. Record rationale and a concrete revisit trigger for the last outcome. If adopted, implement and pass warm/fresh-owner reuse, stale/corrupt witness, changed definitions, lost holds and ABA controls before LC01.

No decision requires old captures, a performance benchmark or historical fixture reproduction. Fresh current semantic admission, detached restoration and independent actual backing observation remain preservation constraints. The coordinator owns the decision's execution status and receipts.
