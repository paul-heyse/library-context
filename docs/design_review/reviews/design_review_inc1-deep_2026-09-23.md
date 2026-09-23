# Design review: increment 1 as delivered (deep)

- **Date:** 2026-09-23
- **Depth:** deep (ADR-0004 cadence: deep after increment 1). This review also covers slice 1.8's
  compact review (deviation log D15), and slices 1.6 and 1.7, which were never reviewed.
- **Target:** commits `e735fde..6c86f53` on `main`, with DESIGN.md (§1.2, §1.4, §1.5, §3.2, §4.1,
  §5, §6.4, §9, §9.1, §10, §11, §12), ADR-0019, ADR-0010, ADR-0017, ADR-0004, the slice 1.4 and
  1.5 reviews with their Dispositions, and the deviation log (D1–D16).
- **Anchor:** every code citation is `git show 6c86f53:<path>` at the line given. Two things
  changed under the review, and neither was reviewed:
  - `afef05a` (slice 2.0, config and log only) landed on `main`.
  - Uncommitted increment-2 edits appeared in the working tree (`pass_b.rs`, `flows.rs`, the
    codebook and `findings.rs`).
- **Reviewer:** a fresh `design-reviewer` subagent. **Author:** the implementing agent, under the
  operator's "run straight through" instruction.

---

## 1. Decision and scope

**Proposal.** Increment 1 (DESIGN §1.2 row 1) is one complete path over real FastMCP 4.0.5:
- analytics config → invocation projection → Pass A;
- Stage F: assertions and briefs;
- brief documents embedded through the global `embedding_cache`;
- a byte-identical serving generation (`lctx bundle`);
- `python/lctx_mcp`: hybrid retrieval, hydration, both MCP tools and the resource;
- the end-to-end pilot with a stdio smoke.

**Status (§D).** Implemented. Most of the path is **Tested** by named tests. The pilot
reproduction is **Tested** by this review (P2). The §1.5 ranking is **Measured** as failed.

**Observable outcome.** An agent can call `search_capabilities("fastmcp", …)` over stdio and get
ranked briefs for four FastMCP decorators. It can then hydrate any brief whole: its evidence
statuses, its supports, and verbatim evidence. The generation can be rebuilt byte for byte from
the store.

**Baseline.** The CPG (C1–C6, H1) was complete and reviewed. ADR-0019 decided the analysis tables
before slice 1.4.

**Supported scope.** One primary seed (`fastmcp.FastMCP.tool`) and three distractors
(`resource`, `prompt`, `custom_route`). The assertion kinds are `outcome`, `public_access`,
`coordinates`, `parameter` and `analysis_boundary`. The tables are Pass A's.

**Non-goals.** Passes B and C, communities, FCA, the generation lifecycle (increment 4), gold
scoring (§12, increment 3) and manual review (§10.4, increment 3).

**Constraints.**
- One operator and straight-through execution: sixteen judgment calls were logged, not
  pre-approved.
- The live embedding legs need the GPU service, which this review was told not to start.
- The held-out set is sealed: only its README and `MANIFEST.sha256` were read.

### Method and coverage

**Read in full:**
- `synth.rs`, `cpg-core/src/bundle.rs`, `cpg-schema/src/bundle.rs`, `projection.rs`,
  `lctx-analytics/src/{graph,pass_a,config}.rs`, `analyze.rs`, `embed.rs`, `lctx-embed/src/lib.rs`;
- `attempt.rs`, lines 25–125 and 366–530;
- the analysis and synthesis rules in `rules.rs`, lines 225–470;
- every file of `python/lctx_mcp`, with its tests;
- `scripts/ranking_check.py`, `gold_extract.py` and `embed_conformance.py`;
- `justfile` and `pyproject.toml`.

**Read in part:** `findings.rs` (the policies, recipes and `ANALYSIS_BACKED`) and the ledger and
fixture tests in `cpg-core/tests/{analysis,bundle}.rs`.

**Checks run (outcomes):**

| Command | Outcome |
|---|---|
| `just test-all` (HEAD `afef05a` = `6c86f53` + config; the test binaries needed no rebuild, so the code was 6c86f53's) | **passed**: nextest 152/152 (106.5 s), pytest 44/44, rule tests 4/4, `adr lint` 19, family/deny/pyrefly-fork/shear, gold. 1 min 56 s |
| `lctx compile fastmcp --embedder fake` from a `git archive 6c86f53` build, into a scratch store and environment (the `just pilot` equivalent; P2) | **passed**: snapshot `5c4b6f80…`, 32.3 s (extract 27.3 s), peak 3,640 MiB |
| `python -m lctx_mcp.smoke` on that generation | **passed** (4 briefs found first and hydrated) |
| `just pilot` itself | **not_run**: it writes `build/store`; P2 is the same run into scratch |
| `just pilot-live`, `just embed-conformance` | **blocked**: they need the vLLM service, which the review was told not to start |
| `scripts/ranking_check.py <live generation> --embedder none` | **ran**: `FastMCP.tool` first for 1 of 2 `fm.register` aliases (lexical-only) |
| `sha256sum -c MANIFEST.sha256` in `eval/heldout` | **passed** (18 files; hashes only, no contents read) |

**Probes** (this review, 2026-09-23):

| # | Probe | Result |
|---|---|---|
| P1 | pyrefly `dump-config` | `python/lctx_mcp/src` is **skipped**. It is on the site-package path through the editable install, so it lands in `project-excludes`. Checking the files explicitly gives 0 errors today |
| P2 | Independent rerun of the 1.9 fake pilot: a 6c86f53 binary, relocated environment and store | `content_digest` `39215595…` equals snapshot `381778b7`'s. All nine served files are byte-identical to generation `8d5e5ba2…`. The manifests differ only in `snapshot_id` and key. `findings`, `witnesses`, `analysis_invocations` and `briefs` are identical |
| P3 | `compiler_digest` across template versions | Snapshot `fa29cd66` (`template_version` 3) and `381778b7` (4) share compiler digest `199d5ed98e96…` (F1) |
| P4 | Served evidence against the installed bytes | 45 of 45 spanned evidence rows of live generation `9470c532…` are verbatim. 17 unspanned rows are requiredness facts |
| P5 | Stdio | Driving the real server over a raw pipe gives 3 stdout lines, all JSON-RPC. A deliberately noisy FastMCP server that prints at import and inside a tool **still passes** the same `Client(StdioTransport)` round trip that `test_stdio.py` uses (F6) |
| P6 | Malformed 200 responses to `parse_embeddings` | A missing `embedding`, a top-level list, `data: null` and string components raise `KeyError`, `AttributeError` and `TypeError`, not `EmbedderError`. `search` then raises instead of degrading (F7) |
| P7 | Per-term BM25 on the live `lexical_text` | For "Register functions and expose component metadata" only `register` is in the vocabulary. Its df is 4 of 4, so its idf is 0.105, and the scores 0.041–0.044 order the briefs by length (F2) |
| P8 | Readers of `[briefs]` | Nothing reads `budget` or `serve_unreviewed`. The committed bundle test compiles 4 briefs under `budget = 2` (F3) |
| P9 | Where the served gap metric comes from | `unresolved_slots` is `{}` in the live manifest, while every brief lacks the Applicable case and Usage pattern sections (F4) |

**Not inspected, or asserted only:**
- The C1–C6 families, Stage A and the CLI's acquisition path. They are out of scope.
- The vLLM HTTP client beyond its parse and rejection code.
- The MERGE concurrency probe tests: they exist and pass, but were not attacked.
- The live vector-leg ranks and the 1.9 conformance figure (worst cosine 0.9999334). These rest on
  the author's commit message, because they are blocked here.
- The held-out task contents, which are sealed.
- Pass A's traversal kernel was re-read, not re-attacked. The slice 1.4 review attacked it.

**Housekeeping.** `cargo build --release -p lctx` in the repository rebuilt
`target/release/lctx` at 17:18. That binary is build output, not source, and it includes the
uncommitted working-tree edits. P2 therefore used a separate `git archive 6c86f53` build, and the
pilot-store queries in P3, P4 and P9 used that build too.

---

## 2. Authority and lifecycle map

| Concept | Identity | Authority | Revision boundary | Update path | Derived |
|---|---|---|---|---|---|
| Analytics config (subsystem, seeds, budgets) | its canonical JSON digest → the compiler run's `run_id` → `content_digest` | `libraries/fastmcp/analytics.toml` | its commit | edit before the gold freeze, by ADR after it (ADR-0004 amendment) | invocation `parameters` JSON |
| Invocation projection | `ProjectionSpec::digest` | `cpg_schema::projection::invocation()` | compiler digest (`attempt.rs:75-77`) | code + `COMPILER_OUTPUT_VERSION` | petgraph `Graph`, never persisted |
| Findings, witnesses, members | `FindingKey`; `edge_id`/`invocation_id` as lineage | Stage E rows (`pass_a.rs`) | the snapshot | recompute | Stage F supports |
| Kind policy | (kind, section, status) | `findings::ASSERTION_POLICY` | compiler digest, through the rules' SQL | code | the published `assertion_policy` table + generated rules |
| Status derivation | — | `findings::derive_status` | — | code | its SQL recompute in `semantic:assertion-status-derived`, equality-checked with injected cases: two expressions of one function, **reconciled** |
| Templates and extractive rules | the text is in `assertion_id`; `template_version` is lineage | `synth.rs` + `TEMPLATE_VERSION` | **none: absent from `compiler_digest` (F1)** | code + a version bump, enforced by the ledger test | brief documents, lexical text |
| Embedding spec | SHA-256 of canonical JSON | `specs/embedding/*.json` | spec hash | a new file | Rust `include_str!` and packaged Python copies, **reconciled** by tests; `embedding_specs` rows |
| Vectors | `(spec_hash, input_hash)` | global `embedding_cache` (insert-only MERGE) | the cache version recorded per snapshot | append only | bundle `vectors` |
| Served schemas | serving schema digest | `cpg_schema::bundle::files` **and** `generation.py::expected_schemas` | `bundle::FORMAT` | both, **reconciled** by `specs/serving/schema_digests.json` (tested on both sides) | manifest digests |
| Generation | key = SHA-256 of the manifest without its key | derived from one published snapshot | the key | rebuild | — |
| Gold | `authoring_sha256` per family | `.claude/skills/fastmcp` | `check_gold.py` | skill refresh | `eval/gold/fastmcp-4.0.5.json` (evaluation scripts only) |
| Held-out set | `MANIFEST.sha256` | `eval/heldout/` | commit `b2fd8c1` | none until increment 5 | — |
| Review gate | `briefs.review_state` | synthesis (always `unreviewed`) | brief id | ADR-0020 (increment 3) | `serve_unreviewed` knob, **inert (F3)** |

**Deliberately opaque.** The template wording and the sentence heuristics (`ends_summary`,
`paragraphs`, `CHANGELOG_STEMS`) are ordinary code. They sit behind a versioned template
contract, which charter §F permits. The defect is that the version is not part of the compiler's
identity (F1).

**Identity behaviour.** P2 establishes the following at pilot scale:
- **Relocation.** Content ids, finding and brief ids, and served bytes survive a new environment
  path and a new store.
- **Rerun.** A rerun gets a new `snapshot_id`, so a new generation key over byte-identical files
  (D11, by design).
- **Template change.** D14 kept assertion and brief ids, because `template_version` is lineage.
  So a document-only change keeps ADR-0020's future verdicts valid. This is correct.

---

## 3. Semantic contracts and invariants

| Contract | Representation | Enforced at | Failure | Evidence |
|---|---|---|---|---|
| An assertion's status is a function of its supports | `derive_status` + SQL recompute | validation, before publish | the attempt is refused | **Tested**: 3 injected cases |
| A brief is documentation-only exactly when it cites no `ANALYSIS_BACKED` finding | `semantic:brief-cites-analysis`, both ways | validation | refused | **Tested**: injected cases; `helpers.describe` |
| Evidence text is its span's bytes | `semantic:evidence-text-bytes` | validation: **length only** (`rules.rs:379-382`) | refused on a length mismatch | Bytes are **Tested** on the fixture only; P4 checks the pilot (O4) |
| One vector space per generation | `semantic:one-embedding-spec` + the builder | validation, and the bundle build | refused | **Tested** (`mixed_embedding_specs_are_refused`) |
| Every embedded document has a cached vector | `semantic:document-vector-cached` + the build's count check (`bundle.rs:553-564`) | validation, and the bundle build | refused | **Tested** |
| The generation is what its manifest says | sha256, rows, schema digest, key | Rust `verify`; Python `load` at lifespan | refused at connect | **Tested** (`a_changed_generation_is_refused`, `test_a_mismatched_generation_fails_at_connect`) |
| Query and compile vectors share one spec | lifespan compares the client spec hash with the manifest's | lifespan | refused at connect | **Tested** |
| Stdout carries only the protocol | env set before import, `show_banner=False` | none that can fail (F6) | a corrupted stream in a real client | behaviour **Tested** by P5; the named test cannot detect a violation |
| Degraded mode when the embedder fails | `except EmbedderError` (`server.py:168`) | `search` | a masked tool error for malformed shapes (F7) | **Tested** for a down service only |
| The analytics config governs the review gate and the brief budget | `[briefs]` fields | **nowhere** (F3) | silently ignored | P8 |
| `compiler_digest` identifies the code that produced the output | `compiler_digest()` | per snapshot | **the template version and synthesis SQL are missing (F1)** | P3 |

**Absence and uncertainty.**
- **Kept distinct in the rows:**
  - `unresolved` (Outcome only);
  - `complete_under_stated_model` vs `partial` per invocation;
  - `traversal_stop` for the depth bound;
  - `incomplete_resolution` per site;
  - "requiredness not observed".
- **Collapsed on the served surface:**
  - an absent section is not a slot, so the served `unresolved_slots` map reads `{}` (F4);
  - a candidate final arc is rendered as "call" (F5).

**Equivalence.**
- The served files are byte-identical from one store (Tested) and across a relocated rerun with
  the fake embedder (Tested, P2).
- Live vectors are not reproducible across stores (vLLM, E2). ADR-0019 review O10 defers the
  bound to 3.1.
- The Rust and Python serving digest forms are the same bytes (Tested on both sides).

---

## 4. Derivation and execution (compressed)

The attempt order is sound (`attempt.rs:366-520`), and each step is written like any other table:
1. Derive.
2. Stage E: Pass A, written as its own commits.
3. Stage F: synthesis.
4. Embed. The merge happens only after every batch is embedded, so a failed embedder merges
   nothing.
5. Write the synthesis tables.
6. Validate.
7. Append to `snapshots`.
8. `lctx compile` then builds the generation, from the published snapshot alone.

The bundle is written to `.building-<key>-<pid>` and renamed into place, and an existing key
with different bytes is refused. Python reads only the generation and its packaged specs.

**Relationship structures.** The projection keeps call, property and definition arcs typed
(`arc_kind`), and keeps candidate arcs with their modality. What collapses them is the finding
*kind*:
- a path made only of definition arcs is typed `bounded_delegation_path` (O3);
- the template count adds definition arcs to "call sites" (F5).

**Boundary contracts.**
- The Arrow IPC files are normalized: one batch per file, declared types, zeroed null slots,
  V5 metadata, 64-byte alignment.
- The Python mirror of the schemas is tied to Rust by the shared known answers.
- The Python embedder's parser is looser than Rust's typed `serde` parser (F7).

---

## 5. Journeys

**Ordinary extension: an increment-2 assertion kind** (for example a Pass B restriction in
Limits). The work is:
- append a codebook value;
- add an `ASSERTION_POLICY` row (the policy table and rules derive from it);
- write a template;
- declare the evidence kind.

The bundle serves the new kind's text and section with no change (`text_of`, `section_of` from
the policy), which is good locality. Three things go wrong silently:
- The `TEMPLATE_VERSION` bump does not move the compiler digest (F1).
- If the template cites guard source bytes as `span` evidence, the derivation publishes it as
  `documented` (O2).
- Its Limits entries still do not count as `unresolved` slots when absent (F4).

**Meaningful change: D14, traced.**
- `TEMPLATE_VERSION` went from 3 to 4, and the brief documents lost their boundary lists.
- The assertion and brief ids were unchanged, because `template_version` is lineage. That is
  correct.
- The `content_digest` moved only because the embedding keys moved.
- The compiler digest did not move (P3).
- With `--embedder none` the documents carry no key, so the same change would leave
  `content_digest` unchanged while `brief_documents` differs.

**Boundary: Rust → Arrow IPC → Python.**
- Schema: the serving digest, recomputed by pyarrow and checked against both the manifest and
  the expected schema.
- Identity: the manifest key, recomputed with `json.dumps(sort_keys=True)`. This equals serde's
  sorted compact form for these values (ints, strings, nulls), as every successful load shows.
- Vectors: checked to be finite unit vectors at load.
- Spec: its hash is recomputed from the served JSON.

Nothing is lost. The codebook values are served as text, so Python needs no codebook.

**Interruption.**
- **The embedder is down at compile.** The attempt aborts, is not published, and merges no cache
  rows.
- **The embedder is down at query time.** The search runs lexical-only and says why (Tested).
- **The embedder answers malformed JSON with status 200.** The call fails with a masked error
  instead of degrading (F7).
- **A crash mid-bundle.** Only a dotted staging directory remains, and `lctx bundle` rebuilds.
- **An ambiguous `snapshots` append.** Unchanged since slice 2.

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **pass** (documentary divergence noted) | Every second expression is reconciled by a test: the serving schemas (known answers, both sides), the spec copies, and the status derivation (SQL recompute). The **composition** of `compiler_digest` is stated three ways that disagree: ADR-0019 L195-200 (template version and synthesis SQL), DESIGN §3.4.1 L567 (neither those nor the projection digests), and the code (projection digests and analytics libraries, no template version) | Fold into F1 |
| G2 Semantic fidelity | **fail** (narrow) | The served `unresolved_slots: {}` cannot be told apart from "no gaps" (F4). Templates render a definition arc as a call site, a candidate final arc as "call", and "returns or registers" without a fact (F5) | F4, F5 |
| G3 Validity | **pass** | Invalid generations, specs and schemas are refused at load. Malformed embedder answers reach a *safe* handling path: a masked error, never wrong data | — |
| G4 Hidden behavior | **pass** for this scope | The compiler reads no skill path (grep). The server makes no network call beyond the embedder, and the update check is off. D3's config author was docs-only. The freeze-point question (O1) concerns `afef05a`, outside the range | O1 (recommendation) |
| G5 Consistency and recovery | **pass** | The bundle is built only from a published snapshot. Staging and rename are atomic. The key covers provenance (D11). One generation per process, resolved at start. P2 reproduces the snapshot and the generation bytes | — |
| G6 Transformation and reuse | **fail** | `compiler_digest()` (`attempt.rs:72-89`) omits `synth::TEMPLATE_VERSION` and the synthesis SQL, which ADR-0019 L195-200 requires. P3 shows two template versions under one compiler digest. `content_digest` then cannot tell synthesis revisions apart | F1 |
| G7 Truthful capability claims | **fail** (narrow) | `serve_unreviewed` and `budget` are accepted and ignored, so D4's reversal is inert (F3). Three oracles that DESIGN and AGENTS rely on cannot fail: the stdio test, pyrefly over `lctx_mcp/src`, and `ranking_check.py` (F6). The Python client does not apply §11.1's rejections to every shape (F7). The §1.5 retrieval failure is **honestly** labelled, so it does not fail G7 | F3, F6, F7 |

Gate results are not averaged. G6 decides the Revise; G2 and G7 are narrow and cheap to fix.

---

## 7. Findings

### 7.1 Findings

Ordered by severity. Each row is one cause.

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **`compiler_digest` does not identify the synthesis code, so neither it nor `content_digest` distinguishes template revisions.** | DM-31, DM-32, DM-48, DM-12 · G6 (G1 documentary) | **Code.** `compiler_digest()` hashes engines + `LIBRARIES`, `COMPILER_OUTPUT_VERSION`, udf version, derivations + projection digests, contracts and rules (`attempt.rs:72-89`). `synth::TEMPLATE_VERSION` (`synth.rs:42`) and the synthesis queries (`labels_sql`, `decl_sql`, `mention_sql`, `params_sql` in `synth.rs`) are not inputs. **Claims.** ADR-0019 L195-200: "`compiler_digest` gains … the projection and synthesis SQL … the template version". The ledger comment says an output change without a bump fails "(the compiler digest would not move)" (`tests/analysis.rs`), implying a bump would move it. It does not for `TEMPLATE_VERSION`. **P3.** Snapshot `fa29cd66` (`template_version` 3) and `381778b7` (4) both carry compiler digest `199d5ed98e96…`. The compiler run's producer revision is the same too, so the `lctx-compiler` producer claims one revision for two synthesis codes | Increment 2 changes a Limits or Coordinates template. After D14 those texts are not in the brief document, so the embedding keys do not move either. The rerun then has the **same** `content_digest` and compiler digest as its predecessor, with different published assertions. So: <br>• §9.8's oracle ("reruns with the same `content_digest` give identical output") reports a false nondeterminism; <br>• any reuse keyed on `content_digest` serves stale text; <br>• §B6's "rebuildable from … the `compiler_digest`" is false for every assertion's text. <br>With `--embedder none` a D14-style document change is invisible to both digests | Fold `TEMPLATE_VERSION` and the synthesis SQL (or one synthesis output version) into `compiler_digest_of`. Restate DESIGN §3.4.1's row to match the code, including the projection digests and analytics library versions it already hashes. About 10 lines | **None exists.** Extend `the_compiler_digest_follows_each_input` with a template-version argument. Record the compiler digest in each ledger row, so a version bump that does not move it fails |
| **F2** | **The §1.5 retrieval requirement fails, and increment 1 has no oracle or pre-registered procedure for deciding a fix without tuning to the gold. DESIGN's cause is right but incomplete.** | DM-40, DM-59, DM-60 · (G7 is not failed: the claim is labelled) | **The check** always exits 0 (`ranking_check.py:52`), is in no recipe, and has 2 queries over 4 briefs. **The metric.** §12(b), the pre-registered keep-rule metric, is hit@5, which is trivially 1 with 4 briefs. So D14's 4 → 2 hybrid move changed nothing the keep rule can see, and its evidence is one query. **The mechanism** (P7): <br>• the lexical leg matches only `register`, which is in every brief (df = N, idf 0.105); <br>• BM25 length normalization orders the briefs by length (0.041–0.044); <br>• `FastMCP.tool` is long because it lists 15 controls; <br>• RRF (`retrieval.py:143-149`) gives that non-discriminating leg a full vote next to the vector leg. <br>DESIGN §1.5 L144-147 names "thin briefs" and "document length", but not the fusion rule or the length penalty on controls | Increment 2 adds parameter docs and usage to briefs. That makes them longer, so the lexical leg penalizes richer briefs further. With 15–25 briefs, any gold-informed tweak to fusion, `b` or the document fields decided after looking at `fm.register` spends the development set that §9.8 and §12 depend on. The distractors are by construction sibling "Decorator to register a …" methods (D3), so §1.5's rank-1 test against them is adversarial to the lexical leg | Before increment 3's briefs, and before any further gold-informed measurement: <br>1. Amend ADR-0010 with a pre-registered retrieval evaluation: every subsystem family's aliases, hit@1 and hit@5 over the full brief set. <br>2. Decide, **with a gold-independent rationale**, how fusion treats a leg with no discriminating signal for a query (for example: a leg whose matched terms all have df = N does not vote, or near-ties are ties) and whether BM25 length normalization applies to the controls list. <br>3. Restate §1.5's criterion for the 15–25 brief corpus. <br>4. Make `ranking_check.py` exit non-zero on a miss, and give it a `just` recipe that records its conditions | **None exists.** A pytest on a constructed generation whose briefs all share the query's only lexical term: the fused order must equal the policy's (for example the vector order). The recipe's exit code for the live leg |
| **F3** | **`[briefs] serve_unreviewed` and `budget` are declared, validated and hashed, but nothing reads them.** So D4's recorded reversal ("set it to `false`") does nothing | DM-43, DM-19, DM-59 · G7 | **Declared:** `config.rs:60-64`. **Read by:** nothing in `cpg-core`, `lctx` or `lctx_mcp` (P8). **Bundle:** its `briefs` query has no `review_state` filter (`bundle.rs:92-100`). **Budget:** the committed bundle test compiles 4 briefs under `budget = 2` (`tests/bundle.rs:30`, `:122`). D16 raised `budget` from 4 to 5 "for the new seed", as if it bound | At increment 3 the operator sets `serve_unreviewed = false` to enforce §10.4. Every unreviewed brief is still served, and only the run ids change. A sixth seed publishes six briefs under `budget = 5` | Enforce both: the bundle refuses or omits unreviewed briefs when false, and the config check refuses more seeds than the budget. Or delete both fields until a consumer exists. Fix D4's "Reverse by" | **None exists.** A config test (seeds over budget are refused). A bundle test (`serve_unreviewed = false` gives no unreviewed brief, or a refusal) |
| **F4** | **The served gap metric says "no gaps" for briefs that lack whole sections.** 1.5 review O6 deferred this as an internal metric. Since 1.8 it is served to agents in every `SearchResult.coverage` | DM-08 · G2 (ADDENDUM Q4) | `coverage()` counts only assertions whose status is `unresolved` (`bundle.rs:476-480`), and only the Outcome template emits one. The live manifest reads `"unresolved_slots": {}`. Every brief has no Applicable case and no Usage pattern (§10.3), and `custom_route` has no depth or budget limit. The summary is returned as `coverage` (`server.py:191`) | An agent reading search results, or the §B11 LLM-trigger computation (§12), sees zero unresolved slots and treats the briefs as complete. The distinction ADDENDUM Q4 requires (unresolved vs absent) is lost exactly where it is served | Take O6's decision now: which sections are slots (at least Applicable case and Usage pattern, from increment 2). Emit `unresolved` placeholders for them, or serve a second map, `sections_absent`, and rename the key to what it counts. 2.1 is about to fire O6's trigger anyway | **None exists.** A fixture test: the served summary names each brief's missing sections |
| **F5** | **Three templates still state more than the fields they read** (the 1.5 review F3 cause, in new places) | DM-24, DM-59 · G2 | (i) `count` (`synth.rs:821-832`) counts witness **paths**. `chosen` includes every shortest final arc (`pass_a.rs:218-219`), definition arcs among them. For `def outer(): def inner(): …; inner()`, the finals to `inner` are one call and one definition, so the brief says "`outer` already calls `inner` (2 call sites)". No fixture has this shape (`analysis_shapes/pkg/server.py`). (ii) Limits say "calls" / "reaches call" for every boundary's final arc (`synth.rs:995-1001`). On the live pilot, 5 of 34 boundary findings end in a candidate (override-open) arc (`builtins.list.append`, `builtins.type.__new__`/`__init__`). (iii) "`X` defines `T`, a nested callable it returns or registers" (`synth.rs:838`): neither a return nor a registration is read or cited | (i) A Coordinates claim, a behavioural statement, overcounts call sites. (ii) Limits assert a definite call where the evidence is a candidate target. (iii) A nested `def` that is only stored or unused is described as returned or registered | Count call-kind arcs only. Group Limits by the final arc's modality ("may call"). Drop "returns or registers", or back it with the lexical fact (a `return` reference, or an argument use, of the nested binding). Bump `TEMPLATE_VERSION`, which F1 makes visible | **Test:** add the three shapes to `analysis_shapes`, with targeted assertions in `templates_say_what_the_findings_show` |
| **F6** | **Three oracles that DESIGN or AGENTS rely on cannot fail** | DM-60, DM-59 · G7 | (i) `test_the_server_speaks_only_the_protocol_on_stdout` (`tests/test_stdio.py`), cited in DESIGN §11.3 L2312-2313 as "writes nothing but the protocol". P5: a server that prints to stdout at import and in a tool passes the same round trip, because the MCP client logs and skips bad lines (`mcp/client/stdio.py:221-224`). (ii) `pyproject.toml` L6 ("the root environment runs their tests and type-checks them") and `just py-check`: pyrefly skips `python/lctx_mcp/src` (P1). (iii) `ranking_check.py:52` returns 0 on any outcome | A stray `print` in the served package, or a dependency writing to stdout, corrupts the stream for real stdio clients while `just check` stays green. A type error in the server passes `just check`. A §1.5 regression goes unnoticed | (i) Replace it with a raw-pipe test: spawn `python -m lctx_mcp`, send initialize and a call, and require every stdout line to parse as JSON-RPC (P5's script). (ii) Fix the pyrefly config so `lctx_mcp/src` is a project file (a `site-package-path` or `project-excludes` override). (iii) Give the ranking check a non-zero exit (F2) | Each correction is its own test. For (ii), a pytest that runs `pyrefly dump-config` and requires `python/lctx_mcp/src/lctx_mcp/server.py` among the covered files |
| **F7** | **Degraded mode covers a down service, not a malformed answer, and Python does not apply every rejection that Rust does** | DM-30, DM-43 · G7 (narrow) | `parse_embeddings` indexes `d["embedding"]` and calls `parsed.get` unguarded (`embedder.py:98-110`). P6: four shapes escape as non-`EmbedderError`. `search` catches only `EmbedderError` (`server.py:168`). Rust parses with typed `serde` and rejects every shape (`lctx-embed/src/lib.rs:55-57`). §11.1 claims both clients "apply the same rejections" | A proxy, a misconfigured URL or a different server version answering 200 with another JSON shape turns every search into a masked internal error, not a lexical-only answer | Map `KeyError`, `TypeError`, `AttributeError` and `ValueError` inside `parse_embeddings` to `EmbedderError`. Share the rejection cases through `specs/embedding/` | **Test:** a `MockTransport` returning P6's four bodies. Each search is `lexical-only` with a reason |
| **F8** | **The record of increment 1 is behind the tree.** No single item moves a gate; together they mislead the next session | DM-59, DM-55 | See §9's label audit. `STATUS.md` was last updated at `008c4fc`, before `e735fde`: it says "Next: slice 4" and lists the `pyproject` restructure as owed. DESIGN's revision history has no rows for 1.7–1.9, and a stray `> Decision: ADR-0012` line with blank lines breaks the table (L2403-2410). ADR-0019 is still `proposed`, evidence `Proposed`, after three amendments and six slices built on it. D14 amended §11.1 (governed by ADR-0010) with no ADR-0010 amendment. The 1.6 and 1.7 compact reviews were skipped with no deviation-log entry (D15 covers only 1.8) | The next session starts from the wrong slice and the wrong label strengths. ADR-0001's revisit trigger ("a step skipped twice because it costs more than it catches") looks half-fired. But the two compact reviews that did run (1.4 and 1.5) each returned Revise with P1 items, so the evidence says the cadence is earning its cost | Run the handoff skill. Add the revision rows and fix the table. Accept ADR-0019 with evidence Tested, or state its acceptance condition. Add an ADR-0010 amendment for D14. Log skipped reviews as deviations | **No mechanical oracle.** Keep this in prose: a revision-row lint would be machinery without a demonstrated consumer (DM-58) |

### 7.2 Observations

None of these moves a gate alone. Each has its fix, oracle or trigger.

| # | Observation | Evidence | Fix · oracle |
|---|---|---|---|
| O1 | **The gold freeze point is set too late, and D16 shows the cost.** ADR-0004's amendment freezes the analytics config "before the first gold scoring". D3 names that point as 3.3. But the 1.9 §1.5 check already scored compiled output against gold aliases. `afef05a` (outside this range) then edited `[seeds]` after that measurement, with no ADR. Its "mechanical" rules, written by an author who has read the gold, select `fastmcp.FastMCP.mount`, which is a gold operation (`fm.mount_proxy`) | ADR-0004 L95; D3; D16; `eval/gold/fastmcp-4.0.5.json` | Amend ADR-0004: the freeze is the first use of gold against compiled output (1.9), recorded at 6c86f53's config digest. Later config edits go by ADR, stating what gold their author has seen. Test: `check_gold.py` compares the config digest with the recorded freeze |
| O2 | `EvidenceKind::Span` always derives `documented` (`findings.rs:450`). The code comment says "a docstring span", but nothing requires a `span` to lie in a docstring | Parameters cite code bytes as `fact` today, correctly | Rule `semantic:span-is-docstring`: a `span` lies inside a declaration's docstring range. Trigger: the first template that cites code bytes (Pass B, 2.1) |
| O3 | A path made only of **definition** arcs is typed `bounded_delegation_path`. `custom_route`'s depth-1 finding is a definition, and it counts toward `ANALYSIS_BACKED`. The codebook doc (`codebook.rs:818-820`) says "a bounded path, or a candidate arc", not "a definition" | Live pilot: kind 2, depth 1, `arc_kind` definition, 1 finding | Document it in the codebook, or split the kind. Trigger: a consumer reads `finding_kind` as "calls" (the §12(a) scoring or the §9.8 ablation, 3.3) |
| O4 | At publication, `semantic:evidence-text-bytes` checks lengths only. Byte equality is checked only in the fixture test | `rules.rs:379-382`. P4: 45 of 45 pilot spans verbatim | A byte comparison against `source_files.text`, if DataFusion slices binary. Trigger: a second library or a new evidence source |
| O5 | The held-out seal is self-referential: `MANIFEST.sha256` lives inside the directory it seals. Git history (`b2fd8c1`, the only commit) is the real seal | P9 | Record the manifest's own SHA-256 in DESIGN §12, and check it in `just gold`. Trigger: before increment 5, or any commit touching `eval/heldout` |
| O6 | The served `outcome_status` is `COALESCE(…, 'unresolved')` (`bundle.rs:95`), and no rule requires exactly one Outcome assertion per brief. Two Outcome assertions would duplicate a brief row, which Python's dict would silently collapse | Synthesis always emits exactly one today | `semantic:one-outcome-per-brief`. Trigger: applicable-case splitting (2.5) |
| O7 | D8's claim that the wider policy "cannot be abused" is stronger than its enforcement. The derivation requires *some* documented evidence, not *parameter-doc* evidence, so a parameter assertion citing the seed's summary docstring would publish `documented` | `derive_status`; `ASSERTION_POLICY` | A rule tying a `documented` parameter's evidence to the parameter's own docs. Trigger: 2.1 (parameter docs) |
| O8 | DESIGN §1.5 L138 writes "**Measured** (…): **failed**", mixing a §D label with a check outcome (ADDENDUM §3) | — | Two sentences: the ranks (Measured, with conditions) and the check outcome (`failed`, with its command and date) |
| O9 | D11's key moves with `snapshot_id`, so every rerun makes a new generation directory over identical files (P2: `8d5e5ba2` and `6c8145b5`) | By design | Increment 4's activation should compare file digests, not keys, if it deduplicates |

**Checked and clean.** Each entry says what would break without it.
- **Normalization.** Without the cast and builder rebuild (`bundle.rs:159-250`), `Utf8View` or
  scan metadata would change bytes between rebuilds. P2 reproduces all nine files across a
  relocated rerun.
- **Two schema authorities, reconciled.** Python's `expected_schemas` is a hand mirror, and it
  would drift silently. `test_expected_schemas_are_rusts_known_answers` and
  `serving_schema_digests_are_the_shared_known_answers` hold both sides to one file.
- **Key semantics (D11).** Hashing only the file digests would let two provenances collide under
  one key.
- **The vector inner join.** `bundle.rs:143-146` would silently drop documents without vectors,
  but the count check at `:553-564` refuses the build instead.
- **The MERGE ordering.** One merge after all batches, so a failed embedder leaves no partial
  cache rows.
- **Lifespan checks.** The sha, rows, schema digest (manifest vs expected vs actual), key, spec
  and unit-norm checks all fail at connect (Tested). The generation path is resolved once, at
  start (`__main__.py`).
- **Error mapping.** `CapabilityError` becomes `ToolError` or `ResourceError`, and argument
  bounds become `ToolError` (Tested). P5 saw the snapshot-mismatch error in-band as `isError`.
- **Stdout in fact.** P5: the real server wrote only JSON-RPC.
- **Verbatim evidence at pilot scale** (P4).
- **Determinism** (P2): identical `findings`, `witnesses`, invocations and briefs across a
  relocated rerun.
- **Gold isolation.** No compiler path reads `.claude/` or `eval/` (grep). Only `eval`-side
  scripts read the gold.

### 7.3 Applicability and verdicts

**Groups that bore on this scope:**

| Group | Why |
|---|---|
| 2 | Absence on the served surface (F4); validity at the serving boundary |
| 3 | Content ids and the reproducibility key (F1) |
| 5 | Templates as transformations (F5) |
| 7 | Reuse keys (F1); relationship kinds (O3) |
| 9 | The Python client as a provider adapter (F7) |
| 10 | Lineage and reproducibility (F1, P2) |
| 11 | Verification (F6) |
| 12 | Claims and proportionality (F2, F3, §8) |

**Groups that did not:**
- **1** (authority): documentary only. Every duplicated expression is reconciled by a test.
- **4** (declarative composition): the kind policy is declared once and generates its rules,
  which is proportionate.
- **6** (effects): checked under G4 and G5, and clean.
- **8** (performance): no performance claim is at stake. The pilot costs 32 s and 3.6 GiB, and
  synthesis 0.05 s.

**Verdicts:**

| Verdict | Principles |
|---|---|
| Violated | DM-31 and DM-48 (F1); DM-08 (F4); DM-24 (F5); DM-43 (F3, F7); DM-60 (F6) |
| Satisfied | DM-11 and DM-15 (content ids, P2); DM-14 (publish-then-bundle); DM-23 (the generation is traceable and non-competing); DM-40 (sorted adjacency, total arc order, P2); DM-42 (a declared type grammar that refuses anything else); DM-46 (witness lineage, verbatim evidence) |
| Unresolved | DM-59 for §1.5's retrieval claim: failed, honestly labelled, with the procedure for a fix undecided (F2) |

---

## 8. Alternatives and architectural leverage

| Alternative | Duplication and locality | Correctness and operational risk | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| **Current: Arrow IPC generation, schemas mirrored in Rust and Python and tied by known answers; BM25 + cosine + RRF + promotion** | One served schema, two expressions, one oracle. Adding a served column touches `bundle.rs` (Rust), `generation.py` and the known answers | Sound and reproducible (P2). The fusion rule's behaviour on a non-discriminating leg is undecided (F2) | About 930 lines of Rust and 560 of Python for the serving side | Bundle 0.09 s; server load trivial at 4 briefs | Keep. It is paid for, and Arrow is LanceDB's input format if its trigger fires |
| **A simpler serving format: sorted JSON Lines with base64 little-endian `f32` vectors; Pydantic models as the Python contract** | Removes the type grammar and IPC options, but keeps two expressions (serde structs and Pydantic) | Equally byte-stable if Rust alone writes it. Loses Arrow's typed `FixedSizeBinary` ids | About half the serving code | Irrelevant at this scale | Would have been the lighter choice in 1.7. Switching now buys nothing (DM-58 cuts both ways) |
| **Simpler retrieval: vector ranking + exact-symbol promotion, with the lexical leg only in degraded mode or as a tie-break** | Removes fusion, a decision that currently has no stated policy | Degraded mode keeps BM25. At 4–25 briefs with an 8B embedder, the measured failure is exactly a lexical vote on a non-discriminating term | Less code; a `bm25s` dependency only for degraded mode | The only live evidence: the vector leg ranked the seed first where hybrid ranked it second (author's measurement; blocked here) | The **counter-design F2 must be judged against**, through the pre-registered procedure, not adopted on `fm.register`'s two aliases |

**Justified abstractions.** `ProjectionSpec`, `ASSERTION_POLICY`, the ledger and the serving
digest grammar each have a consumer and an oracle.

**Machinery without a consumer:** the `[briefs]` fields (F3).

**Ordinary code.** The templates and the sentence and paragraph heuristics should stay ordinary
code behind `TEMPLATE_VERSION`. A template DSL would not pay for itself at six kinds.

**Deep counter-design check.** The smallest brief that meets §1.1 without Pass A would be
docstring + signature + aliases. Pass A adds Coordinates (1–2 sentences per brief) and the
limits, which D14 has now taken out of retrieval. Whether that difference helps agents is the
question increment 5's raw-vs-compiled evaluation answers. Nothing in increment 1 settles it,
and nothing in increment 1 claims to.

---

## 9. Verification and measurement plan

| Claim or risk | Label now supported | Oracle | Conditions and expected result | Current result or gap |
|---|---|---|---|---|
| The rerun reproduces snapshot and generation | **Tested** (P2, 2026-09-23) | a manual rerun; no committed test at pilot scale | fake embedder, relocated environment and store | passed. The fixture-scale test exists (`a_generation_rebuilds_to_the_same_bytes`) |
| Same `content_digest` → same output | Proposed until F1 | F1's unit test and ledger check | template bump ⇒ compiler digest moves | **fails today** (P3) |
| Served schemas agree across languages | **Tested** | known answers on both sides | — | passed |
| Stdout carries only the protocol | behaviour **Tested** by P5; the named test cannot fail | F6(i) raw-pipe test | every line is JSON-RPC | gap |
| Degraded mode | **Tested** for a down service | F7 test | malformed 200 ⇒ `lexical-only` | gap |
| Retrieval by task wording (§1.5) | **Measured** failed (author, live); lexical-only 1 of 2 re-measured here | F2's recipe and pytest | pre-registered set, hit@1 and hit@5 | undecided |
| Evidence is verbatim | **Tested** (fixture) + P4 (pilot) | O4 | — | length-only at publication |
| Review gate and budget | **Proposed** (inert) | F3 tests | `serve_unreviewed = false` ⇒ none served | gap |
| Live Rust/Python conformance | **Tested** in spike E2; 1.9 live run per commit message | `just embed-conformance` | cosine ≥ 0.9995 | **blocked** here |

**Label audit of DESIGN**, as the tree supports it:

| Section | Written | Supported | Note |
|---|---|---|---|
| §B12, §B13, §B14 | Proposed / Interface-checked | **Implemented**, **Tested** (bundle tests, 19 `lctx_mcp` tests, spec, request-body and MERGE tests) | under-labelled |
| §3.4.1 `compiler_digest` row (L567) | Implemented, Tested | stale in both directions (F1) | — |
| §6.4 Activation (default Interface-checked, from §6's header) | — | **Proposed** (increment 4) | not implemented |
| §9.8 "reruns with the same `content_digest` give identical output" | Proposed | Tested at pilot scale for the fake embedder (P2); false for synthesis-only changes until F1 | — |
| §11.1 conformance | Tested (E2 spike) | add slice 1.9's live run with its conditions | — |
| §11.3 stdio test | Tested | **overclaimed** (F6) | — |
| §1.5 retrieval | "Measured … failed" | split the label from the outcome (O8) | — |
| ADR-0019 `evidence:` | Proposed | Implemented and Tested | status still `proposed` (F8) |

**Cost accounting** (this review):
- Pilot compile: 32.3 s wall (extract 27.3 s), peak 3,640 MiB.
- Synthesis 0.05 s; embed (cached) 0.07 s; bundle 0.09 s.
- `just test-all`: 1 min 56 s, of which `every_rule_kind_rejects_its_violation` takes 106 s.

Nothing here needs optimizing.

---

## 10. Exceptions, unresolved decisions and the deviation log

**Unresolved decisions** (each blocks nothing in increment 2, but must be decided where stated):

| # | Decision | Owner | Decide before |
|---|---|---|---|
| U1 | The fusion policy for a non-discriminating leg, and the pre-registered retrieval evaluation (F2) | operator, by an ADR-0010 amendment | any further gold-informed retrieval measurement; increment 3's briefs |
| U2 | Which brief sections are slots, so absence is counted (F4; 1.5 review O6) | operator, by an ADR-0019 amendment | slice 2.1's first Limits template |
| U3 | Where the gold freeze falls, and whether D16's seed edit needs an ADR (O1) | operator, by an ADR-0004 amendment | slice 2.1's first pilot |

**No SHOULD-level exception** is requested. F1–F7 are MUST-level gaps with small fixes; none is
waived here.

**The deviation log, judged:**

| # | Verdict | Reason |
|---|---|---|
| D1 | **Sound as a stopgap** | Sealed before any brief (the seal verifies, P9). The independence caveat is disclosed. The reversal (operator-written tasks before increment 5) is the right trigger. Harden the seal (O5) |
| D2 | **Sound** | ADR-0004 amended. The keep rule must judge the served stack |
| D3 | **Sound on independence; the freeze point is not** | A docs-only author under stated rules is the right remedy for ADR-0019 review F3. But "frozen before 3.3" leaves the config editable after 1.9's gold-based check (O1) |
| D4 | **Unsound as recorded** | Its reversal is inert: nothing reads `serve_unreviewed` (F3). Serving unreviewed briefs in increment 1 is consistent with §10.4, which starts at increment 3 |
| D5 | **Sound** | Typed definition arcs answer 1.4 review F1 better than marking a boundary. Residual template claims (F5(i), (iii)) and a kind conflation (O3) remain |
| D6 | **Sound** | Fail-closed, and it refuses no pilot seed |
| D7 | **Sound** | Decidable and tested on a corpus fixture. The pilot never reaches leg 2 (all four seeds have docstrings) |
| D8 | **Sound for increment 1, overstated** | "Cannot be abused" needs a parameter-evidence rule (O7) |
| D9 | **Sound** | A depth ≥ 2 boundary always accompanies a delegation, so D9 changes the label only for a seed's own depth-1 calls into dependencies. That is Pass A's positive observation. Note that the label then means "the seed's body calls something", so it discriminates little |
| D10 | **Sound** | A finding keeps the stop's invocation link and is excludable by kind |
| D11 | **Sound** | See O9 for increment 4 |
| D12 | **Sound** | A declared migration, and it makes §6.4's single derivation path true |
| D13 | **Sound** | ADR-0001's consumer rule |
| D14 | **Sound in principle, weakly evidenced, under-recorded** | The category argument (analysis scope is not a capability limit) is gold-independent, and 1.5 review O7 registered it before any measurement. But its measured benefit is one query and invisible to the pre-registered metric (F2). The governing ADR-0010 was not amended (F8). Its `TEMPLATE_VERSION` bump did not move the compiler digest (F1) |
| D15 | **Acceptable** | This review covers 1.8. The 1.6 and 1.7 skips were not logged (F8) |
| D16 | **Out of range; flagged** | O1 |

---

## 11. Decision and implementation changes

**Decision: Revise (small).** Increment 1 meets its §1.2 exit as a path:
- every stage in the row exists and runs on real FastMCP 4.0.5;
- it reproduces byte for byte across a relocated rerun (P2);
- it serves both tools over stdio with a correct stdout (P5).

G6 fails: `compiler_digest` omits the synthesis code that ADR-0019 put in it (F1). That is a
MUST-level identity gap on behaviour the design claims, and slice 2.1 is about to exercise it.

G2 and G7 fail narrowly:
- the served gap metric (F4);
- template residuals (F5);
- inert config controls (F3);
- oracles that cannot fail (F6);
- one degraded-mode path (F7).

§1.5's retrieval criterion is failed and honestly recorded. What is missing is a gold-independent
way to decide the fix (F2).

None of this reopens a §B decision. Increment 2 may proceed, with F1 landed before 2.1's first
`TEMPLATE_VERSION` bump.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1**, before 2.1 bumps `TEMPLATE_VERSION` | F1: fold the template version and synthesis SQL into `compiler_digest`; restate DESIGN §3.4.1 | DM-31, DM-48 | The pilot compiler digest moves between template versions | Unit test on the digest's inputs; compiler digest in each ledger row |
| **P1**, before increment 3's briefs and any further gold-informed retrieval change | F2: an ADR-0010 amendment with the pre-registered retrieval evaluation and a fusion policy for non-discriminating legs; restate §1.5 for 15–25 briefs; the ranking check fails on a miss | DM-40, DM-59, DM-60 | The amendment exists before the next live ranking run | pytest on a shared-term generation; a `just` recipe |
| P2, with 2.1 | F4 and U2: count absent slot sections in the served summary | DM-08 | The fixture summary names missing sections | Fixture test |
| P2, with 2.1's templates | F5: count call arcs only; say "may call" for candidate final arcs; drop or back "returns or registers" | DM-24, DM-59 | The three fixture shapes render truthfully | Assertions in `templates_say_what_the_findings_show` |
| P2 | F3: enforce or delete `serve_unreviewed` and `budget`; correct D4 | DM-43, DM-59 | Config and bundle tests | The same tests |
| P2 | F6: raw-pipe stdio test; the pyrefly config covers `lctx_mcp/src`; the ranking check's exit code | DM-60 | `pyrefly dump-config` lists `server.py`; the noisy-server shape fails the new test | The tests themselves |
| P2, with 2.1's first pilot | O1 and U3: amend ADR-0004's freeze point; decide D16's status | DM-28, DM-59 | The freeze digest is recorded | `check_gold.py` compares the digests |
| P3 | F7: parse errors become `EmbedderError` | DM-30, DM-43 | P6's shapes degrade | `MockTransport` test |
| P3 | F8: handoff, revision rows, labels (§9), ADR-0019 acceptance, ADR-0010 amendment for D14, log the skipped reviews | DM-59, DM-55 | `adr lint`; STATUS names increment 2 | none (prose) |

### Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| O2: `span` must lie in a docstring | No template cites code bytes as `span` yet | The first template that does (Pass B, 2.1) |
| O3: definition-only paths typed as delegation paths | The text distinguishes them, and no consumer reads the kind as "calls" | §12(a) scoring or the §9.8 ablation reads `finding_kind` (3.3) |
| O4: a byte-level evidence rule | P4 verified the pilot; the fixture test covers bytes | A second library, or a new evidence source |
| O5: seal the held-out manifest from outside | Git history is the seal until then | Before increment 5, or any commit touching `eval/heldout` |
| O6: one Outcome per brief | Synthesis emits exactly one | Applicable-case splitting (2.5) |
| O7: parameter-doc evidence for a `documented` parameter | No documented parameter exists yet | 2.1 (parameter docs) |
| O9: activation dedupes by content | No activation yet | Increment 4 |
| §8's JSON bundle | Already paid for | A third served-schema grammar extension, or a second served consumer |
| ADR-0019 review O10: the rebuild bound under vLLM nondeterminism | Its trigger stands | Slice 3.1 |
| 1.4 review §8: a petgraph-free BFS | Its trigger stands | Increment 2 ends with no analysis using the `Graph` container |
| 1.5 review O6 | **Promoted to F4**, because it is served now | — |

**Final check.**
- The design's claims match the evidence except where §9's label audit says otherwise. The
  largest gap is `compiler_digest` (F1).
- The supported scope matches the implemented guarantees, apart from the inert `[briefs]`
  controls (F3).
- Later extensions have a clear, validated path once F1 makes synthesis revisions visible and
  F2 fixes how retrieval changes are decided.
