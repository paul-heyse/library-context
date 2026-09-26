# Stage 3 expression and completion slice

## 1. Scope, outcome and coverage

**Interface-checked, 2026-09-26. Bounded decision: Accept scoped after correction.** This is a change/conformance
review of the dirty S1/S2 implementation over `6a3416f`, against
[ADR-0057](../../adr/0057-compositional-stage3-semantics.md). It is not the additional target
or deeper principles-alignment review the operator excluded, and does not certify Stage 3.
Standard: core 3.0, code-intelligence 1.1 and the repository binding.

The reviewed boundary comprises `lctx-analytics::evaluation`, the new
`lctx-analytics::completion`, the associated schema/codebooks, core acquisition and shared
reconstruction, and the adjacent finite-summary and FORMAT 8 native consumers. The owning
architecture is [behavioral analysis §9.9](../../design/sections/behavioral-analysis.md), with
[synthesis and serving §11.3](../../design/sections/synthesis-and-serving.md#section-11-3).
The [active plan §3.0](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns current execution status; this review records the inspected issues, not a second queue.

Inspection occurred while the author continued implementation. The F01/F02 corrections were
re-inspected after landing, including the loop guard on unique local initialization. Neither schema snapshot acceptance
nor an integrated qualification receipt is claimed here. Review commands were source reads and
diff inspection; tests, formatting, pilot and integrated gates were **not_run** by this reviewer.
Author-reported focused test outcomes were leads, not independently rerun evidence. The author
reports six focused completion/proof-cap tests passed; the source/Delta/native rerun remains
pending at this review checkpoint and is not claimed as passed. The reviewer read the
[independent probe, receipt and raw output](../evidence/2026-09-26_expression-completion/README.md):
the recorded command passed 120 CPython/Hypothesis controls, and the two implicit-action child
processes started but did not complete before their timeout. Those observations challenge the
Python semantic premises; they are not a compiler-versus-runtime comparison or proof of divergence.

### Owners, facts and composition

| Owner | Responsibility and consumer contract | Fidelity and limits |
|---|---|---|
| `cpg-schema` | Persisted expression/statement outcomes, ordered steps, binding inputs, append-only kinds | Derived certificates identify source facts and snapshot; normal evaluation is separate from exact Boolean value |
| `evaluation` | Ordered selected operands and pinned normal-call/signature admission | Source reads are normal/opaque; arbitrary truthiness is refused; skipped branches do not contribute steps; depth/work refusals remain typed |
| `completion` | Statement outcomes and ordered pending-return frame actions | `Normal`, abrupt completions and `Unknown` are explicit; normal finalizers preserve a pending return, abrupt finalizers withhold it |
| Core adapters/validators | Acquire relations, order writes and reconstruct the same semantic owner | No second statement interpreter in SQL; reconstruction checks stored outcomes and ordered proofs |
| Finite summaries/native | Embed frame proofs and admit immutable served paths | Existing FORMAT 8 proof-length limit and incomplete structural closure constrain this new producer; F01 and S6 remain material |

The relevant change scenario is extending a normal finalizer from `pass` to a selected
expression or local binding. The semantic addition belongs in `completion`, reusing expression
certificates; core and finite consumers transport its typed steps. This is a credible module
boundary: pure tests need no Delta or embedding infrastructure. However, consumer resource
limits and implicit Python execution must also be part of that contract, as F01/F02 show.

For nested pinned calls, `prepare_calls` requires all declared list signatures to bind the
same explicit arguments to the same formal names. Missing required formals, unsupported
unpacking and ambiguous targets withhold completion. Omitted optional parameters are not
evaluated at the call site and are not invented as exact values. The restriction to nonempty,
agreeing overloads is conservative; support for broader signatures/default values remains S2
work. Adding an ordinary normal model should change the model declaration and its source
fixtures, rather than introduce another SQL expression evaluator.

## 6. Correctness and fidelity gates

Verdicts below are inspection judgments for the corrected slice, not command outcomes.

| Gate | Verdict | Evidence and required action |
|---|---|---|
| G1 Authority | pass | Schema owns rows; analytics owns semantics; core reconstructs rather than copying the interpreter |
| G2 Semantic fidelity | pass, scoped | F02 correction withholds opaque raises, prior bindings and loop-enclosed assignments; broader completion remains unsupported |
| G3 Validity | pass, scoped | Ordered reconstruction and conservative guards have enforcement points; schema acceptance and pending source regressions are separate execution receipts |
| G4 Hidden behavior | pass, scoped | F02's implicit constructor/rebinding paths are refused rather than assumed normal |
| G5 Consistency and recovery | pass, scoped | F01 correction shares the native bound and retains a typed producer refusal; the source/native regression receipt remains pending |
| G6 Transformation and reuse | pass, scoped | Snapshot-scoped rows, deterministic sorting and aligned proof capacity are present; FORMAT 9's full closure remains excluded |
| G7 Truthful capability claims | pass, scoped | Implementation is explicitly partial; no broader qualification is inferred |
| G8 Library leverage | pass, scoped | Python completion meaning belongs in the bespoke semantic module under §B5; relational acquisition remains DataFusion-owned |
| CI-G1 Fidelity | pass, scoped | The corrected normal-outcome admission retains unknown on the implicit-execution cases identified here |
| CI-G2 Evidence closure | pass, canonical scope | Ordered expression detail is expanded into frame proofs and reconstructed before publication; full served structural closure remains the accepted S6 target |
| CI-G3 Evaluation integrity | pass, scoped | No gold/reference input was introduced in the examined production path |

## 7. Findings and applicability

<a id="F01"></a>
### F01 — Completion output and native proof admission have different limits

**Interface-checked; high priority.** `completion.rs` permits a 4096-work statement/frame
certificate. `summaries/finite.rs::direct_flows` and `push_finite_path` append its finalizer
steps without a final proof-size check. `python/lctx_semantics/src/lib.rs` rejects a summary
with more than 64 proof steps. A function returning a formal through a `finally` suite with
64 `pass` statements supplies 64 finalizer witnesses plus the raw identity witness: the
canonical producer can admit a proof that the native executor refuses to load. Longer newly
supported assignments/branches reach the same discrepancy sooner. The mismatch also existed
for sufficiently large pass-only proofs; the expanded producer increases its relevance.

This violates FP-02/FP-05, DP-08/DP-20 and CI-08: the consuming contract does not accept all
producer successes, and resource exhaustion becomes a generation-level failure instead of an
origin-specific unknown. **Correction owner:** schema/finite-summary proof admission, with native
as the adjacent consumer. Establish one shared bounded proof contract; apply the bound after
all predecessor, nested-call and finalizer steps are assembled and retain an explicit refusal
for the affected origin. Changing only the native constant leaves other proof producers
unbounded. **Closure:** focused at/beyond-limit producer controls, followed by a real
source-to-Delta-to-native case that preserves the typed refusal. **Disposition:** plan §3.0
S1/S6 and §6 W1/W5; the plan owns current status.

**In-review correction, Implemented by inspection:** the author added schema-owned
`MAX_SUMMARY_PROOF_STEPS` and producer checks in both named assembly functions while this
review was being written. A focused cap test was added. This record does not claim that test
or the full source/native closure has run; the plan should retain the resulting receipt.

<a id="F02"></a>
### F02 — Statement completion must account for implicit execution

**Interface-checked; high priority.** The initially inspected `StmtRaise` branch treated a
normally evaluated opaque operand as an immediate raise, and a bare handler could turn that
into `Normal`. Python instantiates an exception-class operand, which can invoke an unproved
constructor. During review the author restricted this branch to exact non-exception literal
operands, retaining unknown for opaque values. That correction is **Implemented by inspection**;
this reviewer has not run its regression.

The related `StmtAssign` route admits any normally evaluated right-hand side and directly
bound local name as `Normal`. Replacing a previous local value can release its last reference
and invoke its destructor. A finalizer assigning `marker = None` can therefore run unproved
code if `marker` previously held an object; scope and binding identity alone do not certify
normal completion. Python documents both class instantiation by `raise` and possible
destructor invocation by local rebinding in its
[simple-statement semantics](https://docs.python.org/3/reference/simple_stmts.html).
The existing architecture distinguishes divergence from total normal completion, and no
object-finalization exclusion was found in the inspected owner.

This concerns FP-05, DP-02/DP-08/DP-18 and CI-06. **Correction owner:** completion admission
and its runtime-model contract. Preserve the opaque-raise refusal. For local assignment,
either establish first initialization/harmless old-value release from source evidence, or
make a deliberate and visible model assumption about object finalization; do not equate a
normal RHS with a normal statement. The latter choice needs the usual owning-document/decision
route if it changes accepted semantics. **Closure:** a positive fresh local binding, an opaque
exception-class withholding case, and a prior-object overwrite case challenged independently
against Python. **Disposition:** plan §3.0 S2 and its runtime-oracle obligations in S7; current
status belongs in that plan.

**In-review correction, Implemented by inspection:** the author now limits local assignment
to a unique binding for `(snapshot, scope, name)`, so an earlier parameter/assignment/handler
binding with that name withholds completion. Ordered expression proof steps are also expanded
into completion proofs, retaining the operand evidence rather than only the root syntax.
The initial correction still admitted a unique syntactic assignment under a loop. The
re-inspected correction now performs a bounded ancestor walk and refuses `for`/`while`,
cycles, missing parents and exhausted ancestry before admitting initialization. The new
`repeated_initialization` source control requires those per-statement outcomes to remain
unknown. Expression proof expansion is charged against completion work. These changes close
the identified admission defects **by inspection**; the pending source-test receipt must remain
distinct from this judgment.

### Boundaries already supported and unresolved work

**Interface-checked:** FP-01/FP-03/FP-04/FP-06 are satisfied for the inspected separation of
source contracts, pure semantic functions and store adapters. Proof ordering is explicit;
source reconstruction is useful against tampering, though agreement with the same producer is
not an independent Python-semantics oracle. Duplicate read certificates refuse admission,
opaque names do not establish truthiness, and normal/effect/identity evidence is kept distinct.

**Unresolved:** complete predecessor statement coverage, contextual/default values, supported
synchronous context-manager exits, typed handler matching, multi-channel summaries and full
FORMAT 9 closure are outside the claimed implemented slice and remain in S2–S6. Completion's
pure inputs also rely on publication's keyed source contracts; that precondition should remain
explicit when adding standalone consumers. No acceptance of malformed arbitrary row sets is
inferred from the current helper tests.

## 8. Library fit and total complexity

**Interface-checked:** this change uses existing Arrow row declarations and DataFusion
acquisition/validation, while the selected-child and frame semantics are Python-specific
transformations under §B5/ADR-0057. A generic interpreter framework or replacing the SCC engine
would not supply the missing implicit-execution premises. The smaller viable correction is
to strengthen the shared completion/proof contracts and their existing consumers. No new
library API or dependency selection is recommended by this bounded review; the broader
multi-channel scheduler comparison remains S4 work.

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied, scoped | Expression/statement semantics can be extended and tested in analytics; schema changes deliberately reach adapters |
| A2 Encode meaning structurally | satisfied, scoped | Outcome kinds, implicit-execution refusals and shared proof capacity are explicit admission premises |
| A3 Extend through composition | satisfied, scoped | Canonical expression/frame composition carries ordered evidence and respects the existing native capacity |

**Bounded change decision: Accept scoped, by inspection after F01/F02 corrections.** This
accepts the restricted expression/frame contract, not all S2 execution semantics or full native
evidence closure. Revisit when predecessor consumers broaden statement use, implicit protocols
gain support, or FORMAT 9 changes proof admission. Pending source/native tests and reviewed schema
snapshots remain execution obligations, not presumed passes. **Enclosing architecture: not
assessed** beyond the stated adjacent consumers. The accepted Stage 3 design, remaining S1–S8
work and integrated qualification remain separate obligations in the forward plan.

### Subsequent implementation receipts (2026-09-26)

The implementation agent subsequently reported `passed` for the four focused source/Delta/native
tests recorded in [STATUS](../../../STATUS.md#last-verified-2026-09-26), including the real
64-step positive/65-step refusal and implicit-execution withholding cases. It reviewed and
accepted the schema snapshots, then passed 15 schema/binding cases. These receipts resolve the
execution checks that were pending at review time; they are not tests independently run by this
reviewer. F01/F02 are corrected for this bounded slice; full Stage 3 remains unqualified.
