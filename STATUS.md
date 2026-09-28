# Status

_Updated 2026-09-28 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Product checkpoint

- **PR3 Implemented, Tested and deployed:** original artifacts/spans, contextual scenarios, attributed package/configuration/task evidence, and bounded typed `get_evidence`/`get_operation` expansion. Independent pure evidence derivation composes with the catalog. [ADR-0076](docs/adr/0076-catalog-contextual-evidence.md), [product owner](docs/design/sections/api-and-evidence-product.md).
- **Libraries:** pep508_rs0.9.2 with explicit URL values, mailparse0.17.0 and rust-ini0.21.3 interpret preserved source metadata. Schemars/dev jsonschema remain shared contract tools. Salsa stays evidence-only; Ascent retains S4. [PR3 evidence](docs/design_review/evidence/2026-09-28_pr3/README.md).
- **Current versions:** compiler110, extractor36, template21, behavior-model catalog7, bundle15/projection5/wire2, migration011; standard1024 unchanged. Delta remains canonical; PG remains rebuildable.
- **Single execution/disposition owner:** [forward plan §3.0/§6.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution). AP/F03, CLF/F02 and PR3/F01–F08 are closed at bounded Tested strength. CLF/F01/F03 retain PR4–PR5 extension/reuse obligations.
- **Fidelity:** context, intent and parse/binding/environment/execution status stay separate. Release metadata is not copied into per-operation requirements. No minimal-install or arbitrary-task inference. Existing/additive behavioral/query scope remains; Stage3 is incomplete.
- **PR0 comparison blocked:** release/material parity and independent confirmation admission remain unresolved. The sealed candidate remains unexecuted; no Context7 superiority claim.
- **Operator schema011:** both final live profiles ready; behavioral `d874d369…` selected with exact profile `ff645e4a…`. Six legacy generations/artifacts retained. `build/pr3-operator-cutover/current.dump` restored all 86 tables/eight generations and current serving in 28.59 s. ANN stays unadmitted.
- **Preserved recovery:** `build/pr3-baseline/` contains schema010 source `5aebab4`, reconstructed matching CLI, saved native modules, checksums and protected dump/receipt/artifacts. Matching-runtime restore passed in 21.41 s. All earlier recovery assets remain; never reverse-migrate or bypass reader checks.
- **Development:** Workspace Cargo profiles use O2 with incremental dev/release builds; imported dependencies use O3 without incremental compilation ([ADR-0075](docs/adr/0075-incremental-workspace-builds.md)). Shared host sccache budget is 100 GiB. These operator edits remain preserved; the PR3 gate exercised the current settings without a build-performance claim. The root editable native environment remains; fastdev only. The temporary controlled vLLM service stopped after live operator verification.

## Verification boundary

| PR3 command / evidence, 2026-09-28 | Outcome and scope |
|---|---|
| `just fmt`; focused Rust/schema/PG/runner controls | **passed:** original and failed-parse context, negative/deferred/override intent, provider ordering, receipts, Binary round trip and bounded expansion. Nine schema/identity snapshots were inspected before acceptance as the declared migration |
| `just test-all` | **passed:** 465 Rust, 191 Python, 20 real-PG Rust and two PG Python tests; all lint/policy/SQLx legs. [Complete log](docs/design_review/evidence/2026-09-28_pr3/raw/test-all.log) includes explicit selection/skips |
| `deployment_check.py ... --out build/pr3-qualified-tasks` | **passed:** final SHA-bound isolated programmatic and CLI stdio listing/call; both actual results are 5 |
| `pilot_probe.py build/pr3-qualified-pilots build/pr3-qualified-tasks`; `compare_profiles.py`; `evidence_probe.py` | **passed:** both fresh live profiles through real PG/MCP, all seventeen catalog relation multisets match with explicit citation normalization, original-byte pagination/task receipts; two-profile/86-table restore 20.23 s |
| `recovery_probe.py build/pr3-mixed-recovery`; `operator_cutover.py ... build/pr3-operator-cutover` | **passed:** disposable010→011 and operator populated recovery; all eight generations preserved, selected-legacy/corrupt-artifact refusals; current profiles served and exact selection restored |
| `just docs-test`; `just adr index`; `just adr lint` | **passed:** 39 documentation-tool tests and 42 ADR records; shared-skill publication and retained ADR-chain regressions repaired |
| `just docs-check`; `git diff --check` | **passed:** 173 canonical pages, zero offline link errors; patch whitespace clean |
| Incremental/cache policy | Prior operator policy controls retained. PR3's complete gate used the current Cargo settings; build/cache benchmarking **not_run** |

## Next

1. PR4: typed requirements, contextual eligibility and addressable retrieval units, using PR2/PR3 contracts.
2. PR5 journeys/browse and qualified rebuilds; PR0 independent parity/admission before PR6 comparison. Retained research activates only for a selected product task or exposed-claim defect.

The [PostgreSQL plan](docs/plans/postgresql-integration-plan_2026-09-27.md) retains completed PG0–PG17 and conditional mechanisms. The [runbook](docs/postgresql.md) owns current operations; the [PR3 review](docs/design_review/reviews/design_review_pr3-implementation_2026-09-28.md) owns the bounded architectural assessment. Concurrent shared-skill edits are preserved separately from PR3.
