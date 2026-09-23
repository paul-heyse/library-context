# Design review: increment 2, slice 2.2 (Pass C and usage patterns), compact

## 1. Decision and scope

**Decision: Revise.** G2, G3 and G7 fail on text that is published or publishable (§6). No §B
decision is reopened. The handoff relation is sound for what it counts. Two things are wrong:
what it admits as an endpoint, and how the usage pattern checks that it is self-contained.

**Proposal:** Pass C (DESIGN §9.3) and usage patterns (§10.5):
- the declared relation `cpg_schema::flows::handoffs_sql`;
- the kernel `lctx_analytics::pass_c`;
- `cpg_core::usage`, which selects the pattern;
- the Stage F templates for `usage_pattern` and `handoff`, and the pattern's code in the brief
  document;
- the fixture `docs_shapes/usage/handoff.py` and the test
  `handoffs_and_usage_patterns_come_from_official_code`;
- the codebook appends and the kind policy rows;
- ADR-0019's trailing Amendments heading (`e42ff4e`);
- the closure of the 2.1 review's O7 in `30561df`.

**Status:** Implemented. The Tested scope is set out in §9.
**Reviewer / author:** design-reviewer subagent (fresh context) / the slice-2.2 session.
**Affected revisions:** `cf0990f`, `e42ff4e` and `30561df` (all reviewed at `30561df`).
- Pilot snapshot `c9309c7813d1e4b9a9b238611924ef40` (FastMCP 4.0.5, published at `30561df`).
- HEAD moved to `8b27dc0` (slice 2.3) during the review. Among the reviewed files, that commit
  changes only the visibility of `flows::codes` and `flows::call_targets`.

**Observable outcome:** each brief's Usage pattern section holds:
- one verbatim, dedented statement subset of official usage code (`documented`);
- up to three `handoff` sentences (`structurally_observed`).

**Supported scope and non-goals:** named handoffs (`x = producer(...)`, then `consumer(..., x)`
later in the same block) and nested handoffs (`consumer(producer(...))`). No type-compatibility
candidates, no executed fixtures, and no CFG (§13).

### Method and coverage

**Read at the grain cited, at `30561df`:**
- `flows.rs`: `handoffs_sql` and the fragments it shares (`call_targets`, `starred`,
  `maps_formal`);
- `pass_c.rs`, all of it;
- `usage.rs`, all of it;
- the Pass C wiring in `analyze.rs`;
- in `synth.rs`: the preferred-site map, the `usage_pattern` and `handoff` drafts, and the
  document line;
- in `findings.rs`: `ASSERTION_POLICY`, `FINDING_STATUS`, `EVIDENCE_STATUS`, `derive_status` and
  `ANALYSIS_BACKED`;
- `semantic:brief-cites-analysis`;
- the codebook and rules-snapshot diffs;
- the fixture, the test and the `smoke.py` diff;
- `scripts/adr.py`'s accepted-record check.

**Also read:**
- DESIGN §1.5, §3.2 (`findings` row), §9.3 and §10.2–§10.5 at `30561df`;
- ADR-0019 with its amendments;
- deviation log D17, D24 and D25;
- the Deferred rows C3 O4, C4 O2 and C5 O2;
- the 2.1 review's O7 and its disposition;
- slice 2.3's `communities.rs`, for §8 only (not reviewed).

**Ran** (2026-09-23):

| Check | Outcome | Command and conditions |
|---|---|---|
| Reviewed commit | **passed**: nextest 159/159 | `git archive 30561df` into the scratchpad, then `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass --no-fail-fast` with a separate `CARGO_TARGET_DIR` |
| Probe 1: `review_probe_pass_c_and_usage` | **passed**. This means F1(b), F1(c), F2, F3 and F4(a) reproduce, each with `validate()` empty | Scratch copy only: three fixture directories (`probe_a`, `probe_b`, `probe_c`) and one test appended to `syntax.rs`. The fixtures are quoted in the findings |
| Probe 2: `review_probe_pass_c_is_identical_across_order_and_location` | **passed**: 2,477 bytes of handoff findings, members, `usage_pattern`/`handoff` assertions, `example` evidence and Pass C invocations are identical | Scratch copy: two compiles of `usage` plus `probe_a`, relocated, the second with `reverse_module_order` (which reaches the corpus run through `..input.clone()`, `cpg-extract/src/lib.rs` `run`) |
| Free-name check over the served patterns | It flags `Request` (custom_route) and the probe's `handler_from_elsewhere`, and nothing in the other four pilot patterns | `python3 free_names.py` in the scratchpad: an `ast` walk, loaded names minus bound names minus builtins |
| Pilot queries | See the findings | `target/release/lctx query --store build/store --snapshot c9309c78…`, read-only. `handoffs_sql` was printed from the export and run verbatim |
| `adr lint` | **passed** (19 records) | `uv run python scripts/adr.py lint` in the repo at `8b27dc0`, clean tree |
| pytest (`smoke.py`, `test_server.py`), `just test-all`, `just pilot` | **not_run** | As instructed, the published snapshot stood in for a pilot run. The Python side was read, not run |

The scratch target and export were deleted after the runs.

**Not attacked, so asserted:**
- the bundle and serving path for patterns, beyond reading the smoke's parse;
- `dedented` on tab or mixed indentation, and on multi-line strings inside an indented
  statement;
- Pyrefly's `init` targets for a class without its own `__init__`;
- the correctness of each pilot occurrence beyond the aggregates quoted. The five patterns and
  two doc blocks were checked against source.

## 2–4. Authority, contracts and derivation (compressed)

**Authority.**
- `handoffs_sql` is declared once. Its text enters `flows::digest()`, which feeds both the
  compiler digest and each Pass C invocation's `projection_digest`.
- The argument → formal mapping is the shared fragment. **2.1 review O7 is closed:**
  `handoffs_sql` joins `maps_formal("fm", "o", "sr", "ct.receiver")` with
  `LEFT JOIN starred sr ON sr.call_node_id = o.consumer_site` (`flows.rs:522,528`). This is the
  same fragment and `*` guard as `argument_flows_sql` (`flows.rs:296`).
  - Pass B's known-answer test (`swap(*extra, name)`) exercises the guard.
  - Pass C's fixture has no `*` shape, although the 2.1 row named "Pass C's known-answer test"
    as the regression protection. The closure holds by construction.
- The role order has one definition: `usage::crate_rank` calls `pass_c::role_rank`. The
  candidates SQL restates it as a `CASE`, and the two agree.

**Q1: the handoff relation, case by case** (`flows.rs:452-536`).

| Case | Code | Evidence | Verdict |
|---|---|---|---|
| Single binding | `single`: one binding event per (scope, name), counting `del`, annotation-only and `global` events | fixture `rebound` (not counted) | sound, conservative; Tested |
| Read once, receiver reads as setup (D25) | `reads` counts references except those whose name is an `ExprAttribute`'s `value` (`flows.rs:465`) | fixture `helper.run()` then `primary.tool(helper)` (counted), `twice` (not) | sound for bare reads. The exclusion covers **every** attribute access, not only receivers (F4b) |
| Same block, later statement | `st.parent_node_id = bd.block AND st.field = bd.block_field AND st.ordinal > bd.after` | fixture | sound. A consumer in another branch, handler, clause or nested block is rejected. Within one loop body, each iteration rebinds |
| Consumer is its statement's value, awaited or not | `statement_of`: parent `StmtExpr`/`StmtAssign`, or `ExprAwait` under one | code | sound. `return c(x)`, `y: T = c(x)`, `c(x).m()` and `x = await producer()` are not recognized (coverage, conservative) |
| `with` items | follows from `statement_of`, for the **named form only** | code | named: excluded. Nested: counted (O5) |
| Nesting | an argument whose value node is a call | fixture `primary.tool(make_server("nested"))` | sound: the argument is the producer's result |
| Conditional blocks, loops | same-block plus ordinal | code | sound |
| Reassignment, second bare consumer, return, store | `single`, then `n = 1` | fixture | sound; Tested |
| Chained assignment `a = b = p()` | two `Assignment` bindings of one call, each read once | probe A | **false**: two consumers counted as two clean handoffs (F4a) |
| Endpoint domain | any `function` node (`flows.rs:88`) | pilot, probe A | **wider than stated** (F2) |
| Occurrence uniqueness | `DISTINCT`, which includes `consumer_modality` (never read by the kernel) | pilot: 2,233 rows = 2,233 distinct (consumer site, producer site, formal) | sound on the pilot |

**Q2: usage-pattern selection** (`usage.rs:374-424`).
- The builder takes the innermost statement holding the site (`usage.rs:376`). It then adds,
  transitively, the same-block statements before it that bind a read name, and imports from
  anywhere. It refuses a read bound elsewhere.
- Three things are never examined:
  - reads with no binding (`usage.rs:390`: `None` is taken for "a builtin", but the reads query
    at `usage.rs:313` selects only `binding_id`, so an unresolved read is also `None`);
  - names read only in annotations (§10.5's "Not modelled");
  - the statement that encloses the block.
- Publication checks that the code parses (`usage.rs:495`, and `ast.parse` in the smoke).
- The text is verbatim per statement; the code is each statement dedented by its own indent.
- Every claim in the text is grounded (the path and the code), except that a doc block's path
  is the compiler's materialized module (F5).

**Q3: statuses and the no-bypass rule.**
- `usage_pattern`: one `example` evidence row per statement (`documented`), plus the cited
  handoff finding (`structurally_observed`). `derive_status` gives `documented`, and the policy
  permits `documented` and `fixture_checked` (`findings.rs:387`).
- `handoff`: the finding alone, so `structurally_observed`, which is the only status the policy
  permits.
- `semantic:assertion-status-derived` and `semantic:assertion-policy` pass in the test and in
  every probe.
- Two caveats:
  - Test statements are also `example` evidence, so a test-sourced pattern is `documented`.
    §10.2's row reads "stated in an official docstring, doc or example" (L2132). F1(c) makes
    this consequential.
  - `Handoff` is in `ANALYSIS_BACKED` (`findings.rs:577`), which is right under §1.5. F2 lets a
    doc-local helper be that backing.

**Q4: determinism and identity.**
- `handoffs_sql` is totally ordered over unique rows.
- `pass_c` groups in a `BTreeMap` and sorts each group by (role, path, start, sites).
- A finding's id hashes its members (role, ordinal, node, path label) and `witnesses_omitted`.
  The score is lineage (`pass_c.rs:220`).
- `usage.rs` reads `reads` in unordered SQL order, but the result is a transitive closure. The
  winner is chosen by `(role, !handoff, len)` with a strict `<` over an ordered candidate list.
- Module ids hash the release id and the relative path.
- Probe 2 confirms all of this; no repository test does (F6).

## 5. Journey: the pilot `FastMCP.tool` brief's Usage section

1. `usage::patterns` ranks the example `examples/apps/inspector_demo.py` first, as the smallest
   example candidate. It publishes `mcp = FastMCP(...)` and `@mcp.tool() def fail(): """Always
   raises an error."""; raise ValueError(...)`, `documented`.
2. The pattern's code enters the embedded brief document (`synth.rs:1776`).
3. Pass C's three most frequent handoffs follow:
   - `require_scopes` → `auth` (24);
   - `require_roles` (2);
   - "what `_lctx_blocks.d_docs_servers_authorization_mdx_81284412.block_3.require_tenant`
     returns" (1).

   The last is a function the doc page defines for itself (block_3's
   `def require_tenant(tenant_id)`), under a module name that exists only inside the compiler
   (F2, F5).

The `custom_route` brief's pattern is doc block 10 of `docs/deployment/running-server.mdx`.
- The block imports `from starlette.requests import Request`.
- The pattern reads `Request` in `health_check`'s annotation but drops that import (F1a).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **pass** | One relation declaration, digested twice. One mapping fragment (2.1 review O7 closed). One role order. `FINDING_STATUS` and `ASSERTION_POLICY` generate the rules (rules-snapshot diff). The spine is stale in places (O2), but no second authority exists in code | O2 (P3) |
| G2 Semantic fidelity | **fail** | A published pattern drops an import it needs (F1a, pilot). A pattern drops its enclosing `pytest.raises` (F1c, probe B). A doc-local helper is published as a direct API connection (F2, pilot). A pattern cites a handoff it does not show (F3, pinned by the test). A chained alias counts as two clean handoffs (F4, narrow) | F1, F2, F3, F4 |
| G3 Validity | **fail** | §10.4's "every snippet … refers only to existing public APIs" and §10.5's self-containment are the rejection path, and the code implements only "parses". An unbound name reaches publication with `validate()` empty (F1a pilot, F1b probe) | F1 |
| G4 Hidden behavior | **pass** | Pass C and `usage` read session tables only. The Ruff parse is a pure function. `analytics.toml` is unchanged. The fastmcp skill is not an input | — |
| G5 Consistency and recovery | **pass** | No new publication path. The analysis rows go through the attempt's write with generated key, ref and codebook rules, whose value sets were extended in the rules snapshot. Not re-attacked | — |
| G6 Transformation and reuse | **pass** (Tested by the review probe only) | `COMPILER_OUTPUT_VERSION` 6 and `TEMPLATE_VERSION` 7 were bumped. The flows digest carries `handoffs_sql`. Ids are content-derived with the score as lineage. Probe 2 shows identical rows across location and module order, but no repository test covers Pass C or usage (F6) | F6 (P3) |
| G7 Truthful capability claims | **fail (narrow)** | §10.5 L2272 "a candidate that reads a name bound anywhere else … is refused" is false for unresolved reads, and L2262 "removes no precondition" is false for annotations and enclosing constructs (F1). The codebook's "one public callable" (`codebook.rs:842`) and §9.3's "public APIs" (L1938) are not enforced (F2). "The handoff it shows" (L2276) is false for producer seeds (F3) | F1, F2, F3 |

## 7. Findings

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **A usage pattern is accepted when it parses and its resolved reads are block-local.** Names bound nowhere, names read only in annotations and the enclosing construct are never checked | DM-07, DM-24, DM-59 · G2, G3, G7 | `usage.rs:390` `let Some(binding) = binding else { continue; // a builtin }`. The reads query (`usage.rs:313`) selects `rr.binding_id` only, so a builtin (`builtin_name`) and an unresolved read (`reason`) both arrive as `None`. `usage.rs:376` takes the innermost statement, and nothing reads its parent. Publication is `parse_module(&code).is_err()` (`usage.rs:495`); the smoke adds `ast.parse`. **Instances:** **(a) Pilot, published.** `FastMCP.custom_route`'s pattern reads `Request` in an annotation. Block 10 imports it, and the pattern drops the import (1 of 5 pilot patterns). §10.5 L2278 discloses the mechanism. **(b) Probe C.** `srv = make_server("u"); srv.tool(handler_from_elsewhere)` is published `documented`. The pilot has 753 unresolved reads in 387 doc-block modules (C5 O2's figures, re-queried). 14 seed candidate statements for `tool` and `resource` read one (`json`, `do_expensive_work`, `products_db`, …). None won only because an example outranked them. **(c) Probe B.** Inside `with pytest.raises(TypeError):`, the pattern `s = make_server("b"); s.tool(1)` is the seed's published `documented` usage pattern, and the `with` is gone. On the pilot, 31 test sites of the five seeds sit directly in a `with` body, and 29 of those `with` statements contain `raises(`. **(d)** L2246 "refers only to existing public APIs" has no check. It does not fire on the pilot: `fastmcp.FastMCP` and `fastmcp.server.auth.require_scopes` are exports | FastMCP declares `Requires-Python >=3.10`. On 3.10–3.13, the custom_route pattern raises `NameError` when `health_check` is defined. On 3.14 (the pilot env) it fails only when the annotations are evaluated. For a seed whose only official use is in tests (likely among increment 3's 15–25 seeds), the brief's Usage pattern can be the call a test asserts **fails**, labelled `documented` and embedded for retrieval | In `usage.rs` (about 40 lines): (1) select `builtin_name` and refuse a read that has neither a binding nor a builtin name; (2) refuse a candidate whose block is not a module or function body, or carry the enclosing headers; (3) after assembly, walk the Ruff AST, annotations included, and refuse any loaded name that is not bound, imported or builtin (or add the module import that binds it); (4) implement or narrow L2246's public-API clause | **None exists.** Tests: probe B's test module and C5 O2's own oracle (a `docs_shapes` document with two blocks, the second using the first's `mcp`), each asserting no pattern. Smoke: the free-name check. On the pilot it flags exactly `Request`, and nothing in the other four patterns |
| **F2** | **Handoff endpoints are any function the call graph reaches, including functions the usage code defines.** The pilot publishes a doc-local helper as an API connection | DM-09, DM-59 · G2, G7 | `targets` is `call_targets()`: `edges.dst_kind = function` under accepted evidence (`flows.rs:88`). Nothing restricts it to the release or to public callables. Against this, three statements: the relation's doc comment ("what a release callable returns"); the `Handoff` code's doc comment ("what one public callable returns", `codebook.rs:842`); and §9.3's question ("Which public APIs already connect without an adapter?", L1938). **Pilot:** 668 of 2,233 occurrences (30%) have a producer or consumer defined in tests or doc blocks. 3 of the 10 handoff findings have such an `other`: `…block_3.require_tenant`, `…block_6.require_access_level` and `tests.server.test_mrtr_guards.two_question_server`. `FastMCP.tool`'s brief publishes the first. **Probe A:** `examples.probe_handoffs.local_factory` → `Server.tool` is published | Pass C exists to show that no adapter is needed. Here, a user-written adapter is published as the connection itself, under a module name only the compiler has. `Handoff` is in `ANALYSIS_BACKED`, so such a finding can label a brief analysis-backed on the strength of a doc-local helper | Restrict both endpoints to release declarations (the declaration's module has `source_files.role = release`), or to public callables. Slice 2.3's `communities::public_callables_sql` (`8b27dc0`, not reviewed) already computes the latter. One join in `handoffs_sql`; the flows digest changes | **None exists.** Add probe A's `local_factory` to `usage/handoff.py`, and assert its absence in `handoffs_and_usage_patterns_come_from_official_code` |
| **F3** | **A producer seed's usage pattern cites a handoff it does not show, and the test pins it** | DM-46, DM-59 · G2 | Stage F cites every handoff finding that has a `ConsumerSite` **or `ProducerSite`** member equal to `pattern.site` (`synth.rs:1451`). `build` adds only earlier statements, so a producer site's pattern never holds the later consumer. `syntax.rs:1058` asserts `pkg.make_server`'s pattern `helper = make_server("helper")` with one cited finding, and `primary.tool(helper)` is absent. Probe A: `a = b = make_server("chain")`. Not fired on the pilot: every seed there is a consumer | L2276 ("citing … the handoff it shows") is false for any factory seed. The preference also ranks a one-line `x = producer(...)` above more informative candidates, because it is a handoff site | Extend a named producer site's pattern forward to its consumer statement (same block, later; check its reads the same way), or cite a handoff only when the pattern holds its consumer site | Flip the test's second assertion. Add a rule: each handoff a `usage_pattern` cites has a consumer-site member inside one of the assertion's `example` spans, with an injected case |
| F4 | **The named form rejects less than §9.3 and D25 state** | DM-24, DM-59 · G2 (narrow) | **(a) Chained assignment.** In `bound` (`flows.rs:472-483`), a `StmtAssign` with two name targets yields two bindings of one call. Probe A (`a = b = make_server("chain"); host.tool(a); host.tool(b)`) counts 2 occurrences, although one object reaches two consumers. **(b) The receiver exclusion** drops every read that is an `ExprAttribute`'s value (`flows.rs:465`), where D25 and L1959 say `x.method()` and `@x.tool`. Pilot: of 456 values consumed by any `FastMCP` method, 49 have another attribute read (assert comparisons, `f(x.attr)`, `y = x.attr`), and 17 of them pass or store the attribute's value | The occurrence count includes cases §9.3's "additional consumers" and "escapes" exclude. Each counted occurrence still passes the producer's result, so no sentence is false | Reject a producer statement with more than one target. Either narrow the exclusion to an attribute that is a call's callee or a decorator, or restate D25 and §9.3 as "any attribute access on `x`" | Fixture lines for both shapes in `usage/handoff.py` |
| F5 | **Doc-block provenance is published under the compiler's materialized path** | DM-46 · G2 (narrow) | "From `{path}`" (`synth.rs:1422`) and "e.g. in `{example}`" (`synth.rs:1495`) use `source_files.path`, which is `_lctx_blocks/d_<document>/block_<n>.py` for a doc block (§3.2 `docs`). Pilot: 2 of 5 patterns and 5 of the 8 published handoff sentences name such a path | Nobody can open the cited file. The real location (`docs/deployment/running-server.mdx`, block 10) is one join away (`code_blocks.module_path` → `documents.path`) | Render a doc block as its document path plus block ordinal, in the pattern text and in member labels | A `semantic:` rule, "no assertion text contains `_lctx_blocks`", with an injected case |
| F6 | **No repository test covers Pass C or usage determinism** | DM-40, DM-53 · G6 | `pass_a_is_identical_across_module_order_and_location` compiles with `corpus: None` (`analysis.rs:116`), so Pass C and usage produce no rows there. ADR-0019's Consequences say "shuffled input gives byte-identical analysis tables … as each slice lands" | A change that makes `usage.rs` order-sensitive would pass every repository test. It already iterates `reads` in unordered SQL order | Keep probe 2 as a test | The probe, as a test |

**Observations** (no finding; recorded so that their silence is not read as clean):
- **O1. Override-open consumers are unqualified.** All 210 seed occurrences on the pilot are
  `candidate` (override-open) consumer arcs. The relation selects `consumer_modality`, but
  `pass_c.rs` never reads it, and the producer's modality is not selected at all.
  - On the pilot, each site's only target is the seed, and `FastMCP.mount` has one release
    declaration, so no sentence is false.
  - The Pass A and Pass B templates say "may call" or "overridable" for the same modality.
    §9.3 should either qualify it or state that a usage call is named by the method the code
    calls.
- **O2. Parts of the spine are stale:**
  - §10.2's kinds table (L2157) has no `usage_pattern` or `handoff` row, although the revision
    row says §10.2 changed;
  - the §3.2 `findings` row (L494) still ends "`usage_patterns` with Pass C", against D24;
  - §9.3's Proposed "Output" (L1948) lists example id, region, binding and consumer argument,
    which the finding does not carry. The Implemented half is right.
  - There is no oracle beyond prose.
- **O3. ADR-0019's amendment placement.**
  - ADR-0019 was accepted at `083507f`. The slice-2.2 entry was appended at `cf0990f`, under
    the heading "(in place; the record is still proposed)". According to `e42ff4e`'s message,
    the lint failed at that commit.
  - `e42ff4e`'s trailing sentence, "Those above were made while the record was proposed", is
    false for that entry.
  - `adr lint` compares against HEAD only (`git_head_text`), so once an edit is committed it
    becomes the baseline.
  - The entry also records D24's reversal of the ADR body's `usage_patterns` sentence, a
    decision rather than a factual correction. `adr.py`'s own comment reserves trailing
    amendments for factual corrections.
  - Proportionate fix: one dated trailing line saying the 2.2 entry was appended after
    acceptance, at `cf0990f`.
- **O4. Size alone picks the pattern.** The pilot's `FastMCP.tool` pattern is a tool that
  "Always raises an error". It is verbatim and valid, but it is what gets embedded.
- **O5.** The `with`-item exclusion applies only to the named form. A nested
  `consumer(producer())` inside a `with` item, lambda, comprehension or conditional expression
  is counted. The value flows directly either way; DESIGN L1961 should say "named".
- **O6.** `max_occurrences` reuses `[pass_a] max_witnesses` and records it in the invocation's
  parameters, as D17 does for Pass B. This is not logged as a deviation, and it is harmless.

**Applicability.**
- **Group 2 (validity, absence) and Group 5 (derivation)** carried F1, F2 and F4.
- **Group 10 (lineage)** carried F3 and F5.
- **Group 12 (claims)** carried the G7 halves of F1–F3.
- **Groups 3 and 8 (identity, determinism)** carried F6. Identity is sound by reading and by
  probe.
- **Group 11:** the codebook appends are verified append-only (`pass_c_handoffs` = 2, finding
  kind 9, member roles 5–6, assertion kinds 9–10), and the snapshots were updated.
- **Groups 6, 7 and 9:** no new effects, workspaces, providers or performance claims. The pilot
  time was not re-measured.
- **Over-construction:** none found. D24 avoided a table that would have duplicated the
  assertion machinery (§10).

## 8. Alternatives (brief, at compact depth)

- **Chosen:**
  - a declared SQL relation with a 230-line grouping kernel, for handoffs;
  - a statement closure with a Ruff re-parse, for patterns.

  Both are proportionate. Two strengths show by what would break without them:
  - the shared `maps_formal`: without it, Pass C would map positionals after `*` (2.1 review O7);
  - the score as lineage: without it, every new test occurrence would rename a finding.
- **Simpler alternative for doc blocks: publish the whole block, untrimmed, when the block has
  no unresolved read.** Doc blocks are already curated units, and on the pilot they supply 2 of
  the 5 patterns.
  - It removes F1(a) and F1(c) for doc blocks with no trimming logic, and it makes C5 O2's gap
    a refusal.
  - Examples and tests still need the closure, with F1's checks.
  - Block lengths were not measured, so whether this is better overall is unresolved.
- **For F2, reuse rather than build:** slice 2.3's public-callable relation is the endpoint set
  that §9.3's question names.

## 9. Verification: label audit and top gaps

| Claim (DESIGN at `30561df`) | Label claimed | Label supported | Evidence or gap |
|---|---|---|---|
| §9.3: named after a receiver use and nested are counted; two consumers and reassignment are not | Tested | **Tested** for those four shapes | `handoffs_and_usage_patterns_come_from_official_code` (passed). Chained aliases are counted (F4a, probe A) |
| §9.3: over official examples, tests and doc blocks, between release callables (code doc) | Tested | **Violated** for the endpoint domain | F2 (pilot, probe A) |
| §9.3 pilot: 156 FastMCP → `mount(server=…)`, 24 `require_scopes` → `tool(auth=…)` | Measured | **Measured** | re-queried on `c9309c78` |
| §10.5: an unbound read makes a candidate "not self-contained … refused" | Tested | **Implemented for resolved bindings only** | F1b (probe C) |
| §10.5: trimming "removes no precondition" | Proposed (spine) | **Violated** | F1a (pilot), F1c (probe B) |
| §10.5: "citing … the handoff it shows" | Tested | **Tested for consumer seeds; false for producer seeds** | F3; the test pins the false case |
| §10.5: the code parses (Ruff), and the smoke parses it again | Tested | **Tested** (Rust); the smoke is not_run here | `usage.rs:495`; `smoke.py` read |
| §10.4: "refers only to existing public APIs" | Proposed | **Not implemented** for patterns | F1d |
| Determinism of Pass C and usage (ADR-0019 Consequences) | Tested | **Tested by review probe only** | F6 |

**Top gaps**, where new oracles should land: F1's three shapes plus the smoke's free-name
check; F2's local-helper line; F3's flipped assertion and rule; F6's probe.

## 10. Exceptions and deviations assessed

- **D24 (usage patterns as assertions, no table): sound.**
  - Kind, section, derived status, verbatim evidence and content identity carry over unchanged.
  - The cost is DM-10's: the served code is recovered from the text by string splitting (the
    smoke's `split("```python\n")`). That is acceptable while the evidence rows hold the
    statements.
  - The ADR-side recording is O3.
- **D25 (receiver reads are setup): sound in intent, wider in code.**
  - Its rationale cites §10.5's setup, but the pattern builder never includes receiver-setup
    statements. The mount pattern omits the example's `@news_app.tool` definitions. This is
    valid minimal usage and not a defect.
  - The code's exclusion is every attribute access (F4b).

## 11. Decision and implementation changes

**Decision: Revise.**
- The handoff relation is sound for what it counts (§2–4), identity and determinism hold under
  probe, and no §B decision is reopened.
- G2, G3 and G7 fail on published text:
  - a pilot pattern lacks an import it needs, and the acceptance check cannot see unbound or
    enclosing-context problems (F1);
  - a doc-local helper is published as an API connection (F2);
  - a producer's pattern cites a handoff it does not show (F3).

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1** | F1: refuse unresolved reads; refuse (or carry) enclosing constructs; free-name check over the assembled AST, annotations included; implement or narrow §10.4's public-API clause; fix §10.5's text to match | DM-07, DM-24, DM-59 | The custom_route pattern imports `Request`, or another candidate wins. Probes B and C publish no pattern | Two `docs_shapes` cases (probe B; C5 O2's two-block document); the smoke's free-name check |
| **P1** | F2: handoff endpoints are release (or public) callables | DM-09, DM-59 | `require_tenant`, `require_access_level` and `two_question_server` findings are gone on the pilot. The 156 and 24 counts are unchanged | `local_factory` line in `usage/handoff.py` |
| P2 | F3: a producer pattern reaches its consumer statement, or cites no handoff | DM-46, DM-59 | `pkg.make_server`'s pattern holds `primary.tool(helper)`, or cites 0 findings | Flipped assertion; the shows-its-handoff rule with an injected case |
| P3 | F4: one-target producers; narrow D25's exclusion or restate it | DM-24, DM-59 | Probe A's chain is not counted | Fixture lines |
| P3 | F5: doc-block locations rendered as document plus block | DM-46 | No `_lctx_blocks` in any assertion text | `semantic:` rule with an injected case |
| P3 | F6: keep probe 2 as a test | DM-40, DM-53 | — | The test |
| P3 | O2 (spine rows), O3 (one dated trailing line in ADR-0019), O5 (L1961 "named") | DM-59 | `adr lint` passes | prose |

### Deferred

| Item | Status | Why not now | Reopen when |
|---|---|---|---|
| C3 O4: implicit unbinding; `del` and annotation-only events as read candidates | **Fired** (Pass C now runs over examples); **harmless here** | `single` counts every binding event, so a name with a `del`, annotation-only, `global` or handler event is never a producer binding. `usage.rs` keeps such statements verbatim or refuses the candidate | A Pass C shape or a pattern reads a handler name after its handler, or FCA or a CFG reads handler bindings |
| C4 O2: callable identity, `super()` and `KwCall` in types | Not fired | Pass C reads call edges, not types. §9.3's type-compatibility `candidate` is not built | Pass C builds type-compatibility candidates, or FCA reads callable types |
| C5 O2: continuation doc blocks compiled alone | **Fired** (§10.5 reads doc blocks: 2 of 5 pilot patterns) | The store keeps the gap explicit, but `usage.rs` reads the 753 unresolved reads as builtins | **Closed into F1.** Its named oracle (the two-block document) is F1's test |
| Increment-1 deep review O4 / 2.1 Deferred: a byte-level evidence rule | **Fired again** (new `example` spans) | `example` text is sliced from `source_files.text` at the same offsets, so it is verbatim by construction. The rule checks length only, and the fixture byte test has no corpus | As the 2.1 row says; add `example` evidence to the Pass C test's byte check when F1 lands |
| O1: override-open consumers unqualified | New | No false sentence on the pilot | A release subclass overrides a seed method, or a usage call's static receiver is a base class |
| O4: size-only pattern selection | New | Valid and verbatim | Increment 3's manual review pass (§10.4) or the 3.3 evaluation flags an unrepresentative pattern |
| `MAX_CANDIDATES` = 12 truncates silently | New | Every pilot seed has a pattern | A seed's Usage slot is absent while it has candidate sites |
| ADR-0019 review O6: in-memory composition of Stage E methods | Not fired | Pass C reads no other method's findings. Stage F reads Pass C's findings only to prefer sites | Unchanged |

**Final check.**
- The claims match the evidence except §10.5's self-containment, trimming and "shows" sentences,
  and the "public" endpoint wording (F1–F3).
- The supported scope is right for the handoff shapes the relation counts. It is too wide for
  endpoints and for pattern acceptance until F1 and F2 land.
- The extension path (increment 3's seeds) is where F1(c) and F3 will fire: test-only seeds and
  factory seeds.

## Disposition (2026-09-23, commit after slice 2.4)

| Item | Disposition | Where |
|---|---|---|
| F1 | Fixed (D30). Unresolved reads refuse a candidate. Only a module or function body qualifies, so no header is dropped. A free-name check runs over the assembled code, annotations included, with builtins from `ruff_python_stdlib` at the analyzed minor version. §10.4's public-API clause is narrowed. Pilot: `custom_route`'s pattern now imports what it reads | `usage.rs` (`build`, `free_names`); `free_names_include_annotations_and_skip_builtins`; `usage/refused.py` (`pkg.Server.stop` gets no pattern); DESIGN §10.4, §10.5 |
| F2 | Fixed. Both handoff endpoints are release declarations. Pilot: the three usage-helper findings are gone; `mount` 153 and `tool` 24 | `handoffs_sql`; `local_factory` in `usage/handoff.py` |
| F3 | Fixed (D30). A pattern cites a handoff only when a whole occurrence of it lies inside the pattern. Within a role, a pattern that shows one wins. The flipped test pins `make_server`'s pattern as the nested one, and the rule `semantic:usage-pattern-shows-its-handoff` has an injected case | `synth.rs`, `usage.rs`; `rules.rs`; `syntax.rs` |
| F4 | Fixed. A producer statement has one target. The receiver exclusion is narrowed to a called attribute or a decorator, which is D25's text | `handoffs_sql`; `a = b = …` and `print(named.run)` in `usage/handoff.py` |
| F5 | Fixed. `flows::usage_files_sql` names a doc block by its document and fence number, for Pass C and the patterns alike. The rule `semantic:no-materialized-block-path` has an injected case | `flows.rs`, `usage.rs`; `rules.rs` |
| F6 | Fixed. `pass_c_and_usage_patterns_are_identical_across_location_and_module_order` compiles `docs_shapes` with its corpus twice | `syntax.rs` |
| O2 | Fixed: §10.2 rows, §3.2's findings row, §9.3's Output | DESIGN |
| O3 | Fixed: a dated trailing line in ADR-0019 records the placement and lists the later appended values | ADR-0019 |
| O5 | Fixed in §9.3 ("the nested form is counted wherever it occurs") | DESIGN |
| O1, O4, O6, `MAX_CANDIDATES` | Deferred with this review's triggers (O6 is logged as D17's reuse) | — |
| Deferred rows | As this review's table says. C5 O2 is closed into F1. The byte-level evidence rule stays deferred: `example` spans are sliced from `source_files.text` by construction | — |

Pilot after the fixes (Measured, 2026-09-23, `lctx compile fastmcp --store build/store-next` and
`lctx_mcp.smoke`): snapshot `8a882a72`, generation `0db5bc736fdb1115`, stdio smoke passed, 33.0 s,
peak 4,048 MiB.
