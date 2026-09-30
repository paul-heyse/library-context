# Operator database transition rehearsal (cutover plan P1.13)

**Consumer:** plan §4.1.1 row P1.13 and §4.2; `scripts/postgres_transition.py`.

**Question.** Does the transition move the operator database, which carries the pre-baseline
migration history, onto the service baseline without losing its retained service rows?

## The operator database (read-only `plan`, 2026-09-29)

- Database `lctx`, 845 MB, with no connections.
- Migration history 202609270001–202609280012: 12 legacy versions.
- Retained service tables `lctx_cache.specs`, `lctx_cache.embedding_values`, `lctx_ops.attempts`
  and `lctx_ops.events`: **0 rows each**. The fingerprint is the empty digest
  `d41d8cd98f00b204e9800998ecf8427e`.
- Left behind in the archive: `lctx_report` (empty) and `lctx_serving` (about 1 GB of pilot
  projections, dormant until cutover phase 5).

## Rehearsal (`rehearse.py`, 2026-09-29, HEAD c65c0c7)

1. The operator database was dumped by its owner, which is read-only on the source: the retained
   services, `lctx_report` and `public._sqlx_migrations` with their rows, and `lctx_serving`
   schema-only (286 KB in all).
2. The dump was restored into a disposable pinned PG18 provisioned by the real bootstrap SQL.
3. The transition ran against the release `lctx` at c65c0c7.

| Step | Outcome |
|---|---|
| `lctx store install` on the legacy database | refused, exit 1 (legacy history) |
| `plan` | exit 0; legacy history reported; `lctx_serving` left behind |
| `prepare` | exit 0: `lctx_next` created, `store install` ran there, retained rows copied, fingerprints equal |
| `switch --confirm-switch lctx` | exit 0: archive `lctx_retired_20260930001329`; retained fingerprints equal |
| `lctx store check` on the switched database | exit 0 (clean) |
| `lctx runs list` | exit 0 |

Evidence status: **Tested** on a disposable copy. The integrated control is
`tests/scripts/test_postgres_transition.py`, which uses a legacy shape with rows.

## The real run

It is **blocked** on PostgreSQL superuser access. `prepare` creates a database and `switch` renames
two, and this session's `sudo -u postgres` needs a password. The operator authorized the
transition. The operator runs, in a terminal where sudo can prompt:

```sh
uv run python scripts/postgres_transition.py plan
uv run python scripts/postgres_transition.py prepare --port 5432
uv run python scripts/postgres_transition.py switch --confirm-switch lctx
target/release/lctx store check && target/release/lctx runs list
# later, separately:
uv run python scripts/postgres_transition.py drop-retired --confirm-drop lctx_retired_<stamp>
```
