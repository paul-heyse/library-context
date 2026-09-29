# PostgreSQL PG12–PG15 query integration evidence

Dated 2026-09-28. The [PostgreSQL plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream)
owns scope; [forward-plan §6.1](../../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
owns findings. ADR-0069 binds the context/query contract. These receipts qualify the bounded
implementation; PG16 operator rollover/recovery and PG17 assembled live ANN/cost acceptance remain
open. Tests use disposable PG18.6/pgvector 0.8.6 containers and this repository's release target.
Operator migrations remain 001–004; migrations 005–007 have not been applied to that database.

## Focused functional receipts

All Rust commands used `CARGO_TARGET_DIR=/home/paul/library-context/target`. Early provider controls
used a local patch while developing the owned fork. The final provider control uses the immutable
root pin `e6fc4c40ec0ffdb7c89371371b18c5f819cc79a9`, with no patch config.
The `raw/` logs and frozen archives below use Git LFS.

| Command or selector | Outcome and boundary |
|---|---|
| `cargo test --release -p lctx-postgres --test serving production_import -- --ignored --nocapture` | **passed:** atomic ready, idempotent retry, explicit selection, two generations with unchanged old pin, ready cleanup refusal, corrupt retained artifact refusal |
| Same target, `interrupted_import` | **passed:** actual batch-lock cancellation stays invisible, interrupted attempt resumes, concurrent retries converge; repeated after SQLx lease repair |
| Same target, `publication_refuses` | **passed:** frozen transport/batch conflicts, stored-content validation failure, no serving visibility, inactive cleanup retains terminal diagnostics |
| Same target, `captured_reference_parity` | **passed:** 6,302 complete operation answers, 20 full brief answers and four facet pages match pre-cutover answers from the same canonical fake-1024 snapshot; 49.77 s. Generation/cursor encoding and explanatory notes are intentionally compared by their own contract controls |
| Same target, `exact_ranks_and_ann` | **passed:** independent exact per-view ranks, finite/spec/membership refusals, HNSW build without activation |
| `uv run pytest python/lctx_mcp/tests/test_qualification.py` | **passed:** frozen independent f64 reference matches; real prepared-plan evidence rejects the tiny fixture's ANN profile; default exact selection retained. This is refusal qualification, not a live ANN quality pass |
| `cargo test --release -p cpg-core --test postgres admitted_provider -- --ignored --nocapture` | **passed:** actual final-pin pushed/local values and schemas, aliases/key joins, unsupported local casts/functions/binary projection, empty metadata, late physical residual, cancelled server work and concurrent repeatable-read capture. Outer joins stay local; their null-preservation control passes |
| Same target, `pg_published_bundle_replays_every_byte_after_database_stops` | **passed:** report retains an unready canonical summary, then reports 51 ready relations, exact profile and selection; all exported bytes replay with PG stopped |
| `LCTX_POSTGRES_TEST=1 uv run pytest tests/scripts/test_postgres_serving.py` | **passed:** real async lifecycle and one-slot cancellation: `pg_sleep(5)` under a two-second server deadline retains capacity until drain, then permits reuse; errors remain sanitized |
| `uv run pytest python/lctx_mcp/tests/test_lifecycle.py` | **passed:** cancelled CPU work retains admission slot; whole-request deadline covers multiple phases |
| Native/projection Python selectors plus serving projection Rust target | **passed:** declared schema/context identity, tamper refusal and pure native image behavior; five Rust contract tests and 26 focused Python controls |
| Python operation/server/value-path/stdio selectors and localized repair reruns | **passed:** actual PG tool responses, unknown coverage, aliases, cursor binding, complete evidence and stdio protocol; smoke helper uses a pinned PostgreSQL generation |

The portable `just test-postgres` fixture exercises the product path without the archived pilot.
Use `LCTX_TEST_PROJECTION` with a verified exported bundle for direct Rust selectors above.

## Retained same-input parity inputs

`projection.tar.gz` contains the verified FORMAT 12 projection of canonical snapshot
`f783ed5ba6e51720892c37a13bc77584`; `reference.json.gz` holds the previously captured file-reader
answers. The frozen answers do not call or reconstruct the new SQL repository. Both use fake
vectors and therefore make no live retrieval-quality claim. The test checks every captured field
except the deliberately changed generation/cursor representation and explanatory notes.

```sh
mkdir -p build/pg12-parity-reference
tar -xzf docs/design_review/evidence/2026-09-28_postgresql-query/projection.tar.gz \
  -C build/pg12-parity-reference
gzip -dc docs/design_review/evidence/2026-09-28_postgresql-query/reference.json.gz \
  > build/pg12-parity-reference/reference.json
just test-postgres-reference "$PWD/build/pg12-parity-reference/projection" \
  "$PWD/build/pg12-parity-reference/reference.json"
```

## End-of-slice checks

The named `just test-all` run **failed initially** on nine stale Rust test consumers. Its full
no-fail-fast run retained 428 passes; all nine corrected consumers then **passed** their targeted
rerun (437 ordinary Rust tests across retained/repair receipts). Python retained 168 passes;
one stale ranking-guard test then **passed** its targeted repair (169 tests). Two explicit PG
Python tests were skipped in that suite and **passed** separately. Unaffected tests were not
repeated to manufacture a single green invocation.

| Command | Final outcome and receipt |
|---|---|
| `cargo nextest run --release -p cpg-core -p lctx --no-fail-fast -E '<nine failed names>'` | **passed**, `raw/rust-consumer-repairs.log`; old file-serving calls now exercise production PostgreSQL or pure native reference inputs, according to the assertion |
| `just py-check` plus `pytest tests/scripts/test_gold_match.py -k test_the_ranking_check_is_blocked_by_a_degraded_live_alias`; `pyrefly check --summary=none` | **passed across retained/repair receipts**, `raw/python-suite-initial.log`, `raw/python-ranking-repair.log`, `raw/pyrefly.log`; no mocked result establishes PostgreSQL acceptance |
| `just test-postgres` | **passed**, 17 real PostgreSQL tests and two real async lifecycle controls, `raw/postgres-gate.log` |
| `cargo nextest run --release -p cpg-core --test postgres --run-ignored only -E 'test(pg_published_bundle_replays)'` | **passed** after the report correction, `raw/report-multi-release.log`; normal corpus acquisition supplies the second release, loading state retains one canonical summary, ready state emits 51 relations, PG-offline bundle replay remains byte-equal |
| `just fmt-check lint`; targeted final report Clippy; `just fixtures-check` | **passed**, `raw/format-and-lint.log`, `raw/report-clippy.log`, `raw/format-and-fixtures.log`; 84 fixtures parse |
| `just rules-scan rules-test lint-agents deps gold sqlx-check` | **passed**, `raw/policy-and-sqlx.log`; final owned Git pin, fresh seven-migration disposable database and offline SQLx metadata |
| `just pilot build/postgresql-pg15-pilot-store build/pg15-pilot.log <disposable serving config>` | **passed**, `raw/pilot-and-capture-error.log`; fresh fake-1024 compile took 151.8 s, import and all 20 MCP brief smoke checks passed. A subsequent evidence helper misparsed the bundle path; that helper failure did not fail `just pilot` |
| `lctx db report --store build/postgresql-pg15-pilot-store --snapshot dcdab24df4e341e238276da5c52ac1b9 --generation 712eb62155beb656b960fb5fafc1128bab6a5f70d94402caa81bceff68ec798a --serving-config <disposable config> --format json` | **passed**, `raw/pilot-report.json` and `raw/pilot-report-capture.log`; reimported the existing fresh pilot, no repeated compile or operator mutation |
| `just docs-check`; scoped publisher excluding only `docs/external-review-postgresl-options.md` | **failed** on that unchanged supplied input's missing H1; **passed** ADR/agent checks and 153 canonical pages/offline links; `raw/docs-check.log`, `raw/docs-scoped.log` |
| `lctx db status` | **passed**, `raw/operator-status.log`; operator PG18.6 reachable, intentionally `schema_current=false` until PG16 applies 005–007 |

The integrated pilot exposed a report-only assumption that a snapshot has one release row.
An acquired library can also have a separate documentation/usage corpus. The corrected report
uses the same canonical library aggregation as bundle publication, retaining one snapshot summary
and one row per serving relation. Both the real FastMCP pilot and acquired-corpus regression above
exercise this correction.

Clean Python 3.14 wheel installation had already passed 17 MCP/stdio/lifecycle checks before the
operator clarified the development workflow. That completed check is extra historical evidence
only (`raw/clean-wheel-functional.log`). All subsequent work used the root editable environment;
standalone packaging and clean-install checks are deferred until an actual distribution consumer
requires them. Formatting and broad checks followed functional completion.

No production ANN profile has been qualified or selected. Live recall/latency, representative
RSS/WAL/contention, populated projection restore and operator migration/rollover remain **not_run**
for this slice, owned by PG16/PG17. Stage 3 semantic acceptance is unchanged.
