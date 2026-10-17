# Catalog upper execution: native preparation and admission

**Implemented / focused-Tested, 2026-10-17.** This standalone companion specifies native
execution changes from the [upper-execution review](../design_review/reviews/design_review_catalog-upper-execution-performance-target_2026-10-17.md).
The [coordinator](catalog-upper-execution-plan_2026-10-17.md) owns shared contracts, approved
rule directions, dependencies and the sole §6 disposition ledger. This document does not
supersede existing plans or own their unresolved native obligations.

The [coordinator’s current checkpoint](catalog-upper-execution-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint) owns the subsequent corrections and the2026-10-17 functional audit at `160c5254`. The audit traced UN01–UN05 through actual native/compiler/publisher callers and found no remaining native-package implementation gap. The pending native Summary journey is a separate cross-track confirmation; FA01/FA02 in the compiler failure paths precede it. Earlier focused receipts retain their original scope.

## 1. Basis and foundation assessment

Source inspected at **d37a14add3e18d6a101aa4710dcd279b5445e896**, 2026-10-17. The implemented
baseline has exact indexed membership access, current contributor/context checks, protected
operation pins, successful-work coalescing with failed-flight eviction, bounded external
ordering and independent receiving admission. These are foundations to extend, not competing
systems to replace. The [storage owner §6.1](../design/sections/storage-and-publication.md#61-admitted-content-and-native-publication)
and [semantic owner §15.1](../design/sections/semantic-model.md#section-15-1) remain architectural
authorities. This paragraph describes the authoring baseline; current UN01–UN05 implementation,
verification and remaining work are recorded only in [coordinator §6](catalog-upper-execution-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint).

| Current mechanism | Source and assessed mismatch |
|---|---|
| Live earned selectors | `crates/lctx-native/src/installed_view/earned.rs:293–331` audits, prepares full logical contributor runs and retains them, although selected-member access consumes relation membership partitions |
| Assurance and logical preparation | `installed_view/audit.rs:2598–2676,2736–2813,3014–3134,5152–5294` perform protected membership/backing acquisition separately; full logical preparation is eager on live selection |
| Capacity fallback | `installed_view/audit.rs:5126–5150` retries a whole contributor with a smaller page after a terminal local Resource, including completed membership work |
| Original completion | `crates/lctx-compiler-store/src/compiler.rs:6092–6148,6303` probes both original-bearing families for every contribution and reads payloads one at a time |
| Blob and range assurance | `crates/lctx-native/src/originals.rs:436–546` revalidates bytes and deduplicates blobs only within a call; `installed_view/audit.rs:3137–3263` independently reads ranges and a full blob |
| Nominal agreement | `crates/lctx-compiler-store/src/admission.rs:2321–2387,3009–3039` plans pairwise complete typed comparisons; `admission/reference_preparation.rs:61–94` already materializes original inputs once |
| Sort execution | `installed_view/audit.rs:623,4771–4783,5152–5294,6220–6251,6306–6338` invokes synchronous ordering inside async work; `crates/lctx-native/src/acknowledged_candidates.rs:84–110,672–715` already owns acknowledged blocking execution |

The supported workload includes many contributors, original-bearing and original-free outputs,
shared blobs across families/pages, repeated view selections, nested payloads, skewed rows and
concurrent logical attempts. Cold preparation must avoid unnecessary state; reuse must remain
bound to actual lifetime and trust premises. Benefits are reasoned from removing acquisition,
sorting and repeated joins; no timeout attribution or quantitative speed claim follows.

The operator approved **RC02, RC05, RC06 and RC08 on 2026-10-17**, retaining their safety and
equivalence conditions. UE00 owns their ADR/architecture routes before dependent implementation;
this plan does not amend accepted ADR bodies. Core FP-07/A4, DP-10/19/20 and the efficient
architecture heuristics guide demand separation, bulk access, retry scope and worker placement.

## 2. Native target and boundaries

Protected acquisition belongs to the existing InstallationAudit or producer completion operation.
It observes complete promised inputs under live exact context, contributor incarnation, native
client, pin and resource authority. Consumer preparation derives compact membership partitions
or demanded full logical runs from those observations. Neither product grants compiler semantic
admission or substitutes for independent receiving observations.

Ordinary live selection retains sufficient audited membership and exact contributor protection.
Frozen/retained identity, logical checksum and export consumers explicitly demand complete
normalized logical runs. An operation needing both feeds compatible consumers from its protected
acquisition rather than repeating acquisition. A later full demand cannot pretend an earlier
membership-only result retained backing it did not retain: reacquire what is necessary under
current checked premises, or consume an actually retained protected source. Successful scope-local
preparation is reusable only while its complete validity premises and actual owner remain live.

Original handling follows the same boundary: producer completion can reuse its checked blob work
across layouts/pages, but receiving independently earns its own observations. Relation-group
agreement compares complete selected typed payloads and retains every original source's authority.
Blocking sort execution changes placement, not who owns pins, scratch, reservations or drainage.

All paths preserve complete output digests/counts and EOF, canonical payload validation, endpoint
and original associations, complete alias reachability, final guarded visibility and transport
drainage. Failed flights and poisoned sorters cannot publish successful prefixes. Cancellation,
Resource, primary/secondary errors and native Unknown remain distinct. No global proof cache,
new scheduler, permanent duplicate canonical store, thread/job cap or larger deadline is selected.

## 3. Work packages and dependencies

| Package | Source obligations | Required capability before integration | Delivered capability |
|---|---|---|---|
| UN01 | F01, I04, RC02 | UE00 settles protected-acquisition/demand contracts and accepted-rule routes | Audited membership preparation for selectors; full logical runs only for actual demand, with migrated consumers |
| UN02 | F02; Resource portion of I09 | UN01's implemented immutable membership/backing-ID preparation and typed page boundary | Exact effect-free page retry; completed membership work is not replayed |
| UN03 | F03, RC05; original-sharing portion of I09 | UE00's output-absence and operation-local preparation contracts | Scoped completion, bounded payload sets and exact blob/range preparation within each trust boundary |
| UN04 | F04, I05, RC06; multi-view portion of I09 | UE00's full typed equality and independent-source contracts; I05 settled locally | One complete relation-group disagreement operation with atomic premise publication |
| UN05 | F14, RC08; native preparation part of F13 | UE00's acknowledged blocking/private-compute contract | Audit sorting/folding uses joined blocking execution; native preparation supports eligible contributor overlap |

The table is not a whole-document barrier. UN03 output scoping and UN04 equality investigation
can proceed independently after their specific contracts. UN05 can establish the reusable
blocking owner before UN01 migration, while final integration must include UN01's resulting sort
sites. UN02 requires working preparation products, not merely an agreed interface. US02 owns
the wider F13 overlap implementation; it consumes UN01/UN05 native contracts and owns native
contributor launch/integration sequencing. Shared `audit.rs` changes need one editing owner or
sequential integration even when packages are logically independent.

<a id="un01"></a>
### UN01 — Demand-separated audited membership and full logical preparation

**Investigation I04:** enumerate every caller retaining or consuming logical contributor runs:
live selection, compound frozen/retained boundaries, checksum, family identity composition,
export/backup, retained/cold readers and detached incoming/installed admission. Trace the actual
owner, demanded families and lifetime rather than inferring demand from a shared helper name.
Record the demand classification in the owning architecture section during implementation.

The target is the approved lazy split. Keep a fused full-preparation path only where an actual
full-identity consumer needs it. Ordinary selector calls must not construct backing runs merely
because the current earned capability retains them. Assurance still reads and checks the full
promised contributor, including unselected members; selected relation demand cannot narrow that
promise. Necessary assurance should produce reusable membership partitions while its rows are
already acquired. If full logical identity is demanded at the same boundary, also feed its
normalized contribution/membership/backing runs from that acquisition, under separately charged
retention. Avoid retaining rich rows solely to make a hypothetical future demand cheap.

Migrate earned capability construction and selected-member composition to the compact product.
Migrate every actual full consumer to explicit demand, including indirect family-root and export
callers. Remove eager `_prepared` retention and superseded duplicate preparation calls once their
consumers move; keep one exact logical normalization recipe and unchanged flat six-family v4
meaning. Do not add a legacy reader or a second identity algorithm. If inspection establishes a
previously undocumented full selector obligation, resolve its owner and rule impact before
weakening it; an implementation shortcut is not evidence of that obligation.

**Focused closure:** an ordinary exact selector produces independently specified members without
constructing full backing runs; full-root demand produces the same six-family roots for equal
logical content under differing physical copies. Exercise overlaps, complete-empty views,
alias chains/cross-owner reachability, unselected malformed backing, late membership failure,
changed incarnation/context/pool and cancellation. Keep failed-flight eviction and retained
protection through the last borrower. Full-root equivalence must compare against the existing
logical recipe or independently specified fixture rows, not two calls to the same new mapper.

<a id="un02"></a>
### UN02 — Retry only the reducible backing page

Consume UN01's completed membership partition and immutable ordered backing-ID preparation.
Separate exact page acquisition from normalization/sorter lifetime. On a typed capacity refusal
that identifies reducible page workspace, discard that page's temporary observations and retry
its exact IDs in smaller bounded groups. Do not restart contributor membership scanning or
normalization. Preserve exact missing/extra-ID, canonical payload and finality checks for every
accepted page; publish the complete product only after all pages and drains succeed.

The Resource classification must describe the failing workspace, not parse error text or treat
all terminal local Resource as page pressure. Retained-run/sorter refusal, an irreducible row,
unrelated reservations, transport failure, Unknown and context drift retain their original
terminal meaning. Singleton acquisition retains its current capacity/refusal behavior. Failed
sorters remain poisoned: do not append a successful retry into a partially failed builder. Where
the actual operation cannot isolate a failed page from its sorter, stage a complete page before
committing it to that builder under admitted workspace.

**Focused closure:** force a later-page reducible refusal and verify membership work remains
completed, exact backing IDs are neither omitted nor duplicated and only the failed page is
reacquired. Force a singleton refusal, sorter refusal, late stream error and cancellation to
show no false success or unrelated fallback. Extend the existing
`backing_window_resource_retry_never_erases_completion_or_context` controls. UE01 may inspect
retained evidence for I09 occurrence; absence of an observed fallback does not block this repair.

<a id="un03"></a>
### UN03 — Exact output scoping and bulk original work

Pass the earned contribution output inventory/counts into original completion. Skip a family
only when that current inventory establishes it absent; a guessed profile, empty query result
or mutable statistic is insufficient. For present SourceArtifact/ArtifactChunk memberships,
collect bounded exact payload pointers, retrieve them as sets and verify cardinality, ownership,
content, semantic keys and canonical layout/source metadata before original work. Preserve the
ordered cursor and exact EOF contract. Remove scalar payload queries and unconditional absent
family probes from this completion path.

Implement one sealed operation over spillable exact layout IDs and compact blob/layout work. Validate each distinct layout's source association and hold, then drain all pending layout work before completing unique blobs. This shares work across pages and both original-bearing families without a retained revision cache or invalidation from later own drains. Every blob preserves its exact incarnation/revision CAS and independent byte readback. Conclusively observed same-incarnation premise changes follow existing owned recapture/revalidation; retirement, ABA, remote uncertainty and unrelated errors cannot authorize another effect or reused success. A completed flag alone never proves byte integrity.

Independently extend receiving `audit_original` so a protected byte acquisition feeds compatible
full-blob and range-hash obligations together. Use the existing `original_bytes_batch/read_ranges`
route where it supplies exact ranges, preserving full-blob audit and each layout/range/root
association. A full audit may stream bytes once into several hashes without retaining a whole
blob. Keep producer and receiving preparation separate, even for identical content. Remove
superseded repeated readback inside each migrated boundary, not the independent boundary itself.

**Focused closure:** original-free earned output issues no original-family query; a blob shared
across pages and source/chunk families earns one successful byte preparation within the operation
while all layouts/holds validate. Receiving starts fresh. Exercise overlapping/disjoint ranges,
changed layouts, missing pieces, byte corruption, revision change, incarnation change and late
read failure. Check exact returned bytes/range hashes against explicit fixture bytes. Original
sharing frequency informs optional preparation representation, not whether scalar queries and
earned-absent work are removed.

<a id="un04"></a>
### UN04 — Groupwise nominal payload agreement

**Investigation I05:** establish an exact relation-group operation using current pinned
DataFusion/Arrow inputs. Compare union/group execution with sorted ID-group comparison; consider
native grouping only if its actual typed semantics and access plan satisfy the same contract.
Inspect exact local implementations and physical plans for supported payload types. The default
is to compose the existing prepared reference targets into one relation-group operation; select
its mechanism by demonstrated complete equality and bounded workspace, not backend preference.

For each same-relation group of applicable selected sources, compare every complete typed payload
sharing a nominal ID. Equal duplicate rows are allowed; any field disagreement refuses before
reference-family use. Equality must match the current per-field `IS DISTINCT FROM` behavior,
including NULL, nested values, ordered lists and schema-specific semantics. `COUNT DISTINCT`
that drops NULL, an unverified canonical encoding or a digest-only comparison is unacceptable.
If grouping cannot express exact supported equality, a sorted stream can compare each ID group
using the authoritative typed comparison; if neither is established, retain current comparison
temporarily and keep UN04/F04 open rather than weaken equality.

Observe every original selected owner currently required by admission before publishing any
premise. Keep earned-empty authority checks and identity-only special cases. Reuse the existing
per-input materialization once; retire the nested pairwise planning loop after all supported
relation groups migrate. Group processing can share data inside its independent receiving
boundary; it cannot import producer trust. Preserve cancellation and failure-atomic publication
of pending reference premises.

**Focused closure:** compare old and new complete-payload agreement on explicit rows with three
or more owners, equal duplicates, absent IDs, NULL/non-NULL, nested structs/lists, changed list
order, identity-only and complete-empty sources. Include conflict in a late source, original-owner
drift and cancellation to expose premature premise publication. Inspect the resulting plan/work
structure to establish one grouped operation per relation without asserting measured latency.
I09 multi-view occurrence remains optional prioritization evidence.

<a id="un05"></a>
### UN05 — Acknowledged blocking sort and native concurrency contract

Move substantial synchronous sort, merge, fold and spill work in audit preparation onto the
existing acknowledged blocking owner. Extend its command vocabulary with bounded row batches
or a complete bounded unit as required, avoiding one handoff per row and avoiding a new pool.
Async native transport remains asynchronous; blocking workers consume already owned inputs or
receive bounded admitted commands. Transfer reservations and actual guards with submitted work.
Audit task completion must include worker terminality, scratch release and primary/secondary
failure composition, even after the waiting caller is cancelled or lost.

Cover logical contributor preparation, local aliases, selected-member composition and family
folds, including the UN01 demand-separated paths. Remove direct substantial synchronous file
I/O/merging from those async bodies. Retain fitting in-memory and spilled behavior, current
logical ordering, poison exclusion and final pin/context checks. Merely replacing `tokio::spawn`
with `spawn_blocking` is insufficient because started blocking work cannot be aborted.

Provide US02 with independently owned contributor-preparation work products and terminal handles.
US02 migrates the sequential contributor launch loops to eligible overlap after duplicate work
is removed. Sharing immutable prepared inputs does not permit concurrent mutation of one sorter,
unreserved queued results or publication in arrival order. Actual input/work/output admission,
deterministic integration and retained last-borrower ownership govern useful overlap; no arbitrary
concurrency cap is a substitute for that contract.

**Focused closure:** fitting and spilled runs yield identical expected order and roots. Pause a
merge and demonstrate independent async I/O/cancellation progress; cancel or lose its waiter and
show acknowledged work remains owned until terminality. Exercise late worker failure, poisoned
builder, cleanup failure and reservation release. Under US02 integration, block one contributor
and show an independent one progresses, shuffled completion preserves results and every started
worker joins after failure. This establishes placement/ownership, not a measured starvation cause.

## 4. Verification, cutover and completion

Implementation uses compile checks on affected crates and minimal revealing contract controls.
Pure equality/ordering/preparation controls need no database. Native persistence cases use the
existing stable logical fixture only after authenticated readiness; no scratch service, maintenance,
regeneration or operator adoption is implied. Protected prior Unknowns remain with their existing
owners and are not cleared by passing new controls or confirmed host cleanup.

Representative command routes are `cargo check --release -p lctx-native -p lctx-compiler-store`
and explicitly filtered Rust controls through `just fixture -- cargo nextest run --release ...`
where a native fixture is required. Select actual test targets after implementing the contract;
do not run broad families merely because several packages share a crate. UE90 owns integration
with consumer/scheduling packages, R01 feedback separation and applicable scope-end hygiene.
Use normal parallelism, unchanged deadlines and the existing run/capture owners. Full Catalog
campaigns, assembled qualification and benefit measurement are outside this plan set's completion
endpoint; their omission cannot become a claim of full product acceptance.

No schema or portable format change is presumed. If a selected physical representation requires
one, UE00 must resolve its owner/decision route before dependent implementation, then schedule
authenticated quiescence and regeneration from pinned inputs rather than retaining compatibility
readers. Existing retained/Unknown state remains protected until its actual lifecycle authority
settles it. Do not use target regeneration as an unapproved way to make controls pass.

Completion requires all named consumers migrated, superseded paths removed, exact logical and
trust contracts preserved, package-specific focused controls passed and consequential decisions
recorded at their architecture owner. I04 closes with an explicit full-demand consumer map; I05
closes with supported equality/mechanism evidence. I09 frequency evidence may remain unmeasured
without blocking justified corrections. Record actual outcomes only in coordinator §6; package
requirements above remain the closure contract rather than a second mutable status register.
