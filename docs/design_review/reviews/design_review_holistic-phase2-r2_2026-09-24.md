# Design review: holistic-assessment plan, Phase 2 (R2): `public_paths`, bundle `FORMAT` 2, gold matcher version 2 (compact)

## 1. Decision and scope

**Decision: Revise (small). No rescore until F1 and F2 are closed.**
- **Accepted:**
  - the pre-registration and its (a)/(b)/(c)/units rules as implemented in `score_gold.py`;
  - the typed-query foundation;
  - `public_paths` as the one public-path authority;
  - `brief_members`/`own`;
  - A2(c)–(e);
  - A7;
  - the tokenizer equivalence.
- **Blocking the rescore (G7 fails, narrowly):**
  - **F1.** `FORMAT` 2 changed which names feed `lexical_text`, and no registration covers that
    change. The amendment says "Nothing else about retrieval changes", but on the pilot the word
    `proxy` is now in all 20 briefs' lexical text, where it was in 3. The lexical leg therefore
    drops it from every query.
  - **F2.** The "one matcher" is only partly shared. `ranking_check.py` does not apply the
    degraded→`blocked` rule. It also resolves paths through a second function with the opposite
    tie-break.
- **Also to close:**
  - F3 is a G3 fail, narrow: "one path, one node" is assumed by four consumers but not validated.
  - F4 concerns two DESIGN claims about `public_paths`' fidelity to `member()` that are broader
    than what is pinned.
  - F5 (D48) is accepted as Deferred.

**The five questions, answered** (details in §7):
1. **Matcher against registration.**
   - No registered rule changed after `28443a0`:
     - `docs/adr/0010-…md` has no later commit;
     - DESIGN §12 only gained the *Implemented* paragraph (`377d7fe`).
   - `gold_match.resolve`/`jaccard`/`hits` and `score_gold.py` match the registered rules clause
     by clause (§3).
   - Where the code diverges:
     - ranking_check's blocked rule and resolver (F2);
     - two derived quantities in `score_gold.py`: the root and the (c) denominator (F2; no effect
       on the pilot);
     - the lexical name set, which the registration never fixed (F1).
2. **`member()` fidelity.** The seed rank, the three refusals and the stub pick are faithful at
   one level of membership. The ORDER BY terms and predicates are the same (§3). The domain
   differs in two unstated ways:
   - nested members are no longer rows;
   - an **undefined** base fails open, as it always did.

   DESIGN claims more than either (F4). D48 is acceptable for the rescore: no gold operation,
   seed or selected brief names either property with a setter. It goes to Deferred (F5).
3. **Inherited spellings in lexical text:** an **unregistered retrieval change** (F1). DESIGN §6.4
   records it, but only after the fact, in the implementing commit `d413430`.
4. **Tokenizer:** yes, at the pinned versions.
   - An exhaustive probe over all 1,112,064 Unicode scalar values found 0 differences between
     Rust 1.98.1 and Python 3.14.7 (Unicode 16.0.0).
   - Exactly two non-ASCII code points lowercase to an ASCII token: U+0130 and U+212A. Both are in
     `tokens.json`.
   - A Unicode-table upgrade is not caught (§9, optional oracle).
5. **Still needed before the Phase 4 rescore can be trusted:** §11's checklist.

**Proposal (commits in scope, on `main`).**
- `28443a0`: the ADR-0010 amendment (matcher version 2), DESIGN §12, and an ADR-0019 amendment
  line.
- `7286ca0`: `ArrowColumn::read`, `query_row!`, `relations!`, `sql::fetch`/`Params`.
- `f2f88b9`: `public_paths` (relation plus analysis table), three rules, `public_shapes`, and the
  hand-built unresolved-base session.
- `eacdcba`: consumers moved to the relation; `member()` and `public_callables_sql` deleted.
- `367421e`: `brief_members` are the seed's public paths, with `own`;
  `semantic:brief-member-public`.
- `76a0f8b` and `195eef8`: `LayerSpec`, `UnfollowedReason`, the build-script source digest.
- `d413430`: bundle `FORMAT` 2 in Rust and Python; `tokens.json`.
- `47578d4`: A7 (the pydantic strict parse, `responses.json`, `CapabilityError` as
  `ValidationError`).
- `377d7fe`: `gold_match.py`, both scorers, `just score`.
- `8c6b25b` (R1's fixes) lies between these commits and is **out of scope**.

**Status:** Implemented. §9 audits the Tested scope.
**Reviewer / author:** design-reviewer subagent (fresh context) / the Phase 2 session.
**Affected revisions:**
- Reviewed at `377d7fe`.
- During the review `main` moved to `aec6827` (`c9a4194`, `0e98d85`, `aec6827`: Phase 3 and
  Phase 4 prerequisites, not reviewed). None of them touches `scripts/`, `public.rs`, the lexical
  build or ADR-0010, so F1–F4 stand at HEAD.
- Pilot snapshot `ec03626f63304c3c156dd87c727a9ead`, generation `f0e8646632471e64` (`FORMAT` 2).
  The `FORMAT` 1 comparison is generation `3039ddc7c30266a5` (snapshot `b3f91e11`, step 5).

**Observable outcome.**
- One relation names every public spelling.
- Seeds resolve by lookup: 0.89 s → 0.02 s, the author's measurement.
- `fastmcp.FastMCP.http_app` promotes `TransportMixin.http_app`'s brief. Verified in the served
  `symbol_map`, together with `FastMCP.run` and `…proxy.FastMCPProxy.tool`.
- The gold is matched by declaration node.

### Method and coverage

**Read, at `377d7fe`:**
- all ten commits' diffs;
- `public.rs` in full, against `member()`/`resolve_seeds` at `eacdcba^:crates/cpg-core/src/analyze.rs:299-478`;
- `analyze.rs:289-333`;
- `attempt.rs:80-140` and `:450-520`;
- `bundle.rs` (queries `:89-160`, `lexical` `:290-331`);
- `cpg_schema::bundle::{tokens, name_tokens}` (`:227`, `:238`);
- `query.rs`, `sql.rs`, `delta::to_schema`, `build.rs`;
- the public-path and brief-member rules (`rules.rs:614-658`);
- `derived.rs` `AncestryTargets`;
- `gold_match.py`, `score_gold.py`, `ranking_check.py` and their tests;
- `retrieval.py:1-60`, `generation.py`, `embedder.py`;
- `server.py`'s `FORMAT` 2 hunk;
- ADR-0010 L157–197;
- DESIGN §6.4, §9's opening, §9.1, §11.2 and §12;
- deviation rows D46–D52;
- the `public_shapes` fixture and snapshot;
- `tests/public.rs`;
- `public_paths_agree_with_the_seed_resolution`.

**Run** (outcomes in §9):
- `just test-all` twice;
- a standalone exhaustive tokenizer probe;
- read-only `lctx query` on `ec03626f`;
- read-only analysis of the served Arrow files of `f0e86466` and `3039ddc7`;
- a mapping check of the gold's 157 spans against `build/envs` (a file check, not a score).

**Not run (instructed):** `score_gold.py`, `ranking_check.py`, `just pilot`.

**Asserted, not re-verified:**
- step 4's "`lctx diff` 5dc48895 → 3c2be6da, published output unchanged";
- step 5's "every served file byte-identical but `assertions.arrow`" (I found neither full snapshot
  id in a log);
- the pilot smoke 20/20;
- the −32602 wire code (read the test, did not trace FastMCP).

**Guarantees not attacked:**
- ADR-0019's config-scoped publication under an interrupted attempt;
- A2(c)'s kNN lineage in `extra_digest` (read the commit summary and the enum only);
- the `rerun-if-changed` coverage of `build.rs` for newly added files.

**Out of scope:** Phase 3/4 commits after `377d7fe`, and R1's fixes.

## 2. Authority and lifecycle (prose)

`cpg_schema::public::public_paths` is the single authority for "which public spellings name which
node". It is a config-scoped relation (`$roots` from the analytics config). It is persisted as the
analysis table `public_paths` before Stage E (`attempt.rs:457-475`) and published with the
analysis group (ADR-0019 amendment). Its SQL joins `compiler_digest` (`attempt.rs:100-102`).

Every consumer derives from it:
- seed resolution (`analyze.rs:295`, a lookup);
- the preferred name (`public.rs:142`);
- `brief_members`. `semantic:brief-member-public` checks both directions: every member is a seed
  row, and every seed row is a member.
- `symbol_map` and the served `public_paths` (`bundle.rs:137-150`);
- the gold matcher (`gold_match.resolve`).

D49's mention lookup stays on `exports`. It answers a different question: which seed an export
node that a mention targets names. That includes private-origin paths, which `public_paths`
excludes by policy, so it is not a second authority.

**Authority gap:**
- The invariant "one path names one node", which D48 calls A1's rule, has no enforcement point
  (F3).
- The **lexical name set** is derived from `brief_members` by a join in `bundle.rs:296`. No
  registration fixes what it is (F1).

## 3. Contracts checked against the registration and `member()`

**Matcher, clause by clause** (ADR-0010 L164–197 → code):
- **Exact path equality, no fallback:** `gold_match.py:37,44`.
- **Node set = resolved nodes:** `:46`.
- **Class rule:** by construction. A class path resolves to the class node.
- **Unresolved listed apart, in two groups, kept in the union as strings:** `:47-51`, `:32`. The
  gold has no duplicate operation within a family (checked), so the tuples equal the registered
  set.
- **(a), best over briefs of |F∩{seed}|/|F∪{seed}|, mean over 22:** `:55-60`,
  `score_gold.py:75-77,157`.
- **(b), every alias, `limit=5`, hit = seed ∈ node set:** `score_gold.py:86-113`.
- **(c):** code unchanged.
- **`matcher_version`, and each alias's mode:** `:147`, `:98-106`.
- **Degraded under `vllm` → `blocked`:** `:43-45,212-213`.
- **Divergences:** F1 and F2.

**`public_paths` against the deleted `member()`:**
- **Container pick:** `ORDER BY s.is_stub, e.declaration_node_id` (`public.rs:48-49`) equals
  `min_by_key(is_stub)` over the same order.
- **Seed rank:** `CASE is_overload … + CASE provider_node_map … DESC, start_byte DESC, node_id`
  (`:81-83`) is the old `decl_sql`'s order.
- **Nearest definer:** `min(ordinal)` (`:86-88`), which equals the first chain entry with a def.
  The MRO excludes the class itself (`tables.rs:434`), so ordinal −1 is the class.
- **Refusals:**
  - `blocked` (`:68-72`) is a null target or a non-release-class entry, before the definer;
  - `rebound` (`:73-77`) is a binding other than `def`/`class`/annotation, at or before the
    definer.

  Both are the old predicates.
- `ancestry_targets` is keyed `[snapshot_id, ancestry_fact_id]` (`derived.rs:677`), so an ordinal
  cannot have two candidate targets. The old tie-break order is unreachable.
- **Domain differences:**
  - one member level only (F4b);
  - roots and private segments filtered (a stated policy);
  - one refusal message (D48, stated).

## 5. One journey: a brief's lexical text from `FORMAT` 1 to `FORMAT` 2

The registered baseline is `FORMAT` 1. There a brief's names part was the words of its
`public_alias` members, the seed's spellings through its own container (`format_1_members`,
removed in `d413430`). `FORMAT` 2's `lexical()` instead left-joins **all** `brief_members`: own
spellings and those inherited through every public subclass (`bundle.rs:292-319`).

Measured on the pilot (2026-09-24; served files of `3039ddc7` against `f0e86466`, tokenized by
`lctx_mcp.retrieval.tokenize`):
- The document bodies are identical in 20 of 20 briefs. So the vectors are unaffected, and the
  plan's "no re-embedding" holds.
- The names part gains **131** new (brief, token) pairs over the 20 briefs:
  - `proxy` and `fastmcpproxy`, through `fastmcp.server.providers.proxy.FastMCPProxy.*`, reach 19
    briefs;
  - `Provider.disable`'s names go from 6 to **62** distinct tokens (50 inherited spellings:
    `file`, `upload`, `approval`, `form`, …);
  - `Tool.from_function` gains 16.
- Document frequency over the 20 lexical texts: `proxy` 3 → **20**, `mcp` 16 → 19, `fast`
  15 → 19. `Lexical.discriminating` keeps only words with `0 < df < size` (`retrieval.py:46-48`),
  so **`proxy` no longer counts in any query**.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **Pass** | §2: one relation; `member()` and `public_callables_sql` deleted; `brief-member-public` holds both directions. `query_row!` reads through the same `ArrowColumn` mapping as writes; D46 avoids a second mapping | — |
| G2 Semantic fidelity | **Pass (scoped)** | Tokenizer exhaustively equal (§9). D48's setter-as-property-node is a recorded, scoped deviation with no consumer on the pilot (F5) | Deferred row (§11) |
| G3 Validity | **Fail (narrow)** | Three consumers read a path as naming one node, and `symbol_map` promotes whatever it names. Nothing rejects two nodes under one path, although the relation can produce them (F3). Elsewhere validity holds: A8's null and code rejection (`a_null_or_an_unknown_code_is_an_error`), bound parameters (`parameters_are_bound_never_spliced`), A7's shared corpus in both suites | F3's rule |
| G4 Hidden behavior | **Pass** | No compiler code reads `eval/gold` or `.claude/skills`; the only hits are the analytics-freeze digests in tests (`git grep`). A2(e) removes the "code changed, no version bump" hidden input. D52 is disclosed (but see O4) | — |
| G5 Consistency and recovery | **Pass** | `public_paths` is written in the attempt before Stage E, validated by three rules, and published by the one `snapshots` append. The Python loader rejects a manifest whose `format ≠ 2` (`generation.py:187`). Serving schema digests are regenerated | — |
| G6 Transformation and reuse | **Pass** | The relation SQL and the source digest join `compiler_digest`, and the roots are in the config digest. The vector cache copied into the fresh store (D50) is keyed by `(spec_hash, input_hash)`, over unchanged bodies (§5) | — |
| G7 Truthful capability claims | **Fail (narrow)** | F1: ADR-0010 L196, "Nothing else about retrieval changes", is contradicted. F2: DESIGN §12 says the matcher is "shared by `score_gold.py` and `ranking_check.py`" and that a degraded live run is `blocked`, which is true of `score_gold.py` only. F4: the refusal and nested-class claims | F1, F2, F4 |

## 7. Findings

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | `FORMAT` 2's lexical text is built from **every** public spelling of the seed. That is a retrieval change the pre-registration does not name, and its closing sentence denies it | DM-59, DM-49 · G7; ADDENDUM Q15 | **Code:** `bundle.rs:296` left-joins all `brief_members` (own and inherited) into `name_tokens`. **Registration:** ADR-0010 L186–197 registers only "each distinct name token once" and promotion by any spelling, then "Nothing else about retrieval changes". **After the fact:** DESIGN §6.4 L1661 records the set in `d413430`, the implementing commit. **Wording:** DESIGN §12 L2930 says "Lexical text holds each distinct token once", but the body is not deduplicated; only the names part is. **Measured:** §5 | The Phase 4 (b) metric, which decides retrieval techniques under the keep rule, would measure matcher v2 plus a lexical input no registration fixed. Concretely, `proxy` is in every brief, so the lexical leg ignores it in every query. A brief's vocabulary now scales with how many subclasses inherit its seed: `Provider.disable` has 50 inherited spellings. That is the alias-count leak the tokens-once rule was registered to remove, moved from term frequency to document frequency | **Before any score**, add one ADR-0010 amendment line fixing the name set on gold-independent grounds. The options: own spellings only; the seed's container spellings (`FORMAT` 1's set); or every spelling (§8). Disclose this review's measurement, as the amendment disclosed the direction it knew. Correct §11.2 and §12 to say "each distinct *name* token once". Either choice is a small edit to `lexical()` | **None exists.** Test: a bundle test on `analysis_shapes`, where `pkg.Alpha.tool` names `Server.tool` through a public subclass, asserting the names line includes or omits `alpha` as registered |
| **F2** | The registered "one matcher" is partly re-implemented per script. Registered rules are applied by `score_gold.py` only, or computed from other sources | DM-59, DM-02 · G7 | **Resolver:** `ranking_check.py:41` resolves through `node_of`, a first-match scan (`gold_match.py:68-73`), while `resolve` builds a last-wins dict (`:37`). **Degrade:** it prints `degraded_reason` (`:53-54`) but exits 0 whenever the seed is first (`:61`), whatever the mode. **Root:** `score_gold.py:71` classifies under or outside the root by `gen.library`, the distribution name, not the release's public roots. **Denominator:** `:127-130` drops unmapped spans from (c)'s denominator (the registration says 157). The pilot has 0 unmapped (checked) | **Degrade:** 4.4 runs `just ranking-check <gen> vllm`. If the service degrades mid-run (it is down, or an answer is rejected by §11.1's checks), a lexical-only first rank counts, and the check exits 0: `passed` on evidence §11.1 says is not evidence. **Root:** in a library whose import root is not its distribution name, or that has several roots, every unresolved operation under the root is reported as "outside". **Denominator:** a wrong `--sources` silently shrinks the denominator | Move `status`, the root (the served `public_paths`' first segments, or the manifest) and the unit denominators into `gold_match`. `ranking_check` then uses `resolve` and `status` and exits 2 on a degraded live alias. Count an unmapped span as a miss, or block | Pytest (`tests/scripts`): a degraded `vllm` alias makes the ranking check exit 2; an operation under a second root is under the root; an unmapped span stays in the denominator |
| **F3** | "One path, one node" is declared (D48) and assumed, but not validated. The table key admits two nodes per path | DM-07, DM-02 · G3 | **Key:** `findings.rs:267-270` (`[snapshot_id, node_id, access_path]`). **Collision:** `public.rs:103` unions `direct` and `methods`. A direct export `pkg.C.f` (function `f` of module `pkg/C.py`) and the member path `pkg.C.f` (class `C` re-exported as `pkg.C`) both pass `semantic:public-path-exported`. **Consumers, with different ties:** `analyze.rs:299` (BTreeMap, last wins), `gold_match.py:37` (dict, last wins), `:68` (first wins), and `symbol_map`, which keeps both. **Pilot:** 0 duplicate paths among 4,763 (checked on the served file). This is constructible from the SQL; no fixture exercises it | Seed resolution, the matcher and the ranking check silently resolve one spelling to different nodes. Promotion lifts a brief the matcher does not credit | Add the rule `semantic:public-path-one-node` (`GROUP BY snapshot_id, access_path HAVING count(DISTINCT node_id) > 1`), or break the tie in the relation as Python does (the class attribute shadows the submodule) | Test: the rule with an injected case in the rules test; optionally a `public_shapes` case |
| **F4** | DESIGN says `public_paths` carries `member()`'s rule. Two unstated differences in its **domain** make the claim broader than what is pinned | DM-59, DM-08 · G7; guidelines §2 (unknown targets explicit) | **(a)** DESIGN L1833 says "Nothing is inherited past an unresolved base", and §9.1 L1860 says the walk "fails closed". But an **undefined** base is dropped by Pyrefly, and its successors' members are inherited: the snapshot pins `pub.Unresolved.run \| pub.core.Base.run`, and `syntax.rs:1583` states it in a comment. The refusal fires only for a null target, which "no validated snapshot holds" (L1842). **(b)** The old walk resolved any depth (`resolve_seeds` looped over `parts[k..]`). `methods` (`public.rs:97-102`) covers only members of *exported* classes, so `pub.Outer.Inner.m` has no row. L1841 says `public_shapes` "covers … nested classes", but nothing asserts the absence | **(a)** For a release class whose optional-import base did not resolve, seeds, promotion and the matcher name the successor's `def`, although the dropped base may define that name. **(b)** A configured seed or gold operation at depth 2 is refused or unresolved under no stated policy. There is no pilot impact: 0 null targets, and none of the 7 unresolved operations under the root is nested | Add one sentence each to §9's opening and §9.1 stating the one-level member domain and the undefined-base fail-open. Pin both with assertions. Detecting a dropped base (header base expressions against Pysa's `base` rows) can wait for a trigger | Test: in `public_paths_follow_the_member_rule`, assert that `pub.Outer.Inner.m` is absent, and that `pub.Unresolved.run` is present as the documented fail-open |
| **F5** | D48: the seed rank's "last in source order" makes a property's **setter** its public node. This is faithful to `member()` | DM-24, DM-08 · G2 | `lctx query` on `ec03626f`: `FastMCP.instructions` has a `[property]` def (byte 21078, 0 paths) and a `[setter]` def (byte 21180, 4 paths). `FastMCPStreamableHTTPSessionManager.event_store` is the same. These are the only two same-name non-overload class defs on the pilot. `declarations.decorators` already distinguishes them. No gold operation, config seed or selected brief names either | A read of `mcp.instructions`, which runs the getter, is credited to no public node. A brief seeded by a property would describe the setter's `(self, value)` | Accept for the rescore. Deferred row (§11) with its trigger | A `public_shapes` property-plus-setter case when fixed |

**Observations** (none moves a gate):
- **O1: the class ceiling.**
  - 47 of the gold's 125 operations resolve to classes. No brief in the default is seeded by a
    class: the configured seeds are methods, and selection draws from callables
    (`CALLABLE_KINDS`).
  - `fm.discovery` and `fm.middleware` are class-only, so 4 of 44 aliases cannot hit unless a
    class is configured as a seed.
  - Under the default's 20 seeds, 8 families (16 aliases) have a seed in their node set.
  - The class rule is gold-independent and registered. The rescore report should state this
    ceiling, so that a low (b) is not read as a retrieval failure.
- **O2: the tokenizer.** Add a per-suite exhaustive assertion that the non-ASCII code points
  whose lowercase yields an ASCII token are exactly {U+0130, U+212A}. It is cheap, and it catches
  a Unicode-table upgrade that `tokens.json`'s 8 text cases would not.
- **O3: the version-1 numbers.** DESIGN §12's version-1 Measured scores, and the amendment's
  disclosed "+type-layer 16 against 14", were measured under `FORMAT` 1 lexical text. They are not
  a baseline for version-2 scores. The plan already cuts `FORMAT` 1 rescoring; the report should
  not compare across.
- **O4: D52's exposure.** A fake-vector run still ranks with **real** BM25 over the real lexical
  text, so "not evidence" holds for the vector leg only. The author has seen a `FORMAT` 2 (b) that
  partly reflects F1's name set. F1's line should therefore rest on the gold-independent argument
  and cite D52. This review's §5 also names one alias word (`proxy`), which is disclosed here for
  the same reason.

**Applicability.**
- **Bore on this scope:**
  - Groups 1 and 2 (authority, validity: `public_paths`, the row decoder);
  - Group 3 (identity: node against path; the compiler digest);
  - Group 5 (fidelity to `member()`);
  - Group 9 (the Rust↔Python boundary: tokenizer, embedding responses, the bundle);
  - Groups 10–12 (the pre-registration's falsifiability, DM-59; the oracles, DM-60;
    proportionality, D46 and D51).
- **Did not bear:**
  - Groups 6–7's concurrency and incrementality: nothing in scope schedules or invalidates;
  - Group 8's performance: the only timing is the author's 0.89 s → 0.02 s, which is not a claim
    this review relies on;
  - Group 4's templates: the relation is ordinary SQL behind a named contract. Charter §F places
    it correctly.

## 8. Alternatives for F1's registration (the one real choice here)

| Alternative | Duplication and locality | Correctness risk | Cost | Evidence | Assessment |
|---|---|---|---|---|---|
| Every spelling (current `FORMAT` 2) | One join. Promotion and lexical text share one set | The vocabulary scales with subclass count (`Provider.disable` +56). Common subclass words (`proxy`) become non-discriminating for every query | None | §5, Measured | Defensible ("a spelling is the node"), but it undoes the tokens-once rationale at the document-frequency level |
| The seed's container spellings (`FORMAT` 1's set) | Needs the container rule, which `resolve_seeds` already has | Re-uses the registered baseline. `custom_route` keeps its inherited aliases through `FastMCP`, as before | Small | `3039ddc7` | The registered status quo. The least change to argue |
| **Simpler: own spellings only** | A filter on `own`. Matches the brief document's "Public access" text, which is own-only | Loses `fastmcp.FastMCP.*` tokens for mixin briefs (`http_app`, `run`), which promotion still covers | Smallest | — | The simplest and most principled, since the text describes what the declaration's module declares. Its cost is the mixin briefs' lexical recall on the subclass name |

The choice is the operator's. It must be made on these grounds, not on a score.

## 9. Verification

| Claim or risk | Label | Check | Conditions | Result |
|---|---|---|---|---|
| Scope builds and tests green | Tested | `just test-all` on a `git archive 377d7fe` export, with its own `CARGO_TARGET_DIR` and the gitignored skills copied in | fmt-check; clippy `-D warnings`; nextest 214/214; py-fixture; pytest 60; pyrefly; rules-scan; rules-test 6/6; lint-agents; adr lint (20); fixtures-check (54 files); deps (family, deny, pyrefly-fork, shear); gold | **passed** (lint-agents first failed only because the export lacked the gitignored skill directories, and passed once they were present) |
| — | — | `just test-all` on the live working tree | `c9a4194` plus another session's uncommitted edits to `diff.rs` and `tests/analysis.rs` | **failed** at fmt-check on those edits. Out of scope; they were later committed as `0e98d85` |
| Rust and Python tokenizers are identical | Tested (probe) | Standalone compile of `tokens`' body (a verbatim copy) with rustc 1.98.1, against `lctx_mcp.retrieval.tokenize` on Python 3.14.7 | Every scalar value alone and between ASCII letters, plus final-sigma strings | **passed**: 0 differences; ASCII-yielding non-ASCII code points {U+0130, U+212A} |
| The matcher implements the registration | Tested (constructed rows) | `tests/scripts/test_gold_match.py` (4 tests) | An inherited spelling, the class rule, unresolved operations, the blocked status | passed (in the pytest 60 above). ranking_check's parity is untested (F2) |
| `public_paths` refuses as `member()` did | Tested | `public_paths_follow_the_member_rule` (snapshot), `nothing_is_inherited_past_an_unresolved_base` (hand-built session), `public_paths_agree_with_the_seed_resolution` | The fixtures. The agreement test became tautological at `eacdcba`, because briefs are resolved by the relation itself | passed. F4's two domain limits are not asserted |
| `FORMAT` 2 promotes inherited spellings | Tested, and verified on the pilot | `test_an_inherited_spelling_is_promoted`; `symbol_map` of `f0e86466` | `http_app`, `run`, `FastMCPProxy.tool` | Promotion works on the pilot, and no symbol names two briefs |
| Plan Phase 2 verification: `kind, own, count(*)` | Measured | `lctx query` on `ec03626f` | — | function own 1,108 and inherited 1,220; async own 718 and inherited 1,210; class 507 (own). One node has no own path (`…mcp_config.*MCPServer.to_transport`) |
| Gold spans map with the default `--sources` | Measured | File check over `build/envs/fastmcp` | 157 spans | 0 unmapped |
| Step 4's published output is unchanged | Asserted | The author's `lctx diff` | — | not re-run |

**Top gaps:** F1's missing test; F2's missing ranking-check tests; F3's missing rule.

## 10. Exceptions (D46–D52)

- **D46: no `serde_arrow` spike.** Agreed. `query_row!` reuses the one `ArrowColumn` mapping,
  where a second mapping would need serde impls and would check neither codes nor metadata
  (DM-02, DM-58).
- **D47:** out of scope (R1).
- **D48:** accepted as Deferred (F5).
- **D49: the mention lookup kept on `exports`.** Agreed. Private-origin mentions need `exports`,
  and the lookup answers a different question (§2).
- **D50: the `FORMAT` 1 shim, and the fresh store with the copied cache.** Agreed. It kept each
  served change in its registering commit, and the cache key is semantic.
- **D51: no `CommunityLayer` codebook.** Agreed under DM-58. `LayerSpec` is the one declaration,
  and no stored column carries a layer code.
- **D52:** adequate as a disclosure. Add O4's qualification when F1's registration line cites it.

## 11. Decision and implementation changes

**Decision: Revise (small).**

**Reason:**
- The implementation of the registered rules in `score_gold.py` is faithful.
- `public_paths` is a sound single authority, and the tokenizer is exhaustively equivalent.
- But the retrieval input that the keep rule's metric depends on changed without registration
  (F1), and the §1.5 check can pass on degraded evidence (F2). Both are cheap to close, and both
  must close before any score.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1, before any score | F1: an ADR-0010 amendment line fixing the lexical name set (§8), disclosing §5 and D52. The matching `lexical()` edit if the choice is not "every spelling". Fix the §11.2 and §12 wording | DM-59 · G7 | A commit that precedes every score commit in `git log` | The bundle names-line test (F1) |
| 1, before any score | F2: `status`, the root and the denominators in `gold_match`; `ranking_check` on `resolve` and `status` | DM-59, DM-02 · G7 | A degraded live alias exits 2 in both scripts | pytest (F2) |
| 2, before 4.2 | F3: `semantic:public-path-one-node` with an injected case | DM-07 · G3 | The rule passes on the pilot (0 today) | The rules test |
| 3, before R3 | F4: two DESIGN sentences and two assertions | DM-59 · G7 | §9's opening and §9.1 match the pinned behaviour | `public_paths_follow_the_member_rule` |

**What else stands between here and a trusted Phase 4 rescore** (question 5):
1. F1 and F2 above. F3 is recommended.
2. The plan's Phase 4 prerequisites. ADR-0020 review F7 and O3 (`lctx diff` keyed by seed, chunk
   and text hash) and D3's correctness pieces are claimed by `0e98d85`, which landed during this
   review and was **not reviewed** here.
3. 4.1's ADR-0020 re-registration and DESIGN §9.8, committed before any score.
4. Phase 3's byte-identical proof (the guard is at `c9a4194`). The rescore should run on the
   post-Phase-3 compiler.
5. The ablation JSON (4.2) must record:
   - the gold's sha256, the bundle `FORMAT`, `matcher_version` and the units;
   - the class ceiling (O1).

   `score_gold.py`'s JSON records neither the sha256 nor `FORMAT` today.
6. Plan step 10's `just pilot` after these fixes, with live vectors only when ≥30 GB is free.

**Deferred**

| Item | Why not now | Trigger that reopens it |
|---|---|---|
| F5 / D48: getter-first seed rank via `declarations.decorators` | No gold operation, seed or selected brief names a property with a setter (pilot: 2 such properties) | A seed, gold operation or selected brief names a property that has a setter; or a Related or usage consumer shows property reads |
| F4(a) detection of a dropped undefined base | 0 on the pilot, and the claim is narrowed instead | A release whose acquired environment lacks an optional dependency that a public class subclasses |
| O2's per-suite Unicode assertion | Equivalence is proven for the pinned versions | A Rust toolchain or Python bump (a `docs/pins.md` row) |

**Final check.** The implementation matches every registered matcher rule except where F2 names
it. The claims that go beyond the evidence are F1's "nothing else changes" and F4's domain
sentences. Each has a narrow fix, and each can close before any score is computed.

## 12. Disposition (author, 2026-09-24)

| Finding | Disposition | Where |
|---|---|---|
| F1 | **Fixed; the operator's choice made by the author on the review's grounds, for the operator's review (deviation log D53).** Registered first (`7bebdd4`, an ADR-0010 amendment before any score): a brief's lexical names are its seed's **own** spellings, each distinct name token once. It depends on the node alone, matches the brief's own Public access text, and keeps subclass counts out of document frequency; the review's measurement and D52's exposure (O4) are disclosed there. `lexical()` joins own members only; a bundle test asserts `pkg.Server.tool`'s names omit `alpha` while `pkg.Alpha.tool` still promotes it. §6.4, §11.2 and §12 say "name tokens" | `bundle.rs`, `tests/bundle.rs` |
| F2 | **Fixed.** `gold_match` holds the one path lookup (`paths`, used by `resolve` and `node_of`), the roots (the served paths' first segments), `status`; `ranking_check` exits 2 when a live alias degraded; an unmapped span is a miss in (c)'s denominator (`span_recall`). The score JSON records the gold's sha256, the bundle format, the units and the class ceiling (O1). Pytest: a second root, an unmapped span, a degraded live ranking check | `scripts/gold_match.py`, `score_gold.py`, `ranking_check.py`, `tests/scripts/test_gold_match.py` |
| F3 | **Fixed.** `semantic:public-path-one-node`, with an injected case (a second node under an existing path) | `rules.rs`, `tests/analysis.rs` |
| F4 | **Fixed.** §9's opening and §9.1 state the one-level member domain and the undefined-base fail-open; `public_paths_follow_the_member_rule` asserts `pub.Outer.Inner.m` is absent and `pub.Unresolved.run` present | DESIGN, `tests/syntax.rs` |
| F5 | **Deferred** with the review's trigger (§11) | — |
| O1 | The class ceiling is recorded in every score JSON, for the rescore report | `score_gold.py` |
| O2 | Deferred with the review's trigger | — |
| O3 | Noted: no version-1 score is compared with a version-2 one | — |
| O4 | Disclosed in F1's registration line | ADR-0010 |
