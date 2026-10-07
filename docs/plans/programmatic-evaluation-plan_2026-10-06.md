# Primary programmatic evaluation and grounded outer feedback

**Primary loop implemented; selected interface integration pending, 2026-10-06.** Supporting plan for EV1–EV5
in the [combined coordinator](evidence-retrieval-and-evaluation-plan_2026-10-06.md).
It realizes the [review's](../design_review/reviews/design_review_evidence-retrieval-and-programmatic-evaluation_2026-10-06.md)
information-sufficiency recommendations, F01/F03 and approved RC01/RC02/RC04. The coordinator
alone owns their current finding disposition and cross-plan package state.

Programmatic evaluation repeatedly runs declared tasks through production operations, judges actual
information with independent expectations, shrinks failures and diagnoses losses. Agentic evaluation
is an outer loop, providing grounded feedback to both the system and these task models/judgments.
It is not a prerequisite for the inner loop or the sole expected-answer engine.

## 1. Baseline, owners and useful first capability

Inspected f2064111, 2026-10-06. Existing scripts/product_eval.py is a hash-bound agent A/B/C
trial harness with protected confirmation inputs and explicit blocked parity. Its isolation and
receipts are useful outer-study foundations; changing its heading cannot make it the new loop.
Existing pure model validators, generated soundness cases, Hypothesis, numerical/graph kernels,
serialization and source observers can support focused implementation. Their historical passes
are not newly run or complete evaluator qualification.

Place typed private evaluation contracts and pure judgment kernels in an offline Rust evaluation
module/crate outside production semantic declarations and the CLI/PyO3 serving dependency closure.
Use a small Python development runner for Hypothesis, experiment I/O and later optional numeric/
optimizer tooling. It consumes generated evaluator wire contracts instead of defining another
string-key semantic model. A narrow executable JSON input/output seam allows Python to call the
Rust kernel without requiring native Python adapters or a database. Use one long-lived bounded
JSONL worker per campaign batch, with batched case input/output, bounded queues, explicit per-case
status and process cancellation; do not launch a process for each Hypothesis example or task.
Share immutable admitted task inventories/numerical blocks within the campaign. Small Rust-native
property controls can call the pure kernel directly. Do not expose private tasks or witness
expectations as MCP tools or canonical library records.

First useful milestone: independent finite cases plus real final-renderer observations produce
per-task sufficient/insufficient/inconclusive results and minimized information-loss cases.
This needs no live embeddings, loaded solver, native activation or paid agent. It establishes
kernel/local rendering behavior only. Subsequent native journeys use actual persistent fixtures
and public serving interfaces; fixtures cannot stand in for runtime provider readiness.

## 2. Task, observation and judgment contracts

Conceptual products below carry required meanings, not a type/table for each row:

| Product | Required inputs/output meaning |
|---|---|
| EvaluationTask | Public request/context, information question, allowed visible follow-ups, packet/journey envelope, supported task class and private independent expectation |
| OracleBasis | Source/fixture/admitted-fact/natural-positive/runtime basis, exact input/qualification, scope/completeness, model/revision and unknown limits |
| Observation | Exact final structured/text bytes plus publicly visible references, source maps, qualifiers, omissions and actual expansion transcript |
| Witness expression | Leaves matching meaningful observations to independent predicates/anchors; All/Any/Exists with compatible contextual assignments |
| Judgment | Applicability/model status, observed epistemic outcome, sufficient witness or insufficiency reason, and separate operation failure |
| Experiment | Frozen task/split/oracle/judgment/observation/numeric definitions and baseline/candidate realization/policy/settings; comparison outputs |
| Feedback proposal | Grounded outer observation, cause classification, independent basis and proposed system change or evaluator revision |

Public production EvidenceDemand is separate from EvaluationTask: it requests information, without
expected identities, private answers, countermodel worlds or mutant labels. The runner passes only
the public projection. Admission rejects private fields at the production boundary rather than
trusting callers to omit them. No oracle-derived text is embedded or inserted into retrieval.

Task contexts include only relevant distinctions: pinned release, subject/operation/variant,
configuration, setup/deployment and qualification scope. A source expression for a declared default
is not an effective default under every override or runtime state. Unsupported distinctions remain
unsupported; they do not become false expected answers.

Judge **immediate delivered sufficiency** separately from **bounded-expandable sufficiency**.
Immediate observation contains final emitted fields/spans and their readable interpretation;
opaque IDs, checksums and metadata labels cannot substitute for omitted condition/default text.
Delivery maps are checked against independent anchors and bytes; an incorrect but schema-valid
map must fail. The evaluator does not reconstruct expected text by running the production renderer.

Expandable evaluation uses only supported public calls, visible references and fixed total call/
byte/token limits. Initial policy deterministically follows relevant visible evidence/section
references in declared order; it cannot use private expected-anchor IDs to choose calls. Pin the
whole journey to its realization, record actual attempted calls and distinguish stale/refused/
budget-exhausted from absent evidence. A reference that cannot be followed is not delivered content.

## 3. EV1 — Context-compatible witnesses and finite worlds

Witness leaves return sets/relations of independently supported assignments, not a role-hit Boolean.
All naturally joins assignments on shared context variables; Any unions valid alternatives;
Exists projects context only after the complete conjunction. Thus one signature/default/example
context must satisfy the task, rather than a separate existential overload for each role.
Retain original qualifications, candidate status and compatible release/setup. Multiple source,
contract or qualified-result alternatives may be sufficient; exact anchor equality alone cannot
reject a genuinely equivalent independently grounded route.

Implement straightforward finite sets and relational joins first, with a separate tiny exhaustive
reference and hand-derived examples. Nominal task/evidence universes own bitset mappings and lengths.
Do not use runtime EvaluationAtom IDs or production requirement classifiers as expected witnesses.
Mandatory interpretation dependencies are part of the observed witness, not presumed background.

Positive task requirements cannot be an accidentally empty expression. Intentional no-demand/
NotApplicable tasks have explicit meaning. Missing inventory/completeness is not a successful
empty domain. Expected-failure, skipped, unknown and authored examples remain distinct.

For a supported countermodel class:

1. Independently define finite worlds, public task input/context, observable information and answer.
   Observations may constrain only readable delivered information and declared allowed interpretation.
2. Find at least one admissible world agreeing with the task and observations. None means an
   inconsistent oracle/observation model, not vacuous sufficiency.
3. If two matching worlds answer differently, return an insufficiency witness naming the missing
   distinction. If every world in the declared complete class answers alike, report scoped
   determinacy; no unrestricted program-correctness claim follows.
4. Incomplete/unsupported world semantics, timeout or refusal remain inconclusive with reasons.

Worlds share the task input/context before comparing answers. IDs do not identify behavior.
The None-override versus truthiness-override twin-program case must remain ambiguous from signatures
and hidden IDs alone, and become determinate only when actual distinguishing source/contract
information is present. Conflicting qualified documents are alternatives/uncertainty, not blindly
conjoined axioms yielding false inconsistency.

Keep applicability, epistemic result and execution status separate. Results distinguish
sufficient, insufficient, model-relative unknown, NotApplicable, incomplete/unsupported oracle,
inconsistent model, inventory/budget infeasibility, and execution failure. Do not coerce these
into numeric zero or include unscorable tasks in a hidden denominator.

## 4. EV2 — Independent populations and evaluator sensitivity

| Population | Independent basis and supported claim | Limits |
|---|---|---|
| Hand-authored finite constructive cases | Tiny source/task/world definitions with independently enumerated answers | Explicit complete finite class, including revealing negative and alternate-positive cases |
| Generated miniature libraries | Grammar/factor assignments produce source plus separate expected model; production compilation supplies actuals | Expected logic cannot call the producer/classifier under test; grammar-family splits prevent template leakage |
| Admitted-fact references | Exact independently expressed queries over admitted inventory | Retrieval/selection relative to inventory, never extraction completeness |
| Natural source/doc/example pairs | Source-grounded positive evidence and contextual applicability | Missing labels are unjudged, not negatives; no invented exhaustive relevance set |
| Isolated generated executable programs | Concrete supported CPython observations with declared inputs/environment | Only newly generated execution cases run; repository fixtures and analyzed libraries remain input data; one execution is not unexecuted-path proof |

Freeze split keys before search: common source/grammar templates, semantic factors and release/
corpus families group together. Development is available for optimization; validation diagnoses
generalization; protected confirmation/gold/heldout remains isolated under current admission.
Do not open sealed plaintext or silently relabel the old blocked confirmation candidate.

Establish source-complete, admitted-inventory, retained-candidate and actual-packet ceilings.
For small finite cases, enumerate rendered context-closed packet alternatives, count actual bytes/
tokens with shared setup/dedup/separators, and find feasible sufficient alternatives. A simple
additive item cost is a labeled approximation. If none fits, report budget/inventory infeasibility,
not a rank failure. Larger tasks need no exact minimum or solver campaign to be useful.

Protect the evaluator itself with hand-derived tiny cases and a simpler exhaustive reference.
Inject schema-valid semantic mutants: swap anchors/release/context, remove readable conditions but
leave IDs, drop setup, merge incompatible variants, mislabel expected failure, corrupt a valid-looking
delivery map, or misclassify an inaccessible reference as delivered. Include positive alternates,
qualified conflict, intentional NotApplicable, unsupported/inconsistent models and empty-positive
refusal. Admission rejecting a mutant does not demonstrate judgment sensitivity; run appropriate
mutants through valid observations too.

Mutation results separate caught, surviving, equivalent/invalid and inconclusive outcomes.
Inspect a survivor before labeling it a failure. Shrink source/task/context together, preserving
oracle support and the original failure class. Retain minimized actual cases and relevant seeds
for current controls, not an archive of every trial.

Hard metamorphic laws apply only to declared semantics, such as irrelevant source removal preserving
an independently unchanged witness. Paraphrase, reorder sensitivity and approximate ranking stability
are robustness experiments unless the actual operation guarantees invariance. Reference-free
coverage/consistency signals are diagnostics, never substitute sufficiency truth.

## 5. EV3 — Primary runner, numerical references and loss diagnosis

The runner admits an experiment and public requests, invokes the unchanged selected production
interfaces, captures exact responses/journeys and independent judgments, and emits typed task/
stage rows plus minimized failures. Pure, renderer, disposable-native and separately activated live
lanes report their actual readiness; a missing prerequisite blocks dependents only.

Evaluate information sufficiency at several declared packet/journey sizes, task strata and
epistemic classes. Preserve counterexamples/unknown counts and false-claim observations separately;
do not trade a hard semantic failure for improved ranking. Conventional relevance/novelty metrics
remain supplemental with explicit complete/partial judgments.

Separate numerical effects under the same eligible admitted population:

- Blocked exhaustive full4096 scoring is the retained-value reference.
- Exhaustive1024 scoring isolates projection loss.
- Native1024 HNSW versus that reference isolates ANN loss.
- Raw limits/context aggregation and expansion identify discovery/frontier loss.
- Candidate-union full rescoring, optional reranking, bundle selection and final rendering isolate
  later ordering/information loss.

Use bounded matrix blocks and top-k state, explicit precision/backend/tie policy, controlled GPU
reduced-precision behavior and small higher-precision CPU checks. Exhaustive candidates do not mean
exact real arithmetic or semantic relevance. Compare exact ties canonically; record tolerance where
numeric comparison needs it. Local NVFP4 values are not upstream BF16 benchmark weights.
Store-backed references replay admitted values without starting an encoder.

Stage diagnosis follows source/inventory → binding/render/partition → admission/value/projection →
scope/index nomination → caps/grouping → expansion → fusion/rescore → packing → serialized delivery →
bounded navigation. Separate missing oracle/model coverage and budget infeasibility from losses.
Controlled injection of complete eligible candidates or sufficient packets localizes a failure;
it is a diagnostic ablation, never production task success.

Columnar experiment rows may use existing Arrow/DataFusion for context-key joins, stratified summaries
and reproducible comparisons once sets are inconvenient. No production classifier UDF acts as the
reference. Null/duplicates and completeness remain explicit.

## 6. EV5 — Library fit and conditional investigations

| Mechanism | Decision, consumer and trigger |
|---|---|
| Finite sets/circuits/enumeration | Initial witnesses/worlds/tiny packet references; no solver prerequisite |
| Hypothesis6.168.1 | Existing structured generation/shrinking and supported public journey state machines; target cheap pure kernels first, not multiplying expensive subprocess suites. [Stateful guide](https://hypothesis.readthedocs.io/en/latest/stateful.html) |
| Proptest | Add for Rust pure-kernel contextual structures where native shrinking avoids transport. Dependent strategies preserve bindings; do not duplicate every Python case. [Strategies](https://docs.rs/proptest/1.11.0/proptest/strategy/trait.Strategy.html) |
| FixedBitSet | Existing availability/coverage sets with nominal universe and equal lengths; private indices never become persistent IDs |
| BDD0.6.3 | Conditional repeated Boolean restriction/quantification over evidence alternatives; separate variables from runtime atoms, order/resource policy and inconclusive refusal. Counts are assignments, not probabilities/minimal witnesses. [API](https://docs.rs/biodivine-lib-bdd/0.6.3/biodivine_lib_bdd/struct.Bdd.html) |
| Z3 | Optional named value/theory reference beyond finite cases; one evaluator encoder with satisfiable-first Sat/Unsat/Unknown/reason. Avoid status-losing get_consequences convenience; a qualified raw call must preserve its actual outcome. Optimize bounds are available through normal Rust APIs. [Solver](https://docs.rs/z3/0.21.1/z3/struct.Solver.html), [bounds](https://docs.rs/z3/0.21.1/z3/struct.Optimize.html) |
| PyTorch | Existing service version is2.14.0; later blocked numeric references over stored values, no inference startup. Exact locked precision-API syntax is an integration lookup; generic main/stable docs are leads, not exact version qualification. [Numerical accuracy](https://docs.pytorch.org/docs/stable/notes/numerical_accuracy.html) |
| petgraph/FCA | Role-aware motifs, factor/split coverage and independent small concept checks; connectivity is not proof, corpus implications are not universal laws |
| Optuna | Conditional adaptive development search when grid/random search needs campaign orchestration. Frozen evaluator/objective, independent hard constraints; multiobjective report/prune unsupported in researched5.0 API. [Exact Trial source](https://github.com/optuna/optuna/blob/v5.0.0/optuna/trial/_trial.py) |
| ir_measures | Conventional rank/diversity diagnostics once a genuine judgment population exists; nDCG/alpha-nDCG cannot represent compatible AND witnesses. [Measures](https://ir-measur.es/en/latest/measures.html) |
| Semantic/cargo-mutants | Domain mutants first; later scoped actual kernel campaigns with baseline-passing controls, not compiler/store-wide mutation |
| ACTS | Conditional constrained t-way factor coverage when valid products grow inconvenient; configurations are not oracle answers. [NIST](https://csrc.nist.gov/projects/automated-combinatorial-testing-for-software) |
| CP-SAT | Conditional substantial integer packing beyond tiny enumeration. Retain status/objective/best bound/gap: researched9.15 OPTIMAL may mean configured tolerance, not exact minimum. [Status contract](https://github.com/google/or-tools/blob/v9.15/ortools/sat/cp_model.proto#L694-L718) |
| Kani | Optional stable small Rust-kernel assurance with nonvacuous assumptions/unwind checks and isolated compatible toolchain; no proof of oracle-model adequacy. [Unwinding](https://model-checking.github.io/kani/tutorial-loop-unwinding.html) |

The main design and appendix both follow finite-first selection. No installed solver, dependency
upgrade, framework or proof campaign is an initial prerequisite. Prospective research versions
are applicability evidence, not mandated dependency versions. Selected dependencies declare exact
versions under the operator's 2026-10-06 replacement policy; ordinary additions/bumps require no
ADR or holds row and use affected controls. No wholesale lockfile update is implied. Licensing
is not a technical rejection reason.

## 7. EV4 — Versioned experiments and outer feedback — F01

Freeze within each inner comparison: task population/split and public requests, oracle/model/
judgment/observation meanings, completeness/applicability, journey limits, metrics and numeric
precision/ties, baseline/candidate configs and actual source/native/encoder/scorer realizations.
Development optimization can change system parameters under that fixed yardstick. Keep protected
confirmation/gold/heldout outside optimization. The runner records deliberate changed variables
and blocks incompatible mixed comparisons; no automatic score pooling across meanings.

Outer studies invoke actual public interfaces under declared tasks/budgets. A feedback triage
operation consumes observed failure/success, relevant response/journey and independently grounded
source/fixture/runtime basis. It outputs one or more classifications:

- System defect or improvement under unchanged task meaning: route to source/embedding/retrieval/
  delivery owner and add an independent regression case.
- Evaluator task/model/coverage/observation/judgment gap: revise private evaluator meanings with
  the independent basis, supported domain and affected tasks explicitly identified.
- Usability/navigation/presentation gap: improve that boundary and, if warranted, introduce a
  realistic task stratum without falsely turning the old semantic result into failure.

Raw agent opinion, success or misuse does not automatically become expected truth. A formally
sufficient packet can be hard to use; a confident agent can succeed without correct evidence.
Multiple causes may coexist.

An evaluator meaning change produces a new revision/comparison baseline or separate stratum.
When unchanged tasks and retained exact packets remain applicable, rejudge the same packets under
old/new versions to expose the changed yardstick. Changed requests/populations cannot pretend to
be the same comparison. No permanent historical evaluator runtime is required; keep only artifacts
needed by named current comparisons, plus durable minimized controls/current conclusions.

Examples: repeated failure to locate visible evidence expansion supports a navigation correction
and possibly a new realistic journey task. A source-backed missing precedence requirement changes
the evaluator's coverage/expectation and establishes a new baseline. Neither alters independent
source facts or promotes agent judgments into compilation.

Existing agent A/B/C studies remain separate usability/differentiation claims with comparable
evidence admission, independent grading and sealed confirmation. Their parity blockage does not
block finite inner-loop work. Old task populations and receipts are not silently reused as
qualified confirmation for a changed evaluator. No outer trial is run during authoring.

## 8. Packages, verification and completion

EV1 contracts/kernel and EV2 independent controls run before a model or database. EV3 integrates
available pure/renderer lanes first, then actual native production journeys as their prerequisites
become implemented. EV4 supplies experiment/feedback governance early enough for all tuning;
paid outer studies and confirmation remain later separately activated consumers. EV5 is conditional
investigation tied to explicit losses, not a mandatory last phase.

Required focused controls cover compatible/foreign contexts, alternate witnesses, expected-failure/
unknown intent, correct IDs with missing meaning, corrupt maps, mandatory setup, empty-positive
refusal, satisfiable twins, inconsistent/incomplete worlds, candidate ceilings/no feasible packet,
law versus robustness distinction, valid semantic mutants, shrinking and comparison-version mismatch.
Generated executable observations run isolated supported programs only; silence is never negative
proof. Actual native expansion/cursor/cancellation/final-envelope controls use owned persistent
fixtures and the production public interface.

Development/confirmation contamination controls reject private oracle fields at production inputs,
verify frozen comparison identities and refuse incompatible revisions. Tiny finite references
remain separately implemented and challenge the main judgment code. Nominal schema roundtrips alone
cannot establish oracle adequacy.

Functional completion establishes a useful repeatable programmatic loop, actual selected interface
coverage, truthful outcomes, sensitive independent judgments, actionable minimized loss cases and
implemented grounded feedback/version operations. It does not establish broad natural-language
usefulness, comparative superiority, live checkpoint quality or measured performance.
The coordinator owns finding closure and remaining conditional investigations.

Authoring evidence: static review/capability/source work completed, 2026-10-06. Functional tests,
live numerical/agent studies, native runtime checks, sealed populations and performance campaigns
not_run. Exact commands/outcomes enter the current checkpoint only after actual execution.
