# Design review: ADR-0020, the §9.8 keep rule applied (standard)

**Date:** 2026-09-23 · **Depth:** standard. ADR-0020 changes §B4, so a `standard` review is owed
before acceptance (ADR-0001). · **Mode:** the ADR and its DESIGN amendments (§B4, §9.4, §9.6,
§9.7, §9.8, §12), read against the ablation evidence and the code that makes the new default.
**Target revision:** commit `12bcc46` ("Slice 3.3: lctx diff, budget 20, the live ablation and the
keep rule"). The author committed it while this review was running. The ADR text is byte-identical
to the working-tree draft this review started from. Later uncommitted edits (`synth.rs`,
`codebook.rs`, `findings.rs`, DESIGN §1.4) are out of scope.
**Reviewer:** `design-reviewer` subagent (fresh context). · **Author:** the session executing the
remaining-scope plan.

## Answers to the five questions

1. **Is the rule applied as registered?** Mostly yes, with one measurement defect that changes a
   decision.
   - **Denominators.** ADR-0010's amendment registers which aliases are searched (those of
     touched families) and reports counts. "Hits over all 44 aliases" is the same count, because
     an untouched family cannot be hit (`score_gold.py:91-93`). The scorer prints touched-family
     denominators (14/16 against 13/14; (c) 4/57 against 4/59), so a reader of the printout gets a
     different framing.
   - **Communities' (b)-up/(c)-down trade** is applied correctly under the ADR's all-units
     convention. Under touched-family rates for both (b) and (c), communities lower (b) (0.875
     against 0.929), so they are again not kept. Only a mixed reading (count for (b), touched rate
     for (c)) keeps them. F9 asks for one sentence that states the convention for all three
     metrics.
   - **FCA and kNN.** They are invisible to (b) by construction. The reviewer verified this on
     the served generations, and they are also invisible to (a) and (c). The rule therefore could
     not have kept them, and the ADR says so honestly.
   - **The defect (F1).** The scorer compares access-path strings, not callables. With callable
     identity, `+type-layer` improves hit@5 by 2 and raises (a) and (c), so the rule would adopt
     it, while ADR-0020 says "none is adopted". Communities lose on all three metrics under
     callable identity, so their decision stands, for a different reason.
2. **Is "removed" = off by default, code kept, a faithful reading?** Yes for what the rule
   governs, which is published output.
   - ADR-0011 already made "no community detection" the ablation's baseline, and keeping the
     frozen parameters makes reinstatement test the same technique.
   - This holds only if the retained code has a consumer that can actually run. The revisit
     trigger has no mechanism, and there is no exit condition (F5).
   - Two side effects of the default change: run identity no longer names the technique set
     (F3), and `+rca` is now silently inert (F8).
3. **Are the labels and numbers correct?** All ten rows in the ADR, §9.8 and §12 match
   `build/ablation/*.score.json`, `*.diff.json` and the committed
   `eval/ablation/fastmcp-4.0.5_2026-09-23.json`. The reviewer's reruns of the default and kept
   scores are identical. The label `Measured` fits the numbers, but not the conclusion "none
   improves hit@5" (F1). Two wording errors:
   - "`fastmcp_tasks` … outside the release" is wrong: §1.4 puts `fastmcp-tasks` in the release.
   - "variants cost nothing" carries no label.

   Neither the ADR nor §9.8 cites the committed record (F6).
4. **What does the default lose?**
   - **Breadth.** 18 of 20 briefs are `FastMCP` members (8 of 20 before). The six `Context`
     briefs are gone, with fm.apps, fm.auth and fm.sampling_elicitation.
   - **§1.5.** The ranking check drops from 1 of 2 to 0 of 2, because two of fm.register's own
     operations now outrank the seed. The Consequences omit this (F4).
   - **Important controls.** 29 `structurally_observed` implications go from that section, which
     increment 5 asks about.
   - **The revisit trigger** is not adequate. The increment-5 evaluation has no arm that carries
     the removed content, so it cannot attribute a failure to that content (F5).
5. **Is there a correctness issue in `lctx diff` or the scorer?**
   - **The scorer: yes, and it is decision-changing (F1).**
   - **`lctx diff`: no decision-changing defect.** Its document row is keyed by the
     content-derived `brief_id`. It therefore reports 14 and 20 retrieval documents changed for
     `-fca` and `-knn`, while the served text and vector inputs are identical (F7).

## 1. Decision and scope

**Proposal.** ADR-0020 (`proposed`, `evidence: Measured`) applies §9.8's keep rule to slice 3.3's
ablation:
- communities, FCA and kNN become off by default;
- no off-by-default variant is adopted;
- "removed" means off by default, with the code, tests and frozen parameters kept as variants;
- the revisit trigger is the increment-5 held-out evaluation or a second library.

DESIGN amends §B4, §9.4, §9.6, §9.7, §9.8 and §12. `Techniques` now derives `Default`, so every
technique is off (`analyze.rs:53`).

**Status.**
- The ablation numbers are **Measured** (2026-09-23; budget 20; live Qwen3-Embedding-8B, spec
  `713f6d85…`; `score_gold.py --embedder vllm`).
- The new default is **Implemented** and **Tested**: `lctx diff` shows the pilot-live snapshot
  `1a7244fe` equal to the kept variant `2c6b8407` (reviewer run).

**Observable outcome.** Smaller briefs. No `statistically_derived` assertion in the default:
35 before, 0 now (served `assertions`, `3f1781ff` against `438801c4`). Stage E's analyze step
falls from 5.39 s to 3.15 s (the `default.log` and `kept.log` compile logs).

**Baseline.** The old default had communities, FCA and kNN on; §9.8's rule was pre-registered
(DESIGN since `ff88b75`, and ADR-0010's D19 amendment for (b)).

**Supported scope.** The keep decisions for the pilot, FastMCP 4.0.5, at budget 20. The
held-out check and a second library are out of scope.

### Method and coverage

**Read.**
- The ADR, and DESIGN §B4, §1.2, §1.4, §1.5, §9.4–§9.8, §10.2, §10.3 and §12.
- ADR-0010's amendment, ADR-0011 and ADR-0004's amendment.
- Deviation log D37–D43.
- `scripts/score_gold.py`, `scripts/ranking_check.py` and `crates/cpg-core/src/diff.rs`.
- `Techniques` and the technique gates in `analyze.rs`, the digest path in `attempt.rs`, and the
  new tests.

**Reproduced or measured by the reviewer:**

| Check | Command | Outcome |
|---|---|---|
| Rescore old default | `uv run python scripts/score_gold.py build/generations/3f1781ffc836e686 --embedder vllm` | passed (14/7, 0.0444, 8, 4; identical to recorded) |
| Rescore kept set | same, `7e6580700af404bb` | passed (13/9, 0.043, 7, 5) |
| Recorded rescorings | `cmp default.score.json default.rescore{1,2}.json` | passed (identical) |
| §1.5, old default | `just ranking-check build/generations/3f1781ffc836e686 vllm` | failed (1 of 2: ranks 3, 1), as claimed |
| §1.5, kept set | `just ranking-check build/generations/7e6580700af404bb vllm` | failed (0 of 2: ranks 3, 3), as claimed |
| New default equals kept | `target/release/lctx diff --store build/store --from 2c6b8407… --to 1a7244fe…` | passed ("published output unchanged") |
| Run identity across defaults | `target/release/lctx query --store build/store --snapshot <s> "SELECT … FROM runs"` for `641a3e94`, `1a7244fe`, `2c6b8407` | measured (F3) |
| Diff and variant tests | `INSTA_UPDATE=no cargo nextest run -p cpg-core -E 'test(a_diff_is_a_join_on_content_ids) \| test(a_variant_is_the_default_changed_by_name_and_labelled_canonically) \| test(variants_add_relational_attributes_and_layers)'` | passed (3/3; run twice, before and after `12bcc46`, the second time with the author's unrelated edits in the tree) |
| ADR metadata | `just adr lint` | passed |
| Committed record | a script comparing `eval/ablation/fastmcp-4.0.5_2026-09-23.json` with `build/ablation/*` | passed (every row, snapshot and generation id) |
| Retrieval documents | a scratch script over each generation's `lexical_text` and `vectors.input_hash` | measured: identical for all 20 briefs in `-fca`, `-knn`, `+rca` and `+mention-layer` |
| Hidden hits | a scratch script calling `lctx_mcp.server.search` (live vectors) on the fm.http_asgi aliases | measured (F1) |
| Runtime identity | `FastMCP.http_app is TransportMixin.http_app` in the release environment | `True` |

**Not run.**
- `just check`, `just test-all` and `just pilot-live`. The tree was moving; the commit message
  reports test-all and pilot-live as passed, and that is not verified here.
- The vLLM service was stopped by the author after the reviewer's live runs. Re-checking F1 needs
  `just embed-serve`, and is `blocked` without it.

**Not attacked, so asserted.**
- Determinism of compile-time vectors across compiles. The served inputs being identical where
  documents are identical is consistent with it.
- Whether selection and Related order are otherwise unchanged by the label.
- The ADR-0013 and ADR-0017 trigger checks recorded in D43.

**Side effect.** A `uv run --project libraries/fastmcp` probe created `libraries/fastmcp/.venv`
(gitignored). The reviewer removed it.

## 2–4. Authority, contracts and derivation (compressed)

- **Authority.**
  - §9.8 holds the keep rule; ADR-0010's amendment holds (b)'s registration; ADR-0020 holds the
    default.
  - `Techniques` (`analyze.rs:52-107`) is the single code authority for what runs.
  - DESIGN still carries a second, weaker keep criterion (§1.2 row 3) and a Related section
    described without the default (§10.2, §10.3). See F6.
- **Contracts.**
  - The rule's inputs are the scorer's (a), (b) and (c). The scorer's unit is the access-path
    string (`score_gold.py:37-42, :79, :91-93`), while §12 and the gold speak of operations,
    which are callables. See F1.
  - `lctx diff` compares content ids per table and statements per brief title (`diff.rs:51-67`),
    and reads each snapshot through its own published session (`diff.rs:103-108`; ADR-0017).
- **Derivation.**
  - The default's identity is `config.digest()` when the label is `None` (`attempt.rs:298-305`).
    The label is relative to `Techniques::default()` (`analyze.rs:110-122`).
  - `compiler_digest()` (`attempt.rs:86`) has no technique input. See F3.
  - E0 runs only for `knn` or `knn-layer` (`analyze.rs:676`), as the ADR says. RCA runs whenever
    `rca` is set (`:1540`), but FCA only under `fca` (`:1601`, `:1608`). See F8.

## 5. Journeys

- **Meaningful change: the default itself changed.** Snapshot `641a3e94` (communities, FCA and
  kNN on) and `1a7244fe` (all off) carry the same compiler run: `a02781090d35…`, config
  `f9718291…`. The kept variant `2c6b8407`, which has the new default's exact technique set,
  carries a different run, `d71ed99b…`. Identity is inverted in both directions (F3).
- **Ordinary extension: reinstatement at the revisit.** The operator runs
  `--analytics +communities,+fca,+knn` and gets the old technique set under frozen parameters.
  This works (fixture tests use it: `analysis.rs` `kernels()`). `+rca` alone is accepted but
  inert (F8).
- **Boundary: gold to score.** Gold operation `fastmcp.FastMCP.http_app` and brief member
  `fastmcp.server.mixins.TransportMixin.http_app` are one function object. The string join
  drops it (F1).
- **Failure.** No new publication path; interruption behaviour is unchanged. Not applicable
  beyond G5.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **fail** (documentary) | §1.2 row 3 keeps a technique "only if the §9.8 ablation shows it changes published output", against §9.8's metric clause. §10.2's kinds table and §10.3's Related row describe community, FCA and kNN content as brief structure with no "variant" qualifier. §9.5's consumer is "the order of a Related line". ADR-0011's Decision ("Communities … fill the brief's Related field") has no pointer to ADR-0020 (F6) | F6: sentences, plus an ADR-0011 amendment line |
| G2 Semantic fidelity | **pass** | The default publishes no `statistically_derived` assertion (35 → 0, served `assertions`), so ADDENDUM Q6 is vacuous in the default. Variants keep their statuses. Related is not a slot section, so the §B11 `absent_slots` metric is unaffected (manifests: `applicable_case` 20 in both) | — |
| G3 Validity | **pass** | The new default published through validation. `lctx diff` shows it equal to the kept variant (reviewer run). Parse refuses community layers without communities (Tested) | F8's refusal belongs here too |
| G4 Hidden behaviour | **pass** (conditional) | No margin was added after the fact (Option 2's reasoning is right). The budget was set at the registered midpoint before any output at 20, with the gold seen disclosed (ADR-0004 amendment). `lctx diff` is read-only | F1's matching rule and F2's round procedure must be registered, with the direction the review found disclosed, **before** rescoring (ADDENDUM Q15) |
| G5 Consistency and recovery | **pass** | One pinned session per snapshot (`diff.rs:103-108`); no new write path | — |
| G6 Transformation and reuse | **fail** | Measured run-id collision across two technique sets. `content_digest` has no technique input beyond the embedded-keys digest, so a compile without an embedder collides too, against §9.8's own oracle (F3) | F3 |
| G7 Truthful capability claims | **unresolved** | "None improves hit@5 … None is adopted" holds only under string equality of paths. Under callable identity, `+type-layer` passes the rule (F1). The Consequences omit the §1.5 regression (F4). `+rca` is accepted and does nothing (F8) | The author decides the matching rule (F1) and re-applies the rule |

## 7. Findings

Ordered by severity: the decision's evidence first (F1, F2), then identity (F3), then the omitted
consequence and the trigger (F4, F5), then spine, tooling and wording (F6–F9).

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **(a) and (b) join gold operations to brief access-path strings, not to callables. On the pilot this hides a hit that decides `+type-layer`, and (a) counts path spellings** | DM-11, DM-24, DM-59 · G7 | `score_gold.py:37-42` builds each brief's set from `brief_members.access_path`. `:79` takes Jaccard over those strings; `:91-93` touch and relevance are `p & ops`. fm.http_asgi's operations are `fastmcp.FastMCP.http_app`, `…create_streamable_http_app` and `starlette…Starlette`. The brief `fastmcp.server.mixins.TransportMixin.http_app`, whose members are its two declaring-class spellings, is in `-communities` (`288490f9`), `+type-layer` (`4b9a2ecd`) and the kept set (`7e658070`). `FastMCP` inherits it, and `FastMCP.http_app is TransportMixin.http_app` is `True`. **Live search:** this brief ranks 1st for both fm.http_asgi aliases in all three generations. (c) already credits it: the kept set's one extra recalled span is fm.http_asgi's `mixins/transport.py` 372–412, the `def` itself. **(a):** the seed `FastMCP.tool` has 3 spellings, so its fm.register Jaccard is 1/8, while `Tool.from_function`'s 2 spellings give 1/7. The kept set's "higher" fm.register (0.1429 against 0.125) and fm.mount_proxy (0.2 against 0.1667) are spelling counts. No other brief in any variant matches a gold operation only by name (reviewer scan) | **Under callable identity** (Measured by the reviewer, 2026-09-23; (b) is the recorded count plus the two searched aliases): the old default is 14/7, (a) 0.0584, (c) 4. `-communities` and the kept set are 15/11, 0.0713, 5. `+type-layer` is 16/10, 0.0826, 6. So `+type-layer` improves (b) without lowering (a) or (c), and the registered rule adopts it, while ADR-0020 says none is adopted. Communities lose on all three metrics: the decision stands, but the "(b)-up/(c)-down trade" disappears. "Touches one gold family fewer" becomes "the same number". The keep set is undecided, because `+type-layer` needs communities (F2) | Decide the matching rule. Resolve gold operations and brief members to declaration nodes; the in-progress §1.4 measurement already joins the gold's operations to the public-callables relation. The rationale is independent of the gold (two spellings, one object). Record it as an ADR-0010 amendment that says the review found it and which decision it moves. Rescore the ten existing generations (no recompile) and re-apply the rule. Alternatively, keep string equality and state in ADR-0020 that `+type-layer`'s rejection rests on it | **None exists.** Add a test: a pytest case over the scorer's matcher, with a constructed generation whose brief member is inherited through a public subclass named by a gold operation |
| **F2** | **The rule's order of application and dependent techniques are unspecified** (latent now; binding once F1 is fixed) | DM-59 · G7 | Every variant was judged one at a time against the old default, and the removals were then applied jointly. `+type-layer` was measured only with communities on, and `Techniques::parse` refuses it without them (`analyze.rs:104-106`). Neither §9.8 nor ADR-0020 says what happens when a technique passes and its prerequisite fails | Under F1, the rule adopts a layer while removing its prerequisite. Against the new default, `+type-layer`'s generation beats the kept set: (b) 16 against 15, (a) 0.0826 against 0.0713, (c) 6 against 5. This is inferred: that generation also had FCA and kNN on, which changed neither selection nor any metric here. So `+communities,+type-layer` plausibly passes, and the adopted default could be communities over the type layer | Pre-register before rerunning, independent of the gold: apply the rule in rounds from the adopted default until no single change, or declared bundle (a technique with its prerequisites), passes. Then compile and score the bundle directly | None mechanical. The committed ablation record gains a `round` field (a record, not a test) |
| **F3** | **Run identity does not name the technique set, because the variant label is relative to a default that ADR-0020 changes** | DM-12, DM-32, DM-48 · G6 | `analyze.rs:110-122`: the label holds only the changes from `Techniques::default()`. `attempt.rs:298-305`: when the label is `None`, the digest is `analysis.config.digest()`. `compiler_digest()` (`attempt.rs:86`) has no technique input. **Measured:** runs in `641a3e94` (communities, FCA, kNN on) and `1a7244fe` (all off) share compiler run `a02781090d35143eec76f30420913548`, config `f9718291…`. `2c6b8407`, with `1a7244fe`'s exact technique set, has run `d71ed99b…`. The `content_digest`s differ only through Stage E's embedded-keys digest (`attempt.rs:166-177`) | Two executions with different techniques share an execution identity (§B6 provenance cites that run). A compile without an embedder of the old and new default would share `content_digest` with different findings, contradicting §9.8's oracle "reruns with the same `content_digest` give identical output" and ADR-0020's "a snapshot always says which techniques made it". A label recorded before ADR-0020 (e.g. `-knn`) now parses to a different set | Digest the **absolute** on-set, every flag with the default included, and write the label as absolute. A few lines in `attempt.rs` and `label()`. Say it in §9.8's variants paragraph | **None exists.** Add a test: the config digest is injective over all 2⁸ flag sets and never equals the bare `config.digest()`. It fails today for the default |
| **F4** | **The new default fails the §1.5 ranking check worse, and the Consequences omit it; the cause is sibling briefs from the seed's own family** | DM-59 · G7 | Reviewer runs: the old default ranks `FastMCP.tool` 3rd and 1st (1 of 2); the kept set ranks it 3rd and 3rd (0 of 2). For "decorate callable imperative add tool…", `fastmcp.tools.Tool.from_function` and `fastmcp.FastMCP.add_tool`, both fm.register operations, rank above it. The ADR's Context says "The keep rule does not read it." The Consequences are silent | A definition-of-done item (§1.5) gets worse because of this decision, and no one is named to resolve it. With concentrated selection, a seed-first criterion cannot pass while selection adds siblings from the seed's family | Add it to the Consequences with its cause. Name the owner: an ADR-0010 amendment restating §1.5 for 15–25 briefs (e.g. "a brief of the seed's family first", on a rationale independent of the gold), or a selection rule, decided at the increment-3 deep review | `just ranking-check <generation> vllm` (exists; recipe tier) |
| **F5** | **The revisit trigger has no mechanism, and the retained code has no exit** | DM-58, DM-59 · — | Revisit: "the increment-5 held-out evaluation … attributes an agent failure to a removed technique's missing content". §12's agent evaluation compares "raw-evidence retrieval … against compiled briefs"; there is no arm with the removed content. **Removed** (served assertions, `3f1781ff` → `438801c4`): 29 `implication` (Important controls, `structurally_observed`), 14 `shared_signature`, 15 `related`, 20 `doc_link` → 0. The second clause (a second library) has no scheduled library | A failure caused by a missing implication looks like any other failure, so the trigger stays silent. The kernels, their fixture tests (`kernels()`), §9.4–§9.7, leiden-rs and the `rand` pin then stay with no condition for deletion. That is DM-58's retained-machinery case, and "cost nothing when off" is an unlabelled benefit claim | Add an old-default arm (`--analytics +communities,+fca,+knn`, one compile) to the increment-5 plan, or re-run failing tasks against it, and name the measure. Add an exit: no attributable benefit at increment 5 and no second-library reversal → delete by superseding ADR | **None exists** until the increment-5 harness has that arm. Say so in the plan |
| F6 | **The spine disagrees with itself, and three statements are wrong or uncited** | DM-02, DM-59 · G1 | (a) §1.2 row 3 has the output-only keep criterion; §9.8 has output **and** metric. (b) §10.2's kinds table (`related`, `implication`, `doc_link`, `shared_signature`) and §10.3's Related row give no default qualifier. (c) §9.5's consumer includes "the order of a Related line". (d) ADR-0011's Decision has no ADR-0020 pointer. (e) §12 and D42: "19 of the gold's 125 operations are outside the release (… `fastmcp_tasks` …)". §1.4 puts `fastmcp-tasks` in the release; its 5 operations are outside `public_roots = ["fastmcp"]`. (f) Option 4's "variants cost nothing when off" has no label: runtime is measured (analyze 5.39 → 3.15 s), maintenance is not zero. (g) Neither ADR-0020 nor §9.8 cites `eval/ablation/fastmcp-4.0.5_2026-09-23.json`, the only durable copy of the per-variant snapshot and generation ids (`build/` is gitignored) | An implementer following §10.3 builds Related into every default brief. A reader applying §1.2 keeps every technique. A reader of §12 concludes the tasks extra is not analysed | Sentences. ADR-0020's `design:` gains §1.2, §9.5, §10.2 and §10.3. One amendment line in ADR-0011. "Outside the configured public root". Cite the record in the ADR and §9.8 | Prose; no mechanical oracle |
| F7 | **`lctx diff` keys retrieval documents by the content-derived `brief_id`, so it cannot answer whether a technique reached the document** | DM-49, DM-53 · — | `diff.rs:56-60`: `encode(brief_id) \|\| chunk \|\| sha256(text)`. `mfca.diff.json` reports 14/14 documents one-sided and `mknn.diff.json` 20/20. The served `lexical_text` and `vectors.input_hash` are identical for all 20 briefs in `8b39be02` and `86f94ee4` against `3f1781ff` (reviewer comparison) | §9.8's named oracle reports that FCA and kNN change 14 and 20 retrieval documents: the opposite of what ADR-0020 argues and what the generations show. The (b)-visibility question cannot be read off the diff. No decision changes | Key document rows by the brief's seed, chunk and text hash | Test: `a_diff_is_a_join_on_content_ids` asserts no one-sided `brief_documents` rows for `-fca` (its snapshot shows 1/1 today) |
| F8 | **`+rca` is accepted without `fca`, and under the new default it does nothing** | DM-43 · G7 | `parse` refuses only community layers without communities (`analyze.rs:104-106`). RCA runs whenever `rca` is set (`:1540`), but FCA, RCA's only consumer, runs only under `fca` (`:1601`, `:1608`). The dependency held before ADR-0020 because `fca` was on | `lctx compile --analytics +rca` publishes the default's output under its own label and digest. A second library's `+rca` ablation would read "changes no published output" and fire ADR-0011's trigger falsely | Refuse `+rca` without `fca`, beside the layer check, and state it in §9.8 | Test: `a_variant_is_the_default_changed_by_name_and_labelled_canonically` asserts `parse("+rca")` is an error |
| F9 | **(b) has a stated convention, but (c) does not, and the decision on communities depends on using one convention for both** | DM-59 · — | The ADR: "Hits are counted over all 44 gold aliases". (c) is shown as a count out of 157 without saying so. Scorer output: (b) 14/16 against 13/14, (c) 4/57 against 4/59 | All-units counts give "(b) up, (c) down": not kept. Touched rates for both give "(b) down": not kept. Counting (b) but taking (c) as a touched rate gives communities raising (a), (b) and (c): kept. A reader of the printout reverses the ADR's framing | One sentence: all three metrics over all gold units (22 families, 44 aliases, 157 spans), and the decision is invariant under either consistent convention. It is moot under F1, where communities lose on every reading | Prose; the committed record already carries the units |

**Observations** (recorded so that silence is not read as clean):
- **O1.** `lctx diff` joins briefs by title (`diff.rs:64-67`). Titles are unique on the pilot
  (20/20, reviewer check), but uniqueness is not enforced: a duplicate title would merge two
  briefs' statements.
- **O2.** `changes_published_output` (`diff.rs:171-175`) reads briefs and statements only, so an
  evidence-only change reads "unchanged".
- **O3.** The fixture test compares assertion kinds as the strings "13" and "15", a second
  spelling of the codebook. Append-only codebooks keep it stable.
- **O4.** `compiler_digest()` still folds the off-by-default techniques' parameter and relation
  digests, so editing an unused kernel's frozen parameters changes the default's content digest.
  This over-keys (DM-33), which is harmless, and it also keeps those parameters under the freeze.
- **O5.** `unreachable_operations` counts distinct operations against an occurrence total
  (`score_gold.py:82-84, :147`). Here 19 = 19, because none repeats.
- **O6.** Breadth, measured: 18 of the kept set's 20 briefs are `FastMCP` members (15 declared on
  it; `TransportMixin.http_app`, `TransportMixin.run` and `Provider.disable` inherited), against
  8 of 20 before. The six `Context` briefs (`elicit`, `info`, `input_responses`,
  `report_progress`, `request_context`, `request_id`) are gone. The ADR names the concentration
  but not what it drops.

**Strengths, stated as what would break without them.**
- **The committed ablation record.** Without it, once `build/` is cleaned nothing could tie the
  table to snapshots. The reviewer matched every row.
- **No margin after the fact.** A margin chosen after seeing a one-unit trade would have kept
  communities: Option 2's reasoning is correct.
- **Frozen parameters kept.** Without them, reinstatement at the revisit would test a different
  technique.
- **A default with no statistical output.** Without it, ADDENDUM Q6 would still need policing in
  every default brief.

**Applicability.**
- **Bore on this scope:**
  - Group 12 (DM-58, DM-59: the rule, retention and claims);
  - Group 10 (DM-48, DM-49: identity and diff legibility);
  - Group 3 (DM-11, DM-12: path against callable, run identity);
  - Group 7 (DM-32: the reuse key);
  - Group 9 (DM-43: variant capability);
  - Group 1 (DM-02: spine).
- **Did not bear:**
  - Groups 2, 4 and 11 beyond DM-53: no schema, codebook or declaration change. The diff test's
    snapshot is not a schema contract.
  - Groups 6 and 8: no new effects or execution mechanism. The runtime saving is a measured side
    note.

## 8. Alternatives

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| Baseline: old default (communities, FCA, kNN on) | Related, implications and doc links in every brief | 35 statistical assertions per pilot generation to police; the §1.5 check at 1 of 2 | Leiden 40 runs, FCA, E0 | analyze 5.39 s | Fails the registered rule for communities under every convention and matching tried |
| Proposed: ADR-0020 default | One flag set; kernels kept as variants | F1 leaves the adoption set undecided; F3 identity; F8; breadth concentrated on one class; §1.5 at 0 of 2 | Kernels still maintained and tested | analyze 3.15 s | Holds for FCA and kNN. "None adopted" is not yet established |
| **Simpler viable alternative:** no Leiden; a structural diversity cap in selection (at most ⌈budget/3⌉ seeds per declaring class or public owner) | Replaces the one job communities did for the default (diversity) with a structural rule; no statistical status | Must be pre-registered and pass the same rule; not measured. Addresses O6 and plausibly F4's sibling crowding | One selection rule; no dependency | Not measured | Not proposed by the ADR, which leaves "a later technique that restores diversity" open. It is the cheapest candidate for F2's next round. Deleting the kernels (Option 4) is simpler still in code, but loses the revisit path; while F1 may reinstate communities, keeping them is justified |

**Abstractions justified by current needs.** The variants mechanism has a consumer (the ablation,
the revisit), provided F5 gives the revisit a mechanism. **Ordinary code.** The keep rule is
arithmetic over three numbers and should stay that way; no registry of techniques is needed
beyond `Techniques`.

## 9. Verification and measurement plan

| Claim or risk | Label | Check | Conditions and expected result | Current result or gap |
|---|---|---|---|---|
| The ablation table's numbers | Measured | `score_gold.py --embedder vllm` per generation; the committed record | Budget 20, live Qwen3-8B, 2026-09-23 | Reproduced for the default and kept set; all rows match the record |
| "None improves hit@5" | Proposed (contested) | Rescore under callable identity (F1) | `+type-layer` 16 against 14 | Gap: the matching rule is undecided |
| The new default equals the kept variant | Tested | `lctx diff` `2c6b8407` → `1a7244fe` | "unchanged" | passed |
| FCA and kNN never reach the retrieval document | Measured (reviewer) | Served `lexical_text` and `vectors.input_hash` compared | Identical for 20 of 20 briefs | Holds; `lctx diff` misreports it (F7) |
| A snapshot names its technique set | Proposed (false today) | F3's injectivity test; `runs` query | Distinct run per set | failed by measurement (shared run `a0278109…`) |
| `+rca` requires `fca` | Proposed | F8's parse test | parse error | Gap |
| The revisit can observe removed content | Proposed | An increment-5 old-default arm | A per-task comparison | Gap (F5) |
| §1.5 on the new default | Measured | `just ranking-check <gen> vllm` | Exit 0 | failed (0 of 2) |

**Cost accounting.** F1's correction is a scorer change plus rescoring ten existing generations,
with no recompile. F2's round is 2 or 3 compiles of about 45 s each, plus scoring. F3 is a few
lines and a unit test.

## 10. Exceptions and deferred items

No SHOULD-level exception is claimed.

### Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| O1: duplicate brief titles in `lctx diff` | Titles unique on the pilot | A second library, or a template change to titles |
| O2: evidence-only changes read "unchanged" | No technique in this ablation changes evidence alone | A technique whose ablation changes only evidence or findings |
| O3: codebook integers in the diff test | Append-only codebooks | `diff.rs` next edited (render kind names) |
| O4: off-by-default parameters key the default | Harmless over-keying; keeps the freeze meaningful | F5's exit deletes a kernel |
| O5: distinct against occurrence count | 19 = 19 on this gold | A gold where an out-of-root operation repeats |

## 11. Decision and implementation changes

**Decision: Revise.**

**What stands:**
- FCA and kNN off by default. They are invisible to all three metrics, so the rule decides them
  by construction, and the reviewer verified it.
- Communities alone not kept, under every convention and matching rule tried.
- No margin after the fact.
- "Removed = off by default, code kept" as the reading of "removed by ADR", with F5's exit
  added.

**What blocks acceptance:**
- **G7 is unresolved.** The adoption half of the decision rests on a scorer that compares path
  strings. Corrected to callables, `+type-layer` passes the registered rule, and the adopted
  default may become communities over the type layer (F1, F2).
- **G6 fails.** The default change leaves one run identity naming two technique sets (F3).

This is not "small": fixing F1 may change what the default is.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1** | F1: decide the matching rule (callables via declaration nodes), register it with its gold-independent rationale and the review's finding disclosed, rescore the ten generations, re-apply the rule | DM-11, DM-24, DM-59 | A rescored table in the ablation record; ADR-0020's keep and adopt lines follow from it | pytest over the scorer's matcher (inherited-member case) |
| **P1** | F2: register the round procedure (bundles for dependent techniques) before rerunning; if F1 moves `+type-layer`, compile and score `+communities,+type-layer` against the new default | DM-59 | Round 2 in the ablation record | The record's `round` field |
| **P1** | F3: the absolute technique set in the config digest; the label absolute | DM-12, DM-32, DM-48 | `runs` differ between `641a3e94`-like and `1a7244fe`-like compiles | Unit test: the digest is injective over the flag sets and never the bare config digest |
| P2 | F4: the §1.5 regression and its cause in the Consequences; name the owner of §1.5's restatement | DM-59 | ADR text | `just ranking-check` |
| P2 | F5: an old-default arm in the increment-5 plan; a deletion condition in the ADR | DM-58, DM-59 | Plan and ADR text | The increment-5 harness |
| P2 | F8: refuse `+rca` without `fca` | DM-43 | parse error | Unit test |
| P3 | F6: spine sentences (§1.2, §9.5, §10.2, §10.3); ADR-0011 pointer; "outside the public root"; label "cost nothing"; cite the ablation record | DM-02, DM-59 | Text | Prose |
| P3 | F7: document rows keyed by seed, chunk and text hash | DM-49, DM-53 | `-fca` shows 0/0 documents on the fixture | The diff test's assertion and snapshot |
| P3 | F9: state the metric convention for all three metrics and the invariance | DM-59 | Text | Prose |

**Final check.**
- The numbers match the evidence.
- The keep and remove claims match it for FCA, kNN and communities.
- The adopt claim does not yet match it (F1).
- The identity claim "a snapshot always says which techniques made it" does not hold (F3).
- The revisit path is not yet a validated route back (F5).
