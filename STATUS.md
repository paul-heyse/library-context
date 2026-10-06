# Status

_Updated 2026-10-06 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Graph-native implementation audit complete: the full pivot is not yet fully realized.**
The [independent audit](docs/design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md)
examined clean production `b90cb611` across the complete coordinator scope. Three fresh reviewers
inspected compiler/projections, native realization/lifecycle and consumers/retirement; the principal
reviewer reconciled decisive cross-boundary evidence. The audit records twelve findings.
[Coordinator §7](docs/plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities)
is the sole current disposition owner; all twelve are open. No production remediation was performed.

Runtime PostgreSQL, generation readers/commands and Python product retrieval are retired in the
inspected reachable closure. Typed graph transport, store-free compilation, native persistence,
search, named projections and all ten native/CLI/PyO3/MCP tool routes are present. Current
documentation retirement remains incomplete. ADR-0128 and the two nominated target/capabilities
reviews remain the selected architecture; native querying is not awaiting an adoption decision.

The remaining gaps concern semantic admission, executable dependency identity, explicit cold audit,
backup finality, coherent selection, scoped vocabulary, nested continuations, typed safe errors,
resident/replayed compiler preparation and native bulk boundaries. The report separates concrete
correctness failures from qualitative execution-fit findings and unresolved evidence.

**Current audit evidence, 2026-10-06:**

- Normalized-environment `cargo build --release --locked -p lctx-publisher -p lctx-serving
  --message-format=json`: **passed** on unchanged audited production sources.
- `UV_NO_SYNC=1 uv run --no-sync python
  docs/design_review/evidence/2026-10-06_graph-native-pivot-audit/run_probes.py`: **passed** as
  diagnostic reproduction. Failed selection still published the new serving pin; cold audit
  accepted after seven actual Catalog search-document texts were altered. Both intended product
  guarantees **failed** in those cases. Setup attempts failed before execution; the corrected
  runner supplies the pinned nightly's separate metadata/code artifacts.
- [Evidence README](docs/design_review/evidence/2026-10-06_graph-native-pivot-audit/README.md)
  records sources, commands, observations and limits. Owned disposable persistent server,
  credentials, binaries and scratch were removed. No operator state was inspected or activated.

**Audit publication checks, 2026-10-06:** `UV_NO_SYNC=1 just docs-check` **passed**
(321 canonical pages, zero link errors). Explicit Ruff checking of the otherwise excluded
probe runner **passed** after import/style repairs; the initial empty selection was not counted
as verification. `UV_NO_SYNC=1 just turn-end` **passed** with no generated/production-source
changes. These checks validate documentation/probe hygiene, not product correctness.

**Prior implementation receipts remain historical:**
[Coordinator §8](docs/plans/graph-native-pivot-plan_2026-10-05.md#8-current-checkpoint) owns the
2026-10-05–06 native SDK/projection, publication, eligible search, ten-tool Catalog and in-process/
stdio MCP successes and exact exercised revisions. Scope-end leaves passed there; the final
NativeSession wheel on production `599010a0` was built, installed and import-checked. These do not
close the newly identified findings or demonstrate every optional behavioral branch. The earlier
cache control exercised sequential winner reuse; actual concurrent conflict behavior is unresolved.

**Compiler stage remains complete / user-accepted, 2026-10-05.** Its interrupted-test limits are
preserved in coordinator §8. Static audit includes that source and identifies additional unmet
obligations; the stopped suite was not restarted. The isolated Catalog diagnostic compiled a small
first-party fixture for the specific cold-audit question, not compiler-suite qualification.

Broad `just qualify`, legacy snapshots/parity, real-library/live-Qwen journeys, operator adoption
and performance measurement remain **not_run** under the agreed audit scope. Heldout/gold data,
operator services/client registrations, unrelated worktrees and shared Cargo caches are preserved.
Efficient design follows first principles and library capabilities; no cost proofs, detailed work
accounting, query-adoption gates or new permanent audit gate were added.

**Next:** remediate the open coordinator findings in the report's order: semantic/executable
integrity; backup/selection/audit; scoped consumer/error/continuation behavior; compiler/native
bulk execution and current documentation retirement. Remediation and Q1 activation are separate
work, with targeted functional validation; neither was performed by this audit.
