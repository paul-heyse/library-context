# Status

_Updated 2026-09-30 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Phase 3 design

- **Proposed:** the [detailed Phase 3 plan](docs/plans/semantic-model-phase3-detailed-plan_2026-09-30.md)
  specifies normalized entities/resolutions, effective callables, complete events, policy memberships,
  binding applicability/admission, projections, scoped coverage and dependency-ordered packages.
- Proposed ADR-0101/0102 cover cumulative self-contained generations with completed-stage reads,
  and typed normalized semantic ownership. DESIGN §15 links the proposals. No production model,
  dependency pin or runtime database changed in this design scope.
- [Design evidence](docs/design_review/evidence/2026-09-30_phase3-design/README.md) records exact
  resolved library features and 13 existing reader/runtime tests. These establish the baseline,
  not completed-stage reads or normalized compilation. Plan §11 owns new design findings;
  the parent cutover plan §8 retains cross-phase implementation dispositions.
- [Independent review](docs/design_review/reviews/design_review_phase3-plan_2026-09-30.md):
  **Accept scoped**, at Proposed strength; both design findings corrected and reinspected.
  Their N5/Q implementation and independent controls remain not_run.

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

## P0–P2 qualification (historical receipt, 2026-09-30)

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

## Review evidence policy

The [review binding](docs/design_review/design_principles/binding/library-context.md#reviews-in-this-repository)
explicitly leaves static review versus optional probes to the reviewing agent's judgment of
complexity, criticality and uncertainty. AGENTS.md, the skill and template carry the clarification;
implementation acceptance checks and Tested/Measured evidence requirements remain in force.
The earlier documentation-policy check failed on a stale ADR index; the Phase 3 design check
regenerated the index and passed. The unrelated generic skill-validator rejection of existing
`model-baseline`/`user-invocable` metadata was not repaired or claimed verified by this work.

## Phase 3 design checks (2026-09-30)

| Command | Outcome |
|---|---|
| `uv run python docs/design_review/evidence/2026-09-30_phase3-design/inspect_dependencies.py` | passed: locked/offline metadata, 24 selected resolved packages |
| `python3 scripts/build_environment.py -- cargo test --release -p cpg-core --test model_runtime --test generation_read` | passed: 13 existing tests, including real PostgreSQL readers |
| `just adr index`; `just adr lint` | passed: 56 records |
| `just docs-check`; `git diff --check` | passed for the design documents; no product qualification implied |
| `uv run python scripts/library_utilization.py`; `just library-catalog` | passed: no catalog drift; refreshed 65-library/91-capability usage index under the supplied end-of-session workflow |
| Phase 3 implementation, `just test-all`, pilots, product journeys | not_run: this task produces the detailed design; P0–P2 receipts above retain their scope |

## Next work

Resolve proposed ADR-0101/0102 at the implementation boundary (D0), then implement the detailed
plan's R1 → R2 → R3: frontier descriptors, atomic completed-stage/facts checkpoints, source-bound
readers and resource/connection admission. Review that assembled boundary before N1–N7.
Provider-session F04/F07 and store lineage F08 remain implementation obligations, not design closures.
P4 still owns the composition engine, attributed derived type/guard claims and dispatch policies;
P5 owns serving and product qualification. Product PR6/new features remain paused until phase 5.
