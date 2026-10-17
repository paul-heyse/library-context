# Normal checking versus complete fact collection

**Interface-checked source comparison, 2026-10-17; no new runtime measurements.** Supports the
[principal review](../../reviews/design_review_python-fact-collection-efficiency-target_2026-10-17.md).
The [evidence README](README.md) pins repository, fork, parent and latest-upstream identities.
This document qualifies capabilities and alternatives; the principal review owns the judgment.

## 1. What normal Pyrefly teaches, and what it does not establish

Pyrefly's [upstream architecture](https://github.com/facebook/pyrefly/blob/4e602d5f490e7723392904efbe606d1b559d311c/ARCHITECTURE.md)
organizes analysis around module exports, bindings and solutions, with module-level reuse and
parallelism. It explicitly anticipates cyclic module relationships. Replacing that established
engine with a project-owned graph scheduler is not needed to obtain these capabilities.

At both our fork and inspected main, ordinary check requests Errors for specified modules and
normally Exports for imported modules. Debug/trace/Glean reporting raises retention; Pysa reporting
also raises dependency requirements. Current main's
[check command](https://github.com/facebook/pyrefly/blob/4e602d5f490e7723392904efbe606d1b559d311c/pyrefly/lib/commands/check.rs#L1762)
shows this distinction. Errors does not preserve the same AST, answers and answer traces as
Everything. Consequently a fast diagnostic check is not a measurement of our richer output.

Our `pyrefly_stage::session` creates one State and cancellable transaction per captured input,
runs all admitted root handles on a shared available-CPU pool, then extracts in source-path
order. The transaction's default remains Exports; roots and explicitly selected later dependency
waves request Everything. There is no fresh full checker per source file and no blanket
Everything default for all imports. The State's Inline executor governs later caller-owned
queries, not the already joined parallel analysis wave.

However, raising a previously loaded dependency's requirement causes a rebuild: exact fork
`state/state.rs:1332–1338` clears the load and calls `rebuild(true)` for a dirty requirement.
Later selected dependencies can therefore be reprocessed. `definitions()` also constructs the
whole native module report before applying its keep filter. These are source-established work
mechanisms, not proof that dependency promotion dominates elapsed time. Native completeness,
cross-module MRO and imported origins may require substantial context even for a small output.

## 2. Reuse native collection preparation

The exact fork's `PysaReporter::report_module` constructs one `PysaResolver`,
`ModuleAnswersContext` and captured-variable collection before exporting definitions and calls.
Current [upstream reporter](https://github.com/facebook/pyrefly/blob/4e602d5f490e7723392904efbe606d1b559d311c/pyrefly/lib/report/pysa.rs#L282)
uses the same composition pattern. Our root definitions and calls each construct these products;
types constructs another context/resolver. Sharing compatible preparation within one exact
module and transaction can retain the native semantics without a new persistent cache.

`PysaResolver` owns a local cross-module solution cache. The transaction retains native
`Arc<PysaSolutions>` after demand; recreating the resolver does not prove repeated full inference.
`ModuleAnswersContext::create` clones native shared handles and requires retained AST and answers.
Avoid describing all context construction as a whole-module analysis.

Our installed reporter has `write_files=false` and returns before its report collectors run.
The adapter's explicit collectors are therefore not duplicates of an automatic full report export.
The optional Pysa parity tap does export another call graph; harness timings with that tap do not
describe ordinary production collection.

## 3. Earlier extraction is a qualified alternative

Both the pinned source and [current State](https://github.com/facebook/pyrefly/blob/4e602d5f490e7723392904efbe606d1b559d311c/pyrefly/lib/state/state.rs#L1653)
offer solutions-phase observation. Native reporters execute before AST eviction. The generic
solutions hook runs after the conditional AST eviction and before answer eviction; its signature
alone does not supply our full AST-dependent collection contract. `ModuleAnswersContext::create`
requires both. Simply changing Everything to Errors or moving every collector into that hook
would not preserve facts.

A viable earlier-collection design must establish which native state is needed, its final
analysis epoch, cross-module readiness, exact canonical attachment, and ownership of Types and
their heap-backed dependencies until encoding finishes. Callbacks may run during parallel
solving and repeat after rebuilding. They must not publish provisional or duplicate-epoch facts,
block solver workers indefinitely on native ingestion, or lose cancellation/failure outcomes.
Compact operation-owned products and deterministic later emission are alternatives to retaining
all opaque analysis state; they need a measured or reasoned total-lifecycle advantage over the
current simpler post-wave route. No particular callback/queue design is accepted by this review.

`PysaSolutions` itself is not our complete fact inventory: retained answers, native overload and
expected-type traces, class metadata and exact runtime observations have separate consumers.
`FlatSymbols`, present in both locked and current source, preserves source declaration sites and
parents; it does not replace typed signatures, call graphs, canonical syntax or runtime flow.

## 4. Current upstream is a comparator, not a drop-in replacement

Latest main retains the relevant requirement, resolver and reporter mechanisms; their existence
does not establish a new upgrade-only speed improvement. Main lacks our non-writing reporter flag
and transaction reporter getter. Our fork also exposes native observations and overload identities
consumed by the adapter. Any upgrade must preserve or replace each required observational seam
and qualify changed semantics, not merely compile the same entry point names.

The exact parent-to-fork delta is modest in source volume but includes state, solver and overload
code. Range-based native observation methods scan binding keys per queried call/context item
(locked `state/state.rs:2347–2355` and `alt/solve.rs:1941–1947`);
small patches can still introduce repeated work. Inspect the invoked code before concluding that
the fork is either the performance cause or operationally identical to normal checking.

## 5. Ruff and ty: share work only across equivalent contracts

Canonical Ruff parses are already reused across consumers. Selected diagnostic collection invokes
the full checker with Noqa enabled and disabled; contextual semantic observation invokes it again
with different settings. The pinned checker exposes an observation-enabled traversal, so compatible
preparation and successful traversal sharing are credible. But enabling selected lint rules can
affect checker work, and observer stop currently gates deferred/final semantic passes. A fused
collector must preserve both diagnostic channels and diagnostic completeness independently of
observational stopping, and report contextual incompleteness explicitly when observation stops.
Existing no-observer lint parity does not establish all those equivalences.

Normal ty project checking parallelizes file work and explicitly clears eligible parsed ASTs.
The flow adapter instead prepares one virtual file universe and queries modules serially within
one same-revision database. Its exported-name queries can avoid full semantic indexing, while
wildcard/import reasoning still needs the complete declared source universe. Parallel queries,
earlier AST release and compact results are usable capability directions, not permission to
remove cross-file context or change callback failure/order behavior.

The runtime view deliberately uses parser-mediated token recovery before same-length name
renaming. Bare lexing is not equivalent for malformed interpolations. Reusing original tokens
requires matching source type, target version, recovery and source identity; transformed text
still needs its own semantic parse when bytes change.

The pinned observation/cancellation facilities should be composed with the real producer owner.
Ruff exposes checkpoints; Salsa has nonblocking cancellation tokens, whereas a blocking
cancellation trigger has different synchronization consequences. These mechanisms do not promise
instant interruption of every parser or cycle-recovery region. A correction must state what
work can still drain before cancellation is observed and preserve joined finality.

## 6. Further source mechanisms consumed by the review

These observations retain the breadth of the bounded inquiries; their current judgment and
investigation disposition remain in the principal review.

| Mechanism | Baseline source and implication | Preservation boundary |
|---|---|---|
| Canonical Arguments lookup | `type_records.rs:1748–1757` scans all canonical nodes for each call although `syntax_records::Spans` already retains parent/child maps | Unique child, kind/role, source identity and ambiguity; no first-span shortcut |
| Parameters and native class identity | `type_records.rs:1574–1604`, `1911–1922` repeatedly search parameter/formal/annotation or class-binding inventories | Native definition identity, duplicate/functional-class outcomes and exact parameter ordinal |
| Type structure before deduplication | `type_records.rs:334–405`, `541–710`, `892–925`, `1760–1840` reconstruct terms, lists, displays and candidate types; `domain/batching.rs:74–95` deduplicates only after validation/hash | Truncation depends on depth and encounter work; chosen generic typing is not interchangeable with a nongeneric candidate |
| Native lexical correspondence | `ruff_lexical.rs:244,309`, `pyrefly_stage.rs:3058–3091` search scope/event/target collections repeatedly | Preserve each current first/unique/ambiguous interpretation and native versus local lexical meaning |
| Flow correspondence | `ty_flow.rs:774–804` scans all raw nodes per attachment; `cpg-flow/src/predicate.rs:79–120` walks the module for pattern cases | Complete same-span candidate shapes, canonical attacher authority and case identity |
| Node-kind conversion | `cpg-flow/src/lib.rs:572–575` formats native debug names and searches model codes, while canonical extraction has a typed mapping (`typed_syntax.rs:768`) | Nominal version boundary and exhaustive supported-kind correspondence, not layout assumptions |
| Flow canonical preparation | `ty_flow.rs:138–184` hydrates relations including Occurrences; `bundle.rs:428–440` reads Occurrences again for attachment | One shared preparation can serve distinct operations without merging independent admission or dropping referenced target closure |
| Raw flow expansion | `cpg-flow/src/lib.rs:304–334`, `507–542`, `931–1042`, `1162–1197` own complete raw vectors, temporary binding lists and cloned formulas before `ty_flow::Writer::write` | Apply attempt admission before owned growth; retain every candidate, ordinal, loop expansion, approximation and operational failure distinction |

The raw-flow issue concerns our adapter's containers, not a promise to account every opaque
third-party analyzer allocation or predict host RSS. Per-condition and source-size limits do
not establish a combined byte bound on all candidate/formula copies. Merely moving resource
refusal earlier is not sufficient to establish a credible complete collection route.

Existing controls were inspected as future closure routes, not executed: Pysa harness parity,
typed type/overload cases, Ruff context/diagnostic tests, flow attachment and Bound-unattached
inventories, parser-recovery controls and callback-order/limit controls. Their separate meanings
matter. A harness comparing Pysa reports does not certify every typed type/support/coverage row;
a custom cancellation sink does not establish that production connects cancellation; and observer
neutrality with identical lint settings does not establish equivalence across the different
empty-rule/selected-rule and suppression configurations used here.
