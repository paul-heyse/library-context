# Status

_Updated 2026-09-28 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Product checkpoint

- **PR2 Implemented, Tested and deployed:** ordered surface aspects, attributed configuration/default/factory records and exact field links; shared Rust requests/full packets for all six MCP tools and the capability resource; immutable indexed catalog inputs and pure derivation. [ADR-0073](docs/adr/0073-catalog-wire-contracts.md), [ADR-0074](docs/adr/0074-catalog-surface-associations.md), [product owner](docs/design/sections/api-and-evidence-product.md).
- **Libraries:** Schemars1.2.2 and offline dev-only jsonschema0.58.2 cover wire and existing format owners. Salsa0.28.2 remains an evidence-only roots/persistence experiment; no production incremental engine is added. Ascent retains S4. [PR2 evidence](docs/design_review/evidence/2026-09-28_pr2/README.md).
- **Current versions:** compiler109, extractor35, template21, behavior-model catalog7, bundle14/projection4, migration010. The 1024 embedding specification and analysis-library pins are unchanged. Delta remains canonical; PG remains rebuildable.
- **Single execution/disposition owner:** [forward plan §3.0/§6.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution). AP/F02 and PR2/F01–F05 are closed at bounded Tested strength. CLF/F01–F03 retain named PR3–PR5 extension/reuse obligations, not unfinished migration of current tools.
- **Fidelity:** source contracts survive unknown wrappers; recognized registration metadata does not prove behavior. Exact field associations do not prove value survival through intervening mutation. Existing and specified additive behavioral/query scope remains; Stage3 is incomplete.
- **PR0 comparison blocked:** release/material parity and independent confirmation admission remain unresolved. The sealed candidate remains unexecuted; no Context7 superiority claim.
- **Operator deployed schema010:** both live profiles ready, behavioral `e50090b2…` selected with exact profile `ff645e4a…`. Four legacy generations/artifacts retained. `build/pr2-operator-cutover/current.dump` restored all 81 tables/six generations and current serving in 21.61 s. ANN stays unadmitted.
- **Preserved recovery:** `build/pr2-baseline-8334c86/source/` contains source/reconstructed schema009 CLI, saved native modules, runtime checksums and protected `pre-cutover.dump` with receipt/artifacts. Its matching-runtime restore passed. Earlier PG17/PR1/PG16/PG8 backup assets remain; never reverse-migrate or bypass reader checks.
- **Development:** Workspace Cargo profiles use O2 with incremental dev/release builds; imported dependencies use O3 without incremental compilation ([ADR-0075](docs/adr/0075-incremental-workspace-builds.md)). Shared host sccache budget is 100 GiB. These operator edits are separate from PR2. The root editable native environment remains; fastdev only. The temporary controlled vLLM service stopped after live operator verification.

## Verification boundary

| PR2 command / evidence, 2026-09-28 | Outcome and scope |
|---|---|
| `just fmt`; focused Rust/native/schema/MCP/PG controls | **passed**; inspected schema snapshots accepted as the declared migration; nominal-ID positive/compile-fail doctests also passed |
| `just test-all` | **passed:** 458 Rust, 188 Python, 19 real-PG Rust and two PG Python tests; all lint/policy/SQLx legs. [Complete log](docs/design_review/evidence/2026-09-28_pr2/raw/test-all.log) records the environment-change boundary |
| `pilot_probe.py build/pr2-pilots-v2`; `compare_profiles.py build/pr2-pilots-v2` | **passed:** both fresh live FastMCP profiles through PG/stdio MCP, all twelve catalog relation multisets equal; four-current-generation/81-table restore 21.45 s. Fake vectors qualify mechanics only |
| `recovery_probe.py build/pr2-mixed-recovery` | **passed:** disposable009→010, all six current/legacy fixture generations preserved; both current profiles served; selected-legacy and corrupt-artifact refusals; 10.40 s restore |
| `salsa_catalog_probe`; isolated `persistence-probe` | **passed:** eight same-fact root cases match clean/indexed output; persisted/transient DTO and incompatible-envelope controls. No production reuse or speed claim |
| `just sqlx-check`; `LCTX_POSTGRES_TEST=1 uv run pytest tests/scripts/test_postgres_serving.py` | **passed:** migrated SQLx offline checks and two PostgreSQL Python tests |
| `just docs-check`; `just adr index`; `just adr lint`; `git diff --check` | **passed** on the staged PR2 export: 169 canonical pages, zero offline link errors, 40 ADR records; patch whitespace clean. Concurrent environment/ADR edits excluded |
| `cargo metadata --offline --no-deps --format-version 1`; disposable Cargo profile compile (2026-09-28) | Prior separate policy controls **passed**. PR2 later rebuilt/tested catalog, CLI, PG and SQLx with the new settings; a complete workspace gate after that policy change is **not_run** |
| Incremental/cache follow-up (2026-09-28) | TOML/profile assertions and Python syntax **passed**; builds, tests, benchmarks and documentation checks **not_run** by operator direction. Running builds/cache servers left untouched |

## Next

1. PR3: contextual original scenarios and deployment/package evidence through the existing pure catalog and shared wire owners.
2. PR4–PR5 typed retrieval/new agent tools; PR0 independent parity/admission before PR6 comparison. Retained research activates only for a selected product task or exposed-claim defect.

The [PostgreSQL plan](docs/plans/postgresql-integration-plan_2026-09-27.md) retains completed PG0–PG17 and conditional mechanisms. The [runbook](docs/postgresql.md) owns current operations; the [PR2 review](docs/design_review/reviews/design_review_pr2-implementation_2026-09-28.md) accepts scoped source conformance, with qualification evidence kept separate.
