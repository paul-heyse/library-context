# Design review: Stage 2.9 semantic bridge, compact pass

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Uncommitted Stage 2 end repairs R1–R8 and Stage 2.9 X1–X3, X6: `cpg-flow`, `cpg-extract` runtime binding, condition schema, the developer flow command and execution oracle; ADR-0022 §Conditions/§Places/§Verdicts and DESIGN §3.9 |
| Standard | Core 2.0, code-intelligence profile 1.0 and `binding/library-context.md` through `standard.toml` |
| Tier · purpose | Change · conformance |
| Reviewer · date | Design-reviewer agent · 2026-09-24 |
| Decision | **Accept** after the corrected-tree disposition in slot 12; initial pass was Revise |

**Outcome sought.** A ty predicate becomes a condition on the evaluation it denotes, runtime-special names are decided through lexical bindings, losses are counted, and CPython execution challenges the may-analysis before ADR-0022 is accepted.

**Baseline.** The Stage 2 end review's §12 disposition records R1–R8 repairs and their focused tests; the external assessment's X1–X6 supplied the Stage 2.9 risk cases. Those review documents are evidence, while ADR-0022 (still proposed), DESIGN and the executing code are the subjects. The new condition encoding changes stored identity and the extractor output version is 23.

**Method and coverage.** Read the applicable standard, binding, previous reviews, plan Stage 2.9, the changed production and test paths, and the service path that renders conditions. Traced predicate translation to skipped reaching rows and to conditions served by `get_operation`. Ran local CPython/lctx counterexamples for `isinstance` rebinding and nonlocal mutation, and a CPython `sys.version_info` tuple check. Examined the existing R1–R8 disposition and their code/test routes; did not independently rerun the full workspace or pilot while the implementation agent was running those gates. Did not review Stage 3, unrelated extractors, bundle publication internals or every condition-algebra rewrite. Tests present in the tree are *Implemented* evidence until a named gate run reports its result. Binding conflicts K1–K3 did not affect the decision; this review follows the core/profile and reads accepted historical `DM-nn` citations through core §I.

## 2. Authority and identity map

| Fact or concept | Semantic type and identity | Authority / owner | Revision and update | Derived representation |
|---|---|---|---|---|
| Source predicate and reaching relation | ty predicate and use-def handle within one pinned module | `ty_python_core` through `cpg-flow` | Provider revision, source content and runtime context | `flow_tests`, `flow_reaching`, `flow_regions` |
| Runtime-special import binding | Lexical reference and candidate binding IDs | `cpg-extract` lexical rows plus `export_syntax` | Release and extractor version | `RuntimeBindings` spans passed to `cpg-flow` |
| Condition atom | Operator, operands and evaluation identity | `cpg-schema::condition`, constructed by `cpg-flow::predicate` | Module path/content key; extractor output version 23 | Encoding, condition ID, literal rows and served `condition` text |
| Skipped candidate | Cause assigned during flow lowering | `cpg-flow::SkipCounts` | One module attempt | Flow coverage `detail` counters |
| Observed execution | CPython 3.14.7 line/instruction event in generated program | `sys.monitoring` worker | One test invocation | Oracle assertion against `lctx flow` JSON |

**Code-intelligence fact and fidelity table.**

| Family / relation | Provider and revision | Fidelity | Coverage / unknowns | Identity | Consumer |
|---|---|---|---|---|---|
| Flow uses, definitions, reaching, regions and tests | ty 0.0.14 via `cpg-flow::PROVIDER` | Extracted and translated; condition is derived may-behavior | Per-module flow coverage; ty's ambiguous terminal is admitted; syntax errors mark partial | Release module, source span and provider-local relation ID | Behavior model, condition tables, served operation fates |
| Runtime-special bindings | Lexical resolver and import syntax, same release | Resolved only when all candidates agree | Unresolved/mixed candidates stay ordinary conditions | Reference span and binding IDs | Runtime predicate translation |
| Skipped counts | Flow translator | Derived diagnostic, with precedence rather than exclusive causal proof | Candidate counts, not persisted missing rows | Module coverage row | Operator diagnosis |
| Oracle observations | CPython `sys.monitoring` on tiny generated programs | Observed | Fixed grammar and bounded inputs only; no pilot execution | File, line and instruction position | Validation lane only |

The module key hashes source and path, so conditions are revision-specific. The initial definition-set sharing key was unsafe because static binding sets did not track intervening effects (F01); the corrected tree uses evaluation sites. The raw identity-bearing encoding is served in fate `condition` strings; no second mutable condition authority was found in this scope.

## 3. Contracts and invariants

| Contract | Enforcement point | Failure behavior | Evidence |
|---|---|---|---|
| Distinct evaluations share an atom only if their outcomes must agree | `Translator::atom` and call sites | A false conjunction drops a feasible row; F01 | `predicate.rs:100–110, 197–245`; counterexample below |
| Runtime decisions require agreed lexical import resolution | `runtime_bindings` and `Translator::runtime` | Otherwise leave ordinary atom | `cpg-extract/src/flow.rs:31–115`; `predicate.rs:250–338` |
| Every false skip has a truthful category | `skip_cause`, `runtime_in_diagram`, `sources` | Mislabelled count; F04 | `lib.rs:637–646, 782–785, 841–845`; `predicate.rs:462–488` |
| Every observed statement and reaching definition is admitted | `test_flow_soundness.py` assertions | Missing model rows evade the check; F03 | `:114–121, 193–215` |

An unsupported or opaque test remains a may-condition, not proof of absence. A `false` condition is omitted before persisted facts and should be justified by ty, the runtime view or a valid contradiction. The condition encoding promises syntactic identity in Stage 2; it does not promise Boolean canonicality. R1's negative premises, R2's runtime-unreachable state and R3's budget state remain separate from absence in the inspected code paths and prior review disposition.

## 4. Derivation and execution

| Stage / question | Projection or input; method and model | Budget, output and evidence | Boundary / exactness |
|---|---|---|---|
| Import resolution: is a name special at runtime? | All lexical candidate bindings for a reference and recognized import syntax; only unanimous candidates are marked | One span set per module | Same-release typed rows to an in-process span adapter; no dynamic import semantics claimed |
| Predicate lowering: when can a path occur? | ty decision diagram and source AST to DNF over evaluation atoms; ambiguous terminal admitted | 16 conjunctions × 8 literals; `OverBudget` stays unknown | Intended conservative may-analysis, but F01/F02 give counterexamples |
| Flow extraction: which definitions reach a use? | ty use-def map, path condition and loop-header expansion | Skips `false`, increments module counters, emits direct rows | F04 affects diagnostic fidelity; no all-path closure is stored |
| Behavior and serving: what does an operation do? | Same-snapshot flow rows into behavior findings and `get_operation` fates | Five verdicts, premise and boundary reasons; raw condition encoding | R1–R8 test routes exist; the changed encoding reaches the service intact |
| Oracle: does observed execution fit? | CPython generated-program monitoring vs `lctx flow` JSON | 18 Hypothesis examples plus one reaching test in the inspected script | F03: present assertions do not establish the stated subset relation; command omits resolved runtime bindings |

These are independent relation kinds: source control reachability, use-def reaching and value-source flow are not renamed as each other. The oracle reads only temporary generated programs, not pilot code or gold references. It uses the `sys.monitoring` built-in and Hypothesis rather than a custom execution tracer.

## 5. Journeys

- **Rebinding and mutation.** `x` has one definition while `C` is rebound from `int` to `str`. `f(1)` executes the nested `return 1` on CPython; `target/debug/lctx flow` reports that statement's region as `false` (2026-09-24 local probe). A second probe changes a captured `x` through `nonlocal x` inside `change()` between two `x is None` tests; CPython again returns 1 while the region is `false`. Thus even a singleton identity test on a name with the same static definition set can change across an effectful call (F01). Attribute, call and truthiness tests are already per site in the inspected translator.
- **Runtime context.** `sys.version_info(3,14,7,'final',0) > (3,14,7)` is true in the pinned interpreter; the three-element model compares equal. This drops the true branch (F02). Two-element comparisons in `flow_shapes` exercise a different case.
- **An unresolved name.** Mixed lexical candidates are left as ordinary atoms. This preserves may-behavior; the independent extractor test checks parameter and alias cases. The direct oracle command supplies empty runtime bindings, so this journey does not receive CPython differential coverage (F03).
- **Served claim.** The condition ID is retained in the snapshot and its identity-bearing encoding flows into a fate's `condition`; tool instructions now say established/conditional are may-behavior. This review did not find a new cross-snapshot join path. The display remains a raw encoded string, a deferred usability point below.

**R1–R8 replay at the changed boundary.** The repaired routes are present, with the prior review §12 supplying their last executed pilot result; this table identifies what still needs a fresh version-23 run. R4 now also depends on F01's evaluation identity.

| Prior finding | Inspected repair and regression route | Compact verdict before fresh gates |
|---|---|---|
| R1 negative premise | Concrete, non-overridden body premises and `semantic:refuted-not-overridden`; `behavior.rs:556–574` checks abstract/stub/overridden against concrete | Implemented; no new defect found |
| R2 unreachable declaration | `runtime_unreachable` operation/fate path; `behavior.rs:576–595` checks a TYPE_CHECKING-only constructor | Implemented; X2 resolution still needs oracle coverage |
| R3 budget factoring | `guards_of` refuses `OverBudget` (`flow_model.rs:907–922`); `conditions.rs:83` tests no factoring past the cut | Implemented; no new defect found |
| R4 false loop/opaque paths | Loop-carried composition retains use-side condition (`flow_model.rs:715–722`); `behavior.rs:485–502` checks the loop and opaque-rebind probes; semantic false-condition rule | Implemented, but F01 reopens the broader false-path guarantee |
| R5 cycle memo | Lowlink completion controls memoization (`flow_model.rs:618–740`); `behavior.rs:509–515` tests cycle source | Implemented; no new defect found |
| R6 caught raise | Escaping raises only become guards (`flow_model.rs:907–922`); `behavior.rs:517–527` checks caught and suppressed | Implemented; no new defect found |
| R7 missed attribute loads | `flow_attribute_loads` feeds release-wide name query (`flow_model.rs:372–379`); `behavior.rs:534–553` checks call receiver and `__dict__` | Implemented; no new defect found |
| R8 text-based test roots | `flow_tests` plus use-def facts replace `root_names`; `behavior.rs:503–507, 529–532` checks typed rebind and overwrite | Implemented; no new defect found |

## 6. Gates

| Gate | Verdict | Independent evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | ty predicate, lexical bindings and schema condition each have separate roles; source module key scopes the derived encoding | — |
| G2 Semantic fidelity | **Fail** | F01 and F02 drop CPython-reachable regions | Correct both lowerings |
| G3 Validity | **Fail** | F01 yields `false` before a reachable fact can be published; F03's oracle does not detect all missing rows | Enforce safe sharing and admission checks |
| G4 Hidden behavior | Pass | Oracle's executed inputs are generated test programs; compiler analysis still receives explicit source and context | — |
| G5 Consistency and recovery | Pass in changed scope | No new publisher or retained closure; module coverage receives skip counts and prior publication gate is unchanged | — |
| G6 Transformation and reuse | **Fail** | Predicate-to-atom sharing changes may-reachability (F01); runtime tuple lowering differs from CPython (F02) | Site identity where stability is unproved; full tuple semantics |
| G7 Truthful capability claims | **Fail** | ADR-0022 and DESIGN say sharing is stable and version comparisons use full tuple ordering; code contradicts both. Oracle is described as observed⊆admitted but checks fewer observations (F03) | Amend implementation or narrow claims |
| G8 Library leverage | Pass | Domain-specific translation uses pinned ty facts; `sys.monitoring` and Hypothesis cover the generic oracle mechanisms; no clear replaced library capability in this slice | — |
| CI-G1 Fidelity | **Fail** | A `false` region can mean a feasible path (F01/F02); X3 cause can be misreported (F04) | Correct translation and diagnostics |
| CI-G2 Evidence closure | Pass in changed scope | The fate keeps its condition from the generation; no changed synthesis or cross-generation lookup was found. Prior same-snapshot validators remain in place | — |
| CI-G3 Evaluation integrity | Pass | The generated programs and expected subset property live in tests; no gold path feeds compile, and fixed shapes were declared before this run | — |

**Applicable principles.** DP-01/02/03/04/07/08/11/13/15/19/21/22/23/24 and CI-01/02/04/06/07/08/10/11/12/13 bear on the changed path. DP-02, DP-03, DP-08, DP-11, DP-22, DP-23 and CI-02, CI-06 are **Violated** by F01–F03. DP-21 and CI-04 are **Violated** for X3 diagnostics (F04). The others named here are **Satisfied** within the inspected scope, except CI-11 is **Unresolved** for the whole product because full bundle grounding was not independently checked in this compact pass; the gate above is scoped to unchanged join paths. DP-05/06/09/10/12/14/16/17/18/20 and CI-03/05/09 were examined for a new burden but have no new in-scope decision or mechanism; no verdict about the wider product follows.

## 7. Findings

| ID | Finding | Principles · gate | Evidence | Consequence | Correction | Verification |
|---|---|---|---|---|---|---|
| **F01** | Definition-set identity is treated as proof that separate evaluations agree, even though class operands or captured names can change | DP-02, DP-04, DP-08, CI-02, CI-06 · G2/G3/G6/CI-G1 | Initial tree: `predicate.rs:207` shared nonsingleton `IsValue` and `:212–245` shared `isinstance(x,C)` using only `x`'s definitions. Local probe: `C=int; if isinstance(x,C): C=str; if not isinstance(x,C): return 1`, called with `x=1`, returns 1 on CPython but `lctx flow` gives `return 1` region `false`. A second probe: a closure changes `nonlocal x` between two `x is None` tests; CPython returns 1 and the modeled region is again `false`, despite the same apparent definition set | A reachable branch and its facts are silently absent, so a negative claim can be unsound | Use site identity unless a stronger proof covers all outcome-affecting operands and mutations, including closure/global writes. Singleton operands alone do not prove the subject stable. Amend ADR wording accordingly | Add both counterexamples and non-singleton `is` to `flow_shapes` and the runtime oracle; assert observed lines admitted |
| **F02** | The runtime view models `sys.version_info` as a 3-tuple, whereas CPython exposes a 5-tuple | DP-08, DP-11, CI-06 · G2/G6/CI-G1 | `predicate.rs:269–275` constructs `[major,minor,micro]`. `uv run python -c` on CPython 3.14.7 reports `sys.version_info > (3,14,7)` true and `<= (3,14,7)` false | Version-guarded code at an equal three-part prefix can be assigned the opposite branch and disappear | Compare against the full `(major, minor, micro, releaselevel, serial)` value under a recorded runtime context, or leave comparisons undecided unless modeled completely | Three-part equality/prefix and five-part tuple cases under resolved `sys`; compare to CPython |
| **F03** | The execution oracle does not check every observation against the same runtime-view path the compiler uses | DP-23, CI-06, CI-10 · G3/G7 | `test_flow_soundness.py:116–121` skips an observed line when no region starts there; `:199–214` skips an observed local access without a modeled use or definition and only requires one match. `lctx/src/main.rs:161–165` supplies `RuntimeBindings::default()`, bypassing resolved TYPE_CHECKING/sys/os decisions | The test can pass while a branch or reaching definition is omitted; X2 regressions receive no runtime differential check | Assert admission for each in-scope observed statement and each mapped LOAD/STORE, with explicit out-of-model exclusions; feed the command the same lexical runtime bindings as compilation, or run a separate extractor-backed differential path | Inject one omitted region/reaching row as a negative test; run parameter, alias and 3-part version programs through the runtime lane |
| **F04** | Nested runtime-view decisions are counted as stable contradictions | DP-21, CI-04 · G7/CI-G1 | `predicate.rs::runtime_in_diagram` checks `runtime_decides` on the whole predicate expression, but `Translator::test` recursively decides `BoolOp`, unary `not` and conditional expressions; `lib.rs:841` has the same whole-test check for value sources | `if TC and x:` can discard a row due to the runtime view while coverage reports a stable-atom contradiction, misleading diagnosis of a missing fact | Carry decision provenance out of the recursive translation, or recurse over expression children when assigning skip cause; preserve the documented precedence | Fixture with resolved `TC` nested under `and`, `not` and conditional expression; assert exact category, including a value source |

**Strengths that carry weight.** Per-site atoms now separate impure calls and mutable places; the codebook append preserves prior codes; resolution requires unanimous candidates; `OverBudget` and unknown remain distinct from `false`; R1–R8 have focused regression routes in the prior review §12. These are concrete improvements, but do not offset the counterexamples.

## 8. Library-leverage ledger

| Capability | Bespoke code | Candidate built-ins / library | Fit and gaps | Recommendation |
|---|---|---|---|---|
| Program predicate and use-def facts | `cpg-flow` adapter and translator | Pinned ty semantic index | ty owns the underlying facts, but not this product's runtime may-model | Keep the adapter and specialize only the translation policy |
| Source import binding | `runtime_bindings` row adapter | Existing lexical resolver | Existing facts have the needed candidate information; no second parser is required | Keep and pass the same adapter to the oracle |
| Execution observations and input variation | `test_flow_soundness.py` | CPython `sys.monitoring`, Hypothesis | Both are used; the missing work is assertion coverage, not a missing library | Tighten the assertions and route |
| Condition Boolean kernel | Current bounded DNF | `biodivine-lib-bdd` already qualified for Stage 3 | Stage 2 only needs bounded local conditions; Stage 3 composition motivates the library | Keep Stage 3 migration; do not expand Stage 2 bespoke algebra |

## 9. Alternatives

| Alternative | Semantic duplication / locality | Bespoke code | Correctness / operations | Cost evidence | Decision |
|---|---|---|---|---|---|
| Previous Stage 2 translator | Text and line-based identity, spelling runtime view | Small | X1/X2 known false exclusions | External review P1/P2 | Rejected |
| Initial Stage 2.9 tree | Definition-set sharing plus per-site default, resolved binding and oracle | Moderate | F01–F04; counterexample excluded | Initial focused runs only | Revise |
| Library-owned path | ty facts, existing lexical rows, CPython/Hypothesis oracle; Stage 3 BDD | Adapter and domain policy only | Same intended semantics after F01–F04 fixes | No new performance claim | Selected mechanism after correction |
| Simplest viable Stage 2 close | Per-site identity by default, including singleton tests unless subject stability is proved across effects; compare full runtime tuple or leave undecided; assert every observation | Less policy than current sharing | Conservative and testable; may lower precision | No speed claim | **Recommended**; coincides with the library-owned path for this slice |

## 10. Verification plan

| Claim / risk | Label now | Check and conditions | Expected result / current gap |
|---|---|---|---|
| R1–R8 fixes survive identity migration | Implemented; prior §12 **Tested** on 2026-09-24 snapshot `162bda5a…` | Named behavior probes, analysis rules, `just test-all`, pilot on new extractor version | Need new run results; previous snapshot cannot certify version 23 |
| Evaluations do not merge mutable outcomes | **Tested counterexamples** on the initial tree (local CPython and `lctx flow`, 2026-09-24) | `isinstance` class rebinding; captured-name mutation; non-singleton identity; existing call/mutation shapes | Both `return 1` regions were `false`; F01. Corrected focused tests passed; see disposition |
| Version comparisons match CPython | **Tested CPython side**, code-inspected translator (2026-09-24) | Three- and five-element comparisons at an equal three-part prefix | Current 3-tuple model differs; F02 |
| Observed statements and definitions are admitted | Implemented oracle, limited coverage | Hypothesis shapes, injected omission and extractor-backed runtime case | Current tests may pass despite omission; F03 |
| Skips explain losses | Implemented counters | Nested resolved runtime test and exact category assertions | No nested cause case yet; F04 |
| Serving remains same-snapshot, truthfully worded | Implemented tool wording, code-inspected fate path | `get_operation` through a fresh generation; same-snapshot validator and a may-behavior assertion | Existing serving tests cover shape; pilot and evaluation still needed on hardened generation |

## 11. Authority changes and exceptions

ADR-0022 is proposed, so F01/F02 corrections belong in its §Conditions, §Places and runtime-view text and in DESIGN §3.9 before acceptance. No binding §B pivot is required by these Stage 2 findings. No SHOULD deviation needs an exception record in this scoped review.

**Deferred friction, with triggers (binding §4).**

| Item | Why deferred here | Reopen when |
|---|---|
| Raw `#module:source` suffix in served fate `condition` | Identity is preserved and no incorrect user decision was demonstrated; the service currently carries one encoded string | Agent evaluation confuses labels and IDs, or Stage 3 BDD introduces a separate display rendering |
| Prior review R10's Pyrefly-typed receivers and largest-scope/residue reports | Marked Proposed in ADR/DESIGN; outside this compact repair | A field refutation needs typed non-`self` receiver evidence, or a report consumer exists |
| Deeper memory/alias models and protocol calls | Stage 2 explicitly states primitive-operand and per-site assumptions | A Stage 3 summary or evaluation item depends on alias effects or a dunder method |

## 12. Decision

**Initial decision: Revise.** F01 and F02 violated the intended conservative may-analysis with observed/runtime-semantic counterexamples. F03 prevented the new oracle from certifying that invariant across all observations or resolved runtime decisions. F04 made the diagnostic count misleading. The corrected-tree decision follows below.

| Priority | Change | Findings | Acceptance evidence |
|---|---|---|---|
| Correctness | Restrict cross-site sharing with mutation-aware proof or use per-site atoms; compare the full version tuple or leave it undecided | F01, F02 | CPython counterexamples admitted; flow shapes and extractor gate pass |
| Verification and diagnosis | Make admission assertions exhaustive for their declared scope, include runtime bindings, classify nested decisions | F03, F04 | Injected negative tests, oracle, `just test-all`, `just pilot` |
| Cost | Stage 3 BDD library remains the planned condition engine | — | Measure when Stage 3 implementation exists |

The review's supported claim is limited to this semantic bridge. It does not certify Stage 3 or the whole product's evidence closure.

### Corrected-tree disposition (2026-09-24)

This section evaluates the implementation response to F01–F04. The findings above remain as the counterexamples and correction rationale; their original gate table describes the initial tree.

| Finding | Inspected correction | Executed evidence and limit | Status |
|---|---|---|---|
| F01 | `Translator::atom` now keys every source test by evaluation site; pattern tests use their provider predicate identity. ADR-0022 and DESIGN say definition sets alone do not prove stability. The `isinstance` rebinding, nonlocal change and non-singleton identity cases are in the fixture/oracle. | `cargo test -p cpg-flow --test flow_shapes -- --skip the_flow_facts_are_pinned`: **passed 27/27**; `uv run pytest tests/scripts/test_flow_soundness.py -q`: **passed 13/13** (reported by implementation agent, 2026-09-24). | Corrected at focused scope |
| F02 | The runtime view compares the known three version fields as a proper prefix of CPython's five-field `sys.version_info`; comparisons with longer literal tuples remain undecided and this boundary is stated in ADR-0022. | Three-part version shape is in both flow fixture and deterministic CPython oracle; same focused commands **passed**. | Corrected at focused scope |
| F03 | Each observed line now requires a modeled region, and each monitored local read after a local store requires a matching use, definition and reaching row. Every seed shape runs deterministically; Hypothesis adds input variation. The developer CLI accepts validated binding spans, and the separate extractor integration test covers the lexical join. | `uv run pytest tests/scripts/test_flow_soundness.py -q`: **passed 13/13**. The CLI lane tests translation under explicit spans; it does not itself re-run lexical resolution. | Corrected within that stated boundary |
| F04 | Runtime-decision detection follows exactly the expression forms recursively lowered by `test`, rather than all AST children; nested Boolean/not cases are classified without attributing opaque-call children. | Code-inspected against `Translator::test`; `cargo test -p cpg-flow --test flow_shapes -- --skip the_flow_facts_are_pinned` **passed 27/27** for aggregate counts. A nested exact-category fixture was not run. The categories remain diagnostic and precedence-based. | Corrected by direct code reasoning; focused count test passed |

`just check` **passed**: 269 Rust tests, 93 Python tests, 7 rules, Pyrefly and ADR lint (implementation agent's executed report, 2026-09-24). `just pilot` **passed** on snapshot `33c2af49309acd623ea39a1a0a0b34f1`, generation `00125a871f42d232`, with 20/20 smoke, 69,046 reaching rows, 13,776 condition rows, 43.4 s and 4,108 MiB peak. `budget_reached` was 66 versus 65 in the earlier definition-set build: a measured one-case precision cost of safer identity, not a new false exclusion. `git diff --check` **passed** after the generated flow snapshot was inspected and accepted.

`just pilot-live` **passed** on snapshot `e96ecf2a73c45683cadb373ba963bf90`, generation `7f7c59e6dbc8076c`, with 20/20 smoke. The dated final Stage 2.9 addendum in `structured_eval_stage2_2026-09-24.md` records `just structured-eval build/generations/7f7c59e6dbc8076c 2 http://127.0.0.1:8000`: **passed** the pre-registered exit rule with 12 present, 6 partial, 2 absent of 20 positive items, none incorrect or misleading, and none of six negatives claimed. Q23.a ranked first and Q24.a third on live vectors. The evaluation covers the registered questions, not an exhaustive proof of the may-analysis.

| Gate | Corrected-tree verdict at this checkpoint | Basis / remaining action |
|---|---|---|
| G1 | Pass | Authority routes unchanged; site identity is now explicit |
| G2 | Pass | F01/F02 counterexamples and operator/tuple fixture cases pass; `just check` passed |
| G3 | Pass | Oracle asserts per observation; negative binding-span check; `just check` passed |
| G4 | Pass | Generated-program worker and explicit analyzer inputs unchanged |
| G5 | Pass | Pilot published one coherent snapshot and generation after validation |
| G6 | Pass | Site-specific lowering admits the counterexamples; condition budget remains an explicit unknown |
| G7 | Pass | Full check, fake/live pilots, serving smoke and registered Stage 2 exit rule passed |
| G8 | Pass | Library roles unchanged |
| CI-G1 | Pass | May-analysis counterexamples admitted; nested runtime skip attribution corrected; registered evaluation found no incorrect or misleading positive |
| CI-G2 | Pass in changed scope | Same-generation service route unchanged; pilot smoke 20/20 |
| CI-G3 | Pass | Deterministic seed corpus and gold isolation unchanged |

**Final principle verdicts.** The initial violations of DP-02, DP-03, DP-08, DP-11, DP-21, DP-22, DP-23 and CI-02, CI-04, CI-06 are **Satisfied** in the corrected Stage 2.9 scope by the mechanisms and checks above. The other applicable principles listed in slot 6 remain **Satisfied** within this scope. CI-11 for the entire product remains outside this compact review; within the changed serving path it is **Satisfied** by the pinned generation lookup and unchanged same-snapshot grounding route. No MUST gap or unresolved in-scope gate remains. The Stage 3 condition kernel and query service need their own review.

**Final decision: Accept for the reviewed Stage 2 scope.** F01–F04 are closed by the inspected corrections and focused tests, the full check, both pilots and the registered evaluation. The 6 partial and 2 absent positive items are explicitly recorded with Stage 3 owners; they do not violate Stage 2's pre-registered exit rule or masquerade as complete answers. ADR-0022 can be accepted. `just test-all` remains the repository's before-commit gate and is not claimed as executed by this review.
