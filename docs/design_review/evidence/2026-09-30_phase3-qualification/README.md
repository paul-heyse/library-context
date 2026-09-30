# Phase 3 qualification

Consumer: the Phase 3 detailed plan §11/Q and its assembled exit review. Evidence, not authority.
**In progress, 2026-09-30.** Focused implementation controls passed before Q; full phase acceptance
remains pending. `raw/` retains command outputs via Git LFS.

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
controls. Exact logs are in `raw/`; final CLI build and full-library pilots are in progress.

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
