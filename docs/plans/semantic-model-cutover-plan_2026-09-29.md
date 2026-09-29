# Plan: the semantic model cutover

**Active execution owner, 2026-09-29.** [ADR-0085](../adr/0085-typed-semantic-domain.md),
[ADR-0086](../adr/0086-immutable-postgresql-generations.md), and
[ADR-0087](../adr/0087-clean-semantic-reconstruction.md) govern this plan.
The target is [DESIGN §15](../design/sections/semantic-model.md).

## 1. Outcome and completion

Ordinary typed Rust domain definitions in `lctx-model` own atomic units, attributes, relationships,
identity and invariants. A bounded derive lowers those definitions to Arrow and metadata;
`lctx-postgres` lowers the validated model to PostgreSQL. PostgreSQL is the only relational store.
DataFusion is compute, not another schema authority. There are no compatibility readers, adapters,
legacy identity maps, old-store flags, or dual writers.

**Authorized scope:** reconstruct phases 0–2. Analysis, catalog and MCP availability is suspended
until phases 3–5 implement those capabilities on the same model. A facts-only generation advertises
that frontier and cannot answer higher-layer requests with empty tables. Product work stays paused.
The forward plan retains product context and unrelated findings.

**Qualification:** compile checks and focused tests during execution; formatting and the integrated
active-scope gate once all phase 0–2 functional work is implemented. A phase's decision is not evidence
of implementation. PostgreSQL qualification uses a real disposable server, including concurrency.
Independent semantic fixtures, oracles, benchmark assets and held-out isolation are preserved.

## 2. Baseline and reopened work

Baseline `a265758` contains an empty model membership registry, relation metadata separate from row
definitions, untyped IDs, mutable table specifications, partition publication and compatibility
machinery. The earlier phase-0 completion claim is **reopened**. The
[core review](../design_review/reviews/design_review_cutover-core_2026-09-29.md) is Revise.
Existing code is reusable only where it meets the new contract; it is not architectural authority.
The historical baseline had 57 raw families, 22 normalized families and 121 analysis/catalog families.
Every raw field must be mapped to a current typed owner before its source is deleted.

## 3. Global mechanisms

### 3.1 Typed domain ownership

`Id<T>` distinguishes targets. Entity keys, qualified assertion keys and support/run keys have
separate meanings. One generated typed key supplies equality, hashing and ID derivation. Conditions,
modality and approximation distinguish assertions; repeated supports never strengthen a conclusion.
Conflicting payload for an equal key is rejected unless an explicit domain merge owns it.
Field type, key participation, reference role and provenance are orthogonal. Collections of references
become relationship rows. Tagged sums enforce the active payload, absent inactive arms, and the
difference between an absent arm and a present optional value. Subtype references enforce the subtype.

A root manifest declares membership once. Only a privately constructed `ValidatedModel` admits
storage and execution. Generated declarations, Arrow schemas/codecs and relation inventories derive
from domain definitions. The derive handles actual shapes rather than becoming a general DSL.
`serde_arrow` 0.15.1 uses explicit schemas and Arrow 59; SeaQuery 1.0.2 lowers SQL in the store owner.
Salsa remains internal to ty 0.28.2. No new incremental or inference engine is introduced without a
consumer. SQLx, pgpq, the provider fork and the bounded BDD kernel keep their narrow responsibilities.

### 3.2 Immutable generation schemas

Each generation owns ordinary tables in its own schema. A stable control schema owns state,
manifests, receipts and selection. There are no parent partitions or attach/detach transitions.
Keys and foreign keys are generation-qualified; local cyclic references are supported by creating
tables and keys before references. Semantic and physical schema digests are distinct from producer
and content digests.

Lifecycle: staging → sealed → validated → published; failed and retired are terminal. Selection is a
separate pointer. Sealing waits for writes and removes writer access before stored-content validation.
A receipt binds sealed content to the complete required validator set. Publication changes state and
grants atomically. A reader validates model/schema identity and holds a lease for its lifetime.
Retirement requires an unselected generation and exclusive access, then removes schema and registry
state atomically. Lost leases invalidate readers. Failed attempts are never repaired in place.

### 3.3 Typed stage execution

Stage input/output declarations alone own production; relation definitions do not duplicate writers.
Model membership, writer uniqueness, required inputs, read-before-write and cycles (including self
cycles) are checked. Provider outcomes are Complete, Partial, Failed or NotRequested. Output assembly
preserves attribution and alternatives; consensus belongs to normalization. Each DataFusion stage
sees only declared inputs and shares one attempt memory pool. Boundaries accept `Batch<R>`.

## 4. Phases

<a id="phase-0--core-contracts-store-kernel-and-migration-tooling"></a>
### Phase 0 — Typed domain and executable contracts

| Package | Implementation and owner | Acceptance |
|---|---|---|
| P0.1 | Replace governing decisions and this plan; map raw fields to model owners | No active compatibility target; no dropped raw information |
| P0.2 | Domain groups: package/release/source/module; syntax/declaration; provider/context/run/assertion/support/coverage; call alternatives; type terms; flow/places/predicates/conditions; documents/package/deployment | Nominal references; acyclic source/occurrence keys; support separated from assertions |
| P0.3 | Bounded `lctx-model-macros` derive, generated key/schema/codec, root membership, `ValidatedModel` | Positive and compile-fail controls; invalid sums/subtypes/duplicate keys refused |
| P0.4 | Correct calls, structured paths, atom identity, transfer and verdict contracts | All C03–C13 controls below, retaining independent expected answers |
| P0.5 | Typed stages, provider outcome contracts, restricted sessions | Missing/double writer, unknown input, self-cycle, failure/not-requested controls |
| P0.6 | Production subset source → module → occurrence → assertion → support/coverage through permanent lowerings | Typed → Arrow → real PostgreSQL COPY → typed readback; fresh design review before scaling |

P0.4 requires structural length-tagged paths; occurrence-based atoms (including distinct with-items);
separate potential higher-order and direct invocation; one owner for receiver classification and call
normalization; complete target alternatives rather than row-count uniqueness. Transfer keys exclude
accumulating conditions. Composition extends paths only through identity, maps all bindings, preserves
local opaque guards and refuses unsupported substitutions. ControlInfluence and Selection are first-
class relationships. ScopeBoundary yields unknown; value approximation creates an obligation;
rendering truncation is not semantic uncertainty. Derivation premises are typed references.

### Phase 1 — PostgreSQL store and runtime cutover

| Package | Implementation and owner | Acceptance |
|---|---|---|
| P1.1 | `lctx-postgres` generated generation DDL and control catalog | Only validated model accepted; cyclic refs, codebooks, tagged sums and keys enforced |
| P1.2 | Seal/validate/publish/fail/select/retire lifecycle, COPY and receipts | Real concurrency controls: late writes, failed validation, failed publication, lease/retirement race |
| P1.3 | `cpg-core` provider sessions with pinned lease and digests | Readback fidelity; pinned read survives a different generation selection; digest mismatch rejected |
| P1.4 | Quiesce old project runtime; remove Delta, partition kernel, bundles, adapters, legacy IDs and runnable downstream orchestration | No alternative store or compatibility flag; independent semantic expectations retained |
| P1.5 | CLI model describe, store install/reset, generation list/select/retire, database/generation query; dev/test PostgreSQL and pin policies | Disposable database lifecycle; supported commands expose actual availability |

Do not delete unrelated PostgreSQL operational services or protected evidence. No stage may turn a
suspended analysis into a fabricated empty result. Provider integration stays outside the Python
storage wheel. Retirement must not rely on DETACH CONCURRENTLY recovery.

### Phase 2 — Complete attributed facts

| Package | Implementation and owner | Acceptance |
|---|---|---|
| P2.1 | Finish every raw domain family and old-field disposition in model | All existing raw information preserved; no references to future L1 entities |
| P2.2 | Convert `cpg-extract` and `cpg-flow` to typed producer bundles | Provider indices confined locally; preserve pinned acquisition, shared Pyrefly/Ruff parse, ty second parse |
| P2.3 | Single assembly owner and indexed source/span/kind/role join | Scalar oracle agrees; names, attributes, subscripts, nested/same spans and `.py`/`.pyi` controls; ambiguous stays unresolved |
| P2.4 | Canonical atoms/BDD; structured paths; attributed support and coverage | Shuffled-input determinism; unknown/partial/not-requested distinguished; no independent DNF |
| P2.5 | `compile <library> --through facts --profile catalog\|behavioral --database …` | Required provider failure aborts; explicit partial facts disclose limits; actual availability frontier |
| P2.6 | Delete obsolete extraction schemas/hashes/conditions and finally `cpg-schema`; rebuild facts generations | Both profile facts pilots and fixture corpus; no old runtime copy; MCP remains unavailable |

Content equality excludes timestamps and measurements. Fresh recomputation is required; stage cache
optimization is deferred. Once all functional packages are done, run `just fmt`, updated
`just test-all` (including real store and compile-fail tests), facts pilots for both profiles,
`just docs-check`, assembled design review and handoff. Report suspended product/MCP gates separately
as not_run, not as restored product qualification.

### Phase 3 — Normalized relations

Restore entities, normalization, effective callables, signatures, bindings, policy views and program
projections using the same typed model. Independent semantic controls remain required.

### Phase 4 — Analysis and catalog

Restore transfers, obligations, summaries, behavioral evidence, catalog and retrieval. Findings remain
open until assembled consumers demonstrate the intended semantics. A removed implementation does not
retire its capability obligation.

### Phase 5 — Serving

Generate views and inventories from the model; restore native executor/MCP against one leased
published generation. No bundle import or copied serving schema. Qualify end-to-end journeys before
resuming PR6 or new features.

### 4.1 Execution decisions

ADR-0085–0087 supersede the earlier execution decisions. No compatibility, parity-to-bug requirement,
partition migration or upfront ablation is authorized. Independent behavior controls remain required.

### 4.2 Execution status

**In progress: P0.1–P0.3/P0.5/P0.6 foundations.** Replacement decisions and revised plan accepted.
Implemented a bounded derive for records, codebooks and tagged sums; nominal IDs, generated keys,
explicit-schema codecs, subtype references, validated membership and a typed stage schedule. The
production subset covers source/module/occurrence, provider/context/run, syntax support and coverage.
The generation store has ordinary schema lowering, COPY, seal/validate/publish, leased reads,
selection, retirement and abort cleanup. It is not connected to the production compiler or CLI.

Focused evidence (2026-09-29; commands prefixed by `python3 scripts/build_environment.py --`):

| Command | Outcome and boundary |
|---|---|
| `cargo test --release -p lctx-model --test domain` | passed: 10 focused domain controls, including sum null semantics and nominal subtype construction |
| `cargo test --release -p lctx-model --doc` | passed: five negative declarations with five positive partners; one pre-existing ignored legacy example |
| `cargo test --release -p lctx-postgres --test generations` | passed: real disposable PG18 typed source/support/coverage round trip; writer drain and revocation; stored reference and subtype refusal; pool-bound reader leases; retirement, selection, contract mismatch and failed-attempt cleanup |
| `just docs-check`; `uv run python scripts/adr.py lint` | passed: documentation publication and current decision references; not architecture qualification |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

The independent interim storage review found five defects. Nested options are now rejected,
unsupported digest arrays excluded, leases keep their pool permits, and failed attempts have atomic
cleanup. Model fingerprints include owned semantic/generator sources and dependency manifests;
external fixture declarations must supply semantic source bytes. Source inspection accepted the
corrections within this boundary; concurrent/cancelled abort and the new sum/subtype design still
require the assembled P0 review.

**Still open:** complete raw-field disposition and domain families; P0.4 semantic policies and indexed
join; stage runtime/session enforcement; bounded large-table reads; complete store/CLI commands;
producer migration; deletion and quiescence. The old pipeline and framework remain present, unwired
to the new generation path. No adapter between them has been added. No phase exit is qualified.
Earlier phase-0/WP1.0 receipts do not qualify this target.

## 5. Deletion and preservation obligations

Delete legacy code by ownership boundary after preserving raw-field mappings and independent controls.
No legacy adapter inventory is maintained. Preserve fixtures, oracles, protected benchmarks, evaluation
isolation and unrelated operational services. Git retains removed implementation. Old runtime readers
must be quiesced before replacing or retiring project state.

## 6. Deferred capability obligations

Phase 4 retains finite summaries/discharge, completion/evaluation, frame exits and call execution,
context protocols/values, modeled identities/actions, behavioral reachability, communities/PageRank,
FCA/RCA and optional kNN wherever their existing consumer remains. Retirement needs consumer evidence
and an explicit decision; deleting old code during reconstruction does not retire these obligations.

## 7. Tooling, pins, skills and documents

Phase 0 adds the bounded derive and explicit Arrow codec dependencies. Phase 1 removes Delta family
and skill selection and replaces store/CLI/PG test commands. Phase 2 updates fact owners and tests.
Model, PG and producer digests, generated declarations and SQLx metadata must be regenerated from their
owners. Phases 3–5 update their owners when implemented. No documentation labels target as Tested
without corresponding evidence.

## 8. Findings disposition

This table owns the current disposition of the review's findings. Each closes by construction in the
phase named, and only on its closure evidence.

| Review finding | Disposition | Responsible WP | Closure evidence |
|---|---|---|---|
| [F01](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F01) unknown default served as absent | open → phase 4 | 4.5 | tri-state evaluation; provider-constructor and factory fixtures `Unresolved` |
| [F02](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F02) call relation has no owner; origin hidden | open → phases 3–4 | 3.1, 3.5, 4.5 | five policy views; S1 as one policy edit; decorator-registration fixture shows its origin |
| [F03](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F03) association basis dropped | open → phase 4 | 4.5 | basis and origin required columns; candidate-only fixtures `Unresolved` |
| [F04](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F04) no transfer algebra; identity served as computed | open → phase 4 | 0.4, 4.2 | composition table; P0 `facade` renders `unchanged` |
| [F05](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F05) two condition representations | open → phases 2, 4, 5 | 2.2, 4.1, 5.2 | occurrence-keyed atoms; no parallel DNF; served `condition_id`; ty-upgrade identity control |
| [F06](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F06) no place/transfer vocabulary | open → phases 3–4 | 0.4, 3.1, 4.3, 4.5 | P0 rerun: plain-class option→field→reader and per-branch supply |
| [F07](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F07) distributed refusal/obligation; silent cost cap | open → phases 0, 4 | 0.4, 0.6, 4.3, 4.5 | one obligation owner; cost-cap control; budget reason; memory pool |
| [F08](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F08) no derived-result contract; lineage gaps | open → phase 4 | 4.6, 4.7 | one emitter; input invocations; finding-ID rule; derivation views |
| [F09](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F09) nominal projection layer | open → phases 3–4 | 3.2, 4.6 | projection by declaration; ADR-0044 amendment (made 2026-09-29) |
| [F10](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F10) serving is a second hand authority | open → phases 0, 5 | 0.2, 5.1, 5.5 | generated DDL, views and inventories; no hand serving file |
| [F11](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F11) implicit stage composition; producer identity mislabelled | open → phases 0–1 | 0.7, 1.2 | stage-table refusal controls; producer identity covers every canonical producer; the pipeline schedules from the table |
| [F12](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F12) per-request rebuilds and round trips | open → phase 5 | 5.1 | generation-scoped prepared catalog; set-based hydration |
| [F13](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F13) pilot recognizer; lexical parameters outside the digest | open → phases 4–5 | 4.4, 5.4 | named authored model; policy digest |
| Review observations: browse unknown ownership, `EmptyUnderCoverage`, lexical-only reason | open → phase 5 | 5.1 | disclosed in responses |

**Operator decision (2026-09-29).** Do not repair these in the legacy code. The new contracts must
make them unrepresentable.


### Core review findings

All findings from the [core review](../design_review/reviews/design_review_cutover-core_2026-09-29.md)
remain open; acceptance requires implementation plus focused evidence and the P0.6 review.

| Finding | Owner and closure |
|---|---|
| C01 | P1.2/P1.3: schema-per-generation retirement, reader lease and select/retire concurrency controls |
| C02 | P1.3: reader rejects model/physical digest mismatch before decoding |
| C03 | P0.5: typed model membership and sole stage writer authority, including self-cycle refusal |
| C04 | P0.4/P2/P3: attributed direct/potential/higher-order alternatives; one normalization owner |
| C05 | P0.4: paths compose only through identity; map complete binding sets and boundary outputs |
| C06 | P0.4: preserve opaque local guards; refuse unsupported substitution rather than erase conditions |
| C07 | P0.4: typed ControlInfluence and Selection with separate value-transfer meaning |
| C08 | P0.4: transfer key excludes merged condition; stable derivation references |
| C09 | P0.4: scope-boundary unknown; approximation obligation; rendering budget separate |
| C10 | P0.4: receiver Unknown and one classification policy |
| C11 | P0.2/P2: acyclic source identities, typed syntax kinds and structural occurrence discriminators |
| C12 | P0.4/P2.3: indexed region join with scalar oracle and unresolved ambiguity |
| C13 | P0.3/P3–P5: derivation targets follow typed references and generated view grants |
| C14 | P0.1: DESIGN §15.1 assigns provider registration to cpg-core and PostgreSQL effects to lctx-postgres |

## 9. Risks

The principal risks are incomplete raw-family migration, parallel semantic definitions, invalid
publication races and accidental claims of restored downstream capability. The field mapping, single
root model, real concurrency tests and generation availability frontier address these directly.

## 10. Deferred, each with a trigger

Stage reuse requires the existing measured-workload trigger. New reasoning engines and application
Salsa require a concrete consumer. PostgreSQL performance tuning follows measured phase-exit costs.
Cross-release symbol matching waits for its consumer. Product PR6 waits for phase 5 qualification.
