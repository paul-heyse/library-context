# Nested pass finalizers — core 3.0 calibration review

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Commit `e97e6ba27a757dbbf2cd58223c1cade82a8ac277` against parent `e76c9512706f54310f3af27ce82685b8fb1ae840`: ordered nested pass finalizers, ADR-0037. Source citations below are **at e97e6ba**, unless marked otherwise; current working-tree line numbers are not substitutes. |
| Standard | Current working-tree core **3.0**, code-intelligence profile **1.1**, and library-context binding, loaded through [standard.toml](../design_principles/standard.toml); current `design-review` and companion profile skills. This is a fresh calibration assessment, not a reclassification of an older review. |
| Tier · purpose | **Change · conformance**, against ADR-0037 and the affected accepted contracts at the target revision. |
| Reviewer · date | Independent Codex review agent · **2026-09-25**. |
| Maturity and outcome | Incomplete Stage 3. **Revise** the adjacent native proof-consumption boundary. The compiler's ordered pass derivation is coherent in the inspected slice, but its output cannot traverse the existing native loader. |
| Supported scope | Pending direct and exact modeled returns through literal pass-only finalizers; ordered finite proof construction, identity, publication reconstruction, and the immediately adjacent bundle/native consumer. Assignment and acyclic wrapper producers were inspected for their use of the shared finalizer attachment. |
| Baseline and delta | Parent supports a sole pass finalizer. The change admits a bounded chain, queries all pass witnesses, and bumps compiler output version 69 → 70. It changes neither the Arrow layout nor the existing `FinalizerPass` kind. The native vocabulary defect F01 exists in the parent as well; this change newly routes nested proofs into it. |
| Expected changes | Add another nesting level without changing proof meaning; carry the resulting direct or modeled proof through native loading and rendering. The Stage 3 plan's exit/fate, finite proof, and FORMAT 7 work make these immediate consumers, not hypothetical extensibility. |
| Method and coverage | Read-only `git show`, `git diff`, and targeted `git grep`; current AGENTS, STATUS, standard and skills were read. No checkout switch, other calibration review, production edit, test execution or integrated gate. Historical source/tests were inspected, not executed. Current STATUS describes later work and is used only to avoid confusing the historical target with the current checkout. |

**Evidence strength:** code existence is **Implemented**; contract and propagation conclusions are **Interface-checked**, dated 2026-09-25. No new **Tested** or **Measured** semantic claim is made.

### Affected owners, contracts and composition

| Owner | Responsibility and contract | Dependencies and consumers |
|---|---|---|
| `cpg-schema::behavior` | Owns which pending frames are discharged and the ordered source witnesses. `return_exit_statuses` is a local exit candidate, explicitly not a witness that the return expression or preceding work completes. | Ruff syntax and ty exit regions → safe status → ordered pass query → summary producers. `behavior.rs:533–555,2795–2898`. |
| `cpg-core::summaries` | Composes finite path evidence with condition and call evidence. Pass steps are grouped by return fact, sorted by ordinal, and checked for dense order. | `return_pass_steps` supplies direct paths and `push_finite_path`; modeled, assignment and local-call paths share the latter. `summaries.rs:196–224,270–354,415–481,507–605,661–687`. |
| `cpg-schema::id` and codebook | Own canonical proof identity and the closed proof-kind vocabulary. The hash includes ordered kind, evidence and condition triples. | Publisher, validator, bundle and native loader should consume these meanings. `id.rs:212–249`; `codebook.rs:1332–1347`. |
| `cpg-core::validate` / attempt | Reconstruct source statuses and complete flow/step rows; compare stored and expected rows before publication. | Same production reconstruction, not a separately authored semantic validator. `validate.rs:113–127,403–439,581–594`; `attempt.rs:885–911`. |
| Bundle → Python generation → native executor → value-path rendering | Preserve canonical rows, reject invalid input, then expose ordered proofs for one generation. | Bundle derives kind text from the codebook; generation transports it unchanged. The native executor independently re-lists allowed texts and rejects `finalizer_pass` (F01). `bundle.rs:213–227`; `generation.py:413–444`; native `lib.rs:244–275`; `value_paths.py:163–175`. |

**CI fact and fidelity scope.** Syntax nodes are attributed Ruff 0.0.11 observations; exit regions are attributed ty 0.0.14 flow facts, joined by source range and function (`behavior.rs:2748–2782`; pins at this revision in `docs/pins.md`, Analyzers). Statuses and pass sequences are **derived**, not runtime observations. Every pass step carries a source fact and return condition. Finite paths retain their verdict and approximation flag; their identity changes when ordered evidence changes. The local proof does not establish whole-operation normal completion.

**Changed analysis contract.** The ancestry universe is the return's same-function parent chain; it is not a call-graph reachability proof. Each controlling frame must have exactly one direct `StmtPass` final-body child. An unsafe frame withholds admission, and a walk capped at 128 records `BudgetReached`. The pass query starts only from statuses with no reason, excludes the finalizer currently containing the return, and orders pending frames by increasing ancestry depth. The compiler persists witnesses in `summary_flow_steps`, without another independently editable pass table (`behavior.rs:1593,2800–2898`).

### Change scenarios

| Scenario | Owner and contract change | Observed propagation / context needed | Judgment |
|---|---|---|---|
| Add a third pass-only nesting level to the existing direct or modeled return shape | Source input changes; no new semantic category or schema is needed. | The same bounded ancestry query supplies another ordered witness. Direct assembly appends it; modeled/assignment/wrapper assembly inserts it before `ReturnExit`; identity and reconstruction consume the sequence. A focused fixture would settle execution evidence. | **Interface-checked:** compiler propagation is localized; no new arity branch is needed. Not executed. |
| Load and render a newly admitted nested proof through the existing FORMAT 7 path | No domain contract change is needed: `FinalizerPass` already exists. | Bundle uses `SummaryFlowStepKind` text. Python passes every row to `SemanticExecutor`. Its private string whitelist rejects the emitted kind before rendering can consume it. A semantic vocabulary addition currently requires an independent native edit. | **Interface-checked:** concrete contract divergence and failed composition, F01. |
| Extend to an effectful finalizer or an unproved predecessor call | This is a new execution-proof obligation, not another instance of pass nesting. | ADR-0037 explicitly withholds that guarantee. The current pass-specific query cannot be treated as the future action/fate contract. | **Proposed / excluded:** revisit the owning L2/L3 contract when those paths are scheduled. No architectural acceptance for that scenario. |

## 6. Correctness and fidelity gates

Gate verdicts are inspection judgments, not command outcomes.

| Gate | Verdict | Evidence and scope | Action |
|---|---|---|---|
| G1 Authority | **fail** | Proof kinds are declared in `codebook.rs:1335–1346` and independently redefined by native `lib.rs:250–253`; they already disagree. | F01. |
| G2 Semantic fidelity | **pass**, local compiler contract | Singular nullable pass fields retain their one-frame meaning; multi-frame evidence is represented by ordered typed rows. Unsafe/capped frames are not admitted as safe (`behavior.rs:2843–2863`). | Keep the local-exit versus completed-execution distinction. Native interoperability fails separately under G6. |
| G3 Validity | **pass**, inspected publication path | Status and full proof equality reconstruction rejects altered source evidence/order; dense ordinals are checked before composition (`summaries.rs:217–221`; `validate.rs:403–439,581–594`). | A shared mistake remains possible; reconstruction is not an independent semantic oracle. |
| G4 Hidden behavior | **pass**, changed path | The query and proof assembly read explicit session relations and construct rows. Native rejection is explicit; no new ambient read or hidden effect was found in the changed path. | Broader analyzer hermeticity was not re-audited. |
| G5 Consistency and recovery | **pass**, changed ancestry/composition scope | Capped ancestry produces a reason and cannot seed pass reconstruction; native failure aborts loading rather than presenting a partial index as complete. | Crash/retry qualification and whole-program resource bounds were not reassessed. |
| G6 Transformation and reuse | **fail** | A valid canonical proof exported unchanged to the native boundary is rejected because its category is absent from the second vocabulary. Ordered canonical identity itself is preserved (`id.rs:233–248`). | F01. |
| G7 Truthful capability claims | **fail** for the adjacent native route | The inspected direct/nested producer and bundle have an implementation route; loading those outputs into the advertised proof consumer does not. ADR-0037's narrower local-exit claim is properly bounded. | F01; do not describe the complete route as qualified. |
| G8 Library leverage | **pass** | Relational ancestry, counting and ordering use the existing DataFusion route; hashing uses the canonical recipe. No replacement generic algorithm or framework is introduced. | See §8. |
| CI-G1 Fidelity | **pass**, local exit scope | The pass fact remains source evidence; status safety is conditional on a pending return. The code and ADR do not turn it into a whole-operation completion guarantee. | General predecessor completion and recursive execution remain outside this acceptance. |
| CI-G2 Evidence closure | **pass**, canonical publication scope | Pass references are checked against syntax facts and reconstructed into exact ordered proofs (`rules.rs:207–215`; `validate.rs:427–436`). | Native loading is blocked by F01; complete served per-step source-span resolution is not established here. |
| CI-G3 Evaluation integrity | **n.a.** to production delta | The added oracle test executes separate generated source; no evaluation reference is added to compiler inputs. The gold pipeline was not changed or audited. | No evaluation-quality claim. |

## 7. Findings and applicability

<a id="F01"></a>

| Field | F01 — Native proof-kind authority diverges from the canonical codebook |
|---|---|
| Priority / origin | **High, inherited consumer defect.** Present in `e76c951` as well as `e97e6ba`; the newly admitted nested proofs exercise it. This is not attributed as a newly introduced whitelist omission. |
| Finding and consequence | `FinalizerPass = 10 => "finalizer_pass"` is a supported canonical kind. Both changed producers emit it and the bundle serializes its codebook text. Native `SemanticExecutor::new` maintains a separate list of ten kind strings that omits it. For otherwise valid rows containing this kind, the constructor deterministically reaches `ValueError("invalid summary proof step")`; the generation loader wraps that as `GenerationError("invalid semantic index: …")`. **An entire generation containing the proof fails to load**, not merely the affected value-path query. This consequence follows from the branch and call chain; no runtime reproduction is claimed. |
| Principles / judgments / gates | FP-02, FP-03, FP-04; DP-01, DP-15, DP-24; A1, A2, A3; G1, G6, G7. |
| Evidence | `crates/cpg-schema/src/codebook.rs:1332–1347`; `crates/cpg-core/src/summaries.rs:311–317,424–434`; `crates/cpg-core/src/bundle.rs:223–227`; `python/lctx_mcp/src/lctx_mcp/generation.py:425–427,443–444`; `python/lctx_semantics/src/lib.rs:244–255`. Parent native lines 248–255 contain the same omission. |
| Architectural consequence | The ordinary scenario “consume another instance of an existing proof kind” requires knowledge of an unrelated consumer's private category list. An added proof kind can satisfy schema/publication checks while the consumer's independent definition drifts. Passing local compiler tests cannot protect this boundary. |
| Correction / owner | **Native semantic loader**, using the **cpg-schema codebook** as vocabulary authority. The native crate already depends on cpg-schema. Decode or validate the text through `SummaryFlowStepKind`'s declared values and use the resulting typed value where behavior depends on a kind; remove the independent string whitelist. `Codebook::all()` and `text()` already exist (`codebook.rs:8–16,41–48`). An intentionally unsupported category would need an explicit consumer capability contract, not an accidental omission. No new provider abstraction is needed. |
| Closure evidence | Inspect that native kind recognition derives from the codebook, then run a focused real native load/query using compiler-produced direct and nested-finalizer rows. It must preserve both pass citations and their order; unknown kind text must still be rejected. This protects an actual cross-boundary regression. A fabricated fixture that omits the new kind cannot close it. |
| Disposition owner | **Transferred 2026-09-25** to [active plan ARC-01](../../plans/behavioral-model-forward-plan_2026-09-24.md#ARC-01), grouped with architecture-calibration F01 after current-source inspection. That row owns current status, sequencing and closure evidence. This historical review's **Revise** decision and reviewed revision are unchanged. |

Applicable foundations and supporting rules: **FP-01/FP-05/FP-06 satisfied within the compiler slice** by the owned relational derivation, explicit evidence/order and bounded immutable composition inputs. **FP-02/FP-03/FP-04 violated across the native boundary**, as F01 demonstrates. DP-03/04/08/11/12/18/21/23 are satisfied at the inspected local derivation/publication boundary; DP-01/15/24 fail at the consumer boundary. DP-13/14/16 are satisfied for the changed algorithmic mechanism. No claim is made for unrelated effects, heuristics, graph projections, acquisition or whole-system lifecycle contracts.

The direct producer and `push_finite_path` attach pass sequences differently because the direct raw-identity and modeled proofs have different existing shapes. Both consume the same pass index. That duplication alone is **not** a second finalizer-safety authority and is not a separate finding. Likewise, the second bounded ancestry query depends on the safe status; it does not independently decide that an effectful frame is safe.

### Verification and uncertainty

| Evidence / command | Outcome and meaning |
|---|---|
| `git diff e97e6ba^ e97e6ba -- <affected paths>`; `git show e97e6ba:<cited path>`; `git show e97e6ba^:python/lctx_semantics/src/lib.rs` | **passed**, source inspection on 2026-09-25. Establishes implementation and inherited-versus-new distinction only. |
| `git diff e97e6ba^ e97e6ba --check` | **passed**, whitespace check only, 2026-09-25. |
| ADR-0037:42–45 and commit title | Historical author-reported focused test success dated 2026-09-25. The ADR does not retain a precise command; this review does not upgrade that report into an independently verified result. |
| `cargo nextest run --release -p cpg-core --test compile -E 'test(nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries) \| test(pinned_identity_models_require_and_publish_their_real_formals)'` | **not_run**. Inspected test bodies cover direct/modeled ordering and missing/reversed pass evidence (`compile.rs:647–788,1787–1807`). The compiler test uses a `FakeEmbedder` for unrelated analytics; it is not a full product receipt. |
| `uv run pytest tests/scripts/test_flow_soundness.py -k pending_value_return_through_nested_inert_finalizers -q` | **not_run**. Inspected generated-source oracle checks observed return/value-region admission (`test_flow_soundness.py:423–438`); it does not validate final `summary_flow_steps` or native loading. |
| `just test-all`; `just pilot` | **not_run**, per task scope and integrated-acceptance timing. |

No current-tree correction, historical executable reproduction, three-level runtime case, capped-ancestry runtime case, nested assignment/wrapper fixture, full served evidence lookup, performance result, complete predecessor/callee completion proof, or Stage 3 acceptance was established. These limits are not converted into semantic passes. The source-level F01 mismatch needs no additional test to establish the divergent vocabulary; a meaningful cross-boundary test is part of its eventual correction.

## 8. Library fit and total complexity

| Capability / owner | Candidates and fit | Choice, burden and limit |
|---|---|---|
| Bounded ancestry and ordered pass witnesses / cpg-schema | Existing DataFusion SQL route versus a separate Rust graph walk. The inspected source uses recursive CTEs, aggregate qualification and window ordering; DataFusion 55.1.0 is the declared pin at this revision (`docs/pins.md`, Rust family). | Retain the relational implementation: its inputs already are relations, publication uses the same query, and a second graph representation would add mappings without a new topology requirement. This is an inspected existing integration, not a fresh qualification of upstream APIs. |
| Proof assembly / cpg-core | Existing ordered proof vectors and canonical hash versus a new generalized exit workflow. | Reuse the current proof contract. The pass-specific domain rule is small and owned; a framework is not justified by adding one nesting level. Effectful or nonlinear exit composition is the explicit revisit trigger in ADR-0037. |
| Kind validation / native loader | Existing cpg-schema `Codebook` values versus a private text whitelist. | Use the already linked codebook and delete the second vocabulary (F01). This removes an independent semantic decision without adding a library, lifecycle or configuration surface. |

The simplest compiler alternative is to retain the parent's unknown outcome for nested frames; it preserves sound local boundaries but forfeits the requested finite proof. A boolean-only safe flag loses frame evidence and ordered identity. ADR-0037's sequence is the appropriate inspected compiler mechanism. Its consumer boundary still needs correction. No dependency upgrade, new library API or broader technology selection is proposed.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and required action |
|---|---|---|
| A1 Localize change | **violated across the reviewed boundary** | A third frame is local in the compiler, but an already declared proof category requires an independent native vocabulary edit. Remove the second category authority (F01). |
| A2 Encode meaning structurally | **violated across the reviewed boundary** | Canonical typed steps, ordered identities and shared reconstruction are present. The native contract independently defines which kinds exist and has diverged. F01 prevents overall satisfaction. |
| A3 Extend through composition | **violated across the reviewed boundary** | Direct, modeled, assignment and wrapper paths compose the pass query; bundle and native loading cannot compose for its existing kind. Correct F01 before accepting that route. |

**Bounded change decision: Revise.** The source-level ordered pass extension is coherent under its stated pending-return assumptions. Acceptance for the requested scope includes the adjacent finite-proof consumer, where a demonstrated inherited defect rejects the new output. The narrow compiler strengths do not offset G1/G6/G7 or A1–A3.

**Enclosing architecture: needs revision at the inspected compiler-to-native contract; otherwise unresolved.** This review does not certify assembled L2/L3, general normal completion, effects/resources/callbacks, SCC closure, native evidence completeness or product qualification. It makes no claim about whether later commits corrected F01.

The next action belongs to the **native semantic loader owner**: derive proof-kind recognition from cpg-schema and exercise a real compiler-produced finalizer proof through native loading. Keep this review as dated evidence; assign current execution disposition through the active plan during calibration triage.
