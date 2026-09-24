# Handoff: the remaining scope of the forward plan

**For:** the next session, resuming
[`behavioral-model-forward-plan_2026-09-24.md`](behavioral-model-forward-plan_2026-09-24.md)
after a machine restart.
**Written:** 2026-09-24, at HEAD `9a97790`. The tree is clean except for the operator's untracked
`docs/full_cpg_pipeline_external_review.md`.
**Authority:** none. The plan, DESIGN.md and the ADRs decide. This document holds the context a
fresh session would otherwise have to rediscover: where each remaining step touches the code, the
design details already worked out, the pitfalls already hit, and the probe material the reboot
deletes (the session scratchpad lived under `/tmp`).

---

## 0. Resume in this order

1. Read `STATUS.md` and AGENTS.md. The review standard changed: ADR-0023 (below).
2. Read the forward plan: §4.1 (the condition model) and §5, Stage 2.9.
3. Read this handoff's §2 (Stage 2.9, step by step) and §8 (pitfalls).
4. Start at **Stage 2.9.2**.

## 1. State at handoff

**Commits since the last pushed state.** Nothing is pushed; push only when asked.

| Commit | What |
|---|---|
| `9a97790` | Handoff: `STATUS.md` |
| `24f381a` | ADR-0023 follow-up: "principles §D" in `scripts/adr.py` and its test; plan citations re-keyed |
| `4fa7978` | The forward plan, the external-review assessment, deviation B23 (D-10 to D-13 approved) |
| `4a79501` | Stage 2.9.1: the Stage 2 end review's fixes R1–R11 (see §1.1) |
| `fc3942d` | (operator's session) ADR-0023, the layered design standard |
| `3129c7c` | (operator) a snapshot of part of the end-review fixes, mid-work |
| `a7e1e2c` | Stage 2 follow-ups and the Stage 2 structured evaluation |

**Last verified (2026-09-24):**
- `just test-all` passed at `24f381a`: nextest 262/262, pytest 80, rule tests 7/7, lint-agents,
  adr lint (23), fixtures 65, deps, shear, gold.
- `just pilot` passed on `4a79501`'s tree: snapshot `8f40e20ab5fdcfb3ba654c63731d9fbd`,
  generation `4c50ccdc2710b122`, 41.5 s, smoke 20/20.

**Pilot baseline** (snapshot `162bda5a…`, the same code before the test-attribution rule):
- operations: 636 established, 535 unknown, 363 not analyzed (classes);
- behaviors: 15,415 in all. By verdict, 4,276 established, 3,554 conditional and 19 refuted. The
  unknown ones, by reason:
  - 6,137 `call_transfer`;
  - 1,347 `override_dispatch`;
  - 42 `budget_reached`;
  - 39 `abstract_body`;
  - 1 `runtime_unreachable`;
- flow facts: 66,954 uses; 69,010 reaching rows; 6,148 tests; 13,764 attribute loads; 11,184
  conditions;
- flow model: 1.73 s.

**Stores.**
- `build/store` is the live pilot store.
- The moved-aside stores `build/store-pre-{a1,stage1,revise,stage2.6,b15,stage2review}-2026-09-24`
  (about 11 GB) are kept for the operator to delete. Never delete them unasked.
- About 189 GB were free on the disk.

### 1.1 What Stage 2.9.1 built (for orientation)

| Where | What |
|---|---|
| `crates/cpg-flow/src/lib.rs` | Emits `tests` (every ty predicate expression, per span, with its condition) and `attribute_loads` (every attribute load by name, plus `getattr`/`hasattr` with a literal name) |
| `crates/cpg-flow/src/predicate.rs` | Versioned places (`place@line`) where a scope tests a rebound place at two versions (a per-scope survey of ty's predicates: `versions_tested`); opaque atoms carry a version; the runtime view (`runtime`, `:301`) |
| `crates/cpg-schema/src/condition.rs` | `Atom::Opaque { text, version }`, encoded `opaque("…")@line`; resolution over sorted conjunctions; `given` returns `self` past the budget |
| `crates/cpg-schema/src/tables.rs` | `flow_tests`, `flow_attribute_loads` (flow family) |
| `crates/cpg-core/src/flow_model.rs` | A three-way `Transfer` (Identity, Derived, Call); `reach` with a lowlink memo (`:622`); loop-carried steps keep the use's side; `guards_of` and `normal_path` (`:907`, `:929`, the guards' product factored first); raise `escapes` (`BUILTIN_EXCEPTIONS`, `may_catch`, frames of `try`/`with suppress`); `unreachable` declarations; `not_behavior` (stub, raise-only, abstract); `overridden`; test attribution through `flow_tests` (identity and derived transfers only); `__dict__` dynamic accesses |
| `crates/cpg-core/src/behavior.rs` | `runtime_unreachable` status and claims; `raises_when` only for escaping raises; unmapped arguments name their release callee |
| `crates/cpg-schema/src/rules.rs` | `semantic:condition-not-false`, `premise-no-attribute-load`, `refuted-not-overridden`, `unreachable-not-established` (injected cases in `crates/cpg-core/tests/analysis.rs`) |
| `fixtures/python/behavior_shapes/release/bpkg/probes.py` | The end review's probes; known answers in `crates/cpg-core/tests/behavior.rs::the_stage2_end_review_probes_get_their_answers` |
| Codebooks | `boundary_reason` 19 `call_transfer`, 20 `abstract_body`; `EXTRACTOR_OUTPUT_VERSION` = 22 (`crates/cpg-extract/src/config.rs:31`); the all-techniques guard `f55cbc21…` (`crates/cpg-core/tests/syntax.rs:1769`); the table count `48 + 21 + 32` (`crates/cpg-core/tests/compile.rs:93`) |

## 2. Stage 2.9, step by step

The plan's §5 Stage 2.9 lists the steps. What follows is the detail already worked out for each.

### 2.9.2 Evaluation identity (X1; decision D-10)

**The defect.** An atom is keyed by kind + spelled place + literal, or by opaque text, so two of
ty's predicates with the same text merge into one Boolean variable. Eight shapes reproduce this
(§7.1). CPython 3.14.7 executes every statement the translator decided `false` (§7.2).

**The target** (plan §4.1):

- **Identity.** An atom is kind (the Python operator kept) + operands + evaluation site. Its text
  is a label.
- **Operators kept:**
  - `is None` → `is_none`;
  - `== None` → `equals(p, None)`, which is no longer `is_none`;
  - `is True`, `is False` and `is <literal>` → a new identity atom. Append it to `condition_atom`
    (`codebook.rs:1267`; the next free code follows `Opaque`, never renumber). Suggested name
    `is_value`.
  - `!=` and `is not` are the negations.
- **Sharing across sites** is allowed only for `is None`, `is <literal>` and `isinstance` on a
  **name** (not an attribute place) at the same **definition set**. Everything else is per site:
  - truthiness, `==` and `in`;
  - any attribute place, which a call may mutate;
  - calls and all other opaque tests;
  - the synthetic predicates: `NON_EMPTY`, `SUPPRESSES`, `FINALLY` and `UNDECIDED` in
    `predicate.rs`, which are fixed strings today.
- **Versions** are definition sets, not lines. In `predicate.rs::version` (`:156`), take the
  reaching bindings' identities (for example, their `target_range` start bytes, sorted) instead of
  the max line. The display may still show the latest line. `versions_tested` (`:101`) and the
  opaque versioning (`opaque`, `:64`) then key on the set.

**Encoding: a design point still open.** A per-site atom needs a site in its encoding and its id.
`conditions` rows are content-addressed and module-agnostic, so a site alone (a byte offset)
could collide across modules. Options:
- (a) the site as `@<start byte>` in the encoding, plus the module in the atom's identity input;
- (b) the site as a line and column for display, with identity from (module id, span).

Stage 3's summaries translate a callee's conditions into the caller's places, and there per-site
callee atoms stay foreign atoms. **Recommendation:** a site marker that carries (module, start
byte) into the id, and a display of `L<line>`. Record the choice in ADR-0022 §Conditions.

**Consumers that key on atom encodings, to check after the change:**
- `flow_model.rs`'s `atom_reads` (test attribution) and `raise_sites.parameters`;
- `normal_path`/`given`: a guard and the fates it gates come from one predicate site, so
  factoring still works;
- the `tested` map (`tests` fates);
- `Condition::parse`/`encode` (the round-trip is tested in `conditions.rs` and the unit tests).

**Tests** (keep them to what pays for itself; the operator prefers judgment to ceremony):
- a `flow_shapes` case per §7.1 shape, plus same-line bindings (`x = a; x = None`) and a merged
  definition;
- the pinned `flow_shapes` snapshot moves;
- `conditions.rs` known answers for the new encodings.

**Migrations:** `EXTRACTOR_OUTPUT_VERSION` 23; the all-techniques guard moves as a declared
migration; schema snapshots if a column changes.

### 2.9.3 The runtime view by resolution (X2)

**The defects:**
- `predicate.rs:304-305` decides any name spelled like the sentinel, or dotted `X.TYPE_CHECKING`,
  as false;
- `:308` and `:327-329` recognize `sys` and `os` by spelling;
- `:315-316` truncates the context version to the literal's length and compares prefixes.

**The fixes:**
- **Resolved spans.** `crates/cpg-extract/src/lib.rs:601` builds each `flow::FlowModule` inside
  the per-module loop, where that module's lexical output (`lexical.rs` `LexicalOut`: bindings,
  references, resolutions) and its imports (`export_syntax`'s `imported_module` and
  `imported_name`) are available. Add to `FlowModule` and `cpg_flow::Input` the spans of names
  that resolve to `typing.TYPE_CHECKING` or `typing_extensions.TYPE_CHECKING` (aliases included),
  and of `sys`/`os` names bound to the stdlib modules.
- **The translator** decides a sentinel false only when its span is in the set; otherwise it is
  an ordinary truthiness atom spelled `TYPE_CHECKING`. `sys` and `os` tests are decided only for
  resolved spans.
- **Keep the rename.** It stops ty deciding the name itself, by spelling.
- **The version fix.** `sys.version_info` has five fields, so a literal tuple that is a proper
  prefix of the context version is **less**. With an equal prefix and a shorter literal, the
  ordering is `Greater`.

**Cases:**
- `def choose(TYPE_CHECKING): if TYPE_CHECKING: …` is not decided;
- `from typing import TYPE_CHECKING as TC; if TC:` is decided;
- `config.TYPE_CHECKING` is not decided;
- `>`, `>=`, `<=`, `==` against `(3, 14)` under 3.14.7 give True, True, False, False.

### 2.9.4 Counted skips (X3)

`cpg-flow/src/lib.rs:601` (reaching rows) and `:697` (value sources) skip a condition that is
`is_never()`. Count the skips per module by cause:
- ty's own always-false;
- the runtime view;
- a stable-atom contradiction.

Put the counts in the module's flow `coverage` row: `detail` is free text, with the columns
`status`, `reason` and `detail`. Do not add a lint.

### 2.9.5 The runtime soundness oracle (X6; decision D-12)

**Invariant.** Every executed statement's region is admitted (not `false`), and every observed
reaching definition is among the flow facts. Value flows join at Stage 3.5.

**Parts:**
1. **A developer subcommand, `lctx flow <file.py>`**, printing the flow facts of one file as JSON:
   regions with spans and conditions, uses, definitions, and reaching rows. `crates/lctx/src/main.rs`
   has `Library`, `Acquire`, `Compile`, `Bundle`, `Diff` and `Query` today. `cpg_flow::index` takes
   `Input { path, text }` and a `RuntimeContext`.
2. **A pytest driver** (in `tests/`, or beside the Python tests). It generates or loads small
   programs, runs each in a subprocess on the pinned interpreter (`uv run python`, 3.14.7) under
   `sys.monitoring` LINE events (script in §7.2), with inputs chosen to drive both branches of each
   test, and asserts that every executed line's statement region is not `false` in `lctx flow`'s
   output.
3. **Hypothesis**, added as a Python dev dependency: a small grammar of tests, rebindings, `for`
   over `range`, `try`/`with`, calls with mutation (`reset()`, `clear()`), `==` against `is`, and
   version checks. The seed corpus is §7.1's shapes plus the end review's probes.
4. **Isolation:** generated programs only; never `fixtures/python/` (never executed) and never the
   analyzed library; a temporary directory, a timeout, no network.

Keep it a test, not a lint (operator preference).

### 2.9.6–2.9.7 The stated model, and ADR-0022 amended in place (it is `proposed`)

- **§Conditions:** evaluation atoms, the preserved operators and the new atom, the sharing rule,
  counted skips, the encoding choice.
- **§Places:** definition-set versions. Mutation and impure calls are why sharing is limited.
- **§Composed layers:** the runtime view by resolution; version-tuple semantics.
- **§Verdicts:** the approximation direction. Positive verdicts are may-behavior the model admits:
  AMBIGUOUS is read as true, calls are assumed to return, operators, f-strings and containers
  work over primitive operands, and context managers other than `suppress` are assumed not to
  suppress. A negative holds under a complete may-analysis. Say the same in the tool
  descriptions (`python/lctx_mcp/src/lctx_mcp/server.py`).
- **While editing ADR-0022,** re-key its old citations: DM-nn → DP-nn or CI-nn, "charter §D" →
  "principles §D", "ADDENDUM §n" → "binding §n" (maps: `core/design-principles.md` §I,
  `binding/library-context.md` §8).
- **DESIGN §3.9** follows.

### 2.9.8 Re-apply Stage 2's exit rule

1. Run `just pilot`.
2. Check the GPU: `nvidia-smi --query-gpu=memory.free --format=csv,noheader`. It must show at least
   30 GB free, or the live leg is `blocked`.
3. Start `just embed-serve` in the background, and wait for it:
   `until curl -sf http://127.0.0.1:8000/v1/models; do sleep 2; done`.
4. Run `just pilot-live`.
5. Run `just structured-eval build/generations/<gen> 2 http://127.0.0.1:8000`. It writes
   `build/structured/stage2.md`.
6. Stop vLLM: `pkill -f "[v]llm serve Qwen"`. Keep the brackets, so the pattern does not match its
   own command line.

Rate Q04, Q10 and Q21–Q24 against the rule in `eval/behavior/fastmcp-4.0.5.toml`. Add a dated
addendum to `docs/design_review/reviews/structured_eval_stage2_2026-09-24.md`.

**Expected changes since attempt 2:**
- more `tests` fates;
- no raises where the function catches them;
- `transport` no longer "tests" `self.transport`;
- conditions with `@` versions or site markers.

A failure is fixed within Stage 2.9. ADR-0021's revisit needs two consecutive failures.

### 2.9.9 Compact re-review, then accept ADR-0022

- **The reviewer:** the `design-reviewer` subagent, which now loads `design-review` and
  `design-review-code-intelligence` (ADR-0023). It cites DP-01–24, G1–G8, CI-01–13, CI-G1–CI-G3
  and `binding/library-context.md`.
- **Scope:** the end review's R1–R8 fixes, and X1, X2, X3 and X6.
- **This is the first review under the new standard.** Record any friction (a missing slot, an ID
  that does not fit, a principle needing a repository reading) in its Deferred table, and tell the
  operator. That is ADR-0023's revisit trigger.
- **Accepting:** edit ADR-0022's frontmatter `status: proposed` → `accepted`. There is no
  `adr accept` command. Run `just adr index` and `just adr lint`. An accepted ADR is immutable.

### 2.9.10 Handoff

Run the `handoff` skill. Keep `STATUS.md` to 60 lines or fewer.

## 3. Stage 3 (increment 4's remainder)

### 3.0 The condition kernel (decision D-11)

- **A new ADR** (`just adr new`) amending §B10's "not a solver" wording: propositional decision
  diagrams and a typed theory for primitive places are allowed; theory solvers stay excluded. It
  also amends DESIGN §3.9 and §9.9. A `standard` review follows.
- **The library:** `biodivine-lib-bdd = "=0.6.3"`. MIT. Its dependencies are already locked except
  `num-rational` (`rand` 0.8.8, `num-bigint` 0.4.8, `num-traits` 0.2.19 and `fxhash` 0.2.1 are).
  `deny.toml` allows multiple versions.
- **Its API** (docs.rs, read 2026-09-24):
  - `BddVariableSet::new_anonymous(n)`, `.variables()`, `mk_literal(var, bool)`, `mk_true()`,
    `mk_false()`;
  - `Bdd::and`, `or`, `not`, `imp`, `iff`, `and_not`;
  - `var_restrict`, `restrict(&[(var, bool)])`, `exists(&[vars])`;
  - `is_false`, `is_true`, `size`, `sat_witness`;
  - `to_optimized_dnf()`, which returns `Vec<BddPartialValuation>`;
  - `Bdd::binary_op_with_limit(limit, a, b, op_function::and)`, which returns `None` past the
    limit;
  - `Bdd` implements structural `Eq`, and a fixed variable set gives canonical forms.
- **The probe** (§7.3) measured it and confirmed canonicity, factoring by equivalence,
  compatibility, the limit and deterministic rendering.
- **Spike exit tests:**
  - ids reproduce `crates/cpg-schema/tests/conditions.rs`'s known answers under a deterministic
    variable order (atoms sorted by identity);
  - normal-path factoring by equivalence matches Stage 2's `given` (G with G ∧ N ≡ F ∧ N);
  - determinism under shuffled input;
  - a node-limit case;
  - on the pilot, `budget_reached` (42) falls, and the limit's hits and compile time are reported.
- **Then:**
  - lossless persisted conditions: a node table in the `flow` family, with ids from the canonical
    BDD. The DNF encoding becomes a rendering with a display budget.
  - the typed theory (D-13): equality, membership and truthiness atoms are shared, and literal
    exclusions added (`x == "a"` excludes `x == "b"`), only where Pyrefly's type at the use is a
    builtin immutable scalar (`str`, `int`, `bool`, `None`, their literals and unions). The source
    is the `type_observations` table (`tables.rs:1044`), joined by span.
  - an `implies`/`compatible` API;
  - the `approximated` flag (a diagram path crossed AMBIGUOUS or an assumed return).

### 3.1 Models catalog v1

Files: `crates/cpg-schema/models/{external,frameworks}.toml`, in the access-path grammar, with
append-only ids and `origin = synthetic_model`.
- **The list:** the superseded plan's D-7 list (the forward plan, Stage 3.1), plus `typing.cast`
  (identity: argument 2 → return) and the builtins that carry a value (`str`, `dict`, `list`,
  `tuple`).
- **Guards:** the catalog's digest joins `compiler_digest` (`crates/cpg-core/build.rs` hashes the
  sources); no model cites `.claude/skills`.
- **CrossHair 0.0.110** checks each model of a pure callable: `crosshair diffbehavior mod.real
  mod.model --max_uninteresting_iterations 5`, in an isolated venv and worker. It executes code.
  §7.4 has the probe.

### 3.2 The L2 remainder (not built in Stage 2)

- `handlers`: caught types, and the action (re-raise, convert, swallow, value);
- `callbacks`;
- `resources`;
- exits: normal, exceptional, `finally`.

`raise_sites.escapes` and `flow_model.rs`'s catch frames are a start: reuse `may_catch` and
`BUILTIN_EXCEPTIONS`.

### 3.3 Summaries: resolving `call_transfer`

**Today.** A use inside a call is `through_call`. A claim reached only that way is `unknown`
(`call_transfer`): 6,137 on the pilot.

**The design:**
- Treat call results as intermediate values. Each call's arguments are already `Argument` sinks
  (`flow_values`); `argument_at` in `flow_model.rs` maps an argument span to (argument node, call
  node, formal).
- A callee's summary (formal → `ReturnValue`, field writes, `Raise[T]`) resolves a flow through
  the call:
  - transfer present → `established` or `conditional`;
  - summary complete and no transfer → the claim is dropped, or refuted where a premise holds;
  - summary incomplete → `unknown` with its reason.
- A receiver goes through `Parameter[self]`. Nested calls compose innermost first.
- **Order:** SCCs bottom-up (petgraph `tarjan_scc`; see the `rust-graphs` skill). A fixpoint per
  SCC, bounded by path depth k and the kernel's node limit, widening to `unknown`
  (`budget_reached`). Joins over override-open candidates. Invocation rows record budgets.
- **Tables:** `summary_flows`, `summary_effects`, `summary_boundaries` (DESIGN §9.9 has the
  access-path grammar). Report the `call_transfer` count after.

### 3.4 Ascent

A spike only on its trigger: three or more recursive rule families repeating the worklist shape.
ascent 0.8.1 is in the cargo registry cache. Its relations are `Vec`s with FxHash indexes, so a
single-threaded run is deterministic.

### 3.5 The Pysa oracle (deviation B21)

**What was verified:**
- Pysa lives at facebook/Pysa and still ships as `pyre-check`. 0.10.0 depends on `pyrefly`, uses
  Pyrefly by default (`--use-pyre1` opts out), and has a manylinux wheel.
- It picks up whatever `pyrefly` is on `PATH` (`~/.local/bin/pyrefly` 1.3.1), so pass
  `--pyrefly-binary`.

**The spike's setup** (a scratch venv on Python 3.12). Each project directory needs:
- `.pyre_configuration`: `{"source_directories": ["."], "search_path": ["<site-packages>"], "taint_models_path": ["models"]}`;
- `models/taint.config`: `{"sources": [], "sinks": [], "features": [], "rules": []}`.

Run `pyre --noninteractive analyze --no-verify --save-results-to ../out --infer-self-tito`.
`taint-output.json` holds one JSON object per line; models are `{"kind":"model","data":{…,"tito":[{"port":"formal(x, position=0)[field]", …}]}}`.

**Measured on FastMCP 4.0.5** (copied `fastmcp` and `fastmcp_tasks`, with the pilot environment's
site-packages as the search path): 8.5 s wall, 702 MiB peak, 2,051 models, 6,054 TITO ports,
4,412 carrying `obscure` features. It confirmed that `client_name` does not reach
`_parse_call_tool_result`'s result.

**Never copy Pysa's obscure-callee default:** an `obscure` or `broadening` port is not our
`established`.

### 3.6–3.7 `behavior_shapes` part 2 and tools

- the "used only for logging" case, and a wrapper that disables an option of its callee;
- the runtime oracle extended to observed value flows;
- summarized flows and effects in `get_operation`;
- effect and role filters in `find_operations`;
- a compatibility filter ("fates compatible with `transport == 'sse'`"), which Q09 needs.

**Exit:** `behavior_shapes` part 2 passes, and Stage 3's pre-registered rule over Q01, Q03, Q05
and Q09 passes. Then increment 4's `compact` review.

## 4. Stages 4 and 5, and closing increment 5

These are as in the forward plan's §5. The pointers:

- **Stage 4:**
  - the registry in `cpg-schema` (TOML with `deny_unknown_fields`), definitions as a Rust enum AST
    compiled to DataFusion SQL, using the kernel's `implies`/`compatible`;
  - `concept_members` with witnesses;
  - `lookup_concepts` lists every concept while there are few; `explain` gives a witness chain;
  - FCA as candidate facets only.

  Pre-register the concept queries in `eval/behavior/` **before** reading Stage 4's output.
- **Stage 5:**
  - framework models (name registries, middleware chains, lifespan, ContextVars, `__fastmcp__`
    metadata);
  - deferred execution: a coroutine's creation is not its execution. The read phase needs "when it
    runs" for Q07, Q08 and Q11;
  - protocol operations, only on their trigger;
  - protocols mined from usage, labelled *observed pattern*;
  - rules parameterized by user code (Q02, Q11);
  - Graph-FCA offline.
- **Closing increment 5:**
  - unseal `eval/heldout/` (it has `MANIFEST.sha256`, `README.md` and `fixtures/`) and verify the
    manifest;
  - write targets before running;
  - run the packet on the final generation, and assess it in the same rubric;
  - decide the §B11 LLM trigger;
  - a `deep` review.

## 5. Evaluation (pre-registered: `eval/behavior/fastmcp-4.0.5.toml`)

| Stage | Questions (items) | Exit rule |
|---|---|---|
| 1 (done) | Q13–Q20 | — (rated "met in part"; operator review pending) |
| 2 (passed on attempt 2; re-applied at 2.9.8) | Q04 (7), Q10 (7), Q21 (4), Q22 (3), Q23 (3), Q24 (2) | No positive item incorrect or misleading; no negative claimed; at least half the positives present or partial |
| 3 | Q01, Q03, Q05, Q09 (7 each) | The same shape, over the stage-3 questions |
| 4 | Concept queries, to be written before Stage 4's output is read | — |
| 5 | Q02, Q06, Q07, Q08, Q11, Q12 (7 each) | The same shape, over the stage-5 questions |

- **The packet:** `just structured-eval <generation> <stage> [embed_url]` runs
  `scripts/structured_eval.py`. Search items rank lexical-only without the URL.
- **Assessments** go in `docs/design_review/reviews/structured_eval_stage<N>_<date>.md`, with the
  rubric present / partial / absent / incorrect / misleading. The assessor is the author; no API
  agents.
- **ADR-0021's revisit** fires if a stage's exit fails twice in a row.

**Stage 3's targets, already written** (see the TOML):
- **Q01:** `@mcp.tool` options (unchanged, transformed, consumed, which raise);
- **Q03:** `tasks=` defaults, where `strict_input_validation` acts (through a ContextVar at call
  time);
- **Q05:** error masking and conversion in `FastMCP.call_tool`, `mask_error_details` against the
  setting, `ToolError`, timeouts, mounted children, `raise_on_error`;
- **Q09:** options ignored or conflicting per transport. This needs the compatibility filter.

## 6. Constraints and operator preferences

- **Commits:**
  - small, to `main`, each naming its stage, ADR and test outcome, ending with
    `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`;
  - stage paths explicitly;
  - never push unless asked; never force-push or `reset --hard`;
  - don't commit the operator's files without say-so.
- **Snapshots:** read the `.snap.new` diff, then `cargo insta accept --workspace` (not `-p`, which
  fails). Never `cargo insta review`. A schema snapshot change is a declared migration: say so in
  the commit.
- **Codebooks are append-only.**
- **Reports:** passed / failed / blocked / not_run, with the command. Label design claims with
  principles §D (Proposed … Tested … Measured), and date them.
- **Evaluation:**
  - the gold (`.claude/skills/`) is never a compiler input;
  - `eval/heldout/` stays sealed until increment 5's end;
  - no API agents evaluate content.
- **Fixed files:**
  - `analytics.toml` is frozen (an edit is an ADR-0021 amendment);
  - don't edit `docs/design_review/design_principles/core/` (a verbatim copy);
  - keep the superseded banners.
- **Operator preferences** (memory, 2026-09-24):
  - no new lints for design alignment;
  - judgment over required probes, tests or records;
  - licence is never a criterion;
  - structured qualitative evaluation;
  - DESIGN.md is not trimmed for length.
- **GPU:** live legs only with ≥30 GB free, then `pkill -f "[v]llm serve Qwen"`.
- **Judgment calls** go to `docs/design_review/reviews/deviations_behavioral-model_2026-09-24.md`,
  from B24 onward.
- **Reviews** use the layered standard: `design-review` + `design-review-code-intelligence`,
  usually through the `design-reviewer` subagent. The cadence: `compact` per stage, `standard`
  for an ADR changing a §B decision, `compact` at increment 4's end, `deep` at increment 5's end.
- **Pending for the operator:**
  - the Stage 1 and Stage 2 evaluations;
  - deviations B1–B23;
  - the moved-aside stores;
  - the forward plan;
  - whether to commit `docs/full_cpg_pipeline_external_review.md`.

## 7. Probe material (the scratchpad did not survive the reboot)

### 7.1 The eight translation defects: shapes for `flow_shapes` cases

Under the context (3, 14, 7), the translator (P1, 2026-09-24) gave:
- `emit()` under `false` in `impure`, `mutated`, `cleared`, `eq_vs_is`, `eq_none`, `choose`, and
  the second `emit()` of `version`;
- the first `emit()` of `version` (the `<=` branch) under `true`;
- no `found = i` reaching row at `return found` in `two_ranges`.

```python
import sys


def impure(probe, emit):
    if probe():
        if not probe():
            emit()


def two_ranges(n, m):
    found = None
    for i in range(n):
        found = i
    for j in range(m):
        found = j
    return found


def mutated(self, emit):
    if self.x is None:
        self.reset()
        if self.x is not None:
            emit()


def cleared(items, emit):
    if items:
        items.clear()
        if not items:
            emit()


def eq_vs_is(x, emit):
    if x == True:
        if x is not True:
            emit()


def eq_none(x, emit):
    if x == None:
        if x is not None:
            emit()


def version(emit):
    if sys.version_info <= (3, 14):
        emit()
    if sys.version_info > (3, 14):
        emit()


def choose(TYPE_CHECKING, emit):
    if TYPE_CHECKING:
        emit()
```

### 7.2 The runtime check (P2), a seed for the 2.9.5 oracle

Run on CPython 3.14.7 with `uv run python`. The `emit()` lines that executed were exactly the
seven the translator decided `false`; the `<=` branch did not execute; `two_ranges(2, 0)` returned
1.

```python
import sys, types
SRC = open("shapes.py").read()                     # §7.1's source
code = compile(SRC, "m.py", "exec")
m = types.ModuleType("m"); exec(code, m.__dict__)
lines = set(); mon = sys.monitoring; TOOL = mon.PROFILER_ID
mon.use_tool_id(TOOL, "probe")
mon.register_callback(TOOL, mon.events.LINE,
                      lambda c, line: lines.add(line) if c.co_filename == "m.py" else None)
mon.set_events(TOOL, mon.events.LINE)
answers = iter([True, False]); m.impure(lambda: next(answers), lambda: None)
print(m.two_ranges(2, 0))                          # 1: the loop-1 value reaches the return
class S:
    x = None
    def reset(self): self.x = 1
m.mutated(S(), lambda: None); m.cleared([1], lambda: None); m.eq_vs_is(1, lambda: None)
class Weird:
    def __eq__(self, other): return True
m.eq_none(Weird(), lambda: None); m.version(lambda: None); m.choose(True, lambda: None)
mon.set_events(TOOL, 0); mon.free_tool_id(TOOL)
emit = [i + 1 for i, t in enumerate(SRC.splitlines()) if t.strip() == "emit()"]
print(sorted(l for l in emit if l in lines), "of", emit)
```

### 7.3 The BDD probe (P3)

A scratch crate with `biodivine-lib-bdd = "=0.6.3"`, built with Rust 1.98.1 in release mode.

**Confirmed:**
- R9's counterexample in two orders gives equal BDDs, rendered `!d | !b | a & c`;
- two site variables keep `p ∧ ¬p′` satisfiable;
- `F ∧ N == c ∧ N` (factoring by equivalence) holds;
- compatibility works;
- `binary_op_with_limit(1, …)` returns `None`;
- the rendering is deterministic.

**Measured:**
- 500 conditions (16 conjunctions of ≤8 literals over 20 atoms) built in 30 ms; 499 conjunctions
  in 15 ms; 50 `to_optimized_dnf` renderings in 77 ms;
- over 40 atoms: 499 conjunctions in 423 ms, 31 of them past a 50,000-node limit;
- random 32×8 conditions over 40 atoms did not finish in 10 minutes.

So a node limit and a bounded rendering are required.

```rust
let vars = BddVariableSet::new_anonymous(4);
let v = vars.variables();
let lit = |i: usize, pos: bool| vars.mk_literal(v[i], pos);
let f = lit(0, true).and(&lit(2, true)).or(&lit(1, true).and(&lit(2, true)));
let n = lit(0, true).or(&lit(1, true));
assert!(f.and(&n) == lit(2, true).and(&n));             // F given N is c
let limited = Bdd::binary_op_with_limit(50_000, &f, &n, biodivine_lib_bdd::op_function::and);
```

### 7.4 CrossHair (P4)

`crosshair-tool==0.0.110`, in a Python 3.14 venv.
- `crosshair diffbehavior ops.eq_true ops.is_true` found `x=1`, where `x == True` returns True and
  `x is True` returns False.
- `ops.eq_none` against `ops.is_none` found a class whose `__eq__` returns True.

### 7.5 Other library facts (2026-09-24)

- **OxiDD 0.12.0:** about ten sub-crates; fallible `AllocResult` APIs; manager capacities fixed at
  creation; multithreaded by default. Deferred.
- **z3 0.21.1 / z3-sys 0.13.1** (2026-09-23): the system `libz3.so.4` is installed, so it would
  link without building Z3. Deferred (§B10).
- **Hypothesis 6.168.1** and **bytecode 0.19.0** support Python 3.14.

## 8. Pitfalls already hit

- **`test-all` stops at its first failure,** so later steps show as not run. Re-run the whole
  suite before reporting.
- **Edits made through Python or shell** bypass the format hook: run `just fmt` before
  `just test-all`, or `fmt-check` fails.
- **`pkill -f <pattern>` can match its own shell command line.** Use the bracket trick
  (`[v]llm`), or `pgrep` and kill by PID.
- **The ast-grep rule `mdx-names-declared`** flags the string literals `"Warning"`, `"ParamField"`
  and `"body"` in `cpg-core/src`. `flow_model.rs` is ignored for Python exception names. A new
  file naming them needs the same treatment, or its strings built another way.
- **The Python test suite** (`python/lctx_mcp/tests`) runs against the `analysis_shapes` fixture
  generation. A semantic change such as R1's stub premise can break an expectation there (for
  example, `test_a_parameter_never_read_is_refuted_only_under_its_premise`).
- **The analysis diff snapshot** (`crates/cpg-core/tests/snapshots/analysis__diff_minus_fca.snap`)
  counts `analysis_shapes`' behaviors, so any change to what is attributed moves it.
- **A schema migration** needs a fresh store: move `build/store` aside and copy its
  `embedding_cache` and `embedding_specs` into the new one, as each earlier migration did.
- **clippy `-D warnings`:** `type_complexity` fires on tuple-heavy collections, so add a `type`
  alias; `field_reassign_with_default` fires on setting a field after `Default`, so use a struct
  literal.
- **A Bash call's working directory can drift** to a subdirectory. Use absolute paths or `cd`
  first.
