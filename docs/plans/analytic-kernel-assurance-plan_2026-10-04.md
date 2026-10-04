# Independent assurance for bounded FCA and implication analysis

**Implemented and focused-Tested K1/K2, 2026-10-04.** Supporting design for O08 in the
[coordinator](code-facts-analytical-enrichment-plan_2026-10-04.md); §7 owns current receipts
and disposition. `dbabbe6904a3` is the authoring baseline. The exact development-only odis/bit-set
integration and four independent finite-context controls passed. The production charged kernel
is retained. This establishes the recorded oracle/control scope, not current full-series Q1
acceptance, performance measurement or a production-library substitution.

## 1. Selected library use and production fit

Retain the production finite FCA/implication kernel and its model outcome. Add **odis =2026.9.1
as a development-only independent oracle** for implication consequences and iceberg behavior.
Existing fcars concept checks are useful but do not independently establish the production
implication basis's consequences; a hand-written example can reproduce a shared interpretation bug.
Odis's canonical basis supplies a distinct algorithmic comparison on exhaustively bounded contexts.

The choice is technical. Production `Context::analyse(min_support, max_examined)` charges context,
scratch, basis and results, counts examined concepts/pseudo-intents and retains sound partial results
with explicit outcome. Odis's batch CanonicalBasis/Iceberg paths have no equivalent internal
work/allocation/cancellation limit. NextClosure computes the next closure before yielding the current
one; capping returned rows is not capping examined work. SearchBudget applies to drawing, not this
analysis. Post-hoc charging or a file-size cap would weaken the current attempt contract. Therefore
production substitution is **not selected**, and completing this plan does not wait for a benchmark
or broad rewrite of odis. Reopen only if a precharged step/cancellation/support API meets that
contract, or an explicit ADR deliberately changes the required contract.

Use pinned source/primary documentation, including the
[odis implication engine interface](https://docs.rs/odis/2026.9.1/odis/traits/implication_engine/trait.ImplicationEngine.html).
No license is a rejection reason, and `just deps` does not check licences (ADR-0125), so AGPL-3.0
does not block the chosen dev dependency.

## 2. K1 — Integrate the independent finite-context oracle

Own the harness in `crates/lctx-analytics/tests`; root owns shared manifests/lock/pins/policy changes.
Add exact workspace versions (oracle pins with a pins row) `odis = "=2026.9.1"` and `bit-set = "=0.8.0"`, referenced only from
the analytics dev-dependency surface. bit-set 0.8.0 already exists in the current lock and supplies
the public type used by this odis release. Existing fcars remains for its independent concept route.
Qualify odis's full transitive dependency/policy burden, including reqwest/rust-sugiyama despite this
adapter not using network/drawing. Do not add network calls to tests or pull odis into production
capability closure. Update pins with its oracle purpose; end-of-turn hooks own manifest generators.

Convert production nominal object/attribute IDs to deterministic sorted usize bijections for each
case. Build `FormalContext<usize>` through public constructors: add every attribute with empty
BitSet first, then objects with their incidences. Direct incidence-field mutation would leave private
caches stale. Preserve zero-incidence objects/attributes, duplicate incidence patterns and empty
universes; the conversion must not drop them because they lack edges.

Normalize output to semantic sets. Where one oracle emits a full closure as conclusion, remove the
premise before comparing implication pairs. Do not demand identical basis order/representation;
bases can differ syntactically while generating the same closure. Independently compute finite
incidence double derivation as a third check, using the input matrix rather than the production
kernel's closure helper. The test matrix specification owns expected incidence, not either adapter.

Fixed test bounds: at most eight attributes and twelve objects, finite fixed-seed cases and explicit
case count. This bounds the unbudgeted development oracle; it does not establish production resource
qualification. Use small exhaustive attribute-subset enumeration. No large random/corpus campaign
is required for completion.

## 3. K2 — Check consequences, support and partial outcomes

For completed min_support=0 runs, close **every** attribute subset under the production basis and
odis basis and compare both to direct incidence double derivation. Check each retained implication
is sound and reports correct support. Compare concept extents/intents as sets through the existing
independent concept route. Shuffling input or nominal ID assignment cannot change semantic sets.

For positive support thresholds 1, 2, |G| and |G|+1, compare concepts with an independently enumerated
full concept set filtered by extent size. For implication consequences, restrict the starting subsets
to those whose support meets the threshold and compare their closure to full independent closure.
Filtering the full basis by each implication's support is not an iceberg-basis oracle; an attribute
subset below threshold is outside that completeness claim. Retain each algorithm's actual supported
contract and diagnose a mismatch rather than papering it over with equal output counts.

Required cases: no objects, no attributes, empty context, singleton, full/no incidence, duplicate
rows, shuffled rows/columns and common attributes requiring empty-premise implications. A revealing
matrix with object intents `{a,b}` and `{b}` entails a→b and rejects b→a. Deliberately delete a
necessary valid implication or insert an invalid one and confirm the checker detects altered
consequences/soundness; flip one incidence and independently update the expected result. The
oracle harness must fail on these mutations rather than merely reproduce stored golden output.

Partial max_examined=0/small-cap runs require sound retained results and correct bounds/status,
**not** complete-basis equality. Exercise reservation-denial errors and the explicit examined-item cap,
including its initial extent evaluation before the cap check. The current API has no generic
work-budget/cancellation parameter; do not assert one. Verify memory reservations while retained
context/lattice/results live and release to zero after drop. Failed,
partial and complete outcomes remain distinct. No oracle success can promote a partial production
run to complete. If these controls expose an actual kernel defect, repair only that algorithm/owner
and its focused regression, preserving nominal output semantics and model replay.

The production model/context adapter and catalog/S0 implication text retain “in the finite extracted
context,” exact context/attribute IDs, support, configured bounds and partial-result language.
Do not convert FCA association into behavioral/runtime implication or change persisted implication
identity merely to match another basis representation. Odis objects/indices remain test-local.

## 4. Verification, deletion and completion

Scoped release compile/tests cover the oracle adapter, independent consequences, positive support,
mutation controls and production resource outcomes. Example route: release `lctx-analytics` FCA tests
with the new oracle target; use the precise test name once implemented. Model tests verify retained
partial/outcome/catalog meaning if touched. No production data/schema migration or new FCA engine
selection knob is expected. Delete hand-maintained test copies only when the new independently
specified oracle replaces their actual coverage; do not delete domain-specific admission/charging.

Move the enduring fit rationale to analytics/pins guidance with accurate evidence labels. Historical
ADRs remain immutable. No speed, memory scaling or maintenance reduction is claimed without a
separate measured baseline. Coordinator Q1 owns final family/deny checks through `just hygiene`
and same-tree `just test-all` after functional scope, not a full gate after this independent slice.
Completion means stronger independent assurance with the bounded production contract retained;
production library substitution remains an explicit technical non-fit, not an unresolved blocker.
