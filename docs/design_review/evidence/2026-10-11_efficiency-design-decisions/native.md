# Native mechanism decisions

**Proposed designs; static investigation and finite models Tested, 2026-10-11.**
This supports [N01–N11](../../../plans/efficiency-alignment-native-plan_2026-10-11.md).
It closes mechanism choices, not product findings, implementation or native acceptance.
Baseline: `439f3d2d`, isolated `efficiency-decisions` worktree. Source-review and audit
observations were reread against this baseline; older plan documents were not consumed.
The shared worker/executor contracts and execute-plan, design principles and efficient
architecture heuristics guided the inquiry. Relevant tradeoffs are H13–H20 and H22–H25:
one checked completion, operation-sized effects, explicit physical lifetimes and narrow
write-conflict domains, without a general scheduler or copied validation authority.

## Evidence and limits

- `python3 docs/design_review/evidence/2026-10-11_efficiency-design-decisions/native/finite_protocols.py`
  **passed**, 2026-10-11. [Raw output](raw/native-finite-results.json) records nine synthetic
  groups covering N01–N11. The independently authored standard-library script imports no
  production code. It checks identity sensitivity, failure prefixes and necessary conflict
  edges. Its abstract conflict model is explicitly not a mock native acceptance result.
- [Engine cases](native/engine-cases.json) specify the coordinator's dedicated RocksDB
  experiments. This worker ran no server, fixture, environment preparation or product
  controls. Engine evidence belongs to the coordinator's campaign, not the finite model.
- Coordinator [RocksDB raw receipt](raw/engine-02-observations.json), reread 2026-10-11:
  existing-write/first-absent-UPSERT/shared-reference-guard/named-FOR-UPDATE/all-stripes-cut
  overlap cases reject the slower transaction with `TransactionConflict` and retain only
  the winner's effect; independent stripes both commit. The unguarded range-phantom
  negative commits both, with retirement seeing an empty set and the new hold present.
  THROW rolls back its effect; range FOR UPDATE refuses. The exact copied patched binary
  hash is `710229df33346fdf9733ef86d46e31874f0cacc2691529327e19e6534477b8ed`.
  Dedicated child exit zero/wait-confirmed is recorded separately. These are engine
  functional cases, not product protocol/64-stripe qualification or throughput measurements.
- Read-only `uv run --no-sync python` invocation of `native_campaign.py::validate_engine`
  on that persisted receipt **passed**, including cleanup facts. An initial system Python3.12
  invocation **failed** parsing the imported Python3.14 storage owner; rerunning with repository
  Python3.14.7 corrected only the interpreter, without environment synchronization.
- Context7 was resolved first to `/surrealdb/docs.surrealdb.com` and queried for transaction
  snapshot/FOR UPDATE/phantom behavior. Current docs say named-record conflict detection,
  not predicate serializability. That is a discovery lead; exact source below governs.
- Exact server source inspected at
  `/home/paul/.local/state/library-context/surrealdb.tools/server-build/3efaf277b46867beb1e75f6cae56eb30dfd40878bbb823ef953213cf1cd84680/source`.
  Tracked provenance identifies upstream commit
  `238bfeb11f5725bebed370167656748df8067595` plus the timeout repair. Read-only
  `sha256sum` matched `Cargo.lock` `6ec3773437dcad1ce1e354398f8a2f7ead00e185f9ff7afc187655670a0dbbd5`
  and patched executor `281d6cac3fefbc1f207129a830bb37b9a58a69400405dc95312003b1b732c425`.
  Under that source, `surrealdb/kvs-rocksdb/src/lib.rs:786` enables snapshots;
  `:1969–1990` registers `get_for_update_opt` against the captured snapshot;
  `:2050–2074` buffers `put` with commit-time conflict tracking. Exact-record FOR UPDATE
  target enforcement is `surrealdb/core/src/dbs/iterator.rs:617–625`; it is not a range lock.

Repository source paths below are relative to baseline root. Abbreviations: **C**
`crates/lctx-surrealdb/src/compiler.rs`; **CTL** `crates/lctx-surrealdb/src/control.rs`;
**AUTH**, **HIST**, **RET** the corresponding modules under `control/`;
**NP** `crates/lctx-publisher/src/native_publication.rs`; **W**
`crates/cpg-core/src/workspace.rs`. All production statements are static observations,
**Implemented/static-inspected, 2026-10-11**. Proposed names below are concrete interfaces
to implement, not claims that those Rust types already exist.

## N01 — One recoverable bounded installation operation

Select native `install_window(owner, mutation_permit, ImmutableWindow) -> NativeWriteReceipt`.
The window contains anchors, immutable payloads, role/reference rows and ownership edges in
dependency order. Its charge includes generated SQL, encoded bindings, payloads, guard IDs,
hold rows and response bookkeeping together. Plan the whole window before submission; split
only at a boundary where every committed child remains reachable from an already durable owner.
No statement-count proxy may ignore payload/hold expansion. `ensure_physical_rows` (C:3040ff),
`ensure_immutable_rows` (C:3180ff), Loader `ensure_window` and `insert` call this single owner.

Keep durable intent/outcome records. A private `PreparedNativeRequest` holds operation,
issuance, owner identity/epoch, SQL/codec identity and the complete binding digest once;
the intent binds only used control values. Public `execute_request` recomputes/compares the
payload digest. The effect transaction performs guarded full-value collision checks and writes
all rows/holds plus resolved outcome atomically. A late collision rolls back this entire
window. Earlier acknowledged windows remain owned recoverable progress, not a fictional
whole-corpus rollback. Retiring guard, changed expected incarnation and completed producer
reject before any new reachability becomes visible.

`RetryObservation {attempt, elapsed, last_conflict}` belongs to CTL `run_transaction`.
Use exponential jittered backoff with a maximum delay, cancellation-aware waits, and a caller
stop/deadline policy; this bounds observation/retry intervals rather than silently imposing a
healthy-work runtime limit. On stop or ambiguous acknowledgement, reconcile the **same**
`NativeRequest`, returning committed/fenced/pending with that request; never mint replacement
identity while uncertainty remains. Outcome release failure reports the already committed effect.
Reject no-intent direct writes and “new request per retry”: draining, history and recovery consume
durable requests (AUTH:146–238; CTL:169–222). The finite late-collision and lost-ack model passed;
actual rollback, native late collisions and lost-ack reconciliation remain implementation controls.

## N02 — Nominal checked ingress, two acknowledgements and paged exact input

Select model-owned `CheckedCanonicalBatch {relation, model, schema, canonical_codec,
rows, canonical_bytes, charge}` constructible only by checked model construction. Native accepts
its exact qualification without recanonicalization; untyped/raw/imported rows call the checked
constructor independently. This narrows the repeated `Batch::with_reservation` → `canonical`
path without claiming all validation is canonicalization. A different schema/encoding cannot
reuse the nominal guarantee.

`NativeSink::accept(batch) -> IngressReceipt {sequence, retained_charge}` transfers byte ownership
to the sink; it does **not** acknowledge a database commit. `await_committed(sequence)` yields
`NativeWriteReceipt {request, disposition}`. Combined in-flight charges cover all accepted buffers,
lowered/native windows and retries until commit/reconciliation plus owned cleanup. `finish()` first
closes acceptance, drains accepted sequences and transport, then returns complete inventory;
failure/cancellation closes acceptance and drains too. No contribution can become completed while
an accepted write is pending. Dependency-ready provider scheduling is compiler C07's responsibility;
native owns remote completion and cancellation. Existing W `ProducerOutput::push` and
`cpg-core/src/facts.rs::compile_facts` migrate together with C `write_batch_inner`.

Original chunks enter one bounded flattening iterator across successive chunks; offsets and headers
retain their own validation. Flush on combined row/byte allowance, not each logical chunk. Source
membership and outgoing-hold iteration uses exact ordered keyset pages over a held, frozen source:
cursor `(relation, semantic_key, stable_physical_id)` or hold ID with strict progress, duplicate,
ordering and EOF checks; close/drain each page before submitting writes. An interrupted installation
has no completion fact and resumes from durable checked page progress or discards/rebuilds its
private incomplete realization. No unprotected moving set or transport setting substitutes for a
snapshot. Selected backup keeps its explicit transaction and pin. Reject all-corpus collection and
an unbounded queue. Finite cross-chunk/charge/keyset cases passed; actual transport cancellation,
provider-failure drainage and checked ingress/raw-conflict controls remain required.

## N03 — Finite write stripes plus complete exact predicate guards

Select **write** coordination, not read-only installation comparison. Installation creates
64 `native_admission_stripe` records, with the count and hash domain versioned in physical format.
`stripe = H(operation_id) mod 64`; retries retain it. This finite collision set is not a worker,
job or thread limit. Intent and payload each update their stripe revision and verify its
`generation, era, closed_through`. New-owner admission additionally requires `admission_open`;
current-era cleanup/maintenance remains authorized when admission is closed. Installation
admission close/open and era cut update **all** stripes plus the installation marker in one
bounded transaction. A cut verifies the fully drained predicates in that same transaction.
All old-stripe effects either serialize before it or conflict/refuse on retry. Durable intents
committed before the cut participate in unresolved-work checks; uncommitted intents cannot appear
after a successful cut under old authority. A long-lived read-only marker cannot provide this.

The fixed transition touches 64 stripe rows plus bounded marker/effect/receipt metadata, below the
current 128-row native window allowance; stripe fields are closed hashes/integers/booleans, never
growing arrays. Installation preflight sizes the **whole** generated SQL/bindings/control payload
against the 8-MiB transfer allowance, with an enforced 1-KiB encoded stripe maximum; it refuses an
incompatible layout before transition effects rather than splitting an atomic cut. A finite maximal
metadata encoding check accompanies the 64-stripe decision. The native campaign uses two stripes
to expose the conflict law; it does not qualify the future 64-stripe SQL/schema implementation.

Closed admission and stripe conflicts do **not** establish physical remote drainage. Preserve the
host maintenance/backup owner, NativeRows drain ownership and server-side terminal evidence. A lost
client/query cancellation acknowledgement leaves remote work, pins and recovery obligations owned;
child exit or a dropped transaction handle cannot release them. Era cut, destructive migration,
cold backup and retirement still require their existing actual-drain authority. Coordinator S09/D10
addresses dropped remote transactions; N03 cannot substitute native logical predicates for that fix.

Separate owner issuance from ordinary effects. Preserve monotone numeric admission allocation only
for actual attempt/pin/backup/lifecycle owner types with a named ordinal consumer. An ordinary
installation-owned effect uses an owner derived from installation identity and operation UUID;
create its exact guard/local epoch in its own stripe transaction, without any global ordinal or
installation-record RMW. Retries retain that owner. Its state/effect/reference/history incidence
uses the same exact guards and terminal cleanup obligations in the inventory below; it is not an
untracked transient owner. Replace cross-owner `epoch <= retired_through`
as object authorization with exact `ExpectedObject {record, incarnation, phase,
tombstone_revision_if_vacant}`. Active-object operations compare incarnation/phase, while vacant/
retired reactivation also compares the tombstone revision; unrelated active hold updates do not
invalidate a captured expected incarnation. Every planned mutation carries these values in its request digest.
Guarded fresh activation compares the exact vacant/retired predecessor, increments incarnation and
publishes a new active guard; old requests cannot adopt it by refreshing expected values during
retry. If a noncommitted operation must replan, first establish its fenced disposition, then issue
a new request. Same committed-request replay reads its outcome. This change must migrate all
guard/hold/cleanup/retirement consumers in one closed, drained native-format transition; keeping
old scalar comparisons beside new local epochs would be unsafe. Global ordinal allocation remains
only where a consumer actually requires an ordinal, never as implicit fencing for ordinary writes.

The full predicate inventory and conflict partners are:

| Protected predicate | Both partners write this guard, then recheck under it |
|---|---|
| New attempts, pins, backups and lifecycle-owner admission vs admission close/cut | Request stripe vs all-stripe transition; owner creation writes its exact owner guard |
| Intent/payload/reconciliation/outcome disposition | Exact effect record plus request stripe; reconciliation never executes predecessor payload |
| Owner state/epoch vs close, freeze, successor | Exact owner guard/record; mutation additionally checks permitted family and phase |
| Incoming/outgoing holds vs claim/finalize/reactivation | Exact guards of **both** hold ends; hold release also updates both ends |
| Global cold-backup admission vs retirement claim/page/finalize | Dedicated `native_backup_gate` revision and active count, maintained atomically with backup holds; retirement writes gate and refuses nonzero count |
| Selected logical backup vs collection | Exact publication pin/reachability and transaction snapshot; no new global blocking of unrelated retirement |
| Drain's unresolved effects/open-or-closing attempts/live pins/active backups/registering retirement jobs | All-stripe close/cut plus matching stripe write on every creation/state transition into these sets; current-era maintenance keeps stripe writes |
| History eligibility: incoming/outgoing holds, outcome references, owner effects | Exact candidate/history-target guard; **every** incidence insertion, release and deletion writes it |
| History attempt eligibility: contributions, bindings, cleanup and attempt effects | Exact attempt/history-target guard shared by those incidence producers/removers and collector |
| History retirement/item eligibility, parent state and successor/predecessor references | Exact job/parent/item guards and lineage guard; children and successor links write parent guard before insertion |
| History page horizon/cursor/expected revision/receipt | Checkpoint guard plus request stripe; exact page effects and receipt commit together |
| Outcome retention/release vs history deletion | Guard for referenced effect/object plus exact reference guard; retention refuses absent target |
| Retirement registration/remaining-item insertion vs job completion | Exact job guard; registration must finish before era cut |
| Retirement/cleanup successor handoff vs predecessor pages | Exact lineage root and source/target owners; source permanently superseded, only remaining durable scope rehomed |
| Successor's traversal of older predecessor pages vs history deletion | Actual predecessor references hold/write their parent guards; history refuses referenced lineage until released |

History scan candidates may be captured outside the transaction; final eligibility and exact
candidate/parent state are rechecked under these writes. Empty predicate scans are safe only because
all possible insertion partners conflict on their representing guard. No range phantom assumption.
The inventory covers HIST `eligible`, `page_sql`, outcome retention/release and RET claim/page/finalize,
registration/recovery, plus cleanup successors; it cannot be reduced to hold-vs-retire alone.
Reconciliation after cut uses fresh current maintenance authority, generation and exact old request;
it only fences/observes old work. Successors own copied remaining scope with original cutoffs,
incarnations, completion receipts and chain references, never refresh an old invocation's epoch.

Reject a single ordinary installation write (unnecessary shared conflict), read-only era comparison
(stale reads/phantoms), unbounded per-predicate scheduler, and stripes without exact history/object
guards (incomplete safety). Retain dedicated shared backup coordination because its actual global
predicate requires it. Source proves a sufficient write-conflict primitive; the finite conflict
graph checks 12 partner classes in both commit orders and a disjoint-owner case. Coordinator engine
cases must confirm actual first-guard conflicts, stripe-cut conflicts, phantom negatives and rollback.
Future implementation adds full native hold/backup/history/lineage races, cut before-intent delivery,
lost receipts and repeated interrupted successor recovery before replacing ERA_GUARD.

## N04 and N06 — Logical completed closure and physical installed realization

These are two witnesses with different owners. Model owns logical content; native owns installed
completion and physical lifetime. Current RecoveryFamily/COMPLETED_FAMILIES (`domain/recovery_closure.rs`)
already enumerate six completed families; C `state_identity_selected` streams all six in v3.
N06 selects six canonical sorted **family roots computed once at protected freeze**, then a versioned
composition, rather than an incremental commutative XOR or a custom Merkle database:

`LogicalCompletedIdentity/v4 = H(model, canonical_codec, ordered[(family_tag, row_count,
canonical_rows_digest)] for Contributions, Memberships, Views, TypedBacking, Bindings, Aliases)`.

Each family digest frames full normalized logical keys and values, independent of attempt IDs,
physical contributor choice and storage order. Identical duplicate logical keys collapse only after
full-value equality; conflicting duplicates refuse. Membership entries include logical contributor,
relation, nominal key and full payload content. Contributions include full spec/outcome/inputs/output
inventory; views include full descriptor and explicit empty outputs; typed backing includes complete
model canonical rows and logical original references; bindings include source snapshot, boundary,
selectors, roles/configuration; aliases include complete source/target meaning. Full original bytes
are admitted through logical artifact content/range claims and N10 blob completion, not silently
removed because physical chunks change. Absence and unrequested states remain identity-bearing.

Freeze closes producer acceptance, drains accepted work, then native atomically changes the attempt
to frozen and records the exact family inventory revision. `MutationPermit {owner, epoch, phase,
family, producer}` is required at all supported row mutation entrypoints, including generic
`ensure_rows_owned`, guarded mutable effects, binding/alias/backing/metadata/import helpers and Loader.
Identity-bearing additions/changes reject after freeze, including descriptor or same-format alias
changes; publication-derived mutation has a separate staging capability. Raw privileged maintenance
invalidates physical validity and cannot coexist with ordinary readers. A process-local flag or
binding revision is insufficient. Once six roots have been independently checked and committed under
this protected revision, admission/seal compose/reuse them without rereading original bytes.

N04 introduces `HeldCandidate {request, spec, outcome, source_record, source_incarnation,
logical_contribution, completed_identity, hold_owner}`. Discovery alone returns a candidate pointer.
`hold_expected` atomically acquires against its expected incarnation. Compiler receiver preflight
validates capture/spec/profile/outcome/exact dependencies and requested output requirements before
any receiver attachment. Attachment then records selected logical→physical correspondence and holds
those existing contributors directly. No attempt contributor or N membership copies on exact-spec
hit. Changed specs have a new logical contribution and run normally. Corruption is an error; absent,
retiring or incompatible candidates may be a typed miss; unknown effects remain recoverable failure.

`InstalledViewCompletion/v1 {installation, validity_epoch, physical_schema, member_codec,
view_record, view_incarnation, logical_view, view_closure_identity,
selected[(logical_contribution, physical_record, incarnation, contribution_completion)],
membership_rows, membership_digest, backing_completion, frozen_inventory_revision}` is native-local.
Publish only after the **view-scoped six-family dependency closure** is closed, exact contributor closure/mapping and source holds
are established, every expected member has been checked against actual stored member/payload/backing,
extras/conflicts have been excluded, and all pages/streams reach checked EOF. Count equality alone
never establishes it. A private installation root owns incomplete member rows and source holds;
resume from checked durable pages, or nominate/discard that private realization and rebuild before
publishing completion. Generic mutation cannot alter its members/selected mapping. Same logical view
can have equivalent physical copies; each realization names its actual selection. No process-global
“first matching contributor” decision is recovery authority.

This is a **view-scoped completion freeze**, not N06's final attempt admission freeze. Completed
contributor membership and its exact typed-backing/alias inventories are immutable; dependency
binding descriptors are canonical held values captured by the completed spec. Seal that closed
inventory under its scope guard, including explicit empty outputs, without closing unrelated
producers in the same attempt. Current/future bindings outside this selected closure cannot enter
its membership or absence answers. Later N06 closes the complete requested attempt inventory and
composes its six roots once. Requiring final attempt freeze before C04's intermediate view reads
would create a completion dependency cycle and is rejected. Generic mutation permits respect both
completed-scope and final-attempt fences; idempotent equal payload insertion cannot expand a frozen
scope's member/alias inventory.

Compiler C03/C04 consume `HeldInstalledView {completion_id, logical_view, view_record,
view_incarnation, view_realization_id, member_codec, selected, hold_owner}`. Direct deterministic
addresses become `view_realization_id = H(logical_view, sorted_selected_records_and_incarnations,
member_codec)` and `H("native-view-member/v2", view_realization_id, relation, nominal_key)`.
The capability's exact namespace prevents equivalent copies with different physical pointers from
colliding. A missing member proves absence only with this completion and
live hold. Returned relation/key/node/content still receives exact semantic checks. Native selection,
compiler point reads, cold recovery, selected backup, publisher and retirement migrate together.
Portable export carries logical closure/claims, not this completion; destination independently builds
new physical selections/incarnations and earns fresh completion. Model/manifest/detached transfer
version the logical identity together; physical remapping alone does not change it.

Reject cardinality witnesses, descriptor-only completion, arbitrary first logical matches and a
per-contribution digest sum that loses alias/binding/empty-view content or double-counts overlap.
Finite models passed all six mutation/order tests and missing/extra/same-count-substitution/EOF/stale
view cases. Actual freeze-versus-each-mutation, receiver-rejection-with-zero-attachment-effects,
source-attempt closure, cold equivalent-copy choice and failed installation recovery remain controls.

## N05 and N09 — Root reachability, binding authority and release

Select durable `PublicationStagingRoot {attempt, manifest, derivation, state, incarnation}` created
and held by its attempt **before** any derived child. Loader/reference/search/statistics writers
hold children under this root, not a flat list later retransferred at seal. The marker transaction
ensures equal marker bytes, guarded publication→staging-root reachability and state sealed together.
Attempt release happens afterwards; one root edge replaces N child transfers. Independent concurrent
identical publication compares the same manifest/derivation/definitions and converges under its
publication guard; a losing private staging root is durably nominated before release. Recovery
finds roots by state/owner even when the first child was never written. A committed marker remains
committed if readback, invalidation or attempt close fails. RET already bounds outgoing pages;
preserve claim/incarnation/successor checks instead of inventing whole-fanout transactions.

Select native `retain_product(consumer, request, completed_contribution)` and
`release_product(consumer, product, expected_incarnation) -> ReleaseReceipt`.
Consumers are explicit warm-reuse policy, view realization, publication, backup, active reader or
recovery owner. Last-owner release atomically nominates the exact root in durable retirement work
before deleting that owner's incidence; interruption resumes the named release receipt. Attempt
close enumerates **all** attempt-held roots (failed/incomplete/no-product/artifact-only included),
nominates any without a surviving product owner, then releases attempt ownership. Ownerless islands
with outgoing holds are not discovered by an “unheld leaf” sweep. Candidate rejection never releases
its reclamation obligation. Shared surviving content remains protected by peer incoming holds.

Terminal reuse selects `admit_terminal_candidate(candidate, recovery_hold, current_requirements)`:
validate exact captured request/spec, reusable declared outcome, complete frozen logical content,
selected physical closure and current independent semantic admission. Source-attempt admission is
not required; current candidate admission is. Poisoned, unresolved, interrupted and resource-partial
results cannot be relabelled complete. Recovery holds have `active → terminal_releasing → released`
with explicit consumer completion or qualified expiry-triggered **recheck**. Expiry only queues
release inspection; inspect borrowers/references and nominate remaining roots before release.
History compaction observes recovery references until this finishes.

Canonical `BindingDescriptor/v2` stores the complete validated `CompletedBinding` meaning: boundary,
source kind/relation, requested/resolved profile selectors, source semantic role and dependency/view,
configuration and explicit absent/unrequested distinction. Its key hashes that complete descriptor;
equal view identity alone is not sufficient. Completed-view/publication/staging/recovery roots hold
canonical binding descriptors. Attempt-specific `BindingIncidence {attempt, binding_key}` becomes
releasable only after these authoritative holders exist. Selected backup reads descriptors from the
publication's exact binding inventory, replacing `selected_backup.rs`'s arbitrary stored-binding by
view lookup before incidence deletion is enabled. Attempt close drains incidence with exact attempt
guard; HIST attempt eligibility still checks contributions, cleanup/effects/outcome and lineage
consumers. A retained contribution may keep an attempt needed; binding release never erases it.

Reject age/size expiry deletion, permanent product roots, flat child transfer and binding dedup by
view. Native owns lifecycle/release; compiler owns reusable outcome semantics; publisher owns staged
visibility. Migrate NP, Loader, derived search, references/statistics, attempt cleanup, product lookup,
selected backup, restore and history together. Finite crash-prefix/shared-child/binding examples
passed; future native controls interrupt every nomination/transfer/release page, exercise ended
recovery holds, source-unadmitted reusable completion and fully eligible terminal compaction.

## N07 and N08 — One derivation, actual reconciliation and exact publication eligibility

Select `DerivationIdentity/v1 = H(reference_lowering, participant_lowering, lexical_analyzer,
search_document_lowering, statistics_lowering, vector_lowering, context_eligibility,
model/physical_codec, trusted_definition_epoch, responsible_implementation_inventory)`.
Include actual executable definitions and relevant native source; ordinary producer identity already
carries native implementation (audit correction), but retained old manifests need this **current**
derivation identity explicitly. It belongs in **all derived row/edge/document/vector/statistics keys
and indexes**, staging root, publication marker, SnapshotHandle and current eligibility predicates.
Same-format row addresses are `H(derivation, family, logical_source, local_role/key)`; shared canonical
payloads/views are not rewritten. Every query, resource and continuation uses its handle's exact
derivation namespace and definitions; no “latest” fallback. Old and new same-format realizations
coexist only for named live pins and retire through the root release route. Cross-format readers
quiesce and cut over; no new old-format reader is added.

Each derived family is lowered **once** into an operation-owned bounded/spillable expected stream,
then installed. One independent actual-store traversal compares expected keys/full rows and finds
missing/extra/reference/context/statistical mismatches; expected-stream cleanup waits for this
comparison's terminality. This comparison is physical reconciliation, not independent semantic
validation by repeating the same lowering. Shared model semantic admission remains a separate
premise. Definitions are verified once under their immutable epoch hold. NP finalization consumes
N06's admitted freeze witness and N04/N10 exact original/backing validity, avoiding repeated original
readback only inside that unchanged scope. If any witness is absent/stale, use fresh admission/audit;
never simply skip checks while the stronger protocol is unimplemented. Cold restore independently
checks bytes/typed content and compares regenerated derived claims; it does not reuse source trust.

`try_existing_publication(manifest, derivation, definitions) -> HeldPublication | Miss` performs
one guarded marker lookup/full-byte equality, exact view/completion/binding reachability validation,
nonretiring expected incarnation and pin/hold acquisition atomically before returning. It rejects
stale current-context/negative witnesses; a hash hit is not equality. Retirement conflicts on the
same publication guard. The hit avoids creating staging/derived rows and never proves whole
recompilation did zero original reads. Multiple identical publishers converge; a committed result
with later cleanup failure is reported as committed. C01/D02 fingerprint narrowing must retain all
responsible derivation source rather than merely add a handle hash.

Reject handle-only versioning, mixed derived namespaces, lowering twice as an oracle, and a shortcut
which checks then pins without a common guard. Finite stored missing/extra/changed comparisons,
two-disjoint-derivation addresses and six shortcut mutation classes passed. Actual consumers needing
migration: NP, definitions, Loader/reconciliation, derived_search, NativeReader/selection, source and
catalog resources, search/statistics/vector routes, continuation codecs, backup/import/restore and
retirement. Future controls run two live derivations over one retained manifest, retire one with the
other pinned, race identical publication/shortcut retirement, and corrupt every stored derived family.

## N10 — Blob header and offset chunks, one copy of bytes

Select `OriginalBlob/v1 {full_content_hash, byte_length, byte_format, completed}` keyed by the full
content hash qualified by length/format; `OriginalBlobChunk {blob, offset, bytes}` keyed by
`H(blob, offset)`, bounded to the existing physical chunk allowance. Hash hits compare actual canonical
bytes, not just hashes. `OriginalLayout {artifact_semantic_id, revision/path/input meaning, blob,
range_layout}` retains revision-scoped semantic identity. ArtifactChunk logical rows keep their
current canonical `(source, start, len, digest, body)` meaning; native projection/hydration obtains
body from layout→blob chunks rather than persisting a second serialized body per artifact. Blob owns
chunks; each layout holds the blob. Chunk content is not separately deduplicated.

Blob completion requires checked full EOF, exact ordered `[0,length)` coverage, no gaps/overlaps/
duplicates, each range digest/full hash and byte-length/format match. Empty originals have a header,
length zero, correct empty full hash and no chunks; absence is distinct. Interrupted blob construction
remains under an attempt/staging owner without completion. Shared existing completed blob attachment
uses expected incarnation. Retiring one artifact only releases its layout→blob hold; any other layout,
publication/reader/backup/recovery holder prevents blob retirement. Blob retirement pages chunks.

Migrate **all** byte consumers: C `persist_original_chunk`, `original_stream`, ArtifactChunk field
projection and `hydrate_original_row`; Loader originals; `reconciliation.rs` expected headers,
physical ranges/assembly; native/source/resource retrieval; completed backing validation; model
RecoveryFamily adapters; selected backup original enumeration; detached export/import and restore
original reconstruction; admission; root release and retirement. Portable data carries logical
layouts and unique content bytes/claims, then destination reconstructs native blob ownership and
earns completion; it imports no source attempt authority. Existing manifest semantic artifact IDs
do not merge across revisions. New physical schema/realization identity names this shape.

Reject artifact-keyed duplicate bytes and second chunk-dedup indexes. Finite valid/empty, gap,
overlap/duplicate/prefix/hash-corrupt and two-revision-sharing controls passed. Future native controls
check cross-chunk source ranges, concurrent first blob install, full-byte collision rollback,
interrupted completion, shared-holder retirement and portable cold round trips through every adapter.

## N11 — Exact durable premise key with cold invalidation

Select compiler-owned `AdmissionPremise/v2` with this explicit key:

`H(model_digest, invariant_declaration_digest, validator_factory_implementation_digest,
relevant_producer_implementations, profile, ordered_declared_inputs[(input_role,
requested_selector, resolved_selector, requested/absent/unrequested_state,
logical_view, exact_dependency_completed_identity)], validation_configuration,
physical_realization_schema_and_codec, installation_identity, physical_validity_epoch,
ordered_held_inputs[(record, incarnation, installed_completion_id)])`.

The factory digest is explicit registration from the implementation inventory, not
`Invariant::digest`'s declaration-only hash. The factory declares all semantic/configuration/ambient
dependencies; a validator reading undeclared clock, files, environment or mutable external state is
ineligible for durable reuse. Dependency roles and selectors cannot be replaced by just view IDs.
The record stores success only after all declared inputs and final physical membership/backing
checks complete. Consumption obtains expected-incarnation holds and validates active N04/N06/N10
physical validity under the same guards before trusting success. Missing/obsolete/failed premises
cause fresh validation; coherent wrong bytes or invalid completion are corruption errors, not hits
or quiet misses. No invariant is omitted merely because its dependency relation is empty.

Choose **cold invalidation**, not cross-restart trust recovery. The existing host service/executable
owner supplies a fresh physical-validity epoch on daemon recovery/restart or privileged mutation,
under closed admission/drain and all-stripe transition; ordinary attach refuses unregistered changed
daemon identity. This epoch is different from semantic identity and issuance era. Premises are
durable across compiler processes/attempts while the same protected service validity epoch survives;
after cold recovery, restore, export/import or maintenance changing relevant storage, revalidate
and establish fresh physical completion/premises. Portable exports exclude them as acceptance
authority. Unnoticed external privileged writes lie outside supported ordinary capability access;
granting that access requires invalidating the epoch, not assuming bytes remain protected.

Native owns holds/completion/validity; compiler W `validation_premise` and invariant registration
own logical/factory dependencies; service lifecycle owns cold epoch. Reject declaration-only keys,
portable trust labels and permanent cross-boot acceptance. Finite tests changed all 14 premise fields
and excluded portable physical authority; future controls change factory only, physical incarnation
only, requested selector only, profile only and restart/restore, then verify reuse/refusal/fresh
validation paths and actual backing corruption classification.

## Transition decision — Adjacent conversion when retained obligations cannot be rebuilt

Select a **quiesced rebuild for reproducible disposable project content**, gated by an exact
retained-obligation inventory, and an **actual adjacent old→new transition** for every remaining
named obligation. Pinned library inputs alone do not reconstruct detached external imports, old
same-format pinned derivations, warm products, artifact-only output, exact outcome evidence or
unfinished release/recovery/retirement lineages. Therefore a blanket “bump and rebuild” is rejected.
No store reset, deletion, migration execution or operator adoption is authorized here.

The current `lctx-surrealdb::upgrade` contract covers a particular source4→history-receipt-v5;
its no-backfill progress cannot be reused as evidence for these layouts. Extend its immutable
source/target/executable/protocol/preflight/verifier contract with an actual adjacent target.
Before effects: inventory exact named readers, products, publications, bindings, detached inputs,
backups/cold assets, retained outcomes and unfinished jobs; prove all borrowers drained. For
reproducible content retain named pinned input routes and rebuild selected retained closures.
For other content the migration reads old typed rows under exclusive old authority, converts complete
binding descriptors, captures logical six-family identities, maps exact physical contributor choices,
builds blob layouts/chunks and installed-view completion under fresh destination ownership. Any old
ambiguous/corrupt/incapable closure refuses rather than choosing first contributor. Converted imported
data receives independent destination semantic admission, not source admission tokens.

Migrate live named lifecycle obligations explicitly: fence predecessor effects, retain exact outcome
and evidence references, rehome only remaining cleanup/retirement scope with original cutoffs and
incarnations to the new protocol, preserve durable nominations and successor chain references, and
invalidate admission premises. Completed historical receipts can remain data owned by their named
consumer, never executable old-format runtime authority. Current in-format derivation pins require
exact handle mapping or explicit consumer release before cross-format cutover; no silently rewritten
handle. Commit new marker/schema/definitions only after independent converted-family completeness,
blob hashes, release/lineage inventory and destination admission checks. Partial pages resume using
the immutable migration operation/progress; no dual runtime readers/writes. Remove old physical
content only after destination obligations and consumer release are proven, through the existing
maintenance owner and separately authorized effects. This is a concrete migration requirement,
not an automatic cleanup permission.

Future migration controls cover empty, populated, detached/artifact-only, retained older manifest,
warm product, active outcome reference and interrupted lineage states; interrupt every conversion
phase and verify exact successor recovery plus cold backup/restore. Coordinator owns the separate
cold-backup hash-fusion choice and campaign. All product findings remain **Planned** until these
selected designs are implemented and receive their actual scoped acceptance.
