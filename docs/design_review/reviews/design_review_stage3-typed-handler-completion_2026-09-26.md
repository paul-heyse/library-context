# Stage 3 typed handler completion

## 1. Scope, outcome and coverage

**Interface-checked, 2026-09-26. Change/conformance; Accept scoped after correction.** This
reviews the dirty typed-handler and bare-re-raise extension after `011c638`. It is separate
from the earlier expression/frame and predecessor checkpoints and is not the broader
design-alignment review excluded by the operator. Standard: core 3.0, code-intelligence 1.1
and the repository binding. Authority: [ADR-0057](../../adr/0057-compositional-stage3-semantics.md)
and [behavioral analysis §9.9](../../design/sections/behavioral-analysis.md). The
[active plan S2/S7](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns execution status and remaining qualification.

Inspected: `completion.rs`, `ExactRuntimeException`, context-class retention/extraction,
the `except*` syntax marker, preparation of handler types before completion, the existing
handler-type relation and MRO contract, shared proof references, and the added source cases.
Tests, formatting and integrated gates were **not_run** by this reviewer. New source checks
were pending when the review was requested; no pending run is reported passed.
During review the author reported the later-handler positive failed. Follow-up inspection of
the exact pinned Pyrefly source established F01 below. The pyrefly-ruff skill's pin was checked
against `docs/pins.md`; Context7 was also queried, but its results did not establish this
internal MRO contract. The exact local checkout supplied the decisive evidence.

### Responsibilities, fact fidelity and extension scenario

| Owner | Responsibility and explicit contract | Fidelity/consumer |
|---|---|---|
| Schema | Exact runtime exception kinds and the corresponding retained builtin class identity | `TypeError` is the only supported exact kind; extraction and pure completion share the declaration |
| Context extraction | Retain the pinned class and attributed provider MRO under the analyzed context | Bundled-typeshed identity, not source spelling or the host Python environment |
| Handler-type relation | Preserve bare, pinned-builtin and unknown handler type outcomes with resolution evidence | Completion admits only the supported exact builtin case or a bare handler |
| Pure completion | Carry a pending outcome and active exception, select handlers in order and unwind finalizers | Ordered class/MRO evidence enters the existing completion proof; uncertainty refuses |
| Core | Prepare handler observations before semantic composition and reconstruct publication output | No separate handler interpreter added to orchestration |

The expected change is adding another exact exception produced by an already bounded semantic
rule. The schema declaration supplies its retention identity, the semantic rule supplies its
exact origin, and the existing matching/unwind mechanism consumes it. A potential modeled
exception cannot enter this route merely because it has a class name: it needs an actual
exceptional completion premise. Publishing exception summaries as a new channel remains S4;
the current persisted completion row does not independently expose an exact exception class.

## 6. Correctness and fidelity gates

These are inspection judgments for the restricted extension, not qualification receipts.

| Gate | Verdict | Evidence and scope |
|---|---|---|
| G1 Authority | pass | Schema owns exact exception identities; attributed context rows own class/MRO observations |
| G2 Fidelity | pass, scoped | Exact `TypeError` is distinct from potential exceptions; corrected F01 retains native MRO completeness |
| G3 Validity | pass, scoped | Handler identity, unique pinned class/module, source linkage and publication reconstruction are checked; tests remain pending |
| G4 Hidden behavior | pass, scoped | Arbitrary exception construction, named-handler cleanup and exception-group handling are refused |
| G5 Consistency/recovery | pass, scoped | Active exception is saved/restored around handler and finalizer execution, including refusal paths |
| G6 Transformation/reuse | pass, scoped | Ordered source and class/MRO evidence survives completion composition; no new generic proof interpretation is added |
| G7 Claims | pass, scoped | No full L2/S2, exception-channel or integrated acceptance is claimed |
| G8 Library fit | pass, scoped | Public pinned APIs now retain completeness that the report projection drops; no fork/pin change or tail-name heuristic |
| CI-G1 | pass, scoped | Unknown earlier handlers stop selection; nonmatch requires retained underlying completeness |
| CI-G2 | pass at canonical boundary | Cited source/class/module/MRO facts enter the proof; full FORMAT 9 structural closure remains S6 |
| CI-G3 | pass, scoped | No evaluation-reference input is introduced by class retention or handler matching |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Report MRO omits object and drops linearization completeness

**Interface-checked against the pinned source.** The initial matcher required the raised
class's dense MRO to end in builtin `object` before proving nonmatch. Pyrefly deliberately
omits both self and object from `ClassMro`'s stored ancestors. Moreover,
`PysaClassMro::Resolved(Vec<ClassRef>)` drops `ClassMro::Resolved::linearization_complete`;
the underlying resolved vector can be a recovery prefix after nonlinearizable inheritance.
Thus the object-tail premise withholds the intended later-handler positive, while merely
removing that premise or accepting any nonempty/dense report is insufficient to prove absence.

Evidence: pinned `pyrefly/lib/report/pysa/class.rs` lines 160–163 and 576–583;
[ClassMro's contract and completeness API](https://github.com/facebook/pyrefly/blob/3e3177d0f4755b56c2d5a710d830eed89b14c2e3/pyrefly/lib/alt/types/class_metadata.rs#L816-L918).
The no-fork API route is already public at this pin: `get_all_classes` and `get_class_mro`
operate on the `ModuleAnswersContext` already constructed by context extraction;
`ClassId::from_class` maps to the existing report key. Retain the underlying
`linearization_complete()` result in the attributed context contract, and treat object as
an omitted implicit ancestor under that qualified provider contract. A narrowly pinned
builtin hierarchy model is an alternative, but a tail-name heuristic is not a replacement
for completeness.

**Owner:** context extraction/schema and completion's nonmatch premise. **Principles:**
DP-02/DP-08/DP-15, CI-02/CI-04. **Closure:** later unrelated builtin handler then TypeError
matches with retained complete MRO evidence; incomplete/cyclic/marker MRO stays unknown;
source/projection tests preserve the completeness field. **Disposition:** active plan S2/S7.

**Correction re-inspected, Implemented:** extraction now maps `ClassId::from_class` from
`get_all_classes` to `get_class_mro(...).linearization_complete()`, using its existing
`ModuleAnswersContext`. The attributed `ContextClassMro` rows retain the flag for ordinary and
marker rows; a schema check requires complete rows to be acyclic. Missing metadata is
conservatively false. Completion requires nonempty, dense, noncyclic, fully identified
ancestors and all completeness flags before nonmatch. The object-tail requirement is removed.
A positive cited ancestor may still establish ancestry from a recovery prefix; absence may
not. The compile probe, source rerun and reviewed schema migration are pending at this
checkpoint, not presumed passed.

### Other inspected properties

The following properties hold at source-inspection strength for the corrected extension:

- Explicit raises establish `TypeError` only for the supported exact invalid primitive
  operands after their evaluation succeeds. Opaque values remain unknown; bare re-raise
  requires a known active exceptional state. No arbitrary exception constructor is assumed
  to complete.
- Handler matching follows source order. An unknown type cannot be skipped to reach a later
  convenient handler. Matching uses the pinned class identity or an attributed MRO ancestor,
  and retained evidence includes the type reference/resolution and class facts.
- Nonmatch requires dense, noncyclic, fully identified raised-class MRO rows with retained
  native completeness as corrected under F01; the caught class must itself have `BaseException`
  ancestry. An arbitrary builtin such as
  `object` is not silently treated as a valid nonmatching handler. The
  `ContextClassMro::marker_shape` contract ensures resolved rows have all ancestor identity
  fields, and the pure raised-MRO matcher also checks them. Provider attribution remains a
  premise; this is not a claim about arbitrary runtime class mutation.
- A selected named handler is refused before its body, preserving uncertainty about binding
  replacement and implicit cleanup. A nonmatching named handler need not be refused because
  its binding never executes. `except*` is explicitly marked at extraction and refused by
  this completion path.
- A handler executes with the pending exact exception active and restores the previous
  exception afterward. An exceptional pending completion is active while its finalizer runs;
  normal finalization preserves the pending outcome and abrupt finalization replaces it.
  These are separate from the exception still active in an enclosing handler.
- Handler scans, MRO evidence and emitted proof expansion charge the existing completion
  work budget; proof admission still uses the shared summary cap. No eager class closure or
  new unbounded return/condition expansion is introduced.

FP-01/FP-03/FP-04/FP-06 are satisfied for this bounded change: the exact-kind declaration,
provider observations, Python composition and store lifecycle remain distinct, with pure local
tests available. FP-02/FP-05 are satisfied for F01's corrected explicit completeness boundary. This assessment does
not establish completeness of Python exception handling or
qualify the older modeled-handler candidate relations as actual completion evidence.

**Focused verification still worth retaining:** direct re-raise inside a handler is present
in the new fixture. Also distinguish a handled exception from one still active: after
`except TypeError: pass`, a bare re-raise in that try's finalizer must not reuse the handled
exception; a nested finalizer inside an active handler may re-raise it; an inner handled
exception in a finalizer must restore its outer pending exception. Current save/restore logic
supports these distinctions by inspection. Independent runtime controls should challenge
them as S7 grows; this review does not claim those checks ran.

## 8. Library fit

**Interface-checked:** class identity and hierarchy come from the existing pinned provider
contract. Reimplementing builtin ancestry by names or adding a separate class graph would
duplicate authority. An execution engine or reasoning-library change would not supply
Python's pending-outcome/active-exception distinction. The existing pure module and bounded
table adapters are sufficient for this extension; no dependency or framework change is proposed.

## 12. Judgment

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | satisfied, scoped | Exact-kind retention is shared; matching and unwind behavior stay in completion |
| A2 Encode meaning structurally | satisfied, scoped | Pending/active exceptions are typed and native MRO completeness survives the report projection |
| A3 Extend through composition | satisfied, scoped | Handler selection and finalizers compose the existing expression/statement contracts and ordered proof |

**Bounded decision: Accept scoped by inspection after F01 correction.** Excluded: arbitrary exception construction,
named matched-handler cleanup, exception groups, general handler-entry proofs, dynamic class
mutation, new exception summary channels and full native structural closure. Revisit when one
of these consumers is introduced or the provider MRO contract changes. Source/native reruns,
schema migration checks and independent runtime evidence remain execution obligations.
**Enclosing Stage 3 architecture and integrated qualification: not assessed.**

## Implementation receipt — 2026-09-26

**Tested, scoped.** Following the inspection correction, the author ran:

- `cargo check --release -p cpg-core --quiet` — **passed**, including the public pinned Pyrefly API route.
- `uv sync --locked --reinstall-package lctx-semantics` — **passed**, editable native build.
- `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p lctx-analytics --lib -p cpg-core --test compile --test bundle -E 'test(completion::tests) | test(composed_argument_reads_keep_ordered_source_evidence) | test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` — **passed**, 11 cases. Source/Delta/native controls include typed/superclass/later handlers, re-raise, nonmatch, shadowing, named cleanup, exception groups, unknown earlier types and tampered MRO completeness.
- `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema --test contracts --test codebooks --status-level fail --final-status-level fail` — **passed**, 13 cases after reviewing and accepting compiler-output-79 schema snapshots. Forced snapshot generation was preparation only.
- The [independent oracle command](../evidence/2026-09-26_typed-completion/README.md) — **passed**, 100 generated CPython/Hypothesis observations, separate from compiler admission.

The initial old re-raise assertion failed because the supported result is now Raise; it was
updated with an independent inactive-state withholding control. The later-handler source case
failed under the incorrect object-tail requirement and passed after retaining native completeness.
Formatting, integrated gates and Stage 3 acceptance are **not_run**, per the operator's timing.
