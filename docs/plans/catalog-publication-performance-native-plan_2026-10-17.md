# Catalog publication performance: native realization and recovery

**Proposed implementation plan, 2026-10-17.** Companion to the [coordinator](catalog-publication-performance-plan_2026-10-17.md).
The coordinator alone owns current dispositions, shared decisions and completion. This document
develops F05/F06/F07/F12 and I02/I04, including cold/candidate/original consumers. It does not
supersede existing plans or authorize operator adoption. All replacements below remain Proposed.

## 1. Foundations and selected direction

Baseline `789223017833c364085ef62f9333df082d88444a` retains the source inspected in the
[native inquiry](../design_review/evidence/2026-10-17_catalog-publication-performance/native-effects-and-lifetimes.md)
and [library inquiry](../design_review/evidence/2026-10-17_catalog-publication-performance/library/README.md).
Native writes already bulk-insert rows and relations. The amplification is the expanded
payload/membership/hold/guard/predicate representation, repeated preparation and point observation,
and a complete host recovery lifecycle around every small physical window.

Select a **versioned prepared installation packet** and **operation-root recovery with bounded
native pages**. Packet preparation and lifetime belong to native execution; the model keeps
logical identities, relation meanings and admission. Prefer compact nongraph immutable blocks
with indexed logical locators and extent-level protection as the physical candidate. Keep graph
adjacency native where real traversal/access consumers justify it. PN00 must settle the actual
layout and its consumers before layout-dependent implementation; this is not permission to
append blocks above the unchanged per-row protection machinery.

No new database, connection pool, workflow engine, generic proof registry or retry framework
is justified. Preserve the current actual executor, classified conflict retry, pins, service
owners and Unknown ownership. Larger128-row windows or replacement of one nested lookup are
possible intermediate improvements, not completion of the structural target.

## 2. PN00 — Decide I04's complete physical and recovery contract

**Consumes:** PP00's RC04/RC05 routes, source consumers and model logical identity.
**Delivers:** one selected schema/codec/access/lifecycle design, its exact consumer migration
map and minimal functional cases. Native implementation that depends on layout waits for this
decision; packet property preparation, PN01 and existing-layout set observations need not.

Compare prepared row packets, compact nongraph blocks with indexed locators, and fully packed
blocks without row locators. The middle candidate is preferred because it reduces payload/
protection objects without forcing point or selected reads to scan unrelated blocks. Fully
packed storage is unsuitable if it hides required selective access. Row packets are a useful
preparatory capability but retain intrinsic expanded per-row machinery.

Settle these concrete outputs using source and small functional constructions:

- Block/extent identity, canonical row order, admitted construction/hydration bounds and logical
  locator representation. Choose exact logical lookup and full-row reconstruction semantics.
- Contribution scans, nominal-key conflict access, node-qualified membership, scope nomination,
  selected hydration, complete-empty/absence and whole logical checksum routes. Identify required
  indexes and their actual consumers; replace rather than silently drop their obligations.
- Equal logical rows arriving in different packets/blocks remain idempotent; conflicting nominal
  payloads within one selected completed view refuse. Independent valid views may select different
  immutable payloads for one nominal key. Locator uniqueness and sharing must preserve that scope;
  physical address equality always requires full identical content. Locator address equality alone
  cannot establish logical payload equality.
- Decide whether content is shared across extents or copied. Shared bytes need explicit current
  reachability and new-reference/retirement exclusion; copying trades storage for simpler
  collection. Do not keep fine-grained holds beneath packed rows without a named requirement.
- Original-byte references, graph endpoint roles, independently received content, semantic
  checksums, transport export/restore, product retention and reader/retirement behavior.
- Exact old objects, codecs, indexes, generic control paths and installation definitions removed
  after migration. Logical distinctions do not automatically require separate physical objects.

Record why the selected realization has a credible route for many fact families, shared content,
high-degree graphs and selected requests. A benchmark is not required. If compact blocks cannot
meet access/lifecycle obligations without equivalent machinery, document the counterexample and
select another conforming realization. A row-packet-only milestone cannot silently close the
remaining F05 amplification. Any unresolved material obligation blocks that part of completion,
not independent corrections.

**I02 route:** explain server work from pinned source first: stored-UDF replanning, scalar locks,
index maintenance and simultaneous arrays/transaction state. Host wire/resident admission does
not bound all server memory or examined work. Request a focused probe only if a concrete choice
depends on unavailable behavior, such as a locator's selective access or locked-target lowering.
Specify the question and decision-changing outcomes before launch. Otherwise record that finer
timing attribution is unnecessary for the structural decision. No blanket benchmark/environment
campaign, inferred server memory attribution or invented speedup.

## 3. PN01 — Correct observation session ownership

**Consumes:** the current actual client/read owner and RC10. **Delivers:** production observation
helpers with borrowed session ownership, plus explicit checked closure for intentional forks.
This is independent of PN00 and should be available early.

`control/held.rs` wraps `client.clone()` in a new Arc. The vendored SDK clone creates a new
authenticated session; `NativeReader::private` defaults to not owning its detach. Change helper
interfaces/callers to borrow the actual `Arc<Surreal>` or existing reader/recovery owner. Preserve
query EOF/drainage separately. Where the consumer genuinely needs an independent lifetime,
construct a deliberate factory-owned session and join its checked close.

Migrate staging readback, retained contribution acquisition, terminal candidate admission and
held-candidate scans, plus the window helper controls. Existing prepared streaming already
avoids accidental SDK cloning; reuse that ownership principle. Do not merge independently
owned readers merely because they happen to use the same transport channel.

**Focused acceptance:** ordinary observations create no incidental child session; deliberate
independent readers survive their source appropriately and acknowledge detachment; failed
query/close/cancellation preserves the exact unresolved obligation. A Drop-queued detach or
host cleanup alone is not checked session finality. No permanent leak or timeout cause is
asserted from the historical source finding.

## 4. PN02 — Prepare once and implement the selected installation realization

**Consumes:** PP00's SC04 and the implemented native owners; PN00's settled layout is required
for layout-dependent installation, selector and collector changes. **Delivers:** the actual
prepared packet and selected physical realization used by compiler ingress and derived writes.

The packet consumes canonical typed rows and exact contribution/model context. It owns charged
preparation backing, immutable values or block payloads, logical memberships/locators, required
graph relationships, distinct targets/parents, attachments, expected incarnations, declared
versus changed incidence, original request/page identity and complete resident/wire admission.
Canonical typed graph anchors travel directly; remove JSON reparsing to rediscover them.
Derive identity/size properties once and carry them through preserving operations into request
hashing, wire encoding and issue. Retry retains the exact original request and owner context.

Define a private unique-entry packet contract for the address shapes its real producers emit.
Use typed indexes or qualified injective encodings for those shapes, with full native equality
checks as necessary. Reject genuinely unsupported/ambiguous declarations before effects; do
not accidentally reject legitimate generated compound identities. Do not introduce universal
Value hashing, stringification collisions or a new public key algebra. NONE/NULL, numeric and
compound equality remain authoritative in the supported domain. Malformed duplicate packet
first-entry behavior is not an invariant that prevents a precise new contract.

Prepare exact existing-value correspondence in sets where permitted, then revalidate actual
commit-sensitive premises under the proper scalar locks and exact conflict addresses. Keep
complete equal-existing acceptance and changed-existing refusal, completed-parent inventory,
incarnation/tombstone checks, declared-target admission even for equal rows, new-holder retirement
exclusion and atomic payload/ownership/outcome visibility. The pinned engine does not accept
an array-valued `FOR UPDATE` target. Unlocked set observation is not a lock substitute.

Fuse compatible payload, locator/membership and attachment work instead of forcing three
independent window streams. Pack by the complete operation's physical expansion and wire/
resident bounds. The changed schema and codecs must be consumed by completed-view selection,
semantic admission, candidate verification, export/restore, publication, evidence hydration and
collection before the new representation is used. Coordinate graph relations and nongraph
blocks through one semantic identity, without rebuilding a parallel canonical store.

Prepare against the existing effect owner first where useful; route accepted pages through
PN04 once its recovery contract is working. No temporary legacy-reader fallback remains at
final cutover. Removal of repeated validation applies only to established preserving paths;
independently received values still earn their validation.

**Focused acceptance:** missing/equal/conflicting logical rows, equal rows across blocks,
duplicate nominal candidates, required indexes/access, changed parents/incarnations, originals,
graph relations, new holds racing retirement, exact wire/resident refusal and full rollback.
Inspect ordinary and independent receiving consumers. Compare reconstructed logical content
and model identities against independently specified rows, not block-address equality alone.
Include two independently valid views selecting different payloads for the same nominal key,
and refusal when their conflicting selections are combined into one completed view.

## 5. PN03 — Share bounded exact set observations across distinct assurance owners

**Consumes:** existing checked readers and SC04 equality/selection contracts. **Delivers:**
set-shaped observation and correspondence in captured candidates, cold state verification and
original completion. It can use the existing layout first; final hydration follows PN00/PN02.

Use one ordinary native-owned bounded set operation, not a general validator framework. It
accepts typed intent and actual protected reader/pin, observes the required identities, and
returns checked correspondence including missing/foreign/duplicate observations. Reassociate
by exact typed identity; permitted set reads omit missing/forbidden rows and cannot be zipped
positionally. If comparison uses native full equality, a SQL join must not silently substitute
SQL NULL or duplicate semantics.

For terminal captured candidates, build the selected contributor universe once. Nominate all
relevant nominal candidates within the selected completed view through qualified membership access, compare full expected values,
then acquire protection and observe backing in sets. Preserve conflicts even when the expected
content-addressed payload also exists. Replace the per-row owner-hold search filtering
dereferenced semantic fields; direct expected-ID lookup alone is insufficient.

For cold `verify_state`, independently consume actual backing and membership correspondence
without fetching backing again for every member. Keep complete inventory, EOF, foreign-node,
missing-node and full-payload checks. Producer preparation does not confer cold receiving truth.

For originals, batch layout, artifact and ownership observations while validating each distinct
layout/source/range. Drain all selected layouts before each unique blob's completion. Preserve
the current shared byte readback/CAS and independent incoming/canonical hashes; do not make
equal blob content erase distinct layout obligations or cause duplicate blob hashing.

**Focused acceptance:** missing/foreign/duplicate observations, conflicting nominal payload plus
expected payload, multiple contributors, shared blobs with distinct valid/invalid layouts,
complete-empty selections and late stream failure. Validate each receiving/retention boundary
independently while sharing only correspondence mechanics and actual protected acquisition.

## 6. PN04 — Replace per-window host recovery with operation-root/native-page recovery

**Consumes:** RC05's governed protocol, PP00's SC05, working native packet/page interface and
actual accepted-work owners. PN00 supplies layout-dependent progress identity; recovery contract
design does not wait for all consumer migrations. **Delivers:** one durable host discovery root
per operation/segment, exact atomic native page outcomes/progress and safe terminal release.

Before first issue, durably bind operation identity, target/generation, original owner/era and
native progress root. Every bounded page atomically commits payload/incidence changes, exact
original request identity/outcome and its acknowledged prefix/progress. Keep pages ordered where
their prefix depends on the predecessor; independently owned segments may execute concurrently.
There is no global writer cap and no automatic segment per semantic kind.

Each page records only its necessary exact receipt and advances bounded progress. Do not copy
the complete operation payload, accumulated receipt inventory or growing acknowledged prefix
into every page. Earlier receipts remain protected through the operation root and indexed
correspondence. Removing per-page sidecars must not introduce quadratic native journal growth.

Retain exact page receipts under the operation root while a legitimate old retry or unresolved
owner may need them. A last-page-only record is insufficient. Replay checks full owner/request
identity and returns the stored terminal result without rerunning payload. An absent page is
fenced against delayed execution. Never allocate a fresh request identity to replay Unknown
production work. Host-loss recovery resolves/fences the named operation from durable native
state; it need not invent resumable semantic computation or reconstruct a production payload.

Terminal operation knowledge becomes durable before releasing the last retaining witness.
Closure joins accepted descendants and excludes late pages. Freeze/close account for admitted,
open and unresolved segments; collection cannot delete their required receipts. Failed settlement
keeps the actual recipient. Original Unknown and later cleanup settlement remain separate facts.

Use existing history page commit/replay as a source-qualified precedent, not a wholesale
maintenance-only implementation transplant. Remove the ordinary two-envelope-writes-plus-delete
cycle for each small page after consumers migrate; operation creation/terminal closure still
have necessary durable ordering. Do not merely defer writes in memory or remove all outcome refs.

**Focused acceptance:** crash before issue, commit before acknowledgement, recovery of an
acknowledged prefix, delayed duplicate of an earlier page, changed request content, partial
refusal, cancellation, absent fencing, generation/era cut and crash around last-witness release.
Confirm actual executor/session finality and zero host reservations separately from durable
native obligations. Broader historical Unknown resolution is not a side effect of these tests.

## 7. PN05 — Narrow terminal cleanup without weakening collector exclusion

**Consumes:** RC06's decision and an actual permanent production fence; PN00/PN04 supply the
new extent/segment reference hazards where relevant. **Delivers:** page-shaped terminal cleanup
that no longer repeats a complete64-stripe production cut without a concrete protection need.

Separate permanent producer eligibility from mutation of terminal history, references and
remaining content. Name the exact guards/predicates and conflict writes excluding incoming
references, collector progress, history deletion and predecessor/successor lineage. Keep owner,
incarnation, origin, epoch and complete hold-page correspondence. Nominate children before
deleting their retaining holds, atomically with recoverable page progress.

Preserve genuine full freeze/close/admission/maintenance transitions. A terminal flag alone
does not exclude context-free references or history collection. If a page still requires a
complete cut, state the concrete race it settles; it cannot be retained solely because the
generic wrapper currently calls it. Coordinate root/extent collection with PN02 and keep
new-holder versus retirement exclusion on the exact authoritative address.

**Focused acceptance:** delayed production, incoming reference and collection races, cleanup
restart, interrupted nomination/deletion, changed era, retained page outcomes and shared content
surviving another owner's cleanup. Confirm no un-nominated content is released.

## 8. PN06 — Integrate codecs, service-owned cutover and regeneration

**Consumes, early slice:** settled PN00 contract, candidate definitions/codecs and existing
installation/target ownership. **Consumes, final slice:** working affected PN02–PN05 consumers,
all changed readers/writers and authenticated shared-target drainage. **Delivers:** one current
physical/control generation and focused functional compatibility with the target.
Necessary consumer migrations ship with their capability, not as optional late work.

The early PN06 slice qualifies and installs the selected schema/control definitions through
the existing owner so actual new-format controls have a real target. It can use an explicitly
owned development campaign while shared targets remain untouched. The final PN06 slice replaces
shared project state only after all changed current readers/writers work and the actual service
owner permits authenticated drain. These slices prevent a dependency cycle between new-format
functional controls and the final cutover; candidate qualification alone is not shared adoption.

Version physical schema, control definitions, codec and realization identities together. Update
relevant architecture and decision owners through PP00's route. Use existing installation,
run/storage and service-maintenance ownership for authenticated drain, installation and
regeneration from pinned project inputs. Coordinate exact source/client/native-executor
quiescence before replacing state. No ordinary scratch fallback, old-format reader, rollback
data copy or preserved obsolete generation without a current named consumer.

Existing protected Unknowns, unresolved executors and cited evidence retain their actual owners.
If they block a shared-service cutover, name that prerequisite and continue independent code/
focused work where possible. Do not reset, discard or relabel them to obtain a pass. Explicitly
authorized destructive/schema controls use the existing wholly owned development-target route,
with exact purpose/generation/consumer and authenticated disposal after complete drain.

**Focused acceptance:** actual small persistent write/read/completion/admission over the new
format; selective locator reads and logical reconstruction; required query indexes; independent
receiving and cleanup; interrupted cutover refusal and checked current generation. No operator
selection/adoption or full real-library regeneration is implied. PP90 owns combined compile/
hygiene/focused verification, PP99 the later full journeys and remaining measured-benefit claims.
