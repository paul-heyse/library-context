# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and checkpoint

- **Increment 4 / Stage 3 remains functionally incomplete.** [Forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) owns P0–P7 and the frozen exit rule; §6 owns finding disposition.
- **P0 done:** preregistration, baseline diagnostic, native limits and descriptor exemption.
  AMBIGUOUS reachability remains withheld by design.
- **P1 implemented; change review to settle:** ADR-0064, decorated-summary refusal, claim-keyed
  discharge and native citation admission. The existing Stage 3 discharge review is concurrent work.
- **P2 partly done:** compile-fixture/CPython harness; original W5/W7/W12 default-cap traces remain
  open. P3 includes the additional pinned Logger models.
- **PostgreSQL PG8–PG11 complete:** `de2343c` decisions/baseline; `1800348` implementation;
  `fda9297` async qualification. Standard 1024/spec format 2, shared Rust projection contracts,
  COPY/Arrow codecs, pure semantic wheel and async storage wheel, maintained provider fork,
  PG18.6/pgvector 0.8.6, separate roles/credentials and four explicit migrations are deployed.
  [Plan](docs/plans/postgresql-integration-plan_2026-09-27.md), [runbook](docs/postgresql.md),
  [bounded evidence](docs/design_review/evidence/2026-09-27_postgresql-expansion/implementation.md).
- **Architecture:** ADR-0068 governs expanded serving/spec/effects; ADR-0067 retains canonical
  Delta/receipt authority. [Fresh review](docs/design_review/reviews/design_review_postgresql-foundations_2026-09-27.md): **Accept scoped** after identity/empty-codec corrections.
- **Versions:** compiler **107**, extractor **33**, template **20**, catalog **7**, FORMAT **11**.
  Spec/projection migration requires fresh outputs. PG7 rollback binary/locks/protected config/dump
  remain under `build/postgresql-pg8-baseline-6403b60/`; old readers use a separately restored PG7 DB.

## Last verified (2026-09-27)

Checks used `/home/paul/library-context/target`. Formatting and broad checks followed functional completion.
The full no-fail-fast Rust run retained 431 passes; all six migration-related failures passed targeted repair runs.

| Command | Outcome and scope |
|---|---|
| `just test-all` plus targeted repairs and remaining recipes | **passed across retained/repair receipts:** 437 ordinary Rust tests, 11 real PG tests, one async PG test, fresh native fixture, 164 Python tests, strict fmt/Clippy/Ruff/Pyrefly, rules, agents, 83 fixture parses, deps/gold and fresh-schema SQLx metadata |
| `just pilot build/postgresql-pg11-pilot-store build/pg11-pilot.log` | **passed:** fresh fake pilot, 143.0 s; MCP smoke, 20 briefs |
| Controlled 1024 live compile and Rust/Python conformance | **passed:** cosine ≥0.9999176; warm compile succeeds with the owned endpoint stopped; exact consumed-vector receipts agree |
| Offline `lctx bundle`; `projection_probe` 1×/10× | **passed:** all 52 replay files byte-equal; measured COPY codec peak 376,608 KiB below the registered 512 MiB ceiling; repeated-row transport workload only |
| PG7/expanded restore drills; `lctx db migrate/check/status` | **passed:** table fingerprints agree; local server 180006, `lctx_app`, exact current migration history |
| Bogus-DSN offline build; deliberate stale `.sqlx` control | **passed:** full graph builds offline; fresh-schema checker rejects the changed query; original metadata restored |
| `just adr lint`; `just docs-check`; scoped publisher | **passed:** 36 ADRs. **failed:** supplied external review lacks H1; **passed:** 150 canonical pages excluding only that unchanged input |

## Known boundaries and next

- **Next PostgreSQL step: PG12** production COPY import, partitions/index build, validation and atomic
  readiness. PG13 owns exact query cutover, PG14 retrieval, PG15 admitted federation, PG16/17 operations
  and assembled acceptance. The file MCP remains the reference consumer; PostgreSQL projections
  have contracts/schema/codec foundations, not production imported generations or ANN routing.
- [Forward-plan §6.1](docs/plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings) owns PGS/PGK/PGE/PGF disposition. PG9 and PGF corrections are tested closed; broader serving findings remain open. W9 stays closed; arbitrary endpoint attestation remains W16 work.
- PostgreSQL acceptance does not complete Stage 3. Resume P1 review disposition, then P2.3 default-cap
  traces and P3/P4 channels. The semantic packet, clean-wheel query and Stage 3 exit remain open.
- An undocumented, analysis-free documentation seed fails `semantic:documentation-only-has-outcome`
  (fail-closed); generated packages only, unscheduled.
- Operator: retain protected backups; this task's vLLM is stopped. Unrelated Stage 3 review and
  external-review input remain preserved and uncommitted.
