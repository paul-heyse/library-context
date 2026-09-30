# Status

_Updated 2026-09-30 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Semantic model cutover

- **Implemented and Tested:** remaining phase 0–phase 2 scope, including P1.13, A10–A16,
  B1–B3, facts CLI, C1x–C3x and assembled Q. The [cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md)
  owns execution receipts (§4.2), dormant deletion obligations (§5–§6) and findings (§8).
- `lctx-model::domain` owns the typed contract; immutable PostgreSQL generations own facts.
  Native syntax/symbol/call/type/flow/document/deployment producers use declared stage sinks,
  shared validation and facts admission. No old-ID bridge or dual store was introduced.
- `lctx compile fastmcp --through facts --profile catalog|behavioral` publishes without selection.
  Catalog Flow is NotRequested; behavioral Flow is requested. Partial and unavailable evidence
  retain boundaries. Other frontiers, analysis/catalog compilation and serving remain unavailable.
- Native qualification repairs: ADR-0095 preserves positional order; ADR-0096/0098 retain
  unavailable expanded signatures/slots; ADR-0097 separates dictionary keys; ADR-0099 preserves
  Unicode/NUL as typed UTF-8 bytes; ADR-0100 retains residual child ports with DisplayOnly closure.
  ADR-0094 makes the generation registry the sole owner of new compile outcomes.
- Model digest: `dad6dc7983136391c358394dfd3befb7a23aaf30e42719b8e0b57f513a6ed0ef`.
  Schema snapshot diffs were read before acceptance; this is a rebuilt schema migration.
- [Exit review](docs/design_review/reviews/design_review_p0-p2-exit_2026-09-30.md): Accept scoped for the P0–P2 facts frontier; resource and downstream exclusions remain explicit.
  [Qualification evidence](docs/design_review/evidence/2026-09-30_facts-qualification/README.md)
  records commands, composite repairs, profiles, repeated content and resource limits.

## Verification (2026-09-30)

Cargo commands use `python3 scripts/build_environment.py --`; detailed focused receipts: plan §4.2.

| Command | Outcome |
|---|---|
| `just fmt`; `just test-all` | passed: 674 Rust/11 skipped; 219 Python/56 skipped; 217 in the PostgreSQL repeat suite/one test+one binary skipped; doctests and remaining gate components |
| `uv run python docs/design_review/evidence/2026-09-30_facts-qualification/pilots.py` | passed: catalog 474.317s, behavioral 628.956s, repeat 669.475s; repeated behavioral content identical; published without selection |
| `uv run python .../refusals.py`; `uv run python .../measurements.py` | passed: capture/provider resource refusals, unchanged registry/store check; three-profile stage/RSS measurements and leased input queries |
| `uv run pytest tests/scripts/test_flow_soundness.py -q` | passed: 18 independent oracle tests |
| `just docs-check` | passed: 226 canonical pages, zero link errors |
| `just library-catalog` | passed: regenerated current catalog/usage index (65 libraries, 91 capabilities) |
| Served-product evaluation, held-out confirmation and performance comparison | not_run: phases 3–5 unavailable |

The complete gate is a composite receipt after initial formatting, lint, snapshot, rule and native
pilot failures were corrected. Skips are explicit dormant/measurement controls. An integration
receipt does not establish retrieval accuracy or complete runtime semantics. Reservations admit
retained typed state; native parse/solver heaps, allocator retention, transient adapter collections,
SQLx high-water and sizing calibration remain named allowances rather than an absolute RSS cap.

## Retirement and preservation

- Real PostgreSQL transition passed; four retained service fingerprints stayed equal, archive
  removed, protected 22812-byte mode 0600 service dump retained. [Receipt](docs/design_review/evidence/2026-09-29_operator-transition/README.md).
- The measured duplicate behavioral confirmation generation was retired after qualification;
  catalog and primary behavioral facts remain published/unselected. Its receipt and final store check passed.
- Nine exact obsolete runtime paths and 960 stale generated `_lctx_blocks` files were removed
  after scoped inventory. Locked sources, environments, backups and benchmark captures remain.
- `cpg-schema` and lower legacy APIs survive only for inventoried dormant P3–P5 consumers.
  Native extraction/flow production dependencies no longer include that crate. Retained old
  independent answer obligations have Git recovery anchors; no dormant suite is claimed active.
- Concurrent reviewer/settings/model-alias and optional-skill guidance changes remain preserved;
  they are outside this implementation's commits. ADR-0093 policy remains current. Prior generic
  skill-validation metadata failures were not changed or claimed repaired by the cutover gate.

## Next work

Begin phase 3 normalized relations. Read DESIGN §15's owner and the cutover plan's phase 3 scope,
then move each dormant consumer at its ownership boundary and delete its legacy declaration.
Before introducing P3/P4 stage readers, resolve provider-session F04/F07 and store lineage F08.
P4 still owns the composition engine, attributed derived type/guard claims and dispatch policies;
P5 owns serving and product qualification. Product PR6/new features remain paused until phase 5.
