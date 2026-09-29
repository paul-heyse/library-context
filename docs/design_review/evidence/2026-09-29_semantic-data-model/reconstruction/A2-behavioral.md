# A2 — Behavioral reconstruction (Phase 1, design/target review)

- **Code baseline:** `35afc09`, with `crates/` and `python/` clean (see `baseline.txt` in this folder).
- **Method:** source reading only. Nothing was built, tested or run, and no repository file was edited.
- **Labels:**
  - **Implemented** means read in the source at the baseline.
  - **Interface-checked** means inferred from signatures, SQL or test bodies without executing anything, or a behaviour derived by reasoning over the code.
  - Nothing here is labelled Tested.
- **Row counts:** quoted only from `docs/design_review/evidence/2026-09-28_pr5/raw/behavioral-current-status.json`. That file was written 2026-09-29 01:01 by the concurrent docs session, after this baseline. It gives the served PostgreSQL relation counts for the FastMCP behavioral generation. It is reported, not re-run.
- **Citation form:** `path::symbol:line`. Paths are relative to `/home/paul/library-context`.

---

## 0. Decision-relevant headlines

1. **Real fixpoint engines: two, not twelve.**
   - Only the source-reach solver (`crates/cpg-core/src/flow_model.rs::Model::solve_reach:957`) and the finite local-call worklist (`crates/lctx-analytics/src/summaries/finite.rs::finite_flows_with_pair_limit:2200-2451`) iterate to a fixpoint.
   - They disagree on the three things a generic driver would have to own:
     - **Merge:** OR-join of conditions versus keeping distinct witnesses under dominance.
     - **Cap semantics:** widen to unknown versus refuse the whole component.
     - **Key/witness split:** none versus a semantic key plus `summary_id`.
   - The other ten engines are BFS with first-path-wins, recursive exact interpreters, a memoized DAG translation, a chain validator, a fixed two-stage pipeline, and a bounded SQL ancestor climb.
   - A shared *driver* is not justified by current code. Shared *components* are (§2.3):
     - one refusal-set with a declared priority;
     - named budgets;
     - reuse of the SCC schedule for source-call normals, which `execution.rs` currently hard-codes as one level.
2. **Four independent boundary-reason policies, with a concrete divergence (Interface-checked).** `summary_boundaries.reason` and `summary_origin_coverage.reason` can disagree for the same origin. Take a local-call seed refused `missing_evidence` with `local_through_call`. Two ways this happens: the `SourceCallBinding` result is `Err(MissingEvidence)` (`finite.rs:2106-2116`), which is exactly what the method-receiver binder mismatch in §4.1 yields, or no binding row exists (`:2117-2119`). Then:
   - `summarize_boundaries` filters `MissingEvidence` and falls back to `call_transfer` (`finite.rs:307-324`).
   - `origin_coverage` takes min(priority, code) over {MissingEvidence, CallTransfer} = `missing_evidence` (`finite.rs:2745-2746`).
   - Discharge reads only the former.
3. **Served conditions are DNF display text with no structural id.**
   - `behaviors.condition` is served in the bundle as a string, and `behaviors` has no `condition_id` column (`crates/cpg-schema/src/bundle.rs:298-322`, `crates/cpg-schema/src/behavior.rs:1122-1178`).
   - `BoundedCondition` computes DNF and BDD in parallel (`crates/cpg-schema/src/condition_kernel.rs::BoundedCondition:794-987`). Its DNF has a 16×8 budget and becomes `over_budget` independently of the BDD. A served Conditional fate can therefore carry condition text `over_budget` while the BDD decided the verdict (Interface-checked).
4. **Finite summaries are narrow; almost everything is open.**
   - They cover `Parameter → ReturnValue`, kind `Value` only, and local callee composition only when the return expression *is* the call (`crates/cpg-schema/src/behavior.rs::local_call_summary_flow_seeds:2806-2890`, which requires `fc.call_start_byte = f.sink_start_byte`).
   - Served counts: 23 `summary_flows` / 145 steps against 3,705 `summary_boundaries`; 15,711 `behaviors`; 2,107 `behavior_discharges`.
   - `x = inner(a); return x` is seeded only for pinned models.
   - `return self._inner(x)` is probably always refused `missing_evidence`. The SQL binder applies a receiver shift and `bind_arguments` does not, and `call_binding.rs:271-280` requires them to agree (Interface-checked).
5. **Coverage is a dead end.**
   - `summary_origin_coverage` is produced (`finite.rs::origin_coverage:2684`, `crates/lctx-analytics/src/completion.rs::coverage:85`), persisted and validated for equality.
   - Nothing reads it for conclusions: not discharge, not behaviors, not serving.
   - `summary_contract::coverage_domain` (`crates/cpg-schema/src/summary_contract.rs:48`) has no production caller.
6. **Discharge validity is restated four times.**
   - The native (`python/lctx_semantics/src/lib.rs::admit_discharges:1263`) and SQL (`crates/cpg-schema/src/rules.rs:3469-3520`) restatements are strictly weaker than the producer (`crates/lctx-analytics/src/summaries/discharge.rs::Decisions::from_outcome:35`). Neither checks that the proved origin has no `summary_boundaries` row.
   - Only `crates/cpg-core/src/validate.rs::validate_summary_flows:743-767` re-derives by reusing the producer.
7. **No CFG or dominators anywhere.**
   - Control conditions come only from ty reachability TDDs. Narrowing constraints are never read.
   - Occurrence identity is `(module_node_id, byte span)`. ty and Pyrefly parse the same buffer, so there is no range translation, but at least three consumers reimplement the "innermost containing region" rule.
   - Only name places carry publication parity rules. Attribute and subscript places have none.
8. **Transfer strength is interpreted in at least eight places.**
   - A discharged `Returns` claim is served as `transfer=call`, `value="computed:<byte>"` and verdict `established`, although its proof is a `Value` (identity) summary (`crates/cpg-core/src/behavior.rs:967-977, 1259-1272`; Interface-checked).
9. **Places: seven encodings, no shared type.** `Parameter[...]` is spelled three incompatible ways:
   - by node hex in `source_key`;
   - by name in `summary_flows.input_path` and the models;
   - as mixed forms in `negative_premises.place_key`.

---

## 1. Fact/fidelity table: flow, condition, summary, behavior, model and discharge

Fidelity vocabulary used here:
- **extracted:** a provider row with a `facts` row.
- **resolved:** a provider row joined through resolution.
- **derived:** a deterministic analysis table (ADR-0019), with no `facts` row and not a coverage unit.
- **inferred:** a may-analysis or approximation.
- **heuristic:** a name- or text-based association.

Provider rows carry the schema `Fidelity` codebook (`crates/cpg-schema/src/codebook.rs:66`). Flow facts are `NormalizedStructural` with origin `AnalyzerAssertion` (`crates/cpg-extract/src/flow.rs:230-232`). Pysa facts are `ReportProjection` (`crates/cpg-extract/src/pysa_map.rs:82`).

**SERVED marking:**
- **B** = bundle file, which is also the PostgreSQL `lctx_serving.*` table (`crates/cpg-schema/src/bundle.rs::files:48`).
- **N** = native IPC file (`crates/cpg-schema/src/serving_projection.rs::NATIVE_FILES:12-42`).
- **T** = read by an MCP tool.
- All three are gated to the `behavioral` profile by `serving_projection.rs::relation_requested:177`. Under the default catalog profile every behavioral file projects zero rows.

### 1.1 Flow family

| Relation | Provider & revision | Fidelity | Coverage & unknown representation | Identity recipe | Consumers |
|---|---|---|---|---|---|
| `flow_uses`, `flow_definitions`, `flow_reaching`, `flow_values`, `flow_value_calls`, `flow_regions`, `flow_tests`, `flow_test_types`, `flow_test_exact_origins`, `flow_attribute_loads` (`crates/cpg-schema/src/tables.rs:799-1072`, family `Flow`) | `cpg-flow`: ty_python_core **=0.0.14**, ruff_python_parser **=0.0.14**, salsa **=0.28.2** (`Cargo.toml:70-77`), over the same text buffer Pyrefly parsed (`crates/cpg-extract/src/lib.rs:644, 683-688`) | extracted. Conditions are inferred: ty `AMBIGUOUS` becomes true plus `approximated` (`crates/cpg-flow/src/predicate.rs::diagram:575-577`). `through_call` is purely syntactic. | Per module only: `coverage(fact_family=Flow)` is `CompleteUnderStatedModel`, `Partial` (SyntaxError) or `Failed` (OutsideProviderModel), with skip counts in a `detail` string (`crates/cpg-extract/src/lib.rs:929-954`). No per-occurrence unknown row. Row-level `approximated` flags. `loop_carried` on reaching rows. | `use_id = H("flow_use", module, start, end)`, `definition_id = H("flow_definition", module, kind, start, end, place)` (`crates/cpg-extract/src/flow.rs:297-338`). Every row `fact_id = H("fact", run, table, fields…)` (`facts.rs::FactSink::fact:68-72`). | compile-internal: flow_model, behavior SQL, summaries. Parity rules (`crates/cpg-schema/src/rules.rs:2487-2550`, name places only). **Not served.** |
| `flow_value_call_links` (`crates/cpg-schema/src/derived.rs:1100`) | derived join of ty call path to Ruff 0.0.11 `call_syntax`/`arguments` by span | resolved | Typed status per step: BoundCallee, BoundArgument, MissingCall, AmbiguousCall, MissingArgument, AmbiguousArgument (`codebook.rs:1553-1562`) | fact id | local seeds (`crates/cpg-schema/src/behavior.rs:2847-2851`). Not served. |
| `flow_test_leaves` (Flow) and `flow_test_value_links` (Findings, `behavior.rs:1593`) | cpg-flow predicate lowering plus entry-link derivation (`crates/cpg-core/src/entry_links.rs`) | extracted / derived | unmatched operands are counted, not recorded (`crates/cpg-extract/src/flow.rs:474-493`) | leaf `atom_id = H("bdd-atom", atom)` (`flow.rs:464`) | **SERVED B+N+T** (`inspect_value_paths` → `ValuePath.value_links`). Local control links (`behavior.rs::local_call_value_links:2948`). |
| `value_flow_contributions` (`behavior.rs:1221`) | `crates/cpg-core/src/flow_model.rs::run:1718-1798` (source-reach fixpoint) | derived; inferred (may) | `flow_reach_boundaries(use_id, BudgetReached)` for capped uses (`flow_model.rs::reach_boundary_rows:1040`). Capped existing origins are widened to `unknown(SourceOverBudget)` (`:1007-1017`). Flags `captured`/`approximated`. | `origin_id = H("value-flow-origin", fact, use, source_key, identity, through_call, local_through_call, upstream_identity, upstream_through_call)` (`crates/cpg-schema/src/id.rs::recipe::value_flow_origin:138-148`). Path properties are part of identity. | summary seeds and candidates, predecessor candidates, discharge membership, rules. Not served. |
| `value_flows` (`behavior.rs:1303`) | same producer, presentation merge (`flow_model.rs:1817-1880`) | derived | as above. The condition is an OR over contributions, with DNF text in the `condition` column. | key `(module, sink, start, end, source_key, identity, through_call)` | behavior scan (`crates/cpg-core/src/behavior.rs::v2_flows:165`, `:893-1009`). Not served. |
| `flow_reach_boundaries` (`behavior.rs:1206`) | flow_model | derived | check `reason = 6` (BudgetReached only) | key `(snapshot, use_id)` | `summary_boundary_candidates.reach_budget` (`behavior.rs:2958-2976`). Not served. |
| `value_flow_predecessor_candidates` / `_compatibility` (`behavior.rs:1342/1376`) | SQL, then `crates/lctx-analytics/src/summaries.rs::predecessor_compatibility:126` | derived | `compatible_under_atoms: Option<bool>` XOR `boundary_reason`; loop-carried gives `UnsupportedControlFlow` | key `(successor_origin, predecessor_origin, reaching_fact)` | `modeled_assignment_return_paths` (`behavior.rs:2444`). Not served. |
| `raise_sites`, `field_accesses`, `dynamic_accesses`, `ambient_reads`, `singletons` (`behavior.rs:1655-1732`) | flow_model | derived; `field_accesses` matches by name, which is heuristic | condition DNF text; `escapes` flag | span keys | `ambient_reads` is **SERVED B+T** (`get_operation` → `SettingRead.condition` as DNF text; the bundle drops `condition_id`). `singletons` is **SERVED B**. The others are internal. |
| `negative_premises` (`behavior.rs:1779`) | flow_model premises | derived | `holds=false` plus reason | `place_key` string | IsRead behaviors. **SERVED B** as `place_claims` (Field/Global kinds only). |

### 1.2 Condition family

| Relation | Provider | Fidelity | Unknown | Identity | Consumers |
|---|---|---|---|---|---|
| `conditions`, `condition_nodes` (tables.rs:1052-1072, Flow) | cpg-flow lowering (`predicate.rs::diagram:559`) through biodivine-lib-bdd **=0.6.3** (`Cargo.toml:35`) | extracted, DAG-canonical | `root_id NULL` plus `boundary_reason` as **KernelBoundary text** (`condition_kernel.rs::KernelBoundary::code:38`), not the BoundaryReason codebook | `condition_id = H("condition-bdd", root)`, node `H("bdd-node", atom, low, high)` (`condition_kernel.rs:191-197, 595-597`); a boundary root is `H("condition-bdd-boundary", Debug(reason), DNF)` (`:955-961`) | flow_model hydration (`flow_model.rs:1344-1392`). **SERVED B+N.** |
| `analysis_conditions`, `analysis_condition_nodes` (behavior.rs:1266) | `flow_model::condition_catalog_rows:675` from compile-local `condition_models` | derived | same KernelBoundary text | same | summaries, native. **SERVED B+N.** `inspect_value_paths` shows only the opaque `condition_id`. |
| `condition_literals` | legacy DNF (`crates/cpg-extract/src/flow.rs:537-617`) | display | – | – | internal |

### 1.3 Summary family

| Relation | Producer | Fidelity | Unknown | Identity | Consumers |
|---|---|---|---|---|---|
| `summary_flows` (behavior.rs:1464) | `finite.rs::finite_flows:1465`, re-run by the validator (`validate.rs::validate_summary_flows:659`) | derived; may-path under source/model abstraction (`finite.rs:1112-1113`) | Emitted verdicts are only Established or Conditional (`finite.rs:903-909, 1434-1438`). Unknown lives in `summary_boundaries`. | `summary_id = H("summary-flow", fn, param, origin, input_path, output_path, kind, condition, return_site, return_region, n, [kind, evidence, condition]…)` (`id.rs::recipe::summary_flow:271-297`). The proof is sealed into identity. | **SERVED B+N+T** (`inspect_value_paths` → `ValuePath`). Native reads a subset of columns and ignores `input_path`/`output_path`/`kind`/`approximated` (`python/lctx_semantics/src/ipc_input.rs:300-322`). |
| `summary_flow_steps` (behavior.rs:1496) | same | derived support | – | key `(summary_id, ordinal)` | **SERVED B+N+T** (`ValuePath.steps[]`). `evidence_id` is not dereferenceable by `get_evidence` (per the serving inventory). |
| `summary_boundaries` (behavior.rs:1515) | `finite.rs::summarize_boundaries:252` | derived | one reason per `(origin, condition)` | key `(snapshot, source_origin_id, condition_id)` | discharge; **SERVED B+N+T** (`OpenBoundary`). |
| `summary_origin_coverage` (behavior.rs:1562) | `finite.rs::origin_coverage:2684` (ValueOrigin/Value) plus `completion.rs::coverage:85` (ExecutionSite × {Completion, Exception}) | derived | `complete` + `reason` + `witness_count` + `witnesses_omitted` | key `(subject_kind, subject_id, condition, channel, phase)` | **None besides the validator and referential rules** (`rules.rs:1606-1633`). Not served. |
| `summary_components` (behavior.rs:1440) | `crates/cpg-core/src/summaries.rs::call_components:222` → `crates/lctx-analytics/src/summaries.rs::call_components:33` | derived | – | `component_id = H("summary-component", sorted members)` (`id.rs:300-307`) | finite local-seed ordering only (`finite.rs:2005-2044`). Validator re-derives (`validate.rs:542`). Not served. |
| `return_entry_statuses`/`_steps`, `return_exit_statuses`/`_steps`, `statement_completions`/`_steps`, `expression_evaluations`/`_steps`, `call_executions`/`_steps` | `completion.rs::complete:1439`, `crates/lctx-analytics/src/evaluation.rs::evaluate:903`, orchestrated by `crates/lctx-analytics/src/execution.rs::prepare:7` | derived | a `reason: Option<BoundaryReason>` on each status row; `work` counts | per table: fact keyed plus ordinal | summary producer and certificates. Not served. |
| `return_completion_certificates`, `source_*` proof tables, `model_frame_exit*`, `source_*_identities`, `model_context_protocols` | `crates/cpg-schema/src/completion_proof.rs::certify:57` and the per-module `identity()` recipes | derived support | load-or-refuse admission (`completion_proof::admit`, `frame_exit::admit_proof`, …) | ids hash ordered-step digests, e.g. `H("return-completion-certificate", …, entry_digest, exit_digest)` (`completion_proof.rs:44-54`) | **SERVED B+N** as admission inputs, filtered to cited rows. Not rendered. |

### 1.4 Behavior family

| Relation | Producer | Fidelity | Unknown | Identity | Consumers |
|---|---|---|---|---|---|
| `behaviors` (behavior.rs:1122) | `crates/cpg-core/src/behavior.rs::run:245` (Pass B findings, Stage-2 flow claims, handoffs, delegations) | derived. `Forwards` from Pass B is resolved plus syntactic binder. Field association is heuristic by name (`behavior.rs:1100-1238`). | `verdict` (5-valued) + `boundary_reason`. `operations.boundary_reason` is chosen by `reason_rank` (`behavior.rs:126`). | `behavior_id = H("behavior", op, kind, param, callee, target, value, site)`; flow claims wrap it as `H("flow-behavior", …, transfer, scope)` (`crates/cpg-schema/src/behavior.rs:1800-1838`). The `value` display text is part of identity. | **SERVED B+T**: `get_operation` Fate (verdict, boundary, `condition` **DNF text**, transfer, discharges); facets. |
| `behavior_steps` (behavior.rs:1184) | same | derived | `condition: Option<String>` (DNF) | `(behavior_id, step)` | rule `established-needs-definite-path`. Not served. |
| `operation_facets`/`_status` | `crates/cpg-core/src/catalog.rs:252-380` | derived | per-facet status | – | **SERVED B+T** (selection tools) |
| `argument_flows`, `guards`, `parameter_reads`, `handoffs`, `delegations` | SQL `crates/cpg-schema/src/flows.rs::argument_flows_sql:214` | resolved; syntactic binder | `value_class Other` | ordered SQL | Pass B (`crates/lctx-analytics/src/pass_b.rs::run:352`). Not served. |
| `witnesses` (findings.rs:130) | Pass A/B | derived | – | `(finding, path, step)` | **SERVED B** only as `support_witnesses` when a brief cites them |

### 1.5 Model family

| Relation | Producer | Fidelity | Unknown | Identity | Consumers |
|---|---|---|---|---|---|
| `model_targets`, `model_transfers`, `model_effects`, … (behavior.rs:705-763) | authored catalog `crates/cpg-schema/models/external.toml`: 8 transfer rules, all Parameter → ReturnValue, identity or transform | synthetic model (`Origin::SyntheticModel`) | per-rule `coverage = {transfers = complete / partial / unspecified …}` and `modality` | `(model_id, target, rule_id)` | modeled seeds. Not served. |
| `model_applications`, `model_argument_bindings`, `modeled_transfer_sites`, `modeled_exact_value_transfers`, `modeled_argument_evaluations` (behavior.rs:2069-2402) | SQL over Pysa `call_targets` (Pyrefly fork `a07b7ba…`, `Cargo.toml:55`) | resolved (Pysa ReportProjection) | `status Unknown` + `reason`; `candidate_set_complete_under_model`, `has_unresolved_remainder` | keys include `pysa_fact_id` | finite modeled seeds. Not served. |
| Pysa TITO | evaluation-only oracle (`docs/design_review/evidence/2026-09-25_pysa-tito-rule/README.md`; test `crates/cpg-core/tests/compile.rs:1981`) | external | – | – | **Not a compiler input** |

### 1.6 Discharge family

| Relation | Producer | Fidelity | Unknown | Identity | Consumers |
|---|---|---|---|---|---|
| `behavior_discharges` (behavior.rs:1538) | `crates/lctx-analytics/src/summaries/discharge.rs::grade:77` called from `crates/cpg-core/src/behavior.rs:1259-1272` | derived | `decision Open` + `reason` (default `call_transfer`) | key `(behavior_id, origin_id)` | **SERVED B+N+T**: `Fate.discharges[]`; native `admit_discharges` validation only. Served count 2,107. |

---

## 2. Engine signatures (a paper probe for a shared fixpoint driver)

### 2.1 Signature table

LOC counts are approximate line spans of non-test code.

| # | Engine (site) | State domain / lattice | Transfer | Merge / join | Schedule / order | Termination, budgets, behaviour at cap | Semantic key vs witness | Outputs incl. refusals | LOC |
|---|---|---|---|---|---|---|---|---|---|
| 1 | **Diagram lowering** (`crates/cpg-flow/src/predicate.rs::diagram:559-589`) | memo `ScopedReachabilityConstraintId → BoundedCondition`. Boolean functions over evaluation atoms (BDD) plus an absorbing `Err(KernelBoundary)`; DNF in parallel. | Shannon expansion `p∧T ∨ ¬p∧F`, predicate lowered to an atom (`predicate.rs::test:97, compare:139, isinstance:203, pattern:480`) | none (DAG, memoized) | recursive DFS from the root | ty's TDD is acyclic; no work counter, unbounded recursion depth (Interface-checked). Kernel caps `MAX_ATOMS=128`, `MAX_NODES=50_000`, `MAX_PAIR_WORK=1e6` (`condition_kernel.rs:19-23`) give `Err`, persisted as `root_id NULL` plus a boundary. | key = BDD root id; no witness | `conditions`/`condition_nodes`, DNF display, `approximated` flag. Refusal is a boundary-root row. | ~30, plus 727 file |
| 2 | **Source reach** (`crates/cpg-core/src/flow_model.rs::Model::reach_step:823`, `solve_reach:957-1025`) | per use: `BTreeMap<(Origin, FlowTransfer), Source{condition, captured, approximated}>` | Parameter/field/captured definitions give `Identity`. A definition value source composes `transfer.max(t3).max(bound)` (`:942`) with condition `at ∧ c2 ∧ s` (the use side only when loop-carried, `:933-937`). | `merge`: condition OR, flags OR (`:755-767`). Change is detected by condition id (`same_sources:771`). | `BTreeSet<use id>::pop_first` (lowest id). Changed nodes reschedule parents (`:984-994`). | Monotone growth over a finite atom set; global `MAX_REACH_WORK=1_000_000` (`:769`). At cap, the pending set plus all ancestors are marked incomplete, existing conditions are widened to `unknown(SourceOverBudget)` (`:995-1018`), and `flow_reach_boundaries(BudgetReached)` is emitted per use (`:1040-1053`). | key `(Origin, Transfer)`. **No witness**: the reaching chain is not recorded. Contributions keep only `(fact, use, upstream transfer)`. | `value_flow_contributions`, `value_flows`, reach boundaries | ~216 |
| 3 | **SCC schedule** (`crates/lctx-analytics/src/summaries.rs::call_components:33-124`) | petgraph `kosaraju_scc` over sorted, deduped ids | – | – | Condensation Kahn order with a `BTreeSet<(min member id, idx)>` ready queue: callee-first and input-order independent | None needed; errors if the condensation is cyclic. The `recursive` flag is never read by any consumer (`finite.rs` uses only `component_order`, `:2005-2013`). | component id = hash of the sorted members | `summary_components` | 92 |
| 4 | **Finite value worklist** (`finite.rs:2200-2451`; prep `:2003-2199`; primitives `crates/lctx-analytics/src/summaries/worklist.rs`) | `Frontier<SemanticFlow, (fn, param)>` of `Witness{summary_id, depth, cost}` (`worklist.rs:21-67`), plus per-seed `AlternativeProgress<SemanticFlow>` (`:71-102`) | Compose a caller local seed with a callee witness: predecessor entry proof, CalleeResolution, ReturnSource/ArgumentEvaluation, ParameterBinding, CallSite, CallTarget, specialization steps, CalleeSummary, ReturnExit (`finite.rs:2283-2356`). The callee condition must specialize to **true** (`specialize_local_condition:559-663`, `:650`). `admit_callee_proof` (`:2357-2376`). | **No join.** Nondominated `(depth, cost)` per key: an insert is rejected if any old witness is ≤ on both dimensions (`worklist.rs:39-45`). | Groups in `(component_order, component_id)` order (callee-first). Within a group, `BTreeSet<(seed index, witness id)>` (`worklist.rs::ComponentQueue:107-137`). | Depth ≤ `MAX_PATH_DEPTH=8` gives `SummaryDepthLimit`. Cost ≤ 64 (`MAX_SUMMARY_PROOF_STEPS`): an over-cost witness is emitted but never propagates (`finite.rs:2419-2421`). Pair cap 1e6 on queue size and charges: at cap, **every seed in the component** is refused `SummaryPairWorkLimit` (`:2441-2443`), including already-proved ones. `known_summary_ids` dedups repeats (`:2405-2409`). | key `SemanticFlow` = (fn, param, source fact, origin, condition, return site/region, input/output strings, kind, verdict, refusal, approximated) (`finite.rs:377-392`). Witness = `summary_id`. The call site is **not** in the key. | `summary_flows` (every admitted witness, not only nondominated), steps, `SummaryRefusal` per alternative. `CallTransfer` when a seed is unexamined (`:2444-2446`). Lifecycle admission afterwards in `(path_depth, id)` order (`:2482-2590`). | ~250 + ~200 |
| 5 | **Witness-omission closure** (`finite.rs::unexpanded_witnesses:430-462`) | `BTreeSet<summary_id>` | semantic groups with more than one witness, then reverse `CalleeSummary` citations | union | `BTreeSet::pop_first` | Bounded by rows; no budget | – | `summary_origin_coverage.witnesses_omitted` (not consumed) | 33 |
| 6 | **Modeled chain composition** (`finite.rs::modeled_chain_proof:1192-1347`) | none: validates one fixed chain | Recursive `visit` builds the proof outer→inner | – | step index | `MAX_MODELED_CHAIN_STEPS=8` gives `SummaryDepthLimit`; `MAX_MODELED_CHAIN_ARGUMENTS=128` gives `BudgetReached` (`:1206-1211`). An empty step gives `CallTransfer`; any mismatch gives `MissingEvidence`. | – | proof or refusal | 156 |
| 7 | **Pass A BFS** (`crates/lctx-analytics/src/pass_a.rs::run:91-395`) | `depth[]`, `parent[]` over the invocation projection | arc | first arrival wins | FIFO over sorted adjacency | `max_depth` is the model; vertex and edge budgets give `StopReason` and `partial` (`:121-147, 269-275, 383`) | Witness = BFS path plus up to `max_witnesses-1` alternates that differ in the final arc. `witnesses_omitted` is presentation truncation. | findings, members, witnesses | ~305 |
| 8 | **Pass B worklist** (`crates/lctx-analytics/src/pass_b.rs::run:352-675`) | states `(callable, formal, source, suppressed)` (`:368-388`) | Follow an `argument_flows` row whose `source_parameter = formal` and `value_class ∈ {Parameter, Alias}` | none: first path wins (`visited`). `suppressed` keeps the clean and suppressed variants distinct. | FIFO `VecDeque` | Depth bound only gives `stop_reason = DepthLimit`, with completion always `CompleteUnderStatedModel` (`:665-667`). `behavior.rs` maps this to `BudgetReached` (`crates/cpg-core/src/behavior.rs:406-415`). | first witness path per state | findings (forwarding, transformed, conditional_raise, unfollowed) | ~325 |
| 9 | **Completion kernel** (`crates/lctx-analytics/src/completion.rs::Kernel:326-1439`; driver `complete:1439-2311`) | a single exact `CompletionOutcome{Normal, Return, Raise{TypeError}, Break, Continue}` (`crates/cpg-schema/src/summary_contract.rs:794-850`) plus the active exception, initialized names and proof list | Per statement kind: Pass, Expr, Assign, Return, Raise, Break, Continue, If, With, Try. Loops and others give `UnsupportedControlFlow` (`completion.rs:1230-1433`). Finalizer replacement via `after_finalizer`. | **None**: branch selection by `truth()` under an assumed condition; undecidable gives a refusal | syntax recursion; one run per statement and per (return site × requested condition) | `MAX_WORK=4096`, reset per request; `MAX_DEPTH=128` (`:27-28, 2063, 2126`). Gives `Completion{Work,Depth}Limit`. The source-call proof limit (>64) gives `SummaryProofLimit` (`:354-365`). | – | `statement_completions`/`_steps`, return exit and entry statuses/steps, certificates, `call_executions`, source bodies. `reason` column on each row. | ~2,000 |
| 10 | **Expression evaluator** (`crates/lctx-analytics/src/evaluation.rs::Evaluator:74-620`; driver `evaluate_with_limits:907-1153`) | `Value{Bool, Int, Float, None, Tuple{nonempty, retained}, Literal, Retained, Unknown}` (`:29-42`) | Python value semantics over closed syntax; short-circuit selection | none | recursive | `MAX_WORK=1024`, `MAX_DEPTH=64` (`:25-26`) give `Expression{Work,Depth}Limit`; unsupported gives `UnsupportedControlFlow` | – | `expression_evaluations`/`_steps`, invocations, frames | ~1,000 |
| 11 | **Base→enriched** (`crates/lctx-analytics/src/execution.rs::prepare:7-57`) | – | evaluate → complete (no source calls) → `source_call::prepare` → evaluate → complete | – | a fixed two stages; "not an accidental two-iteration fixed point" (`:46`) | n/a: exactly one source-call level | – | enriched outcome; base bodies | 57 |
| 12 | **Recursive SQL CTE** (`crates/cpg-schema/src/behavior.rs::modeled_handler_climb_sql:2044-2062`). `rules.rs:3024` is a type-term rule, not behavioral. | syntax ancestor rows | parent step | UNION ALL | SQL | `depth < 128` | – | handler candidates | 19 |

### 2.2 Shared mechanisms: what is shared today and what could be

| Mechanism | Shared today by | Could serve ≥3 engines? | Assessment |
|---|---|---|---|
| **SCC / callee-first scheduling** | 1 (the finite local worklist) | Possibly 3. (a) Finite value (today). (b) Evaluation/completion with source-call normals, where `execution.rs` hard-codes one source-call level; a callee-first schedule would generalize the "fresh source-call domain" to n levels. (c) Future effect/exception channels (forward plan P3 `summary_effects`). | **Justified as a component.** The petgraph wrapper is already generic (`summaries.rs:33`). Reuse is a schedule, not a driver. |
| **Bounded nondominated frontier and AlternativeProgress** | 1 | Only if more channels become summaries. Pass A keeps "shortest plus alternates by final arc" and Pass B keeps "first path" on purpose. | Keep `worklist.rs` as the primitive for summary channels. Do not force it on BFS passes, whose witness policies are presentation choices. |
| **Budget charging** | 0 shared. Eight bespoke counters: reach work, pair work, completion work/depth, expression work/depth, BDD pair/node/atom, DNF 16×8, Pass A vertex/edge, Pass B depth, chain steps/args, CTE depth. | Yes, all of them | Share a **named budget type** that carries `(unit, cap, BoundaryReason)`. Do **not** merge the units. Distinct reasons (e.g. `ExpressionDepthLimit` vs `CompletionDepthLimit`) are informative, and caps mean different things (a model bound in Pass A depth versus truncation). |
| **Canonical ordering** | Convention everywhere: BTree maps, sorting by ids, `ComponentQueue` | – | Already a convention; nothing to extract beyond `ComponentQueue`. |
| **Witness omission** | 1 (finite coverage flag, unconsumed). Pass A has a same-named flag with a different meaning. | No | A naming hazard, not a component. |
| **Refusal emission and priority** | 0 shared. Four policies: `finite.rs::refusal_priority:235`; `discharge.rs:40` (lowest code); `crates/cpg-core/src/behavior.rs::reason_rank:126`; `crates/lctx-analytics/src/summaries.rs:148-153` (BudgetReached first). Three vocabularies: `BoundaryReason`, `KernelBoundary` text, native `TheoryBoundary` strings (`python/lctx_semantics/src/lib.rs:1510-1517`). | Yes: finite, completion, evaluation, reach, behaviors, native | **The most justified shared component:** one refusal set per subject plus one declared priority (§4.2). |

### 2.3 Where a generic driver would hide real differences

1. **Join versus alternatives.**
   - Reach is a may-analysis over a join-semilattice: conditions are OR-merged per `(Origin, Transfer)`.
   - The finite worklist must **not** join. Joining two witnesses would erase distinct support, and an OR of callee conditions would break "the callee condition must reduce to true" (`finite.rs:650`).
2. **Cap semantics.** There are three incompatible behaviours:
   - reach widens existing results to unknown and marks ancestors;
   - the finite worklist refuses *every* seed of the component, even proved ones (`finite.rs:2441`);
   - Pass A/B report a stop reason but keep findings.
3. **Key/witness split.** Only the finite worklist has one. Reach has keys but no witnesses. BFS passes have witnesses but no semantic keys (the state is the key and the first path is the witness).
4. **Termination argument.** Reach relies on lattice height plus a global work cap. Finite relies on a depth bound (8), a cost bound (64) and a pair cap. Completion and evaluation rely on structural recursion plus per-request work. A driver that hides the basis would make "complete under stated model" unverifiable.
5. **Refusal attribution depends on each engine's internals.** In the finite worklist, a witness over the cost bound is invisible to callers, so the caller is refused `call_transfer` (unexamined), not `summary_proof_limit` (`finite.rs:2208-2213, 2419-2421, 2444-2446`; Interface-checked). A generic driver would have to expose "a witness dropped by bound" as a first-class event to avoid this class of misattribution.

**Verdict:**
- A shared fixpoint driver is **not justified** (two genuine fixpoints with incompatible merge and cap semantics).
- These components **are justified**:
  - the SCC schedule, reused for n-level source-call normals;
  - a refusal/priority policy;
  - named budgets;
  - `worklist.rs` kept as the summary-channel primitive.

---

## 3. Places and transfers (paper input for `transfer(owner, in_place, out_place, kind, condition, provenance_class, support)`)

### 3.1 Place and port representations (Implemented unless noted)

| Representation | Root binding | Depth | Containers | Unknown suffix / unknown | Site |
|---|---|---|---|---|---|
| Model `InputPath{Parameter, ReceiverField, Global}` / `OutputPath{ReturnValue, Parameter, ReceiverField, Global, Raise}`; `render()` gives `Parameter[name]`, `Parameter[r].Field[f]`, `Global[m.n]` | formal **name**; bound to a call later by SQL | ≤1 (a receiver field) | none | endpoint status `Unknown` + `OutsideProviderModel` except bound Parameter → ReturnValue | `crates/cpg-schema/src/models.rs::InputPath:372-419`, `OutputPath:426-474`; `crates/cpg-schema/src/behavior.rs::modeled_transfer_sites:2267` |
| Model path id `H("behavior-model-input-path", kind, names)` | names | – | – | – | `models.rs::InputPath::id:403` |
| Flow-model `source_key` = `Parameter[<node hex>]` or `Field[<qualified class>.<field>]` | parameter **node id**; field = class node + field **name** (per class, not per instance) | a field origin only at depth 1; suffixes containing `.` or `[` are rejected | none | `captured`/`approximated` flags; reach boundaries | `crates/cpg-core/src/flow_model.rs:1445-1460, 1725-1752, 1818-1845` |
| Provider place text (ty `PlaceExpr` Display: `self.x`, `a.b.c`, `d["k"]`) | text in scope, joined by span | arbitrary, as text | literal subscripts only | none | `crates/cpg-flow/src/lib.rs:118-136, 694-762`. A store is detected by `place.contains('.' or '[')` (`flow_model.rs:1807`). Chained stores keep only the first place (`:1809`). |
| `summary_flows.input_path` / `output_path` = `Parameter[<name>]` / `ReturnValue`, plus a `parameter_node_id` column | name + node | 0 | none | in `summary_boundaries` | `finite.rs:910-914, 1371-1375` |
| Frontier subject `(function, parameter)`; the output port is checked by a string compare with `OutputPath::ReturnValue.render()` | node ids | – | – | – | `finite.rs:415-419, 2206, 2233` |
| `negative_premises.place_key` (`Parameter[<node>]`, `Field[…]`, `Global[m.n].field`) | mixed | ≤1 | – | `holds=false` + reason | `crates/cpg-schema/src/behavior.rs:1779` |
| Binder port `BoundArgument{argument_fact_id, parameter_fact_id, name}` | signature-parameter **fact id**; positional by order, keyword by name; **no receiver shift** | 0 | variadics and unpacking refused | `Err(BoundaryReason)` | `crates/cpg-schema/src/summary_contract.rs::bind_arguments:566-676` |
| SQL binders: `maps_formal` (receiver offset) and `model_argument_bindings` (Pysa receiver shift, all-signature agreement) | formal node / name | 0 | – | status `Unknown` + reason | `crates/cpg-schema/src/flows.rs::maps_formal:151-163`; `behavior.rs:2092-2190` |
| Catalog field link `(signature, ordinal, formal?) → field_id → reader` | field **node id**; formal nullable | 1 | – | kind `declared_parameter` = association only (reason text) | `crates/cpg-core/src/surface.rs:880-999, 1003-1087` |

**Missing everywhere:**
- no container-content port, element port or unknown-suffix/wildcard port;
- no instance-sensitive field (fields are class and name);
- no argument out-port `Arg(site, formal)` in the model grammar.

### 3.2 Transfer-stating relations mapped to the 7-tuple

| Relation | owner | in_place | out_place | kind | condition | provenance_class | support | What unification loses / what the relation carries extra |
|---|---|---|---|---|---|---|---|---|
| `model_transfers` (8 rules, `external.toml`) | `target_node_id` (+ `model_id`, `revision`) | `input_path_*` | `output_path_*` | `ModelTransferKind{Identity, Transform}`; only Identity is consumed | **none** | `SyntheticModel` | `rule_id` | **Extra:** `modality`, revision pin, per-channel `coverage` completeness, which the tuple has no slot for. |
| `modeled_transfer_sites` / `modeled_exact_value_transfers` | caller + `call_site` | bound argument expression | call result | `transfer` | none; joined later | model + `pysa_fact_id` | `call_fact_id`, `rule_id` | **Extra:** `candidate_set_complete_under_model`, `has_unresolved_remainder`, `target_modality`. A tuple without a *site* dimension loses call-site context. |
| Pysa TITO (`formal(value, position=0) → LocalReturn`) | callable string | formal name + position | LocalReturn | TITO + features | none | external oracle | raw JSON (LFS) | Not a compiler input. Evidence README "Boundary" says Pysa silence is not proof of no flow. It would enter only as `provenance_class = oracle`. |
| Raw flow (`flow_values` + `flow_reaching` + `flow_value_calls`) | scope | use occurrence / reaching definition | sink span (Definition/Argument/Return/Yield/Raise) | identity/derived/through_call | provider `condition_id` | ty provider (`approximated`) | fact ids | **Loses** the ordered `call_path` and `loop_carried` unless support is typed. |
| `value_flow_contributions` | `sink_function_node_id` | `source_key` / param / class | `flow_value_fact_id` | `FlowTransfer` + `local_through_call` / `upstream_*` | BDD `condition_id` | derived (`captured`/`approximated`) | `origin_id` only | **Already a closure** (reach fixpoint) with **no step table**: the reaching chain is not recoverable except by re-running flow_model (`validate.rs::validate_value_flow_contributions:829-871`). Path properties are hashed into identity (`id.rs:127-148`). |
| `value_flows` | `function_node_id` | `source_key` | sink + `argument_node_id` / `call_site` / `place` | identity / through_call | **OR** over contributions | derived | members in memory only (`FlowModelRows.value_flow_members`, `flow_model.rs:640`) | **Loses** per-contribution conditions and the call. **Extra:** the store `place`. |
| `behaviors` Forwards / Derives / Returns / Stores | `operation_node_id` | `parameter_node_id` | callee formal (`target_node_id`), `target_name`, or the return site | `BehaviorKind` + `FlowTransfer` | **DNF string only** | verdict + `boundary_reason` | `behavior_steps`, `behavior_discharges` | **Extra:** verdict grading, discharge. `value` mixes the rendering with the site byte, and it is part of identity. |
| `argument_flows` (Pass B) | caller + `edge_id` | source param / alias / literal | `formal_node_id` | `ValueClass` + modality + phase | booleans (`conditional`, `may_catch`, `value_tested`) | invocation-projection arc | `edge_id` | A second producer of Forwards. Stage 2 skips identity rows to a release target because "Pass B owns them" (`crates/cpg-core/src/behavior.rs:916-924`). |
| `summary_flows` | `function_node_id` | `parameter_node_id` / `Parameter[name]` | `ReturnValue` + return site / region | `SummaryFlowKind` (Value only) | `condition_id` | derived; `approximated`, `path_depth` | ordered steps sealed into `summary_id` | Unknown lives in `summary_boundaries`. The unified relation needs first-class open rows. |
| `catalog_field_links` | class / signature | `(ordinal, formal?)` | `field_id` (`reader` for readers) | `exact_storage` / `declared_parameter` / `exact_reader` (strings) | none (the proof is straight-line) | `source_fact_id` + free-text reason | `direct_store` proof recomputed, not persisted (`crates/cpg-core/src/surface.rs:432-631`) | `exact_reader` is **association**, not value identity over time (`surface.rs:1082`, `crates/cpg-core/src/behavior.rs:1088-1089`). |
| `source_parameter_identities` / `_modeled_` / `_context_value_` | function | parameter | return-read occurrence | implicit identity | `condition_id` | lexical reconstruction | digests | **Support certificates**, not transfers (steps `SourceParameterIdentity`, `SourceModeledIdentity`, `ContextEntryValueIdentity`). |

**Generic losses of the 7-tuple:**
- `site`: needed for matched call/return;
- `modality`;
- `exit` / channel;
- open status + reason (first-class unknown);
- typed ordered support with an explicit alternative (OR) structure;
- the association-versus-identity distinction;
- per-contribution versus merged conditions.

At minimum the tuple needs `site`, `modality`, `channel`, `status/reason` and `support_id`.

### 3.3 Paper sketches over a unified relation

**Option → field → reader** (e.g. `Config(timeout=t, title=n)`, then `cfg.title` read in `render()`):
1. `transfer(ctor_call_site, Arg(site, kw=title), Formal(Config.__init__.title), bind, cond_site, resolved, binder_proof)`.
2. `transfer(Config.__init__, Formal(title), Field(Config, title), store, cond_store, derived, direct_store_proof)`. Only `exact_storage` qualifies; `declared_parameter` must be `kind = association`.
3. `transfer(Config, Field(Config, title), Read(render, attr_node), read, cond_read, derived, exact_reader)`: **association**, because no intervening-mutation proof exists.
4. `transfer(render, Read(...), Return/Arg(...), value, …)`.

Composition must stay per field node. Today steps 2–4 exist only as depth-2 `Unknown/ScopeBoundary` rows ("field association only; intervening mutation unproved", `crates/cpg-core/src/behavior.rs:1100-1238`), gated on `exact_reader` whose formal equals the store's parameter. Expected in tests: 3 rows for `RecordHolder` and 0 for plain `Holder` (`crates/cpg-core/tests/compile.rs:215-238`; Interface-checked).

**Facade delegation** (`def f(x): return inner(x)`):
- `transfer(f, Formal(f.x), Arg(site, inner.p), bind, …)` + `summary(inner, Formal(p), Return)` + `transfer(site, Result(site), Return(f), identity)`.
- Composition joins on **site**.
- Today:
  - only the `return inner(x)` shape is seeded (the call span must equal the return sink span, and there must be exactly one call step: `crates/cpg-schema/src/behavior.rs:2842-2846`);
  - resolution must be unique and complete (`:2857-2863`), with a definite mapping (`:2867-2871`).
- `x = inner(a); return x` stays `call_transfer`, because assignment seeds exist only for pinned models (`behavior.rs::modeled_assignment_return_paths:2444`).
- Method facades `return self._inner(x)` are probably refused `missing_evidence` (the receiver-shift mismatch, §4.1).

### 3.4 Controls

| Control | Current code | Would it hold in a unified relation? |
|---|---|---|
| `Config(timeout=t, title=n)` must not link title to t | **Holds**: keyword binding by name in all three binders; field links per field name or `direct_store` (`surface.rs:885-900`). Closest test: `crates/cpg-core/tests/catalog.rs:141-151` (Interface-checked). **Hazard:** a hand-written swapped `__init__` (`self.timeout = title`) still yields a name-based `declared_parameter` link from formal `timeout` to field `timeout` (`surface.rs:899`, `:972-976`). That is labelled an association, but a consumer reading it as a transfer would link `t` to the field that actually holds `n`. | Only if `declared_parameter` is `kind = association`, never composed as a transfer, and one binder owns the receiver shift. |
| Two calls to one identity function must not cross | **Holds by construction.** A seed is `(sink use, call site, argument ordinal, callee formal)`, and the callee summary is instantiated inside the caller's proof at that site (`finite.rs:2283-2356`). The call site is fixed by the seed SQL, which requires the call span to equal the return span. Tests: `finite.rs:3845` (parallel origins stable under shuffle), `compile.rs:1715-1735` (Interface-checked). `x = f(a); return f(b)` is simply never seeded. | Crosses unless ports are site-qualified and composition is matched call/return on `site`. Callee conditions must be substituted in the caller's vocabulary (today: `local_call_value_links` + `specialize_local_condition`, `finite.rs:559-663`). |
| An unresolved callee stays open | **Holds.** No seed without `candidate_count = 1`, `target_count = 1`, `candidate_set_complete_under_model` and `NOT has_unresolved_remainder` (`behavior.rs:2857-2863`). The candidate falls back to `call_transfer` (`finite.rs:295-306`). The Stage-2 `through_call` row is `Unknown/CallTransfer` (`crates/cpg-core/src/behavior.rs:980-984`). The discharge default is Open `call_transfer` (`discharge.rs:61-72`). | Needs first-class open rows (kind unknown + reason) or a claim-specific coverage relation. A bare transfer relation would read absence as no-flow. |

---

## 4. Semantic-interpretation map

Before looking for divergences, each item records whether one site is mechanically derived from another.

### 4.1 Argument binding

**Sites:**
- **(a)** SQL `maps_formal` (`crates/cpg-schema/src/flows.rs:151-163`), used by `argument_flows_sql:214`. That feeds Pass B, `behaviors` `release_target` (`crates/cpg-core/src/behavior.rs:805-823`) and the local-seed and local-argument mappings (`crates/cpg-schema/src/behavior.rs:2819-2822, 2906-2912`).
- **(b)** `model_argument_bindings` SQL (`behavior.rs:2092-2190`), with the Pysa receiver shift and all-signature agreement.
- **(c)** Rust `summary_contract::bind_arguments:566`, used by `crates/lctx-analytics/src/call_binding.rs:263`, `context_protocol.rs:93` and `evaluation.rs:791`.

**Derived?** (d) `local_call_arguments` derives from (a). (c) is independent, but `call_binding.rs:264-282` **cross-checks** (c) against (a) and returns `MissingEvidence` on disagreement. (b) is independent.

**Divergences (Interface-checked):**
1. **Receiver.** (a) adds `a.receiver` to the positional ordinal and (c) does not. A method call `self._inner(x)` binds `x → self` in (c) and `x → x` in (a). The mismatch gives `MissingEvidence`, so method facades never compose.
2. **Arity errors.** For `def f(a)` called as `f(x, y)`, (a) maps `x → a` and silently drops `y`, so Pass B / `behaviors` may state `Forwards` established. (c) returns `Err(UnsupportedControlFlow)` because there is no positional slot. Pass B therefore claims forwarding through a call that raises `TypeError`.

### 4.2 Boundary-reason priority

**Sites:**
- `finite.rs::refusal_priority:235` (depth < work < node < cond-work < atom < budget < outside < missing = all others).
- `finite.rs::summarize_boundaries:307-324`: `reach_budget` overrides everything; `MissingEvidence`/`CallTransfer` refusals are filtered out; the fallback order is budget > call > outside > unsupported.
- `finite.rs::origin_coverage:2745-2764`: min over refusals ∪ boundary reason, *including* MissingEvidence and CallTransfer; the fallback order is budget > outside(uncertified) > call > no-witness → call.
- `discharge.rs:40`: lowest numeric code.
- `crates/cpg-core/src/behavior.rs::reason_rank:126` (operations): BudgetReached < OverrideDispatch < AmbiguousBinding < UnresolvedTarget < Outside < all others.
- `crates/lctx-analytics/src/summaries.rs:148-153`: BudgetReached first, else the first root's reason, else MissingEvidence.

**Derived?** No. All are independent.

**Concrete divergences (Interface-checked):**
1. **Binding refused `MissingEvidence`.** A local seed refused `MissingEvidence`, on a candidate with `local_through_call = true`. The refusal comes either from `SourceCallBinding` `Err(MissingEvidence)` (`finite.rs:2106-2116`), for example the receiver mismatch in §4.1 (so every method facade `return self._inner(x)`), or from a missing binding row (`:2117-2119`):
   - `summary_boundaries.reason = call_transfer`;
   - `summary_origin_coverage.reason = missing_evidence`, since both have priority 7 and code 7 < 19.
2. **Reach budget with a kernel refusal.** A candidate with `reach_budget = true` and a refusal `ConditionNodeLimit` (for example from a widened condition's diagram):
   - boundary: `budget_reached`;
   - coverage: `condition_node_limit`.
3. **Discharge versus coverage.** Discharge would pick `budget_reached` (code 6) over `summary_depth_limit` (21) if one origin had both. It is latent, because `summary_boundaries` is keyed per `(origin, condition)` and an origin determines its condition.
4. **Operations versus summaries.** `operations.boundary_reason` ranks BudgetReached first; summaries rank SummaryDepthLimit first. Same enum, opposite "most informative" rule.
5. **Native vocabulary.** Exact-input answers return `KernelBoundary` text (`node_limit`, `atom_limit`, …) or `budget_reached` / `conflicting_proof` / `condition_boundary` strings (`python/lctx_semantics/src/lib.rs:1446-1450, 1510-1517`), not `condition_limit()`'s `condition_node_limit` etc. (`summary_contract.rs:205-213`). These are two served spellings of one cause.

### 4.3 Verdict from condition

**Sites:**
- `finite.rs:903-909` (direct: false → skip, true → Established, else Conditional).
- `finite.rs:1434-1438` (`push_finite_path`, via `condition_is_true`).
- `finite.rs:2253-2257` (callee witness consistency).
- `crates/cpg-core/src/behavior.rs:1274-1294`:
  - `Err` → Established becomes `Unknown/BudgetReached`;
  - non-always → Conditional;
  - `approximated()` → `Unknown/MissingEvidence`.
- SQL rules over **DNF text**: `semantic:condition-not-false` (`rules.rs:3564-3570`) and `semantic:unreachable-not-established` (`rules.rs:3601-3612`, on `conditions.encoding = 'false'`).
- Native: a positive verdict needs a non-false diagram; the check does not require Established ⇔ true (`python/lctx_semantics/src/lib.rs:478-493`).

**Derived?** No. The two finite sites share code paths; the others are independent.

**Divergences (Interface-checked):**
1. **Kernel-bounded condition.** Finite refuses with the mapped kernel reason (e.g. `condition_atom_limit`). `behaviors` says `budget_reached` whatever the kernel cause.
2. **Approximated condition.** Finite refuses `outside_provider_model` unless certified. `behaviors` says `missing_evidence`.
3. **Native.** It accepts `established` with a non-true condition, which the producer never emits. Only publication validation (`validate.rs:659`) protects this.
4. **DNF-reading rules.** They cannot see a BDD-false condition whose DNF is `over_budget`.

### 4.4 Discharge validity

**Sites:**
- **Producer:** `discharge.rs::Decisions::from_outcome:35`. Proved only if an Established or Conditional summary cites the origin **and** `summary_boundaries` has no row for it. `grade:77` requires all members to be proved.
- **Validator:** `validate.rs:743-767` reuses `Decisions` and `grade` on the re-derived outcome. **Mechanically derived ✔.**
- **SQL rules:** `rules.rs:3469-3520`:
  - `call-transfer-discharged`: a non-unknown call claim needs all members proved;
  - `discharge-cites-summary`;
  - `discharge-sibling-closure`: re-derives membership by the sink-span join.
- **Native:** `python/lctx_semantics/src/lib.rs::admit_discharges:1263-1293`. A proved row must cite a loaded positive summary of the same origin; an open row needs a reason; duplicates are refused.

**Divergence (Interface-checked).** A `Proved` row for origin O citing an established summary of O, while `summary_boundaries` also has a row for O (a refused sibling alternative):
- producer and validator: Open;
- SQL rules and native: **accept**.

Sibling closure is SQL-only; native does not check it.

### 4.5 Coverage production

**Sites:**
- `completion.rs::coverage:85`: subject ExecutionSite × {Completion, Exception}, condition always, default reason `MissingEvidence`. Exception coverage with `witness_count = 0` claims an empty escaping-exception domain.
- `finite.rs::origin_coverage:2684`: ValueOrigin × Value.

**Derived?** Shared only `SummarySubject::columns` and the row type.

**Divergence.** Different default-reason chains (§4.2). **Exposure: none.** No consumer reads coverage (§0.5). `coverage_domain` (`summary_contract.rs:48-59`) is used only in a test (`completion.rs:2419`).

### 4.6 DNF versus BDD

**Sites:**
- `BoundedCondition` (`condition_kernel.rs:794-987`) runs `and/or/not` on the legacy DNF and the diagram in parallel.
- `given()` has two algorithms with fallback (`:893-940`).
- `encode()` returns the DNF (`:951-953`); `id()` returns the BDD root id, or a boundary hash that includes the DNF text (`:955-961`).
- The DNF budget is 16 conjunctions × 8 literals, else `over_budget` (`crates/cpg-schema/src/condition.rs:38-40, 560-571`).

**Readers of DNF strings:**
- `flow_model.rs:1360` parses provider `conditions.encoding` into the display half only.
- Produced as text: `value_flows.condition`, `raise_sites.condition`, `ambient_reads.condition`, `behaviors.condition` (`crates/cpg-core/src/behavior.rs:1288`) and `behavior_steps.condition` (`:296-300`).
- Rules: `condition-not-false`, `unreachable-not-established`.

**Is DNF served?** **Yes.** `behaviors.condition` becomes `Fate.condition`, and `ambient_reads.condition` becomes `SettingRead.condition` (bundle `crates/cpg-schema/src/bundle.rs:298-322`). There is no `condition_id` beside them.

**Divergence input (Interface-checked):** a behavior claim merging 17 value-flow rows under distinct single-atom guards (the OR in `claim()`, `crates/cpg-core/src/behavior.rs:851-853`):
- DNF: `over_budget`;
- BDD: fine, so the verdict is `conditional`;
- served: `condition = "over_budget"`.

`atom` / `Atom::parse_encoded` works on atom text, not DNF (`finite.rs:573`, `specialize_local_condition`), so it is unaffected.

### 4.7 Transfer strength

**Sites:**
- Reach composition by `max` (`flow_model.rs:942, 1076`; ordering Identity < Derived < Call, derived `Ord` on `codebook.rs:1403-1407`).
- `FlowTransfer::of` (`codebook.rs:1411-1417`).
- `v2_flows`: identity ∧ ¬captured ∧ ¬approximated ∧ exact → `ValueClass::Parameter`, else `Other` (`crates/cpg-core/src/behavior.rs:200-206`).
- Pass B follows Parameter/Alias only (`pass_b.rs:397-401`).
- `behaviors`: Forwards/Derives and "unchanged"/"computed" (`crates/cpg-core/src/behavior.rs:925-977`).
- Seed SQL filters:
  - direct: `f.identity ∧ v.identity ∧ v.upstream_identity ∧ ¬through_call…` (`crates/cpg-schema/src/behavior.rs:2518-2521`);
  - local: `upstream_identity ∧ local_through_call` (`:2881`).
- `ModelTransferKind`: only Identity is consumed.
- `SummaryFlowKind`: Value only.
- Store association requires `transfer == Identity` (`crates/cpg-core/src/behavior.rs:1106`).

**Derived?** No.

**Divergence (Interface-checked):** `def f(x): return ident(x)` with `ident` summarized `Value`.
- The claim row is `Returns`, `transfer = call`, `value = "computed:<byte>"` (`crates/cpg-core/src/behavior.rs:967-977`).
- The discharge proves it, so `verdict = established` (`:1269-1272`).
- The served fate says **computed**, while the proof says the value is returned unchanged.
- `Call` absorbs `Derived` in `max`. The upstream flag keeps them apart for seeds, but not in `behaviors.transfer`.

### 4.8 Atom hashing

**Sites** (all `IdHasher::new("bdd-atom").str(atom)`):
- `condition_kernel.rs::atom_name:266-271` (BDD variable name);
- `crates/cpg-extract/src/flow.rs:464` (producer of `flow_test_leaves.atom_id`);
- `crates/cpg-schema/src/primitive_theory.rs:211` (link check);
- `python/lctx_semantics/src/lib.rs:596`;
- `crates/cpg-core/src/validate.rs:2134`;
- `primitive_theory.rs:378` is test-only and hashes `atom.encode()`.

**Derived?** No shared function: the same literal recipe appears five times in production.

**Equivalent today:** every site hashes the stored canonical atom text, and hydration rejects `atom.encode() != node.atom` (`condition_kernel.rs:103`). The risk is drift only.

The kind tag is not declared in `id::kind`. That module has 36 constants, against 118 `IdHasher::new("…")` literal call sites (a grep count over `crates/` and `python/` excluding `tests/` directories, including inline test modules).

---

## 5. D1 occurrence identity

**One buffer, two parses, joined by span (Implemented).**
- ty indexes exactly the text Pyrefly parsed (`crates/cpg-extract/src/lib.rs:644, 683-688`).
- `TYPE_CHECKING` is renamed to the same-length `TYPE_CHECKIN_` sentinel, so offsets never move (`crates/cpg-flow/src/lib.rs::rename:287`). A module that already uses the sentinel is refused.
- `cpg-flow` exports only `Span{start, end}`, place strings and `BoundedCondition`s. Scopes are identified by name span (`lib.rs::Walk::scope:551`).
- **All joins are downstream**, on `(module_node_id, bytes)`:
  - **exact equality:** scope to declaration (`flow_model.rs::scope_join:302`), parameter to `bindings` (`:421-434`), argument sink (`:1494-1497`), captured reads, exit sites (`crates/cpg-schema/src/behavior.rs:3339-3355`), expression reads (`behavior.rs::expression_reads_sql:1877`), call links (`derived.rs:1116-1162`);
  - **containment ("innermost region"), reimplemented at least three times:** `flow_model.rs::Regions::at:1243`, `crates/lctx-analytics/src/context_protocol.rs:274-292`, and the pinned-call SQL (`behavior.rs` ~`:2004`).

**Which IDs come from which parse:**

| ID | Recipe | Parse |
|---|---|---|
| `use_id` | `H("flow_use", module, start, end)` | ty span |
| `definition_id` | `H("flow_definition", module, kind, start, end, place)` | ty span and ty place text |
| `fact_id` | `H("fact", run, table, fields…)` | ty |
| `condition_id` | `H("condition-bdd", root)` | ty spans inside atoms |
| atom evaluation identity | `Site{module_key, start, end}` or `Synthetic{module_key, H("flow-synthetic-predicate", Debug FileScopeId, Debug ScopedPredicateId)}` (`predicate.rs:591-605`) | ty; **ty-internal indices leak into atom identity**, so a ty upgrade can change ids on unchanged text (Interface-checked) |

Ruff 0.0.11 / Pyrefly ids (declarations, bindings, syntax nodes, call syntax, arguments) are separate. They meet flow ids only through the span joins above. None of the flow tags is in `id::kind`.

**When a range does not match:**
- Coverage is per module only, with skip counts in a `detail` string (`crates/cpg-extract/src/lib.rs:929-954`).
- Publication-blocking parity rules check use↔reference, definition↔binding and reaching-within-candidates (`rules.rs:2487-2550`), but **only for name places**, and by span, not kind.
- **Silent losses (Interface-checked):**
  - a LEFT JOIN miss leaves `function_node_id`/`parameter_node_id` NULL, so there is no origin (`flow_model.rs:888-905, 1739`);
  - `exit_sites` is an INNER join, so a `return` whose span misses its region disappears;
  - `Walk::region` defaults to `always()` when no range contains the statement (`crates/cpg-flow/src/lib.rs:859`);
  - test-leaf operand misses are counted, not recorded (`crates/cpg-extract/src/flow.rs:474-493`).
- **Good pattern:** `flow_value_call_links` keeps a typed status per step (`crates/cpg-schema/src/derived.rs:1116-1162`).

**Control conditions.**
- They come **only from ty reachability TDDs**:
  - per-use `reachability_constraint` (`lib.rs:767-771`);
  - `range_reachability` for regions (`:846-860`);
  - `predicates()` for tests (`:452`).
- **Narrowing constraints are never read.**
- Lowering: Shannon expansion on `if_true`/`if_false`; `AMBIGUOUS` becomes `always().with_approximation()` (`predicate.rs:575-577`). ty's ambiguous child is not followed. That is sound for runtime Boolean atoms, which is ty's third value collapsed (Interface-checked).
- Deep or subscripted places become opaque atoms (`predicate.rs:92-93`). The runtime view fixes `TYPE_CHECKING` / `sys.version_info` / `sys.platform` / `os.name` when lexically resolved (`predicate.rs::runtime:336`).

**CFG / dominators.** None anywhere in `crates/` or `python/`. The only "dominated" is the (depth, cost) frontier (`worklist.rs:16`). The control model is ty TDDs plus syntax-driven completion (`completion.rs`).

---

## 6. Derivation, support and obligations

### 6.1 Step, support and proof tables

| Table | Premises / roles | Alternatives (OR)? | Sealing |
|---|---|---|---|
| `summary_flow_steps` (`behavior.rs:1496`) | Ordered `(kind, evidence_id, condition_id)`. Kinds are `SummaryFlowStepKind` 0–38 (`codebook.rs:1441-1492`), e.g. RawIdentity, CalleeResolution, ArgumentEvaluation, CallSite, CallTarget, ModelFrameExit, PrecedingCallNormal, ModelRule, DefinitionReaching, ReturnSource, CalleeSummary (a recursive premise), CallerConditionLink/CalleeConditionLink, ParameterBinding, Source*Identity, FinalizerPass, ReturnExit. | **No explicit OR.** Alternatives are separate `summary_id`s per `SemanticFlow`. Dominated witnesses are still emitted. | `summary_id` hashes the ordered steps (`id.rs:271-297`) |
| `return_entry_statuses` / `_steps` (`behavior.rs:592-622`) | per (return site, condition): the entry walk's statement premises | no | `return_completion_certificates.entry_digest` (`completion_proof.rs::proof_digest:32`) |
| `return_exit_statuses` / `_steps` (`behavior.rs:623-660`) | exit walk: frames, finalizer passes | no | `exit_digest` |
| `statement_completions` / `_steps` (`behavior.rs:553-590`) | per statement: CompletionStatement, DefinitionHeaderEvidence, … | no | – (keyed by fact) |
| `expression_evaluations` / `_steps` (`behavior.rs:844-880`) | per operand: status + evidence (`known` check `status <> 2`) | no; short-circuit selects one | `steps_digest` (`frame_exit.rs:35`) |
| `call_executions` / `_steps` (`call_execution.rs:13-45`) | callee, arguments, defaults | no | `H("call-execution", …, default_formals_digest)` (`:61`) |
| `model_frame_exits` / `_arguments` / `_steps` (`frame_exit.rs:29-70`) | argument expressions, return parameter | no | `arguments_digest`, `identity` |
| `source_call_bindings` / `_header_steps` / `_normals`, `source_body_*` (`source_call.rs:38-57`, `source_body.rs:38-64`) | header evidence, body completion | no | `header_digest`, `H("source-call-normal", binding, body, count, kind)` |
| `return_completion_certificates` (`completion_proof.rs:14-54`) | entry and exit proofs | the "exact condition, else sole candidate" rule (`finite.rs:2503-2508`) | counts + both digests |
| `behavior_discharges` (`behavior.rs:1538`) | per member origin: summary or reason | a claim is the **AND over members** | key only |
| `behavior_steps` (`behavior.rs:1184`) | Pass B hop chain with modality + DNF condition | first witness only | – |
| `witnesses` (`findings.rs:130`) | Pass A/B arc paths | `path` index = alternates (Pass A) | finding id hashes the paths (`pass_b.rs:608-619`) |
| `value_flow_contributions` | **No step table.** The reaching chain is implicit; identity covers `(fact, use, source_key, flags)`. | contributions are alternatives; `value_flows` ORs them | `origin_id` recipe |
| Source identity certificates (`parameter_identity.rs:37`, `modeled_identity.rs:27`, `context_value.rs:46`) | lexical reconstruction of a returned read | no | recipe ids |

**Overall shape of support:**
- flat, ordered, conjunctive step lists, sealed into row identity;
- alternatives appear only as sibling rows (no OR node);
- recursion appears only as `CalleeSummary` citations, admitted in dependency order (`finite.rs:2558-2564`);
- the local reaching chain is the one derivation with **no** persisted support: re-derivation (`validate.rs:829-871`) is the only check.

### 6.2 Obligations and unknowns

| Encoding | Where |
|---|---|
| per-use capped reach | `flow_reach_boundaries(reason = BudgetReached)` |
| boundary condition root | `conditions.root_id NULL` + KernelBoundary text |
| per-origin summary residual | `summary_boundaries.reason` |
| claim-specific completeness | `summary_origin_coverage(complete, reason, witness_count, witnesses_omitted)` (unconsumed) |
| per-member discharge | `behavior_discharges(decision Open, reason)` |
| negative claims | `negative_premises(holds, boundary_reason, reason)` |
| status-row refusals | `reason` columns on statement / expression / return-entry / return-exit / call-execution rows |
| compatibility | `value_flow_predecessor_compatibility(compatible_under_atoms?, boundary_reason?)` |
| model endpoints and argument bindings | `status Unknown` + `reason` |
| call-link joins | `flow_value_call_links.status` |
| behaviors and operations | `verdict` / `boundary_reason`; `operations.behavior_status` / `boundary_reason` / `status_reason` |
| per-module flow coverage | `coverage` |
| analysis passes | Pass A/B `stop_reason` |

The `coverage_domain` vocabulary is declared but not enforced. The `SummaryRefusal` list (every reason per alternative) is **not persisted**: it collapses to one boundary reason.

### 6.3 Row counts

Docs-reported, FastMCP behavioral generation, served relations, 2026-09-29, not re-run:

| Relation | Rows |
|---|---|
| `summary_flows` | 23 |
| `summary_flow_steps` | 145 |
| `summary_boundaries` | 3,705 |
| `return_completion_certificates` | 23 |
| `behaviors` | 15,711 |
| `behavior_discharges` | 2,107 |
| `analysis_conditions` | 8,421 (23,942 nodes) |
| `conditions` | 14,120 (28,856 nodes) |
| `flow_test_leaves` | 6,882 |
| `flow_test_value_links` | 178 |
| `model_frame_exits` | 1 |
| all `source_*` proof and identity files | **0** |

The catalog profile has 0 for all of these (`catalog-current-status.json`). The proportion proved among the 2,107 discharges is not recorded in the docs.

---

## 7. External-review claim ledger (behavioral)

| Claim | Status | Evidence |
|---|---|---|
| **E7** "worklist separates semantic keys from witness IDs and retains nondominated depth/cost representatives" | **Confirmed, refined** | `worklist.rs::Frontier:21-67` (key K, `Witness{id, depth, cost}`, insert rejects if dominated or equal on both, at most MAX_PATH_DEPTH+1 per key). `SemanticFlow` (`finite.rs:377-392`) excludes `summary_id` and path length. **Refinements:** (1) used by exactly one engine, the local-call composition; (2) the key includes verdict, refusal, approximated and the display path strings, but not the call site; (3) dominated witnesses are still emitted as rows, and only propagation is pruned; (4) a witness over the cost bound (64, expanded source-call length) is silently non-propagating, and its callers are refused `call_transfer` rather than `summary_proof_limit` (Interface-checked); (5) omission is disclosed only in unconsumed coverage. |
| **E8** "finite-summary infrastructure preserves parameter origins, conditions, call bindings, return-site evidence, refusals and coverage" | **Refined** | **Preserved:** origin (`source_origin_id`, `parameter_node_id`), caller condition plus callee condition in steps, bindings (ParameterBinding, CallSite, CallTarget, CalleeResolution, ArgumentEvaluation), return site (ReturnExit, finalizers, entry proof, certificates). **Refusals:** collapsed to one reason per origin in `summary_boundaries` (§4.2 divergence); the full `SummaryRefusal` list is not persisted. **Coverage:** persisted but consumed by nothing. **Scope:** Parameter → ReturnValue, Value channel, parameter origins only. Field, captured and non-return origins are never candidates, so discharge defaults them to Open `call_transfer` (`discharge.rs:59-72`). Only `return callee(...)` composes. Pilot: 23 flows against 3,705 boundaries. |
| **E9** "condition checks distinguish compatibility under declared atoms from selecting a reaching definition or proving a modeled call completes" | **Confirmed, refined** | Module doc `crates/lctx-analytics/src/summaries.rs:1-4`. `predecessor_compatibility:126-172` returns `Option<bool>` or a boundary; loop-carried gives `UnsupportedControlFlow`. **Refinements:** (1) compatibility is propositional over independent atoms; the BDD has no theory, so `equals(p,1) ∧ equals(p,2)` is "compatible"; (2) it is consumed only as a SQL join input (`behavior.rs::modeled_assignment_return_paths:2444`); (3) admission then requires **implication** (`finite.rs:1893-1919`) plus the modeled-call proof and completion certificates, so the distinction does hold in code. |
| **E10** "flow provider preserves uses, definitions, guarded reaching relationships and value sources" | **Refined** | All are emitted with path conditions (`crates/cpg-flow/src/lib.rs`, `crates/cpg-extract/src/flow.rs`). **Not preserved:** narrowing constraints; definition-side conditions on loop-carried reaches; ty ambiguity (collapsed to true + approximated); exact-false branches (counted only); structured places (strings); deep or subscripted atom places (opaque); constant sources; callee semantics (`through_call` is syntactic); type-parameter scopes; per-occurrence join completeness for non-name places. |
| **E11** "the code builds a caller–callee graph, computes SCCs and a deterministic callee-first schedule" | **Confirmed, refined** | `crates/cpg-core/src/summaries.rs::call_components:222`: arcs from Pysa `call_targets` between Function/AsyncFunction declarations, excluding higher-order and annotation targets. `crates/lctx-analytics/src/summaries.rs::call_components:33-124`: petgraph Kosaraju plus canonical Kahn. Order-independence tests are at `:180-235`. **Refinements:** consumed by exactly one engine (the local-seed sort and groups, `finite.rs:2005-2044`); the `recursive` flag is unused; `execution.rs` hard-codes a single source-call level instead of using the schedule. |

---

## 8. Behavioral relations served today

Code at `35afc09`. Serving requires `--profile behavioral`; under the default catalog profile every row below projects 0 rows (`serving_projection.rs::relation_requested:177-192`).

| Served relation | Bundle / PostgreSQL | Native | MCP tool and field |
|---|---|---|---|
| `behaviors` | ✔ | – | `get_operation` → `Fate{verdict, boundary_reason, condition (DNF text), conditional, transfer, value, discharges[]}`; relationships (delegates, supplies_literal, hands_off_to, takes_from) |
| `behavior_discharges` | ✔ | ✔ (validation only) | `get_operation` → `Fate.discharges[]` |
| `operations` (`behavior_status`, `boundary_reason`, `status_reason`) | ✔ | ✔ (node_id) | `OperationPacket` |
| `operation_facets`, `operation_facet_status` | ✔ | – | `find_operations`, `search_operations`, `compare_operations`, `browse_library`, `get_operation` |
| `ambient_reads` | ✔ (`condition_id` dropped) | – | `get_operation` → `SettingRead.condition` (DNF) |
| `singletons` | ✔ | – | `get_operation` |
| `negative_premises` as `place_claims` (Field/Global) | ✔ | – | `FieldRecord.never_read` |
| `summary_flows`, `summary_flow_steps`, `summary_boundaries` | ✔ | ✔ | `inspect_value_paths` → `ValuePath{summary_id, source_verdict, condition_id, steps[], boundary_reason}`, `OpenBoundary` |
| `conditions`, `condition_nodes`, `analysis_conditions`, `analysis_condition_nodes` | ✔ | ✔ | opaque `condition_id` only |
| `flow_test_leaves`, `flow_test_value_links` | ✔ | ✔ | `inspect_value_paths` → `ValuePath.value_links` |
| summary proof and identity files (`return_completion_certificates`, `source_*`, `model_frame_exit*`, `model_context_protocols`) | ✔ (filtered to cited rows) | ✔ | load-or-refuse admission only |
| `witnesses` | only as `support_witnesses` when a brief cites them | – | `get_capability` |

**Not served:**
- `value_flows`, `value_flow_contributions`, `flow_reach_boundaries`, and all `flow_*` provider tables except test leaves and links;
- `behavior_steps`, `argument_flows`, `guards`, `parameter_reads`, `handoffs` and `delegations` as tables;
- `summary_origin_coverage`, `summary_components`;
- the completion, evaluation and execution tables;
- all `model_*` / `modeled_*` tables.

**Exposure consequences for the defects above:**
- §4.2 divergence #1: the served `OpenBoundary.reason` follows `summary_boundaries`. Coverage is not served, so the disagreement is invisible to agents but decides discharge.
- §4.6 (DNF `over_budget`) and §4.7 (`computed` on a discharged identity): **directly served** in `get_operation` fates.
- §4.4 (weaker native and SQL discharge checks): only matters if a generation bypasses `validate.rs`.
- §4.2 #5 (KernelBoundary text): served through `inspect_value_paths` exact-input answers.
- `ValuePath.steps[].evidence_id` cannot be dereferenced through `get_evidence` (span/scenario/deployment references only, `crates/cpg-schema/src/evidence.rs:51-55`).

---

## 9. Not verified / open

- The proportion of the 2,107 `behavior_discharges` that are proved, and how the 3,705 `summary_boundaries` are distributed by reason, would need a read-only `lctx query` on `build/store`. This was not run.
- The receiver-shift refusal (§4.1) and the proof-limit misattribution (§2.3 item 5) are reasoned from code. Neither has a test found.
- No fixture has `x = f(a); return f(b)`, a literal `timeout`/`title` constructor, or a >16-disjunct served condition.
