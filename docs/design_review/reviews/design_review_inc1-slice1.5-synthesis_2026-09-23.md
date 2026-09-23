# Design review: increment 1 slice 1.5, Stage F synthesis (compact)

**Date:** 2026-09-23 · **Depth:** compact · **Mode:** code plus DESIGN.md, at commit `2d9e1d3`.
- **Why a review is owed.** The slice adds eight analysis tables, an extractive rule, templates
  and eight rules, so ADR-0001 owes a `compact` review.
- **Where it ran.** Built and tested in a `git archive` export of `2d9e1d3` in the session
  scratchpad, with its own `CARGO_TARGET_DIR`. The author worked in the main tree throughout:
  - slice 1.6 landed as `f831ea5` during the review, leaving `synth.rs`, `findings.rs` and
    `pass_a.rs` unchanged from `2d9e1d3`;
  - later, uncommitted edits touch `synth.rs`, `findings.rs`, `rules.rs` and `pass_a.rs`, and
    were not reviewed.

  Every citation below is therefore to the text at `2d9e1d3`.
- **Out of scope.** Pass A and the projection (slice 1.4). The 1.4 review
  (`design_review_inc1-slice1.4-pass-a_2026-09-23.md`, committed in `f831ea5`) owns them.
  Where Stage F only surfaces a Pass A gap, this review points there instead of re-filing it.

**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`2d9e1d3`.
**Prior reviews:** `design_review_adr-0019-analysis-results_2026-09-23.md` and its Disposition
(F7, F8, O4 and O8 were owed by or bear on 1.5). Findings here are F1–F6; observations are O1–O7.

## 1. Decision and scope

**Target.**
- `crates/cpg-core/src/synth.rs`, read in full: the Outcome order, templates, status derivation,
  ids, the brief document.
- `crates/cpg-schema/src/findings.rs`: the eight synthesis tables, `ASSERTION_POLICY`, and the
  `recipe` evidence, assertion and brief ids.
- `crates/cpg-schema/src/rules.rs` L245–337 (at `2d9e1d3`): the eight new semantic rules, and
  `EDIT_GUARDS`.
- `crates/cpg-core/tests/analysis.rs`: the briefs snapshot, the verbatim-evidence check, the
  ledger and the injected cases.
- Also read:
  - the `attempt.rs` Stage F wiring;
  - `analyze.rs` `resolve_seeds`;
  - `pass_a.rs` L150–260 for what the templates read;
  - the committed snapshots `analysis__briefs.snap` and `analysis__brief_documents.snap`.
- Checked against:
  - DESIGN §1.5, §10.1–§10.5 and §11.1;
  - ADR-0005;
  - ADR-0019, with its review and Disposition.

**Observable outcome claimed** (DESIGN §10 L1916–1921, §10.3 L2011–2032; commit message).
- Programmatic assertions and briefs, with statuses derived rather than chosen.
- Verbatim extractive text, with the evidence checked byte for byte.
- The kind policy published and enforced.
- The §10.4 grounding checks, and the §1.5 no-bypass rule.
- DESIGN labels the kinds, the policy, status propagation, the Outcome order and "the grounding
  rules below" as **Implemented** and **Tested**.

**Supported scope.**
- Five increment-1 kinds, plus `related`, which is declared but unused.
- One brief per seed, and no applicable cases.
- A byte proxy for the 2,048-token cap.
- No Pass B, no usage patterns and no manual review. These are later slices.

### Method and coverage

**Checks run in this session** (2026-09-23, on the export of `2d9e1d3`):

| Check | Command | Outcome |
|---|---|---|
| Format | `cargo fmt --all --check` | passed |
| Rust tests | `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass --offline` | passed: 122/122 in 167.0 s. The slowest is `every_rule_kind_rejects_its_violation` |
| Lints | `cargo clippy --workspace --all-targets --quiet --offline -- -D warnings` | passed (run after the probe file was removed) |
| ADR lint | `python3 scripts/adr.py lint` (system Python 3.12.3; the script is stdlib only) | passed: 19 records |
| Rules | `ast-grep scan`; `ast-grep test --skip-snapshot-tests` | passed: 4/4 |
| pytest, pyrefly, ruff, `lint-agents`, `just deps`, `just gold` | — | not_run. The slice adds no Python. In the main tree, `uv run` would sync the author's modified environment |
| `just pilot` | — | not_run. The existing snapshot was queried read-only instead |

**Pilot** (read-only: `target/release/lctx query --store build/store --snapshot fce89120da8ea4e43b7e69a22ad6bec7`).
- **Asserted:** the binary (built 15:27:41) matches `2d9e1d3`. `synth.rs` and `findings.rs`
  equal the commit and predate the binary. `rules.rs` was edited after the commit. The snapshot's
  counts match the commit message: 4 briefs, 63 assertions and 45 evidence rows.

What the queries found:
- **Q1.** 4 briefs (`FastMCP.tool`, `.prompt`, `.resource` and `.custom_route`). All are
  `documentation_only = false` and unreviewed.
- **Q2.** Supports by kind:
  - outcome: 4, each with 1 evidence row;
  - public_access: 4 and coordinates: 5, each citing findings;
  - parameter: 41, each with 1 fact evidence row;
  - analysis_boundary: 9 assertions with 33 finding supports.
  - **2 resolved assertions have no support row:** "Calls more than 2 steps below
    `fastmcp.FastMCP.tool` (`.prompt`) were not followed…", both `structurally_observed` (F1).
- **Q3.** The evidence is 41 `fact` rows and 4 `span` rows, with 0 `passage` rows. **All 45 are
  byte-identical to the installed files** under `build/envs/fastmcp/…/site-packages`. This was
  checked by slicing each file at `start_byte..end_byte` against the hex of `text`.
- **Q4.** 77 exact mentions exist, and none names a seed or a seed's export. So no pilot brief
  exercises the passage leg of the Outcome order.
- **Q5.** 21 of 31 boundary findings are at depth 2: `prompt` 8/8, `tool` 10/13 and `resource`
  3/10 (F3).
- **Q6.** 24 parameter assertions (15 of `tool`'s parameters, 9 of `prompt`'s) carry no
  requiredness, because their `parameter_semantics` join is null (O5).
- **Q7.** DESIGN's §B11 gap-metric query returns no rows (O6).
- **Q8.** 1,109 symbol nodes have two `context_definitions` rows. Every one has a single label
  (O4).
- **Q9.** The brief documents are 283–1,087 bytes, with `input_hash` null.

**Probes.** A scratch test in the export, moved out before clippy. Nothing was added to the
repository.

| Probe | What was run | Result |
|---|---|---|
| **P1** | The fixture compiled twice (reversed module order, another directory). The 8 synthesis tables compared with `SELECT * EXCLUDE (snapshot_id) … ORDER BY 1,2,3` | All identical |
| **P2** | The fixture with `max_witnesses = 1` | "`pkg.Server.tool` already calls `pkg.helpers.helper` (1 call site)." `helper` has 2 sites and the finding has `witnesses_omitted = true`. `validate` found no violations |
| **P3** | (a) Every parameter assertion relabelled `documented`. (b) Every evidence support row removed, leaving 2 Outcome and 3 parameter assertions unsupported | `validate` found no violations in either case. The published fixture already has one unsupported `structurally_observed` Limits assertion |
| **P4** | `synth::lead_sentence` over the pilot's exact-mention passages: 55 (passage, mention) pairs. 22 pairs were skipped because the printer truncates their passage cell: 21 in the 444,390-byte changelog passage, 1 in `docs/updates.mdx` | The chosen sentence contains the mention in 9 pairs and does not in 46. Examples below |
| **P5** | A hard-wrapped first sentence | Cut at the line break. In the pilot, 1 of 37 distinct passages is affected: "Azure AD B2C (Business-to-Consumer) uses different endpoints, scope URIs, and" |
| **P6** | `lead_sentence` on `docs/changelog.mdx` passage 0 | It returns "`**[v4.0.5: No Country for Loose Ints](https://github.com/PrefectHQ/fastmcp/releases/tag/v4.0.5)**`" |

P4 examples:
- `Middleware.on_initialize` → "FastMCP 4 is a stable release:"
- `FastMCP.from_openapi` → "Everything above, FastMCP handled for you."
- `Context.session` → a sentence about the low-level SDK `Server`.

**Measured on the installed FastMCP tree** (Python `ast`, a heuristic):
- 151 of 669 public, non-overload methods of public classes have no docstring. Seeded, each
  would take the passage leg, or be `unresolved` without an exact mention.
- 28 of 1,247 public docstrings wrap their first line into a second (O1).

**Not inspected, or asserted only.**
- Status propagation's `unresolved` branch and the `scope` role. No increment-1 finding can
  carry either.
- The budget-truncation Limits text (`synth.rs:788–791`) and the over-cap refusal (`:944–950`).
  No test or pilot run reaches them.
- The bundle-side use of `brief_documents`, which is slice 1.6.

**Guarantees not attacked.**
- Codebook append-only: the snapshot test exists, and I did not diff older codes.
- Delta publication of the new tables: they go through the existing `write_analysis`, which was
  not re-attacked.

## 2. Authority and lifecycle (compressed)

| Concept | Authority | Derived / checked by | Gap |
|---|---|---|---|
| Kind → section, permitted statuses | `ASSERTION_POLICY` (`findings.rs:306–340`) | `policy_rows` → `assertion_policy`; `semantic:assertion-policy`; `…-policy-published` | DESIGN §10.2's table (L1978) permits only `structurally_observed` for `parameter`, while the code also permits `documented` (F5) |
| An assertion's section | its kind only | — | ADR-0019 review O8 closed: no second or third copy. **Strength:** without it, a `related` row could be filed under Limits |
| An assertion's status | `derive_status` (`synth.rs:206–224`), a switch on kind | `semantic:assertion-status-propagation` (floor only) | No ceiling (F1) |
| Evidence text | source bytes (`source_files.text`, `passages.text`) | `semantic:evidence-text-bytes` (length); test (spans only) | Passage evidence is unchecked (F5) |
| Assertion, brief and evidence ids | `recipe::*` (`findings.rs:498–556`) | no oracle | F6 |
| `documentation_only` | `!uses_analysis` (`synth.rs:906`) | `semantic:brief-cites-analysis` | Constant `false` (F4) |

## 3–4. Contracts and derivation (merged)

| Invariant (DESIGN) | Enforcement | Rejects (injected case) | Gap |
|---|---|---|---|
| A status is one the kind permits (§10.2) | `semantic:assertion-policy` | an outcome marked structural | — |
| Statistical or unresolved support dominates (§10.2) | `…-status-propagation` | findings doctored statistical | Cannot fire on increment-1 data. `FINDING_STATUS` pins every kind to `structurally_observed` (O2) |
| **No status exceeds its evidence (§10.4 L2040)** | **none** | — | **F1**: P3 and Q2 |
| Text iff resolved | `…-text-iff-resolved` | text nulled | — |
| A support cites exactly one thing | `semantic:support-cites-one` | both null | — |
| Evidence text is its span's bytes (review F8) | length rule, and the test's byte check | end + 1 | The rule compares lengths, not bytes. The test covers module spans only (F5) |
| Named public symbols exist (§10.4 L2037) | `…-brief-member-exported` | foreign prefix | Only the exported prefix is checked (O3) |
| No bypass of the analytics (§1.5 L141) | `…-brief-cites-analysis` | every finding support removed | Vacuous (F4) |
| Never drop a limit for length (§10.4) | over-cap refuses the compile | not tested | §9 |

**Statistical output never states a control or limit (ADDENDUM Q6).** It holds by construction:
- Controls and Limits kinds permit no statistical status;
- the statistical kind `related` maps only to the Related section.

The guarantee depends on Stage F recording its supports, which nothing requires (F1). A future
template that reads a community finding but writes no support row would publish a
`structurally_observed` Limit, and every rule would pass.

## 5. Journey: a seed without a docstring (traced in code; its Outcome computed by P6)

`fastmcp.tools.Tool.from_tool` has no docstring (checked with `ast`). Suppose Stage F runs it as
a seed:
1. Its declaration has a null docstring span, so the passage leg runs (`synth.rs:551`).
2. The mention SQL orders the passages by `d.path, p.ordinal` (`:377`). `docs/changelog.mdx`
   sorts before `docs/updates.mdx`, so the first passage holding an exact mention of it is
   changelog passage 0: one 444,390-byte passage of `<Update>` blocks.
3. `lead_sentence` skips the `<Update …>` line and takes the first prose paragraph (P6).
4. Result: Outcome = "`**[v4.0.5: No Country for Loose Ints](https://github.com/PrefectHQ/fastmcp/releases/tag/v4.0.5)**`",
   with status `documented`, evidence `passage`, and verbatim text.
5. Every rule passes:
   - the length rule holds;
   - the policy permits `documented`;
   - the test's byte check never joins passage evidence.
6. The §B11 gap metric counts the slot as filled.

`Client.close` and `StdioMCPServer.to_transport` would get the same Outcome: changelog passage 0 is also their first exact mention by path (queried 2026-09-23). `Middleware.on_initialize`
would get "FastMCP 4 is a stable release:". ADR-0005 orders the Outcome sources precisely so
that text not about the seed never fills a slot the gap metric counts.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 — Authority | **fail** (documentary, narrow) | DESIGN §10.2 L1978 permits `structurally_observed` for `parameter`. `ASSERTION_POLICY` (`findings.rs:322–329`) also permits `documented`, and the published policy follows the code. Otherwise G1 holds: a section has one authority, the published policy is tied to the constant, and evidence text is a checked copy | Amend one of the two (F5) |
| G2 — Semantic fidelity | **fail** | Passage-leg Outcomes are not about the seed, yet published `documented` (F2: P4, P6). Templates state what the finding does not carry (F3): depth-2 boundaries read as direct calls (pilot Q5); a capped witness count reads as the call-site count (P2); candidacy is pinned on the wrong step (the committed snapshot); methods read as "Call it as". Unsupported assertions read `structurally_observed` (F1) | F2, F3, F1 |
| G3 — Validity | **fail** | §10.4's "no assertion's status exceeds its evidence" has no enforcement. P3's two violations and the pilot's two unsupported assertions all pass validation (F1). The other seven rules reject their injected cases (nextest passed) | F1's rule |
| G4 — Hidden behavior | **pass** | Stage F reads only the attempt's session tables and Stage E's rows (`synth.rs:274–497`). There are no filesystem, environment or clock reads. UAX #29 is pinned (`unicode-segmentation =1.13.3`). The gold is not read | — |
| G5 — Consistency and recovery | **pass** | Stage F rows are written through `write_analysis` before validation and the `snapshots` append (`attempt.rs`). An over-cap brief errors before publication. That path is Implemented, not tested | §9 |
| G6 — Transformation and reuse | **pass** (probe-checked) | Ids are content-derived, and evidence excludes the run-scoped fact (review F7). P1: identical across module order and location. Asserted beyond P1: no committed oracle pins the identity (F6). Assertion supports are hashed in draft order, where ADR-0019 L130 says "sorted" | F6 |
| G7 — Truthful capability claims | **fail** | DESIGN §10 L1916–1921 labels the Outcome order and "the grounding rules below" **Implemented and Tested**. The passage leg is untested end to end and defective (F2). The status ceiling is absent (F1). "Every evidence text byte-for-byte" skips passage evidence. The commit's "§1.5 no-bypass rule" cannot fail (F4) | F5, F4 |

An unresolved gate is not a pass. No score is given.

## 7. Findings

### 7.1 Findings

Ordered by severity. Each row is one cause.

| # | Finding | Principle IDs | Evidence | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| F1 | **An assertion's status has a floor but no ceiling. A resolved assertion may cite nothing, or cite evidence that cannot support its status.** | DM-07, DM-08, DM-46 · G3, G2 | `derive_status` returns `StructurallyObserved` for any non-Outcome kind with no findings and no evidence (`synth.rs:221–223`). The depth and budget Limits drafts carry `findings: Vec::new(), evidence: Vec::new()` (`:795–800`). The only status rule is the floor from findings (`rules.rs:282–293`). DESIGN L2040 lists "No assertion's status exceeds its evidence" as a §10.4 check. ADR-0005 L66: "Every sentence in a brief is traceable…" | **Pilot Q2:** 2 published `structurally_observed` Limits cite nothing. Their invocation, which holds the fact, is not cited. **P3:** a `parameter` assertion relabelled `documented` with only fact evidence passes, and so does an assertion stripped of all supports. Q6's guarantee rests on this: a template that forgets its support rows can publish a statistical claim as structural, and no rule notices | (1) Declare an `evidence_kind → status` map beside `FINDING_STATUS` (fact → structural; span, passage, example → documented; fixture run → fixture_checked). (2) Derive the status as a function of the supports only: no support → `unresolved`. Generate one rule that **recomputes and compares for equality**, replacing the floor-only rule. (3) Cite the depth or budget stop, either as a finding Pass A emits or as `invocation` evidence (a codebook append) | **None exists.** The recompute rule, with two injected cases: an evidence support removed, and a `parameter` relabelled `documented` |
| F2 | **The passage leg of the Outcome order publishes a passage's lead sentence, not a sentence about the seed, and cuts hard-wrapped sentences at the line.** | DM-08, DM-24, DM-59 · G2 | `synth.rs:551–553` takes the first passage (ordered `d.path, p.ordinal`, `:377`) with *any* lead sentence. `lead_sentence` (`:164–202`) ignores where the mention is. `split_sentence_bound_indices().next()` (`:199`) runs over raw lines, and UAX #29 treats LF as a paragraph separator. DESIGN L1999 ("the lead sentence of a doc passage that explicitly mentions the entry point") admits both readings | **P4:** in 46 of 55 pilot pairs the chosen sentence does not contain the mention, e.g. "Everything above, FastMCP handled for you." **P6/§5:** three docstring-less public methods would publish a changelog release title as `documented`. **P5:** "…scope URIs, and". 151 of 669 public FastMCP methods have no docstring, so any of them seeded would take this leg. The §B11 gap metric counts these slots as filled, which is what ADR-0005's order exists to prevent | Make DESIGN §10.3 decidable: "the sentence containing the exact mention" (or the paragraph's lead sentence only when the mention is in it). Split sentences on a view with soft line breaks joined, and map back to bytes. Exclude changelog and release-notes documents. **Until then, the simpler honest option:** make leg 2 `unresolved`. No pilot brief changes | **None exists.** A `docs_shapes`-style fixture with the mention in paragraph 2 and a wrapped first sentence: assert the Outcome, or `unresolved`. Extend the byte check to passage evidence (F5) |
| F3 | **The templates read a subset of a finding's fields and state what the missing fields would forbid.** | DM-24, DM-59, DM-46 · G2 (ADDENDUM Q5) | The Limits templates, "`X` calls into code outside…" (`synth.rs:748–759`), ignore `f.depth`. "`X` already calls `Y` ({paths} call sites)" counts witness paths (`:637`, `:644–648`) and ignores `witnesses_omitted` (`pass_a.rs:198`). "through `{via}`, an overridable call" (`:649–657`) sets `candidate` if **any** step of any path is a candidate (`:638`), but names step 0 of path 0 (`:639–642`). "Call it as `{access_path}`" (`:611`) is used for methods. The witness `phase` (`property_get`) is rendered as "calls" | **Pilot:** "`fastmcp.FastMCP.prompt` calls into code outside the analyzed release…: `builtins.BaseException.__init__`, …". All 8 targets are at depth 2, below `PromptDecoratorMixin.prompt` (Q5: 21 of 31). **P2:** "(1 call site)" for 2 sites. **The committed `analysis__briefs.snap` row 2 pins** "through `pkg.handlers.run`, an overridable call", although `run` is a definite direct delegation and the candidate arc is `run → Handler.handle`. All 4 pilot briefs say "Call it as `fastmcp.FastMCP.tool`". An agent reading that calls the unbound method | Make templates total over the finding's fields: "reaches" at depth ≥ 2, or group by depth. Count call sites from the arcs, or say "at least N" when `witnesses_omitted`. Name the step whose modality is candidate. "Reachable as" for access paths, with the seed's kind deciding the call form. Name property reads as reads. Bump `TEMPLATE_VERSION` | The `briefs` snapshot exists **but pins the misattribution**, so correct it. Add targeted assertions on `analysis_shapes`: depth-2 boundary wording, the P2 cap case, and the candidate step named |
| F4 | **§1.5's no-bypass rule cannot fail, and `documentation_only` cannot be true.** | DM-59, DM-08 · G7 | Every seed resolves through an exported prefix, or the compile is refused (`analyze.rs:227`), so its aliases are non-empty. Pass A always emits `public_alias` (`pass_a.rs:163`). Stage F cites it (`synth.rs:612`). So `uses_analysis` is always true (`:827`), the flag is always false (`:906`), and the rule (`rules.rs:330–335`) is one-directional. Its injected case works only by deleting every finding support | `custom_route`'s brief has 0 arcs, a complete invocation, and no delegation, limit or restriction. It is labelled analysis-backed on its export lookup alone. §1.5 and §12 cannot tell "a derivation family supports a claim" from "only the alias ran". The meta-test counts the rule as falsifiable | Define "analysis-backed" as citing a derivation-family finding (delegation, restriction, handoff; `public_alias` excluded). Make the rule bidirectional. Require that a `documentation_only` brief has a `documented` Outcome. Or declare the rule an edit guard and narrow the claim | **None exists.** A fixture seed whose Pass A finds only its alias: assert the flag, and each direction's injected case |
| F5 | **Claims and checks outrun the evidence.** | DM-59, DM-02, DM-53 · G7, G1 | DESIGN L1916–1921 labels the Outcome order and "the grounding rules below" Implemented and Tested. **Outcome order:** leg 2 is only unit-tested, on a one-line paragraph, and neither the fixture nor the pilot reaches it. Leg 3 is covered only by `derive_status`'s unit test. **Grounding:** the ceiling is absent (F1), and the symbol check is prefix-only (O3). **Byte check:** the test joins `source_files` (`analysis.rs:319–320`), so passage evidence is skipped by construction, and `checked >= 3` (`:347`) does not require the non-ASCII case. **Policy:** L1978 differs from `findings.rs:322–329` | A reader, or the 1.6 bundle author, trusts passage evidence and the §10.4 checks that no test ever saw | Relabel: leg 1 **Tested**; leg 2 **Implemented**, or disabled (F2); grounding ceiling **Proposed** until F1 lands. Extend the byte check to passage evidence through `passages`. Reconcile the `parameter` row, citing ADR-0019 if `documented` stays | Labels are prose. The byte check becomes a test once passage evidence exists in a fixture (F2's) |
| F6 | **Nothing pins the new identity recipes.** The evidence recipe's "fact as lineage" (review F7) and assertion and brief ids can change unnoticed, and the ledger digests text only. | DM-15, DM-32, DM-53 · G6 | ADR-0019 L120: "A property test per contract". `contracts.rs:167` covers findings only. The ADR-0019 review named "the context-variation test" as the acceptance evidence for F7. It was not added. `AssertionKey` hashes supports "in order" (`findings.rs:515`), where ADR-0019 L130 says "sorted supports". The ledger (`analysis.rs:353–368`) hashes the findings query and assertion **texts**: not ids, statuses, evidence, members or documents. The determinism test (`:276–288`) lists only Stage E's four tables | Re-adding `cited_fact_id` to the evidence id, or reordering drafts, renames every assertion and brief while every committed test stays green. ADR-0020's verdicts, bound to `brief_id`, would then go stale after any unrelated `uv lock` change. An id-only change to the output also needs no `TEMPLATE_VERSION` bump. P1 shows the ids are stable today | Add property tests for `recipe::evidence`, `AssertionKey` and `brief`: identity columns change the id, and the lineage `cited_fact_id` and `template_version` do not. Add P1's table list to `pass_a_is_identical_…`. Digest `SELECT * EXCLUDE (snapshot_id)` of all synthesis tables in the ledger. Sort supports, or amend ADR-0019 L130 to "in order" | **None exists.** The three property tests; the extended determinism test; the context-variation test from the ADR-0019 review (two contexts differing by an unrelated installed file give equal evidence, assertion and brief ids) |

### 7.2 Observations (no gate moves on these alone)

| # | Observation | Fix |
|---|---|---|
| O1 | The docstring leg takes the first **physical line** (`summary_span`, `synth.rs:139–159`). The passage leg takes a UAX #29 sentence. 28 of 1,247 public FastMCP docstrings wrap. `MCPConfig` would read "A configuration object for MCP Servers that conforms to the canonical MCP configuration format". `get_setting` would read "Get a setting. If the setting contains one or more `__`, it will be" | One sentence rule for both legs: the first sentence of the first paragraph, inside the literal's bytes, after F2's soft-break join. Test: a wrapped summary |
| O2 | `semantic:assertion-policy-published` compares `policy_rows()` with a `VALUES` list. Both are built from `ASSERTION_POLICY` (`synth.rs:236–248`, `rules.rs:256–280`), so no compile can violate it. The meta-test counts it as exercised, via a doctored table. `…-status-propagation` cannot fire until increment 2, when statistical finding kinds exist | Add the first to `EDIT_GUARDS`, as the C6 review set out. Note the second's first falsifiable slice |
| O3 | `semantic:brief-member-exported` accepts any member suffix of an exported prefix (`rules.rs:324`), e.g. `fastmcp.FastMCP.nonexistent`. Construction refuses a missing member (`analyze.rs:232`), so this is a weak validator, not a live defect | Also check that the member path's last name is `declaration_node_id`'s name. Better, that it lies on the export target's MRO |
| O4 | Labels: `ORDER BY n.node_id`, and the first row wins (`synth.rs:287`, `:300`). Q8: 1,109 nodes have two rows, one per run, with equal labels today. If they ever differ, the text changes, and with it the assertion and brief ids | `ORDER BY n.node_id, label`, or `min(label)` per node |
| O5 | Requiredness comes from `parameter_semantics` (`synth.rs:432–434`, `:709–713`), but only the syntax fact is cited (`:698`). For overloaded seeds the join is null, and "required/optional" silently disappears (Q6: 24 of 41). Listing the implementation's parameters also hides the overload contract (`tool(name_or_fn: F)`) | Cite the semantics fact as a second evidence row. Say "requiredness not observed" when it is absent. Pass B can own overloads |
| O6 | The §B11 gap metric counts only explicit `unresolved` assertions, and only the Outcome template emits one. Empty sections (custom_route's Coordinates and Limits; every brief's Usage pattern and Applicable case) are not slots. Q7 reads 0 | Decide which sections emit an `unresolved` slot when empty, before ADR-0005's increment-3 count. Deferred |
| O7 | Each brief document's Limits line is mostly `builtins.*` boundary lists, e.g. `prompt`'s 763 bytes. This text enters BM25 and the vector (§11.1). It is the same fan-out as 1.4 review O5. `custom_route`'s missing limit is 1.4 review F1: Stage F cannot state what Pass A does not emit | A retrieval hypothesis. Measure it with 1.6's §1.5 ranking check before changing the projection. Deferred |

### 7.3 Applicability and verdicts

**Groups that bore on this scope:**

| Group | What bore on it |
|---|---|
| 2 (invariants and absence) | DM-07 violated (F1); DM-08 violated (F1, F2, O5) |
| 5 (derivation) | DM-24 violated (F2, F3). DM-22: the templates have a version, not a contract over their fields (F3) |
| 10 (provenance) | DM-46 partly violated: 2 pilot assertions cite nothing (F1); O5 |
| 11 (verification) | DM-53 unresolved (F6) |
| 12 (claims) | DM-59 violated (F4, F5) |
| 3 (identity) | DM-11 and DM-15 satisfied on P1's evidence; the regression oracle is missing (F6) |
| 1 (authority) | DM-02 documentary divergence only (G1) |

Groups 1 and 3 bore only narrowly.

**Groups that did not bear on this scope:**
- **6 and 7:** Stage F is a pure, in-attempt function of session tables. G4 and G5 were checked.
- **8:** synthesis takes 0.06 s per the commit's pilot. That was not re-measured here, and no
  performance claim is at stake.
- **9:** no new provider. `unicode-segmentation` is pinned, with a dated pins row.
- **4:** templates are ordinary code behind a declared policy, which is proportionate.

## 8. Alternatives (compressed)

| Alternative | Duplication and extension | Risk | Verdict |
|---|---|---|---|
| As committed | Status is a kind switch in Rust (`derive_status`) plus a floor-only SQL rule. The two derive one fact in two places, and neither bounds it from above | F1 | — |
| **Simpler viable: status as a declared function of supports** | One `evidence_kind → status` map and one SQL rule that recomputes the status and requires equality. It replaces the Outcome special case, the propagation rule and the missing ceiling. A new evidence kind is one map row | Removes F1's class by construction | **Recommended** |
| Outcome leg 2 disabled until F2's rule and fixture exist | Removes the leg (−30 lines). Docstring-less seeds become `unresolved`, and the gap metric counts them, which is ADR-0005's purpose | None for the 4 pilot briefs | **Recommended if F2 is not fixed before 1.7 serves briefs** |

## 9. Top verification gaps and labels now supported

| Claim | Label now | Gap |
|---|---|---|
| Evidence text verbatim | **Tested** for module spans and facts (fixture). Verified on the pilot, 45/45 on disk, 2026-09-23 | Passage evidence has never been produced or checked (F2, F5) |
| Outcome order, leg 1 | **Tested** | — |
| Outcome order, leg 2 | **Implemented** (unit test on one line) | A docs fixture (F2) |
| Status propagation (floor) | **Implemented**; injected case passes | Unfalsifiable on increment-1 data (O2) |
| Status ceiling (§10.4) | **Proposed** (no mechanism) | F1's rule |
| Template fidelity | **Tested as a snapshot that pins a defect** | F3's targeted assertions |
| Identity recipes and determinism | **Implemented**; probe P1 only | F6's tests |
| Budget-truncation Limit; over-cap refusal | **Implemented** | A `max_vertices = 1` fixture run (the ADR-0019 review F4 oracle) and an over-cap fixture |

## 10. Exceptions and unresolved decisions

No SHOULD-level exceptions are requested. Two decisions for the author:
1. **What leg 2 of the Outcome order selects** (F2). This is a DESIGN §10.3 amendment, not an
   implementation detail.
2. **Whether `parameter` may be `documented`** (F5). Either DESIGN §10.2 changes (with an
   ADR-0019 note), or the policy does.

## 11. Decision

**Decision: Revise.** G1 (documentary), G2, G3 and G7 fail on behaviour the slice claims.
- Two of the failures appear in the published pilot briefs:
  - unsupported `structurally_observed` Limits;
  - Limits and access text stating direct calls that the findings do not show.
- The passage leg would publish text unrelated to the seed as `documented`. §5 shows three
  FastMCP methods that would receive a changelog title.

The fixes are small, and none reopens ADR-0019's decisions. What holds:
- the policy's single authority;
- byte-located docstrings, verbatim in the pilot;
- text iff resolved;
- in-attempt publication;
- identity that is deterministic under reordering (P1).

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| P1 | F1: status as a function of supports; cite the depth or budget stop; the recompute rule | DM-07, DM-08 | Pilot: 0 unsupported resolved assertions | The rule, with 2 injected cases |
| P1 | F2: decide leg 2 (the mention sentence plus soft-break join, or disable it) | DM-08, DM-24 | P4 and P6 rerun | A docs fixture test |
| P1 | F3: total templates, and correct `analysis__briefs.snap` | DM-24 | Pilot Limits say "reaches" at depth 2 | Targeted assertions, including P2's cap case |
| P1 | F5: relabel DESIGN §10; reconcile the `parameter` row; byte-check passage evidence | DM-59, DM-02 | `adr lint`; the test | The extended byte check |
| P2 | F4: a bidirectional, derivation-family no-bypass rule, or an edit guard | DM-59 | An alias-only fixture seed | Its injected cases |
| P2 | F6: property tests; extend the determinism test and the ledger; sort supports or amend ADR-0019 L130 | DM-15, DM-53 | P1 as a committed test | The tests |
| P3 | O1–O5 | — | per row | per row |

### Deferred

| Item | Trigger that reopens it |
|---|---|
| O6: unresolved slots for empty sections | The first increment-2 section template (Pass B limits or a usage pattern), and in any case before ADR-0005's increment-3 count |
| O7: `builtins` lists in the brief document | 1.6's §1.5 ranking check puts a target brief below a distractor |
| O2 (second half): propagation's first falsifiable data | Increment 2's first statistical finding kind |

## Disposition (2026-09-23, the slice 1.5 review-fix commit)

| Item | Disposition | Where |
|---|---|---|
| F1 | Fixed. The status is `findings::derive_status` of the supports, with `EVIDENCE_STATUS` and `STATUS_STRENGTH` declared; no support gives `unresolved`. `semantic:assertion-status-derived` recomputes it and compares for equality; it replaces the floor-only rule and has three injected cases (an evidence support removed, a parameter relabelled `documented`, a statistical finding). The depth and budget Limits cite Pass A's new `traversal_stop` finding (D10) | `findings.rs`, `rules.rs`, `pass_a.rs`, `synth.rs`; `an_assertion_status_is_a_function_of_its_supports` |
| F2 | Fixed by the stricter rule (D7): the mention must be in its paragraph's lead sentence; soft-break view; changelogs excluded. Writing the fixture test found a second defect: a member seed's class export counted as a mention of the member. Seed exports now come from `exports.target_node_id` | `synth.rs`; DESIGN §10.3; `an_outcome_from_the_docs_is_the_sentence_that_mentions_the_seed`, `an_outcome_sentence_holds_its_mention` |
| F3 | Fixed. Limits are split by depth ("Callables `X` reaches call …"). The call-site count says "or more" under the cap. Each hop that is a definition, a property access or an override-open call is named where it occurs. Public access follows the declaration's form (function, class, method, class or static method, property). Property reads are called reads. `analysis__briefs.snap` corrected. `TEMPLATE_VERSION` 3 | `synth.rs`; `templates_say_what_the_findings_show`; fixture `prepare` gains a depth-2 boundary |
| F4 | Fixed. `ANALYSIS_BACKED` (D9); `semantic:brief-cites-analysis` both ways; `semantic:documentation-only-has-outcome`. The alias-only seed `pkg.describe` gives a documentation-only brief. Each direction has an injected case | `findings.rs`, `rules.rs`; `analysis_shapes` `helpers.describe` |
| F5 | Fixed. DESIGN §10's labels name the tests that hold them. The `parameter` row is reconciled (D8). The byte check covers spanned evidence only, requires one span past a non-ASCII byte, and passage evidence is byte-checked on the corpus fixture | DESIGN §10; `analysis.rs`, `syntax.rs` |
| F6 | Fixed. Property tests for the evidence, assertion and brief recipes. The determinism test covers the seven synthesis tables. The ledger digests every identity column of them. `synthesis_ids_ignore_the_runs_they_cite` compiles for two platforms: every cited fact id differs, and no finding, evidence, assertion or brief id does. Supports are sorted in `AssertionKey`. Finding: changing a dependency file renames its unowned module's symbols and the boundary findings naming them, by design (DESIGN §3.4.1) | `contracts.rs`, `analysis.rs` |
| O1 | Fixed with F2: one sentence rule for both legs | `summary_span`; `a_summary_span_is_the_first_sentence_of_the_first_paragraph` |
| O2 | Fixed. `semantic:assertion-policy-published` is an edit guard. Propagation is now falsifiable through the recompute rule's statistical case | `rules.rs` |
| O3 | Fixed. A member path must end in the declaration's name, with an injected case. The MRO check stays with `member()`'s fail-closed walk (slice 1.4 review F2) | `rules.rs` |
| O4 | Fixed. The least label per node | `synth.rs` |
| O5 | Fixed. The semantics fact is a second evidence row, and "requiredness not observed" is said when Pysa has none. Overload contracts stay with Pass B | `synth.rs` |
| O6, O7, O2 (second half) | O6 and O7 stay deferred with their triggers. The second half of O2 is closed by the recompute rule | — |
