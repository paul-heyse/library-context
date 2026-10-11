# Supporting inquiry: native effects, publication and lifecycle

Independent supporting `design-reviewer` inquiry, integrated by root on 2026-10-11.
It read original findings before tracing source. This is supporting evidence, not the
principal verdict. `O-Fnn`/`O-RCnn` identify original findings/rule impacts.

Consumed by the [principal audit](../../reviews/design_review_efficiency-alignment-audit_2026-10-11.md).

Baseline **B** is `2b05c6c239fb6569472751faf126f5e39f822853`; examined production
paths match historical `88f33bc0dd8bc67ef9291115c3a487b5f54bcb57`. Source observations
are **Implemented/static-inspected, 2026-10-11**; remedies are **Proposed**.
Builds/tests/fixtures/queries/service operations/sync/real-library compiles/benchmarks:
**not_run**. Source searches and direct contract reads passed as static inquiry.

Abbreviations: **C** `crates/lctx-surrealdb/src/compiler.rs`; **CTL**
`crates/lctx-surrealdb/src/control.rs`; **AUTH/HIST/RET** the `authorization.rs`,
`history.rs`, `retirement.rs` modules under `crates/lctx-surrealdb/src/control/`;
**NP** `crates/lctx-publisher/src/native_publication.rs`; **W**
`crates/cpg-core/src/workspace.rs`. Line numbers refer to B, not the live working tree.

## Complete comparison of O-F01–O-F15 and native O-F17

| Original | Independent diagnosis and decisive evidence | Remedy, countercase and ordering correction |
|---|---|---|
| O-F01 | **Confirm structural hotspot.** `AUTH::ERA_GUARD:89` locks/increments installation current; intent and payload run it (`execute_owner:182–194`). Independent owners share a write-conflict point. No retry-share/throughput magnitude established. | Read-only ordinary fencing needs a complete phantom/predicate inventory: retirement, incoming holds, backups, drain, era cut, **history page predicates**, outcome references and retirement lineage. Some partners already lock exact objects; do not assume all protection absent. Ordinary ERA_GUARD does not require admission-open, so preserve current-era maintenance authority separately from new admission. Separate revision allocation; retain owner epochs/reconciliation. Striping is an alternative, not foreclosed. |
| O-F02 | **Confirm indefinite immediate retries; qualify cancellation.** `CTL::run_transaction:169–179` retries retryable errors with `yield_now`, cloning bindings, without backoff/counters/stop policy. The future is externally cancellable. | Identity-preserving retry observation/backoff is useful independently of O-F01. Any stop must return the original request's recoverable disposition: timeout does not mean an ambiguous effect did not commit. Preserve `AUTH::reconcile_request:233–238` and `request_disposition:216–232`. |
| O-F03 | **Confirm effect multiplication.** `C::ensure_physical_rows:3038–3119` separately ensures anchors, payloads, payload holds and reference holds. Three/four is a maximum pattern; empty groups avoid calls. | Fuse payload/ownership within combined SQL/binding/payload budget while retaining collisions, admission, retiring/incarnation checks and atomic reachability. O-F01 is **not a strict prerequisite**: sequential fusion removes crossings now, though larger conflicts can cost more to retry. |
| O-F04 | **Confirm chunk-grain crossings.** `write_batch_inner:3017–3020` invokes `persist_original_chunk` per chunk; the latter windows its own 64-KiB bodies (`3122–3154`); `ensure_immutable_rows:3158–3204` separates row/hold effects. | Use a bounded iterator spanning chunks/batches, not an all-corpus vector. Preserve range/content validation and source→chunk holds. No hard dependency on O-F01. |
| O-F05 | **Mostly confirm; split protocol from local cleanup.** `AUTH::effect_for_owner:146` and `execute_owner:161` hash bindings twice; intent receives cloned full bindings at 183 though SQL needs control metadata. All effects use intent/outcome machinery. | Remove unused intent payload and redundant hashing through a trusted internal request path; retain public mismatch checks. Eliminating durable intent/outcome is a separate unresolved redesign: `CTL::drain_effects:198–222`, reconciliation, retained outcome references and compaction consume them. No production `execute_request` callers outside tests were found, but the public retained-request contract remains a legitimate countercase. |
| O-F06 | **Confirm long-lived outer streams; correct completion mechanism.** `attach_retained_product:1589–1615` holds a membership statement while writing windows. Completion streams at 3355ff, but `new_membership_keys:3564–3583` is a **read overlap check**, not a write effect. | Keyset pages or disk-backed capture can shorten statement lifetime while retaining exact immutable input, order and drainage. Process freeze alone is not a universal native witness. No failure threshold measured. Do not equate the 10-s transaction setting with a total 10-s external backup snapshot lifetime. |
| O-F07 | **Confirm serial provider/write scheduling.** `cpg-core/facts.rs::compile_facts:43–86` awaits providers/completion; `W::ProducerOutput::push:3494–3551` awaits acknowledged writes. | Schedule ready providers by dependencies; bounded write-behind must hold charges through acknowledgement and drain before completion/seal. O-F01 affects overlapping write throughput, but local extraction can overlap native I/O before that redesign. No speedup measured. |
| O-F08 | **Confirm narrow repeat canonicalization.** `domain/record.rs::Batch::with_reservation:692–738` validates/sorts/deduplicates; `domain/model.rs::canonical:748–758` decodes/sorts/rejects duplicates again. | Carry a nominal checked canonical batch keyed to relation/schema/encoding on this trusted path. Raw/imported inputs still require checking; do not imply `canonical` performs every row validation. Lower priority than crossings/full-state work. |
| O-F09 | **Confirm full-state reconstruction and repeated seal call; correct per-row claim.** `completed_state:4130–4154`, `state_identity_inner:4161ff`, `state_identity_selected:4168–4207` capture/sort/serialize state and validate backing/originals; `NP::seal:149–152` asks again. `next_batch(1)` is local state iteration, **not one backing projection per row**: pending rows/bytes flow through `admit_backing_row` and `validate_backing_batch` (`4194–4198,6256ff`). | Compositional identity requires validated completion and native immutability of every identity-bearing row. Binding revision alone is insufficient. Smaller correction: compute once, batch local iteration/serialization and compare a sound seal witness; backing projection is already batched. Keep cold import/restore/audit validation. This additional batching correction was independently reread by fresh principal and root. |
| O-F10 | **Confirm duplicate derivation, preserve useful reconciliation.** `NP::seal:146–162` repeats state, Loader and search reconciliation; `derived_search.rs::materialize_search:1455–1470`/`reconcile_search:1472–1487` run lowering/lexical analysis twice. `install_definitions:95–103`/`seal_staging:194` verify definitions twice. Loader reads originals (`reconciliation.rs:212–239`). | Remove exact duplicate derivations/checks while retaining actual stored-row validation including references/extras. Same lowering twice is not an independent semantic oracle. Removing the remaining cold boundary requires a complete native witness. `begin_contribution_inner:2769` and `bind_inner:4072` already reject admitted owners; these checks should be credited, but do not prove all generic mutation paths closed. |
| O-F11 | **Split.** No unchanged-publication shortcut is confirmed. Blanket derivation identity omission is **incorrect for ordinary compilation**: `producer_fingerprint.rs:16–78,119–191` includes native source through cpg-core's dependency; `W::execution_implementation:2216–2220`, `artifact_manifest.rs::producer_inventory:139–152`, `Manifest::content` (`domain/graph.rs:4190–4209`) propagate it. | Current publisher/derivation identity remains relevant for retained imported/restored manifests, whose provenance is intentionally old. A new handle alone does not isolate old/new derived rows sharing canonical dependencies: require derivation-key/eligibility or explicit generation cutover. A fast path verifies current definitions, exact manifest and retained reachability. “Zero original reads” can describe publication, not automatically the entire recompilation. |
| O-F12 | **Confirm per-row ownership transfer; reject retirement objection as stated.** `NP::seal_staging:252–282` recreates holds per row. But `RET::resume_retirement:146–186` charges claim and edge units, bounds outgoing pages, and can resume a high-degree root. | Staging-root ownership is viable if durable/discoverable before child effects and atomically transferred into sealed reachability. Existing retirement does not require the entire fanout in one effect. Preserve crash recovery, incarnation and successor-era lineage. |
| O-F13 | **Confirm recovery/reclamation gap; correct “unheld.”** Attach requires source admitted (`C:1543`). `CTL::close_attempt:475–501`/`cleanup.rs::CleanupHolds::step:30` release holds/delete unadmitted products without an inspected production route nominating abandoned roots. | Durably nominate roots before hold release or transfer recovery ownership. Much content is an **ownerless root island with outgoing holds**, not literally all unheld bytes. Distinguish compatible terminal completion from poisoned state. Generic retirement exists; an owning route is missing. No age-only deletion. |
| O-F14 | **Confirm original storage duplication.** `domain/source.rs::SourceArtifact:11–19` includes input/path/content; `persist_original_chunk:3141–3149` keys by artifact entity/position. Same bytes in revision-scoped artifacts can repeat. | Content-keyed blobs plus artifact layout/header references preserve logical artifact identity. Update hydration (`C:4949ff,6120ff`), recovery, backup/import/restore and invariants together. Shared-blob holds/incarnations must survive retirement of only one retained release. |
| O-F15 | **Confirm workspace-only premise memo.** `W::validation_premise:1551–1597` keys model/invariant/profile/selectors/views; map at 1599–1607 is local. `Invariant::digest` (`domain/model.rs:824–831`) covers declaration, not factory source. | Durable premises are optional, lower priority. Include validator/producer implementation, physical incarnation and cold ingress/invalidation semantics. Exact dependencies alone do not prove unchanged backing. Keeping cold admission changes remedy disposition, not the true observation that memo scope is local; do not withdraw diagnosis merely because RC17 is declined. |
| O-F17 | **Confirm attachment copies; correct recovery prerequisite.** `attach_retained_product:1557,1583,1589–1615,1638,1648` creates an attempt contributor, copies members and reinstalls views. `registered_or_install_views:1362–1428` enumerates all members then holds contributors. Receiver checks happen after attachment (`workspace_products.rs:167–214`). | Cold recovery **already follows logical closure** (`logical_recovery_closure:1020–1088`, `capture_state:1123ff`, `view_owners:3971–3992`), not current-attempt-only ownership. Persist/hold the chosen physical incarnation; current cold lookup chooses the first logical match (`validate_view_contributions:3905–3940`, selected backup113ff). Validate receiver before mutation; distinguish miss from corruption. Existing-view installation needs durable completion, not row-count inference. Exact-spec reuse can remove repeated N-sized copies; changed provenance specs still need distinct incidence. |

## Complete assigned rule-impact comparison

| Original | Independent judgment |
|---|---|
| O-RC01 | **Conditionally support.** Include history predicates (`HIST::page_sql:204–220`) in phantom inventory, existing exact-object locks, maintenance authority, revision allocation, reconciliation and successor-era recovery. F03/F04/F07 are not categorically blocked until this changes. |
| O-RC03 | **Consider controlled terminal recovery.** Require outcome/spec/provenance/physical checks under a recovery hold. Reclamation remains necessary if reuse stays excluded. |
| O-RC06 | **Separate assurance boundaries.** Duplicate derivation is not independence; exact stored-row reconciliation remains valuable. Remove duplicates first; remaining cold-check removal needs native witness. `restore.rs::restore:810–852` remains independent ingress. |
| O-RC07 | **Proposed format change.** Versioned composition of completion digests needs validated physical closure and ingress/witness changes. Once-per-attempt batched hashing is a smaller alternative. |
| O-RC09 | **Correct scope.** Ordinary compilation already indirectly binds native source. Current derivation over retained artifacts is the gap. Version derived identities/eligibility where necessary; handle-only change is insufficient. Preserve old pinned readers or explicitly quiesce/retire their generation. Keeping v2 does not prevent every changed lowering from republishing. |
| O-RC13 | **Agree not recommended.** Per-attempt databases trade canonical sharing/reuse/lifecycle for isolation. Static contention or RC01 refusal alone does not establish necessity. |
| O-RC16 | **Support physical alternative.** Byte dedup can preserve revision-scoped artifact identity. Recovery serialization, ownership and shared-blob retirement belong to the same change. |
| O-RC17 | **Optional lower-priority alternative.** Durable premises need logical input, validator implementation and physical integrity authority. Cold re-admission is a legitimate design choice, not refutation of local memo scope. |

## Additional lifecycle findings

The principal retains NC01 as [F18](../../reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F18)
and NC02 as [F19](../../reviews/design_review_efficiency-alignment-audit_2026-10-11.md#F19).

### NC01: retained native products lack a complete owner release route

`C::retain_product_identity:1496–1526` creates per-request/per-attempt `native_product`
roots with contribution holds. Admitted close releases attempt holds only (`CTL:459–468`);
ordinary publication retirement nominates the publication root (`publisher/backup.rs:237–252`).
`HIST::PHASES:67` has no native-product phase. No owning production route to retire obsolete
product roots was found in the inspected creation/close/retirement/history routes and scoped
deletion searches. Generic `retire_reachable` exists, so this is an ownership/integration gap,
not absence of any deletion primitive. F17 dedup does not retire distinct obsolete specs.

Give retained products named lifetime/consumers and an explicit release/nominating route;
retain warm reuse and all live/view/publication/backup holds. Do not infer release from age
or size. This is static lifecycle evidence, not a measured leak or deletion authorization.

### NC02: binding metadata obstructs terminal-attempt compaction

`bind_inner:4072–4079` creates attempt-specific `compiler_binding` rows and holds only the
view. Inspected close/cleanup do not remove bindings. `HIST::eligible:76` refuses an attempt
with any remaining binding; `PHASES:67` has no binding phase. A terminal bound attempt cannot
reach that compaction condition through the examined lifecycle routes.

Blind deletion is wrong: `selected_backup.rs:180–188` selects a stored binding by logical
view. Establish a canonical binding descriptor or an authoritative retained representative
for cold consumers, then release redundant attempt bindings and compact eligible attempts.
Preserve retained backup/recovery needs and crash-safe nomination. No history-size measurement.

### Derivation upgrades belong to the existing upgrade owner

`upgrade.rs::publish:122–135` seals schema progress/journal and installation generation
atomically; `maintenance_client:40–53` requires closed admission and externally proved drain.
That alone does not separate old/new derived eligibility over an old retained manifest.
Treat this as an extension of O-F11/O-RC09, not another duplicative finding.

## Preserved strengths and frozen overlay

Selected backup uses an external transaction and exact publication pin, traverses logical
dependencies, exports selected canonical/recovery content, drains/cancels and releases ownership
(`backup.rs::backup_owned:24–88`, `selected_backup.rs:71–199`). Restore independently admits
reconstructed state, checks definitions/regenerated claims and seals (`restore.rs:810–852,1038–1046`).
Proposed shortcuts must not silently bypass these cold boundaries.

The frozen compiler overlay batches state-source roots near B line1207; selection does likewise.
These reduce crossings, leaving O-F09's full-state/hash/backing work. Frozen derived-search,
reader and selection changes strengthen negative/companion-context eligibility and budgeting.
They do not remove O-F10 duplicate lowering and must not be credited to the original baseline.
No control/attachment/view-installation change closes the other inspected findings.

## F38 clarification and inquiry limits

Catalog alias omission is not established as reachable through the inspected producer.
`domain/graph.rs::canonical_place_endpoints:713–715` only maps `value::Place` to normalized
`EntityRef::Place`; `write_batch_inner:2897–2914,2981–3015` uses exactly this operation.
Package/release/input-distribution/catalog-member discovery is outside that mapping. Final member
hydration separately checks exact membership and canonical/nominal identity. Generic place-capable
reads may need aliases; do not transfer that concern to every catalog discovery predicate.

No exhaustive phantom-safety proof, conflict rate, byte-saving estimate, alternative import alias
shape, complete upgrade/restore qualification or executed engine plan was established. Baseline and
overlay were read separately. All service/deployment qualification remains outside this audit.
