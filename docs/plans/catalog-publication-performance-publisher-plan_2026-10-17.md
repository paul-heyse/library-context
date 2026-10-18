# Catalog publication performance: public publication

**Approved implementation plan, 2026-10-17.** Companion to the [coordinator](catalog-publication-performance-plan_2026-10-17.md),
which owns shared decisions, dependencies and sole current dispositions. This document covers
F09–F11 and their definition, role, search, backup/restore and reader integrations. It is not
an update to any existing plan. Production implementation and qualification are not established
by publishing these documents.

## 1. Baseline and resulting behavior

Baseline `789223017833c364085ef62f9333df082d88444a` retains the reviewed source. Ordinary
`lctx-publisher::seal_completed` already consumes admitted immutable completed content rather
than exporting and independently reimporting it. Keep that direct route. Portable receiving
and restore still earn authority from actual incoming and installed content independently.

The [publication/admission inquiry](../design_review/evidence/2026-10-17_catalog-publication-performance/publication-and-admission.md)
establishes repeated full definition inventory, canonical expected-role reconstruction and
search correspondence work. These public operations are separate from the timed recursive
Analysis control: that control never calls public sealing. Do not attribute public-path
savings to its 900-second result.

The target publication consumes a live admitted source and an operation-owned executable
context, lowers each necessary deterministic expectation once, installs derived content under
actual native ownership, freshly reconciles actual installed content, and commits its exact
manifest marker only after coherent finalization. Previously earned expectations do not grant
permission to skip actual role/root comparison or independent receiving.

Required semantic owners remain in `lctx-model`; native physical addressing, pins, staging,
incidence and receipts remain in `lctx-native`; the publisher orchestrates the actual protected
operation. No new publisher store, generic cache or definition service is required.

## 2. PB01 — Scope definition verification and retain its valid preparation

**Consumes:** PP00's RC08 route and SC06 protected-context contract. **Delivers:** demanded
definition verification for ordinary publication, plus preserved full/cold/snapshot-specific
verification where those operations require it.

Current `crates/lctx-publisher/src/definitions.rs:95–111` retrieves database functions,
analyzers and tables, then fields/indexes for every table; `verify_expected` needs a declared
subset. Publication setup and final seal repeat it. Parse the trusted blueprint once into
the exact required definitions and affected tables, with current normalization/equality.
Use the narrowest supported metadata observations. At minimum, one necessary database-level
inventory must not trigger detailed queries for unrelated tables. Qualify a narrower native
API before relying on it; no assumed `INFO FOR FUNCTION` or remote prepared-plan facility.

Retain the verified expected/actual definition context for this publication only while the
actual installation, service generation, engine version, definition epoch, source/staging owner
and exclusion from definition-affecting maintenance remain valid. Define which actual owner
establishes that exclusion, and how invalidation is observed. A descriptor/hash or elapsed-time
cache entry is not proof of installed definitions. If the selected owner does not protect a
particular metadata premise, preserve its necessary final observation rather than trusting
the initial capture.

Migrate setup and seal to consume the same protected interpretation and required subset.
Audit the analogous selected-backup, restore-comparison and inspection callers. Selected
backup's required metadata/data comparison must remain within its coherent external snapshot.
Installation/realization identity may genuinely cover the entire installation; keep that
whole-inventory operation separate instead of applying subset semantics everywhere.

**Preserve:** exact required schema/function/analyzer/version agreement, privilege boundaries,
current source/derivation identity, maintenance exclusion and explicit drift refusal. No
unrequested DDL or environment synchronization is an ordinary verification side effect.

**Focused acceptance:** add unrelated tables/functions and demonstrate no unrelated per-table
detail acquisition on the ordinary route; missing/changed required definitions refuse; changed
generation/context invalidates retained preparation; full installation capture still detects
its required whole-inventory difference. Include selected-backup snapshot correspondence.

## 3. PB02 — Carry canonical and reference-role expectations into sealing

**Consumes:** RC09's decision, actual admitted source/staging/pin/budget ownership and the
existing reconciliation interface. **Delivers:** an opaque ordinary-publication preparation
that retains exact expected roles and sufficient canonical properties, with fresh actual
reconciliation at seal.

`native_publication.rs:268–296` reads canonical entities/assertions in groups of128 and flushes
reference lowering. Later `lctx-native/src/reconciliation.rs:43–219` redecodes/re-encodes the
same canonical content and rebuilds expected roles. Extend the existing owner pattern of
`PreparedSearch`, not a parallel transferable certificate system.

During actual admitted canonical lowering, prepare exact expected role rows, external endpoints,
family identities and the specific canonical properties the seal needs. Retain these in
charged spillable ordered backing under actual source client/pin, staging owner, derivation,
pool and operation identity. The operation checks their complete premises before and after
use. Keep expected content distinct from the actual persisted observations it will challenge.

The native sink packs actual derived rows using complete physical byte/resident bounds and
the selected PN02 contract. An outer logical128-row loop must not force separate undersized
effects merely because record boundaries differ from edge expansion. This packing correction
can initially use the existing installer; PB02 does not require all of PN04's journal work.

At finalization, enumerate actual selected content independently of the expected ID list,
compare complete required rows and absence/extras, reconcile root inventory and preserve the
guarded marker transition. Retained canonical properties discharge only their named predicates;
they do not bypass independent original-byte checks or actual mutation-sensitive state. Keep
the current audit revision, source eligibility and staging/completion/admission incarnations.

Cold receiving/import/restore reconstruct their expectations from actual content and retain
independent admission. An ordinary opaque preparation cannot be serialized or supplied by an
incoming archive. If a receiving path later retains its own newly earned preparation within
the same protected operation, it must first complete that independent reconstruction.

**Focused acceptance:** missing, extra and changed roles; parallel positions; repeated external
endpoints; changed source/client/pin/pool/derivation/staging; canonical family disagreement;
concurrent equal publishers with distinct staging realizations; changed root incidence before
marker commit. Compare prepared and independent cold reconstruction against hand-stated
expected roles, not just two calls sharing the same lowerer.

## 4. PB03 — Prepare search correspondence once across its real installation barrier

**Consumes:** current SearchWindow/Basic/Companions, exact semantic links and SC06 ownership.
**Delivers:** indexed option/binding correspondence and bounded retained/spooled preparation
between document construction and occurrence construction. No semantic rule change is assumed.

In `lctx-native/src/derived_search.rs:1970–2099`, index ParameterEntityLink by actual source
entity once. Use that one-to-many index for both declared-name resolution and dependency
construction. Preserve competing-name refusal and all link dependencies, including nameless
parameters. Keep canonical dependency ordering/deduplication and actual subject identities.

The current `lower` streams SearchWindow and loads Basic twice, with document-family writes
between passes; each part then scans all bindings for its window/part. Prepare a charged
index by `(window,part)` and retain/spool the minimum exact immutable Basic/correspondence
needed for occurrence lowering. Reuse successful keyed acquisition under exact premises.
Avoid retaining whole unrelated catalogs in memory. Prefer bounded reacquisition for a
particular structure only when its retained form would introduce more total machinery; state
which data remain reread and why.

Document installation before references is a real dependency. Keep that barrier while avoiding
repeated correspondence construction. Consume/release each preparation after its last borrower.
Use current prepared search/lexical expected runs for fresh final actual reconciliation;
do not create another independently mutable search interpretation.

Preserve distinct channel witnesses, source occurrences, alternative links, scope/model
identity and current shared-document deduplication. Scalar lexical posting queries compensate
for a qualified compound-index limitation; replacing them with two `IN` lists is not part of
this correction unless the real access plan is independently qualified. No embedding inference
or submitted MCP query runtime is scheduled here.

**Focused acceptance:** competing names, empty primary sets, multiple bindings per part,
repeated text/vector values with distinct occurrences, nameless linked parameters, bounded
spill, failed document installation and cancellation before occurrence installation. Verify
fresh actual search/lexical reconciliation still detects missing/changed/extra rows.

## 5. PB04 — Integrate ordinary, receiving and reader lifecycle consumers

**Consumes:** working relevant PB01–PB03 slices and any PN02/PN04/PN06 changes those paths
actually use. **Delivers:** the complete public workflow over the new protected preparation
and native representation, with revealing functional controls. Necessary consumer migrations
ship with each changing package; PB04 verifies the combined boundary rather than postponing
them to a final inventory exercise.

Exercise ordinary admitted sealing, unchanged-publication convergence and an independently
received small data-only artifact/restore. Preserve one exact publication/view/eligibility/
definition identity, writer drain before final reconciliation, complete actual EOF, root
checks and marker atomicity. Backup retains selected recoverable logical and physical closure;
restore compares claims but never executes incoming control/definition SQL as authority.

Keep every original source/layout/range/hold check required at its receiving boundary. Existing
OriginalCompletion and reconciliation already share blob work by exact identity where valid;
PN03's layout set observation must preserve that strength, not add duplicate blob hashing.
Selected closure traversal already deduplicates discovered views/contributors. Do not revive
the retired repeated traversal or classify it as a new defect.

After an acknowledged marker, record committed identity before reader/attempt/session cleanup.
A cleanup failure retains committed publication and the exact unresolved obligation. F12's
session fix must preserve primary publication's existing explicit session finality. Readers
and evidence continuations retain their publication/definition pin until actual release;
retirement cannot infer release from process absence or from a different generation's success.

**Focused acceptance:** one small genuine ordinary publication and one independent receiving
case using existing owner routes, plus a held reader across another logical publication,
concurrent winner, invalid incoming derived claim and injected finalization/cleanup failure.
Test logical validation scopes; no operator selection/adoption or blanket shared-store reset.
If a schema/permission/destructive case needs a disposable development target, use PN06's
explicit owner/cutover route and authenticated drainage. Never create a scratch fallback.

### Selected protection and finalization details

The actual Loader does not establish exclusion from definition-changing maintenance. Retain
parsed required declarations and the required-table set, but freshly observe actual required
definitions and engine identity at seal. Full installation identity and selected backup retain
their independent coherent inventory obligations. No elapsed-time definition cache is introduced.

Role preparation is opaque and bound to the actual source client, pin, staging allocation,
realization and resource pool. Canonical rows and complete role expectations use charged ordered
spillable backing. The actual seal independently enumerates current canonical/role/search/root
content; expected IDs do not nominate the universe being challenged. Search retains individual
Basic frames across the genuine documents-before-occurrences barrier, with indexed parameter
links and window/part bindings, and consumes the preparation after its last borrower.

Record the committed marker in both its native lease and the returned completion before any
fallible operation-root finalizer, reader acquisition, session close or attempt cleanup. A
subsequent failure carries that exact committed publication identity. Portable SQL continues to
represent independently validated logical compiler rows; physical block rows are not accepted
as archive authority. Backup decodes current physical content into that logical transport, and
receiving reconstructs fresh independent native ownership.

### Focused actual mutation fixture

Each original `native_publication::expectations_controls` case compiles the genuine small
Catalog, fully admits that completed Workspace with its actual captured inputs, and constructs
ordinary `Publication::new(Admission::Completed(...))`. No raw export, standalone carrier,
receiving import or extra setup selector is a prerequisite for these ordinary publication
controls. Portable export/import keeps its separate production purpose and dedicated controls.

Mutate the actual admitted native rows or installed analyzer under the wholly owned campaign's
maintenance authority, then restore exact prior values on success, error and panic before
another dependent case. Preserve public seal, independent cold search, foreign Loader/pool
refusal, committed-marker/finalizer and exact native finality assertions. A failed public seal
permanently finalizes its source: restoring rows does not restore admission eligibility. Each
negative seal therefore owns its own genuine compile and admission; do not reset lifecycle
flags, reuse a failed owner or add production APIs for test convenience. Whole-instance cloning
is not an assumed substitute for establishing fresh live admission/session ownership.

The original five direct cases and their existing360s allowances are restored. Their aggregate
compile/admission/publication endpoint remains unverified after earlier timeout observations;
separate native transport passes do not close it. Any further allowance change requires its own
explicit disposition, rather than silently splitting the endpoint through an export fixture.

## 6. Completion and later validation

PB01 and PB03 can progress independently. PB02 needs protected-context and source ownership,
not an artificial whole-native-plan barrier. New physical formats require exact locator/read/
reconstruction consumers before their public use; those are actual PN00/PN02 prerequisites.
Coordinate shared `derived_search.rs`, loader and staging edits with native work.

PB04's small controls establish functional integration, not full public performance, recursive
Analysis correctness or whole Catalog acceptance. PP90 owns compile/hygiene and focused combined
checks; PP99 records the later full publication/recursive/backup/restore journeys and remaining
measurement questions. Existing plans and their unresolved acceptance do not change here.
