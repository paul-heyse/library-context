# Design review: capability-compiler design (standard)

**Date:** 2026-09-22 · **Depth:** standard · **Mode:** document-stage. There is almost no code:
`crates/cpg-schema/src/lib.rs` is still a doc comment.
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that rewrote
DESIGN.md and drafted ADR-0004 … ADR-0011
**Prior review:** `design_review_design-spine-baseline_2026-09-22.md` (compact, Not Accept)

## 1. Decision and scope

**Proposal.** The subject is `docs/design/DESIGN.md` as rewritten on 2026-09-22 for the capability
compiler: §1, §B1–§B14, §3–§13. It comes with:
- ADR-0004 … ADR-0011, all `proposed` (ADR-0006 supersedes ADR-0003);
- `docs/initial_plan/DISPOSITION.md`;
- ADDENDUM §1 (§B1–§B14);
- `docs/pins.md`.

The working tree is uncommitted: ADR-0004 … ADR-0011 and DISPOSITION are untracked, and DESIGN is
modified. I reviewed the files as they are on disk.

**Status of the claims.** DESIGN's own labels are:
- **Proposed** by default;
- **Interface-checked** for §B1, §B4, §B7, §B8, §B13, the vLLM/Qwen part of §B14, §4.2, §5, §6,
  the §9 libraries and §11;
- **Tested** for §B9/§7.

**What is being judged.**
1. **Increment 1 (DESIGN §1.2 row 1).** Is it decidable? That is, would two competent implementers
   reading only DESIGN and the ADRs build the same semantics for the six families, Pass A, template
   synthesis, the bundle, retrieval and the two FastMCP tools?
2. Authority between the Delta store, the derived views and the serving bundle.
3. Whether the synthesis rules (§10) can be enforced.
4. Proportionality, given ADR-0005's analytics-first operator direction.
5. Consistency across DESIGN, the ADRs, DISPOSITION, ADDENDUM §1 and pins.

**Baseline.** The seeded spine was a fact substrate with no product (ADR-0004, Context). The
baseline review returned Not Accept, with G1, G2, G4, G5 and G6 unresolved.

**Supported scope and non-goals.** DESIGN §1.3.

**Constraints.**
- One operator.
- The operator directed the design to be analytics-first (ADR-0005). That direction is taken as
  given; the review judges whether its guardrails work.
- DESIGN is 1,083 lines against a budget of about 1,100 (ADR-0004).

### Method and coverage

**Read in full:**
- DESIGN.md;
- ADR-0001 … ADR-0011 and the ADR index;
- DISPOSITION.md and pins.md;
- ADDENDUM, REVIEW_REFERENCE, the charter, the directive and the template;
- the baseline review;
- `justfile`, `Cargo.toml`, `pyproject.toml`, `STATUS.md`;
- `crates/cpg-schema/{Cargo.toml,src/lib.rs}`;
- `scripts/adr.py` (amendment rules).

**Initial_plan read at line grain:** L1581–L1625, L1650–L2179 and L2233–L3085. The rest was read
only through the DISPOSITION rows and the baseline review.

**Ran (check outcomes):**

| Command | Outcome | Detail |
|---|---|---|
| `just check` | passed | fmt-check; clippy `-D warnings`; nextest 2/2 passed (`family_smoke`); pytest 10 passed; pyrefly; `rules-scan` not_run (no rules yet); lint-agents ok; adr lint ok (11 records) |
| `just deps` | passed | `Cargo.lock: family ok`; cargo-deny bans, licenses and sources ok |
| DISPOSITION completeness (an ad-hoc script comparing each row with the IP's level-1/2 headings) | passed | 120 of 120 headings covered. Two cosmetic title mismatches: L156 (apostrophe) and L233 (abbreviated title) |

**Interface spot-checks,** each read myself in the pinned skill content:
- **deltalake.** These claims hold at the cited cards:
  - `delta.commit.1` (Err after the new version became visible). The card's own guidance is
    "Reload/reconcile after ambiguous error".
  - `delta.write.3` (`SaveMode::Ignore` appends).
  - `delta.open.2` (a loaded handle's snapshot wins over the builder's version).
  - `delta.read.2/3/4`.
  - `delta.storage.4`.
  - `delta.replay.1`.
  - The vacuum default of Lite with `dry_run=false` (`delta.retention.md` L3). DESIGN cites the
    claim id `delta.retention.1`, but that id itself records the Full-vs-Lite orphan result.
  - Binary read back as BinaryView (`DeltaScanConfig.schema_force_view_types`).

  I did **not** find the "FixedSizeBinary is written as generic BINARY" claim, so it is asserted
  here. ADR-0009's spike covers it.
- **petgraph 0.8.3.** These hold:
  - `edges_directed` lists the most recently added edge first (`operations/petgraph.graph_impl.Graph.md`
    L714, L743);
  - `Graph::remove_node` retargets a held index (`capabilities/graph.container.md` L20);
  - `page_rank(graph, damping_factor, nb_iter)` has **no edge-weight parameter**.

  I did not find "Bfs and Dfs visit siblings in opposite orders", so it is asserted. DESIGN's rule
  doesn't depend on it, because it never relies on walker order.
- **fastmcp 4.0.3.** These hold:
  - `ToolError` (`exceptions.py:54`);
  - `mask_error_details` (`server/server.py:300`);
  - `ctx.lifespan_context` (`server/context.py:418`);
  - `structured_content` (`client/client.py:256`);
  - the snake-case `ToolAnnotations` hints.

  I also read `FastMCP.add_tool`/`FastMCP.tool` (`server/server.py` L1795–L1851) and the gold
  `content/capabilities/fm.register.json`.
- **pyrefly-ruff.** These hold:
  - `UnresolvedReason` has 14 variants (`api/pyrefly.report.pysa.pysa_report_capnp.md` L122–L125);
  - `coverage report --public-only` exists and means "Only report symbols reachable from public
    modules via re-export chains" (`index/cli.tsv` L160);
  - the interpreter and search-path flags cited under F10.
- **vLLM 0.30.0 (installed source).** `dimensions` is rejected for non-Matryoshka models
  (`pooling_params.py` L173–L178). The no-space `Query:` template is a model-card claim that I did
  not verify.
- **Not verified at all:** leiden-rs 0.8.1 (no skill), LanceDB 0.39, the Qwen revision, and the
  FastMCP 4.0.3 → 4.0.5 diff.

**Attacked:**
- an Err on the publication commit itself (F8);
- the bundle written before or after the append (F1);
- rebuilding a generation after deleting it (F1);
- a statistical Outcome reaching a search hit (F2);
- FCA run over a Leiden community (F2);
- the gold leaking into the compiler through the subsystem boundary (F5);
- two providers disagreeing on what is "public" (F4);
- an overloaded pilot entry point (F3);
- disabling a technique for ablation (F6, F7);
- a Pyrefly run under a different `PATH` (F10);
- the Python server reading an IPC file (F9).

**Clean on inspection** (examined, no defect found):
- §5's determinism rules and three identities;
- the §6.2 reader steps;
- §8's read-only, shared and not-delegated rules;
- §3.4's identity rules, apart from F6;
- the §11.1 response checks;
- §3.6's `ifCalled` and `artificial-call` handling;
- §4.2 "Pysa is authoritative for calls".

**Not attacked, so asserted:**
- leiden-rs determinism;
- NextClosure correctness;
- BM25/RRF;
- whether the symlink rename is atomic;
- concurrent compile attempts;
- Delta Binary statistics and the cost of `snapshot_id` filtering;
- Pysa decode volume and non-ASCII column units. These have named spikes or belong to later
  increments.

### Continuity with the baseline review

| Baseline | Now | Evidence | Residual |
|---|---|---|---|
| **F1** column authority and missing domains | **partially resolved** | §3.5 gives values for `extraction_mode`, `modality`, `resolution_status` and `resolution_domain`. §3.3 settles `Int16` categories, so the `utf8`/`Int16` split is gone. §3.2 lists families by increment. ADR-0008 names insta snapshots and an append-only codebook test. | `model` (§B6, L184–L185) still has no domain. Pysa's 14 unresolved reasons (§3.6, L444) have no codebook. `finding_kind` and `assertion_kind` are deferred "as their consumers land" (L418), but increment 1 *is* their first consumer. `cpg-schema` is empty and neither named oracle exists yet. Carried into F3 and F4. |
| **F2** rules dropped in condensation | **resolved** | (a) L370–L372, (b) L429, (c) L374, (d) L527, (e) L431, (f) L373, (g) L433, (i) L350–L351. (h) is dissolved by ADR-0006: there is no Ruff hook. | — |
| **F3** publication protocol | **resolved for the canonical store** | ADR-0009 and §6.1–§6.2: a `snapshots` append after validation; abort on error; a retry gets a new `snapshot_id`; readers filter by `snapshot_id`; `failed` coverage still publishes | The Err on the publication append itself (F8). The bundle's order relative to the append (F1). |
| **F4** run and ID contract | **largely resolved** | §3.4.1, §4.0 and ADR-0007 | The release is in neither `run_id` nor `content_digest`. There are no finding/assertion/brief ID rules, and it is unclear which run carries the analytics-config digest (F6). Pyrefly's ambient inputs go beyond config discovery (F10). |
| **F5** coverage and issue tables | **resolved** | §3.7 (tables with grain); §3.2 L313–L319 defines "fact family" | — |
| **F6** validator placement | **resolved, and now moot** | §B2 L145 places the validators in a core-only crate. ADR-0006 removed the adapter workspaces that F6 was protecting. | The suggested `check_family.py` assertion was never added, and it is no longer needed. |
| **O1** settled-looking section, proposed ADR | **partially resolved** | ADR-0003 is superseded, and §B8 now cites ADR-0006 | The same pattern now covers eight proposed ADRs (O5). |
| **O2** `Cargo.toml` comment | **resolved** | `Cargo.toml` L11–L14 | — |

### Focus 1 in one table: is increment 1 decidable?

| Component | Decidable? | Where two implementers diverge |
|---|---|---|
| `provenance` | mostly | `model` has no domain. §B6 doesn't say whether a family row *is* a `facts` row or references one. (F4) |
| `exports` | **no** | What "public" means: Ruff `__all__`/aliases vs Pyrefly re-export reachability (F4) |
| `signatures` | **no** | One `parameters` row fed by two producers; three declarations for `FastMCP.tool` (F3, F4) |
| `calls` | mostly | `arguments` has no increment-1 producer in §4.2 (F4) |
| `coverage` | yes | — |
| `findings` | partly | Pass A finding kinds: yes. No ID rules for findings, assertions or briefs (F6). No domain for "evidence" (F3). |
| Pass A | partly | Its bound, "the subsystem", is defined only by gold families (F5). The seed for an overloaded access path, and how three witnesses come from a parent-pointer BFS, are undecided (F3). |
| Template synthesis | **no** | Delegation has no brief section. Assertion kinds are deferred. The Outcome needs a docstring that no increment-1 family carries (F3). |
| Serving bundle | partly | The file list is given. Its source, its order relative to publication, where `evidence` and vectors come from, and the generation key are not (F1). |
| Retrieval (exact cosine) | yes, mechanically | Over a one-brief corpus, "retrieval by task wording" (§1.5) is vacuous (O1) |
| FastMCP tools | mostly | No Arrow reader among the dependencies. Hits carry neither status nor relevance nor coverage. An unknown `library` has no defined behaviour (F9). |

## 2. Authority and lifecycle (reconstructed)

Cells marked *invented* are ones that DESIGN plus the ADRs do not decide.

| Concept or fact | Identity | Authority | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| Table contracts, codebooks | Arrow `Schema`, `Int16` codes | `cpg-schema` (§B2) | schema digest per table in the `snapshots` row (§6.1) | code change plus insta snapshot (ADR-0008); the crate is empty | Delta schema. Bundle IPC schemas. The Python loader's expectation: *invented*, with no route from Rust (F9) |
| Release | `release_id` (§3.4.1) | acquisition manifest (§4.0) | *invented*: not in `run_id` or `content_digest` (F6) | re-acquire | the `node_id` prefix |
| Context, producer, run | `context_id`, `producer_id`, `run_id` | the `provenance` family | per run | a new run | — |
| Extracted facts | `node_id`, `fact_id` | family tables (§3.2) | `snapshot_id` plus the Delta version | append-only | `nodes`/`edges` views; projections. Which producer owns which table or column: *invented* (F4) |
| "Public" | — | *invented*: two producers, no selection rule (F4) | — | — | Pass A seeds; the §8 exports check; §12(a) |
| Subsystem boundary | — | *invented*: defined only through gold families (F5) | — | — | the Pass A bound; the community universe |
| Findings | `finding_kind`; `finding_id`: *invented* | `findings` table | snapshot | a compiler run (whether it is a `runs` row: *invented*) | assertions |
| Assertions | `assertion_kind` (deferred); `evidence_status` | `assertions` table | snapshot | template synthesis | bundle `assertions` |
| Evidence | *invented*: no table and no id domain (F3) | — | — | — | bundle `evidence` (F1) |
| Briefs | `brief_id`: *invented* derivation (F6) | `briefs` | snapshot | synthesis | bundle `briefs`, `symbol_map`, embedding text |
| Embedding spec | `spec_hash` | Rust compile side; the Python query side's copy is *invented* (F9) | per generation | spec change | bundle `embedding_spec` |
| Vectors | `spec_hash + input_hash` | *invented*: "stored in the bundle", which is derived (F1) | not in `content_digest` (F6) | a vLLM call on a cache miss | bundle `vectors`; kNN and doc-link findings (increment 3) |
| Published snapshot | `snapshot_id` (random) | the `snapshots` Delta append (§6.1) | that commit | a new attempt | readers |
| Serving generation | generation key: *invented* | `generations/<key>/` plus `MANIFEST.json` | names the snapshot | rebuild; switch the symlink | server lifespan state |
| Analytics config | its digest | the versioned config (§9) | in `run_id`, but *which* run is *invented* (F6) | pre-registered | the method parameters on findings |
| Gold reference | `authoring_sha256` | the fastmcp skill | evaluation only | — | the §12 extract |

**Deliberately opaque behaviour.** Each of these has a contract. The vLLM one lacks a persistence
contract (F1).

| Behaviour | Contract |
|---|---|
| Pyrefly resolution | report-level: fields, 14 reasons, sorting |
| vLLM numerics | declared nondeterministic; captured through the cache |
| leiden-rs | seeded; `rand` pinned; ablated |

**Identity behaviour.**
- An identical rerun gives the same `node_id` and `fact_id` with a new `snapshot_id`. Correct.
- A new analyzer revision gives new `fact_id`s. Correct.
- Renaming a file changes `node_id`s. That is declared (release-scoped paths), so acceptable.
- A changed docs tarball changes neither `run_id` nor `content_digest`. Wrong (F6).
- An overloaded method has three declarations under one access path, and no seed rule (F3).

## 3. Contracts and invariants

| Invariant | Enforcement boundary | Failure behaviour | Gap |
|---|---|---|---|
| Unique `(snapshot_id, key)` | §8 `GROUP BY … HAVING` | publication rejected | — |
| Foreign references | §8 `LEFT ANTI JOIN` | rejected | — |
| Coverage completeness | §8, family × module | rejected | — |
| Assertions cite existing findings **and evidence** | §8, §10.4 | rejected | **evidence has no table to join against**, so the rule can't be written (F3) |
| Every public symbol in a brief exists in `exports` | §8 | rejected | which producer's exports? It passes under either reading (F4) |
| No assertion's status exceeds its evidence | §10.4 | rejected | there is no kind → section/status declaration and no combination rule. Outcome step 3 contradicts the rule (F2) |
| Publication only after validation | §6.1 ordering | nothing published | an Err on the append itself (F8) |
| Readers pin versions and filter `snapshot_id` | §6.2 | — | no mechanical oracle yet (§9) |
| One generation per process | §11.3 lifespan | — | — |
| Query and document vectors share one spec | §B14 conformance vectors (at test time); model-name check | response rejected | no runtime `spec_hash` check at the server (F9) |
| The gold is never an input | §1.4, prose | — | no mechanism, and the subsystem is defined through the gold (F5) |
| Determinism | §5 sorted adjacency; §9.8 | test | the rerun oracle is keyed on an incomplete digest (F6) |

**Absence and uncertainty.** The lattice is strong:
- five coverage statuses;
- four resolution statuses;
- nine boundary reasons;
- `unresolved` assertion slots;
- a nullable Boolean where null means unknown, not `false`;
- omitted report fields take their documented default.

Two places collapse states the caller needs to tell apart:
- **(a) Search hits.** A hit carries `outcome` but no status (§11.3 L1006). An `unresolved` or
  `statistically_derived` Outcome looks the same as a `documented` one.
- **(b) An unknown library.** `search_capabilities(library=…)` for a library that isn't served has
  no defined result, so an empty hit list can mean "no match" or "not served" (F9).

**What the design decides well** (what would break without it):
- **§6.2 step 2** (unloaded builder plus a version assert). Without it, `delta.open.2` silently
  reads the latest version.
- **§6.1 "retry = new `snapshot_id`".** Without it, `delta.commit.1` lets a retry duplicate rows
  under one id.
- **§5 sorted adjacency.** Without it, witness choice follows petgraph's newest-first order.
- **§3.6 `ifCalled` → `potential` on a `Reference`.** Without it, Pass A traverses callables that
  are never invoked.
- **§4.2 "Glean pairs never used for `calls`".** Without it, unresolved calls vanish.
- **§11.1 "never send `dimensions`".** Without it, vLLM rejects the request (verified).

### Focus 3: can the synthesis rules be enforced?

**The rule as written can't be enforced mechanically.** "Control", "limit" and "behavioral claim"
are not typed. Assertion kinds are deferred (§3.5 L418). An assertion supported by findings with
different statuses has no defined status, and a finding computed over a statistically chosen scope
has no inherited status.

**It is also internally inconsistent.** Outcome step 3 publishes a behavioural statement (the
Outcome, "what this built-in mechanism lets the caller accomplish", IP L1705) chosen by embedding
similarity.

**It becomes enforceable with three declarations and one query:**
1. Per `assertion_kind`, `cpg-schema` declares the brief section and the permitted
   `evidence_status` set.
2. An assertion's status is the weakest status among its supporting findings *and* the findings
   that defined their scope.
3. Outcome step 3 is either dropped or turned into a doc link, which the rule already permits.

The oracle is then a DataFusion validator in the §8 library returning zero rows, plus a negative
fixture (F2).

**The Outcome order itself is deterministic given its inputs, but step 2 is not decidable for the
pilot.** The FastMCP 4.0.3 docs/examples/tests corpus contains **0** occurrences of `FastMCP.tool`
and **1,344** of `@mcp.tool`/`mcp.tool(`. Neither DESIGN nor the ADRs defines "explicitly
mentions":
- An exact-qualified-name rule never fires for FastMCP methods.
- A bare-attribute rule (`.tool`) fires almost everywhere.
- An instance-typed rule needs receiver resolution over the docs code, which is not specified.

## 4. Derivation and execution (the increment-1 path)

| Stage | Output contract | Effects | Gap |
|---|---|---|---|
| A. acquisition and context (§4.0) | releases, distributions, contexts | reads wheels and a tarball by sha256 | the analysis venv's lock isn't distinguished from the project lock, which pins fastmcp **4.0.5** (F10) |
| B. extract (§4.2) | family batches: Ruff in-process; Pysa and `coverage report` as subprocesses | subprocess; filesystem | per-table producers (F4); Pyrefly's ambient interpreter and search paths (F10) |
| C. provider identity | `provider_node_map`; unmapped rows go to `boundaries` | — | — |
| D. relations | family tables; views | — | the views have no increment-1 reader (F12) |
| E. invocation projection plus Pass A (§5, §9.1) | findings with witnesses | — | seeds and witnesses (F3); the subsystem bound (F5) |
| F. templates (§10) | assertions, briefs | — | the section/kind contract (F3); the status rule (F2) |
| G. validate → `snapshots` append → bundle → smoke → activate | a published snapshot; a generation | Delta commits; network (vLLM); filesystem; symlink | bundle source and order; vectors (F1); append Err (F8) |
| Serve (§11) | `SearchResult`, `Capability` | network (vLLM), declared through degraded mode | Arrow reader and schema check; hit fields (F9) |

**Relationship structures.**
- The invocation projection keeps call, `potential` and `synthetic` relations apart (DM-34).
- The community multiplex deliberately sums relationships with different meanings (calls, co-use,
  types, co-mention, kNN). That is acceptable because its output is `statistically_derived`,
  provided the status survives downstream use (F2).

**Coherent publication.** Decided for the canonical store. Undecided for the bundle (F1).

## 5. Representative journeys

**J1: interruption at publication.** The attempt validates, and the compiler writes the bundle from
its in-memory batches. §6.4 says only that "publication also writes" it, which permits this. Then
the `snapshots` append returns Err. By `delta.commit.1`, the row may already be visible. §6.1
aborts and retries with a new `snapshot_id`. Two outcomes are possible:
- The first attempt is **published but recorded as aborted**, with no bundle of its own. The
  retry publishes a second snapshot with the same `content_digest` (F8).
- If the smoke test passed before the Err, `generations/active` can point at a bundle whose
  snapshot's publication is unknown. The server reads files only, so it cannot tell (F1).

**J2: meaningful change (ablation).** The operator disables community detection, as §9.8 does.
The analytics-config digest changes. Under §3.4.1 and §9 (L722–L723), that digest is in `run_id`.
- If assertion and brief IDs derive from that run, every ID changes, and the "diff of published
  assertions" reports total change.
- If they are content-derived, the diff is meaningful.

Nothing decides which (F6). Either way, the keep/remove rule is "changes something" (§9.8) or
"shows gain" (§9.4). There is no metric for gain that isn't the gold (F7).

**J3: boundary (Rust → IPC → Python).** `cpg-schema` declares the `briefs` schema in Rust.
`lctx_mcp` "depends on `fastmcp` 4.0.x, `numpy` and `httpx`" (L999–L1000), none of which reads
Arrow IPC, and `uv.lock` contains no `pyarrow` at all. Whatever reader gets added will take the
schema from each file, which §B2 L146–L147 forbids ("no schema inferred … includes the serving
bundle"), unless MANIFEST carries schema digests to check against (F9). The hit's `outcome` then
crosses to the agent without its `evidence_status` (F2).

**J4: ordinary extension (Pass B's `conditional_raise` becomes a Limits assertion, increment 2).**
Needed:
- a new `finding_kind`;
- a new `assertion_kind`;
- a template;
- its brief section;
- its permitted statuses;
- the §10.4 check.

Today the section and status rule would live in template code *and* validator SQL: two
independently edited expressions of one fact (DM-02). With the per-kind declaration from §3, the
extension is one `cpg-schema` declaration, one template and a fixture, which is charter §E's shape.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **unresolved** | §B12 declares the right hierarchy. But vectors exist only in the derived bundle (L354, L969–L972), `evidence` has no canonical table (§3.2 vs §6.4 L641), the bundle's derivation source and order are open (§6.4), and the bundle schema's authority does not reach Python (§B2 vs §11.3). Question 1 (trace every assertion to a run) is open: no ID rules for findings, assertions or briefs. | F1, F3, F6, F9 |
| **G2** Semantic fidelity | **unresolved** | §10.2 L890–L891 forbids statistical behavioural claims, and §10.3 L910 fills the Outcome statistically. Of the two readings, one fails G2: a statistical Outcome shown in hits without its status (L1006). There is no status combination or propagation rule, and FCA over a Leiden community is labelled `structurally_observed` (L817, L825). "Public" has two producers (F4), and `model` has no domain. | F2, F4 |
| **G3** Validity | **unresolved** (narrow) | Every other invariant has a named boundary and a rejection outcome (§3 table), which is an improvement on the baseline. But "assertions cite existing evidence" (§8 L691, §10.4 L919) has no target domain, so a dangling evidence id can reach `get_capability` hydration. The server's load-time validation of IPC files is unstated. | F3, F9 |
| **G4** Hidden behaviour | **unresolved** | "The subsystem" bounds Pass A (L734) and the community universe (L792), yet it is defined only as "covers gold families …" (L79–L81): an undeclared path from the gold into the compiler (ADDENDUM Q15). The §4.0 claim that "nothing ambient can change an answer" rests on disabling config discovery, while Pyrefly also queries an interpreter on `PATH`, applies search-path heuristics and can walk up for fallback paths (pyrefly-ruff `cli.tsv`). | F5, F10 |
| **G5** Consistency and recovery | **unresolved** (narrow) | The canonical protocol is decided and matches the skill evidence (baseline F3 resolved). Open: an Err on the `snapshots` append itself, where the skill says to reconcile and §6.1 says abort (F8), and whether the bundle is written after the append and from the published snapshot (F1). ADDENDUM Q9 needs the generation to name a *published* snapshot. | F1, F8 |
| **G6** Transformation and reuse | **unresolved** | Projection and traversal determinism is decided and interface-checked. But `run_id`/`content_digest` omit the release and the vector store, so §9.8's rerun oracle compares unequal inputs. The ablation diff has no defined equivalence. The server does not check the query spec against the generation's `embedding_spec` at runtime (Q12). | F6, F9 |
| **G7** Truthful capability claims | **unresolved** | §B9/§7 **Tested** holds: `just check` (family_smoke 2/2) and `just deps` both passed in this session. The Interface-checked claims I spot-checked hold (§1, Method). But the serving package as specified cannot read its bundle (F9). `coverage report --public-only` and Ruff both claim `exports` with different definitions of public (F4). §1.5's "successful retrieval by task wording" is vacuous over increment 1's one-brief corpus (O1). | F4, F9, O1 |

No gate failed outright. Each one is unresolved because a decision is missing. None is a decision
that was made and loses meaning. The one exception is the §10.2/§10.3 pair, where one of the two
readings fails G2.

## 7. Findings

These are in severity order. F1–F5 are authority and correctness gaps on in-scope behaviour.
F6–F10 are correctness gaps in keys, publication and boundaries. F11–F12 are proportionality and
record-keeping. Instance lists follow the table.

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| F1 | **The serving bundle is declared a derived projection, but its derivation is unspecified, and two of its contents have no canonical source.** | DM-02, DM-23, DM-14, DM-32, DM-48 · G1, G5 | §B12 L257: "a derived, immutable, rebuildable projection". §6.4 L639: "Publication also writes a serving bundle", with no source (Delta read-back or in-memory batches) and no order relative to the append. §3.3 L354: vectors are "not in Delta (bundle only)". §11.1 L969–L970: vectors are "stored in the bundle, and recorded as run inputs, because vLLM numerics vary with batching", but where the cache lives and how long it lasts are unstated. §6.4 L641 lists `evidence`, which the §3.2 findings family (L335) has no table for. | (a) J1: an activatable generation names a snapshot whose publication is unknown. (b) Deleting and "rebuilding" a generation re-requests vectors, which come back different, so rankings change. From increment 3, kNN and doc-link findings in Delta (§9.7) depend on vectors held only in a derived artifact, so the canonical store can't be replayed. (c) `evidence` computed at bundle time is a second authority for an assertion's support. | Build the bundle **only** after the `snapshots` append, by reading the published snapshot at its recorded versions: one derivation path. Persist vectors as a captured observation, either as a Delta table keyed by `(spec_hash, input_hash)` (`Binary` with a width check, mirroring §3.3) or as a declared cache store with its own manifest. Source `evidence` from a canonical table (F3). About 10 DESIGN lines, plus a sentence in ADR-0009 and ADR-0010. | **None exists.** Test: rebuild a generation from Delta with the vector store present and assert byte-identical IPC files. Test: an injected Err after the bundle build leaves nothing that can be activated. |
| F2 | **The `statistically_derived` restriction is not closed.** Outcome step 3 contradicts it. There is no rule for combining or propagating status. FCA over a community relabels statistical scope as structural. Search hits drop the status. | DM-08, DM-59, DM-42, DM-46 · G2, G7 | §10.2 L890–L891: statistical output "may never state … a behavioral claim". §10.3 L910: Outcome step 3 is "the nearest passage by embedding … (`statistically_derived`)". §9.6 L817 and L825: FCA objects are "the public APIs of one community", yet its output is `structurally_observed`. ADR-0011 L72 says "All three outputs are `statistically_derived`", which contradicts §9.6. §11.3 L1006: hits carry `outcome` but no status. §10.4 L921 cites the rule with no mechanism. | (a) An entry point has no docstring and no explicit doc mention. A passage about a sibling API clears the margin and becomes the Outcome. §10.4 either rejects it, which makes step 3 dead code, or passes it, and the hit shows it unlabelled. (b) Step 3 fills `unresolved` Outcome slots, which is **the very count the §B11 gap metric uses** to decide whether an LLM is needed, so the metric is masked. (c) "Every API in community C accepting `tags` also accepts `meta`" is published as structural while C's membership moves with the seed. | Publish the step-3 result as a doc link, which the rule already allows, and leave the Outcome `unresolved`, or drop step 3. In `cpg-schema`, declare each `assertion_kind`'s brief section and permitted statuses. Define an assertion's status as the minimum over its supporting findings and their scope-defining findings. Add `evidence_status` to hits. Fix ADR-0011 L72. | **None exists.** A §8 DataFusion validator (assertions ⋈ kind policy ⋈ supporting findings) returning zero rows. A negative fixture: a template that emits a limit from a statistical finding fails. A fixture where FCA over a Leiden community yields `statistically_derived` implications. |
| F3 | **The increment-1 path from Pass A to a brief is specified at the level of names, not contracts.** See (a)–(f). | DM-01, DM-09, DM-17, DM-22, DM-43 · G2, G3, G7 | Instances (a)–(f) below | For the only increment-1 brief, `FastMCP.tool`, one implementer seeds from three declarations and renders delegation under Public access. Another seeds from the implementation only and puts delegation in Evidence. A third leaves the Outcome `unresolved` because no increment-1 family carries docstrings. §1.5's "delegation" end-to-end example, and the first data point of the gap metric, differ by implementer. ADR-0004's own revisit trigger ("facts outside the declared v1 slice") is already foreseeable from the text. | Settle (a)–(f) in about 15 lines across §1.2, §3.2, §9.1 and §10.3. | **None exists.** Test: an insta snapshot of the rendered `FastMCP.tool` brief from the pinned release, reviewed once. Test: the append-only codebook test for `assertion_kind`. Test: the §8 evidence FK rule on a fixture with a dangling id. |
| F4 | **Increment-1 families have two producers each, with no per-table assignment and no rule for which one a consumer reads; "public" has two definitions.** | DM-02, DM-13, DM-41, DM-43, DM-06 · G1, G2, G7 | §4.2 L516–L519: `exports` comes from Ruff (`__all__`, aliases) *and* from `coverage report --public-only` ("reachable from public modules via re-export chains", `cli.tsv` L160). `signatures` comes from Ruff (default and annotation text) *and* Pysa (kind, requiredness), while §3.2 L328 puts all of these in one `parameters` row. `arguments` (L329) has no increment-1 producer in §4.2. §B6: facts are "never collapsed into mutable node properties". §9.1 step 1 has no selection rule. `model` (L184–L185) has no domain. Pysa's 14 reasons (L444) have no codebook. | Seeding Pass A from Pyrefly's public set versus Ruff's gives different seeds, and §8's "public symbol exists in `exports`" passes both ways. One implementer writes half-null `parameters` rows per provider, another writes one merged row with unclear run and lineage. Pass B and the Important-controls section then read different data. | Assign each increment-1 **table** to one producer, or declare the merged row a Stage-D relational derivation that carries both `fact_id`s. Define "public" once, for example Pyrefly reachability as the definition with `__all__` as corroboration, and record a disagreement as a `boundaries` row. Give `model` and the Pysa reasons codebooks in §3.5. | **None exists.** Test: a fixture package with an `__all__`/re-export disagreement asserts one public set and a `boundaries` row. The ADR-0008 insta snapshots would guard the column split. |
| F5 | **"The subsystem" bounds the compiler but is defined only through the gold.** | DM-28, DM-31, DM-59 · G4 | §1.4 L79–L81: "the server-components surface, which covers gold families `fm.register`, … (about 30 operations)". It is used as the Pass A bound (L734) and the community universe (L792). §1.2 row 1 picks the entry point "↔ gold `fm.register`". The prohibition (L82–L84) has no mechanism. The same tarball corpus the compiler needs already sits under `.claude/skills/fastmcp/build/acquired/`. | The natural precise source is `content/capabilities/fm.*.json` `operations`. If the seeds or the module set come from it, §12(a)'s Jaccard scores the compiler against the list that chose its seeds, and editing the gold changes compiler output with no change to `content_digest`. | Declare the subsystem in the pre-registered analytics config as public roots or module prefixes of the library, with its digest in `content_digest`. State that the increment-1 seed is a hand-registered config entry. One sentence in §1.4 and one in ADR-0004. | **None exists.** `ast-grep` rule in `rules/` flagging `.claude/skills` path literals under `crates/` and `python/`, with fixtures. Test: the acquisition manifest and analytics config name no skill path. |
| F6 | **Identity and reuse keys are incomplete for comparison and ablation.** | DM-12, DM-15, DM-31, DM-32, DM-48 · G6, G1 | §3.4.1: `run_id` = context, producer, families, analysis-config digest (L391). `content_digest` = runs, compiler build, analytics config, spec hash (L394). Neither includes `release_id` or the vector store. There are no ID rules for findings, assertions or briefs. §9 L722–L723 puts the analytics-config digest in "`run_id`" without saying which run. §9.8 L842: "reruns with the same `content_digest` give identical output". | (a) A re-acquired tarball with changed docs gives the same `content_digest`, so the rerun oracle compares different inputs. A vector-cache miss breaks "identical output" on the same digest. (b) J2: whether an ablation diff is a join or a total change depends on an undecided ID rule. (c) `capability_id` stability across generations is undefined. | Put `release_id` (or the acquisition-manifest digest) in `run_id`. Derive `finding_id`, `assertion_id` and `brief_id` from content (subject `node_id`, kind, canonical payload), excluding config digests. State that the analytics config belongs to the compiler's own run. Add the vector-store digest to `content_digest`, or scope §9.8 to cache hits. | ADR-0007's proptest and `run_id`-sensitivity test cover the rest. **Missing:** a test that changing the tarball sha changes `content_digest`, and a test that identical findings keep their IDs under two analytics configs. |
| F7 | **The ablation keep/remove rule, which is the proportionality control for analytics-first, is either vacuous or circular.** | DM-58, DM-59, DM-57 · G4 (Q15, partly) | §9.8 L844 and §1.2 L52: kept if it "changes published output". §9.4 L797: "only if the ablation shows gain". §1.4 L83–L84: parameters "not tuned against the gold". §12(b): the gold is a "dev set only". The named consumers of communities ("seed selection and grouping", L783) and of `page_rank` ("orders seeds", L806) have **no field** in §10.3 or §6.4, with ~30 operations and 15–25 briefs. | Under "changes", community detection survives through FCA scope whether it helps or hurts, or is removed because seeds and grouping never reach the bundle. Under "gain" scored on the gold, choosing techniques is tuning on the gold, and §12(a)–(c) stop being an unbiased evaluation. Either way, ADR-0005's "techniques must earn their place" never operates. | One rule in §9.8: keep a technique if it changes published output **and** improves a pre-registered dev metric (§12(b)) without lowering (a) or (c). Keep increment-5 held-out tasks as the unbiased check. Give grouping a published representation, or drop it as a consumer. | **None exists.** A `just` recipe (e.g. `just ablate <technique>`) that emits the semantic diff and the dev metrics. |
| F8 | **An Err on the publication commit itself is classified as an abort.** | DM-30, DM-14 · G5 | §6.1 L606–L608: "Any write error aborts the attempt … (`delta.commit.1`)". The skill's `delta.commit` card says: "on an ambiguous error, independently reload and compare committed state". | The first attempt is published but logged as aborted, with no bundle. The retry publishes a duplicate with the same `content_digest`. Readers are not harmed (both snapshots were validated), but the attempt record is wrong (J1). | After an Err on the `snapshots` append only, re-read `snapshots` and classify the attempt as published or unpublished before retrying. One sentence in §6.1 and ADR-0009. | **None exists.** Test: inject an error after the commit on the append (the skill probe `operation_error_can_follow_a_durable_commit` shows this can be injected at this pin) and assert the attempt is classified as published. |
| F9 | **The serving package as specified cannot read its bundle, and the bundle's schema contract has no route into Python.** Hits also omit relevance, coverage and status. | DM-43, DM-02, DM-52, DM-42, DM-08, DM-31 · G7, G1, G2, G6 | §11.3 L999–L1000: the dependencies are `fastmcp`, `numpy` and `httpx`. `uv.lock` has no `pyarrow`. §6.4 L640 requires declared schemas "from `cpg-schema`" (Rust). §B2 L146–L147: no inferred bundle schema. ADR-0010 L38 counts "a pyarrow-24 pin" as a cost of the *rejected* option. §11.2 L984 promises "relevance and coverage information", but the §11.3 hit fields lack them. There is no runtime `spec_hash` check (§B14 relies on test-time conformance vectors). An unknown `library` has no defined result. | The implementer adds an unpinned reader and takes each file's own schema, which §B2 forbids, or hand-writes a Python schema copy, which is a second authority. ADR-0010's option comparison is off by exactly the item it charged the rejected option. `library="pyarrow"` returns empty hits that look like "no match". A server started against a generation built with a different spec embeds queries with its own template. | Name and pin the Python Arrow reader (pins.md). Put per-file schema digests from `cpg-schema` in `MANIFEST.json` and have the lifespan reject any mismatch. Check the query client's `spec_hash` against the generation's `embedding_spec` at startup. Add status, relevance and coverage to hits. Make an unknown library a `ToolError` or an explicit mode. Re-run ADR-0010 option 2's cost comparison. | **None exists.** Test: `fastmcp.Client(mcp)` against a fixture bundle (ADR-0010's spike), plus a schema-mismatch fixture and a spec-mismatch fixture that must fail at lifespan. |
| F10 | **Pyrefly's ambient inputs go beyond config discovery, and the analysis venv isn't separated from the project lock.** | DM-28, DM-31, DM-48 · G4 | §4.0 L486–L488 relies on disabling config discovery. Pyrefly also has `--python-interpreter-path`, `--fallback-python-interpreter-name` ("available on your PATH"), `--skip-interpreter-query`, `--conda-environment`, `--disable-search-path-heuristics`, `--enable-fallback-search-path` (walk-up) and `--use-ignore-files` (pyrefly-ruff `cli.tsv`). §4.0 says "a uv venv built from a lock", but the project `uv.lock` pins `fastmcp` **4.0.5**, while the analyzed release is 4.0.3. | On two machines with different `python` on `PATH`, `mcp` types resolve differently, Pysa targets differ, and `context_id` is identical. Reusing the project venv puts fastmcp 4.0.5 in site-packages next to the 4.0.3 wheel. The ordered search paths make this detectable, but nothing prevents it. | The generated config sets the interpreter path to a **separate** analysis venv locked at 4.0.3's dependencies (or skips the interpreter query and gives `site-package-path`), disables the heuristics and walk-up, and fixes ignore files. Record the `pyrefly dump-config` digest in `contexts`. Add this to ADR-0006's spikes. | **None exists.** Test: `pyrefly dump-config` under the generated config gives identical output with `PATH` and `VIRTUAL_ENV` perturbed. |
| F11 | **An unrecorded departure: the research input's truth review before publication was dropped.** | DM-59 | IP L1965 ("inspect the published assertions … directly"), L2631–L2634 ("Evidence review"; "manually inspect the initial small corpus before publication") and L3079 ("~15–25 **reviewed** briefs"). DESIGN §10.4 is "mechanical" only, and §1.2 row 3 dropped "reviewed". DISPOSITION marks L1949 and L2605 as adopted or adapted without mentioning it. | An extractively selected Outcome or limit (a doc passage's lead sentence that mentions the entry point but describes another API) passes every §10.4 check and is published as `documented`. Nothing checks that it is true of the entry point. | Restore a one-pass manual review of the increment-3 corpus, recording `manual_review` (already in the codebook), or record the departure in §10.4 and DISPOSITION with its reason. | Prose (the DISPOSITION row). There is no mechanical oracle for truth review, **and none exists**. |
| F12 | **Over-construction: the materialized `nodes`/`edges` views have no increment-1 reader, and DESIGN names two sources for the projection.** | DM-58, DM-23, DM-02 | §3.2 L320–L322 says the views are generated, and "Graph projections (§5) are built from these views". §5 L559 says the projection "is built by joining call-site ownership with call targets". ADR-0008 L62 requires the views to be regenerated. | Increment 1 builds and maintains a view generator with a regeneration contract and no reader. Either the invocation projection is built twice, once from the views and once from family joins, and the two can drift, or one path goes stale. | Keep the family → node/edge **mapping**, which endpoint-kind validation (§8) consumes. Defer materializing the views until a pass reads them, and fix the §3.2 sentence. | None needed: this removes machinery. |

**F3 instances.** Each is the one sentence two implementers would read differently.
- **(a) Delegation has no brief section.** §10.3 L897–L905 maps only `public_alias` (to Public
  access) from Pass A. `direct_delegation` and `bounded_delegation_path` (§9.1 L737–L738), which
  carry the "it already coordinates …" insight (IP L1766–L1775), have no section.
- **(b) Assertion kinds and templates for increment 1 are deferred** "as their consumers land"
  (§3.5 L418), and increment 1 is that consumer.
- **(c) Docstrings.** Outcome step 1 is "the entry point's docstring summary line" (L908).
  Docstrings reach only the `docs` family (§4.2 L516), which arrives in increment 3 (§3.2 L334).
  `declarations` has no docstring column (L327). `FastMCP.tool` does have one ("Decorator to
  register a tool.", `server/server.py` in the fastmcp skill).
- **(d) Evidence ids have no domain.** They appear in §10.2 L873, §10.3 L905, §8 L691, §10.4 L919
  and the bundle (L641), but not in §3.2.
- **(e) Overloads.** `FastMCP.tool` has two `@overload` stubs plus an implementation
  (`server/server.py` L1809–L1851). §3.4 keeps redeclarations distinct. §9.1 says only "from each
  seed". Neither the seed rule nor which signature is public is stated.
- **(f) Witness selection.** "Explicit BFS with parent pointers" yields one path per vertex, while
  "≤ 3 witness paths per target" and "parallel call sites are preserved" (L562) need several. How
  alternatives are chosen, and whether parallel arcs each count as a witness, is unstated. IP
  L2352 says "a shortest witness and a bounded number of alternatives".

**Observations.** None of these moves a gate.

| # | Observation | Correction | Oracle |
|---|---|---|---|
| O1 | Increment 1 has one brief, so "successful retrieval by task wording" (§1.5) passes for any query | Add distractor briefs for 2–4 non-gold entry points, and require the gold `task_aliases` to rank `fm.register`'s brief first | test (none exists) |
| O2 | §1.2 row 1 lists the families without `findings`. §3.2 L335 and ADR-0008 include `findings` in increment 1 | Add it to the row | prose |
| O3 | ADR-0011 L72 ("all three outputs are `statistically_derived`") contradicts §9.6 L825. §9.4 L797 ("gain") contradicts §9.8, §1.2, ADR-0005 and ADR-0011 ("changes") | Fold into F2 and F7 | prose |
| O4 | Review cadence is stated three ways. ADR-0001's Decision says deep at every increment. Its amendment, ADR-0004 and AGENTS.md say deep after 1/3/5 and compact after 2/4. The design-review SKILL.md cadence table and `design-reviewer.md` still say deep at every increment. The ADR-0001 amendment records a *decision* through a channel `scripts/adr.py` L47 reserves for "factual corrections". And an accepted ADR defers to a proposed one. | Update the skill table and the agent description once ADR-0004 is accepted, and keep amendments factual | `just lint-agents` could check the cadence string (not today) |
| O5 | DESIGN states eight `proposed` ADRs as current truth without marking which sections wait on a spike (the baseline O1 pattern) | Mark "(pending spike, ADR-NNNN)" on §4.2, §6, §9.4 and §11.1, or have `just adr index` list them | `just adr lint` could flag a `> Decision:` line that cites a proposed ADR with spikes (not today) |
| O6 | Stale records: - `STATUS.md` still says increment 1 is the fact substrate and ADR-0003 is open; - the `pyproject.toml` description says "No product Python in stage 1"; - `fastmcp>=4.0.5` has no upper bound, while pins.md says 4.0.x | Refresh at handoff; add `<4.1` | prose |
| O7 | DESIGN is at 1,083 of about 1,100 lines before any increment-2 content | Move probe-level detail (§6.1–§6.3 `delta.*` notes, §5 petgraph notes) into the table contracts or the skills, or budget by ADR | prose |

**Applicability.**
- **Bore on this scope:**
  - Group 1: authority of the bundle, vectors, evidence and "public".
  - Group 2: absence, `model`, status.
  - Group 3: IDs and publication.
  - Group 4: DM-17 assertion templates and DM-16 kind declarations (J4). DM-20 is satisfied
    (§8 is read-only).
  - Group 5: the synthesis contract and status propagation.
  - Group 6: publication failure.
  - Group 7: keys and vectors.
  - Group 9: Pyrefly, the FastMCP and Python boundary.
  - Group 10: lineage and replay.
  - Group 11: named oracles.
  - Group 12: ablation, removal and falsifiability.
- **Group 8:** DM-40 (determinism) bore and is well handled in §5. DM-39 is **not applicable**:
  DESIGN makes no quantitative performance claim. ADR-0010's "trivially cheap at corpus size" and
  ADR-0006's decode-time spike are stated as hypotheses or spikes, not results.
- **§13 deferred material:** not applicable, because it is out of scope by construction.
- **Maturity scores:** omitted. This is document-stage, and the gates are unresolved.

## 8. Alternatives and leverage

| Alternative | Duplication and locality | Risk | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| Baseline spine (reviewed this morning) | Fact substrate with no product | No consumer for any table (ADR-0004 option 1) | — | none | superseded |
| Proposed (ADR-0004 … ADR-0011) | One authority per fact is declared. The kind → section/status rule is split across templates and validators (J4). Spec application is written twice (Rust and Python) and held together by conformance vectors. | F1–F10 | 8 ADRs; ~1,100 DESIGN lines | none (hypotheses, labelled) | Keep the direction |
| **Simpler viable alternative** | The same increments and analytics-first direction, plus: (1) build the bundle only from the published snapshot after the append; (2) capture vectors once into a canonical store, so the bundle is a pure projection; (3) use one embedding client (Python `httpx`), called by the compiler as a subprocess like Pyrefly, so the spec is applied once; (4) don't materialize `nodes`/`edges` until something reads them; (5) define the ablation rule before increment 2 | Removes one competing authority (vectors held in the bundle), one duplicated semantic implementation (Rust and Python spec application) and one reader-less generator | It adds a subprocess step. A vLLM HTTP boundary already exists, so the "avoid a language crossing" rationale (ADR-0010 Consequences) buys little. Conformance vectors shrink to a model-drift regression fixture. | none either way | **Recommended for (1), (2), (4) and (5).** (3) is the operator's call: the conformance vectors make two clients safe, but they don't make two clients necessary. |

**Is the analytics scope credible? (focus 4).**
- **Increment 1 is not over-built.** Its only analytic is Pass A, and the analytics-first
  direction costs nothing until increment 2.
- **The cost lands in increments 2 and 3** in these pieces:
  - Leiden with CPM, multiplex layers, hub down-weighting and 10-seed consensus;
  - `page_rank` combined with usage counts, where the combination is undefined and `page_rank` is
    unweighted (verified);
  - FCA, then RCA;
  - kNN;
  - LFR planted-partition fixtures, which test leiden-rs more than our use of it.
- **The subject is about 30 public operations and 15–25 briefs.** For communities and centrality
  the declared consumers have no field in the published output (F7). As specified, the ablation
  would either keep them by accident, because they change FCA scope, or remove them for not
  showing up.
- **So the scope is credible only as an experiment with a working removal rule.** ADR-0005 and
  ADR-0011 have the right guardrails: a named consumer, determinism, the ablation, and removal by
  ADR. F7 is what makes them operate. FCA is the technique best placed to earn its keep here,
  because its "shared controls" and implication consumers map to real brief sections. It has to
  run over a declared subsystem scope, not a community scope, or inherit the community's status
  (F2).

**Abstractions justified by current need:**
- the `snapshots` table (publication);
- the generation directory plus symlink (one generation per process);
- codebooks (typed categories);
- the family → view mapping (endpoint validation).

**Ordinary code:** the ID encoding, the report decoders, Pass A, the templates, the validators,
NextClosure and the retrieval fusion. None of them warrants a DSL or a generator.

## 9. Verification plan

| Claim or risk | Label | Oracle | Conditions and expected result | Current |
|---|---|---|---|---|
| §B9 one dependency family | Tested | `just deps`; `family_smoke` | one version per family crate; a Delta write, then a DataFusion query | **passed** (`just deps`, `just check`, this session) |
| §6 Delta behaviours relied on (§6.1–§6.2) | Interface-checked | deltalake skill claims `delta.commit.1`, `delta.write.3`, `delta.open.2`, `delta.read.2`–`.4`, `delta.storage.4`, `delta.replay.1` | as cited | read this session; FixedSizeBinary → BINARY asserted |
| §5 traversal determinism | Interface-checked (petgraph) · Proposed (our rule) | test: shuffled-input fixture gives byte-identical witnesses | two insertion orders | none exists |
| §11.3 FastMCP surfaces | Interface-checked | `Client(mcp)` in both protocol modes (ADR-0010 spike) | object `structured_content` | none exists |
| Failed attempt invisible (G5) | Proposed | ADR-0009 spike: an injected validation failure leaves no `snapshots` row | the next reader sees zero rows | none exists |
| Publication-commit ambiguity (F8) | Proposed | test: an error injected after the commit on the append | the attempt is classified as published | none exists |
| Bundle is a projection (F1) | Proposed | test: rebuild from Delta plus the vector store | byte-identical IPC | none exists |
| Status rule (F2) | Proposed | §8 validator plus negative fixture | zero rows; the crafted statistical limit is rejected | none exists |
| Increment-1 brief (F3) | Proposed | insta snapshot of the `FastMCP.tool` brief | reviewed once; `INSTA_UPDATE=no` | none exists |
| Public set (F4) | Proposed | fixture where `__all__` and re-export reachability disagree | one public set plus a `boundaries` row | none exists |
| Gold isolation (F5) | Proposed | `ast-grep` rule on `.claude/skills` literals; manifest test | no hits | `rules-scan` **not_run** (no rules yet) |
| Keys (F6) | Proposed | ADR-0007 proptest and `run_id` sensitivity; tarball-sha and stable-id tests | the ids and digests move exactly as declared | none exists |
| Ablation (F7) | Proposed | `just ablate <technique>` | a semantic diff plus dev metrics | none exists |
| Serving read path (F9) | Proposed | lifespan schema-digest and spec-hash mismatch fixtures | the server refuses to start | none exists |
| Ambient Pyrefly (F10) | Interface-checked (flags) · Proposed (our config) | `pyrefly dump-config` invariance test | identical under a perturbed `PATH`/`VIRTUAL_ENV` | none exists |
| Retrieval by task wording (O1) | Proposed | gold `task_aliases` against a corpus with distractors | the right brief is at rank 1 | none exists |

**Cost accounting.** Not material at document stage. The only material costs are the pinned GPU
service for vectors and Pysa decode volume, and both have named spikes.

## 10. Exceptions and unresolved decisions

The design claims no SHOULD deviation. Every gap above is recorded as unresolved, not excepted.

## 11. Decision

**Decision: Not Accept, as a decidable specification for increment 1.** The direction stands: a
capability compiler, programmatic synthesis, a Delta canonical store with a derived serving bundle,
and analytics that have to earn their place by ablation.

**Reason.** All seven gates are **unresolved** and none has failed. Each is a decision not yet
made, and most are one to three sentences. This is a clear improvement on the baseline:
- five baseline items are resolved: F2, F3, F5, F6 and O2;
- three are partly resolved: F1, F4 and O1;
- none is still fully open.

The author has to decide:
1. **What the bundle is derived from, when it is built, and where vectors live** (F1).
2. **Whether Outcome step 3 exists.** Also: a per-kind declaration of section and permitted
   statuses, and a rule for status propagation (F2).
3. **The increment-1 path from Pass A to the brief:** the delegation section, the assertion kinds,
   the docstring source, the evidence domain, overload seeds and witness selection (F3).
4. **One definition of "public", and one producer per table** (F4).
5. **A subsystem definition on the compiler side,** independent of the gold (F5).

After those, sentence-level fixes remain: F6, F8, F9, F10 and F11, plus the F7 rule before
increment 2.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | Bundle built from the published snapshot after the append; vectors in a canonical store; an `evidence` source (F1) | DM-02, DM-23, DM-14 | §6.4, §11.1 and §3.2 lines; ADR-0009/0010 sentences | rebuild-equality test |
| 1 | Close the status rule: step 3 becomes a doc link; the kind → section/status declaration; propagation; status in hits (F2) | DM-08, DM-59 | §10.2–§10.4 and §11.3 lines; ADR-0011 L72 fixed | §8 validator plus negative fixture |
| 1 | The increment-1 synthesis contract, (a)–(f) (F3) | DM-01, DM-22 | §1.2, §3.2, §9.1 and §10.3 lines | insta snapshot of the brief |
| 1 | Per-table producers; one definition of "public"; `model` and Pysa-reason codebooks (F4) | DM-02, DM-06 | §4.2 and §3.5 lines | disagreement fixture; ADR-0008 insta |
| 1 | Subsystem defined in the analytics config (F5) | DM-28, DM-31 | §1.4 line | `ast-grep` rule plus a manifest test |
| 2 | Keys: the release in `run_id`; content-derived finding/assertion/brief ids (F6) | DM-15, DM-31 | §3.4.1 rows; ADR-0007 | tarball-sha and stable-id tests |
| 2 | The serving read path: a pinned reader, schema digests, a spec check, an unknown-library result (F9) | DM-43, DM-02 | §11.3 lines; pins.md row | lifespan mismatch fixtures |
| 2 | Reconcile after an append Err (F8); a separate analysis venv and a pinned interpreter (F10) | DM-30, DM-28 | §6.1 and §4.0 lines; ADR-0006/0009 spikes | the two tests named in §7 |
| 3 | The ablation rule and a published representation of grouping (F7) | DM-58, DM-59 | §9.8 line | `just ablate` |
| 3 | Record or restore the truth review (F11); drop the materialized views (F12); O1–O7 | DM-59, DM-58 | DESIGN and DISPOSITION lines | prose |

### Per-ADR recommendation

| ADR | Is the decision sound? | Accept now? |
|---|---|---|
| **0004** scope, pilot and increments | Yes: a capability compiler, FastMCP 4.0.3, evaluation-only gold, five vertical increments | **Not yet; no spike needed.** Accept in the same commit that defines the subsystem without the gold (F5) and has §1.2 row 1 name the docstring source and the `findings` family (F3(c), O2). |
| **0005** programmatic synthesis | The direction is sound: no LLM in v1, never in the query path, templates plus extractive selection. The statistical rule as written isn't closed. | **Stay proposed** until §10.2 and §10.3 agree (F2) and the §9.8 keep/remove rule is defined (F7). No spike is needed. Its revisit trigger stays after increment 3. |
| **0006** Ruff in-process, Pyrefly 1.3.1 as a subprocess | Yes. The interface evidence is verified (CLI surfaces, 14 reasons, `--public-only`), and superseding ADR-0003 is well founded. | **Stay proposed until its four named spikes pass,** with two added: `dump-config` invariance, and a separate analysis venv at 4.0.3 (F10). |
| **0007** run contract and identity | Yes. A random `snapshot_id`, content-derived ids and a `content_digest` is the right split. | **Accept after amendment, before the first Delta write,** because ids become persistent. Add the release to `run_id` and the finding/assertion/brief id rules (F6). No spike is needed; its named proptest and sensitivity test are the oracles. |
| **0008** family authority, codebooks, coverage | Yes. Typed family tables are the only writable authority, and the views are derived. | **Stay proposed** until slice 1 lands the first `cpg-schema` family with its insta snapshots, and the multi-producer rule, `model` and the evidence domain are decided (F4, F3(d)). |
| **0009** publication protocol | Yes. It is the simpler protocol, grounded in the pinned skill evidence. | **Stay proposed until its named Delta probe passes.** Before acceptance, add reconcile-on-append-Err (F8) and "the bundle is built from the published snapshot after the append" (F1). |
| **0010** agent interface and retrieval | The direction is sound: in-process exact search, LanceDB deferred, one spec. | **Stay proposed until its three spikes pass** and the Python reader, schema digests, runtime spec check and vector persistence are decided (F9, F1). Re-run option 2's cost comparison with the reader counted on both sides. |
| **0011** analytics algorithms | The library choices are plausible, and the traversal determinism rules match verified petgraph behaviour | **Stay proposed until the increment-2 leiden-rs spike,** which is its own and correct timing. Before then, fix L72 (F2) and adopt the F7 rule. |

### Deferred

| Item | Why deferred | Reopen when |
|---|---|---|
| One embedding client instead of two (§8, item 3) | The operator's choice; the conformance vectors make two clients safe | the conformance vectors disagree beyond tolerance (ADR-0010's revisit), or a third client appears |
| The LFR fixture suite versus a small hand-built fixture | It belongs to increment 2 | the increment-2 compact review |
| How `page_rank` is combined with usage counts (`page_rank` is unweighted) | increment 2 | before §9.5 is implemented |
| Concurrent compile attempts | One operator; appends commute | a second compiler process or CI runs against the same store |
| Load-time sha256 verification of MANIFEST files | Covered in part by F9's schema digests | a generation is ever copied between machines |
