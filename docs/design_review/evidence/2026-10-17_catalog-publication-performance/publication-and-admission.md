# Publication, completion and admission: source inquiry

2026-10-17; baseline `7f353fa1fffd081eb9f25003b61cbea9f74ded67`.
This bounded coordinator inquiry supports the [principal review](../../reviews/design_review_catalog-publication-performance-target_2026-10-17.md).
It is source evidence and corrective reasoning, not another overall verdict or disposition
register. No new product command, service operation or native probe was run. Diagnoses below
are Interface-checked; remedies and their performance benefits remain Proposed.

## Distinguish the operations

The failed recursive control compiles the complete Analysis frontier and persists private
producer outputs. It does not invoke the public publisher. Therefore the public-path issues
below are additional review findings, not an attribution of its900-second timeout.

Ordinary `lctx-publisher::seal_completed` accepts admitted immutable completed content and
publishes a manifest over it; it does not export then independently reimport it. Portable import
and restore retain separate receiving admission. See `crates/lctx-publisher/src/lib.rs:75,111`
and `native_publication.rs:82-205`.

Publication establishes exact bindings and a source lease, forks an explicitly owned child
session, verifies executable context, holds the admitted state, freezes eligibility, and may
find an already completed identical publication. New staging materializes reference roles and
search projections. Finalization drains ordinary writers, verifies current context, reconciles
actual native content, commits a marker and records that commitment before cleanup. These are
distinct obligations; removing one indiscriminately would not be an efficiency correction.

## Declared definition demand versus full database inventory

`crates/lctx-publisher/src/definitions.rs:95-111` retrieves all database functions, analyzers and
tables, then sequentially asks for fields and indexes of every table. `verify_epoch:144-156`
also retrieves the server version and prepares expected definitions. The actual predicate in
`verify_expected:173` requires the declared definitions to be present; unrelated definitions
are not part of that comparison's required subset.

This inventory runs during publication setup (`native_publication.rs:117`) and again at seal
(`:345`). Related uses occur in selected backup (`selected_backup.rs:119-141`), restore
definition comparison (`restore.rs:262-280`) and catalog inspection. Full installation or
physical-realization inventory can legitimately need a wider identity; it must not be collapsed
into the narrower ordinary executable-epoch check merely because both observe definitions.

The proposed owner accepts the required definition set and actual protected installation/
generation context, prepares its interpretation once, and reuses only while those premises
remain current. Cold inspection remains independent. A cached hash cannot replace observation
if relevant DDL may change without invalidating its owner. A narrower native INFO interface is
not assumed available; its exact API and permission behavior need qualification if selected.
Even retaining one operation-local full inventory would remove repeated local preparation only
when a stable executable-context premise justifies it. This is a demand/validity design choice,
not a recommendation for an unbounded global schema cache.

## Retain canonical lowering, keep fresh actual reconciliation

`native_publication.rs:268-296` streams admitted entities and assertions in128-logical-row
groups. Each group separately awaits reference lowering. `crates/lctx-native/src/loader.rs:558-675`
expands those logical rows into role and external-endpoint records and flushes the group.
Thus an outer128-record grouping can force a flush below the native transfer byte/row budget;
the number of derived edges per input record varies. Logical grouping and physical effect
packing are currently coupled.

Later, `control/staging.rs:311-440` establishes exact frozen eligibility and audit revision,
calls `Loader::reconcile_owned`, reconciles retained search expectations and complete root
inventory, then performs the guarded publication-marker effect. `reconciliation.rs:43-219`
rereads the full canonical entity/assertion payloads, decodes and validates them, re-encodes
their complete physical rows, hashes family content, regenerates role expectations, and
compares those expectations with fresh actual roles. This repeats deterministic preparation
already needed when the canonical rows and reference roles were materialized.

There is an internal positive model: `derived_search.rs:2450-2564` retains `PreparedSearch`
under exact owner/client/source-pin/staging/budget/realization/scope identity. Seal reuses the
expected rows while independently comparing fresh actual search and lexical rows. It does not
trust a claimed result from an external file.

Extend that responsibility boundary to canonical and reference-role preparation where the
same actual immutable admitted source and staging owner establish validity. Transfer charged,
possibly spillable expected rows and exact family identities. Keep actual payload agreement,
all roles, absence/extras, original-byte requirements, current eligibility, audit revision and
marker CAS. The target must say exactly which failure each retained property discharges and
which actual observation remains necessary; merely skipping the second reconciliation is not
an acceptable correction. Independent receiving continues to reconstruct its own expectations
from actual incoming content.

Original reconciliation (`reconciliation.rs:227-244`) validates each actual layout while
deduplicating blob-byte validation by blob identity. That is an intentional independent
receiving/physical assurance boundary and already avoids per-layout duplicate blob hashing.

## Search preparation has correspondence and replay amplification

`derived_search.rs:1970-2099` batches keyed option subjects, entities, links, parameters and
shapes. Inside that bounded result, each source-parameter option scans all parameter/entity
links to derive its name (`:2066`), then scans them again for dependencies (`:2084`). Build a
one-to-many index by entity once. Preserve competing-name refusal and dependency membership,
including links whose parameter has no usable name; sort/deduplicate dependencies exactly as
before. This is a local correspondence improvement, not a new global query engine.

`derived_search.rs:2100-2203` first streams SearchWindows and loads `Basic` to construct document
rows, finishes/writes five families, then streams windows and loads `Basic` again before loading
`Companions` and constructing occurrences. Each part scans the companion binding collection
for its window/part. Retain an exact bounded prepared window/correspondence, or spool its compact
information, across the legitimate document-before-occurrence write dependency. Index bindings
by their actual window/part key. Do not infer that all native writes can execute simultaneously
inside one transaction, or that the document existence dependency is artificial.

Existing companion reads are scoped/keyed, lexical posting/statistics context is owned, and
shared document payloads intentionally deduplicate repeated occurrences. The scalar posting
lookups in `lexical_stats.rs:490` compensate for a known compound-index access-path limitation:
two `IN` lists are not an established better plan. Any alternative must qualify its real access
path and preserve exact postings, corpus identity and scoring. Embedding inference and submitted
MCP query execution remain outside this review's runtime scope.

## Optional retained reuse currently delays producer readiness

`crates/cpg-core/src/workspace.rs:3400-3474` flushes writers, declares empty output families,
joins ingress, closes the actual producing scope, and completes the native singleton. It then
awaits optional `retain_product` before returning a coherent producer result to its consumers.
`workspace_products.rs:371-397` distinguishes effect-free Declined preparation from actual
Ready retention and sends actual work to the existing native executor.

The retained recursive run records13 retention phases totaling118.866s, with compound selection,
inventory receiving and child installation nested in them. These overlapping durations cannot
be added to wall time or assumed entirely critical. The call ordering does establish that each
executed retention operation delays that producer's ready handoff.

Separate semantic completion/readiness from optional reuse construction. First decide which
properties and durable ownership are essential to hand off the completed view; any remaining
retention may overlap independent work under a named actual owner and final join. Reuse may be
very valuable and must not be removed automatically. Neither an unjoined task nor converting an
ambiguous native effect into a harmless cache miss preserves current finality. If retention is
truly mandatory for a particular handoff, state that contract and optimize the required operation
instead of calling it optional. Shared request/failure policy remains in the native owner.

Completion itself already includes useful fast paths: `lctx-compiler-store/src/compiler.rs:6400-6659`
streams a singleton's actual membership to EOF, computes its identity, commits completion and
declares its views; the singleton contribution route avoids rescanning a native union. Composition
must establish actual count/conflict coverage when it joins distinct contributors. Private
preparation plus short changed-predecessor visibility checks in `workspace.rs:3556` are already
implemented; the older broad visibility lock is not a current defect.

## Assurance preparation and receiving analogues

Admission executes model semantics against exact selected inventories. In
`crates/lctx-compiler-store/src/admission.rs:1408-1641`, whole-input memo reuse is excluded when
earning assurance, whereas scoped assurance carries executed capabilities and checks their
premises. `admission/scoped.rs:309-321` reacquires selected capabilities per invariant before
memo use; reuse separately revalidates existing capabilities (`:372-378`).

Consider one operation-owned selected-capability preparation and, where semantically sufficient,
an opaque executed whole-input property tied to exact immutable dependency views, epoch, ledger,
frontier, pool and actual pin owner. A digest or earlier success alone is insufficient. Determine
which independent failure class each pass detects before removing it. This is a shared assurance
design opportunity, not a demonstrated wrong answer or a direction to bypass earned admission.

The explicit cold `verify_state` path (`compiler.rs:6885` onward) is independently receiving,
not ordinary compilation. It first streams and validates backing records (`:6905-6918`), then
issues a fresh node query for each membership (`:6921-6952`). Set-oriented exact typed candidate
observation and reconciliation can preserve missing/foreign/duplicate/full-payload checks while
avoiding a network request for each membership. This is a clear structural recurrence; it is
not the timed recursive route. Likewise original completion (`originals.rs:753-775`) gathers
layout IDs then queries layout, layout validity and ownership separately per layout. Batch those
observations while retaining each distinct actual layout's check and shared blob completion.

## Restore, backup, selection and failure boundaries

Selected backup retains exact logical closure and the physical realization needed to read it.
`selected_backup.rs:208-247` uses SnapshotViews acquisition and RecoveryTraversal to share discovered
view queries. It does not replay the old repeated closure expansion. Exact transport contribution
observations and any remaining per-object work are candidates for the same typed set preparation;
logical and physical identity checks must remain distinct.

Restore (`restore.rs:1230-1380,1489-1585`) admits a checked data-only carrier, verifies exact roots,
creates a new attempt, reconstructs actual content, regenerates roles/search and compares supplied
derived claims before sealing. It does not execute arbitrary supplied SQL. Its independent byte and
semantic admission cannot be replaced with the producer's or backup's claimed truth. Bounded
canonical import packing may use the shared prepared-effect mechanism without changing this trust
boundary. Existing shared OriginalCompletion means repeated source-layout handling is not proof
that all blob bytes are redundantly rehashed.

`native_publication.rs:488-532` records an acknowledged marker before closing its completed reader,
attempt and owned child session, then uses a new explicit reader for protected readback. A cleanup
failure therefore retains the committed result instead of pretending publication never happened.
Retirement and selection remain pin/incarnation/reader-quiescence operations. Changes to throughput
must preserve their exact hazard addresses, terminal receipts and last-reference ordering.

These public workflow findings are independent of pre-Summary acquisition and pair cardinality.
The combined target should share prepared-operation, exact-set observation and ownership transfer
concepts where useful, without inventing one universal executor, global proof registry or mandatory
new service. The principal review decides their priority, interactions and rule impacts.
