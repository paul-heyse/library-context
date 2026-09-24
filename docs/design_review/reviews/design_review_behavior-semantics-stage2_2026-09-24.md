# Design review: ADR-0022's Stage 2.1 decisions (standard)

**Date:** 2026-09-24 · **Depth:** standard. ADR-0022 changes §B5, and its D-2 amendment to ADR-0012
changes §B1. So a `standard` review is owed before acceptance (ADR-0001), and ADR-0022 §Decision
names this review as the gate. · **Mode:** document review. Three kinds of source are read only
where a claim depends on them: `ty_python_core` 0.0.14, the repository's Stage 1 code and the
pilot's release source.

**Target** (working tree on `8b59d69`):
- `docs/adr/0022-behavior-model-semantics.md`:
  - the five new subsections, §The flow provider (D-2), §Conditions (F5), §Verdicts (F6), §Identity
    (F9) and §Composed layers (F11);
  - the amended verdict table and Consequences.
- `docs/adr/0012-pyrefly-ruff-in-process.md`: the 2026-09-24 amendment (a second parser line
  confined to `cpg-flow`).
- `docs/design/DESIGN.md` §3.9, as amended (`git diff`).

**Context:**
- `design_review_behavioral-model-adrs_2026-09-24.md`: its F5, F6, F9 and F11, and its §12;
- the plan's Stage 2, §7, §10, §11 and §15;
- deviation B11;
- `eval/behavior/fastmcp-4.0.5.toml`, Q04 and Q10.

**Reviewer:** `design-reviewer` subagent, with fresh context. **Author:** the session executing the
behavioral-model plan.

## Answers to the five questions

**1. Do the decisions close F5, F6, F9 and F11 as posed?** Mostly in form. Five points remain where
two implementers would build different things.

| Earlier finding | What it asked | Now | Status |
|---|---|---|---|
| F5 polarity | Atoms carry a polarity | `[!]kind(…)`; `is not None` is `!is_none` | **Closed** |
| F5 disjunction | A bounded set of conjunctions | DNF of at most 16 × 8, absorbed | **Closed in form.** The lowering from ty's ternary decision diagrams is not fixed (F2) |
| F5 places | One grammar for §3.9 and §9.9 | "One grammar serves conditions, `value_flows` and §9.9's access paths" is asserted, but §9.9 still reads `Parameter[self].Field[f]` and `Global[<module>.<name>]`. Spelled and resolved forms are not reconciled | **Open** (F4) |
| F5 literals | Type-tagged literals; `x in {a}` read as `x == a`; `isinstance` compatibility | `None`/`True`/`False`, decimal integers and JSON strings; everything else is opaque | **Mostly closed.** Some encoding corners are open (F10). Compatibility rules are unstated (F11 recommends deferring them) |
| F5, today's predicates | How §9.2's supported predicates map | Not addressed | Deferred with the earlier F7, to Stage 2.6 (§10) |
| F6(a) budget | One verdict for a budget cut | `unknown` (`budget_reached`); `not_analyzed` narrowed | **Closed** |
| F6(b) opaque | The record's verdict | `conditional`, with the text shown | **Closed** |
| F6(c) region, boundary kinds, dynamic access | A rule that can be written | Region = Pass B's reach = `operations.behavior_status`; the release for module globals; `dynamic_access` and `override_dispatch` added | **Partly closed.** The rule can be written for parameter-forward claims only (F1). The reach relation is incomplete (F3). §3.9's places paragraph still sends computed names to `unresolved_target` (F6) |
| F6(d) modality and `evidence_status` | State the relations; add a row to ADDENDUM §3 | Both relations stated; ADDENDUM §3 unchanged | **Closed in the ADR.** The ADDENDUM row is missing (F6). The may/must reading is implicit (F2) |
| F9 provider | Revision and settings in identity | The ty revision and "the runtime view's version" are in `producers.revision`; `contexts` carries the Python version and platform | **Closed.** The second counter duplicates `EXTRACTOR_OUTPUT_VERSION` (O5) |
| F9 registry | Labels, digest, gold rule | The id comes from the key, and labels are digested | Identity **closed**. The gold rule for concepts and the membership `basis` are deferred to Stage 4 (§10) |
| F11 | Where the checker and runtime views meet | The CPG keeps the checker view; a behavior takes the runtime view at its site; a path bound only under `TYPE_CHECKING` is `unknown` | **Closed**, except a name defined in both branches, the case F11 posed (F9) |

Would two implementers build the same thing from this text alone? **No, at five points:**
- **The region of a negative claim about a field or a module global.** Both Q04 and Q10 turn on
  one (F1).
- **ty's third truth value** (AMBIGUOUS). It appears at every `try`, most `for`s and every `with`
  (F2).
- **The receiver of a dynamic access** (F3).
- **Place identity across modules** (F4).
- **Encoding corners** (F10).

**2. Is `semantic:refuted-needs-complete-region` writable, and is the region decidable?**
- **Writable** as one join for claims about a parameter's forwards. There the region really is
  Pass B's reach, and `operations.behavior_status` is per operation.
- **Not writable as stated for fields and module globals:**
  - A field's reads sit in other methods and in base classes, outside the storing operation's
    Pass B reach. The rule would check the wrong region and pass a false refutation.
  - For a module global the "region is the whole release". That region has no operation row to
    join. Its "no call site … open" conjunct can never hold on a real release.
- **Decidability.** Conjuncts 1–3 are decidable from existing tables. Conjunct 4 ("no dynamic
  access … can reach the place") is decidable only for accesses that have a receiver, and only once
  the receiver is resolved (F3). It is per place, so a per-operation column cannot hold it "exactly"
  (F1).

**3. Is the `TYPE_CHECKING` rename a sound runtime override, and does ty 0.0.14 decide anything else
at index time?**
- **The rename is sound on these points:**
  - **Byte ranges:** both spellings are 13 ASCII bytes.
  - **Strings and comments**, if the rename works on tokens, as stated.
  - **`typing.TYPE_CHECKING`:** `is_if_type_checking` matches `Attribute.attr`, which the rename
    also changes (`builder.rs:6207-6215`).
  - **Imports and a module-local `TYPE_CHECKING = False`:** every token in every release module is
    renamed alike.
- **Unstated:**
  - which lexer yields the tokens;
  - what happens if the sentinel already occurs in the source;
  - that the evaluator, the place text and the opaque text must read the sentinel, or the original
    bytes, rather than ty's AST spelling;
  - that `from typing import TYPE_CHECKING as TC` is decided by neither ty nor our view. That is
    imprecise, not unsound, and none occurs on the pilot (F7).
- **"ty decides no other test that differs from runtime"** holds against `resolve_to_literal`
  (`builder.rs:2197-2215`). It also folds `not <foldable>`, which is runtime-sound.
- **ty 0.0.14 makes three other index-time reachability decisions:**
  - **Literal iterables.** A `for` over an empty literal iterable is unreachable, and one over a
    non-empty literal is definitely entered (`builder.rs:4761-4763`, `6194-6205`). Both are sound.
  - **Handlers of a `try` with no checkpoint.** A `try` suite with no exception checkpoint has
    unreachable handlers (`builder.rs:5066-5078`). What counts as a checkpoint follows ty's
    "known safe" model, which ignores ambient exceptions (`builder.rs:2490-2588`). That departs
    from runtime at the margin, and the stated model does not name it (F7).
  - **The AMBIGUOUS terminal.** It is not a decision but a third value that the two-valued DNF has
    no mapping for (F2).

**4. Is the dynamic-access model proportionate and honestly labelled?**
- **In kind, and in labelling, yes.** Five access kinds, `evidence: Proposed`, and the
  untyped-receiver assumption named in every negative answer.
- **Read literally, it misses the pilot's own example.**
  - `Settings.get_setting` reads through a local alias: `settings = self`, then
    `getattr(settings, attr)` (`settings.py:51,57`). So the stated rule, "its receiver is `self`",
    does not match.
  - An implementer who reads it syntactically refutes "`server_dependencies` is never read". That
    is Q04.g, the pre-registered misleading item.
- **Two gaps besides.** Typed receivers other than `self` are unaddressed. The receiver-less kinds
  (`import_module`, `__import__`, `exec`/`eval`, a module-level `__getattr__`) reach nothing
  stated (F3).

**5. Over-built for Stage 2, or missing for its exit?**
- **Over-built:**
  - The three-valued compatibility evaluator and its JSON corpus. Neither Q04 nor Q10 asks whether
    two conditions can hold together, and Q9, which does, is Stage 3's (F11).
  - A second hand-bumped identity counter (O5).
- **Missing for Q04 and Q10:**
  - per-place premises (F1);
  - a resolved place key (F4);
  - the AMBIGUOUS mapping (F2);
  - a **read phase** (import, construction snapshot, per call), which Q04 grades and nothing defines
    (F8).
- **Q10.d at best `partial`.** It needs its condition carried through a local
  (`effective_mode = self.mode`, `client.py:866`) and through a field. The Stage 2 decisions do not
  provide that. Expect `partial`, and say so in the Stage 2 packet rather than growing Stage 2.

---

## 1. Decision and scope

**Proposal.** The Stage 2.1 decisions of ADR-0022:
- **The provider.** `ty_python_core` 0.0.14 supplies the flow IR's raw material in `cpg-flow`,
  behind a byte-range parity rule and a same-length `TYPE_CHECKING` rename.
- **Conditions.** They are in DNF over polarized atoms, with a fixed encoding and a 16 × 8 budget.
- **Verdict rules:**
  - an opaque condition gives `conditional`, and a budget cut gives `unknown`;
  - a refutation premise, and two new boundary reasons;
  - how verdicts map onto `modality` and `evidence_status`.
- **Provider identity.**
- **How the checker and runtime views compose.**
- **ADR-0012's amendment** admits the second parser line.

**Status.** Every clause is **Proposed**, except the spike's numbers, which are labelled
*Measured* (O4).

**Affected revisions.** Working tree on `8b59d69`. The implementation began in the same tree during
this review (Method).

**Observable outcome.** An agent asks Q04 or Q10 and gets:
- each read with its condition and phase;
- `refuted_under_model` only where the premise holds;
- a named `unknown` where it does not.

**Baseline.** Stage 1:
- Pass B's recognizers, and per-operation `behavior_status`;
- no negative fates (the earlier F2 disposition);
- the `verdict` codebook with no refutation rule.

**In scope:** plan Stages 2.1–2.5 and the Stage 2 exit (Q04, Q10). **Not in scope:** Stage 3's
summaries and models, and Stage 4's registry, beyond the identity sentences.

### Method and coverage

**Read in full:**
- **The standard:** the skill, the charter's framework (§A–§H), the ADDENDUM, REVIEW_REFERENCE and
  the template.
- **The ADRs:** ADR-0022 and ADR-0012, and the ADR-0002 amendment.
- **DESIGN:** the DESIGN diff, §3.9 as amended, §B1, §B5, §9's behavior-scan paragraph
  (L2115–2130) and §9.9.
- **Context:** the Stage 0 standard review; the plan's Stage 2, §7, §10, §11 and §15; B11; Q04
  and Q10.

**Source read myself:**
- **`ty_python_core-0.0.14/src/`** (Interface-checked, 2026-09-24):
  - `predicate.rs` in full;
  - `reachability_constraints.rs:1-140` and `370-560`;
  - `use_def.rs:940-1000` and `2110-2140`;
  - `builder.rs`:
    - `build_predicate` and `resolve_to_literal` (2188-2240);
    - `is_if_type_checking` (6207-6226);
    - the known-safe model (2490-2588);
    - `record_ambiguous_reachability` (2590-2594) and its four callers (4714, 4779, 5032, 5296);
    - `Try` (5022-5078, 5240-5280);
    - `Assert` (4241);
    - `literal_iterable_truthiness` (6194-6205);
    - `is_direct_range_call` (6126).
  - **Environment reads** across the four ty crates (grep).
- **Repository:**
  - `cpg-schema/src/behavior.rs` (the `operations` table);
  - `cpg-core/src/behavior.rs:540-575` (how `behavior_status` is computed);
  - `cpg-schema/src/rules.rs:620-636`;
  - `cpg-extract/src/config.rs:25-35` and `408-430` (the producer id);
  - `rules/no-catch-unwind-in-extractor.yml`;
  - `scripts/check_family.py`;
  - `HEAD:crates/cpg-schema/src/codebook.rs:157-159`.
- **The pilot's release** (`build/envs/fastmcp/.../site-packages`):
  - `settings.py:46-70`;
  - `client/client.py:190, 446, 461, 661-663, 837, 866-887`;
  - `client/mixins/tools.py:63`;
  - `_compat.py:119-124`;
  - `fastmcp/__init__.py:20-21, 49-92`.

**Measured myself** (2026-09-24, read-only `grep` over `fastmcp` and `fastmcp_tasks`):
- **`TYPE_CHECKING`.** Its 75 tests are all `if TYPE_CHECKING:`; every other occurrence is an
  import. There is no `as` alias, and the sentinel `TYPE_CHECKIN_` never occurs.
- **Module-level `def __getattr__`:** 6, one of them in `fastmcp/__init__.py:49`, the module that
  binds `settings`.
- **`importlib.import_module(`:** 10, including literal names at `fastmcp/__init__.py:87,92`.
- **`vars(`:** one real call, `vars(session)` at `client/client.py:190`.
- **No builtin `exec(` or `eval(` call.** The three `exec(` hits are `create_subprocess_exec` and a
  docstring.
- **`libraries/fastmcp/.python-version`:** 3.14.7.

**Checks run:**

| Command | Outcome |
|---|---|
| `just adr lint` | **passed** (`adr lint: ok (22 records)`) |
| `just lint-agents` | **passed** |
| `just check`, `just test-all`, `just pilot` | **not_run.** This is a document review. Implementation files appeared in the tree while it ran (below), so an outcome would not be attributable to the target |

**Not reviewed:**
- **The implementation in flight.** These appeared or changed in the tree during the review, at
  07:02–07:05 on 2026-09-24:
  - untracked: `crates/cpg-flow/`, `crates/cpg-schema/src/condition.rs`,
    `crates/cpg-schema/tests/conditions.rs` and `specs/serving/conditions.json`;
  - modified: `Cargo.toml`, `codebook.rs` and `lib.rs`.

  Only the ty pins in `Cargo.toml` are cited (O1). The working tree's `boundary_reason` appends
  (`dynamic_access` = 16, `override_dispatch` = 17) are noted, not reviewed.
- **The spike's code.** B11 says it ran outside the repository.
- Plan Stages 3–5.

**Guarantees not attacked, so asserted:**
- the spike's numbers (213 ms, 47 MiB, the parity counts);
- that ty's use-def map is right for exceptional flow;
- that 16 × 8 fits the pilot's conditions;
- that the salsa 0.28.2 pin holds;
- **how far AMBIGUOUS spreads.** That is from source reading, not execution, and F2 names the check.

---

## 2–4. Authority, contracts and derivation (compressed)

**Authority map** for what this change adds. ⟂ marks a cell the text leaves undecided.

| Concept | Identity | Authority | Revision boundary | Gap |
|---|---|---|---|---|
| Flow uses, definitions, reachability | Module and byte range; producer-scoped | ty 0.0.14 through `cpg-flow`, normalized | `producers.revision` | ⟂ lowering of the AMBIGUOUS terminal (F2); parity guarded one way (F5) |
| Runtime view | — | Our evaluator, over renamed source | `contexts` (version, platform) | ⟂ whether it reads the sentinel or the original bytes (F7) |
| Places | Spelled text in the record's scope | ADR §Conditions | — | ⟂ a resolved key for joins across the release; §9.9 is a second grammar (F4) |
| Conditions | `H("condition", encoding)` | The encoder | `compiler_digest` | ⟂ encoding corners (F10) |
| Refutation premise | `operations.behavior_status` | The Pass B scan | Snapshot | ⟂ a premise per place; the release region (F1) |
| Dynamic-access reach | — | The stated model | — | ⟂ aliases, typed receivers, receiver-less kinds (F3) |
| Provider pins | `Cargo.toml`, `docs/pins.md` | ADR-0012 amendment; ADR-0002 declared family | Lockfile | `pins.md` rows (O1) |

**Invariants:**

| Invariant | Enforcement boundary | Failure | Status |
|---|---|---|---|
| Flow uses ⊆ `references`, except annotations | A `semantic:` parity rule | Validation rejects | Proposed; decidable |
| `references` ⊆ flow uses; `bindings` ↔ flow definitions | **None** | — | Missing. This is the direction that protects negative claims (F5) |
| ty's reaching definitions ⊆ C3's candidates | The spike only | — | Missing as a standing rule (F5) |
| `refuted_under_model` only under a complete premise | `semantic:refuted-needs-complete-region` ⋈ `operations.behavior_status` | Validation | Right for parameter forwards; the wrong region, or no target, for fields and globals (F1) |
| One id per condition | The encoder, plus a unit test | Test | Proposed; corners open (F10) |
| The rename moves no byte | None named | — | Test proposed (F7) |
| ty folds no test the runtime view decides | None (it is a revisit trigger) | — | Missing (F5) |

**The absence lattice (DM-08).**
- **Now distinct:** a budget cut (`unknown`) against out of scope (`not_analyzed`); an opaque
  condition (`conditional`) against a boundary (`unknown`); a module with no flow IR
  (`syntax_error`, `undecodable_source`, both existing reasons).
- **Still collapsing:**
  - ty's AMBIGUOUS terminal, which could read as `true`, as opaque or as `unknown` (F2);
  - a class operation's `not_analyzed` status against a refutation about its constructor's
    parameter. `semantic:behavior-covers-public` forces a class row to be `not_analyzed`
    (`rules.rs:634`), and the rule's join target for constructor parameters is not stated (F1).

**The ty boundary (DM-41, DM-42).**
- **What crosses** is byte ranges, place text and our condition data.
- **The lowering is the one semantic transformation.** It takes a scope's ternary decision
  diagram over ty predicates (`reachability_constraints.rs:14-33`), maps its atoms, applies the
  runtime view, and builds a DNF.
- **Its contract covers:**
  - four of ty's eleven `PredicateNode` kinds, `IsNonTerminalCall` and three read as opaque
    (`predicate.rs:162-220`); the others fall to "any other test is opaque", which is decidable;
  - not the terminal AMBIGUOUS (F2).

**Publication.** Unchanged. Flow facts are extractor output and go through the same attempt,
validation and `snapshots` append.

---

## 5. Journeys (compressed)

**Extension: promote ordering comparisons to an atom.** 32 of 522 raising `if`s are ordering
comparisons (Stage 0 review, Measured).
- **What it takes:** one `condition_atom` append, an encoder case and known answers.
- **Consequence:** records whose test was opaque get new condition ids. That is expected, and
  `compiler_digest` moves.
- **Locality holds.**

**Meaningful change: a ty bump to 0.0.15.** `producers.revision` moves `producer_id`, which is
correct (DM-31). The two failure modes the revisit trigger names have no oracle:
- ty stops indexing some use, so a `references` row has no flow use;
- ty starts folding another test at index time.

So a bump that passes the one-way parity rule can silently drop reads (F5).

**Boundary: ty's AST becomes our condition data.**
- After the rename, ty's `if` test is the name `TYPE_CHECKIN_`.
- An evaluator written against the ADR's sentence ("`TYPE_CHECKING` … as false") would leave the
  block `conditional` on `truthy(TYPE_CHECKIN_)`.
- Place text cut from ty's AST would publish the sentinel (F7).

**Adversarial, on the pilot:**
- **a. Q04.g.** `get_setting` reads through `settings = self` (`settings.py:51,57`).
  - Read syntactically, the receiver is not `self`, so the access is an untyped one.
  - The untyped receiver is assumed not to reach `Settings`.
  - So "`server_dependencies` is never read" is refuted. That is the pre-registered misleading
    claim (F3).
- **b. Q10.d.** `Client.__init__` stores `prior_discover` to `self._prior_discover`
  (`client.py:461`).
  - It is read in `Client.prior_discover` (`:661-663`) and in `Client._negotiate` (`:886`, inside
    the `try:` at `:870`).
  - Neither method is in `__init__`'s Pass B reach, which follows parameters only (DESIGN §9, L2117).
  - A refutation "`prior_discover` is never read", judged on `__init__`'s region, passes the rule
    (F1).
  - The read at `:886` also carries AMBIGUOUS from the `try` (F2).
- **c. Q10.f.** `Client.__init__` writes `self.name` (`client.py:446`), and the read is in
  `ClientToolsMixin` (`client/mixins/tools.py:63`), a **base** class. A field identity keyed by the
  writing class, or "`self` in `C` or a subclass", misses that read (F1, F3).

**Interruption: ty panics on a module.**
- ADR-0012 §Panics aborts on panics "in code that touches Pyrefly".
- `rules/no-catch-unwind-in-extractor.yml` covers `crates/cpg-extract/src/**` only.
- So whether a ty panic aborts the attempt or degrades one module to "no flow IR" is unstated, and
  a `catch_unwind` in `cpg-flow` would not be flagged (O3).
- Either choice is safe for publication. Only the first is the ADR-0012 default.

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **fail** (documentary) | See F6 for the contradictions between the spine and the new decisions. The main ones: §B1 still says "No second parser or type checker" (DESIGN L261; the ADDENDUM §1 row) against ADR-0012's amendment. §B5 L336–337 still says the provider "is decided by the Stage 2.1 spike … either … or". ADR-0022 L56 says "Normal form: sorted conjuncts" against §Conditions' DNF. §9.9's path grammar contradicts "one grammar" (F4). §3.9 L1110–1111 sends computed names to `unresolved_target` against `dynamic_access`. The Stage 0 review's deferred O3 named the spike's outcome as its trigger, and that trigger has fired | Sentences (F6, F4) |
| G2 Semantic fidelity | **unresolved** | The earlier F6(a), (b) and (d) close. Open: the AMBIGUOUS lowering and its procedure (F2); the region for field and global places (F1); receiver resolution and the reach of receiver-less kinds (F3); the resolved place key (F4); ty's exception assumption (F7); a name defined in both branches, and one reason code used with two meanings (F9); encoding corners (F10) | F1–F4 before Stage 2.5; F7, F9 and F10 with their tests |
| G3 Validity | **unresolved** | The refutation rule has an identified boundary, but for field places it checks the wrong region and passes an invalid row (F1 and journey b). Parity rejects only the harmless direction, so a ty bump that drops uses reaches publication (F5) | F1; F5 (two-way parity, reaching ⊆ candidates) |
| G4 Hidden behavior | **pass** (document stage) | The ty revision is in `producer_id`. The runtime view's inputs are in `contexts`. The rename is a declared transformation. The ty line's environment reads are parallelism knobs only (`ruff_db-0.0.14/src/lib.rs:100-101`), though `just deps` does not classify them the way it does the Pyrefly fork's (O2). The gold rule for concepts is Stage 4's (§10) | O2 |
| G5 Consistency and recovery | **pass** | The publication protocol is unchanged, and a panic aborts the process before any `snapshots` append. The panic policy should still be stated (O3) | O3 |
| G6 Transformation and reuse | **unresolved** | The TDD → DNF lowering has no complete contract: the AMBIGUOUS terminal, the procedure, saturation (F2). Identity is sound; the second counter is redundant (O5) | F2 |
| G7 Truthful capability claims | **unresolved** | Stage 2's exit claims Q04 and Q10. Q04's "when is each one read" has no defined read phase (F8). Q04's dead settings are unreachable as refutations under the release region as written (F1). ty's fitness for conditions (`try`, `with`, `for`, `match`) was not exercised by the spike, whose `flow_shapes` exit test moved to `cpg-flow`'s tests (B11; O4) | F8; F1; run `flow_shapes` before Stage 2.5 |

An unresolved gate is not a pass. G1's failure is documentary: sentences fix it.

---

## 7. Findings

Ordered by cause severity. First come correctness and authority on Stage 2's in-scope behavior
(F1–F7), then exit readiness (F8, F9), then encoding (F10), then proportionality (F11).

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **The refutation premise is defined per operation (Pass B's parameter reach), but Stage 2's negative claims are about places. For the two place kinds Q04 and Q10 turn on, fields and module globals, the region is wrong or can never be satisfied, and the rule has no target** | DM-08, DM-07, DM-59 · G2, G3, G7 | ADR-0022 §Verdicts: "An operation's **region** is the set of functions Pass B reaches from it"; "That is exactly `operations.behavior_status = established`"; "For a module-global place … the region is the whole release". DESIGN §9 L2117: the scan "follows parameters". `behavior_status` is one value per operation, computed from the depth bound and open sites alone (`cpg-core/src/behavior.rs:547-563`). The ADR's fourth conjunct is per place. **Pilot:** `_prior_discover` is written in `Client.__init__` (`client.py:461`) and read in `prior_discover` (`:663`) and `_negotiate` (`:886`); `self.name` is written in `Client.__init__` (`:446`) and read in the base `ClientToolsMixin` (`mixins/tools.py:63`). Q10 lists the classes `fastmcp.FastMCP` and `fastmcp.Client`, whose status is `not_analyzed` by rule (`rules.rs:634`) | **(a)** "`prior_discover` is never read", judged over `__init__`'s Pass B region, which holds no reader, passes the rule and is false. That is the "accepted but unused" error again. **(b)** A per-operation column cannot hold "no dynamic access reaches *this place*". Implementer A ignores the conjunct; B blocks every refutation of the operation. **(c)** For settings, "no call site in the region is open" is unsatisfiable on any real release, so the settings sentence never yields a refutation. If an implementer drops the conjunct instead, the rule has no operation row to join. **(d)** A refutation about a constructor parameter joins a class row that is `not_analyzed` by construction, so it is rejected, or it attaches to `__init__`, which the text does not say | **Premises per place kind**, in one small relation (place key, premise kind, complete or not, reason). Each is a query over existing facts: **parameter or local:** no `reference_resolutions` row resolves to the binding (closures included), and the module has flow IR; **field `f`:** across the release, no attribute load named `f` on **any** receiver (this covers bases and mixins without types), every module has flow IR, and no dynamic access reaches the class (F3); **module global:** reads of the resolved root anywhere in the release (F4), with the same dynamic-access test; **forward chains:** today's `behavior_status`. For fields and globals, replace "no open call site" with the stated assumption "read by release code" (external readers such as pydantic's serializers are outside the model). The rule joins each `refuted_under_model` row to its premise. Say which operation carries a constructor parameter's claims | **None exists.** Rule `semantic:refuted-needs-complete-region` with injected cases: a parameter stored to `self.f` and read only in another method; one read only in a base-class method; a module global read under two spellings. Plus a `behavior_shapes` case for each |
| **F2** | **The lowering from ty's ternary decision diagrams to our two-valued DNF does not say what ty's third value means or which procedure builds the DNF** | DM-15, DM-42, DM-24 · G2, G6 | `reachability_constraints.rs:14-33`: the formula is ternary; `:98` is the AMBIGUOUS terminal. `:139`: `MAX_INTERIOR_NODES`, and at saturation new operations return AMBIGUOUS (`:410, :447, :517`). `builder.rs:2591` `record_ambiguous_reachability` is called at the start of every `try` (`:5032`), for a `for` over a non-literal, non-`range` iterable (`:4779`), at a `with` exit (`:4714`), and at `break`/`continue` inside a context manager (`:5296`). `use_def.rs:2134` records each binding with the current `self.reachability`, so bindings in a `try` body carry the terminal, and the end of the `Try` arm shows no restore of the pre-`try` reachability. ADR §Conditions maps four predicate kinds, not the terminal | **Three readings.** A (AMBIGUOUS → `true`): `try` and loop bodies are unconditional, and a saturated scope silently turns `conditional` into `established` (DM-42). B (AMBIGUOUS → opaque): most behaviors are `conditional` on an "opaque" atom with no source text, although the ADR defines opaque text as "the test's source". C (AMBIGUOUS → `unknown`): most behaviors are unknown. Q10.d's read inside `try:` (`client.py:870-887`) gets three different verdicts. **The procedure too:** enumerating TDD paths gives `a \| (!a & b)`, and expanding the test expression gives `a \| b`. Under absorption alone these are two ids for one condition | **State the lowering.** Atoms are two-valued in our view, so only `if_true` and `if_false` are followed. The AMBIGUOUS terminal reads as `true`, because verdicts state **may**-behavior. Say so; this also settles the earlier F6(d) may/must thread. A scope whose builder saturated, or whose DNF exceeds 16 × 8, is `unknown` (`budget_reached`). **Fix one procedure:** enumerate the paths to `true`, map atoms, evaluate the runtime view, expand `and`/`or`/`not` in tests, then simplify by absorption and by dropping `!x` from a conjunction when `x` alone is a disjunct | **None exists.** The deferred `flow_shapes` known answers (B11), with the expected conditions: `try`/`except`/`finally`, `with` suppression, `for x in items`, `for x in []`, `match`, `elif` chains and `a or b`. Plus a unit test of the procedure's canonical form |
| **F3** | **The dynamic-access reach model, read literally, misses the pilot's own motivating case, leaves typed receivers other than `self` unaddressed, and gives the receiver-less kinds no reach** | DM-08, DM-04, DM-59 · G2 | ADR §Verdicts: "it reaches the fields of class `C` when its receiver is `self` in a method of `C` or of a subclass, or a module global bound to an instance of `C`". `Settings.get_setting` does `settings = self` … `return getattr(settings, attr)` (`settings.py:51,57`); `set_setting` does `setattr(settings, attr, value)` (`:64,70`). The getattr family is qualified "with a non-literal name". `importlib.import_module` is not, although the pilot's calls use literals (`fastmcp/__init__.py:87,92`). `fastmcp/__init__.py:49` has a module-level `__getattr__`, in the module that binds `settings` (`:20`) | **Receivers.** Implementer A matches `self` syntactically, so `getattr(settings, attr)` has an untyped receiver and is assumed not to reach `Settings`. "`server_dependencies` is never read" then becomes `refuted_under_model`: Q04.g, the pre-registered misleading claim, fails. Implementer B follows the alias and says `unknown`. **Receiver-less kinds.** One implementer lets `fastmcp/__init__.py`'s `__getattr__` make every negative claim about that module's globals `unknown`; another lets it reach nothing | **Receivers:** a receiver whose reaching definitions (flow IR, through local copies) include `self`, a module global bound to an instance of `C`, or a value Pyrefly types as `C`, a subclass, or a base or mixin of `C`. Any other receiver is the named assumption. **Receiver-less kinds:** `import_module` and `__import__` reach module objects only, and only with a non-literal name (like `getattr`); a module `__getattr__` supplies names the module does not bind, so it reaches no bound place; `exec` and `eval` reach every place (the pilot has none) | **None exists.** The planned `behavior_shapes` case "a setting read by string" must use the **aliasing** shape (`s = self; getattr(s, name)`), not `getattr(self, name)`. Add a module `__getattr__` case asserting that it blocks no refutation of a bound global |
| **F4** | **Place identity: "one grammar" is asserted, but the spelled grammar gives one release-wide place one key per spelling, and §9.9 keeps a second, resolved grammar** | DM-02, DM-15, DM-11 · G1, G2 | ADR §Conditions: "Places are written as spelled in the record's scope … One grammar serves conditions, places in `value_flows`, and §9.9's access paths." DESIGN §9.9 L2806–2807, unchanged: `Parameter[name]`, `Parameter[self].Field[f]`, `Global[<module>.<name>]`. **Pilot:** one singleton is spelled `settings.log_enabled` (`fastmcp/__init__.py:21`) and `fastmcp.settings.mcp_camelcase_compat` (`_compat.py:123`) | Q04's "which settings are never read" joins reads across modules. Grouped by spelled text, `settings.X` and `fastmcp.settings.X` are two places, so a setting read under one spelling looks unread under the other. §9.9's `Global[fastmcp.settings]` is a third key for the same thing (G1). A place at root + 3 segments (`fastmcp.settings.log.level`) is out of grammar under one spelling and in grammar under another | **Two layers.** The spelled form stays the condition's text, with syntactic equality as declared. The **join key** for `value_flows`, field reads, premises (F1) and summaries is the **resolved** place: the root resolved through `reference_resolutions` or the flow IR's reaching definitions to a binding or a module global's qualified name, then at most two segments. State that §9.9's `Global[m.n]` and `Field[f]` are that key's written form | **Test:** a fixture that reads one module global as `import pkg; pkg.cfg.x` and as `from pkg import cfg; cfg.x` yields one place key. The §9.9 sentence is prose |
| **F5** | **The parity rule guards the harmless direction. The direction that protects negative answers, and the revisit trigger, have no oracle** | DM-53, DM-60, DM-43 · G3 | ADR §The flow provider: "A `semantic:` rule rejects a flow use that matches no reference"; DESIGN §3.9: "Every flow use joins a `references` row". The spike checked both directions and reaching ⊆ candidates (B11), but only ty → ours is a rule. Revisit trigger: "a ty upgrade breaks range parity or decides a test at index time that differs from runtime" | A ty bump that stops indexing a use (in a `match` guard, or a new syntax form) leaves a `references` row with no flow use. The flow IR then misses a read, and "never read" is refuted while the rule passes. A bump that starts folding `sys.version_info` at index time marks a branch unreachable before our runtime view sees it, and nothing fails. The revisit trigger cannot fire | **Two-way parity:** every reference has a flow use, and every binding a flow definition, after the three declared normalizations, with the residue declared and counted (empty on the pilot, per B11). **A standing rule** `semantic:flow-reaching-within-candidates`: the spike's exit test, kept. **A `flow_shapes` case per runtime-view form** (`TYPE_CHECKING` as a name and as an attribute, `sys.version_info`, `sys.platform`, `os.name`), asserting that ty leaves a predicate there and that else-branch bindings are reachable | Rules with injected cases, plus the `flow_shapes` test |
| **F6** | **The spine and the ADRs disagree with the new decisions in seven places** | DM-02, DM-59 · G1 | (a) DESIGN §B1 L261 "No second parser or type checker"; the ADDENDUM §1 §B1 row; AGENTS.md "both linked in-process over one parse". (b) §B5 L336–337: the provider "is decided by the Stage 2.1 spike … either … or". (c) ADR-0022 L56–57 "**Normal form:** sorted conjuncts of normalized atoms" against §Conditions "Normal form: DNF"; L48–50 "recorded here by amendment". (d) ADR-0012's amendment: ty parses "from the same text Pyrefly read", which the rename makes untrue. (e) DESIGN §3.9 L1110–1111: a computed name is `unsupported_unpacking` or `unresolved_target`, against the new `dynamic_access`, so the earlier F6(c) consequence persists. (f) "Three vocabularies (ADDENDUM §3)": ADDENDUM §3 has no `verdict` row, which the earlier F6(d) asked for. (g) ADR-0022 Consequences L235 "Revisit: at the spike's outcome", against the frontmatter | A session applying §B1 (which ADDENDUM routes to G1 and G7) treats `cpg-flow` as a violation, or holds flow facts to "one parse". A reader of ADR-0022's Decision bullets builds conditions from conjunctions only. `getattr(settings, name)` is emitted as `unresolved_target` under §3.9's places paragraph, indistinguishable from an unresolved call | Sentences. §B1 gains "except `cpg-flow`'s flow facts (ADR-0012 amendment)", mirrored in the ADDENDUM row and AGENTS.md. §B5 names ty. ADR-0022's Decision bullets are revised in place (the ADR is `proposed`). ADR-0012's "same text" becomes "the same text with `TYPE_CHECKING` renamed". §3.9's computed-name clause points to `dynamic_access`. A `verdict` row goes into ADDENDUM §3 | Prose; no mechanical oracle. `just adr lint` does not compare DESIGN with the ADRs |
| **F7** | **The runtime override's contract and ty's one runtime-divergent assumption are unstated** | DM-41, DM-04, DM-42 · G2 | **The rename.** Both spellings are 13 bytes, so ranges hold. `is_if_type_checking` matches a `Name` id or `Attribute.attr` over a dotted name (`builder.rs:6207-6215`), and renaming the attribute token defeats both. Unstated: the lexer; a sentinel already in the source; whether the evaluator, the place text and the opaque text read ty's AST (the sentinel) or our bytes; `import TYPE_CHECKING as TC` (neither view decides it). **ty's exception model.** A `try` suite with no checkpoint has unreachable handlers (`builder.rs:5066-5078`). A checkpoint is omitted for "known safe" expressions: literals, lambdas, bound name loads, `is` comparisons (`:2490-2588`). Ambient exceptions (`KeyboardInterrupt`, `MemoryError`) are ignored. The ADR's stated model says "exceptional exits" without this assumption | A rename by text pattern changes `"TYPE_CHECKING"` string literals, so `equals(x, "TYPE_CHECKIN_")`. Place text cut from ty's AST publishes the sentinel. An evaluator that matches only the original spelling leaves 75 pilot blocks `conditional` on `truthy(TYPE_CHECKIN_)` instead of unreachable. **Exceptions:** a `try: <safe code> except KeyboardInterrupt: cleanup()` handler yields no behavior, although it runs at runtime (rare on the pilot) | **One paragraph:** tokens from Pyrefly's parse; refuse or re-pick if the sentinel occurs; the evaluator matches the sentinel in both forms; place text and opaque text are cut from the original bytes by range; an `as` alias of `TYPE_CHECKING` stays a `truthy` atom, as declared. **One bullet in the stated model:** exceptions come only from operations ty's model says can raise, and ambient exceptions are outside it | **Test:** the renamed text equals the original outside the renamed ranges. A fixture with `TYPE_CHECKING` in a string, a comment, an f-string field, `typing.TYPE_CHECKING`, and a module-local `TYPE_CHECKING = False`. A `flow_shapes` case for a handler around safe code |
| **F8** | **Q04 grades "when is each read (at import, as a snapshot at construction, or on each call)", and no Stage 2 decision defines a read phase** | DM-59, DM-43 · G7 | Q04's text and Q04.b–e. DESIGN §3.2 lists `ambient_reads` for Stage 2 with no fields. ADR-0022 and §3.9 do not define a phase. Plan §7 makes Q04 Stage 2's exit | Implementers classify differently. Is a read in `__init__` that is stored to a field a snapshot? Is a read in a function called from module scope (`_configure_logging`, Q04.b) at import time? One implementer's `per_call` is another's `construction`, and the exit is graded on it | Define the phase by the reading site's scope. A module or class body is `import`. `__init__` and `__post_init__` are `construction`, and a snapshot when the value is stored to a field. Anything else is `per_call`. A read reached from module scope through calls is Stage 3's (summaries), so state that Q04.b is `partial` in Stage 2 | **Test:** a `behavior_shapes` setting read once in each phase |
| **F9** | **Composed layers are decided for a path bound only under `TYPE_CHECKING`, not for a name defined in both branches (the case F11 posed), and `unreachable_in_context` takes a second meaning** | DM-06, DM-08 · G2 | ADR §Composed layers: "A public path bound only under `TYPE_CHECKING` … is `unknown` with `unreachable_in_context`". `HEAD:codebook.rs:157-159`: `unreachable_in_context` is "A definition **the analyzer's context** never binds". The earlier F11: "a library with a `def` in both branches". The pilot has 2 `else`-bindings of 75, both annotation-only (Stage 0 review, Measured) | With a `def` in both branches, the exports seed is the `TYPE_CHECKING` facade (checker view). The runtime view finds its sites unreachable, so it yields no behavior. The scan meets no boundary, so the operation is `established` with no behaviors, which reads as "does nothing". One reason code then means "the checker never binds this" on one row and "the runtime never binds this" on another | One sentence: an operation whose seed declaration the runtime cannot reach is `unknown`, whether or not a runtime definition exists. Give it its own reason, an append such as `runtime_unreachable`, rather than overloading `unreachable_in_context` | The planned `flow_shapes` `TYPE_CHECKING` case, extended with a public `def` in both branches (test) |
| **F10** | **Encoding corners give two ids for one meaning, or leave the choice open** | DM-15 · G2 | ADR §Conditions: literal kinds, `member_of` sorted, strings as JSON strings. Unstated: whether a singleton `member_of(x, {"a"})` is `equals(x, "a")`; `x == None` against `is_none(x)`; `x is True` (not an atom kind); operand order (`None is x`, `"sse" == x`); `-1` (a unary op in ruff's AST); JSON escaping (`"é"` against `"é"`); the sort order (bytewise on the encoding?); whether opaque text keeps comments inside a parenthesized test | Two implementers give `x in ("a",)` and `x == "a"` different ids, or write one non-ASCII string two ways. The known-answer corpus then fixes whichever one ran first | A short list in §Conditions: singleton `member_of` becomes `equals`; `== None` becomes `is_none`; `is True`/`is False` become `equals` with `True`/`False`; the literal is always the second operand; a negative integer literal is an integer; strings are serialized by one named serializer (serde_json, ASCII-unescaped); sort bytewise; opaque text is taken from the test's range with comments removed | The planned Rust conditions unit test, with these cases |
| **F11** | **The compatibility evaluator and its JSON corpus are built ahead of their consumer** | DM-58, DM-57 · — | ADR §Conditions: "Compatibility … decided by a finite-domain evaluator". DESIGN §3.9: it is "held to `specs/serving/conditions.json`". The plan's Stage 2.4 builds it. Neither Q04 nor Q10 asks whether two conditions can hold together. Q9 (transport option conflicts) is Stage 3 (plan §7). The JSON corpus existed so that two implementations could share it; the Python twin is gone (the earlier F12). Its rules (is `1` compatible with `True`? are two `isinstance` atoms disjoint?) are unstated | Stage 2 builds and maintains a three-valued evaluator with invented known answers that no Stage 2 question checks, plus a serving-directory spec with one reader. When Q9 arrives, its actual needs (two parameters' mode sets) may reshape it | Defer the evaluator to Stage 3, with Q9. Stage 2 needs only the encoder and the runtime view. Keep its known answers as a Rust test until a second reader exists; drop `specs/serving/conditions.json` | None needed: this removes machinery. If it is kept: the compatibility rules above as unit tests |

**Observations** (not findings):
- **O1.** The working tree's `Cargo.toml` pins `ty_python_core`, `ty_module_resolver`, `ty_vendored`,
  `ruff_db` and salsa. `docs/pins.md` has no row for any of them, and `check_family.py`'s
  `unpinned()` requires one, so `just deps` fails until the rows exist. The declared-family
  exception is not in the committed `check_family.py` either. It is in flight, and should land with
  the crate.
- **O2.** `ruff_db-0.0.14/src/lib.rs:100-101` reads `TY_MAX_PARALLELISM` and `RAYON_NUM_THREADS`.
  These are parallelism knobs, and output-neutral if the index is built per file. Say so, or
  classify them in `just deps` as the Pyrefly fork's reads are.
- **O3.** State ty's panic policy: abort the attempt, as ADR-0012 §Panics does, or record the
  module's flow coverage as `failed`. Widen `rules/no-catch-unwind-in-extractor.yml`'s `files` to
  `crates/cpg-flow/src/**` if the policy is abort.
- **O4.** The spike's numbers are honestly labelled *Measured*, with conditions (FastMCP 4.0.5, 275
  modules, 2026-09-24).
  - Their harness is not in the repository (B11).
  - The `flow_shapes` exit test was not run, so the constructs where ty's representation matters for
    conditions were not exercised: `try`, `with`, `for`, `match`.
  - D-2 is well supported for parity and cost. For conditions, ty's fitness is Proposed until
    `flow_shapes` runs (F2).
- **O5.** "The runtime view's version" is a second hand-bumped counter beside
  `EXTRACTOR_OUTPUT_VERSION` (`cpg-extract/src/config.rs:29-31`), and the two mean the same thing
  (DM-56). Fold it into the existing one. Pin `flow_shapes`' flow facts in an insta snapshot, as the
  variant and id snapshots do today, so an unbumped change shows. `cpg-flow`'s own code (the
  normalizations, the rename, the lowering) is otherwise covered by no digest.
- **O6.** The parity residue's reason, "Python 3.14 evaluates [annotations] lazily", holds for the
  pilot (3.14.7). It does not hold for a library pinned below 3.14 without
  `from __future__ import annotations`. The durable reason is that annotations are not modelled as
  reads: §3.2's `references` excludes them.

**Applicability.**
- **Groups that bore:**
  - 2 (absence, semantic types): F1, F2, F3, F9, F10;
  - 3 (identity and equivalence): F4, F10;
  - 9 (providers and boundaries): F2, F5, F7;
  - 11 (verification): F5;
  - 12 (proportionality, falsifiable claims): F8, F11, O4;
  - 1 (authority): F4, F6.
- **Principle verdicts:**
  - DM-08 is **satisfied** for budget cuts and opaque conditions, and **unresolved** for the
    AMBIGUOUS terminal and for place premises;
  - DM-15 is **unresolved** (F2, F10);
  - DM-02 is **violated** (documentary: F6, and §9.9 in F4);
  - DM-07 is **unresolved** (F1);
  - DM-31 and DM-32 are **satisfied** for provider identity;
  - DM-04 is **unresolved** (F7's exception assumption);
  - DM-60 is **unresolved** (F5);
  - DM-58, a SHOULD, is **violated** (F11);
  - DM-59 is **satisfied** for labels (O4).
- **Groups that did not bear:**
  - 6 (mutable workspaces): the attempt protocol is unchanged;
  - 8 (performance): 213 ms and 47 MiB are measured, and no target is claimed;
  - 4 (declarative composition): no new declaration surface;
  - 7 beyond DM-31 and DM-32: no new caches;
  - 5 (derivation): it bore only through the lowering (F2).

---

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| **Baseline:** Stage 1 (Pass B recognizers, no flow IR, no negative fates) | One relation per meaning | Q04 and Q10 cannot be answered: no field reads, no conditions over `self` | Built | Pilot (the Stage 1 packet) | Rejected by ADR-0022's Options; this review agrees |
| **Proposed:** ty plus the rename, DNF, one region per operation, the compatibility evaluator in Stage 2 | One place grammar is claimed, but there are two (F4) | False refutations for field places (F1). The AMBIGUOUS terminal is lowered three ways (F2). Q04.g is exposed (F3) | `cpg-flow`, the encoder, the evaluator and its corpus | 213 ms, 47 MiB, *Measured* by the spike | The provider choice stands. The premise design needs revision |
| **Simpler viable alternative:** the same provider, rename and encoder, plus **premises per place kind** in one relation (F1). Field premises are release-wide and **name-based** (no attribute load named `f` on any receiver). Receivers are resolved through reaching definitions (F3). AMBIGUOUS reads as `true`, with verdicts declared as may-behavior (F2). No compatibility evaluator until Q9 (F11) | One premise relation replaces overloading `behavior_status`. There is one resolved place key (F4) | Sound under the stated model without types or alias analysis for fields, and it covers mixins. **Precision cost:** a common field name (`name`) blocks refutation, and "only as a log prefix" claims (Q10.f) stay `partial`. The trigger for typed narrowing is a Stage 2 item graded `partial` for a name collision | Less: no evaluator; four SQL premises over existing facts | Same provider | **Recommended.** It is decidable with facts that exist, closes Q04.g and journey b, and removes Stage 2 machinery that has no consumer |

**Abstractions justified by current needs:**
- the `condition_atom` codebook and the encoder, whose consumers are the ids and `get_operation`;
- the parity and reaching-within-candidates rules, which guard a moving external provider;
- one premise relation, whose consumer is the refutation rule and Q04's and Q10's negative items.

**What remains ordinary code:**
- the TDD → DNF lowering;
- the rename;
- the normalizations;
- the runtime-view evaluator.

None of these needs a declaration surface.

---

## 9. Verification and measurement plan

| Claim or risk | Evidence label | Check | Expected | Gap |
|---|---|---|---|---|
| Refutations only under a complete premise per place | Proposed | `semantic:refuted-needs-complete-region` with the field, base-class and two-spelling cases (F1) | Rejects each injected row | Rule and cases |
| One lowering for each ty diagram | Proposed | `flow_shapes` expected conditions for `try`, `with`, `for` and `match` (F2) | The declared DNF | Test (the deferred exit test) |
| String-keyed settings reads block refutation | Proposed | The aliasing `get_setting` case in `behavior_shapes` (F3) | `unknown` (`dynamic_access`) | Test |
| One place per global across spellings | Proposed | The two-import fixture (F4) | One key | Test |
| Parity in both directions; reaching ⊆ candidates; ty folds no runtime-view test | Tested for parity and reaching (spike, outside the repository); Proposed as rules | Two rules plus `flow_shapes` runtime-view cases (F5) | Reject injected drops; ty leaves predicates | Rules and test |
| The rename moves no byte and touches only identifier tokens | Proposed | A byte-diff test plus the string, comment and f-string fixture (F7) | Only the sentinel ranges differ | Test |
| Read phase | Proposed | A `behavior_shapes` setting read in each phase (F8) | One phase each | Test |
| A both-branch `def` | Proposed | Extended `flow_shapes` case (F9) | `unknown` with its own reason | Test |
| One id per condition meaning | Proposed | Rust unit test with F10's cases | One id | Test |
| Flow extraction cost | Measured (spike, 2026-09-24) | `just pilot` stage timings once `cpg-flow` lands | Reported; no target | Per stage |

---

## 10. Exceptions and deferred items

No SHOULD-level exception is claimed. The MUST-level gaps (F1, F2, F3, F4 and F5) are recorded as
unresolved, not excepted.

### Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| The earlier F5's "how §9.2's supported predicates map to conditions" | It belongs with seed findings becoming renderings (the earlier F7) | Stage 2.6 |
| The gold rule for concepts, and membership's `basis` (the earlier F9) | Stage 4's registry | Stage 4's plan |
| Typed receivers to narrow name-based field premises | The name-based floor is sound. Precision waits for a need | A Stage 2 packet item graded `partial` because of a field-name collision |
| Condition substitution through locals and fields (Q10.d) | Summary territory | Stage 3.2 |

---

## 11. Decision and implementation changes

**Decision: Revise.** ADR-0022 stays `proposed`.

**What stands:**
- **D-2: ty behind a range-parity rule and a same-length rename.** The source bears out the
  premise: `TYPE_CHECKING` is the only test `resolve_to_literal` decides against runtime. The rename
  defeats both its forms.
- **Conditions:** DNF with polarized literals, the atom set, the literal kinds, the 16 × 8 budget,
  and opaque atoms that stay stated.
- **Verdicts:** `budget_reached` is `unknown`; `not_analyzed` means out of scope or not requested.
- **The mapping to `modality` and `evidence_status`.**
- **Identity:** provider identity in `producer_id`, and concept ids from their keys.
- **Composed layers:** the CPG keeps the checker view, and a behavior takes the runtime view at
  its site.
- **The ADR-0012 amendment's confinement** (nothing of ty or ruff 0.0.14 crosses `cpg-flow`'s API),
  once F6(d) is corrected.

**What blocks acceptance:**
- **G1 fails** (documentary): F6, and §9.9 in F4.
- **G2, G3 and G7 are unresolved on Stage 2's own exit.** Negative claims about fields and module
  globals have no sound premise (F1). The dynamic-access model misses Q04.g's shape (F3). Place
  identity does not join across spellings (F4). The ternary terminal has no lowering (F2).

**What can proceed now:** the `cpg-flow` crate, the rename (with F7's test), the encoder and parity.
The refutation verdict (plan 2.5) and the negative answers to Q04 and Q10 wait for F1, F3 and F4.

**The smallest route to acceptance:**
1. Write the F1–F4 sentences, F6's corrections and F7's paragraph into ADR-0022 §Conditions and
   §Verdicts and into DESIGN §3.9, §9.9, §B1 and §B5.
2. Add F5's two rules and the `flow_shapes` cases. Run `flow_shapes` (O4).
3. Re-review at `compact` depth, then accept.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| **P1** | F1: premises per place kind in one relation; the rule joins premises; the operation for a constructor parameter | DM-08, DM-07, DM-59 | ADR §Verdicts and DESIGN §3.9 text | The refutation rule with field, base-class and two-spelling cases |
| **P1** | F3: receivers through reaching definitions and Pyrefly types; the reach of receiver-less kinds | DM-08, DM-04 | ADR §Verdicts text | `behavior_shapes` aliasing case; module `__getattr__` case |
| **P1** | F2: the AMBIGUOUS terminal read as `true` (may-behavior); saturation `unknown`; one procedure | DM-15, DM-42 | ADR §Conditions text | `flow_shapes` expected conditions; procedure unit test |
| **P1** | F4: a resolved place key; §9.9 as its written form | DM-02, DM-15 | ADR, §3.9 and §9.9 text | Two-spelling fixture |
| **P1** | F6: §B1, §B5, ADR-0022's Decision bullets, ADR-0012 "same text", §3.9 computed names, the ADDENDUM §3 row, the Consequences revisit line | DM-02, DM-59 | Text | Prose |
| **P2** | F5: two-way parity; reaching ⊆ candidates; runtime-view folding cases | DM-53, DM-60 | Rules in `rules.rs` | Injected cases; `flow_shapes` |
| **P2** | F7: the rename's contract; ty's exception assumption in the stated model | DM-41, DM-04 | ADR text | Byte-diff test; fixture |
| **P2** | F8: read phase | DM-59 | §3.2 and §3.9 text | `behavior_shapes` phases |
| **P2** | F9: a both-branch `def` is `unknown` with its own reason | DM-06, DM-08 | ADR §Composed layers text; codebook append | `flow_shapes` case |
| **P3** | F10: encoding corners | DM-15 | ADR §Conditions list | Unit test |
| **P3** | F11: defer the compatibility evaluator to Stage 3; drop the serving-path corpus | DM-58 | Plan 2.4 and §3.9 text | None needed |
| **P3** | O1–O6 | — | `pins.md` rows; the catch-unwind glob; one counter | `just deps`; `just rules-test` |

**Final check.**
- **Claims against evidence.** They match: every new clause is Proposed, and the spike's numbers
  carry their conditions (O4).
- **Scope against guarantees.** They do not match yet. Stage 2's exit claims negative answers about
  settings and constructor parameters, and the premise that would license them is defined for
  parameter forwards only (F1).
- **Extension paths.** They are clear for atoms and boundary reasons, which are codebook appends.
  They are not yet clear for a ty upgrade, whose regressions no oracle catches (F5).

---

## 12. Disposition (author, 2026-09-24)

The review's recommended alternative (§8) is adopted: the same provider, rename and encoder,
**premises per place kind**, receivers resolved through reaching definitions, the ambiguous
terminal read as `true` with verdicts declared as may-behavior, a resolved place key, and **no
compatibility evaluator until Q9**. ADR-0022's Decision was rewritten in place (it is `proposed`).
It is accepted once Stage 2's implementation meets the oracles below, because several decisions are
now small enough to be tested rather than argued.

| # | Disposition | Where |
|---|---|---|
| F1 | **Adopted.** `negative_premises` per place kind: a parameter or local, a field (release-wide and name-based), a module global, and a forward chain (`behavior_status`). External readers are outside the model. A constructor parameter's claims attach to `__init__`. The injected cases (field read only in another method, only in a base, a global read under two spellings) land with the relation in Stage 2.6 | ADR-0022 §Verdicts; DESIGN §3.9 |
| F2 | **Fixed.** One lowering procedure: only `if_true`/`if_false` are followed; ambiguous reads as `true` (may-behavior); runtime view first; then contradiction, `!x`-beside-`x` and absorption. The `flow_shapes` known answers now exist: `try`/`except`/`finally`, `with` suppression, `for` over an unknown iterable and over `[]`, `match`, an `elif` chain, `a or b`, and a pinned snapshot of every use and region (`cpg-flow` tests, 21/21). ty's saturation is not observable through its API; the pilot's largest scope will be reported | ADR-0022 §Conditions; `crates/cpg-flow/tests/flow_shapes.rs`; `cpg_schema::condition` |
| F3 | **Adopted.** Receivers through reaching definitions and local copies (`self`, a global bound to an instance, a Pyrefly type, relatives included); `import_module`/`__import__` reach modules only; a module `__getattr__` reaches no bound place; `exec`/`eval` reach every place. The `behavior_shapes` case uses the aliasing shape | ADR-0022 §Verdicts; Stage 2.7 |
| F4 | **Adopted.** Two layers: spelled (condition text) and resolved (the join key, whose written form is §9.9's). The two-import fixture test lands with `value_flows` | ADR-0022 §Places; DESIGN §3.9, §9.9 |
| F5 | **Adopted.** Two-way parity and `semantic:flow-reaching-within-candidates` are rules of the `flow` family (Stage 2.3); the runtime-view forms have `flow_shapes` cases (name, attribute, `sys.version_info`, `os.name`, a `def` in both branches, the word in a string, a comment and an f-string) | ADR-0022 §The flow provider; `flow_shapes` |
| F6 | **Fixed.** §B1 names the one exception, mirrored in ADDENDUM §1 and AGENTS.md; §B5 names ty; ADR-0022's Decision bullets rewritten; ADR-0012's amendment says the text is renamed; §3.9 routes computed names to `dynamic_access`; ADDENDUM §3 has a `verdict` row; the Consequences' revisit line agrees with the frontmatter | DESIGN, ADDENDUM, AGENTS.md, ADR-0012, ADR-0022 |
| F7 | **Fixed.** The rename works on name tokens from ruff 0.0.14's lexer (strings and comments untouched; f-string fields renamed); a module already naming the sentinel is refused; place, literal-derived and opaque text are cut from the module as written; ty's exception model is a stated assumption. Tests: `the_rename_touches_names_only_and_keeps_every_byte_offset`, `a_handler_around_safe_code_is_unreachable_in_the_stated_model`, `a_string_holding_the_word_keeps_it` | `cpg-flow`; ADR-0022 |
| F8 | **Adopted.** Read phase by the reading site's scope (`import`, `construction` with `snapshot`, `per_call`); a read reached from module scope through calls is Stage 3's, so Q04.b is expected `partial` | ADR-0022 §Verdicts; DESIGN §3.9 |
| F9 | **Fixed.** `runtime_unreachable` appended to `boundary_reason` (code 18); `unreachable_in_context` keeps the checker's meaning. `flow_shapes` has a `def` in both branches | codebook; ADR-0022 §Composed layers |
| F10 | **Fixed.** Canonical forms stated and implemented (`== None` is `is_none`, `is True` is `equals`, literal second, singleton set is `equals`, negative integers, serde_json strings, bytewise sort, comments stripped from opaque text) | ADR-0022 §Conditions; `cpg_schema::condition`, `crates/cpg-schema/tests/conditions.rs` |
| F11 | **Adopted.** The evaluator is removed and `specs/serving/conditions.json` deleted; the normal-form known answers are a Rust test. Stage 3 reintroduces compatibility with Q9 | `cpg_schema::condition` |
| O1 | **Fixed.** `docs/pins.md` row; `check_family.py` declares the extra family with its scope (dependents `cpg-flow` only) and exact salsa pins; `just deps` passed | pins; `scripts/check_family.py` |
| O2 | **Noted.** `ruff_db` reads `TY_MAX_PARALLELISM` and `RAYON_NUM_THREADS`; the index is built per file on the driver thread, so they are output-neutral. Stated in the pins row when the provider is wired into the extractor | Stage 2.3 |
| O3 | **Fixed.** Panics abort; `no-catch-unwind-in-extractor` covers `crates/cpg-flow/src` | rule |
| O5 | **Fixed.** No second counter: `cpg-flow` output changes bump `EXTRACTOR_OUTPUT_VERSION`; `flow_shapes`' facts are pinned by an insta snapshot | ADR-0022 §Identity; `the_flow_facts_are_pinned` |
| O6 | **Fixed.** The residue's reason is that `references` does not model annotations as reads | ADR-0022; DESIGN §3.9 |
