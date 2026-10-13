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

## 4. CU03 — logical identity without a transport detour (Catalog F07)

[Compiler identity/export](../../crates/lctx-compiler-store/src/compiler.rs) currently constructs
portable completed rows for checksum-only demand, then lowers/resorts them into logical form;
export repeats portable preparation. Split useful demand while keeping one authoritative recipe.

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
