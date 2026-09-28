# Status

_Updated 2026-09-28 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Product checkpoint

- **PR1 Implemented, Tested and deployed:** catalog compilation is the default; `--profile behavioral` enriches the same catalog. Ordered source/provider contracts, stable public members, constructor associations and original declaration/type evidence reach PostgreSQL and FastMCP. [ADR-0072](docs/adr/0072-catalog-compile-profiles.md), [product owner](docs/design/sections/api-and-evidence-product.md), [qualification](docs/design_review/evidence/2026-09-28_catalog/README.md).
- **PR0 Implemented; comparative admission blocked:** frozen development protocol and isolated API/function-tool runner; 12 local controls pass. Twelve A/B/C smoke admissions are blocked/unscored because Context7 FastMCP4.0.5 material parity is unestablished. The separately sealed confirmation candidate remains unexecuted and independently unadmitted; late-seal timing is disclosed. No product differentiation is claimed.
- **Single execution/disposition owner:** [forward plan §3.0/§6.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution). AP/F01 closed at Tested strength; AP/F02–F06 remain open for PR2–PR6. Scoped reviews accept the repaired [PR0 contracts](docs/design_review/reviews/design_review_pr0-evaluation_2026-09-28.md) and [PR1 composition](docs/design_review/reviews/design_review_pr1-implementation_2026-09-28.md).
- **Catalog library target accepted, 2026-09-28:** [ADR-0073](docs/adr/0073-catalog-wire-contracts.md) selects Rust wire contracts/Schemars, offline schema conformance, nominal IDs and isolated catalog derivations. Implementation remains Proposed; [forward-plan §6.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition) owns open CLF/F01–F03 and §7 owns conditional Salsa/Ascent/cache/library triggers. The design and both plans are aligned; no dependencies or runtime behavior changed.
- **Current versions:** compiler108, extractor34, template20, catalog7, bundle13/projection3, migration009; unchanged 1024 embedding specification and library pins. Delta remains canonical; PostgreSQL projections remain rebuildable.
- **Operator deployment verified 2026-09-28:** PG18.6/pgvector0.8.6, migrations001–009. Both live profiles are ready; enriched generation `2081c10e…` is selected with exact profile `ff645e4a…`. Two older ready generations/artifacts remain retained. ANN remains conditional, with no admitted candidate.
- **Preserved recovery:** `build/pr1-operator-cutover/current.dump` plus receipt/artifacts; retained schema008/runtime at `build/postgresql-pr1-baseline-a6c9fcf/` with all 35 runtime checksums verified. Keep earlier PG17/PG16/PG8 backup assets. Old runtime requires its separately restored matching schema; no reverse migration or reader bypass.
- **Retained scope:** existing and specified additive behavioral/query structures, implementations, tests and obligations remain. Stage3 is incomplete; retained Stage3–5 research is activated only by a product task or exposed-claim defect. [§6 findings](docs/plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition) retain their original truth criteria.
- **Development:** cached Cargo target and root editable native environment; no wheels built. Functional checks preceded final formatting/integrated gates. The temporary embedding service used for live qualification is stopped after the checks.

## Verification boundary

| Prior PR0–PR1 command / evidence, 2026-09-28 | Outcome and scope |
|---|---|
| `just fmt`; `just test-all` | **passed:** 446 Rust, 184 Python, 18 real-PG Rust and 2 PG Python tests; all format/lint/Pyrefly/rules/ADR/agent/fixture/dependency/gold/SQLx legs. Final log: `build/pr1-operator-cutover/gates/test-all.log` |
| `uv run python docs/design_review/evidence/2026-09-28_catalog/pilot_probe.py build/pr1-pilots-v3` | **passed:** fresh catalog/behavioral FastMCP compiles, import/selection and stdio MCP serving with live vectors; fake-vector legs qualify mechanics only. Four-current-generation restore 24.69 s |
| `compare_profiles.py build/pr1-pilots-v3` | **passed:** same nine catalog contract multisets, excluding only run-qualified citation identities and optional brief state; source bytes and stable public/signature/type identities retained |
| `lctx db migrate/check`, live `serving import-bundle/reconcile/select`, both `lctx_mcp.smoke` runs | **passed:** final runtime admits and serves both live artifacts; explicit enriched selection |
| `postgres_backup.py restore-drill build/pr1-operator-cutover/current.dump` | **passed:** 78 tables, four retained generations, current-profile serving and exact selection; 12.49 s. Separate old-runtime restore and mixed/refusal controls also passed |
| `just docs-check`; `just adr index`; `just adr lint`; `git diff --check` | **passed:** 163 canonical pages, zero offline link errors,38 ADR records and clean patch whitespace |
| Context7 A/B/C model comparison; confirmation | **blocked** on release/material parity and independent admission; no model execution/scoring or sealed plaintext inspection |

Design-adoption verification, 2026-09-28: `just adr index`, `just adr lint`, `just docs-check` and
`git diff --check` **passed**: 39 ADR records, 166 canonical pages, zero offline link errors.
Product tests and benchmarks **not_run** for this documentation-only scope; prior PR0–PR1 receipts
were not rerun. The library review retains its dated source/Context7/upstream interface evidence.

## Next

1. PR2 detailed execution: refine and implement the accepted shared wire/derivation foundations alongside bounded surface/configuration specificity, using existing catalog identities and the forward-plan §3.0 acceptance controls.
2. PR3: contextual original scenarios and deployment/package evidence, retaining unknowns and attribution.
3. Independent evaluation: resolve PR0 parity/candidate admission before PR6 comparison. PR4–PR5 typed retrieval and agent journeys remain planned; no confirmation retuning or superiority claim.

The [PostgreSQL plan](docs/plans/postgresql-integration-plan_2026-09-27.md) retains completed PG0–PG17 and conditional library consumers. [Runbook](docs/postgresql.md) owns current operations. Arbitrary embedding-endpoint attestation remains W16; general semantics, ontology work, ANN tuning and packaging stay off the immediate product path.
