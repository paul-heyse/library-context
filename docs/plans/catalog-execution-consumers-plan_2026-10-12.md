# Catalog execution: consumer preparation

**Accepted direction / Proposed implementation, 2026-10-12.** Companion to the
[coordinator](catalog-execution-plan_2026-10-12.md), the sole disposition/checkpoint owner.
This document owns Catalog-review F05–F08 and their affected consumers. The combined planning
request activates their former deferral triggers. They remain distinct closure obligations even
when sharing a preparation primitive with F01–F04. F08 is low priority and does not delay the
critical completion correction or genuine Catalog journey.

## 1. Foundations, boundaries and dependencies

Prefer operation-owned preparation over immutable input, exact consumer selection and assurance
at its actual trust boundary. Reuse the native ordered-run/point-index owner, compiler compact
edge/root-group preparation and borrowed `SelectedRows` where their representation fits. Native
rows, dump spools, Arrow batches and Python responses have different trust/lifetime contracts;
they need no universal storage object, framework or cache registry.

The [completion companion](catalog-execution-completion-plan_2026-10-12.md) supplies CP00's protected
input contract and CP01's canonical ordered fold. CU01 and CU04 can develop independently of the
native representation cutover. CU02 consumes CP00's exact selection contract; CU03 consumes CP01
and migrates with CP04's final/cold boundary. Independent checker state and detached admission
remain separate from preparation reuse. All status and actual receipts belong to coordinator §6.

## 2. CU01 — prepare restore lookup once (Catalog F05)

[Restore](../../crates/lctx-publisher/src/restore.rs) already inventories descriptors and traverses
adjacency once. Preserve that architecture. `Dump::rows` reopens the sequential table spool for
source/blob point queries and other checks rescan complete tables. Replace those repeated queries
with preparation owned by the exact decoded dump operation.

Collect demanded originals during the compiler-record pass; scan original headers once to resolve
those demands, then scan source/entity and blob tables once for their exact required IDs. Retain a
charged typed selected closure for the restoration operation. Share complete table validation with
that pass where its independence permits; do not build a universal all-row dump index. Reuse
`ordered_rows`' collision-checked sorted runs and disk point-index pattern only for an actual lookup
consumer not served by the selected metadata, with a typed dump-ID adapter. Preserve required
duplicate/foreign-row validation even for records not retained in the selected lookup inventory.
Charge decoded values, index metadata, temporary keys, cursor buffers and retained backing; spill
under the existing owner when necessary. Release preparation after all restore borrowers drain.

The lookup result means exact present record, missing required record, or conflict/corrupt record;
a zero count or first matching duplicate is insufficient. Preserve source/artifact identity,
original layouts/ranges/digests, alias topology, declaration completeness, required endpoints and
foreign/extra-row refusal. Do not carry producer acceptance or serialized native authority into a
destination. Actual incoming bytes and actual installed content independently earn restoration
and semantic admission through the shared production validators.

**Migration:** move every restore source/blob point lookup and repeated table discovery that uses
the same immutable dump to the operation preparation. Keep native and dump adapters separate,
sharing semantic closure rules without sharing success flags. Remove replaced per-ID spool scans
and temporary compatibility helpers in the same package. This is not a redesign of whole-service
backup or an expansion into the older SA9 recovery campaign.

**Acceptance:** use a fresh small dump with several originals/layouts and reused backing, shuffled
record order and a required ID near the end. Independently specify expected lookup/admitted content.
Cover missing/duplicate/conflicting IDs, corrupt bytes/aliases, empty tables, spill, late spool failure
and cancellation; no failed preparation yields a restored capability. Source inspection establishes
one relevant table preparation rather than a scan per lookup; elapsed time is not the criterion.
One focused detached export/import/restoration case verifies actual destination admission and exact
original evidence, without preserving old-format data or requiring a historical backup corpus.

## 3. CU02 — prepare partition positions over the exact union (Catalog F06)

[Scoped execution](../../crates/lctx-compiler-store/src/scoped_execution.rs) currently recovers each
root's partition by rescanning/copying hydrated union rows. Use the existing compact topology and
[root-group preparation](../../crates/lctx-compiler-store/src/consumed_rows.rs) to prepare positions
once for the exact union, row order and root-partition membership. Consumers borrow through
[SelectedRows](../../crates/lctx-compiler-store/src/scoped_batch.rs), or gather with Arrow's existing
selection/take kernels where a retained compact batch is useful. Choose at the actual consumer;
zero-copy is not a goal that overrides lifetime or locality.

The preparation binds complete input/view/contributor selection, ordering, model/projection and
partition meaning, plus the caller's resource owner. Bind positions to exact input-port identity,
including repeated relation ports; preserve every matching position unless the source contract
establishes uniqueness. Gather in original union order, with private root namespaces and nominal
absence semantics preserved. Retain backing until its last borrower ends;
charge indexes, gathered arrays and live input together. Avoid a new copy of rich rows merely to
filter them again. A distinct owner cannot consume uncharged state from another pool.

Root output selection is separate from the graph universe needed by each analysis. Preserve
required intermediate nodes, isolates, directed parallel relationships, context and source lineage.
Each validator/checker retains its independent state and semantics. Shared discovery never turns
one checker result into another checker's proof. A topology-global checker must still see its
complete declared topology, even when its emitted rows are root-scoped.

**Migration:** both current mask/filter sites and their actual analysis/checker consumers move to
the prepared positions. Remove their repeated union scans/rich copies after migration. CP04's
protected internal selection supplies input authority; it must not require a newly installed view
for each root. New analysis semantics, behavioral research stages and ranking changes are out of
scope merely because these consumers run analyses.

**Acceptance:** explicit overlapping partitions over a small union; a path/cycle crossing an output
partition; an isolate and parallel assertions; empty and all-row selections; shuffled canonical
input; wrong-input/order/resource-owner refusal. Independently specify each consumer's required
input and expected result. Compare borrowed and gathered routes only where both are retained for
actual consumers. Cancellation and last-borrower drop preserve ownership. The genuine generated
analysis/native-query journey remains the enclosing functional evidence.

**CP04 physical-codec seam integrated in source, 2026-10-13.** Cold completion descriptors and
native recovery now use the compound v2 contributor/selector inventory, while independent portable
state remains format4. Existing cold/export consumers explicitly freeze their source before export;
no mutable export or old physical-format reader is retained. CU01's dump lookup preparation and
CU02's partition-position implementation remain separate required work.

## 4. CU03 — logical identity without a transport detour (Catalog F07)

[Compiler identity/export](../../crates/lctx-compiler-store/src/compiler.rs) now separates checksum
from portable preparation in source (2026-10-13). Direct canonical families feed logical identity;
export prepares portable rows once, independently folds their actual normalized identity, compares
it against the frozen boundary, then encodes preserved payload frames. Current focused receipts
and remaining CU03 acceptance belong to coordinator §6. One authoritative v4 recipe remains.

**Native transfer correction (Implemented source, acceptance pending, 2026-10-13).** The existing
maximum projection chunk exposed a unit mismatch: the complete native/JSON coexistence estimate
was compared with the8MiB wire cap. Resource reservation continues to use that original estimate.
Transfer preflight instead uses the pinned protobuf codec's complete QueryRequest encoded length,
including a conservative bound for native context/batch fields and all lowered SQL/controls/bindings.
Both payload and intent are checked before any remote intent. Opaque canonical/body Bytes, nominal
keys, chunk size and the transfer cap are unchanged; no legacy codec or schema migration follows.
The original maximum payload/scan/export/import/cold control remains required, alongside genuine
wire oversize and final-wrapper/pre-intent refusal. A small binary cross-pool pass alone does not
qualify maximum ingress. Coordinator §6 owns the original failure and current acceptance receipts.

The maximum case now explicitly prepares its source under its existing256MiB producer pool before
the unchanged8MiB consumer scan. CP00/CP02 separate those owners: this cannot establish8MiB total
memory or cold maximum acquisition. The original cold8MiB failure remains recorded; its prepared
JSON frame alone is about16MiB, and the larger conservative reservation is not measured allocation.
All maximum bytes/keys/export/import assertions remain. Small binary first acquisition still uses8MiB.
Full expected-record observations use the declared64MiB physical-row bound; guards remain narrow.
Actual row width and the unchanged32×+64KiB workspace charge are admitted separately. Current-source
acceptance of these corrections belongs to coordinator §6.

For checksum/final-state identity, lower validated descriptors and exact protected members directly
to the six canonical logical families and use CP01's ordered fold. Preserve v4 JSON row payloads,
`RecordId::Ord` order, unique row counts, explicit views/empties/bindings/aliases and full completed
contribution outcomes. Physical member order cannot substitute for normalized logical order.
Publish the complete identity only after all required inputs and terminal checks succeed.

For an explicitly requested export, require frozen content or an independently retained read-only
boundary before preparing portable rows. `AuditCapture`, read admission and `wait_scans` alone do
not freeze writers. Migrate mutable callers and tests to obtain owned finalization first; do not add
a mutable-export fallback or infer a snapshot from a lease. Prepare the portable representation
once for that exact boundary and reuse it through encoding/output and compatible same-operation
consumers. Check the identity of actual emitted portable content against that boundary. Portable
ordering and logical hashing order remain distinct contracts. Share descriptors, immutable source
runs and normalization recipes only where their complete dependencies coincide; do not force
checksum-only callers to pay for export or make export bytes the canonical semantic authority.
Charge transport preparation/output through their actual lifetimes and release only after delivery
or owned cancellation/drainage.

**Migration:** checksum-only callers, frozen completion, admitted artifact identity, explicit export
and publisher transport adapters use the intended route. Detached import and restore continue to
independently reconstruct/validate actual incoming and installed content; a transported root or
producer cache is not acceptance. Remove duplicate portable preparation and redundant rich clones
once their consumers migrate. Keep no old-format reader or dual runtime path after hard cutover.

**Acceptance:** independently specified canonical streams/roots and deterministic portable output
for a small mixed boundary, including explicit empty/selective views and aliases. Equivalent selected
content in different physical arrangements gives the intended logical result, without allowing one
physical copy's authority to substitute for another. Binding/alias/outcome changes affect the proper
families. Cover unequal duplicates, byte/null distinctions, late terminal failure and failed output.
One actual fresh destination import/admission case challenges the producer; a round-trip using only
the new producer's own normalizer is insufficient as its sole oracle. No DAG or digest concatenation.

## 5. CU04 — retain one Rust recipe response per immutable Spec (Catalog F08)

[Python Spec](../../python/lctx_mcp/src/lctx_mcp/embedder.py) exposes four identities by repeatedly
calling the same Rust function and decoding JSON. Retain one decoded Rust response per canonical
specification, reused by encoder/query/document/projection properties. Rust still owns identity
meaning; Python neither reconstructs hashes nor observes ambient model/service configuration.

Bind reuse to the canonical string and Rust recipe version used by the actual immutable Spec.
A frozen dataclass does not deeply freeze its `fields` dictionary: cached identities must not depend
on mutable aliases or silently diverge from text rendering. Keep canonical identity input immutable;
make parsed field access read-only/defensively owned. Choose one eager private typed response after
canonicalization at Spec creation; the canonicalization and recipe functions both use the same Rust
Spec parser, so this introduces no separate domain validator. Properties read that frozen response.
Use checked construction and migrate `native.py`/other actual callers so direct construction cannot
pair inconsistent canonical text and fields. Preserve factory/helper values and parsing errors.
Direct bypass construction or field mutation is intentionally refused earlier rather than supported
as a second authority. This is a local adapter contract correction, not a process-global cache.

**Acceptance:** pure adapter cases verify unchanged property values and refusal behavior, one Rust
identity call and one decode across repeated identity properties, separate specifications and
no mutable field path changing a cached identity's premise. Compile affected Rust/Python bridge
components. These are local recipe controls, not content embedding, external service calls or an
MCP-submitted query campaign. CU04 can land after critical-path Catalog acceptance, but it is part
of this plan's required functional scope before final handback.

## 6. Overall consumer completion

Each package supplies its own migrated-consumer inventory, replacement/deletion evidence and
focused controls to the coordinator. Reuse of CP01 does not close CU01/CU02/CU03 by association.
Source-established removed scans/copies are architectural evidence; speed/capacity claims remain
unmeasured. These corrections introduce no new daemon, scheduler, evidence registry or mandatory
profiler. CE01 reviews their composition, CE02 regenerates current state where needed, and CE03
reconciles the older tracks only after required delivery and acceptance.

**Open cold-alias incidence contract (2026-10-13).** The six-family v4 union hashes aliases but
not their contributor ownership. Exact native selectors already preserve that distinction; portable
import cannot infer it from a shared source. Nonempty alias ingress refuses explicitly until a bound
carrier/identity decision is implemented across export, artifact and selected dump restoration.
Bind a carrier independently of its own header; require exact logical owner IDs and refuse ambiguous
physical copies with different alias incidence unless the envelope represents those copies/selectors.
Do not silently widen eligibility or change the v4 recipe. Coordinator §6 owns disposition and evidence.

The bounded independent design assessment proposes a separate model-owned transport envelope while
preserving v4 semantic roots. It must bind exact view→portable-copy mappings, full descriptors and
complete copy→alias inventories (explicit empties), ordered counts/EOF and codec identity. Equal
logical descriptors with differing local alias sets require distinct portable copy labels and fresh
destination owners; neither logical-owner union nor restore's first matching physical copy is safe.
Aliases retain their canonical semantic-family binding. Labels confer correspondence, not authority.

The expected envelope identity must come from a trusted export/backup result retained by its caller,
not the incoming header or sidecar itself. Detached export and selected backup therefore need an
explicit returned transport receipt and import/restore an independently supplied expectation, or a
new externally expected artifact/publication handle. Source pins/backup snapshot and charged ordered
runs survive through full EOF, file sync/staged commit and checked finalizers before receipt release.
Destination creates fresh contributions, endpoint/alias holds and exact view mappings, then independently
earns current physical and semantic admission; never import native holds, guards, attempts or pins.
Partial/Unknown ingress retains existing obligations. This remains **Proposed** pending the operator's
identity direction and concrete API migration; a self-consistent incoming digest is insufficient.

Qualification must distinguish v4 content equality from selector-local alias eligibility. Deduplication,
unchanged-publication reuse and caches claiming the latter need the combined identity or an actual
independently validated physical boundary. If product semantic equality must imply that eligibility,
a new semantic identity version is the coherent alternative. Required revealing cases are swapped owner
incidence with equal v4 union, equal descriptors/different copies, shared/reconvergent sources and
cross-contributor chains, explicit empty and missing/foreign copy inventories, fresh destination
identity with exact selector absence, and both artifact/selected-backup admission plus failed EOF/drain.
Coordinator §6 owns the decision; this advice does not authorize an old-format reader or close CU03.

The read-only caller map establishes the bounded migration for that proposed option:

| Owner | Required contract change |
|---|---|
| Model `domain/completed.rs` | Separate envelope/receipt types and codec, ordered copy/selector inventories, counts and EOF; preserve semantic v4 |
| Compiler `compiler.rs` | Preserve portable copy labels and entire selector mappings instead of collapsing copies; export returns semantic/envelope receipt; import requires independent expectation and rebinds fresh destination owners |
| Compiler `artifact.rs` | Carry returned receipt through export, verification, `VerifiedExport` and detached readmission; live verification may derive expectation from its protected source |
| Publisher `selected_backup.rs` | Capture full selected-copy correspondence and alias inventories under the existing publication pin and single transaction snapshot |
| Publisher `backup.rs` / `backup_import` | Return receipt only after EOF, checked snapshot/session/pin finalizers, staged sync/publication and parent sync; decode bounded data-only framing and require expectation on restore |
| Publisher `restore.rs` | Replace first matching physical copy and global source expansion with verified envelope correspondence; pass it through lower completed-state import |
| CLI compile / native artifact and backup routes | Expose returned export/backup receipt and accept explicitly retained expectations across later processes; incoming manifests/sidecars alone cannot supply trust |

Direct transport, detached artifact and selected-backup tests all need the decisive equal-v4/
different-incidence cases, including root-qualified dependency copy choices and explicit empties.
No source attempts/pins/holds become destination authority. Snapshot/publication identity currently
does not bind this incidence. This map specifies work; it does not establish approval or completion
of the proposed external contract.

**Additional current equivalence gaps (static review, 2026-10-13).** This obligation also affects
native consumers, not just cold framing. `Publication::new` computes the semantic/binding/definition
handle and discovers an unchanged publication before acquiring the incoming admitted physical
completion. Native `find_existing_publication` and both staging convergence branches independently
validate the winning boundary but never compare its selector-local alias eligibility with the incoming
one. A protected winner is not evidence that it is equivalent to that incoming selection.

`NativeCompilerStore::attach_held_product` separately skips incoming mappings for an already known
logical view and retains the existing installed capability with `or_insert`. Independent candidate
validation does not establish equivalence to that existing receiver selection. Original-request replay
and terminal recovery keep their exact original boundary and do not have this substitution gap.

The selected identity direction must cover publication handle/reuse/convergence and receiver collision
handling in addition to the caller map above. Compare independently protected effective eligibility,
bind the relevant envelope identity, or refuse unequal physical selections; semantic v4 equality alone
is insufficient. Receiver compatibility checks must precede native effects and remain coherent under
concurrent attachments, without partial metadata publication. Until delivered and functionally checked,
these native equivalence obligations remain open CP04 work even when first-publication controls pass.
