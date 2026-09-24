# Design review: re-review of the Stage 0 ADR set's dispositions (compact)

**Date:** 2026-09-24 · **Depth:** compact. This is a re-review of the dispositions the author made
to `design_review_behavioral-model-adrs_2026-09-24.md` (decision **Revise**, F1–F14; its §12). ·
**Mode:** document review of the fixes, with code and pilot data read where a disposition cites
them or depends on them.
**Target:**
- commit `d9efcbf`: ADR-0010, 0011, 0020, 0021 and 0022; DESIGN.md; `eval/gold/analytics-freeze.json`;
  `scripts/check_gold.py`; AGENTS.md; the plan's §15;
- `2fb10b8` (the pre-registered `eval/behavior/fastmcp-4.0.5.toml`, for F13);
- the Stage 1 code the dispositions cite. It was uncommitted when this review began and was
  committed during it as `34219bc` (06:17:15). Code citations below are at `34219bc`.

**Reviewer:** `design-reviewer` subagent (fresh context). · **Author:** the session executing the
behavioral-model plan.

## Answers to the three questions

1. **Does each "Fixed" disposition close its finding without a new contradiction?**
   - **Closed:** F3 (a)–(d), F4, F12 and F13.
   - **Closed with a residue:**
     - **F1:** closed in substance, but the spine now states the scan's reach two ways (R3), and
       its oracle does not pin reach (R5).
     - **F2:** no negative fate *kind* remains, but a Stage 1 rule admits a negative *verdict*
       (R2).
     - **F10:** it introduced a contradiction in §B14 (R4).
     - **F14:** the review merge exists only in the plan (O2).
     - **F3 (e):** three pointer lines remain (O3).
   - **Not closed: F8's `complete`.** It is defined by listing the kinds of `unknown` row, and
     the list leaves out what actually bounds Stage 1:
     - the depth cut, which the code does not record at all;
     - unresolved call sites;
     - the handoff facets;
     - `raises`, which is inferred but classed as declared.

     So `complete = true` would be asserted over rows that are missing (R1, measured on the
     pilot).
2. **Are the "Open until Stage 2" and "Deferred" dispositions acceptable?**
   - **F5, F9 and F11:** yes, with their trigger (Stage 2's start, after the provider spike, with
     a `standard` review).
   - **F6:** yes for its Stage 2–4 content. Not for the two parts Stage 1 already exercises: the
     budget cut, since Stage 1's scan is depth-bounded (R1(a)), and the refutation premise, since
     Stage 1 ships the rule (R2).
   - **F7 (deferred to Stage 2.6):** acceptable. The brief's Limits already state the subsystem
     stop. But the interim §9 sentence the disposition cites does not exist (R3).
3. **Can ADR-0021 and the amendments be accepted, with ADR-0022 staying proposed?**
   - Yes, scoped:
     - ADR-0021;
     - the ADR-0002 and ADR-0005 amendments;
     - the ADR-0011 and ADR-0020 amendments, with R6's correction;
     - the ADR-0010 amendments, **except** the `find_operations` `complete` clause (R1) and the
       view-identity sentences (R4).
   - ADR-0022 stays `proposed`. Its Stage 2 review inherits Stage 1's published verdict codes.
   - **Decision: Accept scoped** (§11).

---

## 1. Decision and scope

The decision under review is whether the Revise findings are closed well enough to accept the
set. In-scope behavior is Stage 1: the `behavior` analysis tables,
`get_operation`/`find_operations`/`search_operations`, `FORMAT` 3 and the views. Stages 2–5 stay
**Proposed** targets. Every new DESIGN claim in the fixes is labelled Proposed, or Measured with
a date, and correctly so.

### Method and coverage

**Read in full:**
- the `d9efcbf` diff (every file);
- ADR-0021 and ADR-0022;
- the amendment tails of ADR-0002, 0005, 0010, 0011 and 0020, and the headers of 0001 and 0004;
- the prior review;
- DESIGN §1.1–§1.5, §B4, §B5, §B10, §B13 and §B14;
- DESIGN §3.2's behavior rows, §3.9, §9's opening, §9.2 (the Worklist and trace bullets), §9.7,
  §9.8's head, §9.9, §11.1, §11.3, §12's head and structured evaluation, and §13;
- the deviation log B1–B9, and the question set's header and Q01.

**Code read myself (at `34219bc`):**
- `crates/cpg-core/src/behavior.rs`, all of it;
- `crates/cpg-schema/src/behavior.rs`;
- the codebook and rules diffs;
- `crates/lctx-analytics/src/pass_b.rs:280–625` (the worklist, depth cut and completion);
- `crates/cpg-schema/src/concepts.rs:25–85` (`attributes_sql`);
- `cpg-core/src/embed.rs::embed_texts` and `lctx_analytics::neighbours::windows`;
- the F1 test and its fixture `fixtures/python/analysis_shapes/release/pkg/registry.py`;
- `scripts/check_gold.py`'s new block.

**Measured myself** (2026-09-24; `target/release/lctx query --store build/store --snapshot
0ec3355a10ac736b184f0365bef1ff68`, the first Stage 1 snapshot, read-only):
- **`operations`:** 1,534 rows. All 1,171 callables are `established`, all 363 classes are
  `not_analyzed`, and none is `unknown`.
- **`behaviors`:** 4,404 rows (2,711 `established`, 507 `conditional`, 1,186 `unknown`). None is
  `refuted_under_model` and none is `not_analyzed`.
- **The depth cut** (`pass_a.max_depth = 2`; `analytics.toml`):
  - 57 operations hold depth-2 forward states with further parameter or alias flows, 80 states in
    all.
  - Leaving out next states the same operation already reaches, **48 operations lose 85 forwards**
    at depth 3.
  - The most cut next states: `fastmcp.FastMCP.read_resource`, `fastmcp.Context.get_prompt` and
    `fastmcp.cli.client.call_command`, 4 each.
- **Unresolved calls.** Call sites directly in public callables (`resolutions`): 208 `unresolved`
  in **100 operations**, and 32 `partial` in 28.
- **Direct raises.** Public callables hold 492 direct `raise` statements. At most 453 have a
  class-typed raised observation. **38 bare re-raises and one union** (`BaseException |
  ToolError` in `fastmcp.server.providers.AggregateProvider.get_tool_by_hash`) have none.
- **F1's reach.** **305 operations outside the 16 prefixes hold 1,036 forwards (depth ≥ 1) into
  callees outside them.** This uses the prior review's test: a qualified-name prefix match.
- **F7.** For **14 of the 20 seeds**, 62 of their 164 Pass B rows (forwards, supplies_literal,
  raises_when, unfollowed) name a callee outside the prefixes.
- **F13's commit order:**
  - `2fb10b8` was committed at 06:04:17.
  - Snapshot `0ec3355a` was appended at 06:05:35 (the `snapshots` Delta log, version 6), and it
    is the first to hold the 9 Stage 1 tables (84 tables).
  - The previous snapshot, `39cc8ce5` (05:43:43, version 5), holds 75 tables and no Stage 1
    table.

**Checks run:**

| Command | Outcome |
|---|---|
| `cargo nextest run -p cpg-core behaviors_cover_public_callables_outside_the_subsystem` | **passed** (1 test, on the Stage 1 tree committed as `34219bc`) |
| `just adr lint` | **passed** (`ok (22 records)`) |
| `just gold` | **passed** |
| `just lint-agents` | **passed** |
| `check_gold.main()` with an in-memory freeze naming ADR-0004 (`PYTHONPATH=scripts uv run python -`, `json.loads` patched; no file changed) | exit 1: "the freeze names ADR-0004, whose status is superseded". With ADR-0021: exit 0. **F3(d)'s oracle works** |
| The rule `semantic:refuted-needs-complete-region`'s SQL, over `behaviors` plus one injected `refuted_under_model` row on an operation that holds an `unfollowed` (`unknown`) row (`lctx query`, pilot) | 0 violations: **the rule accepts it** (R2) |
| `just check`, `just test-all`, `just pilot` | **not_run**. This is a document re-review. The tree holds another session's uncommitted Stage 1 edits (`site_*` columns on `behaviors`). `34219bc` reports `just test-all` passed, which I did not reproduce |

**Not reviewed:**
- The Python serving code for the behavioral tools. Uncommitted work
  (`python/lctx_mcp/src/lctx_mcp/operations.py` and its tests) appeared in the tree during this
  review and was not read. R1's `complete` semantics should be settled before it lands.
- The question set's targets beyond Q01.
- The plan outside §15.
- `eval/heldout/`, which is sealed.

**Guarantees not attacked, so asserted:**
- that the F4 criterion is operable per stage;
- the Stage 2 algorithms (dominators, SCC summaries);
- `FORMAT` 3;
- the timing figures in `34219bc`;
- that view windows stay under 2,048 tokens (O4).

---

## 2–4. Authority, contracts and derivation (compressed)

**Changes to the authority map since the prior review:**
- **Freeze:** it answers to ADR-0021, and `check_gold.py` enforces an active record (Tested).
- **Universe and scan:**
  - ADR-0021 and §1.4 give the universe and the scan's reach. The code does the same:
    `pass_b::run(…, |_| true, …)` at `behavior.rs:194`.
  - §9.2:2160 still gives the kernel's reach as "subsystem callees only", and §9's opening does
    not say otherwise (R3).
- **Keep rule:** ADR-0021 and §9.8 give the criterion and the exit.
- **Views:** the cache key is stated two ways in §B14. The code keys on the text alone (R4).

**Contracts:**
- **The verdict codebook** is committed with all five codes (`codebook.rs`, `Verdict`). Its
  `not_analyzed` text still reads "cut by a budget".
- **`behavior_status`:** the only producers are `established` (every scanned callable) and
  `not_analyzed` (classes). The `unknown` branch (`behavior.rs:464–465`) is unreachable, because
  `pass_b::run` always returns `CompleteUnderStatedModel` (`pass_b.rs:624`). A depth cut sets only
  `stop_reason` (`:375–376`, `:625`), and the surface scan discards it: its invocation writes
  `stop_reason: None` (`behavior.rs:330`). The seed passes keep it (`analyze.rs:1265`).
- **§9.2:2134–2137's rule** "What is declined leaves a trace … 'not listed as passed on' is never
  read as 'not passed on'" therefore holds for briefs and not for `behaviors` (R1(a)).

**Derivation.** No change to the publication protocol. The nine Stage 1 tables are analysis
tables written before Stage E (deviation B6).

---

## 5. Journey (the one chosen: an exhaustive query at the edge of a budget)

An agent calls `find_operations(where=[{facet: forwards_to, value: X}])`, where some operation
reaches `X` only at depth 3.
- **§11.3.** `complete = true` unless some operation has an unfollowed read for `forwards_to`.
- **The depth-3 forward** produces no row. The cut state's own reads are skipped too
  (`pass_b.rs:480`), and the operation is `established`.
- **The result:** the match is missing and `complete = true`. On the pilot this can happen for
  48 operations (Measured).

The same shape recurs:
- for `delegates_to` over the 100 operations with unresolved sites;
- for `raises` over the 38 bare re-raises;
- for `hands_off_to` and `takes_from`, which have no `unknown` kind at all.

This is the false-exhaustiveness case that F8 named, now reached through a different door.

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **fail** (documentary, narrow) | **Closed:** F3 (a)–(d). §1.4, §12(b), §9.8, §B4, §B10 and §11.2 are revised; §1.5's live rules are out of "history"; the freeze file names ADR-0021, and `check_gold` refuses a superseded record (probe above). **New:** §1.4:173 says the scan follows parameters "into any release function (§9's opening)". §9's opening (L2043–2045) does not say so, and §9.2:2160 says "into subsystem callees only" (R3). §B14:487–488 puts the view "inside the input's identity" while :489–491 has two views of one text "share a vector"; ADR-0010:240–242 still says "distinct keys" (R4). **Residue:** the pointers in ADR-0001:74–75, `docs/pins.md:74` and ADDENDUM Q17 (O3) | R3 and R4: sentences in the accepting commit |
| G2 Semantic fidelity | **fail** (Stage 1) · unresolved (Stages 2–4, deferred) | **Closed:** F2's fate vocabulary. `behavior_kind` has no negative kind, and the codebook snapshot holds it. **Fails:** the surface scan publishes a depth-cut operation as `established` with no row for the cut (48 operations and 85 forwards on the pilot; `pass_b.rs:624`, `behavior.rs:202,330`) (R1(a)). The Stage 1 refutation rule accepts `refuted_under_model` on any scanned operation, including one with an `unknown` row (R2). **Stages 2–4:** F5, F6 (the Stage 2 part) and F11 are open with a trigger | R1(a) and R2 before Stage 1's exit |
| G3 Validity | **pass** (design level) | §11.3: `where` is a conjunction of named terms with no negation; unknown facets and values are invalid params that name the valid ones; `limit` is 1–50; a cursor from another generation or request is invalid params. Not yet implemented | Stage 1.6 Client tests (planned) |
| G4 Hidden behavior | **pass** (disposition scope) · unresolved (F9, deferred) | F13: the set's commit precedes the first Stage 1 snapshot (Measured above), and B3 records what the question-set subagent saw. The freeze has a live authority. The surface invocation records scope, `max_depth`, roots and window bytes, with the scan digest. Its discarded `stop_reason` is counted under G2 (R1(a)) | — |
| G5 Consistency and recovery | **pass** (design level) | The cursor binds the generation key, the request's canonical hash and an offset. Every result names its `snapshot_id` (§11.3) | Stage 1.6 test: a stale cursor is refused |
| G6 Transformation and reuse | **pass** (code) · unresolved (F9, deferred) | One spec per snapshot. The view is a column (`operation_documents.embedding_view`). The key is the text's hash (`behavior.rs:593`), so one text has one vector per spec, which is valid reuse (deviation B8). The query instruction is kept knowingly, with a trigger. The contradictory wording is counted under G1 (R4) | — |
| G7 Truthful capability claims | **fail** (Stage 1 `find_operations`) | **Closed:** F1's scope claim is now true in the data (1,534 operations; 1,036 out-of-prefix forwards). **Fails:** §11.3 promises `complete = true` for a declared-facet result and for a behavioral one with no listed `unknown` row. Rows are missing for depth cuts (48 operations), unresolved sites (100 operations) and bare or union raises (39). The handoff facets have no `unknown` kind (R1). §9.8 names a consumer for kNN that does not read it (R6) | R1 before `find_operations` lands (plan 1.6); R6 as a sentence |

A failed gate is not averaged away. G2 and G7 fail on one cause (R1), and it sits in one ADR-0010
clause and in one kernel path. The decision (§11) takes both out of the accepted scope rather than
holding back the rest of the set.

---

## 7. Findings

### 7.1 The dispositions, checked

| Finding | Author's disposition | Verdict | Evidence |
|---|---|---|---|
| F1 | Fixed | **Closed in substance.** The residue is R3 (spine) and R5 (oracle) | The ADR-0021 Decision and §1.4:169–173; `behavior.rs:194` `\|_\| true`; test **passed**; pilot 305 operations and 1,036 forwards (Measured) |
| F2 | Fixed; the reference-resolutions rule deferred to Stage 2 | **Closed for fates.** The deferral is acceptable, and R2 is a hole | §3.2's behavior row. `behavior_kind` = forwards, supplies_literal, raises_when, unfollowed, delegates, hands_off_to, takes_from, with no negative kind. On the pilot, no `refuted_under_model` or `not_analyzed` rows |
| F3 | Fixed | **Closed (a)–(d); (e) partial** (O3) | The diff; the `check_gold` probe; `just gold` **passed** |
| F4 | Fixed | **Closed.** R6 residue | The ADR-0021 Decision (criterion and exit); §9.8:2640–2650; the ADR-0011 correction |
| F5, F6, F9, F11 | Open until Stage 2 | **Acceptable, except F6's Stage-1-live parts** (R1(a), R2) | ADR-0022:86–96 and §3.9 list them |
| F7 | Deferred to Stage 2.6 | **Acceptable.** The cited interim §9 sentence is missing (R3) | 14 of 20 seeds diverge (Measured); the brief's Limits state "release code outside the subsystem" (§10.3:2919–2920) |
| F8 | Fixed | **Partly.** `where`, no negation, the cap, the cursor and `explain` are closed; **`complete` is not** (R1) | §11.3 Semantics |
| F10 | Fixed | **Closed** for the spec, the query instruction and windowing. **New contradiction** (R4) | §11.1:3038–3048; §B14 |
| F12 | Fixed | **Closed** (O5 trivial) | ADR-0022:62–64; §B13; the ADR-0010 correction |
| F13 | Already resolved | **Verified** | Commit and Delta-log timestamps (above); deviation B3 |
| F14 | Fixed | **Closed.** The review merge is not in an authority (O2) | ADR-0022:52–53; §3.9; §9.9; plan §15 |

### 7.2 New findings

The order follows the skill's severity ranking: correctness and authority on Stage 1 first, then
documentary authority, then oracles and proportionality.

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **R1** | **`complete` is defined by listing the kinds of `unknown` row, and the list leaves out what bounds Stage 1. The depth cut is not recorded at all** | DM-08, DM-42, DM-43 · G2, G7 | (a) **Depth cut:** `pass_b::run` returns `CompleteUnderStatedModel` always (`pass_b.rs:624`) and marks a cut only in `stop_reason` (`:375–376`, `:625`). The surface scan tests only `completion` (`behavior.rs:202`) and writes `stop_reason: None` (`:330`), so every callable is `established`. §3.9's table and the codebook say a budget cut is `not_analyzed`, and ADR-0022 lists the question as open. Stage 1 publishes it as `established`, neither answer. (b) **`delegates_to`:** §11.3 names "a candidate (override-open) arc" as its only `unknown` kind. An `unresolved` site has no target, so it has no `delegations` row. (c) **`hands_off_to` and `takes_from`:** no `unknown` kind is listed, yet handoffs come only from official usage, via Pass C's direct-handoff recognizer. (d) **`raises`:** it is filed under facets "stated by the release's text" and "complete under the stated model". It is actually Pyrefly's inferred class of each raised expression (`concepts.rs:64–72`, with no `declared` filter, unlike `parameter type` and `returns`). Deviation B9 says "a *typed* `raise`"; §11.3 drops that qualifier | **Measured, on the pilot:** 48 operations lose 85 depth-3 forwards; 100 operations have 208 unresolved sites; 39 of 492 direct raises give no value. `find_operations(forwards_to = X)` and `(raises = ToolError)` can return `complete = true` while an operation that forwards to `X` at depth 3, or re-raises a caught `ToolError`, is absent. `get_operation` shows such an operation as `established`. This is F8's false exhaustiveness again, and it breaks §9.2:2134–2137 for the behavior tables | (a) Record the cut. Append `depth_limit` to `unfollowed_reason` and emit an `unfollowed` row (`unknown`) at each cut state, or set `behavior_status = unknown` with the invocation's `stop_reason`. This answers F6(a) for Stage 1: a stop during analysis is `unknown`. (b) `unresolved` and `partial` sites become `delegates` rows with verdict `unknown` and their reason. (c) Handoff facets are never `complete`; they are relative to official usage. (d) Move `raises` to the behavioral class, with an `unknown` row for a bare or untyped raise. Simpler still, see §8 | **None exists.** Test: an `analysis_shapes` chain of depth 3 from a public callable yields the `unknown` row (or `behavior_status = unknown`). Stage 1.6 Client tests: `complete = false` with a depth cut, with an unresolved site, and with a bare raise; handoff facets are never `complete = true` |
| **R2** | **The Stage 1 refutation rule admits the negative claim §3.2 excludes** | DM-07, DM-08 · G2 | `rules.rs:653–661`: it rejects `refuted_under_model` only `WHERE … o.behavior_status <> {established}`, and every scanned callable is `established` (all 1,171 on the pilot). Deviation B7 makes "the operation's own scan" the Stage 1 premise. Yet §3.2 says Stage 1 "sees parameter reads only at call arguments" and emits "no negative fate". **Checked:** an injected refutation on an operation that holds an `unfollowed` (`unknown`) row gives 0 violations | Nothing in Stage 1 emits a refutation today, so F2's closure rests on the absence of a code path rather than on a rule. A Stage 1.x change that writes "parameter never forwarded" as `refuted_under_model` would validate and publish: the "accepted but unused" error F2 closed | Until Stage 2 defines regions, the rule rejects **every** `refuted_under_model` row. Move the injected case in `the_analysis_rules_reject_their_violations` onto an `established` operation | Rule plus injected case (the test exists; its case changes) |
| **R3** | **F1's reach is stated in §1.4, contradicted in §9.2, and absent from the §9 place both it and F7's disposition cite** | DM-02, DM-04 · G1 | §1.4:173 says "Its scan follows parameters into any release function (§9's opening)". §9's opening, "Per public callable" (L2043–2045), says nothing on reach. §9.2:2159–2160 says "bounded by Pass A's depth. It follows parameters and aliases into subsystem callees only". F7's disposition says "§9 says the two differ in mask", and no such sentence exists (searched) | A reader of §9.2 concludes that `behaviors` stop at the subsystem, which is F1's ambiguity again. `get_operation` for 14 of 20 seeds shows forwards and raises that the brief omits (62 rows, Measured), with no design sentence saying why. The brief's Limits disclose the stop, which is why this is documentary | One sentence in §9's "Per public callable" paragraph: the surface scan runs Pass B unmasked, into any release function; the seed passes keep the mask, so a seed's brief and its behaviors differ in reach, and the brief's Limits say so (F7, deferred to 2.6). Add a "for seeds" qualifier to §9.2's Worklist bullet | Prose. The behavioral half is R5's test |
| **R4** | **§B14 states two cache keys for views, and ADR-0010's amendment still states the rejected one** | DM-02, DM-32 · G1, G6 | §B14:487–488 says "A view … is a template id and version inside the input's identity". §B14:489–491 says "Two views with the same text share a vector". ADR-0010:240–242 says "inside the input's identity … two views of one operation have distinct keys", and the F10 correction appended in `d9efcbf` does not retract it. The code: `input_hash(&spec.document_text(&d.text))` (`behavior.rs:593`), text only; deviation B8 records the choice | Two implementers diverge on a cache key. Following L487 folds the view into `input_hash`, which re-keys every view vector and embeds identical texts twice | Delete "inside the input's identity" from §B14. Add one ADR-0010 amendment line retracting "distinct keys" in favour of B8 | **None exists.** Test: two views with identical text yield one `input_hash` and one cache row |
| **R5** | **F1's oracle pins coverage, not reach** | DM-60 | `behaviors_cover_public_callables_outside_the_subsystem` asserts depth-0 `raises_when` rows for `pkg.Catalog.remove` and `.load`, which raise in their own bodies (`registry.py:24–33`). Pass B seeds its states whatever `inside` says (`pass_b.rs:340`) and reads guards on every visited state. `inside` gates only flow targets (`:370`, `:490`) | Restoring the subsystem mask at `behavior.rs:194` leaves the test green. The pilot's 1,036 out-of-prefix forwards have no guard | Add a fixture operation outside the prefixes that forwards into an out-of-prefix helper at depth 1, and assert its `forwards` row | Test (extend the existing one) |
| **R6** | **kNN's named consumer does not read kNN's output, and the variant list is incomplete** | DM-58, DM-59 | §9.8:2648–2649 and ADR-0011:218 name "kNN vectors (`search_operations`' ranking, Stage 1, as views)". The `+knn` variant produces `doc_link` findings, community labels and the kNN layer (§9.7:2579–2582, 2594–2604). `search_operations` ranks query-to-view cosine over `operation_documents`, which the variant neither writes nor reads. ADR-0020:112 still says "Communities, FCA and kNN get new consumers". PageRank, RCA and the type and mention layers are not listed | At Stage 1's evaluation, the `+knn` and default arms give the same `search_operations` answers, so the criterion's first clause cannot be met. Or the arm is read as views on or off, judging §B14 infrastructure as a variant. Two operators diverge, and the exit's "no tool consumer" clause is dodged by a misnamed consumer | List every variant (communities, FCA, kNN, PageRank, RCA, the three layers) with its real consumer (a tool, a brief, or none) and the stage its judgement falls in. Views are not a variant. Add one ADR-0020 line pointing to ADR-0011's correction | None mechanical (prose), as for F4 |

### Observations (not findings)

- **O1.** The Stage 1 code landed as `34219bc` while ADR-0021 was `proposed`, during this review.
  ADR-0001 permits it. It means this decision is read against code that has already landed.
- **O2.** F14's review merge is only in the plan (§15). DESIGN §1.2:66–67 and ADR-0021:81 still
  say every stage ends with a `compact` review. One sentence in §1.2 is enough.
- **O3.** F3(e) pointers remain:
  - ADR-0001:74–75 ("ADR-0004 now records the review cadence");
  - `docs/pins.md:74` ("the pilot per ADR-0004");
  - ADDENDUM Q17 ("a named consumer in the brief").

  The first two resolve through `superseded-by`. Q17 is the question a reviewer actually reads.
- **O4. §11.1.**
  - "An over-long document is chunked at whole parts …" is glued onto the "One query
    instruction" bullet (L3048).
  - "So they stay under the cap without a token count" is unmeasured. View windows go through
    `embed_texts`, which counts no tokens; only the brief path does (`embed.rs:219`). They are
    bounded by vLLM's `--max-model-len 8192`, so an over-cap window is not silently truncated.
    It is a hypothesis until the pilot's largest view window is tokenized.
- **O5.** `specs/serving/conditions.json` is kept "for the day a serve-time consumer exists"
  (§3.9:1124–1126). The Rust test suffices until then.
- **O6. Stage 1 has already answered part of F6 in published rows.**
  - A raise under a Pass B guard is `conditional`, with its test site (`behavior.rs:259`).
  - A non-definite delegation is `unknown`, with its modality in `value` (`:346`).

  Both match the prior review's recommendations. ADR-0022's Stage 2 review inherits them.
- **O7.** §3.9's open list omits F9's registry-identity half, which ADR-0022:93–94 carries.

**Applicability.**
- **Groups that carried findings:**
  - 1 (authority and boundary): R3, R4;
  - 2 (absence and unknowns): R1, R2;
  - 7 (dependencies and reuse): R4;
  - 9 (capabilities and loss): R1;
  - 12 (proportionality and falsifiable claims): R5, R6.
- **Groups that did not bear:**
  - 5 (derivation) and 6 (mutable workspaces): the publication protocol is unchanged;
  - 8 (performance): no claim is judged. The stage timings in `34219bc` are not reviewed;
  - 11 (evolution): the codebooks are append-only, and R1's `depth_limit` is an append.

---

## 8. Alternatives (R1 only)

| Alternative | Effect | Risk | Why |
|---|---|---|---|
| **Complete the list of `unknown` kinds** (R1 (a)–(d)) | `complete` becomes truthful per facet | Each new bound (Stage 2's budgets, new facets) must remember to add its kind | Works, but it is a list to keep in step |
| **Simpler: in Stage 1, `complete = true` only for the truly declared facets.** These are `parameter`, `parameter_type`, `returns`, `decorator`, `async`, `kind` and `module`, all annotation or text only. Behavioral facets and `raises` always return `complete = false` with the reason "bounded analysis", plus the `unknown` list as designed | Removes (b)–(d) as a class. The Stage 1 exit ("controls and handoffs answered for operations with no brief") does not need behavioral completeness | Loses a positive completeness claim until Stage 2's summaries can back one | **Recommended.** R1(a) is still required: `get_operation` must not show a cut operation as `established` |

---

## 9. Verification (top gaps)

| Claim | Label now | Check | Gap |
|---|---|---|---|
| A depth cut leaves an `unknown` trace | Proposed (violated in code) | `analysis_shapes` depth-3 test (R1(a)) | Test |
| `complete` is never true over missing rows | Proposed | Stage 1.6 Client tests (R1) | Tests |
| No refutation in Stage 1 | Proposed | The rule rejects every `refuted_under_model` row, with an injected case (R2) | Rule change |
| The scan reaches outside the subsystem | Measured (pilot); not pinned | A fixture forward at depth 1 outside the prefixes (R5) | Test |
| A parameter with no fate is served as `not_analyzed` for its other channels (§3.2) | Proposed | A `get_operation` Client test: never omitted, never "unused" | Test (Stage 1.6) |
| Identical view texts share one key | Implemented (`behavior.rs:593`) | A unit test (R4) | Test |
| View windows stay under 2,048 tokens | Hypothesis | Tokenize the pilot's largest view windows on the first `--embedder vllm` compile (O4) | Measurement |
| The pre-registered set precedes the judged snapshot | Measured (this review) | The commit-order check in the packet script (the prior F13 oracle) | Recipe, deferred |

---

## 10. Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| F5, F9, F11; F6's Stage 2 content (opaque atoms in the normal form, regions, `dynamic_access`, verdict ↔ modality and `evidence_status`) | Decided with the spike's result | Stage 2's start, after the D-2 spike; `standard` review (ADR-0022) |
| F7: briefs as a second derivation | The brief's Limits disclose the subsystem stop. 14 of 20 seeds differ (Measured) | Stage 2.6 (Stage F renders controls from `behaviors`), provided R3's sentence lands now |
| F2's rule against `reference_resolutions` | No negative fate kind exists in Stage 1, and R2 closes the verdict route | Stage 2's `field_writes` and `value_flows` |
| F13's commit-order recipe | The packet script does not exist yet | The Stage 1 structured-evaluation packet script |
| O4: view-window token bound | Not silent: vLLM bounds at 8,192 tokens | The first `--embedder vllm` compile with views |
| O5: `conditions.json` | No consumer | An ADR adding a serve-time condition filter |
| O7: the registry's identity in §3.9's open list | Stage 4 | Stage 2's `standard` review (with F9) |

---

## 11. Decision and implementation changes

**Decision: Accept scoped.**

| Record | Decision | Condition |
|---|---|---|
| ADR-0021 | **Accept** | R3's sentence in §9, in the accepting commit. O2 and O3 are optional there |
| ADR-0002 amendment (declared families) | **Accept** | The prior review's O3 (§B1 against `ty`) stays deferred to the spike |
| ADR-0005 amendment (a consumer in the served model; not a solver) | **Accept** | — |
| ADR-0010 amendments (`7f0f687`, `d9efcbf`) | **Accept scoped** | **Excluded until R1 is fixed:** the `find_operations` `complete` clause and `raises`' place among declared facets. Settle this before `find_operations` lands (plan 1.6), preferably by §8's simpler route. **Retract** "inside the input's identity" and "distinct keys" (R4) |
| ADR-0011 amendments | **Accept** | The flow and summary algorithms stay **Proposed** until Stage 2. R6's consumer list replaces "vectors per view" as kNN's consumer |
| ADR-0020 amendments | **Stand.** The record stays `proposed` by its own text | One line pointing its "Communities, FCA and kNN get new consumers" to ADR-0011's correction (R6) |
| ADR-0022 | **Stays `proposed`** (acceptable) | Its Stage 2 review takes Stage 1's published verdict codes and O6's choices as given. R1(a) and R2 are Stage 1's answers to F6's parts that Stage 1 already exercises |

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1** | R1(a): record the surface scan's depth cut as an `unknown` row, or as `behavior_status = unknown` | DM-08, DM-42 | On the pilot, no `established` operation has a cut state | `analysis_shapes` depth-3 test |
| **P1** | R1(b)–(d): §11.3's `complete`, by §8's simpler route or by completing the list | DM-43, DM-59 | §11.3 text; an ADR-0010 amendment line | Stage 1.6 Client tests |
| **P1** | R2: no `refuted_under_model` in Stage 1 | DM-07, DM-08 | Rule text | The injected case moved onto an `established` operation |
| **P2** | R3: §9's reach and F7 sentence; the §9.2 qualifier | DM-02, DM-04 | Text | R5's test |
| **P2** | R4: §B14 and ADR-0010 view identity | DM-02, DM-32 | Text | The identical-text key test |
| P3 | R5: a fixture forward at depth 1 outside the prefixes | DM-60 | The test | Itself |
| P3 | R6: the variant consumer list; the ADR-0020 pointer | DM-58, DM-59 | §9.8 text | Prose |

No further review is owed for R3–R6 or the observations. R1 and R2 are checked at Stage 1's
review, which is the increment-3 `deep` review under F14's merge.

**Final check:**
- **Claims against evidence.** They match, except `complete` (R1) and the `established` status of
  cut operations (R1(a)).
- **Scope against guarantees.** The whole-surface claim now holds in the data (Measured). Stage
  1's exhaustive-answer guarantee does not, yet.
- **Extension paths.** Stage 2's decisions are open with a trigger, and Stage 1 has not
  foreclosed them (O6).

---

## Disposition (2026-09-24, by the author)

| Finding | Disposition | Oracle |
|---|---|---|
| R1 | **Fixed.** `complete` is true only for facets read from the declaration and its annotations; `raises` and the behavioral facets are never complete in Stage 1. Pass B's depth cut is now read (`stop_reason`), and call sites resolution leaves open (`behavior:open_sites`) make an operation's `behavior_status` `unknown`, with `status_reason`. The scan's invocation is `partial` when any operation was cut. Pilot `4dcb40b0`: 991 `established`, 180 `unknown`, 363 classes `not_analyzed` | `test_declared_facets_answer_completely_and_raises_never_does`, `test_a_behavioral_facet_lists_what_could_hide_a_match`; the invocation summary in `test_the_tools_round_trip_in_both_protocol_eras` |
| R2 | **Fixed.** `semantic:refuted-needs-complete-region` rejects every `refuted_under_model` row in Stage 1, because no region exists yet | The injected case in `the_analysis_rules_reject_their_violations` |
| R3 | **Fixed.** §9's opening states the scan's reach, the statuses and the F7 mask difference; §9.2 says the seed passes stop at the subsystem, the scan does not | Wording |
| R4 | **Fixed.** A view is a column, not part of the cache key (§B14; ADR-0010 correction line) | By construction: `input_hash` is the request text's hash |
| R5 | **Fixed.** `fixtures/python/analysis_shapes/release/pkg/relay.py` forwards `path` into `pkg.Catalog.load` from outside the prefixes; the F1 test asserts that forward. The analysis ledger did not move | `behaviors_cover_public_callables_outside_the_subsystem` |
| R6 | **Fixed.** kNN has no tool consumer either; FCA over behavioral attributes is the only named consumer (§9.8; ADR-0011 correction line) | Wording |

With these fixes, ADR-0021 is **accepted**, and the ADR-0002, 0005, 0010 and 0011 amendments stand. ADR-0020 stays `proposed` by its own text, and ADR-0022 stays `proposed` until Stage 2's `standard` review.
