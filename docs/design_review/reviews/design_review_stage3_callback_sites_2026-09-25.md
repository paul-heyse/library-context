# Stage 3 modeled callback sites — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `modeled_callback_sites` producer, schema, publication validator and source fixture |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as candidate-local modeled callback evidence only |

The target is a cited callback action at a real source call site, preserving the authored action and exit separately from source-argument identity and call-target certainty. I read the model catalog, model application and argument-binding inputs, relation, schema, validation and focused `atexit.register` cases. This slice does not establish whole-operation callback registration, callback invocation, local callback storage or a resource fate. Integrated tests and the pilot remain `not_run`.

## 2. Authority and fidelity

| Fact or relation | Authority and identity | Fidelity and coverage | Consumer |
|---|---|---|---|
| Source call target | Ruff call/Pysa target fact plus `model_applications` | Candidate target, original modality and completeness | Callback-site relation |
| Authored action | Committed catalog's `model_callbacks` rule and pinned context target | Synthetic model action, exit and modality | Callback-site relation |
| Callback value | `model_argument_bindings` for the same rule/path/call | Bound source argument or explicit unknown reason | Callback-site relation; future summary |
| Published site | Snapshot/call/Pysa fact/model/rule | Candidate-local application, not an observed invocation | Shared validator; future L2/L3 |

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Call, model rule and argument evidence retain distinct identities and origins. | — |
| G2 Semantic fidelity | Pass, scoped | Registration, storage, forwarding and invocation remain distinct codebook actions; exit and modalities survive. | — |
| G3 Validity | Pass | Typed shape/referential rules and shared reconstruction reject forged status or missing rows. | — |
| G4 Hidden behavior | Pass | No callback is executed; only committed model and source facts are read. | — |
| G5 Consistency and recovery | Pass | Written and validated in the same snapshot attempt. | — |
| G6 Transformation and reuse | Pass | Model/call/rule identities and compiler version 33 govern rebuilds. | — |
| G7 Truthful claims | Pass, scoped | Row docs and DESIGN distinguish candidate modeled action from execution or whole-operation fate. | — |
| G8 Library leverage | Pass | DataFusion joins typed source/model relations; Arrow/Delta and shared validation own persistence. | — |
| CI-G1 Fidelity | Pass, scoped | An unknown callback argument remains unknown; no registration is relabelled as invocation. | — |
| CI-G2 Evidence closure | Not applicable | No served callback claim reads this row. | Revisit at L3/FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | Test fixture is independent of catalog and gold reference. | — |

## 7. Findings and principle verdicts

No in-scope defect was established. **D01:** even a bound callback argument at a complete candidate target does not prove the modeled normal exit occurred. A whole-operation `registered` claim could be false if the call raises or is path-conditional. Compose flow regions, exits, call dispatch and model effects before serving. **D02:** release-local callback storage/forwarding/invocation remains unmodeled; absence of site rows cannot refute those actions.

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are satisfied within the candidate-site scope by the authority and gates above. DP-07/12/20 and CI-03/05/07/08/09/11/13 remain obligations for flow composition, graphs and serving.

## 8. Library-leverage ledger

| Capability | Own code | Qualified built-in | Fit and decision |
|---|---|---|---|
| Source/model/action join | One domain relation in `behavior.rs` | DataFusion joins and Arrow typed columns | Keep one source of application meaning; no Python adapter rule. |
| Callback result | Not yet composed | Existing condition kernel, petgraph SCC | Use only after path/exit and candidate closure. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| Bound registration, two unknown argument shapes, tamper, schema/rules/ledger and compiler digest | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass` | `passed` (7/7) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as candidate-local modeled callback evidence only.** The action, exit, dispatch and argument status are independently preserved. D01 and D02 bar a served callback fate or negative claim from this source alone.
