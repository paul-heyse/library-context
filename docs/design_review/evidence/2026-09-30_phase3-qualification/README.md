# Phase 3 qualification

Consumer: the Phase 3 detailed plan §11/Q and its assembled exit review. Evidence, not authority.
**Implementation and composite gate passed, 2026-09-30.** Full-library output comparison and
remaining measurements are **not_run at user direction**. `raw/` retains command outputs via Git LFS.

This is a **composite qualification**. The first `just fmt` passed. Initial `just test-all`
attempts stopped on local Clippy issues; those were repaired. Assembled review then identified
F01 (missing capability/scope outcomes) and F02 (whole-event uncertainty missing from graph
availability). Both corrections passed focused controls and independent source reinspection.

The first workspace nextest run had 702 passing and 34 failing tests (12 skipped). The failures
were the expected model snapshot and stale conformance fixtures: raw-fact suites selected the
whole newly expanded model, one lifecycle fixture lacked sink completion acknowledgement, and
five new Python fixture directories lacked corpus registration. Conformance suites now select
their actual facts or retained analysis contract; production validation was not weakened.
The 129 affected contract controls passed on rerun. Schema review found 72 new relations, six
new invariants and the native function-origin field; existing fact relations and codebooks were
preserved. The reviewed snapshot was accepted with `cargo insta accept --snapshot ...`.

A broad SQL rule also matched the safe `StageSession` wrapper; its method is now named `query`,
with unchanged prepared read-only planning and scan admission. The rule itself stays intact.
`just fmt-check lint` passed. `just py-check` passed (219 Python tests, 56 explicit skips, and
Pyrefly). Rules, agent lint, fixture parsing, dependency policy, gold and doctests passed.
The real PostgreSQL repeat suite passed all 257 tests (two tests plus one binary skipped),
including the repaired fixtures, four normalized-generation controls and three native graph
controls. Exact logs are in `raw/`; the final release CLI build passed. The current test inventory contains
749 tests (737 active, 12 explicit skips); the new empty-scope control passed separately and in
the PG repeat suite.

Store reset initially refused because its cleanup inventory named control tables absent in the
previous installation. Reset now checks table presence while retiring old control records; it
never reads/migrates their content. The missing-table regression passed in `installation`.
`migrate.py` then rebuilt only generation/control state and checked the current store; all five
retained-service fingerprints were identical. The importer configuration was raised from two
connections/no provider allowance to ten connections/eight provider slots before the successful
reset. The migration receipt records the configuration immediately before that successful rerun.
Old runtime generations were removed; no compatibility reader or rollback generation remains.

`pilots.py` uses the protected normal runtime configuration, one fresh process for each facts or
normalized profile, existing acquired inputs, a shared host, warm build caches and no forced OS
page-cache eviction. It never selects a generation. It records CLI JSON, time-v RSS, stage
reservations/RSS/time, frozen relation cardinalities and PostgreSQL storage sizes, and two-second
connection-state samples. These are measurements under stated conditions, not absolute RSS bounds
or a controlled performance comparison. Canonical repeated content excludes run timestamps and
telemetry by the generation store's existing content contract.

Canonical records own semantics. Native petgraph graphs are built once per input/context/spec in
each normalized collection, encoded as Postcard bytes in bounded BYTEA chunks, and hydrated for
publication validation or later analysis. Runtime dense indices are private. Graph bytes have no
separate reuse hash or incremental update lifecycle. Phase 4 algorithms and Phase 5 serving remain
outside this qualification.

The facts/catalog FastMCP command completed successfully in 528.641 seconds and published
unselected generation `7456ea1e2518e198597c8b67e0a18561`. The user then directed that additional
full-library output comparisons be skipped and catalog regeneration be the final action.
The just-started normalized/catalog compile was stopped; its staging generation
`273d61a8ce09f47fbc6e7d5ee9256e7a` was aborted. `lctx store check` then passed with one generation
and zero findings. No full-library normalized, behavioral/repeat, hydration timing or measured
budget-refusal result is claimed. The prepared measurement/refusal scripts remain unexecuted
recovery aids for the detailed plan's explicit measurement obligation. Automated graph roundtrip,
resource refusal and real-store publication controls did pass as part of the composite gate.

The assembled source review remains Accept scoped. Its O1 pilot-scale measurement obligation is
unmeasured, and O2 remains P4's read-only algorithm adapter handoff. Neither integration tests nor
this interrupted qualification establishes retrieval quality, a speedup or an absolute RSS cap.
The final session action is `just library-catalog`; its output is not separately validated.
