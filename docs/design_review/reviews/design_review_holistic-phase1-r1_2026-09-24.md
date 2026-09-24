# Design review: holistic-assessment plan, Phase 1 (R1): D1, A2(a), B1's first piece, A3 (compact)

## 1. Decision and scope

**Decision.**
- **Accept** D1 (`3cb4d96`), A2(a) (`239c800`) and B1's first piece (`116832d`).
- **Revise (small)** A3 (`a67d8f8`, superseded by `ca5ff77`).
  - The facts, the extractor and the four new rules hold, and every gate passes.
  - Two things are missing:
    - DESIGN §10.3 names two scoping branches that stop a documentation statement landing on the
      wrong subject, and neither has a discriminating test (F2). That is a DM-54/DM-07 MUST-level
      gap on behaviour DESIGN labels Tested.
    - The anchor that decides which brief a warning or `<ParamField>` lands in is neither cited
      nor validated (F1).
  - Both must close before D45 is decided, because D45 changes exactly that anchor.

**D45 (§10).** Keep the exact-mention anchor now. Do not widen it in Phase 1, and do not choose a
widening from the pilot's yield. Before 4.1's re-registration commit:
- land F1;
- pre-register **one** widening candidate as a content variant that 4.3 decides: a
  receiver-resolved mention (`@mcp.tool`, where the page's examples bind `mcp` to a `FastMCP`);
- record the heading/Card-title anchor and the code-block call anchor as rejected.

**Proposal.**
- `3cb4d96` (D1): `analyze::run` takes the attempt's `Stages` and marks each technique; the single
  "analyze (Pass A)" mark goes.
- `239c800` (A2(a); ADR-0020 review F3, F8):
  - `variant_config_digest` hashes the config digest with the whole technique set's JSON;
  - the selection invocation records the set;
  - `+rca` without `+fca` is refused;
  - `COMPILER_OUTPUT_VERSION` 16.
- `116832d` (B1, first piece): `EvidenceRow::new` derives `evidence_id` from the row's own
  columns. All eight Stage F sites use it.
- `a67d8f8` (slice 3.4, interim): `documented_warning` (16) through the string helper
  `synth::warnings`. It is superseded by the next commit.
- `ca5ff77` (A3):
  - two new tables, `doc_components` and `doc_component_attributes`, from markdown-rs's mdast;
  - codebooks `component_form` and `attribute_value_kind`;
  - four semantic rules, and a ParamField branch added to
    `semantic:documented-parameter-cites-its-doc`;
  - Stage F warnings from `<Warning>` components;
  - a top-level `<ParamField>` as a parameter's description after the docstring;
  - `EXTRACTOR_OUTPUT_VERSION` 19 and `TEMPLATE_VERSION` 16.

**Status:** Implemented. The Tested scope is audited in §9.
**Reviewer / author:** design-reviewer subagent (fresh context) / the Phase 1 session.
**Affected revisions:**
- `main` at `ca5ff77`. The later `28443a0` is docs only (the ADR-0010 pre-registration).
- Pilot snapshot `8c76bc75efac42085c2ab2a1595de0fb`, generation `9829b85dcff20462` (the author's
  `build/pilot.log`, 2026-09-24 00:43).

**Observable outcome.**
- Stage E's time is attributed per technique.
- Two technique sets never share a run id.
- An evidence row's id is a function of the row.
- MDX components are facts from the one parse. Stage F reads warnings and parameter descriptions
  from them instead of by string search.
- On the pilot, no warning and no `<ParamField>` description reaches a brief (Measured,
  re-derived in §9).

**Baseline.** The interim `synth::warnings` was a second parser of MDX:
- it matched inside code fences and inline code;
- it missed `<Warning title=…>`;
- it read a `<ParamField>`-nested warning as a warning about the whole seed (assessment A3).

### Method and coverage

**Read:**
- the five diffs;
- `cpg-extract/src/docs.rs` (`collect_in` :344, `document` passage cutting :551 and components
  :705);
- the `tables`/`codebook`/`rules`/`graph` diffs and the rules snapshot;
- in `synth.rs`: the mention query (:655), the component load (:754), `param_field` (:804),
  `fielded` (:1330), `agreed` (:168) and the warnings loop (:1984–2043);
- in `analyze.rs`: `Techniques`, `variant_config_digest` (:140), `SelectionParameters` (:545) and
  the D1 marks;
- in `attempt.rs`: `compiler_digest` and the Stage E call;
- `findings.rs:568`;
- `diff.rs:51`;
- the three named tests, the docs unit test, the ledger and the rule-injection cases;
- the `docs_shapes` fixture;
- DESIGN §3.2 (docs row), §8, §9.2, §9.8, §10.2 (kind table) and §10.3;
- D44 and D45;
- the plan's Phase 1 and 4.1–4.3;
- the charter, the addendum and the graph guidelines.

**Ran:**
- `just test-all`: passed (§9).
- `target/release/lctx query` (read-only) on the pilot snapshot, to re-derive DESIGN §10.3's
  counts and to probe the corpus. The queries are in §9.

**Not run:**
- `just pilot`. It appends to `build/store`, and this review changes one file only. The author's
  log was read instead, and its counts re-derived read-only.
- Mutation testing. F2's surviving-mutant claims are by inspection of the fixture.
- `lctx diff`.
- The Python side, which these commits do not touch.

**Attacked:**
1. Whether DESIGN's "top-level" matches the code and the pilot's shapes (F3).
2. Whether a nested warning or a nested `<ParamField>` could be misattributed without a test
   noticing (F2).
3. Whether the validator enforces what Stage F's policy promises (F1).
4. Whether `param_field` fails open on a missing parent.
   - It does: `components.get(..)?` returns `None`, which reads as "about the seed"
     (synth.rs:804–814).
   - But `doc-component-parent` (the parent contains the child) and `doc-component-in-passage`
     together force a parent into its child's passage.
   - Validation gates publication, so no published snapshot can hold that state. **Holds.**
5. Whether A2(a)'s digest collides. The 256-set test covers it. **Holds.**
6. Whether recording the set in the selection invocation makes every variant "change published
   output" under the keep rule. It does not: `lctx diff` compares findings, assertions, evidence,
   briefs and documents only (diff.rs:51–62). **Holds.**
7. D45's code-block counterfactual. Reconstructed approximately (§9, F4).

**Asserted, not attacked:**
- markdown-rs's JSX spans beyond the unit test and the pilot's validation.
- `(document, ordinal)` uniqueness. The key includes `fact_id`, as in every ordinal-keyed docs
  table, which is a repository convention and not this change's.
- JSX inside an MDX expression (`{cond && <Warning/>}`) is invisible to the mdast. A grep of the
  pinned FastMCP docs tree found no such case.

**This review's own reads under DM-59.** To answer D45, the reviewer:
- read the corpus's component structure;
- read which pilot parameters are undescribed;
- repeated the author's code-block probe.

It computed no other candidate anchor's brief output. §10 says what that means for 4.3.

## 2. Authority and lifecycle

- **`doc_components` and `doc_component_attributes`: raw docs facts (family Docs).**
  - The one authority is markdown-rs's mdast, through `collect_in`, in the same parse as
    passages, code blocks, links and mentions. The second parser is gone (`grep` finds no
    `"<Warning>"` search left in `crates/cpg-core/src`).
  - Identity is `fact_id`, keyed by (document, ordinal). There is no graph node.
  - The migration is declared: `EXTRACTOR_OUTPUT_VERSION` 19 moves `producer_id`, and the
    codebooks are appended to the registry's end.
- **The Stage F policy** (which component means what):
  - It lives in `synth.rs`.
  - The component vocabulary (`ParamField`, `Warning`, `body`, `title`) is string literals,
    spelled again in rules.rs:326–328 (F5).
- **The anchor** ("the passage mentions the seed exactly") is Stage F's mention query
  (synth.rs:655–668). It is not in the evidence or the rules (F1).
- **The technique set** is `Techniques`.
  - Its whole JSON joins `variant_config_digest` (analyze.rs:140; attempt.rs:301) and the
    selection invocation's parameters (analyze.rs:545).
  - A new flag changes every set's JSON, so it moves every run id, which is conservative.
- **The evidence id** is `EvidenceRow::new` (findings.rs:568). It is the only constructor in use,
  by convention only (F6).

§3–§4 are merged into §5 and §7 at this depth.

## 5. One journey: reversing D45 (a changed anchor)

D45's "Reverse by" says "the machinery is anchor-agnostic". Trace a switch from the exact mention
to any other anchor:
1. `passages` (synth.rs:655–730) is built from exact mentions only, and it feeds Outcome leg 2,
   `fielded` and the warnings loop. A widening changes all three unless the new anchor gets its
   own map.
2. The warning text hardcodes "(which mentions `{seed_label}`)" (synth.rs:2037). Under any anchor
   other than a mention of the seed, that published sentence is **false**. The template must
   change with the anchor.
3. The warning cites one Passage evidence row, its inner bytes (synth.rs:2025–2033). The
   `<ParamField>` description cites its lead (synth.rs:1381–1393). The anchor is not a support
   row, so `lctx diff` shows the new assertions but not why they attach, and no rule can see the
   anchor.
4. `semantic:documented-parameter-cites-its-doc`'s new branch checks only that the ParamField's
   `body` equals the parameter's name (rules.rs:321–333). It would accept any widening, right or
   wrong. No rule names `documented_warning` at all.

So a reversal today means a template change plus a validator that does not exist. That is F1.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | pass | One parse yields every docs fact, and the string helper is deleted. The evidence id is derived from the row (findings.rs:568). The technique set is digested from the struct itself (analyze.rs:140). The component vocabulary is spelled in two crates, but a drift fails validation closed (F5) | F5 (low) |
| G2 Semantic fidelity | pass | Fences and inline code yield no component (Tested). An expression's value is never read as a literal: Stage F loads only `value_kind = literal` (synth.rs:754). The warning text claims only co-location ("which mentions"). A field-scoped warning is skipped unless it names a seed parameter (synth.rs:2008). "Top-level" is worded ambiguously in DESIGN, but the implemented meaning is the plan's, and DESIGN's next sentence supports it (F3) | F3 |
| G3 Validity | pass | The anchor and the scoping are enforced by construction in Stage F. Each of the four new rules rejects an injected violation, and the harness asserts the named rule fires (syntax.rs:967–979). The shared validator's ParamField branch is weaker than the policy and has no negative case (F1). With F2's missing negative tests, this is why A3 is Revise | F1, F2 |
| G4 Hidden behaviour | pass | The component load reads the attempt's session only. There is no ambient read, and nothing is read from `.claude/skills/` | — |
| G5 Consistency and recovery | pass | Both tables are in `for_each_table` (tables.rs:1167–1168), so they are written and published within the attempt. `an_attempt_publishes_every_table_and_readers_see_only_published_rows` counts 39 + 21 + 13 | — |
| G6 Transformation and reuse | pass | `EXTRACTOR_OUTPUT_VERSION` 19 moves the producer; `TEMPLATE_VERSION` 16, the contracts and the rules are in `compiler_digest` (attempt.rs:136). A2(a) closes the F3 run-id collision. The Stage F component SQL is covered by `TEMPLATE_VERSION` plus the ledger only: A2(b) is open and planned for Phase 3 | — |
| G7 Truthful capability claims | pass | DESIGN §10.3 states zero yield on the pilot, labels it Measured, and it re-derives. The "Implemented and Tested" labels outrun the tests on two branches (F2), a DM-59 label defect rather than an unbacked capability | F2, F4 |

## 7. Principle findings

Ordered by severity. Each Verification cell names an oracle tier (ADDENDUM §4).

**F1. The anchor that decides which brief a documented warning or `<ParamField>` description
lands in is neither cited nor validated.**
- *Principles:* DM-07, DM-46, DM-60; guideline §10 (evidence sufficient to substantiate
  agent-facing claims).
- *Evidence:*
  - Each warning cites only its inner bytes (synth.rs:2025–2043), and a fielded parameter only
    its lead (synth.rs:1381–1393).
  - No rule names `documented_warning`.
  - rules.rs:321–333 accepts a Passage evidence row that is a ParamField lead whose literal `body`
    equals the parameter's name. It checks neither "no ParamField ancestor" nor that the lead's
    passage anchors the seed.
  - So the rule's stated guarantee ("not any documentation of its operation, nor another
    parameter's") does not hold for this branch.
  - No injected case exercises the branch.
- *Consequence:*
  - Pilot `<Card>`s hold ParamFields for `@tool`, `@resource` and `@prompt` Decorator Arguments,
    the `OAuthProxy` and `OIDCProxy` Parameters, and the `FastMCP` constructor: 21 passages. They
    share names such as `name`, `title`, `version` and `tags`.
  - A Stage F regression, or a D45 widening, could attach `resources.mdx`'s `title` field to
    `FastMCP.tool`'s `title` parameter.
  - That assertion would pass validation and publish as `documented`. `lctx diff` would show the
    new text without the anchor that caused it.
- *Correction (S):*
  - Cite the anchoring mention's fact as `scope` support on both assertion kinds.
  - Add `semantic:documented-warning-anchored`: the evidence is a `Warning` component's inner
    span; its passage holds a scope-cited anchor to the seed; a field-scoped warning names a seed
    parameter.
  - Tighten the ParamField branch to the same anchor, plus "no ParamField ancestor".
  - `TEMPLATE_VERSION` 17.
- *Verification:* test. Injected cases in `the_corpus_rules_reject_their_violations`: move a
  warning's evidence to an unanchored passage; re-point a lead to a nested field or to another
  passage. **None exists today.**

**F2. DESIGN §10.3 names scoping branches to prevent misattribution, and none has a
discriminating fixture. The pilot's dominant shapes are missing from the fixture.**
- *Principles:* DM-54, DM-07, DM-59; guideline §12 (known-answer shapes).
- *Evidence:*
  - In the fixture (quickstart.mdx:43–61), the only nested `<Warning>` sits under `timeout`, which
    is a parameter.
  - `grace` (nested) and `force` are not parameters of `stop`. `fielded`'s name test
    (synth.rs:1343) drops them before the ancestor test (:1344) is ever the deciding condition.
  - No warning sits under a field that is not a parameter.
  - By inspection, each of these mutants leaves every fixture output unchanged:
    - `Some(_) => continue` becomes "about the seed" (:2008);
    - the ancestor test is dropped (:1344);
    - "nearest ParamField" becomes "outermost" (:804–814).
  - "The nested `grace` and the stray `force` describe nothing" (syntax.rs:1368) is vacuously
    true.
  - On the pilot, **all 173 ParamFields have a parent**:
    - 123 under `Card`;
    - 48 at depth 3, under `ParamField > Expandable`;
    - 2 directly under `ParamField`.
  - The fixture has no field under a container that is not a ParamField. DESIGN labels both
    bullets "Implemented and Tested".
- *Consequence:*
  - The defect A3 exists to remove, a field-scoped warning stated about the whole seed, could
    return in Phase 3's module-by-module conversion of `synth.rs` or in B6's builder split. Those
    are declared "byte-identical under the ledger and the guard".
  - No test would fail, and the pilot's zero yield would not show it either.
- *Correction (S; appended per D44 so that no earlier offset moves):*
  - a `<Warning>` inside `force` (skipped);
  - a `<Warning>` inside the nested `grace` (nearest wins, so skipped; "outermost" would say
    `timeout`);
  - a new undescribed parameter of `stop` whose only field is nested
    `ParamField > Expandable > ParamField` (excluded);
  - another whose field sits in a `<Card>` (included);
  - a `<Warning>` in a passage that mentions no seed.
  - Relabel §10.3 as Tested for what these cases cover.
- *Verification:* test. Extend `a_documented_warning_is_a_limit` and the
  `docs_shapes_fielded_parameters` snapshot.

**F3. "Top-level" means three things in DESIGN, and §10.3's rule uses the one the code does
not.**
- *Principles:* DM-59 (specifiability), DM-02.
- *Evidence:*
  - DESIGN:506, "passages (each top-level heading's section…)", means a root-level heading of any
    depth. The pilot's § Decorator Arguments is a `###` heading.
  - DESIGN:506 and tables.rs:1062, "parent ordinal … null at the top level", mean depth 0.
  - DESIGN:2597, "a **top-level** `<ParamField>`", means what the code does (synth.rs:1344): no
    `<ParamField>` ancestor. The plan states that meaning; DESIGN does not.
- *Consequence:*
  - An implementer who reads §10.3 in the schema's sense writes `parent_ordinal IS NULL`. That
    could be Phase 3's relation for this query, or a second library's review.
  - On the pilot, that excludes all 173 ParamFields. On today's fixture it changes nothing (F2),
    so no test fails.
- *Correction:*
  - One sentence in §10.3: "one with no `<ParamField>` ancestor; a `<Card>` or `<Expandable>`
    around it does not count".
  - Say "root-level heading" for passages.
- *Verification:* test, F2's Card-wrapped case. Otherwise prose.

**F4. DESIGN §9.8 and §10.2 are stale against these commits, and §10.3's counterfactual is
labelled Measured without a reproducible query.**
- *Principles:* DM-02, DM-59.
- *Evidence:*
  - §9.8, DESIGN:2432, still says two things that are no longer true:
    - "A variant's canonical label (`-knn,+rca`) joins the compiler run's config digest". Since
      `239c800`, the whole set's JSON joins it (analyze.rs:140).
    - "the default's parameters, and so its invocation ids, are unchanged". The selection
      invocation's parameters, and so its id, now differ per set (analyze.rs:1206–1220).
  - DESIGN:2430 omits "RCA needs FCA" (analyze.rs:111).
  - §10.2's kind table (DESIGN:2517–2535):
    - it has no `documented_warning` row;
    - its `parameter` row still says "documented (with parameter-doc evidence only)", although a
      Passage-evidenced ParamField lead now makes a parameter documented.
  - §10.3's "13 warnings in 7 briefs … 44 passages" is a probe whose query is committed nowhere.
    The reviewer's reconstruction gives 14 warnings in 9 briefs, and exactly 44 passages for
    `TransportMixin.run` (§9).
- *Consequence:*
  - DESIGN is authoritative by construction. A session that writes the 4.3 targets from §10.2,
    or reasons about ablation ids from §9.8, works from the pre-change contract.
  - D45's rejection of the code-block anchor rests on a number nobody can regenerate.
- *Correction:*
  - Amend the three passages.
  - Commit the probe's SQL, or relabel those counts "probe, 2026-09-24".
- *Verification:* prose; there is no mechanical oracle. Optional: a test that every
  `AssertionKind` name appears in §10.2.

**F5. The component vocabulary is spelled out in two crates, and its Mintlify scope is not
declared.**
- *Principles:* DM-52, DM-56, DM-04.
- *Evidence:*
  - The literals `"ParamField"`, `"Warning"`, `"body"` and `"title"` appear at
    synth.rs:808–809, 1342–1343, 2000 and 2022, and at rules.rs:326–328.
  - `<ParamField>` and `<Warning>` are Mintlify components. §10.3 does not say that a library
    documented with Sphinx or MkDocs has no such source.
  - 7 pilot ParamFields use `path=` rather than `body=`, and they are never read. That policy is
    unstated.
- *Consequence:*
  - Widening the policy in `synth.rs` without the rule makes validation reject it. That fails
    closed, so it is not a G1 failure: the cost is a coordinated edit.
  - On a second library, a zero reads as "no warnings" rather than "no source".
- *Correction:*
  - One small `cpg_schema` declaration (a component role, and the attribute that names a
    parameter), read by both Stage F and the rule.
  - One sentence in §10.3 naming the convention.
- *Verification:* an ast-grep rule against the literals in `crates/cpg-core/src`, once the
  declaration exists. Until then, none.

**F6. `EvidenceRow::new` is the only constructor by convention alone.**
- *Principles:* DM-60, DM-02.
- *Evidence:*
  - The fields are public, because `table!` generates them.
  - `EvidenceRow {` appears only in the `impl` (findings.rs:568).
  - No rule recomputes `evidence_id`.
- *Consequence:* a new evidence site could build a literal and hash values other than those it
  writes. It would publish an id that is not a function of its row, so deduplication by id would
  merge two different texts or split one.
- *Correction:* an ast-grep rule that forbids `EvidenceRow { $$$ }` outside
  `cpg-schema/src/findings.rs`. Extend it to Phase 5's constructors.
- *Verification:* an ast-grep rule plus a rule test. **None exists.**

**Observations (not findings).**
- **O1 (D1).** On the pilot, "analyze: Pass A" is 0.00 s and "seed selection" is 0.90 s. That is
  `resolve_seeds` (D2's cost), now visible where it was hidden before.
- **O2.** The assessment's "9 ParamField-nested warnings" was measured on 4.0.3. The 4.0.5
  snapshot has 2 (the parents of all 93 warnings: `Step` 12, `ParamField` 2, none 79).
- **O3.** `28443a0` (Phase 2's pre-registration) landed before R1's disposition. It does no harm:
  R1's D45 recommendation targets 4.1, which is still ahead.

**Applicability.**
- Groups that bore on this scope:
  - 2 (invariants and negative tests: F1–F3);
  - 3 (identity: A2(a), B1);
  - 7 (dependencies and versions: A2(a), G6);
  - 9 (the markdown-rs adapter);
  - 10 (lineage: F1);
  - 11 (migration, tests and generation: F2, F5, F6);
  - 12 (DM-58/59: D45, F4).
- Groups that did not:
  - 6: no new effects, and publication is unchanged;
  - 8: D1 is instrumentation and makes no performance claim;
  - 4: only F5 touches declarations;
  - 1: the modelling boundary is unchanged, apart from F5's undeclared scope.

## 8. Alternatives

| Alternative | Duplication and locality | Risks | Cost | Why selected or rejected |
|---|---|---|---|---|
| Baseline: the interim string helper | A second MDX parser | Matches in fences, misses titles, misattributes nested warnings (A3) | — | Rejected by the assessment |
| As built: components as facts, read by Stage F | One parse; the policy strings sit in two places (F5) | The anchor is not cited (F1); branches are untested (F2) | Two tables, two codebooks, four rules; writes about 0.01 s on the pilot | Selected |
| Simpler: delete the helper, ship no warnings or ParamField descriptions until an anchor reaches them, and add no component tables | No decisions to maintain | None today: it publishes nothing on the pilot, which is exactly what the built version publishes | Negative | **Rejected, narrowly.** The tables are cheap and validated, and they are the input D45's candidate needs. Removing them later is a delete. If 4.3 rejects every widening, though, the Stage F consumers publish nothing on the only library (Deferred R1-1) |

## 9. Verification and measurement

**Check outcomes (2026-09-24).**
- `just test-all`: **passed**.
  - nextest 202/202 (`the_corpus_rules_reject_their_violations` 63 s;
    `every_rule_kind_rejects_its_violation` 116 s).
  - pytest 52 and pyrefly.
  - ast-grep scan, and 4 rule tests.
  - `adr lint` (20 records) and `lint-agents`.
  - `fixtures-check` (52 files).
  - `just deps` (family, deny, Pyrefly fork, shear).
  - gold.
  - The run finished at 00:47:56, on code identical to `ca5ff77`. The uncommitted Phase 2 edits
    in the worktree are later (00:48–00:53, by mtime).
- `just pilot`: `not_run` by the reviewer. The author's `build/pilot.log` was read (snapshot
  `8c76bc75…`, 20 briefs, the per-technique marks present), and its counts were re-derived as
  below.

**Claims and their labels.**

| Claim | Label | Evidence | Gap |
|---|---|---|---|
| Components come from the mdast; fences and inline code yield none; nesting, form, spans (non-ASCII) and attribute kinds are recorded | Tested | `components_are_the_mdast_elements` (docs.rs:780); the `a_corpus_documents_its_library` snapshot (spans sliced in Rust) | — |
| The four A3 rules reject their violations | Tested | syntax.rs:932–962 through the named-rule assertion at :967–979 | The ParamField branch of the parameter rule has no case (F1) |
| Titled, fenced, inline and parameter-scoped warnings | Tested | `a_documented_warning_is_a_limit` | The skip and nearest branches are untested (F2) |
| A `<ParamField>` describes an undescribed parameter; the docstring comes first | Tested | the `docs_shapes_fielded_parameters` snapshot (`timeout` from the field, `drain` from the docstring) | Nested and Card shapes are missing (F2, F3) |
| Disagreeing fields state nothing | Tested (unit) | `fields_describe_a_parameter_only_when_they_agree` | No integration case |
| Pilot: 1,250 components (450 nested, depth at most 3); 1,605 attributes (1,504 literal, 70 expression, 31 bare); 93 `Warning`, 173 `ParamField`; 0 `documented_warning` assertions; 60 of 97 parameters documented; one warning-holding passage has any exact mention | **Measured**, re-derived | `lctx query` on `8c76bc75…`: `count`s over `doc_components` and `doc_component_attributes`, `assertions` by kind and status, and warning passages joined to `mentions` with `class = 0` | — |
| Code-block anchor: 13 warnings in 7 briefs; `run` anchors 44 passages | Probe, not reproducible as stated | Reviewer's reconstruction (`code_blocks` → edge 34 → `call_syntax` → edge 7 → a brief's seed; changelog paths excluded): 14 warnings, 9 briefs; `run` 44 passages | F4 |
| Every technique set has its own config digest, never the bare one | Tested | `every_technique_set_has_its_own_config_digest` (analyze.rs:1701) | — |
| `+rca` without `+fca` is refused | Tested | `a_variant_is_the_default_changed_by_name_and_labelled_canonically` | — |
| A2(a), B1 and D1 leave findings and briefs unchanged | Tested | the ledger rows 16 and 15, one digest (`briefs_are_synthesized_from_findings_and_verbatim_evidence`); snapshots unchanged | — |
| Per-technique Stage E timing | Implemented, observed | `build/pilot.log` | — |

## 10. D45: the anchor

**Recommendation: keep the exact-mention anchor now.**
- Do not widen it in Phase 1, and do not pick a widening from the pilot's yield.
- Make "wait for 4.3" a falsifiable test rather than a deferral, by doing (a)–(d) below before
  the 4.1 re-registration commit and before any variant's output is read.

**Why not now.**
1. **The zero is honest.** "Zero reach a brief" is Measured. The published sentence "(which
   mentions `seed`)" is literally true of every warning the exact anchor admits. The zero is a
   limit of the anchor, not evidence that FastMCP documents no warnings for these operations
   (DESIGN §10.3 already says so).
2. **Each known alternative has a defect that does not depend on its yield.**
   - **Heading path or `<Card title>`** (`The @tool Decorator`, `@tool Decorator Arguments`):
     - This is a lexical match on display text.
     - The short names are ambiguous in this very release. `fastmcp.tools.tool`,
       `fastmcp.resources.resource` and `fastmcp.prompts.prompt` are public next to the three
       `FastMCP` methods (Interface-checked against `exports` on `8c76bc75…`).
     - Publishing a lexical attribution as `documented` would undo §3.2's rule that exact and
       lexical mentions are never merged, the line Outcome leg 2 already holds (G2).
   - **Code-block call:**
     - It is observably noisy: `mcp.run()` boilerplate anchors 44 passages to
       `TransportMixin.run`.
     - It reaches no `<ParamField>`, because tools.mdx's `@mcp.tool(...)` block leaves `mcp`
       unbound.
     - Its output has already been seen. A noise filter written now would be tuned after seeing
       results, which is the DM-59 shape the pre-registration exists to prevent.
3. **The machinery is not yet anchor-agnostic** (§5, F1). A widening today would publish a false
   template sentence, and nothing would validate it.

**What to do now.**

**a. F1 first.** The anchor becomes a cited `scope` support with a rule. An anchor change is
then a data change that `lctx diff` shows and validation checks.

**b. Pre-register exactly one widening candidate** as a content variant (say
`+receiver-anchor`) in 4.1. It is decided by 4.3's content rule and deleted if it fails
(4.1's exit).
- **Definition:**
  - Inline code in prose, `x.op` or `@x.op`, where every code block of the same document that
    binds `x` binds it to a call Pyrefly types as an instance of a class `C`.
  - `C.op` resolves to a seed, by `member()`'s refusal rule or by Phase 2's `public_paths`.
  - It is a new mention class, never merged with `exact`, with `candidate` modality.
- **Scope:** Limits (warnings) and Controls (ParamField descriptions) only. Outcome leg 2 stays
  on `exact`, so the variant's effect can be attributed.
- **Template:** the text names the spelled anchor, e.g. "in `docs/servers/tools.mdx` § Decorator
  Arguments (which names `@mcp.tool`; `mcp` is a `FastMCP` in this page's examples)". It never
  says "which mentions `FastMCP.tool`". The anchor is cited as `scope` support under F1.
- **Why this one:**
  - Its definition is the current anchor's semantics (the prose names the operation) with one
    more spelling. It is fixed by what the docs say, not by what it yields.
  - Its precondition has only been string-probed: `mcp = FastMCP(` appears in 15 of tools.mdx's
    45 code blocks, 18 of resources.mdx's 23 and 5 of prompts.mdx's 17.
  - Its brief output has not been computed by anyone, and should not be until the 4.3 query set
    is committed.
- **Labels:**
  - "A wider anchor attaches statements to the wrong operation" is **Proposed**, a hypothesis for
    4.3.
  - The name collision behind it is **Interface-checked**.
  - The candidate's yield is unknown.

**c. Record the other two anchors as rejected**, each with a trigger:
- the heading/title anchor, only for names that are unambiguous across the release's public
  paths;
- the code-block anchor, not without an ADR.

Mark the code-block counts as post-hoc (F4).

**d. The 4.3 query set carries positive and negative anchor targets.**
- Write it from the docs and source only. Preferably use the docs-only subagent that wrote the
  D3 config: the author's session and this reviewer have both now seen the corpus's shapes. Seeing
  input docs is legitimate for writing targets, but seeing a candidate's yield is not.
- Examples of the target shape:
  - `FastMCP.tool`'s `timeout` and `version` described from tools.mdx § Decorator Arguments;
  - `ToolAnnotations.title` (the field nested under `annotations`) never describes
    `FastMCP.tool`'s `title`;
  - no field of the `OAuthProxy Parameters` card or the `FastMCP`-constructor cards describes a
    decorator's parameter.

**If the candidate fails 4.3.**
- D45 closes with the exact anchor, and §10.3 says that Controls and Limits have no component
  source on this library.
- Deferred R1-1 then asks whether the Stage F consumers stay.

## 11. Decision and implementation changes

**Decision:**
- Accept D1, A2(a) and B1's first piece.
- **Revise (small)** A3, with F1 and F2 before D45 is decided.
- No §B decision is touched, so no ADR is owed for this review's corrections. D45's variant is
  registered in the ADR-0020 re-registration (4.1).

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: cite the anchor as `scope` support; add `documented-warning-anchored`; tighten the ParamField branch; injected cases | DM-07, DM-46 | Each new case fails validation naming its rule; the ledger has a `TEMPLATE_VERSION` 17 row | test |
| 1 | F2: fixture additions for skip, nearest, nested-real and Card-wrapped fields, and an unanchored warning; relabel §10.3 | DM-54, DM-59 | The three mutants in F2 each change a snapshot | test |
| 2 | §10 (a)–(d): the D45 candidate, the rejections and the anchor targets, registered in 4.1 before any variant output | DM-59 | In `git log`, the registration commit precedes the variant's first packet | `just` recipe order (4.2/4.3) plus the review of the 4.1 commit |
| 2 | F3: the "top-level" sentence, and "root-level heading" for passages | DM-59 | — | test (F2's Card case) |
| 3 | F4: fix DESIGN §9.8, §10.2 and §10.3's probe label | DM-02, DM-59 | — | prose |
| 3 | F5: a component-role declaration shared by Stage F and the rule; name the Mintlify scope | DM-52, DM-04 | One declaration read at both sites | ast-grep rule |
| 3 | F6: an ast-grep rule against `EvidenceRow` literals | DM-60 | The rule test flags a literal outside `findings.rs` | ast-grep rule |

**Deferred.**

| # | Item | Trigger that reopens it |
|---|---|---|
| R1-1 | The Stage F component consumers (warnings, `<ParamField>` descriptions) publish nothing on the pilot | 4.3 rejects every widening candidate: then decide whether to keep them at zero yield (stated in §10.3) or remove them |
| R1-2 | Warnings under `<ResponseField>` would be stated as about the seed. There are none on the pilot: the parents of all 93 warnings are `Step` 12, `ParamField` 2 and none 79 | A `<Warning>` under `<ResponseField>` in a compiled corpus. The plan already defers the `ResponseField` consumers |
| R1-3 | `<ParamField path=…>` (7 on the pilot), `query=` and `header=` are never read | A seed parameter documented only that way |

**Final check.**
- A2(a), B1 and D1: the claims match the evidence.
- A3: the claims match the evidence on the pilot (zero, Measured) and on the fixture paths it
  exercises.
- The negative branches and the anchor are where A3 is unsafe today. They are also exactly what
  D45 would change, so they close first.

## 12. Disposition (author, 2026-09-24)

| Finding | Disposition | Where |
|---|---|---|
| F1 | **Fixed.** Both assertion kinds cite the anchoring exact mention as `scope` Fact evidence (its bytes, citing the mention fact). New rule `semantic:documented-warning-anchored`: the quote is a `<Warning>`'s inner bytes, in a passage whose scope anchor is an exact mention of the brief's seed, and a field-scoped warning's nearest `<ParamField>` names a non-receiver parameter of the seed. The `<ParamField>` branch of `semantic:documented-parameter-cites-its-doc` now requires the anchor and no `<ParamField>` ancestor. `TEMPLATE_VERSION` 17, with a ledger row | `synth.rs` (`anchor`, `Draft::scoped`), `rules.rs` (`anchored_sql`), test `documentation_statements_are_anchored_to_their_seed` (five doctored views, each failing its named rule) |
| F2 | **Fixed.** Appended to `quickstart.mdx`: a `<Warning>` under `force` (skipped), one under `backoff` nested in the parameter `retries` (skipped: the nearest field decides), a field for the new parameter `mode` under `ParamField > Expandable` (describes nothing), a `<Card>`-wrapped field for the new parameter `retries` (describes it), and a `<Warning>` in a passage that mentions no seed (not stated). `stop` gains `mode` and `retries`. Each of the review's three mutants now changes `a_documented_warning_is_a_limit` or the `docs_shapes_fielded_parameters` snapshot. DESIGN §10.3 records the Tested scope | fixture, `syntax.rs` |
| F3 | **Fixed.** §10.3 defines *top-level* as "no `<ParamField>` ancestor; a `<Card>` or `<Expandable>` does not count"; §3.2 says "root-level heading" for passages. F2's `<Card>` case is the test | DESIGN |
| F4 | **Fixed.** §9.8: the whole technique set joins the digest, the selection invocation's id differs per set, `rca` needs `fca`, and `lctx diff` does not compare invocations. §10.2: a `documented_warning` row, and the `parameter` row names the `<ParamField>` source. §10.3: the code-block counts are relabelled a probe, with both counts and no committed query | DESIGN |
| F5 | **Fixed.** `cpg_schema::mdx` declares `WARNING`, `PARAM_FIELD`, `PARAM_NAME` and `TITLE`, and states the Mintlify scope; Stage F and the rules read it. ast-grep rule `mdx-names-declared` (with a rule test) rejects the literals in `crates/cpg-core/src` and `rules.rs` | `mdx.rs`, `rules/mdx-names-declared.yml` |
| F6 | **Fixed.** ast-grep rule `evidence-row-through-new` (with a rule test) rejects an `EvidenceRow { … }` literal outside `cpg-schema/src/findings.rs`. Phase 5's constructors extend it | `rules/evidence-row-through-new.yml` |
| §10 D45 | **Accepted.** The exact anchor stays (deviation log D47). (a) is F1, done. (b) the one widening candidate, a receiver-resolved mention (`+receiver-anchor`, a new mention class, Limits and Controls only, its text naming the spelled form), is registered in the ADR-0020 re-registration (plan 4.1) before any of its output is computed. (c) The heading/title anchor (reopened only for names unambiguous across the release's public paths) and the code-block anchor (reopened only by an ADR) are recorded as rejected there. (d) 4.3's query set carries positive and negative anchor targets, written from the docs and source only | plan 4.1, 4.3 |
| O1–O3 | Noted. O1's `resolve_seeds` cost is Phase 2 step 4's to remove | — |
| R1-1, R1-2, R1-3 | **Deferred**, with the review's triggers | this table (§11) |
