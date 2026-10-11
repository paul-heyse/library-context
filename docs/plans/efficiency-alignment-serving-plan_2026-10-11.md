# Efficiency alignment: serving, selection and transport

**Proposed, 2026-10-11.** Companion to the new
[coordinator](efficiency-alignment-plan_2026-10-11.md), which owns baseline, rule decisions,
progress, finding disposition and combined completion. Sources: original O-F35–O-F40/O-F42/O-F44;
audit A-F08–A-F11. No existing plan was read or superseded. Product implementation remains Planned. The 2026-10-11 design investigation is retained in
[development/serving decisions](../design_review/evidence/2026-10-11_efficiency-design-decisions/development-serving.md);
pure probe passes and source inspection establish no product closure or measured improvement.

## 1. Target and foundations

Serve an exact frozen ranking policy over an eligible immutable realization. Prepare executable
constants for an executable epoch, native eligibility for the protected context it actually validates,
and compact domain information for its reader pin. Hydrate only the requested page. Keep uncertainty,
negative witnesses, omitted results, deterministic ties and exact final response-byte admission visible.
Separate embedding I/O from CPU/native demand while preserving ownership until actual work drains.

Current exact serving, bounded prepared-cache ownership, selected native access and full-value reference
controls are useful foundations. Concurrent source adds current-context eligibility, first-binding and
support/anchor checks, and charged root batching. Retain those protections: an old positive node check
is not a current-context witness. The original G1 membership/alias allegation remains unproven; optimize
through the native semantic owner rather than treating that allegation as an established defect.

The workload includes many small pages from one pin, sparse selected domains inside larger shared
cohorts, changing pins/definitions, and slow remote embeddings alongside non-search requests. Caches
must follow those lifetimes. A global warm flag or an early rank cap can make a cheap answer incorrect.

## 2. Packages

<a id="S01"></a>
### S01 — Align exact ranking, references and physical index maintenance

**Sources:** O-F37/RC10; A-F08. Exact frozen ranking is the approved product policy. Align the product
owner, serving design and runbook, recording a superseding ADR where an accepted decision changes.
Actual independent ANN consumers exist: `scripts/programmatic_native.py::native_numeric_reference`,
its script tests, and the CLI/evaluator/serving verification route. Migrate those consumers before
removing unused HNSW maintenance and its schema/index definitions.

Keep full 4096-value canonical vectors, independent F64-scalar-to-F32-projection checks, exhaustive
small-cohort reference scoring (currently at most 256) and private-worker full-value reference checks.
Add comparison to the actual served candidate output, with independent expected eligibility/order.
Production exact SQL cannot be its own oracle. Migrate report vocabulary and controls that currently
require `native_ann` together with the reference implementation.

Then retire unused ANN DDL from materialization, update realization/index identity and restore controls.
Do not remove model/provider analytics neighbor APIs: their exact method contracts are separate. A
scoped consumer search supports this migration; it is not proof that every external consumer is absent.

**Closure:** ties, F32 projection, missing/corrupt values, eligibility and served ordering agree with
independent finite reference results; no required consumer calls removed ANN indexes; restore builds
the exact selected realization. No recall/latency improvement claim follows from functional equivalence.

<a id="S02"></a>
### S02 — Prepare process constants once per immutable executable epoch

**Sources:** O-F35; A-F08. Compute executable/model/wire identities and derived operation/cursor keys
once for their immutable executable epoch. Pass owned prepared values through query preparation.
Keep native first-use installed-definition/epoch mismatch refusal; a process-local cached identity
cannot attest to mutable installed service state. A new executable or semantic definition starts a new
scope. Keep the per-call query-vector input-hash check: request bytes are not an executable constant
or a pin-owned fact. This small slice can precede pin/domain preparation.

**Closure:** repeated requests reuse constants; changed definitions refuse or select the new epoch;
operation and continuation identities retain all semantic inputs. No mutable-path later digest may
replace the completion-time evidence described in D05.

<a id="S03"></a>
### S03 — Prepare domains and validated eligibility at their actual lifetimes

**Sources:** O-F35/O-F38/RC06; A-F08. Extend the charged `PreparedCache` with compact roots/domain,
selection and classification/policy preparation. Key by pin/epoch, operation, relation dependencies
and the current-context/absence facts each result consumes. Carry a validated witness, not an unqualified
positive membership boolean. Shared cache state remains charged, evictable and owned through readers.

Use native selected membership/alias/nominal-conflict meaning. Preserve current first-binding,
support-link, map and anchor changes that invalidate previously valid contexts. Missing and extra
rows or nominal conflicts must still be detected at the appropriate protected boundary. Eviction
causes recomputation, not semantic degradation. N08 supplies explicit derivation eligibility for
retained manifests; no retained shortcut precedes it.

**Decision selected, Interface-checked 2026-10-11:** retain the current native selection query
owner. `selection::sparse_membership` nominates physical IDs, uses indexed reverse aliases and selected
view membership, then validates nominal conflicts; relation nomination starts from selected membership.
The inspected route is already batched/selective, so no query rewrite or native EXPLAIN claim follows.
Extend the charged `PreparedCache` at its exact pin/epoch with domain/index preparation. Current-context
eligibility still consumes primary binding absence and complete companion/support/map/anchor facts from
`derived_search::eligible_context_rows`; a positive membership boolean cannot substitute for them.
No native query experiment is needed for this retain decision. Cold/warm invalidation controls remain
functional implementation work, including N08 eligibility before retained-manifest shortcuts.

**Closure:** new binding, support, alias/conflict, definition and pin changes invalidate exactly the
needed preparation; cold and warm calls have equal refusals; high-degree preparation charges/drains;
a released pin cannot lend live authority to another request.

<a id="S04"></a>
### S04 — Page from compact ordered state and hydrate the page once

**Sources:** O-F35; A-F08. Consume S03's validated domain and ordered compact IDs. Continuations use
that exact context; fetch rich payload only for the page the caller requests. Avoid rediscovering or
hydrating the whole domain on every page. Prepare global validation separately where current semantics
require failure for corrupt off-page data; late hydration must not silently hide that failure.

Preserve omission/availability sections, cursor identity and lifetime, deterministic page order, nested
scenario associations and exact final-byte admission. A bounded page is not permission to truncate
semantic decisions upstream. Distinguish evictable preparatory indexes from retained ranked results:
a fresh request can rebuild preparation, but an expired, foreign-session or evicted ranked-result
continuation must refuse without rediscovery. Keep its frozen entry as the authority; do not rebuild
it from current source even under an apparently equal pin. Preserve the existing
`ranked_results::expiry_foreign_session_and_eviction_refuse_without_rediscovery` contract.

**Closure:** concatenated pages equal the intended ordered result; off-page corruption keeps the defined
failure; nested records remain associated; repeated and stale cursors have truthful outcomes; a tiny
page avoids full rich-payload hydration without inventing incomplete coverage.

<a id="S05"></a>
### S05 — Select and score through exact, bounded ranking kernels

**Sources:** O-F36/O-F38/RC11; A-F08. Index document tiers and use bounded exact selection/top-k kernels
instead of full repeated sorts or linear victim selection. Default to batched exact scoring of selected
eligible targets. Eligibility precedes admission, caps and tier counting. Preserve distinct-target
counting, exact branches, unit/scope/context meaning, ties and full winner rescoring.

A cohort-first lazy scorer was a bounded alternative, rejected by the decision below. Preserve both
result and failure semantics without scanning a huge shared cohort for a tiny pin. Close only the
indexing/batching obligations actually delivered. No approximate shortlist or pre-eligibility cap is
an acceptable replacement.

Lexical serving uses exact-view/family term statistics and FULLTEXT/equality paths; do not transplant a
vector scoring reorder into lexical retrieval. A new ranking policy would need an explicit product
decision beyond this package. S01 supplies one authoritative exact policy and independent controls.

**Decision selected, Interface-checked 2026-10-11:** selected-first exact batching is final for this
increment; reject optional cohort-first lazy scoring. It adds shared-cohort work for sparse pins and
cannot skip malformed/nonfinite losing selected candidates while preserving present refusal semantics.
Index document tiers and use bounded exact top-k/selection with eligibility before caps and distinct-target
counting; preserve full winner rescoring, ties and branch policy. Finite functional controls must compare
failure as well as result equivalence, including corrupt losers. No optional scorer campaign or speed
claim is needed to reject the optional route.

**Closure:** sparse/dense domains, equal scores, repeated targets, ineligible high scorers, corrupt
losers and exact branch priorities preserve output and failure semantics; retained state is charged.
Native query shapes and finite reference results establish implementation, not measured speed.

<a id="S06"></a>
### S06 — Sort small inputs in memory and spill under owned reservations

**Sources:** O-F40; A-F11. Delay temporary-directory/run creation until needed; keep a fitting tail in
memory. Size runs from actual available reservation and use bounded fan-in merge. Keep file/descriptor
and memory charges through merge, cancellation and cleanup, with poison/error state preserved.

The existing sorter's repair is the default. Compare a composed DataFusion sort plus adjacent
conflict validation before a broader replacement, using the pinned capability contracts. Library sort
alone does not establish equal-key/equal-bytes acceptance and equal-key/different-bytes refusal.
Do not add a second generic external-sort framework.

Migrate all shared sorter consumers, including compiler, reconciliation and backup. Local improvements
are not evidence that per-process reservation budgets bound aggregate host memory; RC12 remains unchanged.

**Decision selected, Interface-checked 2026-10-11:** repair the shared sorter using explicit
memory/disk backing: retain fitting sorted/deduplicated rows under reservation, lazily allocate scratch
at first spill, and merge a memory tail through bounded existing carry/fan-in mechanics. Migrate replay/
point-index/cursor consumers and keep poison/full-value collision checks and all reservations until final
cleanup. Existing code eagerly allocates TempDir and flushes fitting tails at finish, which is the bounded
defect. A credible replacement would compose public DataFusion `SortExec`/DataFrame sort, globally ordered
output and an adjacent full canonical-value conflict validator; `ExternalSorter` is private at 55.1.0.
It additionally needs budget/disk-lifetime and Arrow/async adapters for current synchronous/replayable
consumers. Reject that broader replacement here; hash equality or DISTINCT does not satisfy conflict
semantics. DataFusion spill is IPC, independent of Parquet.

**Closure:** empty/small cases create no spill artifacts; oversized cases spill/merge under reservations;
late conflicting duplicates refuse; cancellation and I/O failure drain and retain truthful cleanup;
all consumers retain ordering and collision semantics.

<a id="S07"></a>
### S07 — Separate embedding wait from native demand, share the HTTP lifetime

**Sources:** O-F39; A-F09; original §7.10 HTTP observation. Both tools and capability resources must
enter `NativeExecutor` through the same ownership policy. Give embedding preparation its bounded
lifetime, then acquire Rust-owned CPU/query admission when native work can execute. Preserve the
Python pre-native checks and final JSON-RPC byte boundary (`BoundedStdioWriter`), not just inner
result size. Pending demand must not silently become unbounded task or payload retention.

Use a lifespan-owned HTTPX2 client, closed after accepted requests and workers drain. Validate exact
locked APIs before implementation; current source inspection and documentation support client reuse.
A cancelled coroutine does not imply the detached worker or remote native operation has stopped.
Close fences new work, cancels accepted operations, drains owned workers, and then closes sessions/client.

**Closure:** two blocked embedding calls do not exhaust all native slots for unrelated non-search and
resource requests; cancellation retains ownership until completion; close/admission races settle once;
final oversized envelopes still refuse. Functional fake delays expose scheduling; actual transport
controls establish session/drain behavior. No new cross-client daemon or global scheduler is required.

<a id="S08"></a>
### S08 — Construct the tokenizer from the selected verified bytes

**Sources:** O-F44; A-F10. Use verified buffers for tokenizer/config construction through pinned
byte-construction APIs. Preserve selected asset manifest, preprocessing, templates, special tokens
and recorded identity. Reopening the selected path is not equivalent to consuming its verified bytes.
The audit classifies this as a narrow validity issue (G3), not a blanket product failure.

**Closure:** replacing the file between verification and construction either leaves the verified
in-memory construction unchanged or causes an explicit mismatch refusal; it cannot alter behavior
under the old identity. Normal selected assets retain tokenization semantics.

<a id="S09"></a>
### S09 — Bridge CLI signals to owned cancellation and establish drain limits

**Sources:** O-F42; A-F09. Connect CLI external cancellation to the existing cancellation/owned-drain
path: stop new effects, drain provider/native/transport work, abandon only where allowed, and report
committed, not-committed and unknown outcomes truthfully. Signal receipt or child exit is not proof
of transaction finality. D10 separately integrates direct nextest processes; a CLI handler cannot
cover them.

**PD02 resolved as a source/native defect, 2026-10-11:** the exact server revision
`238bfeb11f5725bebed370167656748df8067595` has bounded channels and a deadline watchdog, but its
framing state owns the running query future without a drop-time drain transfer. The scoped source
trace and coordinator's dedicated engine campaign establish the missing owner on raw response-stream
abandonment. Preserve recoverable request identity and unknown disposition until the selected owner
fix is implemented and qualified; installed patched definitions remain the authority. Request deadlines,
shared-service state and remote holds do not change because a Python/CLI task disappeared.

**Decision selected, Interface-checked 2026-10-11:** source inspection establishes that
`QueryFraming.run: Option<QueryStreamRun>` is the sole server execution-future owner; `ReleaseOnDrop`
trips cancellation and cleans session/live-query state but never polls/transfers the run. The timeout
watchdog also does not drive a dropped future. The exact core construction invokes the executor inline;
WebSocket teardown explicitly drains handlers to avoid the same open-transaction drop. This is an owned
execution gap on raw gRPC abandonment, not proof that every current `NativeRows` operation leaks.

Select the smallest server owner transfer as the target: on framing Drop cancel first, close item
delivery, take/register the remaining run in a server-owned drain task before returning, preserve request/
transaction/session authority until run finalization, reconcile late live-query results, then release
ephemeral session authority exactly once. Retain drain tasks through shutdown; an untracked spawn or
abort-on-timeout is insufficient. No main-tree vendor patch is authorized by this design investigation.
The coordinator's dedicated campaign owns exact disconnect/normal-control evidence and executable identity.
On 2026-10-11 its exact patched server emitted an open-transaction-drop error after raw tonic Begin plus
250 ms disconnect inside a transaction; normal EOF committed, and autocommit disconnect preserved prior
committed effects. Client exit 0, later health and confirmed server shutdown do not qualify the missing
execution owner. See the linked decision evidence and `raw/grpc-01-*` receipts. The target owner transfer
is required; interim unknown disposition and a defect-reproduction probe do not close S09/D10.

Current lost-connection/terminal-proof disposition is recovery-owned unknown. Preserve operation/attempt
identity and effect/reader holds until existing journal/control reconciliation proves finality; disconnect
is not an acknowledgement and no-write observation is not read drainage. Keep `Completion::RemoteState::Unknown`
and the stronger `NativeRows` explicit statement/outer-transport drain and pin protection. S09 bridges
SIGINT/SIGTERM at `lctx::main` into shared cancellation/admission fences and awaits owned work rather than
dropping the command future. D10 independently binds the same primitives inside actual test processes.
Use actual `Workspace::cancellation`/`drain_report`, `NativeCompilerStore::drain_report` and reader
`NativeRows::drain_transport`, composing their typed `Completion` reports before permitted fixture
release. The server handoff owns request identity/run/release state under the existing gRPC lifetime;
lost transport does not convert that internal terminal result to a client acknowledgement.
Future functional controls cover pre-execution, in-transaction, blocked channel/no-output, committed
before disconnect, deadlines, shutdown and lost acknowledgement with separate local/remote outcomes.

**Closure:** cancel during capture/provider send/native effect/commit acknowledgement/reader work;
accepted work drains or remains explicitly recovery-owned, with no false rollback claim; committed
results remain committed; process and remote terminal state are distinguished. No hard-kill cleanup
guarantee is invented where the execution environment cannot provide one.

## 3. Integration and migration

S01 precedes a changed ranking implementation and ANN retirement. S03 precedes S04 and consumes N08
for retained-manifest shortcuts. S02/S08 and the bounded sorter work can proceed independently.
S07/S09 share transport ownership; D10 consumes their common cancellation primitives while owning a
separate direct-test entrypoint. N04/N06/N11 witnesses can remove repeated validation only when their
complete physical validity scope is present; ordinary and cold checking remain distinct.

A new operation uses existing pin/epoch preparation and final-byte admission. A new derivation creates
its own realization rather than mutating a live reader's state. A new remote embedding backend conforms
to the same demand/drain ownership without acquiring native CPU capacity while waiting for a network
result. These boundaries localize extension without multiplying services or policy registries.

Functional exit covers exact results/refusals, continuations, lifetimes, native selected access and
transport cancellation with affected real components. Shared-service maintenance, operator activation,
real-library campaigns and performance experiments are not implicit authorizations in this document.
