# Catalog execution speed: extraction and capture integrity

**Proposed implementation plan · 2026-10-18 · baseline `9c457a0c`.** This companion develops
provider-side F07 and capture-integrity F10 from the [comparative review](../design_review/reviews/design_review_catalog-execution-speed-comparative-target_2026-10-18.md).
The [execution-speed coordinator](catalog-execution-speed-plan_2026-10-18.md) owns the combined
target and its [§6 checkpoint](catalog-execution-speed-plan_2026-10-18.md#6-sole-disposition-and-current-checkpoint)
is the sole disposition/evidence owner. All work below is Proposed; inspection of current code
is not implementation or functional acceptance of these replacements.

## 1. Purpose and existing foundation

Improve the extraction of the same supported attributed facts, rather than replacing rich export
with ordinary type checking. The [acquisition/extraction owner](../design/sections/acquisition-and-extraction.md)
and [semantic owner](../design/sections/semantic-model.md) govern source identity, provider roles,
coverage, fidelity, original evidence and source correspondence. Core 3.3, efficiency heuristics
1.0 and code-intelligence profile 1.5 apply, especially H6/9/12/18–22/24/26: remove repeated
correspondence and unchanged integrity work before adding execution concurrency.

The native Pyrefly integration requests `Require::Everything`, uses the library's available-CPU
analysis pool, exports rich root records and promotes supporting dependencies. Export can trigger
lazy solving and import promotion after the initial wave. Normal CLI Errors/Exports checking is
not equivalent; CLI report-timings also changes module execution. Ruff remains canonical syntax,
Pyrefly retains its attributed structural/protocol observations, and ty retains its explicitly
selected flow view. Their embedded parsers and independent revisions are not interchangeable.

Inspected production seams at the baseline:

- `cpg-extract/src/pyrefly_stage.rs::run_wave` joins a native analysis wave before serial root
  export/lazy queries; promotion and coverage are part of the same real extraction contract.
- `protocol_records.rs` gathers canonical call/context sites, then requests native observations
  per query. The pinned fork's corresponding APIs search module bindings repeatedly.
- `type_records.rs::term` increments work on every encounter; depth greater than 32 or work
  greater than 100,000 produces explicit Truncated terms with BudgetReached. Deduplication
  after construction does not make encounter accounting irrelevant.
- `bundle.rs::finish` verifies declared outputs, checks cancellation, calls `verify_closed`
  after most providers, then finishes writers. `capture.rs::verify` walks membership and
  rereads each captured artifact, so full integrity verification scales with provider boundaries.
- Writers and provider completion already retain real sink/backpressure/failure ownership;
  do not replace them with detached tasks or infer provider completion from queued bytes.

The [crosswalk](../design_review/evidence/2026-10-18_catalog-execution-speed-comparative/recommendations.md)
E1-01, E1-08, E1-15–18, E2-03, E2-17–19 and D15/D16 supply the relevant external observations
and corrections. Unsupported claims that analyzer work is always negligible or a diagnostic CLI
is a throughput oracle are not requirements. No speed ratio is established by this document.

## 2. Contracts and selected direction

### Exact observations and prepared correspondence

Prepare module-local native binding correspondence once under the same live transaction/solver
context that owns the observations. Select a native bulk observation API where it avoids repeated
binding search without duplicating solver policy. Otherwise add an exact native index used by the
existing per-site observation operations. AX00 settles this narrow interface choice against the
pinned fork before AX01 changes it; no general analyzer abstraction or replacement engine is needed.

Keys include the actual module/transaction identity and native range/site kind and any async or
observation distinction required by the native API. Preserve first-match/ambiguity behavior,
canonical attachment, call/with ordering, statuses, diagnostics and typed fidelity. Native absence
or unavailable attachment remains unknown/unavailable according to the model, never an invented
negative edge or a DisplayOnly observation relabelled NativeStructural.

Writer selection follows typed record declarations. Prepare the record-to-writer correspondence
once rather than repeatedly searching a name list during hot emission. Keep exact role, table,
contract and empty declaration checks; a direct dispatch slot is an implementation mechanism,
not independent permission to emit a record into another provider's output.

### Independent work and bounded lifetime

Use existing owned work and the native library's legitimate sharing boundaries for independent
module/root preparation. The initial analysis pool is already parallel; adding another generic
pool is not the change. If the pinned transaction cannot safely support concurrent lazy exports,
prepare independent immutable data in parallel and retain the required native access boundary.
Do not manufacture unsafe sharing or clone state in a way that changes solver/import semantics.

Canonical emission and required dependency-promotion waves remain deterministic under the
existing model. Queues consume the operation's resource budget; real downstream capacity controls
backpressure. Accepted writes, nested workers and native work drain on cancellation/failure.
Independent provider completion may overlap, but a provider is Complete only after its declared
outputs, capture validity, accepted writes and dependencies satisfy their actual contracts.

### Type-work semantics

Preserve encounter/depth/truncation behavior in the initial optimization. Cache immutable encoding
or display only with all relevant provider/context, depth, remaining-work and opacity premises,
and still account for required encounters. If exact reuse would require replaying the entire work,
prefer simpler local preparation or no memoization. Changing the limit from encounters to unique
types, or making truncation depend on parallel scheduling, is not a silent performance correction.
Any later semantic-policy proposal must be explicit, independently assessed and routed through the
model owner; this series does not authorize a new limit or weaker coverage.

### Capture-owned integrity

A capability over exact capture membership and bytes replaces repeated whole-tree verification
only after every actual consumer read is protected. Prefer immutable captured buffers or owned
indexed source storage where the analyzer permits it; otherwise use a capture-owned read-verified
route that checks the actual returned bytes against the manifest and closes the captured universe.
A final whole-capture check alone cannot exclude change→read→restore. A same-user permission bit,
mtime, an open path or a digest from an earlier read is not an immutable/read-verified capability.

All Ruff/Pyrefly/ty reads, promoted dependencies, configuration, document-derived code and original
byte access must be covered or retain the necessary checks until a sound replacement exists.
Intermediate outputs remain provisional until required integrity is established. An integrity
failure cancels dependent work and refuses completed admission; it must not leave an apparently
Complete mixture of source generations. Keep source handles/backing alive through last borrower.

**Rule route:** RC08 is approved by the operator on 2026-10-18 and changes the frequency/mechanism,
not the integrity guarantee. SP00 routes the decision before removing checks. RC01/RC02/RC05 also
apply where providers migrate to private source output or demand changes preparation. Required
fact families and supported optional configurations remain; demand may remove unnecessary work,
but not facts merely because a default downstream technique is disabled.

## 3. Work packages

### AX00 — Settle capture-read coverage and native export boundaries

**Consumes:** current source, pinned analyzer implementation and review F07/F10 now. SP00 is
required before selecting a changed integrity/durability policy, not before read-only discovery.
No telemetry platform, native service or full-library run is required to establish these contracts.

Map actual native protocol query correspondence and solver attachment, type work and module
promotion dependencies. Compare one module-local index with one native bulk observation route;
select the simpler exact interface and its lifetime. Inspect exact pinned code, not an unqualified
latest upstream API. Reuse the existing python-analyzers capability skill and check its version
against the active fork before transferring claims. A dependency bump is not a prerequisite.

Map each analyzer/configuration/dependency/original reader to the capture mechanism it really uses.
Choose immutable backing or verified actual reads for each reader class and identify necessary
adapter changes. The decision must explain how mutation during a read, additions/deletions and
change→read→restore are excluded or refused. If a library still reads uncontrolled paths, retain
its existing protection and block only the affected scan removal, not independent export fixes.

Deliver the selected correspondence API, independent work units, unchanged type-limit policy and
capture capability contract in the enduring extraction owner through the scheduled decision route.
Identify exact slices of SC00/SC01 needed for future private outputs. A settled interface is not a
claim that every read or borrowed buffer is already protected.

**Closes locally when:** AX01/AX02 have a safe native sharing boundary and AX03 has an exhaustive
reader migration obligation, error/lifetime behavior and a revealing concurrent-mutation scenario.
Only genuinely unresolved library semantics trigger a focused probe during execution.

### AX01 — Prepare protocol correspondence and direct typed dispatch

**Consumes:** AX00's native API choice and existing canonical source/attachment contract.
Independent of final private-source migration and of full catalog performance measurements.

Implement the module-local index or native bulk operation at the native semantic owner. Build
once per exact transaction/module lifetime, charge it before allocation and invalidate it when its
actual module/solver premises change. Adapt protocol export to consume the prepared correspondence
without repeated module-binding scans. Retain native solving/error/observation semantics rather
than caching an observation across incompatible contexts.

Move hot writer selection to prevalidated typed correspondence. Migrate both actual emission and
empty-output completion so neither path independently reinterprets provider inventory. Remove the
old repeated scan paths once all affected callers use the prepared mechanism. Keep any different
native query path only when it has a genuine separate contract/consumer.

**Focused closure:** calls and nested sync/async with sites, ambiguous/missing canonical attachment,
multiple similar ranges, normal/exceptional exit status, unavailable/lazy observations, imported
modules and provider contexts. Hand-authored expected attachment/fidelity challenges are needed;
comparing only old and new methods can preserve a shared defect. Include resource refusal and
cancellation without partially published observations or lost diagnostics.

### AX02 — Execute independent export work without changing fact semantics

**Consumes:** AX00 sharing/ordering decision; AX01 correspondence where module jobs use it.
SC00/SC01 are prerequisites only for the slice migrating provider results to new private backing;
existing safe extraction-local improvements can land against the current sink first.

Migrate independent root/module work onto existing owned CPU/async composition while preserving
required native transaction attachment and promotion waves. Factor immutable per-module/export
preparation from mutable solver operations. Merge canonical output and aggregate coverage by the
existing ordered semantic rules, not task completion order. Use direct dispatch from AX01, actual
bounded charged queues and downstream acknowledgements; do not split by family solely to create
more concurrent writers.

Eliminate repeated type presentation/construction only under the unchanged encounter/depth rule.
Use local reusable preparation where exact, and leave non-substitutable work uncached. Ensure
parallel jobs do not turn the existing per-operation encounter limit into per-thread limits or
race for a global counter that makes output depend on scheduling. Independent provider completion
may overlap where its domains/effects are independent; retain required writer/descriptor ordering.

Retire obsolete serial orchestration only for the migrated independent work. The initial native
parallel analysis, necessary solver waves, same-generation source checks and all selected fact
families remain production behavior, including Behavioral and optional protocol consumers.

**Focused closure:** canonical identical attributed outputs under shuffled module completion,
import promotion, recursive types, repeated/display-only terms, encounter/depth boundaries and
truncation, partial/unavailable coverage, backpressure, cancelled providers and joined accepted
writes. Compare explicit expected boundary outcomes as well as ordinary output equivalence.
No worker/thread caps or longer timeout are substitutes for a valid ownership design.

### AX03 — Carry established capture integrity through provider completion

**Consumes:** AX00 exact reader coverage decision and SP00/RC08 decision route. Implemented
capture backing/reader protection must precede removing any provider `verify_closed` check.
SC00 consumes capture validity as a premise; it does not itself make mutable files immutable.

Implement the chosen capture-owned capability and migrate actual reader classes together with
provider completion. Verify bytes at their actual consuming read when immutability is unavailable;
keep exact membership, paths, role/configuration and dependency closure bound to the capability.
Ensure all native lazy reads and promoted dependencies use protected sources. Derived code keeps
its exact original mapping and original bytes rather than becoming an unqualified new input.

Reduce repeated whole-tree scans only after the affected protection exists. Local writer finish
checks may remain distinct from global integrity finalization; route the established property to
all dependent consumers and invalidate/refuse affected results on failure. Keep shared backing,
file/reader handles and charges through asynchronous workers and last borrower. Private cleanup
uses existing source/storage owners, not mtime or an independent purge.

Migrate Facts-frontier admission and higher consumers to the exact protected capability through
SC01/SN00's existing validity ownership. A cancelled/integrity-failed provider cannot mint reusable
source validity. Remove unused scanner-only paths once every current consumer is protected; keep
independent incoming admission and fresh native-state assurance where their trust boundary differs.

**Focused closure:** byte mutation during read, change→read→restore, late added/deleted paths,
symlink/forbidden entries, dependency/config changes, lazy imports, document-derived source mapping,
consumer cancellation and finalization failure. Expected result is exact original bytes or explicit
refusal, never mixed-source Complete output. Establish that scan frequency no longer multiplies
with provider boundaries without adopting a numerical performance target.

## 4. Integration and completion

AX01 and safe AX02 preparation can progress alongside AX03 reader migration. Coordinate shared
`bundle`/`pyrefly_stage` edits; logical independence does not permit conflicting changes. The
compiler's SC02 owns demand semantics, SC03 owns cross-provider scheduling/visibility, and SC00
owns private completed data. This plan supplies actual provider completion and integrity premises
rather than redefining those owners. Analysis AA04 owns C2/S0 parallel work from review F07.
Diagnostics owns optional native callbacks/counters and interpretation of elapsed time; those
observations are not required before a source-established correspondence correction.

During implementation run touched-crate compile checks and minimal revealing affected controls.
Use real native library integration for claims about its export/solver behavior. Pure capture,
correspondence and limit tests need no database. SP90 owns integrated functional scope over both
profiles, optional configurations and requested frontiers; SP95 separately owns later comparable
FastMCP benefit measurement. No benchmark command family, diagnostic CLI parity claim or extensive
environment qualification is added here. Embedding inference/MCP query runtime remain excluded.

At cutover keep original evidence, current codebooks and required fact families; remove replaced
linear dispatch/correspondence and verification paths only after their consumers migrate. Project
regeneration follows the coordinator/native owner, without old-format readers or test-only
production APIs. Package completion requires correct output, integrity, resource lifetime and
failure joins. It is separate from speed measurement and from the broader review finding's closure.
