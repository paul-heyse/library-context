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

Lifecycle: staging → sealed → validated → published. Failed is a stored terminal state permitting
only abort; abort and retirement remove the schema and all registry state atomically, so retired is
terminal by absence. Each attempt owns a lifecycle connection holding its attempt lock; an
interrupted attempt is listed and can only be aborted. Selection is a
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

### 4.1.1 Detailed remaining execution order

The operator accepted this refinement of the remaining phase 0–2 work on 2026-09-29, starting
from `7aa7a30`. It replaces the earlier P0-A…P2-C rows. ADR-0085–0088 remain governing. All
packages are one functional scope: each commit carries a compile check and its focused command
(prefixed `python3 scripts/build_environment.py --`); formatting and the integrated gate wait for
the complete scope. No package is complete merely because it is listed here. Execution is native
in the working session, with `design-reviewer` reviews on the high-risk slices named below and
assembled reviews at the P0 and P0–P2 exits.

**Operator decisions (2026-09-29).**

1. **Downstream stays dormant but compiling.** `lctx-analytics`, `lctx-embed`, `python/*`,
   `cpg-schema`, the P3–P5 modules of `cpg-core` (analyze, behavior, flow_model, summaries,
   entry_links, catalog, catalog_domains, evidence, surface, synth, usage, retrieval, embed,
   bundle format code) and of `lctx-postgres` (serving, repository, selection, retrieval,
   hydration, evidence, evidence_search, packet, journeys, journey_cursor, profiles, diagnostics,
   report, import, projection) stay workspace members and keep compiling. Only their
   Delta/snapshot entry points are cut; no path runs them in P0–P2. Tests that no longer compile
   move to a non-target `tests/dormant/` directory with a header naming the removed runtime and
   owner; compilable tests needing removed runtime carry `#[ignore = "suspended: P<n> <layer>"]`.
   Conversion or deletion is P3–P5 scope.
2. **`cpg-schema` is not deleted in P2.** P2-C is selective: `cpg-extract` and `cpg-flow` stop
   depending on it; each surviving module records its dormant consumer and retiring phase (§5).
3. **Behavioral dependency context defers to P4.** Modeled-class and runtime-exception context
   requests are a recorded obligation (§6). P2 behavioral is catalog facts plus ty flow facts.
4. **P0–P2's own scope still pivots hard.** Delta, the `lctx_store` partition kernel,
   legacy-ID/parity adapters, old raw extraction and the runnable old orchestration/CLI go.

**Execution decisions (override only by operator decision).**

| # | Decision |
|---|---|
| T1 | Shared vocabulary uses declared stage contributions: `Stage.contributes` plus attempt-owned handoffs. Contributors hand rows to the relation's single writer, which deduplicates across batches; the schedule orders writer after contributors. Keeps one writer per relation without an all-in-memory assembler |
| T2 | Resource slices R1/R2 precede new contract work, so budget-taking `Batch::new/read` and `Invariant.create` signatures land before new tests |
| T3 | Contract amendments land in P0 (K1): `ProviderModule` (Artifact/Bundled/Unresolved) replaces `ProviderSymbol.module: String` and `TypeVariable.module`; `PlaceRoot::Local{scope, name}` (code 8), because reaching requires one place shared by a use and all its definitions; DESIGN §15.4 amended |
| T4 | Composition policy: Yield/Raise yield a typed `UnsupportedControlFlow` obligation; substitutable guards are `IsNone`/`IsValue` only; unwitnessed formal guards keep refusing; the pure `discharge`/`Meter` policy is ported so its expectations stay executable |
| T5 | `FactFamily` codes 0, 5, 6, 11 and 12 leave the enum with their numbers reserved: documented, never reused, never renumbered |
| T6 | Capture takes the complete analyzer-readable input closure (site-packages `.py`/`.pyi`/`py.typed`/`.pth`, dist-info METADATA/entry_points/RECORD, selected corpus files). Derived blocks and receipts live under a reserved `_lctx/` namespace in the frozen copy; a stale `_lctx_blocks/` in a source tree is refused |
| T7 | Only an Exact span match attaches. Other outcomes write a `SubjectBoundary` retaining candidates plus Partial coverage with appended reasons `AttachmentAmbiguous`/`AttachmentUnmatched`; dependent facts are counted, never guessed |
| T8 | Test-operand typing is decoupled from ty: Pyrefly types every test-position load; the leaf-to-type join is P3 |
| T9 | Partial facts publish with disclosure and a structured reason; a required provider failure aborts; compile never selects |
| T10 | `failed` is a stored terminal state permitting only abort; abort and retire remove schema and all registry state atomically. Each attempt owns a lifecycle connection holding an attempt advisory lock, so interrupted attempts list as `Interrupted` and can never publish. Lock order: installation → selection row → generation → attempt → relations |
| T11 | One service baseline migration (`lctx_cache.*`, `lctx_ops.attempts/events`) replaces migrations 0001–0013; legacy history is refused. The operator database moves offline: new database, retained-service copy, rename, archive kept |
| T12 | The 21 dormant `query_file!` entries stay frozen in `.sqlx`; `cache.rs` uses runtime queries; `sqlx-check`/`sqlx-prepare` leave `test-all` until P5; `lctx_serving` is not installed |
| T13 | Provider sessions use a bound pool of N (default 2) one-shot connections, each holding its own lease and checking digests and live columns before any scan. Transport loss or a failed cancellation drain is terminal (`Lost`); a confirmed drain returns the connection. Federation is removed; reference-column indexes wait for measurement |
| T14 | ADR-0089 (accepted at K1, 2026-09-29) records T1, T3, T6 and T7. ADR-0090 (P1.4) supersedes ADR-0002 for the delta-rs family removal. Composition, admission and lifecycle policies amend DESIGN §15.4/§15.6/§15.8/§15.11 in place |

**Operator-authorized steps.** The project is in its design phase; on 2026-09-29 the operator
authorized executing these planned steps without a further prompt: pushing the provider-fork
commit (P1.9), the operator-database transition (P1.13), deleting obsolete `build/` runtime copies
(C3x, after an inventory) and dropping the retired database. Targets are still inspected first,
and protected evidence, evaluation assets and unrelated services are preserved.

**Order.** P0: R1 → R2 → K1 → D0 → C1 → C2 → C3 → C4 → C5 → C6 → R3 → D1 → E1 → X0. P1: P1.1 →
P1.13. P2: A0 → A1–A2 → A3–A5 → A6–A11 → A12–A14 → A15 → A16 → B1 → B2 → B3 → Dc → C1x → C2x →
C3x → Q. Bounded reviews follow C5 (C4+C5), R3 (R1–R3), P1.7, P1.10, A8 (A6–A8) and B2.

**Review focus.** Failure modes owned by a named test: (1) composition over every alternative ×
signature variant, never one merged invocation (C5); (2) NotRequested, Unavailable, Partial and
complete-empty stay distinct from producer to `generation show` (D1, P1.7, B1); (3) shuffled order
and transport batch size leave content digests unchanged (B2); (4) a pinned reader never
reconnects to another generation or sees it vanish under selection change or retire (P1.7,
P1.10); (5) acquisition never writes into the captured tree and a change during capture aborts (A2).

#### Phase 0 remainder

| ID | Deliverable | Controls (pre-written; each positive has a negative twin) | Focused command |
|---|---|---|---|
| R1 | Reserved batches and bounded writer: derived `HeapSize::heap_bytes` and `Record::encoded_bytes_hint`; `Batch::new/read(.., &ResourceBudget)` reserve before encoding and hold the reservation; `domain/batching.rs` `BatchWriter<R>` with `TransferLimits` (4096 rows, 8 MiB, 64 MiB row) and a reserved id→digest map; `trait StageSink` and `StageOutput<S>` in `stages.rs`; `GenerationAttempt: StageSink`; `GenerationStore::copy`/`pin` take the budget; duplicated read constants removed | 10 000 rows → 4096/4096/1808; three 3 MiB rows → 2/1; 20 MiB row alone, 70 MiB refused; equal cross-flush duplicate once, conflicting one refused; drop returns reservation to 0; short budget refuses before encoding; empty declared output written | `lctx-model` `domain_resources`, `domain`, doc; `lctx-postgres` `generation_stages`, `generations` |
| R2 | Charged validator state: `Invariant.create` takes `&ResourceBudget`; `domain/charged.rs` (`ChargedMap`, `ChargedSet`); every check converted incl. generated support checks and `GuardIndex`/`ScopeIndex`/`TypeIndex`; 1M/3M cardinality caps removed; `validate(g, &budget)` reserves read buffers | Tiny budget → `Resource`, generation stays sealed, abort cleans, funded retry passes, reservation 0 | `domain_resources` + affected suites; `lctx-postgres` `generations` |
| K1 | T3 amendments plus ADR-0089 draft | Bundled vs artifact module symbols distinct; unresolved never equals resolved; two definitions of local `x` share a Place, sibling scopes differ; existing calls/types/flow/guard fixtures and PG readbacks pass | `domain_calls`, `domain_types`, `domain_flow`, `domain_guard_rebase` (model and PG) |
| D0 | T1 stage contract: `Stage.contributes` and `Stage.coverage` in the schedule digest; contributor→writer ordering; attempt-owned handoffs readable only by `ReadPermit<R>`; pure `MemoryGeneration: StageSink` with PG-equal content digests | Self-contribution and late contributor refused; undeclared handoff read refused; handoff released after last reader; lexical fixture digest equals PG | `domain_stages`, `domain_memory`; PG `generation_stages` |
| C1 | `domain/declarations.rs`: `SymbolDeclaration` and `ParameterDeclaration` L0 assertions (kind match, containment, 1:1, context, native-provider support); stored `ArgumentKind`, `CallSyntax`, `CallArgument`; binder over call syntax; missing link → `NoSourceDeclaration`, never name matching | `def f(a, b=1)` accepted; parameter outside def, duplicate mappings, class→def, foreign provider, context mismatch refused; `f(x, y=z)` has two arguments; missing/outside/mis-kinded argument refused | `domain_declarations`, `domain_calls`; PG `domain_declarations` |
| C2 | C04/C10: `PhaseGroup`, `site_facts` over direct non-Potential alternatives, `SiteTargets::admitted` (Summary uses site facts), `NativeCallee`, `normalize_site` via `classify_receiver` only | Agreement unique; cross-provider f/g not unique; `C()` New+Init unique per phase; `x()` Call+Init no Summary; `map(f, xs)` f excluded, binding refused; incomplete no Summary; empty list Unresolved; `C.method(obj, x)`/static None, unknown ambiguous, `obj.m(x)` Bound; old admission matrix re-expressed | `domain_calls` |
| C3 | C12 owner rule: `domain/occurrence_owner.rs` `OwnerTable::build` stack sweep, per-item oracle, budgeted; fixture `semantic_owner` | Default-arg call → module, body call → def, decorator → module, class body → class, lambda, comprehension, return annotation → module; shuffled equals oracle; tiny budget refuses | `domain_owner`; `cpg-extract` `typed_owner` |
| C4 | `Predicate::BoundGuard`; `conditions/stability.rs` `StabilityWitness` (parameter-only reaching) and `GuardSubstitution`; `substitute_call_guards`; formal/receiver operands only after `BoundGuard` | `timeout is None` witnessed → BoundGuard; reassigned formal → `ConditionTransferUnsupported`; Unbound/truthiness/attribute/default refused; catalog → NotRequested; local guard conditional; stored refusals; `InvokedGuard→formal` still refused | `domain_stability`, `domain_guard_rebase`; PG `domain_stability` |
| C5 | C05–C07: `domain/composition.rs` `compose_call`/`compose_site` over every admitted alternative × variant; one `map_root`; condition = caller ∧ substituted callee; `CallCompositionStep` derivation with the stored `call_composition_frames` check; `Modality::weakest`, `Approximation::join`; the `semantic_stability` pair extended by `tests/fixtures/composition.rs` | Twenty pre-written answers, incl. derived caller/callee paths, `update(t, k, v)` caller-owned mutation, raise/yield obligations, missing witness refused, `collect` projections, receiver path, Selection row, two targets → two candidates, oversized condition → limit obligation | `domain_composition`, `domain_transfer`, `domain_paths`; PG `domain_composition`; review |
| C6 | Re-express binder, path, place, transfer-key, discharge and C03 stage expectations; then delete old `calls`, `transfer`, `vocab`, `obligation`, `projection`, `relations`, `stage`, `derivation`, `condition/`, `id::recipes`, `lctx_id_v2`, their tests and `cpg-core/tests/model_policies.rs`, `cpg-schema/tests/obligation_mapping.rs`. Keep `decl/`, `id.rs`, `legacy::ID_TAG_V1`; `ddl.rs` goes in P1.2 | Re-expressed tests pass first; workspace compiles | named suites; `cargo check --workspace --all-targets` |
| R3 | Ruff total-work bound: source bytes admitted before traversal; every `SourceOrderVisitor::visit_*` counts and short-circuits once halted; `SyntaxWork` reported | 50 000 statements at nodes=10 → ≤ depth residual; 50 000-element list bounded; 300 levels → depth refusal; 17 MiB over 16 MiB → zero callbacks and `ResourceRefused` | `cpg-extract` `typed_limits`, `typed_conformance`; review R1–R3 |
| D1 | `domain/admission.rs`: `Frontier`, `FamilyRequirement`, `FrontierContract::facts` (facts relations reference only facts relations), `preflight`, `expected_coverage`, `reconcile`, `AdmissionCheck` → privately constructed `FactsAdmission`; T5 codes; `facts_relations()`/`analysis_relations()` | Four-artifact input: catalog Flow NotRequested, behavioral requested; missing/extra/duplicate rows, attempted catalog Flow, all-Unavailable required family, Complete-with-Partial refused; empty complete scope accepted; subset fails preflight; compile-fail doctest | `domain_admission`, `domain_stages`, doc |
| E1 | P0-E: permanent `SyntaxProvider`; `typed_stages.rs` capture and syntax stages over `StageSink`; `typed_conformance` on the stage-bound path; ignored `typed_subset_envelope` over pinned fastmcp input → dated evidence folder (Measured) or `blocked` | Subset refused as facts and unselectable; relocation determinism; changed text refused; tiny budget publishes nothing | `cpg-extract` `typed_conformance` |

**P0 exit (X0).** Assembled P0 design/target review of `7aa7a30..HEAD` (scenarios: P2
multi-provider producers, P3 equivalence and views, P4 SCC composition, mid-cutover contract
change, facts frontier refusing a higher layer, budget exhaustion, Unavailable/NotRequested
providers; G1–G8, CI-G1–G3). Exit requires dated receipts for every slice; C03, C05, C06, C07,
C10 and C12 closed at contract level; C04 closed except cross-provider equivalence and SQL views
(P3); C08/C09 re-confirmed; C11's one-predicate-one-atom check → A14; C13 consumer side → P4/P5;
C01/C02 → P1; input-validation F02 closed or narrowed to P1.9; review Accept or corrected Revise;
handoff.

#### Phase 1

Every commit leaves `cargo check --workspace --all-targets` passing with dormant code compiling.

| ID | Deliverable | Controls | Focused command |
|---|---|---|---|
| P1.1 | Quiesce the old CLI: trim `Cmd`, delete `parity`/`rebuild`/`serving` handlers, `db.rs` → `runs.rs`, drop `lctx`'s analytics/embed dependencies, `compile` exits 3 before work, `lctx_mcp` entry points exit 3, pilot recipes removed | Compile exits 3 without invoking uv; library/acquire/flow still work | `lctx` `acquire`; `just build-features` |
| P1.2 | Remove partition kernel and parity adapters: `lctx-model/src/ddl.rs`, `lctx-postgres/src/store.rs`, `cpg-core/src/{store_read,postgres_read,parity}.rs` and tests; `legacy.rs` trimmed to `ID_TAG_V1`/`IdHasher::new` | Remaining model suites pass | `domain`, `decl_sample` |
| P1.3 | Remove Delta runtime and orchestration: `delta`, `snapshot`, `attempt`, `rebuild`, `stage_cache`, `diff`, `derive`; `producer.rs` for dormant `analyze`; bundle Delta entry points; Delta `CoreError` variants; uncompilable dormant tests → `tests/dormant/`; operations discovery half; justfile and Python skips (`test_flow_soundness.py` stays live) | Dormant crates compile; skips report not_run | `cpg-core` `model_runtime`, `dependency_audit`; pytest flow oracle |
| P1.4 | Drop delta-rs family and federation (pin-check; ADR-0090): workspace manifests, `build.rs` engines, `family_smoke`, logging filters, `deny.toml`, `check_family.py`, pins, skill selection, Delta ast-grep rules, AGENTS.md, DESIGN §B7/§B9/§7 | Family check passes; no `deltalake` in the tree | `just build-features`; `check_family.py`; its pytest |
| P1.5 | Service baseline (T11/T12): single baseline migration; `Error::LegacyHistory`; `OwnerPool::verify` (non-superuser owner without CREATEROLE/BYPASSRLS or runtime-role membership); `roles.rs`; runtime cache queries; `testing` feature `DisposableDatabase`; all generation PG tests install as the owner | Fresh baseline only services; repeated migrate no-op; legacy history refused unchanged; cache round trip; attempts/events; grant matrix; superuser install refused | `lctx-postgres` `services`, `generations`, `generation_stages`, domain suites; `typed_conformance` |
| P1.6 | Generated install/check/reset: pure `Lowering` phases and physical digest; templated control DDL; `store check` compares live `pg_catalog` descriptors against in-transaction shadow schemas for every state, roles and ACLs, unexpected objects and orphans; `store reset --confirm` drops only inventoried objects | Owner required; digest mismatch; clean in every state; each injected drift reported; reset refuses live lease/attempt, spares services and unrelated schemas, reinstalls | `lctx-postgres` `installation` |
| P1.7 | Attempt-owned lifecycle (T10, uses D1): `begin(writer, &mut Execution, &FrontierContract, budget)` typestates; `planned_outputs`, `stage_outcomes`, `admissions`, `failures`, `failed` state; validate runs `AdmissionCheck`; publish requires planned = written outputs and a matching admission, atomically with grants; select requires facts; reader grants on control tables | Late write vs seal (50×); failed validation abort-only; failed publication atomic under three faults; interrupted attempt never published; live abort Busy; lease vs retire (50×); select vs retire; atomic retire under fault; no deadlock; subset `begin` refused without registry row | `lctx-model` `domain_admission`; `lctx-postgres` `lifecycle`, `generation_stages`, `generations`; review (C01) |
| P1.8 | `GenerationCatalog` list/show: state, frontier, profile, digests, coverage summary, reader count, writer liveness | Fields per state; two leases counted; reader cannot mutate | `lctx-postgres` `generation_catalog` |
| P1.9 | Owned provider fork (T13): byte-bounded row chunks, `MemoryReservation` hooks, one-shot bound pools with drain-and-return and `lost`, nullable-list narrowing. Operator confirms the push; then rev, patch, pins row, `just build-features` | Fork `bounded_chunks` | fork crate test |
| P1.10 | Driver-neutral lease protocol in `lctx-postgres`; `cpg-core/src/generation_read.rs` `GenerationSession` and `InspectionSession`; `GenerationTable` with exact schema and closed filter pushdown (C02) | Readback fidelity incl. 65 MiB and empty; pushdown; digest/column mismatch before scan; pin survives selection; lease blocks retire; byte bounds and reservation refusal; cancellation drains to the same backends; transport loss terminal; stage registration | `cpg-core` `generation_read`, `model_runtime`; review |
| P1.11 | CLI: `--database` replaces `--database-config` (discovery kept; sibling files select roles); `model describe`, `store install/check/reset`, `generation list/show/select/clear-selection/retire/abort`, read-only `query --generation`, `compile` stub, `runs`; exit codes 0/1/2/3 | Binary against a disposable database: describe text/json, store commands, generation commands, conformance select refused, DDL query refused, compile 3, discovery | `lctx` `store_cli` |
| P1.12 | Ops: `postgres_transition.py` (plan/prepare/switch/drop-retired) with a disposable control; bootstrap provider/owner pool sizes and validation timeout; backup table list; justfile PostgreSQL recipes; `docs/postgresql.md`, AGENTS.md, DESIGN §6/§B7/§15.11, §8 evidence | Transition control | `LCTX_POSTGRES_TEST=1` pytest |
| P1.13 | Operator-confirmed transition: quiesce readers, rehearse from the real archive, then plan/prepare/switch; fingerprints equal, `store check` clean, `runs list`; record receipts, delete the tool, handoff | Fingerprints equal | operator-run |

**P1 exit.** No `deltalake`, `buoyant_kernel` or `datafusion-federation` in `Cargo.lock`; no
Delta, snapshot, partition, parity, `lctx_id_v2` or old CLI code; only the service baseline and
generated store are installed by the service owner; `store check` clean in every state and
drift-sensitive; lifecycle and provider-session suites pass; CLI exposes only target and retained
commands; the operator database is transitioned or explicitly deferred.

#### Phase 2

Providers run as typed stages; each producer group deletes its own old row emission from
`run_release` in the same commit, so no relation is ever written twice. An old test goes only
with its independent replacement. Each stage processes the library input and then the corpus;
each Pyrefly session is dropped before the next opens. Provider indices never leave a provider.

| Stage | Profiles / effect | Sole-writer outputs | Contributes |
|---|---|---|---|
| `acquire` | both / Acquisition | package, release, input, origin, acquisition, corpus, distribution, verification, artifact, chunk, ownership, unowned, use, derived artifact, environment fingerprint | — |
| `pyrefly` | both / Extraction | Module, Occurrence; syntax, lexical, symbol, signature, declaration, call and type records | attribution vocabulary, qualifications, evidence, literals, conditions, subject boundaries, attachment outcomes |
| `ty_flow` | behavioral / Extraction | flow records, evaluation atoms, predicates, place parts, flow call paths | vocabulary, conditions, literal sets |
| `documents` | both / Extraction | document records | vocabulary |
| `deployment` | both / Extraction | report values/collections, reported environments, task reports and observations, deployment observations | vocabulary |
| `assemble` | both / Pure | contributed vocabulary relations, `ProviderCoverage`, `CoverageScope` | — |

Catalog profile: `ty_flow` is not scheduled and `assemble` writes Flow NotRequested for every
expected scope. `ty_flow` declares lexical and import-alias inputs; `documents` declares public
name and declaration inputs; the Ruff-vs-Pysa call and `__all__` comparisons stay inside `pyrefly`.

| ID | Deliverable | Controls | Focused command |
|---|---|---|---|
| A0 | Provider framework: `ProviderStage`, `StageContext` (emit, contribute, handoffs, attacher, captured inputs, budget); big-stack provider thread, bounded channel to an async pump holding `StageAccess`; `Provider` build digest over lockfile, provider sources and Pyrefly patch (F11); `Attacher`; `cpg-core/src/facts.rs` `compile_facts<S: StageSink>`; fixture-corpus skeleton | Undeclared emit/contribute/read refused; panic aborts; reservations 0; order preserved | `cpg-extract` `bundle`; `cpg-core` `facts_driver` |
| A1 | Acquisition model: source roles, `UnownedArtifact`, `DerivedArtifact`, `EnvironmentFingerprint`; exactly-one ownership class | Double class, orphan derivation and `_lctx/` originals refused | `domain_input` (model and PG) |
| A2 | T6 acquisition: pure inventory, full closure capture, `_lctx/` namespace, `acquire` stage; delete tree writes, `release_rows`, old `capture()` and corpus id hashing | Tree byte-identical before/after; namespace collision and stale blocks refused; location independence; environment dependence; RECORD tamper; change during capture aborts | `cpg-extract` `acquisition`, `capture` |
| A3 | Syntax model: placement/field, `SyntaxDetail`, declaration (+decorators), import alias, `__all__`, parameter syntax, class field syntax, `SubjectBoundary`, attachment outcome/candidates, appended obligation codes | Text derivable from bytes; foreign optional subject and scopeless boundary refused | `domain_syntax`, `domain_coverage`; PG `domain_syntax` |
| A4 | `pyrefly` phase 1: complete typed syntax (replaces string paths), declarations, imports, parameters, class fields, call syntax, parse/undecodable coverage, `__all__` boundary; delete `walk.rs`/`syntax.rs` rows | syntax_shapes, unicode_bom offsets, dunder_all; distinct with-items | `typed_syntax_shapes` |
| A5 | Lexical records from the recognizer; delete `lexical.rs` rows | static_branches, lexical_shapes | `typed_lexical` |
| A6 | Symbol model: symbol, dependency module, function/class traits, ancestry (base/MRO), annotations, public names, parameter docs, symbol sequences | Cyclic MRO; untraced ≠ traced; display-only annotation not structural | `domain_symbols` (model and PG) |
| A7 | Calls amendment: provider call site, receiver class and traits, `Overrides`, native unresolved reason | Overrides never direct in `site_facts` | `domain_calls` (model and PG) |
| A8 | Type completion: remaining term arms, roles, parameter lists, test operand, function bodies, record fields | Truncated nesting display-only; record flags round-trip | `domain_types` (model and PG); review A6–A8 |
| A9 | Symbols producer incl. declaration links and dependency context (referenced ∪ exported; `Catalog`/runtime-exception reads deleted) | Pysa CLI `harness`; `keys`; public_shapes | `typed_symbols`, `harness` |
| A10 | Calls producer via `normalize_site`; call boundaries; `variants` as known answers | pysa_variants, `map`, `C()`, missing range boundary | `typed_calls` |
| A11 | Types producer incl. T8; delete `pysa_map`/`types`/`context`/`public` rows | type_shapes, type_guard operands | `typed_types` |
| A12 | Flow model and kernel: test, leaf, call path/step, attribute load, value path records; `CondGraph<L>`; bounded `Diagram::from_graph` | Independent truth tables; malformed graph; limits → boundary | `domain_flow`, `domain_conditions`; PG `domain_flow` |
| A13 | `cpg-flow` off `cpg-schema`: provider-local leaves and condition graphs; no BDD, id hasher or predicate keys; structural known answers replace insta snapshots | Distinct occurrences and with-items; opaque synthetic predicates | `cpg-flow` `flow_shapes`, `call_paths`; `cargo tree -p cpg-flow -i cpg-schema` empty |
| A14 | `ty_flow` stage: exact attachment, Places, atoms, canonical BDDs; `lctx flow` retargeted; `cpg-extract/src/flow.rs` deleted | flow_shapes, call paths, type_guard, runtime resolution; index permutation identity (F05); C11 one predicate → one atom per generation; no DNF/literal writes; Python flow oracle | `typed_flow`; `test_flow_soundness.py` |
| A15 | `documents` stage with materialized block digest check; delete `docs.rs` rows | docs_shapes, semantic_documents, corrupt block | `typed_documents` |
| A16 | `deployment` stage: metadata, entry points, configuration, receipts, fingerprint; malformed receipts retained as Failed interpretation with Partial coverage; delete `metadata`/`observations` rows | semantic_deployment, malformed receipt, identity mismatch | `typed_deployment` |
| B1 | Assembler: vocabulary writer, exact coverage matrix into `AdmissionCheck` through P1.7 `begin`, required failure aborts, Partial reasons | Missing/extra row refused; profiles differ only in Flow | `cpg-core` `facts_admission`; PG `facts_generation` |
| B2 | Attachment scalar-oracle matrix (names, attributes, subscripts, same-span kinds, decorators, f-strings, BOM, `.py`/`.pyi`, forced Ambiguous/Unmatched/BudgetExceeded) and determinism (shuffle, 4096/1/97-row batches, relocation, repeats, ambient refusal) | Equal content digests | `attachment_oracle`, `determinism`; review |
| B3 | Fixture-corpus runner in `cpg-core/tests/fixture_corpus.rs`: listing equals registered cases; both profiles in memory; three cases also PG with equal digests; re-homed raw known answers; P3–P5 obligation pointers; `just fixture-corpus` | Unregistered fixture fails | `fixture_corpus` |
| Dc | `lctx compile <library> --through facts --profile catalog\|behavioral --database <cfg>`: other frontiers refused before side effects; digest check; acquire → capture → schedule → publish; availability report; never selects | `--through analysis` side-effect free; injected required failure leaves no registry row | `lctx` `compile_facts` |
| C1x | Remove the extraction husk (`extract`, `run_release`, `ExtractOutput`, `FactSink`, `fact_row!`, `write_ipc`, `lctx-extract` binary, legacy ids/recipes, cpg-schema dependencies) | Gates below | workspace check |
| C2x | Selective pruning; dormant-test answers re-homed or recorded as obligations with the Git revision; regenerate the surviving `cpg-schema` module → consumer → phase map into §5 | Gates below | workspace check |
| C3x | §4.1.2 dispositions with evidence, §4.2, §5, §6, DESIGN §15 labels, AGENTS.md, ADR-0089 consequences brought current; `build/` runtime-copy deletion from an inventory | docs and ADR lint | `just docs-check` |

**Gates.** `rg` over `crates/cpg-extract` and `crates/cpg-flow` finds no `cpg_schema`,
`FactSink`, `fact_row!`, `IdHasher`, `recipe::`, `ExtractOutput`, `write_ipc`,
`BoundedCondition`, `condition_kernel`, `ConditionLiteral`, `_lctx_blocks` or `lctx_id`;
`cargo tree -i cpg-schema` excludes both crates; `cargo tree -i deltalake` is empty;
`scripts/check_family.py` passes; `cargo check --workspace --all-targets` passes.

Operator interface (no legacy aliases): `model describe --format text|json`; `store install|check|reset`;
`generation list|show|select|clear-selection|retire|abort`; `query --generation <id> <SQL>`; and
`compile <library> --through facts --profile catalog|behavioral`. Database commands take
`--database <protected-config-path>` and preserve environment/default config discovery. Query is
read-only. Inspection exposes actual profile/frontier, coverage and contract/content digests.
Embedding/analytics/bundle/old-store arguments leave facts compile; compile is explicitly
unavailable (exit 3) between P1.1 and Dc. Installation/reset touches only inventoried
project-owned semantic objects and retained service contracts, not unrelated data.

Qualification (Q) after all functional packages: `just fmt` once; updated complete `just test-all`
(real PG, compile-fail doctests, fixture corpus, flow oracle); facts pilots for both profiles on
fastmcp, with a repeated behavioral run giving an identical content digest, per-stage time and
peak RSS, frontier `facts`, no automatic selection and profile-correct Flow coverage;
`just docs-check`; assembled P0–P2 review; handoff. Suspended gates are reported not_run:
semantic-soundness script, MCP smoke, structured evaluation, sqlx check and dormant
catalog/analysis/serving suites. Content equality excludes telemetry, timestamps and generation
IDs. No incremental cache or new reasoning engine is added. A capability's deletion does not close
its retained obligation.

### 4.1.2 Raw-field migration inventory

**Proposed mappings; source inventory inspected 2026-09-29.** This is the deletion checklist for P2,
not a second executable schema. All 57 `for_each_table!` entries are listed, including inline
macro fields. The destination domain owns all listed payload fields unless an explicit transformation
below applies. No table is marked migrated merely because its owner appears here.

Common transformations: `snapshot_id` becomes generation containment, never a semantic field;
`fact_id` becomes qualified proposition identity plus separate typed supports. Legacy `*_id` node/fact
references are reconstructed as nominal references to the corresponding domain object; no old ID
reader or lookup bridge survives. Optional fields retain unknown/absent meaning through domain sums
or validated options. Display names remain presentation and cannot replace source/provider identity.

| Raw relation | Remaining fields (including legacy identity/reference fields) | Destination owner |
|---|---|---|
| `facts` | `run_id`, `table_name`, `origin`, `extraction_mode`, `modality`, `fidelity`, `model_id` | attribution: qualified assertions and typed support records |
| `runs` | `run_id`, `release_id`, `context_id`, `producer_id`, `families`, `config_digest` | attribution: ProviderRun/RunFamily, AnalysisContext, Provider |
| `contexts` | `context_id`, `python_version`, `python_platform`, `search_path`, `site_package_path`, `config_digest`, `environment_digest`, `lock_digest` | attribution: ProviderRun/RunFamily, AnalysisContext, Provider |
| `producers` | `producer_id`, `tool`, `revision`, `build_digest` | attribution: ProviderRun/RunFamily, AnalysisContext, Provider |
| `releases` | `release_id`, `library`, `requirement`, `lock_digest`, `distributions`, `installer`, `label` | input: InputOrigin/Acquisition, Package/Release, DistributionVerification and InputDistribution |
| `distributions` | `context_id`, `name`, `version`, `artifact_sha256`, `record_digest` | input: InputOrigin/Acquisition, Package/Release, DistributionVerification and InputDistribution |
| `captured_artifacts` | `artifact_id`, `release_id`, `context_id`, `path`, `source_kind`, `source_digest`, `byte_len`, `body`, `alignment`, `provenance`, `observations` | input/source plus deployment: original SourceArtifact bytes and typed interpretation/task observations |
| `source_files` | `module_node_id`, `release_id`, `module_name`, `path`, `is_package`, `is_stub`, `content_digest`, `byte_len`, `utf8`, `distribution`, `role`, `text` | input/source: SourceArtifact, Module, ArtifactOwnership and ArtifactUse |
| `context_modules` | `module_node_id`, `module_name`, `origin`, `path`, `distribution`, `version` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `context_definitions` | `symbol_node_id`, `module_node_id`, `module_name`, `kind`, `key`, `name`, `qualified_name`, `is_top_level`, `signature_count` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `context_parameters` | `symbol_node_id`, `module_node_id`, `signature_index`, `form`, `ordinal`, `kind`, `name`, `required` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `context_class_mro` | `class_node_id`, `module_node_id`, `ordinal`, `ancestor_module`, `ancestor_key`, `ancestor_name`, `cyclic`, `linearization_complete` | syntax/types: provider-qualified dependency symbols, signatures and ancestry observations |
| `declarations` | `node_id`, `module_node_id`, `parent_node_id`, `qualified_name`, `name`, `kind`, `start_byte`, `end_byte`, `name_start_byte`, `name_end_byte`, `docstring`, `docstring_start_byte`, `docstring_end_byte`, `is_overload`, `decorators` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `export_syntax` | `node_id`, `module_node_id`, `kind`, `imported_module`, `imported_name`, `alias`, `level`, `resolved_module`, `start_byte`, `end_byte`, `dunder_all_literal` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `public_names` | `access_path`, `access_module`, `name`, `origin_path`, `origin_module_node_id`, `via_dunder_all`, `origin_module`, `origin_name`, `access_module_node_id`, `origin_symbol_kind` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `parameter_syntax` | `node_id`, `function_node_id`, `ordinal`, `name`, `kind`, `default_text`, `default_start_byte`, `default_end_byte`, `annotation_text`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `parameter_docs` | `function_node_id`, `module_node_id`, `name`, `text`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `pysa_functions` | `module_node_id`, `module_name`, `function_key`, `name`, `name_start_byte`, `name_end_byte`, `is_overload`, `is_staticmethod`, `is_classmethod`, `is_property_getter`, `is_property_setter`, `is_stub`, `is_def_statement`, `defining_class`, `overridden_base`, `defining_class_module`, `defining_class_key`, `overridden_module`, `overridden_key`, `signature_count` | types: provider-qualified callable signatures, class traits and ancestry |
| `parameter_semantics` | `module_node_id`, `module_name`, `function_key`, `signature_index`, `form`, `ordinal`, `kind`, `name`, `required`, `annotation`, `annotation_classes`, `annotation_classes_exhaustive`, `annotation_scalar` | types: provider-qualified callable signatures, class traits and ancestry |
| `class_ancestry` | `module_node_id`, `module_name`, `class_key`, `class_name`, `name_start_byte`, `name_end_byte`, `relation`, `ordinal`, `ancestor`, `mro_cyclic`, `ancestor_module`, `ancestor_key` | types: provider-qualified callable signatures, class traits and ancestry |
| `pysa_classes` | `module_node_id`, `module_name`, `class_key`, `class_name`, `name_start_byte`, `name_end_byte`, `is_synthesized`, `is_dataclass`, `is_named_tuple`, `is_typed_dict` | types: provider-qualified callable signatures, class traits and ancestry |
| `call_syntax` | `node_id`, `module_node_id`, `owner_node_id`, `start_byte`, `end_byte`, `callee_start_byte`, `callee_end_byte`, `in_annotation`, `positional_count`, `keyword_count` | calls: syntax sites/arguments and separately qualified target alternatives |
| `arguments` | `node_id`, `call_node_id`, `ordinal`, `kind`, `keyword`, `start_byte`, `end_byte`, `value_start_byte`, `value_end_byte` | calls: syntax sites/arguments and separately qualified target alternatives |
| `syntax_nodes` | `node_id`, `module_node_id`, `owner_node_id`, `parent_node_id`, `kind`, `field`, `ordinal`, `start_byte`, `end_byte`, `detail` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `scopes` | `node_id`, `module_node_id`, `kind`, `owner_node_id`, `parent_scope_id`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `bindings` | `node_id`, `scope_id`, `module_node_id`, `name`, `kind`, `ordinal`, `site_node_id`, `start_byte`, `end_byte`, `value_start_byte`, `value_end_byte`, `static_branch`, `static_polarity` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `references` | `node_id`, `name_node_id`, `scope_id`, `module_node_id`, `name`, `parent_node_id`, `field`, `start_byte`, `end_byte` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `reference_resolutions` | `reference_id`, `binding_id`, `captured`, `builtin_name`, `reason` | syntax: qualified declaration/export/parameter/tree/scope/binding/read/resolution observations |
| `type_terms` | `node_id`, `kind`, `display`, `detail`, `literal_json`, `class_module`, `class_key`, `variable`, `anchor_module`, `anchor_start`, `anchor_end` | types: structural terms/arguments, qualified type/body/record-field observations |
| `type_term_args` | `parent_node_id`, `role`, `ordinal`, `child_node_id`, `name`, `parameter_kind`, `required` | types: structural terms/arguments, qualified type/body/record-field observations |
| `type_observations` | `module_node_id`, `subject_node_id`, `role`, `declared`, `term_node_id` | types: structural terms/arguments, qualified type/body/record-field observations |
| `function_implementations` | `function_node_id`, `module_node_id`, `body_kind`, `is_abstract_method`, `is_in_protocol_class`, `is_in_type_checking_block`, `is_overload` | types: structural terms/arguments, qualified type/body/record-field observations |
| `record_fields` | `node_id`, `class_node_id`, `module_node_id`, `record_kind`, `name`, `ordinal`, `term_node_id`, `declared`, `start_byte`, `end_byte`, `has_default`, `init`, `alias`, `kw_only`, `required`, `read_only` | types: structural terms/arguments, qualified type/body/record-field observations |
| `record_field_syntax` | `field_node_id`, `class_node_id`, `module_node_id`, `name`, `start_byte`, `end_byte`, `annotation_text`, `value_text`, `value_start_byte`, `value_end_byte` | types: structural terms/arguments, qualified type/body/record-field observations |
| `documents` | `node_id`, `release_id`, `path`, `content_digest`, `byte_len`, `title`, `parsed` | documents: source-backed document/section/code/link/mention/component records |
| `passages` | `node_id`, `document_node_id`, `ordinal`, `level`, `heading`, `heading_path`, `start_byte`, `end_byte`, `text` | documents: source-backed document/section/code/link/mention/component records |
| `code_blocks` | `node_id`, `document_node_id`, `passage_node_id`, `ordinal`, `language`, `meta`, `start_byte`, `end_byte`, `code`, `content_digest`, `module_path` | documents: source-backed document/section/code/link/mention/component records |
| `doc_links` | `passage_node_id`, `ordinal`, `url`, `title`, `text`, `start_byte`, `end_byte` | documents: source-backed document/section/code/link/mention/component records |
| `mentions` | `passage_node_id`, `class`, `source`, `form`, `access_path`, `qualified_name`, `start_byte`, `end_byte` | documents: source-backed document/section/code/link/mention/component records |
| `doc_components` | `document_node_id`, `passage_node_id`, `ordinal`, `parent_ordinal`, `depth`, `name`, `form`, `start_byte`, `end_byte`, `inner_start`, `inner_end`, `lead_start`, `lead_end` | documents: source-backed document/section/code/link/mention/component records |
| `doc_component_attributes` | `document_node_id`, `component_ordinal`, `ordinal`, `name`, `value`, `value_kind` | documents: source-backed document/section/code/link/mention/component records |
| `pysa_calls` | `payload_id`, `module_node_id`, `module_name`, `caller_key`, `site_kind`, `callee_kind`, `site_detail`, `start_byte`, `end_byte`, `phase`, `higher_order_index`, `target_kind`, `target_module`, `target_key`, `target_name`, `receiver_class`, `receiver_module`, `receiver_key`, `implicit_receiver`, `implicit_dunder_call`, `is_class_method`, `is_static_method`, `unresolved_reason`, `is_attribute` | calls: syntax sites/arguments and separately qualified target alternatives |
| `flow_uses` | `use_id`, `module_node_id`, `place`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `annotation` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_definitions` | `definition_id`, `module_node_id`, `place`, `kind`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `value_start_byte`, `value_end_byte` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_reaching` | `use_id`, `definition_id`, `condition_id`, `approximated`, `loop_carried` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_values` | `module_node_id`, `sink`, `sink_start_byte`, `sink_end_byte`, `use_id`, `identity`, `through_call`, `condition_id`, `approximated` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_value_calls` | `module_node_id`, `flow_value_fact_id`, `use_id`, `step`, `call_start_byte`, `call_end_byte`, `operand_start_byte`, `operand_end_byte`, `role` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_regions` | `module_node_id`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `condition_id`, `approximated` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_tests` | `module_node_id`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `start_byte`, `end_byte`, `condition_id` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_test_leaves` | `module_node_id`, `scope_kind`, `scope_start_byte`, `scope_end_byte`, `predicate_key`, `test_start_byte`, `test_end_byte`, `condition_id`, `atom_id`, `atom`, `leaf_start_byte`, `leaf_end_byte` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_test_types` | `module_node_id`, `leaf_fact_id`, `atom_id`, `use_id`, `use_fact_id`, `operand_start_byte`, `operand_end_byte`, `role`, `place`, `term_node_id`, `term_fact_id`, `origin` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `flow_attribute_loads` | `module_node_id`, `start_byte`, `end_byte`, `name` | flow: typed places, occurrence-backed events, reaching/value/control/predicate observations |
| `conditions` | `condition_id`, `root_id`, `encoding`, `stated`, `display_truncated`, `boundary_reason` | conditions: canonical BDD/occurrence atoms; display-only projections retired |
| `condition_nodes` | `node_id`, `atom`, `low_id`, `high_id` | conditions: canonical BDD/occurrence atoms; display-only projections retired |
| `condition_literals` | `condition_id`, `conjunction`, `ordinal`, `atom`, `positive`, `place`, `argument` | conditions: canonical BDD/occurrence atoms; display-only projections retired |
| `coverage` | `run_id`, `scope_kind`, `scope_node_id`, `fact_family`, `status`, `reason`, `detail` | attribution: ProviderCoverage plus structured scope/subject boundaries |
| `boundaries` | `module_node_id`, `subject_node_id`, `fact_family`, `reason`, `start_byte`, `end_byte`, `detail` | attribution: ProviderCoverage plus structured scope/subject boundaries |

Specific transformations and deletion conditions:

- `facts.table_name` is replaced by concrete typed support targets. `origin`, `extraction_mode`,
  `fidelity` and provider surface remain evidence attributes; modality/condition/approximation qualify
  propositions and participate in their identity. Run identity does not qualify proposition identity.
- Acquired distribution strings split into package/version references. Source text, UTF-8, package
  and stub flags derive from retained bytes/path; original bytes must survive failed interpretation.
  Context distribution association must remain explicit, including unowned top-level input entries.
- Rendered flow `place`, predicate keys and string atoms are replaced by structural places and
  occurrence-keyed atoms. Keep original diagnostic text only as presentation. `conditions.encoding`
  and `condition_literals` have no independent semantic store: canonical BDD roots/nodes own truth;
  terminal truth, unknown/refusal and approximation must remain distinguishable.
- Calls retain direct/higher-order distinction, phase, target/receiver alternatives, unknown receiver
  and unresolved remainder. Target absence never certifies completeness. Span attachment retains
  ambiguity and provider identity; normalized entity ownership waits until L1.
- Documents/code retain byte-coordinate spaces and links to original artifacts. Optional parser
  failure cannot erase evidence. Annotation/default/literal payloads require lossless typed values;
  a display string alone does not establish structural semantics.
- `snapshots` is outside the 57 raw families: generation registry/content and relation receipts replace
  its publication metadata. Delta table versions and snapshot-key columns are deleted at cutover.

Nested captured evidence is part of the same inventory, not an opaque JSON escape hatch:

| Current structure | Fields retained in typed deployment/evidence records |
|---|---|
| `Checks` | `parse`, `binding`, `environment`, `execution` |
| `ContextRequirement` | `kind`, `expression`, `evidence` |
| `ScenarioDetail` | `spans`, `context`, `intent`, `checks`, `requirements`, `extraction`, `analysis_module`, `option_bindings`, `omitted_options`, `omitted_requirements` |
| `OptionBinding` | `site_id`, `ordinal`, `keyword`, `kind`, `expression`, `span_id`, `coordinate_space` |
| `DeploymentEnvironment` | `release_id`, `lock_digest`, `environment_digest`, `runtime_digest`, `interpreter_digest`, `python_version`, `platform`, `requirement`, `metadata` |
| `TaskReceipt` | `format`, `policy`, `task`, `runner_sha256`, `source_path`, `source_sha256`, `environment`, `command`, `tool`, `arguments`, `elapsed_ms`, `timeout_seconds`, `execution`, `tools`, `result`, `diagnostic` |
| `TaskObservation` | `target_artifact`, `receipt` |
| `AssociationSupport` | `edge_id`, `fact_id`, `support_fact_id`, `target_id`, `modality`, `phase`, `unresolved_reason`, `context_span_id`, `analysis_module`, `start_byte`, `end_byte`, `coordinate_space` |
| `DeploymentDetail` | `distribution`, `version`, `field`, `original`, `name`, `extras`, `marker`, `constraint`, `interpretation`, `diagnostic`, `environment_digest`, `lock_digest`, `task`, `referenced_path` |

`CheckStatus`, `ContextStatus`, `Intent`, `Alignment`, `EvidenceKind` and `EvidenceRef` become
typed codes/sums. Metadata maps, task arguments/commands/tools, requirements, option bindings and
evidence-reference collections become ordered/keyed relationship records. Preserve receipt format,
policy, runner/source hashes, execution result/diagnostic and omission counters. Scenario/association
conclusions are P4 work, but their raw evidence inputs cannot be dropped during P2. Captured
`observations` must be decoded and attributed by the new producer, without retaining an old-format
runtime reader. Malformed interpretation retains the original artifact with explicit failure coverage.

### 4.2 Execution status

**In progress: P0-A–P0-D foundations, native P0-E subset and bounded generation lifecycle corrections.**
No phase exit is qualified. The old pipeline remains active and the new generation path is not
connected to the production compiler or CLI. No compatibility adapter connects them.

**Implemented / Tested (2026-09-29, bounded):**

- Ordinary record/codebook/sum/Assertion derives, nominal IDs, generated keys and explicit Arrow
  codecs. Model admission enforces references, subtype membership and assertion/support companions;
  assertion qualification must participate in identity.
- Source metadata plus canonical 1 MiB artifact chunks preserve original binary evidence, including
  files above the row limit. Capture and sealed validation prove full length/digest and canonical
  ordering; failed capture cannot resume successfully. The extractor now freezes a complete selected
  input inventory before provider startup, reserves copy/chunk/metadata buffers, detects changed input
  and verifies the frozen bytes afterward. Its caller must quiesce acquisition; this is not an atomic
  filesystem snapshot or total-RSS qualification. Input manifests, acquisition/distribution/
  corpus relationships, ownership/use and source spans have shared stored invariants.
- Typed values, literal sets, places, occurrence-keyed atoms and canonical BDD nodes. Independent
  truth tables and malformed-node controls cover bounded operations and persisted hydration.
  Capture-safe substitution preserves unmapped opaque guards; display truncation is separate.
- Qualified syntax propositions and generated concrete supports retain context, condition, modality,
  approximation, invocation, surface, evidence, origin, mode and fidelity. Validators check condition
  contexts, evaluation and operand-Place source ownership, assertion/evidence scope and required support.
  Ordinary source assertions also validate guard operands; an evaluation in local bytes does not
  authorize an operand from another input/artifact.
  Coverage-matrix controls exist; full compiler schedule/coverage admission remains open.
- A pinned Pyrefly retained-AST emitter produces structural occurrences and qualified identifier
  observations directly, without legacy rows/IDs. A disposable conformance harness exercises
  permanent PG lowerings and readback, relocation, changed-text refusal and incomplete coverage.
  The harness now uses the reusable captured tree. Emission/depth limits are not total traversal-work
  limits; acquisition/assembler integration remains open.
- Provider-qualified symbols, complete signature membership, typed call destinations/channels/receivers
  and complete alternative membership have generated lowerings and stored invariants. Native support
  must belong to the symbol provider. The whole-variant binder preserves all formals, defaults, empty
  aggregates, binding kind and positional/keyword projections; forged shape lookups refuse. Named
  policies exclude potential/higher-order invocation from dataflow and use the full declared set for
  uniqueness. P3 equivalence, constructor normalization and SQL projection remain open.
- Stable TransferKey vocabulary, qualified alternatives and condition-OR aggregation retain original
  evidence IDs. ControlInfluence and Selection remain distinct from value flow. Shared support checks
  both Place endpoints and populated call sites against scope/acquisition. Structural path composition
  extends only through identity; wildcard/unknown/numeric-alias cases preserve uncertainty. Whole-call
  root/binding composition and opaque caller rebasing remain open.
- Derivation fields declare their rule, nominal conclusion and premise roles in their Rust record.
  Model admission generates one cross-source cycle invariant; proof source identity survives explicit
  conclusions. Existing pinned petgraph supplies iterative cycle detection. PostgreSQL generates
  explanation views and publication grants from the same declarations. Self/mutual proof-step cycles
  refuse before publication. P4 producers and serving explanations remain open.
- Attachment indexes and retained ambiguous answers own shared-budget reservations, with preallocation
  admission and failure cleanup. A flat sorted index retains scalar-oracle equivalence. This qualifies
  attachment buffers only, not parser/validator/COPY totals or process RSS.
- Typed stage access refuses undeclared reads/writes and incomplete, failed or cancelled execution.
  Schedules now bind profile, effect, producer code and configuration. The PostgreSQL conformance
  sink binds one execution exclusively, rejects foreign write permits, and requires successful
  generation-specific writes (including explicit empty outputs) before receipt-bound sealing.
  Stage/output receipts are stored atomically with sealing; a successful no-op cannot replace a write.
  COPY wire buffers reserve from the supplied attempt budget before pgpq encoding, using its own
  size hints. This does not yet account all producer/codec/driver allocations.
  `cpg-core::model_runtime` constructs fresh catalogs over a shared DataFusion runtime, rejects
  foreign stage/attempt read permits and SQL mutations. A neutral reservation interface shares the
  compute pool; accounting every producer/index/validator/codec buffer remains open.
- The pure verdict policy treats scope boundaries and approximations as Unknown, preserves explicit
  NotRequested, requires complete exact coverage for refutation, and ignores display truncation.
- PostgreSQL generation lowering, streamed COPY/read validation, writer-draining sealing, exact
  relation/validator receipts and pool-bound reader leases. Subset creation is explicitly
  `create_conformance`; it cannot select a production facts generation. Cleanup removes schema,
  registry, receipts and generation events atomically. Retry returns AlreadyAbsent only for complete
  absence; orphaned schema/registry state requires explicit repair. Production facts admission is
  not implemented.

Focused evidence (2026-09-29; Cargo commands prefixed by `python3 scripts/build_environment.py --`):

| Command | Outcome and boundary |
|---|---|
| `cargo test --release -p lctx-model --test domain --test domain_assertions --test domain_stages` | passed: 17 domain, three assertion and four stage controls, including profile/effect/code/config identity and exact attempt permits |
| `cargo test --release -p lctx-postgres --test generation_stages --test generations` | passed: real PG18 stage-bound sink and two lifecycle/chunk tests; sibling binding, no-op completion and foreign receipt refusal, explicit empty output and COPY budget controls |
| `cargo test --release -p lctx-model --test domain --test domain_conditions --test domain_assertions --test domain_stages --test domain_resources --test domain_verdicts` | passed: 29 focused model controls |
| `cargo test --release -p lctx-model --test domain_transfer --test domain_assertions --test domain_paths` | passed: five transfer, three assertion and two path controls; independent reviewer rerun |
| `cargo test --release -p lctx-model --test domain_transfer --test domain_assertions` | passed: six transfer and three assertion controls after author guard-operand follow-up; local/corpus positives and foreign/artifact-scope refusals |
| `cargo test --release -p lctx-model --test domain --test domain_resources` | passed: 17 domain and three resource controls; independent reviewer rerun |
| `cargo test --release -p lctx-model --test domain_derivation`; `cargo test --release -p lctx-postgres --test domain_derivation` | passed: three model controls and one real PG18 four-case test; independent reviewer rerun after proof-source correction |
| `cargo test --release -p lctx-postgres --test domain_transfer` | passed: real PG18 transfer/control/selection/support readback, generated view targets, cross-scope call-site and guard-operand refusal |
| `cargo check -p cpg-extract -p cpg-core` | passed after generated invariant API change; existing third-party future-incompatibility warnings remain |
| `just build-features` | passed: CLI union regenerated; lower model macro crate excluded; no dependency pin upgraded |
| `cargo test --release -p lctx-model --test domain_calls` | passed: seven call/signature/binding controls, independently rerun after review corrections |
| `cargo test --release -p lctx-postgres --test domain_calls` | passed: real PG18 signature/call/support readback and wrong-provider refusal; contract fixtures, not producer qualification |
| `cargo test --release -p cpg-extract --test capture`; `cargo test --release -p cpg-extract --lib capture::tests` | passed: three capture controls plus changed-during-capture unit; 65 MiB input under a 3 MiB capture reservation budget, not RSS |
| `cargo test --release -p lctx-model --doc` | passed: seven negative/positive declaration pairs including nominal proof targets; one pre-existing ignored legacy example |
| `cargo test --release -p lctx-postgres --test generations` | passed: two real disposable PG18 tests, including 65 MiB evidence, stored corruption and contract refusal, write drain, leases, conformance selection refusal, cleanup retry and orphan repair |
| `cargo test --release -p cpg-extract --test typed_conformance` | passed: real pinned AST to typed records, sealed PG18 validation/readback and conformance-only selection refusal |
| `cargo test --release -p cpg-core --test model_runtime` | passed: two fresh-catalog/capability/shared-pool controls; no total-memory/RSS claim |
| `just docs-check`; `uv run python scripts/adr.py lint` | passed: documentation publication and 44 current ADR records |
| `cargo test --release -p lctx-model --test domain_lexical` | passed: four lexical controls, including independently attributed alternatives and strict/equal-span ancestry |
| `cargo test --release -p lctx-postgres --test domain_lexical` | passed: real PG18 good/foreign-optional-value conformance cases |
| `cargo test --release -p lctx-model --test domain_documents` | passed: three document shape/attribution controls, including corrected enclosing heading paths |
| `cargo test --release -p lctx-postgres --test domain_documents` | passed: real PG18 good/foreign-optional-span conformance cases |
| `cargo test --release -p lctx-model --test domain_coverage` | passed: scope/input/corpus ownership matrix |
| `cargo test --release -p lctx-model --test domain_assertions --test domain_lexical --test domain_documents --test domain_transfer` | passed: 16 controls after ownership consolidation |
| `cargo test --release -p lctx-postgres --test domain_coverage --test domain_documents` | passed: real PG18 good/foreign coverage and document cases |
| `cargo test --release -p lctx-model --test domain` | passed: 18 controls after builder-reservation follow-up; owned/borrowed identity and unit/optional sum parity |
| `cargo test --release -p lctx-model --doc` | passed after codec change: seven positive and seven compile-fail controls; one old ignored example |
| `cargo test --release -p lctx-postgres --test generations` | passed after codec change: real PG18 lifecycle and 65 MiB chunk/corruption controls |
| `cargo test --release -p lctx-model --test domain_flow` | passed after F01: three controls including the six-case broad-input structure matrix |
| `cargo test --release -p lctx-model --test domain_flow --test domain_transfer` | passed before F01: two flow and six transfer controls after composite-subject expansion |
| `cargo test --release -p lctx-postgres --test domain_flow` | passed after F01: real PG18 good/foreign-place-root conformance cases |
| `just docs-check` | passed: 193 canonical pages and zero link errors after MDX source-reference correction |
| `cargo test --release -p lctx-model --test domain_types --test domain_assertions` | passed after type F01: four type and three assertion controls, including 20 opaque-fidelity combinations |
| `cargo test --release -p lctx-postgres --test domain_types` | passed after F01: real PG18 modeled good/foreign-owner and nested truncated structural rejection/display-only acceptance |
| `cargo test --release -p lctx-model --test domain_deployment` | passed: three report/membership/source controls |
| `cargo test --release -p lctx-postgres --test domain_deployment` | passed after initial Busy failure and explicit-release correction: real PG18 three-case contract test plus two-reader retirement protection |
| `cargo test --release -p lctx-model --test domain_guard_rebase` | passed after guard F02: five controls, including direct/nested formal/receiver/local eligibility and ordinary unrebased positives |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase` | passed after guard F02: real PG18 eight-case source/eligibility matrix and typed origin readback |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase --test domain_transfer --test generation_stages --test generations` | passed after acknowledged rollback/COPY correction: five tests, including active-COPY budget refusal/success, immediate cleanup and 65 MiB; precedes final guard F02, separately rerun above |
| `cargo check -p lctx-postgres`; `cargo check -p cpg-extract -p cpg-core` | passed after transaction/COPY and guard changes; final guard F02 subsequently compiled by the focused tests above; existing third-party future-incompatibility warnings remain |
| `just docs-check` | passed at restart checkpoint: 197 canonical pages and zero link errors |
| R1: `cargo test --release -p lctx-model` | passed 2026-09-29: all suites and doctests after budgeted `Batch::new/read`; seven resource controls incl. 4096/4096/1808 and 2/1 flushes, lone 20 MiB row, refused 70 MiB row, cross-flush duplicate/conflict, pre-encoding refusal, explicit empty stage output |
| R1: `cargo test --release -p lctx-postgres --test generation_stages --test generations` plus the ten `domain_*` PG suites | passed 2026-09-29: real PG18; `StageOutput` through `GenerationAttempt` writes 5000 rows in transfer batches, emits a repeated row once and returns the attempt budget to zero |
| R1: `cargo test --release -p cpg-extract --test typed_conformance`; `-p cpg-core --test model_runtime`; `cargo check --workspace --all-targets` | passed 2026-09-29 |
| R2: `cargo test --release -p lctx-model` | passed 2026-09-29: every stored invariant retains state only through `charged` containers bound to the validation budget; cardinality caps and global closure/lineage/proof totals removed, per-condition kernel and guard-depth bounds kept; charged-container and tiny-budget invariant controls |
| R2: `cargo test --release -p lctx-postgres --test generation_stages --test generations` plus the ten `domain_*` PG suites; `-p cpg-extract --test typed_conformance`; `cargo check --workspace --all-targets` | passed 2026-09-29: real PG18 `validate(g, budget)` refuses on a 64 KiB budget leaving the generation sealed and unpublishable, then a funded retry validates with the attempt budget back at zero |
| K1: `cargo test --release -p lctx-model --test domain_calls --test domain_types --test domain_flow --test domain_guard_rebase --test domain_transfer`; the same five under `-p lctx-postgres` | passed 2026-09-29: `ProviderModule` origins stay distinct and bind bundled/unresolved modules to their provider and context; `PlaceRoot::Local` shares one place across definitions in a scope and refuses a sibling scope's variable; updated fixtures read back through real PG18 (ADR-0089) |
| D0: `cargo test --release -p lctx-model` (incl. `domain_stages`, `domain_memory`); `-p lctx-postgres --test domain_lexical --test generation_stages`; `-p cpg-core --test model_runtime`; `-p cpg-extract --test typed_conformance`; workspace check | passed 2026-09-29: contributions/coverage in the schedule digest; self/writerless/cyclic contributions refuse; contributed rows merge once into the writer and hand off to the reader, with all reservations released after the last reader; un-handed-off outputs and unmerged contributions refuse; `MemoryGeneration` refuses absent references, repeated keys and invariant failures, is batch-order independent, and its digest equals the PG18 store's for the lexical fixture (ADR-0089) |
| C1: `cargo test --release -p lctx-model` (incl. `domain_declarations`, `domain_calls`); `-p lctx-postgres --test domain_declarations`; workspace check | passed 2026-09-29: `def f(a, b=1)` symbol/parameter links validate; refused with their messages: parameter outside its symbol's declaration, a parameter with two occurrences, an occurrence with two parameters, class symbol at a def, another provider's support, another context's qualification. `CallSyntax` fixes two ordered arguments; partial/foreign argument sets, keyword-kind mismatch, a missing argument and an argument outside the call are refused; binder answers unchanged through call syntax; PG18 round trip and stray-parameter refusal, memory digest equals the store's |
| C2: `cargo test --release -p lctx-model` (incl. `domain_sites`, `domain_calls`); workspace check | passed 2026-09-29: policies are asked only of `SiteTargets` over every resolution at a site; same-provider agreement is unique and summarized, cross-provider disagreement is not (Dataflow keeps both), `C()` New+Init is one unique construction, `x()` Call+Init is not, incomplete resolutions are never summarized, `map(f, xs)` is unique on `map` with `f` excluded from Dataflow/Summary, potential targets are excluded from every call policy, an empty report is one incomplete unresolved alternative, and receivers come only from `classify_receiver` (unknown never summarized) |
| C3: `cargo test --release -p lctx-model --test domain_owner`; `-p cpg-extract --test typed_owner`; workspace check | passed 2026-09-29: `OwnerTable` sweep equals the `owner_of` oracle on a structural tree and on the real pinned Ruff parse of `semantic_owner/owner.py`; defaults, return annotations, decorators and bases belong to the module, `h()`/comprehension `k()` to `f`, `k2()` to `C`, `m2()` to its lambda; shuffled/reversed input is order independent; path gaps, repeated positions and short budgets refuse (C12) |
| C4: `cargo test --release -p lctx-model` (incl. `domain_stability`, `domain_guard_rebase`); `-p lctx-postgres --test domain_stability --test domain_guard_rebase`; workspace check | passed 2026-09-29: a witnessed whole-formal `IsNone` guard at `select_timeout(t)` becomes a `BoundGuard` over `t` while a callee-local guard stays an opaque `InvokedGuard` in the same condition; refused: missing witness (`ConditionTransferUnsupported`, never true), flow not requested (`NotRequested`), default-bound formal (`DefaultStabilityUnknown`), truthiness and attribute operands; stored refusals: bound guard without substitution, second reaching definition, Partial coverage, assignment definition, bound operand other than the call argument; `InvokedGuard→BoundGuard→formal` accepted and `InvokedGuard→formal` refused; PG18 round trip, memory digest equals the store's (C06) |
| C5: `cargo test --release -p lctx-model` (incl. `domain_composition`, `domain_transfer`, `domain_paths`, `domain_stability`); `-p lctx-postgres --test domain_composition --test domain_stability`; workspace check | passed 2026-09-29: `read_timeout(c)` maps `x` to `x.timeout` at the site; derived callers and callees drop field paths, identity facades keep them; `update(t, k, v)` stores into the caller's `t[*]` owned by the caller; `Config.__init__` field writes pass unchanged; raise/yield → `UnsupportedControlFlow`; callee-local roots and a non-owning caller → `Err`; `with_default` → `DefaultUnavailable`; `collect(a, b, k=c)` → `[1]`/`['k']`, `collect()` → Disjoint; receiver path; an unwitnessed formal guard → `ConditionTransferUnsupported`, flow not requested → `NotRequested`, a witnessed one is restated and its influence yields the Selection (C07); a callee-local guard stays conditional; b and ¬b callers merge to true with both alternatives kept; two targets → two Candidates, one binding variant Definite, two Candidate, unresolved → `UnresolvedTarget` (`map`'s `f` never binds, `CallTransfer` from C2, so never reaches composition); an oversized condition → a limit obligation. Stored: PG18 round trip of caller, callee and composed transfers, step premises and Selection, memory digest equals the store's; refused: a callee premise from another symbol, a non-composed conclusion, a callee guard not restated at the call (C05/C06/C07) |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

Independent bounded reviewers accepted artifact/capture/acquisition corrections and the
[assertion/condition foundation](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md)
after its three corrections. The [native syntax subset review](../design_review/reviews/design_review_semantic-native-syntax_2026-09-29.md)
accepted the bounded producer/store seam without qualifying P0. The
[call contract review](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md) accepted
three corrected findings, and the [input capture review](../design_review/reviews/design_review_semantic-input-capture_2026-09-29.md)
accepted the checked-copy and bounded reservation contract. The [earlier input review](../design_review/reviews/design_review_semantic-input-validation_2026-09-29.md)
still has open coordinated-resource qualification. The
[transfer/path review](../design_review/reviews/design_review_semantic-transfer-contract_2026-09-29.md)
accepted its scope after call-site ownership correction; the
[attachment resource review](../design_review/reviews/design_review_semantic-attachment-resources_2026-09-29.md)
accepted buffer accounting; the [nominal derivation review](../design_review/reviews/design_review_semantic-nominal-derivation_2026-09-29.md)
accepted typed targets, cycle validation and PG projections after proof-source identity correction.
Stage/session/lifecycle changes await their
assembled review; none of these bounded receipts establishes enclosing architecture acceptance.

**Representative lexical contracts (Implemented; focused Tested, 2026-09-29):** nominal scope, binding event and target records; qualified scope/binding/reference/resolution observations and generated supports. Optional subjects retain provenance. Shared checks enforce structural ancestry and same-source relationships while preserving independently attributed provider disagreement at a common ordinal. Real PG18 COPY/validation/readback covers the representative fixture and rejects a foreign optional value. These are contract fixtures; the raw lexical producer and complete field disposition remain P2 work.

**Representative document contracts (Implemented; focused Tested, 2026-09-29):** source-span-backed passage/code/link/mention/component identities, qualified observations and generated supports, plus typed literal/expression/bare/spread attributes. Shared checks preserve cross-heading component parents and validate subtype, parent/depth, optional-span source ownership and code-content digests. Heading paths name enclosing headings. Model and real PG18 fixtures pass; raw document producer integration and all-field parity remain P2 work. The [bounded document review](../design_review/reviews/design_review_semantic-documents_2026-09-29.md) accepted this contract scope, without qualifying producer coverage or a phase exit.

**Shared scope ownership (Implemented; focused Tested, 2026-09-29):** one `ScopeIndex` policy owns artifact/module/input/release containment and explicit direct corpus membership for both assertion supports and stored provider coverage. Matching provider/context no longer permits coverage for an unrelated input. The bounded [ownership review](../design_review/reviews/design_review_semantic-scope-ownership_2026-09-29.md) accepted the consolidation. The model matrix checks all four scope kinds with direct/corpus positive and foreign controls; real PG18 checks good and foreign input coverage. Production expected-matrix construction and complete facts admission remain open. Native Ruff traversal still has emission/depth limits, not a total-work termination bound.

**Codec allocation reduction (Implemented; focused Tested, 2026-09-29):** generated record identity hashes key fields by reference; explicit owned-key APIs remain available. Struct/sum encoders push borrowed physical rows into pinned serde_arrow builders, reserving known row capacity, without building an intermediate vector or cloning active sum payloads. Identity framing and schemas remain unchanged. Focused controls include all-unit/optional sums and a 1 MiB value; PG18 chunk readback still includes 65 MiB input. This removes known intermediates; it does not establish complete reservations or RSS limits. The [bounded codec review](../design_review/reviews/design_review_semantic-codec-allocation_2026-09-29.md) accepted the correction.

**Representative raw flow (Implemented; focused Tested, 2026-09-29):** nominal uses/definitions refer to shared occurrences and places; qualified use/definition/reaching/value/region observations carry generated supports. An explicit Unbound target differs from absent coverage; raw identity cannot pass through an unresolved call. Shared validation includes transitive Place roots and same-source event/scope/value geometry, independently of broad coverage authorization. The [bounded flow review](../design_review/reviews/design_review_semantic-flow_2026-09-29.md) accepted F01's correction. These are contract fixtures; full ty emission, call steps, tests/type leaves and P2 field parity remain open.

**Representative type contracts (Implemented; focused Tested, 2026-09-29):** structural terms reuse nominal native symbols and lossless literals; provider/context/module/anchor/slot/origin/kind identify type variables. Ordered sequence membership is digest-checked. Recursive restrictions and alternate presentations remain qualified relationships outside variable identity. Shared support validation checks transitive native owners and requires DisplayOnly when any dependency is opaque/truncated. The [bounded type review](../design_review/reviews/design_review_semantic-types_2026-09-29.md) accepted F01's correction. Model controls and real PG18 four-case readback/refusal pass; complete native type forms and Pyrefly producer migration remain P2 work.

**Representative deployment contracts (Implemented; focused Tested, 2026-09-29):** typed report values and digest-checked ordered/keyed collections preserve commands, tools, arguments and environment metadata. Reported environments and task results remain captured claims; receipt/target source ownership is independently checked. Full unsigned duration roundtrips without narrowing. Model controls and real PG18 good/foreign-target/missing-child cases pass. An observed asynchronous drop/retire race was corrected with consuming, acknowledged `GenerationLease::release`; two-reader controls retain Busy until both readers release. The [bounded deployment review](../design_review/reviews/design_review_semantic-deployment_2026-09-29.md) accepts this scope. Actual receipt parsing, verification, nested Scenario/OptionBinding relationships and complete field mapping remain P2 work; full reader/provider lifecycle remains P1.

**Local guard rebasing (Implemented; focused Tested, 2026-09-29):** appended typed InvokedGuard predicates retain the original atom and every nested invocation; caller occurrences distinguish call sites. BDD simultaneous substitution preserves condition structure without existential elimination. Construction and stored validation share local operand eligibility, refuse formal/receiver rebasing without binding/stability evidence, and enforce bounded lineage/context checks. Support validation authorizes every origin evaluation and operand. Five model controls and a real PG18 eight-case matrix pass after the two corrections in the [bounded guard review](../design_review/reviews/design_review_semantic-guard-rebase_2026-09-29.md). This is a contract fixture, not whole-call substitution or a migrated producer.

**Acknowledged transaction cleanup (Implemented; focused Tested, 2026-09-29):** the generation store awaits commit/rollback for completed operations and explicitly aborts active COPY before rollback on encoding/send/budget refusal. Failed finalization remains an unconfirmed outcome and quarantines the connection. This corrects the observed immediate-abort Busy failure; paired low/high COPY budget controls and adjacent lifecycle/transfer tests pass. The [bounded rollback review](../design_review/reviews/design_review_semantic-generation-rollback_2026-09-29.md) accepts the source correction. Cancellation retains SQLx drop cleanup; production cancellation, transport-failure injection and assembled provider lifecycle remain open.

**Restart checkpoint (2026-09-29):** stopped at the user's requested boundary after completing the guard and transaction corrections. No phase exit is qualified. The old production pipeline remains active; representative domain fixtures do not establish producer migration. Resume from the next paragraph, preserving §4.1.1 order. Formatting, integrated gates and pilots remain not_run until the authorized functional scope is complete.

**Next / still open:** the §4.1.1 packages in order, starting at R1 (reserved batches and the
bounded writer). Whole-call composition, stability-witnessed substitution, call-site facts,
occurrence ownership, coordinated allocation (including total Ruff traversal work), the facts
frontier contract and the stage-bound production subset complete P0; P1 and P2 follow their
tables. Earlier Phase 0 receipts do not qualify this target.

## 5. Deletion and preservation obligations

Delete legacy code by ownership boundary after preserving raw-field mappings and independent controls.
No legacy adapter inventory is maintained. Preserve fixtures, oracles, protected benchmarks, evaluation
isolation and unrelated operational services. Git retains removed implementation. Old runtime readers
must be quiesced before replacing or retiring project state.

Downstream code owned by P3–P5 is not deleted in P0–P2 (operator decision 1 in §4.1.1). It stays a
compiling workspace member without a runnable path; tests needing removed runtime are ignored with
an owner or moved to a non-target `tests/dormant/` directory. `cpg-schema` survives P2: P2-C
removes only what loses its last consumer, and C2x records each surviving module's dormant consumer
and retiring phase here. `cpg-extract` and `cpg-flow` must not depend on it after C1x.

## 6. Deferred capability obligations

Phase 4 retains finite summaries/discharge, completion/evaluation, frame exits and call execution,
context protocols/values, modeled identities/actions, behavioral reachability, communities/PageRank,
FCA/RCA and optional kNN wherever their existing consumer remains. Retirement needs consumer evidence
and an explicit decision; deleting old code during reconstruction does not retire these obligations.

Also retained for Phase 4: behavioral-profile dependency context for authored-model classes and
exact runtime exceptions (deferred from P2 by operator decision 3); the producer build digest's
authored-model catalog component; scenario, option-binding, association, requirement and check
conclusions, whose raw inputs P2 records must carry. Phase 3 owns cross-provider symbol
equivalence, stored call bindings and SQL policy views (C04 remainder), stored occurrence
ownership and the test-leaf to operand-type join.

## 7. Tooling, pins, skills and documents

Phase 0 adds the bounded derive and explicit Arrow codec dependencies. Phase 1 removes Delta family
and skill selection and replaces store/CLI/PG test commands. Phase 2 updates fact owners and tests.
Model, PG and producer digests, generated declarations and SQLx metadata must be regenerated from their
owners. Phases 3–5 update their owners when implemented. No documentation labels target as Tested
without corresponding evidence.

## 8. Findings disposition

| Source finding | Current disposition | Owner and closure evidence |
|---|---|---|
| [flow F01](../design_review/reviews/design_review_semantic-flow_2026-09-29.md) | addressed within reviewed slice | Flow source-structure invariant separates file geometry from input ownership; broad-input negatives for use/definition/region scope, definition value and value sink, plus acquired foreign Place-root positive; independent reinspection accepted |


| Source finding | Current disposition | Owner and closure evidence |
|---|---|---|
| [codec-allocation F01](../design_review/reviews/design_review_semantic-codec-allocation_2026-09-29.md) | addressed within reviewed slice | Both generated ArrayBuilders reserve the known row count; source reinspection and post-correction domain18 pass; runtime/RSS impact unmeasured |


The bounded [lexical review](../design_review/reviews/design_review_semantic-lexical_2026-09-29.md)
accepted corrected representative contracts on 2026-09-29; P2 mapping remains open.

| Source finding | Current disposition | Owner and closure evidence |
|---|---|---|
| lexical F01 | addressed within reviewed slice | Raw lexical validator no longer imposes consensus over provider-independent qualification/ordinal; two independently supported interpretations coexist in the focused model control |
| lexical F02 | addressed within reviewed slice | Reference parents require strict structural ancestry; self-parent and unrelated same-span refusals plus valid equal-span ancestry; independent source reinspection accepted |


The bounded [stage/sink review](../design_review/reviews/design_review_semantic-stage-sink_2026-09-29.md)
accepted the corrected boundary on 2026-09-29; full production admission remains open.

| Source finding | Current disposition | Owner and closure evidence |
|---|---|---|
| stage-sink F01 | addressed within reviewed slice | `lctx-model` exclusive sink binding plus `lctx-postgres` generation-specific successful-output tracking and atomic stage receipts; real PG18 sibling/no-op refusal and explicit-empty positive; independent source reinspection accepted |

The bounded [input-validation review](../design_review/reviews/design_review_semantic-input-validation_2026-09-29.md)
has the following separate finding namespace (2026-09-29):

| Source finding | Current disposition | Owner and closure evidence |
|---|---|---|
| input-validation F01/F03/F04/F05 | addressed within reviewed slice | `lctx-model` input/source invariants; real PG manifest/span/cross-input ownership refusals and multi-distribution positive; reviewer source reinspection accepted |
| input-validation F02 | open → §4.1.1 R1–R3, P1.9 | model/store/producer coordinated memory accounting including conversion copies and invariant state; large-input measured envelope and refusal controls required; server row admission and per-row COPY alone are partial |


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


### Assertion foundation review findings

The [bounded assertion review](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md)
reinspected these corrections on 2026-09-29. Closure is limited to the foundation; P0-E assembly stays open.

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [F01](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md#F01) | `lctx-model::domain::assertion` | closed (bounded): condition evaluation input/scope validation; same-input/corpus positive and foreign/missing/out-of-scope negative controls |
| [F02](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md#F02) | `lctx-model-macros` | closed (bounded): keyed qualification required by derive; paired declaration controls |
| [F03](../design_review/reviews/design_review_semantic-assertion-foundation_2026-09-29.md#F03) | `lctx-model::domain::model` | closed (bounded): generated companion dependency; incomplete membership rejected |


### Bounded native call contract findings

The [call review](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md) independently
reinspected corrections and reran seven controls on 2026-09-29. P2/P3 and C04 closure remain open.

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [F01](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md#F01) | `lctx-model::domain::calls` | closed (bounded): native provider/support ownership invariant; signature/call positive and crossed-provider controls plus real PG18 refusal |
| [F02](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md#F02) | `lctx-model::domain::calls` | closed (bounded): shape lookup identity checked before binding; forged optional-for-required map refuses |
| [F03](../design_review/reviews/design_review_semantic-call-contract_2026-09-29.md#F03) | `lctx-model::domain::calls` | closed (bounded): binding kind and aggregate projections; keyword keys and receiver/varargs/implicit positions retained |

### Transfer and derivation review findings

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [transfer-contract F01](../design_review/reviews/design_review_semantic-transfer-contract_2026-09-29.md#F01) | `lctx-model::domain::assertion` | closed (bounded): populated call site shares endpoint scope/acquisition checks; local/corpus positives and unrelated/artifact-scope refusals, real PG18 publication refusal, independent reinspection |
| [nominal-derivation F01](../design_review/reviews/design_review_semantic-nominal-derivation_2026-09-29.md#F01) | model derivation/derive/common invariant | closed (bounded): retain source step and conditional conclusion-to-source edge; self and mutually dependent explicit steps refuse; model and real PG18 independently rerun |

### Type contract review findings

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [type F01](../design_review/reviews/design_review_semantic-types_2026-09-29.md#F01) | type/support validation and attribution derive | closed (bounded): transitive opaque/truncated dependency requires DisplayOnly; direct/nested model matrix and real PG18 refusal/acceptance, independent reinspection |

### Deployment and reader-release review findings

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [deployment F01](../design_review/reviews/design_review_semantic-deployment_2026-09-29.md#F01) | `lctx-postgres::generations` | closed (bounded): consuming explicit release acknowledges unlock before cleanup; two-reader PG control retains Busy after first release and permits retirement after second; broader P1 lifecycle open |

### Guard and transaction correction findings

| Finding | Responsible component | Current disposition and evidence |
|---|---|---|
| [guard F01](../design_review/reviews/design_review_semantic-guard-rebase_2026-09-29.md#F01) | model qualification/support validation | closed (bounded): qualification inputs match their handler; support keeps Predicate/GuardIndex; corrected real PG18 guard validation passes |
| [guard F02](../design_review/reviews/design_review_semantic-guard-rebase_2026-09-29.md#F02) | model guard construction and stored invariant | closed (bounded): shared local-root policy rejects direct/nested formal/receiver wrappers; model five-test and PG18 eight-case controls pass, independent source reinspection accepted |
| [rollback F01](../design_review/reviews/design_review_semantic-generation-rollback_2026-09-29.md#F01) | generation transaction/COPY boundary | closed (bounded): awaited rollback and active COPY abort; immediate cleanup and paired budget controls pass in the five-test PG18 suite; cancellation/transport injection remain P1 work |

### Core review findings

All findings from the [core review](../design_review/reviews/design_review_cutover-core_2026-09-29.md)
remain open; acceptance requires implementation plus focused evidence and the assembled P0 exit review (§4.1.1 X0).

| Finding | Owner and closure |
|---|---|
| C01 | P1.7 (lifecycle), P1.10 (lease): schema-per-generation retirement, reader lease and select/retire concurrency controls |
| C02 | P1.10: reader rejects model/physical digest mismatch before decoding |
| C03 | D0, C6: typed model membership and sole stage writer authority, including self-cycle refusal |
| C04 | C2, A7/A10, P3 (equivalence, views): attributed direct/potential/higher-order alternatives; one normalization owner |
| C05 | C5: paths compose only through identity; map complete binding sets and boundary outputs |
| C06 | C4, C5: preserve opaque local guards; refuse unsupported substitution rather than erase conditions |
| C07 | C5: typed ControlInfluence and Selection with separate value-transfer meaning |
| C08 | X0 re-confirmation: transfer key excludes merged condition; stable derivation references |
| C09 | X0 re-confirmation: scope-boundary unknown; approximation obligation; rendering budget separate |
| C10 | C2: receiver Unknown and one classification policy |
| C11 | C1, A3–A4, A14, B2: acyclic source identities, typed syntax kinds and structural occurrence discriminators |
| C12 | C3, B2: indexed region join with scalar oracle and unresolved ambiguity |
| C13 | implemented derivation contract; P4/P5 consumers: derivation targets follow typed references and generated view grants |
| C14 | P0.1: DESIGN §15.1 assigns provider registration to cpg-core and PostgreSQL effects to lctx-postgres |

## 9. Risks

The principal risks are incomplete raw-family migration, parallel semantic definitions, invalid
publication races and accidental claims of restored downstream capability. The field mapping, single
root model, real concurrency tests and generation availability frontier address these directly.

## 10. Deferred, each with a trigger

Stage reuse requires the existing measured-workload trigger. New reasoning engines and application
Salsa require a concrete consumer. PostgreSQL performance tuning follows measured phase-exit costs.
Cross-release symbol matching waits for its consumer. Product PR6 waits for phase 5 qualification.
