# A13–A16/B2 producer review

## 1. Scope, outcome and coverage

**Decision: Accept scoped after correction/re-inspection of F01 below.** The inspected producer
boundary preserves exact source attachment, canonical condition truth with typed uncertainty,
raw nested/call-path distinctions, and reported document/deployment provenance. The one concrete
refusal-classification defect found during review now delegates to the model owner.

| Field | Value |
|---|---|
| Reviewer/date | Independent review agent, 2026-09-30 |
| Standard | [standard.toml](../design_principles/standard.toml): core/template 3.2, CI 1.3 and [binding](../design_principles/binding/library-context.md); both design-review skills |
| Tier/purpose | Scheduled bounded design/target review of A13–A16 and B2 |
| Code | Current dirty shared main: `cpg-flow/{lib,predicate,native}.rs`, `cpg-extract/{ty_flow,document_parser,deployment,deployment_parser}.rs`; adjacent model/attachment/support/bundle/driver contracts and focused producer/oracle tests |
| Authorities | [DESIGN §15](../../design/sections/semantic-model.md), [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md) A13–A16/B2 and §8; prior [A12 re-inspection](design_review_a12-flow-contracts-reinspection_2026-09-30.md) |
| Supported scope | Native observations mapped to nominal facts, qualified by model/context/coverage; independent attachment geometry and physical-layout determinism; supplied task reports are decoded and associated, not executed |
| Exclusions | Q/pilots and integrated release qualification; P3–P5 normalization, transfer summaries, analyses and serving; pending new PG required-failure/representative-corpus controls |
| Method | Fresh source and test inspection; pinned ty-flow reference consulted and matched against docs/pins.md; root-attributed focused receipts; no production edits, probes, formatting/lint or integrated gates by reviewer |

Source judgments are **Implemented, inspected 2026-09-30**. Executed claims are **Tested only for
the root-attributed cases and commands in §10**. The registered fixture sweep establishes
producer/admission integration across those inputs, not independent accuracy of every fact or
the enclosing product. Unrelated shared dirty-tree changes are outside the review.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and change boundary | Consumer/dependency contract |
|---|---|---|
| `cpg-flow` | Read pinned ty semantic index; expose native source coordinates, local handles, expression graphs and structural places | No persisted IDs, BDD/DNF or predicate-key authority; A14 owns nominal attachment |
| `ty_flow` | Map exact native syntax geometry, scope/place/atom distinctions and raw reach/value/test observations | Consumes declared typed syntax/lexical handoffs and shared attacher/model; does not establish interprocedural transfers |
| `document_parser` | Interpret captured MDX/tree offsets; recognize names through declared API vocabulary; verify derived block identity | Native structural records and weaker recognizer assertions remain distinct |
| `deployment` and pure parser | Interpret selected captured metadata/configuration; validate supplied receipt association | Stores reported runtime/interpreter/execution values separately from independently checked captured fields |
| `lctx-model` | Identity, attachment outcomes, canonical Boolean algebra, uncertainty, refusal mapping, support/coverage and sealed validity | Providers consume governing contracts; Arrow/PG derive from declarations |
| Bundle/stage runtime | Declared effects/relations, budgeted batch transfer, contribution ownership and failure propagation | Provider threads emit into attempt-owned bounded channels; shared vocabulary is assembled by its writer |

**CI fact/fidelity table**

| Family | Provider/revision and fidelity | Coverage/unknowns | Identity and consumer |
|---|---|---|---|
| Flow | `ty_python_core 0.0.14`/Ruff 0.0.14/salsa 0.28.2; generated supports use NativeTraversal/NativeStructural | Inexact attachment produces typed outcomes/boundaries; source/model/condition refusal makes Partial or Unavailable; native failure is Failed | Occurrence/context/predicate/operand atoms; nominal Places and ordered raw paths; P3/P4 remain consumers |
| Documents | `markdown-rs` 1.0.0; raw document status, native structure, derived normalized recognition distinguished | Malformed/undecodable/bounded traversal recorded; ambiguous/lexical mentions are Candidate | Source spans/document-node identities; captured materialized blocks; no runtime or behavioral claim |
| Deployment | Pinned lctx provider build digest; metadata/configuration Recognizer, task reports ReportDecode/ReportProjection | Invalid/oversized/mismatched receipt is retained as Failed interpretation and Partial coverage | Captured source spans plus ReportedEnvironment/TaskReport identity; reported success remains reported evidence |

This model is adequate at the facts frontier: observable analyzer distinctions govern conversion
and validation, rather than merely naming output records. It deliberately does not equate raw
reaching definitions or through-call dependencies with value-transfer proof.

## 3. Contracts, constraints and testing boundaries

`ty_flow::Writer::attach` derives a native kind/role, then accepts only Attached::Exact. Innermost,
Ambiguous, Unmatched and BudgetExceeded preserve typed AttachmentOutcome/Candidate/Boundary
records; they do not become nearest-span facts. Native syntax shape ambiguity is also a boundary.
Scope/root selection uses declared syntax and lexical resolutions, including agreement before
special import/builtin runtime classification. The shadowed-name positive/negative twins exercise
this rule, rather than interpreting spelling as a runtime binding.

`qualify` maps native leaves to EvaluationAtoms and consumes the A12 GraphCondition result.
Approximation::Unknown/Exact enters AssertionQualification explicitly. Canonical condition nodes
alone represent truth; no stored provider-local handle or parallel DNF encoding is introduced.
Synthetic predicates lacking a source event refuse instead of minting a span/name surrogate.
Kernel refusal now uses `obligation::from_kernel` (F01).

Nested bindings stay ReachingDefinition::Nested, including loop expansion; Undefined/Deleted
remain Unbound. The adapter uses native indices only to join the native use/definition arrays,
then persists nominal identities. Raw call paths preserve actual operand wrappers and are subject
to the shared use/operand/complete-path invariant reviewed at A12.

Document code blocks verify parsed code digest/length against the already captured derived
artifact before stating a materialized link. A source/capture mutation refuses before publication.
MDX attribute literal/expression/bare/spread arms and component ancestry stay typed. Recognition
does not execute JSX or code and cannot establish API behavior.

Receipts require independent captured release/lock/environment/version/platform/requirement/
metadata and selected source/hash association, plus the declared interaction policy and runner
hash. Runtime/interpreter digests and execution status remain ReportedEnvironment/TaskReport
fields. The stage executes neither corpus code nor its claimed command. Failed execution can be
successfully interpreted: extraction completeness is distinct from reported check status.

## 4. Composition and execution

Stages declare relation inputs/outputs, contributed vocabulary, coverage, provider identity,
profiles and effect class. Typed syntax/lexical handoffs precede Flow/Documents; shared vocabulary
assembly follows its contributors. No producer writes derived serving claims. Provider build
identity includes the production source closure, lockfile and Pyrefly patch; the closure includes
scripts/specs, including the embedded receipt runner.

**CI analysis records for scoped transformations**

| Question/transform | Inputs/universe and method | Precision/model | Budget/partial outcome | Output/evidence |
|---|---|---|---|---|
| Native coordinate → source fact | Captured artifact, native span/kind/role, typed OccurrenceIndex | Exact match only; scalar oracle independently checks geometry | Attachment visits/alternatives bounded; typed inexact outcome | Nominal occurrences, boundary candidates and qualified support |
| Native reach constraint → condition | Per-scope ty diagram; translated source predicates; root Boolean graph closure | Disclosed Boolean runtime model; AMBIGUOUS terminal becomes unknown-qualified may-truth | Graph/depth/kernel limits refuse; no false fallback | Canonical Condition/ConditionNodes; EvaluationAtom lineage and qualification |
| Document name recognition | Captured document tree, declared public/declaration vocabulary | Recognition/exact lexical shape, not semantic invocation proof | Source/traversal limits disclosed; Candidate for ambiguous/lexical mention | Typed mention/source spans with DerivedAnalysis/Recognizer support |
| Deployment/receipt interpretation | Selected captured descriptions and supplied reports | Normalized descriptions or reported observations; no runtime replay | Byte/field limits; invalid interpretation retained as Failed/Partial | Original values, typed report collections and source/environment links |

The pinned ty-flow reference confirms the native ternary diagram and AMBIGUOUS terminal. The
producer explicitly discloses Boolean runtime predicates and does not take `if_ambiguous` edges;
this is a stated model translation, not a newly discovered missing traversal. Synthetic outcomes
without source geometry are unavailable, not silently coalesced by text.

Pipeline-owned typed indexes/source buffers/atom and expression maps use attempt-budget admission;
transferred batches retain reservations across the bounded provider channel. Documents and
deployment reserve conservative input/collection allowances before parsing/retention. Native
parser/semantic-index heaps are explicitly provider-owned and remain a Q measurement obligation.
This inspection establishes reservation ownership and failure routes, not calibrated peak RSS or
representative-scale memory/performance qualification. No new resource framework is justified.

## 5. Change and failure scenarios

| Scenario/kind | Governing owner and observed route |
|---|---|
| New occurrence/call wrapper, domain instance/composition | Native coordinates map through kind/role and exact attachment; starred operands preserve the typed wrapper; shared path validation governs nesting |
| Repeated same-spelling guard plus nested rebinding | Occurrence-keyed atoms stay distinct; Nested is never restated as a parameter/ordinary Unbound reach |
| Analyzer/runtime-special binding changes | Agreement of lexical resolution controls runtime classification; shadowing stays opaque; source-local handles do not enter persistent identity |
| MDX component/new recognition candidate | Parser structure and shared document types carry syntax; recognition stays a separate weaker assertion instead of changing runtime semantics |
| Invalid receipt, new reported execution instance | Captured bytes survive; source/environment mismatch becomes Failed interpretation/Partial; reported runtime fields remain disclosed |
| Input/provider order, relocation, row transfer size | B2 shuffles order and 4096/1/97-row boundaries under both profiles; semantic content digest is the equality contract |

These meet the scoped analyzer-upgrade/add-fact/evidence journeys. Mechanism replacement is
governed by the typed handoff/attachment contracts; neither a replacement analyzer nor native
index permutation experiment is executed by this review. Native order independence is inspected
at the transient-to-nominal boundary; executed B2 evidence covers the physical variations named.

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence/reason |
|---|---|---|
| G1 Authority | pass after F01 | Model owns refusal translation, identity, geometry and qualification; no separate provider truth/store contract |
| G2 Semantic fidelity | pass, scoped | Exact attachment; Unknown propagated; raw nested/value paths and reported evidence not promoted |
| G3 Validity | pass, scoped | Shared model validation/admission and capture-integrity refusal; independent focused negatives |
| G4 Hidden behavior | pass | Declared Extraction effects; captured inputs; no executed analyzed code/environment expansion; ambient knobs refused |
| G5 Consistency/resources | pass for declared boundary | Attempt-owned reservations/outputs; failed/partial/unavailable outcomes distinct; provider native heap qualification remains Q |
| G6 Transformation/reuse | pass, scoped | Nominal identities/canonical conditions and independent layout digest tests; reported fields retain original distinctions |
| G7 Truthful claims | pass | Attribution/labels and exclusions below; fixture integration is not broad semantic accuracy or product qualification |
| G8 Library leverage | pass | ty semantic index, shared attacher/BDD kernel, markdown-rs, PEP parsers, mailparse/INI and serde decoding are reused |
| CI-G1 Fidelity | pass, scoped | Candidate/Unknown/report distinctions and unavailable boundaries are explicit |
| CI-G2 Evidence closure | n.a. | No served claims in A13–A16/B2; source/support lineage inspected, P5 remains the serving revisit boundary |
| CI-G3 Evaluation integrity | pass, scoped | Production uses captured inputs and declared vocabulary; reference fixture expectations are test inputs only |

These are reasoned gate judgments, not additional executed checks.

## 7. Findings and applicability

<a id="F01"></a>

### F01 — Native condition refusal bypassed the model's typed classification

**Initial defect and immediate correction, inspected 2026-09-30.** `ty_flow::Writer::qualify`
initially mapped every Diagram::from_graph error to generic BudgetReached and retained the
KernelBoundary only in a formatted string. AtomLimit therefore lost ConditionAtomLimit, and
malformed/unsupported conversion was misclassified as a budget event. The model already owns
the closed translation in `domain::obligation::from_kernel`.

- **Rules/consequence:** FP-02/04/05, DP-01/02/08/21, CI-02/04; G1/G2/G6. A condition limit or
  unsupported graph would require a consumer to parse free text to recover the actual refusal.
- **Correction observed:** `crates/cpg-extract/src/ty_flow.rs:181` now calls
  `lctx_model::domain::obligation::from_kernel(boundary)` before writing SubjectBoundary. The
  model's mapping remains the single owner (`crates/lctx-model/src/domain/obligation.rs:197`).
- **Closure evidence:** fresh read of the corrected branch and authoritative mapping is
  sufficient for this one-call delegation repair. No separate failing/passing probe was invented;
  focused-suite receipts remain separately attributed in §10.
- **Disposition route:** [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md),
  A14 owner, stable review F01. The plan owns current execution status; this is dated evidence.

No further material in-scope finding remained after that correction. FP-01–06 and applicable
DP/CI rules are satisfied for the scenarios in §5. P3 normalization, P4 transfer/SCC behavior,
heuristics and P5 served evidence closure are excluded responsibilities, not waived obligations.

## 8. Library fit and total complexity

The native provider owns ty extraction and a deliberately small runtime translation. The typed
adapter composes existing lexical/attachment/condition/flow contracts instead of adding another
semantic engine. Document parsing uses native mdast offsets; metadata/requirement decoding uses
established parsers; the remaining code is specialized association/recognition policy.

The ty-flow skill's pinned 0.0.14/0.28.2 line matches docs/pins.md. Its reachability contract was
consulted for native-versus-runtime distinctions, not substituted for current source evidence.
No universal provider registry, second DNF/BDD engine, custom metadata grammar or new lifecycle
layer would simplify these bounded responsibilities. No new library API is proposed.

## 9. Alternatives and tradeoffs

| Alternative | Semantic/maintenance consequence | Judgment |
|---|---|---|
| Nearest-span attachment | Easier to emit a fact, but changes source authority and hides ambiguity | Rejected; exact match or typed boundary is appropriate |
| Persist native handles/rendered predicates | Couples identity to provider layout and restores parallel truth encodings | Rejected under DESIGN §15/A12 |
| Execute or trust receipt status as independent verification | Makes extraction effectful or promotes supplied reporting beyond evidence | Rejected; association plus reported status is appropriate |
| Shared typed refusal translation | One owner handles limit/unsupported distinctions; no caller-specific policy | Adopted correction for F01 |
| Require semantic invocation proof for every document mention | Conflates recognition and downstream normalization/analysis | Current Candidate/recognition boundary is appropriate; P3 handles links |

## 10. Verification and uncertainty

All following **passed** receipts are root-attributed, dated **2026-09-30**, not reviewer runs.
Cargo commands use `python3 scripts/build_environment.py -- cargo test --release`.

| Command suffix | Receipt and conditions |
|---|---|
| `-p cpg-flow --test flow_shapes --test call_paths` | passed, 31 + 1 native-provider tests |
| `-p cpg-extract --test determinism --test attachment_oracle --test typed_flow` | passed, 2 + 1 + 3; scalar geometry/forced outcomes and order/relocation/transfer sizes |
| `-p cpg-extract --test typed_flow --test typed_calls` | later rerun passed, 4 + 8; re-homed type-guard/runtime/BOM answers; native typed tests inspect real producer outputs |
| `-p cpg-extract --test typed_documents` | passed, 3; nested MDX/attribute arms, malformed recognition and captured-block corruption refusal |
| `-p lctx-model --test domain_documents` | passed, 3; shared document-contract boundary |
| `-p cpg-extract --test typed_deployment` | passed, 2; complete report field retention plus malformed/oversized/source/environment negatives |
| `-p cpg-extract --test acquisition --test capture`; `-p lctx-model --test domain_input` | root reports passed, 9 / 4 / 6 for related capture/input controls |
| `-p cpg-core --test fixture_corpus -- --nocapture` | passed, 1 test: 47 registered cases, both profiles, after starred-wrapper and native nested-marker corrections; integration/admission sweep, not all independent known answers |
| `-p cpg-core --test facts_generation` | earlier root receipt passed, 1 real PG test; newer required-failure and representative PG fixture-comparison controls remain pending and are not counted |

F01's final delegation correction is **Implemented by source re-inspection**. The root reported
focused runs above; this review does not assert that every listed run followed that final
one-line correction. Existing scalar attachment expectations and explicit guard/receipt negatives
are independent of production transformations. The fixture sweep and shared validation do not
independently establish analyzer accuracy for every registered case.

`just fmt`, `just test-all`, Q/pilots and downstream P3–P5 are **not_run by reviewer** and excluded.
New PG failure/corpus controls are **not_run at this review's receipt boundary**. `just docs-check`
and final `just library-catalog` are **not_run by reviewer**; root owns final session checks and
shared catalog regeneration. No performance or total-memory measurement is inferred.

## 11. Authority changes and dispositions

Record F01's correction/closure in the active cutover plan rather than adding a separate status
register. No new architectural decision is required for delegation to an already-owned mapping.
Preserve the A12 original findings and re-inspection as prior evidence; this review extends the
consumer boundary and does not overwrite their scope.

Q retains native-heap/publication measurements, facts pilots and integrated qualification. Pending
real PG failure/corpus controls retain their own execution disposition. P3–P5 own normalization,
transfer discharge, analytics and serving. They cannot be inferred from accepted producer facts.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | satisfied | Native provider, typed attachment/model, document recognition and reported receipt association have distinct owners and bounded contracts |
| A2 Encode domain meaning explicitly | satisfied after F01 | Exactness, uncertainty, nested reaches, operand crossings, report-versus-observation and refusal distinctions govern producer behavior |
| A3 Extend through composition | satisfied | New syntax/report instances reuse typed owners and stage handoffs; F01 now consumes the shared refusal operation |

**Bounded decision: Accept scoped after F01 correction.** No material reviewed contract gap
remains in the inspected tree. The result rests on source inspection and the explicitly scoped,
composite root receipts above.

**Enclosing architecture: unresolved beyond this boundary.** Pending new PG controls, Q/pilots and
the assembled P0–P2 review remain open; P3–P5 retain their capability obligations. This review does
not claim full phase completion, independent end-to-end accuracy, product availability or release
qualification.

## Follow-up: native signature interpretation (2026-09-30)

**Accept scoped at source-inspection strength.** The full facts pilot exposed native parameter
forms outside the model's former list grammar. [ADR-0095](../../adr/0095-preserve-native-positional-order.md)
preserves provider ordinals and per-slot positional kinds, while shared construction, stored
validation and binding use their common grammar rank. The keyword binder continues to exclude
positional-only slots. The independent `native_signature` fixture checks the native `self`,
`__context` order; it is an explicit fixture-corpus family.

[ADR-0096](../../adr/0096-preserve-unavailable-native-signatures.md) appends
`SignatureForm::NativeUnavailable = 3` for native lists such as expanded constructors with
duplicate slots. `calls.rs::validate_shapes` retains each slot's validity and the 4096-member
ceiling. `symbol_records.rs` converts only an `Invalid` List interpretation; operational limits
and resource errors still fail. Ordered memberships and displayed annotations survive, while
source parameter declaration links are excluded. `bind` refuses the form with
`OutsideProviderModel` before assigning arguments. This is evidence preservation with a typed
interpretation boundary, rather than deduplication or an invented callable signature.

`pyrefly_stage.rs::write_symbols` emits unavailable-signature boundaries under the supplied
scope. Root Signatures coverage becomes Partial with `OutsideProviderModel`. Supporting
dependency definitions additionally accumulate the returned boundary counts into Input
Signatures coverage, so an unavailable dependency variant cannot leave that coverage Complete.
Supporting scope remains the definitions referenced by analyzed roots; complete dependency
analysis and transitive definition closure are not claimed.

**Tested, attributed root receipt, 2026-09-30:**
`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_calls -p cpg-extract --test typed_symbols`
passed 15 model controls and five symbol controls in `build/native-unavailable-tests.log`.
The new model control retains duplicate ordered slots through Arrow roundtrip and rejects
binding; the symbol control independently preserves ADR-0095 native order. That receipt does
not itself execute a duplicate-slot producer/root-versus-dependency coverage control; those
paths are inspected implementation until additional controls or the repeated pilot supply
execution evidence. Q and assembled exit acceptance remain pending their complete receipts.

The assembled review owns its distinct deployment-target finding F01. Its reservation and
fail-stop correction has an attributed focused producer refusal receipt; this review's F01
continues to identify the earlier canonical kernel-refusal mapping defect. Persistent native
indexes now hold `StateCharge`; transient adapter collections, native solver heaps and SQLx
retained buffers remain explicit measured-envelope limitations, not a universal strict RSS
guarantee.

## Follow-up: A6–A8 F05–F07 dispositions (2026-09-30)

Fresh inspection under core 3.2/CI 1.3 closes the original producer triggers scoped, and updates
their individual rows in cutover-plan §8 rather than leaving A10/A11 work deferred after migration.
The original source-review IDs remain stable.

- **F05:** `calls.rs::ProviderCallable` encodes symbolic, module-body, class-body and decorator
  attribution independently of semantic occurrence ownership. Implicit `SymbolKind` allocations
  are reserved and refused by `ProviderSymbol::validate`. The model's
  `call_site_support_authenticates_implicit_and_symbolic_callers_transitively` covers all four
  forms and provider/context negative twins; native `typed_calls::variants_keep_channels_phases_and_native_owners`
  exercises the producer and excludes pseudo-symbols. The model/producer correction is closed;
  P3 still owns normalized callable/entity correspondence with the one occurrence-owner rule.
- **F06:** `type_records.rs` maps named Function/Overload types to provider-qualified symbols;
  anonymous Callable terms have structural slots and returns. Declaration TypeVar, ParamSpec and
  TypeVarTuple values retain anchored module/provider/context identity and restrictions.
  `domain_types` tests nominal callable closure ownership, module-distinct variable identity,
  foreign-variable refusal and changed callable identity; `typed_types` exercises native
  ParamSpec and structural terms. The bare-name identity defect is closed scoped. P3 still owns
  equivalence between independently attributed providers.
- **F07:** `typed_types::declared_def_parameters_have_typed_observations_at_their_formal_declarations`
  checks linked annotated def parameters receive declared Parameter observations. Non-def
  native signature annotation displays remain enforced DisplayOnly (`symbol_records.rs` and
  `ParameterAnnotationObservation::FIDELITY`); this is implementation inspection, not an
  independent non-def structural recovery receipt. The A11 obligation is closed scoped.
  P4 remains responsible for class-set/exhaustiveness/scalar derivation under
  `lctx-model::domain::{types, derivation}` contracts and `lctx-analytics` guard/type analysis;
  the trigger is the first guard-refutation consumer. Opaque/truncated/display-only inputs
  must retain uncertainty, and negative hierarchy claims require Complete MRO evidence.

These dispositions do not add an execution receipt, qualify Q or close the remaining normalized
or analytical consumer obligations. No production changes or test commands were made for this sweep.

## Follow-up: dictionary fields and native slot text (2026-09-30)

**Accept scoped at source-inspection strength under core 3.2/CI 1.3.**
[ADR-0097](../../adr/0097-typed-dictionary-field-membership.md) gives anonymous dictionary fields
their own `TypedDictFieldList`/`TypedDictField` owner. The native adapter retains string keys,
requiredness, order and referenced terms directly. The ordered digest and shared charged
`typed_dictionary_membership` invariant reject duplicate keys, ordinal gaps, missing members
and changed fields; the 4096-field construction ceiling is a typed Limit. TypeIndex follows
field terms through the nominal list owner, including transitive fidelity/ownership validation.
Nominal TypedDict record observations likewise retain arbitrary string keys; other record
models retain their nonempty-name rule. Relations are in the model and declared producer
outputs; generated Arrow/PG lowerings replace the former schema without an old-ID bridge.

[ADR-0098](../../adr/0098-native-slot-interpretation.md) separates retained native slot text from
callable interpretation. ParameterShape and CallableParameter retain an empty string while
preserving slot-kind and requiredness checks. Bindable Signature List validation rejects empty
names; NativeUnavailable retains its ordered members and binding still refuses. Callable
List/Partial terms reject empty names at shared type-shape validation.
`CallableForm::NativeUnavailable = 5` is appended without changing previous codes; it retains
slots and returns, requires DisplayOnly support through the type closure, and grants no ordinary
callable interpretation. `type_records.rs` marks its native mapping opaque and contributes a
Types boundary; existing boundary accounting makes Types Partial. The prior unavailable
signature mechanism makes Signatures Partial, including supporting Input scope.

The `dictionary_keys` fixture independently supplies empty, whitespace and non-identifier keys
with TypedDict/Unpack. Model controls mutate dictionary membership and unavailable-callable
shape/fidelity. The native control checks retained keys, Signatures Partial with
`OutsideProviderModel`, and the actual empty slot of a Signature NativeUnavailable. The fixture
does not emit a TypeTerm Callable NativeUnavailable. An added assertion expecting that type
form was incorrect and was removed; its evidence was not relabeled. Callable NativeUnavailable
mapping remains source-inspected, while its form/refusal and transitive DisplayOnly policy are
model mutation-Tested. No material source defect was found in this bounded review.

**Tested, attributed root receipts, 2026-09-30:** focused model Types 14, model Calls 15,
native Types four, real PG Calls two and real PG Types one passed. The submitted commands use
`python3 scripts/build_environment.py -- cargo test --release` with the named `domain_types`,
`domain_calls` and `typed_types` targets; `build/typed-dictionary-tests.log` supplies the initial
dictionary receipts. The actual corrected native control rerun in
`build/native-field-read-controls.log` passed four `typed_types` tests, 11 `generation_read`
tests and two `model_runtime` tests, attributed to the root on 2026-09-30. This review adds no
execution command. The model digest is
`ebf0f2aa3216b61a805d7064dcdedd164407cb2b0800761f16f4133f25d903eb`; root reports the schema
snapshot diff inspected/accepted (two dictionary relations, their reference target/membership
invariant, and appended callable-form code 5). The complete Q gate and both-profile/repeat pilots
are rerunning against that digest; assembled exit acceptance remains pending their receipts.

## Follow-up: Unicode/NUL producer correspondence (2026-09-30)

<a id="F02"></a>
### F02 — Supported Unicode values were lowered to PostgreSQL text

**Original defect, root-attributed Q evidence:** full catalog COPY of
`literal_values.string_value` failed with PostgreSQL SQLSTATE 22021 for an embedded NUL
(PostgreSQL log, 2026-09-30 07:51:46). Python Unicode literals and mapping/native expanded slot
keys admit that value; its rejection by physical text storage loses supported facts. This is
G2/G6 semantic fidelity and lowering correspondence, with A2 affected. Responsibility is the
model scalar/field declarations and their permanent producer/store lowerings.

**Disposition: closed scoped, composite focused Tested receipt below.**
[ADR-0099](../../adr/0099-preserve-unicode-values-as-utf8-bytes.md) introduces `Utf8Text`, whose
private String remains valid UTF-8 and whose serde/declared scalar lowers to Binary/bytea. Decode
rejects invalid UTF-8. Its Key implementation delegates to String, preserving semantic identity
and the distinction from Literal::Bytes; it does not introduce escaping or a second reader.
Literal strings, TypedDict keys, record-field names and native signature/callable slot names use
this value. The inspected Ruff, Pyrefly and ty adapters convert retained strings with `Into`,
without removing NUL, renaming or reinterpreting keys. Empty/non-ASCII/ordinary strings retain
their earlier structural rules; unavailable callable forms retain their refusal/fidelity policy.

**Implemented, source-inspected 2026-09-30:** producer/model correspondence is coherent for this
bounded repair. The `string_values` native fixture independently supplies `"\\0"`, `"é\\0終"`
and a NUL TypedDict key; it is registered as the 50th fixture-corpus family. Added model controls check shared Arrow roundtrip and serde binary decode, invalid
UTF-8 refusal, existing String key equivalence and string-versus-byte literal distinction. The
real PostgreSQL control uses COPY, seal, shared validation, publication, typed lease readback and
retirement. Native callable slot/key conversion beyond those observed instances remains
source-inspected; no independent accuracy claim is inferred for every native form.

**Composite focused receipt, attributed root, 2026-09-30:** initial compilation failed for missing
serde_json linkage and a missing budget helper in the model test. The repair uses serde's binary
deserializer without adding a dependency. A subsequent model assertion compared source order to
Batch's declared ID order; sorting the expected records corrected that control.
`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test unicode_values -p lctx-postgres --features testing --test unicode_values -p cpg-extract --test typed_types`
then **passed** model Unicode one, real PG Unicode one and native Types five in
`build/unicode-values-tests.log`. This establishes tested closure for F02's lossless Unicode
value lowering and observed native NUL/key instances. Schema snapshot/model-digest review and
the complete Q gate/pilots remain pending; root reports `just fmt` passed before that rerun.
The pre-NUL gate's 669 Rust/219 Python passes, attributed to the root, qualify the earlier tree
only. This review does not promote them to acceptance of the changed physical schema. No
production edits or test commands were made by this reviewer.

## Follow-up: residual children at native type ports (2026-09-30)

<a id="F03"></a>
### F03 — Type-shape validation rejected declared display-only residual children

**Original defect, root-attributed Q evidence:** the full catalog pilot failed after 515 seconds
with `a bound method binds a callable`; cleanup left the generation registry empty. Its archived
catalog failure artifact and the complete repair qualification remain the author's evidence.
The pre-change 672 Rust/219 Python gate receipt does not qualify the repair.

**Source correspondence, inspected 2026-09-30:** the BoundMethod function, Overload signature,
and Generic parameter/body ports are lowered through `type_records.rs::term`. That operation
explicitly admits Other for an unnamed native overload and Truncated at its depth/work bounds,
retaining provider/context and native display. The producer propagates opaque fidelity to each
enclosing term. Shared TypeIndex support validation follows every child and already refuses
residuals unless the full closure is DisplayOnly and belongs to the supporting provider/context.
The structural-shape check instead unconditionally requires callable/type-variable arms at these
ports. Thus a declared and correctly qualified residual envelope still cannot validate. This is
G2/G3/G6 correspondence and A2 validity, owned by the shared model type shape/support contract.

**Closed scoped, source-reinspected and attributed focused Tested receipt, 2026-09-30.**
[ADR-0100](../../adr/0100-preserve-residual-type-ports.md) and the live shape validator permit only
Other or Truncated in addition to existing native arms at the four ports. Native parents/order
and Generic's nonempty TypeParameter sequence remain intact. Support retains transitive
DisplayOnly/provider/context enforcement; no residual is relabeled as native callable structure.
The new `opaque_children_keep_typed_ports_without_structural_support` matrix covers all four
ports with Other/Truncated, DisplayOnly positives, NativeStructural negatives and foreign-context
negatives. Existing `every_form_keeps_its_shape` still rejects ordinary wrong bound-function and
generic-body arms.
`python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_types -p lctx-postgres --features testing --test domain_types`
**passed** 15 model tests and one real PG test (74.21 seconds) in
`build/opaque-type-ports.log`, attributed to the root. The PG control adds 16 residual envelopes;
ordinary None-arm negatives remain passing. This closes F03's shared shape/support correspondence
defect within that tested scope. Subsequent root-attributed read-only SQL against the sealed
catalog generation `ae9f939136a2f8190105b6cac83388ad` identifies 59 BoundMethod children as
Other (`unnamed_native_overload`) and no Truncated children, in the qualification evidence's
`raw/bound-residual-diagnostic.txt`. This confirms the native-instance correspondence at issue;
it does not infer stronger type meaning. Repeated full Q/pilot qualification
remains pending. This reviewer made no production edits or execution command.

## Follow-up: partitioned flow runtime lookup (2026-09-30)

**Accept scoped at source-inspection and attributed focused Tested strength.** Root's
behavioral pilot was interrupted after 802.70 seconds when a preserved gdb sample located CPU
in the adapter's repeated global event/import/identifier scans. The archived
`pilot-runtime-index-receipt.json` reports the preceding catalog command passed in 484.39
seconds and behavioral exited 130. That interrupted run is failed, not a completed throughput
measurement or behavioral qualification. Root reports safe SIGINT/staging abort retained the
published catalog.

The inspected `ty_flow.rs::Index` fix adds charged source partitions for imports, identifiers,
references and scopes, and partitions bindings by scope and declarations by owning occurrence.
Each partition derives exactly the source/owner field already required by the lookup predicate,
and retains the original BTreeMap or vector iteration order. Root/read/scope predicates and
resolution agreement remain explicit after narrowing the candidate collection. Derived slots
reserve before insertion and retain the same Index charge lifetime. No semantic fact or
admission condition is added by a partition.

Import-special classification is computed once across **all** BindingEvents into a charged
global map. Per-source runtime references still read that map through their resolved events,
preserving cross-source resolution behavior; restricting that map to the current source would
have changed semantics. Class/builtin/runtime predicates and unanimous-resolution classification
remain unchanged. The declaration/scope partitions preserve uniqueness detection and first-match
behavior. The implementation is a bounded lookup refinement within the existing producer owner,
with no model-contract decision or new ADR required.

`build/flow-index-check.log` records the root's compile check passed.
`python3 scripts/build_environment.py -- cargo test --release -p cpg-extract --test typed_flow --test determinism`
**passed**, attributed root receipt 2026-09-30: four flow controls (the runtime-resolution control
now spans 17 modules) and two determinism controls in `build/flow-index-tests.log`. This closes
the lookup correction at source/focused semantic strength, not full-library throughput or capacity.
The existing 50-family corpus, fresh gate and pilots remain pending their repeat receipts; no new
fixture family was added by this correction. Root reports `just fmt` passed.
The root recovered the pre-fix source embedded by `include_str!` in the prior compiled
CLI into `build/ty-flow-pre-index.rs` (47,300 bytes, with a unique start marker and unchanged EOF
suffix). Direct reviewer comparison confirms the old global import-classification map, each
unchanged filtering predicate, and preserved iteration/first-match order. This strengthens
source equivalence evidence but is not an executed old/new semantic digest comparison. The producer fingerprint changes and full Q/pilots must
rerun. No production edit, signal or execution command was made by this reviewer.

## Follow-up: comprehension walrus place ownership (2026-09-30)

<a id="F04"></a>
### F04 — Definition root fallback confused evaluation scope with binding owner

**Original defect, root-attributed Q evidence:** the repeated behavioral pilot failed the shared
Bound reaching invariant because a definition and its use had different Places. The diagnostic
identifies `fastmcp/utilities/logging.py`: comprehension 2921–3071, `package_path` read 2931–2943,
and walrus target 3010–3022 with value 3026–3052. Both native rows retain the same comprehension
scope and spelling. The read's lexical resolution selects its enclosing function, while the
definition's store occurrence has no root read and previously fell back to the comprehension.
The unequal typed root IDs follow from that asymmetric lowering, not from a cross-place native
reaching claim. This is an A2/G2/G3/CI-G2 producer correspondence defect in
`cpg-extract::ty_flow::Index::root`; the shared model's same-Place Bound invariant remains valid.

**Source reinspection, 2026-09-30:** `lexical.rs::enter` routes a walrus store through
`non_comprehension` before `bind`; `lexical_records.rs::records` retains that owner in the typed
BindingObservation. The pinned native builder separately marks the containing Python binding
but creates the definition with its current comprehension scope. Likewise `cpg-flow::Walk::def`
retains the scope supplied by the use/loop traversal, while its Place spelling comes from the
definition's native place table. Evaluation-scope attribution and lexical variable identity
therefore have distinct owners; replacing one with the other would erase raw information.

**Closed scoped, source-reinspected and attributed focused Tested receipt, 2026-09-30.**
The live fix adds a charged `occurrence_bindings` partition. Exact attached occurrence and root
name select BindingObservation.scope only when the target candidates agree; conflicting target
owners fail explicitly. Every inserted partition key/slot reserves through the existing Index
StateCharge before retention. Compound stores do not acquire a root from a nearby or containing
binding event: their root-name reads still supply ownership evidence.

Read alternatives now compare binding **scopes**, rather than requiring the same BindingEvent.
Different candidate definitions of one variable in one owner scope share its Place. Every
candidate must be a Binding target with a retained BindingObservation and the same scope;
Builtin, Unresolved, missing observations or disagreeing scopes cannot be discarded to form
agreement. Without such agreement, the existing explicit native-scope fallback remains; this
does not establish a cross-scope binding equivalence. No new positive cross-place reaching claim
is admitted, and the writer still refuses a Bound edge whose final Places differ.

The resulting PlaceRoot remains Formal for a rebound parameter variable, with its formal
declaration preserved; it does not become the caller's Entry value. Native FlowUseObservation
and FlowDefinitionObservation scope attribution remains unchanged. `Walk::use_` and
`loop_bindings` continue to retain NestedBindings as Nested and expand loop headers without
inventing ordinary definitions for either marker.

The native focused control `reaching_places_keep_the_binding_owner_across_scope_and_member_reads`
now includes enclosing-function, module, nested-comprehension and formal-rebinding walrus cases,
alongside existing global/closure/member/item cases and shared invariant validation. Local roots
are asserted to belong to a non-comprehension owner; the formal case checks the exact parameter
declaration. Selecting the specific typed lexical owner is also source-inspected through exact
site/name evidence. The first walrus regression failed before the repair.
`python3 scripts/build_environment.py -- cargo test --release -p cpg-extract --test typed_flow --test determinism`
then **passed** five typed-flow tests (including nine ownership cases) and two determinism tests,
attributed to the root on 2026-09-30, in the qualification evidence's
[`flow-binding-owner-receipt.json`](../evidence/2026-09-30_facts-qualification/flow-binding-owner-receipt.json)
and [`raw/flow-binding-owner-tests.log`](../evidence/2026-09-30_facts-qualification/raw/flow-binding-owner-tests.log).
The ambiguity/missing-observation branches are source-inspected, not independently native
fixture-tested by these positives. The producer fingerprint changes again; complete Q and both
pilots remain pending. No P3–P5 normalization, call-transfer or analysis claim is accepted here.
This reviewer made no production edits or execution commands.
