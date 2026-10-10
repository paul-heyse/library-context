# SurrealDB architecture, recoverable transitions and selective execution

**Proposed implementation target · 2026-10-10.** This companion integrates the
[architecture/capability review](../design_review/reviews/design_review_surrealdb-architecture-and-capability-leverage_2026-10-10.md),
F01–F07 and all nine investigation avenues. The
[persisted coordinator §8](persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns their sole current disposition, using SA-F01–SA-F07 aliases; §9.1 owns execution receipts.
Package descriptions and coverage here are not a second progress ledger. This target refines
UP/HS/NE/PC/GK/GR; conflicting open physical routes yield to it while their acceptance obligations survive.

## 1. Outcome, baseline and assessed foundations

Make installed-store upgrades complete, recoverable operations; prepare unchanged definitions
once per valid operation; let sparse requests examine relevant membership and matching documents;
and execute bounded sets without decomposing them into sequential per-item database calls.
Preserve the explicit semantic model, immutable completed views, original provenance, exact pins,
independent admission, terminal certainty and specialized finite Rust computation.

Authoring inspected dirty `main`, HEAD `125e74f5775e1354ce900cc8d71b190e311dac27`, on2026-10-10.
Current owners are [storage §6](../design/sections/storage-and-publication.md),
[semantic §15](../design/sections/semantic-model.md) and
[serving §11](../design/sections/synthesis-and-serving.md), with native upgrade/control,
publisher and host-service adapters. The applied standard is core/template3.3, heuristics1.0
and code-intelligence1.5 through [standard.toml](../design_review/design_principles/standard.toml).
Authoring performs no migration, service, production-code or storage operation.

The failed immutable installer `53f28eeb61887c168fdd2ecb35a662d8aea35b1bfd7b53779b5b290992f14369`
omits required retirement fields. Run `20261010T174350.233Z-05d48c` failed and cleaned up;
main's format4 publication is retained, validation remains format3/admission closed. Queued
native bodies did not run. These are historical scope-specific receipts, not fresh observations
or acceptance of later source; coordinator §9.1 retains their exact meaning.

The workload combines concurrent commands/worktrees, repeated compilation, retained releases,
small evidence requests, complete analytics, high-degree ownership and accumulating history.
Output limits alone do not bound examined work. Necessary complete validation/analytics remain
complete; avoidable repeated preparation, unrelated selection and singleton crossings are corrected
without waiting for a timing campaign. No speed, capacity or memory reduction is quantified here.

| Foundation | Assessment and consequence |
|---|---|
| Model, exact views and recovery closure | Suitable semantic authority. Share declarations/preparation, never producer expectations as the oracle for actual or cold admission. |
| Stable service and immutable host candidates | Suitable deployment owner. Existing absent/intent-only replacement cannot recover a partially translated scope; introduce a checked reconciliation successor. |
| Existing native journal | Suitable original migration identity, inadequate recovery progress. Separate translation, reconciliation and verifier progress underneath it. |
| Native indexes, aliases and physical envelopes | Suitable set-access foundation. Limited relation reads and occurrence windows need typed per-node answers and selective ordered access. |
| Frozen lexical statistics | Suitable publication-scoped answer policy. Native FULLTEXT nominates matches; its global scores do not become ranking authority. |
| Retirement, eras, references and resource pools | Suitable guards/lifetimes. Batch physical crossings while preserving per-item checks, original outcomes and charged ownership. |

No universal graph/cache/workflow framework or second canonical store is needed. A new fact
family changes its model/codec/closure adapter; an analytic declares its actual projection;
a new release coexists through exact identities; a transport substitutes at the existing native boundary.

## 2. Confirmed rule changes and decision route

| Source-qualified item | Operator decision | Required route before dependent effects |
|---|---|---|
| [Review RC01](../design_review/reviews/design_review_surrealdb-architecture-and-capability-leverage_2026-10-10.md#RC01) | **Accepted,2026-10-10:** complete compatible transition and durable atomic pass/page progress | SA0 records a complementary migration decision to ADR-0145 and updates storage §6.3/runbook. Preserve immutable identity, original authority, legacy protection and independent validation. |
| **SA-RC04 — supplemental authoring discovery** | **Accepted,2026-10-10:** checked successor for partially translated unpublished scopes | Extend the explicit recovery contract beyond absent/intent-only replacement. Preserve original migration identity and completed publications; reconcile actual state without fabricated historical progress; refuse unrecognized states. SA2/SA3 depend on this route. |
| [Review RC02 — conditional](../design_review/reviews/design_review_surrealdb-architecture-and-capability-leverage_2026-10-10.md#RC02) | Carry forward accepted [UP-RC07](unified-persistent-surrealdb-plan_2026-10-09.md#2-confirmed-rules-and-decision-routes),2026-10-09 | SA8 may select a complete qualified transport alternative; endpoint rules/SDK hold change only with its full contract and normal exact dependency policy. Patched gRPC remains initial. |
| [Review RC03 — conditional](../design_review/reviews/design_review_surrealdb-architecture-and-capability-leverage_2026-10-10.md#RC03) | Carry forward accepted [holistic RC03](holistic-state-management-plan_2026-10-10.md#2-confirmed-rule-changes-and-decision-route),2026-10-10 | Optional narrower coordination requires equivalent delayed-operation ordering, era cuts and recovery exclusion. Batching does not depend on removing the guard. |

SA-RC04 is a plan-authoring decision, not a newly invented source-review RC. Accepted ADRs remain
immutable. Complement ADR-0145 for compatible additional recovery semantics; supersede a conflicting
mechanism if the selected implementation genuinely changes its guarantees. Architectural owner and
runbook changes accompany the decision; acceptance of these choices does not establish implementation.

## 3. Complete migration and durable recovery — F01/F02

### 3.1 A complete transition, including protected legacy states

The native upgrade owner declares the source, supported partial forms, complete transformed forms
and final target invariants for every affected family. Inventory required fields against actual
target declarations, rather than relying on ordinary defaults to backfill existing rows. A bounded
read-only preflight checks source identities, types, relationships and transformability across the
legacy universe before new translation effects. It does not copy that universe into memory.

Complete existing-row assignments can retain the final declarations: assign required new fields
in the same update that establishes their compatible state. In particular, protected legacy retirement
jobs receive explicit zero registration counters, meaning no current-protocol roots were registered;
they retain `legacy_protected`, epoch0 and no authority to execute. Zero is not a claim that their
legacy scope was empty. Extant ambiguous items remain protected by exact holds. Preserve original
outcomes, generations, identities, watermarks and sufficient cleanup origins.

Use a transitional declaration only where the field contract proves a complete update under the
target cannot work; finish final enforcement before publication. Do not broaden write semantics with
`DEFAULT ALWAYS`, turn malformed input into defaults, or infer executable roots from legacy items.
Preflight catches malformed late input before avoidable translation; runtime checks still protect
every effect and final independent verification remains mandatory. Its legacy-data reads are
non-mutating, but their acknowledged progress is durable under §3.2; “read-only” does not mean
restart the whole preflight scan on every retry. Install/check only the migration overlay and
necessary identity machinery before this pass, not the incompatible target transformation.

### 3.2 Separately identified migration progress

Use a small native migration-only progress overlay in each affected database, owned by the existing
upgrade operation. It is not part of canonical/compiler/runtime control schema identity, a portable
content family or ordinary serving state. Its exact declarations and protocol digest are independently
pinned in the immutable upgrade contract and checked against actual definitions before use.
Do not omit a declaration that ordinary runtime behavior needs merely to keep the target hash equal.

This separation preserves completed main's exact target while allowing same-database atomic page
effects and progress for validation. Current declaration/readback accepts required-definition subsets;
selected export emits its declared recovery closure, not arbitrary runtime tables. Migrate every
relevant inventory/definition/recovery consumer so the overlay has explicit treatment. Full privileged
recovery includes it with host evidence; portable selected-content export excludes its authority.

Use one versioned table with one bounded current-progress record per original migration in each
database, not a historical record per page. Progress binds original migration, scope, generation,
active successor execution, source contract, final target, transition/protocol,
checkpoint-schema, preflight-contract and verifier identities. Store current pass/family, stable keyset
position, nested owner/contribution/product position where needed, revision and completion state. A pass's enumeration
universe is fixed or explicitly separated from tables it creates; cleanup insertion cannot silently
change a cursor's meaning. Reuse existing row/byte/effect bounds; expose distinct progress facts through
existing logs/status, without a second host/native task ledger.

Each bounded data page locks and rechecks its exact progress revision and closed migration authority,
applies deterministic effects and references, and advances progress in one transaction. Reconcile a
lost acknowledgement by reading that exact revision/outcome; never grant a new native operation or
blindly repeat an uncertain effect. Preflight, translation, legacy protection, cleanup construction
and independent verification have different completion meanings. Preflight pages atomically check
actual source/recognized partial forms and advance only their own cursor under the closed,
exclusive-effects premise. Bind their fixed enumeration/source-form contract explicitly; declared
compatible transformations preserve that prefix, while relevant contract/source/unrecognized-state
changes invalidate it. Same-candidate retry and compatible successor reconciliation must not rerun
acknowledged valid preflight prefixes. This progress is not a final-validation certificate.
Verifier changes invalidate incompatible verification
progress, not acknowledged compatible transformations. Declaration windows have a separate actual
definition/readiness reconciliation boundary: do not claim unqualified atomic coupling of asynchronous
index construction and a data-page receipt. Closed admission and governed exclusive effects
are prerequisites for reusing verification progress.

Publication atomically checks required completed passes and selected verifier, updates the installation
marker and existing native journal, and seals the overlay. Keep DDL removal out of this transaction.
Persist the host's completed-scope evidence afterward; lost host acknowledgement reconciles native
publication. Retain the sealed overlay while a named upgrade/recovery consumer needs it, then retire
that exact overlay through its owner after durable terminal evidence and explicit release. It is neither
unbounded historical runtime retention nor permission to collect original migration outcome references.

### 3.3 Checked successor for the current mixed state

The host owns predecessor inventory, borrower drainage, daemon restart/termination certainty,
credential reconciliation and maintenance-owner transfer. Database table names or OWNER permissions
do not supply that capability. The immutable successor records its candidate, predecessor chain,
original native migration and credential identities, completed scopes, overlay/protocol and source/target.
Durable successor creation precedes authority transfer; ordinary attachment never substitutes a binary.

Completed main is preserved/skipped only after exact closed target marker, matching published native
journal and durable host checkpoint agree. Do not reopen it or manufacture page receipts. Validation
admits only enumerated pristine/translated/repairable forms with exact source marker, generation and
original journal. An authorized implementation preflight establishes its actual state; authoring's
historical receipt is not sufficient authority to repair it.

Unreceipted partial work requires one bounded reconciliation from the beginning, verifying or repairing
actual pages and atomically establishing **new successor** progress. Matching-looking rows or old log
positions are not predecessor receipts. This reconciliation is itself resumable, so another interruption
does not restart its completed prefix. Reject foreign identity, incompatible protocol/target, unsupported
mixed forms, live predecessors and uncertain unfenced effects. Keep exact-candidate retry and existing
intent-only replacement as distinct routes; do not simply permit every `declarations`/`translated` journal.

## 4. Operation-valid catalog preparation — F03

Loader/installer owns one actual catalog capture and checked desired-declaration plan per database
operation; publisher inspection owns its own compatible capture. Parse/hash desired declarations once,
compare named actual definitions and apply only missing compatible declarations in affected groups.
Same-name meaning conflicts refuse unless an explicit governed transition supplies their replacement;
immutable pinned definitions are never overwritten by ordinary installation. Update or invalidate affected
capture entries after acknowledged DDL; unknown acknowledgement requires targeted actual reconciliation.
Maintenance exclusion supplies validity during installation. Ordinary publication relies on immutable
definition epochs and checked expected meanings, not a stale global catalog cache.

Carry index readiness separately from definition presence. Check all required indexes, including
unchanged indexes encountered on retry, through terminal readiness. Retain actual mismatch readback,
independent backup inventories and cold admission. Installation success does not become a certificate
for a different database, executable epoch, session or future mutable catalog.

## 5. Typed selective access, nomination and hydration — F04/F05/F06

### 5.1 Per-node membership and limited ordered relations

Introduce a bounded native membership result keyed by each requested physical/nominal identity and
exact selected views. Retain requested-node provenance through batched reverse-alias ancestry and
cycles; represent absence and conflicting selected payloads explicitly. An internal “any member found”
boolean cannot stand in for these answers. Use existing equality-prefix indexes and bounded ID batches;
charge operation-local shared-dependency memoization to its actual request/read owner.

For `relation_bodies(relation, limit)`, nominate only relevant relation membership, resolve aliases and
merge deterministic ordered streams across exact views. Apply eligibility and physical/nominal conflict
semantics before global deduplication and the output limit. Per-view limits must not discard a later
qualifying or globally earlier row. A zero/empty request retains required handle/authority checks.
Complete reconciliation and global analytics keep explicitly complete routes. A limited relation request
must not initialize complete unrelated selection preparation as its default.

### 5.2 FULLTEXT nomination with frozen ranking

Query actual indexed search-family **tables**, not RecordId sources, for matching document IDs.
Pinned3.3.0 `@OR@` matching is the first query form to qualify against the declared analyzer; inspect
its exact behavior rather than reconstructing analyzer output with guessed token splitting. Preserve
OR over analyzed terms and zero-IDF matches. Union indexed exact-name/path/option equality occurrence
branches before eligibility/deduplication: current `rank_occurrences` can admit an exact hit at score0
even without a matching scored document. Preserve that raw-query equality behavior when analysis is
empty or punctuation-only; do not silently replace it with an empty result. No positivity filter on
native BM25 determines membership.

Stream bounded nomination windows into indexed occurrences and exact dependency/unit eligibility.
Apply candidate quotas only after selected eligibility; unrelated retained content cannot consume them.
Use publication-frozen statistics and existing tie/order policy for final ranking. Native global scores
and approximate HNSW do not replace those policies. Avoid an arbitrary early global candidate limit
that makes the answer depend on unrelated publications.

Qualify actual composed planner/analyzer behavior. If the selected table query cannot express complete
nomination efficiently, choose a scoped exact derived nomination representation under the existing
materialization owner and its explicit schema transition. Do not restore complete-frontier enumeration
as the normal selective route. Shared final ranking preparation stays charged and cancellation-safe.

### 5.3 Window-level eligibility and payload hydration

For each bounded occurrence window, deduplicate its dependency identities, obtain per-node membership
once, evaluate **all dependencies** for each occurrence, and batch accepted payload reads. Preserve
occurrence-to-document/provenance correspondence, absent late dependencies, equal nominal/different
physical values and alias ancestry. Reuse common unit/dependency answers only within a valid charged
operation. Do not let one selected dependency authorize the entire occurrence or one result authorize
every requested node. Read owners retain session/pins/charges through cancellation and physical terminals.

## 6. Bounded lifecycle sets — F07

History collection nominates a bounded ordered range, then rechecks every candidate and relevant
parent/reference/hold/era guard inside bounded atomic pages. Return exact removed/protected outcomes
with committed checkpoint revision instead of singleton post-delete reads. Cursor movement covers the
examined range, including protected records; later reference release is reconsidered by a fresh qualified
invocation, not a rewind that changes completed work. Preserve original acknowledgement reconciliation
and avoid allocating rich per-candidate effect records to collect rich effect history.

Read retirement root/child incarnations as bounded sets, retain per-item identities and recheck each
inside the guarded page. Migration cleanup builds bounded exact owner sets with sufficient original
attempt/contribution/product evidence; it consumes migration progress from §3. Bound actual destructive
effects and bytes, not just parents. High-degree output nomination and hold deletion remain atomic.
Singleton effects are appropriate only where a concrete conflict/failure contract requires them.

Keep installation ordering initially. This work does not release migration references, invent safe
history horizons, narrow backup protection or allow old issuance to act on a reactivated incarnation.
HS5/HS6 successor and history qualification remain required before destructive collection.

## 7. Capability decisions and bounded investigations

These are design inputs or conditional work within existing owners. Record executed results in
coordinator §9.1 and affected packages, not a second investigation-status register. Static source can
settle fit; actual execution is needed for Tested/Measured claims and uncertain composed behavior.

| Avenue / package owner | Selected route, remaining question and dependent work |
|---|---|
| **Migration planning — SA0/SA2; native upgrade** | Select the small native progress protocol plus existing frozen host candidate/planning. Inspected SurrealKit `1.0.0-beta.6`, revision `fd7b075c7619138cf5b8704d9f1938a63e3c977f`, executes steps and completion marks separately; its lock-loss path does not establish cancellation. Its frozen-file/rollout planning benefits are real but duplicate existing candidate ownership without replacing page atomicity/fencing/index readiness. Reopen adoption for a named multi-rollout/schema-file planning consumer that removes more machinery than it adds. No dependency change now. |
| **Native modeling — SA8; model/native/control** | Compare concrete backlinks/deletion rejection and named maintained aggregates with existing membership/hold/invariant consumers. `REFERENCE ... REJECT` is not assumed to check insertion-time target existence; retain explicit existence/admission checks unless exact evidence establishes replacement. Synchronous views/events add write work; async events cannot establish admission. Adopt only a concrete simplification with complete lifecycle/cold semantics; schema changes use a distinct explicit transition after current recovery. |
| **Search — SA5/SA6; native/serving** | Qualify analyzer punctuation/compound/repeated/empty terms, zero scores, exact branches and composed plans at3.3.0. Compare independent complete small-case results; query fallback is a scoped exact nomination representation, not semantic narrowing. Gates selective lexical release. |
| **Transport — SA8; NE0/UP5/HS9** | Compare a complete progressive WS `query_stream`/`query_cancel` adapter with maintained patched gRPC, including same-session backup, provisional/statement/outer/physical terminals, cancellation/retraction and connection loss. Retain gRPC unless the composed alternative wins; an endpoint edit or buffered stock WS route is insufficient. Does not delay F01–F07. |
| **Cache/lifetime — SA8; NE7/GK/GR/HS8/HS9** | Finish exact absence/member mutation, one live flight under eviction, cancelled waiters, retention expiry, external borrows and shared-budget controls. Narrow snapshot-pin lifetime only after the capture-versus-retirement race and finalizer ownership are settled. No new cache-validity authority. Gates corresponding existing acceptance, not all native corrections. |
| **Engine/service — SA8; UP1/UP7/HS11** | Establish effective installed RocksDB/cache/compaction/snapshot settings and executor width against concurrent compile/serve work. Use native metrics for named questions. Low-cache/embedded upstream reports are leads, not this failure's diagnosis; no inferred memory quota or fixed worker cap. Adopt settings only against exact configuration and preserved durability. |
| **Coordination — SA8; HS5/HS6** | Initial guard remains. Any narrower protocol must preserve era-cut and delayed-effect exclusion under snapshot isolation, restart and backup/recovery. Read-only optimism is not equivalent to a conflicting write. Equivalent interleavings gate narrowing, not batching. |
| **Legacy release — SA8; HS6/UP8** | Inventory named migration consumers and sufficient original provenance; qualify explicit exact-reference release before collection. Conservative references are not automatically leaks. No age/size/resolved-flag deletion. Gates destructive release only. |
| **Notifications — SA8; UP9 consumers** | No new LIVE/changefeed consumer is established. Keep a named-consumer trigger: adopt only when notification/synchronization removes real repeated lookup or supplies required behavior, with gap/reconnect handling. Durable outcomes/manifests retain authority. |

Pinned additional selection inputs include the [explicit OR operator tests](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/language-tests/tests/language/indexes/full_text/matches-operator.surql),
[reference backlink handling](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/core/src/doc/field.rs#L877)
and [deletion rejection](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/core/src/doc/purge.rs#L521).
These are inspected source contracts, not probes executed here.

Capability evidence is pinned by the source review's §5 and fresh focused source inspection, not by
training recall. Refresh relevant APIs through the selected library skill, Context7 and exact source
before implementation. License alone is not a rejection criterion. Routine exact dependency moves use
the repository's normal policy; a selected library never silently assumes host/native authority.

## 8. Execution packages and cross-plan integration

| Package | Working prerequisite | Delivered behavior and revealing acceptance |
|---|---|---|
| **SA0 — decisions and contracts** | Accepted §2; current owner/source evidence | Complementary migration ADR, storage/runbook and overlay treatment; complete field/pass/identity contracts and library selection. Decision metadata before dependent effects. |
| **SA1 — complete transition** | SA0 field/source contract | Complete row assignments and bounded transformability preflight; populated legitimate source, protected legacy scope and malformed late row. |
| **SA2 — atomic native progress** | SA0 checkpoint/overlay contract; working SA1 transformations | Separate preflight/declaration/translation/cleanup/verification progress, atomic page effects and lost-ack reconciliation; interrupt in every pass and nested cleanup enumeration. Compatible transformations preserve qualified preflight prefixes; sealed progress joins publication. |
| **SA3 — host reconciliation successor** | SA0 authority contract; working SA1/SA2 native route | Immutable checked mixed-state successor, predecessor drainage/credential preservation, main skip and resumable validation adoption; old candidate and foreign/malformed states refuse. |
| **SA4 — catalog preparation** | Existing immutable definition/maintenance owner; SA0 overlay inventory treatment for upgrade consumer | One operation-valid capture and changed-group plan, targeted unknown-DDL reconciliation and readiness for existing indexes. Independent publication/cold checks retained. |
| **SA5 — selective membership and relation access** | Existing exact view/alias/index contracts | Typed per-node results and relation-aware ordered limited reads; cross-view aliases, conflicts, empty/rare relation and late qualifying member. No full unrelated startup. |
| **SA6 — selective lexical and occurrence windows** | Working SA5 membership slice, exact occurrence/frozen-ranking contracts | Table-source nomination plus batched all-dependency eligibility/hydration; analyzer/zero-IDF/exact branches, unrelated-publication growth, cancellation and exact rank equivalence. |
| **SA7 — bounded lifecycle crossings** | Existing HS5/HS6 guards/horizons; cleanup consumes working SA2 progress | Window-level guards/effects/outcomes for history, root/child registration and cleanup; protected/eligible interleaving, high degree, incarnation race, budget and lost acknowledgements. |
| **SA8 — bounded capability integration** | Named §7 consumers and existing package contracts | Resolve remaining nine-avenue questions at their owners; conditional adoption or reasoned trigger with dependent acceptance explicit. Known corrections do not await unrelated optional features. |
| **SA9 — coordinated final-source acceptance** | Actual migrated SA1–SA7 consumers and decision-gated SA8 outcomes | Join HS11/UP9 once, including surviving NE9/GK7/GR6/PC6/PJ5/CU6 obligations; independently check current native/publication/search/restore/MCP/Python behavior. |

Dependencies are delivered capabilities, not numeric barriers. SA4 and SA5 can progress once their
consumed contracts exist; SA6 needs SA5's actual per-node result, not migration completion to write
pure logic. Native execution waits for qualified service compatibility. SA2/SA3 share native/host
recovery surfaces, SA5/SA6 share reader/search, and SA7 overlaps control migration: one integrator
owns those common contracts. Preserve small async boundaries, finite synchronous kernels and normal
compiler/test parallelism; do not rebuild vocabulary-sized coroutines to share these operations.

UP1 receives migration/recovery semantics; UP5/HS4/NE2 receive selective readers/search;
UP8/HS5/HS6 receive lifecycle batching; NE0/NE7/GK/GR retain their transport/cache evidence.
These refinements supersede incompatible open access/replay mechanisms, not semantic promises.
HS lifecycle successors and SA schema-upgrade successors are different authority contracts.
BC3 profile adoption, SM8 service-survival qualification and real-library/operator activation remain separate.

## 9. Verification, cutover and completion

Resolve current commands with `just verify --help` and `just verify --print --select FAMILY:BOUNDARY`;
use actual test names and source identities. Relevant boundaries are `tooling:python`, `store:rust`,
`serving:rust`, `compiler:producer`, `compiler:cli` and `serving:mcp`, with pure `model:rust` where
meaning changes. Native controls borrow the installed validation service with logical isolation;
DDL/restart/global lifecycle cases require exact maintenance ownership. No scratch fallback or
per-test schema install is introduced. Verification observes readiness; synchronization needs a guard-free interval.

| Revealing case | Independent distinction required |
|---|---|
| Populated format3, each new required field absent; malformed late row | Valid complete update succeeds, malformed input refuses early, protected legacy work gains no execution authority; actual target invariants checked. |
| Interruption before/after commit and lost response in each pass, including preflight | Effects/progress commit together; same compatible work resumes at acknowledged progress; compatible translation does not restart preflight; relevant source/contract changes invalidate its prefix; changed verifier cannot reuse old verification meaning. |
| Main published, validation partly translated; repeated successor | Main/history remains exact; unreceipted validation is reconciled once under new receipts; nested cursors survive interruption; no fresh native identity or unfenced predecessor. |
| Catalog groups changed/unchanged; interrupted index creation | One valid inventory/plan, actual mismatches and existing-but-unready indexes still detected; wrong database/epoch and unknown DDL invalidate reuse. |
| Multiple views, aliases, conflicts, empty/rare relation | Global ordering/dedup/limit agrees with independent small enumeration; unrelated selected families do not initialize sparse access. |
| Rare/zero-IDF terms, punctuation, compound/empty terms and exact branch | Complete nomination and frozen ranking agree with an independent complete small-case calculation; unrelated retained publications cannot change eligibility, quota or score. |
| Shared dependencies and a late missing dependency | Per-occurrence all-dependency eligibility and payload correspondence remain exact under batched membership/hydration; charges/session survive early cancellation. |
| Protected/eligible history interleaved; high-degree retirement and incarnation race | Per-item guards and effect budget remain exact; atomic checkpoint/protected outcomes survive unknown acknowledgement; original references/horizons remain protected. |
| Publication/backup/cold restore/serve while another view exists | Actual and independent cold admission, pinned definitions, original evidence and terminal/cancellation contracts remain composed on one realization. |

During development use touched compile checks and minimal revealing controls. At functional scope
completion run affected controls/applicable leaves once, then the one coordinated assembled command:
`just verify --qualify --cli` (current supported form, retaining CLI assertions). `just qualify` is its
assembled shortcut without that explicit CLI option; do not silently drop those assertions. Rerun affected
failed boundaries using retained receipts. Existing failures/timeouts remain open until their own
matching-source evidence; tests are not waived, serialized or replaced with cached success.

Implement decision/owner changes before effects; qualify the transition/progress/successor together
before another long installed migration attempt. Rebuild only explicitly reconstructible project
projections under the existing drained maintenance route. Retire superseded scalar/full-frontier/
replay paths with their migrated consumers, keeping required independent global checks. Migration-only
progress has the explicit terminal/consumer lifecycle in §3.2; protected evidence/reference release
requires its own qualified owner. No compatibility-only runtime reader or automatic cleanup is added.

## 10. Source coverage and authoring checkpoint

| Source obligation | Contract/package route; disposition owner |
|---|---|
| F01 / RC01 | §3.1, SA0/SA1/SA9; coordinator SA-F01 |
| F02 / RC01 and supplemental SA-RC04 | §3.2–§3.3, SA0/SA2/SA3/SA9; coordinator SA-F02 |
| F03 | §4, SA4/SA9; coordinator SA-F03 |
| F04 | §5.1, SA5/SA9; coordinator SA-F04 |
| F05 | §5.2, SA6/SA9; coordinator SA-F05 |
| F06 | §5.1/§5.3, SA5/SA6/SA9; coordinator SA-F06 |
| F07 | §6, SA2/SA7/SA9; coordinator SA-F07 |
| Review §10's nine avenues and conditional RC02/RC03 | §7, SA0/SA5–SA8 and existing owner packages; executed outcomes in coordinator §9.1 |
| Review A2/A4/G8 violations and unresolved G5/G6/CI-G2 | §3–§9 corrections and composed independent final-source acceptance; no enclosing closure from authoring |

**Authoring checkpoint,2026-10-10:** target is Proposed; source/API evidence is Interface-checked
within its stated revision, not product Tested or Measured. The
[independent target review](../design_review/reviews/design_review_surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md)
accepts the amended target: its initial F01 found uncheckpointed preflight replay, now corrected in
§3.1/§3.2/SA2 and revealing interruption controls. Original Revise/F01 remains in that review;
coordinator SA-PLAN-F01 records target-only closure. Focused publication verification is recorded
at the coordinator's actual-receipt owner. Native/host execution,
service actions, dependency changes and runtime qualification are **not_run** for this authoring task.
Next implementation work is SA0's decision/owner route and SA1–SA3's complete recoverable migration,
with independent ready access packages permitted on their actual prerequisites.
