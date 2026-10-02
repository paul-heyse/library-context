# PostgreSQL expanded architecture — design review

Retired source citations below are historical paths within this review's recorded baseline
and dated inspection scope, including any working-tree limitations. They do not point to
replacement owners. Recover committed source through [Git history](../../README.md#historical-recovery);
the findings and their original evidence strength remain unchanged.

**2026-09-27 · Design / target · Decision: Revise the existing target for expanded PostgreSQL use.**

**Recommendation — Proposed:** make PostgreSQL the transactional and online serving store,
including normalized, immutable serving generations and `pgvector` retrieval. Keep Arrow/DataFusion
and Delta as the authoritative analysis, validation and published-evidence path. Adopt the compatible
DataFusion PostgreSQL reader and federation stack through a qualified, pinned fork; use `pgpq` with
the existing SQLx connection for bulk ingestion. This is a substantial expansion beyond the deployed
cache and operational ledger, without replacing the compiler's semantic engine or canonical store.

**Operator decision:** 1024 dimensions is the standard for all new production embeddings, including
retrieval and compile-time analytics. The operator has accepted the official benchmark evidence;
this review does **not** impose another dimension-fidelity or analytics-quality gate. Migration must
still correctly identify the new spec, request and normalize its vectors, rebuild dependent outputs,
and reject incompatible generations. Historical 4096-dimensional generations keep their identity.

The supplied revisions remove the earlier dependency-family objection. They do not all perform the
same job: `datafusion-postgres` serves DataFusion through the PostgreSQL protocol;
`adbc-driver-datafusion` serves DataFusion through ADBC. The PostgreSQL table providers read an actual
PostgreSQL database. Choose them by that direction, rather than installing the entire catalog.

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Subject | `219070e` baseline; existing PostgreSQL plan and storage/serving owners, embedding clients and launcher, MCP generation/hydration/query paths, native IPC input, Stage 4 authoring and the supplied bridge revisions |
| Standard | [Core 3.0](../design_principles/core/design-principles.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md), [repository binding](../design_principles/binding/library-context.md) |
| Reviewer | Codex, with fresh bounded reviews of embedding/pgvector and PostgreSQL/Arrow bridges; root independently inspected the decisive interfaces and consumers |
| Outcome sought | A concrete replacement target for the next revision of the PostgreSQL implementation plan; no deployment, dependency, production-code, ADR or plan mutation in this review |
| Baseline | PG0–PG7 are implemented and previously qualified. SQLx owns exact cache admission, attempt history and derived snapshot discovery. File generations own serving; Delta snapshots own published facts and consumed-vector receipts |
| Included | Embeddings, exact cache, operations/facets/coverage, brief assertions and complete support, native conditions/summaries, ranked retrieval, serving publication, operational federation, review events and future concept projections |
| Excluded | Finishing Stage 3 reasoning, changing analyzer semantics, another embedding-quality investigation, replacing canonical Delta, opening SQL/ADBC network services, distributed deployment and a new generic storage-provider framework |
| Evidence boundary | Source inspection, isolated dependency resolution/builds and bounded real-PG bridge probes. Existing PG acceptance is historical evidence, not a new run. No expanded product integration or performance qualification is claimed |
| Preserved work | Concurrent Stage 3 discharge review and the supplied external library assessment remain unchanged |

Current owners are [storage/publication §6](../../design/sections/storage-and-publication.md),
[synthesis/serving §§10–11](../../design/sections/synthesis-and-serving.md), the
[PostgreSQL plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream), and the
[forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md).
The [evidence index](../evidence/2026-09-27_postgresql-expansion/README.md) distinguishes each
tested boundary from unresolved integration work. All observations below are dated 2026-09-27.

## 2. Responsibilities, dependencies and semantic ownership

**Implemented baseline / Proposed replacement boundaries:**

| Owner | Responsibility and contract | Dependencies and expected change |
|---|---|---|
| `cpg-schema` and existing semantic kernels | Canonical Arrow facts, codebooks, identity, coverage, five verdicts and evidence relationships | No database client. A new fact changes its owning declaration and explicit projections, not a PostgreSQL-discovered schema |
| `cpg-core`, DataFusion, Delta | Compile, validate and publish one canonical snapshot with exact consumed vectors | PostgreSQL cache remains an optimization. Published evidence remains replayable without PostgreSQL or the embedder |
| Existing Rust PostgreSQL services | SQLx migrations, exact cache, attempts/events and operational state | Add capabilities to coherent modules; do not introduce a second application transaction family |
| Rust serving projection writer | Derive typed relations and artifact references from an already published snapshot; validate, index and mark one projection ready | `pgpq` encodes Arrow batches into SQLx COPY. Compiler and serving-schema identities remain separate |
| Rust serving repository | Exact selection, generation-qualified hydration and vector candidate retrieval; typed requests/results | SQLx and pgvector. It owns SQL details and completeness queries, not behavioral evaluation or transport rendering |
| Python FastMCP application | MCP request/response validation, transport lifecycle and existing lexical/rank composition | A coarse async Rust boundary supplies batches/results. Remove whole-corpus Python relational maps after cutover; retain bounded lexical state until its own replacement is justified |
| Existing native semantic executor | Evaluate conditions and pinned summaries from a validated immutable native image | Remains pure Rust with Arrow input. Do not make every evaluation reach PostgreSQL or embed SQLx in the kernel |
| DataFusion PostgreSQL read adapter | Explicit, read-only PG views joined with pinned Delta relations | Qualified table provider/federation; declared schema conversion and read view. Not an application write repository |
| Future review workflow | Append decisions against an exact subject revision; freeze a selected event revision into compiler inputs | PostgreSQL is authoritative for authored events, not for retroactively changing published facts |

**Proposed flow:** published Delta snapshot → validated serving projection → PostgreSQL ready
generation → generation-pinned Rust queries → existing MCP result contracts. The native image and
lexical corpus are named by the same manifest and loaded once. A selected review-event revision flows
in the other direction as an explicit compiler input, never as an ambient join against “latest.”

| Fact family | Provider/revision and fidelity | Coverage, identity and downstream preservation |
|---|---|---|
| Source, type and flow observations | Existing pinned extractors; their recorded extracted/resolved fidelity | Preserve source/run/context/snapshot IDs, unresolved targets and coverage rows; PG projection does not upgrade confidence |
| Operation facets and behaviors | Compiler-derived five-verdict facts | Carry facet status, model, condition, reason and evidence. Missing row plus incomplete coverage is unknown |
| Findings, witnesses, assertions | Existing analytic/programmatic synthesis lineage | Keep relation kinds, parallel witnesses, all supporters and source spans; ranking never becomes support |
| Consumed embeddings | Exact float32 receipts under a complete spec identity | New spec is 1024. Cache bytes and receipt digests stay exact; pgvector is a derived searchable representation |
| Search rankings | Lexical/cosine/RRF policy and later explicitly approximate candidate policy | Heuristic discovery, not behavioral truth or exhaustive coverage. Record profile and generation |
| Stage 4 concepts | Authored registry/definition and derived memberships under the forward plan | Keep registry source and definition digest authoritative; PG indexes labels/facets/memberships with verdict and witness |
| Operator review events | Authored event stream with exact subject revision and provenance | “Unreviewed” remains explicit; an absent event is not approval. Frozen revision, rather than mutable current status, enters publication |

## 3. Contracts, constraints and testing boundaries

**Proposed contracts**, grounded in the implemented owners:

| Contract | Invariants and enforcement | Lifecycle, failure and verification boundary |
|---|---|---|
| Embedding spec and client | One 1024 spec records MRL prefix selection, normalization order and launch admission; both clients send matching dimensions; reject wrong width, nonfinite/nonunit values and wrong model | New hash invalidates reuse; regenerate analysis and serving outputs. Verify request/response and receipt correctness, not 4096-versus-1024 task quality |
| Exact cache | Existing `(spec, input)` identity, canonical winner and exact byte codec | Keep `bytea` receipts/cache representation. A similarity index never answers cache membership or resolves concurrent winners |
| PG serving manifest | Canonical snapshot/content identity, projection format/digest, compiler/catalog/kernel/spec identities, row counts and immutable native/lexical artifact digests | All keys and foreign keys include the serving generation. Same canonical snapshot can have multiple versioned projection formats without rewriting its identity |
| Ready generation | Loading → validating/indexing → ready; serving role can read only ready projections | Partial loads remain invisible. Retried imports compare content; conflicting same-identity data fails. A final short transaction publishes readiness and an optional selection pointer |
| Exact operation selection | Complete allowed universe; per-term match/no/open; exact totals, unknown totals and stable pagination | SQL must preserve [`find_operations`](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/python/lctx_mcp/src/lctx_mcp/operations.py) semantics. A missing facet cannot become false through an inner join or `NOT EXISTS` alone |
| Hydration | Selected entity IDs resolve deterministic full evidence closure within the pinned generation | Fetch sets in bounded batches. A missing required supporter is corruption/error, not a reduced answer or another semantic search |
| Native evaluation | Existing bounded Arrow IPC format, actual schema metadata and fixed-width IDs | Native image digest is checked before construction. Backend conversion cannot relax its structural or budget checks |
| PG→Arrow view | Declared physical-to-domain mapping, schema/metadata reconstruction, value validation and explicit predicate semantics | Empty outputs retain the declared schema. Unknown types and invalid IDs fail; never drop unsupported columns or rows silently |
| Query read view | Immutable generation predicates on every relation, or an explicitly frozen mutable-data view | A DataFusion session/pool is not a shared PostgreSQL transaction. Handle resource limits, cancellation and retention explicitly |

**Interface-checked:** the current Rust request struct omits dimensions and
[`embed_serve.py`](../../../scripts/embed_serve.py) provides no Matryoshka admission override.
The pinned model configuration lacks the metadata vLLM checks. In pinned vLLM 0.30.0, the pooler
slices before normalization; dimension requests require Matryoshka admission. The launcher must
derive an explicit override from the versioned spec and both clients must request 1024, rather than
merely changing a JSON width. See the [pinned pooler](https://github.com/vllm-project/vllm/blob/v0.30.0/vllm/model_executor/layers/pooler/seqwise/heads.py)
and [embedding evidence](../evidence/2026-09-27_postgresql-expansion/embedding/README.md).

**Proposed lifecycle detail:** load relations and build ordinary indexes while the new projection
is unpublished, validate cross-table closure and artifact availability, then publish readiness.
This avoids requiring concurrent index creation for initial imports. Runtime servers pin the selected
generation once; a later pointer update affects new servers only. Initially retain all ready
generations, matching the current conservative retention policy. Before enabling deletion, add
explicit protection for running readers and referenced artifacts; do not infer safety from a stale
heartbeat. PostgreSQL restore can rebuild projections from canonical snapshots, and operational
events still require their own protected backup.

## 4. Composition and execution

### 4.1 Serving without whole-corpus relational hydration

**Implemented:** [`generation.load`](../../../python/lctx_mcp/src/lctx_mcp/generation.py) validates
the entire file generation, constructs dense vector matrices and materializes tables as Python
dictionaries. [`server.py`](../../../python/lctx_mcp/src/lctx_mcp/server.py) builds additional
support/evidence indexes. Replacing only the cosine calculation would leave these costs and their
representation coupling intact.

**Proposed:** separate a small immutable generation handle, native prepared image and lexical
state from the relational serving repository. PostgreSQL stores operations, paths, facets/status,
parameters, brief/assertion/support relations and searchable vectors. Existing portable bundles
remain the export/recovery/reference format; native files remain content-addressed immutable
artifacts referenced by the PG manifest. PostgreSQL readiness covers their availability and hashes.
The database is not a new blob store for every native IPC file.

Use a Rust serving module/crate only to isolate database effects from the pure native executor.
Expose coarse operations such as resolve operation, select operation page, hydrate selected briefs,
enumerate exact vector ranks and retrieve approximate vector candidates. Python retains MCP models
and its lexical/fusion owner initially.
Do not expose arbitrary SQL or one asynchronous crossing per witness. Transfer exact selection and
hydration semantics to one repository owner, then delete the production dictionary-query path;
keep independent known-answer fixtures, not two permanent production implementations.

**Interface-checked / Proposed integration:** `pyo3-async-runtimes` 0.29.0 matches the existing
PyO3 0.29 family and converts Tokio futures into Python awaitables. Configure one process runtime;
create and close the SQLx pool with the application lifespan. Python cancellation cancels the Rust
future, but server-query cancellation and connection reuse still need a targeted integration check.
Do not create a runtime per request or claim the module's process-global runtime is destroyed at
each application shutdown. [Pinned manifest](https://github.com/PyO3/pyo3-async-runtimes/blob/v0.29.0/Cargo.toml),
[Tokio interface](https://github.com/PyO3/pyo3-async-runtimes/blob/v0.29.0/src/tokio.rs).

### 4.2 Query and analysis record

| Question / projection | Method, universe and settings | Accuracy/model and budgets | Output and evidence linkage |
|---|---|---|---|
| Which operations meet all requested facets? | Full generation-qualified kind/path universe; compose match/no/open per facet; stable entity order and request-bound cursor | Exact under materialized facet coverage; bounded page and separately counted unknowns. No ANN candidate restriction | Existing `OperationSet`, completeness and unknown reasons; same snapshot and evidence IDs |
| What does this selected operation/brief claim? | Exact identity joins through every required supporter, witness and span | Deterministic complete hydration within declared response/native budgets; explicit refusal on overflow/corruption | Existing structured answer and native semantic evaluation; no score used as evidence |
| Which operations/briefs are relevant? | Existing lexical policy, each operation vector view, best chunk per entity, RRF K=60 and exact-symbol promotion | Ranked discovery. Exact vector mode first; optional HNSW profile explicitly approximate with bounded candidate depth and underfill behavior | Ranking/profile metadata plus deterministic hydration from the same generation |
| What do embedding analytics infer? | Existing declared E0/kNN/centroid/community projections, rebuilt using standard 1024 receipts | Preserve each existing exact-under-model or heuristic label and frozen parameters; ANN does not replace compiler kNN | Findings and invocations retain method/model/spec/projection/evidence lineage |
| How do PG operational records relate to canonical runs? | DataFusion joins declared PG views with Delta tables pinned at the recorded versions/snapshot | Immutable records can be generation/attempt qualified; coherent mutable reports require a frozen read view | Diagnostic/operational output with explicit scope; never silent compiler input |
| Which authored concept/review revision informed a result? | Freeze registry/definition/event revision before derivation; index resulting memberships and witnesses | Definition and condition kernels retain ownership; unknown membership remains unknown | Concept/review provenance linked to canonical publication, not mutable status at answer time |

### 4.3 Retrieval contract

**Proposed:** use `vector(1024)` float32 and cosine distance; make pgvector the selected target
instead of the previous future LanceDB path. Exact mode is the reference and appropriate small-corpus
default. HNSW is the selected scalable option once its independent ANN behavior is qualified;
MRL adoption is already settled. Do not introduce half precision or binary quantization now.

**Interface-checked:** pgvector 0.8.6 supports exact search and HNSW/IVFFlat; 1024 fits its
2000-dimensional `vector` index limit. Filtered ANN can return too few rows, and iterative scans
remain resource-bounded. Generation isolation therefore belongs in index layout and query planning,
not only in post-filtering. Prefer generation-scoped partitions/indexes for retained serving sets;
confirm the actual plan, including prepared queries. [Pinned pgvector documentation](https://github.com/pgvector/pgvector/blob/v0.8.6/README.md).

**Implemented baseline / Proposed preservation:** current code ranks all eligible entities in each
vector leg, then fuses ranks. A chunk-level `LIMIT k` can omit distinct entities or an entity that
would win after fusion. Exact reranking of those candidates does not restore the omitted universe.
An ANN profile must declare candidate depth per view, best-chunk deduplication, filter handling,
bounded widening/exact fallback or explicit underfill, stable tie handling and score provenance.
Adding an index must not silently change an “exact” query into ANN. Test the execution plan and
use an explicit exact route when required.

The exact-rank operation streams entity/rank pairs per view after complete eligible-entity scoring
and best-chunk aggregation. With Python-owned fusion this still transfers and retains O(entities ×
views) rank state, although it removes O(chunks × dimensions) resident vector arrays. The repository
owns query/transfer deadlines and byte/row caps; the fusion owner preflights retained rank state.
Exceeding an exact-route budget returns an explicit resource refusal, or invokes an explicitly
requested approximate profile. It never truncates a leg and labels it exact. ANN candidate queries
have a separate bounded result contract. This makes the cost visible without moving lexical policy
or inventing a second fusion implementation merely to reduce the number of language crossings.

Current NumPy scoring accumulates in float64 over float32 inputs. PostgreSQL exact nearest-neighbor
search means exhaustive comparison, not guaranteed bit-identical arithmetic to that implementation.
Specify numerical tolerances and deterministic tie behavior; a change to numeric ranking semantics
gets a retrieval-profile version. Preserve complete per-leg ranks for the parity route, even if the
database computes them. The scalable route is a separately declared candidate/ranking policy.

Retain `bm25s` Lucene BM25, its discriminating-term abstention and exact-symbol promotion initially.
PostgreSQL `ts_rank`/trigram matching is useful additional discovery but is not a drop-in BM25 replacement.
A later lexical migration needs its own policy and consumer; no speculative extension stack is needed
for pgvector adoption. Dense vector arrays and general relational maps can leave Python, while lexical
and native state still consume memory. End-to-end startup/RSS/latency benefits remain **Proposed**.

### 4.4 Arrow ingestion and federated reads

**Tested:** an isolated real PG18 probe fed `pgpq` binary output directly into SQLx `copy_in_raw`,
preserving checked IDs, signed zero, text null/empty distinctions and nullable arrays; explicit abort
left the connection usable. This establishes the composition, not every domain type or throughput.
`pgpq` has no pgvector-specific encoder: use its supported `real[]` staging path followed by a
validated conversion to `vector(1024)`, or the SQLx pgvector type adapter for smaller batches.
Keep import transaction ownership in SQLx. [Bridge evidence](../evidence/2026-09-27_postgresql-expansion/bridges/README.md).

**Interface-checked:** the migrated PostgreSQL provider uses row protocol conversion in chunks of
4000, not an Arrow-native PostgreSQL engine. It maps `bytea` to variable-width Binary and UUID to
Utf8, and does not recover the repository's original domain field metadata. The provider's own
`source_type` metadata survives the assembled query; this is not universal metadata loss. Successful expression unparsing
is normally advertised as `Exact` pushdown; each physical SQL execution obtains its own pooled
connection. These facts require a narrow domain adapter and a maintained fork, not just a Cargo pin.
[Pinned provider](https://github.com/CaptainEureka/datafusion-table-providers/tree/33095588fcdd17301a5d1c340dcd66cd60e41ec8),
[exact source locations](../evidence/2026-09-27_postgresql-expansion/bridges/README.md).

The provider owns a separate Rust-Postgres pool; it cannot reuse the SQLx pool. Its configuration
accepts explicit fields or libpq keyword syntax, and the probe rejected the SQLx-style URL as a
`connection_string`. Adapt the protected application config once at this boundary, with explicit
TLS, secret redaction and a combined connection budget for both pools. Avoid propagating the
driver's option vocabulary into compiler or MCP contracts.

**Proposed:** expose a small set of read-only, explicitly typed PG views. Normalize into declared
Arrow contracts and enforce fixed-width IDs, nullability, list shape, enum/code meanings and metadata.
Qualify both table scan and federation rewrites; restrict unqualified expressions and collations
instead of trusting “SQL can be printed” to establish semantic equivalence. Inexact filters retain
their residual and must not receive a premature limit. Use byte-stable identity ordering and an
explicit text collation where ordering/comparison must match Rust/Python.

For immutable ready generations, explicit generation predicates and retention provide coherent
cross-connection reads. For mutable reports, first materialize the required relations in one
read-only repeatable-read transaction, or explicitly export/import its snapshot into every worker
before queries. The current provider does not implement that protocol. A pool or multiple independent
repeatable-read transactions is insufficient. [PostgreSQL 18 snapshot rules](https://www.postgresql.org/docs/18/sql-set-transaction.html),
[snapshot synchronization](https://www.postgresql.org/docs/18/functions-admin.html#FUNCTIONS-SNAPSHOT-SYNCHRONIZATION).

## 5. Change and failure scenarios

**Implemented traces and Proposed routes:**

| Trigger | Owner and contract change | Propagation / hidden knowledge exposed | Settling evidence |
|---|---|---|---|
| Standard becomes 1024 | Embedding spec, launcher and both clients | Editing width alone leaves request bodies and launch metadata at 4096 behavior; analytics, receipts and serving must rebuild from the new spec | Shared request/normalization controls, cold/warm receipt replay and one new generation; no dimension-quality bakeoff |
| Add a facet or concept membership | Canonical declaration plus serving projection/query owner | Today Python maps and selectors assume complete in-memory tables; SQL must retain per-facet unknown coverage and evidence | Fixture with absent-complete and absent-incomplete cases, same exact answers and totals |
| Import is interrupted after some tables | Projection writer and manifest lifecycle | A table's existence or populated vector index cannot imply the whole generation is ready | Kill/retry and mismatched-content controls; reader never sees partial closure |
| New library release or serving schema | Canonical snapshot identity plus independently versioned projection identity | Entity aliases/cursors/supporters must not resolve through a newer active pointer | Two generations served concurrently; cross-generation cursor and supporter rejection |
| Query filter selects few entities with many chunks | Retrieval profile | Fixed chunk top-k can underfill and distort per-view RRF; cross-generation ANN worsens it | Adversarial duplicate-chunk/filter/tie cases; explicit approximate versus exact route |
| A nullable array/ID crosses PostgreSQL→Arrow | Domain codec and provider fork | Physical SQL/Arrow compatibility does not restore fixed-width IDs or native metadata | Round-trip declared schemas, empty result schema, wrong width, null element and unsupported-type refusal |
| Mutable attempt state changes during federation | Read-view owner | Independently acquired connections can join mutually inconsistent observations | Frozen transaction materialization or explicit imported-snapshot test under concurrent update |
| MCP request is cancelled or DB unavailable | Serving pool/lifespan owner | Moving sync dictionary tools to asynchronous I/O changes timeout and cancellation behavior | Cancellation, pool exhaustion, startup readiness failure and connection reuse controls; no silent cross-generation fallback |
| Operator reviews a published brief | Review-event owner and frozen-input selection | Directly editing projection status would create a competing published truth | Conflicting revision/event test and replay from the selected immutable review input |
| Add another database-facing tool | Typed Rust repository boundary | Repeating SQL in Python introduces a second completeness/hydration owner | New consumer composes existing typed operations; no independent migration or semantic query catalog |

## 6. Correctness and fidelity gates

The first verdict assesses extending the **existing accepted target** to the newly selected scope.
The second assesses this review's **Proposed replacement contracts**, using design-level reasoning.
An unimplemented contract is not itself an architectural failure. Product qualification is a
separate column; historical PG0–PG7 acceptance is not reversed.

| Gate | Existing target / Proposed replacement | Reason and remaining qualification |
|---|---|---|
| G1 Authority | pass / pass | Delta canonical evidence, PG authored events/operations and derived serving are distinct; registry and semantic kernel retain their owners |
| G2 Semantic fidelity | unresolved / pass at contract level | F02/F04/F05 specify exact facet/unknown/support closure, native schema normalization and separate retrieval profiles; backend semantic conformance is not yet tested |
| G3 Validity | unresolved / pass at contract level | F03/F04 reuse canonical validators and add explicit codec/readiness checks; executable enforcement remains implementation work |
| G4 Hidden behavior | unresolved / pass at contract level | F01/F02/F04 make launch overrides, serving effects and mutable read views explicit; operational integration not_run |
| G5 Consistency and recovery | unresolved / pass at contract level | F03/F04 specify immutable ready generations, conservative retention and coherent mutable materialization; interruption/cancellation controls not_run |
| G6 Transformation and reuse | unresolved / unresolved for full provider adoption | New spec and retrieval identity contracts are specified; the fork's actual admitted predicate/cast subset and complete domain codec still need qualification before semantic queries use them |
| G7 Truthful capability claims | pass / pass | Resolution, compilation, execution and proposals are distinguished; no full serving, performance or dimension-fidelity claim is inferred from bridge probes |
| G8 Library leverage | pass / pass | SQLx+pgpq, pgvector and compatible providers replace generic client/COPY/vector/federation work without redundant application drivers |
| CI-G1 Fact fidelity | unresolved / pass at contract level | F02/F05 retain exact/unknown selection, complete hydration and the pure semantic executor; exact-answer integration not_run |
| CI-G2 Traceable publication | unresolved / pass at contract level | F03 makes same-generation closure and artifact readiness publication conditions; corruption/replay integration not_run |
| CI-G3 Independent evaluation | pass / pass | No gold/held-out input or parameter tuning; standard 1024 is an operator decision. Future ANN controls assess the index policy independently |

## 7. Findings and applicability

These are target-design gaps, not claims that unimplemented features currently return bad answers.
**Disposition transferred 2026-09-27:** the [forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
is the single current status owner for **PGE/F01–F05**. The updated
[PostgreSQL plan](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream) owns PG8–PG17's detailed
work and acceptance. The findings below retain this review's dated evidence and corrections;
planning does not establish implementation or closure.

<a id="F01"></a>
### F01 — The selected embedding width lacks an explicit end-to-end request/launch contract

**Interface-checked · FP-02/04/05; DP-08/09/24; A1/A2; G4/G6.**
`Spec` (`crates/cpg-core/src/embed.rs`) hashes dimensions, but
[`lctx-embed`](../../../crates/lctx-embed/src/lib.rs), the
[Python embedder](../../../python/lctx_mcp/src/lctx_mcp/embedder.py) and launcher implement the
old no-dimensions route. Merely changing the JSON cannot establish 1024 service output and can make
all responses fail width checks. Vectors also feed compile-time analytics, so reusing old derived
outputs under the new identity is invalid.

**Correction:** version the spec's MRL/normalization/launch contract; derive the override and both
request bodies from it; use 1024 for all new production inputs/outputs; rebuild dependent analytics,
receipts and serving generations. Preserve the exact cache codec and historical spec interpretation.
Remove the “never send dimensions” rule and 4096-specific new-generation assumptions. Do not create
a permanent 4096 analytics/1024 retrieval split.

**Closure:** request/response known answers, finite/unit/width refusal, spec mismatch, exact cold/warm
receipt replay and controlled launch conformance. These verify implementation, not the already
accepted fidelity decision. **Disposition:** [forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).

<a id="F02"></a>
### F02 — A vector-only database integration leaves serving ownership tied to full-file hydration

**Implemented trace / Proposed correction · FP-01/02/03/06; DP-06/16/17/18; A1/A3; CI-04/11.**
`generation.load`, `operations` and `server` materialize relational tables and support indexes in
Python. Adding SQL only to search leaves exact lookup, unknown reasoning and hydration dependent
on those maps; adding independent SQL copies to individual MCP tools would duplicate the semantic
decisions. A new facet or evidence family would then require hidden changes in two query systems.

**Correction:** split generation identity/native/lexical state from a Rust serving repository.
Transfer exact selection and batched evidence hydration into that owner, retaining typed tool
contracts and the pure semantic executor. Use a coarse asynchronous Python boundary with one pool
lifecycle. Delete superseded production maps/query paths after answer parity; retain portable
exports and independent fixtures. Direct Psycopg is the simpler alternative if Rust reuse is
abandoned, but do not implement both production query owners.

**Closure:** exact get/find/hydration answers across unknown coverage, aliases, condition verdicts,
parallel witnesses and pagination; one new fact/facet traced through declared projections; bounded
DB/native fixtures and cancellation/lifespan checks. **Disposition:** [forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).

<a id="F03"></a>
### F03 — Readiness must cover the whole serving/evidence generation, not individual indexes

**Proposed gap · FP-04/05; DP-03/04/19/20; A2; G3/G5; CI-11/13.**
The old F2 deferral names staged import but does not settle the expanded PG relations plus native
artifact lifecycle. Importing vectors before supporters, selecting “latest” during hydration, or
deleting an artifact still used by a reader breaks same-generation answers without changing any
individual SQL row's validity.

**Correction:** separate canonical snapshot and serving projection identities; generation-qualified
keys/FKs; immutable native/lexical artifact references; import counts/digests and shared evidence
validators; invisible loading states and atomic ready/selection transaction. Separate importer and
read-only serving roles. Keep retention disabled initially; introduce reader protection before
deletion. Rebuild projections from canonical data, not from partially restored derived tables.

**Closure:** interrupted/retried import, duplicate/conflicting identity, missing supporter/artifact,
cross-generation cursor, two-reader rollover and restore/rebuild controls. **Disposition:** [forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings),
alongside existing PGS/F02 without rewriting that source finding.

<a id="F04"></a>
### F04 — A compatible provider graph does not preserve domain schemas, predicates or read views

**Interface-checked and bounded Tested · FP-02/03/05/06; DP-08/11/15/19; A2/A3; G2/G5/G6.**
The inspected migration maps fixed identities through variable-width Binary, reconstructs fields,
uses permissive `Exact` pushdown and acquires connections per SQL execution. A successful build
cannot establish native schema equivalence, PostgreSQL/DataFusion expression equivalence or one
MVCC snapshot. The native decoder explicitly requires fixed-width IDs and declared metadata.

**Correction:** maintain the migration in an owned, immutable-revision fork; add a narrow declared
schema codec and restricted read views. Qualify scan and federation predicates/limits separately;
decline unsupported transformations. Immutable generation predicates are the default read-view
mechanism. Materialize mutable inputs through one transaction until a real exported-snapshot
adapter is required. ADBC remains an alternative transport, not an exemption from these contracts.

**Closure:** one resolved dependency family, compiled feature graph, actual provider results and
domain round trips, empty schemas, adversarial null/order/filter/limit cases and concurrent mutable
read-view controls. Existing bridge probes close only their named subset. **Disposition:** [forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings);
relate to PGS/F04, leaving Cornucopia-specific PGK/F01 separate.

<a id="F05"></a>
### F05 — Indexed vector candidates cannot silently inherit exhaustive selection or current full-rank fusion

**Implemented trace / Interface-checked · FP-02/05; DP-02/08/11; A2; G2/G6; CI-04/09/13.**
Current retrieval aggregates best chunks and ranks all allowed entities per view before RRF.
Filtered ANN over chunks has a different universe; exact reranking of retrieved candidates does
not recover missing entities/ranks. Moving facet selection onto that universe would additionally
turn unknowns or unvisited matches into false completeness. PostgreSQL full-text rank also differs
from the existing lexical policy.

**Correction:** separate exact facet enumeration, exact-vector parity and explicit ANN discovery.
Version candidate depth, per-view aggregation, filter strategy, arithmetic/ties, underfill/fallback
and fusion semantics. Generation-scoped indexes and deterministic evidence hydration are mandatory.
Keep BM25 policy and compiler kNN until an independently scoped change selects another mechanism.

**Closure:** independent 1024 exact-search reference, filtered/duplicate-chunk/multiple-view/tie cases,
explicit ANN result metadata and resource-limit behavior; exact operation answers unchanged.
ANN recall/latency checks concern index approximation, not MRL fidelity. **Disposition:** [forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings);
planned work replaces the previous 4096 index obstacle and LanceDB-only future route.

**Foundation verdicts for the Proposed replacement:** FP-01/06 are satisfied by the separated
repository, semantic executor and application lifecycle in F02; FP-04/05 by the authority, typed
identity and readiness contracts in F01/F03. FP-02/03 are satisfied for serving composition but
unresolved for unrestricted provider substitution: F04 still requires an admitted semantic subset.
DP-13/14/16 favor the selected thin library composition; DP-01/05 preserve canonical and authored
authority. These are architectural judgments, not claims of implemented enforcement. No claim is
made about unrelated extraction or reasoning-rule compliance.

## 8. Library fit and total complexity

**Interface-checked unless a tested subset is named. Proposed selections, not installed dependencies.**
Keep the repository's exact **DataFusion 55.1 / Arrow 59.3** pins. A dependency requesting55.0/59.0/59.2
semver can resolve to those versions; the user's baseline does not require downgrading the repository.
See [bridge source and resolution evidence](../evidence/2026-09-27_postgresql-expansion/bridges/README.md).

| Library / pin candidate | Direction and fit | Target decision and total burden |
|---|---|---|
| SQLx 0.9.0, existing features | Application→PG; typed static queries, pool, migrations, transactions and raw binary COPY | Retain as application driver/migration owner. Avoid duplicate transaction clients, pools and migration histories |
| `pgvector` Rust 0.4.2, `sqlx` feature; PG extension 0.8.6 | SQLx vector adapter accepts SQLx>=0.8,<0.10; extension supplies distance/index mechanisms | Select for 1024 search projections. Pin client, extension and disposable image separately; operator deployment is a later explicit step |
| `pgpq`0.12.0 | Arrow 59 RecordBatch→PG binary COPY encoder | Select for projection imports with SQLx COPY. Real-PG composition tested; retain explicit domain validation and vector staging conversion |
| `datafusion-federation`0.5.7 | DataFusion optimizer/execution support for remote query pushdown | Select with the PG read adapter; it is not a connection/transaction consistency mechanism |
| `datafusion-table-providers-postgres` at `CaptainEureka/...@33095588fcdd17301a5d1c340dcd66cd60e41ec8` | PG→DataFusion 55/Arrow 59, internally Rust-Postgres pooling and row conversion | Select migration as the starting point for an owned fork. Accept the provider's internal driver as a bounded read-adapter dependency; do not turn it into a second app write API |
| `datafusion-table-providers-adbc`, same revision | ADBC source→DataFusion, including a native PG driver | Qualified alternative for large Arrow reads after actual native-driver/type/cancellation evidence. Do not run it alongside the PG provider for the same default consumer without a measured reason |
| `adbc_core`, `adbc_driver_manager`, `adbc_ffi`0.24.0 | Rust API and C-ABI driver boundary; accepts Arrow 59 | Compatible reserved path. Native PG driver is a separately pinned/deployed binary. C ABI removes Arrow Rust major coupling, not type or lifecycle qualification |
| ADBC 0.25-dev `c942d481c6e083040c68676e3dd454dad89503e9` | Wider Arrow>=58,<61 declaration | No need to prefer development code over compatible 0.24 absent a required feature; retain as documented upgrade route |
| `r2d2_adbc`0.3.0 | Blocking ADBC connection pool | Only with a selected synchronous ADBC consumer; bounded blocking executor and one pool. Broad core dependency requires a resolved 0.24 anchor, not manifest-only assurance |
| `datafusion-postgres` family `eda0da032ed8d6003b5041fce67c1e5b2f101876` | PostgreSQL wire server over DataFusion; `arrow-pg`, PG catalog/functions are companions | Compatible future SQL-client surface, not PG storage ingestion. Add only for a named psql/BI client; separately own exposed SQL, auth, cancellation and pinned-session scope |
| `adbc-driver-datafusion`0.27.0 source `3d24e0f3ad8bf914b9d2a48d0151fc313b8ae28c` | DataFusion→ADBC clients | Compatible future embedded/columnar client interface, not the PG ADBC driver. Avoid introducing it just to read PostgreSQL |
| `tokio-postgres`0.7.x | General PG protocol client, already used inside provider family | Technically suitable alternate driver; no separate direct app dependency merely for COPY, since SQLx+pgpq composition works |
| `pyo3-async-runtimes`0.29.0 | Rust async repository→Python awaitable | Select if implementing the proposed Rust serving owner; small boundary with existing PyO3 0.29.2, not a second semantic executor |
| Psycopg3; optional SQLAlchemy | Python→PG; appropriate for Python-owned relational workflows | Keep as documented alternative/future operational consumer. Main serving uses the Rust repository; do not install an ORM or Alembic to duplicate Rust-owned schema/query meaning |
| SeaQuery + SQLx binder; Cornucopia | Dynamic query construction / alternate generated query catalog | Use SeaQuery only when typed variable composition exceeds SQLx's clear static/query-builder path. Cornucopia remains an alternative driver/catalog architecture, not another layer on this stack |
| `connector_arrow`0.12.2 | Arrow 58 source adapter | Exclude from this pinned family; the other compatible routes suffice |

The isolated lock resolves the supplied alternatives together with SQLx and the exact delta-rs
revision without duplicating DataFusion/Arrow/ADBC families. This is stronger than a manifest check,
but not a reason to enable all optional packages in the product. Provider, native driver, server
extension and protocol endpoint each have distinct build/runtime obligations.

## 9. Alternatives and tradeoffs

**Proposed comparative judgment; end-to-end cost improvements are not measured.**

| Alternative | Change/local reasoning and authority | Machinery, test boundary and decision |
|---|---|---|
| Existing Delta + PG cache + full-file serving | Strong offline publication; whole-corpus Python maps and vector scans remain the online access model | Simplest current deployment; preserve as migration oracle/export. Insufficient target for the now-selected indexed relational/search consumers |
| Selected Delta canonical + PG serving/operations | Canonical compiler evidence remains unchanged; one Rust online repository localizes joins and transactional behavior | Adds explicit projection import/readiness and PG availability to serving; removes dense-vector/full relational hydration paths. Best fit for agreed expansion |
| PG for all canonical facts | Multi-table transactions and direct indexed queries simplify some publication mechanics | Would migrate every writer, Arrow contract, validator, replay and historical reader. Current bridge conversions do not establish exact canonical round trips; no demonstrated need to replace the accepted Delta model |
| LanceDB vectors + PG relational serving | Could support dedicated vector workloads and existing future trigger | Adds a third serving-state lifecycle and cross-store candidate/hydration coordination.1024 removes the main pgvector obstacle; choose PG for this target |
| Psycopg3 serving repository | Fits existing Python tool/query owner with less FFI integration | Viable simpler implementation alternative, but leaves database query/type logic Python-owned while Rust imports and tools grow. Choose Rust for shared domain ownership; reconsider only if async binding cost exceeds actual reuse |
| ADBC-first PG scans | Columnar transport can reduce row conversion for bulk scans | Native driver distribution, blocking lifecycle and semantic codecs remain. Keep a real alternative, activate when a representative scan demonstrates benefit over the selected provider |
| Load every available bridge/protocol/ORM | Many interfaces are individually compatible | Duplicated pools, session semantics, feature graphs and support surfaces without new product behavior. Reject this composition, not the individual libraries |

PostgreSQL now has sufficient concrete consumers to expand without waiting for the old generic
“measured bottleneck” trigger: exact relational serving, generation-qualified filtered retrieval,
selective evidence hydration and operational joins. Performance measurements should size and tune
that deployment, rather than reopen the operator's choice of 1024 or justify every basic DB primitive.

## 10. Verification and uncertainty

| Claim | Label/date and command/evidence | Outcome / limit |
|---|---|---|
| Supplied revisions exist and align with repository family | Interface-checked, 2026-09-27; exact source manifests and [bridge probe lock](../evidence/2026-09-27_postgresql-expansion/bridges/Cargo.lock) | Resolved compatible family; no root manifest/lock changed |
| Combined candidate graph with exact delta-rs and SQLx | Tested, 2026-09-27; isolated `cargo metadata` and feature-build commands in [bridge README](../evidence/2026-09-27_postgresql-expansion/bridges/README.md) | **passed:** both providers/federation/delta-rs/SQLx/pgpq compile; optional protocol-server packages were resolved, not compiled |
| pgpq→SQLx binary COPY | Tested, 2026-09-27; isolated `cargo run` with a disposable PG18 instance | **passed:** checked primitive/array cases and abort/reuse; no throughput or pgvector import claim |
| Provider/federation behavior | Interface-checked plus named executable subset in bridge evidence | **passed:** real-PG null filter/limit, empty result and same-source join; complete domain predicates/read views unresolved |
| 1024 dimension choice | Operator decision, 2026-09-27 | Settled input. No further dimension-fidelity validation required |
| MRL normalization/launch | Interface-checked, 2026-09-27; pinned installed vLLM sources and model config | Correct implementation route identified; controlled 1024 live launch not run in this design review |
| Existing offline geometry observation | Measured, 2026-09-27; [embedding evidence](../evidence/2026-09-27_postgresql-expansion/embedding/README.md) | Informational probe completed before operator clarification; not a quality evaluation or adoption gate |
| Payload size | Calculated from float32 width | 1024×4=4096 bytes/vector versus 16384 at 4096, before storage/index overhead; no claim of 4× end-to-end speed or smaller model weights |
| Previous PG0–PG7 acceptance | Historical Tested, 2026-09-27; [original evidence](../evidence/2026-09-27_postgresql/README.md) | 430 ordinary Rust  + 8 real-PG  + 154 Python tests and live/replay/restore controls reported there; not rerun here |
| Expanded product acceptance | `just test-all`, `just pilot`, PG serving/ANN/native-ADBC checks | **not_run:** review-only scope; implementation does not yet exist |

Implementation should qualify semantic known answers and failure controls before broad performance
measurement. The relevant corpus sizes are entity/chunk counts, evidence fan-out, selective-filter
cardinality, retained generations and native/lexical resident bytes. Record import time/WAL/index
size, startup/RSS, exact/ANN latency and pool saturation on those workloads. A single COPY probe
or dimensional payload calculation cannot establish whole-system improvement.

## 11. Authority changes and dispositions

**Proposed next-document changes; none are made by this review.**

| Owner | Change for the next planning/decision step | Finding route |
|---|---|---|
| PostgreSQL implementation plan §§2–7 | Promote F2 serving, F3 pgvector and F5 COPY/provider work into the integrated scope; include standard 1024 and Rust async serving. Replace obsolete incompatibility/4096 obstacles with pinned revisions and concrete contracts | PGE/F01–F05; preserve PGS/PGK source IDs and PG0–PG7 historical receipts |
| Forward plan §3.4/§6 and Stage 4 | Reflect PG serving and frozen review/concept projections as dependencies of their actual consumers. Preserve Stage 3 reasoning/exit obligations and concept-definition ownership | Transfer scheduled PGE disposition here by link, not a second mutable register |
| ADR-0065 successor and §6.5 | Expand Rust PG service ownership to serving/import/query effects; maintain exact cache and one migration owner | F02/F03/F04; supersede accepted record if decision changes, never edit its rationale in place |
| ADR-0066 successor, §B13 and §§11.1–11.4 | Replace file-only serving target, future LanceDB selection and 4096 no-dimensions rule with pinned PG generation/readmodel/1024/ANN contracts | F01/F02/F03/F05 |
| ADR-0067 and §6 canonical publication | Retain Delta publication and exact consumed-vector receipts; link the new derived projection lifecycle without claiming a cross-store atomic commit | F03; supersession only if a later decision actually changes canonical ownership |
| Schema/bundle/native contract owners | Declare projection format, domain codec and generation metadata once; preserve source/type/codebook/evidence authority and compatibility rules | F02/F03/F04 |
| Pins, Cargo/uv, runbook and development checks | Record exact maintained fork, features, bridge/native/extension versions; explicitly deploy extension and roles; update conformance/request corpora and SQLx metadata as required | F01/F04/F05; use existing checks rather than adding a new process registry |

F1 review workflow remains tied to a selected operator-facing workflow; its event/freeze contract is
now explicit. F4 dynamic queries and F6 direct Python/ORM remain consumer-specific alternatives.
F7 notifications/jobs use durable rows and optional wakeups only when workers exist. F8 pgrx remains
unnecessary for ordinary candidate fetch plus the native executor. F9 canonical replacement remains
outside this target. F10 gains concrete projection retention/restore work, but replicas, pooler and
PITR topology remain requirements-driven. SQL-wire/ADBC client surfaces are documented expansion
options, not unrequested background servers.

## 12. Architectural judgment and decision

| Judgment | Existing target / Proposed replacement | Scenario basis |
|---|---|---|
| A1 Localize change | unresolved / satisfied, Proposed | F02 gives selection/hydration one owner; F01 derives client/launch choices from one spec. New facets no longer require independent Python and Rust SQL meaning |
| A2 Encode meaning structurally | unresolved / satisfied, Proposed | F01/F03/F05 separate spec, canonical snapshot, projection, read view and ranking profile; readiness and unknown states have explicit contracts |
| A3 Extend through composition | unresolved / satisfied for serving and COPY; unresolved for unrestricted federation | Native evaluation, lexical policy and Rust query operations compose without repeated semantic owners. F04's provider expression/codec boundary remains a qualification obligation before broader queries are supported |

**Bounded decision: Revise the existing target and then the existing plan.** Adopt 1024 as settled, select
PG relational serving and pgvector, retain SQLx, add pgpq and the qualified compatible provider/federation
path, and preserve canonical Delta plus the pure native semantic engine. The revised plan should
sequence spec migration → projection contracts/import → exact serving/hydration → explicit indexed
retrieval → bounded federation and integrated acceptance. ADBC and client protocols retain concrete
future routes without redundant default machinery.

**Enclosing architecture:** the initial deployed PostgreSQL scope remains previously qualified.
The proposed serving boundaries have a credible architecture; the selected provider's unrestricted
semantic substitution remains unresolved under G6/A3, and expanded product qualification is
**not_run**. Stage 3 completion remains governed by its existing forward plan. The next action is
the requested plan/ADR-owner revision using this review's findings; deployment has not been expanded
during the review.
