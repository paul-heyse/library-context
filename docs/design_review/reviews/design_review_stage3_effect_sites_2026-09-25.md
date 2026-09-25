# Stage 3 modeled effect sites — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Typed effect subject, `modeled_effect_sites`, publication validator and `print` fixture |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as candidate-local effect evidence only |

I traced the authored `builtins.print` effect through the typed catalog, pinned target,
source call, Arrow/Delta row and shared reconstruction. I examined subjectless and
bound/unknown subject contracts. This slice does not identify a stream, prove call
completion, or supply a served operation effect. Integrated tests remain `not_run`.

## 2. Authority and fidelity

| Fact | Authority and identity | Fidelity and boundary | Consumer |
|---|---|---|---|
| Source call | Ruff/Pysa call and resolution | Candidate target with modality and open-set state | `model_applications` |
| Authored effect | Committed model rule and pinned context target | Effect kind/argument, modality and optional typed subject | `model_effects` |
| Subject | Exact pinned-signature argument binding when path is a parameter | Unqualified, bound, or unknown with reason | `modeled_effect_sites` |
| Published application | Snapshot/call/Pysa/model/rule key | Candidate modeled action, not observed execution | Shared validator; future L3 |

## 4–5. Transformation and known-answer journey

The catalog compiler emits the subject path kind from its AST. DataFusion joins one model
rule to one pinned call application and optionally to a source argument binding. The
subjectless `print` model produces `unqualified`, with no fake stream expression. A named
parameter subject requires every signature to bind the same explicit argument; field and
global subjects remain unknown. The shared validator reconstructs the row; a changed
subject status is rejected. A shadowed unrelated call has no application.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Call evidence and authored model remain distinct with pinned ids. | — |
| G2 Fidelity | Pass, scoped | Subject absence is typed as unqualified, unlike unknown or bound. | — |
| G3 Validity | Pass | Codebook/reference/shape rules and reconstruction guard the published relation. | — |
| G4 Effects | Pass | Compilation reads pinned facts; analyzed code is not executed. | — |
| G5 Consistency | Pass | Relation is written and validated in one snapshot attempt. | — |
| G6 Transformation | Pass, scoped | Compiler output version 36 names the migration; no operation effect is inferred. | — |
| G7 Claims | Pass, scoped | DESIGN and row contract limit the claim to candidate modeled effect. | — |
| G8 Library leverage | Pass | DataFusion joins and Arrow/Delta contracts replace bespoke relational/storage code. | — |
| CI-G1 Fidelity | Pass, scoped | Potential modality, subject absence and dispatch openness survive. | — |
| CI-G2 Evidence closure | Not applicable | No served effect conclusion consumes this row. | Revisit in L3/FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Source fixture is independent of model catalog and gold. | — |

## 7. Findings and principle verdicts

No in-scope defect was established. **Deferred E01:** a candidate `io.write` model on a
source call cannot show the call reached normal completion or that every possible target
writes. L3 must combine the call region, exit and target-set closure. **Deferred E02:**
subjectless I/O does not identify `sys.stdout`, `file` or another stream; a future
subject claim needs a separately cited argument/default resolution. Absence of a modeled
effect cannot refute an effect under partial or unspecified coverage.

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are
satisfied in candidate-site scope. DP-07/12/20 and CI-03/05/07/08/09/11/13 remain L3 and
serving obligations.

## 8–9. Library leverage and alternatives

| Alternative | Fit | Decision |
|---|---|---|
| Infer a default stream at `print` calls | The model's potential effect does not establish actual stream selection. | Reject. |
| DataFusion join over model/call/argument relations | Keeps provenance and uncertainty as typed Arrow columns. | Use. |
| Promote model effect directly to caller verdict | Loses path, dispatch and completion boundaries. | Reject. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Effect source, tamper, schema/rules/ledger | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass --no-fail-fast` | `passed` (7/7) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as candidate-local effect evidence only.** E01 and E02
bar a whole-operation or stream-specific claim from this row alone.
