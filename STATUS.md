# Status

_Updated 2026-09-30 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Phase 3 implementation

- **Accepted target:** [ADR-0101](docs/adr/0101-cumulative-normalized-generations.md) and
  [ADR-0102](docs/adr/0102-normalized-semantic-ownership.md). The
  [detailed execution plan](docs/plans/semantic-model-phase3-detailed-plan_2026-09-30.md) §11
  owns package receipts and finding disposition; the parent owns cross-phase obligations.
- **Implemented / focused-Tested:** R1–R3 frontier descriptors, atomic stage completion and
  private facts checkpoint, source-bound readers, invariant eligibility, scoped availability,
  shared resource and physical-scan admission, terminal drain/close. Fork pin `a41da22`.
  [Foundation review](docs/design_review/reviews/design_review_phase3-foundation_2026-09-30.md):
  Accept scoped after F01/F02/O1 correction and reinspection.
- **Implemented / focused-Tested:** N1 entities, total symbol correspondence, owner materialization,
  parameter/field links and public exposures, charged store-read stage and shared exact-output
  validation; native origin distinguishes callable source fields from synthesis. Store rebuild due Q.
- **Implemented / focused-Tested:** N2 total lexical/import/ancestry/mention/type/binder/place/
  test-operand relations, charged driver and shared exact-output validation. Native and pure
  controls preserve module coordinates, missing outcomes and exact role/context linkage;
  real PostgreSQL controls passed for both profiles. Catalog relies on validated NotRequested
  flow coverage. Normalized publication remains unavailable until N7.
- **Implemented / focused-Tested:** N3 effective callable/body authority and raw signature slots;
  N4 events/policies/views/flow links; N5 stored bindings, total variant sets and private admission.
  Pure/native and both PostgreSQL profile controls passed; plan §11 owns exact composite receipts.
- [N1–N5 review](docs/design_review/reviews/design_review_phase3-normalization_2026-09-30.md): Accept scoped after F01/F02 corrections and focused controls.
  N6–N7/X/Q remain. Formatting, lint, integrated gate, pilots and phase exit are not_run until
  all functional scope is implemented; Q must measure complete charged index/output/replay residency.

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
- The historical P0–P2 schema snapshots were reviewed and accepted. Phase 3 changes the schema;
  its final snapshot acceptance and store rebuild are still part of Q.
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

## Current implementation checks (2026-09-30)

Cargo commands below run through `python3 scripts/build_environment.py --`.

| Command / scope | Outcome |
|---|---|
| R1–R3 release `domain_admission`, `domain_memory`, `domain_resources`, `domain_stages`, `generation_stages`, `stage_reads`, `stage_checkpoint`, `generation_read`, `model_runtime`, `stage_validation` | passed; exact commands/counts in detailed plan §11 |
| Fork release `bound_lifecycle` at `a41da22` | passed: three real PostgreSQL terminal-close/cancellation/drain controls |
| `cargo check -p cpg-core` | passed: N1 and initial N2 compile checks |
| Release `normalized_entities`, `domain_symbols`, `typed_symbols`, `domain_owner` | passed: 4, 11, 5 and 2 focused tests |
| Release `normalized_stage` | passed: N1 then N1→N2 real store controls, including validation and release of retained resources |
| Release `cpg-extract --test normalized_relations`; `cpg-core --test normalized_relations`; `lctx-model --test domain_normalized` | passed: 7 native controls, 2 real store profiles and 3 pure/codec controls |
| Release N3–N5 native/pure suites and core `normalized_relations` through N5 | passed: both PG profiles, policy views, binding/admission and review correction controls; plan §11 |
| `just fmt`, `just test-all`, schema snapshot acceptance, Phase 3 pilots/measurements, final review/docs/catalog | not_run: functional implementation is ongoing |

Initial focused failures exposed facts-fixture scope, retained capture accounting and catalog's
unrequested flow source; those were corrected. Remaining owner/fixture scope adjustments are
part of implementation and Q, not evidence that the full gate currently passes.

## Next work

Implement N6 projections, then N7 normalized publication,
legacy retirement and whole-scope qualification. P4 owns composition/analysis/catalog; P5 owns
serving and product qualification. Product PR6/new features remain paused until phase 5.
