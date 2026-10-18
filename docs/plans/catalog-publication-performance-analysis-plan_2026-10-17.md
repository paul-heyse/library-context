# Catalog publication performance: analysis products and preparation

**Approved implementation plan, 2026-10-17.** Companion to the [coordinator](catalog-publication-performance-plan_2026-10-17.md),
the sole owner of shared decisions, dependencies and current dispositions. This document covers
F02/F03, I03 and conditional candidate/SCC scheduling improvements. Compiler integration is
developed in the [compiler companion](catalog-publication-performance-compiler-plan_2026-10-17.md).
Existing plans remain unchanged; no replacement algorithm, schema or performance result is
implemented by this document.

## 1. Baseline and selected target

Baseline `789223017833c364085ef62f9333df082d88444a` retains the source in the
[review](../design_review/reviews/design_review_catalog-publication-performance-target_2026-10-17.md)
and [analysis/emission inquiry](../design_review/evidence/2026-10-17_catalog-publication-performance/analysis-and-emission.md).
The model already owns qualified transfers, finite witnesses, questions, standings, residuals,
conditions and actual invocation topology. Petgraph SCCs and the BDD condition kernel fit
their semantics. The target improves preparation and result detail, preserving those owners.

Select **model-owned finite question and coverage inputs, with authentic proof evidence kept
separately**, instead of making every attempted proof pair a mandatory persisted intermediate.
Use the existing CallClosure claim/member/standing products where sufficient. A new concept
does not automatically require another stored relation or discovery-witness family. The ordinary
solver initially visits the same canonical candidates and charges the same logical work; removing
the trace need not first change the bounded prefix or proof algorithm.

Lossless pair-trace factorization is a fallback only if PA00 identifies a real supported consumer
requiring exact attempted-pair enumeration. A current all-family replay assertion is a consumer
to migrate, not by itself a product requirement. Do not keep a permanent second production
trace route, historical reader or compatibility store.

## 2. PA00 — Settle I03's question/member evidence contract

**Consumes:** PP00's RC02 decision route and current model reductions/consumers. **Delivers:**
the exact question/member/coverage contract and independently specified revealing cases;
PA01's production cutover waits for it. Final recursive cardinality is not a prerequisite.

`summary_consequences.rs:918–945` currently discovers questions from all PairOutcome variants
and residuals. Removing Disjoint/Subsumed records without replacing discovery would erase
questions. Define one model-owned operation admitting a finite question from eligible actual
operands, using the same eligibility authority as consequence construction. It coalesces
repeated discoveries immediately into a canonical set. The solver does not independently
reclassify applicability from a convenient subset of successful witnesses.

Preserve the current scopes explicitly:

- The invocation-relative question key is `(event,input,output,qualification)`. Pair-derived
  questions use the **caller's qualification**, not the composed witness qualification.
- Refused pair causes apply by `(attempt,input,output)` without qualification, and affect each
  matching qualified question according to current precedence.
- Residuals apply by `(event,input,output)` to matching members. A Proven witness contributes
  only when its complete qualification equals the question qualification.
- Actual call-member inventory, excluded/unknown applicability, binding/coverage authorities,
  origin boundaries and complete-empty membership remain explicit. Missing proof is not absence.

Consequence production consumes this domain, actual member inventory, admitted finite witness
DAGs, member refusals/residuals and authentic qualifications. Preserve existing questions,
standings, verdicts and required evidence meaning. RC02 permits revising trace realization;
it does not implicitly approve narrower refusal scope or altered answer semantics.

**Bounded remaining decision:** determine the smallest sufficient member-evidence representation.
Trace every Refused construction to its required residual/support and every explanation/admission
consumer. Use a small pure comparison over representative operand inventories to establish whether
existing witnesses/residuals suffice. If they do, use them directly. If they do not, define one
compact member-evidence relation with the missing nominal scope/reason/support, without retaining
one row per rejected pair. Identify its actual consumer and version its model/schema identity.
Use private in-memory preparation when no persisted consumer needs the relation.

Independently reconstruct the domain from eligible finite operands for model admission where
required, instead of trusting a producer's claimed question list. Source-specific known answers
challenge eligibility and scope. Existing same-kernel replay is valuable input/adapter evidence,
not an independent algorithmic oracle.

**Decision completion:** record the chosen representation, precise eligibility owner, consumer
mapping, schema/semantic revision and preserved scopes in the coordinator's decision explanation.
If changing observable question/qualification/refusal meaning is necessary, expose the additional
rule impact before dependent implementation. Do not silently widen this approval.

## 3. PA01 — Cut producers, consequences and consumers over from mandatory pair traces

**Consumes:** PA00's settled contract and working model declarations; existing output/native
sinks are sufficient initially. **Delivers:** production Summary with the new semantic inputs,
migrated consumers and removal of the obsolete mandatory PairOutcome path.

At the same canonical candidate visitation points, update the model-owned domain, refusal and
proof products directly. Keep distinct Disjoint/Subsumed/Refused/Proven computational outcomes
where they govern work/progress, without materializing an independently owned database row
for every outcome merely to discover a question. Residual-only questions enter through the
same domain authority. Keep all finite witnesses needed by aggregation, proof origins, costs,
control influence, guard substitution, S0 support and native path explanation.

Migrate `summary_consequences`, semantic admission, synthesis/native explanations where affected,
ordinary prepared replay, full-family comparisons, output declarations/codecs/schema snapshots
and source-specific test queries. Preserve distinct equal-cost witnesses and exact binding/
substitution premises. Trace-only serialization and unused premises disappear when their last
consumer moves. Codebook identities stay append-only; remove obsolete declarations without
renumbering unrelated codes. Persisted schema changes use explicit revision/regeneration.

Keep candidate discovery, canonical queue insertion/pop accounting, representative admission,
callee-first SCC sequence, frame-wide member bound and shared-callee progress unchanged in this
first cutover. A smaller output relation does not justify charging less logical work implicitly.
A temporary small test-local reference projection of the old reduction may challenge the new
contract; no legacy production format/reader survives the cutover.

**Focused acceptance:** duplicate discoveries yield one question; Disjoint-only/Subsumed-only
questions remain; Refused plus Proven evidence retains current open/precedence behavior; two
qualifications sharing endpoints preserve qualification-independent refusal and exact proof matching;
residual-only, unknown and excluded members; equal-cost witness explanation; recursive guards,
base-free cycles, shared callees and member exhaustion. Compare question/member inventories,
standings, conclusions, residuals and retained proof closure against independent expected cases.
Check unchanged logical work counts and bounded-prefix results. Revise only trace/schema
expectations deliberately changed by this contract, never behavioral assertions merely to pass.

## 4. PA02 — Share Summary composition preparation and indexed correspondence

**Consumes:** existing protected frame/vocabulary/context and PP00's SC01/SC02. **Delivers:**
actual composition/publication callers without repeated whole-map preparation. This work is
independent of PA01 except for coordinated edits to shared result structures.

`domain/composition.rs` traverses complete maps for allowance and clones complete segment/literal
maps for small projection overlays. Retain established immutable backing-size properties at
their valid frame lifetime. Implement an immutable base plus bounded owned overlay for newly
derived projection data, with charged lookup/iteration and complete exact identity semantics.
New literal/segment IDs needed before persistence remain available to the composer.

Separate guard semantic restatement from evidence construction under the final qualification.
The two current substitutions use different qualification context; reuse only the identical
semantic calculation, then build correctly qualified evidence. Do not cache by textual condition
or endpoints alone. Preserve distinct invocation evaluation atoms and opaque guards.

Prepare component/branch incidence, proof-source correspondence and origin-boundary indexes once
at their true consumer lifetime. Avoid global branch scans per SCC, linear member searches per
proof source and complete origin-boundary scans per closure question. Release component-local
backing after its final SCC/pair consumer; keep shared-calculated topology and lazy ports/signatures.

**Focused acceptance:** keyword/vararg projection IDs, complete condition/qualification differences,
distinct opaque invocations, competing source mappings, stale backing/context, resource refusal
and actual reservation return. Existing finite proof DAG/cost validation stays active. No new
solver, graph closure substitution or proof-pruning policy is implied.

## 5. PA03 — Reuse immutable Completion preparation across statements

**Consumes:** actual admitted body/window syntax and authentic earlier evaluation correspondence.
**Delivers:** shared immutable preparation in the real Completion production callers, with fresh
per-statement requests, proof state and checked owner/context.

`completion_production` currently invokes `completion::complete` per statement, reconstructing
PreparedExecution/PreparedSyntax, then body completion prepares again. Prepare syntax/indexes
once per compatible admitted body/window and index actual earlier-evaluation correspondence.
Use the existing prepared completion interface and `PreparedExecution::shared` or an equivalent
owned extension. Migrate statement and body consumers together. Sharing a nominal body ID is
insufficient when selected backing, context or actual evaluation owner differs.

Retain each statement's selection validation, ambiguous native placement behavior, input and
context identity, actual proof tokens and fresh mutable completion state. Canonical suite order
and stop-after-first-nonnormal are semantic dependencies. Base already shares syntax per compute
cohort; Enriched already shares frame preparation. Preserve those positive patterns instead of
adding a second preparation cache. Append-only Enriched context rounds remain premise-driven.

**Focused acceptance:** a multi-statement body, competing placement premises, distinct contexts,
missing actual evaluation correspondence, early non-normal completion and failure/cancellation.
Demonstrate shared construction in actual callers while independent statement proofs remain
correct and all backing/charges outlive their borrowers.

## 6. PA04 — Resolve candidate-selection and ready-SCC execution opportunities

**Consumes:** explicit question/budget contracts from PA00/PA01 for changes that depend on them;
existing SCC/topology contracts suffice for source assessment. **Delivers:** a bounded decision
and implementation of compatible useful work reduction/overlap, or a concrete necessary-order
counterexample with that ordering retained. It is not an indefinite profiling backlog.

Include the actual Local grain loop, Base-expression loop, SourceCall event grains and Enriched
owner/frame grains, as well as candidate/SCC work. For each, implement compatible independent
overlap or record the concrete ordering, shared semantic state or resource-lifetime reason
requiring its order. Distinguish these grain loops from genuinely sequential body completion
and premise-growing context rounds. A generic new parallel helper alone does not complete
their assessment or migrate their callers.

Early exact compatibility selection can avoid rich composition, but skipping queue candidates
also changes work consumption and which later proof fits a limit. Prefer preserving canonical
logical accounting for skipped combinations while avoiding unnecessary rich construction. If
that route is not straightforward or useful, retain canonical visitation and remove trace costs
through PA01. A deliberate new budget policy is a separate semantic decision, not preapproved
by RC02/RC03. Keep finite-limit residuals and actual accepted-progress semantics truthful.

For kernel parallelism, identify genuinely independent ready condensation components or private
body work. Distinguish immutable preparation/private compute from canonical admission of global
member allowances, vocabulary growth, proof order and result state. Use actual owned jobs and
budget reservations with deterministic integration. Do not add a new executor, arrival-order
semantics or thread caps. If a component has real predecessor/premise dependencies, retain them.
PC03 independently removes all-window delivery barriers; it does not create kernel parallelism
inside the current one-frame recursive fixture.

**Selected implementation, 2026-10-17 (source implemented; focused integration pending):** Local
and SourceCall private grain preparation use actual registered jobs with shared immutable selected
inputs; results, meters, append and proof integration remain canonical. Base expression compute
already uses owned independent jobs with shared syntax; adopting its shrinking selected support
state remains ordered. Completion retains semantic statement order and stop-after-first-nonnormal.
Enriched owner/frame rounds consume and extend actual premise state, so their dependent rounds
remain ordered. Summary's single recursive SCC shares work, vocabulary and frame-wide member
allowances, including shared-callee progress across components. Concurrent mutation or skipping
canonical queue visits would change which bounded proof fits; retain that order and remove pair
trace/preparation cost instead. Independent outer work delivers ready canonical prefixes through
PC03; this does not claim parallelism inside the recursive kernel or a new semantic budget policy.

**Focused acceptance:** independent components, a single recursive SCC, reconvergence/shared
callees, shuffled completion order, global member/work exhaustion and failure after a private
result is ready. A parallel route must preserve required bounded answers and resource/failure
ownership, not just unlimited transfer equality. If a new semantic budget would be necessary,
record that unsupported route and keep current semantics pending a separate operator decision.

## 7. Completion boundary

PA00/PA01 settle model meaning and migrate actual consumers; PA02/PA03 remove repeated preparation
without depending on native format replacement. PA04 has a required decision outcome, not a
required concurrency mechanism regardless of semantics. All remaining introduced choices have
an explicit owner, settling evidence and blocked dependent work.

PA01 can initially use current typed output; PC02 then consumes the new canonical products and
PN02/PN04 optimize their installation. None takes over Summary meaning. PP90 verifies revealing
functional cases and hygiene; PP99 retains the uncompleted full recursive/publication validation
and unknown final disposition/cardinality observations. A focused pass is not a measured speedup
or completion of any older plan's recursive acceptance.
