# Python fact collection: architecture and execution efficiency

**Design / target review · 2026-10-17 · Decision: Revise**

The existing architecture has a sound semantic foundation for collecting the same facts more
efficiently. It separates captured source, independent analyzer observations, canonical
occurrences, typed facts and completed output ownership. It already uses native Pyrefly, Ruff
and ty capabilities for substantial analysis rather than recreating their engines.

The principal weaknesses are in the adapter execution around those engines. Repeated searches
multiply work with module size; the Behavioral adapter prepares the same occurrence inventory
twice; definitions and calls repeat native module preparation; and cancellation does not reach
some substantial native traversals. Raw Behavioral flow expansion also precedes attempt resource
admission and downstream backpressure. These are independent causes. They warrant targeted
corrections rather than an overarching replacement of the analyzers or fact model.

Normal Pyrefly checking is a useful comparison for retention and reporting, but it does not
produce the current fact inventory. Replacing `Require::Everything` with the normal diagnostic
requirement would discard state that existing collectors inspect. Latest upstream does not
establish that this substitution is now safe.

## 1. Scope, standard and evidence

| Field | Assessment |
|---|---|
| Baseline | Clean main `842e1f4e07a1a2bc70286c8f3991cf2ffb82f4c3` |
| Standard | Core/template 3.3, efficient-architecture heuristics 1.0, code-intelligence profile 1.5 and repository binding |
| Reviewer | Independent delegated design reviewer; coordinator publishes and reconciles evidence |
| Purpose | Assess the best structure for efficient collection while preserving the current facts and their meaning |
| Supported scope | Source capture → analyzer preparation/analysis → native queries/traversals → typed fact construction → batch handoff/backpressure |
| Profiles | Catalog and Behavioral, including ty flow |
| Adjacent consumers | Model admission, canonical attachment, completed provider inputs, compiler ingestion and contribution completion |
| Exclusions | Downstream analysis kernels, embedding, MCP serving, real-library qualification, operator adoption and native database internals beyond the ingestion/completion seam |
| Evidence strength | Implemented and Interface-checked source, inspected 2026-10-17; corrections remain Proposed |
| Runtime limit | No fresh execution evidence or quantitative performance claim |

The workload premise is complete pinned Python releases and selected corpora, including modules
containing many functions, parameters, calls, bindings and repeated types. Behavioral collection
adds flow observations, multiple reaching candidates, loops and nested scopes. Concurrent
attempts and interrupted consumers are legitimate cases. Tiny fixtures do not stand in for that
workload.

`STATUS.md` and the runtime coordinator describe current ownership, stopped execution and
outstanding acceptance. They are context, not review criteria. This review neither resumes those
tracks nor closes their protected obligations.

Source coverage includes the relevant extraction/model owners, `pyrefly_stage.rs`,
syntax/type/lexical adapters, Ruff contextual collection, `ty_flow.rs`, `cpg-flow`, batching and
provider lifecycle, and the compiler ingress/completion seam. It also includes the exact locked
analyzer sources and the immutable latest Pyrefly snapshot identified below. It is not an
exhaustive correctness audit of every emitted fact.

The [evidence scope](../evidence/2026-10-17_python-fact-collection-efficiency/README.md) pins
sources and records method limits. The [library comparison](../evidence/2026-10-17_python-fact-collection-efficiency/library-comparison.md)
qualifies native capabilities and alternatives. Source references below use repository baseline
lines; unqualified extraction filenames are under `crates/cpg-extract/src/`, model filenames
under `crates/lctx-model/src/`, and analyzer filenames refer to the identified exact fork.

## 2. Responsibilities and semantic ownership

| Owner | Responsibility and contract | Expected reason for change |
|---|---|---|
| Acquisition and `CapturedInput` | Capture selected bytes, membership, dependencies and source roles; reject changes during capture | Input selection or acquisition requirements |
| Native context configuration | Explicit interpreter, platform, search roots, Ruff settings and model-context selection | Configuration or supported profile changes |
| Independent Ruff adapter | Canonical original-byte syntax, configured contextual and diagnostic observations | Ruff revision, grammar or new observational surface |
| Pyrefly adapter | Native typing, callable/class metadata, Pysa candidates, public names and diagnostics | Pyrefly revision or fact extension |
| `cpg-flow` | ty semantic-index observations and explicitly characterized runtime source view | ty revision or flow characterization |
| `lctx-model::domain` | Identities, occurrence roles, attribution, fidelity, coverage, attachment and fact contracts | Changed domain meaning |
| `StageContext` and batching | Declared outputs, validated typed rows, conflict/deduplication behavior and bounded transfers | Transfer mechanism or lifecycle changes |
| Compiler workspace/ingress | Attempt-owned pending output, acknowledgement, drainage and completed visibility | Persistence or completion mechanism changes |

The dependency direction is appropriate: analyzers establish their own observations; adapters
attach and lower them under model contracts; batching and ingestion realize transfer and
lifecycle without becoming alternate semantic authorities.

The model governs behavior, rather than merely naming output structures. Examples include:

- Source/context checks in `CanonicalSyntax::parse` and contextual attachment.
- Canonical occurrence identity, role and exact/ambiguous attachment outcomes.
- Separate source observations, analyzer assertions and candidate modalities.
- Structural types versus display-only or truncated types.
- Separate coverage states and explicit subject boundaries.
- Producer declarations and validation before typed output is accepted.
- Contribution completion distinct from publication authority.

### Fact and fidelity preservation

| Fact scope | Required distinction |
|---|---|
| Syntax | Latest independent Ruff's original-byte parse remains canonical; Pyrefly's embedded Ruff is a separate nominal family |
| Native calls | Pysa candidates and origins remain native observations; they do not become guaranteed runtime calls |
| Types | Structure, presentations, opaque/truncated closure and query availability remain distinguishable |
| Lexical evidence | Local recognizer output and native Ruff observations retain their attribution and correspondence |
| Flow | ty index observations, runtime-view characterization, reachability and narrowing remain distinct |
| Coverage | Complete, partial, unavailable, failed and unrequested remain distinguishable |
| Identity | Canonical occurrence and semantic identities do not become analyzer-local indices |

“Preserve the same facts” includes payloads, required ordering and multiplicity, native origins,
source attachment, fidelity, coverage and boundaries. Adapter source changes can legitimately
change implementation provenance and dependent identities. Comparisons must expose that
correspondence rather than conceal the new implementation identity to obtain matching hashes.

## 3. Existing strengths and preservation constraints

**Implemented / Interface-checked, 2026-10-17.**

Capture copies and hashes the actual stream, checks file identity/length and rechecks earlier
files before success (`capture.rs:172–223`). Frozen input separates analysis from a changing
source checkout. Canonical parsing verifies exact content and length against its
`SourceArtifact` (`ruff_context.rs:45–60`). These checks address real uncertainty; this review
does not recommend removing them.

Configuration is explicit. `configured` disables search-path heuristics and fallback discovery,
records interpreter/platform/environment information and uses the frozen roots
(`pyrefly_stage.rs:382–444`). Flow uses a virtual filesystem and explicit program settings.

The Pyrefly analysis pool is shared across invocations and uses available CPU parallelism
(`pyrefly_stage.rs:95–119`). The ordered extraction phase is separate from parallel analysis.
The reviewed evidence does not justify imposing worker or thread limits.

Inherited field correspondence already prepares compact name spans once per exact source and
caches refusals without certifying failed preparations as absence. Current-root spans are
reused directly (`pyrefly_stage.rs`, `InheritedFieldSpans` and the field-occurrence callback).
This is a useful existing pattern for the proposed local preparation corrections.

Flow processing already emits one module before querying the next. Callback failure stops
later module queries (`crates/cpg-flow/src/lib.rs:405–462`). The proposed cancellation
correction must preserve this behavior and the shared virtual import universe.

Batching is substantive. `BatchWriter` validates rows, rejects conflicting payloads with the
same identity, deduplicates equal rows across flushes and transfers reservations with batches
(`domain/batching.rs:26–164`). Provider completion flushes output and waits for sink completion.
The production sink retains actual provider joins and drains ingress before marking completion.

These mechanisms should survive the corrections. A bounded outgoing batch does not, however,
bound all analyzer state, adapter inventories or preceding computation.

## 4. Pyrefly: normal checking and fact collection

The locked fork is `63cda076956013cd0bd1d0d05c785f747fe6adc0`, based on upstream
`80cec3f57364bc11d4a39a419f6894a8eabcaa00`. Both sources were inspected. The latest upstream
comparison is immutable snapshot `4e602d5f490e7723392904efbe606d1b559d311c`.

### Retention is selected, not blanket

**Implemented / Interface-checked, 2026-10-17.**

The extraction transaction defaults to `Require::Exports`. Explicit roots and selected
dependency waves request `Require::Everything` (`pyrefly_stage.rs:106–119`, `550–575`,
`1450–1505`; locked `state/state.rs:3626–3634`).

Normal diagnostic checking selects `Errors` for checked files and ordinarily `Exports` for
imported context. Trace/debug/Glean modes request more retention; Pysa reporting also changes
the dependency requirement. Latest upstream retains this policy.
[Latest requirement selection](https://github.com/facebook/pyrefly/blob/4e602d5f490e7723392904efbe606d1b559d311c/pyrefly/lib/commands/check.rs#L1762).

`Everything` preserves ASTs, answers and traces that the current post-analysis collectors read.
Normal diagnostics therefore are not an equivalent fact producer. Conversely, extraction does
not explicitly retain every transitively imported module at `Everything`.

### Pre-eviction collection is a candidate, not a ready replacement

Both locked and latest sources have module reporters and a solutions hook. Their ordering
matters: with reporting enabled at a lower requirement, AST eviction can precede the generic
solutions hook. The hook alone therefore does not establish access to every current fact.
[Latest reporter and eviction ordering](https://github.com/facebook/pyrefly/blob/4e602d5f490e7723392904efbe606d1b559d311c/pyrefly/lib/state/state.rs#L1651).

A module-completion collector could potentially reduce the retained working set, but it would
need to preserve native traces, lazy cross-module queries, source attachment, public-name tracing,
progressive dependency selection, deterministic output and failure ownership. This is
**Proposed** investigation, not a finding that the current engine policy is wrong.

The locked `write_files=false` reporter exits before report collectors
(`report/pysa.rs:267–270`). There is no duplicate automatic report export in that mode. The
demonstrated duplication is the adapter's separate explicit preparation described in F04.

Latest upstream does not supply the fork's exact observational seams unchanged. An upgrade
requires compatibility work and fact comparison; it is not an established performance remedy.

## 5. Execution findings

The six findings are unscheduled. The source review owns their dated diagnosis. Transfer to an
authorized plan must establish the single disposition owner. The revisit trigger for each is
the next implementation of Python fact-collection efficiency.

<a id="F01"></a>

### F01 — Canonical and native lookup work repeatedly scans growing collections

**Priority: High. Diagnosis: Implemented / Interface-checked. Correction: Proposed.**

**Principles:** FP-07, DP-10; A4 violated. No independent false-fact gate failure demonstrated.

Type collection repeatedly searches collections whose membership is stable for the current
module:

- Per-function declaration lookup (`type_records.rs:1539–1544`).
- Per-parameter parameter/formal lookup (`1573–1583`).
- Per-function annotation-key search (`1598–1604`).
- Per-call scan of all span nodes to identify the unique Arguments child (`1746–1758`).
- Signature decomposition searches growing term/member/slot vectors (`541–710`).

Native lexical writing similarly searches scopes for each observation, events for each binding
and source targets for each native target (`pyrefly_stage.rs:3058–3085`).

For a module growing in functions, parameters or calls, these paths repeat unrelated
examination. The call-to-Arguments path alone multiplies calls by the complete span inventory.
The authoritative coordinates are already available, and `Spans` already retains parent/child
structure.

**Correction.** Prepare lookup views at the module/collector lifetime: declaration by exact
name occurrence, parameters by declaration/ordinal, formals by parameter, Arguments candidates
by parent/kind, terms by identity and ordered members by owning sequence/list. Use keyed
scope/event/target access for lexical writing. Ordinary maps and borrowed views are sufficient;
no provider framework is needed.

**Preserve.** Existing first-retained, duplicate, ambiguity and ordering behavior. A map that
silently overwrites duplicates is not an equivalent replacement. Lookup acceleration must not
change the encounter- and depth-sensitive type truncation contract.

**Closure evidence.** Inspect removal of the repeated scans and compare complete facts on a
module with many functions, parameters and calls, including same-span alternatives, overloads,
missing attachments and duplicate native answers. Include limit-edge cases if construction
order changes. A speed claim additionally requires measurement.

<a id="F02"></a>

### F02 — Each flow attachment scans the complete native node inventory

**Priority: High. Diagnosis: Implemented / Interface-checked. Correction: Proposed.**

**Principles:** FP-07, DP-10, CI-07; A4 violated.

`Writer::attach` scans `flow.nodes` for each event, filters by span and optional kind, then
sorts/deduplicates candidate shapes (`ty_flow.rs:774–804`). The canonical attacher is indexed,
but this preceding native correspondence step is not.

As a module gains nodes and flow events, correspondence work grows with their product. Uses,
definitions, reaching candidates, captures and value observations can make many attachment
requests against the same immutable node inventory.

**Correction.** Prepare a span-to-candidate-shapes lookup once per `ModuleFlow`, retaining the
current kind/role filtering and exact ambiguity rule. This is a physical index over native
observations, not new semantic authority.

**Preserve.** Equal-span alternatives, scalar-wrapper exclusion, optional-kind behavior and
refusal when no unique shape exists.

**Closure evidence.** Compare attachment outcomes and emitted facts for duplicate spans,
alternative roles/kinds, unsupported shapes, annotations and a module containing many events.
Source inspection should establish that each request examines its candidate bucket rather than
every node.

<a id="F03"></a>

### F03 — Behavioral collection hydrates canonical occurrences twice for concurrent representations

**Priority: Moderate. Diagnosis: Implemented / Interface-checked. Correction: Proposed.**

**Principles:** FP-07, DP-10, DP-20; A4 violated.

`TyFlow::Index::new` reads and clones the full `Occurrence` relation into its own map
(`ty_flow.rs:138–163`). On first attachment, `StageContext::attacher` reads the same relation
again, collects its batches and constructs another occurrence index (`bundle.rs:428–440`;
`assembly.rs:31–39`).

The first occurrence map remains live while the second read and preparation occur. The cost
includes repeated native transfer/decoding and simultaneous full-map, batch and attachment-index
representations. Source-local indexes do not remove this duplication.

**Correction.** Establish one prepared canonical-occurrence input for this consumer. Its actual
required fields and identities can feed both flow lookup and model-owned attachment. Retain
independent semantic enforcement; sharing an immutable input does not mean sharing an
independently earned admission result.

Viable forms include preparing both views from one read, or constructing the existing
attachment index from the already owned occurrence inventory. Source partitioning is a later
option where the read contract can express it.

**Preserve.** Attachment budget, ambiguity, candidate ordering, source/context separation and
actual reservation lifetimes.

**Closure evidence.** Demonstrate one source read/preparation path for this inventory and
compare both lookup and attachment results. Include a refusal/cancellation case showing that
backing allocations and reservations remain owned until their last consumer ends.

<a id="F04"></a>

### F04 — Definitions and calls repeat native module preparation

**Priority: Moderate. Diagnosis: Implemented / Interface-checked. Correction: Proposed.**

**Principles:** FP-03, FP-07, DP-10; A4 violated.

`definitions` constructs a resolver/context and collects captured variables
(`pyrefly_stage.rs:1812–1819`). `calls` constructs another resolver/context and recollects
captured variables for the same module/session (`2431–2437`). The native collector builds a
definition-to-function map and visits the module AST (`report/pysa/captured_variable.rs:322–335`).

This is repeated preparation over unchanged module premises, not a claim that the analyzer
reruns all type checking.

The native Pysa reporter demonstrates a suitable composition: one context and capture
collection supply definitions and call graphs.
[Upstream shared module preparation](https://github.com/facebook/pyrefly/blob/4e602d5f490e7723392904efbe606d1b559d311c/pyrefly/lib/report/pysa.rs#L284).

**Correction.** Give module-local native extraction one borrowed preparation scope and pass
its context/captures to the existing specialized collectors. Keep typed lowering and each fact
family's attribution separate. Do not replace typed production with serialized report files.

**Preserve.** Lazy resolution, source-versus-synthesized origins, MRO completeness, native
overload identity and definitions-only dependency extraction. Dependency modules must not
acquire call-graph work merely to reuse a helper.

**Closure evidence.** Compare definitions, captures, calls and native origins for nested
functions, nonlocal/global captures, callable fields and dependencies. Inspect that each
analyzed root prepares captures once, while definitions-only dependencies retain their narrower
work.

<a id="F05"></a>

### F05 — Cancellation does not reach substantial Ruff and ty work segments

**Priority: High. Diagnosis: Implemented / Interface-checked. Correction: Proposed.**

**Principles:** FP-07, DP-20; A4 violated. No premature publication or false completion
demonstrated.

Provider cancellation owns native work between sink calls (`bundle.rs:174` onward), and
Pyrefly registers its native cancellation handle. Some other substantial segments do not
consume that signal:

- Ruff's production `Collect` implements `observe`, but not `Sink::checkpoint`
  (`ruff_context.rs:180–310`). The fork supplies checkpoint calls during traversal, but the
  default checkpoint always succeeds (`semantic_facts.rs:64`, `218–232`).
- `cpg_flow::visit_modules` renames/parses and installs every virtual source before its first
  callback (`crates/cpg-flow/src/lib.rs:418–438`).
- A module is analyzed into `ModuleFlow` before the callback can check cancellation
  (`440–460`).

An abandoned consumer can therefore wait for work that has no remaining consumer. Drainage
retains ownership correctly; it does not make the avoidable work useful.

**Correction.** Propagate the existing cancellation authority into Ruff's checkpoint and into
flow preparation/module traversal at useful cooperative boundaries. Keep the shared virtual
import universe and existing model outcomes. Cancellation must abort the attempt through its
owner, not become “missing evidence” or complete empty coverage.

**Preserve.** Actual native joins, private pending output, no partial completed contribution,
exact failed-module correspondence and callback-error stopping behavior.

**Closure evidence.** Use focused DB-free controls that cancel during Ruff traversal, during
flow preparation before any callback, and during a large module traversal. Assert that later
work stops, failure is not relabelled as completion and the actual provider owner drains. No
new thread limits are indicated.

<a id="F06"></a>

### F06 — Adapter-owned flow expansion precedes resource admission and backpressure

**Priority: High. Diagnosis: Implemented / Interface-checked, 2026-10-17. Correction: Proposed.**

**Principles:** FP-07, DP-20, CI-08; A4 violated. DP-20 is violated for these adapter-owned
intermediates. No process-RSS guarantee or premature-publication failure is alleged.

`cpg-flow` builds a complete `ModuleFlow` before the typed writer receives it. That value owns
vectors of uses, definitions, reaching facts, candidates, captures, value sources, regions,
tests, test leaves and native nodes, plus place maps (`crates/cpg-flow/src/lib.rs:304–334`).
`module` creates this accumulator and auxiliary maps without an attempt-budget argument
(`507–542`).

This is more than third-party analyzer retention. Adapter code itself:

- Collects and lowers each use's native bindings into temporary vectors (`931–950`).
- Clones reachability and narrowing formulas into candidates and reaching facts (`971–1042`).
- Expands loop headers into additional temporary and retained results (`1162–1197`).
- Accumulates native nodes and places before flow translation (`563–615`).

The current flow source contains no `ResourceBudget`, `Reservation` or `StateCharge`
integration. Typed admission starts after the completed raw module reaches `Writer::write`
(`ty_flow.rs:630–661`, `1054` onward).

Individual conditions have a 4,096-node bound (`domain/conditions/graph.rs:12`, `97–123`).
That bounds a condition's structure, not the number or combined bytes of retained conditions,
candidate inventories or expanded reaching facts. A source-size bound likewise does not
govern their combined live allocation.

For modules with many uses and many candidate definitions, the adapter retains the necessary
use/candidate expansion and repeated formula copies alongside canonical indexes, original
source buffers, transformed virtual files and Salsa state. Downstream batch backpressure cannot
constrain expansion that has already completed. F03's duplicate occurrence preparation adds to
this working set but is a different cause.

**Correction.** Give raw flow construction explicit ownership under the existing attempt
resource authority, admitting adapter-owned growth before allocation. Couple production to
consumption at useful units—such as completed use/candidate groups—where semantic dependencies
permit. Retain compact definition/use correspondence needed for later references; use a charged
or spillable representation where complete inventories genuinely must remain available.

This is not a recommendation to remove candidates, narrow the analysis universe, introduce
arbitrary caps or reduce normal parallelism. Merely adding earlier refusal would improve
resource truthfulness but would not, alone, establish execution fit for ordinary supported
modules.

**Preserve.** Every native candidate, ordinal, pruned/false-only distinction, loop-expanded
relationship, capture state, formula approximation and source-view identity required by the
existing facts. Successful admitted results must preserve these facts. Resource refusal and
cancellation must propagate as attempt failures through the existing owner; converting them
into the current string-based module error and then `OutsideProviderModel` coverage would
misclassify operational failure.

**Closure evidence.** Inspect reservation/ownership coverage for raw adapter containers,
temporary expansions and transferred values. Use focused DB-free controls with many
uses/candidates, loop expansion and larger formulas to establish exact fact preservation,
explicit refusal before unowned growth, and release after cancellation/last consumption.
Demonstrate that the proposed consumption or spill route serves a credible growing module;
a small batch size or passing tiny fixture is insufficient.

### Relationships and priority

F01 and F02 remove multiplicative lookup work. F03 reduces repeated transfer and overlapping
canonical representations. F04 shares stable native module preparation. F05 governs abandoned
work. F06 gives adapter-owned flow expansion an admitted lifetime and a credible consumption
route.

F03, F05 and F06 share flow-provider interfaces and must be reconciled so that shared inputs,
cancellation and raw-result ownership have compatible lifetimes. They remain independent
closure obligations. Ruff preparation sharing is a qualified investigation with explicit
diagnostic-channel and completeness constraints. F01's safe lookup indexes must remain
distinct from more speculative type-construction memoization. None of these corrections
requires a wholesale analyzer replacement.

## 6. Composition, working set and alternatives

| Route | Assessment |
|---|---|
| Current native engines with corrected adapter preparation/indexes | **Recommended, Proposed.** Removes demonstrated work while retaining established semantic boundaries |
| Normal Pyrefly diagnostics as the fact producer | **Unsuitable as a direct substitute.** Does not preserve retained AST/answers/traces and the full current fact inventory |
| Module-completion extraction before eviction | **Proposed investigation.** Potential working-set benefit; complete observation, lazy-query and failure contracts remain to be established |
| CLI report files followed by reconstruction | Native reports are useful oracles, but this route adds serialization, transfer and reconstruction and does not cover every current typed observation |
| Replace Pyrefly/Ruff/ty with one analyzer | No examined capability establishes equivalent facts; not recommended |
| Persistent cross-run analyzer cache | Not justified by this review. Introduces invalidation and lifecycle obligations beyond the demonstrated module-local reuse |
| More per-module parallelism | Does not remove repeated work; can increase overlapping state and contention |

The complete live set can include retained Pyrefly state, an independent canonical parse,
syntax records and clones, type/call collector outputs, per-relation batch writers and native
ingress. Behavioral execution adds whole-input lexical indexes, original/renamed source
buffers, the ty database and current `ModuleFlow`.

Some duplication is purposeful. The three source interpretations are not interchangeable:
canonical Ruff syntax, Pyrefly's embedded parser and ty's characterized runtime view serve
different facts. Parser-only renaming is also not an established equivalent replacement for
the current recovery-aware transformation.

Some opportunities remain **Proposed**, rather than findings:

- Narrowing `records.clone()` to the syntax subsets actually consumed
  (`pyrefly_stage.rs:906–907`).
- Reducing duplicate type construction/presentation before batch-writer deduplication.
- Reducing retained analyzer state with qualified completion-time extraction.

Type interning requires particular care. `Builder::term` increments work per encounter and
truncates beyond depth 32 or work 100,000 (`type_records.rs:892–925`). Unconditional memoization
can change limit-case facts and fidelity. The current exact-fact-preservation requirement
excludes silently changing that behavior.

<a id="I01"></a>

### I01 — Qualified investigation: share Ruff preparation without conflating diagnostic channels or completeness

**Implemented / Interface-checked, 2026-10-17.** The canonical parse is shared, but Checker work
is not. Selected diagnostics call `check_path` with `Noqa::Enabled` and again with
`Noqa::Disabled` (`diagnostic_records.rs:147–165`). Contextual collection separately runs
`check_ast_with_observation` with an empty rule table (Ruff fork `semantic_facts.rs:178–197`).
Thus a successful fully observed module has three Checker executions over the same parsed
source, including repeated semantic preparation.

This is a meaningful optimization candidate, but the source does **not** establish three
interchangeable computations:

- F401 consults suppression during rule execution, separating ignored and unused imports
  before constructing diagnostics and shared fixes (`unused_import.rs:378–405`, `467–513`).
  Noqa is not exclusively a final diagnostic filter. Differences can affect headline identity,
  not merely omitted fixes.
- The production adapter compares the two diagnostic inventories and refuses unmatched
  enabled results (`diagnostic_records.rs:175–185`, `239–244`).
- Observation cessation currently gates deferred traversal, exports, bindings,
  unresolved-reference analysis and final scopes (`checkers/ast/mod.rs:3599–3644`). Attaching
  a bounded observer to a diagnostic run can therefore change diagnostic completeness unless
  observation stopping and Checker completion are separated.

**Recommendation: Proposed, Moderate priority.** Investigate a shared semantic preparation
or observation-capable diagnostic route. Preserve independent completion of the requested
diagnostic computation when the observational consumer exhausts its own allowance. Preserve
the actual two-channel diagnostic contract; do not derive enabled results by naïvely filtering
the disabled inventory.

This is **not an additional blocking finding at present**: the repeated executions are
demonstrated, but a simpler complete equivalent has not yet been qualified. The investigation
trigger is the next modification of Ruff diagnostic/context collection for efficiency. If an
exact shared-preparation route preserves these contracts, retaining repeated full preparation
without a concrete benefit should then be reassessed under A4.

Settling evidence should include suppressed and mixed multi-import statements, parent-line
suppression, F821/F841, recovered/unsupported syntax, observational row/work refusal, and
comparison of diagnostic identity, message, spans, annotations, channel and coverage. Compare
the facts actually retained by the adapter, while accounting for upstream suppression-dependent
construction.

### Other open design questions and decision triggers

These are **Proposed investigations**, not substitutions accepted by the current review. They
remain unscheduled here until an authorized plan takes ownership. The library comparison
records their source evidence and limits.

| ID | Question, priority and trigger | Required decision evidence |
|---|---|---|
| <a id="I02"></a>I02 | Moderate: can repeated type structure/presentations be prepared once before batching? Trigger: redesign of type lowering beyond F01's lookup-only changes | Complete native/context identity and qualified observations; generic chosen versus candidate types; depth/encounter-sensitive truncation, fidelity, restrictions and defaults. Retain the existing limits unless a separate semantic change is explicitly selected |
| <a id="I03"></a>I03 | Moderate: can selected dependency preparation avoid late requirement promotion/full report construction, or can rich facts be captured before eviction? Trigger: redesign of Pyrefly dependency waves or retained-state lifetime | Native requirement promotion currently rebuilds a loaded module; definitions are exported before the keep filter. Qualify complete enclosing/capture/MRO closure, lazy queries, analysis epoch, exact attachment, cancellation and finality. Neither eager Everything for all dependencies nor Errors-only collection is a justified default |
| <a id="I04"></a>I04 | Moderate: can ty preparation use source-scoped canonical input, release original/parsed state sooner and safely query independent modules concurrently? Trigger: after F03/F05/F06 ownership and cancellation contracts are resolved | Preserve the full virtual import universe, exported-name and wildcard cycles, callback failure/order semantics and bounded owned results. Compare native selective reads or one partitioning pass; avoid scalar per-record reads. Upstream AST clearing can force reparsing, so qualify the composed tradeoff |
| <a id="I05"></a>I05 | Lower priority: can syntax copies and name-kind conversion be narrowed to what each collector consumes? Trigger: editing collector input or node-shape interfaces | Preserve syntax distinctions, parser recovery and all consumed fields. Canonical token reuse requires exact source kind/version/recovery correspondence; no persisted token dataset or new generic framework is justified merely to remove one parse |

## 7. Change and failure scenarios

| Scenario | Assessment |
|---|---|
| Analyze another pinned library | Existing acquisition, configuration and provider contracts compose without changing fact meaning |
| Add a native observational fact | Model relation/meaning, provider declaration and adapter extraction legitimately change; downstream consumers should not gain native analyzer types |
| Upgrade Pyrefly | Changes stay within the native boundary and qualification controls, but hidden/native APIs and fork observations require explicit compatibility work |
| Upgrade Ruff/ty | Nominal parser families and source-view correspondence keep differing interpretations explicit; coordinate compatibility must be re-established |
| Substitute indexed linking | Local physical replacement is credible if duplicate, ambiguity and ordering behavior remain exact |
| Substitute module-completion extraction | Requires a complete account of available state, dependencies and failure ownership; not approved by hook existence |
| Many calls/functions/bindings | F01/F02 show avoidable multiplicative work |
| Many sources in Behavioral | F03 shows repeated full occurrence preparation; one-module result streaming does not remove it |
| Cyclic imports or recursive types | Native solver/recursive-reference semantics must remain; output selection must not cut necessary context |
| Missing/recovered/unsupported input | Existing explicit boundaries and coverage must survive optimization |
| Consumer abandonment | Existing drainage protects ownership; F05 must reduce unnecessary work before drainage can complete |

The corrections are composable and do not require new semantic owners. A deliberate change
to the type work bound, source interpretation or completeness promise would be a separate
semantic change.

## 8. Independent architectural judgments

| Judgment | Verdict | Basis |
|---|---|---|
| A1 Localize change | **Satisfied, scoped** | Analyzer details remain inside coherent adapters; model and completed-output contracts constrain propagation. The selected extension/upgrade scenarios have identifiable owners |
| A2 Encode domain meaning explicitly | **Satisfied, scoped** | Attribution, source/context identity, fidelity, coverage, attachment and completion govern executing behavior |
| A3 Extend through composition | **Satisfied, scoped** | Native capabilities and model operations compose without a new generic framework. F04 is a physical preparation defect, not evidence of missing semantic ownership |
| A4 Fit execution to workload | **Violated** | F01–F06 establish avoidable examination, preparation, live representations, abandoned work and unadmitted raw flow expansion |

FP-01, FP-02, FP-04, FP-05 and FP-06 are satisfied within the examined scenarios. FP-03 is
substantially satisfied, with the physical composition weakness in F04. FP-07 is violated.

DP-10 is violated by the demonstrated repeated preparation and scans. DP-20 is violated by
adapter-owned flow expansion outside attempt resource admission, and cancellation coordination
remains incomplete in F05's segments. CI-08's combined intermediate cardinality/byte control is
incomplete at the raw flow boundary. Existing typed batching, drainage and per-condition bounds
remain valuable but do not discharge those upstream obligations. Scoped source evidence supports
the relevant authority, typing, identity, transformation and explicit-outcome rules. It does
not certify every fact producer or the enclosing publication system.

## 9. Correctness and fidelity gates

These verdicts concern the inspected collection contracts, not release qualification.

| Gate | Verdict | Evidence or limit |
|---|---|---|
| G1 Authority | Pass, scoped | Model meanings and analyzer observations retain distinct scopes; physical indexes need not become new authorities |
| G2 Semantic fidelity | Pass, scoped | Source roles, native candidates, structural/display types, coverage and attachment alternatives remain explicit |
| G3 Validity | Pass, scoped | Captured-source checks, typed validation, conflict refusal and exact attachment guard inspected assumptions |
| G4 Hidden behavior | Pass, scoped | Explicit frozen configuration and virtual flow inputs; no observed silent analyzer substitution |
| G5 Consistency/recovery | Pass at examined seam | Pending-output, provider-join, ingress-drainage and completed-visibility contracts inspected. F06 is a resource-admission MUST gap, not a demonstrated partial-commit defect; native persistence internals excluded |
| G6 Transformation/reuse | Pass for baseline inspected contracts | Source-view and attachment distinctions retained; proposed replacements still require equivalence evidence |
| G7 Truthful capability claims | Pass, scoped | No review claim promotes static evidence into measured speed or current assembled acceptance |
| G8 Library leverage | Pass, scoped | Native analysis/reporting/semantic-index capabilities are used; bespoke code fills model-lowering and observation gaps |
| CI-G1 Fidelity | Pass, scoped | No demonstrated relabelling, unknown-as-absence or model-exceeding claim |
| CI-G2 Evidence closure | Not applicable to serving | Collection evidence/identity seam inspected; served journeys excluded |
| CI-G3 Evaluation integrity | Pass at production-input seam | No inspected route from gold/evaluation reference content into these collectors; wider evaluation not examined |

Passing these gates does not offset A4 or F06's resource-admission MUST gap. None of F01–F06 requires alleging false current facts
to justify revision.

## 10. Verification and closure

**Source inspections passed, 2026-10-17:** clean baseline and exact pins checked; decisive
executing paths and locked upstream comparison read; latest immutable Pyrefly
requirement/reporting ordering fetched and read.

**Not run:** compile checks, product tests, runtime probes, native database queries, services
and real-library qualification. No numerical latency, throughput or capacity result is claimed.

For subsequent implementation, revealing equivalence controls should compare the full required
fact content under explicit implementation-provenance correspondence. Important cases are
same-span ambiguity, repeated parameters/calls, overloads, inherited and synthesized members,
recovered syntax, recursive types, unavailable dependencies and cancellation.

Safe lookup changes need not reopen type construction or analyzer scheduling. A proposed
speed claim needs representative complete collection measurements, including preparation and
handoff. Historical receipts cited by current owners remain historical and scoped.

**Publication checks, 2026-10-17:** `just docs-check` **passed**, 479 canonical pages and zero
link errors, including ADR lint (108 records) and agent lint. The independent reviewer accepted
publication of this principal and its supporting analysis after clarifying the distinction
between complete diagnostics and explicitly incomplete contextual observations. These checks
establish document integrity, not product correctness or performance.

## 11. Rule impacts and disposition

**Rule impacts: none for the six recommended corrections.** They can preserve the existing
provider roles, native analysis transaction, exact typed meaning, source capture, completeness
and completion contracts.

F06 applies the existing attempt resource authority to adapter-owned allocations; it does not
require an RSS guarantee or new concurrency/resource caps. Its newly explicit operational
refusal points must retain the existing failure and completion distinctions.

This review does not recommend changing dependency pins, adding standing gates or creating a
new scheduler/cache framework. It also does not authorize changing type truncation semantics
or substituting a lower-requirement completion-hook architecture. If those alternatives are
selected later, their semantic and architectural rule impacts must be stated at that decision.

F01–F06 remain required design corrections, unscheduled by this review. I01–I05 remain
qualified investigations with the triggers above. A future authorized plan should consume
their stable source IDs and own execution disposition. Existing runtime-plan findings and
protected obligations remain separate.

Until transfer, this source review owns the following **Deferred** dispositions. Deferral means
awaiting an authorized implementation plan, not that the diagnosed design is accepted or that
mandatory corrections can be omitted. The trigger for each is plan creation or implementation
of the named fact-collection boundary; its closure evidence is specified above.

| Source IDs | Responsible boundary | Current disposition |
|---|---|---|
| F01 | Pyrefly/native/canonical correspondence and type/lexical adapters | Deferred pending authorized plan |
| F02 | ty native-shape correspondence | Deferred pending authorized plan |
| F03 | Flow canonical input and attachment preparation | Deferred pending authorized plan |
| F04 | Pyrefly module-local native extraction | Deferred pending authorized plan |
| F05 | Producer cancellation, Ruff checkpoints and flow queries | Deferred pending authorized plan |
| F06 | Raw flow construction, resource admission and consumption | Deferred pending authorized plan |
| I01–I05 | Respective collector owners and the decision triggers in §6 | Deferred qualified investigations; no replacement selected |

## 12. Decision and enclosing status

**Revise the adapter execution structure while retaining its semantic foundation.**

A1–A3 are satisfied in the reviewed scope. A4 is violated by six demonstrated execution
causes. No independent false-fact gate failure was established.

The next consequential design choice is local: prepare stable lookup/correspondence inputs
once, share module-local native preparation, carry cancellation into substantial native
work, and give raw flow construction and consumption an admitted lifetime. Lowering analyzer
retention is a separate, more demanding alternative that has not yet
established preservation of the current facts.

This review establishes neither whole compiler architectural acceptance nor release
qualification. Its recommendations are Proposed; the diagnosed executing structures are
Implemented / Interface-checked as of 2026-10-17.
