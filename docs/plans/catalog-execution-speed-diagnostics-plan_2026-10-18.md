# Catalog execution speed: truthful optional diagnostics

**Approved plan direction / Proposed implementation, 2026-10-18.** Companion to the
[coordinator](catalog-execution-speed-plan_2026-10-18.md). Its
[§6 checkpoint](catalog-execution-speed-plan_2026-10-18.md#6-sole-disposition-and-current-checkpoint)
is the sole owner of current disposition. This document plans implementation; no diagnostic
capability or performance improvement becomes Tested by appearing here.

## 1. Responsibility, baseline and source obligations

The [comparative review](../design_review/reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md#F12)
identifies F12, missing current successful-write observations and discarded terminal timing, and
[F13](../design_review/reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md#F13),
avoidable disabled diagnostic work and repeated parsing. This companion also carries the diagnostic
parts of I02, actual conflict/blocking identification, and I05, sufficient diagnostic depth. It
implements the qualified dispositions in
[D01–D26](../design_review/evidence/2026-10-18_catalog-execution-speed-comparative/recommendations.md#shared-diagnostics-measurement-and-tooling-recommendations),
including omissions and conditional investigations; it does not adopt the external tooling stack
as an indivisible requirement.

Source baseline is `9c457a0c`. The existing `lctx-native::phase` protocol emits begin/terminal
**events**, not duration spans. Dropping a phase correctly leaves its begin incomplete. Native
worker contexts preserve the submitting dispatcher through polling and destruction. Keep that
ownership. The JSON evidence adapter already preserves ordered structured span context; it currently
recognizes phase/resource observations without a sufficient operation-page/read terminal vocabulary.
Successful operation pages are distinct from legacy effects, so legacy timing buckets are not a
total of current native work. The SDK converts absent/invalid statement durations to zero, losing
presence before readers discard useful statistics. `Phase::begin` currently allocates an ID,
captures time and captures context before it checks interest.

The resulting capability observes existing execution under existing run/evidence/storage owners.
It records enough identity, work and terminal information to answer concrete performance questions,
and optionally projects that information into a timeline and compact report. It is not a scheduler,
recovery owner, semantic validator, mandatory collector, universal registry or benchmark framework.
Observer failure never manufactures application success, remote drainage or cleanup finality.

RC09 was approved by the operator on **2026-10-18**. SP00 must route the changed observation and
statistic contract through the existing ADR/design owners before dependent production changes.
Existing raw evidence remains authoritative; consumers migrate together with the typed extension.
Other approved rule changes do not make diagnostic events into evidence of their implementation.

## 2. Shared observation and execution contracts

Record immutable run/build/configuration context once through existing owners. Reference it from
observations rather than repeating expensive hashes, environment discovery or full source context
at every event. Retain the available source-to-binary limits; a run label does not establish exact
binary provenance. Within a process, use a monotonic clock and stable invocation/process/activity
identity. Cross-process comparisons need declared clock alignment and uncertainty; timestamps do
not establish causality.

The vocabulary distinguishes phase events, native operation/page/query/statement observations,
retry/reconciliation observations, work/resource counters and explicit schedule/handoff edges.
Each form carries only qualifiers relevant to its consumer. Operation identity includes actual
root and page ordinal where available; query submissions and retry attempts are separate from
logical operations. Ownership parentage describes containment. Only an actual prerequisite,
acquire/release, ready/start or handoff observation establishes a dependency/resource edge.

Preserve timing **before** the SDK's lossy duration conversion. Distinguish received-valid zero,
received-valid positive, absent and invalid duration. Preserve available original timing and
validity information without inventing a duration for an invalid value. State the scope of each
duration: client preparation, issued-query lifetime, statement execution, stream consumption,
reconciliation or enclosing command. Client-minus-server elapsed is an unexplained remainder;
it is not network, queue, commit or serialization time. Do not clamp mismatched arithmetic into
a claimed partition. Accumulated nested/parallel elapsed is not additive wall time.

Separate ready, prepared, dispatched, started, kernel-finished, required-output-visible,
accepted-children-drained and command-finished milestones. A returned generic leaf does not
establish semantic success. Preserve application Unknown and actual query terminal knowledge
independently of missing, dropped, truncated or failed diagnostic capture. Observer disposal is
not a cancellation/drain acknowledgement.

Aggregate useful counters at module, window, query or operation grain. Distinguish logical rows,
physical records, bytes, statements, submissions, acknowledged pages, retry attempts, cache hits
and examined candidates. Resource reservation/charge is not RSS. Do not add one event, atomic,
allocation or syscall per fact to measure a loop that can report one aggregate.

## 3. Work packages and prerequisite slices

### AD00 — Minimal typed observation, identity and timing-presence contract

**Consumes:** SP00's approved RC09 decision route and the existing phase/context/evidence owners.
It does not wait for the private compiler or native installation redesign. Define only the
vocabulary required by AD01/AD02 and its transport through the existing capture/evidence interfaces.

**Result and consumers:** native producers emit typed forms with stable activity/operation identity,
optional timing presence and honest terminals; evidence consumers preserve those forms and raw
locators. JSON remains structured. Flat text remains a qualified fallback: conflicting identity
or semantic fields produce an unattributed observation rather than guessed parentage. New event
forms must not be silently treated as phases or test-case outcomes. Retain source offsets and
capture-loss limits so a report can link back to the actual observation.

Specify the SDK-facing presence representation and its propagation across ordinary and admitted
query paths before changing either producer or consumer. Prefer a narrow additive change in the
existing vendored transport/terminal contract; do not introduce a second query API solely to obtain
statistics. Inventory in-tree consumers and migrate them together. No legacy database/data reader
is needed for this diagnostic-only contract; retained original evidence still reads as its original
qualified form rather than acquiring invented new fields.

**Focused acceptance:** received zero versus absent/invalid duration; duplicate/conflicting flat
fields; structured nested context; unrelated concurrent invocations; process identity ambiguity;
partial lines and truncated capture. The adapter must preserve unknowns and locators. Application
results and native lifecycle obligations must be identical with capture enabled and disabled.

### AD01 — Current native successes, retries, query terminals and evidence integration

**Consumes:** AD00's implemented producer/adapter slice. Existing actual operation/query owners
permit independent implementation before the new native packet protocol. Later native packages
must migrate these hooks with their real effect boundary instead of retaining obsolete page wrappers.

**Result and consumers:** observe successful and failed actual operation-page application as well
as legacy effects, including readiness/preparation, submission, actual terminal, reconciliation and
acknowledgement where those are distinct. Carry actual root/page/query/statement identity through
retry observations. Recognize `native_operation_page` as well as legacy `native_effect` contexts;
neither event family alone is total native execution. Preserve eager and streaming statement
statistics without requiring a second read or an `EXPLAIN ANALYZE` execution.

The native owner decides committed/refused/Unknown; diagnostics reports that decision and the
observed stages without independently interpreting SQL/error strings into a lifecycle verdict.
The evidence adapter exposes actual read observations as reads, not phases. Repeated attempts do
not become repeated committed operations, and an observer terminal does not settle a pending
native recipient. Remove discarded-statistic plumbing and misleading legacy-only aggregation as
their consumers move to the complete qualified forms.

**Focused acceptance:** genuine successful page; retry followed by commit; terminal refusal;
unknown response and reconciliation; empty and multi-statement read; partial/cancelled stream;
missing/invalid/zero statistics; ordinary and admitted query routes. Use existing fixture/native
owners and minimal revealing cases, preserving exact finality separately from diagnostic capture.

### AD02 — Actual scheduling, preparation, resources and useful aggregate work

**Consumes:** AD00 and the actual boundary contracts of the compiler/native/analysis/extraction
packages being instrumented. Existing shared-boundary improvements can start independently; final
integration follows the new private-source, set-shaped visibility and native packet consumers.
An agreed interface is insufficient where the required execution path has not been implemented.

**Result and consumers:** expose consequential ready/prepared/queued/start/output/children/command
transitions, per-key preparation ownership, actual gate/permit waits and handoffs. Record existing
schedule edges rather than reconstructing a dependency DAG from span nesting. Multiple holders,
unobserved waits and cycles remain explicit limitations; a holder's lifetime is not automatically
the duration it blocked a particular waiter. Use aggregate work to distinguish packet fragmentation,
amplification and changed scope. Native conflict categories come from actual observed operation/key
owners; query digests alone cannot prove conflicting keys or livelock.

Observe acquisition/provider/export/ingress, normalization, selected computation, admission,
publication and finalization at their useful shared boundaries. Separate analyzer processing from
rich-fact export and ingress backpressure. Observe disabled-owner/preparation decisions without
running the disabled work to collect its statistics. Emit analytic method/strategy identifiers
only when the corresponding selected operation actually runs.

For DataFusion, retain and execute one actual physical plan, then inspect that plan's metrics,
including partial/cancelled outcomes. A logical clone is neither a shared scan nor an executed-plan
measurement. Use existing metrics forwarding; opt into `datafusion-tracing` only if it fills an
actual gap after checking exact composition. Pyrefly's existing callbacks/counters are the first
route. Add a read-only snapshot/delta only for a demonstrated consumer; keep detailed demand
collection opt-in and callbacks quick. Accumulated worker elapsed is not exclusive CPU or wall.
Its CLI individual-module timing mode is not an equivalent normal-checking/export baseline.

**Focused acceptance:** same versus distinct preparation keys; pending wait versus ready fast
path; producer completion versus accepted-child drainage; overlapping activities; cancellation
and resource refusal; one actual executed DataFusion plan; optional method disabled. Assert known
edges/counters using independently specified scenarios, not a report generated from its own totals.

### AD03 — Optional Perfetto projection and compact factual reports

**Consumes:** AD00/AD01; use AD02 observations as each actual consumer becomes available. The
exporter must also represent incomplete older/minimal captures truthfully. Existing managed-run
and evidence interfaces supply invocation selection, raw capture, locators and storage lifetime.

**Result and consumers:** opt-in export creates timeline slices, counters and explicit flows using
existing Perfetto import/view/query facilities. Pair Phase begin/terminal **events** explicitly;
adding a span subscriber alone is insufficient. Use separate logical lanes or async tracks for
overlap; ordinary B/E/X slices must obey their nesting contract. Retain unmatched begins, missing
terminals, ambiguous identity and loss as incomplete information. Do not synthesize success at
process exit or merge activities merely because names/clock intervals match.

Provide compact overlap-aware elapsed, work-amplification, retry, top-activity and comparable-run
reports through existing evidence/report routes. Report which interval/identity/dependency coverage
supports each result. A critical path can be claimed only for the recorded dependency graph, with
unobserved edges stated; otherwise report observed ordering/overlap without a dominant-cause claim.
Do not sum nested intervals, infer exclusive CPU from async entered time or report p95/p99 from a
few examples. Preserve raw captures and derived output as separate existing storage components.

**Focused acceptance:** reconverging and overlapping activities; task migration; unmatched events;
ambiguous duplicate IDs; clock mismatch; capture interruption; derived-output write refusal. Source
locators must recover the raw observations. Diagnostic export failure reports failure for that
artifact without changing the captured application's exit or native cleanup status. No mandatory
viewer, collector service, bespoke graph analyzer or new command family is introduced.

### AD04 — Dispatcher-correct disabled path and bounded diagnostic-depth decisions

**Consumes:** the existing captured-context contract; AD00 where new observation forms are used.
The disabled Phase correction can proceed independently of AD01–AD03. It must check interest in
the actual originating dispatcher before expensive observation state is created, then preserve that
dispatcher through async/thread handoff and destruction. An inactive representation avoids unused
ID/clock/context work; it must not consult an unrelated task's subscriber or claim exactly zero cost.

**Result and consumers:** enabled capture retains actual terminals and interruptions; disabled
capture removes avoidable work. Prefer structured events and bounded aggregation over repeated text
parsing. Any optional recorder uses the existing run/storage owner through writer drainage. Define
partial writes, capture failure/loss markers, flush/drop and abrupt termination honestly. Direct
`O_APPEND` is a candidate, not proof of crash/disk durability or low overhead; avoid one write per row.

Resolve I02/I05 in this order: inspect actual successful query/page stats and handoffs; identify the
specific remaining question; choose only the extra observation capable of deciding it. CPU/RSS/I/O
sampling may cover both application and actual server when implicated. Existing `compile-profile`
profiles Rust **compilation**, not catalog runtime; reuse ownership patterns without relabelling it.
Optional symbol/frame-pointer builds preserve intended optimization. On-CPU sampling does not
explain blocked time; Tokio console/runtime metrics apply only to a demonstrated scheduler question.

SDK traceparent work, when justified, covers ordinary **and** admitted gRPC routes. Optional OTLP,
slow logging or server metrics require an explicitly owned diagnostic target or authorized
maintenance, compatible selected features and defined query/parameter exposure. Historical
memory-engine correlation does not qualify the present RocksDB path. Pick thresholds/cadence for
the question; scraping faster than the server refreshes is not fresh data. Do not install a collector,
enable privileged metrics or impose runtime overhead by default.

Fixed-edge what-if scaling remains **deferred**: reopen only when a named decision cannot be
settled by simpler observations, with fixed-scheduling assumptions and no causal speed forecast.
Equivalent cold/warm diagnostic overhead and benefit measurements belong to SP95, after functional
implementation, using actual input/profile/frontier/executable/schema/cache dimensions and complete
milestones. No forced repetition count, baseline campaign or new harness gates source-clear repairs.

**Focused acceptance:** scoped/global subscribers independently enabled/disabled, cross-thread and
async context, dropped future, graceful cancellation, panic, abrupt writer termination and write
failure. Assert truthful loss/incomplete markers. Optional hooks earn acceptance only when selected;
an explicit evidence-backed decision not to add a hook satisfies its investigation, not its runtime
implementation. The rejected 30-second, 95%-attribution and 3%-overhead gates never become tests.

## 4. Integration, retirement and closure

The diagnostic crosswalk has the following explicit execution routes; its detailed qualifications
remain part of each package's preservation contract.

| Source recommendations | Route and retained disposition |
|---|---|
| D01–D03 | AD00/AD01: actual current successes, correlation and present-before-default statistics; correct the claim that the SDK always supplies zero. |
| D04–D09 | AD00/AD02/AD04: qualified identities, actual handoffs and aggregate work; Phase events remain events, entered/poll elapsed is not CPU, ownership is not causality. |
| D10–D11 | AD03/AD04: opt-in existing timeline facilities, honest recorder failure/loss; direct append remains a selected mechanism, not promised durability. |
| D12–D14 | AD01 first; AD04 conditional server correlation/configuration; preserve optional statistics and unexplained remainder, omit fixed server defaults. |
| D15–D18 | AD02 actual Pyrefly/DataFusion observations; read-only accessor and tracing bridge only for a concrete gap; no CLI-equivalent or logical-clone claim. |
| D19–D21 | AD04 conditional runtime/process/scheduler evidence, with actual application/server ownership and unchanged optimization; existing compiler profiling is distinct. |
| D22–D23 | AD03 compact factual reporting; counterfactual simulator deferred under AD04's explicit decision trigger. |
| D24–D25 | SP95 later equivalent cache/overhead/benefit investigation; AD04 specifies selection and semantic limits, without a mandatory new harness. |
| D26 | AD04 dispatcher-correct disabled work reduction; numerical gates omitted, source-clear corrections proceed independently. |

AD00–AD04 deliver useful slices without making a telemetry platform a prerequisite for compiler,
native or algorithmic corrections. Final integration migrates real new execution owners and retires
observations of removed protocols. Add no compatibility execution path solely to keep an old report
bucket populated. Raw historical evidence retains its original semantics and explicit coverage limits.

F12 closes only when actual current successes, terminals/statistic presence and correlation survive
the production/evidence path. F13 closes only when dispatcher-correct disabled behavior, proportional
capture and honest incomplete/failure handling are established. I02 may retain an unmeasured
historical blocking question without blocking a source-established conflict correction; record the
remaining consequence at the coordinator. I05 closes with a sufficient implemented route and explicit
conditional decisions, not by implementing every proposed external tool.

Use compile checks and focused affected Rust/Python controls during implementation, then applicable
leaves at functional completion under the coordinator. Full-library/benefit measurement belongs to
SP95; assembled acceptance belongs to its scheduled coordinator endpoint. All outcomes state their
actual scope. Plan-authoring checks establish document consistency only, not runtime qualification.
