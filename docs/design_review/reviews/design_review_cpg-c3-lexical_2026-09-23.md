# Design review: CPG slice C3, the `lexical` family (compact)

**Date:** 2026-09-23 · **Depth:** compact · **Mode:** code plus DESIGN.md, at commit `733289c`.
The commit ends a slice that adds a fact family, a recognizer, three derived tables and ten edge
kinds, and it carries the C2 compact review's fixes. So ADR-0001 owes a `compact` review.
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`733289c`
**Prior reviews:** `design_review_adr-0014-cpg-graph-catalog_2026-09-22.md` and
`design_review_cpg-c2-syntax_2026-09-23.md`, with their Dispositions. C1 and C2 are re-reviewed
only where this commit changes them. Findings here are F1–F7 and O1–O8.

## 1. Decision and scope

**Target.** Commit `733289c`, read with `git show 733289c:<path>` and `git diff 733289c~1 733289c`,
and built and tested in a detached worktree of the commit:
- DESIGN §3.2 (the `syntax`, `lexical` and `exports` text), §3.4.1, §3.5, §3.7, §3.8 (the C3 row,
  the rules, `graph_gaps`, the Measured block), §4.2–§4.2.4, §8, §9.1–§9.3;
- `cpg-extract`: `lexical.rs` (new, 843 lines, read in full); the diffs of `walk.rs`, `syntax.rs`,
  `lib.rs`, `context.rs`, `pysa_map.rs`, `facts.rs` and `config.rs`;
- `cpg-schema`: the diffs of `tables.rs`, `derived.rs`, `graph.rs`, `codebook.rs` and `id.rs`,
  plus `graph.rs` L1040–1125 (`edges`, `graph_gaps`);
- tests: the `cpg-core/tests/syntax.rs` diff and the `names_resolve_under_python_scoping` snapshot;
  the `compile.rs` and `graph.rs` diffs; `fixtures/python/lexical_shapes/`.

**Observable outcome claimed.** The recognizer runs inside the Ruff walk and emits `scopes`,
`bindings` (every binding event per scope in source order, with its kind, site, value span and
static branch), `references` (every name load outside annotations) and `reference_resolutions`.
DESIGN §3.2 L444 calls the resolution "full Python scoping". Also claimed:
`export_syntax.resolved_module`; derived `identifier_targets` and `import_targets`; ten edge kinds;
variable exports that target a binding. On the pilot, "**No name is unresolved**" (§3.8 L783).
Named consumers: Pass B binding order (§4.2.4), Pass C bindings and values, the import graph,
`if_called` targets, variable exports.

**Supported scope and non-goals.** Resolution is flow-insensitive. Names in annotations are C4's.
`imports_symbol` and separate `global`/`nonlocal` edges are deferred (§3.8 L730).

### Method and coverage

**Checks run in this session:**

| Check | Command | Outcome | Observation |
|---|---|---|---|
| Rust tests at the commit | `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass --offline` in a detached worktree of `733289c`, with a separate `CARGO_TARGET_DIR` | passed | 73/73; 1 slow: `every_rule_kind_rejects_its_violation`, 142 s |
| Lints | `cargo clippy --workspace --all-targets --quiet --offline -- -D warnings`; `cargo fmt --all --check` | passed | same worktree |
| Python and repo checks | `uv run pytest -q`, `uv run pyrefly check --summary=none`, `uv run ruff check --quiet`, `uv run ruff format --check`, `ast-grep scan`, `ast-grep test --skip-snapshot-tests`, `uv run python scripts/adr.py lint`, a fixture `ast.parse` | passed | pytest 21, rule tests 4/4, 14 ADRs, 31 fixtures. These are `just check`'s steps run one by one, not the recipe itself |
| `lint-agents` | `just lint-agents` in the main tree | passed | Agent files and scripts are identical to the commit. The tree later held uncommitted C4 work in other files only |
| `just deps`, `just gold` | — | not_run | The commit changes no dependency |
| Pilot | `target/release/lctx query --store build/store --snapshot bdc03d35…` | ran | The binary (02:15:37) is the commit's: no crate source is newer and the tree was clean at the commit. Its `pilot.txt` matches the snapshot. Not rebuilt |
| Scoping probes | scratch fixtures, plus a print-only test that compiles them, placed only in the worktree and deleted before the checks above | ran | P6–P10 below. Nothing was added to the repository |
| Language facts | `uv run python` (3.14.7): `compile`/`exec` of each construct probed | ran | Each "Python reads …" statement below was checked this way; P11's stub scan used the same interpreter |

**Probes on the pilot (snapshot `bdc03d35…`):**
- **P1 totals.** 3,667 scopes, 22,230 bindings, 39,950 references and 45,713 resolutions.
  Resolutions: 41,352 to a binding (760 captured rows on 734 references), 4,239 to a builtin
  function or class, 122 with a reason, 0 `unresolved_target`. 3,536 references (8.9%) have
  several candidates (9,299 `candidate` rows). Edges: `reads_binding` 40,592, `captures` 760,
  `reads_builtin` 4,239, `introduces` 17,673, `shadows` 1,387, `potential_target` 6,504, and
  `imports_module` 4,526 (every import resolves).
- **P2 the 122 "builtin variables".** `__name__` 118, `__file__` 1, `NotImplemented` 2, `Ellipsis`
  1 (F1).
- **P3 star imports.** None in the release. The star fallback absorbs nothing on the pilot.
- **P4 latent-case counts.** 0 of the 1,230 module- or class-scope references with a same-scope
  candidate precede all their candidates (F3). 1 lambda sits in a `def`'s default, with the `def` as
  its lexical parent: `fastmcp/server/auth/providers/debug.py`, `validate=lambda token: True` (F4).
  The 22 `global`/`nonlocal` declarations (10 and 12) are all routed correctly (F5).
- **P5 coverage.** 4,526 import bindings and 31 lambda parameters have no `introduces` edge (O1).
  1,458 of the 4,526 import bindings have no reader (O7). 14 `from pkg import submodule` imports
  of release submodules, in 12 modules, target the package, and 2 `imports_module` edges are
  self-loops (O3). 334 exports target a binding: 331 assignments, 2 module-level `except … as`
  names and 1 in a `TYPE_CHECKING` branch. One export keeps `variable_origin`, and its origin is
  a dependency.

**Scratch probes** (the compiled fixture, read through the published tables):
- **P6** implicit and outside names (F1). `__doc__` and `__spec__` read builtins' variables. `_T`
  and `Callable` in a function read "builtin, `variable_origin`"; Python raises `NameError`.
  `__class__` in a method is `unresolved_target`. `from rp.helpers import *` followed by `open()`
  reads the builtin `open`, where Python reads the star-imported one. `not_defined_anywhere` reads
  the star import with modality `definite`.
- **P7** flow and routing (F3, F5). `r = len; len = 5` at module level gives a `definite` read of
  `len = 5`, and Python reads the builtin. A class body `a = type; type = "k"` gives the class
  binding, and Python reads the builtin. For `global np_mod; import json as np_mod` in `lazy()`,
  `use_lazy()`'s `np_mod` is `unresolved_target`, and the import binding sits in `lazy`'s scope.
  For `nonlocal v; v = 2` placed before the outer `v = 1`, the store binds in the inner function's
  scope.
- **P8** parents (F4). Inside `outer(v)`, `@decorate(lambda: v) def inner(v=[v for _ in range(1)])`
  makes both free `v` read `inner`'s own parameter, captured. Python reads `outer`'s `v`. Also,
  `class Gen[T]` and `type Pair[T]` leave `T` unresolved (O5), `return err` after its handler
  reads the handler binding (O4), and match captures bind with no `introduces` edge (O1).
- **P9** legal source that publishes nothing (F6). `tags: Annotated[list[str],
  Field(default_factory=lambda: [])] = []` fails `ref:scopes.owner_node_id->nodes` and
  `endpoint:owns_scope`. `global g1, g1` fails `key:nodes`, `key:edges`, and
  `one-per-evidence`/`no-parallel` for `binds`, `introduces` and `shadows`.
- **P10** catch-all (F2). Corrupting every `export_syntax.resolved_module` of `lexical_shapes`
  still publishes: 4 of 4 imports read `unresolved_target`, and every rule passes.
- **P11** reach of the builtins set. A Python `ast` scan of Pyrefly's bundled `builtins.pyi` finds
  289 top-level names, 140 of them not in `dir(builtins)` at 3.14.7 (51 private; `Any`,
  `Callable`, `Iterable`, `Final`, …). This approximates Pyrefly's definitions table, so it is an
  estimate. P6 confirmed `_T` and `Callable` directly.

**Read and not attacked:** publication and recovery (unchanged; `compile.rs` asserts 27 + 18
tables per attempt); determinism beyond the passing identity tests; performance beyond
`pilot.txt`. Also, whether `get_module_info` has content for an imported-only, unowned,
file-backed module (0 on the pilot; 2 imported-only modules are namespace packages). This is
**asserted**: Pyrefly loads the modules a checked module imports. No second library was run under
C3.

**C2 Disposition, verified.** F1: `site_target` now excludes `potential` records
(`graph.rs` `AND f.modality <> {potential}`), and `potential_target` carries them
(6,504 = 5,675 + 829). F2: `site_targets` gives a null reason when no node sits at the span, and
the `typed:site_targets` mutation passes in the run above. F3: new mutation cases and the
`argument_value` lineage. The simpler alternative: `site_ranges` is gone from `pysa_map.rs`,
`walk.rs:264` places by `syntax::placed(node)` alone, and there are 138,136 syntax nodes (+65%,
against that review's +50–60% estimate). F4 only in part: §4.2 L960 still reads "(increment 2)".

## 2. Authority and lifecycle (compressed)

- **The four lexical tables** have one producer, surface `lctx-lexical` (`recognizer`,
  `derived_analysis`, `normalized_structural`).
- **Which names exist outside the module text** comes from other sources:
  - builtins: every entry of Pyrefly's `get_exports(builtins)` (`lib.rs:282-301`);
  - star imports: a fallback candidate (`lexical.rs:764-767`);
  - module implicit globals: not modelled, so they reach the builtins set (F1).
- **`export_syntax.resolved_module`** is our own relative-import arithmetic
  (`walk.rs:163-186`), stored in a `ruff-ast` raw row (O8).
- **`import_targets.reason`** is decided by our own join, though it reads as a provider's
  statement. Pyrefly's "not found" (`context.rs:172-177`) is discarded (F2).
- **The catalogs, rules and `edge_kinds`** are generated from the registry, as in C1 and C2.
  `graph_gaps` is still generated, but its only branch is `… AND FALSE` (`graph.rs:1100`) (O2, F7).
- **Identity.** `scope = H(scope, owner)`, `binding = H(binding, site, name)` and
  `reference = H(reference, name syntax id)` (`id.rs:102-121`), each recomputed by an `id:*`
  rule. The binding recipe assumes one event per (site, name), which the recognizer does not
  guarantee (F6).

## 3–4. Contracts and derivation (merged)

| Invariant | Enforcement | Evidence |
|---|---|---|
| A reference is a role of a placed name | `ref:references.name_node_id->nodes` (generated) | Passes on the pilot. It holds because every expression outside annotations is placed (C2 alternative) |
| Every resolution has a binding, builtin or reason | `typed:reference_resolutions` | **Tested** (`compile.rs` mutation). The "builtin" arm is over-broad (F1) |
| A scope's owner is a node | generated ref and endpoint rules | Correctly rejects the annotation-lambda case, where the extractor emits an owner that is not a node (F6) |
| Id recipes equal their SQL | `id:scopes`, `id:bindings`, `id:references` | `id:bindings` **Tested** by mutation; the other two not injected (F7) |
| One binding event per id | `key:nodes` | Fails closed on `global g1, g1` (P9) |
| Every import has a module or a provider's reason | `typed:import_targets` | **Cannot fail**: the derivation writes the reason exactly when its own join misses (F2, P10) |
| Each identifier site lands on a reference | `typed:identifier_targets`, `lineage:potential_target` | Null reason for a missing reference (the C2 F2 fix, applied). Pilot: 5,755 of 5,755 |
| Lineage for the five simple C3 kinds | generated | Each repeats its edge's own predicate on one table, and `edges` left-joins `nodes` (`graph.rs:1067-1068`), so they can only pass (O2) |
| `introduces` covers every binding with a node site | none: inner join to `nodes`, `lineage: None` (`graph.rs:687-698`) | O1 |
| Determinism | sorted batches; resolution order is `Vec` insertion order; `builtins_used` goes into a `BTreeSet` | **Tested** by the existing identity tests (passed) |

**Absence.** A resolution has four outcomes: a binding, a builtin, `variable_origin` or
`unresolved_target`. Three states the design needs to keep apart are collapsed into them: a
module's implicit global, a stub-only builtins name and a real builtin all read "builtin" (F1). A
name nothing binds, in a module with one star import, reads a `definite` star binding (F1). An
import whose name our arithmetic got wrong reads "the provider could not find it" (F2).

## 5. Journey: do the named consumers get what they read?

- **Pass B §4.2.4 (conservative binding rule): served for function scopes.**
  - The rule is a filter on `bindings` (scope, name, kind ≠ parameter, start before the site).
  - Rebindings Pyrefly drops are kept and marked: 206 bindings on the pilot, 197 of them
    `TYPE_CHECKING`.
  - Walrus, `nonlocal` stores and `del` are routed to their scope. Parameters bind before the
    body is walked, so F5's `nonlocal` ordering defect cannot hide a parameter rebinding. F5's
    unrouted `def`/`import` under `nonlocal` could, but it is rare.
  - The branch marks are a text heuristic (O6).
- **Pass B direct forwarding: served.** The path is argument → `argument_value` → name node →
  reference (`references.name_node_id`) → `reads_binding`/`captures` → parameter binding →
  `introduces` → parameter. All 6,201 `def` parameters have `introduces`; the 31 lambda
  parameters have none (O1).
- **Pass B identity alias and Pass C `x = producer(); consumer(x)`: served, with one trap and one
  gap.**
  - The value span joins `call_syntax`. Reassignment is `shadows`. Additional consumers are the
    other readers of the binding.
  - The trap: in `a, b = f()` each target carries the whole right-hand side, so the consumer must
    check that the name's parent is the statement.
  - The gap: Pass C runs over examples and tests (C5), which are largely module-level code. There
    a read before a later rebinding returns only the local candidates, never the builtin or global
    it actually reads (F3). Ordinals cannot choose a target that is not in the set. This contradicts
    the stated reason for flow-insensitivity (`lexical.rs:11-13`, DESIGN §3.2).
- **Import graph: partly served.** One edge per alias goes to the named module. `from pkg import
  submodule` goes to the package (O3). The unresolved reason is not provider-backed (F2).
- **`if_called` targets: served.** 6,504 `potential_target` edges, split from `site_target` by
  modality.
- **Variable exports: served, undocumented.** 334 exports now target the last module-scope binding
  event by ordinal (`derived.rs:214-231`). §3.2, §3.5 and §3.8 still say `variable_origin` (F7).
  Two targets are module-level handler names that Python deletes at the handler's end (O4).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **fail** (narrow, latent) | `import_targets.reason = unresolved_target` presents our join's miss as the provider's answer, and Pyrefly's actual not-found is discarded (F2, P10). With C3, `variable_origin` on a release-origin variable does the same. Otherwise: one producer per table, and the catalogs are generated | F2 |
| **G2** Semantic fidelity | **fail** | F1 is measured on the pilot: 119 module implicit-global reads are attributed to builtins. A stub-only name or an unbound name in a star-importing module reads as resolved, `definite` (P6). Probes show the correct target missing from the candidate set in module and class scopes (F3), free names resolving through the decorated function (F4, 1 wrong edge on the pilot), and bindings in the wrong scope under `global`/`nonlocal` (F5). F2 merges "not found" with "our arithmetic failed" | F1–F5 |
| **G3** Validity | **fail** (narrow, latent) | `typed:import_targets` cannot fail (P10). `partition:pysa_calls-gaps` runs over a table that is always empty (O2). Sound and unchanged: generated rules, strict casts, validation before publication. The new C3 mutation cases fire | F2, O2 |
| **G4** Hidden behaviour | **pass** | The builtins set and the imported-only modules come from the constructed Pyrefly transaction and finder (no discovery). The walk reads the AST and the text. No new ambient read. The run order is declared | — |
| **G5** Consistency and recovery | **pass** | Publication path unchanged. P9's rejected attempts publish nothing. Not attacked beyond that | — |
| **G6** Transformation and reuse | **pass** | `EXTRACTOR_OUTPUT_VERSION` 7 → 9, so `run_id` moves. The lexical ids are pinned by `id:*` rules. The builtins set follows the Pyrefly revision in `producer_id` and the Python version in `context_id`. Syntax ids are unchanged by the placement change: ordinals in non-body fields are now syntactic (the C2 O1 cause retired). F6(b) is caught by `key:nodes`, not reused | — |
| **G7** Truthful capability claims | **fail** | "Full Python scoping" (§3.2 L444) is contradicted by F1 and F3–F5, and O5 is undeclared. "No name is unresolved" holds partly through F1. "Each new rule rejects an injected violation" (L694-695) is not true of four rules. Several spine lines are stale (F7). The supported scope silently excludes legal source (F6) | F1, F3–F7 |

## 7. Findings

### 7.1 Findings

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | Names from outside the module's own text are over-approximated, so misses read as resolved. (a) "Builtins" is Pyrefly's whole definition table for `builtins.pyi`: its implicit globals, private stub names and the names the stub imports. (b) Module implicit globals (`__name__`, `__file__`, `__doc__`, `__spec__`, `__class__`) are not modelled, so they read builtins' own variables or are unresolved. (c) A star import is tried only after builtins, and supplies any otherwise-unbound name with modality `definite` when it is the only one | DM-08, DM-04, DM-59 · G2, G7; guidelines §2 (unknown targets explicit), §7 (partial ≠ complete) | **(a)** `lib.rs:282-301` maps every `get_exports(builtins)` entry. Pyrefly's `exports()` (`pyrefly/lib/export/exports.rs:438-490`) emits every definition, including `ImplicitGlobal` (as `Constant`) and imports (`OtherModule`, which `lib.rs:295` gives kind `None`, so variable-like). P11: 140 of 289 stub names are not runtime builtins. **(b)** `lexical.rs:750-763`. P2: 119 of the 122 "builtin variable" reads on the pilot are `__name__`/`__file__`. P6: `__class__` is `unresolved_target`. **(c)** `lexical.rs:764-767` comes after the builtin branch, and `lexical.rs:771-775` sets the modality by row count. DESIGN §3.2 says "a star import as a **candidate**". The fixture's own snapshot shows it: `lex/__init__.py:603 name (scope 4) -> binding 11` is a `NameError` at runtime, read from `*`. P6: `open` after the star import reads the builtin | A release that forgets `from typing import Callable` reads `Callable` as a builtin. Nothing unresolved is reported, which is exactly the miss a brief's grounding check needs to see. Every module's `get_logger(__name__)` names builtins' `__name__`. A library using `from numpy import *` gets `definite` `reads_builtin` edges to `builtins.sum` where it calls `numpy.sum`. The pilot's "No name is unresolved" rests in part on (a) and (b) | Take the outside names from Pyrefly's own sets instead of the full export table. Module implicit globals come from `ImplicitGlobal::implicit_globals` (`pyrefly_types/src/globals.rs:61`, public) as module-scope implicit binding events, with an appended `binding_kind` code. Keep the `__class__` cell for methods. Builtins are the `ThisModule`, non-`ImplicitGlobal`, non-private definitions. A star import binds its module's wildcard set (`Exports::wildcard`, public; a fork visibility change if `LookupExport` is not reachable, which is within ADR-0012's budget) before builtins, and a name outside every set stays `unresolved_target`. About 40 lines | **test:** `lexical_shapes` gains `__name__`, a method reading `__class__`, an unimported `Callable`, and a star module that exports `open`. Add a module without a star import, so that the class-comprehension line reads `unresolved_target` instead of `*` |
| **F2** | `import_targets` brings back a catch-all reason, the third instance of the shape ADR-0014's review F1 and C2's review F2 removed. `unresolved_target` is written whenever our own join misses, so `typed:import_targets` cannot fail. Pyrefly's actual "not found" is thrown away. After C3, `variable_origin` does the same for a release-origin variable the recognizer failed to bind | DM-08, DM-07, DM-02, DM-60 · G1, G2, G3 | `derived.rs:900-903`: `CASE WHEN COALESCE(r.module_node_id, d.module_node_id) IS NULL THEN {unresolved}`. The join inputs are our `absolute_module` (`walk.rs:163-186`) and our `release_modules` filter (`lib.rs`). `context.rs:164-179`: an import Pyrefly cannot find "is left out; its import row says so", but no row records Pyrefly's answer. The rule broken is ADR-0014 L117-118 ("No derivation has a catch-all reason") and DESIGN §8 L1399-1401. **P10:** with every `resolved_module` corrupted, the snapshot publishes and all 4 imports read `unresolved_target`. The exports case: `derived.rs:254-256` gives `variable_origin` whenever a variable-like export has no target | A bug in the relative-import arithmetic (an `is_package` test on a new file layout, an off-by-one `level`) publishes every affected import as "optional dependency not installed", with every rule passing. The same goes for a recognizer miss behind a variable export (F5's lazy `global` import is one) | Record the provider's answer in the extractor: name each import with Pyrefly's `ModuleName::new_maybe_relative` (public; retires `absolute_module`) and store whether its finder found it. `import_targets` then gives `unresolved_target` only for a provider not-found and null otherwise. Give `variable_origin` only when the origin module is not a release module. Two `CASE` edits and one extractor column | **test:** a case in `every_rule_kind_rejects_its_violation` that corrupts `export_syntax.resolved_module` must fail `typed:import_targets` (P10: today it publishes) |
| **F3** | Module and class scopes resolve a name to their local bindings whenever the scope binds it anywhere. Python falls back to globals and builtins for a name not yet bound there (`LOAD_NAME`). So the right target can be absent from a `definite` answer, and no ordinal can recover it | DM-04, DM-08, DM-59 · G2, G7 | `lexical.rs:809-813` returns `local(s, name)` when any exists, for every scope kind. DESIGN §3.2 and `lexical.rs:11-13` justify flow-insensitivity with "every binding … is a candidate, with its ordinal, so an order-aware consumer … can choose". **P7:** `r = len` before `len = 5`, and a class body `a = type` before `type = "k"`, each read the local binding `definite`; Python 3.14.7 reads the builtin in both. **P4:** 0 of 1,230 on the pilot (latent) | Pass C over examples and tests, which is module-level code, and any module-level analysis gets a wrong `definite` edge whenever a script reads a builtin or global before rebinding its name. A class that uses `type`, `id` or `list` before defining a member of that name is common in model classes. The consumer contract "choose by ordinal" does not hold in these two scope kinds | In module and class scopes, add the outer resolution (enclosing function for a class, then module, then builtins) as a further `candidate` when the reference precedes the scope's first binding event of the name, or unconditionally. A few lines in `resolve` | **test:** two fixture lines (P7's) with their expected candidate sets |
| **F4** | A new scope's parent is the top of the open-scope stack, not the scope its position evaluates in. A lambda or comprehension in a `def`'s decorators or defaults gets the decorated function as parent, because that scope opens at the `def` node before its decorators and defaults are walked. A comprehension or lambda inside a comprehension's first iterable gets the outer comprehension. Free names then resolve through the decorated function's own parameters | DM-09, DM-04 · G2 | `lexical.rs:175` `let parent = self.open.last().copied()`. `scope_at` (`lexical.rs:194-202`) already knows the right scope for names, but scopes are not placed with it. **P8:** in `outer(v)`, `@decorate(lambda: v) def inner(v=[v for _ in range(1)])` makes both `v` read `inner`'s parameter as captured; Python reads `outer`'s. **P4:** 1 wrong `lexical_parent` on the pilot (no free name, so no wrong resolution). The first-iterable case was read in the code, not probed | C5 compiles test suites, where `@pytest.mark.parametrize(…, [… for … in …])` and lambda defaults are routine. Each such scope gets the test function as `lexical_parent`, and any free name shared with the test's parameters becomes a false closure capture. Pass B could then read a forwarding from a parameter that is not in scope | Parent = `scope_at(node.start())` at `open_scope`. One line | **test:** P8's two lines in `lexical_shapes` |
| **F5** | `global`/`nonlocal` routing covers name stores and `del` only. A `def`, a `class`, an import, an `except … as` or a match capture under a declaration binds in the declaring scope, where `local()` then hides it. A `nonlocal` target is chosen at walk time, so a store textually before the outer binding lands in the inner scope | DM-04, DM-59 · G2, G7 | Routed: `lexical.rs:579` and `583` (`declared_scope`). Not routed: `bind(here, …)` at `238`, `276`, `421`, `462` and `497-534`. Hiding: `lexical.rs:704`. Walk-time choice: `lexical.rs:617-625` (`ps.names.contains_key(name)`). DESIGN §3.2 L444: "a store under `global`/`nonlocal` binds in the declared scope". **P7:** the lazy-import idiom `global np_mod; import json as np_mod` leaves `use_lazy()`'s read `unresolved_target`. `nonlocal v; v = 2` before `v = 1` binds in `inner`. **P4:** 22 declarations on the pilot, all correct (latent) | The lazy-import idiom reads as unresolved, and through `exports` as `variable_origin` (F2). A `nonlocal` rebinding placed before the outer binding is invisible to the outer scope's history, which Pass B's rule reads | Route every binding event through `declared_scope`, and decide `nonlocal` targets in `finish`, when all bindings are known | **test:** P7's two shapes in `lexical_shapes`. The snapshot should also render the owning declaration of each scope, not only its kind, so that a binding moved between two function scopes shows up in the diff (today both print `scope 2`) |
| **F6** | Legal source makes the compile publish nothing. (a) The recognizer opens scopes and records bindings inside annotations, which C2 never places, so a scope's owner is not a node. (b) A site can bind one name twice (`global g1, g1`, legal on 3.14.7), and `H(binding, site, name)` then collides | DM-11, DM-15, DM-54 · G7 | (a) `lexical.rs:289-319` opens lambda and comprehension scopes regardless of `in_annotation`; only the `Load` arm checks it (`lexical.rs:541`). `walk.rs:619-628` passes the flag. (b) `lexical.rs:465-493` binds each listed name with the statement's syntax id, and `id.rs:108` hashes (site, name). **P9:** the pydantic line fails `ref:scopes.owner_node_id->nodes` and `endpoint:owns_scope`; `global g1, g1` fails `key:nodes` and seven more. Pilot 0; the pilot's dependency tree has 0 lambdas in annotation position in 7,888 files | Both fail closed, so no wrong data is published. But one legal line anywhere in a library blocks the whole snapshot. `Annotated[T, AfterValidator(lambda v: …)]` is documented pydantic v2 usage, and the second pilot library is unknown. The supported scope is narrower than "a library" and says so nowhere | In `enter`, skip scopes and bindings when `in_annotation` (annotations are C4's, like references). Emit one event per distinct name of a `global`/`nonlocal` statement (the repeat is idempotent), or key the binding on the name's own range. About 10 lines | **test:** both lines in a compiled fixture; the snapshot publishes |
| **F7** | DESIGN and the test claims are out of step with C3 | DM-59 · G7 | §4.2.2 L1001-1002 still has the collectors "hand the walk the ranges" (removed). §3.8 L744-746, L762-764 and §8 L1399 still describe the partition as "or published gap" and `graph_gaps` as listing identifier sites for C3; it is `… AND FALSE` (`graph.rs:1100`). Exports now target bindings (`graph.rs:340`, `derived.rs:214-231`), yet §3.2 L424-431, §3.5 L586 ("until C3 gives it a `binding` node"), the §3.8 C1 `exports` row and L771 ("335 are variables") say otherwise; the pilot has 334 bindings and 1 reason. "The last module-scope event by ordinal" is written nowhere. §4.2 L960 and L963 still read "(increment 2)" (C2 F4, marked fixed). L694-695 "each new rule rejects an injected violation": `id:scopes`, `id:references`, `typed:identifier_targets` and `typed:import_targets` have no case. L783 "756 are closure captures": P1 has 760 rows on 734 references. The `introduces` comment (`graph.rs:695`) says a name has no node, but names are placed | A reader takes `graph_gaps` for the partition's live third arm, trusts `variable_origin` as the variable-export state, and takes §3.8 L783 as the regression baseline. §4.2.2 describes a coupling the review's alternative removed | Correct the lines, and relabel "full Python scoping" to what is modelled until F1 and F3–F5 land | prose (no mechanical oracle for spine wording), plus F2's test |

**Observations** (moving none of the three severity classes today; each names where it would
land):

| # | Observation | Evidence | Oracle |
|---|---|---|---|
| O1 | `introduces` inner-joins `nodes` and has no lineage, so a binding whose site is not a node gets no edge silently. On the pilot that is 4,526 import and 31 lambda-parameter bindings, plus match captures (P8). The binding → import-alias link exists only as the coincidence `bindings.site_node_id = export_syntax.node_id`, which nothing declares. `shadows` has no lineage either, though one is easy to state | `graph.rs:687-698`, `graph.rs:736-757`; P5 | a lineage by binding kind (every kind but import, from-import, star, match capture and lambda parameter yields one edge); declare the alias join or give the alias a node |
| O2 | Rules that can only pass. `graph_gaps` has one branch, `… AND FALSE`, so `partition:pysa_calls-gaps` checks an always-empty table. The partition now lives in the lineage rules' complementary predicates. The five `simple` C3 lineages repeat their edge's own predicate (C2's `ast_child` precedent), against §8's "a rule that could only pass … is not generated" | `graph.rs:1093-1104`, `graph.rs:1415-1422`, `simple()` | none. Drop the gaps branch and its rule until a family again has unrepresented rows, and state in §8 that the tautological lineages are kept as edit guards |
| O3 | `imports_module` targets the `from` module: 14 `from pkg import submodule` imports go to the package, and `from . import x` in an `__init__` is a self-loop (2). The `imports_symbol` deferral's reason ("exports trace re-export origins") does not cover submodule imports for the import-graph consumer | P5 | none until the import graph has a consumer (Deferred) |
| O4 | Implicit unbinding is not modelled. An `except … as` name is deleted at the handler's end, yet `return err` after the handler reads it `definite`. Two module-level handler names are now export targets (`fastmcp.cli.exc`, `….openai.e`). `del` and annotation-only events are read candidates (14 reads of annotation-only events on the pilot) | P5, P8 | **test:** fixture line; `binding_kind` filter in `reads_binding` |
| O5 | PEP 695 annotation scopes are not modelled: class and `type` alias type parameters are unbound (P8), and function type parameters bind in the function scope. Pilot 0 (FastMCP supports 3.10) | `lexical.rs:257-271`, `273-288`, `398-405` | **test:** fixture lines, when a 3.12+ library is added |
| O6 | `static_test` is a text match, not Pyrefly's decision, though the codebook says "a branch the analyzer decides statically". `if not TYPE_CHECKING:` is not marked, an `elif`'s own test is not evaluated, and `self.version_info_x` matches. Pilot: 206 marked bindings, 0 `version_info`, no `not TYPE_CHECKING` form | `lexical.rs:122-133`, `codebook.rs` `StaticBranch` doc | **test:** unit test on the four forms |
| O7 | A name used only in annotations is not a reference (declared: C4's), so "no reader" does not mean "unused". 1,458 of 4,526 import bindings have no reader on the pilot | P5 | prose in the `reads_binding` direction text |
| O8 | `export_syntax.resolved_module` is our computation inside a `ruff-ast` raw row, and a reimplementation of Pyrefly's `ModuleName::new_maybe_relative` | `walk.rs:163-186`, `545-553` | folded into F2's correction |

**Checked and clean:**
- Reference ids are roles of placed names (generated rule).
- Identifier sites all land on references (5,755 of 5,755), with a null reason on a miss.
- The C2 F1 split holds (0 `potential` evidence on `site_target`).
- `argument_value` lineage and `unique:syntax_nodes` exist, and their mutation cases fire.
- Comprehension first iterables and the class-body skip resolve as Python does (snapshot rows
  `lex/__init__.py:603` and `:617`, byte offsets). Walrus binds in the enclosing function (row
  `:965`). Closure captures through `nonlocal` stores work when the outer binding comes first
  (row `:530`).
- `reads_builtin` lineage covers every builtin function or class row.
- Determinism tests pass.
- Star-import absorption does **not** mask anything on the pilot (P3). The pilot's zero
  unresolved count is plausible for a type-checked library, apart from the 119 implicit globals
  (F1).

### 7.2 Applicability and verdicts

- **Bore on this scope:**
  - Group 1 (authority; the semantic boundary): F1, F5, O8.
  - Group 2 (absence, validity): F1, F2, F3.
  - Group 3 (identity): F6.
  - Group 7 (relationship structures): F4, O1, O3.
  - Group 9 (provider boundary: builtins and imports from Pyrefly): F1, F2.
  - Groups 10–12 (lineage, regression controls, claims): O1, O2, F7.
- **Did not bear:**
  - Group 4: no new declaration surface beyond registry entries of C1's shape.
  - Group 6: no new effects. Publication is unchanged.
  - Group 8: the recognizer runs inside the 0.49 s walk. The 3.83 GB peak is in `validate` and
    comes from graph size, which C6 owns (§4.3). The C2-alternative and C3 shares of it cannot be
    separated from `pilot.txt`, and no claim here rests on the split.

| Verdict | Principles |
|---|---|
| Satisfied | DM-06 (scope, binding and branch kinds as codebooks), DM-09 (endpoint kinds generated; F4 aside), DM-10 (resolution queryable as rows with candidates), DM-40 (determinism **Tested**), DM-51 (schema snapshots; the commit is labelled a migration), DM-52 (rules generated from the registry) |
| Violated | DM-08 (F1, F2, F3), DM-07 and DM-02 (F2), DM-04 (F1, F3–F5: what the recognizer does not model is undeclared), DM-11/DM-15 (F6b), DM-54 (F6a: no annotation boundary case), DM-59 (F1, F3–F5, F7), DM-60 (F2, O1) |
| Unresolved | DM-43: the Python syntax the recognizer supports (PEP 695 scopes, O5) is not declared as a limit |

**Guidelines MUSTs (ADDENDUM §5), for C3:**
- **§2 edge identity, typed endpoints, evidence and snapshot scope: met.** The parallel kinds
  (`potential_target`, `imports_module`) carry payload or alias discriminators.
- **§2 isolates: met** (existence sources for scope, binding and reference).
- **§2 distinguishable derivations: met** (`reads_binding` / `captures` / `reads_builtin`;
  `potential_target` split by modality).
- **§2 unknown targets explicit: partly** (F1 absorbs misses; F2 invents a provider reason).
- **§7 partial ≠ complete: partly.** "No name is unresolved" reads as completeness and is partly
  an artefact of F1.
- **§12 known-answer shapes: partly.** `lexical_shapes` has no genuinely unresolved name (its one
  candidate is absorbed by the star import), and no handler, match, default, decorator, type
  parameter, implicit-global or module-flow shape. Lineage is **met** except O1. Per-stage
  metrics are **met** (`derive identifier_targets`, `derive import_targets`; the walk timing
  includes the recognizer).

## 8. Alternatives (compressed)

| Alternative | Duplication and extension locality | Risks | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| Current: own recognizer; builtins = Pyrefly's full `builtins` export table; star as a fallback candidate; own relative-import arithmetic; reason by join miss | Relative-import naming exists twice (ours and Pyrefly's `new_maybe_relative`). "Outside the module" is decided by three approximations | F1, F2, F6 | 843 + ~120 lines | **Measured** (author, 2026-09-23; `pilot.txt` read here): walk 0.49 s including the recognizer; total 17.4 s, peak 3.83 GB (`validate`) | Selected by the author |
| Current with F1–F6 corrected in place | Same structure, with the fixes local to `lexical.rs`, `lib.rs` and `derived.rs` | O-items remain | ~+80 lines, ~10 fixture lines | same order (**Proposed**) | Minimum for Accept |
| **Simpler viable: the recognizer owns binding history, Pyrefly owns every name set outside the module text** | Implicit globals from `ImplicitGlobal::implicit_globals`; builtins filtered to `ThisModule` non-implicit public definitions; each star import's `Exports::wildcard`; each import's module named by `ModuleName::new_maybe_relative`, with the finder's found / not-found kept as a column. Removes `absolute_module`, the star fallback branch and the catch-all. `unresolved_target` then means "neither the module text nor Pyrefly's sets bind it" | The fork patch may need a visibility change for `wildcard` (ADR-0012's ≤60-line budget). F3–F5 still need their local fixes | About −30 lines of ours, +1 `get_exports` per star-imported module (0 on the pilot), one filter on the builtins map (**Proposed**) | negligible against the walk (**Proposed**, unmeasured) | Recommended. F1 and F2 converge on it, and it puts one authority behind every resolution that leaves the module, as §B1 already requires for semantics |

**What stays ordinary code:** the recognizer's scope stack, stores and resolution. No declarative
scoping language is warranted. The fixes are a parent lookup, a routing call and a fallback
candidate.

## 9. Top verification gaps

| Claim or risk | Label now | Check | Expected result | Gap |
|---|---|---|---|---|
| Resolution follows Python scoping | **Implemented**; **Tested** on the `lexical_shapes` shapes only; contradicted by P6–P8 | fixture lines for P6–P8 (implicit globals, stub-only names, star shadowing, module/class flow, decorator and default scopes, lazy `global` import, `nonlocal` ordering, handlers, type params) | each reads Python's target, or `candidate` including it | F1, F3–F5, O4, O5 |
| An import reason is the provider's | **Implemented**, unfalsifiable (P10) | the `resolved_module` corruption case | fails `typed:import_targets` | F2 |
| Any legal module compiles | not claimed, and false (P9) | P9's two lines in a fixture | publishes | F6 |
| The new C3 rules fire | **Tested** for `id:bindings` and `typed:reference_resolutions` | cases for `id:scopes`, `id:references`, `typed:identifier_targets`, `typed:import_targets` | each fails its rule | F7 |
| "No name is unresolved" on the pilot | **Measured** (P1: 0 `unresolved_target`), with 119 mis-attributed | rerun `just pilot` after F1 | 0 unresolved, 0 `__name__` read as a builtin | F1 |

## 10. Exceptions and unresolved decisions

No SHOULD-level exception is requested. Decisions the author has to make:
- **F1 and §8:** whether the outside-the-module sets come from Pyrefly (recommended) or from local
  fixes to today's approximations. Either way, implicit globals need a representation: an
  appended `binding_kind` code (append-only) or a resolution class of their own. This needs no
  ADR unless the choice is to keep the full export table as "builtins".
- **F3:** whether module and class scopes add the outer candidate always, or only for references
  that precede the first local event.
- **O2:** keep `graph_gaps` as an empty published table with no rule, or drop it until a family has
  unrepresented rows. §3.7 and §3.8 have to say which.

## 11. Decision

**Decision: Revise** (small surface).
**Reason:** The family is well shaped. Scopes, binding history with static branches, references as
roles of placed names, and flow-insensitive candidates serve Pass B's §4.2.4 rule and the forwarding
and handoff recognizers in function scopes. The C2 fixes landed, and the simpler placement
retired that review's two causes. But C3 publishes resolutions that are wrong under Python's own
rules and reads them as `definite`. 119 implicit-global reads on the pilot are attributed to
builtins. Stub-only names, star-imported names and unbound names in star-importing modules resolve
silently (F1). The design's own consumer contract, "choose by ordinal", fails in module and class
scopes (F3). Decorator and default scopes resolve through the wrong function (F4). It also
reintroduces the catch-all reason that ADR-0014 forbids, for imports (F2), and it cannot compile
two kinds of legal source (F6). Each fix is local and cheap now; each gets costlier once C5
compiles examples and test suites through the same recognizer.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: outside names from Pyrefly's sets (implicit globals, filtered builtins, star wildcard sets before builtins) | DM-08, DM-04 | pilot: 0 `__name__` read as a builtin; P6 reads Python's targets | test: the `lexical_shapes` lines, plus a star-free module |
| 2 | F2: provider-backed import reason; `variable_origin` only for non-release origins | DM-08, DM-07, DM-02 | P10 rejected | test: the mutation case |
| 3 | F3, F4, F5: fallback candidates in module and class scopes; parent from `scope_at`; route every binding event and decide `nonlocal` at `finish` | DM-04, DM-09, DM-59 | P7 and P8 read Python's targets; pilot `lexical_parent` for `debug.py` is the class scope | test: fixture lines; the snapshot renders scope owners |
| 4 | F6: no scopes or bindings in annotations; one event per distinct `global`/`nonlocal` name | DM-11, DM-15, DM-54 | P9 publishes | test: fixture lines |
| 5 | F7: correct the DESIGN lines and the rule-coverage claim | DM-59 | — | prose; the four mutation cases |

### Deferred

| Item | Why not now | Trigger that reopens it |
|---|---|---|
| O1: `introduces` lineage and the alias join | Pass B and Pass C reach parameters and names; nothing reads import bindings → alias yet | Import graph or `imports_symbol` consumer lands |
| O2: vacuous gaps rule and tautological lineages | Harmless; a decision, not a defect | Next family with unrepresented raw rows, or the §8 wording is touched |
| O3: submodule imports | `imports_symbol` is deferred with a consumer trigger | Import-graph consumer specified |
| O4: handler `del`, non-value read candidates | 0 reads after a handler on the pilot | Pass C over examples (C5) |
| O5: PEP 695 scopes | 0 on the pilot | A library whose `requires-python` allows 3.12 syntax is added |
| O6: static-branch heuristic | 0 `version_info`, 0 negated forms on the pilot | Pass B reports a `TYPE_CHECKING` explanation, or a library with `sys.version_info` branches |
| O7: annotation-only uses | Declared as C4's | C4 lands type references |
| O8: `resolved_module` authority | Folded into F2 | F2's fix |

## Disposition (author, 2026-09-23)

The author took the simpler alternative and fixed F1–F7 in one commit after CPG slice C4
(`3f0f385`), which had already added the one fork accessor the alternative needs
(`Transaction::get_wildcard`, fork revision `6a93da34`).

| # | Outcome | What changed | Oracle |
|---|---|---|---|
| Simpler alternative | taken | Every name set from outside a module's text is Pyrefly's: its `ImplicitGlobal` set, its `builtins` definitions that are real public names (`ThisModule`, not private, not an implicit global), each star import's `get_wildcard` set. Imports are named by Pyrefly's `ModuleName::new_maybe_relative`, and the finder's answer is kept. Our relative-import arithmetic and the star fallback branch are gone | the `lexical_shapes` snapshot; the pilot |
| F1 | fixed | Module implicit globals are `implicit` binding events at the module's start (a `binding_kind` append), so `__name__` reads the module's own name. A method's `__class__` reads its class's cell (an `implicit` binding of the class, made when read). Star imports bind their wildcard set before builtins; a star module Pyrefly cannot find stays a `candidate` for any otherwise unbound name; a name in no set is `unresolved_target` | test: `lex/flow.py` (`__name__`, `__file__`, `__doc__`, `__class__`, an unimported `Callable`) and `lex/__init__.py` (`open` from the star import; the class-comprehension `name` now `unresolved_target`). Pilot: the 118 `__name__` and 1 `__file__` reads resolve to implicit bindings; 3 real builtin variables remain; 0 unresolved |
| F2 | fixed | A module an import names and Pyrefly's finder cannot find is a `context_modules` row of origin `not_found` (a `module_origin` append) with no node. `import_targets` gives `unresolved_target` only for such a row or an import past the top package, else null, so a misnamed import fails `typed:import_targets`. `exports` gives `variable_origin` only for a dependency origin | test: P10's corruption of `resolved_module` now fails `typed:import_targets` (`compile.rs`). Pilot: every import found, one `variable_origin` export (a dependency origin) |
| F3 | fixed, "precedes" variant | A module or class body reading a name it binds only later also gets the outer resolution (the enclosing function scopes for a class, then star sets and builtins) as a `candidate`. An outer "unresolved" adds nothing, since a module-level loop can read the later binding | test: `r = len; len = 5` and `a = type; type = "k"` read both. Pilot: 0 cases, as P4 found |
| F4 | fixed | A scope's parent is `scope_at` its own start | test: the decorator lambda and the default comprehension in `outer_scope` read `outer_scope`'s `v`. Pilot: `debug.py`'s default lambda has the class scope as parent |
| F5 | fixed | Binding events are collected and emitted in `finish`. Every kind but the declarations themselves, the star marker, parameters and implicit names follows `global` (at the event) and `nonlocal` (decided in `finish`, through scopes that declare the name `nonlocal` themselves). Ordinals are source order per scope | test: the lazy `global json_mod; import json as json_mod` binds at module level; `nonlocal v; v = 2` before `v = 1` binds in `order`. The snapshot renders each scope by its owner |
| F6 | fixed | Nothing inside an annotation opens a scope or binds; a repeated name in one declaration is one event | test: `Model.tags: Annotated[list[str], (lambda: [])]` and `global g1, g1` publish |
| F7 | fixed | DESIGN §3.2 (the `lexical` and `exports` text: what is modelled, what is not), §3.5, §3.7, §3.8, §4.2.2, §8 and the stale "(increment 2)" rows. New injected-violation cases: `id:scopes`, `id:references`, `typed:identifier_targets`, `typed:import_targets`. The captures are 760 rows on 734 references | `every_rule_kind_rejects_its_violation` (23 cases); prose |
| O1 | deferred | as the review's table | — |
| O2 | decided | `graph_gaps` stays a published, empty table with its rule; §3.7 and §3.8 say so. The tautological `simple` lineages stay as edit guards | prose |
| O3, O4, O6, O7 | deferred | as the review's table | — |
| O5 | deferred, declared | PEP 695 annotation scopes are listed as not modelled in §3.2 (DM-43) | prose |
| O8 | fixed | by F2 | — |

**Checks** (2026-09-23): `just test-all` passed (nextest 74/74, including 23 injected-violation
cases; pytest 21/21; rule tests 4/4; `lint-agents`; `adr lint`; fixtures 33; family; cargo-deny;
`pyrefly-fork` at `6a93da34`; gold). `just pilot` passed: snapshot `076151d4…`, every rule; 25,530
bindings, 45,713 resolutions (0 unresolved, 3 builtin variables), 244,673 nodes, 391,193 edges;
19.0 s at 3.93 GB.
