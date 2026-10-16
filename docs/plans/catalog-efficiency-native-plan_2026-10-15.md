# Catalog efficiency: native effects, transport and access

**Proposed, 2026-10-15.** Companion to the [coordinator](catalog-efficiency-plan_2026-10-15.md).
It specifies F02–F04/F07–F08 and I01/I07 from the [source review](../design_review/reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md).
The coordinator §6 alone owns current disposition and dated verification. CE00/CN00–CN05
implementation and focused native controls are complete at the coordinator's stated boundary;
CN06/CN07 are in the approved continuation; CE80 cutover and CE90 assembled qualification remain separate. Specification below
does not establish broader native qualification.

## 1. Responsibility and inspected foundations

The model owns meaning and exact selected inputs. Native control owns effect authorization,
physical identity, commit uncertainty, recovery, reader exclusion and retirement. This plan
changes the physical work and its admission without transferring semantic authority to SQL,
transport configuration or incidental metadata.

Read [storage/publication](../design/sections/storage-and-publication.md), semantic
[physical realization](../design/sections/semantic-model.md#section-15-11), and the review's
[exact library inquiry](../design_review/evidence/2026-10-15_catalog-completion-efficiency/library-capabilities.md).
The source baseline and qualification limits are coordinator §2.

| Source owner | Suitable foundation / remaining gap |
|---|---|
| [ImmutableWindow](../../crates/lctx-native/src/control/immutable_window.rs) | Already admits construction, groups compatible units and composes rows plus holds atomically; internal row lowering repeats shared checks |
| [Effect authorization](../../crates/lctx-native/src/control/authorization.rs) | Exact durable intent, frozen bindings, outcome discoverability and object guards; common attempt predicates serialize otherwise disjoint work |
| [Coordination](../../crates/lctx-native/src/control/coordination.rs) | Fixed complete installation-stripe cut; does not yet establish an attempt participant/freeze cut |
| [Retry](../../crates/lctx-native/src/control/retry.rs) | Awaits the issued transaction terminal before checking cancellation; recovery currently loses stop ownership |
| [Reader](../../crates/lctx-native/src/reader.rs) and patched SDK | Progressive physical-finality route; raw decode/queue precedes application admission |
| [Product recovery](../../crates/lctx-native/src/control/product_recovery.rs) | Protected pin/session retention patterns; not an already implemented general handback recipient |

Suitable common interfaces are refinements of bounded effect plans, exact owner capabilities,
ResourceBudget and finite streams. No new universal native-control framework is required.

## 2. CN00 and CN05 — Complete attempt admission, freeze and close

### CN00: decision and contract prerequisite

F03 is source-established: ordinary intent and payload touch one attempt predicate. A read-only
owner check cannot replace that conflict address, because an older close/freeze snapshot can miss
a newly committed intent. Before removing any fence, define the full participant/effect protocol.

The contract must identify who can admit a producer/window, which intent/payload/reference effects
belong to it, what permanently fences delayed issuance, and which exact generation/era/epoch/
incarnation is checked. Specify how admission closes, how the finite admitted set is established,
how effects are discovered/drained, and how interrupted close/freeze resumes without refreshing
the original request or assuming an absent row proves absence.

Use installation striping as a concrete alternative to compare: ordinary issuance participates
in its selected conflict domain, while a cut validates the complete relevant set. Determine the
appropriate attempt-level domain and complete consumer route. Hash collisions can still conflict;
promise useful disjoint overlap, not conflict freedom. A registry or stripe set must have a
bounded complete access/retirement route; it is not a worker or thread cap.

Cover both [content freeze](../../crates/lctx-native/src/control/frozen.rs) and
[attempt close/cleanup](../../crates/lctx-native/src/control.rs), including OpenAttempt,
FrozenAttempt and terminal-owner effects. Distinguish closing new content from permitting
already admitted completion/reconciliation/cleanup. Inventory checks cannot race later admission.
The protocol must retain outcome references and Unknown obligations until the real owner settles them.

**Decision output:** a concrete chosen protocol, consumer inventory, admissible transitions,
exact cut evidence and failure cases, recorded at the existing architecture/ADR route under RC01.
Source reasoning plus revealing interleavings can settle design; tests alone cannot invent the
protocol. If no complete replacement is established, retain current fencing and F03 open.

### CN05: implementation and migration

After CN00's contract and affected ADR clauses are accepted, migrate intent/payload authorization,
contribution completion, freeze, physical completion, close, cleanup, reconciliation and reference
release together where they participate in the cut. Preserve era-wide installation admission as
a distinct authority. Update schema generation and all actual callers, with exact incarnation
and original request identity retained.

Implement the qualified new route directly. Retire obsolete attempt-wide mutation paths only
after migrated consumers establish complete fencing. Coordinate CN01 edits to shared SQL/schema
and CN04's handback interface; neither an agreed interface nor a passing pure model is native closure.
Regenerate project-owned state through coordinator CE80 if the representation changes.

**Functional controls:** two disjoint producers actually overlap; intent commits after a close
snapshot; admitted payload is delayed across freeze/close; payload commits but response is lost;
producer/effect inventory is incomplete, duplicated or missing; recovery resumes after interruption;
generation/era/epoch/incarnation is stale; terminal/frozen-owner cleanup races new work. Verify
both commit orders and fail-closed cuts, including ABA and live outcome borrowers. Current
thread parallelism stays enabled.

## 3. CN01 — Grouped immutable-window lowering

F02 concerns work inside an existing bounded effect. Keep ImmutableWindow admission, typed units,
compatible grouping, charged construction and atomic rows-plus-holds ownership. Consume one
window and prepare distinct shared parents, exact object guards, creation/incidence obligations
and ordinary/relation row groups.

First settle equality and invalidation meaning: equal duplicate identities within the input
window coalesce where existing semantics permit; unequal duplicates refuse the entire window.
Compare complete existing values, not hashes alone or selected fields. Validate each shared
parent's eligibility once at the actual transaction boundary; retain necessary family-specific
checks. Completed originals refuse missing or changed pieces. Separate all validated existing
rows from genuinely new rows, then use grouped native ordinary/relation insertion where supported.

Pinned INSERT capabilities do not erase per-record storage/index work. INSERT IGNORE and
unconditional duplicate updates are not acceptable substitutes for full equality. Deduplicate
incidence work only after documenting exactly which predicates each logical effect invalidates.
**Implemented / focused-Tested,2026-10-16:** bounded hold cleanup unions the existing
native_incidence_targets of its exact checked page and skips only the exact owner already touched.
Every other target still uses native_touch. The complete owner cut, origin/page/incarnation and
retirement-nomination guards stay in one transaction. Actual one/two-hold and five rollback cases
passed; coordinator §6 owns receipts, unresolved broader finality and the unmeasured latency boundary.
[Completed control](../../crates/lctx-native/src/control/completed.rs) and
[live boundaries](../../crates/lctx-native/src/control/live_view.rs) compare observed revision plus
one; their meaning must survive, with affected consumers migrated in the same package if necessary.

Keep durable intent, payload and outcome-reference ownership distinct unless a complete safer
combined protocol is separately chosen. CN01 may retain the existing attempt fence and proceed
before CN05. It must not incidentally weaken the F03 close cut.

**Output:** one atomic bounded window effect with no repeated shared-parent preparation inside
the window beyond independent necessary protection, preserving exact collisions, era/incarnation,
completion exclusion, rollback and outcome discoverability.

**Functional controls:** equal and unequal input duplicates, repeated parents across families,
equal existing rows, a late unequal row, mixed ordinary/relations, completed and incomplete
originals, exact revision consumers, interruption and rollback of rows plus holds. Inspect the
generated operation shape to establish grouped work; only measured comparisons justify a speed claim.

## 4. CN02 — Backup exclusion without retirement-side global mutation

F04's narrow target is a checked observation of the exact backup gate by retirement, while
backup acquisition/release remain writers. Pinned record-ID FOR UPDATE uses optimistic
snapshot-relative validation, not a blocking or predicate lock. A plain snapshot read is insufficient.

Qualify that removing retirement's revision increment still excludes an active backup under
the actual patched RocksDB realization. Keep exact object/hold guards, incarnation, indexed
bounded pages, atomic nomination and recoverable progress. Reconcile malformed gate records,
count/revision overflow, repeated release and any observer expecting revisions.

Separately assess the supported public backup-hold API. Current selected backup already pins
publication and owns an external snapshot, and the review found no internal acquisition caller
beyond its definition. That search does not prove no external/public consumer. Name the supported
contract before retaining or removing it. The minimum checked-observation fix does not depend
on deleting this API; deleting it requires its own explicit rule/consumer decision.

**Functional controls:** two disjoint retirements; backup acquisition against retirement in
both commit orders; active-backup refusal; release versus retirement in both orders; repeated
release; malformed/overflow state; interrupted/resumed retirement. Use exact native persistence,
not memory-engine evidence as proof of RocksDB validation.

## 5. CN03 — Admit the complete transport ingress

F07 begins before application batches are charged. Current SDK query streaming hardcodes the
record cap to zero; a raw channel can hold two decoded batches alongside a retained delivered
value. The connection adopts a server-advertised message limit unless configured.

### Decision slice

Define a feasible contract covering capability handshake, wire framing, decoded expansion,
raw queue coexistence and delivery ownership. Inspect the exact vendored
[gRPC configuration](../../third_party/surrealdb/src/opt/grpc.rs) and
[stream implementation](../../third_party/surrealdb/src/engine/remote/grpc.rs).
The reviewed Config has a 260 KiB minimum for file-transfer compatibility; do not assume an
arbitrarily small connection ceiling is available. A record cap alone cannot bound a wide row.

Compare exposing per-query record/byte framing and a separately admitted decoder owner with
an allocator/stream adapter that can enforce the actual required bounds. Use existing scoped
ResourceBudget parent/local composition. Establish a defensible expansion bound for the
supported types/compression or an enforceable allocation mechanism; an unproved multiplier is
not an admission guarantee.

A small operation budget may refuse a legitimately wide value, but refusal must occur before
the allocation the contract claims to cover, with truthful supported-width/resource semantics.
Lowering a connection limit without reframing may gratuitously reject a valid batch and does
not solve decoder ownership. Decide how transport charges transfer or overlap with application
charges without premature release or hidden double allocation.

**Decision output:** exact admitted boundary, configuration and protocol/API changes needed,
wide-row support/refusal contract, charge lifetime and capability/provenance updates under RC03.
If a complete defensible route is not found, F07 stays open with its precise limitation.

### Delivery slice

Implement the selected narrow SDK/RPC/server adapter gap and migrate all native finite-reader
entry points that rely on the budget claim. Retain adaptive throughput, progressive decoding,
SINGLE/Rows/EmptyValue distinctions, ordered statement terminals, physical EOF, executor/delivery
joining, cancellation drainage and retained uncertain sessions. Update exact patch provenance,
capability documentation and affected pin holds. Do not substitute buffered queries, transport
timeouts or global concurrency limits. A changed shared transport contract receives final
assembled acceptance at CE90.

**Functional controls:** minimum connection framing; small local budget under a shared parent;
wide scalar/nested/compressed values and one row wider than a desired frame; two queued decoded
batches plus a retained delivered value; simultaneous streams; early refusal; late statement/
transport error; cancellation during decode/delivery and EOF. Demonstrate admission at the
allocation boundary claimed. Quantitative peak-memory claims need separate measurements.

## 6. CN04 — Recovery issuance admission and protected handback

F08 is a different decision from in-flight terminality. Retain retry's rule that an issued
transaction reaches its actual checked terminal before cancellation is considered. After a
conclusive uncommitted conflict, issuing another transaction needs recovery-owner admission.

### Decision slice

Name the independent recovery owner, its stop/admission signal and the exact protected recipient
of a handback. Specify which durable request, SQL/bindings, expected object, original operation,
pins, sessions, references and storage obligations it retains; how a caller obtains the repair/
resume route; and what survives host interruption. Reuse existing protected retention patterns,
not a new general retry framework.

Completion reports separate outcomes; it does **not** itself retain ownership. An Outstanding
or Unknown return without an actual protected recipient is not handback. Missing response can
conceal commit, so no cancellation wait limit may turn uncertainty into uncommitted or cleanup.
Decide who closes new recovery admission while original submitted work remains owned.

Inventory reconciliation and outcome-reference release consumers separately. If payload committed
but reference release is stopped, preserve committed success and the outstanding release obligation.
Do not refresh original identities, expected bytes or bindings on resume. Coordinate CN00's
admission cut and delayed-request protocol; design can proceed before CN05 integration.

### Delivery slice

Replace the empty recovery cancellation context with the selected owned issuance policy.
Check admission before the first new recovery request and between conclusive conflicts.
When stopped, hand back protected obligations and cease new issuance; await already submitted
terminals and retain any unresolved ownership. Route reconciliation, release and other recovery
callers through the actual owner, migrating their reporting and repair consumers together.

**Functional controls:** stop before first recovery issue; stop between conflicts; lost response
after commit; eventual successful reconciliation; repeated handback/resumption; delayed original
requests; committed payload with outstanding reference release; live borrowers at eventual
release. Assert no issuance after handback and no false terminal/cleanup or lost original identity.

## 7. CN06 — I01 selector representation: decision and delivery

**Retain persisted flat / improve private sharing, 2026-10-15 (ADR-0174).** Root-qualified flat
codec1/semantic-v4 mappings remain. Async consumers share protected descriptor/cap handles or admitted
live vector owners; synchronous consumers borrow slices and public vectors materialize once. Fragment
persistence would add receiving/identity/GC machinery while original root/copy/incarnation checks
remain necessary. Reopen if residual flat emission/reconstruction becomes material; no semantic or
wire rule change is selected. Coordinator §6 owns delivery and actual controls.

Compare current flat root-qualified mappings with immutable qualified fragments over reconvergent
suffixes. First inventory actual root/copy/selector consumers and ordinary overlap, using
representative model shapes rather than assuming flat output is the bottleneck. Include compiler
composition, earned completion, completed transport, independent receiving, originals, export/import,
selected backup, cold reconstruction and retirement.

The comparison must preserve ordered identities, root-specific physical choices and incarnations,
complete conflict/absence checks and authority at receiving. Sharing suffix bytes is not evidence
that two roots may share validity. Include fragment construction, lookup, charge/retention,
reconciliation and garbage-collection machinery in the comparison.

Select factorization only if it removes material repeated work through a credible simpler complete
route. Otherwise retain the flat representation with its reason and revisit workload. A changed
codec/semantic representation needs additional explicit rule approval and the architectural
decision route before delivery; RC01–RC04 do not cover it.

If selected, deliver producers and every consumer plus target-state regeneration; remove obsolete
format paths, with no legacy reader or rollback generation retained solely for compatibility.
Coordinate CC00/CC03 contracts before dependent implementation.

**Functional controls:** reconvergent suffixes with equal and conflicting copies, distinct
incarnations/roots, missing entries, ordered identity, independent receiving, original lookup,
backup/export/import and cold reconstruction. Retained flat identity rules cannot be inferred
from successful fragment decoding alone.

## 8. CN07 — I07 query-shaped lexical access: decision and delivery

**Related selector correction implemented / native acceptance pending,2026-10-16.** Reverse
atomic-field nomination submits key windows of32, matching the pinned planner's IN expansion
limit. Transfer batches,1024-root reference windows and execution concurrency retain their
separate owners. The production query builder also supplies EXPLAIN controls for33/65-key
selections, exact frozen membership and newer-owner exclusion; no measured latency claim follows.

**Select shared postings, 2026-10-15 (ADR-0174).** Native document v2 stores length; one immutable
`lexical_posting` per analyzed-document/term stores frequency, shared across selected scopes/occurrences.
Scoring uses bounded document/query-term compound lookup and one matched-row grouping. Selected-corpus
statistics and scoring policy stay. Existing frequency construction, generic immutable endpoint holds,
derivation inventory, five-family expected reconciliation and cold regeneration provide the complete
lifecycle. This is a definition/derivation-qualified physical schema migration; fresh owned targets
regenerate, with no legacy reader. Actual two-dimensional EXPLAIN and scoring controls are required;
coordinator §6 owns their outcome. Shared/operator adoption remains CE80.

Actual current EXPLAIN exposed whole-index iteration for two independent IN lists at the pin.
The selected application correction maps candidate documents and distinct query terms to scalar
two-column equality selects within one enclosing query, flattens matches and orders once before
unchanged grouping. Both pinned planner paths support those full compound equality keys. Explain
inside the same closure route, because a top-level lookalike can use another planner. This needs
no schema/server change; runtime exact-key and scoring acceptance remain with coordinator §6.

The original diagnosis was eligible-document selection followed by filtering each complete stored term
array. Compare indexed postings or another candidate/query-term intersection access path with
compact arrays and bounded candidate windows. Decide whether actual unrelated-term work justifies
added rows/indexes and their construction, reconciliation, backup and retirement.

The lexical statistics owner retains exact frozen selected-view/family N, document frequency
and lengths. Preserve analyzer identity, zero-IDF matches, eligibility before caps, candidate
deduplication, deterministic ties/fusion and winning witnesses. Native table-wide BM25 is not
equivalent to this selected corpus; changing scoring policy is an additional decision.

A selected access path must constrain both candidate and query-term dimensions at the physical
plan, rather than move the same scan behind a wrapper. Exact native query-plan evidence can
settle access-path uncertainty; timing is optional. No embedding inference or submitted MCP
runtime is required.

If beneficial, deliver construction and consumption together with lifecycle/schema updates and
regeneration; otherwise record why arrays are preferable for the supported workload and the
revisit trigger. Do not add a speculative second index without its consumer.

**Functional controls:** empty query/corpus, zero-IDF match, duplicate terms/documents, selected
corpus differing from global corpus, excluded candidates before caps, ties, fusion and witness
identity. Use independent known scoring answers and actual native selection for access claims.

## 9. Integration, acceptance and retirement

**Closeout refinements,2026-10-16:** pin release deduplicates incidence endpoints within its
checked bounded page; the separately committed release fence and exact owner/current holds remain.
Its actual native revision/peer-preservation control passed, with receipt in coordinator §6.
The proposed read-only admission-stripe replacement is rejected: tracked RocksDB reads alone
cannot force the maintenance-first/intent-first predicate interleavings to conflict. Retain all
current stripe writes. Ordered operation errors now remain separate from actual finalization;
retry inspects every related cause and rejects mixed/uncertain/finalization-bearing outcomes.
Original Unknowns remain protected. Source acceptance is distinct from outstanding enclosing
compiler/native/reconstruction controls, owned by coordinator §6.

CN01 and CN02 can proceed independently; CN00 precedes CN05, and CN03/CN04 need their decision
slices before delivery. Shared authorization/schema/reader files have one integrator even when
designs are ready in parallel. No package introduces a runtime serialization barrier for editing convenience.

Every changed public capability or persisted representation migrates its real producer and
consumer in the same delivery. Optional complete replacements can be chosen on technical merit,
but a larger store or local-spill pivot must account for the whole operation and receive any
additional rule approval. Existing SDK EOF/session patches and external snapshot ownership stay.

Use focused controls under existing fixture ownership and current profiles; qualify exact RocksDB
and SDK semantics where claimed. Compile affected crates and run applicable scope-end hygiene,
then participate in coordinator CE90. CE80 owns authenticated cutover and obsolete-generation
removal. No original failure/Unknown, active borrower or operator state is erased to make a control pass.
