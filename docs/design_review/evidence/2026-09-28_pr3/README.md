# PR3 original contextual evidence qualification

**Implemented, Tested and deployed, 2026-09-28.** The [forward plan §3.0/§6.2](../../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns PR3, AP/F03, CLF/F02 and PR3/F01–F08 disposition. [ADR-0076](../../../adr/0076-catalog-contextual-evidence.md)
and [§14](../../../design/sections/api-and-evidence-product.md) own the architecture; the
[implementation review](../../reviews/design_review_pr3-implementation_2026-09-28.md) owns the bounded architectural assessment.

Compiler110/extractor36, bundle15/projection5/wire2 and migration011 implement original artifacts,
spans, scenarios, deployment records and site associations. Binary bodies survive canonical Delta,
portable export and PG bytea. Typed `get_evidence` expands generation-bound references with source
pagination; `get_operation` pages references and admits up to two demonstrations. Large originals
remain readable when typed metadata is explicitly omitted. Release metadata is stored once per
release; provider support and per-site negative intent remain explicit. Execution only occurs in
the separately invoked, fixed-policy task runner.

## Qualification receipts

All commands below were run on 2026-09-28. Rust tests use the release profile; native modules use
the existing editable environment and atomic copies of Cargo-built shared libraries. No wheel was built.

| Command / receipt | Outcome and scope |
|---|---|
| `just fmt`; focused `cargo test --release -p cpg-extract --lib`, `-p cpg-schema --lib evidence::tests`, and `-p cpg-core --test catalog_evidence -- --include-ignored` | **passed:** extraction/receipt integrity, original Unicode/fence context, failed parsing, non-seed APIs, candidate modality, scoped negative/deferred/override controls, pure reconstruction and real-PG pagination. [Focused PG receipt](raw/pr3-pg-final.log), [later subject-layout controls](raw/pr3-scope-targeted.log) |
| `INSTA_UPDATE=no cargo nextest run --release --workspace --no-fail-fast` with the repaired diagnostic selectors | **passed:** shared relational rules again report malformed-input invariants; the complete gate also passed the extended provider-row permutation test. [Diagnostic receipt](raw/pr3-diagnostics-final.log) |
| `just test-all` | **passed:** 465 main Rust tests, 191 Python tests, 20 real-PG Rust tests and two PG Python tests; fmt, Clippy, Ruff, Pyrefly, rules, ADR/agent checks, 90 fixture parses, dependency policy, gold-reference agreement and SQLx verification. [Complete log](raw/test-all.log). The main suite skips 21 explicit/ignored tests; the PostgreSQL selection skips three unselected tests. Frozen-reference comparison was not admitted or run |
| `uv run --no-sync python scripts/deployment_check.py --source build/sources/fastmcp/004bf15a2ba99f077160993c00a404e1a9da83ea --out build/pr3-qualified-tasks` | **passed:** isolated locked environment; actual programmatic and CLI-entry-point stdio listing and `add(2,3)=5`. Both [programmatic](raw/task-programmatic.json) and [CLI](raw/task-cli.json) receipts bind the final runner SHA, source, interpreter/runtime, environment and command. Time/output refusal controls also passed |
| `uv run --no-sync python docs/design_review/evidence/2026-09-28_pr3/pilot_probe.py build/pr3-qualified-pilots build/pr3-qualified-tasks` | **passed:** both fresh live-vLLM FastMCP4.0.5 profiles, final task receipt admission, canonical publication, import and actual stdio MCP. Catalog 173.46 s and behavioral 260.49 s include compile/import/smoke under the concurrent local workload; they are not performance comparisons. [Receipts](raw/pilots.json) |
| `compare_profiles.py build/pr3-qualified-pilots` | **passed:** all seventeen catalog relation multisets match, normalizing only run-qualified citations and profile-specific brief status. Original artifacts, coordinates, typed content, support phase/modality and stable identities remain compared. [Result](raw/profile-parity.json) |
| `evidence_probe.py build/pr3-qualified-pilots` | **passed:** actual MCP span/scenario and both task-deployment expansions, pagination, response bounds and exact original-byte comparison. Each profile has 965 artifacts, 32,320 spans, 3,293 scenarios, 1,726 deployment records and 85,167 associations; 88 associations are release-scoped. [Result](raw/live-evidence.json) |
| Pilot backup/restore | **passed:** 86 tables, both current profiles served, selected behavioral generation preserved, 20.23 s. [Result](raw/live-restore.json) |
| `recovery_probe.py build/pr3-mixed-recovery` | **passed:** disposable schema010→011, six legacy plus two current fixture generations; selected-legacy and corrupt-artifact refusals; both current profiles served, 13.35 s. These unchanged fixture generations also passed the final gate. [Result](raw/mixed-recovery.json) |
| `operator_cutover.py build/pr3-qualified-pilots build/pr3-operator-cutover` | **passed:** migration/check, both live imports/reconciliation, real-vLLM MCP and original evidence, explicit exact selection, populated backup/restore. All 86 tables/eight generations preserved and current serving restored in 28.59 s. [Result](raw/operator-cutover.json) |

## Corrections and limits

The first live export refused the 200,000-row association budget: copying release metadata per API
produced 504,595 rows. Explicit release subjects fixed the model; the limit was retained. A later
profile comparison exposed provider-order leakage into scenario requirements; source-coordinate
ordering and a reversed-input control fixed it. Review F01–F08 also cover oversized metadata access,
unittest overrides, deferred generators, CLI provenance, receipt equality and shared diagnostic reuse.

Initial gate failures are retained in `raw/`: nine reviewed schema/identity snapshots and five new
independent schema digests needed migration baselines; malformed-input diagnostics needed the shared
validator and table-metadata normalization; Python/PG tests needed Binary framing, new argument/field
shapes, schema-sized stdio framing and explicit expanded requests. Pyrefly required a receipt-dict
annotation; its runner-hash change triggered new actual task checks and both live builds. Snapshot
candidate generation used `INSTA_FORCE_PASS=1` only to inspect all diffs, then `cargo insta accept`;
that run is not qualification. The final gate used no failure-accepting overrides.

This qualifies PR3's supported records and boundaries. General evidence search/browse, complete
oversized-metadata browsing, contextual requirement classification and agent journeys remain PR4–PR5.
No Context7 superiority, minimal-install sufficiency, arbitrary task/platform success, effective-default
inference, complete behavioral model or positive ANN admission is claimed. The behavioral live profile
retains its existing optional analysis behavior with `techniques=none`.

## Operator state and recovery

Schema011 selects behavioral `d874d3694612e12d99272f699fe474e5cce9758cb5cf860dd49cbd52fccd58b2`
with exact profile `ff645e4a55461d3041fa4dbbd56f01d90da07b74a55040ec350933ac23ef4586`.
Catalog `0b2fe1205720f3291853671629935afc8137f968437f7a0be66e5be81effce74` is ready. All six older generations/artifacts remain retained.
`build/pr3-operator-cutover/current.dump`, its protected JSON receipt and artifact directory form the
current populated recovery set. Never separate them or bypass reader compatibility.

`build/pr3-baseline/` retains source `5aebab4`, matching reconstructed schema010 CLI, preserved editable
native modules, runtime checksums and protected `pre-cutover.dump` plus receipt/artifacts. Its matching
runtime restore **passed**: 81 tables, six generations, both then-current profiles served and exact
selection preserved, 21.41 s. All earlier PG/PR1/PR2 recovery assets remain. Legacy generations require
their matching retained runtime. The temporary controlled vLLM service stopped after operator checks;
start `just embed-serve` for live query embeddings.
