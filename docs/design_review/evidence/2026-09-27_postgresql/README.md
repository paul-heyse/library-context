# PostgreSQL implementation qualification

**Tested and Measured, 2026-09-27; PG0–PG7 initial scope accepted.** This evidence qualifies the
PostgreSQL service/cache/operations boundary. It does not complete Stage 3 semantics or its
frozen evaluation. The [implementation plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream)
owns conditional future work; the [forward plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
owns finding dispositions. [Operations](../../../postgresql.md) owns ongoing procedures.

## Baseline, criteria and decision

Baseline source is `5e623535ed0ad836b633e5788cd6c3ea1765a248`. Its release binary, lockfile and
existing evaluation receipts remain under `build/postgresql-baseline-5e62353/`. Prior stores,
generations and evaluation artifacts were preserved; new runs use separately named fresh stores.
Compiler output moves from 105 to 106; receipt tables are a schema migration. FORMAT stays 10.
Frozen analytics/ranking/model parameters and sealed heldout inputs were not changed.

Before timing the adapter, we selected 2,463 and 25,000 keys at 4,096 dimensions, cold/warm reads
and one/four concurrent clients. Small-workload budgets: warm bounded reads under 2 seconds;
cold admission under 5 seconds, excluding embedding service time. Pools have at most six
connections per compile; retained vector payload defaults to 256 MiB, with explicit refusal
and a configurable upper bound of 1 GiB. Map/text/returned-vector overhead is additional.
Restore target: daily recovery point and local logical restore within 15 minutes.

**Decision: retain the deployed initial scope.** The measured small cache cases meet their
latency budgets, exact canonical replay survives cache/service loss, and the operational CLI
provides durable attempt history and repairable discovery. These capabilities justify the
service and its measured storage/maintenance cost. No whole-compiler speedup is established:
whole-pilot runs overlapped builds/tests and other host work. The larger cache case describes
cost; it does not establish unbounded scale or fit inside the default receipt limit.

## Deployment and driver

The user ran `uv run python scripts/postgres_bootstrap.py` successfully. `18/main` on port 5432
hosts `lctx`, using loopback SCRAM and a mode-600 operator config; HBA was unchanged. PG16 on
5433 remains down and untouched. `lctx_migrator` owns DDL, and `lctx_app` has limited runtime
grants; neither has superuser/createdb/createrole/replication. Actual server version is `180006`.
Data is under `/var/lib/postgresql/18/main`; config/HBA under `/etc/postgresql/18/main`; socket
under `/var/run/postgresql`. Only `plpgsql` is enabled; no vector/statistics extension was needed.
Host settings: 100 connections, 128 MiB shared buffers, 4 MiB work memory, 1 GiB max WAL, SSL on.

Migrations `202609270001` and `202609270002` are applied. `target/release/lctx db migrate` (idempotent), `db check` and
`db status` **passed** as the application role. Missing-config and wrong-password CLI controls
**passed** (expected rejection, no password/URL in diagnostics). Broad config permissions and unverified remote TLS also fail before connection. Remote verified TLS is
configuration-enforced but **not_run** against a remote deployment; this acceptance is local.
See [service output](raw/postgresql-service.log) and [credential controls](raw/postgresql-credentials.log).

The standalone `probe/` is an isolated workspace, outside production source/dependency hashing:

```sh
LCTX_POSTGRES_TEST_TAG="$(cat specs/postgres-image.txt)" cargo run --locked \
  --manifest-path docs/design_review/evidence/2026-09-27_postgresql/probe/Cargo.toml \
  --target-dir /home/paul/.cache/lctx-postgresql-probe-target
```

**Passed:** SQLx 0.9.0/Tokio/Rustls plus module-reexported Testcontainers, actual PG180006,
concurrent insert/winner visibility, exact binary round trip and app-role DELETE refusal.
Application tests use the same pinned PG18.6 image/digest with durability enabled.

## Acceptance commands and failure challenges

Product commands below use `CARGO_TARGET_DIR=/home/paul/library-context/target` and
`RUST_MIN_STACK=16777216`; the inherited other-project target was explicitly overridden.

| Command / challenge | Outcome and evidence |
|---|---|
| `just test-all` | **passed (exit 0):** 430 ordinary Rust tests, 8 real PG tests, fresh native fixture, 154 Python tests, strict fmt/Clippy/Ruff/Pyrefly, 7 rule suites, ADR/agent lint, 83 fixture parses, dependency/fork/gold policy and fresh-schema SQLx metadata; [complete receipt](raw/postgresql-test-all-accepted.log) |
| `just sqlx-check` | **passed:** freshly migrated disposable PG18, all core targets; [output](raw/postgresql-sqlx-check.log) |
| Modify one `.sqlx` nullability entry, run `scripts/postgres_check.py`, restore original bytes | **passed:** expected nonzero drift rejection, naming the changed query; [negative control](raw/postgresql-stale-metadata.log) |
| `DATABASE_URL=postgresql://unreachable@127.0.0.1:1/nope cargo check --release --workspace --offline` | **passed:** ordinary build uses committed offline metadata; [output](raw/postgresql-offline-check.log) |
| Restart/replay and tracing-focused nextest | **passed:** 2 tests, cache hit/admission/receipt counts and durations without values; [output](raw/postgresql-restart-tracing.log) |
| `just docs-check` | **failed:** supplied external-review input has no H1; file preserved. **passed:** same publisher excluding only that input, 146 canonical pages; [scoped receipt](raw/postgresql-docs-scoped-final.log) |

Eight real PG cases cover distinct valid competing winners and signed-zero byte fidelity;
forbidden app DDL/UPDATE/DELETE; corrupt digest/admission rejection; retained E0/operation/brief
values across conflicting cache restoration and disconnection; migration advisory locking and
checksum drift; timeouts/cancellation/reuse; append-only event idempotency/conflicts; unknown
recovered history and cutoff-scoped reconciliation; legacy re-admission/partial retry/conflict;
every bundle byte after PostgreSQL stops; restart persistence; bounded concurrent costs; and
forced backend termination while an insert waits on a lock, followed by bounded retry/readback.
Shared publication validators have independent missing-receipt, attribution-mask and digest cases.

Earlier gate failures were corrected before the final receipt: standard Clippy findings,
expected schema/rule snapshot changes, a stale extractor identity snapshot from the preceding
Logger catalog change (`0359548`), and a test-only restart readiness assumption. The extractor
snapshot was reviewed: stable context/node IDs remain stable, while catalog-dependent producer,
run and fact IDs move. No extractor implementation/version was changed. Restart now refreshes
Docker's allocated port and waits up to 30 seconds for authenticated SQL readiness. The byte
replay already passed before that restart-fixture failure. Negative-control failures are not
reported as product failures or hidden as successful runs.

Compiler-affecting queries are inline in hashed Rust source. The source inventory also accepts
`.sql` files under compiler source trees; operational migrations remain separately versioned.
There is no external compiler query whose identity is untracked.

## Fresh pilots and immutable replay

| Route / command | Outcome | Compile / extraction time | Peak RSS (KiB) | Generation |
|---|---|---|---|---|
| Preserved binary `compile fastmcp --embedder fake`, fresh old-format store | **passed** | 174.9 / 51.6 s | 4,156,012 | `1d19dd3b9e6a42e7` |
| `just pilot build/postgresql-pilot-store build/postgresql-pilot-fake.log` | **passed**, MCP smoke 20 briefs | 291.4 / 37.5 s | 3,946,152 | `46404da33007ee28` |
| Current `compile fastmcp --embedder vllm`, fresh live store | **passed**, separate MCP smoke 20 briefs | 314.0 / 54.6 s | 4,081,164 | `62c0ea68246bfea5` |
| Same live inputs/spec, warm PostgreSQL, embedding service stopped before consumption | **passed** | 248.8 / 55.4 s | 3,917,796 | `ac8ca213d6988c6e` |

`/usr/bin/time -v` measured process RSS. The `just pilot` command includes build/smoke, so its
386.43-second command wall time is distinct from the reported 291.4-second compile. All stage
measurements are in the raw pilot logs. These were shared-host qualification runs, not matched
causal performance trials. Two warm fake whole compiles ran concurrently against the deployed cache, each with a fresh
store and a shared generation root, overlapping the full gate. **Passed:** 321.4/306.3 seconds,
extraction 104.2/100.1 seconds, peak process RSS 4,102,772/4,197,916 KiB. Each read all 2,463 keys
from PostgreSQL, retained 40,353,792 payload bytes, and produced the same content digest and all
51 IPC files byte-identical to the cold fake pilot ([comparison](raw/postgresql-concurrent-parity.log)).
Their operation-vector reads took 0.291/0.284 seconds; these warm reads do not replace the separate
cold distinct-winner race/contended admission tests.

Old/new fake routes agree on **all 2,443 operation and 20 brief full request keys and exact
Float32 bytes** ([comparison](raw/postgresql-vector-parity.log)). The old and new content digests
differ because the compiler/schema identity and receipt recipe changed, as required.

The cold live snapshot `acbac322a45dbdafdb0b8c5c9ff303f9` has content digest
`c90b558725e8894ff8f2551847ec5e3dfcad1ae061fb5c6d17835ae31f43562c`.
The warm live snapshot `1c9a1fdfecf7d3407923024ad22247db` has the **same content digest and all
51 IPC files byte-identical**; manifest execution IDs differ intentionally
([comparison](raw/postgresql-warm-parity.log)).

With an unavailable database configuration, `lctx bundle` from the cold live snapshot reproduced
**all 52 files, including the manifest, byte for byte** ([replay](raw/postgresql-live-replay.log)).
The real-PG fixture separately stops the actual container before the same all-file replay, then
restarts and verifies persisted cache entries. Replay does not reconstruct vectors from a provider.

The owned `scripts/embed_serve.py` launch used the pinned Qwen revision/dtype/tokenizer spec.
Workspace-filtered `live_conformance_vectors` and `scripts/embed_conformance.py` **passed**, worst
Rust/Python cosine **0.9999334** ([conformance](raw/postgresql-live-conformance.log)).
`python -m lctx_mcp.smoke` **passed** on the live generation. The owned service was stopped after
qualification; no pre-existing embedding service was disrupted. This is controlled-launch W16
evidence, not attestation of arbitrary third-party endpoints. Native/evidence known answers are
covered by the full fixture/Python suite; Stage 3 question-set acceptance remains separate.

## Recovery, import and operational authority

The whole CLI was exercised with two canonical stores sharing one generation root, missing start
records, repeated reconciliation, a moved generation, and a temporarily moved-away source store.
**Passed:** unrelated-store generations were ignored; recovered start/library stayed unknown;
old discovery became unavailable; durable history remained; restored canonical paths reconciled
again ([output](raw/postgresql-cli-recovery.log)). Direct PG tests additionally prove a discovery
newer than the reconciliation cutoff survives, and unfinished is not automatically interrupted.
Legacy import tests use pinned Delta reads, reconstruct exact original request text, re-tokenize,
compare all admitted bytes, retry a partial import, and reject conflicting existing winners.

`uv run python scripts/postgres_backup.py backup .../populated.dump`, followed by
`restore-drill .../populated.dump`, **passed**. Backup and row fingerprints share one exported
repeatable-read snapshot while another compile can append events. The isolated restore retained
both migrations, 2 specs, **4,926 cache entries, 5 attempts, 472 events, 4 snapshot discoveries and
5 generation discoveries**, with matching fingerprints for every table. Restore took **4.984 s**
([receipt](raw/postgresql-restore-populated.log)), within the 15-minute target. Archives/configs
remain protected local artifacts; no credentials, database archive or request/vector payload is
committed. UTC fingerprint rendering corrected the first empty-database cross-timezone mismatch.
Daily/off-host backup execution remains the documented operator procedure, not a newly installed
scheduler or a tested machine-loss/PITR guarantee.

Rollback control **passed** using the retained prior binary and its own fresh prior-format
fixture store, publishing snapshot `00bbaad75d125cfdfcb4ab3f37993a37` and generation
`67171f3ed8d30645`. New receipt stores/history stayed intact. This rehearses the code/store pair;
it does not downgrade the installed database or reverse-migrate canonical facts.

## Cache cost and resource receipt

Measurements use a real durable PG18.6 container, exact 4,096-dimensional Float32 vectors,
bounded 32-row inserts/128-key reads, and one/four concurrent clients with six connections each.
Clients in each case compete for the same keys. The four cases use separate specs in one database;
reported total database sizes therefore accumulate, while WAL deltas are per case. Transferred
vector payload per client is 40,353,792 bytes (2,463 keys) or 409,600,000 bytes (25,000 keys),
excluding protocol/key/spec overhead and the winner readback. Wire-level total bytes are not
measured. These are exact-key database tests, not embedding computation benchmarks.

| Keys | Clients | Cold admission (s) | Warm read (s) | WAL delta (bytes) | Total DB bytes after case |
|---|---|---|---|---|---|
| 2,463 | 1 | 0.773 | 0.262 | 44,530,888 | 51,050,175 |
| 2,463 | 4 | 2.361 | 0.962 | 46,599,600 | 95,794,879 |
| 25,000 | 1 | 7.543 | 2.831 | 451,964,608 | 530,495,167 |
| 25,000 | 4 | 21.866 | 9.771 | 470,127,760 | 978,925,247 |

These measurements came from the first assembled PG run (7/8 tests passed; unrelated restart
fixture failed). Both the earlier and final measurements are retained. The final gate repeat also passed all preregistered small-case budgets:

| Keys | Clients | Cold (s) | Warm (s) | WAL delta (bytes) | Total DB bytes |
|---|---|---|---|---|---|
| 2,463 | 1 | 0.757 | 0.285 | 44,530,888 | 51,050,175 |
| 2,463 | 4 | 2.686 | 1.370 | 45,724,232 | 95,000,255 |
| 25,000 | 1 | 10.668 | 4.829 | 451,955,408 | 529,700,543 |
| 25,000 | 4 | 37.164 | 14.970 | 878,192,608 | 966,869,519 |

Largest observed container cgroup `memory.peak` was **2,239,729,664 bytes (2.09 GiB)** during
this repeat ([memory receipt](raw/postgresql-database-memory.json)). A one-second polling loop
read the kernel high-water value from live containers using the exact image; seven containers
were observed, including the long cost workload. It includes all backends and file cache and
is not private backend RSS or the installed host cluster's footprint. Fast containers can end
between polls. Cost variation under the shared host and checkpoint/WAL behavior is visible;
no extrapolated throughput or isolated performance superiority is claimed.
At larger scales the receipt payload itself can exceed the default 256 MiB limit; configure an
explicit qualified bound or implement the conditional spill/retention work before claiming scale.

## Review and remaining boundaries

The [assembled review](../../reviews/design_review_postgresql-change_2026-09-27.md) is
**Accept scoped** for source ownership/composition. Its corrections to recovery, retry, bounded
import, consumer attribution and optional journaling were implemented before qualification.
This runtime evidence is separate from the review's source cutoff and its `not_run` statements.
F1–F10 remain consumer-triggered proposals: no direct Python DB package, ORM, dynamic query AST,
pgvector, Arrow federation, notification scheduler or pgrx dependency was installed. Remote TLS,
HA/PITR and arbitrary-endpoint attestation remain outside the locally deployed scope.
