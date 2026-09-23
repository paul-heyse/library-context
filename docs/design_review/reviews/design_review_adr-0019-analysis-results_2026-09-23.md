# Design review: ADR-0019 (analysis results) and the Phase 0 amendments (standard)

**Date:** 2026-09-23 · **Depth:** standard. ADR-0019 changes §B6, so a `standard` review is owed
before acceptance (ADR-0001). · **Mode:** document review of commit `e735fde` (ADR-0019; the
ADR-0004, ADR-0010, ADR-0011 and ADR-0017 amendments; DESIGN §1.2, §B4, §B6, §3.2, §3.4.1, §4.1,
§5, §6.1, §6.2, §9.4, §9.5, §11.2, §11.3, §13), read against the code it changes.
**Reviewer:** `design-reviewer` subagent (fresh context). · **Author:** the session executing the
remaining-scope plan (`~/.claude/plans/we-have-not-yet-pure-swing.md`).
**Timing:** nothing of ADR-0019 is implemented at `e735fde`. Slice 1.4 (table groups,
`analytics.toml`, the compiler run, the invocation projection, Pass A) is starting. That makes
findings on table contracts and identity recipes cheapest to act on now, and the priorities in
§11 are ordered for that.

## 1. Decision and scope

**Proposal.**
- ADR-0019 (`proposed`, `evidence: Proposed`) makes analysis results typed tables of a new
  `findings` family, outside the `nodes`/`edges` catalogs, with no `fact_id` column:
  - `analysis_invocations`, `findings`, `finding_members`, `witnesses`, `evidence`, `assertions`,
    `assertion_support`, `briefs`, `brief_assertions`, `brief_members`, `brief_documents` and
    `assertion_policy`;
  - provenance held in each row: run (`lctx-compiler`), model, extraction mode, the invocation
    record;
  - content ids with no analytics-config digest, from the five recipes at ADR-0019 L112–118;
  - an `lctx-compiler` run that declares no families;
  - a language-neutral serving schema digest (SHA-256);
  - crates `lctx-analytics` and `lctx-embed`.
- ADR-0017 amendment:
  - a `snapshot`/`global` read mode per table;
  - `embedding_cache` written by an insert-only MERGE;
  - `row_count` for a global table is the rows used;
  - `content_digest` takes a digest of the keys used.
- ADR-0004 amendment: hybrid retrieval moves into increment 1, and `analytics.toml` is frozen
  before the first gold scoring.
- ADR-0010 amendment: bm25s, `services/vllm`, and the stdio start flags.
- ADR-0011 amendment (in place, still proposed): RBER, our own γ grid, integer aggregation, and
  `compute_flow` as PageRank's oracle.

**Status.** Every ADR-0019 claim is **Proposed**. DESIGN §B6's section label says otherwise; see
F5.

**Observable outcome.**
- Stage E/F results become published, validated, snapshot-qualified relations that the bundle
  reads.
- An ablation diff becomes a join on content ids.
- A review verdict binds to a brief's content.

**Baseline.**
- §3.2 named the `findings` family's tables but gave them no contract.
- §B6 said findings would be `facts` rows.
- No table had a read mode. `register` always selects one commit and filters `snapshot_id`
  (`snapshot.rs:138–153`).
- `schema_digest_of` falls back to the `snapshots` schema for an unknown name
  (`attempt.rs:184`).
- `semantic:source-role-by-run` assumes every run over a release is an extractor run
  (`rules.rs:171–176` at HEAD).

**Supported scope and non-goals.**
- In scope:
  - the table contracts, identity and provenance;
  - the compiler run and the existing rules it meets;
  - the global read mode and MERGE against `snapshot.rs`, `attempt.rs` and ADR-0017's invariant;
  - what slices 1.4–1.9 depend on.
- Checked only where they touch the above: ADR-0010's Python layout, bm25s, ADR-0011's
  algorithms (two source checks, below), and the sealed held-out tasks (D1).

**Constraints and uncertainty.**
- The MERGE concurrency behaviour is unprobed. Its probe is owed in slice 1.6, with a declared
  fallback.
- ADR-0020 (review verdicts) is cited but does not exist.
- The working tree holds another session's uncommitted slice-1.4 edits (see Method).

### Method and coverage

**Read in full:**
- ADR-0019 and ADR-0017;
- the `e735fde` diffs of ADR-0004, ADR-0010, ADR-0011, ADR-0014, the ADDENDUM and DESIGN;
- the plan and the deviation log;
- the charter's DM-11–DM-33, DM-46–DM-60 and §A–§H;
- REVIEW_REFERENCE, ADDENDUM, and the operator's graph guidelines (all of it, §7, §8 and §10 in
  particular).

**DESIGN sections read:** §1.1–§1.5, §B3–§B14, §3.2, §3.3, §3.4, §3.4.1, §5, §6.1–§6.4, §9 (intro),
§9.1, §9.8, §10, §11 and §13.

**Code at HEAD, read in full or at the cited grain:**
- `cpg-core`: `attempt.rs`, `snapshot.rs`, `delta.rs`;
- `cpg-schema`: `table.rs` and `rules.rs`;
- `tables.rs`: provenance, coverage, `snapshots`, the group macros;
- `derived.rs`: every SQL that joins `runs`;
- `codebook.rs`: the provenance codebooks.

**Not read:**
- DESIGN §3.5–§3.8, §4.2–§4.3, §8 and §9.2–§9.7 beyond the lines cited;
- `graph.rs`'s registry;
- `cpg-extract`, and the test suites.

**Working tree (a lead, not the target).** Another session's uncommitted slice-1.4 edits were
present during this review, observed 2026-09-23T14:49:
- `crates/cpg-schema/src/findings.rs`, new;
- diffs to `rules.rs`, `codebook.rs` and `id.rs`.

They are cited only where they show how the ADR's text is being read. Line numbers there may
move.

**Checks run** (each ran, and its output was read):

| Check | Outcome |
|---|---|
| `just adr lint` | **passed** (19 records, 2026-09-23). It checks that `design:` refs resolve to headings. It does not check that a referenced section names the ADR, or that a cited ADR exists (F2, F5). |
| Probe P1 against the pilot, `target/release/lctx query --store build/store --snapshot 1e6de86f764aefd5eba87482eef52be0 "<SQL>"` (read-only). `runs` is extended with a no-family run over the library release (`array_except(families, families)`), and then the committed `semantic:source-role-by-run` SQL is run | **passed** (ran): **275 violations**, every library module. With `cardinality(r.families) > 0` added: **0**. `unnest(families)` yields no rows for the empty list, so the three `coverage:*` rules pass vacuously for such a run. |
| Probe P2, same command. For `call_target` (7) and `site_target` (15) edges ⋈ `facts` (modality) ⋈ `pysa_calls` (phase), count the (src, dst) pairs with several edges, and with mixed modality or phase | **passed** (ran): `call_target` has 68,140 pairs, each with one edge. `site_target` has 30,834 pairs, 70 of them with two or more edges, and every pair has one modality and one phase. |
| `just check` / `just test-all` | **not_run**. The working tree holds another session's uncommitted edits, so running them would test those edits, not `e735fde`. The commit message's `just test-all: passed (2026-09-23)` was not re-verified. |

**Interface-checked at pinned source** (2026-09-23):
- leiden-rs 0.8.1 `RBER` (`quality.rs`: `p = 2m/(N(N−1))`, N the total node weight) and
  `compute_flow` (`infomap.rs:191–231`: uniform teleport, uniform dangling redistribution). Both
  are as ADR-0011's amendment states.
- FastMCP 4.0.5 in `build/envs/fastmcp`, the same version as the serving pin:
  `check_for_newer_version` is called only from `log_server_banner` (`utilities/cli.py:205`) and
  the CLI (`cli/cli.py:132`), so `show_banner=False` alone prevents the PyPI GET.

**Guarantees not attacked, and so only asserted:**
- that an insert-only MERGE commits on an `appendOnly` + CHECK table without rewriting files;
- that two concurrent MERGEs conflict rather than both insert (probe owed in 1.6);
- bm25s scoring and ties;
- the serving digest's type grammar;
- the hybrid-retrieval move (ADR-0004), except its bearing on evaluation (F3);
- ADR-0011's other amendments (γ grid, `run_multiplex`, integer aggregation) beyond the two
  source checks.

## 2. Authority and lifecycle map

Reconstructed from the ADR and DESIGN. Cells marked *(undecided)* are the ones I had to invent;
they are the evidence behind the findings cited.

| Concept | Semantic type and identity | Authority / owner | Revision boundary | Update path | Derived representations |
|---|---|---|---|---|---|
| Analytics config: subsystem, seeds, budgets (policy) | canonical JSON. Its digest is the compiler run's `config_digest` | `libraries/<name>/analytics.toml` (git) | its commit is the pre-registration. "Frozen" before gold scoring: where it is recorded, and what checks it, *(undecided)* (F3) | edit until frozen, then by ADR | the compiler `run_id` → `content_digest`. Per-method `parameters` JSON: which fields each method carries is *(undecided)* (O3) |
| Kind policy (`assertion_kind` → section, statuses) | code in `cpg-schema` | `cpg-schema` | `compiler_digest` | code change | `assertion_policy` table per snapshot; generated rule |
| Method → finding-status policy | *(absent)* (F6) | — | — | — | — |
| Projection spec | code (`cpg_schema::projection`, planned) | `cpg-schema` | `compiler_digest` (its SQL) | code change | `analysis_invocations.projection_digest` only; the spec text is not persisted (O3) |
| Extracted facts (observations) | `facts` + raw tables, `fact_id` per run | the extractor runs | snapshot | raw write, once | catalogs, derived tables |
| Operator review verdict (the one analysis-side input) | `facts` of a `manual-review` run | operator (`reviews.toml`) | raw write | append | `briefs.review_state` (derived). ADR-0020 is *(unwritten)* (F5) |
| Invocation (execution) | content id (method, params digest, projection digest, subject, seed); key `(snapshot, invocation)` | Stage E kernel | snapshot | one commit | findings cite it |
| Finding (result) | content id (kind, subject, related, "canonical payload"). The payload per kind, and finding↔invocation cardinality, are *(undecided)* (F1) | Stage E kernel | snapshot | one commit | assertions via `assertion_support` |
| Finding `evidence_status` | a codebook value chosen by the kernel | the kernel *(no declared policy)* (F6) | snapshot | one commit | assertion status by propagation (§10.2) |
| Evidence | content id (kind, fact, node, module, span); run-scoped through `fact` (F7) | Stage F SQL | snapshot | one commit | bundle `evidence`; its resolved text is a copy of `source_files.text` / `passages` (F8) |
| Assertion | content id (kind, subject, section, case, text, status, sorted supports) | Stage F templates | snapshot | one commit | bundle |
| Brief, and `capability_id = brief_id` | content id (seed, case, sorted (section, ordinal, assertion)). The entity key (seed, case) is unnamed (O7) | Stage F | snapshot | one commit | bundle, `symbol_map`, `brief_documents` → vectors |
| Brief section | stated three times: `assertion_policy`, `assertions.section`, `brief_assertions.section` | *(which is authoritative is undecided)* (O8) | — | — | — |
| Embedding vector | `(spec_hash, input_hash)` | `embedding_cache` (global) | table version recorded per snapshot | insert-only MERGE | bundle `vectors` |
| Compiler run | `run_id(release, context, producer(lctx-compiler, compiler_digest), [], config digest)` | raw write | snapshot | one commit with `runs` | `content_digest` |
| Serving schema digest | SHA-256 over a language-neutral form | ADR-0019 L142–150 **vs** DESIGN §6.4 (F2) | per build | — | `MANIFEST.json`; the Python lifespan check |

**Deliberately opaque behavior.** Pass A's BFS, witness selection and the templates are ordinary
code behind the table contracts, which is the right placement (charter §F). Their output version
is hand-bumped (O4).

**Identity behavior.**
- *Parameter change:* an unchanged finding keeps its id (the intent).
- *Context or producer change:* every evidence, assertion and brief id changes (F7).
- *Candidate ↔ definite change of an arc on a depth ≥ 2 path:* the finding id is unchanged (F9).
- *A second invocation producing the same content:* key collision (F1).

## 3. Semantic contracts and invariants

| Contract or invariant | Representation | Enforcement boundary | Failure behavior | Verification evidence |
|---|---|---|---|---|
| Every analysis table gets key, ref and codebook rules | `for_each_analysis_table!` → `shapes()`/`contracts()` | validation | the compile is rejected | Proposed (the working tree has the wiring) |
| `schema_digest_of` refuses an unknown name | `attempt.rs` | publish | error | Proposed. Today it falls back to `snapshots` (`attempt.rs:184`) |
| The compiler run does not trip extractor rules | `semantic:source-role-by-run` scoped to runs with families | validation | — | **Tested** by probe P1 (ad hoc, not a regression test): 275 violations unscoped, 0 scoped |
| A content id covers every identity-bearing field | five hand-written recipes (L112–118) | none: no SQL recompute, and determinism tests cannot see an omitted field | a collision → the key rule rejects; an omission → silent | Unresolved (F1) |
| One finding per content per snapshot | key `(snapshot, finding_id)`, one `invocation_id` per row (working tree) | `key:findings` | a valid compile is rejected when two invocations yield one finding | Unresolved (F1) |
| Statistical output never states a control or limit | kind policy + status propagation (§10.2) | validator | reject | Proposed. It trusts the finding's own status, which nothing polices (F6) |
| Truncation is distinguished from the requested bound | `completion`, `stop_reason`, `omitted_paths` | none stated | — | Unresolved (F4) |
| Evidence text equals the cited bytes | none | — | — | Unresolved (F8) |
| A snapshot-qualified table has one commit per attempt | `write::<T>` → `append` stamps `lctx.snapshot_id` (`delta.rs:166–190`) | `commit_adds` (`snapshot.rs:99–121`) | `ForeignCommit` | Tested for the existing tables (ADR-0017). Analysis tables keep it (§4) |
| A global table's key is unique over the whole table | `key:embedding_cache` | validation | reject; the declared recovery is `embedding_cache_v2` | Proposed |
| Concurrent MERGEs do not duplicate a key | delta-rs conflict detection | probe (1.6) | fallback: a lock file | Proposed, unprobed |

**Absence and uncertainty.** The design distinguishes:
- `unresolved` assertion slots (null text);
- `incomplete_resolution` findings for unknown targets (§5 `unresolved_sites`);
- `coverage_status` on the invocation.

It does not yet distinguish:
- a requested depth bound from an execution budget (F4);
- "no analysis requested" (no analytics config) from "analysis found nothing" (O9).

**Equivalence requirements.**
- Byte-identical analysis tables for shuffled input, and identical ids across identical compiles
  (L175–176).
- Semantic equivalence of two findings is whatever the recipe hashes. That is F1's point.

## 4. Derivation and execution design

Stages E and F run inside one attempt, after `derive_all` (`attempt.rs:268–274`) and before
`validate_costed`:
- projection SQL on the attempt's session;
- then `lctx-analytics` (Arrow in, Arrow out);
- then `write::<T>` + `register`, with one commit per table;
- then Stage F in SQL and Rust over the registered findings;
- then validation over everything, then publish.

**The compiler run's rows** join the extractor's `runs` and `producers` batches before `write_raw`:
- `write_raw` requires each raw table exactly once (`attempt.rs:143–146`);
- `content_digest` reads run ids from that batch (`attempt.rs:246–251`);
- so the ADR's "go in the raw write" is the only placement that keeps both, and ADR-0017's one
  commit.

**Coherent publication** is unchanged for snapshot-qualified tables. `embedding_cache` is the
declared exception:
- several commits per attempt;
- read at its recorded version over all files;
- `row_count` is the rows used.

**Provider selection.** leiden-rs RBER and `compute_flow` match their amendment text (Method).

## 5. Representative journeys

**Ordinary extension: Pass B's `conditional_raise` (slice 2.1).**
- New codes: a `finding_kind`, an `analytic_method`, a `member_role` if guards are members.
- The `findings.condition_node_id` column already exists in the working tree
  (`findings.rs:86`).
- But neither the ADR's finding recipe (L115: "kind, subject, related node, canonical payload
  (witness steps by node ids, depth, stop reason, omitted flag)") nor the working tree's
  `FindingKey` (`findings.rs:184–206`) hashes it.
- So two guards in one function raising one exception type get one id. The extension needs an
  edit to a hand-written recipe that nothing flags (F1). With identity derived from the contract,
  the column's declaration would carry it.

**Meaningful change: `depth` 2 → 3 in `analytics.toml`, or a Pyrefly bump.**
- Depth change:
  - a new compiler `run_id` and new invocation ids;
  - findings unchanged in content keep their ids, as intended;
  - briefs whose supports changed get new ids, and the ablation diff is a set difference on ids.
  - Pairing "the same capability, changed" still needs (seed, applicable case) (O7).
- Pyrefly bump:
  - every `fact_id` changes (it hashes `run_id`, DESIGN L544);
  - so every fact-citing `evidence_id` changes, and with it every assertion and brief id (F7);
  - every recorded review goes stale, even where no claim changed.
  - Meanwhile a depth-2 path whose arc became definite keeps its finding id (F9).

**Boundary: the serving bundle.**
- ADR-0019 gives the bundle files a language-neutral digest. DESIGN §6.4 still prescribes
  `cpg-schema`'s `canonical_schema` form (F2).
- The evidence text the bundle serves is a stored copy with no check against the bytes it cites
  (F8).

**Interruption: validation fails after Stage F's MERGE.**
- The snapshot-qualified commits stay invisible (ADR-0017).
- The vectors the MERGE inserted stay in the global cache. That is correct for a content-keyed
  cache: a later attempt reuses them only under the same spec hash.
- `--unpublished` inspection of that attempt has no rule for which `embedding_cache` version to
  read (O5).
- A crash while the MERGE's lock-file fallback holds the lock leaves the recovery of a stale lock
  unstated (O5).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 — Authority | **fail** (documentary) | DESIGN is authoritative by construction, and §6.4 L1590–1592 prescribes the bundle digest form ADR-0019 L142–150 rejects. §10.1 L1906 and §9 L1715 are also stale (F2). Otherwise G1 holds: results stay outside the catalogs, so there is no second graph authority; `facts` is untouched; review state is derived from verdict facts | Amend §6.4, §10.1 and §9 (F2). Add the evidence-text rule (F8) |
| G2 — Semantic fidelity | **unresolved** | Requested bound vs execution budget vs presentation cap are not mapped to `completion` (F4, guidelines §7/§10 MUST). Finding status is unpoliced, so ADDENDUM Q6 has a path (F6) | Decide the mapping. Declare a method → status policy |
| G3 — Validity | **pass** (Proposed) | Every new table gets generated key, ref and codebook rules through its group. `schema_digest_of` refuses unknown names. The committed `source-role-by-run` would reject a compiler run (P1: 275), and the ADR scopes it (P1: 0) | Rule cases with a compiler run (plan 1.4) |
| G4 — Hidden behavior | **unresolved** | The subsystem, seeds and distractors are written by an author with access to the gold. "Frozen" has no mechanical record (F3; ADDENDUM Q15). The FastMCP banner GET is closed by `show_banner=False` (verified at source) | Decide distractor selection and authorship before `analytics.toml` is committed |
| G5 — Consistency and recovery | **pass** (Proposed) | Analysis tables are written once each, and the invariant is kept. The global read mode and MERGE are decided, and probe-gated with a fallback. Open points are inspection-only or later triggers (O5, O6) | O5 before 1.6. O6 when a method consumes another's findings |
| G6 — Transformation and reuse | **unresolved** | The identity recipes leave the per-kind payload and the finding↔invocation cardinality undecided (F1). Evidence ids are run-scoped under a "content" label (F7). Modality and phase are excluded on edge_id's reason (F9) | Contract-derived identity. Decide cardinality |
| G7 — Truthful capability claims | **fail** | §B6's new bullets inherit "Implemented and Tested" (DESIGN L10–12, L229–230) and cite a non-existent ADR-0020. "One recipe in Rust and SQL" (L108) has no route for variable-length payloads. The working tree already says "Rust-only" (F5) | Relabel. Mark ADR-0020 as unwritten. Replace the SQL claim |

An unresolved gate is not a pass. No dimension score is given.

## 7. Principle findings

Ordered by severity: correctness and authority, then semantic duplication and extension, then
cost. Each row is one cause.

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| F1 | **A finding's identity is not defined beyond Pass A, and the finding↔invocation relation is not decided.** Two results that should differ can share an id, and one result produced twice cannot be stored. | DM-15, DM-11, DM-12, DM-53 · G6 | ADR L115 hashes "kind, subject, related node, canonical payload (witness steps…, depth, stop reason, omitted flag)". It does not say which node is the subject per kind, and omits the columns later kinds need: the condition node (L72), member roles, cited facts. The working tree's `FindingKey` omits `condition_node_id`, `evidence_status`, `score`, member roles and `cited_fact_id` (`findings.rs:184–206`). `findings` is keyed `(snapshot, finding_id)` with one `invocation_id` (`findings.rs:70–75`). The evidence recipe (L116) has no field that identifies a fixture run. L108 promises "one recipe in Rust and SQL", but `lctx_id` takes scalars only (DESIGN §3.4.1 "Ids in SQL"), and the working tree says "Rust-only: no SQL recomputes them" (`findings.rs:153`) | (a) **1.4:** two seed access paths in `analytics.toml` that resolve to one declaration give identical invocation ids (method, params, projection, subject, no RNG seed), so `key:analysis_invocations` rejects the compile. (b) **2.1:** two `conditional_raise` guards in one function differ only in their condition node, so they collide. Either `key:findings` rejects a valid compile, or a dedup silently drops a limit. (c) **2.3:** ten Leiden seed runs yield one membership, so one finding id carries ten invocations. That is a key failure, or nine invocations silently unlinked. (d) **Increment 5:** two runs of one fixture give one `evidence_id`. The determinism oracles (L175–176) pass in every case | Declare identity **per contract**. Mark each column `identity` or `lineage`, where lineage covers ids like `edge_id`/`invocation_id` and value columns like `score` where the ADR wants a stable id. Hash every identity column, plus child rows (witness steps, members) in key order, with one generic function. Decide the cardinality: rooted analyses carry their root (the seed) as subject, which the working tree's `semantic:witness-chain` already implies. A result several invocations produce is published once (e.g. the consensus) or linked by a `finding_invocations` table. Dedupe seeds by declaration when the config loads. Then either extend `lctx_id` to take lists, so an `id:` rule recomputes these ids from the stored rows, or drop "and SQL" from L108 | **None exists.** Add: a property test per analysis contract (perturbing any identity column changes the id; perturbing a lineage column does not); a duplicate-seed config case (1.4); a two-guard fixture (2.1); an `id:` rule if `lctx_id` gains list arguments |
| F2 | **DESIGN was not amended where ADR-0019 decides.** §6.4 still prescribes the store's schema form for bundle digests, which is the form the ADR rejects. | DM-02, DM-23 · G1 | ADR-0019's `design:` lists §6.4, §9 and §10. §6.4 L1590–1592 says "per-file schema digests in `cpg-schema`'s canonical schema form (field name, type, nullability, metadata; conformance-tested in Rust and Python)". Its Decision line is ADR-0017, ADR-0012. `table.rs:75` documents `canonical_schema` as "The canonical schema form (DESIGN §6.4)". ADR L142–150 keeps that form for the store and adds a separate SHA-256 language-neutral form for the bundle. Also stale: §10.1 L1906, "ordered witnesses (fact ids, spans)", where the ADR has node-id steps with `edge_id` lineage; and §9 L1715, "record its method and parameters in `analytic_method` / `findings`", where the ADR has `analysis_invocations` | AGENTS.md makes DESIGN the current truth. The slice-1.7 implementer following §6.4 hashes Rust `DataType` display strings. Python cannot reproduce those (pyarrow's `item` vs Delta's `element`, plan I4), and the lifespan check (§11.3) then refuses every generation, or is loosened to pass. That is the failure the ADR was written to prevent | Amend §6.4 (and the `table.rs:75` comment), §10.1's witness bullet and §9's rule line. Add ADR-0019 to their Decision lines | **None exists.** Extend `just adr lint`: for a proposed or accepted ADR, each `design:` section (with its subsections) has a `> Decision:` line naming the ADR. It fails on §6.4 today |
| F3 | **The subsystem, seeds and distractors are written by an author with gold access, and "frozen" has no mechanical record.** So the §1.5 ranking check and the §12(a) score can be fit by construction. | DM-28, DM-59 · G4 (ADDENDUM Q15) | §1.4 L100–110: the subsystem is "hand-written from the library's own public API", and the gold "never for tuning". §1.5 L136–137: the target brief "must rank first against the distractor briefs for the gold family's `task_aliases`". Plan 1.4 and its "Execution mode" table: the executing session writes `analytics.toml` (prefixes, public roots, 3 distractors, "for example `FastMCP.resource`, `FastMCP.prompt`, `Client.call_tool`"), and the plan lists the `fastmcp` skill (the gold) among its first sources. ADR-0004's amendment makes the config "frozen (its commit recorded)" without saying where the record lives or what checks it. Deviation D1 already concluded that such an author should not write the held-out tasks | Distractors set how hard "ranks first" is: a distractor lexically far from the `fm.register` aliases makes it trivial. The prefixes set the Jaccard (a) achievable against the six gold families. Either can be fit to the gold without intent, and the §9.8 keep rule and the §1.5 done criterion then grade the author's knowledge, not the compiler | Before `analytics.toml` is committed, fix the distractor choice by a rule stated in DESIGN (e.g. every other public component-registration entry point of the seed's class, or the k subsystem entry points nearest the seed by a named criterion), **or** have a docs-only subagent write the subsystem and distractors, as in D1. Log it. Commit the frozen config digest in a file, and have scoring refuse a snapshot whose compiler-run `config_digest` differs | The exposure itself has **no mechanical oracle** (process; the deviation log is the record). "Frozen": a check in the 3.3 scoring script comparing the snapshot's compiler-run `config_digest` with the committed frozen digest |
| F4 | **No stated mapping from stop reasons and `omitted_paths` to `completion`.** The requested bound, execution truncation and presentation caps collapse. | DM-08, DM-30 · G2 · guidelines §7 MUST, §10 MUST | ADR L65, "Its completion as a `coverage_status`", and L71, "the stop reason, `omitted_paths` and depth". DESIGN §9.1 L1731–1732: "`omitted_paths = true` when more existed". Nothing says which stops make an invocation `partial`. The working tree's `omitted_paths` doc joins "more witness paths" (a presentation cap) with "more of the neighbourhood than the budgets kept" (execution truncation) in one bool (`findings.rs:81–83`). Its `stop_reason` codebook puts depth, vertex/edge budgets, the witness cap, scope boundaries and unresolved sites in one list, with no stated semantics | `FastMCP.tool`'s BFS hits the 128-vertex budget. If `completion` reads `complete_under_stated_model`, or synthesis treats the flag like a witness cap, "already coordinates" lists a subset with no Limits entry. An agent then reads a missing delegate as "does not delegate" (ADDENDUM Q4). Guidelines §10: "Do not replace an earlier complete artifact with a partial one under the same validity label" | State in ADR-0019: the depth bound and the subsystem, external and synthetic boundaries are complete under the stated model; the vertex or arc budget is operational truncation, which makes the invocation `partial` and makes Stage F emit a limit (or an unresolved slot); the witness cap is presentation only. Split the bool into `witnesses_omitted` and `neighbourhood_truncated`, or derive the latter from the invocation | **None exists.** `analysis_shapes` run with a vertex budget of 1: the invocation is `partial` and a Limits assertion exists (1.4/1.5) |
| F5 | **Claims outrun evidence.** §B6's new bullets inherit "Implemented and Tested", cite an ADR that does not exist, and the ADR promises an SQL recipe it cannot have. | DM-59 · G7 | DESIGN L10–12: "A section's label applies unless a line says otherwise". §B6 L229–230 is "**Implemented** and **Tested**". The new bullets L234–238 (in-row provenance; "Operator review verdicts (ADR-0020) … are `facts` rows") are unlabelled. `docs/adr/` has no 0020. ADR L108: "one recipe in Rust and SQL" (see F1). ADR L134–135 says the coverage rules must be scoped, but P1 shows they pass vacuously for a no-family run (O1) | A reader, or the next review, takes in-row provenance and verdicts-as-facts as implemented and tested. The verdict decision cites a record nobody can check | Label the two bullets **Proposed**. Write "ADR-0020 (to be written, slice 3.5)". Replace the SQL claim with what F1 decides. Correct L134–135 to name only `semantic:source-role-by-run` | Labels are prose. **Mechanical, cheap:** extend `just adr lint` so every `ADR-\d{4}` cited in DESIGN.md exists, or is marked as to be written |
| F6 | **A finding's `evidence_status` is chosen by its kernel, and no policy ties it to the method.** The §10.2 rule that statistical output never states a control trusts that value. | DM-08, DM-59 · G2, G7 (ADDENDUM Q6) | ADR L70 lists `evidence_status` among the finding's columns, and nothing constrains it. DESIGN §10.2 L1941–1944 derives an assertion's status from its supporting findings. The kind policy constrains assertion kinds only | A community or PageRank finding stamped `structurally_observed` by a kernel bug propagates that status into a `parameter` or Limits assertion, and the §10.2 validator (assertions ⋈ policy ⋈ supporting findings) passes it | Declare a method (or finding-kind) → permitted-status policy beside `assertion_policy`, e.g. `pass_a_bfs` → `structurally_observed`, and statistical methods → `statistically_derived`. Generate its rule. Or compute the status from the method instead of storing it. Also add a rule that every invocation's run is the `lctx-compiler` run | **None exists.** A generated policy rule with an injected statistical finding marked `structurally_observed` |
| F7 | **`evidence_id` includes the run-scoped `fact_id`.** So evidence, assertion and brief ids are per extractor run while §3.4.1 labels them "content". | DM-12, DM-15, DM-32 · G6 | ADR L116: the evidence recipe includes `fact`. DESIGN L544: `fact_id` scope "per run". The run id hashes the context (L543), whose id hashes the environment and lock digests (L541). L117–118: assertions hash their supports, and briefs hash their assertions. DESIGN L545 gives all of them scope "content". ADR L79–80 keeps `edge_id` out of finding ids because it is *producer*-scoped. `fact_id` is scoped more narrowly still | A `uv lock --upgrade-package` of an unrelated dependency, an extractor config change or a Pyrefly bump renames every brief that cites a fact. Every ADR-0020 verdict then goes stale although no claim changed, and ablation across two contexts is not a join. The failure is conservative (no wrong reuse), but it costs a full re-review and contradicts the stated scope | Treat `fact_id` as lineage, like `edge_id`. Identify evidence by (kind, node, module, span, digest of the resolved text), which is content-bound and run-independent. Or keep run scope, and relabel L545 and ADR-0020's premise | **None exists.** A test: two compiles of one fixture whose contexts differ by an unrelated installed file give equal evidence, assertion and brief ids (or, if run scope is kept, differing ids, so the choice is pinned) |
| F8 | **The resolved text in `evidence` is a copy of authoritative source bytes, with no stated consistency check.** | DM-23, DM-07 · G1, G3 | ADR L82–84: each evidence row "holds the resolved text", "because the server reads no Delta". The bytes are authoritative in `source_files.text` (ADR-0015) and `passages`. DESIGN §3.4 L517–521: coordinates are byte offsets, "never assumed to match". DataFusion's `substr` counts characters | If Stage F resolves spans in SQL, every quote after a non-ASCII character is misquoted. The bundle then serves a sentence attributed to a span that says something else, and nothing rejects it | Resolve text in one byte-based function shared by Stage F and the bundle. Add a rule `octet_length(evidence.text) = end_byte - start_byte` (the precedent is `semantic:source-text`), plus an exact-bytes check where the text is built. Alternatively, resolve at bundle time from `source_files` | **None exists.** The rule, with an injected case: a span over a multi-byte character |
| F9 | **Witness steps drop modality and phase from identity on `edge_id`'s reason, which applies to neither.** | DM-15, DM-24 · G6 | ADR L79–80: "The `edge_id`, modality and phase are **lineage**. Call edge ids are producer-scoped". Modality and phase are codebook values, not producer-scoped hashes. Probe P2: within a snapshot, (site, callee) fixes both (68,140 `call_target` pairs, 30,834 `site_target` pairs), so including them costs nothing there. In the working tree, a candidate arc at depth 1 is a `bounded_delegation_path`, not a `direct_delegation` (codebook diff), so modality reaches identity only through kind, and only at depth 1 | Across snapshots (a Pyrefly bump, a class-hierarchy change, an ablation), a depth-≥2 path whose arc flips candidate ↔ definite keeps its finding id. The ablation join reports "unchanged" for a change §3.6 treats as meaningful | Step key = (call site, callee, modality, phase), with `edge_id` alone as lineage. Two `Int16` per step | A test on the `analysis_shapes` Overrides fixture: the same path with the arc definite yields a different finding id |

### Observations (no gate moves on them alone; each with its fix)

| # | Observation | Fix |
|---|---|---|
| O1 | Only `semantic:source-role-by-run` needs scoping. P1 shows `coverage:declared-family`, `coverage:complete` and `coverage:family-has-scope` pass vacuously for a run with `families = []`, because `unnest` of an empty list yields no rows. Separately, `coverage:declared-family` accepts any `FactFamily::all()` value, so once `findings` and `embedding_cache` are appended, a run that wrongly declares one passes that rule, and `coverage:complete` then demands per-module coverage for it | Scope only `source-role-by-run`: the working tree's `cardinality(r.families) > 0` gives 0 in P1. Make `coverage:declared-family` reject the non-coverage families. Add a rule case with a compiler run |
| O2 | `rules()` generates `fact:`/`fact-payload:` rules for any table with a column **named** `fact_id` (`rules.rs:248–268`). An `evidence` table citing a fact through a column named `fact_id` would demand a `facts` row of `table_name = 'evidence'` and fail every compile | Name the column `cited_fact_id`, as the working tree's `finding_members` does. Add a contracts test that no analysis table has a `fact_id` column |
| O3 | The ADR stores parameters as JSON because "a digest alone would not say what ran" (L66), yet the projection spec (L60) and the analytics config are persisted only as digests. Which config fields each method's `parameters` carries is not stated, and the subsystem mask and seed list are result-affecting for Pass A (guidelines §3: "All result-affecting projection and invocation settings participate in artifact validity") | Publish projection specs per snapshot as canonical JSON, like `edge_kinds`, or put them in the invocation row. State that `parameters` holds every config field the method reads. Test: changing each `analytics.toml` field changes some invocation's `parameters_digest` |
| O4 | The kernel code and templates enter `compiler_digest` only through the hand-bumped "analytics output version" and "template version" (ADR Consequences). A forgotten bump gives two outputs one `compiler_digest`, so the rerun oracle cannot tell a code change from nondeterminism. There is precedent: `COMPILER_OUTPUT_VERSION` | A test pinning the digest of the analysis-fixture snapshots to the version constant, so an output change without a bump fails |
| O5 | Global-table loose ends. (a) `attempt_versions` finds a table's commit by `lctx.snapshot_id` (`snapshot.rs:173–200`). The MERGE must stamp it too (`delta.rs:171–174` stamps only `append`), and an attempt whose keys were all cached has no commit of its own, so which version `--unpublished` reads is undecided. (b) `snapshots.row_count` is documented as "Rows the attempt wrote" (`tables.rs:1034`) and now also means "rows used". (c) Once E0 lands (increment 3), the "keys used" behind the `content_digest` input and `row_count` are not reconstructible from any snapshot table. (d) The lock-file fallback has no stated stale-lock recovery. (e) The read mode is to be declared both as a per-table mode and as a group macro (plan 1.4) | (a) Stamp the MERGE commit. For a global table, `--unpublished` reads the latest version, labelled as such. (b) Amend the column doc per read mode. (c) Record the used keys in a snapshot-qualified relation, or derive them from named tables. (d) Put the attempt id and pid in the lock, and state the takeover rule. (e) Declare the mode once, as `Table::READ_MODE`, with the macro only iterating; a test ties the two |
| O6 | ADR-0017's invariant holds only if **all** Stage E methods write each analysis table in one commit. Slice 2.6's `related` and seed selection, and 3.2's RCA, consume other methods' findings. Re-reading `findings` through SQL mid-attempt would need a second commit | State in ADR-0019 that Stage E composes methods in memory (Arrow), and that `findings` is written once after the last method. Otherwise ADR-0017's revisit trigger fires |
| O7 | `capability_id = brief_id` is a revision identity (content). The entity, the capability at (library, seed, applicable case), is unnamed, so an ablation diff shows a changed brief as a removal plus an addition | Name the entity key (seed, applicable case) in §3.4.1. The ablation report pairs briefs on it and diffs by content id |
| O8 | A brief section is stated in three places: `assertion_policy` (kind → section), `assertions.section` (hashed into the assertion id) and `brief_assertions.section` | The policy is the authority. Either drop the other two or check them with a rule |
| O9 | An attempt with no analytics config (fixture tests, a source tree) is undefined: are the analysis tables written empty, and is there a compiler run? Absent tables would make every generated analysis rule fail to plan | Always write the analysis tables, empty when there is no config. The compiler run exists iff a config exists, so "not requested" is readable as no compiler run |
| O10 | vLLM vectors differ between requests (E2), and `content_digest` hashes keys, not vectors. From increment 3, a result that depends on E0 vectors is rebuildable from its snapshot **within one store's cache** only. Two fresh stores with equal `content_digest` can differ, which bounds the §9.8 rerun oracle and ADR-0019's rebuild claim | State the bound in ADR-0019's rebuild sentence and in §9.8 (the cache version is part of "the snapshot") |
| O11 | ADR-0010's stdio claim holds at source (Method). The planned `StdioTransport` test observes stdout only, not the network GET or the `version_cache*.json` write the amendment is about | In that test, point `fastmcp.settings.home` at a temp directory, and assert that no cache file appears |

**Applicability.**

| Groups | Bearing on this scope |
|---|---|
| 3 (identity, versions, consistency) | Carries most of the findings: F1, F7, F9, O7 |
| 2 (absence) | F4, F6, O9 |
| 1 (authority) and 5 (derivation) | F2, F8, O8 |
| 10 (provenance) | O3, O10 |
| 11 (verification) | F1's recompute, O4 |
| 6 and 7 (effects, reuse, the global table) | O5, O6 |
| 12 (proportionality) | The crate split and the table set, judged in §8 |

These groups did not bear on this scope:
- **Group 4** (declarative composition), except as the identity-generation suggestion.
- **Group 8** (performance): the ADR makes no performance claim, and Stage E's cost is measured
  by slice 1.4's `just pilot`.
- **Group 9** (providers): the embedder trait boundary is ADR-0010's, and was not reviewed here.

Not used: optional maturity scores. The gates carry the decision.

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| Baseline (`e735fde^`) | The `findings` family named with no contracts. §B6 says findings would be `facts` rows | Tables outside every group silently get no rules (the ADR's Context) | — | — | Superseded |
| Option 1: findings as `facts` rows | One provenance mechanism | The `facts` commit moves after Stage F, which restructures the attempt around ADR-0017's one commit. Observations and results mix in one table (DM-13). `fact:` rules apply to derived rows | Undoes H1's release of raw batches (the C6 F3 memory fix) | H1 measured the release's benefit. This option was not measured | Rejected by the operator. **I agree** (DM-13, ADR-0017) |
| Option 2: results in the catalogs | A second graph authority | `nodes`/`edges` are derived in Stage D, before analysis | — | — | Rejected. **I agree** |
| ADR-0019 as written | Provenance in-row through invocation → run. Five hand-written recipes, with the per-kind payloads undecided. Status chosen by the kernel | F1, F4, F6, F7, F9 | ~12 contracts and 2 crates, each with an increment-1 consumer | none | — |
| **Simpler viable: ADR-0019 with contract-derived identity and policies** | **One** identity rule: each contract marks identity vs lineage columns, one generic hasher covers row plus children, and the rule is recomputable in SQL if `lctx_id` takes lists. A method → status policy beside the kind policy. One byte-based text resolver checked by a rule. A new kind or column is one declaration (charter §E) | Removes the F1/F7/F9 class by construction. F6 becomes a generated rule | A column attribute in `table!` plus one function. Fewer per-kind decisions than five recipes | none | **Recommended.** It keeps every ADR-0019 table and decision, and replaces the hand-enumerated recipes |

**Abstractions justified by current needs.**
- `for_each_analysis_table!`: its consumers are the raw write, the rule generator and the digests.
  It closes exactly the silent-gap class the ADR's survey found.
- The read mode: one global table today, but without it `register` refuses every published
  snapshot with `ForeignCommit`.
- `lctx-analytics`: kernels testable without DataFusion or Delta.
- `lctx-embed`: keeps `reqwest` out of `cpg-core`, behind a trait.
- `assertion_policy` as a table: its consumers are inspection and the generated rule, and
  `edge_kinds` is the precedent.

The table set follows output meanings (guidelines §10: specific schemas, not a property bag).
Nothing here is over-construction. The working tree declares only the four tables slice 1.4 emits,
which matches ADR-0008's "declare only what is emitted".

**What remains ordinary code:** Pass A's BFS and witness choice, the templates, and the Leiden,
PageRank and FCA kernels, each behind the table contracts.

## 9. Verification and measurement plan

| Claim or risk | Evidence label | Test / analysis | Conditions and expected result | Current result or gap |
|---|---|---|---|---|
| A compiler run passes the extractor-only rules | Tested (probe P1, ad hoc) | a rule case with a compiler run (plan 1.4) | 0 violations; 275 without the scoping | Not yet a regression test |
| Every analysis table has generated rules that reject violations | Proposed | `every_rule_is_exercised_or_declared_an_edit_guard` plus injected cases | each new rule fails on its injected row | Owed per slice |
| Identity completeness | Proposed | property test per contract; `id:` rule if `lctx_id` takes lists | identity column perturbed → new id; lineage column → same id | **Missing** (F1) |
| Determinism | Proposed | shuffled `analysis_shapes` gives byte-identical tables; two identical compiles give equal ids (L175–176) | byte equality | Owed in 1.4. It cannot detect F1 |
| Truncation vs requested bound | Proposed | vertex budget 1 on `analysis_shapes` | invocation `partial`; a Limits assertion | **Missing** (F4) |
| Statistical output never states a control | Proposed | method → status policy rule; §10.2 validator | an injected mis-stamped finding is rejected | **Missing** (F6) |
| Evidence text equals the cited bytes | Proposed | length rule plus an exact check at construction | a multi-byte span case is rejected when wrong | **Missing** (F8) |
| The global read mode | Proposed | `published()` over a snapshot whose `embedding_cache` version is another attempt's MERGE commit | reads succeed; no `ForeignCommit` | Owed in 1.6 |
| MERGE under concurrency | Proposed | 1.6 probe (4) | one row per key | Unprobed. The fallback is declared |
| The bundle digest is reproducible in Python | Proposed | shared Rust/Python known answers (L147–148) | equal digests | Owed in 1.7. DESIGN §6.4 must be amended first (F2) |
| `analytics.toml` is frozen | Proposed | scoring compares the frozen digest | a mismatch refuses | **Missing** (F3) |
| Stage E cost on the pilot | — | `just pilot` at 1.4 | rows, time and peak (Measured) | Not yet run. Baseline 29.9–33.4 s, 3.6 GiB (STATUS) |

**Cost accounting.** The document states nothing to account for. The material new costs are:
- Stage E's projection batches and the dense graph at pilot scale (68,140 call-target arcs, P2);
- the global `key:embedding_cache` scan, which grows with every library's vectors.

It reads the key columns only.

## 10. Exceptions and unresolved decisions

No SHOULD-level exception is requested. The global table's multiple commits per attempt are a
declared contract change in ADR-0017's amendment, not a charter exception.

The unresolved decisions, which the author has to make:
1. identity columns, and finding↔invocation cardinality (F1);
2. the completion mapping (F4);
3. the finding-status policy (F6);
4. distractor and subsystem authorship (F3);
5. evidence identity scope (F7).

## 11. Decision and implementation changes

**Decision: Revise.** The core decision is sound, and this review agrees with it over Options 1
and 2:
- typed analysis tables outside the catalogs, with in-row provenance;
- content ids without a config digest;
- the compiler run in the raw write;
- the global read mode with an insert-only MERGE.

G1 and G7 fail on documentation and labels, which are one-commit fixes. G2, G4 and G6 are
unresolved on behaviour slice 1.4 builds now. The ADR should stay `proposed` until F1–F6 are
decided. Slice 1.4 can proceed if its first commit's contracts carry the P1 corrections below.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| P1: before 1.4's contracts are committed | F1: identity/lineage columns per contract, one generic hasher, cardinality decided, seeds deduped by declaration. F9: modality and phase in the step key. F4: the completion mapping and the split flag. F6: the method → status policy (`pass_a_bfs` now) | DM-15, DM-08, DM-59 | the ADR text amended; contracts snapshot | property test; duplicate-seed case; vertex-budget-1 fixture; policy rule with an injected case |
| P1: before `analytics.toml` is committed | F3: the distractor rule or docs-only authorship, logged; the frozen digest file | DM-28, DM-59 | the D-log entry; DESIGN §1.4/§1.5 sentence | scoring digest check (3.3) |
| P1: same docs commit | F2: §6.4, §10.1 and §9 amended. F5: §B6 bullets labelled Proposed; ADR-0020 marked unwritten; "and SQL" replaced; L134–135 corrected | DM-02, DM-59 | `just adr lint` | the two `adr lint` extensions |
| P2: by 1.5 | F7: evidence identity (fact as lineage), or run scope stated. F8: the byte resolver and its rule. O2: `cited_fact_id` | DM-15, DM-23 | Stage F fixtures | context-variation test; the evidence-text rule |
| P2: by 1.6 | O5 (a)–(e) | DM-14, DM-30 | 1.6 probe | `published()` over a foreign MERGE commit |
| P3 | O1, O3, O4, O8, O9, O11 | DM-31, DM-02 | per the table above | per the table above |

**Deferred (proposed), each with the trigger that reopens it:**

| Item | Trigger |
|---|---|
| O6: in-memory composition of Stage E methods, stated | Slice 2.6 (`related`, seed selection) or 3.2 (RCA) reads another method's findings |
| O7: the named entity key for ablation pairing | The first §9.8 ablation report (3.3) |
| O10: the rebuild bound under vLLM nondeterminism | Slice 3.1 (E0 embeddings feed findings) |

**Final check.** The design's claims do not yet match their evidence (§B6's labels). Its scope
does not yet match the guarantees it names: "content" scope and "Rust and SQL" recipes. Its later
extensions have a clear route only once identity is derived from the contracts (F1). The rest,
from the table set and provenance route to the compiler run and the global table protocol, is a
sound base for slices 1.4–1.9.

## Disposition (2026-09-23, before slice 1.4's first commit)

| Item | Disposition | Where |
|---|---|---|
| F1 | Fixed. Identity and lineage columns per contract (`findings.rs` doc and `FindingKey`); the finding id hashes the condition node, status, witness steps and members; recipes stated Rust-only; the property test `a_finding_id_follows_every_identity_column`; two seeds naming one declaration refuse the compile (`analyze.rs`); cardinality for multi-invocation results decided (consensus published once, or `finding_invocations` with 2.3) | ADR-0019 amended; `crates/cpg-schema/tests/contracts.rs` |
| F2 | Fixed. §6.4 (serving schema digest), §9's rule line and §10.1's witness bullet amended, with ADR-0019 on their Decision lines; `table.rs`'s doc names the store form. `just adr lint` now checks that each listed section's Decision line names its record (10 older gaps fixed with it) | `scripts/adr.py` `design_decisions`; `tests/scripts/test_adr.py` |
| F3 | Fixed. The subsystem and distractors rewritten by a docs-only subagent under stated rules (deviation log D3); the frozen-digest check lands with the 3.3 scoring | `libraries/fastmcp/analytics.toml` |
| F4 | Fixed. The completion mapping stated; `omitted_paths` split to `witnesses_omitted`, with truncation on the invocation (`partial`) | ADR-0019, DESIGN §9.1, `findings.rs`; Stage F's Limits entry lands in 1.5 |
| F5 | Fixed. §B6's bullets labelled; ADR-0020 marked "to be written"; "and SQL" replaced; the coverage-rule claim corrected. `adr lint` now fails on a cited record that does not exist | DESIGN §B6; `scripts/adr.py` |
| F6 | Fixed. `FINDING_STATUS` policy and `semantic:finding-status-policy`; `semantic:invocation-run-is-compiler`; each rejects an injected violation | `rules.rs`; `crates/cpg-core/tests/analysis.rs` |
| F7 | Decided now (evidence identity by content; the fact is lineage), built in 1.5 | ADR-0019 table |
| F8 | Owed by 1.5 (the byte resolver and its rule) | plan 1.5 |
| F9 | Fixed. Modality and phase in the step key | `StepKey`; the property test |
| O1 | Fixed. Only `semantic:source-role-by-run` scoped; `coverage:declared-family` refuses non-coverage families | `rules.rs` |
| O2 | Fixed. `no_analysis_table_has_a_fact_id_column` | `contracts.rs` |
| O3 | Fixed for Pass A: `parameters` holds every field it reads (seed, budgets, prefixes, public roots); the projection spec's SQL is in the compiler digest | `analyze.rs` |
| O4 | Owed by 1.5 (the output-version pin test, once briefs exist) | plan 1.5 |
| O5 | Owed by 1.6 | plan 1.6 |
| O8, O11 | Owed by 1.5 and 1.8 | plan |
| O9 | Fixed. Analysis tables always written, empty without a config; the compiler run exists iff a config does | `attempt.rs` |
| O6, O7, O10 | Deferred with the triggers above | — |
