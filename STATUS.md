# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and checkpoint

- **Increment 4 / Stage 3 remains functionally incomplete.**
  [Forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
  owns P0–P7 execution and the frozen exit rule; §6 owns finding disposition.
- **P0 done:** `2c6a59c` preregistration, `50969ba` baseline diagnostic, `ddcc149` native limits,
  `8a7c936`/`a8f6b49` descriptor exemption. AMBIGUOUS reachability remains withheld by design.
- **P1 implemented; change review to settle:** ADR-0064 (`9263af8`), decorated-summary refusal
  (`220a0c8`), claim-keyed discharge and native citation admission (`d8fa5f5`). The existing
  `design_review_stage3-discharge-change_2026-09-27.md` is preserved as concurrent work.
- **P2 partly done:** `8c3ca5c` compile-fixture/CPython harness; original W5/W7/W12 default-cap
  traces remain open. P3's additional pinned Logger models landed in `0359548`.
- **PostgreSQL PG0–PG7 implemented, deployed and qualified:** SQLx cache, exact snapshot-local
  vector receipts, attempt/event history and reconciled discovery; PG18.6 local database/roles,
  two explicit migrations, protected config, backup/restore and operational CLI.
  [Plan](docs/plans/postgresql-integration-plan_2026-09-27.md),
  [runbook](docs/postgresql.md), [evidence](docs/design_review/evidence/2026-09-27_postgresql/README.md).
  ADR-0065/0066/0067 are accepted; F1–F10 remain consumer-triggered future capabilities.
- **Versions:** compiler **106**, extractor **33**, template **20**, catalog **7**, FORMAT **10**.
  `used_embeddings`/`embedding_uses` are schema migrations; use fresh stores under ADR-0048.
  Prior binary/lockfile and baseline artifacts remain under `build/postgresql-baseline-5e62353/`.

## Last verified (2026-09-27)

`CARGO_TARGET_DIR=/home/paul/library-context/target`, `RUST_MIN_STACK=16777216`.

| Command | Outcome and scope |
|---|---|
| `just test-all` | **passed:** 430 ordinary Rust tests, 8 real PG18 tests, native fixture, 154 Python tests, strict fmt/Clippy/Ruff/Pyrefly, rules, 37 ADRs/agents, 83 fixture parses, deps/gold and fresh-schema SQLx metadata |
| `just pilot build/postgresql-pilot-store build/postgresql-pilot-fake.log` | **passed:** fresh fake pilot and MCP smoke, 20 briefs |
| Live `lctx compile`, Rust/Python conformance, MCP smoke | **passed:** controlled pinned launch, worst cosine 0.9999334; warm compile succeeds after embedder shutdown |
| Offline `lctx bundle`; two concurrent compiles; restore/rollback controls | **passed:** live bundle all 52 files byte-equal; concurrent/cold IPC and content parity; 472 backed-up events restored; prior binary/store works |
| Bogus-DSN offline build; stale `.sqlx`; credential/permission/TLS controls | **passed:** offline build and expected negative-control rejections |
| `lctx db migrate/check/status` | **passed:** 180006, `lctx_app`, current schema; migration is idempotent |
| `just docs-check`; scoped same publisher | **failed:** supplied external review lacks H1; **passed:** 146 canonical pages excluding only that unchanged input |

## Known boundaries and next

- PostgreSQL acceptance does not complete Stage 3. The [forward-plan findings](docs/plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
  own PGS/PGK dispositions; W9 is tested closed for its cache defect, while arbitrary endpoint
  attestation stays deferred under W16. No Python database client or future search stack is installed.
- A documentation-only brief for an undocumented, analysis-free seed fails
  `semantic:documentation-only-has-outcome` (fail-closed); generated packages only, unscheduled.
- Resume the P1 change-review disposition, then P2.3 default-cap traces and remaining P3/P4
  channel contracts: CallableFormal/CallableEntry with the F08 escape premise, `summary_effects`
  and the multi-channel worklist. The Stage 3 packet, clean-wheel query and exit assessment remain open.
- Operator: use the runbook's daily/pre-upgrade protected backups; owned qualification vLLM is stopped.
