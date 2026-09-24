# Design review: the end of Stage 2 of the behavioral-model plan (compact)

**Date:** 2026-09-24 · **Depth:** compact (ADR-0001: the end of a slice that adds a fact family,
analysis tables and behaviors) · **Mode:** code plus document.

**Target** (HEAD `a7e1e2c`, tree clean):
- ADR-0022 (`status: proposed`); DESIGN §3.2 (`behavior` and `flow` rows, L626–627), §3.9, §11.3.
- `crates/cpg-flow/src/{lib,predicate}.rs`, `crates/cpg-extract/src/flow.rs`,
  `crates/cpg-schema/src/{condition,behavior}.rs`, the flow rules in `crates/cpg-schema/src/rules.rs`,
  `crates/cpg-core/src/flow_model.rs`, the Stage 2 parts of `crates/cpg-core/src/behavior.rs`.
- Tests: `crates/cpg-flow/tests/flow_shapes.rs`, `crates/cpg-schema/tests/conditions.rs`,
  `crates/cpg-core/tests/behavior.rs`.
- Commits `201c3e4`, `1b5b5bc`, `a7e1e2c`; deviations B15–B20; `structured_eval_stage2_2026-09-24.md`.

**Reviewer:** `design-reviewer` subagent, fresh context. **Author:** the session executing the plan.

---

## Answers to the brief

### 1. ADR-0022's acceptance: the Stage 2 standard review's oracles

The Stage 2 standard review's §12 made acceptance conditional on these. "Met" means the oracle
exists, ran in this session (`just test-all`, passed) and checks what the finding asked for.

| # | Oracle asked for | Met? | Evidence (read in this session) |
|---|---|---|---|
| F1 | Premises per place kind in one relation; the rule joins them; injected cases: a field read only in another method, one read only in a base, a global read under two spellings | **Partly** | The relation: `flow_model.rs:1637-1788`. The rule: `rules.rs:761-768`. Its injected case (`tests/analysis.rs:1483-1497`) covers only a missing premise, not `holds = false`. `behavior_shapes` covers a field no load names (`Plain.never_read`), the aliasing reader, and a parameter never read (`tests/behavior.rs:271-312`). **Not there:** a field read only in a base class, and one field read under two spellings (`:233-236` asserts one spelling). **New:** the parameter premise refutes stub, abstract and overridden methods (**R1**). The field premise misses loads on call and subscript receivers (**R7**) |
| F2 | One lowering (AMBIGUOUS is `true`); `flow_shapes` answers for `try`, `with`, `for`, `match`, `elif`, `a or b`; a unit test of the procedure | **Met** | `predicate.rs:451-480` (AMBIGUOUS is `true` at `:467-469`); `flow_shapes.rs:187-203, 328-381, 445-454, 486-494`; `conditions.rs:6-67`. The "largest scope reported per run" (ADR L192) is not reported (**R10**). The normal form is not unique (**R9**) |
| F3 | Receivers through reaching definitions and local copies, a Pyrefly type, relatives; `import_module`/`__import__` reach modules; module `__getattr__` reaches nothing; `exec`/`eval` everything; the aliasing case | **Partly** | Local copies of `self` (`flow_model.rs:1566-1594`), a singleton global (`:1595-1605`), relatives (`:1647-1657`), modules (`:1616`) and `exec`/`eval` (`:1617, 1643`) are implemented. The aliasing case passes (`config.py` `get_setting`; `tests/behavior.rs:305-310`). **Not implemented:** Pyrefly-typed receivers (ADR L266) and `__dict__` loads (ADR L262; DESIGN L1220). The loop at `:1531-1553` visits calls only (**R10**). There is no module `__getattr__` case |
| F4 | A resolved place key; a two-import fixture | **Met in substance** | `resolve`/`resolve_dotted` (`flow_model.rs:782-801, 1221-1232`). On the pilot, 46 settings reads join across `settings.X` and `fastmcp.settings.X` (the evaluation, *Measured*). The fixture checks one spelling per field. B19's rule is not in the ADR (**R11**) |
| F5 | Two-way parity; reaching within candidates; runtime-view cases | **Met** | `rules.rs:207-266`; injected cases `tests/compile.rs:504-560`; `flow_shapes.rs:389-418` |
| F6 | Spine sentences (§B1, §B5, ADDENDUM §1/§3, AGENTS.md, §3.9's computed names) | **Met** | DESIGN L1125-1128; ADDENDUM §1 §B1 row and the §3 verdict row; AGENTS.md. New lag: **R11** |
| F7 | The rename's contract, with tests | **Met** | `lib.rs:181-208`; `flow_shapes.rs:427-444, 455-481` |
| F8 | Read phase by scope; one `behavior_shapes` read per phase | **Partly** | `flow_model.rs:1393-1416`. `import` and `per_call` are tested (`tests/behavior.rs:230-243`). `construction` and `snapshot` have no fixture case, and are Measured only through Q21.a. A read inside a lambda or comprehension gets no reader and so the phase `import` (O1) |
| F9 | `runtime_unreachable` for an operation whose declaration the runtime cannot reach; a `flow_shapes` `def` in both branches | **Not met** | The codebook entry exists, and `flow_shapes.rs:464-472` checks the flow facts. **No code emits `runtime_unreachable`** (grep over `crates/`). On the pilot, `MCPServerConfig.__init__`, declared only under `if TYPE_CHECKING:`, is served `established` and its `data` is `refuted_under_model` (**R2**) |
| F10 | Canonical forms, with a unit test | **Met** | `predicate.rs:230-251`; `conditions.rs:6-67` |
| F11 | Defer the compatibility evaluator; drop the serving corpus | **Met** | No compatibility code; `specs/serving/` has no `conditions.json` |
| O1 | Pins rows; family check | **Met** | `docs/pins.md` L72; `just deps` passed |
| O2 | ty's environment reads stated | **Met** | `docs/pins.md` L72 |
| O3 | `no-catch-unwind-in-extractor` covers `cpg-flow` | **Met** | `rules/no-catch-unwind-in-extractor.yml:20`; `just rules-test` passed |
| O4 | `flow_shapes` run | **Met** | 23 known answers, in `just test-all` |
| O5 | One counter; `flow_shapes` facts pinned | **Met** | `EXTRACTOR_OUTPUT_VERSION = 21` (`cpg-extract/src/config.rs:31`); `the_flow_facts_are_pinned` (`flow_shapes.rs:495`) |
| O6 | The residue's reason | **Met** | ADR L119-120 |

**Recommendation: keep ADR-0022 `proposed`.** Four of the gaps sit in the ADR's own text, so they
block acceptance:
1. **The parameter premise (ADR L248)** admits bodies that the operation's callers never run:
   stubs, `raise NotImplementedError`, methods that a release subclass overrides, and
   declarations the runtime cannot reach. On the pilot, 44 of the 63 served refutations fall in
   one of these classes. 16 are provably misleading: 15 on overridden methods and 1
   `TYPE_CHECKING` stub. The other 28 are default stubs, mostly hooks meant to be overridden, such
   as `MessageHandler.on_message` (**R1**).
2. **Composed layers (L322-324)** are decided but not implemented, and the one pilot case is
   served wrong (**R2**).
3. **Versioned places (L221-231)** claim that a feasible path is no longer dropped. It still is,
   around a loop's back edge and for opaque tests (**R4**).
4. **The normal path (L193-197)** assumes a raising guard leaves the function, and never checks
   it (**R6**).

The OverBudget bug (**R3**) is code-only, but it violates the ADR's budget rule, so it must be fixed
before the ADR is accepted. The amendments are small, and the ADR is `proposed`, so they can be
written in place. **Accept** after the amendments, the fixes for R1–R4 and R6 with their oracles,
and a `compact` re-review.

### 2. The evaluation's fixes (B15), attacked

| Fix | Verdict | Why |
|---|---|---|
| **`through_call` / `call_transfer`** | **Sound in direction, incomplete at the edges** | A use inside a callee, a receiver or an argument is `Call`. A path takes its weakest step (`lib.rs:703-711`, `flow_model.rs:452-466, 620`). Such claims are `unknown` (`behavior.rs:909-913, 1086-1089`), and the `identity_not_through_call` CHECK holds (`schema behavior.rs:325`). Nothing reached only through a call stays `established`. **Gaps:** (a) a decorated `def` records no value (`lib.rs:416`), so `return wrapper` after `@wraps(fn)` yields no claim at all, where it should yield an `unknown`; (b) a `lambda` or generator body is walked as `derived` (`lib.rs:703-711`), although it runs only when called; (c) `sink()` keeps only the strongest transfer per origin (`flow_model.rs:652-661`), so `return x if f else g(x)` states the identity branch and drops the `call_transfer` one. (a)–(c) lose precision, never soundness (O3, O4) |
| **Self-subsuming resolution in `Condition::from_dnf`** | **Sound; terminates; not canonical** | Each step replaces `l ∧ D` by `D` when `¬l ∧ E` with `E ⊆ D` is present, which is equivalence-preserving (`condition.rs:274-303`). A Python port brute-forced 20,000 random DNFs over 4 atoms: 0 counterexamples. Every step removes one literal, so the loop ends within the total literal count. **Not confluent:** the Rust normalizer gives one condition two encodings under two conjunction orders (scratch test `review_probe_conditions`). So ADR L187's "must normalize to one encoding" is false, and `given` can miss a factor (**R9**) |
| **Guard-product factoring (`normal_path`)** | **Exact algebra, two unsound premises** | `given` removes `F` only when every conjunction is `Fᵢ ∪ R` with every `Fᵢ` used, so `self ≡ factor ∧ R` (`condition.rs:381-420`). It can still remove a literal the normal path does not imply: (i) **`OverBudget == OverBudget` returns `true`** (`:382-384`). Over-budget guards enter `guards_of` (`flow_model.rs:715-720`), so a claim whose own condition is over budget becomes `established` (**R3**, 4 pilot claims). (ii) **A raise its own function catches** is still a guard, so its negation is dropped from fates it actually gates, and a `raises_when` is served for an exception that never leaves the function (**R6**). The normal path is function-wide, and includes guards after the site. That is right only under the reading "on executions that raise at no guard", which the ADR should say |
| **Versioned places (`place@line`)** | **Consistent; still unsound around loops and for opaque tests** | Spelling is decided per scope and per plain place (`predicate.rs:69-120`), so regions, raise guards, `given` and value conditions spell one value alike. In forward (loop-free) code, two same-spelled tests on one path read one value: a rebinding between them lies textually after the first test and raises the max. **`try`, `match`:** sound (handlers get the max of the body's bindings, which is conservative; match subjects and guards are surveyed). **Nested scopes:** conditions never compose across scopes (`captured` sources are `true`, `flow_model.rs:562-577`). **Loops:** the flow model composes a loop-carried definition's condition (iteration k) with the use's (k+1) (`flow_model.rs:612-624`). The ADR's own caveat (L229-230) is then violated, and feasible flows are stated under `false` (pilot: `Settings.get_setting` `attr`; probe `accumulate` is served **`conditional` under `false`**). **Opaque atoms are never versioned** (`predicate.rs:62-64`), so `if "." in name: name = d` … `if "." in name: raise` drops the flow `d → sink` entirely (probe `opaque_rebind`) (**R4**). **Consistency with `given` and raise guards:** yes. **With `tests` and `raise_sites.parameters`:** no. They strip `@line` and match by root name (`flow_model.rs:1313, 1461`), so a rebound value is attributed to the parameter (**R8**) |
| **`root_names`** | **A second, weaker reading of what the flow IR already knows** | It lexes the opaque text (`flow_model.rs:671-706`): keyword-argument names count as reads, while f-string fields and triple-quoted strings are mis-lexed. The pilot's `ProxyDCRClient.validate_redirect_uri` lists `validate_redirect_uri( redirect_uri=resolved, …)` as a test of `redirect_uri`. The flow IR already records every use in the test's span, with its reaching definitions (**R8**) |
| **Unmapped argument → release callee** | **Adequate** | At a site with exactly one release callee, a `**`/`*` argument is `derives` into that callee, and the callee's modality sets the verdict (`behavior.rs:764-775, 864-877`). On the pilot: 35 `established`, 43 `conditional`, 9 `override_dispatch`, 39 `call_transfer`. At a site with two or more release callees it falls back to the callee text as `established` (`:878-881`), against one verdict per arc. That happens 0 times on the pilot (O6) |

### 3. Gates: §6. **B16–B20:** B16's precondition belongs in the ADR (R6). B17 and B18 make DESIGN §3.2 and §11.3 stale. B19 is a place-identity rule that belongs in ADR §Places. B20 is serving presentation and needs no ADR (**R11**).

---

## 1. Decision and scope

**Proposal.** Stage 2 as built:
- the `flow` family, from ty through `cpg-flow`;
- the condition language and its lowering;
- the flow model's analysis tables (`value_flows`, `field_accesses`, `ambient_reads`,
  `dynamic_accesses`, `raise_sites`, `singletons`, `negative_premises`);
- the Stage 2 behaviors (`derives`, `stores`, `returns`, `reads_setting`, `is_read`, `tests`);
- the evaluation's six fixes.

**Status.**
- **Tested:** the provider, the lowering and the encoding (`flow_shapes`, `conditions`), and the
  behaviors on `behavior_shapes` (9 tests).
- **Measured:** on the pilot, below.

**Observable outcome.** `get_operation` serves fates with conditions, setting reads with phases, and
negative claims under premises.

**In scope:** plan §6 Stage 2. **Not in scope:** Stage 3's summaries and models; Stage 4.

### Method and coverage

**Read in full:**
- the standard: the skill, charter §A–§H, the ADDENDUM, the template;
- ADR-0022; the Stage 2 standard review (all of it);
- the deviations log; the Stage 2 evaluation;
- plan §6 Stage 2 and §15; DESIGN §3.2's two rows, §3.9 and §11.3;
- the code: `condition.rs`, `predicate.rs`, `cpg-flow/src/lib.rs`, `flow_model.rs`,
  `behavior.rs:150-310, 740-1216`, `rules.rs:150-290, 740-790`;
- the tests: `tests/behavior.rs`, `conditions.rs`; `flow_shapes.rs` by function, with the
  runtime-view, versioning and loop tests read.

**Skimmed only:**
- `cpg-extract/src/flow.rs`, a row writer;
- the bundle and `lctx_mcp` code, read only where a served answer was checked.

**Not reviewed:**
- `cpg-flow/src/db.rs`;
- ty's internals beyond what the Stage 2 standard review read;
- the structured evaluation's ratings, which are the operator's to review;
- Stage 1 Pass B code outside the Stage 2 hooks.

**Probes.** They were run outside the repository:
- **Where:** a `git clone --shared` of `a7e1e2c` in the session scratchpad, with its own
  `CARGO_TARGET_DIR`.
- **Fixture:** `behavior_shapes` plus a module `bpkg/probe.py`, holding `cycle`, `opaque_rebind`,
  `typed_rebind`, `accumulate`, `caught`, `overwrite`, `Holder` and `reader`.
- **Harness:** a test (`review_probe`) that extracts and compiles it exactly as
  `tests/behavior.rs::compiled` does, then prints the tables.
- **Conditions:** a Rust test (`review_probe_conditions`) on `Condition`.

No repository file other than this review was changed.

**Guarantees not attacked, so asserted:**
- ty's use-def map under exceptional flow;
- the spike's numbers;
- two-way parity beyond its injected cases;
- the pilot's incidence of the memo defect (R5) and of opaque rebinding (R4b);
- the determinism of `flow_model` under shuffled input.

**Checks run** (2026-09-24):

| Command | Outcome |
|---|---|
| `just test-all` | **passed**: nextest 260/260, pytest 80, pyrefly, rules-scan, rule tests 7/7, `lint-agents`, `adr lint` (22), fixtures 64, `just deps` (family, cargo-deny, pyrefly-fork, shear), gold |
| `just pilot` | **passed**: snapshot `8730d102f07f0553544d72feb392c188`, content `e87e2694…` (the same content digest as `9c82f3db…`; every row count identical), generation `b29515e9fa41d820`, 39.8 s, smoke 20/20. The queries below read `8730d102…` read-only |
| `cargo test -p cpg-core --test review_probe` (scratch clone) | **passed** (it prints tables; the probe asserts nothing) |
| `cargo test -p cpg-schema --test review_probe_conditions` (scratch clone) | **passed** (it prints; see R3 and R9) |
| `just embed-serve`, `just pilot-live`, `just structured-eval` | **not_run** (excluded by the brief) |

## 2–4. Authority, contracts and derivation (compressed)

| Fact | Authority | Enforcement | Gap |
|---|---|---|---|
| Uses, definitions, reaching, regions | ty through `cpg-flow` | Two-way parity rules | — |
| Which names a test reads | The flow IR's uses in the test's span | **None.** `root_names` re-lexes the text | R8 |
| Condition identity | `cpg_schema::condition` | Syntactic, declared | The normal form is not unique (R9) |
| A negative claim's premise | `negative_premises` | `semantic:refuted-needs-complete-region` (join only) | The premise is too weak (R1); field loads are undercounted (R7) |
| An operation's runtime reachability | ADR §Composed layers | **None** | R2 |
| The normal path | `normal_path` over `raise_sites` | None | R3, R6 |

**Absence (DM-08).**
- `unknown` (`call_transfer`, `dynamic_access`, `budget_reached`) is distinct from `refuted`.
- **Collapsed:**
  - `false` is served as a condition, under `conditional` or `unknown` (R4);
  - a budget cut becomes `established` (R3);
  - a stub body's "never read" is indistinguishable from a real one (R1).

## 5. Journeys (compressed)

**Probe results** (`review_probe`, 2026-09-24):

| Shape | Expected | Served | Finding |
|---|---|---|---|
| `accumulate`: `cur=None; for it in items: if cur is None: cur = p else: sink(cur)` | `p → sink`, feasible from iteration 2 | `forwards p → sink`, **`conditional` under `false`** | R4a |
| `opaque_rebind`: `if "." in name: name = default` … `if "." in name: raise` … `sink(name)` | `default → sink` when the first test is true and the second false | **no `default` claim at all**; `name → sink` `established` | R4b |
| `typed_rebind`: the same with `is None` | Both flows | Both, correctly versioned (`is_none(name@19)`) | Fix (4) works for translated atoms |
| `cycle`: `x = p; while h(): y = h(x); x = g(y)` | `y` (arg of `g`) ← `h`, `p`, `g` | ← `h` only | R5 |
| `caught`: `try: if x is None: raise LookupError …; sink(y) except LookupError: pass` | `y → sink` when `!is_none(x)`; no raise escapes | `y → sink` **`established`**; `raises_when x is None` | R6 |
| `overwrite`: `mode = other; if mode == "http": …` | No `tests` for `mode` | **`tests mode: equals(mode,"http")`** | R8 |
| `Holder(token)`; `reader(): return make_holder("t")._token` | `Field[…Holder._token]` read | **premise `holds = true`** | R7 |

**The pilot** (`8730d102…`, *Measured*):
- **63 `refuted_under_model` claims.** 15 of them are on methods that release subclasses override
  (7 raise-only bodies, 2 stubs, 6 real bodies): for example, `Prompt.render`'s `arguments` (read
  by `FunctionPrompt.render`) and `AuthProvider.verify_token`'s `token` (read by
  `JWTVerifier.verify_token`). Both are served by `get_operation`. Of the rest, 28 are on stub
  bodies (`...`, `pass`, a docstring, such as `MessageHandler.on_message`), 1 is a `TYPE_CHECKING`
  stub, and 19 are on concrete bodies that no release class overrides. The classification is an
  AST pass over the release, matching classes by simple name.
- **R3.** 4 claims built from over-budget flows are served `established` with no condition
  (`FastMCP.read_resource` `uri` ×2, `IdentityAssertionValidator.validate` `client_id` and
  `resource_url`). 25 raise sites are over budget.
- **R4.** Three `value_flows` rows are under `false`, and two of them are served:
  `Settings.get_setting` and `set_setting`, where `attr` derives into `getattr`/`setattr`.

## 6. Acceptance gates

| Gate | Result | Evidence | Required action |
|---|---|---|---|
| G1 Authority | **fail** (narrow, and documentary) | Which parameters a test reads is derived twice: from the flow IR's uses, and by lexing and root-name matching (`flow_model.rs:671-706, 1306-1323, 1454-1480`). The two disagree (R8). The spine lags B17–B19 (R11) | R8, R11 |
| G2 Semantic fidelity | **fail** | Misleading refutations are served (R1, R2). A budget cut is served as `established` (R3). A feasible flow is stated under `false` or dropped (R4, R5). A caught raise removes a real condition (R6). `holds = true` is published for a read field (R7) | R1–R7 |
| G3 Validity | **fail** | Behaviors under condition `false` pass validation into the snapshot (R4). A holding premise that `syntax_nodes` contradicts passes (R7). The refutation rule's injected case covers only a missing premise (O5) | Rules `semantic:condition-not-false` and `semantic:premise-no-attribute-load` |
| G4 Hidden behavior | **pass** | The flow model reads only snapshot tables. `cpg-core` and `cpg-schema` sources are digested into `compiler_digest` (`attempt.rs:91-110`, `build.rs`). `cpg-flow` output is pinned by `EXTRACTOR_OUTPUT_VERSION` and its snapshot. ty's environment reads are classified | — |
| G5 Consistency and recovery | **pass** | Publication is unchanged. A duplicate behavior id fails the attempt (`behavior.rs:1128-1139`) | — |
| G6 Transformation and reuse | **fail** | The normal-path rewrite runs without its preconditions: the OverBudget equality (R3), and a raise that does not leave the function (R6). Loop-carried composition conjoins conditions from two iterations under one spelling (R4) | R3, R4, R6 |
| G7 Truthful capability claims | **fail** | `runtime_unreachable` is claimed (ADR L322-324; DESIGN L1232, L1263) and never emitted; the unsupported case falls back to `established` plus a refutation (R2). Claimed with no route: `__dict__`, Pyrefly-typed receivers, the per-run saturation report and the residue count (R10) | R2, R10 |

An unresolved or failed gate is not offset by the passing evaluation: its exit rule graded 6
pre-registered negatives, and none of the 63 served refutations is among them.

## 7. Findings

Ordered by cause severity: correctness on Stage 2's headline capabilities (R1–R7), then
authority and extension (R8, R9), then claims and spine (R10, R11).

| # | Finding | Principle IDs | Evidence | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| **R1** | **The parameter premise refutes "never read" on bodies the operation's callers do not run: stubs, `raise NotImplementedError`, methods a release subclass overrides, and runtime-unreachable declarations** | DM-08, DM-59, DM-24 · G2 | ADR L248: the premise holds when "No reference resolves to the binding …; the module has flow IR". Code: `unread` (`flow_model.rs:376-385`); `holds` unless the module is incomplete or `exec`/`eval` exists (`:1659-1690`); served at `behavior.rs:955-982`. On the pilot, 15 of 63 refutations are on methods a release subclass overrides, and 29 more are on non-overridden stub or `TYPE_CHECKING` bodies. 19 are on concrete, non-overridden bodies. `get_operation("fastmcp.prompts.Prompt.render")` gives `arguments: is_read refuted_under_model`, and the docstring says "Subclasses must implement this method" | An agent asking what `Prompt.render` or `AuthProvider.verify_token` does with its argument is told it is never read. Every concrete implementation reads it. This is the Q04.g error class, on the same override-dispatch ground that makes `self.m(...)` `unknown` | **ADR amendment** to the premise table: a parameter premise also needs (a) the declaration to be runtime-reachable (R2); (b) a body that is not a stub (only `...`/`pass`/docstring) or raise-only; (c) no release class overriding the method (MRO edges plus declarations by name). Otherwise the claim is `unknown` (`override_dispatch`, or an appended reason for an abstract body). Say that "never read" is about this body and cannot apply to a user override | **None exists.** `behavior_shapes`: a base method `def f(self, x): ...` overridden by a subclass that reads `x`, and an `abc.abstractmethod`. A rule `semantic:refuted-not-overridden`, with an injected case |
| **R2** | **`runtime_unreachable` is decided and labelled, but nothing emits it. The one pilot case is served as `established`, with a refutation** | DM-43, DM-59, DM-06 · G7, G2 | ADR L322-324; DESIGN L1232, L1263. `grep -rn RuntimeUnreachable crates/` finds the codebook only. `MCPServerConfig.__init__` (`mcp_server_config.py:171-179`, inside `if TYPE_CHECKING:`) is `behavior_status 0`, and its `data` is `refuted_under_model`. The class record serves it as the constructor (B20) | This is the Stage 2 review's F9 prediction verbatim: a checker-only stub reads as "does nothing and never reads `data`", while pydantic's `__init__` consumes every key at runtime | Mark an operation `unknown` (`runtime_unreachable`) when its declaration's region is `false`. `flow_regions` already has it: one join in `behavior.rs`'s status pass. Skip premises there | **None.** `behavior_shapes`: a public class with a `TYPE_CHECKING`-only `__init__`. A rule: no `established` operation whose declaration lies in a `false` region |
| **R3** | **`Condition::given` returns `true` when both the condition and the factor are over budget, so a budget cut is served as `established` with no condition** | DM-42, DM-08 · G2, G6 | `condition.rs:382-384`: `if self == factor { return Condition::always(); }`. `guards_of` pushes an over-budget guard's `not()` (itself over budget) (`flow_model.rs:715-720`). `normal_path` folds it into `all` (`:743-746`). `behavior.rs:1107-1109` then keeps `Established`. Scratch test: `OverBudget.given(&OverBudget)` encodes `true` | The ADR says a larger condition "is not stated, and its record is `unknown` with `budget_reached`" (L190-191). Instead, 4 pilot claims in 2 operations (for example, `uri` derives into `ResourceError` in `FastMCP.read_resource`) are served as unconditional facts | `given` returns `self` unless both sides are `Dnf`. `guards_of` skips over-budget guards; a function with one states its fates unfactored | A unit case in `conditions.rs`: `OverBudget.given(&OverBudget) == OverBudget` |
| **R4** | **Conflation stays unsound: (a) across a loop's back edge, where the flow model conjoins two iterations' tests under one spelling; (b) for opaque tests, which are never versioned. Feasible flows are stated under `false` or dropped** | DM-24, DM-42, DM-07 · G2, G3, G6 | ADR L221-231, and L229-230 ("except around a loop's back edge, where ty's diagrams do not unroll either"). But `reach` follows `loop_carried` definitions and conjoins their value's conditions with the use's (`flow_model.rs:612-624`; `lib.rs:123-125` says the loop-carried condition is "the use's side only"). Opaque atoms quote the text, with no version (`predicate.rs:62-64`). Probe `accumulate`: `p → sink` `conditional` under `false`. Probe `opaque_rebind`: no `default → sink` row. Pilot: `Settings.get_setting`/`set_setting` `attr → getattr`/`setattr` under `false`, and `_find_package_root` | Deviation B15 fix (4) claims "a feasible path is no longer dropped as a contradiction". That holds only for translated atoms in loop-free code. A `conditional` claim under `false` contradicts its own verdict, and a dropped flow is a silent absence in a facet that `behavior_status` may mark complete | (a) At a loop-carried step, drop the definition-side literals rather than conjoin them. That is a sound over-approximation, and it matches `Reach.loop_carried`'s own contract. (b) Make each opaque test a fresh atom per test site (identity by span), or version the places read inside its span. Amend ADR §Places: the one-value guarantee holds per iteration and for translated atoms only. (c) A rule rejecting `condition = 'false'` in `behaviors` and `value_flows` | **None.** The `accumulate` and `opaque_rebind` probes as `flow_shapes`/`behavior_shapes` cases, plus the rule, with an injected case |
| **R5** | **`Model::reach` memoizes results computed while a cycle was cut, so later queries reuse incomplete source sets** | DM-24, DM-40 · G2 | `flow_model.rs:527-532` (a memo hit, and a cycle cut that returns an empty set) and `:628-631` (memoizing unconditionally). Probe `cycle`: the argument of `g(y)` gets `h` only. `p` and `g` reach it through `y = h(x)` and `x = g(y)`, but were cut while sink `h(x)` was computed first | In loops, a parameter's `derives`/`forwards` into a later sink goes silently missing. The loss depends on sink iteration order. Pilot incidence is not measured | Memoize only subtrees with no cut; otherwise compute sources per SCC of the use → definition → use graph to a fixpoint | **None.** The `cycle` probe as a `behavior_shapes` case |
| **R6** | **The normal path assumes a raising guard leaves the function. A raise that the same function catches, or that a `with` may suppress, is still factored out, and its `raises_when` is served** | DM-24, DM-04 · G6, G2 | ADR L193-197 ("says only that no error was raised"); `raise_sites` takes every `raise` (`flow_model.rs:386-395, 1297-1342`); `normal_path` (`:730-751`). Probe `caught`: `y → sink` served `established`, and `raises_when x is None` served, though `LookupError` never escapes. Pilot (static AST scan): 21 conditional raises inside a `try` whose handler catches them, 3 of them where the handler continues (`client/elicitation.py:78`, `cli/deploy/command.py:225, 315`) | "Sends `y`" is stated unconditionally for a call that returns without sending. "Raises when `x is None`" is claimed for a function that never raises | **ADR amendment:** a guard is a raise that can leave its function, so not inside a `try` whose handler may catch its type (bare, `Exception`, `BaseException` or the name) and not inside a `with` that may suppress. Otherwise it is neither a guard nor a `raises_when` fate. Record `escapes` on `raise_sites`. State that the normal path is function-wide ("executions that raise at no guard") | **None.** The `caught` probe as a `behavior_shapes` case |
| **R7** | **The field and global premises count only loads on place receivers, so `call().f` and `x[k].f` are missed, and a premise holds for a field the release reads** | DM-07, DM-59 · G2, G3 | ADR L249: "no attribute load named `f` on **any** receiver". `field_reads` comes from flow uses (places only), and skips any place containing `[` (`flow_model.rs:1074-1107`, `:1098`). Pilot: `Field[fastmcp.server.server.FastMCP._worker]` `holds = true`, while `fastmcp_tasks/dependencies.py:139` reads `get_server()._worker`. The `model_config` premise of 25 classes holds while `tools/function_parsing.py:188` reads `schema["cls"].model_config`. Probe `Holder._token` is the same | Latent in serving today (only singletons' `Global` premises are rendered). But `negative_premises` and the bundle's `place_claims` publish a false `holds`, and the refutation rule trusts it. This is the soundness floor the Stage 2 review's F1 demanded | Count attribute loads from `syntax_nodes` (every attribute expression in a load context, by name), not flow uses | Rule `semantic:premise-no-attribute-load` (a holding `Field`/`Global` premise contradicted by a syntax-level load), with an injected case, plus the `Holder` probe |
| **R8** | **Which parameters a test reads is re-derived from text and root names, not from the flow IR's uses and reaching definitions** | DM-02, DM-41 · G1, G2 | `root_names` (`flow_model.rs:671-706`); root prefix `split(['.', '@'])` (`:1313, 1461`). Keyword names count as reads, and f-string fields do not. `@line` is stripped even when the parameter's definition does not reach the test. Probe `overwrite`: `tests mode: equals(mode,"http")` after `mode = other`. Probe `typed_rebind`: `default` gets no `raises_when`, though the raise needs it. Pilot: `ProxyDCRClient.validate_redirect_uri` | Positive `tests`/`raises_when` claims name parameters whose value never reaches the test, and miss ones whose value does. Every new test form widens the gap | Take the uses inside the test's span from `flow_uses`, follow `flow_reaching` to a parameter's definition (identity or derived), and delete `root_names` | The `overwrite` and `typed_rebind` probes as `behavior_shapes` cases |
| **R9** | **The normal form is not unique; ADR L186-188 says it must be** | DM-15 · G6 (declared, so not failing alone) | Scratch test: `!truthy(b) \| !truthy(c) & truthy(a) & !truthy(d) \| truthy(a) & truthy(b) & truthy(c) \| !truthy(d) & !truthy(a)` and a permutation normalize to two encodings (`… \| !truthy(c) & !truthy(d) \| …` against `!truthy(b) \| !truthy(d) \| truthy(a) & truthy(c)`). The normalizer is sound and terminates (§Answers 2) | One condition, two ids. `given` misses a factor met in a different order, so the Stage 2 packet's unreadable-guard problem can recur | Sort conjunctions (then literals) before resolving, and add consensus within the 16 × 8 budget (Blake canonical form is cheap at this size). Or amend L186-188 to "normalizes more cases; equality stays syntactic" | A known-answer pair in `conditions.rs` |
| **R10** | **Claimed without a route: `__dict__` as dynamic access; receivers Pyrefly types as `C`; "the pilot's largest scope is reported per run"; the annotation residue "counted per run"** | DM-43, DM-59 · G7 | ADR L262, L266, L192; DESIGN L1143, L1220. `flow_model.rs:1502-1503` comments `__dict__`, but the loop is over calls (`:1531-1553`). No type lookup exists in `:1566-1606`. `just pilot` prints neither count. Pilot: `self.__dict__.items()` (`client/oauth_callback.py:96`), which the name-based floor happens to cover | A class read only through `__dict__`, or through a typed non-`self` receiver, can have a field refuted. Readers of the spine believe the report exists | Implement, or relabel as **Proposed** in ADR §Verdicts and DESIGN §3.2/§3.9 | `behavior_shapes` `__dict__` and typed-receiver cases; prose for the reports |
| **R11** | **The spine lags B17–B19 and Stage 2's numbers** | DM-02, DM-49 · G1 (documentary) | DESIGN L626: "a stored field's reads in any relative's method compose", where B17 restricts it to unchanged stores (`behavior.rs:995-1006`). `tests` (kind 12, B18) appears in neither §3.2 nor §11.3's `get_operation` list. B19's shadowing rule (`flow_model.rs:779-801`) is a place-identity decision missing from ADR §Places, and it depends on import order (the global must be bound after the submodule's first import). DESIGN L627 gives 66,958 uses and 68,931 reaching rows; the pilot now gives 66,954 and 69,005 | A reader of DESIGN expects computed stores to compose, and does not know `tests` exists. A library that binds the global before importing the submodule resolves wrongly, and nothing says so | Sentences in §3.2 and §11.3. A B19 sentence in ADR §Places, stating the import-order assumption. Refresh or date the counts | Prose |

**Observations** (not findings):
- **O1.** A read inside a lambda or comprehension scope gets no reader, because `scope_join`
  names no declaration. Its phase is therefore `import` (`flow_model.rs:1388-1397`). This occurs 0
  times on the pilot.
- **O2.** An inherited method of a singleton's class counts as a field read (`methods` holds own
  methods only, `:1346-1352, 1384`). Example: `first_sample.register_all`.
- **O3.** The strongest-transfer filter drops a weaker path's condition (`:652-661`).
- **O4.** Decorated definitions have no value source, and lambda and generator bodies read as
  `derived` (`lib.rs:416, 703-711`).
- **O5.** The injected case for `refuted-needs-complete-region` exercises only a missing premise
  (`tests/analysis.rs:1483-1497`).
- **O6.** An unmapped argument at a site with several release callees falls back to `established`
  with callee text (`behavior.rs:772-775, 878-881`). This occurs 0 times on the pilot.

**Applicability.**
- **Groups that bore:** 2 (absence and validity: R1–R4, R7); 5 (derivation: R3–R6); 9 (the
  provider boundary: R8); 12 (claims: R2, R10); 1 (authority: R8, R11).
- **Did not bear:** 6 (publication is unchanged); 7 and 8 beyond digests (no new caches or
  performance claims; flow 0.73 s, flow model 1.56 s, pilot 39.8 s, *Measured* with no target); 4
  (no declaration surface added).

## 8. Alternatives

| Alternative | Duplication and locality | Risk | Cost | Verdict |
|---|---|---|---|---|
| **Baseline:** Stage 1 (no flow IR) | — | Q04 and Q10 unanswerable | Built | Rejected by ADR-0022 |
| **As built** | Test roots derived twice (R8) | R1–R7 | Built | Revise |
| **Simpler viable alternative:** keep the provider, conditions and tables, and change four rules. (1) Refute parameters only in concrete, non-overridden, runtime-reachable bodies, and say `unknown` elsewhere. (2) At a loop-carried step, keep the use-side condition only; make each opaque test site its own atom. (3) A guard is a raise that escapes the function. (4) Attribute tests through `flow_uses` and `flow_reaching`, and count field loads from `syntax_nodes` | One authority per fact. Each rule is one join over existing tables | Precision falls: fewer refutations (19 of 63 remain on the pilot), and opaque repeats no longer contradict. Every rule is sound under the stated model | Smaller than the current code. `root_names`, and the opaque half of versioning, are deleted | **Recommended** |

This is ordinary code behind existing contracts (charter §F). Nothing here needs a declaration
surface.

## 9. Verification plan

| Claim | Label | Check | Gap |
|---|---|---|---|
| No refutation on overridden, stub or unreachable bodies | Proposed | `behavior_shapes` cases and `semantic:refuted-not-overridden` | Rule and cases (R1, R2) |
| A budget cut is never `established` | Tested (scratch) | `conditions.rs` unit case | Case (R3) |
| No condition `false` is served | Measured (2 on the pilot) | `semantic:condition-not-false` | Rule (R4) |
| Loop and opaque flows are not dropped | Tested (scratch probes) | `accumulate`, `opaque_rebind`, `cycle` cases | Cases (R4, R5) |
| A caught raise is not a guard | Tested (scratch probe) | `caught` case | Case (R6) |
| Field premises count every load | Measured (`_worker`, `model_config`) | `semantic:premise-no-attribute-load` | Rule (R7) |
| One encoding per condition | Tested (scratch, a counterexample) | A known-answer pair | Case (R9) |

## 10. Deferred

| Item | Why not now | Reopen when |
|---|---|---|
| O3 and O4 (the strongest-transfer filter; decorators and lambdas) | Precision only | Stage 3's summaries touch transfers |
| O6 (multi-callee unmapped arguments) | 0 on the pilot | A pilot or fixture row appears |
| A per-iteration unrolled loop model | R4(a)'s over-approximation is sound | An evaluation item graded `partial` for a loop condition |

## 11. Decision

**Decision: Revise.** ADR-0022 stays `proposed`.

**Reason.** G2, G3, G6 and G7 fail on Stage 2's own headline capabilities:
- negative claims (R1, R2, R7);
- conditions (R3, R4, R6);
- flows (R5).

The provider, the lowering, the encoding and the parity rules are sound and tested. The evaluation's
exit rule passed without reaching the refutations.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| P1 | R1: amend the parameter premise; implement it | DM-08, DM-59 | ADR §Verdicts text; pilot refutations re-counted | Rule plus `behavior_shapes` |
| P1 | R2: emit `runtime_unreachable` | DM-43 | `MCPServerConfig.__init__` `unknown` | Rule plus a fixture case |
| P1 | R3: `given` over budget | DM-42 | The 4 pilot claims become `unknown` (`budget_reached`) | Unit case |
| P1 | R4: loop-carried conditions; opaque atoms; the `false` rule; ADR §Places sentence | DM-24, DM-07 | 0 rows under `false` on the pilot | Rule plus cases |
| P1 | R6: guards must escape; ADR §Conditions sentence | DM-24 | `caught` probe correct | Case |
| P2 | R5: memo under cycles | DM-24 | `cycle` probe correct | Case |
| P2 | R7: field loads from syntax | DM-07 | `_worker` premise `false` | Rule |
| P2 | R8: attribute tests through the flow IR; delete `root_names` | DM-02 | `overwrite` probe correct | Cases |
| P3 | R9, R10, R11 | DM-15, DM-43, DM-49 | Text or code | Case; prose |

Then run a `compact` re-review of R1–R8 and accept ADR-0022. If two consecutive reviews find
nothing, ADR-0001's revisit trigger fires; this review did not find nothing.

**Final check.**
- **Claims against evidence:** mostly matched. The exceptions are R2 and R10, which are claimed
  without a route.
- **Scope against guarantees:** not yet. "`refuted_under_model` only where the premise holds" is
  enforced, but the premise does not mean what an agent reads it to mean.
- **Extension paths:** clear for atoms and reasons. They are not yet clear for a new test form,
  which would widen R8.

## 12. Disposition (author, 2026-09-24)

The review's simpler viable alternative (§8) is adopted: the same provider, conditions and tables,
with the four rules changed, plus the other fixes. ADR-0022 was amended in place (it is
`proposed`); DESIGN §3.2, §3.9 and §11.3 were brought up to date. Measured on the pilot, snapshot
`162bda5a0fe39cc1cedadbfaa8efd1f9` (2026-09-24, `just pilot` passed, 40.4 s, smoke 20/20).

| # | Disposition | Oracle | Pilot |
|---|---|---|---|
| R1 | **Fixed.** The parameter premise also needs a runtime-reachable declaration, a body that is not abstract, a stub or raise-only (`abstract_body`, boundary reason 20, appended), and no release class that inherits the method defining it again (`override_dispatch`). "Never read" is about this body (ADR §Verdicts) | `semantic:refuted-not-overridden` (injected case: `pkg.Handler.handle`, which `Special` defines again); `the_stage2_end_review_probes_get_their_answers` (`Handler.on_message`, `render`, `Job.run`; `concrete` still refuted) | Refutations 63 → 19; `Prompt.render`'s `arguments` and `AuthProvider.verify_token`'s `token` `unknown` (`abstract_body`) |
| R2 | **Fixed.** A declaration whose statement's region, or an enclosing declaration's, is `false` is unreachable; its operation and every claim about it are `unknown` (`runtime_unreachable`) and its premises do not hold | `semantic:unreachable-not-established` (injected case); the probe `Config.__init__` | `MCPServerConfig.__init__` `unknown` (18), its `data` `unknown` (18) |
| R3 | **Fixed.** `given` returns `self` unless both sides are within budget; `guards_of` skips a guard past the budget | `nothing_factors_out_of_a_budget_cut` (`conditions.rs`) | `budget_reached` 32 → 42; `read_resource`'s `uri` claims now carry their conditions |
| R4 | **Fixed.** (a) At a loop-carried step only the use's side is kept. (b) An opaque test reading a versioned place carries the version (`opaque("…")@line`). (c) A condition `false` is rejected | `semantic:condition-not-false` (injected case); probes `accumulate` (`!is_none(cur)`), `opaque_rebind` (`default → sink`) | 0 behaviors and 0 value flows under `false` |
| R5 | **Fixed.** `reach` memoizes a result only once its cycle closes (a lowlink over the stack) | Probe `cycle` (`p → g` `call_transfer`) | — |
| R6 | **Fixed.** A guard is a raise that may leave its function; `raise_sites.escapes`; a handler may catch by the builtin exception tree, the release's MRO, or anything unresolved; `with suppress(...)` suppresses, other context managers are assumed not to (Stage 3's models) | Probes `caught`, `suppressed` (`y → sink` under `!is_none(x)`, no `raises_when`) | 35 of 790 raises may be caught |
| R7 | **Fixed.** The flow family records every attribute load by name, on any receiver, and `getattr`/`hasattr` with a literal name (`flow_attribute_loads`); the field and setting premises count them | `semantic:premise-no-attribute-load` (injected case); probe `reader` (`Holder._token` does not hold) | `FastMCP._worker` and every `model_config` premise do not hold |
| R8 | **Fixed.** `root_names` is deleted. The flow family records every test ty records (`flow_tests`); a test reads the parameters reaching the uses inside its span | Probes `overwrite` (`tests other`, not `mode`), `typed_rebind` (the raise tests `name` and `default`) | 6,148 tests |
| R9 | **Fixed** for order: resolution runs over sorted conjunctions. The ADR says equality stays syntactic across different inputs | `a_normal_form_does_not_depend_on_conjunction_order` | — |
| R10 | **`__dict__` built**; Pyrefly-typed receivers, the per-run largest-scope report and the residue count relabelled **Proposed** in the ADR and DESIGN | Probe `Snapshot.dump` (`Snapshot.a` does not hold, `dynamic_access`) | — |
| R11 | **Fixed:** DESIGN §3.2 (B17's unchanged-only composition, B18's `tests`, the premise, `runtime_unreachable`, the new tables and counts), §11.3 (`tests`); ADR §Places (B19, with its import-order assumption) | Prose | — |
| O5 | **Fixed:** an injected case where the premise exists and does not hold | `the_analysis_rules_reject_their_violations` | — |

**Deferred**, each with its trigger (the review's §10 and the observations):

| Item | Why not now | Reopen when |
|---|---|---|
| O1: a read inside a lambda or comprehension gets phase `import` | 0 on the pilot | A pilot or fixture row appears |
| O2: an inherited method of a singleton's class counts as a field read | Over-counts reads (sound direction) | A setting answer lists a method |
| O3, O4: the strongest-transfer filter; decorators and lambdas | Precision only | Stage 3's summaries touch transfers |
| O6: an unmapped argument at a multi-callee site | 0 on the pilot | A pilot or fixture row appears |
| A per-iteration unrolled loop model | R4(a)'s over-approximation is sound | An evaluation item graded `partial` for a loop condition |
| Pyrefly-typed receivers; the largest-scope and residue reports (R10) | Proposed; the name-based premises cover the pilot | A refutation on a field read through a typed non-`self` receiver, or a report consumer |

**Checks (2026-09-24):** `cargo nextest run --workspace` passed 262/262; the all-techniques guard
moved as a declared migration (`flow_tests`, `flow_attribute_loads`, `raise_sites.escapes`,
`abstract_body`, `EXTRACTOR_OUTPUT_VERSION` 22); the analysis ledger did not move. A `compact`
re-review of R1–R8 follows before ADR-0022 is accepted.
