# Status

_Updated 2026-10-05 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Testing, validation and verification pivot implemented; assembled acceptance in progress.**
The [assurance coordinator](docs/plans/testing-architecture-pivot-plan_2026-10-04.md)
owns F01–F04 and the current contract/control map. ADR-0126 replaces broad gate cadence and
routine privileged-tamper checking with explicit contract families, owned immutable proof reuse
and independent read-only audit. The supporting validation/verification plans own detailed scope.

`lctx-model` owns one definition per semantic question, explicit relation/stage references,
revision/digest and complete ordered premises. Source-call header/invocation/run independently
reference one replay. Missing premises refuse; scoped generated derivation checks are model-owned.
PostgreSQL receipts bind installation/generation, model/layout, definition, profile/configuration
and exact acknowledged sources/prefixes. Grant receipts refer to conclusions; current admission
remains required on a hit. Failed, partial, interrupted or unconfirmed execution supplies no hit.

Validation sessions share resolved receipts and compatible streams within pure/publication groups,
fuse compatible candidate digests, and fall back to bounded separate execution. Every checker
finishes before any group result is acknowledged. Final frozen empty relations use final receipts
without producer-stage authority. Ordinary output and delta locks drain writers and revoke further
mutation before acknowledgement; later legal appends preserve earlier prefix bodies and membership.
`lctx generation audit` challenges body/aggregate identity, nominal references, proof bindings and
required pure/publication semantics without repair, grants or new acknowledgements.

Seven `verify-*` families prepare only their declared tools/adapters/store/CLI closure. `qualify`
prepares their union once and collects independent family/leaf failures, including keep-going
Clippy. Legacy broad gate aliases are deleted. Read-only Catalog find/retrieval/packet/audit cases
share one immutable seed, fresh requests and named failure collection; mutation cases remain isolated.
Obsolete Pysa/CrossHair evidence is deleted; finite analytic assurance rationale lives at its owner.
No persistent test-pass cache, compatibility receipt readers or historical runtime archive is added.

## Last verified — 2026-10-05

Commands use `python3 scripts/build_environment.py -- …` or its normalized `--shell` environment.
The [coordinator receipt](docs/plans/testing-architecture-pivot-plan_2026-10-04.md#7-current-contractcontrol-map-and-execution-checkpoint)
records failures/repairs and their boundaries; focused receipts are not assembled acceptance.

| Command / boundary | Outcome |
|---|---|
| `uv run --no-sync pytest tests/scripts/test_verify.py tests/scripts/test_build_environment.py -q` | **passed**, 20 scheduling/readiness-key controls; mocked launcher controls do not certify actual families |
| Focused release `lctx-model` definitions/views/closure/sources/expected targets | **passed**, 24; explicit source-check selection repaired before rerun |
| `cargo check --release -p lctx-postgres -p lctx -p cpg-core --tests` | **passed** at integration checkpoint; later focused store runs compiled newer changes |
| Actual PG proof hit/miss, delayed/queued writer and immutable-prefix controls | **composite passed**; harness visibility/dedup mistakes corrected without weakening guarantees |
| Generation/publication/audit controls | **passed**, four; includes registry-only aggregate damage and bounded publication fallback |
| `cargo nextest run --release -p lctx-postgres --test lifecycle --test generation_catalog --no-fail-fast --no-tests=fail` | **passed**, 14. Catalog fixtures now declare exact typed premises; state-only controls use a separate minimal model |
| Final held-empty-vocabulary and exact audit-frame controls | **passed**, two |
| Scoped union readiness within `NEXTEST_TEST_THREADS=8 just qualify` | **passed**, both adapters rebuilt; actual MCP/semantics/storage imports and PG image readiness passed |
| `NEXTEST_TEST_THREADS=8 just qualify` | **failed** initially; analytics, oracles and tooling passed. Provider/store/producer/serving failures were repaired; affected reruns remain pending. Full keep-going Clippy also requires its repair rerun |
| Focused checkpoint/install/stage/budget controls | **composite passed**, 12; checkpoint acknowledgement, writer exclusion, rollback and admission-lifetime controls retained |
| `UV_NO_SYNC=1 just ruff types lint-agents adr-lint docs-check` | **passed** after the authorized existing Stop hook; 300 canonical pages, no link errors |
| `NEXTEST_TEST_THREADS=8 just verify-model` plus `-- -E 'binary(serving_contracts)'` | **composite passed**, 623 controls; two response fixtures omitted mandatory domains, repaired with missing-domain refusal retained; affected target21 passed |
| Package-name normalization control | **passed**, one; explicit empty-name refusal retained |
| `NEXTEST_TEST_THREADS=8 just verify-store --command rust` | **passed**, 59 disposable PostgreSQL controls |
| Optional comparable timings / total RSS | **not_run**; structural scan/setup counts establish no Measured speed claim |
| Real-library reconstruction/activation, live vectors, product comparisons and heldout | **not_run / stopped**, outside this pivot |

Independent implementation review found and prompted repairs to scoped DAG registration, stable
proof-frame persistence, source snapshot acknowledgement checks, audit frame contracts and aggregate
identity. Review of publication fanout/fallback found no additional defects; functional receipts
above, rather than static judgment, establish their Tested boundaries.

## Next

Model623, provider repair1 and store59 are composite/passed. Producer rerun18 passed11 and
failed7: Catalog vocabulary closure omitted checkpoint-qualified empty premises; five structural
mutation cases used a stale sum column. Typed fixture and coordinated premise planning now compile;
29 definition/vocabulary/checkpoint controls passed. Eight affected producer cases are rebuilding,
then native/MCP, serving, CLI schema migration and full keep-going Clippy remain. Full keep-going Clippy belongs to that run.
Formatting/generators remain owned by the existing Stop hook. The operator authorized invoking
that hook at scope end; it has run, with formatting and generators completed. Verify affected checks. Update F01–F04 and applicable Q0/Q1 owners only from actual replacement evidence.

The [target coordinator](docs/plans/target-implementation-alignment-plan_2026-10-04.md)
retains TA-F/LL-F disposition and its implemented foundations. The new assurance control map carries
its surviving fixture/native/MCP obligations; earlier cancelled compilation does not qualify them.
O08 remains with the [enrichment coordinator](docs/plans/code-facts-analytical-enrichment-plan_2026-10-04.md).

## Preserved operator state and authority

**Phase 5 real-library activation remains stopped at the operator's review pivot.**
[Phase 5 §10](docs/plans/semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state)
owns activation/serving limits; [cutover §8](docs/plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
owns older cutover findings. Default store, client registrations, live vectors, gold and heldout
are untouched by this scope. New receipt/install schemas are exercised only in owned disposable
stores; default-store reconstruction is required before later authorized activation.

Current integration worktrees/logs remain until acceptance; clean up fully integrated worktrees
then. Old qualification worktrees retain their existing current consumers. The operator cleared approximately 560 GiB with `cargo clean`, so resumed builds are cold.
Cargo jobs16/frontend1 and Nextest8 remain unchanged; the agent ran no clean or broad reset.
