# Catalog collection efficiency: analyzer preparation and correspondence

**Approved implementation plan · 2026-10-17.** This companion develops analyzer-local
collection work for the [collection coordinator](catalog-collection-efficiency-plan_2026-10-17.md).
The coordinator's [§6](catalog-collection-efficiency-plan_2026-10-17.md#6-sole-disposition-and-current-checkpoint)
is the sole owner of finding disposition and current execution state;
this document defines packages and their completion evidence, without a second progress ledger.
PA00–PA05 execution is authorized; the coordinator records evidence and remaining scope.
Plan approval and focused controls do not qualify the whole compiler.

The source is the [Python fact-collection target review](../design_review/reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md)
and its [pinned evidence](../design_review/evidence/2026-10-17_python-fact-collection-efficiency/README.md)
and [library comparison](../design_review/evidence/2026-10-17_python-fact-collection-efficiency/library-comparison.md).
`P:F01`, `P:F04` and `P:I01/I02/I03/I05` below mean that review's stable `F01`, `F04`,
`I01`, `I02`, `I03`, `I05`; the prefix distinguishes them from other reviews.
The review examined `842e1f4e07a1a2bc70286c8f3991cf2ffb82f4c3`. Plan authoring inspected
`ffc4dc7d7c6feb87766e8bd9b64e2997c19a4dae` on 2026-10-17. Source descriptions below are
**Implemented / Interface-checked at that baseline**, not fresh execution or measured benefits.

## 1. Outcome and scope

The target retains the existing Pyrefly, independent Ruff and ty roles while preparing stable
module inputs once and looking up exact candidates directly. Required work removes demonstrated
repeated searches and duplicate Pysa preparation. Bounded investigations settle whether additional
Ruff sharing, type construction reuse, dependency selection/retention and syntax/token/kind
changes are worthwhile and equivalent. Every investigation ends in an evidence-backed
**retain-or-correct decision**, followed by any selected implementation and consumer retirement.
It cannot finish merely by listing alternatives, and it cannot force an unqualified replacement.

The workload is a complete pinned Python release and selected corpus: modules with many
functions, parameters, calls and lexical bindings; repeated generic and overloaded types;
inherited/synthesized members; dependency chains and cycles; stubs, recovered syntax and
interrupted consumers. A large module can have modest final output while repeated correspondence
or temporary representations still dominate work. Module-local indexes and shared borrowed
preparation address that growth without a new analyzer engine, persistent cache or scheduler.

The [flow companion](catalog-collection-efficiency-flow-plan_2026-10-17.md) owns `TF01`
cancellation, including the Ruff checkpoint, plus `TF00/TF02` raw-flow/canonical-input changes. This plan
consumes those lifetimes and failure contracts. It owns `P:I05`'s decision and any selected
copy/token/kind consumer migration across extraction and flow; the flow author coordinates shared
editing surfaces. TF00 owns the native shape-index contract and lookup; TF02 integrates its
construction into admitted node enumeration and consumes any qualified PA05 mapping. PA05
does not create another attachment or shape-candidate policy.
Existing runtime admission, joins, drainage, cancellation registration and
storage owners are implementation foundations, not tasks to recreate here.

Full Catalog execution, selected-complete acceptance, assembled `qualify`, real-library
qualification and performance benchmarks need later planning at the coordinator. This scope
delivers implementation, revealing focused controls and scope-end compile/lint evidence.

## 2. Foundations and interfaces

The governing owners remain [DESIGN §B1/B8](../design/DESIGN.md),
[acquisition and extraction §4.2](../design/sections/acquisition-and-extraction.md), and
[semantic model §15](../design/sections/semantic-model.md). Model declarations under
`crates/lctx-model/src/domain/` own source/context identity, occurrence attachment,
typed observations, fidelity, ordering, coverage and completion. The core 3.3 principles,
code-intelligence profile 1.5 and efficient-architecture heuristics 1.0 inform the physical
choices below, especially preparation lifetime, indexed access and combined live state.

The focused foundation assessment reaches a bounded conclusion: these semantic owners and the
native engines are suitable foundations; the adapter preparation and access paths need correction.
The plan does not reopen the enclosing model or publication design. Capture verification, exact
source/context matching, native candidate attribution, shared validators and completion ownership
provide guarantees that an optimization must continue to consume. They are not incidental repeated
checks to delete merely because inputs are immutable elsewhere.

| Current surface | Existing behavior and target responsibility |
|---|---|
| `cpg-extract/src/pyrefly_stage.rs::session`, `run_wave` | One captured input, shared available-CPU analysis pool, ordered root extraction and selected dependency waves. Default requirement is Exports; roots/selected waves request Everything. Keep that composition unless PA04 qualifies a replacement. |
| `pyrefly_stage.rs::definitions`, `calls`, `types` | Definitions and calls each create a resolver, answers context and captured-variable collection; types creates another compatible resolver/context. PA01 shares compatible root preparation inside the exact module transaction. |
| `type_records.rs::records`, `Builder::signature_variants`, `Builder::term` | Repeated declaration/parameter/formal/annotation/Arguments and growing type-vector searches; structural construction counts every encounter before depth/work truncation. PA00 accelerates lookup without changing construction; PA03 independently decides further reuse. |
| `syntax_records.rs::Spans::{get,child,parent,nodes}`, `Records` | Existing exact source/span/kind and parent/field structure. `child` only returns a single retained child; it is not automatically equivalent to selecting Arguments by parent/kind across all fields. PA00 uses or extends the actual candidate view with unchanged refusal semantics. |
| `pyrefly_stage.rs::write_native_lexical`, `ruff_lexical.rs` | Scope/event/target correspondence and local versus native observations. PA00 preserves first matches, duplicate multiplicity and missing-owner errors while replacing repeated whole-vector examination. Existing indexed binding candidates are baseline reuse, not duplicate work. |
| `ruff_context.rs::CanonicalSyntax::{parse,diagnostics,observe,context_rows}` | One canonical original-byte parse; diagnostics and observation still prepare Checkers separately. PA02 evaluates sharing while keeping diagnostic completion independent of observer allowance. |
| `diagnostic_records.rs::ruff` | Selected F821/F401/F841, real Noqa-enabled and Noqa-disabled computations, multiset identity matching and unmatched-enabled refusal. PA02 preserves the pair's actual retained output. |
| `pyrefly_stage.rs:906–907`, `syntax_records.rs::Records` | Calls are cloned and the complete syntax record inventory is cloned before `write_records` consumes it. PA05 establishes the actual post-write consumers and narrows ownership/copies where qualified. |
| `cpg-flow/src/lib.rs::{rename,Places::enter_node}`, `typed_syntax.rs::{kind,syntax_kind}` | Recovery-aware original-token renaming; flow currently formats node kinds and searches model codes while canonical extraction has a typed mapping. PA05 assesses exact token correspondence and explicit nominal mapping. |

Relevant sources are under `crates/`; table filenames without that prefix identify
`crates/cpg-extract/src/`. Adjacent consumers are `symbol_records::Declared` and `Locator`,
parameter definition collection, call/type lowering, lexical support emission, typed `BatchWriter`
and provider completion. Their full current consumption determines migration scope; the examples
in this document are revealing cases rather than permission to leave other consumers behind.

The locked Pyrefly fork is `63cda076956013cd0bd1d0d05c785f747fe6adc0`, with embedded Ruff
0.0.14. Independent Ruff/ty is `9080ee7a82a4ec7359ba2ade60839ef1108a2968`, based on Ruff
0.16.10/crates 0.0.16, with Salsa 0.28.5. The `python-analyzers` skill matches base versions;
the repository forks' exact observation and lifecycle seams must be read before transferring a
claim. Review comparison with immutable latest Pyrefly `4e602d5f490e7723392904efbe606d1b559d311c`
is discovery evidence only. No upgrade or engine substitution is selected.

The proposed boundaries accommodate three credible changes without implementing them here.
A new native observation adds model meaning and specialized lowering; it borrows the same exact
module preparation instead of rebuilding captures. A different dependency-export requirement
changes PA04's closure selection and evidence, while canonical attachment remains model-owned.
An eventual analyzer upgrade requalifies nominal node/token correspondence, native context and
diagnostic semantics at the fork boundary; consumers continue to receive typed model facts.
None of these cases justifies exposing native types downstream or adding a generic provider
framework now.

## 3. Preserved contracts and physical target

**Proposed contract-preserving corrections.** An index is a derived physical view over an exact
module/context inventory, never a new identity or semantic authority. Use borrowed row references
or stable vector positions where suitable. Keys include all coordinates the old predicate used;
source/context may be supplied by a validated owning scope rather than repeated in every key.
Grow existing resource charges before adapter-owned allocation and retain backing ownership until
the last consumer finishes. Release indexes with their module or collector, not at session end
without a consumer. No cross-run invalidation protocol is needed.

Each old search has a separate observable contract. A first-match search remains first-retained;
uniqueness requires retaining/counting all candidates; membership requires presence rather than
an arbitrary representative. Ordered member buckets preserve the exact ordinal/identity sorting
and deduplication that signature decomposition currently applies. Never turn these into a generic
last-write-wins map. Preserve the construction/emission encounter order; a faster lookup does not
license a new traversal order or reassign native ordinals.

Module-local Pysa preparation belongs to one exact handle, transaction and analysis epoch.
Definitions and calls borrow the same resolver/context/captures; type lowering may borrow the
compatible context without receiving call graphs. Native module numbers stay run-local. Canonical
source attachment still uses original independent Ruff coordinates; Pyrefly's embedded AST and
ty's runtime view retain explicit nominal/source-role correspondence.

The current `Builder::term` increments `work` on every call and truncates when `depth > 32` or
`work > 100_000`. Preserve that behavior, including threshold crossings, recursive descendants,
opaque closure, presentation and downstream fidelity. Duplicate elimination at the batch writer
does not justify skipping native encounters or construction effects earlier. PA03 must show that
an alternative preserves these effects or retain encounter-driven construction.

Operational cancellation/resource refusal remains attempt failure under existing owners;
it is not absence, `OutsideProviderModel` coverage or successful partial completion. PA02 may
stop contextual observation at its own allowance while allowing requested diagnostics to complete;
attempt cancellation is a separate signal governed by TF00/TF01. Borrowed preparation and queued
products cannot outlive retained native premises or release their charge before drainage.

Full fact comparisons expose implementation provenance changes. Keep actual provider/build/run
identities; describe correspondence between baseline and changed invocation/source identities,
then compare semantic payload, endpoints, multiplicity, required order, native origins, attachment,
fidelity, boundaries and coverage under that explicit mapping. Never overwrite the new digest or
strip unknown differences to manufacture identical hashes. A Pysa JSON report comparison alone
does not cover every typed/support/coverage row.

## 4. Work packages and decisions

All packages are **Proposed**. Logical dependencies below identify usable capabilities, not
whole-document barriers. Most share `pyrefly_stage.rs`, so ready work still requires coordinated
edits. PA00 and PA01 can be implemented independently; PA03 consumes PA00's indexed type views;
PA04 consumes PA01's separated preparation and collectors. Investigations can inspect source
earlier but cannot migrate consumers against an unimplemented lifetime contract.

| Package | Obligation | Required prerequisite and deliverable |
|---|---|---|
| [PA00](#pa00) | P:F01 | Existing exact inventories → module/collector lookup views used by all identified type and lexical correspondence consumers. |
| [PA01](#pa01) | P:F04 | Existing native session → shared borrowed Pysa preparation used by root definitions/calls and compatible types; dependency extraction stays definitions-only. |
| [PA02](#pa02) | P:I01 | Exact Ruff contracts and TF00/TF01 cancellation seam for integration → retain-or-correct decision, then qualified sharing and removal if selected. |
| [PA03](#pa03) | P:I02 | PA00 preserves encounter order and indexed type views → retain-or-correct decision, then any qualified structure/presentation reuse and removal. |
| [PA04](#pa04) | P:I03 | PA01's separable native preparation/collectors and TF00/TF01 failure contract → dependency/retention decisions and selected production route. |
| [PA05](#pa05) | P:I05 | PA00/PA01 consumer needs; flow input/cancellation ownership for token integration → per-surface decisions and selected narrow syntax/token/kind migrations. |

<a id="pa00"></a>

### PA00 — Exact correspondence and type lookup indexes

**Proposed required correction.** Prepare declaration-by-name-occurrence, parameter-by-declaration
and ordinal, formal-by-parameter, annotation-key and Arguments-by-parent/kind views at their actual
immutable module lifetime. Prefer native exact-key lookup where the locked API can answer the
same availability question; otherwise prepare one borrowed key inventory. Confirm whether a native
key API assumes existence or distinguishes missing answers before replacing the annotation scan.
Arguments attachment must reproduce the old complete-parent/kind uniqueness test, including
duplicate entries, missing parents, same-span wrappers and alternatives across syntax fields.

Inside the growing type builder, maintain indexes as rows are appended: first-retained term by
identity, member buckets by sequence and slots by list. Preserve existing sorting/deduplication
at the consuming boundary; avoid sorting a whole growing vector per request. No recursive type
construction or work counter changes belong in this package. Extend the same approach to native
class/binding lookups reached by these collectors where their current first/duplicate semantics
are established, without silently expanding into unrelated semantic fixes.

For `write_native_lexical`, build scope lookup in the current `rows.scopes` then `source.scopes`
precedence, event lookup in retained event order and source-target membership. Migrate support
emission and resolution callers. Preserve missing-owner errors and every native/local support
distinction. Reuse `ruff_lexical`'s existing indexed binding candidates rather than reintroducing
a second index or treating its current tests as newly implemented work.

Completion requires removal of the identified repeated whole-collection searches, explicit
duplicate/first/unique rules in code and focused independent expectations. Use a module with many
declarations, parameters, overloaded calls and bindings plus duplicate/missing/ambiguous candidate
buckets. Compare complete relevant typed records and support/coverage, not only row counts.
A direct small bucket test specifies expected refusal and order independently; the old scan can
serve as a temporary differential oracle but cannot be the only semantic expectation. Inspect that
each query examines its bucket and that admitted index allocation has module-scoped release.
No timing threshold or benchmark campaign is needed to establish removal of repeated examination.

<a id="pa01"></a>

### PA01 — One module-local Pysa preparation

**Proposed required correction.** Establish a lexical preparation scope around root extraction:
borrow the installed reporter's `module_ids`, create one `PysaResolver` and `ModuleAnswersContext`,
and collect captures once for definitions and calls. A closure or ordinary borrowed helper is
sufficient; avoid a self-referential stored object just to hold `ModuleContext` and its resolver.
Definitions retains override/MRO preparation it actually needs; calls retains its native graph
export and typed lowering. Reuse the same compatible context in `types` where native lifetimes
and lazy queries permit, without collecting captures or graphs solely for type consumers.

Migrate `session`, `definitions`, `calls`, `types` and the optional `PysaTap` so the diagnostic
tap consumes already requested products or explicitly owns its extra graph export. Preserve
typed lowering, lazy cross-module resolution, source-versus-synthesized members, native overload
identity, captures and full MRO completeness. The locked native reporter already composes shared
preparation in `pyrefly/lib/report/pysa.rs`; use its native calls without serialized production
report files. Sharing a resolver avoids duplicate preparation, but does not imply the old path
re-ran the full native checker.

Dependency modules instantiate only the preparation required by definitions. The dependency-wave
call site must not export calls or perform root-only type lowering to reuse a helper. Scope
preparation to its actual transaction epoch and drop it before the native state is released.
Failures and cancellation continue through the existing producer owner.

Completion retires separate definitions/calls capture preparation and migrates all call sites.
Focused controls cover nested captures, decorated/overloaded functions, synthesized versus source
callable fields, inherited MRO and definitions-only dependencies. Use the existing Pysa CLI parity
harness as one oracle for native definitions/calls; independently inspect typed capture/origin/
attachment/support records. A structural or local preparation observation establishes one capture
collection per root module and no dependency call-graph work; no permanent telemetry framework
is required. Include failure between consumers to expose premature release or borrowed escape.

<a id="pa02"></a>

### PA02 — Ruff preparation sharing decision and selected implementation

**Selected, 2026-10-17 ([ADR-0183](../adr/0183-collector-preparation-and-retention.md)):** share
immutable source preparation, preserving the independent empty-rule observer and both real Noqa
computations. `PreparedCanonicalSyntax` owns the admitted Locator/Stylist/Indexer lifetime and
ends after context rows. Native `observe_prepared` keeps the same validation, stop statistics and
checkpoints. Fusion remains rejected: observer exhaustion gates deferred/final work, while both
diagnostic channels must complete; detaching those semantics would require another traversal or
shadow state. Revisit if an existing native observer can stop recording independently of semantic
completion with exact statistics and cancellation. Coordinator §6 owns current evidence and disposition.

The investigation compared three concrete routes at the locked fork: retain
the current three Checker computations; share immutable preparation while keeping two diagnostic
computations; or collect observations during a compatible diagnostic traversal while preserving
the pair's full diagnostics. Source proof must identify which configuration/rule-dependent
semantic state is actually reusable and what must be recomputed.

The settling case is suppression-dependent construction, not merely final filtering. Read
`semantic_facts.rs::run`, `checkers/ast/mod.rs::check_ast_with_observation` and
`rules/pyflakes/rules/unused_import.rs` at `9080ee7a`. F401 partitions ignored/unused imports
before diagnostic construction. The production adapter's matching of enabled and disabled
diagnostic multisets remains authoritative; do not infer one channel by naïve filtering.
Observation stopping currently gates deferred/final semantic work. A shared route must detach
observer exhaustion from Checker completion without turning attempt cancellation into exhaustion.
TF01 owns the cancellation checkpoint; PA02 consumes and integrates it, with no duplicate
cancellation package or independently edited checkpoint policy.

Settle the choice with source inspection and focused database-free comparison where source
cannot prove equivalence: suppressed and mixed multi-imports, parent-line noqa, F821/F841,
recovered and unsupported syntax, and observation row/work refusal during deferred work.
Compare identity, message, primary/secondary spans, annotations, ordinal/channel, settings,
contextual completeness and diagnostic completion. No-observer parity under identical settings
cannot settle differing rule/suppression settings. Include attempt cancellation separately.

If qualified sharing reduces meaningful repeated preparation without extra retained semantic
copies or fork machinery that outweighs it, implement the selected native seam, migrate
`CanonicalSyntax`, diagnostics and contextual collection, update exact fork/patch attribution
through existing pin policy if required, and remove replaced preparation/traversal paths.
Otherwise retain the current route with the concrete failed equivalence or lifecycle premise
and its evidence. A retained decision closes this investigation only; it cannot close TF01.
An unspecified future probe or a partially wired optional shared path is not completion.

<a id="pa03"></a>

### PA03 — Type construction and presentation decision

**Selected, 2026-10-17 (ADR-0183):** reuse only the current Other/Truncated encounter's exact
display for presentation. Every recursive encounter, work increment, first registration, native
lookup and output effect remains. Broader memoization is unselected: final identity
does not encode depth/remaining work/variable state or native context, and structural interning
cannot avoid constructing its children. Native-type keys retain recursive graphs and still need
effect replay. Revisit with a pure bounded construction interface that exposes those effects and
complete context; no display-key cache or changed limit is scheduled. Coordinator §6 owns evidence.

After PA00, separate repeated lookup cost from repeated
term/list/presentation construction. Inspect `Builder::{term,build,sequence,slots,signature_variants}`
and call candidate lowering. Compare retaining encounter-driven construction, reusing only safe
presentation/structural products while replaying all required encounter effects, and module-local
native-type memoization. Content-addressed final rows are not by themselves a complete reuse key:
native origin, context, qualification, receiver/role, selected generic versus candidate type and
depth/work state can affect output, fidelity or associated evidence.

Settling evidence specifies the operation's observable effects and compares repeated and unique
types, recursive aliases, generics/overloads, defaults/restrictions and opaque descendants.
Exercise depths immediately around 32 and encounter work immediately around 100,000, including
a repeated subtree first seen shallow and later deep, and a threshold crossed between candidates.
Expect exact baseline truncation, presentations, boundary/support rows and closure fidelity under
provenance correspondence. A prebuilt baseline type graph cannot substitute for native/context
evidence about chosen and candidate types. Resource admission must cover any retained key/products.

Choose retain or the proven bounded reuse scope. If reuse cannot preserve every encounter and
its truncation effects without reconstructing the avoided work, retain current construction;
PA00's index correction remains useful. If a presentation-only route qualifies, implement that
route without implying recursive memoization is qualified. Migrate every selected producer and
consumer, remove replaced reconstruction and experimental dual paths, and retain focused limit
controls. No type-limit semantic change is selected by this plan; a proposal to change it must
reach the coordinator before dependent implementation.

<a id="pa04"></a>

### PA04 — Dependency selection and retained-state decision

**Selected, 2026-10-17 (ADR-0183):** select untapped dependency functions before heavy native
signature/decorator/capture projection using complete classes and borrowed base definitions. Keep
the original qualified-name/lexical-enclosure selection, full class/global/MRO context and complete
base capture eligibility; already referenced functions outside selected payloads remain eligible.
Roots and PysaTap retain full export. The two independent retained decisions are post-wave Everything
and progressive actual closure: generic solver hooks may follow AST eviction, and reporter callbacks
may repeat before a final epoch or after rebuild. Eager Everything for potential dependencies would
expand the live universe before actual demand. Revisit with native final-epoch, rich-data ownership
and cancellation/join guarantees, rather than adding a snapshot queue/scheduler. Coordinator §6
owns the immutable fork identities, focused controls and deferred persistent acceptance.

The investigation settled two related choices explicitly: can dependency
definitions avoid full report construction before their keep filter, and can retention/promotion
or pre-eviction extraction reduce the combined live set while preserving rich facts? Inspect
`session`'s selected dependency waves/keep expansion and `definitions` alongside the locked
`state/state.rs`, `state/require.rs`, Pysa context, reporter and capture collectors.

Current requirement promotion clears loaded state and rebuilds the module. Selected dependency
closure also grows from enclosing definitions, captures and MRO. Compare the existing progressive
waves with qualified selection before report construction and with earlier preparation of the
actually known closure. Do not assume the final required dependency set is statically known, or
replace progressive selection with Everything for every imported dependency. A selective export
must retain enclosing/capture/MRO closure, public-name tracing and definitions-only operation.

Separately compare current post-wave Everything retention with a native pre-eviction collector
that produces compact operation-owned results. The generic solutions hook follows possible AST
eviction; its existence is not sufficient. A candidate must establish final analysis epoch,
all needed AST/answers/traces, lazy cross-module query readiness and lifetime of heap-backed types
until encoding. Reporter callbacks may run in parallel and repeat after rebuild. They cannot
publish provisional/duplicate-epoch facts, block solver workers indefinitely on persistence,
lose deterministic emission or evade cancellation/join ownership. Keep the simplest post-wave
route if obtaining these guarantees recreates its work or adds unsupported lifecycle machinery.

Settling evidence includes a dependency first loaded at Exports then selected for definitions,
cyclic imports, nested/enclosing declarations and captures, inherited MRO across modules,
public re-exports, unavailable dependencies and an interruption during promotion/collection.
Compare complete selected facts, attachment, unresolved outcomes and finality. Source inspection
may settle retain; a replacement with changed phase/lifetime requires a focused executable
control showing stable-epoch output and ownership through refusal/cancellation. Explain the
composed working-set/preparation tradeoff; no latency/RSS claim follows without measurement.

Record retain-or-correct separately for selective definitions and retention timing. Implement
each selected correction through native collector inputs, dependency-wave consumers and typed
emission; remove replaced full-report/filter or retained-state routes after migrated consumers
are complete. If a selected seam needs a fork revision, route that exact change through pin and
patch owners, with affected parity controls. Errors-only collection, analyzer upgrade, a new
scheduler and persistent cache are not defaults or prerequisites. A material architecture/rule
change discovered here goes to the coordinator before dependent implementation.

<a id="pa05"></a>

### PA05 — Syntax copy, token and node-kind decisions

**Selected, 2026-10-17 (ADR-0183):** move declarations/parameters/formals/calls into charged
`RetainedSyntax`, borrowed by call/type consumers, and emit other syntax without a complete copy.
Use `cpg_flow::syntax_kind` as the shared exhaustive latest-family mapping; remove flow's debug-name
formatting and code scan. Keep recovery-aware token parsing: the existing canonical token lifetime
ends in the prior provider phase, so there is no borrowed input to hand off. Different parser/source
settings are correspondence questions, not proof that token streams differ. Revisit when a named
consumer already retains canonical original tokens through flow; do not add a persisted token dataset
or extend the whole canonical AST solely for this reuse. Coordinator §6 owns current evidence.

The investigation makes three explicit decisions
because the equivalence premises differ:

1. **Syntax ownership.** Inventory every consumed field after `write_records` and compare
   borrowing until emission, splitting ownership into actually needed subsets, and retaining
   copies that release larger buffers or simplify a real lifetime. Migrate `types`, `calls`,
   declaration linking and parameter consumers together. Remove the full `records.clone()` and
   any duplicate call storage only where the selected ownership route preserves actual needs.
   Charge retained subsets and keep them through the last consumer. Verify declarations, formals,
   fields, calls and recovered-source behavior with explicit consumer expectations.
2. **Original token reuse.** Compare current parser-mediated `cpg_flow::rename` with consuming
   canonical original tokens. Equality requires exact bytes, source kind, target version and
   parser recovery; canonical context-configured parsing and flow's current Python/default parse
   must not be presumed identical. Test malformed interpolations, comments/strings, sentinels,
   `.py/.pyi`, supported-version syntax and same-length offsets. Bare lexing is already known
   to differ on malformed interpolation recovery and is not a replacement. Transformed bytes
   still require their semantic parse. Implement a borrowed token seam only if exact premises
   and ownership survive flow preparation; otherwise retain the current parse with the specific
   mismatch recorded. Do not add a persisted token dataset or generic parsing framework.
3. **Node kinds.** Compare flow's debug-name/code scan with an explicit typed mapping at the
   exact latest-family boundary. Reuse or place a shared mapping only where crate dependency
   direction and nominal types permit it; do not make `cpg-flow` depend on `cpg-extract` or cast
   Pyrefly's embedded Ruff types into canonical types. Preserve supported shapes, unsupported
   outcomes, scalar wrappers, spans and roles. Qualify mapping coverage independently against
   the native enum/model declaration, then migrate the flow consumer and remove string formatting
   and range-search conversion if selected. No codebook reordering or new semantic kind is implied.

Each decision includes concrete settling evidence and either the migrated implementation/removal
or a justified retain outcome. Coordinate token/node consumers with flow input ownership and
TF00/TF01; do not combine allocation/cancellation rewrites into this package. Parser upgrade or
a new source interpretation would require coordinator disposition rather than silent adoption.

## 5. Verification, migration and completion

**Planned, not_run during plan authoring.** During implementation use compile checks on touched
crates and the smallest revealing contract controls. Pure lookup/type-construction/native-Ruff
controls need no database. Native producer-to-typed-output checks use the existing fixture owner;
do not create a scratch service, install/restart a shared service or inspect operator state.
Readiness failures are `blocked` with the missing prerequisite and repair route, never passes.

The current provider boundary is `providers:extract` in `scripts/verify.py`. For selected native
controls, use `just verify --select providers:extract --nextest-args '-E …'` with an explicit
filter resolved from the actual target, or `just fixture -- cargo nextest run --release -p
cpg-extract …`. Relevant existing targets are `extraction_types`, `extraction_calls`,
`extraction_syntax`, `extraction_contracts`, and library unit tests; inspect grouping before
choosing a filter. `extraction_types` contains structural-type and native-generic controls;
`extraction_calls` contains native callable-variant and overload-origin controls;
`extraction_contracts` contains the Pysa harness. Overload-checker and expected-argument parity
also have separate test targets. The managed provider boundary deliberately narrows some grouped
targets, so use an explicit bare fixture command for a needed case outside its mandatory filter;
adding a nextest filter cannot widen that boundary. For example, a selected structural-type
case can run through `just fixture -- cargo nextest run --release -p cpg-extract --test
extraction_types -E 'test(native_types_keep_structural_terms_record_flags_and_declared_async_return)'`.
Selected cpg-flow checks can use `just verify --select providers:flow --nextest-args '-E …'`.
New cases belong with the owning contract rather than a new efficiency acceptance runner.

At functional scope end, the integrator runs full workspace compile and strict Clippy, plus
applicable Python hygiene once: `cargo check --release --workspace --all-targets`, `just clippy`
(`cargo clippy --release --workspace --all-targets --keep-going -- -D warnings`), and `just ruff`
and `just types` when Python surfaces or their policy require them. Follow the coordinator's
current command/profile selection if it advances; no job/thread caps or wholesale dependency
resolution. These are scope-end non-functional checks, not full product acceptance. The root
owns applicable docs checks and final `just turn-end --paths …` over its owned paths.

Package completion requires migrated production callers, removal of replaced paths and only
their obsolete fixtures/helpers, focused failure/equivalence evidence and updated architectural
owners where enduring meaning or workflow changes. A retain decision removes abandoned candidate
wiring and describes the concrete evidence at the existing owner. It must leave no optional
experimental alternate producer masquerading as completion. New source identities are real;
derived outputs are rebuilt only when their named consumer is authorized. Existing operator
adoption, protected runtime captures and storage obligations are unaffected by this plan's
document creation or focused controls.

The coordinator closes P:F01/P:F04 only after all affected consumers and revealing controls are
complete. Investigation completion is a settled decision plus its selected implementation and
removal, not a claim that the broader performance opportunity disappeared. Whole Catalog,
selected-complete, assembled acceptance and measured benefit remain separate later boundaries.
Report each actual command as `passed`, `failed`, `blocked` or `not_run`, with date and scope;
retain all independent failures rather than weakening checks or claiming a whole-plan pass.

## 6. Rule changes and current authoring boundary

**Rule changes selected: none.** P:F01/P:F04 preserve existing rules. PA02–PA05 keep exact
fact semantics, diagnostic channels, source roles and current type limits as constraints.
Selecting a different type-bound meaning, requirement/completion architecture or source
interpretation would require presenting its concrete consequence to the coordinator for the
operator's explicit decision and existing ADR/owner route before dependent work.

**Authoring evidence, 2026-10-17:** repository/fork source and current owners inspected;
create-plan, design-review/profile, efficient-architecture guidance and matching analyzer skill
consulted. No production implementation, compile/test/probe, service operation, formatter or
benchmark ran in this delegated authoring scope. All implementation packages and predicted
benefits remain Proposed. Refer to the coordinator §6 for the current checkpoint and sole
finding disposition; this section records only this document's evidence boundary.
