# PostgreSQL integration: assembled source review

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Working-tree PG0–PG7 implementation over `5e623535ed0ad836b633e5788cd6c3ea1765a248`: schema receipts, `cpg-core` PostgreSQL/cache/attempt/publication/bundle boundaries, `lctx` operations, migrations and deployment/check/backup scripts |
| Standard | [Core 3.0](../design_principles/core/design-principles.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md), [repository binding](../design_principles/binding/library-context.md) |
| Tier · purpose | Design · target; assembled implementation source and adjacent consumers |
| Reviewer · date | Independent delegated design reviewer, 2026-09-27 |
| Evidence strength | **Implemented, source-inspected**; no reviewer runtime qualification |
| Decision | **Accept scoped** for the ownership, representation and composition boundaries described here. Runtime/release acceptance remains unresolved |
| Included | Exact vectors consumed by operation documents, E0 analytics and briefs; canonical publication and offline bundle inputs; shared cache admission; operational history/discovery; connection and migration lifecycle |
| Excluded | Successful deployment/restore/performance claims; live embedding conformance; complete Stage 3 semantics; PostgreSQL serving/search, direct Python SQL access and the conditional F1–F10 capabilities |
| Method | Targeted source reads and dependency/consumer traces, followed by re-inspection of corrections made during review. Concurrent working-tree changes and running tests are not a fixed release artifact |

The owning contracts are [storage §6.5](../../design/sections/storage-and-publication.md#section-6-5)
and [serving §11.4](../../design/sections/synthesis-and-serving.md#section-11-4).
The [PostgreSQL plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream) owns implementation
and qualification; the [forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
owns the six findings from the storage and stack reviews. This review supplies source evidence,
not a second status register or a claim that those findings have closed.

The operator-authorized sequence puts application integration before incomplete Stage 3 P7.
There is no architectural objection to that sequence while preserved baseline/evaluation artifacts
remain identifiable, compiler/schema changes get new identities and fresh stores, and PostgreSQL
qualification is not reported as Stage 3 semantic acceptance. Those are continuing conditions of
this bounded decision.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and consumer contract | Change boundary |
|---|---|---|
| [`cpg-schema::embedding`](../../../crates/cpg-schema/src/embedding.rs) | Receipt tables, vector codec/digest recipes and append-only consumer bits | A representation/consumer-meaning change is explicit schema work; PostgreSQL does not define semantic IDs |
| [`embed::Session`](../../../crates/cpg-core/src/embed.rs) | One attempt's retained vectors and consumer uses; return the retained committed winner | Shared cache effects are optional only through the explicit uncached fixture route; production embedding requires configured service |
| [`postgres`](../../../crates/cpg-core/src/postgres/mod.rs) and its cache module | SQLx configuration/pool/schema checks, immutable reusable values and typed failure | Effects remain below compilation orchestration; provider I/O does not hold a database lease |
| [`attempt`](../../../crates/cpg-core/src/attempt.rs), [`validate`](../../../crates/cpg-core/src/validate.rs) | Write receipt rows, validate canonical facts, incorporate receipt identity, then publish | The Delta `snapshots` append remains the publication event |
| [`bundle`](../../../crates/cpg-core/src/bundle.rs) and pinned native/Python serving | Read exact values from the selected snapshot and serve a pinned generation | Bundle reconstruction has no PostgreSQL/provider dependency |
| [`postgres::operations`](../../../crates/lctx-postgres/src/operations.rs), [`lctx::db`](../../../crates/lctx/src/db.rs) | Append observations; reconstruct discovery from canonical storage and verified manifests | Database discovery neither publishes a snapshot nor establishes a semantic verdict |
| `postgres::legacy` (removed by ADR-0078) | Explicit read-only import of a selected historical Delta cache version | Retained legacy schema is migration input, not a second runtime cache mode |

The dependency direction is CLI → attempt/effect owners → schema contracts. Analytics still
consumes Arrow inputs rather than a database handle. A new crate or generic storage interface
would add indirection without improving the current variation boundary.

| Fact/representation | Provider and fidelity | Coverage/unknowns | Identity and consumers |
|---|---|---|---|
| Consumed vector | Named embedding spec/provider; exact retained bytes of a heuristic representation | Only values actually returned to consumers; byte replay is not a claim that similarity establishes behavior | Snapshot/spec/input key plus value digest; operation vectors, E0 and brief vectors |
| Consumer use | Attempt collector; observed use attribution | Positive known bit mask; operation/brief document coverage has shared rules | Same full receipt key; validation and replay accounting |
| Publication observation | Verified canonical Delta metadata; observed operational fact | Reconciliation leaves missing historical library/start time unknown | Attempt/snapshot identity and canonical digests; run/discovery CLI |
| Generation discovery | Verified files/manifest and matching canonical publication | Missing/invalid paths become unavailable within reconciliation scope | Location, source store, generation/snapshot/manifest digest; discovery CLI |
| Behavioral claims | Existing Rust/Arrow/DataFusion analysis and native executor | Existing verdict/coverage/model limits persist | No new PostgreSQL semantic projection or reinterpretation in this scope |

## 3. Contracts, constraints and testing boundaries

| Contract | Implemented enforcement | Failure/lifecycle boundary and qualification need |
|---|---|---|
| Exact winner | `Store::admit` inserts immutable full keys, then reads committed winners; `Session` retains winners before return | Retryable insert and winner-read failures enter the bounded loop; real races/ambiguous completion still require runtime evidence |
| Exact local receipt | Versioned little-endian float bits, per-value digest and sorted aggregate receipt digest; shared row/usage/spec/vector validators | Invalid/missing values prevent publication; signed zero and corrupt rows need independent cases |
| Bounded work | Session byte limit, batched cache reads/writes, bounded pool/acquire/statement/lock settings; provider calls outside a lease | Limits fail explicitly; timeout/cancel/reuse and representative residency remain qualification work |
| Schema compatibility | Explicit migration command and runtime exact migration-history/version/checksum checks; actual major 18 check | Normal compilation never migrates. Offline SQLx metadata and real role/schema drift cases are separate checks |
| Operational authority | Append-only attempt/events; reconciled observations identify unknown history; discovery is reconstructible | A failed journal write does not unpublish Delta. An unfinished run is not labeled interrupted without evidence |
| Configuration | Protected file, redacted configuration debug, disabled statement logging and sanitized database errors; verified remote TLS mode | Host credentials, role grants and deployed transport must be exercised; source inspection cannot certify them |
| Legacy admission | Pinned Delta version, exact requested keys in batches, tokenizer/spec re-admission and byte comparison against existing winners | Duplicate keys/conflicts are explicit errors; partial imports are retryable by immutable identity |

## 4. Composition and execution

`attempt::finish` creates one collector, routes operation documents, E0 and brief embedding calls
through it, then consumes its network-free receipt. Persisted receipt/use rows pass the same
publication validation used elsewhere before the canonical snapshot append. Bundle vector joins
use the snapshot-local receipt. Restoring or replacing the shared cache cannot change values
already retained by that attempt or the persisted input of a later bundle reconstruction.

| Stage/question | Projection and method | Model, budgets and evidence linkage |
|---|---|---|
| Which operations have vector representations? | Existing operation documents; one canonical embedding spec/request recipe | Vector similarity remains heuristic. Document/use/receipt joins preserve full snapshot/spec/input identity |
| Which E0 inputs influenced analytics? | Existing E0 projection and analytic method; embedding acquisition now shares the attempt collector | Existing analysis settings and provenance remain authoritative. Analytics-only values enter receipts even without a served document |
| Which briefs can be ranked? | Existing brief documents and vector rendering | Canonical receipt bytes feed the bundle; no query-time database dependency or new behavioral inference |
| What was published/generated? | Canonical snapshot catalog and verified generation manifest, then operational observations | Reconciliation records observation time, not invented execution/creation time; pagination is bounded |

Reconciliation captures a database-clock cutoff before enumeration. It registers canonical
publications, considers only verified generations belonging to the selected store's published
snapshot set, and marks older missing discoveries unavailable within that store/root. It preserves
durable history and newer concurrent discoveries. Historical attempt paths remain historical when
a store moves; new discovery locations are separate observations.

## 5. Change and failure scenarios

| Trigger | Owner and propagation | Source assessment and settling runtime case |
|---|---|---|
| Two attempts submit different bytes for the same key | Cache decides the committed winner; each collector returns and retains that winner | No downstream caller chooses independently. Exercise concurrent distinct candidates and post-commit read failure |
| PostgreSQL disappears during embedding or after publication | Required cache fails compilation before publication; optional journal failures remain diagnostic; later bundle uses Delta | Exercise both sides of publication and PostgreSQL-offline byte-identical rebuild |
| A new embedding consumer is added | Add its meaning to schema `Usage`, call the shared session, add consumer-specific coverage if needed | `KNOWN_MASK`, Rust validation and existing SQL bit checks derive from one owner; no duplicated numeric classifier remains |
| Lost start/published events or a moved store | Reconciliation reconstructs observed publication, preserves unknown history and scopes discovery availability | Exercise lost events, repeated reconciliation, old/new paths and concurrent discoveries |
| Shared generation directory contains other stores or a removed source store | CLI checks the selected canonical snapshot set before registering a generation; verifies canonical digests | Exercise whole CLI scanning, not only direct repository methods; unrelated generations must not abort or be invalidated |
| Import a large historical cache | Payload-free duplicate/count checks plus exact requested-key batches | Exercise partial import/retry/conflict; vector residency scales with the requested batch rather than the entire historical cache |
| Add PostgreSQL-backed serving/search later | Separate F2/F3 projection/import/pinning contract and native-answer parity | Current cache/discovery modules are reusable effects, not evidence that relational serving is implemented |

## 6. Correctness and fidelity gates

Verdicts below apply to **source ownership and composition**, not successfully executed behavior.
The separate runtime column prevents the source decision from certifying PG7.

| Gate | Source verdict and evidence | Runtime/enclosing boundary |
|---|---|---|
| G1 Authority | Pass: schema owns receipt meaning; Delta owns publication; discovery is explicitly derived | Migration/schema/role execution pending qualification |
| G2 Semantic fidelity | Pass: exact bytes and full keys retained; heuristic vectors not promoted to behavioral facts | Real replay and admission cases unresolved |
| G3 Validity | Pass: shared shape/spec/digest/use coverage validation precedes publication | Updated schema/rule regression suite unresolved at review cutoff |
| G4 Hidden behavior | Pass: explicit configuration/migration, no live database lookup during normal offline SQLx compilation, no provider in bundle replay | Deployed configuration and offline-build execution unresolved |
| G5 Consistency/recovery | Pass: single canonical append, explicit observations, scoped cutoff reconciliation and immutable cache keys | Concurrent failures, interruption, restore and moved-path CLI behavior unresolved |
| G6 Transformation/reuse | Pass: exact versioned codec/key/spec and attempt-local retention; same receipt drives bundle vectors | Byte round trips and restored-cache conflicts unresolved |
| G7 Truthful capabilities | Pass within this review: source labels and conditional future scope are explicit | Handoff must retain actual failing/pending checks and Stage 3 limits |
| G8 Library leverage | Pass: existing SQLx pool/migrations/query/transaction mechanisms; bounded domain adapter | No performance superiority claim; bulk/provider alternatives remain conditional |
| CI-G1 Meaning/model | Pass for this storage change: no new interpretation of behavior, unknowns or ranking | Existing Stage 3 semantic findings remain outside this acceptance |
| CI-G2 Evidence closure | Pass for the changed vector boundary: snapshot-local inputs and full-key joins | End-to-end serving/evidence parity remains PG7 work |
| CI-G3 Evaluation independence | Pass for inspected boundaries: no new reference/evaluation input path | Preserved baselines and newly recorded identities remain a sequencing obligation |

## 7. Findings and applicability

No material source defect remains open from this review at its cutoff. The following corrections
were requested and re-inspected during the review; this is dated source evidence, not a claim of
runtime closure or a new execution-status owner.

| Corrected cause | Consequence in the earlier draft | Inspected correction | Qualification owner |
|---|---|---|---|
| Recovery treated discovery as insert-only and observations as complete history | Lost events stayed unfinished; stale paths survived; unrelated generations could block a selected-store scan | Migration 002, `observe_publication`, cutoff/scoped `finish_reconciliation`, canonical generation verification and selected-store filtering | PostgreSQL plan PG6/PG7 operational-state cases |
| Admission/import paths did not consistently preserve bounded reconciliation | A transient winner-read failure escaped retries; legacy import loaded all vectors | Winner-read retries share the admission loop; legacy payload reads are request-filtered batches | PG4/PG5/PG7 concurrency and migration cases |
| Consumer attribution meaning was not fully centralized/enforced | Document coverage could exist without the proper usage bit; extending bits repeated decisions | Schema-owned `Usage`/`KNOWN_MASK`/`valid_mask`; SQL checks derive operation/brief bits from it | PG3 receipt/schema/validator cases |
| Optional journaling did not fully respect optional availability | Missing configuration or stale schema could disrupt a no-embedding compile | Optional configuration path and connection/schema failures produce diagnostics without a required cache path | PG6 optional-journal failure cases |

FP-01–FP-06 are **satisfied at source scope**: coherent effect owners, declared contracts,
composition through one session, schema authority, explicit persistent structures and locally
traceable effects. DP-01–04, DP-09, DP-15–16, DP-18–19, DP-21, DP-23–24 and CI-01/02/09–13 are
supported by those mechanisms. This review makes no new graph-projection or semantic-solver claim;
their broader correctness and existing findings are not settled here.

## 8. Library fit and total complexity

SQLx's PostgreSQL pool, migrations, bound queries and transaction mechanisms supply the actual
effect boundary. The adapter adds project-specific identity, receipt/admission checks and
operational authority, rather than a second generic driver framework. SQLx query macros cover
selected stable queries; runtime-bound operational SQL still needs real-server tests. Neither
offline metadata nor successful compilation would validate its operational semantics by itself.

The prior [stack review](design_review_postgresql-stack_2026-09-27.md) owns the broader library
comparison. This source review does not claim fresh external API research or runtime compatibility.
No Python database library, ORM, generic Arrow bridge or database extension is necessary for these
Rust-owned consumers. Psycopg/SQLAlchemy, typed COPY, relational serving and pgrx remain questions
for their explicit consumer triggers.

## 9. Alternatives and tradeoffs

| Alternative | Change/authority consequence | Decision and revisit |
|---|---|---|
| Continue mutable Delta cache | Keeps one engine but retains the concurrent reuse/publication problem addressed by exact local receipts | Rejected for initial shared cache scope; Delta remains canonical facts/receipt storage |
| Current SQLx effects plus canonical receipts | Adds a service, schema lifecycle and restore work; localizes reuse/operations while preserving file serving and canonical facts | Appropriate source boundary; deployment cost must be justified with PG7 measurements |
| Rust-Postgres/Cornucopia/Refinery | Can serve typed SQL consumers, but changes driver/generated-code/migration integration without removing receipt or publication responsibilities | Revisit for a concrete typed/bulk requirement; PGK/F01 is not closed by selecting SQLx |
| Uncached embedding only | Smallest effect surface, but repeats expensive work and lacks shared reuse | Useful explicit fixture path; not an automatic fallback for required production cache failures |
| PostgreSQL canonical/serving replacement | Moves validators, import/publication/read semantics and native-serving responsibilities into a larger change | Conditional F2/F5/F9; unsupported by this source-only acceptance |

## 10. Verification and uncertainty

**Implemented/source-inspected, 2026-09-27:** the paths and symbols above were read, including
the corrected reconciliation, retry, import and consumer-authority paths. The review performed
no product test, benchmark, deployment, migration or restore command. Implementer-reported running
tests are not adopted as completed receipts here.

| Required evidence | Reviewer outcome | Meaning |
|---|---|---|
| Focused receipt/schema/validator and real PostgreSQL cases | `not_run` by reviewer | Implementer's actual commands/results belong with PG3–PG6 qualification; source existence is not a pass |
| `just test-postgres`, SQLx offline/check route | `not_run` by reviewer | Actual PG18 image/version, app-role grants and database behavior require executed receipts |
| `just test-all`, `just pilot` | `not_run` by reviewer | Integrated acceptance remains PG7; no enclosing Stage 3 certification |
| Live embedding, PostgreSQL-offline replay, backup/restore, rollback and measured compile cost | `not_run` by reviewer | Proposed qualification obligations; fake-provider mechanics cannot qualify live W9/W16 |

The highest-value remaining operational challenge is a whole-CLI reconciliation fixture containing
two stores in one generation root, a moved/missing source, missing prior events and a discovery
written after the cutoff. Direct method tests alone cannot exercise directory selection and path
normalization. Admission/replay checks must independently compare actual bytes, including
analytics-only values and conflicting restored cache state.

## 11. Authority changes and dispositions

Historical ADR-0065/0066 (now superseded by
[ADR-0068](../../adr/0078-current-design-cutover.md)) and
ADR-0067 (retired 2026-09-29; superseded by ADR-0083 (also retired; current owner: [ADR-0086](../../adr/0086-immutable-postgresql-generations.md))) provide the decision
route for the new service/receipt boundaries and their affected predecessors. Accepted decisions
do not establish implementation qualification. The architecture owners and final handoff must
match the implemented split and actual verification results.

The six earlier PGS/PGK findings retain their IDs and single
[forward-plan disposition owner](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).
This review supports receipt/cache/operational source assessment without closing runtime evidence
requirements, external-stack composition or deferred serving/federation/replacement findings.
F1–F10 remain conditional on their named product or measurement triggers; installing PostgreSQL
does not authorize speculative second semantic owners.

## 12. Architectural judgment and decision

| Judgment | Verdict at source scope | Basis |
|---|---|---|
| A1 Localize change | Satisfied | Connection/admission changes stay in effects; consumer meaning stays in schema/session; discovery repair stays in operations/CLI |
| A2 Encode meaning structurally | Satisfied | Full receipt keys, versioned bits/digests, explicit use masks, unknown reconciled history, source-qualified available discovery |
| A3 Extend through composition | Satisfied | All embedding consumers share one collector; publication/validation/bundle consume ordinary canonical tables; operational observations reuse verified canonical metadata |

**Accept scoped:** the assembled source boundary is coherent and the review's material source
corrections are present. This enables continued PG7 qualification, not a completion claim. Runtime
admission/recovery/replay, deployed restore/cost and integrated serving behavior remain unresolved
until their named evidence is recorded. The enclosing Stage 3 architecture remains subject to its
own open semantic findings and acceptance boundary. The next owner action is PG7 qualification
and evidence-calibrated handoff, with a source re-review only if failures require material changes.
