# Design review: CPG slice C2, the `syntax` family (compact)

**Date:** 2026-09-23 · **Depth:** compact · **Mode:** code plus DESIGN.md, at commit `5e3baf9`.
This is the end of a slice that adds a fact family, an extractor surface and three edge kinds, so
ADR-0001 owes a `compact` review.
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`5e3baf9`
**Prior review:** `design_review_adr-0014-cpg-graph-catalog_2026-09-22.md` and its Disposition.
C1 is not re-reviewed here except where C2 changes it (F2 reopens the shape of that review's F1
for a new column). Findings here are F1–F4 and O1–O6.

## 1. Decision and scope

**Target.** Commit `5e3baf9`, read with `git show 5e3baf9:<path>` and in a detached worktree of
that commit (the main working tree already holds uncommitted C3 work):
- DESIGN §3.2 (the `syntax` row), §3.8 (the C2 edges, rules and Measured block), §4.2.2;
- `cpg-extract`: `syntax.rs` (new), `walk.rs` (frames, placement, owner, subtree ranges),
  `pysa_map.rs` (`site_ranges`), `lib.rs` (walk after the collectors, `FAMILIES`);
- `cpg-schema`: `tables.rs` (`syntax_nodes`), `derived.rs` (`site_targets`), `graph.rs` (node
  source, `ast_child`/`argument_value`/`site_target`, `node_columns`, `placed:*`,
  `contained:syntax_nodes`, `typed:site_targets`, `graph_gaps`, the partition rule),
  `codebook.rs` (`syntax_kind`, `syntax_field`, appended kinds);
- `cpg-core/tests/syntax.rs`, its snapshot, and `fixtures/python/syntax_shapes/`.

**Observable outcome claimed.** A raw `syntax_nodes` table (Ruff, `ruff-ast`) holding every
statement, the clause nodes, the expressions Pysa reports sites at, any expression at a Pysa
non-call non-identifier site range, and the full subtree under `test`/`exc`/`cause`/`guard`/`msg`;
declarations and calls placed under their existing ids; a derived `site_targets` mapping each
Pysa attribute, artificial and format-string record to a placed node and a typed target; three
edge kinds; `graph_gaps` reduced to identifier sites. Consumers named: Pass B guards, raises and
handlers; Pass C straight-line regions; FCA raised types.

**Supported scope and non-goals.** Identifier sites and references wait for C3; nothing inside an
annotation is placed; types are C4's.

### Method and coverage

**Read in full at the commit:** `syntax.rs`; the `walk.rs`, `lib.rs`, `pysa_map.rs`,
`tables.rs`, `derived.rs`, `graph.rs` and `codebook.rs` diffs, plus `walk.rs` L120–213 and
L540–641, `pysa_map.rs` L480–560 and L660–820, `derived.rs` L556–583 and L725–795, `graph.rs`
L861–867 and L1040–1160; `cpg-core/tests/syntax.rs` and its snapshot; the fixture; the diffs of
`compile.rs`, the four derived-table snapshots, the graph snapshot, the rules and derivations
snapshots and the id-recipe snapshot; DESIGN §3.1–§3.8, §4.1–§4.2.3, §8, §9.1–§9.3, §9.6;
ADR-0014 L60–130; the ADR-0014 review's F1/F5 and Disposition; the guidelines' MUST lines;
ADDENDUM §2 and §5.

**Checks run in this session:**

| Check | Command | Outcome | Observation |
|---|---|---|---|
| Rust tests at the commit | `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass --offline` in a detached worktree of `5e3baf9`, separate `CARGO_TARGET_DIR` | passed | 72/72 (1 slow: `every_rule_kind_rejects_its_violation`, 83 s) |
| Default loop at the commit | `just check` in the same worktree | failed at `lint-agents` only | fmt-check, clippy `-D warnings`, ruff, nextest 72/72, pytest 21/21, pyrefly, rules-scan and rule tests 4/4 passed. `lint-agents` failed because the gitignored `.claude/skills/*` do not exist in a fresh worktree (missing prerequisite, not a defect of the commit). `uv run python scripts/adr.py lint`, run separately: passed (14 records) |
| Pilot, author's snapshot | `target/release/lctx query --store build/store --snapshot a32f151a…` | ran | Queries P1 and P3–P5 below. At about 01:41 a concurrent C3 pilot deleted `build/store` mid-review |
| Pilot rebuilt at the commit | `cargo build --release -p lctx`, then `lctx compile fastmcp --store <scratch>` from the worktree | passed | snapshot `c70e20f0…`; release `f454411b…` and `content_digest aa696303…` equal the author's snapshot, so P1–P5 describe `5e3baf9`. 113,897 nodes, **165,064** edges, 83,627 syntax nodes. 18.9 s, 2.87 GB max RSS (the host was running the C3 pilot at the same time, so this timing is not comparable with DESIGN's) |
| P2 replay | the `site_targets` SQL copied verbatim from `contracts__derivations_snapshot.snap`, run through `lctx query` with `syntax_nodes` replaced by `syntax_nodes WHERE kind <> 55` (no `ExprName` rows) | ran | Unmutated, it reproduces the published 8,500 / 3,027 split. Mutated: F2 |
| Cost estimate | a Python `ast` count over the 257 FastMCP 4.0.5 files in `build/envs/fastmcp` | ran | 27,226 statements, 104,414 expressions outside annotations (§8; Python's AST is not Ruff's, so an estimate) |

**Probes on the pilot (5e3baf9):**
- **P1 partition.** `pysa_calls` 35,057 = 17,775 call-site rows + 11,527 C2 site rows
  (1,502 attribute, 7,999 artificial call, 84 artificial attribute, 1,942 stringify) + 5,755
  identifier rows. `site_targets` has 11,527 rows: 8,500 with a node and a target, 3,027
  `unresolved_target`, 0 without a node.
- **P3 tree shape.** 83,627 rows with 83,627 distinct ids. In 59,261 (parent, field) groups the
  ordinals are 0..n−1 in source order, with no duplicates. The 28,596 `body` children are all
  statements.
- **P4 `site_target` evidence.** Of 8,500 edges: 829 have `potential` evidence (827 from attribute
  syntax nodes, 2 from call-site nodes), 627 are `property_get`, 5,267 artificial and 1,777
  format-string. Exact-span matches have 197 multi-node spans and 0 ties after `deepest`; each of
  the 13 chained-comparison sites has exactly one candidate.
- **P5 consumers.** 800 `raise` statements: 706 raise a call, 25 a name, 2 an attribute.
  693 raised calls reach `builtins` `BaseException.__new__` and 533 reach `BaseException.__init__`.
  The `init` rows carry the raised class as Pysa's `receiver_class` (693, e.g.
  `builtins:ValueError#60` 263 times). 1,437 `ExprName` nodes outside any subtree are placed only
  because Pysa reported a C2 site at them: 786 f-string interpolations, 251 decorators, 175 `for`
  iterables, 172 values and 51 argument values.

**Not inspected, or asserted only:** the C1 derivations C2 did not touch; publication and recovery
(unchanged, and not attacked); whether Pysa can report a site inside an annotation or at a
non-expression node (0 such rows on the pilot, and not probed on other libraries); performance
beyond the author's figures; `attrs` or any second library under C2.

## 2. Authority and lifecycle (compressed)

- **`syntax_nodes`** has one producer, the `ruff-ast` walk. The placement policy lives in
  `syntax::placed` (an exhaustive match, `syntax.rs:127-228`) plus `walk.rs:229-230`
  (`at_site`, annotation depth). Its **row set also depends on Pysa's output** through
  `site_ranges`. DESIGN §3.2 declares that dependency; §3.4.1 does not (O1).
- **One occurrence, one id.** A `def`, a `class` and a call are placed under their declaration
  and call-site ids (`walk.rs:563-569`). The `syntax_node` existence source excludes those kinds
  (`graph.rs:177-188`), so `key:nodes` can hold. P3 confirms one id per row.
- **"Which Pysa rows are C2 sites"** is written in three places: Rust `pysa_map.rs:522-523`
  (which decides where the walk places nodes), inline SQL in the `site_targets` `sites` CTE, and
  `graph.rs`'s `call_site_rows()` plus the identifier test in the lineage `expected` and its
  complement in `graph_gaps`. No rule reconciles the Rust copy with the SQL copies, because a
  disagreement lands in F2's catch-all.
- **Attribute `if_called` targets** are claimed by two places: the code publishes them as C2
  `site_target` edges, while DESIGN §3.6 L639-640, §4.2.3 L1025, §3.2 L444 and §3.8 L727 assign
  them to a Reference and to C3's `potential_target` (F1).
- **The catalogs, rules and `edge_kinds`** are generated from the registry, as in C1.

## 3–4. Contracts and derivation (merged)

| Invariant | Enforcement | Evidence |
|---|---|---|
| Every declaration, and every call outside an annotation, is placed | `placed:declarations`, `placed:call_syntax` | Pass on the pilot: 13,969 of 14,176 calls, and the 207 in annotations are the 207 boundaries. No injected-violation test (F3) |
| A child lies within its parent, in the same module | `contained:syntax_nodes` (inner join, so module-level children are unchecked) | Passes; no negative test (F3) |
| One parent per node | by construction; `key:nodes` covers `syntax_node` kinds only | Nothing rejects a duplicated decl/call placement row (F3) |
| A statement's ordinal is its block index | every `Stmt` variant is placed (`syntax.rs:130-158`); `Frame::place` counts per field (`walk.rs:95-104`) | **Measured** (P3) |
| `syntax_kind` covers Ruff's `NodeKind` | an exhaustive `match`, plus `#[deny(clippy::wildcard_enum_match_arm)]` | **Tested** by compilation and clippy (`just check`). `fields()` is not exhaustive (O5) |
| Every C2 site row has a node, or its row says why | `lineage:site_target` (explained = any reason), `typed:site_targets` | The site half cannot fail: F2 |
| Identifier rows are published gaps | `partition:pysa_calls-gaps` | P1 |
| A site maps to its deepest exact-span node; a chained pair maps to its comparison | `deepest` anti-join plus `row_number()` by `node_id` (`derived.rs:749-780`) | 0 ties (P4). "Innermost" is not guaranteed in the nested case (O3) |
| Determinism | sorted batches; `HashSet`/`HashMap` used only for membership and counters | **Tested**: `identity.rs` reversed module order over every extractor table; `graph.rs` two locations and reversed order. The rebuild reproduced the content digest |

**Absence.** `site_targets.reason` is one column for two nullable columns. A site with no node gets
`provider_disagreement` even when its target is `unresolved_target`, which overwrites the Pysa
reason (F2). "No `argument_value` edge" means the value's root was not placed. Whether a root is
placed depends on its kind and on Pysa's site set (O1).

## 5. Journey: do the named consumers get what they read?

Traced on the `syntax_shapes` snapshot and the pilot (P3, P5).

- **Pass B guards: served.** `if`/`elif`/`while`/`assert` tests and match guards carry their full
  subtree, names and literals included, with detail text. In the snapshot, `x is None`,
  `mode not in ("fast", "safe")` and `0 <= limit < 10` are complete. Linking a guard's names to
  parameter bindings is C3's job, through `reference = H(reference, name syntax id)`, which the
  placed name's `node_id` already is.
- **Raises: served.** The `exc`/`cause` subtrees are placed, and so is the enclosing chain of
  `if` and `try` bodies, through parent and field.
- **Handlers: served.** `try` → `handler` → handler, with its caught types in a subtree and its
  bound name as detail. `finalbody` and `orelse` are separate fields. Caught types are filed
  under field `test` (O2).
- **Pass C straight-line regions: served for the region structure.** Every statement is placed,
  and ordinals within a body field are contiguous and in source order (P3). So "consecutive
  statements of one block" and the kinds between them (`with` = resource boundary, `return` or
  `yield` = escape) are decidable. The binding `x` in `x = producer()` and the argument `x` in
  `consumer(x)` are **not** placed; Pass C needs C3's bindings and references for them, as
  DESIGN §3.2 L444 already says. A future Ruff statement kind would drop out of body fields
  silently (O5).
- **FCA raised types: structure served, type identity not assigned.** The callee label (for
  example `ValueError`, 50 distinct) is placed text. The class comes from Pysa's
  `receiver_class` on the `init` row for 693 of 800 raises, a C1 column that is a text key with no
  node or edge. The call target names `BaseException.__new__`/`__init__`. `raise Name` and
  `raise err` need C3 or C4. DESIGN names no path from a raise to a class node (O4).
- **Not a named consumer, but on §9.2's list: Pass B `transformed_argument`** ("a literal, default
  or expression supplied downstream"). A literal or plain-name argument has no node, and
  `arguments` has no value kind, so no family serves it yet (O4).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **fail** (narrow, latent) | (a) The C2-site predicate is independently editable in Rust (`pysa_map.rs:522-523`) and SQL (`derived.rs` `sites` CTE; `graph.rs` lineage and `graph_gaps`). A narrowing of the Rust copy publishes silently through F2. (b) Attribute `if_called` records are claimed by C2's `site_target` in the code and by C3's `potential_target` / a Reference in DESIGN §3.6, §4.2.3 and §3.8 (F1). Otherwise one authority: one producer per table, and the catalogs are generated | F1, F2 |
| **G2** Semantic fidelity | **fail** | 829 potential `if_called` targets share the invocation-meaning kind `site_target` ("may invoke the target"), against §3.6's "potential targets on a Reference, not CallSites" and C1's own split (`higher_order_target`) (F1, P4). Three states (Ruff has no node at Pysa's span; our walk declined to place one; the Rust and SQL predicates drifted) collapse into `provider_disagreement`, which overwrites `unresolved_target` (F2) | F1, F2 |
| **G3** Validity | **fail** (narrow, latent) | A site whose node the walk failed to place publishes with every rule passing. P2: 2,455 rows, 0 `typed:site_targets` violations, lineage explained. The four new hand-written rules have never been shown to fire (F3). Unchanged and sound: strict casts, generated rules, validation before publication | F2, F3 |
| **G4** Hidden behaviour | **pass** | The walk reads the AST, the module text and the collectors' in-memory output. The new order (walk after the collectors) is declared in §4.2.2 L985-986. Derivations read only. No new ambient read | — |
| **G5** Consistency and recovery | **pass** | Publication path unchanged. The new tables are part of the attempt (`compile.rs` asserts 23 + 16 tables per published attempt). Not attacked beyond reading | — |
| **G6** Transformation and reuse | **pass** | C2 changes no node id: the id-recipe snapshot moves only `run_id` and fact ids, because enabled families are in `run_id` (§3.4.1). Determinism **Tested**, and the rebuild reproduced the content digest. O1 records an undeclared Pyrefly dependence of non-body ordinals and `ast_child` edge ids | O1 (declaration) |
| **G7** Truthful capability claims | **fail** (narrow) | §8 L1384-1385 "no derivation supplies a catch-all" is false for `site_targets` (F2). §8 L1344 "each rule kind rejects an injected violation" is false for `placed`/`contained` (F3). The direction "may invoke the target" is false for `if_called` rows (F1). Stale spine lines and a wrong Measured edge count (F4) | F1–F4 |

## 7. Findings

### 7.1 Findings

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | `site_target` publishes Pysa's `if_called` records at attribute sites (the site reads a callable **value**, modality `potential`) under the same edge kind as property getters and artificial protocol calls (the site **invokes**). DESIGN gives these records to a Reference and to C3's `potential_target` | DM-34, DM-24, DM-02 · G2, G1; guidelines §2 ("extracted, resolved, derived and heuristic stay distinguishable") | **Code:** `pysa_map.rs:736-744` maps `ac.if_called` with `potential = true` under callee kind `AttributeAccess`. The `site_targets` `sites` CTE takes every non-call, non-identifier row, and `graph.rs:591-624` emits them all as `site_target`, direction "the site … may invoke the target". **DESIGN:** §3.6 L639-640 ("Callable values that may be invoked (Pysa `ifCalled`) become `potential` targets on a `Reference`, not `CallSite`s"); §4.2.3 L1025 (`if_called` (identifier **or attribute**) → target on a Reference); §3.2 L444 (the lexical family's consumers include "`if_called` targets"); §3.8 L727 (C3 `potential_target`: reference or site → callable). **C1 precedent:** potential higher-order targets have their own kind, `higher_order_target`. **P4:** 829 of 8,500 edges have `potential` evidence, 2 of them from call-site nodes | A projection that selects invocation kinds (§3.2 L400-401: "by kind, derivation class and evidence") and includes `site_target` to recover property-getter delegation (the gap in ADR-0014 review F5) also walks `register(self.handler)`-style references as if the attribute read called the handler. Pass A's "never crosses potential" (§9.1 L1437-1438) then holds only for consumers that join `facts.modality`, which `edge_kinds` does not publish. When C3 builds `potential_target` as specified, the same 829 Pysa rows get a second edge of a second kind (`one-per-evidence` is per kind, so no rule notices), or `site_target` silently changes meaning between snapshots | Keep `potential` attribute rows out of `site_target`. Either declare `potential_target` (site → callable, `if_called`) in C2 with C1's `higher_order_target` shape and amend §3.8's C2 and C3 rows, or leave these rows in `graph_gaps` with detail "if_called site: C3" until C3. Either way: one `WHERE` on the `site_target` SQL and lineage, one registry entry or gaps branch, and §3.8 | **test:** `syntax_shapes` gains `register(obj.method)`; assert no `site_target` edge has `potential` evidence. Optional **test:** a per-kind allowed-modality set in the registry, generating a `modality:<kind>` rule over the evidence `facts` rows (none exists today) |
| **F2** | `site_targets` gives `provider_disagreement` to every site with no placed node at its span. That reason is decided by our own policy-filtered join, so the site half of `typed:site_targets` and of `lineage:site_target` cannot fail. It also merges three states and overwrites Pysa's `unresolved_target` | DM-08, DM-07, DM-02, DM-60 · G2, G3, G1 | **Code:** `derived.rs:775` `CASE WHEN x.node_id IS NULL THEN {disagreement}`, documented at L729. The join's right side is filtered by policy: `walk.rs:229-230` places a site node only for an `Expr` outside annotations. The site predicate has a Rust copy (`pysa_map.rs:522-523`) and SQL copies (`sites` CTE; `graph.rs` lineage). **The rule it breaks:** ADR-0014 L118 and DESIGN §8 L1384-1385 ("no derivation supplies a catch-all one"). **Contrast:** Stage C's `provider_disagreement` (`derived.rs:28-33`) is legitimate because its right side, `declarations`, holds every `def`. **P2:** with the `ExprName` rows removed, 2,455 of 11,527 site rows become `provider_disagreement`, with 0 `typed:site_targets` violations; lineage counts them as explained | If the Rust predicate narrows, or `at_site` or the placement policy changes, or a Pyrefly bump reports sites at a node the walk does not place (a keyword, a decorator node, an annotation), those sites publish as "Pysa and Ruff disagree" and every rule passes. A brief then states a limit that is our bug. This is the ADR-0014 review F1 shape, reintroduced for a new column. The `syntax_shapes` assertion `counts[0] == 0` catches it only for the shapes that fixture contains | A site with no node keeps a **null** reason, so `typed:site_targets` rejects it, unless a named provider-stated case appears (a site in an annotation → `outside_provider_model` plus a boundary, as for calls). Derive the Rust `site_ranges` test and the SQL `sites` filter from one declaration (one function per side, plus a test that they agree on every `PysaSiteKind` × `PysaCalleeKind`). Keep `unresolved_target` when the target is unresolved. About three `CASE` edits and one shared predicate | **test:** a case in `every_rule_kind_rejects_its_violation` that drops the site-only `ExprName` rows from `syntax_nodes` (or shifts non-call site spans by 1) must fail `typed:site_targets`. Today it passes (P2) |
| **F3** | C2's new rule kinds are unexercised, one edge kind has no lineage although one can be stated, and nothing enforces "one parent per node" for placement rows | DM-60, DM-54, DM-59 · G3, G7 | `compile.rs:385-389`: 13 mutation cases, none for `placed:declarations`, `placed:call_syntax`, `contained:syntax_nodes`, `typed:site_targets` or `lineage:site_target`. `argument_value` has `lineage: None` (`graph.rs:588`) with the rationale "a plain name or literal is not placed" (L586-587). But every placed `argument`-field child of a call lies in exactly one argument span, so "each yields one edge" is checkable, and P5 shows 51 plain-name argument values **are** placed. `key:syntax_nodes` = (module, start, end, `node_id`), and the node source excludes kinds 2, 3 and 43 (`graph.rs:177-188`), so a duplicated decl or call placement row passes every rule. §8 L1344 claims per-kind negative tests | A walker regression (a decorator call left unplaced, a child filed outside its parent's span) fails only if `syntax_shapes` has that shape. A placement row duplicated under a second parent gives a node two `ast_child` parents (the edge ids differ by source) with every rule passing | Four mutation cases (drop a declaration's placement row; drop a call's; widen a child's span; the F2 case). A `lineage:argument_value` from the syntax side. Key `syntax_nodes` on `(snapshot_id, node_id)` or add a one-parent rule. About 40 lines of test and one registry line | **test:** the four cases in `every_rule_kind_rejects_its_violation` |
| **F4** | DESIGN is out of step with C2 in several load-bearing lines | DM-59 · G7 | §3.8 L692 "C2–C5 are **Proposed**" next to the C2 row's "**Implemented**". §3.8 L755-758: `graph_gaps` still lists C2 sites. §3.8 L771-772 "165,053 edges": the author's `build/pilot.txt` and the rebuild at `5e3baf9` both give **165,064** (65,176 + 83,627 + 7,761 + 8,500). §4.2 L944 "`syntax_nodes` (increment 2)". §3.2 L439 still describes the non-call sites as pending until C2. §8 L1375-1386 lists no `placed`/`contained` kinds and claims no catch-all (F2). The C2 `argument_value` rationale is inaccurate (F3) | A reader who takes §3.8's Measured block as the regression baseline sees an 11-edge drift that is not real. A reader of §8 believes the site half of `typed:site_targets` is falsifiable | Correct the lines. Relabel §3.8's C2 part **Implemented/Tested/Measured** with the C2 test named | prose (no mechanical oracle for spine wording) |

**Observations** (moving none of the three severity classes today; each names where it would
land):

| # | Observation | Evidence | Oracle |
|---|---|---|---|
| O1 | Outside subtrees, placement depends on Pysa's site set. `ordinal` means "position among **placed** children" (`tables.rs` doc), so in non-body fields a sibling's ordinal, and the `ast_child` edge id that hashes it, change when Pyrefly reports a different set of sites. `argument_value` exists for some plain-name arguments and not others. §3.4.1's producer-scope note covers Ruff-scoped syntax ids and Pyrefly-scoped call edges, not this. Body ordinals are unaffected | P5: 1,437 names placed only because of a site (251 decorators, 51 argument values). `lctx_id('edge', kind, src, dst, ordinal, …)` in the edges SQL | none today. Declare it in §3.4.1, or take §8's alternative. **test** if kept: a `@name` + `@call()` decorator list in `syntax_shapes` pins the ordinal meaning |
| O2 | An except handler's caught types sit in field `test`, the code branch conditions use. A guard reader must exclude handler parents | `syntax.rs:291-294` | the syntax snapshot, after appending a `type` field code (append-only) |
| O3 | The chained-comparison fallback is not "innermost". `deepest` drops only a candidate that is another candidate's direct parent, so in `f(a < b < c) < d` both comparisons survive and `row_number()` picks by `node_id` | `derived.rs:767-772`. Latent: 13 pilot sites, one candidate each (P4) | **test:** add that line to `syntax_shapes`; order `deepest` by span width, then `node_id` |
| O4 | Two consumer paths have no owner. FCA raised types: the class is only Pysa's text `receiver_class` on the `init` row (693 of 800 raises), or C3/C4 for `raise Name` and `raise err`. Pass B `transformed_argument`: a literal or name argument has no node and `arguments` no value kind | P5; §9.2 L1455; §9.6 L1528 | none until Pass B and FCA are specified (Deferred) |
| O5 | `fields()` ends in `_ => {}` (`syntax.rs:389`), unlike `placed()` and `syntax_kind()`. A future Ruff statement kind with a body would have its statements filed as `child`, and Pass C's body-field regions would skip them silently | `syntax.rs:236-392` | clippy in `just check`, after an exhaustive match on `AnyNodeRef` with `#[deny(clippy::wildcard_enum_match_arm)]` |
| O6 | A name placed as a `syntax_node` (in a subtree, or at a site) will also be a C3 `reference` node. That is allowed as a role (§3.1 L380-381; ADR-0014 L96-100), but C3's edge list declares no reference → carrier edge, although C2 set the precedent with `argument_value` | §3.8 L727 | C3's review (§10) |

**Checked and clean:** decl and call placement rows share their ids, and one id per occurrence
holds on the pilot (P3). Every statement is placed, and a statement's ordinal is its block index
(P3). The partition of `pysa_calls` sums exactly (P1). `graph_gaps` is identifier-only and
`partition:pysa_calls-gaps` matches it. Determinism is covered by the existing reversal and
relocation tests. `lineage:ast_child` cannot fail on today's SQL, because the edge SQL has no
filter; like C1's `declares` lineage, it guards future SQL edits and is not vacuous in intent.

### 7.2 Applicability and verdicts

- **Bore on this scope:**
  - Groups 1–2 (authority; types, absence): F1, F2, O2.
  - Group 3 (identity): one id per occurrence satisfied; O1.
  - Group 5 (derivation): `site_targets`.
  - Group 7 (relationship structures): F1.
  - Group 9 (provider boundary): the Pysa → walk coupling.
  - Groups 10–12 (lineage, regression controls, claims): F2–F4.
- **Did not bear:**
  - Group 4 (declarative composition) has no new declarations beyond registry entries, which
    follow C1's shape.
  - Group 6 (effects and state): no new effects; publication unchanged.
  - Group 8 (performance): C2 adds 0.5 s and 0.42 GB per the author's Measured figures, and no
    claim rests on more.

| Verdict | Principles |
|---|---|
| Satisfied | DM-06 (typed kinds and fields as codebooks), DM-09 (endpoint kinds generated), DM-11/DM-15 (one id per occurrence, structural path), DM-40 (determinism **Tested**), DM-46 (every Pysa row accounted, P1), DM-51 (schema snapshots; the commit is labelled a migration) |
| Violated | DM-34 and DM-24 (F1), DM-08 and DM-07 (F2), DM-02 (F1(b) and F2's triplicated predicate), DM-60 and DM-54 (F3), DM-59 (F4) |
| Unresolved | DM-31: syntax placement's dependence on the Pyrefly revision is undeclared (O1). DM-43: consumer paths for raised types and literal arguments (O4) |

**Guidelines MUSTs (ADDENDUM §5), for C2:**
- **§2 edge identity, typed endpoints, evidence, snapshot scope: met.** `ast_child` is keyed by
  ordinal, `site_target` is parallel with a payload discriminator, and endpoint and evidence
  rules are generated.
- **§2 isolates: met.** `syntax_node` has its own existence source.
- **§2 distinguishable derivations: partly** (F1).
- **§2 unknown targets explicit: met** for `unresolved_target`, **partly** for missing nodes (F2).
- **§7 partial ≠ complete: met** by the "placed" qualifier in the direction text, **partly** in
  practice (O1).
- **§12 known-answer shapes, lineage and per-stage metrics: met** (`syntax_shapes`; lineage rules;
  `derive site_targets` timing).

## 8. Alternatives (compressed)

| Alternative | Duplication and extension locality | Risks | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| Current: policy placement (statements, clause nodes, site-kind expressions, subtrees, plus any expression at a Pysa site); one `site_target` kind | Placement is decided in three places (`placed`, `in_subtree`, `at_site`), and the last copies Pysa's site predicate into Rust | F1, F2, O1, O4 | ~450 lines in `syntax.rs`, ~140 in `walk.rs` | **Measured** (author, 2026-09-23): +0.5 s, +0.42 GB peak over C1 | Selected by the author |
| Current with F1 and F2 corrected | Same, with potential targets on their own kind or in `graph_gaps`, and one site predicate | O1, O4 remain | + a registry entry, one shared predicate, ~5 tests | same | Minimum for Accept |
| **Simpler viable: place every expression outside annotations** (the exhaustive exporter, unscoped) | Removes `in_subtree`, the subtree stack, `subtree_field`, `at_site`, `site_ranges`, the Rust copy of the site predicate, and the need to run the walk after the collectors. `argument_value` becomes total (one value root per argument, so a lineage rule). Ordinals become syntactic. `provider_disagreement` becomes truthful for expression spans. Pass B literal arguments (O4) and C3's reference carriers (O6) get nodes | Still needs F1's split, the chained fallback and O2. Larger `nodes` for every projection to filter by kind | About −100 lines. Rows: ~104k expressions + ~27k statements + clause nodes by the Python-AST estimate, roughly +50–60% over 83,627 (**Proposed**) | Extrapolated from C2's measured cost, roughly +0.3 s and +0.25 GB (**Proposed**, unmeasured) | Worth an explicit decision before C3. C3's references and Pass B read exactly the names and literals the current policy leaves out, and it retires F2's and O1's causes rather than patching them |

**What stays ordinary code:** the walk, the frame stack and `fields()`. None of it should become a
declaration surface.

## 9. Top verification gaps

| Claim or risk | Label now | Check | Expected result | Gap |
|---|---|---|---|---|
| `site_target` carries only invocation-meaning targets | **Implemented**, contradicted by data (P4) | `syntax_shapes` + `register(obj.method)`; assert no `potential` evidence on `site_target` | 0 rows | F1 |
| A site with no node is rejected | **Implemented**, unfalsifiable (P2) | the mutation case in `every_rule_kind_rejects_its_violation` | fails `typed:site_targets` | F2 |
| The C2 hand rules fire | **Implemented** | four mutation cases | each fails its rule | F3 |
| Rust and SQL name the same C2 sites | **Implemented**, twice | a unit test over every `PysaSiteKind` × `PysaCalleeKind` | agree | F2 |
| Block index = ordinal | **Measured** (P3); **Tested** only through the snapshot's rendering | optional: assert contiguity in `syntax.rs` tests | 0 violations | — |
| DESIGN's C2 pilot figures | **Measured** by the author; edge count wrong | rerun `just pilot` after the fixes | 165,064 edges, less F1's 829 if they move | F4 |

## 10. Exceptions and unresolved decisions

No SHOULD-level exception is requested. Decisions the author has to make:

- **F1:** where attribute `if_called` targets live: a `potential_target` edge declared now, or
  gaps until C3. Either way §3.6, §4.2.3 and §3.8 have to say one thing. An ADR is needed only if
  the choice is to keep potential targets inside `site_target`, which departs from C1's
  `higher_order_target` split and from §3.6.
- **§8 alternative:** placement scope. Keep the policy and declare O1 in §3.4.1, or place every
  expression outside annotations.
- **For C3 (O6):** declare the reference → placed-carrier relation, or state that consumers
  reach it through `lctx_id('reference', syntax node id)`. Note that the recipe must be the
  `opt_*` one, per ADR-0014 review F3.

## 11. Decision

**Decision: Revise** (small surface).
**Reason:** the slice places what its named consumers read. Guards, raises, handlers and block
structure are all there, one id per occurrence holds, and every Pysa row is accounted for on the
pilot. Two things fail on in-scope behaviour. First, C2 publishes 829 potential `if_called`
targets under an invocation-meaning kind that DESIGN assigns elsewhere (G2, latent G1). Second, it
reintroduces a catch-all reason that makes its own site-placement guarantee unfalsifiable (G2,
G3), against ADR-0014's accepted "no catch-all" rule. Both are cheap to fix and both get more
expensive once C3 builds `potential_target` and references on top of them.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: move `potential` attribute rows out of `site_target` (a new kind or gaps); amend §3.6, §3.8 and §4.2.3 so they agree | DM-34, DM-24, DM-02 | the pilot's `site_target` has 0 `potential` evidence rows | test: the `register(obj.method)` case |
| 2 | F2: null reason for a missing node; one site predicate shared by Rust and SQL; keep `unresolved_target` | DM-08, DM-07, DM-60 | P2's mutation fails validation | test: the mutation case, plus the predicate-agreement unit test |
| 3 | F3: four mutation cases; `lineage:argument_value`; one parent per node | DM-60, DM-54 | each case fails its rule | test |
| 4 | F4: correct the DESIGN lines, including 165,064 edges | DM-59 | — | prose |
| 5 | §8: decide the placement scope before C3 | DM-56, DM-58 | a DESIGN §3.2 sentence either way | — |

### Deferred

| Item | Why not now | Trigger that reopens it |
|---|---|---|
| O1: Pyrefly-dependent ordinals and edge ids | Resolved by §8's choice | Keeping policy placement: add the §3.4.1 sentence and the decorator fixture |
| O2: `type` field for handlers | Cosmetic until a reader exists | Pass B's guard reader is written |
| O3: "innermost" comparison | 0 ambiguous cases on the pilot | Any library whose `site_targets` shows a chained site with >1 candidate |
| O4: raised-type identity; literal argument kind | No FCA or Pass B spec yet | FCA attributes or Pass B `transformed_argument` specified (increment 2) |
| O5: exhaustive `fields()` | No new Ruff kind at the pinned line | A ruff pin move (`pin-check`) |
| O6: reference → carrier | C3 scope | C3's review |

## Disposition (author, 2026-09-23)

The author took the simpler alternative, and fixed or recorded every finding in the same commit
as CPG slice C3.

| # | Outcome | What changed | Oracle |
|---|---|---|---|
| Simpler alternative | taken | `syntax_nodes` now places every statement, clause node and expression outside annotations. The Pysa site ranges no longer feed the walk (`site_ranges` removed), there is no subtree machinery, and placement is Pyrefly-independent (answers O1). Pilot: 138,136 syntax nodes (+65%); ordinals and `ast_child` ids depend on the source alone | the `syntax_shapes` tree snapshot |
| F1 | fixed | `site_target` excludes `potential` records. Pysa's `if_called` at attribute sites become `potential_target` edges from the syntax node (829 on the pilot), beside C3's identifier-site potentials | the lineage rules for both kinds, which split the rows by `facts.modality` |
| F2 | fixed | `site_targets` (and C3's `identifier_targets`) give a null reason when no node or reference sits at the span; only Pysa's `unresolved_target` and Stage C's reasons remain. The duplicated Rust predicate is gone | test: shifting every non-call Pysa span fails `typed:site_targets` (`compile.rs`) |
| F3 | fixed | new injected-violation cases: `placed:declarations`, `contained:syntax_nodes`, `typed:site_targets`, the new `unique:syntax_nodes` (one placement per node), plus C3's `typed:reference_resolutions` and `id:bindings`. `argument_value` now has a lineage: one value node per argument of a call outside an annotation, evidence the `arguments` fact | `every_rule_kind_rejects_its_violation` (19 cases) |
| F4 | fixed | DESIGN §3.2, §3.8 and §8 updated. The C2 edge count at the commit, 165,064, is recorded | prose |
| O1 | fixed | by the simpler alternative | the tree snapshot |
| O4 | recorded | the raised type is C4's (`type_observations` of a `raise`'s `exc`) | C4 |
| O6 | fixed | C3 references are roles of their placed name (`references.name_node_id` is a node-valued column of kind `syntax_node`); annotation names are C4's, not references | the generated `ref:references.name_node_id->nodes` rule |
| Others | as the review states | — | — |

**Checks** (2026-09-23): `just check` passed (nextest 73/73). The pilot passed: 234,328 nodes,
335,442 edges, every rule, 17.5 s, 3.91 GB.
