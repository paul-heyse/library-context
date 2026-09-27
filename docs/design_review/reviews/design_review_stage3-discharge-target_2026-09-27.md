# Design review: Stage 3 discharge, non-value channel representation and component driver (target)

**Date:** 2026-09-27 · **Reviewer:** design-reviewer subagent (fresh context; not the author of the
proposal) · **Tier · purpose:** design · target (binding "Reviews in this repository": a changed
ownership/dependency boundary before P1).
**Standard:** core 3.0 (repository-owned, ADR-0040), code-intelligence profile 1.1, library-context
binding. **Revision:** `main` at `ddcc149`, clean tree.

This review is evidence, not authority. Findings keep the IDs below; current disposition belongs in
the forward plan §6 once scheduled (binding §4).

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Three operator-approved, unimplemented targets from forward plan §3.0 "Remaining sequence": **A** S5a discharge (P1), **B** non-value channel representation (P3), **C** generic component driver (P4.1). Code read as the baseline they change. |
| Maturity and outcome | Stage 3 functionally incomplete. The decision should let P1 start without later reopening the behavior↔summary boundary, and fix P3/P4 contracts before code. |
| Supported scope | Value-channel discharge of `call_transfer` on `returns` claims whose contributions are own-function parameter origins (the only origins with `summary_boundary_candidates`, `cpg-schema/src/behavior.rs:2954–2972`); refutation deferred to P4; effect/callback/resource representation and callable-level coverage; the shared SCC driver. Excluded: `derives`/`stores` discharge (plan §7), exception channel, Stage 5 deferred execution, override dispatch. |
| Adjacent consumers | `cpg-core::attempt` (order/publication), `cpg-core::validate` (reconstruction), `rules.rs` semantic rules, FORMAT 9 generation (`bundle.rs`), native `lctx_semantics` (loads summaries, not behaviors), `lctx_mcp` (serves `behaviors` from the generation, `generation.py:1014`). |
| Expected changes | Add refutation (P4); add the effect channel (P3/P4); add `derives`/`stores` discharge (§7 trigger); add a second engine (S4 comparison); a decorated callee in a wrapper; test the grading in isolation; trace a served discharged claim. |
| Baseline | Every `through_call` value flow becomes `unknown`/`call_transfer` unconditionally (`behavior.rs:991–995`, duplicated for field-mediated rows at `1200–1203`); `semantic:call-transfer-never-established` rejects any other verdict (`rules.rs:3341–3348`). `behavior::run` executes before summaries (`attempt.rs:796` vs `1143`); behaviors are written after them (`1189`). |
| Method and coverage | Read: plan §1–§3.0, §6–§7; ADR-0057/0058; §9.9 "Transfer summaries" and "Channel composition contracts"; §3.9 "Value flows and call transfers", "Verdicts"; `behavior.rs` (whole), `attempt.rs:760–1260`, `cpg-core/src/summaries.rs`, `validate.rs:60–175, 589–800`, `flow_model.rs:318–360, 1052–1089, 1700–1857`, `finite.rs:1–480, 516–815, 1345–1470, 2100–2430, 2643–2763`, `worklist.rs`, `actions.rs:1–80`, `summary_contract.rs:1–320`, `cpg-schema/src/behavior.rs` summary/flow tables and seed SQL, `action.rs` tables, `bundle.rs:290–400`, native `ipc_input.rs:18–60`, `lib.rs:95–125`, `call_binding.rs:117–127`, `rules.rs:3320–3425`, the `transfer_alternatives` fixture and its compile test. Three read-only pilot queries (§10). Not examined: native proof decoding internals, Python rendering code beyond hydration, Stage E consumers of `behaviors`. |

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility and hidden decisions | Consumer contract | Allowed dependencies | Expected reason for change |
|---|---|---|---|---|
| `cpg-schema` (`behavior.rs` tables/SQL, `summary_contract.rs`, `rules.rs`, codebooks) | Meaning: relation schemas, append-only codebooks, shared proof admission, SQL invariants | Arrow contracts; `admit_callee_proof`; `coverage_domain` | none upward | New subject kinds, step kinds, discharge relation, rule replacement |
| `cpg-core::flow_model` | Builds `value_flow_contributions` and merges them into display `value_flows` per `(sink key, origin, transfer)` (`flow_model.rs:1711–1857`, `sink()` at `1083–1089`); owns the private `decorated_functions` predicate (`318`) | `FlowModelRows` in memory; persisted contributions/value flows | schema, providers' facts | Membership export (F01); decorated predicate reuse (F02) |
| `lctx-analytics::summaries` (`finite.rs`, `worklist.rs`; proposed `discharge`) | Pure composition of finite value paths, refusals, boundaries, origin coverage; SCC worklist | `FiniteSummaryOutcome` | schema only | Discharge decisions, effect adapter, driver extraction |
| `cpg-core::behavior` | Today: fetches five relations, runs Pass B surface, **authors the flow-claim verdict policy** (overrides, claim merge, post-passes), operations, facets, documents and embedding | `BehaviorRows` | session, flow model, analytics | Split into relations/claims/facets/documents; applies discharge |
| `cpg-core::attempt` | Phase order and publication | one snapshot | all core | New order: … → finite → decide → claims → facets/documents → writes |
| `cpg-core::validate` | Reconstruction equality (reruns `flow_model::run` at `771`, `finite_flows` at `592`) plus SQL rules | violations before `snapshots` append | core producers | Discharge equality; rule replacement |
| Native `lctx_semantics` | Admits summary proofs, path-local `inspect_value_paths`; does **not** load `behaviors` (`ipc_input.rs:18–47`) | FORMAT 9 summary files | schema | Discharge closure check |
| `lctx_mcp` | Serves `behaviors` rows as-is from the generation | MCP responses | generation | Render discharge evidence |

| Concept | Authority today | Update boundary | Derived forms and consumers |
|---|---|---|---|
| Which contributions a flow behavior claims | **Implicit**: `flow_model` merge by `(sink key, origin, transfer)`, then `behavior.rs::claim` merge by `flow_behavior_id` (`859–874`); nothing persists the link | compile only | none; the SQL rule, validator and serving would each have to re-derive it |
| An origin is "proved" | Three views over one outcome: `summarize_boundaries` (proved ∧ ¬refused, `finite.rs:257–293`), `origin_coverage` (call origins never complete, `2711–2719`), proposed discharge (∃ established/conditional summary) | analytics | boundaries, coverage, (discharges) |
| A callable is "decorated" (binding may be replaced) | `flow_model::decorated_functions` (private, core) | compile + validation rerun | behavior post-pass only; **not** the summary producer |
| Valid coverage subject/channel pairs | SQL checks with literal codes (`behavior.rs:1545–1546`) **and** `summary_contract::coverage_domain` (`48–59`), the latter consumed only by a test (`completion.rs:2419`) | schema | publication checks; no production consumer of `coverage_domain` |
| Call-transfer verdict rule | `behavior.rs:991` (+ copy at `1200`), `rules.rs:3341`, §3.9 prose | ADR + section | behaviors, served verdicts |

**CI fact and fidelity table.**

| Fact family | Provider | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| `value_flow_contributions` | `cpg-core::flow_model` over ty flow facts | derived | `approximated`, `captured`, `local/upstream_through_call` flags; reach budget via `flow_reach_boundaries` | `origin_id` (content hash, ADR-0054) | summary seeds/candidates, validation |
| `value_flows` | same | derived display merge | merged flags; non-stored definitions omitted | composite key, no id | behaviors, Pass B |
| `summary_flows`/`_steps` | `lctx-analytics::summaries::finite` | derived, may-flow under the flow-IR model | refusals → `summary_boundaries`; approximated paths refused unless certified (`finite.rs:1066, 1720`) | `summary_id` over ordered proof | native inspection; proposed discharge |
| `summary_origin_coverage` | same + completion | derived coverage | call origins never complete today | subject/condition/channel/phase | none served yet |
| `behaviors` | `cpg-core::behavior` | derived claim with verdict | `boundary_reason`; no evidence link to contributions/summaries | `flow_behavior_id` (transfer and scope included; verdict excluded) | MCP, facets, Stage E |
| `behavior_discharges` (proposed) | discharge decision | derived | open reason / proved summary | proposed `origin_id` key | served evidence, rules |
| `summary_effects`/`_steps` (proposed) | effect adapter over admitted action assessments | derived, potential/definite | subject kinds Unqualified/EntryFormal/Unresolved | proposed recipe | P6 operation semantics |

## 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs | Invariants and enforcement | Effects/failure | Isolated verification |
|---|---|---|---|---|
| A: `discharge::decide` (proposed) | contributions/candidates + `FiniteSummaryOutcome` → per-origin decision | proved iff an E/C summary cites the origin; pure | none | store-free unit tests (good) |
| A: per-behavior grade (proposed inside core `claims`) | member origins + decisions + raw flags → verdict/reason | "flip only if every contribution proved"; overrides after | none | **only through a full compile** today (F05) |
| A: `behavior_discharges` (proposed) | `(origin_id, decision, summary_id?, coverage subject?, reason?)` | validator equality per origin; SQL rule over behaviors; native summary presence | published, served | SQL/validator; **no behavior link** (F01) |
| B: `summary_effects` + steps | admitted assessments/postconditions → effect summaries with subject, trigger, modality, pending outcome, condition, verdict, depth | ADR-0058 typed subjects; step kinds 39–41 appended | published | pure producer tests |
| B: CallableFormal/CallableEntry coverage | callable, formal, channel → complete/empty | "every return admitted; returned values closed constants or fully-covered formal contributions" | published | pure completion tests; **premise insufficient** (F08) |
| C: `drive_component<A: Adapter>` | component edges, witnesses, adapter → admitted rows, refusals | driver: queue, charge, frontier, progress; adapter: applicability and semantics | none | existing worklist tests + adapter tests |

**Absence lattice for a call-transfer claim.** Not a candidate (field/captured/yield origins: no
decision row, must stay `call_transfer`), open with a specific summary refusal, proved, refuted
(P4), budget-cut. The proposal distinguishes these per origin. At the served behavior they all
collapse to `unknown`/`call_transfer` unless the behavior can reach its discharge rows (F01).
Keeping the claim-level reason `call_transfer` preserves pilot metric continuity. Origin-specific
reasons belong in the discharge relation.

## 4. Composition and execution

| Stage (proposed order) | Semantic inputs/outputs | Owner | Policy vs orchestration | Cost observed (pilot log, 2026-09-27) |
|---|---|---|---|---|
| flow model | facts → contributions, value flows, decorated set | core `flow_model` | semantic (core-owned today) | 25.20 s |
| relations | `argument_flows`, guards, reads, handoffs, delegations | SQL in `cpg-schema` | mechanical | 1.85 s |
| summaries | seeds, candidates → flows, boundaries, coverage | analytics | pure | finite composition falls inside the 6.72 s "write source_modeled_identities" mark |
| decide (new) | outcome → per-origin decisions | analytics (proposed) | pure | negligible (joins by origin) |
| claims | Pass B + flow claims + discharge grade | core `behavior` | **verdict policy in core** (F05) | Pass B 1.55 s |
| facets/documents | behaviors → facets; texts → documents, embedding | core | mechanical + I/O | 1.92 s + 0.31 s |
| validate | reconstruction + rules | core | shared validators | 107.25 s total; slowest SQL rule 0.40 s |

The 107 s validation is dominated by Rust reconstruction reruns (flow model, execution/contexts,
summaries), not SQL rules. This attribution is an inference from the stage and rule timings. The
proposed discharge equality reuses the `validate_summary_flows` outcome and adds negligible cost. A
membership check must reuse the single validation-time `flow_model::run` (`validate.rs:771`) and must
not rerun Pass B.

**CI analysis record: discharge.** Question: does every contribution of this claim have a proved
transfer through its call? Projection: own-function parameter-origin contributions at `Return`
sinks (`summary_boundary_candidates`). Method: exact join; no new BDD work (the summary's
`condition_id` is the contribution's, `finite.rs:1409–1442`). Model: may-flow under the flow IR plus
summary proofs (ADR-0057). Budgets: inherited summary caps; a capped origin stays open with its
reason. Output: served verdict plus the cited summary. That citation is missing in the proposed
keying (F01).

**Ordering.** Only `argument_flows` is needed before summaries (`local_call_summary_flow_seeds` and
`local_call_arguments` list it among their dependencies, `cpg-schema/src/behavior.rs:2803–2807, 2898`). No summary input
reads `behaviors`, `operations` or facets, so the new order has no cycle. A `relations()` API is
not required. Writing `argument_flows` (and the other four) with the existing
`write_analysis_query` pattern and letting `claims` read them from the session is the simpler
realization (observation O2).

## 5. Change and failure scenarios

| Scenario | Owner | Contract change | Expected vs observed propagation | Independent edits / hidden knowledge | Evidence |
|---|---|---|---|---|---|
| **Trace a served discharged claim** (CI journey) | serving | behavior → evidence | Expected: behavior row → discharge rows → summary → steps → facts, in one generation. Proposed: discharges keyed by origin; Python serves `behaviors`; native loads summaries only. The behavior→origin link must be re-derived from `value_flows` key + `flow_values` spans + the `claim()` merge rule | Re-deriving membership in SQL, Python and native, from display `value` text for returns (`"computed:<start>"`) and overwritten sink spans for arguments (`behavior.rs:1386–1391`) | F01; code inspection |
| **A decorated callee in a wrapper** (`transferpkg.through_decorated` → `@replace decorated`) | summaries | none, if the producer refuses | Expected: stays unknown. Observed route: local seed SQL has no decorator predicate (`behavior.rs:2802–2893`); worklist has none (`finite.rs:2159–2410`); `call_binding` checks decorators only for fresh nested defaults (`call_binding.rs:117–127`); the behavior decorated override reaches operation/scope and **hops**, and `returns` rows get no hops (`behavior.rs:1259–1279, 1291–1296`). The existing test checks kinds 0/4 for `through_decorated`, not `returns` (11) (`tests/compile.rs:211–221`) | Discharge would be the first consumer to turn such a summary into a served established claim | F02; unresolved: whether Pysa's target for the decorated call matches the seed is not established |
| **Add refutation (P4)** | analytics + schema rules | `refuted` decision with evidence; generalize `semantic:refuted-needs-complete-region` (`rules.rs:3382–3389`), which today requires a `negative_premises` place key | Expected: new decision kind + ordered refutation evidence + one rule generalization. Proposed single optional "coverage subject" cannot cite several callee coverages (`g(x, f(x))`: one contribution per use, possibly several calls) | Rule change not named in the proposal | F07; F08 for the coverage premise |
| **Add `derives`/`stores` discharge** (§7; pilot: 4,619 derives + 344 stores of 6,353 `call_transfer`) | analytics | per-origin proof at non-return sinks: the callee's formal→return summary + binding at the inner call, not a caller summary | Proposed `decide` hardcodes "same `source_origin_id` in `summary_flows`", which only exists for return contributions. `claim()` merges several argument-sink value flows into one behavior (first-inserted verdict wins) | decide, grade and closure checks all change | F03, F07 |
| **Add the effect channel** | analytics + schema | `summary_effects`, steps 39–41, `CalleeProofTarget.channel`, subject kinds | Mostly additive under ADR-0058. Coverage subject/channel pairs must be edited in two authorities; `admit_callee_proof` accepts any non-control step kind (`summary_contract.rs:319–320`) | Two authorities for pairs | F09, F10 |
| **Second engine** (Ascent/datafrog comparison) | analytics | none if the adapter is engine-neutral | A driver extracted from one consumer risks baking in value-only assumptions: formal-keyed dependents, `SummaryFlowsRow` witness store, `call_transfer` as the unexamined fallback (`finite.rs:2161–2166, 2403–2405`) | Adapter shape | F11 |
| **Test the grading in isolation** | core today | — | Expected: typed inputs, no store. Observed: the verdict policy (overrides, merge, post-passes) lives inside the async `behavior::run` that needs a session, Pass B and an embedder | Full compile per case | F05 |
| **Certified approximate modeled return** (`return typing.cast(T, x)` in a `try` region) | core behavior | — | Proved summaries exist for raw-approximated origins only when certified (`finite.rs:1066, 1720`). The proposal re-applies `v.approximated → missing_evidence` after the flip (`behavior.rs:1000–1003`), which silently reverses the discharge for this class | Contradicts the certificate semantics in the channel review: a certificate discharges only the matching origin's approximation obligation (lines 688–690, 1171–1172); uncertified approximation stays open (616–619) | F04 |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | unresolved | Behavior→contribution membership is implicit and would be re-derived by claims, the rule, native and Python (F01). Subject/channel pairs have two authorities (F09). The call-transfer verdict rule has three sites (`behavior.rs:991`, `1200`, `rules.rs:3341`), and the proposal adds a rule without naming the replacement (F06) | F01, F06, F09 |
| G2 Semantic fidelity | unresolved | Decorated callees are not excluded where discharge would read them (F02). The merge join is undefined (F03) | F02, F03 |
| G3 Validity | unresolved | Validator equality per origin + SQL rule are sound. Membership completeness has no enforcement point unless a sibling-closure rule is added (F01, F06) | F01, F06 |
| G4 Hidden behavior | pass | `decide` is pure; splitting documents/embedding out of claims isolates the only I/O in this stage | — |
| G5 Consistency and recovery | pass | Discharges are computed, validated and published in the same attempt before the `snapshots` append (`attempt.rs`, `validate.rs:92–137`) | — |
| G6 Transformation and reuse | unresolved | `claim()` keeps the first-inserted verdict/boundary and ORs conditions (`behavior.rs:868–871`). The discharge join is defined per value flow, not per merged claim | F03 |
| G7 Truthful capability | pass, conditional | Nothing claims P1 reduces pilot `call_transfer`. **Measured:** it cannot at this checkpoint (O1). Appending `refuted` or a coverage column before P4 would widen the surface ahead of a producer | F07 |
| G8 Library leverage | pass | Discharge is domain joining; the bespoke worklist is ADR-0053's deliberate choice with a scheduled engine comparison | — |
| CI-G1 Fidelity | A: unresolved; B: **fail as written** | A: F02, F04. B: the CallableFormal "fully-covered formal contributions" branch certifies empty coverage for a returned object that contains the formal (F08) | F02, F04, F08 |
| CI-G2 Evidence closure | **fail as proposed** | A served established `returns` claim cannot reach its summary without re-deriving membership (F01) | F01 |
| CI-G3 Evaluation integrity | pass | Acceptance uses mechanical fixtures (`local_wrapper`, `labelled`, part-2 wrappers). Evaluation-only requests and the frozen rule are unchanged; no gold path is involved | — |

## 7. Findings

| ID | Finding and scenario consequence | Principles · judgment/gate | Evidence/gap | Correction and owner | Closure evidence | Disposition |
|---|---|---|---|---|---|---|
| <a id="F01"></a>F01 · **high** | **Discharge evidence is keyed by origin, not by claim.** A served established `returns` behavior cannot cite the summary that discharged it without re-deriving membership. For returns that means parsing `value` text; for arguments the sink span is overwritten by the call-site span. The claims step, the SQL rule, the native check and Python would each rebuild the same join | CI-11, DP-01, DP-07, FP-04 · A2, G1, G3, CI-G2 | `behavior.rs:859–874, 970–987, 1386–1391`; `bundle.rs:297–327`; `ipc_input.rs:18–47`; `generation.py:1014` | Key `behavior_discharges` by `(snapshot_id, behavior_id, origin_id)`. Columns: decision, typed proof kind, `summary_id` when proved, reason when open. The membership comes from the owner of the merge: `flow_model` exposes `value_flows` key → member origins, and `claim()` accumulates members across merged rows. Python renders discharge citations. Native checks that each cited summary exists, has the same `source_origin_id`, is established/conditional and is not approximated. Add a SQL sibling-closure rule: for a behavior with a discharge row for origin *o*, every contribution sharing *o*'s `flow_values` sink key, `source_key` and transfer also has a row for that behavior | The served `local_wrapper` claim shows its summary id. A doctored generation with a missing or foreign summary is refused. The sibling-closure rule rejects an injected omission | plan §6 (P1) |
| <a id="F02"></a>F02 · **high, unresolved** | **Composition through a decorated callee is not refused where discharge will read it.** Discharge would serve `through_decorated` returns `value` as established through `@replace`, which replaces the binding. The behavior-layer override does not reach `returns` rows (no hops) | CI-06, CI-02, DP-08 · G2, CI-G1 | Local seeds `cpg-schema/src/behavior.rs:2802–2893`; worklist `finite.rs:2159–2410`; `call_binding.rs:117–127`; `behavior.rs:1259–1279, 1291–1296`; `tests/compile.rs:211–221` | The summary producer refuses a local target in the decorated set as `outside_provider_model`, per origin. Use one predicate: `flow_model::decorated_functions` including the descriptor exemption, passed as explicit input or published. This also protects native `inspect_value_paths` | `through_decorated` has no `summary_flows` row and its kind-11 behavior stays unknown. A wrapper of `Descriptors.make` still composes. Whether a summary exists today is `blocked` (§10) | plan §6 (P1) |
| <a id="F03"></a>F03 · medium | **The claim merge join is undefined.** `claim()` keeps the first-inserted verdict/boundary and ORs conditions. Grading "every contribution proved" per value flow before the merge makes the result depend on insertion order when merged members differ. This is latent for P1 returns (one value flow per claim) and live for the §7 derives extension | DP-08, DP-11 · G6, A2 | `behavior.rs:859–874, 1009` | Carry member origins and per-member grades in the stage-2 map. One pure join: positive only when every member is proved; otherwise unknown with a deterministic reason precedence. Apply it after the merge | A pure test with two merged members of different grades in both insertion orders gives an identical row | plan §6 (P1) |
| <a id="F04"></a>F04 · medium | **Override precedence contradicts certified approximation.** The proposal keeps `approximated → missing_evidence` after the flip. Proved summaries exist for raw-approximated origins only when a certificate discharged that origin's approximation (the modeled-identity case is call-crossing). The flip is therefore reversed exactly where certificates were built to help | CI-06, DP-02 · A2, CI-G1 | `behavior.rs:1000–1003`; `finite.rs:1066, 1720`; channel review lines 688–690 and 1171–1172 (certified origin), 616–619 (uncertified stays open) | For a fully proved claim, the proofs discharge the members' value approximation; raw flags are unchanged. Condition approximation, captured, decorated and unreachable post-passes remain | A certified modeled return in an approximate region (existing modeled-identity fixture) becomes established. An uncertified approximated sibling stays unknown | plan §6 (P1) |
| <a id="F05"></a>F05 · medium | **Grading lives in core's 1,700-line async stage.** Verdict policy for flow claims (overrides, merge, post-passes, and now discharge) is testable only by a full compile with a store, Pass B and an embedder | FP-06, FP-01, DP-17 · A1 | `behavior.rs:264–1718`; plan "pure semantics in analytics" | Pure `decide` and `grade(members, decisions, flags) → (verdict, reason)` in `lctx-analytics::summaries::discharge`, or decisions returned in `FiniteSummaryOutcome` beside boundaries/coverage. Core supplies membership and applies the result. Deduplicate the `991`/`1200` override blocks through the same function | Store-free tests for proved/open/mixed/certified/decorated inputs; one source fixture for integration | plan §6 (P1) |
| <a id="F06"></a>F06 · medium | **Authority changes are unnamed.** `semantic:call-transfer-never-established` rejects every discharged row and must be replaced, not supplemented. §3.9 states the unconditional rule. Three origin-level "proved" meanings coexist without a single definition | DP-01, DP-24, FP-04 · G1, A2 | `rules.rs:3341–3348`; `behavior-model.md:325–331`; `finite.rs:257–293, 2711–2719` | ADR + §3.9/§9.9 amendments. Replace the rule with "Call-transfer E/C ⇒ ≥1 discharge row and all proved". Document the positive (discharge), residual (boundaries) and completeness (coverage) views in `summary_contract` | One call-transfer rule remains. Injected violations: established without discharges, a foreign summary, an open sibling | ADR (§11) |
| <a id="F07"></a>F07 · medium/low | **The decision contract is return-specific and refutation evidence is under-shaped.** Derives/stores (78% of pilot `call_transfer`) need a different proof object. A single optional "coverage subject" cannot cite several callee coverages | DP-16, FP-03 · A3, G7 | Measured pilot counts (§10); `cpg-schema/src/behavior.rs:2954–2972` | Add a typed `proof_kind` codebook now, with only `caller_return_summary`. Do not append `refuted` or a coverage column until P4, which records refutation as ordered evidence steps and generalizes `semantic:refuted-needs-complete-region` | Traced extension: an argument-sink proof kind changes producer, codebook and closure check, not `grade`/`claims` | plan §6 (P1/P4) |
| <a id="F08"></a>F08 · **high (P3/P4)** | **CallableFormal complete-empty value coverage lacks an escape premise.** For `def f(p, acc): acc.append(p); return acc`, p has no return contribution and `acc` is a "fully-covered formal contribution", so coverage is empty-complete. With P5's effect-only `list.append`, P4 would refute `g(x): return f(x, [])`, which returns `[x]` | CI-04, CI-06, DP-03 · CI-G1 | Plan P3 text; §3.9 "containers computed from operands"; §7 points-to deferral | Restrict P3 to closed immutable constant returns plus None fallthrough (sufficient for `_describe`/`labelled`). Otherwise add a no-escape premise: the formal has no argument, definition, captured or raise contribution except to callees with complete-empty coverage and no mutation effect. State it in `coverage_domain`; opaque or unresolved calls block completeness | The counterexample fixture stays incomplete; `_describe.name` is complete-empty | plan §6 (P3) |
| <a id="F09"></a>F09 · low/medium | **The subject/channel validity has two authorities.** SQL checks with literal codes and `coverage_domain` (test-only consumer) would both need edits for CallableFormal/Entry | DP-01 · G1 | `cpg-schema/src/behavior.rs:1545–1546`; `summary_contract.rs:48–59`; `completion.rs:2419` | Generate the checks from `coverage_domain` over codebook `all()`. Refutation and native consumers call `coverage_domain` before using completeness | A pair added on one side fails a test | plan §6 (P3) |
| <a id="F10"></a>F10 · low | **Effect subject and step-kind typing.** "Unresolved" conflates an unknown required binding with a resolved non-entry (local/fresh) subject when an EntryFormal is not rewritten. The shared step codebook has no per-channel admissible set (`admit_callee_proof` passes other kinds) | DP-02, DP-03 · G2 | `summary_contract.rs:319–320`; ADR-0060 subjectless vs unresolved | Add a distinct internal/call-local subject (or an explicit non-lifted status). Declare admissible step kinds per channel in the schema and use them in native and shared admission | Native rejects a value proof containing kind 41; typed rows for a local subject | plan §6 (P3) |
| <a id="F11"></a>F11 · low | **The driver extraction precedes its second consumer.** The value loop's formal-keyed dependents, `SummaryFlowsRow` witness store and `call_transfer` fallback would become driver assumptions | FP-02, DP-16 · A3 | `finite.rs:2159–2410`; `worklist.rs` | Extract `drive_component` in the same slice as the effect adapter. The driver owns queue, charge, `current`, progress, cap and unexamined bookkeeping. The adapter owns applicability subject, apply → admit/refuse/skip, and the fallback reason (effects: from CallableEntry coverage, never "empty") | Both adapters run through one driver; value outputs are byte-identical; the engine comparison drives the same adapters | plan §6 (P4) |

**Observations (not findings).**
- **O1 (Measured, 2026-09-27):** P1 cannot move the pilot `call_transfer` count at this checkpoint.
  All 26 pilot summaries are depth 0 on non-call origins; 3,281 candidate origins remain
  `call_transfer` boundaries. P1 acceptance is fixture-level; pilot movement depends on P4/P5
  producers. STATUS and plan wording should not imply otherwise.
- **O2:** Use `write_analysis_query` for the five relations in place of a new `behavior::relations()`.
- **O3:** Keeping `transfer = Call` keeps claim identity stable across discharge, which matches
  `flow_behavior_id`'s rule that verdicts stay outside identity. The `"computed:"` value text of a
  claim proved as identity is presentation; the served discharge should expose the summary kind.
- **Satisfied as proposed:** pure `decide` in analytics; decisions validated from the existing
  finite outcome with no second producer run; transfer unchanged; refutation deferred to P4;
  value/transform/constant kept in `summary_flows` with separate typed effect tables (ADR-0058
  forbids fabricated value IDs); coverage reused through typed subject kinds.

## 8. Library fit and total complexity

| Capability | Consumer | Candidates | Fit | Burden | Choice |
|---|---|---|---|---|---|
| Discharge decision/grade | served behaviors | plain Rust joins; Datalog (Ascent/datafrog) | exact keyed joins, no recursion | Datalog adds a rule layer for a non-recursive join | Plain Rust in analytics |
| SCC fixed point per channel | value + effect summaries | ADR-0053 worklist; Ascent 0.8.1; datafrog 2.0.1 | worklist conforms today; the comparison is scheduled on the shared contract | a generic driver adds one trait; engines add build/lifecycle | Keep ADR-0053; extract the driver with its second consumer (F11); compare engines on the same adapters |
| Condition handling in discharge | — | biodivine (existing) | discharge needs identity equality only (the summary's `condition_id` is the contribution's) | none | No new condition work |

## 9. Alternatives and tradeoffs

| Alternative | Propagation and local reasoning | Authority and composition | Test boundary | Machinery and risk | Decision |
|---|---|---|---|---|---|
| Baseline (unconditional `call_transfer`) | none | consistent but never uses L3 | — | none | Rejected: blocks the stage exit |
| Proposed (per-origin decide, all-members flip, behaviors after summaries) | one new dependency: behavior → L3 | sound after F01–F06 | pure `decide`; grade in core (F05) | small | **Adopt with corrections** |
| Split claims per contribution (proved rows established, open rows unknown) | identity churn (origin in id), more served rows | more precise for mixed rows | same | medium | Rejected for P1. Revisit if a graded item is partial solely because of mixed rows |
| Serve-time discharge (Python/native joins behaviors with summaries) | semantics in serving | second interpreter (T7) | server setup | high | Rejected |
| SQL-derived discharge updating behaviors | verdict policy split between Rust and SQL | two classifiers | SQL only | low | Rejected |

## 10. Verification and uncertainty

| Claim | Label/date | Check | Outcome |
|---|---|---|---|
| Pilot summaries cover no call-crossing origin | Measured 2026-09-27 | `target/release/lctx query --store build/store-stage3-checkpoint-2026-09-27 --snapshot 8b4fb9ab6dcaa3698c27018593553129 "SELECT s.path_depth, c.through_call, c.local_through_call, c.upstream_through_call, count(*) … FROM summary_flows s JOIN value_flow_contributions c ON c.origin_id = s.source_origin_id GROUP BY …"` | **passed:** 26 rows, depth 0, all transfer flags false |
| `call_transfer` behaviors by kind | Measured 2026-09-27 | same store: `SELECT kind, depth, count(*) FROM behaviors WHERE boundary_reason = 19 GROUP BY 1,2` | **passed:** derives 4,034 (d1) + 585 (d2); returns 1,235 (d0) + 139 (d2); stores 303 + 41; raises_when 16; total 6,353 |
| Summary boundary reasons | Measured 2026-09-27 | same store: `SELECT reason, local_through_call, upstream_through_call, count(*) FROM summary_boundaries GROUP BY …` | **passed:** `call_transfer` 3,281; `unsupported_control_flow` 380; `outside_provider_model` 23; `default_stability_unknown` 10; `scope_boundary` 8 |
| Stage costs | Measured (historical receipt 2026-09-27, author's pilot run) | `/tmp/lctx-stage3-checkpoint-pilot2.log` stage table | flow model 25.20 s; relations 1.85 s; Pass B 1.55 s; validation 107.25 s; slowest SQL rule 0.40 s |
| F02: a summary is produced for `through_decorated` | unresolved | needs a fixture compile with a new assertion; no read-only route without adding test code (`lctx compile-fixture` is P2) | **blocked** (missing prerequisite: a fixture compile entry point or test edit) |
| Integrated gates | — | `just test-all`, `just pilot` | **not_run**: a design review with no code change; end-of-scope acceptance per AGENTS.md |

Known-answer shapes for the P1 slice: parallel origins on one raw fact (`mixed_origin`), a
decorated callee (`through_decorated`), a certified approximate modeled return, a merged
argument-sink claim in both insertion orders, and a doctored generation citing a foreign summary.

## 11. Authority changes and dispositions

| Required change | Route and owner | Findings | Disposition location | Closure |
|---|---|---|---|---|
| **New ADR: discharge of call-transfer claims.** Behaviors are graded after summaries. `behavior_discharges` is keyed by claim and origin with a typed proof kind. All-members join. Proofs discharge value approximation. Decorated-callee refusal. Rule replacement. Refutation deferred with its evidence/premise route | `just adr new`; §3.9 "Value flows and call transfers", §9.9 "Transfer summaries: Discharge", "Native serving boundary" | F01–F07 | ADR + plan §6 rows | ADR accepted; code closure by the P1 change review |
| §9.9 "Channel composition contracts": CallableFormal/CallableEntry domains with the escape premise; `coverage_domain` as the single authority; internal subject kind; per-channel step admissibility | Section amendment under ADR-0058 (no new ADR) | F08–F10 | plan §6 (P3) | P3 fixtures |
| §9.9 "Bounded progress ownership": driver/adapter split | Implementation under ADR-0053/0057 (no ADR) | F11 | plan §6 (P4) | P4 change review |

**Is a new ADR required?** Yes, for A only. The §9.9 "Discharge" bullet and ADR-0057/0058 accept the
target that a matching proof may discharge a claim. They do not decide the choices listed in the
ADR row above. Those are real alternatives, reverse a stated §3.9 rule and a published semantic
rule, and change the §3.9 → §9.9 dependency direction (AGENTS.md "Write an ADR"). B stays inside
ADR-0058 and C inside ADR-0053, each needing only the section amendments above. P4's call-specific
stability ADR remains separate.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | unresolved | The pure `decide` localizes the decision. Grading and override precedence stay in core's session-bound stage, so the served-claim join and the derives extension edit core internals | F05, F03 |
| A2 Encode meaning structurally | violated (as proposed) | Claim↔origin membership is implicit (F01). The call-transfer rule has three sites and one is not replaced (F06). Subject/channel validity has two authorities (F09) | F01, F06, F09 |
| A3 Extend through composition | unresolved | Refutation, derives/stores and a second engine each need contract shape decided now (typed proof kind, ordered refutation evidence, engine-neutral adapter) | F07, F11 |

**Bounded decision: accept with corrections.** In template terms this is Revise at the contract level,
with the direction accepted and no re-review needed if the corrections are adopted as stated.
- **A:** P1 implementation may start once the new ADR and section amendments record F01–F06, with
  F07's typed proof kind. F02 needs its fixture control before any discharged established claim is
  published.
- **B:** P3 must adopt F08 before code; F09 and F10 in the same slice.
- **C:** accepted scoped under F11. Extract with the effect adapter in P4.1, not before.

**Enclosing architecture:** Stage 3 remains functionally incomplete and unassessed as an assembled
whole. This target review certifies neither P1–P4 implementation nor the S8 exit, and says nothing
about release qualification.

| Priority | Change and responsible component | Findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Discharge ADR + §3.9/§9.9 amendments; replace `semantic:call-transfer-never-established` | F01, F04, F06, F07 | ADR accepted; rule injected-violation tests |
| 2 | Summary producer refuses decorated local targets (one predicate) | F02 | `through_decorated` control |
| 3 | Claim-keyed `behavior_discharges`, membership from `flow_model`, merge-aware pure grade | F01, F03, F05 | served citation; order-invariance test; native closure refusal |
| 4 | CallableFormal escape premise; generated coverage checks; subject/step typing | F08, F09, F10 | counterexample fixture; generated checks |
| 5 | `drive_component` with the effect adapter | F11 | two adapters, byte-identical value output |

Next step: the author writes the discharge ADR (owner: `cpg-schema` meaning + `lctx-analytics::summaries`
decision; `cpg-core` orchestration). The P1 change/conformance review then checks F01–F06 against
the implemented slice.
