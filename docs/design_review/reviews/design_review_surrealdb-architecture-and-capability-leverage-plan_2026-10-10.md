# SurrealDB architecture and capability plan — independent target review

**Design-tier · target-purpose · 2026-10-10**

**Amended-target decision: Accept at Proposed strength.** The plan gives the seven source findings credible corrections within suitable existing foundations. It preserves semantic authority, immutable completed views, original migration identity, independent admission, exact reader pins and terminal certainty while correcting migration recovery, repeated preparation, sparse access and singleton database crossings.

**Initial decision: Revise, for F01 below.** The first reviewed target required complete legacy preflight without explicitly giving that pass durable progress or a reusable validity contract. The author amended the plan during this review. Independent inspection of the amendment resolves that finding for the planning target. This does not establish implementation or runtime closure.

The enclosing implementation still needs the source review’s corrections and composed qualification. This review accepts the amended architectural direction and obligations, not the installed migration, current dirty production code or whole product.

## 1. Scope, baseline and assessment boundary

| Field | Scope |
|---|---|
| Principal subject | [SurrealDB architecture companion](../../plans/surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md), including the reviewed preflight amendment |
| Integration subject | [Persisted coordinator §7.5, §8 and §11](../../plans/persisted-graph-execution-plan_2026-10-07.md), and the architecture additions in holistic-state, unified-persistence and native-efficiency companions |
| Baseline | Dirty `main`, HEAD `125e74f5775e1354ce900cc8d71b190e311dac27`; independently confirmed on2026-10-10 |
| Standard | Core/template3.3, efficient-architecture heuristics1.0, code-intelligence profile1.5 and library-context binding, through [standard.toml](../design_principles/standard.toml) |
| Reviewer | Independent delegated design reviewer |
| Maturity | Proposed target; inspected existing implementation supplies foundation evidence only |
| Method | Read-only inspection of the target, integration additions, architectural owners, decisive current source and selected library-skill references |
| Effects | No writes, tests, probes, database queries, service/configuration changes, installations or commits |

The functional outcome is a pinned API/evidence catalog whose discovery, configuration and evidence answers retain precise support and uncertainty in one realization. The workload includes repeated compilation, concurrent commands/worktrees, retained releases, sparse evidence requests, complete analytics, high-degree ownership and growing control history.

The affected owners are [storage §6](../../design/sections/storage-and-publication.md), [semantic model §15](../../design/sections/semantic-model.md) and [serving §11](../../design/sections/synthesis-and-serving.md), with adjacent native upgrade/control, publisher recovery and host-service consumers.

The historical failed installer and main/validation split are inputs recorded by the coordinator. This review did not refresh those observations or use them as authority to repair installed state. It does not re-audit every provider, behavioral kernel, evaluator or service-security configuration.

## 2. Foundations and governing responsibilities

The foundations are suitable because they already separate consequential meanings and effects. The plan needs targeted operation contracts rather than another canonical store or universal workflow framework.

| Owner | Governing responsibility | Proposed change and consumer boundary |
|---|---|---|
| Model | Semantic records, identities, projections, coverage and recoverable content | Preserve declarations and independent semantic admission; physical batching does not redefine meaning |
| Native upgrade | Complete source-to-target transformation and native publication | Add complete intermediate states and independently identified resumable passes under the original migration |
| Host service | Candidate lineage, drainage, credential reconciliation and authority transfer | Add a checked mixed-state successor; native markers alone cannot authorize replacement |
| Loader/publisher definitions | Actual declaration meaning and installation/readiness | Share operation-valid preparation while retaining independent trust boundaries |
| Native reader/search | Exact selected membership, aliases, nomination and hydration | Return per-node answers and execute selective ordered windows |
| Serving | Eligibility, frozen ranking, evidence association and pinned journeys | Preserve exact answer policy while changing candidate access |
| Native lifecycle | Incarnations, eras, holds, references and guarded effects | Batch physical work without weakening per-item authorization or effect bounds |

Decisive existing source supports these boundaries:

- `lctx-model/src/domain/model.rs:8` retains governing relation invariants, validation, proof and projection metadata.
- `lctx-surrealdb/src/compiler.rs:656` constructs base runtime schema identity from canonical, compiler, control and view declarations.
- `lctx-publisher/src/definitions.rs:157` independently checks required actual definitions.
- `lctx-model/src/domain/recovery_closure.rs:1` separates recoverable completed content from runtime authority.
- `lctx-publisher/src/selected_backup.rs:83` selects comparison metadata and completed content rather than exporting arbitrary operational authority.

Those observations support the proposed migration-overlay separation. They do not establish that adding its table is already safe in every consumer. The plan correctly makes inventory, definition and recovery treatment part of SA0/SA2 and expressly forbids excluding a runtime-required declaration merely to preserve a hash.

### Fact and fidelity boundary

| Family or answer | Authority and fidelity | Identity/coverage preserved by the target |
|---|---|---|
| Provider and normalized facts | Attributed observations and model-owned derivation | Provider/source context, exact views, unresolved/conflicting observations |
| Behavioral enrichment | Stated finite model and proof contracts | Qualified verdicts and original evidence; no database-traversal substitution |
| Search occurrences | Derived representation with exact selected dependencies | Document, unit, window, binding and provenance correspondence |
| Search ranking | Governed publication-frozen policy | Analyzer/statistics identity, zero scores, exact branches and deterministic ties |
| Migration progress | Operational observation under original migration authority | Scope, generation, successor execution, contracts, cursor and revision |
| Served evidence | Same-realization admitted content | Pins extend through resources, references, cancellation and continuations |

Migration progress is not canonical content, a historical receipt manufactured from matching rows, or evidence that final admission succeeded.

## 3. Migration and recovery compose coherently

### Complete transition

Companion §3.1 addresses the actual transition defect rather than relying on defaults or fresh empty databases. It requires complete assignments for existing rows, enumerated partial forms and final invariant verification.

Explicit zero registration counters on protected legacy retirement jobs have a coherent meaning: no current-protocol roots were registered. They neither assert that the old scope was empty nor grant permission to execute ambiguous retirement work. Exact holds and original cleanup provenance remain necessary.

The distinction between a complete update under final declarations and a genuinely necessary transitional declaration avoids unnecessarily broad write semantics. Refusing malformed input remains appropriate; broad `DEFAULT ALWAYS` behavior would change future writes rather than merely repair migration.

### Atomic progress and independent verification

Companion §3.2 separates operation identity, pass progress, declaration readiness and final publication. Each data page checks authority and progress revision, commits effects/references with cursor advancement, and reconciles an uncertain acknowledgement through the same operation.

A bounded current-progress record is proportionate to the real recovery need. Historical per-page records are unnecessary when execution advances sequentially through checked revisions and resolves uncertainty before issuing dependent work. Nested cleanup enumeration and stable universes are explicitly covered.

Asynchronous index construction is correctly outside any blanket atomic-page claim. Definition presence and terminal readiness remain separate facts.

Preflight and final validation serve different purposes. Preflight establishes transformability before avoidable translation; independent final validation establishes the actual transformed state before publication. Their separate predicates and completion meanings justify both passes. The amendment now prevents preflight from becoming an uncheckpointed whole-universe prerequisite on every retry.

### Mixed-state successor

Companion §3.3 correctly keeps host and native authority distinct. The host owns predecessor termination/drainage, candidate and credential identity, and durable authority transfer. The native side owns recognized state, exact original migration identity and atomic progress.

Completed main is preserved only through agreement among exact marker, published native journal and durable host checkpoint. Unreceipted validation work requires actual reconciliation and new successor progress; matching-looking rows do not retroactively become predecessor receipts.

The current `_upgrade_replacement_preflight` in `scripts/surrealdb_service.py:1825` permits absent/intent-only unpublished journals. The proposed successor is therefore a real contract extension, appropriately routed through accepted SA-RC04, rather than a claim that current recovery already supports advanced unpublished states.

## 4. Selective execution preserves the answer contract

Companion §4 gives catalog preparation a valid operation lifetime. It retains mismatch detection, targeted reconciliation after uncertain DDL, and readiness checks for indexes encountered on retry. Immutable publication epochs and separate backup/cold checks prevent preparation reuse from becoming ambient catalog trust.

The limited relation route currently reaches complete selected preparation through `reader.rs:217` and `selection.rs`. Companion §5.1 replaces that physical dependency while preserving exact views, reverse-alias ancestry, conflicts, global ordering and deduplication before the output limit. The required late-qualifying and multi-view cases challenge premature termination.

The proposed per-node membership result is necessary for batched occurrence eligibility. An “any member found” answer cannot authorize all requested nodes. Current `selection.rs:236` uses that boolean only inside a per-requested-node loop; removing the loop requires preserving its semantic correspondence explicitly.

Companion §5.2 also preserves a consequential existing search behavior. `lctx-serving/src/search.rs:177` can admit an exact-name/path/option occurrence at score0 without a matching scored document. Native term nomination alone would lose that branch, particularly with empty or punctuation-only analyzed input. The plan explicitly unions indexed equality branches and retains raw-query equality.

Native FULLTEXT nominates documents; frozen publication statistics remain ranking authority. The plan does not claim that the first proposed `@OR@` query is already qualified. It requires analyzer and composed-planner checks and supplies a scoped exact derived nomination alternative if that query cannot preserve completeness efficiently. This is a credible implementation choice within an explicit semantic contract.

Window-level all-dependency eligibility and accepted-payload hydration preserve occurrence correspondence while reducing crossings. Charges, pins and physical terminals remain attached to the read owner through cancellation.

## 5. Lifecycle and capability choices remain proportionate

Companion §6 bounds actual destructive effects and bytes, retains per-item guards, and returns committed outcomes with checkpoint revision. Advancing over examined protected records avoids repeated blocked-prefix work; later release belongs to a fresh qualified invocation.

High-degree retirement still requires bounded child/hold effects and exact incarnation checks. Migration cleanup still needs original attempt/contribution/product evidence. Batching does not release migration references, invent history horizons or remove the installation guard.

The nine investigation avenues in §7 are sufficiently routed:

- Migration planning selects the small native progress protocol with existing host ownership.
- Native references/maintained aggregates require a concrete simplification and complete lifecycle semantics.
- Search qualification gates the changed lexical consumer.
- Transport remains a complete comparison with backup, cancellation and terminality obligations.
- Cache/lifetime questions stay with existing reuse and resource owners.
- Engine settings require actual installed configuration evidence.
- Narrower coordination requires equivalent era-cut and delayed-effect exclusion.
- Legacy reference release requires named consumers and sufficient retained provenance.
- Notifications require a real consumer and gap/reconnect handling.

These are capability decisions and bounded investigations, not nine mandatory adoptions. Neither a WebSocket switch nor guard removal is required for the established corrections.

## 6. Change scenarios and alternatives

| Scenario | Ownership and credible route |
|---|---|
| Add a fact family — domain extension | Model/codec/closure adapters change; migration metadata does not become another semantic inventory |
| Add an analytic — composition | Declare its actual projection and specialized kernel contract; complete global work remains complete |
| Retain another release — instance | Exact views and frozen ranking preserve earlier meaning; unrelated content cannot consume selected quotas |
| Substitute transport — mechanism | Existing native boundary absorbs the change; full streaming, backup, cancellation and terminal contracts qualify the substitute |
| Retry late migration — failure | Acknowledged compatible preflight/data prefixes resume; unknown effects reconcile under original identity |
| Interrupt mixed-state reconciliation — recovery | New successor progress preserves its completed prefix without fabricating predecessor history |
| Request a rare/empty relation — growth | Relation-aware nomination avoids unrelated selected preparation while preserving ordered completeness |
| Retire a high-degree object — skew | Bounded guarded effects and child nomination preserve authority without scalar crossings |
| Cancel an occurrence window — lifetime | Exact read owner retains charges/session/pins until admitted tails terminate |

Keeping coarse replay and complete-frontier sparse access would preserve familiar code but retain demonstrated amplification. A generic migration framework would add overlapping ownership without discharging native page atomicity and host fencing automatically. Moving all execution into either SurrealQL or Rust would also discard useful specialization.

The selected design instead delegates indexed nomination and bounded sets to the database, retains finite semantic computation and frozen answer policy with their Rust owners, and gives durable recovery only to the supported stateful operation that needs it.

## 7. Stable finding and amended-target follow-up

### <a id="F01"></a>F01 — Mandatory whole-universe preflight initially lacked durable progress and validity

**Initial assessment,2026-10-10: Revise.**

**Owner:** Native upgrade preflight/progress contract, SA0/SA2.

**Principles:** FP-07; DP-08/09/19; A4.

The first reviewed §3.1 required a complete read-only transformability scan before new translation effects. Its §3.2 and SA2 described declaration, translation, cleanup and verification progress without explicitly including preflight.

A late interruption could therefore require another complete legacy scan before reaching an acknowledged translation cursor. Bounded memory did not bound recovery replay. The problem was the pass lifetime and validity contract, not the existence of independent preflight and final validation.

**Required correction:** Include preflight in the existing migration-only progress protocol. Bind its predicate, fixed enumeration/source-form contract and closed exclusive-effects premise. State which compatible transformations preserve an acknowledged prefix and which changes invalidate it. Preserve final independent validation.

**Independent follow-up,2026-10-10:** The amended §3.1/§3.2, SA2 package and interruption acceptance row provide those obligations. Only overlay/identity machinery precedes preflight; its legacy-data reads remain non-mutating. Same-candidate retry and compatible successor reconciliation must preserve acknowledged valid prefixes. Relevant source/contract changes and unrecognized forms invalidate them. Preflight progress explicitly confers no final-validation certificate.

**Follow-up judgment:** Resolved for the Proposed planning target by static inspection. Implementation and runtime behavior remain unqualified.

**Sole disposition:** [Persisted coordinator §8, SA-PLAN-F01](../../plans/persisted-graph-execution-plan_2026-10-07.md#SA-PLAN-F01). The original source review’s F01–F07 remain distinct findings with their SA aliases; this target-only correction closes none of them.

No additional blocking target finding was established in the examined scope.

## 8. Independent judgments and gates

### Architectural judgments

| Judgment | Amended-target verdict | Basis |
|---|---|---|
| **A1 — Localize change** | **Satisfied, Proposed** | Changes belong to upgrade, host, declaration, reader/search and lifecycle owners; shared contracts have one integrator |
| **A2 — Encode domain meaning explicitly** | **Satisfied, Proposed** | Original migration, successor execution, pass completion, runtime identity, membership, ranking and lifecycle authority remain distinct governing concepts |
| **A3 — Extend through composition** | **Satisfied, Proposed** | Existing semantic declarations, native access, finite kernels and recovery owners compose without a universal framework |
| **A4 — Fit execution to workload** | **Satisfied for amended target, Proposed** | Resumable preflight/data passes, operation-valid preparation, selective ordered nomination and bounded lifecycle sets remove identified amplification; initial F01 violated this judgment |

### Correctness and fidelity gates

These judgments concern the specified target. They are not fresh runtime passes.

| Gate | Verdict | Basis and limit |
|---|---|---|
| **G1 — Authority** | Pass, target | One semantic/disposition owner; operational overlay cannot confer runtime or import authority |
| **G2 — Semantic fidelity** | Pass, target | Legacy protection, physical/nominal identity, per-node membership and exact search branches remain explicit |
| **G3 — Validity** | Pass, target | Complete transformations, actual definition checks, final invariants and independent admission have identified enforcement boundaries |
| **G4 — Hidden behavior** | Pass, target | Ordinary attachment remains observational; effects require owned maintenance, not implicit repair or adoption |
| **G5 — Consistency/recovery** | Pass, target | Atomic page progress, original-operation reconciliation, checked successors and publication-last are specified |
| **G6 — Transformation/reuse** | Pass, target | Prefix/preparation validity, exact eligibility, frozen ranking and incompatible-reuse refusal are explicit |
| **G7 — Truthful claims** | Pass | Proposed, interface/source evidence and historical execution remain distinguished |
| **G8 — Library leverage** | Pass, target | Native set/index/FULLTEXT capabilities are considered with complete contract gaps and total integration burden |
| **CI-G1 — Fidelity** | Pass, affected target | Attribution, uncertainty, ranking versus proof, and evidence association remain preserved |
| **CI-G2 — Evidence closure** | Pass, target | Same-realization dependencies, actual-state reconciliation and independent cold admission survive |
| **CI-G3 — Evaluation integrity** | Not applicable to changed mechanisms | Evaluation construction/tuning is unchanged; independent small cases are acceptance controls, not product-evaluation certification |

FP-01–FP-06 are satisfied for the proposed ownership and operation boundaries. FP-07 is satisfied after F01’s amendment. Relevant DP-01–05, DP-07–11, DP-13–24 and CI-01–11/13 have explicit scoped preservation or correction routes. This is not a blanket verdict over unexamined extraction or analytical implementation.

## 9. Evidence, rule impacts and completion boundary

| Check/evidence | Outcome |
|---|---|
| `git rev-parse HEAD` | **passed**,2026-10-10; baseline matched |
| Read-only `rg`/`sed` inspection of named owners, source paths and plan contracts | **passed** for this static review investigation |
| `sed -n '68,153p' docs/plans/surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md`, plus SA2 and interruption acceptance inspection | **passed**,2026-10-10; amended preflight contract inspected |
| Builds, tests, database/planner probes, service actions and runtime qualification | **not_run** |
| Documentation publication checks and root handoff | Outside this delegated assignment; coordinator owns their receipts |

No new rule change is required by F01’s correction. It completes the already accepted durable-progress direction.

The plan correctly carries the existing rule impacts: source RC01 and supplemental SA-RC04 require the migration decision/owner/runbook route; conditional RC02/RC03 retain their accepted transport/coordination routes. These choices establish no implementation. Accepted ADRs remain immutable.

The coordinator owns current disposition and actual receipts. SA9 joins HS11/UP9 once, retaining surviving NE/GK/GR/PC/PJ/CU obligations. Independent access work can proceed on its actual prerequisites, while installed execution requires compatible qualified service state. Optional capability investigations do not delay unrelated established corrections.

**Final bounded decision: Accept the amended planning target at Proposed strength.** Preserve F01 and its initial Revise judgment with the follow-up above. The enclosing implementation still needs revision and composed runtime qualification. The next consequential work belongs to the native-upgrade and host-service owners: establish SA0 contracts/decision metadata, then implement and qualify the complete transition, progress and checked successor together before another installed migration attempt.
