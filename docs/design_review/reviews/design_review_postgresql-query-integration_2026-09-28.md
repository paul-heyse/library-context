# PostgreSQL PG12–PG15 query integration review

## 1. Scope, ownership and change scenarios

**Change / conformance review, 2026-09-28.** Core 3.0, code-intelligence profile 1.1 and
the [repository binding](../design_principles/binding/library-context.md) govern this review.
The subject is the PG12–PG15 working tree above `010aeda`, including ADR-0069, migrations
005–007, importer/repository/retrieval, the asynchronous storage and MCP boundaries, finite
DataFusion federation and the operational report. The provider dependency is the owned
`e6fc4c40ec0ffdb7c89371371b18c5f819cc79a9` revision. This fresh reviewer inspected source and
existing targeted receipts; **no tests were run by this reviewer**. Formatting and broader
gates were in progress independently and are not certified here.

The accepted owners are [storage §§6.4–6.5](../../design/sections/storage-and-publication.md),
[serving §§11.2–11.4](../../design/sections/synthesis-and-serving.md),
the then-current ADR-0069 (now carried by [ADR-0070](../../adr/0070-postgresql-recovery-and-index-admission.md)) and the
[PG12–PG15 execution contract](../../plans/postgresql-integration-plan_2026-09-27.md).
The current finding disposition owner is
[forward-plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).

**Supported scope:** immutable projection import and explicit selection; exact operation,
facet, brief and native answers; exact vector ranks and an optional explicitly qualified HNSW
profile; declared immutable reporting views and coherent capture of mutable operational inputs.
Existing compiler semantics and source evidence are preserved, not reanalysed by PostgreSQL.
The adjacent evaluation scripts now query the actual repository while retaining offline
canonical inputs as references. PG16 production migration/rollover/recovery and PG17 assembled
acceptance are excluded. No live ANN profile is qualified by this review or by fake-vector
fixture results. The operator database still has only migrations 001–004.

### Responsibilities and semantic authority

| Owner | Contract and hidden decisions | Dependency direction and realistic change |
|---|---|---|
| `cpg-schema::serving_projection`, `bundle`, `postgres_report` | Authoritative schemas, context, identities, relation/artifact receipts, finite reporting inventory | New semantic content first changes its versioned declaration; codecs and consumers follow it |
| `lctx-postgres::import`, migration-owned capabilities | Frozen transport recipe, bounded COPY, resumable attempts, read-back validation and atomic readiness | Depends on shared contracts; does not publish canonical Delta snapshots or select readers implicitly |
| `repository` / `hydration` | One owner for exact selection, aliases/singletons, match/no/open, request-bound cursors and complete selected support | SQLx mechanics remain inside the effect owner; Python receives coarse records |
| `retrieval`, `profiles`, `qualification` | Exact all-eligible per-view ranks, bounded ANN candidates/rescoring, immutable qualification and routing metadata | Consumes exact repository eligibility; policy/profile changes do not alter generation identity |
| `lctx_storage`, pure `lctx_semantics`, MCP lifespan | Explicit async pool lifetime, pinned generation, pure native kernel, bounded worker slots and wall-clock request deadline | Transport/rendering does not reinterpret conditions or implement relational selection |
| `lctx_mcp.retrieval` | Existing lexical policy, compact RRF and exact-symbol promotion | Consumes IDs/ranks; owns no dense vector matrix or relational store |
| `cpg-core::postgres_read`, owned provider, `report` | Finite pushdown, declared Arrow schemas, cancellation ownership, mutable capture and pinned Delta composition | Provider details do not enter application SQL or semantic execution; general SQL equivalence is unsupported |

**Implemented.** Projection FORMAT 2 binds `ServingContext`; bundle FORMAT 12 derives its outer
context from it. Content, transport, query profile and selection have distinct identities and
effects. A `PinnedGeneration` cannot be constructed by external consumers. Readiness requires
stored-row validation after the write barrier, and serving RLS hides unready generations.
Native/artifact admission rechecks bytes rather than trusting a locator. Ready data is retained.

### Fact and fidelity trace

| Family | Provider/revision and fidelity | Coverage, identity and consumer |
|---|---|---|
| Operations, facets, fates, conditions, discharges | Canonical compiler snapshot and projection definition; materialized derived facts, unchanged model/verdict meaning | Generation-qualified original IDs, explicit unknown totals and reasons; exact tools/native kernel |
| Brief assertions, findings, witnesses, members, attributes, evidence | Canonical validated synthesis; evidence status and model/invocation fields survive hydration | All selected support is fetched by generation-qualified keys; parallel support rows retain order/multiplicity |
| Vector ranks | Pinned 1024 spec and generation, registered PostgreSQL cosine policy; relevance only | Exact enumerates the full eligible entity/view universe; HNSW is labelled approximate and cannot define exhaustive facet results |
| Operational events, import/profile attempts and selections | Mutable PostgreSQL observations captured at one repeatable-read snapshot | Capture timestamp/MVCC token and scope are reported; attempts scoped by store/compiler can include other libraries and say so |

### Composition, extension and failure scenarios

| Trigger | Owner and propagation | Contract/evidence assessment |
|---|---|---|
| Publish another release while a reader is live | Bundle/schema identity → importer → explicit selection; existing reader retains its pin | **Tested in supplied receipts:** two-generation selection does not mutate an old reader; incomplete data cannot appear ready |
| Interrupted COPY or conflicting retry | Importer owns generation lock and atomic rows/receipt transaction; source transport is frozen separately from content | **Tested:** cancellation/resume, concurrent retries, receipt conflict, missing stored content and cleanup refusal. No new compiler or Python lifecycle rule is required |
| Add a served semantic field | Versioned Arrow declaration and compiler projection → shared codec/validator → Rust hydration and Pydantic rendering | **Interface-checked:** representation changes require an explicit migration; rendering legitimately changes, while selection and evidence semantics stay with Rust. A new semantic category is not an ordinary row addition |
| Add a reporting view or predicate | Finite schema/view owner and admitted expression policy → provider conformance | **Implemented:** unsupported expressions remain local; scan and whole-subtree admission share the same algebra, and physical execution is sealed against later predicate bypass. LEFT joins remain local after the review control described below |
| Compare ANN with exact under selective filters | Frozen evaluation request → canonical f64 reference → serving-role exact/ANN queries → immutable profile qualification | **Implemented/Tested for the mechanism:** per-view and fused recall, all required strata, numeric/rank agreement, custom/generic index plans and declared latency must pass; installation alone cannot activate ANN |
| Cancel CPU or database work repeatedly | Native worker owner, SQLx lease owner and provider pool owner | **Tested:** native cancellation retains its slot; provider cancellation drains before restoring capacity; corrected SQLx leases retain capacity until server work drains within its existing statement deadline. See F01 |
| Run retrieval/behavior evaluation | `postgres_session` composes actual repository/native lifetimes; `projection_reference` supplies offline reference facts | **Implemented:** ranking and structured evaluation exercise the new service; references have no route into the compiler or online server |

The projection stage preserves the canonical relation universe and validates full closure before
publication. Exact selection applies typed predicates over that universe and distinguishes
unknown from absence. Retrieval groups every eligible chunk to its entity/view before ordering;
HNSW changes candidate discovery only, then rescores each candidate across every view. Hydration
selects records after ranking and refuses the request at its byte limits rather than returning
apparently complete truncated evidence. No new graph analysis or synthesis algorithm is introduced.

## 6. Independent correctness and fidelity judgments

| Gate | Verdict and evidence |
|---|---|
| G1 Authority | **Pass:** Delta retains canonical authority; schemas/receipts, transport, profiles, attempts and selection are separate concepts. Removed Python relational maps no longer compete with the Rust repository |
| G2 Semantic fidelity | **Pass:** explicit ID widths/metadata and empty schemas; full generation identity and five verdicts; null/unknown distinctions and unsupported conversion refusal |
| G3 Validity | **Pass for inspected publication/query paths:** shared source and stored-row validation, typed pin construction, bounded IPC/response admission and explicit errors |
| G4 Hidden behavior | **Pass:** import/index/qualification/selection are explicit effects; reads do not migrate, select another backend or write canonical facts |
| G5 Consistency and recovery | **Pass in this slice after F01 correction:** publication, native slots, provider cancellation and SQLx drain retain actual work ownership; the one-slot control challenges replacement admission before the server deadline |
| G6 Transformation and reuse | **Pass within finite admission:** pinned generation/spec/profile, exact per-view ranks, explicit approximate route, conservative unsupported residuals and sealed physical scans. LEFT joins are excluded from remote admission |
| G7 Truthful capability claims | **Pass at the stated scope:** targeted receipts are attributed below; live ANN quality, production migration and assembled acceptance remain separate |
| G8 Library leverage | **Pass:** SQLx transactions/pools, pgpq COPY encoding, pgvector search, DataFusion/federation planning and Arrow codecs provide the generic mechanisms; the bespoke code governs application contracts |
| CI-G1 Fidelity | **Pass for serving translation:** verdict/model/coverage and heuristic-vs-fact distinctions survive. This does not settle open Stage 3 semantic findings |
| CI-G2 Evidence closure | **Pass for projection/serving:** complete support validation and generation-qualified hydration; captured operation/brief parity challenges the replacement |
| CI-G3 Evaluation integrity | **Pass:** independent f64 cosine/reference data stays in evaluation; exact repository eligibility and existing lexical policy are intentionally shared when measuring ANN rather than re-evaluating the compiler |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — SQLx cancellation must retain capacity until server work is settled

**Owner:** `lctx-postgres::serving::QueryLease`. **Principles:** FP-05/FP-06, DP-19/DP-20,
G5. **Evidence strength: Interface-checked, 2026-09-28.** At initial inspection,
`QueryLease::drop` called `PoolConnection::close_on_drop`. Pinned SQLx 0.9's PostgreSQL
`Connection::close` sends `Terminate` and shuts the socket; it does not acknowledge that an
executing server statement has stopped before the pool releases capacity. The existing Python
cancellation control waits up to 2.5 seconds for a lock-blocked query whose lock timeout is
2 seconds. That can establish eventual cleanup without establishing the stronger concurrent
server-work bound. The owned provider separately implements cancellation plus a confirming
round trip, retaining or quarantining its capacity.

**Consequence to settle:** repeated cancellation of long SQLx statements may admit replacement
queries while previous server work remains active, exceeding the declared role/pool budget.
Merely preventing an uncertain connection from being reused is insufficient to establish this
resource claim. Application requests now rely extensively on this owner, so the concern is in scope.

**Correction inspected, Implemented/Tested, 2026-09-28:** `LeaseConnection` now retains the
actual SQLx `PoolConnection` in an owned drain task after caller cancellation. `ping` consumes
pending protocol work; a pending database error is followed by another synchronization. Only
after successful drain is that uncertain session discarded. A 305-second outer drain limit
exceeds the largest admitted statement timeout of 300 seconds; a failed/uncertain drain closes
the pool before its capacity can admit replacement work. Pinned SQLx `Pool::close` marks the
pool closed synchronously, so the failure policy does not depend on finishing its awaited close
while holding a lease. No new driver/control pool was introduced.

`test_cancelled_server_work_retains_one_slot_until_statement_deadline` deliberately executes
`pg_sleep(5)` through the real storage wheel with one pool slot and a two-second statement
timeout. After caller cancellation, a second request remains pending and the server has one
active request; the session is then safely discarded and ordinary queries work again.
`uv run pytest tests/scripts/test_postgres_serving.py` **passed**, two cases in
`/tmp/lctx-pg13-lease-functional.log`. The actual interrupted-import test also **passed** with
the repaired lease (`/tmp/lctx-pg12-interruption-lease.log`). These receipts close the resource
ownership uncertainty at the inspected scope. Cancellation returns promptly to the caller;
it does not promise immediate interruption of SQLx server work. That work retains its existing
slot and server deadline. The failure-to-drain path was interface-checked, not fault-injected.

Current disposition belongs only in
[forward-plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).

**Review-time scope correction, not an additional finding:** the initial federation algebra
admitted LEFT self-joins despite alias/filter rewrites whose null-preservation had not been
qualified. The implementation owner restricted remote joins to declared-key INNER joins and
added a filtered-right LEFT JOIN local/remote parity control, which **passed** with the final
immutable provider pin (`/tmp/lctx-pg15-provider-functional15.log`). This is the simpler supported
surface for the current report; reconsider remote outer joins only for a named consumer with
null-preservation controls.

FP-01–FP-06 are **satisfied** for these scenarios: infrastructure, semantic interpretation,
selection, ranking and rendering have coherent owners; shared contracts bind derived
representations and corrected cancellation retains actual resource ownership. DP-01–DP-11 and
DP-13–DP-24 have the inspected mechanisms above, including the F01 correction. Recursion/fixed-point
semantics (DP-12) are unchanged inside the pure native kernel. CI-01–CI-06 and CI-08–CI-13
are satisfied for the translation/serving scope; CI-07 introduces no new analysis routing.

## 8. Library fit, alternatives and verification

**Interface-checked.** SQLx plus pgpq keeps transactional application effects and Arrow ingestion
in their existing owner. Replacing them with a second application driver/ORM would add another
pool, migration or mapping boundary without a selected consumer. The owned Rust-Postgres
provider is justified by the concrete DataFusion report; its declared schemas, stream ownership,
conversion metrics and cancellation contract matter more than broad catalog inference.
DataFusion's optimizer, SQL unparser and federation planner remain the mechanisms; application
code admits a deliberately small semantic surface and repairs inspected unparser limitations.
The simpler viable alternative is local DataFusion residual evaluation, which remains selected
for unqualified functions/casts/joins. ADBC remains a measured-transport alternative, not a
second default stack. pgvector supplies cosine/HNSW; the surrounding exact/ANN distinction,
qualification and generation lifecycle are application policy, not duplicate indexing code.

The independent RRF calculation in the qualification runner is evaluation-only and intentionally
challenges the online Python fusion. It is not an alternative online query owner. The prior file
query engine and dense Python cosine path have been removed; canonical bundles remain portable
reference/recovery inputs. This limits maintenance while retaining non-circular checks.

### Attributed verification, 2026-09-28

These are implementation-owner runs whose logs and corresponding test bodies were inspected.
They are **Tested** evidence for their named cases, not fresh reviewer executions or PG17 acceptance.
All PostgreSQL controls use actual disposable PG18 databases. Early provider runs used the local
patch source corresponding to the later immutable owned pin.

| Command/test | Inspected outcome and scope |
|---|---|
| `cargo test --release -p lctx-postgres --test serving production_import_is_atomic_repeatable_and_selection_is_explicit -- --ignored` | **passed**, `/tmp/lctx-pg12-production-functional2.log`: repeated import, explicit selection, second generation, ready retention |
| Same target, `interrupted_import_resumes_without_partial_visibility` | **passed**, `/tmp/lctx-pg12-interruption.log`: interrupted actual import, invisibility, resume/concurrent retry |
| Same target, `publication_refuses_conflicts_and_incomplete_stored_content` | **passed**, `/tmp/lctx-pg12-refusal-functional3.log`: conflicting receipt, missing rows, cleanup ownership/refusal |
| Same target, `captured_reference_parity` with the frozen reference input | **passed**, `/tmp/lctx-pg13-parity2.log`: 6,302 operations, 20 briefs and registered facet cases; replacement hydration/selection |
| `cargo test --release -p cpg-core --test postgres admitted_provider -- --ignored` with `LCTX_TEST_PROJECTION` | **passed**, `/tmp/lctx-pg15-provider-functional15.log`, final immutable provider pin: schema/metadata, aliases, admitted joins, local outer join/residuals, late physical predicate, actual provider cancellation and coherent capture |
| Same target, `pg_published_bundle_replays_every_byte_after_database_stops` | **passed**, `/tmp/lctx-pg15-report-functional2.log`: loading/ready report identities and diagnostics plus PostgreSQL-offline canonical replay |
| `uv run pytest python/lctx_mcp/tests/test_lifecycle.py` | **passed**, `/tmp/lctx-pg13-lifecycle.log`: cancelled native job retains capacity; whole-request deadline covers several short phases |
| `uv run pytest tests/scripts/test_postgres_serving.py` | **passed**, `/tmp/lctx-pg13-lease-functional.log`: real async wheel lifecycle and one-slot cancellation/drain/reuse; F01 closure |
| `uv run pytest python/lctx_mcp/tests/test_qualification.py` | **passed**, `/tmp/lctx-pg14-qualification-functional2.log`: independent exact reference passes, small-fixture ANN plan qualification fails, profile remains unavailable/exact remains default |

Uncertainty is intentionally bounded: no representative live ANN recall/latency result, production
rollover/recovery, aggregate end-to-end memory/WAL measurement, or assembled code gate is claimed
by this review. These are PG16/PG17 owners. The settled 1024 dimension choice is not reopened.

## 12. Architectural judgment and decision

| Judgment | Verdict and scope |
|---|---|
| A1 Localize change | **Satisfied:** release publication, exact query behavior, ranking policy and reporting extensions have identifiable owners; pure native behavior and offline references remain independently exercisable |
| A2 Encode meaning structurally | **Satisfied after F01 correction:** identity/readiness/profile/fidelity are explicit; native, SQLx and provider cancellation retain actual work ownership rather than only caller-future ownership |
| A3 Extend through composition | **Satisfied:** import/read/qualify/select are separate capabilities; report composes bounded mutable capture with immutable PG/Delta inputs; Python composes coarse repository results with existing lexical/native behavior |

**Bounded change decision: Accept scoped after F01 correction.** No competing semantic authority
or unsupported live ANN activation was found. The source and actual one-slot control resolve the
SQLx cancellation-capacity question. Acceptance covers PG12–PG15 and the finite admitted query
surface. Remote outer joins, other unqualified SQL constructs, production rollover/recovery and
live ANN/assembled qualification remain excluded; a named consumer or the PG16/PG17 steps are
their revisit triggers. Complete the independently running code/documentation gates and record
their results in the execution plan; this review does not substitute for those checks.

**Enclosing architecture:** not certified by this bounded review. Canonical/serving separation
and exact-versus-approximate ownership conform to ADR-0067/0068/0069 at the inspected strength;
production deployment/recovery and assembled expanded-system acceptance remain PG16/PG17,
and unrelated Stage 3 findings remain with the forward plan.
