# Design review: increment 2, slice 2.1 (Pass B and parameter docs), compact

## 1. Decision and scope

**Proposal:** Pass B (DESIGN §9.2) and the raw table `parameter_docs` (§3.2 `signatures`). This
covers:
- the declared relations `cpg_schema::flows` (argument flows, guards);
- the worklist `lctx_analytics::pass_b`, wired in `cpg-core/src/analyze.rs`;
- the Stage F templates `control`, `transformed_control`, `restriction` and the documented
  `parameter`;
- the codebook appends and their policies;
- the rule `semantic:documented-parameter-cites-its-doc`.

Per deviation log D17, this review also stands in for the plan's 2.0 review of Pass B's stated
abstractions, so §9.2's Proposed half is judged together with its code.

**Status:** Implemented. The Tested scope is set out in §9.
**Reviewer / author:** design-reviewer subagent (fresh context) / the slice-2.1 session.
**Affected revisions:** commits `36d1259` (slice 2.1) and `083507f` (template and rule follow-ups),
at HEAD `d126173`. `COMPILER_OUTPUT_VERSION` 5, `TEMPLATE_VERSION` 6. Pilot snapshot
`7799813fb40d477238866279df653cc4` (FastMCP 4.0.5, fake embedder).

**Observable outcome:** each brief gains:
- documented parameter descriptions;
- `control` lines saying where a seed parameter is passed unchanged;
- literals the seed fixes;
- `restriction` lines for raises guarded by a parameter test.

**Baseline:** increment 1. Parameters came from the signature only, and the brief had no controls
or restrictions.

**Supported scope and non-goals:** `call`/`init` arcs into subsystem functions, up to Pass A's
depth. One identity alias. Literals from the seed only. Restrictions are never preconditions
(D18). No CFG or dataflow (§13).

### Method and coverage

**Read at the grain cited:**
- `flows.rs` (all of it);
- `pass_b.rs` (all of it);
- the Pass B wiring in `analyze.rs` (`seed_parameters`, `run`);
- in `synth.rs`: the parameter query, the parameter, control, transformed and restriction
  templates, and the document filter;
- in `walk.rs`: `description_span` and `parameter_docs`;
- the `parameter_docs` spec (`tables.rs`);
- the rule and its injected case;
- the codebook appends and the `findings.rs` policy diff;
- `controls.py` and both named tests with their snapshots.

**Also read:**
- Pyrefly at the pinned `a07b7ba`: `has_implicit_receiver`, `resolve_constructor_callees` and
  `parse_parameter_documentation`;
- DESIGN §3.2, §4.2.4, §9.2 and §10.2–§10.3;
- ADR-0019 and its amendments;
- the deviation log;
- the Deferred tables of the ADR-0019, 1.4, 1.5, C2–C6 and increment-1 deep reviews.

**Ran:**

| Check | Outcome | Command and conditions |
|---|---|---|
| Whole suite, working tree | **passed**: nextest 154/154, pytest 51/51, rule tests 4/4, `adr lint` 19, fixtures 48, deps, gold | `just test-all` in the repo (2026-09-23). **Caveat:** the tree then held uncommitted slice-2.2 edits by another session (`codebook.rs`, `findings.rs`, `flows.rs`), so this outcome is not the reviewed commits' |
| Reviewed commits | **passed**: 154/154 | `git archive d126173` into the scratchpad, then `INSTA_UPDATE=no cargo nextest run --workspace` with a separate target dir |
| Probe 1: `review_probe_substring_collision` | **passed**, which means the misattribution in F3 reproduces | a test on `description_span`, added to the scratch copy only |
| Probe 2: `review_probe_pass_b_restrictions` | **passed**; it prints the texts quoted in F1 and F5 | the scratch copy, with four functions added to `controls.py` (`strict`, `muted`, `Widget`, `probe_attr`) |
| Pilot queries | see the findings | `lctx query` on `7799813f`: findings, assertions, brief documents; reconstructions of `argument_flows_sql` and `guards_sql` with the codebook codes substituted |
| Pilot span checks | see F2, F3 | Python over the pilot environment and source tree, read-only. Spans were checked byte-exact for non-ASCII files and char-exact for ASCII files |
| `just pilot`, `just pilot-live` | **not_run** | the published snapshot was used; the GPU service was not started, as instructed |

**Not attacked, so asserted:**
- that Stage F's control and restriction ordering is total;
- async and generator callees, where the raise happens at await or iteration;
- the bundle and serving path;
- the correctness of each of the 35 pilot `forwarding` findings. `FastMCP.tool`, `mount` and
  `resource` were spot-checked against source;
- whether Pyrefly's `implicit_receiver` is right for decorated or `functools.partial` targets.

## 2–4. Authority, contracts and derivation (compressed)

**Authority.**
- The two relations are declared once, in `cpg_schema::flows`. Their digest enters:
  - the compiler digest (`attempt.rs:83`);
  - each Pass B invocation's `projection_digest`.
- The kernel reads columns by name after a strict cast.
- `FINDING_STATUS` feeds both the kernel and the generated status rule.
- `parameter_docs` has one producer, the Ruff walk calling Pyrefly's parser. Its text is
  Pyrefly's, and its span is our locator's.
- Two meanings are restated rather than derived. Both agree today:
  - the accepted-arc policy (O1);
  - "the receiver" (F8).

**The argument → formal mapping, case by case** (focus question 1):

| Case | Code | Evidence | Verdict |
|---|---|---|---|
| Positional before any `*` | `fm.ordinal = r.ordinal + a.receiver`, formal kind positional-only or positional-or-keyword (`flows.rs:139-141`) | `pass_b_configure` (`swap` both ways, `build`) | sound; Tested |
| Keyword → positional-or-keyword or keyword-only by name; positional-only excluded | `flows.rs:142-143` | `tag=mode` → `Registry.add.tag` | sound; Tested |
| Implicit receiver | +1 when Pysa reports `true_with_class_receiver` or `true_with_object_receiver`. Pyrefly: a static method never has one, a class method always does, a method has one when called on an instance (`call_graph.rs:1278-1306`) | fixture `Registry().add(name, …)`. Pilot: 0 of 32,075 mapped flows count a receiver on a target whose first formal is not `self`/`cls`. 96 map onto a `self`/`cls` formal without one, and all are correct (`Cls.m(obj)`, or a module function whose first parameter is named `cls`) | sound; Tested for an object receiver, pilot-observed for a class receiver |
| `init` phase | Pyrefly resolves `__init__` as a bound method on the instance, so the receiver is the object | pilot: every init flow has receiver 2 (934 parameter flows). Probe 2: `Widget(name)` → `__init__`'s `label` | sound; **not in the repo's tests** (F6) |
| `*` argument | positionals from the first `*` on are unmapped; keywords after it map by name | none in the fixture | sound, but DESIGN's wording is wider (F6) |
| `**` argument, catch-all formals | an inner join to no formal | `passthrough(**options)` absent | sound; Tested |
| Overloads | call targets are implementations: 0 of 24,008 pilot `call_target` edges end on an `@overload` stub | pilot | sound; pilot-observed |
| Decorated targets | mapped by the undecorated `def`'s formals | pilot decorators are all signature-preserving | assumption not stated (O3) |

**Aliases and rebinding (focus question 2).**
- A value maps only when two things hold:
  - the name has exactly one binding event in its scope (`single`, `flows.rs:43-52`). Events in
    statically pruned branches count, so this is stricter than §4.2.4's "before";
  - the reference is not captured.
- An alias is one `Assign` directly under the caller's `def`, whose right-hand side is a bare
  name of such a parameter (`flows.rs:101-114`).
- So "never guessed" holds for:
  - multiple writes;
  - augmented assignment;
  - `del`;
  - annotation-only rebinding;
  - tuple targets;
  - walrus;
  - closures.
- What §9.2 and §4.2.4 promise beyond declining is a **trace**, and it is missing (F4).

## 5. Journey: a restriction from seed to brief

`FastMCP.tool(name_or_fn, …)` calls `self._local_provider.tool(name_or_fn, …)`.
1. The call is an `Overrides` dispatch, so the flow carries modality `candidate`.
2. The worklist reaches `ToolDecoratorMixin.tool`'s `name_or_fn`.
3. The guard `if isinstance(name_or_fn, classmethod): raise TypeError(...)` sits directly in its
   body.
4. Its test reads a parameter bound once and a builtin, so it is a guard.
5. No call on the path is in a `try`, so a `conditional_raise` is emitted.

Stage F then writes two things into the same brief:
- **Controls:** "`name_or_fn` is passed on to `…ToolDecoratorMixin.tool` as `name_or_fn`, over an
  overridable call"
- **Limits:** "`…ToolDecoratorMixin.tool` raises (`raise TypeError`) when
  `isinstance(name_or_fn, classmethod)`; its `name_or_fn` receives `name_or_fn`."

The Limits line drops the qualifier the Controls line states. Probe 2 shows the same path logic
publishing two restrictions the seed cannot trigger: one guarded by a contrary branch, one inside
`contextlib.suppress` (F1). Restrictions are in the embedded brief document: 4 pilot documents
contain one (`synth.rs:1530-1531`).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **pass** | The relations have one declaration, digested twice (compiler, invocation). The status policy feeds kernel and rule. `parameter_docs` has one producer. The restated arc policy (O1) and receiver rule (F8) agree today | F8, O1 (P3) |
| G2 Semantic fidelity | **fail** | Published restrictions drop the path's modality and conditions (F1: fixture, 2 of 4 pilot restrictions, probe 2). Provider truncation is adopted silently: 46 of 1,580 rows (F2). A declined flow is indistinguishable from an absent one, against §4.2.4 (F4) | F1, F2, F4 |
| G3 Validity | **fail (latent)** | A misattributed or spliced description span can reach publication. The locator allows it (probe 1), and the rule named for it checks only "some parameter-doc span in the module" (F3). Not observed on the pilot | F3; F7 |
| G4 Hidden behavior | **pass** | Pass B reads only session tables. Pyrefly's parser is a pure function of the docstring value. `analytics.toml` is unchanged (D17). No gold path | — |
| G5 Consistency and recovery | **pass** | No new publication path. `parameter_docs` goes through `write_raw` with generated key, ref and codebook rules. Pass B rows go through the attempt's analysis write (not re-attacked) | — |
| G6 Transformation and reuse | **pass** | The flows digest is in `compiler_digest` (`attempt.rs:83`). Finding ids are content-derived. `pass_a_is_identical_across_module_order_and_location` compares **every** row of `findings`, `witnesses`, `finding_members` and `analysis_invocations` across module order and location, Pass B's included. The ledger is pinned at output version 5 and template version 6 | O4 (invocation-level record) |
| G7 Truthful capability claims | **fail (narrow)** | §9.2 claims "each overridable hop said" (L1879) and "never when an enclosing handler may catch it" (L1855-1857); neither holds (F1). §3.2 claims "the description's verbatim bytes" (L485); for 46 rows it is a prefix (F2) | F1, F2, F6 |

## 7. Findings

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **A restriction is published without the qualifiers of the path that reaches it**: the path's modality, its branch conditions and any suppressing `with` | DM-24, DM-59 · G2, G7 (ADDENDUM Q5) | The template `synth.rs:1250-1252` is "`{callee}` raises (`{raised}`) when `{test}`; its `{formal}` receives `{p}`". It reads no witness modality, while the control template adds ", over an overridable call" (`synth.rs:1180-1182`). The only decline is `in_try` (`pass_b.rs:287`), and `tries` selects `StmtTry` only (`flows.rs:115-121`). No branch condition on the path is read. Instances: **(a)** `analysis__briefs.snap` pins "`pkg.controls.Registry.add` raises … when `item is None`; its `item` receives `name`." beside the same brief's "as `item`, over an overridable call". The pilot does the same for `ToolDecoratorMixin.tool` and `PromptDecoratorMixin.prompt` (2 of 4 restrictions). **(b)** Probe 2, `if name is not None: strict(name)`, gives "`pkg.controls.strict` raises (`raise ValueError`) when `value is None`; its `value` receives `name`." **(c)** Probe 2, `with contextlib.suppress(ValueError): muted(size)`, gives "`pkg.controls.muted` raises …". **(d)** The control says "over an overridable call" once per target, not which hop: "`…decorate_and_register` as `fn` through `…ToolDecoratorMixin.tool`, over an overridable call" (the overridable hop is the first) | An agent reads FastMCP.tool's Limits entry as unconditional, although the same brief calls that hop overridable. For (b) and (c) a Limits entry is served and embedded that the seed can never trigger | One path-qualifier rule shared by `control` and `restriction`: (1) name each override-open hop, as Pass A's F5 wording does; (2) treat a `with` in the caller like a `try`, which is conservative; (3) decline a guard when a call on the path sits in a branch of its caller whose test reads the flowing value, or qualify the text ("if `seed` reaches it"). About 30 lines across `flows.rs`, `pass_b.rs` and `synth.rs`, plus a `TEMPLATE_VERSION` bump | **None exists for (b), (c) or (d); the snapshot pins (a) as correct.** Test: add probe 2's `strict` and `muted` to `controls.py`, and assert the texts in `templates_say_what_the_findings_show` |
| **F2** | **Parameter descriptions are silently truncated by Pyrefly's Google parser** | DM-42, DM-43, DM-59 · G2, G7 | Pyrefly's `parse_google_params` splits on `':'` before its continuation-indent test (`pyrefly_python/src/docstring.rs`, pinned `a07b7ba`). So a deeper-indented continuation containing a colon starts a new entry (`Note`, `-`, `Example`). The walker then drops that entry as no parameter (`walk.rs:513-515`), and the real parameter keeps a prefix. Pilot: 46 of 1,580 rows end just before such a line. Checked by hand: `OAuthProxy.__init__`'s `require_authorization_consent` is stored as "Consent screen behavior (default True).", while the source continues "- True: …", "- \"remember\": …" (`proxy.py:392-398`). None of the 26 documented seed parameters is affected today | When increment 3 widens to 15–25 briefs, a seed whose docstring lists values (a common FastMCP style) gets a `documented` parameter assertion that reads as complete but lacks the values. No row, fidelity or count says the text is a prefix | After locating, test whether the next non-blank line is indented deeper than the entry header. If it is, either extend to the entry's end by indentation (text from those bytes) or emit nothing and count it. Alternatively, fix the parser in the fork's patch. State the choice in §3.2 | **None exists.** A unit case in `walk.rs` with a "Note: …" continuation, plus a truncated or dropped count in `just pilot`'s output |
| **F3** | **Attributing a description to its parameter is neither sound by construction nor checked** | DM-07, DM-46, DM-53 · G3 | `walk.rs:237` anchors on the first **substring** match of the name (`literal.match_indices(name)`), then takes the first match of the description's first line after it. Probe 1: with `timeout: Optional.` / `out: Optional.`, `out`'s span sits on `timeout`'s line. With two-line descriptions sharing a first line, `out`'s span runs from `timeout`'s text through `out:`. The rule `semantic:documented-parameter-cites-its-doc` (`rules.rs:280-292`) matches evidence to **any** `parameter_docs` span in the module, never by parameter. Its injected case shifts every span by one byte (`analysis.rs:916-923`). The key `(snapshot, function, name)` lets two names share a span. Pilot today: 0 shared spans, 0 overlapping, and all 1,580 spans equal their text modulo whitespace (latent) | A `documented` parameter can cite another parameter's bytes, or a span holding two entries. The byte check passes (the text is the source's bytes), and so does the rule named for exactly this | Anchor on the entry header: the name at a line start followed by `:` or ` (`, or Sphinx `:param … name:`. Refuse unless the located lines, trimmed, equal Pyrefly's. Make the rule join `evidence.node_id` → `parameter_syntax (function_node_id, name)` = `parameter_docs (function_node_id, name)`, with an injected case that swaps two parameters' spans | Keep probe 1 as a unit test in `walk.rs`; add the strengthened rule and its swap case to `the_analysis_rules_reject_their_violations` |
| **F4** | **Declined flows and guards leave no trace, and DESIGN contradicts itself on it** | DM-08, DM-02, DM-59 · G2 (ADDENDUM Q4) | §4.2.4 (L1258-1259): a rebinding before a guard or forwarding site "marks that site `ambiguous_binding` (a boundary)"; §9.2 L1853 cites it. §9.2's Implemented half (L1875-1876) says "declined". The code does the second: `other` values are skipped (`pass_b.rs:218-223`), and nothing is emitted for a rebinding, a computed value, a `*` or `**` argument or an `in_try` decline. Pilot: FastMCP.tool's `meta` (rebound by `meta = dict(meta) if meta else {}`), `task` (computed) and `app` have parameter assertions (brief ordinals 13-15), but no control and no Limits line. The invocation says `complete_under_stated_model` | An agent cannot tell "not passed on" from "passed on in a form Pass B declines". For `meta`, the silence reads as "not forwarded", although a copy is forwarded: this is §4.2.4's motivating shape. The §B11 gap metric does not see it | **U1, the author's decision:** (a) emit one finding per declined seed-parameter flow with a reason (`ambiguous_binding`, handler, unmapped or computed), rendered as one Limits line; about 40 lines and a codebook append. Or (b) amend §4.2.4 to "Pass B declines; nothing records it", and state that model once in each brief's Limits | **None exists.** The fixture already has the shapes (`rebinding`, `checked`, `passthrough`); assert the chosen trace in `pass_b_finds_the_known_answers_on_analysis_shapes` |
| F5 | **"Supported predicate" is undefined, and the guard reader admits property, subscript and method reads** | DM-59, DM-24 · G2 (latent) | §9.2 L1846 says "a supported predicate over known parameters". L1852 says property or subscript access is never guessed. `guards_sql` checks only `ExprName` nodes (`flows.rs:193-202`). Pilot: 86 of 150 guard tests include an attribute or subscript (`path.is_symlink()`, `not route.client.is_connected()`, `redirect_uri is None and self.cimd_document is not None`). Probe 2 gives "`probe_attr` raises (`raise RuntimeError`) when `obj.closed`". All 4 published pilot restrictions read names only | The next seed publishes a check on object state, or on an arbitrary method's result, as "a check on `x`", which is what the never-guessed row forbids | Define "supported" in §9.2. Either reject `Attribute`, `Subscript` and non-builtin `Call` in `guards_sql`, or scope the never-guessed row to argument values and say guards quote any expression over parameters verbatim | Test: a fixture guard on `obj.closed` with the chosen expectation |
| F6 | **The `Tested` label is broader than the tests, and one sentence is wider than the code** | DM-59, DM-54 | §9.2's block (L1860-1887) is "Implemented and Tested", but `controls.py` has no `init` call with an argument (`Registry()`), no class receiver, no `*` argument and no depth-2 chain. Every `pass_b_configure` row is depth 1, and Pass B's `depth_limit` stop is untested. "Never mapped: starred arguments and anything after one" (L1876), yet `flows.rs:142-143` maps a keyword after a `*` (soundly) | A regression in `init` or class-receiver mapping, such as a Pyrefly upgrade changing `implicit_receiver` for `__init__`, passes every test; only a pilot diff would show it | Add `Widget(name)` with a guarded `__init__`, a class-method call, `swap(*extra, name)` and one depth-2 chain to `controls.py`; or relabel those cases Implemented (pilot-observed). Fix the sentence | The `pass_b_configure` snapshot |
| F7 | **The kind policy is wider than D18, and nothing constrains the extra width** | DM-07, DM-59 · G3 (latent) | `findings.rs:362-383` permits `Documented` for `Control` and `Restriction`. No Stage F path produces it: every pilot kind-6 and kind-8 assertion has status 0. §10.2 L2103 says "documented only with precondition documentation", but no rule checks it. D18 says "nothing promotes one" | The first change that cites a docstring span on a restriction (3.4's documented warnings) publishes "the implementation raises …" as `documented`, with nothing checking that the doc states the precondition. This repeats the D8/O7 shape | Narrow both rows to `structurally_observed` now. Widen them in 3.4 together with that slice's rule, as O7 did for parameters | Contracts snapshot plus `semantic:assertion-policy-published`; an injected case when widened |
| F8 | **"The receiver" is decided by name, in three places** | DM-06, DM-02 | `analyze.rs:428`, `synth.rs:1018` and `synth.rs:1517` all use `ordinal = 0 AND name IN ('self','cls')`. The pilot has a module function whose first parameter is `cls` (`fastmcp.utilities.types.get_cached_typeadapter`; 94 flows map onto it) | Seeded, such a function loses a real control from Pass B and from Controls. A method whose receiver has another name gains a spurious control | One helper from the declaration kind (method, class method, static method or function) for all three sites | **None exists.** A unit test on the helper |

**Observations** (no finding, recorded so their silence is not read as clean):
- **O1.** The accepted-arc policy is restated as literals in `flows.rs:57-62`, not taken from
  `projection::invocation()`. `inside` meanwhile uses that projection's mask (`analyze.rs:569`).
  They agree today; build the flows filter from the spec's constants.
- **O2.** §9.2 L1854's visited key `(callable, formal_parameter, mapping_context)` is stale. The
  code and the Implemented half use `(callable, formal, source parameter)`, which is the right
  state.
- **O3.** Decorated targets are mapped by the undecorated `def`. The pilot's flow-target
  decorators are all signature-preserving: `classmethod`, `staticmethod`, `contextmanager`,
  `asynccontextmanager`, `override`, `command`, `lru_cache`, `mcp_tool`, `tool` and
  `abstractmethod`. The assumption is unstated (Deferred).
- **O4.** Pass B's invocation records the flows digest, but not the invocation projection whose
  subsystem mask it applies. The compiler digest covers this per snapshot; per-invocation
  ablation would not see it (Deferred).
- **O5.** A description that cannot be located is dropped uncounted (`walk.rs:518-520`).
- **O6.** Pass B has no structural rule of its own, for example:
  - a `forwarding` formal belongs to its last callee;
  - a `conditional_raise`'s raise lies in the `if` its condition tests.

  `semantic:witness-chain` covers its paths, and the snapshot covers the fixture. Such a pair
  would reject a mapping regression on the pilot.
- **O7.** The argument → formal rule is an inline fragment (`flows.rs:138-143`). In the working
  tree at review time, the uncommitted slice-2.2 `handoffs_sql` (not reviewed) re-expresses it
  **without** the `*`-argument guard. Extract one fragment before 2.2 commits (charter §E).
- **O8.** STATUS.md says 2.1 fired C2 O2, C2 O4, C4 O3 and C6 O2, "noted in the 2.1 commit".
  The commit message does not note them, and no review records a disposition (see Deferred).

**Applicability.**
- **Group 5 (derivation) and Group 2 (validity, absence)** carried F1, F3, F4 and F5.
- **Group 9 (adapters: Pyrefly's parser)** carried F2.
- **Group 10 (lineage)** carried F3.
- **Group 12 (claims)** carried F6 and F7.
- **Groups 3 and 7 (identity, reuse):** checked, with no defect (G6).
- **Groups 6 and 8:** no new effects, workspaces or performance claims. The pilot time was not
  re-measured.
- **Group 11:** the schema migration is an append with a snapshot. The codebook appends are
  verified append-only: finding kinds 6–8, member roles 1–4, assertion kinds 6–8,
  `pass_b_flows` = 1.

## 8. Alternatives (brief, at compact depth)

- **Chosen: declared SQL relations plus a small Rust worklist.** This is proportionate (charter
  §F). The relations are inspectable and digested, and the kernel is ordinary code behind them.
- **Simpler alternative: at depth ≤ 2, Pass B as two self-joins of the flows relation plus a
  guards join, with `row_number()` for the first path.** It removes about 150 lines of Rust. But
  it moves witness choice and deduplication into window functions and must be rewritten if the
  depth becomes a budget. Not better.
- **Where leverage is actually missing:**
  - the mapping fragment and path-qualifier rule, which Pass C and every future template will
    need (F1, O7);
  - the receiver helper (F8).

## 9. Verification: label audit and top gaps

| Claim (DESIGN) | Label claimed | Label supported | Evidence or gap |
|---|---|---|---|
| Mapping: positional, keyword, keyword-only, bound receiver, alias, literal | Tested | **Tested** | `pass_b_finds_the_known_answers_on_analysis_shapes` |
| Mapping: `init` with arguments, class receiver, `*` exclusion, depth 2, Pass B depth stop | Tested | **Implemented** (pilot-observed) | F6; this review's pilot probes |
| "never when an enclosing handler may catch it" (§9.2, Proposed half) | none | **Violated** for `with` | probe 2 (F1c) |
| `control`: "each overridable hop said" | Tested | **Implemented partially**: said once, unattributed; absent in `restriction` | F1 |
| `parameter_docs`: "the description's verbatim bytes" | Tested | **Tested** for the parsed text's bytes: the unit test, the fixture byte check, 1,580/1,580 pilot spans modulo whitespace (probe). **Not** "the description" | F2, F3 |
| Pilot: 35 `forwarding`, 4 `conditional_raise` | Measured | **Measured** | re-queried on `7799813f` |
| Pass B determinism across module order and location | not claimed in §9.2 | **Tested** | the Pass A-named test compares every analysis row; worth saying in §9.2 |

**Top gaps**, where a new oracle should land: F1's shapes; F2's truncation case and pilot count;
F3's swap case; F4's trace; F6's fixture shapes.

## 10. Exceptions and deviations assessed

- **D17 (Pass B reuses `[pass_a] max_depth`; the abstraction review is folded in): sound.**
  - The pre-registered config stays frozen.
  - Each Pass B invocation records `max_depth` in its parameters.
  - This review is the folded abstraction review. §9.2's Proposed half needs F4 (§4.2.4 versus
    declining), F5 ("supported predicate"), O2 (visited key) and F1's handler clause settled.
- **D18 (no promotion; literals from the seed only): sound as a narrowing.** The code matches it.
  The policy does not (F7).

## 11. Decision and implementation changes

**Decision: Revise (small).** Pass B's mapping is sound on every case examined (§2–4), its
identity and determinism hold (G6), and no §B decision is reopened. G2 and G7 fail on published
text: a restriction states less than its path knows and more than the seed can trigger (F1).
Pyrefly's truncation enters the raw table unflagged (F2). §4.2.4's promised trace is missing
(F4). G3 fails latently on description attribution (F3). **U1** (F4, mark or restate) is the
author's decision.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1** | F1: a path-qualifier rule for `restriction` and `control` (per-hop modality; `with` treated as a handler; branch-conditioned calls declined or qualified); `TEMPLATE_VERSION` bump | DM-24, DM-59 | The `Registry.add` restriction carries the qualifier; `strict` and `muted` publish no restriction, or a qualified one | Fixture shapes plus assertions in `templates_say_what_the_findings_show` |
| P2 | F3: header-anchored, self-checking `description_span`; the parameter-keyed rule with a swap case | DM-07, DM-46, DM-53 | Probe 1's literal yields `out`'s own line, or no row | Unit test; injected case |
| P2 | F2: detect a continued description; extend it or drop it and count; §3.2 says which | DM-42, DM-59 | `require_authorization_consent` is whole or absent; the pilot count is printed | Unit test; pilot smoke line |
| P2 | F4 with U1: record declines (a finding plus a Limits line) or amend §4.2.4 | DM-08, DM-02 | FastMCP.tool's `meta` and `task` are either traced or covered by a stated model sentence | Assertion in the Pass B known-answer test |
| P3 | F5: define "supported predicate" (code or DESIGN) | DM-59 | §9.2 and `guards_sql` agree | Fixture guard on `obj.closed` |
| P3 | F6: `init`, class-receiver, `*` and depth-2 fixture shapes, or relabel; fix the `*` sentence | DM-59, DM-54 | The snapshot shows each shape | `pass_b_configure` |
| P3 | F7: narrow `control` and `restriction` to `structurally_observed` until 3.4 | DM-07 | Policy snapshot | Contracts snapshot |
| P3 | F8: one receiver helper from the declaration kind | DM-06, DM-02 | Three call sites use it | Unit test |
| P3 | O7: one mapping fragment shared by Pass B and Pass C before 2.2 commits | DM-02, DM-52 | `handoffs_sql` uses it, `*` guard included | Pass C's known-answer test |

### Deferred

| Item | Status | Why not now | Reopen when |
|---|---|---|---|
| Increment-1 deep review O4: a byte-level evidence rule | **Fired by 2.1** (two new evidence sources: parameter-description spans, and Pass B's code spans as `fact` evidence), not recorded in its disposition | The fixture byte test covers both kinds. This review verified 1,580 pilot spans. DataFusion's `substr` is char-based, so an SQL rule cannot slice bytes | F3's fix lands (add the check in Rust there), or a second library |
| C4 O3: pydantic constraints and record-field controls | **Fired by 2.1** (Pass B's control recognizer landed) with no disposition. Pass B reads no `record_fields`, so §3.2's "Consumers: … controls from record fields" is unbacked | No seed's parameters are a record's fields | A seed whose controls are record fields (a dataclass or pydantic constructor, `**kwargs: Unpack[TD]`); narrow §3.2's consumer wording until then |
| C2 O2: handler types in field `test` | Fired; **satisfied** | `guards_sql` joins tests to `StmtIf` parents only | Closed |
| C2 O4: literal argument kinds | Fired; **answered** by C6's placement | Flows read `argument_value` → `syntax_nodes.kind` | Closed |
| C6 O2: reference → name-node edge | Not fired | Pass B joins the column in SQL, not in a graph | Unchanged |
| Increment-1 deep review O2: `span` must lie in a docstring | Not fired (agree) | Pass B cites code as `fact`; parameter spans lie in docstrings by construction | Unchanged |
| 1.4 review §8: a petgraph-free BFS | Approaching | Pass B is a second analysis without the `Graph` container | Unchanged (increment 2's end) |
| ADR-0019 review O6: in-memory composition of Stage E methods | Not fired | Pass B reads Pass A's projection mask and depth budget, not its findings (see O4) | Unchanged; record the projection in Pass B's invocation when it fires |
| O3: decorated targets | New | No signature-changing decorator on a pilot flow target | A flow target whose decorator is outside a signature-preserving list, or a second library |
| O4: projection digest per Pass B invocation | New | The compiler digest covers the snapshot | The first §9.8 ablation compares Pass B invocations (3.3) |
| Async and generator callees raise at await or iteration, not at the call | New | "Raises" is true when the body runs | A restriction on a generator or un-awaited coroutine reaches a brief |
| D18's reversal: promotion by documented warnings | Standing | No recognizer | Slice 3.4, together with F7's widening and its rule |

**Final check.**
- The claims match the evidence except §9.2's hop and handler sentences and §3.2's "description"
  (F1, F2).
- The supported scope is right for the mapping. It is too wide for restrictions until F1 lands.
- The extension path (Pass C) is clear once the mapping fragment and path qualifiers are shared
  (O7, F1).

## Disposition (2026-09-23, commit after slice 2.2)

| Item | Disposition | Where |
|---|---|---|
| F1 | Fixed. One path-qualifier rule for `control`, `transformed_control` and `restriction`: each override-open hop named, each call its caller makes only on some paths said (`conditional_call` members). A raise is declined when a call on the path sits in a `try` **or `with`** of its caller (`may_catch`), or in a construct of its caller that also reads the flowing value (`value_tested`). `TEMPLATE_VERSION` 8, `COMPILER_OUTPUT_VERSION` 7 (D27) | `flows.rs`, `pass_b.rs`, `synth.rs`; `controls.py` (`strict`, `muted`, `slow`); `templates_say_what_the_findings_show` |
| F2 | Fixed by extension (D27). The locator runs an entry to its end by indentation; its text is then those lines. Pilot: the 46 truncated descriptions are whole, and no other row moved | `walk.rs` `locate_description`; `a_description_cut_at_a_colon_line_is_extended`; DESIGN §3.2 |
| F3 | Fixed. The entry is anchored on its own header line and accepted only when its lines equal (or extend) Pyrefly's; two qualifying headers give none. The rule joins evidence → `parameter_syntax` → that parameter's `parameter_docs` row, with a span-swap injected case | `walk.rs`; `a_description_is_anchored_on_its_own_header`; `rules.rs`; `the_analysis_rules_reject_their_violations` |
| F4 / U1 | Decided: record (D26). `unfollowed_argument` findings (`rebound`, `computed`, `unmapped`) and one `unfollowed_control` Limits line per parameter; §4.2.4 says the trace is a finding. Pilot: `FastMCP.tool`'s `meta` and `task` are now stated | `flows::parameter_reads_sql`, `pass_b.rs`, `synth.rs`; `pass_b_finds_the_known_answers_on_analysis_shapes` |
| F5 | Fixed in code and DESIGN: a supported predicate is parameters, builtins and literals under comparisons, boolean, unary and binary operators, tuples, lists and sets, with builtin callees only | `guards_sql`; `probe_attr` in `controls.py`; DESIGN §9.2 |
| F6 | Fixed. `controls.py` gains a class receiver (`Registry.create`), a constructor (`Widget(name)`), `swap(*extra, name)`, a depth-2 chain and the depth stop, asserted in the known-answer test. The `*` sentence is corrected | `controls.py`; DESIGN §9.2 |
| F7 | Fixed. `control` and `restriction` permit `structurally_observed` only until 3.4 widens them with its rule | `findings.rs`; DESIGN §10.2 |
| F8 | Fixed. `flows::receivers_sql` decides the receiver by Pysa's method kind; the seed parameters and the brief's parameter list both use it | `analyze.rs`, `synth.rs` |
| O1 | Fixed. The flows' arcs take `projection::invocation()`'s accepted evidence | `flows.rs` `accepted` |
| O2 | Fixed in DESIGN §9.2 | — |
| O5 | Fixed. An unlocated description is a `signatures` boundary (`provider_disagreement`), so it is counted. Pilot: one (`truncation_suffix`) | `lib.rs` |
| O7 | Fixed. One `maps_formal` fragment, `*` guard included, serves Pass B and Pass C | `flows.rs` |
| O3, O4, O6 | Deferred with this review's triggers | — |
| O8 | STATUS.md is rewritten at the next handoff; this table records the firings | — |
| Deferred rows | Deep review O4 and C4 O3 are dispositioned as this review's Deferred table says; §3.2 now says record-field controls are not yet read | DESIGN §3.2 |

Pilot after the fixes (Measured, 2026-09-23, `just pilot`): snapshot `c9309c78`, 5 briefs,
generation `492754c864af6ad2`, stdio smoke passed, 33.0 s, peak 4,022 MiB.
