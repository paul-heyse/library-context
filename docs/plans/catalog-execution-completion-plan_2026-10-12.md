# Catalog execution: protected completion and canonical preparation

**Accepted target / Proposed implementation, 2026-10-12.** Companion to the
[coordinator](catalog-execution-plan_2026-10-12.md), which alone owns dispositions and receipts.
This document develops Catalog-review F01–F04 and the surviving native lifecycle obligations.
[ADR-0165](../adr/0165-prepared-completion-and-owned-lifecycle.md) records approved RC01/RC02 and
retains v4 identity. Existing fragment reuse remains a useful implemented foundation; it is not
the completed target described here.

## 1. Foundation assessment and actual consumer boundaries

The workload has many declared output relations, cumulative overlapping views, explicit empties
and later normalization/analysis. A new relation must add its necessary content and semantic
checks without automatically creating a complete shared-state hash, member copy, seal and retention
lifecycle. Existing typed completed descriptors, exact physical mappings, native request ownership,
canonical runs and independently earned admission provide the required foundations.

Current internal uses are operation-local but physically demand InstalledView authority:
`CompletedInputs` chooses predecessor relations and freezes semantic bindings in
[workspace](../../crates/cpg-core/src/workspace.rs); DataFusion provider registration uses
[compiler_provider](../../crates/lctx-compiler-store/src/compiler_provider.rs), and
`CompletedRelation::prepare_batches` routes through registered-view scans in
[compiler](../../crates/lctx-compiler-store/src/compiler.rs). Their streams live under the compilation
Workspace/session. These are the first consumers to migrate, including projected/native/model
streams, not merely one normalization example.

Current `complete_contribution` marks its descriptor completed before constructing/installing all
singleton and cumulative output views. Descriptor completion, structural validation, a digest,
a shared future or a cache hit cannot grant the replacement authority. Exact actual member/backing/
original/alias completeness must first be earned and protected through consumers' complete lifetimes.

| Consumer class | Target authority and physical purpose |
|---|---|
| Same-operation provider, normalization, analysis and checker input | Earned protected contributions plus exact logical relation/contributor/role selection; shared local row/topology preparation as demanded. No mandatory independently sealed per-relation realization. |
| Semantic bindings and contribution dependencies | Retain complete immutable descriptors, explicit empties, roles and transitive selected closure. A semantic reference is not itself a request for a member copy or full six-family identity. |
| Retained reusable compiler product or transfer to a different lifetime | Explicit independently earned boundary owner retaining exact contributing copies and dependency closure. Reuse the existing boundary when its contract suffices; never extend operation-local authority by serialization. |
| Frozen admission and publication | One complete inventory and v4 logical identity for the actual frozen semantic boundary, with exact physical completion, definitions and independent semantic admission before publication. |
| Fresh-process serving, sparse get/search/evidence and publisher audit | Protected published boundary plus exact view selector/mapping and useful indexes; reader pin covers rows, aliases, originals and continuations until drain. |
| Selected backup | Exact selected binding/input closure, marker/inventory and incarnation checks in its coherent transaction; maintain physical-copy selection and byte ownership. |
| Detached import, restore and cold recovery | Independently validate incoming and actual installed state, earn new local physical authority and semantic admission; no portable acceptance witness. |

## 2. CP00 — establish and encode the earned input/boundary contract

Split three operations while retaining one semantic model:

1. **Complete and protect a contribution.** Independently validate the full completed descriptor,
   declared output inventories/counts/digests, actual canonical membership/backing, originals and
   local alias incidence/endpoints through EOF/native finality. Retain selected positive active
   incarnations, exact expected rows, relevant holds and definition/codec/validity context.
2. **Select an internal view.** Bind those earned contributions to the exact relation, dependency
   role, model/profile and immutable descriptor. Prepare demanded rows/indexes without an independent
   durable realization. Composed alias reachability is evaluated over the exact selected contributor
   closure, including cross-contributor chains. Complete-empty and absent remain distinct.
3. **Earn an independently retained boundary.** Freeze/drain identity-bearing mutations, validate
   composed view/binding/alias inventories, earn the actual boundary's physical completion and v4
   logical identity, then permit independently earned admission/publication or retained attachment.

The operation capability owns the real session/pin/cancellation and resource pool. Its reusable
preparations bind exact selected physical copies/incarnations/full descriptors and interpretation
version; equal logical content does not choose a different copy. Actual borrower ownership prevents
Workspace release from leaving an apparently valid detached `CompletedRelation`. Either retain the
needed parent owner through the stream or refuse after its lifetime ends; do not quietly extend a
capability by retaining a descriptor. Failed/cancelled/Unknown preparation publishes no reusable
success and retains actual native obligations until owned drainage/disposal settles them.

**Chosen durable representation:** retain model-owned logical `CompletedView` descriptors and
bindings, but map them to independently necessary boundary realizations and exact selectors rather
than one full physical realization per semantic relation. A frozen admitted/published operation
owns a compound physical completion over its exact contributor/dependency inventory. Multiple
logical views select within it using their declared relation and contributor closure. A separately
retained product may earn a separate boundary because its lifetime is independent. Materialize
member/access indexes once per needed boundary where sparse readers require them; no automatic
member copy/seal per internal output. Exact selected aliases remain scoped to each logical selector,
not every alias visible in the enclosing union.

For an independently retained view, its validated exact selector together with the protected
completed boundary is its view-local physical realization under ADR-0152. Their combined authority
binds complete selected contributor/dependency closure, exact incarnations and codec, with full
member/backing, collision and missing/extra guarantees. Boundary completion alone grants no unchecked
selector or wider membership. Freeze selectors/mappings with every other identity-bearing surface.
Internal selections cannot be serialized into retained authority.

Define the physical selector codec, descriptor-to-boundary inventory and validation together in
model/native owners. Version the changed physical codec/schema/definition identity deliberately;
retain semantic v4 identity and existing logical view meaning. Per-view legacy completion references
cannot coexist as another runtime authority after migration. A shared compound root does not let
one view see another view's members or interpret complete union absence as selected-view absence.

**Decision deliverable:** explicit producer/reader/freeze/retention contracts and source-backed consumer
inventory, plus physical-format changes and independent refusal scenarios. The current source review
supports this direction. A fresh bounded design/target assessment of consequential specified
interfaces must precede CP04 integration; it need not repeat the full codebase review. CP00 is design,
not evidence that any authority is implemented. A new cross-operation proof cache is not selected;
reopen only for a concrete repeated fresh-owner consumer that existing retained boundaries cannot serve.

### Preserved mutation premises and IF04 decision

The inherited writer map remains relevant:

| Mutation | Required validity treatment |
|---|---|
| Header/layout or piece creation/replay | Exact immutable declaration and source/layout associations; layout→blob and blob→piece holds. Generic object guard and incidence predicate are separate mechanisms. |
| Pre-completion finalization | Capture active incarnation/revision, independently read full content, compare relevant premises in the original owned effect. Only conclusively observed typed `premise_changed` permits owned recapture/revalidation. |
| Already completed immutable blob | Real pin and independently validated blob→piece hold chain protect exact declaration/bytes; check same active positive incarnation before/after initial observation and on reuse. Equal replay guard churn alone is not byte invalidation. |
| Retirement/recreation | Retiring/inactive or changed incarnation refuses. Held completed contributions/blobs retain their immutable namespaces and piece ownership. |
| Privileged maintenance | Requires real quiescence and invalidation/reconstruction; no live preparation spans out-of-protocol writes. |

IF04's continuous equal-replay amplification is not established as the timeout cause. CP00 includes
a bounded writer/effect assessment: compare removing redundant replay demand under CP04 with
transaction-local exact replay recognition. Specialize invalidation only if it preserves every
owner/incarnation/declaration, retirement/completion exclusion, incidence predicate and new-hold
obligation, while invalidating real piece creation. A client equality precheck or omitted touch is
insufficient. If no complete simpler specialization is established, retain conservative typed
recapture and the explicit continuous-replay refusal/trigger in coordinator §6; do not leave an
unspecified narrower fence as an implementation prerequisite.

## 3. CP01 — immutable canonical runs and non-consuming ordered union

Extend [ordered_rows](../../crates/lctx-native/src/ordered_rows.rs), whose `PreparedRows` already
owns Arc-backed memory/disk runs, independent cursors and a point index. Its adapter named Physical
also carries normalized logical `Value` rows today. Add the smallest prepared-frame cursor and
non-consuming fold/union interface; no additional library or sorter/cache owner is required.

Lower to canonical logical rows **before sorting**. Contribution/view/binding IDs are normalized;
membership IDs depend on logical completed owner/relation/nominal key. Physical membership order
or relation/nominal order cannot substitute for resulting `RecordId::Ord` order. Existing protected
contributor preparation already performs that sort once and is the reusable source.

The owner fixes ordering and equality: `RecordId::Ord`, equal key/equal JSON payload emits once,
equal key/unequal payload refuses. Expose preserved JSON row payload bytes, excluding the spool's
eight-byte length prefix. `CompletedFamilyHasher::row` retains its own tagging and final count;
state identity remains `native-completed-state/v4`. Empty inputs produce the appropriate empty
family stream/root. Input runs remain reusable after successful merge, conflict, cancellation or
local failure. Return/cache a complete run/root only after every required input EOF and successful
fold. A callback may see a prefix, so it must build unpublished local state until success.

Charge handles, readers/buffers, decoded keys/heads, comparison payloads, transient decoding and
output metadata/bytes before allocation. Memory/memory, mixed and disk/disk routes share semantics.
Use an admitted multiway merge when its working heads fit; otherwise balanced bounded merge stages
write owned intermediate runs without re-sorting ordered input. Select fan-in from actual reservation
costs, not a worker/thread cap. Release intermediate backing after its last borrower; refuse explicitly
if even the minimum conforming merge cannot fit. Resource ownership includes simultaneous retained
input, preparation and output, not only an individual cursor.

Current disk reading decodes to recover/validate keys and the normal cursor decodes again. A private
frame cursor can avoid the latter and repeated re-encoding; removing all key-recovery decoding would
need another validated representation and is not promised. DataFusion 55.1.0/Arrow 59.3.0 remain
appropriate for relational/columnar consumers, but their sorted merge alone does not supply native
conflict/dedup/terminal contracts. BLAKE3 1.8.6 is not an application digest-concatenation operation.

**Acceptance:** fixed literal expected payload order and independently established v4 roots;
normalized IDs that reorder physical input; overlap/equal duplicates/last-run unequal conflict;
permuted/repeated inputs; explicit empties; memory/spill/mixed layouts; bytes/null/signed-zero cases;
input cursor reuse after merge; late read/callback failure, cancellation and reservation refusal.
No failed prefix yields reusable completion. Source establishes removed sorts/encoding, not speed.

## 4. CP02 — dependency-aligned family reuse and one protected preparation

Use the actual compilation operation's preparation owner/shared-future idiom, not a process-global
cache or a second witness mechanism. Reuse complete roots only for identical complete family
inputs under the same protected installation/session/pin/epoch, resource owner and recipe.

| Family | Complete dependencies for reuse |
|---|---|
| Contributions | Full logically normalized completed descriptors: producer/profile/model, outcome/assurance, outputs and explicit empties. |
| Memberships | Exact selected copies' normalized membership runs, including owner, relation, nominal key, backing node and content. Logical contributor ID alone is insufficient. |
| Backing | Exact referenced canonical row set and complete payload bytes under protected copies. |
| Views | Complete validated relation/contributor descriptors and explicit empties. |
| Bindings | Exact validated binding descriptors, roles and referenced views; closure admissibility remains independently enforced. |
| Aliases | Exact selected alias rows/bytes plus successful required composed-set ownership/reachability checks. |

Additions, deletions, absent lookups and changed inventories participate. Changing a binding/alias
need not rebuild unrelated families. Changed/overlapping selected sets still require ordered union
and hashing of resulting rows under flat v4; do not claim all shared-content hashing disappears.
The main saving is fewer demanded complete identities plus no repeated sorting/encoding at those
that remain. Separate established logical bytes from fresh physical acquisition/receiving CAS.

Share full original-blob observation once per protected completed blob and contributor fragments
once per exact protected contribution. Every typed range/layout/source association remains checked.
Contribution preparation waits for physical EOF/drain and final full-row/incarnation checks before
insertion. Failed shared preparation is evicted safely; a cancelled waiter cannot publish stale
success or cancel an independently owned successful peer. Retained payload and map/tree capacity
charges last as long as their actual owners, preserving IF05/IF07/IF10 accounting corrections.

**Acceptance:** contributor additions/removals/set switching; same logical descriptor with changed
selected backing; outcome/assurance changes; independent binding/alias invalidation; zero-length and
shared blobs with different ranges/layouts; off-page corruption and late physical failure; failed
lookup/cancel/retry; wrong session/epoch/resource owner and ABA. Nonempty alias reuse and exact-set
switching need revealing runtime controls rather than being promoted from prior source evidence.

## 5. CP03 — genuine assurance windows and prepared alias topology

For remaining demand, native control/reader owns identity-correlated record/guard/hold windows.
Reuse existing active-guard/owned-key and candidate hydration mechanics; issue actual bounded indexed
set queries and validate locally. Return optional observations with an explicit required-record
adapter: missing optional discovery preserves `None`, whereas missing required row refuses. A
present row with invalid/missing guard is never optional absence. Preserve full expected bytes and
incarnation with independent charged ownership.

For heterogeneous holds, share the deterministic hold-address derivation with production hold
creation, fetch exact requested pair IDs and verify canonical ID plus owner/object/incidence fields.
Do not form a Cartesian owner/object query or hide one query per pair. Reject missing required,
extra, duplicate, foreign, inactive and mismatched results by identity, not counts. Receiving effects
still atomically check the exact premises they consume; bulk read never authorizes an unchecked
later attachment. Keep intentional singleton checks where the operation itself is singleton.

Implement the window/alias primitives and their revealing controls before CP04 consumes them.
Migrate backing/alias audits, remaining completion verification, live attachment and retained-product/
recovery inspections with CP04's reduced-demand route; full consumer migration does not precede
that route as a separate barrier. Keep cancellation/delivery/drainage,
ExpectedRecord reservations, independent local/native outcomes, original request reconciliation and
final CAS. Do not rewrite intent/payload/outcome-reference release, fencing or retry ownership merely
to reduce log/conflict counts. Inspect real query/index/predicate sources if a consequential conflict
question remains; only an authorized tiny case is warranted if source cannot settle it.

Prepare directed alias adjacency once from validated exact owned rows, then traverse a frontier and
visited set. Selection returns reachable nodes/aliases; independent audit also rejects unreachable
owned source aliases. Cache contributor-local integrity/adjacency, but compose reachability over the
selected contributor set; cross-contributor paths can require rows outside emitted output. Preserve
cycles, multiplicity, target identity and exact seed/closure meaning. Include `expand_nodes`' repeated
accumulated-node copying, not just the two full-scan fixpoints.

Use charged compact adjacency for fitting input and existing sorted-run/indexed source-key windows
for larger input, retaining frontier/visited ownership and spill where needed. A new unconditional
whole-heap graph plus ordinary-input refusal is not the target. Native and dump adapters can share
the closure operation without sharing authority or mutable storage. DP-12's existing termination
semantics remain valid; this correction removes repeated work rather than repairing a proved cycle bug.

**Acceptance:** handwritten shuffled requested/result sets, optional absence, required missing record,
missing guard/hold late in a window, foreign/duplicate rows and changed incarnation before CAS;
late transport failure and insufficient reservation. Directed reversed-order chain, cross-contributor
chain, cycle, repeated target, disconnected source, empty seeds, changed contributor set and spill
have independently specified reachable results and audit refusals. Source must show set queries and
frontier traversal, not wrappers around old scalar/scanning loops.

## 6. CP04 — migrate internal, retained, frozen and cold consumers together

CP00's specified contract, CP01/CP02 preparation and CP03's working window/alias primitives
precede their consuming CP04 slices. CP03's remaining consumer migration completes alongside this
package; there is no whole-package CP03↔CP04 cycle. Move internal
`CompletedInputs` provider construction, DataFusion/native table providers and scan/batch factories
to earned operation-local selections. Preserve exact dependency bindings and declaration/coverage
semantics. Stop automatic installation of singleton/cumulative internal views at contribution completion.
Only full independently validated contribution preparation grants the new internal capability.

Migrate `FrozenCompletion.realization_inventory`, `native_completed_view` declarations/writer,
`InstalledViewRef`/inventory and completion factories to compound boundary plus logical selector
representation. Update `NativeCompletedState`, semantic admission, `PublishedCompletedState`, staged
publication bindings, retained compiler-product attachment and recovery factories coherently. Freeze
all identity-bearing mutation surfaces and drain writers before earning the final inventory/identity.
A read admission lease, `wait_scans` or `AuditCapture` alone is not a frozen snapshot.

Fresh-process Loader, installed/published sparse readers, candidate hydration, aliases/originals,
publisher audit and selected backup must all resolve exact selected copies under the new boundary
inventory. Preserve per-selector scope, foreign-member refusal and pin lifetime through delivery and
continuations. Backup retains its transaction-bound marker/inventory/incarnation recheck and transitive
binding/input closure. Detached import/restore and cold recovery re-earn authority from actual state;
update their physical codec and independent final identity routes with CU01/CU03 before deleting the
replaced physical format. OP02 regenerates and checks changed schema assets in this package.

Remove replaced per-view member duplication, installation/seal/completion/hold paths, stale SQL/view
fields, legacy readers and old-format fixtures after actual consumers move. Keep only materializations
whose owner/lifetime or access benefit is identified. Hard regeneration replaces historical data;
no dual writes, rollback assets or second runtime comparison cache. Existing global protocol
coordination is not relaxed merely because fewer effects are required.

**Completion evidence:** source-backed complete consumer migration and deleted-object/effect inventory;
focused internal complete-empty/mixed-selection/cross-alias controls; exact physical authority loss,
incarnation replacement, changed member/off-page backing, cancellation and failed finality refusal;
fresh-owner retained/publication/selected access and independent cold/import identity controls.
Then CE01 runs the actual generated Catalog/admission/publication/native-query journey plus facts
availability. This package cannot claim those unreached stages passed from earlier fragment counts
or a compile-only check. CE02 owns actual regeneration under live-owner drainage, and CE03 owns
older-plan reconciliation; neither is completed by this document.
