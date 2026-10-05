# Design review: SurrealDB as a graph store, Neo4j as an analytical layer, and the snapshot model

| Field | Value |
|---|---|
| Subject | Store, compile pipeline shape, analytics placement, serving query engine and snapshot model of library-context at `f66eb15a` (code identical to `1158ebe2` for every cited path); evidence folders listed below |
| Standard | Core principles and template 3.2; code-intelligence profile 1.3; library-context binding (`docs/design_review/design_principles/standard.toml`) |
| Tier · purpose | Design · target |
| Reviewer · date | design-reviewer subagent (fresh context), 2026-10-05 |
| Supporting document | [Snapshot model and cross-version diff](design_review_surrealdb-graph-store_2026-10-05_snapshot-model.md): decision 3 and the typing-as-graph-metadata question; no competing verdict |
| Evidence | Phase A lanes A1–A8 and A3b: [`2026-10-05_surrealdb-pivot-lanes`](../evidence/2026-10-05_surrealdb-pivot-lanes/README.md). Phase B probes: W1 [`2026-10-05_surrealdb-native-realization`](../evidence/2026-10-05_surrealdb-native-realization/README.md), W2 [`2026-10-05_snapshot-diff`](../evidence/2026-10-05_snapshot-diff/README.md), W3 [`2026-10-05_graph-analytics-parity`](../evidence/2026-10-05_graph-analytics-parity/README.md) |
| Status | Final on the complete static and synthetic evidence (lanes A1–A8, W1–W3) plus the 2026-10-05 real-compile failure. Real-data probe outcomes on the facts layer were recorded by the coordinator after the review was written (§10.2); none changes decisions 1–2. W2 refuted `native_key` as F06's correspondence key (§10.2). The review was concluded at operator direction with the remaining real-data parts `not_run` or blocked by F01. A [coordinator addendum](#coordinator-addendum) (operator-requested, static) judges the full graph-native pivot and amends decision 1's wording; findings, gates and A1–A3 are unchanged. |

---

## Summary of the decision

**Scoped decision: Revise** the compile/validation/store/serving architecture, for causes that
are independent of the store choice. **Pivot the architecture now, not the store: do not adopt
SurrealDB 3.3 now, and keep it as the leading alternative lowering behind the F12 seam
([coordinator addendum](#coordinator-addendum)). Do not integrate Neo4j.** Keep analytics in process and keep PostgreSQL as the canonical store and
serving engine for the present. The next consequential decision (F12) is the **pipeline shape**:
the evidence favours compiling and validating the typed generation in process over immutable
Arrow batches and persisting it once (R3), over continuing to validate by querying the store.
That shape, not the choice of database, accounts for most of the simplicity and cost the
operator's hypotheses attribute to SurrealDB.

**The primary structural finding (F12, §2.6).** The design intent is that every stored element,
query, check and analysis is *connected* to the typed declarations: derived from them and
traceable to them. The implementation also *couples* to them: one relation is one table, one
grant set, one receipt, one validation read, one serving hop, and its lifecycle authority is
realized through per-table privileges; the contract's identity is the bytes of its source text.
Connection is required; coupling is not. Graph projections are the counterexample that shows the
decoupled form already works here. Decoupling (declared, replaceable lowerings with explicit
identity mapping) is store-independent, is the prerequisite that would make any later store change
contained and reversible, and is therefore ordered before any store decision.

The four brief decisions:

1. **SurrealDB role: (d) not adopted now; leading alternative lowering** (amended by the
   [coordinator addendum](#coordinator-addendum); the reviewer's original wording was "(d) not
   adopted" in any role). As primary store it replaces roughly the same volume of
   substrate-compensation code with different bespoke code (leases, publication, validation pass,
   GC, Arrow↔SurrealValue codecs, server operation); weakens "immutable once published" from a
   store-enforced property to a loader protocol; brings traversal semantics that need a wrapper
   before they honour CI-03, CI-04 and CI-08; and adds maturity risk. The 77% of serving code
   that is semantic assembly, the 51 replay validators and every analytic kernel stay in Rust
   whatever the store. As a rebuildable graph/search projection it has **no named consumer**: the
   product exposes no arbitrary traversal (§14.5), served paths are fixed-depth typed reads, and
   ranking has one Rust owner (ADR-0114). Under the recommended pipeline shape (R3) the store's
   role narrows to "bulk-load once, verify, serve", where SurrealDB's gaps (no Arrow bulk path,
   no frozen database) weigh most and its strength (traversal) is unused. Triggers and the
   capabilities a projection would use are in §9. The addendum judges the immutability point
   adequate for this project and not a deciding factor.
2. **Neo4j: not integrated**, neither on SurrealDB nor beside PostgreSQL. GDS does not deliver our
   exact-kNN contract; its Leiden and Louvain are not deterministic under our CI-09 conditions; its
   PageRank lacks our diagnostics; every capability we use is already in process with stronger
   contracts. Its unique capabilities (trained graph embeddings, ML pipelines) have no consumer
   among the product's functional outcomes (§1.1, §14); a named consumer would reopen this (§9).
3. **Snapshot and representation: restate the generation contract mechanism-neutrally; do not
   make generations structurally shared now.** Sharing is about zero across binaries and library
   versions under current identities. Model changes worth making whatever the store:
   cross-version correspondence keys (F06), narrowed model-contract and provider identities (F05),
   and a Merkle relation digest once a consumer exists (F07). Most validation (62 of 119 checks,
   51 of them replays) is closed-world and does not become incremental by sharing. On the
   operator's follow-up, **typing as graph metadata**: the store's typing is a lowering, and the
   authority must stay one Rust declaration because the analyses stay in Rust; graph kinds are
   tables under another name. Merging the 75 uniform support relations and 150 lifecycle tables by
   discriminator is a lowering decision available on any store, coupled to F12 (§2.4).
4. **Second-order consequences** are inventoried per candidate in §8.2, including compile-then-store.

**Defects worth fixing regardless of the store** (§7): the real-library compile failure caused by
validation holding a transaction across unbounded CPU under the product's own 30 s idle cap (F01,
priority 1); reference closure realized four times with costly per-field anti-joins (F02);
uninstrumented triple re-reading in validation (F03); serving semantics in the PostgreSQL effect
crate with hand-chained N+1 hydration (F04); `explain` re-hashing whole derivation relations per
request, contrary to ADR-0126 (F13); over-broad identities that force store resets and prevent
reuse (F05); a provider fork used only as a scan transport (F08); no store-free stage-read seam
(F09); dead services and unselected controls (F10, F11); and publication replay of 51 stages that
neither challenges producer logic nor, under controlled inputs, adds much beyond a determinism
check, where certificate and coverage checks would serve (F14). Bespoke code by component and
mechanism is in §2.7.

**Order of recommendations.** (1) Unblock real scale (F01). (2) Decouple on the model side: a
declared lowering seam, store-free stage-bound reads, identity from declarations (F12 part 1, F09,
F05). (3) Decide the pipeline shape and published realization, with R3 leading (F12 part 2, PR-3).
(4) The remaining corrections. The store question is then contained and reversible.

**Judgments.** A1 **violated** (mechanism substitution, model change and real-scale validation;
F12, F01, F04, F05, F08, F09). A2 **satisfied** for the current supported catalog scope, **violated**
for the target new-release journey (F06). A3 **satisfied** for adding an analytic, **unresolved**
for a new agent journey (F04). Gates G1–G8 and CI-G1–CI-G3 **pass** for the baseline within its
recorded scope; the Revise rests on DP-19/DP-20 MUST violations in claimed behaviour (F01), A1 and
A2. Candidate failures against the gates are recorded separately in §6.

**The operator's hypotheses** (§2.3): "brittle" is supported, with the cause in the store-centric
pipeline and its immutability protocol, which SurrealDB does not remove; "low-performance" has no
measurement, and the visible costs (store-side re-reading, per-field anti-joins, per-request
re-hashing, validation inside store transactions) are pipeline choices that move unchanged to any
store; "far too bespoke" is true of ~8–10k lines of substrate compensation, most of which
compile-then-store retires on PostgreSQL as well; "graph-shaped" holds for the shape of serving
reads and a few kernels, not for storage (≤47 of 878 relations are natural edges) or for the
payload algebra; "snapshots as graph objects with diffs" is a sound model direction whose value
comes from correspondence keys and identity scope, not from a graph store.

---

<a id="coordinator-addendum"></a>

## Coordinator addendum: the full graph-native pivot (2026-10-05)

*The coordinating agent added this at the operator's request, after the review above was written.
It is a static judgment of the strongest form of the hypothesis. It amends decision 1's wording
(Summary, §8.1, §9, §11, §12). The findings, gates and A1–A3 judgments are unchanged. Every
benefit and cost statement here is **Proposed**.*

**The pivot assessed.** The model compiles in memory as a typed graph and persists natively in
SurrealDB. SurrealDB traversals serve it, and nothing remains in PostgreSQL.

**The functional need.** The product compiles a pinned library into a typed graph: entities,
typed relationships with their own identity, provenance on every assertion, explicit unknowns, and
derivations traced to premises. It serves that graph as bounded, evidence-closed answers.

The data layer must:
- hold an immutable, validated snapshot that readers can pin;
- answer fixed-shape traversals (member → contract → evidence → original) and a few bounded
  searches (`explain`, access routes);
- answer hybrid lexical and vector search.

It does not run the analyses. Normalization, transfers, conditions and summaries (about 124k lines)
are the Rust compiler under any store (A2, A3 §1).

**The SurrealDB encoding.**
- **Node kinds.** SCHEMAFULL tables, generated from the Rust declarations. The declarations remain
  the authority, because the compiler needs nominal types (§2.4).
- **Relationships.** Edge tables with their own content key, enforced endpoints and n-ary extra
  links.
- **Attribution as properties.** One `supports` edge kind carries run, surface, origin, mode and
  fidelity, replacing 75 uniform support tables. Observations become edges from the provider run.
  The 150 macro lifecycle tables collapse into a few typed outcome kinds. Physical tables would
  fall from about 1,100 to a few hundred.
- **Snapshot.** In-memory compile and validation, one write into a database per generation, a
  digest check through the reader decode path, read-only reader credentials, a pointer-record
  publication, and removal of the database at retirement.
- **Serving.**
  - Declared paths become fixed-depth arrow traversals with fetches; reverse references are
    native.
  - `explain` and access routes stay bounded Rust searches, or use explicit bounds plus a bound+1
    check.
  - BM25 and the vector index run in the engine, with an exact rerank in Rust.

**What it would mean.**
- **Gains:**
  - Journeys become traversals, removing about 1–1.5k lines of hand-chained hops (A8).
  - Provenance as edge properties matches the domain better than wrapper tables.
  - Search runs in one engine.
  - Tests can use an in-memory engine.
  - PostgreSQL roles and grants, the provider fork and DataFusion transport are retired.
- **Costs:**
  - **Silent failure modes** that conflict with CI-04:
    - silent bound truncation, and null for both unreachable and beyond-the-bound;
    - `INSERT IGNORE` dropping rows;
    - permission-blocked writes returning OK;
    - `query()` reporting success when a statement failed.

    The load-time digest check catches the load-side cases. The query side needs a generated,
    disciplined query layer.
  - **Immature, heavy ingest.** There is no bulk path; the facts layer peaked at 34 GiB. Open engine
    defects include a freeze after large deletes, which retirement performs.
  - **No joins.** Cross-version diffs and ad-hoc inspection move to the client or to DataFusion
    over exports.
  - **Maturity and size.** The core is not SemVer, upgrades are one-way, and the build gains about
    50 crates.
- **Unchanged:** the bulk of the bespoke code and of the time. Both sit in the compiler and in the
  pipeline's store round trips (§2.5, §2.7), not in the query language.

**Judgment: pivot the architecture now, not the store.** The gains sought are architectural and
store-independent:
- connected-not-coupled decoupling (F12 part 1);
- the typed graph compiled in memory as the primary artifact;
- validate once, persist once, verify by digest (R3);
- a graph-shaped persisted form: node kinds, and edge kinds with identity, with attribution and
  lifecycle as edge or node properties or typed outcome kinds.

A SurrealDB pivot would need all of this first. It is also what removes the code and the failure
modes behind the current system's brittleness (F01–F03, F08, F09). Once the seam exists, the
database is a replaceable lowering.

SurrealDB fits the serving shape better. PostgreSQL wins on what this project weighs most: loud
failures, a mature bulk path, joins for inspection and diffs, and fewer moving parts. Design the
F12 lowering so the persisted form is a typed graph, lower it to PostgreSQL first, and keep
SurrealDB as the **leading alternative lowering**. Switching then becomes a contained swap, not a
migration.

**Where this differs from the reviewer's wording.**
1. "Not adopted in any role" becomes **"not now; leading alternative lowering"**. After decoupling
   a switch is cheap, so a standing rejection overstates the case.
2. **Immutability.** Read-only credentials plus a loader protocol plus post-load digest
   verification is adequate for a single-operator system rebuilt from pinned inputs. ADR-0126
   already scopes privileged tampering out of routine checks. It is not a deciding factor, so the
   reviewer's reopening condition (d), a store-enforced immutability primitive, is dropped.
3. **Switch triggers**, any one of:
   - (a) serving grows into open-ended or agent-authored graph exploration;
   - (b) set-based PostgreSQL serving paths cannot meet the deadline;
   - (c) SurrealDB closes its silent-failure and bulk-ingest gaps and shows a stable storage-engine
     record;
   - (e) the set-based PostgreSQL serving paths prove awkward in practice.

---

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Maturity and outcome | Design phase after the hard cutover (ADR-0087). Phases 0–4 implemented within recorded scopes; Phase 5 serving implemented, qualification and activation pending. **No real-library upper-frontier compile has ever completed**: the first authorized one failed on 2026-10-05 (F01). This review informs the store choice, the pipeline shape and the snapshot definition. |
| Product outcome | DESIGN §1.1 and §14: a version-pinned API and evidence catalog. An agent finds built-in features, selects public invocation and configuration, inspects original examples and deployment evidence, and follows precise evidence and uncertainty through ten bounded MCP routes (§14.9). |
| Supported scope | Generation lifecycle and publication (§B7, §15.11, §6); validation (§B3, ADR-0126); projections and analytics (§B4, §15.10, §9); serving (§B12, §B13, §15.12, §11, §14.7–§14.10); embedding and vector receipts (§B14). Adjacent consumers: `cpg-core` stage adapters, `lctx_storage`/`lctx_mcp`, the operator runbook, verification families. |
| Exclusions | Extraction semantics (§B1, §B5, §B8) except where identity scope (F05) touches them; behavioural analysis internals except as validation cost; evaluation (CI-G3 unaffected). |
| Expected changes | Binding §3 seeds plus: a second library or version; cross-version "what changed"; a new agent journey; a new analytic; an invariant or model-field change; hybrid BM25+vector retrieval; testing a transformation without a store; substituting the store mechanism. |
| Profile scope | All canonical relations as stored objects (L0–L3); the four program projections and their analytics; derivation explanations; the ten routes and the capability resource. Workloads: extraction (as storage), graph and program analysis, answering, change (new release), evaluation (unaffected). |
| Method and coverage | Read: role contracts; core 3.2; profile 1.3; binding. DESIGN §1.1 and §2 (§B2–§B14); sections §5–§6, §9, §11.4, §14 (§14, §14.3–§14.11), §15 (§15.1–§15.3, §15.10–§15.13); ADR-0085, 0086, 0103, 0105, 0114, 0126 in full; superseded ADR-0083 (`cfd8f2ab`); the graph-database passages of both external reviews; the library-leverage review's N2 row. Evidence lanes A1–A8, A3b, W1, W2 and W3 in full. Source read by this reviewer: `lctx-postgres` `generations/mod.rs:930-1006`, `operation_sections.rs:1085-1165`, `stage_validation.rs:170-265`, `validation_session.rs:455-660`, `evidence_service.rs:540-632`, `connection_options.rs:20-37`; `cpg-core/src/generation_read.rs:600-612` and every `SELECT` in `cpg-core/src`; `lctx-model/build.rs`; `domain/model.rs:735-760`; `domain/memory.rs:1-175, 295-330, 630-665`; `scripts/producer_fingerprint.rs:30-62`; `cpg-extract/src/bundle.rs:28-48`; the attribute and macro sites for supports and lifecycle families; the `docs/pins.md` provider-fork row; the PostgreSQL server log and the compile log around 2026-10-05 10:09 EDT; one read-only `pg_stat_activity` query at 14:09 UTC. No product tests, compiles or store writes. |
| Not examined | `behavior-model.md` and `behavioral-analysis.md` beyond §B5; `selection/*` internals; the Python serving adapter beyond A8; the code-facts plans. |

**Authority applied.** Under target purpose, §B7, §B10, §B12, ADR-0086 and ADR-0105 are evaluated
as candidates for revision, not as constraints. Prior judgments are evidence: ADR-0083 option 4,
§B10, the external reviews and the library-leverage N2 rejection. None assessed SurrealDB, and none
had the present code, the W1 realization or the real-compile failure.

---

## 2. Integrated assessment

The sections below develop the argument in the order the evidence arrived: the decision kinds
(§2.1), the real failure (§2.2), the operator's hypotheses (§2.3) and the operator's follow-up
framings (§2.4–§2.8). The structural diagnosis that ties them together is **§2.6, connected versus
coupled**; §2.7 quantifies its bespoke-code consequences and §2.8 its validation consequences.

### 2.1 Four decisions, and a fifth that dominates them

The brief separates model/representation, store, analytics placement and serving engine. The
evidence adds a fifth: **pipeline shape**, meaning where validation runs and how often content
crosses the store boundary. It settles more than the store does.

1. **Model and representation decisions that hold whatever the store.** These carry most of the
   lasting value: correspondence keys (F06), identity scope (F05), relation digest (F07), one owner
   for reference closure (F02), declared serving paths (F04), and the physical lowering of uniform
   relation families (§2.4). No candidate store supplies any of them.
2. **Pipeline shape.** Today every stage writes to PostgreSQL, the next stage reads back through a
   forked provider, and validation streams stored contents inside store transactions. That shape is
   the common cause of F01 (real-scale failure), F02 and F03 (re-reading and anti-joins), F08 and
   F09 (fork, no store-free reads), and much of F12 (stage grants, private attempt reads, vocabulary
   delta tables). Compile-then-store (R3, §2.5) removes those causes on PostgreSQL as well as on
   any other store.
3. **The store.** SurrealDB lacks the primitives that would retire the immutability choreography:
   no frozen database, no leases, no publication primitive, no Arrow ingest, no write-time
   reference existence, and `OPTION IMPORT` skips `ASSERT`. W1 proved a native realization works on
   synthetic data, and in doing so rebuilt the same categories of bespoke machinery. Under R3 the
   store only bulk-loads, verifies and serves a finished generation; PostgreSQL does that with
   binary COPY, privileges and pgvector already in place.
4. **Where analytics run.** In process. A2 and W3 show the kernels are a small share of code and
   time, carry domain lattices (BDD conditions, Pareto proof costs, typed refusals), and are already
   library-backed where a library fits. GDS fails contracts we hold (§5.3, §8.1).
5. **The serving query engine.** PostgreSQL, with serving paths declared in the model and realized
   set-based (F04), and `explain` corrected (F13). A8 shows the store-replaceable part of serving is
   ~1–1.5k of ~9.5k lines; the rest is semantic assembly that would become a second authority in
   any query language.

### 2.2 The real-library compile failure

The first authorized real-library compile through catalog (FastMCP 4.0.5, behavioural profile,
all analytics) failed on 2026-10-05 after about 90 minutes:
- extraction took about 28 minutes; the attempt then spent about an hour on per-field reference
  anti-joins (`stage_validation.rs:179-247`), tens of seconds each (coordinator's operational
  observation);
- the lifecycle connection (`lctx_migrator`, pid 509537) was terminated at 10:09:04 EDT with
  "terminating connection due to idle-in-transaction timeout";
- the product sets `idle_in_transaction_session_timeout=30s` itself
  (`crates/lctx-postgres/src/connection_options.rs:34`; also `cpg-core/src/generation_read.rs:610`);
- `ValidationSession` streams inputs through the transaction and then calls `check.finish()`
  with the transaction open and idle (`validation_session.rs:457-497`, `:615-651`); the 51
  derivation-replay checks recompute whole stages in `finish()`;
- at 14:09:00 UTC this reviewer observed the connection `idle in transaction`, its last statement
  an ordered full stream of `occurrences` (959k rows), four seconds before termination;
- the failure reported "Unconfirmed infrastructure failure … rollback failed … cleanup
  unconfirmed", leaving the generation staging and interrupted (facts layer, ~16.6M rows).

No guarantee was breached: nothing was published and the outcome is typed. But the design cannot
complete its central workflow at real scale, and fixture qualification could not reveal it,
because fixture `finish()` calls end within 30 s. The cause is store-independent: unbounded
in-process work inside a transaction whose lifetime the product bounds. A SurrealDB design must
make the same choice (validate outside a transaction and rely on immutability, or hold one), so
the cause would move, not disappear. Under compile-then-store it disappears by construction,
because validation runs before any store transaction exists.

### 2.3 The operator's hypotheses, tested

| Hypothesis | Evidence for | Evidence against | Verdict |
|---|---|---|---|
| **Brittle** | 43 commits on lifecycle/validation core files in three weeks (A1 §4); grants, drains, vocabulary deltas and 84 security-barrier views, advisory leases, a forked provider with its own lifecycle; the F01 real failure; a store reset on every model edit (F05) | Zero codec/COPY defects in the window; serving-semantics fixes are domain, not substrate | **Supported.** The cause lies in the store-centric pipeline (§2.5), the immutability protocol (F12) and identity scope (F05). SurrealDB does not remove these causes. |
| **Low-performance** | Validation inferred at ~47–55% of facts wall time (A2 §2.1, uninstrumented); fixture compiles take 75–115 s each, i.e. large fixed per-generation costs; ~1 h of per-field anti-joins on real data; `explain` scans whole relations per request (F13); N+1 hydration with a lease per query block (A8) | No graph kernel is a material time consumer (A2); no measurement exists after the facts frontier | **Unmeasured.** All performance claims here are **Proposed**. The visible costs are pipeline choices that move unchanged to any store (F01–F04, F13; R3). |
| **Far too bespoke compared with SurrealDB** | ~8–10k src lines + a 1.4k-line patch + ~7k test lines of substrate compensation (A5 §3) | SurrealDB lacks the primitives that would retire them; W1 rebuilt leases, publication, manifest, validation pass and GC; A6: net +50 crates | **Partly supported.** The bespoke volume is real. SurrealDB replaces it with similar volume; R3 retires most of it on PostgreSQL (§8.2). |
| **Many operations are graph-shaped** | Serving paths are multi-hop (243 hop sites); `explain` is a premise BFS; access routes, delegation, SCC scheduling | Served paths are fixed-depth and typed (A8 §5); 77% of serving is semantic assembly; recursion is limited to two operations, both needing explicit truncation that a silent `{..N}` cannot give; kernels carry BDD/Pareto lattices (A2 §1). Storage (A7): ≤47 of 878 relations are natural binary edges; 63% are attribution/lifecycle/vocabulary/blob; the four projections draw arcs from five relations through policy, universe and coverage joins; W1's working SurrealDB schema used no `TYPE RELATION` table | **Shape yes, engine no.** The shape calls for declared paths (F04) and in-process kernels, not a graph engine. |
| **Snapshots as graph objects with diffs** | W1 and W2 show content-addressed records, membership and exact diffs work in PostgreSQL and SurrealDB | Sharing ≈ 0 under current identities; M1 membership costs more than full copies for narrow rows; M2 mutates lifecycle columns that published snapshots read; semantic diff needs correspondence keys no store provides | **Sound model direction, store-independent.** The value lies in F05, F06 and F07. |

### 2.4 Typing as graph metadata (operator follow-up)

The hypothesis: typed relations are an alignment mechanism, so typing could become graph metadata
(node and edge kinds with property schemas) and the graph framework would satisfy alignment with
fewer relations and less code. Full analysis: supporting document §7. Conclusions:

- **Graph kinds do not reduce the count.** A SurrealDB kind with a property schema is a SCHEMAFULL
  table; W1 needed 1,203. Only ~5% of relations are binary edges; 63% are records that qualify a
  node or edge (A7).
- **Physical relations can be reduced on any store.** The 75 supports share one 7-field shape and
  the 150 lifecycle tables share one macro; each family can lower to one physical relation with a
  discriminator, and reference sums to (tag, id) because ids are type-discriminated (§15.3). That
  cuts physical objects by roughly a quarter. Authored code barely changes, because these families
  are already generated from one attribute or macro each.
- **The 878 are authority units.** Each is a unit of single-writer authority, receipts, read
  grants, vocabulary prefix and nominal reference target. Merging physically requires authority at
  (relation, discriminator) granularity: new machinery on PostgreSQL's per-table privilege model,
  unnecessary under R3 or segments, where per-unit artifacts cost nothing.
- **Authority must remain one Rust declaration.** The analyses need nominal types. Graph metadata
  cannot express the content-identity recipe, the 231 invariants and 39 publication checks,
  derivations, stage ownership, projection roles or rich sums (W1 traps). Making it the authority
  creates a second authority or degrades to convention (core §G "Everything is generic").
  Store-side typing (SCHEMAFULL, `ASSERT`, PG CHECKs) is fail-fast defence in depth, not authority:
  `record<t>` does not check existence on write and `OPTION IMPORT` skips `ASSERT`.
- **What graph-native records add** is native polymorphic links (`record<a|b|c>`) and reverse
  navigation (`<~`). Neither offsets the reasons for decision 1.

Risks of merging families: weaker compile-time reference typing for merged families (runtime
subtype checks, as for today's 28 subtype references); discriminator dispatch if typed views are
not generated; kinds reinterpreted by consumers if the generic form leaks past the store adapter.
Disposition: part of F12's decision, not a separate finding.

### 2.5 Compile-then-store (operator alternative R3)

**The shape.** The typed declarations remain the authority. Extraction feeds an in-process typed
compilation; multi-step analyses run over in-memory typed rows and hydrated graphs, as the
cumulative compile already does for graphs; the model's validators run once, in process
(`MemoryGeneration`/`StageSink` already exist); the validated generation is persisted once;
publication follows a check that the stored content equals the validated content, instead of
re-running validation as store queries.

**What already exists (Implemented, source-inspected 2026-10-05).** `MemoryGeneration` validates
"the same way" as the store (unique keys, complete and subtype-correct references, ordered content
digest, every invariant in declared input order) and returns a content digest equal to the
PostgreSQL store's (`memory.rs:1-4`, `:131-143`); it closes vocabulary publication groups with
private deltas, sealed sets and prefix snapshots in memory (`memory.rs:17-24`, `:299+`); it admits
the facts frontier (`validate_facts`). Two gaps: its read side is "an inspection helper for
bounded development inputs" (`memory.rs:109-110`), not a stage-bound, prefix-scoped read (F09); and
its group close reserves about eight times the stored bytes (`memory.rs:313-318`), so it is not yet
memory-frugal.

**The trust rationale.** ADR-0086 and §15.11 require validation of "the sealed stored contents, not
a caller promise"; A5 noted the reason was unstated. The threats that requirement can address, and
whether R3's "validate in memory, then verify stored-content digest equality" preserves coverage:

| Threat | Covered by R3? | Condition |
|---|---|---|
| Write-path codec defect (encode loses or alters a value: -0.0, NaN, Unicode, widths) | **Yes** | The digest is recomputed by reading the stored rows back **through the readers' typed decode path** and must equal the validated digest. A symmetric encode/decode defect is invisible to both today's design and R3, and readers then see exactly what was validated. |
| Partial or duplicated write | **Yes** | Per-relation count and digest equality |
| Concurrent or late writer | **Yes**, if the digest check follows seal | One bulk writer; seal (privilege revoke on PG) before the check; a late row changes the digest |
| Producer nondeterminism between validation and persistence | **Yes** | Validate and persist the same immutable `Arc` batches |
| Producer reading undeclared inputs (hermeticity, CI-10) | **Yes**, if in-memory reads are stage-bound | The replay validators run over declared in-memory inputs; F09's stage-bound read seam is the prerequisite |
| Consumers that bypass typed decode (`lctx query` raw SQL, generated views, the vector artifact) | **Partly** | Generated views and the vector artifact derive from the same codec and keep their own validation (ADR-0114); `lctx query` is inspection, not a served claim |
| Privileged tampering after publication | **No, unchanged** | ADR-0126 already scopes routine reads to legal service operations; `lctx generation audit` remains the explicit check |
| Store-side constraints (FK/CHECK) as defence in depth | **Optional** | Row-local CHECKs still evaluate at COPY; FKs become redundant with the in-memory closure |

So R3 preserves the guarantee if the requirement is restated as: **readers read exactly the
validated content**, established by in-memory validation plus a single stored-content digest check
through the reader decode path. That is an intentional mechanism change, not a weakening, and needs
an ADR amending ADR-0086/§15.11 and ADR-0126's binding text. Today's design already frames each
relation at completion; R3 keeps one such framing and removes the others.

**Generation semantics under R3.**
- *Staging and publication.* The attempt holds no store state until the final load: staging is
  in-process (with optional attempt-local write-once segments), then one bulk load into a staging
  schema or database, seal, digest check, publish. Selection stays separate.
- *Vocabulary epochs.* Realized in memory (`MemoryGeneration` already does). The store sees only
  closed prefixes, so the 302 delta tables, 84 security-barrier views, delta grants and the
  `introduced_epoch` FKs fall away; prefixes become a model-owned read predicate or a per-prefix
  segment.
- *Checkpoints and resumability.* Checkpoints become in-process validation boundaries. No resume
  is promised today (ADR-0094: retry creates a new generation), so nothing is lost. Attempt-local
  segments would permit coarse resume later, behind ADR-0081's trigger.
- *Memory bounds.* The facts layer is ~16.6M rows (more at upper frontiers); the attempt budget is
  64 GiB and the measured facts compile peaked at ~4.1 GB RSS (A2, 2026-09-30). Holding a whole
  generation in memory is **Proposed** and unmeasured; `close_group`'s ~8× reservation would not fit
  as written. The bounded form keeps only declared stage inputs resident and spills completed
  outputs to attempt-local write-once Arrow IPC segments read back by the stage-bound seam. No
  ambient spill is admitted today (ADR-0105), so this needs an attempt-owned directory, quota and
  cleanup contract. ADR-0105 rejected whole-input memory handoffs for lack of "measured rebuild
  cost and a selected incremental consumer"; the real-scale failure and validation cost are now
  the named cause, and PR-3 supplies the measurement.

**With each store.** PostgreSQL: one binary COPY per relation, seal, one framing, publish; stage
grants, `AttemptSession`, private attempt reads, checkpoint FK trials and vocabulary delta tables
are retired, and serving is unchanged. SurrealDB: the bulk load is the weakest point (no Arrow
path; HTTP 413 at ~2 MB; `OPTION IMPORT` skips `ASSERT`; open RocksDB ingest issues) and the
digest check needs an ordered SurrealDB→Arrow decode; nothing in R3 uses its traversal. Published
segments (R2): the persisted generation is the segment set itself, digest-verified by construction;
serving needs an index realization (PR-3).

**Share of the gains.** Of the mechanisms in A5's map, R3 retires or shrinks, on any store:
validation-in-transaction (F01), repeated framing (F03), store-side reference anti-joins and FK
trials (F02), stage-read grants and private attempt sessions, vocabulary delta/view realization,
the provider fork and second driver stack (F08), and container-only stage tests (F09). What remains
store-dependent: the publish primitive, reader pinning and retirement, serving indexes, the vector
artifact and operator SQL. On this reading **most of the hypothesized simplicity, and the removal
of the observed real-scale failure, come from pipeline shape, not from the store**. The
execution-time effect is **Proposed**: R3 removes store round trips, re-reads and per-field
anti-joins, but not the CPU of the 51 replays, which run in memory instead.

### 2.6 Connected versus coupled (operator framing, used as the primary lens)

**The distinction.** *Connected*: every stored element, query, check and analysis derives from the
declarations and is traceable to them. *Coupled*: the declaration's granularity and identity also
dictate physical layout, execution unit, lifecycle and identity scope. §C of the core principles
places "an optimized layout or queryable projection" in "a derived artifact with explicit
dependencies and identity mapping"; FP-02 asks for replaceable implementations behind stable
contracts; DP-14 for thin adapters; DP-17 for module boundaries that follow ownership.

**The four candidate couplings, checked against source.**

| # | Candidate coupling | Verdict | Evidence |
|---|---|---|---|
| 1 | **Physical layout**: one table per relation, with DDL, grants and receipts per relation | **Verified, with a nuance.** The DDL is a generated lowering (connected), but the lowering is fixed one-to-one and the lifecycle protocol *depends on it*: immutability and read scope are realized as per-table privileges, vocabulary prefixes as per-relation delta tables and views, receipts and frames per table. A physical choice carries semantic authority. | `ddl.rs` phases; A5 M1, M4; 878 relations → ~1,086 tables + 126 views (A1) |
| 2 | **Execution unit**: validation, receipts, leases and serving reads per relation | **Verified.** Reference closure runs one SQL statement per referencing field (`stage_validation.rs:179-247`); each relation is framed ≥3 times (A5 M3); serving hydrates one relation per hop, 243 sites, addressing tables directly by `R::NAME` (A8 §4), while only stage and vocabulary reads go through a physical-name mapping (`stages.rs:2030`, `vocabulary.rs:480`); `explain` scans whole relations per depth (F13). The declaration's granularity sets the store round-trip granularity. | Cited source |
| 3 | **Identity scope**: contract and provider identity are source text | **Verified.** The model contract digest is every `.rs` byte of the model plus lockfiles (`build.rs`); provider identity is the whole workspace (`bundle.rs:30-45`). ADR-0124 already defines explicit framed declaration encodings, but the store contract does not use them. | F05 |
| 4 | **Graph realization**: projections hydrated from relational rows and persisted as Postcard bytes | **Refuted.** Projections are declared (roles, universe, policy, parallel-arc policy); the snapshot is a derived computational artifact with an explicit wrapper identity (input, context, projection and format version, petgraph version), domain ids only, dense indices private, validated against canonical rows (`projection/snapshot.rs:1-30`, §15.10, ADR-0103). Arc computation is policy code behind a contract, which is legitimate. This is exactly §C's derived artifact: **connected, not coupled**. | Source and §15.10 |

So three of four couplings hold, and the fourth shows the decoupled form already works in this
codebase. The couplings explain most findings: F02, F03, F04, F08 and F13 are execution-unit
coupling; F05 is identity coupling; F01 and F12's choreography are layout-and-lifecycle coupling.

**The decoupled target.** The declarations remain the sole authority. Representation, execution
granularity, storage layout and identity scope become declared, replaceable lowerings with explicit
identity mapping. Model-boundary changes this requires, all **store-independent**:
1. **A declared lowering plan.** `ValidatedModel` → physical plan: logical relation → physical
   unit(s), discriminator columns for merged families (§2.4), naming, and the identity mapping back.
   Every store adapter, serving included, addresses data through the plan, never through `R::NAME`.
   The seam already exists for stage and vocabulary reads; it is missing for serving and DDL.
2. **Authority units declared logically.** Writer, receipt, read scope and prefix are already
   declared in the stage table and frontier descriptors; the lowering realizes them (per-table
   privileges on PostgreSQL, per-partition checks if families merge, per-segment files under R2/R3)
   instead of the protocol assuming one table per unit.
3. **Execution plans derived from declarations.** One model-owned reference-closure operation
   (F02); validation passes that share ordered streams across invariants (partly present, ADR-0126);
   declared served paths with a set-based executor (F04); premise lookup by conclusion field (F13).
4. **Contract identity from declarations.** Store contract keyed on the framed declaration and
   validation-definition encodings; provider identity on its crate closure (F05).
5. **A store-free realization of every read and write seam** (F09), which also proves the seam.

**Effect on the store candidates.** Without decoupling, a store change must rewrite the coupled
surface: ~9.5k serving lines interleaved with hops, stage reads through the fork, validation
execution and the lifecycle protocol. With it, a store change is a new lowering plus a small set of
effect operations (bulk load, typed frame read, declared-path read, publish, pin and retire), and
the semantic owners stay untouched (Proposed). Decoupling lowers cost and risk for every candidate
alike; it does not make SurrealDB more attractive on the merits in §2.1, but it would make a later
trial cheap and reversible if a §9 trigger fires. Adopting any new store **before** decoupling would
re-couple the model to that store. That is why decoupling is ordered first.

**Guard against over-generalization.** The lowering seam is justified by credible, present
variation: two realizations exist (PostgreSQL and `MemoryGeneration`), and a third is under decision
(R3/R2). It must stay a thin mechanical plan (DP-14), not a provider framework or a generic record
model (DP-16; core §G "Everything is generic"). Typed access remains nominal `Record` types.

### 2.7 Bespoke code removed and added, per component (operator framing)

Less bespoke mechanism means fewer defect sites (DP-13, DP-16, G8, core §C), and today's real
failure arose in bespoke transaction choreography, not in domain logic. The ledger below separates
removal through **[D]** model decoupling (§2.6, F02–F05, F08–F11, F13), **[P]** pipeline shape
(R3, compile then store) and **[S]** the store. Current figures are source lines from A1, A5, A8 and
this reviewer's `wc` (file lines where src-only counts were unavailable). **Every add/remove figure
is a Proposed estimate**; W1's working probe (≈500 lines of Python for schema generation,
snapshot, lease, publish, abort and GC at synthetic scale, without budgets, typed failures,
epochs, receipts or concurrency) is the only concrete datum for SurrealDB-side code, and a
production equivalent would be several times larger.

| Component | Current bespoke | R1: decoupling + corrections on PG | R3: compile then store on PG | R2: published segments | SurrealDB primary | Neo4j layer |
|---|---|---|---|---|---|---|
| Storage and lifecycle (lifecycle, receipts, lease, locks, failure, catalog, install, verify, vocabulary, roles/bootstrap/connection, DDL, physical columns, codec, control SQL) | ≈5.5k | [D] −0.4k vocabulary delta/view realization if predicate-based; [D] −0.2k `lctx_ops`; [D] +0.2–0.5k lowering plan | [P] a further −1.5–2.5k (stage completions, attempt sessions, checkpoint trials, delta realization, per-stage drains); +0.3–0.6k bounded-memory/spill contract | [S] −4–5k; +0.5–1k manifest, pin registry, GC | [S] −5.5k; +3–5k Rust equivalents of W1 (schema generator, snapshot lifecycle, leases, publish/abort, GC, drift check) | +0 |
| Validation execution (store side: validation session, stage validation, publication validation, views, audit) | ≈2.0k src (+1.2k inline tests) | [D] −0.3k duplicate closure realizations (F02) | [P] −1.5–2k; +0.5–1k stage-bound in-memory reads (F09) | as R3 | [S] −2k; +1.5–2k SurrealDB ordered reads, Arrow codec, generated closure pass | +0 |
| Model validators (`validation.rs`, 119 checks, `memory.rs`) | ≈1.8k + check impls (domain) | Unchanged | Unchanged (run in process) | Unchanged | Unchanged | Unchanged |
| Compile-time read path (provider adapter, runtime, query) + fork patch | ≈1.6k + 1.4k patch | [D] −1.6k −1.4k patch; +0.1–0.2k reuse of typed frames (F08) | as R1 | as R1 | as R1 | — |
| Serving hydration (A8: 853 hop + 100 plumbing + 523 charging + 691 closure + 7,331 semantic) | ≈9.5k | [D] −0.5–1k via declared paths; 7.3k semantic assembly relocated to the model (net 0) | as R1 | +1–2k index realization | [S] −1–1.5k hops; +1–1.5k SurrealQL paths with truncation and edge-identity wrappers | — |
| Graph realization and persistence (snapshot, hydration) | ≈0.6k (normalization 1.2k is domain) | Unchanged | Graphs retained in memory across stages; persistence unchanged | as R3 | Net ≈0 (kernels still hydrate petgraph) | +1–2k export, re-keying, import |
| Vectors and cache | ≈0.85k | Unchanged | Unchanged | −0.85k; +0.2k file cache | −0.85k; +0.3–0.5k | — |
| Test infrastructure (harness 1k; substrate-protocol tests ≈6.5k; scripts ≈1.2k) | ≈8.7k | [D] store-free stage tests (F09); unselected targets resolved (F11) | [P] −3–4k substrate-protocol tests; +1k in-memory/spill tests | −6k; +1k GC/pin tests | ≈ rewrite of 6.5k as conflict-retry and engine tests | + container |
| Operations (services, scripts, runbook) | ≈0.85k + 175-line runbook | [D] −0.2k | Smaller runbook | −0.6k scripts | Different runbook of similar size | + JVM operations |
| **Net (Proposed)** | — | ≈ −3 to −4.5k src and the 1.4k patch; ~40 fewer crates | a further ≈ −4 to −6k src and −2 to −3k test lines | largest removal, but serving indexes unqualified | ≈ neutral (±3k), +50 crates, defect-prone choreography moved, not removed | additive |

**Reading.** Decoupling and pipeline shape together remove most of the bespoke store mechanism,
including the code where the real failure arose, without changing the database. SurrealDB removes
PostgreSQL-specific code but needs comparable bespoke code to restore the same guarantees. The
domain code (model ≈124k, serving semantic assembly ≈7.3k) is unchanged by every candidate.

### 2.8 Controlled inputs and certificate checking instead of replay (operator framing)

**The hypothesis.** With hermetic, content-addressed inputs (CI-10) and outputs that carry a
trace to their premises (221 derivation-declared relations), outputs can be validated by checking
certificates instead of re-running producers.

**What replay establishes today.** The 51 D-class checks recompute a stage with the production
function and require exact equality (W2; e.g. `self.output.matches(&normalize(&self.data, …))`).
That establishes (a) outputs are consistent with the stored declared inputs, (b) determinism,
(c) **completeness**: nothing the function would produce is missing, including coverage,
availability, refusal and residual rows, and (d) canonical choices (ordering, tie-breaks, ids). It
does **not** independently challenge the production logic: a shared mistake in `normalize` agrees
with itself. That is core §G's "the outputs match" false positive and the "deriving all tests from
production logic" failure mode (DP-23).

**Which checks admit certificates** (static reading of the classification and file roles; Proposed):

| Group (of the 51 D and 11 G) | Certificate | Emit cost | Check cost | Covers | Does not cover |
|---|---|---|---|---|---|
| SCC schedule; derivation acyclicity (G) | SCC partition + topological order | Already stored | O(V+E) | Soundness of the partition and order | Canonical tie-break choice (needs a deterministic recomputation of order) |
| Witness paths (structural frames, delegation, access-route witnesses) | The stored witness path | Already stored (ControlPath/ControlStep, witnesses) | O(path length) | Each path is real and within bounds | Completeness of enumeration (access routes enumerate *all* routes to a bound; CI-04) and stop inventories |
| Fixpoints and worklists (summary replay, composition, condition entry/stability) | The fixed point plus witnesses; a post-fixpoint check (one transfer application, no change) | Already stored largely | One pass | Soundness of an over-approximation; checked refutation proofs (already certificate-shaped) | Least-fixpoint precision; budget-dependent residuals and refusals; Pareto completeness of proof costs |
| Ranking (PageRank) | Scores; residual of one iteration | Already stored | O(E) | Convergence to tolerance | Reproducibility across runs (replay or seeded determinism tests) |
| Communities (Leiden) | Partition; quality recomputation | Stored | O(E) | Quality value | Seeded reproducibility, which is the governed contract (CI-09) |
| Exact kNN | None cheaper than recomputation | — | O(n²·d) | — | — |
| FCA concepts | Closure of each concept | Stored | Per concept | Each concept is closed | Completeness of the lattice |
| Total functional derivations (7 normalization stages, catalog and evidence builds, selection, synthesis ×8, retrieval ×2, embedding ×2, execution records and evaluation, local semantics/theory/fields, models) | Trace to premises (derivation rows) + key coverage | Mostly stored | Linear with indexes | Premises exist (trace closure, R-class); one output per input key where outputs are total (K-class) | **Value correctness**: checking a binding or a normalized entity means re-running that row's function, i.e. *partitioned replay*, not a cheaper certificate |
| G profile/count checks (8) | Counts | Stored | Trivial | Already cheap | — |
| G no-unreferenced / orphan sweeps | Reference counts or reverse index | Maintained metadata | Linear | Global closure | — |

So roughly a dozen of the 62 D/G checks admit a genuinely cheaper certificate (search, path,
fixpoint, ranking, acyclicity, closure); most D checks are total functions whose "certificate" is
partitioned replay plus trace and key-coverage checks. A graph store adds nothing essential here:
the derivation index already makes traces first-class relations; the checkers are per-rule Rust
either way; an in-memory typed graph (R3) makes trace closure a hash lookup.

**Independence.** Certificate checkers are *more* independent than replay for soundness, because
they are different algorithms checking a property (DP-23). They are *weaker* for completeness
claims: a certificate shows that what is present is valid, not that nothing is missing (CI-04,
CI-08). Completeness needs key coverage where outputs are total, or replay where the claim is
enumeration (access routes, residual inventories, selection domain closure).

**What follows, combined with R3.** Under R3, publication replay re-runs the same function on the
same in-memory inputs in the same binary. Its remaining value is the determinism check, plus a
hermeticity check that stage-bound declared reads (F09) can enforce by construction. Replaying all
51 stages at every publication then roughly doubles stage compute for a check that a qualification
or audit run can perform (Proposed). This is F14.

---

## 3. Responsibilities, dependencies and semantic ownership (slot 2)

### 3.1 Component map

| Component | Responsibility and hidden decisions | Consumer contract | Dependencies / direction | Expected reason for change |
|---|---|---|---|---|
| `lctx-model` (~124k src) | Typed declarations, identities, codebooks, invariants (≈160 named, 119 checks), stage table, frontier descriptors, pure kernels, serving mappings/cursors/ranking, `MemoryGeneration` | `ValidatedModel`; `StageSink`; typed `Rows`; Arrow lowering | arrow, serde_arrow, petgraph, leiden-rs, biodivine; **no SQL dependency** | New domain concept, invariant, analytic, product predicate |
| `lctx-postgres` (21.5k src) | Generation lifecycle, grants, leases, COPY, DDL lowering, receipts and validation execution, vocabulary epochs, **and ~9.5k lines of serving services** (A8) | `GenerationStore`, `GenerationAttempt`, `GenerationLease::read_for`, serving services | sqlx, pgpq, sea-query, pgvector; `lctx-model` | Store mechanism, lifecycle protocol, **and every serving packet change** (F04) |
| `cpg-core` | Stage orchestration; stage reads via DataFusion over the owned PG provider fork; analysis graph hydration | Stage adapters | DataFusion 55.1 + fork (tokio-postgres, native-tls/OpenSSL), `lctx-postgres`, `lctx-model` | Stage schedule and adapters; currently also store substitution (F08, F09) |
| `lctx-analytics` | Native schedule, ranking, concepts and neighbours kernels | Four entry points | petgraph, fixedbitset | New analytic |
| `lctx_storage` (pyo3) / `lctx_mcp` | Thin transport; numerical BM25S; presentation | Ten routes, one resource | `lctx-postgres` | Wire and presentation |

Dependency direction is sound at the model boundary: `lctx-model` has no store dependency and has
a store-free write/validate path. Two boundaries leak:
- **Serving semantics live in the effect crate** (F04). A8 classifies 77% of the ~9.5k serving
  lines as semantic assembly: sum-type dispatch at ~9 points, reconstruction equality checks,
  closure checks returning Contract, availability derivation, budget charging. §15.1 assigns
  serving mappings, cursors and ranking to `lctx-model::domain::serving` and only generation-bound
  effects to `lctx-postgres`; the implementation places packet assembly with the effects.
- **Stage reads exist only over PostgreSQL through the provider fork** (F08, F09). The write side
  has two realizations (`GenerationAttempt`, `MemoryGeneration`); the read side has one.

### 3.2 Semantic ownership (store-relevant)

| Concept or operation | Authority and identity | Revision boundary | Realizations and consumers |
|---|---|---|---|
| Record identity | `#[model(key)]` → BLAKE3 typed key with nominal type discriminator (§15.3) | Model namespace version | Every relation; diff, sharing and correspondence depend on it (F05, F06) |
| Provider/run identity | `Provider{tool, revision, build_digest}`; `build_digest` = workspace source digest + lockfile + patches (`bundle.rs:30-45`) | Any workspace byte | Keys every support, observation and assertion; sharing ≈ 0 across binaries (F05) |
| Model contract identity | `owned-semantics` = every `.rs` byte of `lctx-model` and macros + manifests + `Cargo.lock` (`build.rs`) | Any model byte or lock move | `store install` refusal; reader digest checks; reset on every edit (F05) |
| Reference closure | `field.target()` / `subtype()` declarations | Model | **Four realizations**: generated FKs (trial at checkpoints, permanent at validated), stage-read anti-join SQL (`stage_validation.rs:179-247`), audit anti-join (`audit.rs:109-137`), `MemoryGeneration::references` (`memory.rs:637-660`); not in the model validator set (F02) |
| Relation content digest | `RelationContent`/`hash_rows`, sequential over strictly increasing ids (`model.rs:738-756`) | Model | Receipts, checkpoint reuse, `explain` re-hash, audit (F07, F13) |
| Snapshot immutability | ADR-0086 privilege + ADR-0126 trust "under legal service operations" | Lifecycle protocol | Grants and revokes, drains, vocabulary deltas and views, leases (F12) |
| Validation definitions | ADR-0126 stable IDs/revisions with ordered premises | Model identity | `ValidationSession` executes inside the lifecycle transaction (F01) |
| Served path | **No declared owner**; packets hand-chain `read_for` hops; `PacketBinding::permits` declares read scope per packet kind | Code | 243 hop sites, ~170 single-key (A8; F04) |
| Derivation explanation | Model derivation declarations (~137 rules); §15.12 "source receipts are verified" | Model | `explain` re-scans and re-hashes whole relations per depth (F13) |
| Cross-version correspondence | **None declared** | — | Needed by the new-release journey (F06) |

### 3.3 Fact and fidelity table (profile addition, scoped to storage-relevant families)

| Fact family or relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| Source artifacts, chunks, occurrences | Acquisition; Ruff (canonical syntax) | Extracted | Required families; `FamilyCoverage` per provider | (input revision, path, content hash); occurrence by span/kind/role/path | Everything. Source identity blocks cross-version correspondence (F06) |
| Provider observations and supports (75 supports share one shape, A7) | Ruff 0.16.10, Pyrefly 1.4.0-dev.3, ty 0.0.16 (pins) | Extracted / provider-resolved | Admission requires exact scope × family × provider rows | Run → provider → workspace build digest (F05) | Normalization; serving `claim_basis` chains |
| Normalized entities, call events, alternatives, admissions | Model normalization | Derived; unresolved retained | Normalized availability writer | Source entities keyed by declaration occurrence | Projections; relationships section |
| Program projections and snapshots | Model projection runtime (petgraph, Postcard) | Derived; isolates, typed parallel arcs, gaps | `projection_source_assessments` | `ArcId` lineage; dense indices private | Structural, analytics, SCC schedule |
| Analytics (rank, communities, concepts, neighbours) | Model kernels, leiden-rs | Heuristic, governed (CI-09) | Selected or NotRequested; diagnostics | Definition digest | S0 nomination only, never a claim |
| Derivation steps and proofs | Model derivation declarations (221 relations, A7) | Derived with premises | Bounded explanation | Edge-row ids cited by proofs (A8 §5.3) | `explain`, `claim_basis`, capability packets |
| Catalog, evidence, selection, retrieval units | C0–C2, S0, retrieval | Derived; supported/unresolved/conflicting | Per-requirement coverage | Catalog member ids; public exposure is a relationship | Ten routes |
| Embedding values and consumer receipts | Model embedding spec | Exact consumed bytes | Availability per spec | Spec hash + input hash | Ranking; disposable vector artifact |

No fidelity defect was found at the store boundary: provider attribution, typed fidelity and
unknown-versus-absent survive storage (CI-01, CI-02 and CI-04 pass). The fidelity risks sit in the
candidates (§6).

---

## 4. Contracts: guarantees versus mechanisms (slot 3)

The operator directive asks for guarantees to be separated from the mechanisms that realize them.
This table is the basis for judging every candidate.

| Guarantee (owner) | Current mechanism | SurrealDB primary (W1) | Compile-then-store (R3) on PG | Published segments (R2) |
|---|---|---|---|---|
| Complete, closed, attributable snapshot (§15.11, ADR-0086, CI-13) | Schema per attempt; admission binds model, schedule, coverage and content digests | `snapshot:S` manifest with per-relation count and digest | In-memory admission, then manifest of stored digests | Manifest root with per-relation digests |
| Never visible partially or before validation (§15.11, DP-19) | Staging schema without reader grants | Readers see only `fn::live(spans, $auth.snap)`; pointer flip; W1 `p2.no-partial-visibility` passed (426 polls) | Store holds nothing until the final load; publish after the digest check | Manifest written last |
| Immutable once published (ADR-0086, ADR-0126) | Privilege: no legal service role can write | Content `READONLY`, but the loader rewrites `spans` on shared records that published snapshots read; abort reopens spans; root can redefine fields; reader writes return OK `[]` silently | Privilege, with one bulk writer revoked at seal | Write-once files; digest-verifiable |
| Pinnable by readers (§15.11, CI-13) | Advisory-lock lease checked against digests | Lease record as `$auth`; retirement checked by the application | Unchanged | Pin registry or flock; GC honours pins |
| Reproducible (CI-10, §6.3) | Rebuild from pinned inputs | Same | Same | Same |
| Selection separate from publication | Control pointer | `selection:current` record | Unchanged | Pointer |
| Evidence closure (CI-11) | Reference closure ×4; same-generation receipts; Contract on missing rows | Generated `closure:*` pass via record-link fetches; `record<t>` accepts dangling on write; IMPORT skips ASSERT | One model-owned closure in memory | Model-owned closure |
| Typed fidelity (CI-02), relationship identity (CI-03) | Typed columns; every edge a row; `ArcId` | Typed fields; `+path` drops edge identity and returns walks; record-link modelling keeps identity at two hops per step (A8 §5.3) | Unchanged | Arrow schema |
| Unknown is not absent (CI-04) | Explicit availability; Contract on corruption | `+shortest` returns `null` for unreachable and beyond-bound alike; silent `{..N}` truncation | Unchanged | Unchanged |
| Declared projections, bounded paths (CI-05, CI-08) | Model projection runtime; typed stops | 256-hop cap; silent bound; OOM on unbounded `+path`/`{..}` (W3) | Unchanged | Unchanged |
| Heuristic governance (CI-09, DP-11) | Seeds, canonical labels, recorded settings | No algorithms | Unchanged | Unchanged |
| Pinned, rebuildable serving (CI-13, §B12) | Generated views over the pinned generation; disposable vector artifact | Same database; server process required for several MCP readers | Unchanged | Index realization unqualified (PR-3) |
| Bounded serving (§B13, §14.9) | Budgets, deadline, refusals | Same budgets plus traversal wrappers | Unchanged | Unchanged |
| Embedding spec and consumed-vector receipts (§B14) | Canonical relations + insert-only cache | HNSW approximate; exact = brute force | Unchanged | Cache as files |
| Programmatic synthesis (§B11) | Unaffected | Unaffected | Unaffected | Unaffected |

**Intentional guarantee changes.** SurrealDB primary would change "immutable once published" from
store-enforced to loader-protocol, unless readers verify a membership digest (a scan). R3 changes
"validators inspect stored sealed contents" to "readers read exactly the validated content,
established by a stored-content digest check through the reader decode path" (§2.5). Neither change
may be silent; the second preserves the guarantee's purpose.

**Validation contract (ADR-0126).** Receipts acknowledge complete bindings of immutable frames, and
READ COMMITTED is sufficient "only under enforced unchanged logical frames". The implementation
nevertheless runs checks inside the lifecycle transaction and holds it idle across `finish()`
(F01). The contract already permits validating outside transactions; nothing in ADR-0126 requires
the long transaction.

**Isolated verification.** Write-side model operations are store-free (`MemoryGeneration`);
read-side stage transformations are not (F09), so normalized and higher frontiers need a container
in every test (A5 fact 8).

---

## 5. Composition and execution (slot 4)

### 5.1 Compile pipeline

Stages compose through the typed stage table (§15.11), a sound single-writer authority and a
preservation constraint for any remedy. Two composition problems are store-independent:
- **Validation placement.** Each relation is read at stage completion (framing), at checkpoints and
  at final validation (A5 M3), plus per-field anti-joins before each stage read; the dominant phase
  is uninstrumented (`ValidationStats` counted, never emitted). F01, F02, F03.
- **Fixed per-generation cost.** ~1,100 tables, 1,061 indexes, 126 views and 6–7k CHECKs per
  generation (A1 §2; W2 real staging: 7,272 CHECK, 878 PK). Fixture compiles take 75–115 s each
  (A2 §2.4, under load). This follows from the per-schema realization with a one-to-one logical to
  physical lowering (F12, §2.4).

### 5.2 Serving

A8's two-tier finding is decisive for the serving-engine question.
- **Tier P** prepares ~95 classification relations, ~28 retrieval relations and native inventories
  once per process. Four of the ten routes, and core identity resolution, never hop.
- **Tier R** hydrates per request through 243 `read_for` sites, ~170 of them single-key, and each
  `e.query` block takes a fresh lease (lease transaction, advisory lock, timeout setting).

The one-hop pattern comes from per-relation typing, scope checks and charging, not from a missing
join capability; W1's PostgreSQL references (`journeys.py:66-91`) compute three journeys as single
CTE statements. `explain` is the one true recursive expansion; it scans and re-hashes whole
derivation relations per depth (`evidence_service.rs:557-627`), contradicting ADR-0126's
routine-read trust model and unlikely to fit the 30 s deadline on large derivation relations (F13).

### 5.3 Analysis records for graph analytics under each engine (profile addition)

| Analytic | Projection | Method and settings | Exactness | Budgets / partial | Output linkage | GDS (W3, synthetic) | SurrealDB 3.3 |
|---|---|---|---|---|---|---|---|
| SCC schedule | CallableInvocation; universe `InputEntitiesAndContextTargets`; typed parallel arcs | `kosaraju_scc` + canonical callee-first order | Exact | Charged; typed refusal | SCC rows with lineage | Partition equal, deterministic; no condensation; `topologicalSort` drops cyclic regions | Per-node reachability intersection only; 256 cap; O(V·(V+E)) |
| Summary worklist | SCC-local | BDD conditions, Pareto proof costs | Conservative under the stated model | Depth and proof limits typed | Witness DAG | None (Pregel is a Java plugin) | None |
| Delegation traversal | Invocation ∪ Definition views | Bounded BFS with parent witness | Exact within bound | Typed stops (arcs, vertices, depth) | Witness paths | `bfs`/`dfs` `maxDepth`; budgets not a concept | `+collect`/`+shortest` correct up to the start-node definition; silent bound; unreachable ≡ beyond bound |
| PageRank | Directed count weights | d 0.85, tol 1e-10, 100 iterations, dangling redistribution, residual | Heuristic | Work bound | Rank scores, diagnostics | Unnormalized; no dangling redistribution; no residual; rank order equal after rescaling | None |
| Communities | Undirected conversion | leiden-rs RBER, 4 resolutions × 10 seeds | Heuristic | Quality history | Canonical membership | Modularity, not RBER; seed honoured only at concurrency 1 with a stable load order; Louvain unseeded | None |
| kNN neighbours | Embedding vectors | Exact cosine, top 3, floor 0.5, canonical ties | Exact | Charged | Neighbour rows | **Approximate even at sampleRate 1.0**: 71 of 300 nodes wrong; non-canonical ties | HNSW approximate; brute force exact |
| FCA/RCA | Attributes | NextClosure | Exact | Charged | Concepts | None | None |
| Derivation explanation (serving) | Derivation premises (~137 rules) | BFS depth ≤ 8, nodes ≤ 256, edges ≤ 1,024 | Exact within bound | Explicit `truncated` | Edge-row ids cited | n/a | `{1..8}` over heterogeneous tables; receipt check infeasible in-query |

---

## 5A. Change and failure scenarios (slot 5)

Kind of change follows core §E. "Decoupled" means F12 part 1 plus F04/F05/F09 and R3. Observed
routes are source-traced; proposed routes are Proposed.

| Scenario and kind | Owner | Contract change | Baseline propagation (observed) | Decoupled + R3 (proposed) | SurrealDB primary (proposed) | Evidence |
|---|---|---|---|---|---|---|
| Add a second analyzed library or version (instance) | Acquisition; generation | None | A new pinned input and a full compile; currently blocked at real scale (F01) | Same, without the store-transaction exposure | Same; bulk ingest is the weak point (A3 §5) | §15.11; F01 |
| Cross-version "what changed in this API" (domain concept; profile journey "new release") | `lctx-model` identity and a comparison operation | New correspondence relation; cross-generation consumption decision | Not expressible: identity diff only (F06) | Correspondence key + keyed join over two pinned generations | Same model work; no JOIN, so client merge | W2 probe 2; supporting doc §5.1 |
| New agent journey (composition) | Serving packet | New typed output binding | Hand-chain `read_for` hops and closure checks inside `lctx-postgres` (F04) | One path declaration + one pure assembly function in `lctx-model` | SurrealQL path with truncation and edge-identity wrappers; the same assembly in Rust | A8 |
| Add an analytic (composition; binding seed) | Analytic owner | Declared projection, policy, kernel | Local: model kernel + policy + NotRequested default | Same | Same (no in-database algorithms) | §9, §15.10 |
| Change an invariant or model field (policy or concept; binding seed) | `lctx-model` declaration and validation definition | Declaration and snapshot migration | Any model byte changes the contract digest → operator store reset and full rebuild (F05); DDL regenerates mechanically | Reset only when the declaration encoding changes; validation runs in process | Same reset coupling unless F05 is fixed; SurrealQL DDL regenerated | `build.rs`; A5 fact 4 |
| Hybrid BM25 + vector retrieval (policy/mechanism) | Ranking owner (ADR-0114) | None if the ranking owner is unchanged | Already hybrid: bm25s, exact pgvector, Rust RRF | Same; pg_search if BM25 moves into SQL | FULLTEXT + HNSW + `search::rrf` in one query, but fusion would split the ranking owner; exact vectors need brute force | ADR-0114; A3 §6 |
| Test a transformation without a store (binding seed) | Stage adapters | Read seam | Normalized and higher stages need a container (F09) | Store-free by construction | Embedded engine on a tempdir, still a store; `mem` diverges from production engines | A5 fact 8 |
| Substitute the store mechanism (mechanism substitution) | Lowering and effect adapters | None to semantics | Rewrite of the coupled surface: ~9.5k serving lines, stage reads, validation execution, lifecycle (A1 violated) | A new lowering + bulk load, typed frame read, declared-path read, publish, pin, retire | Without decoupling, re-couples the model to SurrealDB | §2.6, §2.7 |
| Real-scale validation exceeds a lifetime bound (failure) | Validation session | None | Observed: idle-in-transaction termination, unconfirmed cleanup, interrupted generation (F01) | Not reachable: no store transaction during validation | Same protocol choice required | §2.2 |
| Trace a served claim (profile journey) | `explain`, `claim_basis` | None | Works on fixtures; whole-relation re-hash per depth (F13) | Indexed premise lookup; receipts trusted | `+path` drops edge identity; record-link modelling needs two hops per step | A8 §5.3 |
| A module full of unresolved references (profile journey) | Normalization, availability | None | Explicit unresolved records and availability; no absence inference | Unchanged | Unchanged in storage; `+shortest` `null` conflates unreachable with beyond-bound if used | §15.4; W3 |

---

## 6. Correctness and fidelity gates (slot 6)

Verdicts are for the **current baseline** within its recorded scope. The last column records what
each candidate would do to the gate, so that a candidate's failure is not read as a baseline result.

| Gate | Baseline verdict | Own evidence or scope reason | Candidate effect |
|---|---|---|---|
| G1 Authority | **pass** | Reference-closure realizations all derive from `field.target()`; no independently editable definition (repeated enforcement, F02). Serving semantics are not restated in Python (A8). | Graph metadata as authority would create a second authority (§2.4). A SurrealDB projection needs an equivalence proof against canonical content (CI-13). |
| G2 Semantic fidelity | **pass** | Typed rows, sum arms and availability states preserved; refusals typed | SurrealDB traversal: `+path` drops edge identity; `null` for unreachable and beyond-bound alike (fails if used natively) |
| G3 Validity | **pass** | Admission and validators refuse invalid states before publication; the F01 failure refused publication | SurrealDB: `record<t>` accepts dangling references on write; IMPORT skips ASSERT (a post-load pass is mandatory, as in W1) |
| G4 Hidden behaviour | **pass** | Read-only serving; explicit audit | SurrealDB: reader writes return OK `[]`; `CONTEXT` re-evaluated per HTTP authentication changed a pinned view (W1 run 1); VERSION + index returns `[]` silently |
| G5 Consistency and recovery | **pass** (safety) | The F01 failure was explicit and typed and published nothing; liveness is a DP-19/DP-20 violation, not a G5 failure | SurrealDB: silent `{..N}` truncation; M2 lifecycle mutation of shared rows; RocksDB freeze after large deletes (#7576) at retirement |
| G6 Transformation and reuse | **pass** | Projections preserve universe and multiplicity; checkpoint reuse bound to exact frames | GDS kNN approximate; Leiden nondeterministic at concurrency > 1 (fail). R3: passes only if the digest check uses the reader decode path |
| G7 Truthful capability claims | **pass** | §15 and STATUS state real-library pilots as `not_run`; no claim of real-scale completion | — |
| G8 Library leverage | **pass** | Bespoke kernels have stated reasons (§15.11 CI-07 rationale; PageRank diagnostics); F08 is surplus machinery, not missed leverage | — |
| CI-G1 Fidelity | **pass** | Heuristics nominate only (§B11); unknown ≠ absent in serving (A8 §3.7) | GDS determinism; SurrealDB truncation and unreachable conflation |
| CI-G2 Evidence closure | **pass** (fixture scope) | Same-generation closure, Contract on missing rows; real-library serving `blocked` (no published generation) | Projection: served rows must be proven present in the canonical generation |
| CI-G3 Evaluation integrity | **n.a.** | No change to evaluation inputs | — |

---

## 7. Findings (slot 7)

### 7.1 Finding index

| ID | Finding | Principles · judgment | Priority | Store-dependent? |
|---|---|---|---|---|
| [F01](#f01) | Validation holds the lifecycle transaction idle across unbounded `finish()` work under a product-set 30 s idle cap; the first real-library upper-frontier compile failed | DP-19, DP-20, FP-05, §6.5 · A1 | 1 | No |
| [F02](#f02) | Reference closure has four realizations outside the model validator set; per-field anti-joins took ~1 h on real data | DP-01, DP-03, DP-16 · A1 | 2 | No |
| [F03](#f03) | Stored-content validation re-reads relations ≥3 times and is uninstrumented | DP-21, DP-22, FP-06 | 2 | No |
| [F04](#f04) | Serving semantics live in the PostgreSQL effect crate; served paths are hand-chained N+1 hops with a lease per block | FP-01, DP-17, DP-08 · A1, A3 | 2 | No |
| [F05](#f05) | Model-contract and provider identities cover raw workspace bytes; every edit resets the store and re-identifies all attributed facts | DP-04, DP-09, CI-01 · A1 | 2 | No |
| [F06](#f06) | No cross-version correspondence key; the new-release journey cannot be expressed | FP-04, DP-04 · A2 | 3 | No |
| [F07](#f07) | A sequential relation digest prevents O(Δ) digests and relation-level reuse | DP-09 | 4 | No |
| [F08](#f08) | The owned DataFusion PostgreSQL provider fork serves only `SELECT *` scans, adding a second driver/TLS stack and a dual lease realization | DP-14, DP-16 · A3 | 3 | Partly |
| [F09](#f09) | Stage reads have no store-free realization | FP-06, DP-18 · A1 | 2 (R3 prerequisite) | No |
| [F10](#f10) | Historical `lctx_ops` services are retained without a consumer | DP-16 | 4 | No |
| [F11](#f11) | 49 PostgreSQL-dependent test targets are outside every verification family | DP-23 | 3 | No |
| [F12](#f12) | **Primary structural finding.** Declarations are coupled, not only connected, to physical layout, execution unit, lifecycle authority and identity scope; the store-centric pipeline and in-server immutability choreography follow from it. Required: a declared lowering seam, then a decision on pipeline shape and realization | FP-02, FP-03, FP-05, DP-08, DP-14, DP-16, DP-17, DP-19 · A1, A3 | 1 (seam, model-side); 2 (shape decision) | No (seam); partly (realization) |
| [F13](#f13) | `explain` re-scans and re-hashes whole derivation relations per depth per request, contrary to ADR-0126 | DP-01, DP-20, CI-08 | 2 | No |
| [F14](#f14) | Publication re-derives 51 stages with the production functions: not independent of the producer, and under controlled inputs largely redundant; certificate and coverage checks are not used where they apply | DP-23, DP-08, DP-11, CI-04 | 3 | No |

**Observations, not actionable findings.**
- O1. Hand-rolled traversals with linear-scan adjacency (`structural/controls.rs:653-656`,
  `catalog/access_routes.rs:530-700`, `normalized/dispatch.rs:322,348`) are covered by §15.11's
  scoped CI-07 rationale and its reopen trigger.
- O2. Serving index coverage for every `read_for` field is unverified (A8 §6; PR-6).
- O3. GDS or networkx could serve as an optional independent oracle for SCC, WCC and PageRank;
  networkx is the cheaper one.

### 7.2 Findings in detail

<a id="f01"></a>
#### F01 — Validation transaction held across unbounded in-process work

- **Finding.** `ValidationSession` streams invariant inputs through the lifecycle transaction and
  then runs `check.finish()` with the transaction open and idle (`validation_session.rs:457-497`,
  `:615-651`). The 51 replay checks recompute whole stages in `finish()`. The product sets
  `idle_in_transaction_session_timeout=30s` on the same connection (`connection_options.rs:34`).
  Two bounded policies are uncoordinated: an idle cap and unbounded idle work.
- **Consequence.** The first authorized real-library compile through catalog failed on 2026-10-05
  (server log 10:09:04 EDT; `build/review-compile-catalog-behavioral_2026-10-05.log`). Cleanup was
  unconfirmed and the generation is interrupted. Every real-library upper-frontier compile is
  exposed whenever any `finish()` exceeds 30 s. The transaction also holds relation locks
  throughout validation (autovacuum "lock not available" lines precede the failure).
- **Evidence.** Source as cited; operational observation on 2026-10-05, not a test receipt.
  Attributing the idle interval to a specific `finish()` is inferred from the source and the last
  observed statement (an ordered `occurrences` stream); PR-1 settles it.
- **Correction.** Owner: `lctx-postgres` validation session and lifecycle, under ADR-0126.
  - *Within the current shape:* separate the attempt lock (session-level, already held by the
    lifecycle connection) from read transactions. Stream acknowledged immutable frames in bounded
    read statements, run `visit`/`finish` with no transaction open, then acknowledge in a short
    transaction that re-checks the frame bindings (installation, generation, layout, definition,
    prefix). ADR-0126 already states READ COMMITTED suffices under enforced unchanged frames, so no
    guarantee changes.
  - *Under R3:* closes by construction; validation precedes any store transaction.
  - *Rejected:* raising the timeout. That keeps locks across unbounded CPU and only moves the
    ceiling.
- **Challenge case.** A legitimate writer could change a frame between read and acknowledgement only
  if immutability enforcement failed; the acknowledgement re-checks the bindings, and ADR-0126
  already scopes trust to legal service operations.
- **Closure.** A store-family control with a validator whose `finish()` exceeds a lowered idle cap
  passes (and fails on the current code); a real-library compile completes facts-checkpoint
  validation.
- **SurrealDB.** Moves the cause, does not remove it: the same choice between a long transaction and
  validate-then-acknowledge applies.

<a id="f02"></a>
#### F02 — Reference closure has four realizations outside the model validator set

- **Finding.** One rule (non-null id references resolve to an existing, subtype-correct target) has
  four realizations:
  - generated FKs, trial-installed in SAVEPOINTs at checkpoints and vocabulary closes, permanent at
    `validated`;
  - per-field anti-join SQL before stage reads (`stage_validation.rs:179-247`);
  - audit anti-join SQL (`audit.rs:109-137`);
  - `MemoryGeneration::references` (`memory.rs:637-660`).

  The model's validator set does not include the rule; the store supplies it.
- **Consequence.** Changing a reference rule (prefix-aware vocabulary references, or snapshot-scoped
  closure under sharing) requires coordinated edits in three code realizations and the DDL
  lowering; this happened for vocabulary prefixes (A1 §4: d4f0d593). Cost: an anti-join per field;
  ~3,300 reference fields (W1 count) took about an hour on real data before F01. Under sharing, a
  store FK proves only "exists in some snapshot" (W2), so the FK realization would become wrong, not
  merely redundant.
- **Correction.** Make reference closure a model-owned check with a declared definition id
  (ADR-0126 style), executed once per acknowledged frame set. Realizations: the
  `MemoryGeneration` algorithm over typed id sets within the attempt budget (natural under R3), or
  one set-based statement per target relation covering all referencing fields. Keep permanent FKs
  only as cost-free defence at seal; drop the checkpoint trials. Audit calls the same operation.
- **Closure.** One code owner; seeded dangling and wrong-subtype references refused at stage read,
  checkpoint and publication through the same definition id.

<a id="f03"></a>
#### F03 — Validation re-reads relations and is uninstrumented

- **Finding.** Each relation is framed at stage completion, at checkpoints and at final validation
  (A5 M3). `ValidationStats` (row scans, proof hits, check executions) is counted and never emitted
  (A2 §2.6).
- **Consequence.** The phase inferred at ~47–55% of facts wall time cannot be attributed; F01's
  failing interval could not be seen in logs. Performance claims stay Proposed.
- **Correction.** Emit stage-scoped spans and `ValidationStats`. Reuse completion frame receipts at
  checkpoint and final validation where ADR-0126 bindings match. Under R3, framing happens once, at
  the post-load digest check.
- **Closure.** One real facts-layer compile with emitted statistics (PR-2).

<a id="f04"></a>
#### F04 — Serving semantics in the effect crate; hand-chained hydration

- **Finding.** ~9.5k serving lines live in `lctx-postgres::generations`, 77% of them semantic
  assembly (A8 §4): sum-type dispatch at ~9 points, reconstruction equality checks, closure checks,
  availability derivation, budget charging. Served paths are not declared. Each packet hand-chains
  `read_for` hops (243 sites, ~170 single-key N+1 loops), and each `e.query` block re-runs the lease
  protocol (A8 §0.3). `PacketBinding::permits` declares read scope but not paths.
- **Consequence.** Substituting the mechanism (any store, or a set-based executor on PostgreSQL)
  requires rewriting or relocating all serving semantics, because they are interleaved with hop
  effects: A1 is violated for that scenario. A new journey adds another hand chain with its own
  closure checks (A3 unresolved). On real data, N+1 chains with per-block lease acquisition
  multiply round trips under a 30 s deadline (Proposed; unmeasured).
- **Correction.** Two separable obligations:
  1. Move packet assembly into `lctx-model::domain::serving` as pure operations over typed hydrated
     rows, leaving `lctx-postgres` an executor for declared reads (§15.1's stated boundary).
  2. Declare served paths in the model: relation, field, direction, expected cardinality, per-arm
     dispatch for sums, closure checks. Realize each as one set-based read per path per page,
     collecting keys per sum arm rather than per row. The executor keeps per-relation typed decode,
     scope (`permits` over the whole path) and charging. W1's CTE references show the three
     shallow journeys as single statements; the deep, polymorphic shapes (`claim_basis`,
     `resolve_original`) may stay multi-statement but batched.
- **Challenge case.** A closure check that today returns Contract on a count mismatch (outcomes
  versus invocations) must survive set-based reads; declaring expected cardinality per hop keeps it.
  Moving sum dispatch into SQL CASE would create a second authority, so it stays in Rust.
- **Closure.** A traced new packet that adds one declaration and one pure assembly function;
  journeys equal before and after on the fixture generation; a store-free test of packet assembly.

<a id="f05"></a>
#### F05 — Identities cover raw workspace bytes

- **Finding.** The model contract digest captures every `.rs` byte of `lctx-model` and its macros,
  plus manifests and `Cargo.lock` (`crates/lctx-model/build.rs`). `store install` refuses on
  mismatch, so any model edit or `just upgrade` resets the operator store (A5 fact 4; runbook).
  Provider `build_digest` covers the whole workspace source digest, the lockfile and the patches
  (`bundle.rs:30-45`; `producer_fingerprint.rs:37-60`), including `lctx-postgres` and `specs/`.
  Supports are keyed by run → provider, so any edit re-identifies every attributed fact.
- **Consequence.**
  - *Change a model field or invariant:* a comment edit resets the store and forces a full
    real-library rebuild, now known to take hours and to fail (F01).
  - *Second library version, comparison or reuse:* structural sharing is about zero (supporting
    document §2).
  - Fact identity depends on unrelated store code. CI-01 holds for recording, but identity is
    over-scoped.
- **Correction.** Key the store contract on the declaration and validation-definition encodings that
  ADR-0124 already frames. Key provider identity on the provider's crate closure, analyzer patches
  and that closure's lockfile entries. Keep the conservative fingerprint as recorded run context
  (CI-10). Remedy maturity: Proposed; needs an ADR amending §15.3.
- **Challenge case.** A codec edit inside the provider closure must still change provider ids; a
  `lctx-postgres` edit must not.
- **Closure.** Those two revealing controls pass; a comment edit no longer refuses install.

<a id="f06"></a>
#### F06 — No cross-version correspondence key

- **Finding.** `Source` callable, class and parameter entities are keyed by declaration occurrence,
  whose key includes the source content hash and span. `SourceArtifact` includes the input revision;
  `Signature` includes its parameters digest. No declared key corresponds a public API across
  releases (W2 probe 2, static). Module `qualified_name` collides for `.py`/`.pyi` pairs.
- **Consequence.** The profile's "new release of the analysed code" workload and the brief's "what
  changed in this API" capability cannot be expressed. An identity diff reports a changed parameter
  list as a removed and an added signature, and an edited file as wholesale replacement. This is a
  model-adequacy gap (A2) in target scope, independent of the store.
- **Correction.** Supporting document §5.1: a derived correspondence relation keyed by public access
  path, kind and stub flag, with explicit many-to-many ambiguity. A comparison over two pinned
  generations needs an ADR on cross-generation consumption (ADR-0105's linked-generation clause;
  CI-13's single-generation process).
- **Closure.** Real-data uniqueness check (PR-4); a fixture pair of releases with a changed default,
  an added parameter, a rename and an overload, classified correctly.

<a id="f07"></a>
#### F07 — Sequential relation digest

`hash_rows` (`model.rs:738-756`) hashes (id, payload digest) in strictly increasing id order, so any
change rehashes the whole relation. A Merkle tree over sorted ids (preferred) or a multiset hash
gives O(Δ) update and cross-generation relation equality. Low priority: implement with the first
consumer (F06 comparisons or relation-level receipt reuse). Supporting document §5.3.

<a id="f08"></a>
#### F08 — The provider fork is a scan transport

- **Finding.** All 19 compile-time DataFusion queries are `SELECT * FROM "<relation>"` (one with
  `ORDER BY id`); this reviewer re-checked every `SELECT` in `cpg-core/src`. They run over an owned
  fork of `datafusion-table-providers` (a 1,398-line patch with a pins row) that brings
  tokio-postgres, bb8, native-tls and OpenSSL, its own `idle_in_transaction_session_timeout=30s`
  (`generation_read.rs:610`), and the driver-neutral `LeaseDriver` realization. `lctx-postgres`
  already streams typed Arrow frames under a lease (`visit_physical`).
- **Consequence.** A second driver/TLS stack and a forked lifecycle provide what an in-house
  primitive already provides, and a store substitution must replace both. A6: removing the
  PostgreSQL/DataFusion/fork stack drops 158 crates, including OpenSSL.
- **Correction.** Read stage inputs through the store's typed frame stream, or in memory under R3.
  Keep DataFusion only where SQL is consumed (`lctx query`, model-owned logical views), over
  MemTables or Arrow files. Retire the fork, tokio-postgres, native-tls/OpenSSL and the dual lease
  realization. Amend the §B3/§15.11 compute text and the pins row.
- **Closure.** `cargo tree -i openssl-sys` is empty for the workspace roots; stage-read controls
  pass through the new route.

<a id="f09"></a>
#### F09 — No store-free stage-read seam

`StageSink` has `GenerationAttempt` and `MemoryGeneration` realizations; stage reads exist only
through the PostgreSQL provider (`generation_read.rs`, `consumed_rows.rs`, `AttemptSession`), and
`MemoryGeneration::read` is an unscoped inspection helper (`memory.rs:109-110`). Normalized and
higher transformations therefore need a container in every test (A5 fact 8), contrary to FP-06 and
the binding's "test a transformation" scenario. Correction: a read-side trait with stage-bound,
prefix-scoped reads and a `MemoryGeneration` realization, sharing F08's typed-frame route. This is
also R3's prerequisite. Closure: a normalized stage test runs with no container.

<a id="f10"></a>
#### F10 — Historical services retained

`OperationsDb::start_attempt` has no production caller; `lctx_ops` attempts and events are empty;
ADR-0094 makes them historical. They keep `operations.rs`, `runs.rs`, two backup-fingerprinted
tables, a test and a runbook section alive. Retire them (ADR-0042 current working set).

<a id="f11"></a>
#### F11 — Controls outside every verification family

A5 fact 7: these targets are not named in `scripts/verify.py`, so `just qualify` never selects them:
- 19 `lctx-postgres` targets: 15 `domain_*` codec round trips, `services`, `unicode_values`,
  `finite_metrics` and `native_inventory`;
- 24 `cpg-core` targets;
- 6 `lctx` targets.

Either their claims belong to a family or the targets are retired; today neither is stated. Owner:
assurance coordinator (ADR-0126).

<a id="f12"></a>
#### F12 — Declarations coupled, not only connected (primary structural finding; required decision)

- **Diagnosis (§2.6).** The declarations govern meaning (connected), but their granularity and
  identity also dictate physical layout (one table, grant set and receipt per relation), execution
  unit (one validation read, one closure query per field, one serving hop per relation) and identity
  scope (source bytes). Graph projections show the decoupled alternative already works. This
  coupling is the structural cause behind F01, F02, F03, F04, F05, F08, F09 and F13, each of which
  keeps its own closure obligation.
- **Correction, part 1 (store-independent, priority 1 on the model side).** A declared lowering plan
  with explicit identity mapping; authority units declared logically and realized by the lowering;
  execution plans derived from declarations; contract identity from declaration encodings; a
  store-free realization of every seam (§2.6, items 1–5). FP-02 justifies the seam by present
  variation (PostgreSQL, `MemoryGeneration`, the R3/R2 decision). It must stay thin (DP-14, DP-16).
- **The realization the coupling produced.** Every stage writes to PostgreSQL and the next reads
  back. "Immutable once published,
  never partially visible, pinnable" is realized by:
  - per-attempt schemas (~1,100 tables, ~1,060 indexes);
  - writer grants revoked at seal, and drains with `ACCESS EXCLUSIVE` locks;
  - private stage-read grants and attempt sessions;
  - vocabulary delta tables merged under 84 security-barrier prefix views, with an
    `introduced_epoch` column and FKs;
  - advisory-lock leases;
  - `store check` shadow lowering, ACL verification and lock-capacity tuning

  (A5 M1, M2, M4, M5).
- **Consequence.** This concentrates the observed lifecycle defects (43 commits on its core files in
  three weeks), a large fixed per-generation cost, and the transaction-lifetime exposure that F01
  instantiates. The scenarios "add a vocabulary relation" and "change an invariant" touch delta,
  view and grant lowering.
- **Why this is not a SurrealDB argument.** SurrealDB lacks the primitives that would retire the
  choreography; W1 rebuilt leases, publication, a validation pass and GC, and weakened published
  immutability.
- **Correction, part 2 (required decision; remedy maturity Proposed).** Once the seam exists,
  choose the pipeline shape and the realization of the published generation:
  - **R1 store-centric, simplified.** Keep per-stage persistence on PostgreSQL. Validate outside
    transactions (F01); one reference-closure owner (F02); vocabulary prefixes as model-owned read
    predicates instead of delta tables and views; fewer lifecycle phases.
  - **R3 compile-then-store (leading).** In-process typed compilation and validation over immutable
    Arrow batches, with attempt-local write-once segments as bounded spill. One bulk load into
    PostgreSQL, seal, a digest check through the reader decode path, publish (§2.5). Stage grants,
    attempt sessions, vocabulary delta tables, checkpoint FK trials, the provider fork and
    container-only stage tests are retired. PostgreSQL keeps serving, pgvector, leases and
    operator SQL.
  - **R2 published segments.** R3 with the published generation kept as write-once segments
    instead of a PostgreSQL load. Open: serving indexes for declared paths, the 144 lookup indexes,
    memory per serving process.

  The physical lowering of uniform families (§2.4) belongs to this decision: costly under R1,
  unnecessary under R2 and R3. ADR-0083 rejected a file-based canonical store because Delta could
  not enforce keys and references and fed a hand-written serving schema; both premises have since
  changed (store-free model validators; model-derived serving). ADR-0105 rejected whole-input
  memory handoffs pending a measured cause, which F01 and PR-3 now supply.
- **Closure.** Part 1: a traced store substitution scenario in which serving, validation and stage
  code address data only through the lowering plan (no `R::NAME` table addressing outside the
  lowering), and `MemoryGeneration` realizes every read and write seam. Part 2: an ADR choosing the
  shape (R1 or R3) and the published realization (PostgreSQL or R2), with the PR-3 result.

<a id="f13"></a>
#### F13 — `explain` re-hashes whole relations in the request path

- **Finding.** At each depth, `explain` fully scans every derivation relation whose conclusion
  target appears in the frontier, re-hashes all rows and compares them with the stored receipt
  (`evidence_service.rs:557-627`). ADR-0126 and §6.2 state that routine reads trust acknowledged
  immutability and do not rehash; explicit `lctx generation audit` owns recomputation. §15.12's
  "source receipts are verified" is met by checking receipt presence and binding.
- **Consequence.** Two owners disagree on the read-path trust model. On real data the route will
  breach the 30 s deadline whenever a large derivation relation is in the frontier (Proposed; real
  serving is blocked). The cost grows with relation size, not with the bounded explanation.
- **Correction.** Resolve premises by indexed lookup on the conclusion field
  (`read_for(conclusion_field, frontier ids)` per derivation relation and depth), verify the receipt
  binding without rehashing, and leave recomputation to audit.
- **Closure.** An `explain` control whose cost is independent of relation size; a damaged-receipt
  control still refuses.

<a id="f14"></a>
#### F14 — Publication replay is neither independent nor, under controlled inputs, necessary

- **Finding.** 51 of 119 checks recompute a whole stage with the production function and require
  exact equality (W2 `raw/classification.tsv`). They establish input consistency, determinism,
  completeness and canonical choices, but share every logic error with the producer (§2.8).
- **Consequence.** Validation cost tracks stage compute (part of the ~50% inferred share and of the
  idle `finish()` behind F01). Logic errors in a producer are not challenged at publication;
  independent semantic controls exist only where fixtures or oracles provide them.
- **Correction (Proposed; depends on F09 and the F12 decision).**
  - Enforce declared, stage-bound reads by construction (F09), which removes hermeticity as a
    reason for replay.
  - Use certificate checks where a cheaper independent checker exists: SCC partition and order,
    derivation acyclicity, witness paths, post-fixpoint and refutation proofs, PageRank residual,
    FCA closure, trace closure to premises.
  - Use key-coverage checks for completeness where outputs are total per input key.
  - Keep replay where the claim is enumeration, residual inventory, canonical choice or seeded
    reproducibility (access routes, summary residuals, selection domain closure, Leiden), and
    consider partitioned replay there.
  - Move whole-stage determinism replay to qualification and explicit audit rather than every
    publication.

  This amends ADR-0126 and §8; it must not weaken any completeness claim (CI-04, CI-08).
- **Challenge case.** A producer that silently omits rows passes every soundness certificate;
  key coverage or replay must remain wherever a served claim depends on completeness.
- **Closure.** Per-check classification recorded in the model's validation definitions (soundness
  certificate / completeness check / replay); a seeded producer logic error caught by a certificate
  that replay would have agreed with; a seeded omission still refused.

### 7.3 How the findings relate

- **Structural cause: coupling (F12).** Declarations dictate physical layout, execution unit and
  identity scope (§2.6). Execution-unit coupling produces F02, F03, F04, F08 and F13; identity
  coupling produces F05; layout-and-lifecycle coupling produces F01's transaction exposure and the
  choreography. Each keeps its own closure obligation.
- **Shared mechanism: store-centric validation.** F01, F02, F03, F13 and F14 all re-verify
  acknowledged content in the read or validation path instead of trusting immutable acknowledged
  frames, as ADR-0126 decides, or re-derive it with the producer's own function.
- **Alternative remedies.** F01's within-shape correction and R3 are alternative remedies for F01.
  If R3 is accepted and implemented promptly, F01 closes by construction. Otherwise apply the
  within-shape fix, which is small and unblocks every real-data result.
- **Prerequisite order.**
  1. F01 (or R3) unblocks real-library evidence.
  2. F09's stage-bound read seam is a prerequisite for R3 and is useful under R1.
  3. F12's decision precedes the store-side half of F04 and any family-merging lowering. The
     model-side half of F04 (moving assembly into `lctx-model`) is valid under either choice and
     can start now.
  4. F05 precedes F06 comparisons and any reuse.
- **Conflicting recommendations: none found.** R3 makes F02's model-owned closure the only
  realization and F08's route trivial; R1 keeps them as stated. F14 is compatible with both, but its
  savings are largest under R3, where replay no longer guards stored-input fidelity.

### 7.4 Foundation and supporting-rule verdicts

| Principle | Verdict | Scenario evidence |
|---|---|---|
| FP-01 Separation of concerns | **violated** | Serving semantics live with store effects (F04); store code inside fact identity (F05) |
| FP-02 Stable contracts, replaceable implementations | **violated** | Physical layout, execution unit and identity coupled to declarations (F12 §2.6); a store substitution must rewrite semantics |
| FP-03 Composition over entanglement | **satisfied** for analytics and stage composition (typed stage table); **unresolved** for serving (F04) | A new analytic composes through declared projection and policy; a new journey hand-chains hops |
| FP-04 Explicit domain model and scoped authority | **satisfied** for supported scope; **violated** for cross-version correspondence (F06) | One declaration owns meaning; graph projections derive correctly; no correspondence concept |
| FP-05 Explicit structure and constraints | **violated** | Uncoordinated resource bounds (F01); reference closure not in the model validator set (F02) |
| FP-06 Local reasoning | **violated** | Stage transformations need a container (F09); validation phase unobservable (F03) |
| DP-01 One authority per fact | satisfied (repeated enforcement only, F02); **violated** for read-path trust (F13 vs ADR-0126) | |
| DP-04 Identity is semantic and layered | **violated** for contract and provider identity scope (F05) | |
| DP-08 Domain operations carry contracts | **unresolved** for served paths (F04); satisfied for kernels | |
| DP-14 Built-ins and thin adapters | **violated** by the fork used as a scan transport (F08) | |
| DP-16 Minimize bespoke machinery | **violated** (F08, F10, F12 choreography) | |
| DP-17 Module boundaries follow ownership | **violated** (F04) | |
| DP-19 Coherent publication, explicit outcomes | safety **satisfied**; liveness **violated** (F01) | |
| DP-20 Bounded, coordinated resources | **violated** (F01, F13) | |
| DP-23 Verification matches the risk | **violated** (F11, F14) | |
| CI-01–CI-05, CI-08–CI-13 | **satisfied** within the recorded fixture scope (gates, §6) | |

---

## 8. Library fit and total complexity (slot 8)

### 8.1 Focused comparison

| Capability and owner | Consumer | Candidates | Semantic fit at the resolved version | Burden | Choice |
|---|---|---|---|---|---|
| Canonical store and immutability (`lctx-postgres`) | All stages, serving | PG18 (current); SurrealDB 3.3; write-once segments (Arrow IPC/Parquet + manifest); Dolt; TerminusDB | SurrealDB: no frozen database, leases, publication primitive or Arrow path; dangling `record<t>` accepted on write; IMPORT skips ASSERT; open RocksDB/SurrealKV issues (#7576, #7565, #7536, #7424, #7430, #7426, #6735); core not SemVer. Dolt: FK-violating commits in bulk mode; ~40% chunk rewrite for 1% churn. TerminusDB: branch held in memory | SurrealDB: +150 crates (SurrealKV) or +166 (RocksDB, C++ and bindgen); aws-lc-sys; object_store widened to cloud features, unified into DataFusion; net +50 after removing PG/DataFusion (A6). BSL needs a `deny.toml` widening (licences are not a criterion). A server is required for several readers | **Keep PG now; decide the shape (F12).** SurrealDB not adopted now; leading alternative lowering (addendum). |
| Validation execution | Publication, stage reads | Store queries (current); in-process model validators (`MemoryGeneration`) | In-process validators already produce the store's content digest | Memory bounds unmeasured (PR-3) | **In process (R3)**, subject to PR-3 |
| Graph traversal for serving | Ten routes | SurrealQL recursive idioms; AGE (not installed; 1.8.0-rc0 naming); recursive CTE; in-process kernels | Served paths are fixed-depth; recursion only in `explain` and access routes, both needing explicit truncation; SurrealDB has silent bounds, walks without edge identity, OOM on unbounded queries | A wrapper per traversal | **No graph engine.** Declared paths (F04). |
| Graph analytics (`lctx-analytics`, model) | SCC schedule, ranking, communities, neighbours, FCA | petgraph and leiden-rs (in use); rustworkx-core, graphina, igraph; Neo4j GDS | GDS: kNN approximate (71/300 wrong); Leiden modularity not RBER and nondeterministic at concurrency > 1; Louvain unseeded; PageRank unnormalized without residual. Community edition: 4-core cap, one database, offline import, Arrow Enterprise-only. neo4rs is an RC offering Bolt 4.0–4.4 only | JVM server, heap sizing, export/import per generation, re-keying | **In process.** Neo4j rejected. |
| Lexical + vector retrieval | `search_*` routes | bm25s + exact pgvector + Rust RRF (current); SurrealDB FULLTEXT + HNSW + `search::rrf`; ParadeDB pg_search | The exact profile needs brute force anywhere; database-side fusion would split the one ranking owner (ADR-0114); `search::rrf` unprobed | A second engine or extension | **Keep.** pg_search is the conditional candidate if BM25 moves into SQL. |
| Snapshot diff | Cross-version journey (F06) | Keyed outer join (PG/DataFusion); Dolt `dolt_diff`; TerminusDB `/api/diff`; SurrealDB client merge | Every store diffs by primary key; a semantic diff needs a declared correspondence key | — | **Model key + relational join.** |

### 8.2 Second-order inventory per candidate

Rows follow A5's mechanism map. "Gone" means no longer needed; "New" means a new obligation.

| Mechanism (obligation) | Baseline + corrections (R1) | Compile-then-store on PG (R3) | Published segments (R2) | SurrealDB primary (W1) | SurrealDB projection | Neo4j layer |
|---|---|---|---|---|---|---|
| Roles, bootstrap, grant/revoke phases, ACL verification, lock-capacity tuning | Kept, simplified | Reduced to load, seal and reader grants; stage-read grants gone | Gone; file permissions | Gone; **New**: users and levels, `agent` record access, `PERMISSIONS` per table, credential-based immutability | Kept + projection credentials | Kept + Neo4j credentials |
| Per-generation DDL (~1,100 tables) and `store check` | Kept | Kept (fewer objects if families merge, §2.4) | Gone; Arrow schema only | **New** SurrealQL generator (8,234 statements in W1) + `INFO FOR DB` drift check; parser-depth trap | Both | Kept |
| Leases, locks, serving guard | Kept; dual-driver part gone (F08) | Kept for serving; attempt-read leases gone | **New** pin registry; GC honours pins | **New** lease records as `$auth`; retirement checked by the application | Kept + projection binding check | Kept |
| Validation framing, receipts, invariants | Kept; F01, F02, F03 corrections | Invariants in process; one post-load framing | Invariants in process; framing at write | Kept in Rust + **New** SurrealDB→Arrow ordered codec; generated `closure:*` pass | Kept + projection equivalence proof | Kept |
| Reference closure | One model owner (F02) | Model owner in memory | Model owner | **New** validation pass via record-link fetches | Kept | Kept |
| Vocabulary epochs | Kept, or predicate-based | In memory; store sees closed prefixes only | Union of immutable segments ≤ N | **Open design gap** (W1 did not carry epochs) | Kept | Kept |
| Completed-stage reads (`AttemptSession`, source-bound capabilities) | Kept | Gone (in-memory stage-bound reads, F09) | Gone | Rebuilt over SurrealDB | Kept | Kept |
| Load and read codecs (pgpq, COPY, fork) | pgpq kept; fork gone (F08) | One COPY per relation; fork gone | Gone; Arrow IPC/Parquet native | **New** Arrow↔SurrealValue both ways; HTTP 413 at ~2 MB, so ws or embedded needed | **New** exporter | **New** Parquet export + offline `neo4j-admin import` |
| Projection snapshots (Postcard) | Kept | Held in memory across stages; persisted once | Kept as a segment | Kept (kernels need in-process graphs) | Kept | Kept |
| Serving hydration | Declared paths, set-based (F04) | Same | **New** index realization (PR-3) | `<~(t FIELD f)` and link fetches; wrappers for truncation and edge identity; semantic assembly unchanged | As primary + CI-13 equivalence | n/a |
| DataFusion and `lctx query` | DataFusion over MemTables/files; fork gone | Same | Reads files | Gone; `lctx query` becomes SurrealQL | Kept | Kept |
| Embedding cache, `lctx_ops`, backup | Cache kept; `lctx_ops` gone (F10) | Same | Cache as files keyed by (spec, input hash) | Cache in SurrealDB; backup via `surreal export` (SurrealQL text); one-way datastore upgrades | Both | Unchanged |
| Attempt memory and spill | Store holds intermediates | **New** bounded residency + attempt-local segment spill contract (quota, cleanup); ADR-0105 clause revisited | Same | Store holds intermediates | — | — |
| Test harness | Docker PG; store-free stage reads (F09); unselected targets resolved (F11) | Most stage and validation tests store-free; PG only for load, publish, serving | Tempdir; real-store controls only for pins and GC | Embedded engines on tempdir, but `mem` ≠ production engines; a server for multi-process; race tests become conflict-retry tests | Both engines | + Neo4j container |
| Dependencies (A6) | −~40 crates (fork, tokio-postgres, OpenSSL) | Same | −~120 more if the cache leaves PG | Net +50 crates; aws-lc-sys; RocksDB C++/bindgen if chosen; object_store cloud features | Additive (+150–166) | + neo4rs RC (Bolt 4) or Python driver |
| Runbook and operations | Smaller | Smaller still | Directory layout, GC command | Different, similar size: server, engine flags, retention, export | Both | + JVM service, heap, plugins |
| ADR obligations | F01–F13 corrections; F12 ADR | Amend ADR-0086/§15.11 trust wording, ADR-0105 memory clause, ADR-0126 binding text | Supersede ADR-0086 realization clauses | Supersede ADR-0086, §B7, §B12, §B10 | Amend §B10, §B12/§6.4 | Amend §B10 |

**Total complexity.**
- **SurrealDB primary** trades roughly equal bespoke volume for weaker immutability, less mature
  storage and a second query language, while every domain owner stays unchanged.
- **The SurrealDB projection** adds a whole second stack for no named consumer.
- **R3** removes the most machinery while keeping PostgreSQL's mature publish, privilege, vector and
  serving roles. Its new obligation is a bounded-memory and spill contract.
- **R2** removes still more, but leaves the serving-index question open.
- **Baseline plus corrections** fixes the real-scale failure and removes ~40 crates, one driver/TLS
  stack, a fork, dead services and three duplicate enforcement paths. It leaves the store-centric
  choreography in place.

---

## 9. Alternatives and tradeoffs (slot 9)

| Alternative | Change propagation and local reasoning | Semantic authority and composition | Test/substitution boundary | Machinery and risk | Evidence, decision, revisit |
|---|---|---|---|---|---|
| **Current baseline** | Model edits reset the store (F05); serving changes sit inside the effect crate (F04) | Model authoritative; four closure realizations | Containers for stage reads | Real-scale failure (F01); choreography (F12) | Operational failure 2026-10-05 → **Revise** |
| **Current design with targeted corrections (R1)** | Store reset only on contract change; packets declared | One owner per rule | Store-free stage reads and packet assembly | Removes the fork, OpenSSL and dead services; keeps the store-centric choreography | **Minimum now** (F01–F11, F13) |
| **Compile-then-store (R3)** | Stages compose in process; the store sees one validated generation | Model validators are the only validation authority; stored-content check by digest through the reader decode path | Most controls store-free | Retires stage grants, attempt sessions, delta tables, checkpoint trials, fork; **new** bounded-memory and spill contract | **Leading direction for F12**; settle memory bounds with PR-3; ADR amending ADR-0086/0105/0126 |
| **Strongest PostgreSQL design** (AGE, pg_search, pgvectorscale) | AGE adds Cypher without algorithms; served paths need none | A second query semantics | Extension images | None installed on the operator cluster (A4) | **Not now.** pg_search is the conditional candidate if BM25 moves to SQL; AGE only with an ad-hoc traversal consumer |
| **SurrealDB primary** (W1 design) | Domain unchanged; substrate rewritten | Immutability by loader protocol; traversal wrappers; bespoke validation pass | Embedded engines on tempdir; `mem` ≠ production | Net +50 crates; open storage bugs; non-SemVer core; one-way upgrades; a server for several readers | **Not now; leading alternative lowering after decoupling** (addendum; reviewer: Rejected). Real-data load results cannot reverse this; they can only add operational objections or remove the "ingest feasibility unknown" item (§10.2) |
| **SurrealDB as rebuildable graph/search/serving projection** | An additive second store | Must prove equivalence (CI-13); database-side fusion would split ranking ownership | Two engines | Exporter (no Arrow import), server, credentials | **Not adopted; deferred** with the triggers below |
| **Hybrid with SurrealDB canonical for some layers** | Every compile spans two stores | Two canonical authorities without a shared transaction (ADR-0083 option 2's objection still holds) | — | Highest | **Rejected** |
| **Structurally shared snapshots** (any store; Dolt and TerminusDB as comparators) | Lifecycle columns or membership tables | M2 weakens published immutability; M1 costs more than full copies | — | GC, pins | **Not now** (supporting document §5.4). Revisit after F05, given a measured high sharing ratio and a named storage or rebuild problem |
| **Published segments (R2)** | Store changes isolated to file layout and index realization | Immutability by construction; model validators sole owners | Tempdir tests | Serving-index realization unqualified | **Variant of R3** for the published generation; settle with PR-3 |
| **Model decoupling: declared lowerings** (§2.6) | Store, layout and execution-unit changes stay in the lowering and effect adapters | Declarations remain sole authority; identity mapping explicit | Every seam has a store-free realization | A thin plan; risk of over-generalization if it grows into a framework | **Selected, priority 1 (model side).** Prerequisite for any store change |
| **Certificate checking instead of replay** (§2.8) | Per-check classification in validation definitions | More independent for soundness; weaker for completeness | Checkers testable without producers | ~a dozen new checkers; replay retained where completeness is the claim | **Selected direction (F14)**, after F09; amend ADR-0126/§8 |
| **Typing as graph metadata** (§2.4) | Kinds become tables; no count reduction from the graph | Second authority or convention if made authoritative | — | Partition-level authority if families merge on PG | **Rejected as authority**; family merging is a lowering option under F12 |
| **Neo4j GDS** (against Rust libraries and SurrealDB alone) | Export and import per generation | Determinism and exactness gaps | JVM container | 4-core cap; Arrow Enterprise-only; neo4rs RC on Bolt 4 | **Rejected.** Revisit only for a named consumer of a GDS-only algorithm that rustworkx-core, graphina or igraph cannot serve |

**SurrealDB capabilities a projection would use, if a trigger fires:**
- a database per generation with a pointer record;
- record-access users with `CONTEXT` pinned through the lease identity, not `selection:current`
  (W1 run 1);
- `PERMISSIONS FOR select WHERE …` per table;
- record links with named reverse references `<~(t FIELD f)`;
- LIGHTWEIGHT or INLINE edges only where edge identity is not served;
- bounded recursive idioms, always with an explicit bound, a bound+1 probe for truncation and a
  query `TIMEOUT`;
- FULLTEXT BM25 analyzers;
- HNSW only behind an exact re-rank;
- generated SCHEMAFULL/STRICT DDL with flat sum assertions.

**Triggers for the projection:**
- (a) a product journey that exposes open-ended or agent-authored graph exploration over the served
  graph, which §14.5 currently excludes;
- (b) a measured serving-path cost that set-based PostgreSQL reads (F04) cannot bring within the
  30 s deadline;
- (c) SurrealDB gaining a frozen or read-only database, an Arrow bulk path and a stable storage
  engine record.

Any of these reopens the projection through an ADR. The reviewer also required (d), a store-enforced
immutability primitive, before reopening SurrealDB as primary. The [coordinator addendum](#coordinator-addendum)
drops (d) for this project and adds (e): set-based PostgreSQL serving paths proving awkward in
practice. Any of (a), (b), (c) or (e) reopens SurrealDB as the persisted and served lowering.

**Neo4j trigger.** A named consumer for trained graph embeddings, link-prediction pipelines or a
centrality family with no in-process library, after §B10 is amended for that consumer.

---

## 10. Verification and uncertainty (slot 10)

### 10.1 Claims and their evidence

| Claim | Label / date | Basis | Outcome or gap |
|---|---|---|---|
| F01 cause and failure | Implemented (source) + operational observation, 2026-10-05 | Source as cited; server log 10:09:04 EDT; compile log; `pg_stat_activity` at 14:09:00 UTC | Attribution of the exact idle interval is inferred; PR-1 |
| W1 native realization functional on synthetic data | Tested in probe, 2026-10-05 | `…/2026-10-05_surrealdb-native-realization/` (synthetic run: all probe rows `passed`) | Shallow journeys only (A8 §0.5); no `TYPE RELATION` (A7) |
| GDS determinism and parity gaps | Tested in probe (synthetic, 300 nodes), 2026-10-05 | `…/2026-10-05_graph-analytics-parity/` | Real-data parity `blocked` |
| SurrealDB traversal traps (silent bound, null conflation, walks, OOM) | Tested in probe, 2026-10-05 | W3 P4; A3-run | OOM shown on the memory engine only |
| Exact identity and semantic diffs in four stores | Tested in probe (synthetic, 612k rows), 2026-10-05 | W2 | Real diff running on the facts layer |
| Validation incrementality (57 of 119 delta-checkable) | Static review, 2026-10-05 | W2 probe 3 | D/G partitionability not verified per replay |
| Sharing ≈ 0 across binaries and versions | Interface-checked reasoning, 2026-10-05 | Key chain (§3.2) | Real two-generation intersection pending |
| Relation-shape census | Static, 2026-10-05 | A7 | Heuristic classes; boundaries move by tens |
| `MemoryGeneration` validates to the store's digest | Implemented (source), 2026-10-05 | `memory.rs:1-4, 131-143` | Equality on real data not run; memory bounds unmeasured (PR-3) |
| A6 dependency fit | Static resolution, 2026-10-05 | `cargo metadata`, `check_family.py`, `cargo deny` in scratch | Compile on the pinned nightly `not_run` |
| Serving tiers and hop counts | Static, 2026-10-05 | A8 | Index coverage unverified (PR-6) |
| Every performance statement in this review | **Proposed** | — | No benchmark, by operator direction; runtime figures are qualitative observations |

### 10.2 Pending real-data parts

| Part | Status | Revisit trigger | What it would change |
|---|---|---|---|
| Full facts-layer load into SurrealDB (RocksDB) | **Completed** (W1, 2026-10-05; coordinator-recorded outcome): 16.56M identity + 1.04M payload records, counts equal to the compiler's frame receipts, no stall or refusal; container memory peaked at 34.2 GiB under a 48 GiB cap and was not released (qualitative). Seeded violations refused or caught; pinning across a second publish, sharing (243 of 245 relations carried forward) and lease-gated retirement held | Settled | A stall or OOM adds an operational objection to SurrealDB in any role; completion removes only the "ingest feasibility unknown" item. **It cannot reverse decision 1**, which rests on guarantees, semantics, absent consumers and maturity. |
| Generated validation pass at scale in SurrealDB | **Completed** (W1): 868 generated checks, no failures; the 231 model invariants, 39 publication checks and BLAKE3 recomputation were `not_run` (they stay in Rust) | Settled | If it does not complete, a SurrealDB design must read records into Rust validators (already assumed). No verdict change. |
| Real identity diff and sharing between generations | **Partly completed** (W2, 15 facts relations, 3.53M rows, simulated key cascade): exact diffs `passed` in PostgreSQL and Dolt and for SurrealDB identity/content completeness (DuckDB-checked reference); SurrealDB `INSERT IGNORE` silently dropped type-violating rows (two whole relations on the first attempt) while the membership diff still passed; a 3.5M-row `INSERT … SELECT` was refused ("Memtable arena is full"); PostgreSQL sizes: full copies 1,135 MB, shared content + membership 1,597 MB, validity intervals 723 MB; Dolt grew 49.4% for ~1% churn. Sharing between two *real* generations and TerminusDB `not_run` (stopped at operator direction) | Partly settled; two-generation sharing remains open | High sharing across *different binaries* would contradict §3.2's key-chain reading of F05 and reopen supporting document §5.4; ~0 confirms. |
| Correspondence-key uniqueness (`native_key`, `qualified_name`) | **Completed on the facts layer** (W2): `native_key` is an analyzer-positional index (`F:n`, `CF:class:n`), neither unique nor version-stable, so it is **refuted** as the correspondence key; a syntax-containment qualified path is unique for classes (1,995/1,995) and nearly unique for callables (13,862/13,950; collisions are deliberate same-name redefinitions); public path (module, name) unique (4,720). Normalized kinds `blocked`; cross-version stability `not_run` | Settled for key choice; cross-version stability open | Shapes F06's key and its ambiguity handling, not its necessity |
| Real call-graph traversal checks in SurrealDB | **Completed** (W3) on a facts-layer call graph (36,954 vertices, 118,595 arcs; an approximation of the normalized projection): reachability and shortest path equal networkx 66/66; depth bounds silently drop reachable nodes (11 of 66 starts at depth 1); `+path` counts walks (1.8M at depth 12 from the largest 15-node SCC); depth 16 and unbounded `+path` were OOM-killed despite `TIMEOUT` | Settled | `+path` blow-up at realistic depth would confirm the wrapper requirement. No verdict change. |
| Catalog journeys (W1 P1 on real data) | **Blocked**: no published catalog generation (F01) | F01 fixed and a catalog generation published | Deep shapes (`claim_basis`, `resolve_original`, `explain`) in SurrealQL would test A8's awkwardness claims. No verdict change expected. |
| GDS determinism and parity on a facts-layer call graph | **Completed** (W3): SCC/WCC equal networkx and petgraph; PageRank equal after normalization (sum 7,770.8 unnormalized, no dangling redistribution), concurrency 1 vs 4 differs by ~1e-12 and reorders near ties; Leiden varies at concurrency 4 and with load order (ARI 0.82), agrees with our RBER Leiden at ARI 0.44 | Settled | Confirms §5.3/§8.1; no verdict change. |
| GDS parity against published analytics | **Blocked** (same) | Same | None expected for Neo4j; the kNN contract failure and determinism results suffice. |

### 10.3 Probe requests

| ID | Question | Minimal setup | Control | Decision impact |
|---|---|---|---|---|
| PR-1 | Does an idle-in-transaction interval during `finish()` cause F01, and does the corrected protocol remove it? | Store-family control on disposable PG18: lower the idle cap to 1 s via test configuration; a test invariant whose `finish()` sleeps 2 s | The same invariant with no sleep passes | Confirms F01's attribution; becomes F01's closure control after the fix |
| PR-2 | Which validation parts dominate on real data? | Emit `ValidationStats` and spans; rerun the facts-frontier compile | Stage timings already emitted | Orders F02/F03 work; observational, not a benchmark gate |
| PR-3 | Can the generation be compiled, validated and persisted in process within bounded memory, and can serving work over segments? | (a) Load the facts layer (16.6M rows) from the interrupted generation into `MemoryGeneration` as Arrow batches; run facts admission, reference closure and the facts-checkpoint invariants in process; record completion and peak memory qualitatively; compare its content digest with the store's frame receipts. (b) Repeat with attempt-local Arrow IPC spill per relation. (c) Realize three serving paths over segments with in-memory or sidecar indexes | The same checks through the current PG route on the same rows; digest equality is the correctness control | Decides F12: feasible bounded memory → R3; (c) feasible → R2 becomes viable for the published generation |
| PR-4 | Is the proposed correspondence key unique and stable? | Facts layer: collision counts for (provider, `native_key`, role, variant, form) and (module `qualified_name`, stub flag) | A hand-checked FastMCP API list | Shapes F06 |
| PR-5 | Does `explain` breach the deadline on real derivation relations? | After F01, on a published catalog generation | Fixture `explain` | Confirms F13's priority; static evidence already suffices for the correction |
| PR-6 | Is every `read_for` field covered by a lookup index usable without a `generation_id` predicate? | Static comparison of `read_for` (relation, field) pairs with the `ddl.rs:391-406` inventory; `EXPLAIN` one hop on the facts layer | — | Shapes F04's executor and index lowering |

---

## 11. Authority changes and dispositions (slot 11)

No plan owns these findings yet. Under binding §4, this review's rows are the disposition owner until
the coordinator transfers them to a plan's findings table.

| Required change | Route and owner | Findings | Disposition | Closure or revisit trigger |
|---|---|---|---|---|
| Record the store evaluation: PostgreSQL retained now; SurrealDB not adopted now and recorded as the leading alternative lowering behind the F12 seam; Neo4j not integrated; triggers | **New ADR or §B10 amendment citing this review**, replacing §B10's bare exclusion with this evaluation's reasons and triggers, and linking ADR-0086; DESIGN owner §B7/§B10/§B12 | Decisions 1–2 | Open (decision) | Triggers in §9 |
| Pipeline shape and published realization (R1/R3/R2), including family lowering and the mechanism-neutral generation wording (supporting document §6) | **New ADR**, amending ADR-0086/§15.11 (validation-of-stored-contents wording), ADR-0105 (memory handoff clause), ADR-0126 (binding text) | F12 (+ F01, F02, F03, F08, F09 by construction under R3) | Open (decision) | PR-3 |
| Validation outside transactions | Implementation within ADR-0126 (no new decision needed unless superseded by R3); `lctx-postgres` | F01 | Open, priority 1 | PR-1; real facts-checkpoint validation completes |
| One owner for reference closure | Implementation under ADR-0126; amend §15.11 wording on FKs | F02 | Open | Single-owner trace; seeded controls |
| Validation instrumentation and frame reuse | Implementation | F03 | Open | PR-2 |
| Serving assembly into the model; declared paths | ADR (serving path declarations), amending the §15.12/§14.9 extension route; `lctx-model::domain::serving`, `lctx-postgres` | F04 | Open; the model half can start now, the executor after F12 | A traced new packet |
| Identity scope | ADR amending §15.3 (superseding the conservative-fingerprint clause of ADR-0124/0088 as needed) | F05 | Open | Two revealing controls |
| Correspondence keys and cross-version comparison | New ADR amending §15.3, §14 and the ADR-0105/CI-13 interpretation | F06 | Open, planned-capability trigger | PR-4; fixture release pair |
| Merkle relation digest | Implementation with its first consumer | F07 | Deferred; trigger: an F06 comparison or receipt-reuse consumer | — |
| Retire the provider fork | Implementation + pins row removal; amend §B3/§15.11 compute text | F08 | Open | `openssl-sys` absent |
| Store-free stage reads | Implementation | F09 | Open; R3 prerequisite | Container-free normalized test |
| Retire `lctx_ops` | Implementation; runbook | F10 | Open | Removal commit |
| Family membership of unselected targets | Assurance coordinator | F11 | Open | Every target in a family or retired |
| `explain` trust model | Implementation aligned with ADR-0126; clarify §15.12 "verified" | F13 | Open | Size-independent `explain` control |
| Declared lowering seam (connected, not coupled) | ADR amending §15.2 ("lowers into ordinary tables") and ADR-0086's realization clause to "a declared lowering with identity mapping"; `lctx-model`, `lctx-postgres`, `cpg-core` | F12 part 1 (+ F04, F05, F09) | Open, priority 1 | Traced substitution scenario; no `R::NAME` addressing outside the lowering |
| Certificate and coverage checks; replay moved to audit and qualification | ADR amending ADR-0126 and §8 | F14 | Open, after F09 | Seeded logic-error and omission controls |

§B decisions affected: §B10 (reason text and triggers); §B7 and ADR-0086 (validation wording and
realization, through F12); §B3 and §15.11 (compute wording, F08); §15.3 (identity, F05 and F06);
§B12 and §6.4 only if a projection is later admitted.

---

## 12. Architectural judgment and decision (slot 12)

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | **violated** | Mechanism substitution: declarations coupled to layout, execution unit and identity (F12 §2.6); serving semantics interleaved with hop effects (F04); stage reads only through the PG fork (F08, F09). Model-field change: any byte resets the store and re-identifies facts (F05). Real-scale validation fails on an uncoordinated resource bound (F01). | F12 part 1, F01, F04, F05, F08, F09 |
| A2 Encode domain meaning explicitly | **satisfied** for the current supported catalog scope; **violated** for the target new-release journey | The model is explicit and governs construction, validation and serving semantics; graph projections are correctly derived artifacts; no consumer reinterprets reference or availability meaning. No correspondence concept exists for cross-version comparison (F06). Typing must remain a Rust declaration (§2.4). | F06 |
| A3 Extend through composition | **satisfied** for "add an analytic" (declared projection, policy and kernel; binding §3 seed); **unresolved** for "new agent journey" | A new journey hand-chains hops and closure checks inside the effect crate (F04); §15.12's extension route names a packet binding and hydrator but no path declaration | F04 |

**Bounded decision: Revise.** The boundaries to resolve:
- the coupling of declarations to layout, execution unit and identity: a declared lowering seam
  (F12 part 1), which carries the store-independent work;
- the validation transaction protocol (F01);
- the reference-closure owner (F02) and the role of replay versus certificates (F14);
- the serving assembly and path boundary (F04);
- identity scope (F05);
- the correspondence concept (F06);
- the pipeline shape and published realization (F12 part 2).

On the candidates:
- **SurrealDB primary: not now** (addendum; reviewer: Reject); leading alternative lowering behind
  the F12 seam, with triggers (§9).
- **SurrealDB projection: not adopted now**, deferred with triggers (§9).
- **Neo4j: Reject.**
- **Typing as graph metadata: rejected as authority**; family merging is a lowering option.
- **Snapshot:** mechanism-neutral contract; no structural sharing now.
- **Compile-then-store (R3): leading direction**, subject to PR-3.

**Enclosing architecture: needs revision** for the compile/validation/store/serving boundary, at
Interface-checked strength plus one operational failure observation. The model layer, stage table,
graph projections, analytics placement and typed fidelity are accepted for the named scenarios at
Implemented/Tested (fixture) strength. This review is not release qualification. Real-library
qualification remains stopped and is now blocked on F01.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Unblock real scale: validation outside transactions, or R3 if adopted promptly (`lctx-postgres`) | F01 | PR-1; real facts-checkpoint validation completes |
| 1 | Decoupling on the model side: declared lowering plan with identity mapping; store-free stage-bound reads; contract and provider identity from declarations (`lctx-model`, `cpg-core`, `cpg-extract`) | F12 part 1, F09, F05 | Traced substitution scenario; container-free stage test; revealing identity controls |
| 2 | Pipeline-shape and published-realization decision (ADR), with PR-3 | F12 part 2 | PR-3 |
| 2 | One reference-closure owner; instrumentation; `explain` trust alignment (`lctx-postgres`, `lctx-model`) | F02, F03, F13 | Single owner; PR-2; size-independent `explain` |
| 2 | Serving assembly into `lctx-model`, then declared paths | F04 | A traced new packet |
| 3 | Certificate and coverage checks; whole-stage replay to audit and qualification | F14 | Seeded logic-error and omission controls |
| 3 | Retire the provider fork (`cpg-core`) | F08 | OpenSSL absent |
| 3 | Correspondence-keys ADR (`lctx-model`) | F06 | PR-4; release-pair fixture |
| 3 | Unselected controls | F11 | Family coverage |
| 4 | Merkle digest; retire `lctx_ops` | F07, F10 | First consumer; removal |

Prerequisite order differs from priority:
- F01, or R3, unblocks every real-data result.
- Decoupling (F12 part 1, F09, F05) precedes any store change, so that a store change stays
  contained; it also precedes R3, which needs the store-free read seam.
- F12 part 2 precedes the store-side half of F04 and any family merging (§2.4).
- F09 precedes F14 (hermeticity by construction).
- F05 precedes F06 comparisons and any reuse.

**Next consequential decision.** The pipeline-shape ADR (F12 part 2), informed by PR-3, together
with the lowering-seam ADR (F12 part 1). Meanwhile the coordinator either schedules F01's
within-shape fix or commits to R3 immediately, and arranges PR-1 and PR-3. The root agent drafts the
store-evaluation ADR (decisions 1–2). Owner: root agent and the cutover coordinator.
