# Stage 3 modeled resource sites — compact change review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Typed resource paths, `modeled_resource_sites`, source fixture and shared validation |
| Standard | Core 2.0, code-intelligence profile 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as candidate-local resource source evidence only |

I traced `ResourcePath` through catalog compilation, pinned model application, Arrow schema,
DataFusion relation, Delta publication and validation. The focus was whether a resource path
could be linked to a source expression without treating call syntax as runtime object identity.
I examined the `builtins.open` and shadowed-name fixture and the stored-row tamper case. I did
not review lifecycle pairing, condition/exits composition or a served resource claim. The
integrated gate and pilot remain `not_run`.

## 2. Authority and fidelity

| Fact | Authority and identity | Fidelity and coverage | Consumer |
|---|---|---|---|
| Source call | Ruff call plus Pysa target and resolution facts | Candidate call target, with modality and target-set openness | `model_applications` |
| Resource action | Committed tagged model AST, pinned to the target definition | Path kind, action, exit and modality are separate | `model_resources` |
| Resource source | Call expression on `ReturnValue`; exact signature binding on parameter | Source expression only, never runtime object identity | `modeled_resource_sites` |
| Stored row | Snapshot/call/Pysa/model/rule key | Candidate-local source, explicit unknown for unproved field/global/binding | Shared publication validator; future L2/L3 |

## 4–5. Transformation and journey

The catalog compiler emits `resource_path_kind` from the tagged AST. DataFusion joins an
applied target to the authored rule and, only for a parameter path, to the exact argument
binding. A return path cites the call expression and its source fact. A field, global, or
unbound parameter leaves both source ids absent and records an unknown reason. Source ids
and statuses are reconstructed by the publication validator. On the fixture, `open(path)`
produces a candidate acquisition at its call expression; a shadowed `open` produces none.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Tagged catalog path and pinned call target remain distinct authorities. | — |
| G2 Semantic fidelity | Pass, scoped | Input/output, path kind, action, exit, modalities and unknown source are typed. | — |
| G3 Validity | Pass | Codebook, reference and shape rules plus shared reconstruction reject doctored status. | — |
| G4 Hidden behavior | Pass | The join reads pinned facts and catalog; it executes no analyzed code. | — |
| G5 Consistency | Pass | Relation is written and validated in the same snapshot attempt. | — |
| G6 Transformation | Pass, scoped | Source expression ids are cited; compiler output version 34 names the schema migration. | — |
| G7 Truthful claims | Pass, scoped | Row docs and DESIGN do not equate a call expression with a runtime resource or completed action. | — |
| G8 Library leverage | Pass | DataFusion joins, Arrow contracts and Delta publication provide the generic mechanisms. | — |
| CI-G1 Fidelity | Pass, scoped | Unknown source and open target stay explicit; acquisition is not release. | — |
| CI-G2 Evidence closure | Not applicable | No served resource claim consumes the row. | Revisit at summary and FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | The fixture is independent of the authored model catalog and gold reference. | — |

## 7. Findings and principle verdicts

No in-scope defect was established. **Deferred R01:** source expression identity does not
track aliases of the returned object, transfer to another callable, or a matching release.
At L2/L3 composition, give runtime value/lifecycle identity an explicit proof relation; a
call site id must not be used as that identity. **Deferred R02:** an authored normal-exit
acquisition does not prove the source call returned normally. Compose its flow region,
candidate dispatch and exit before serving a behavioral claim. A missing model application
does not establish that no resource was acquired.

DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/10/12 are
satisfied within this candidate-source scope. DP-07/12/20 and CI-03/05/07/08/09/11/13
remain obligations for lifecycle graphs, recursion and serving, not claims of this slice.

## 8–9. Library leverage and alternatives

| Alternative | Fit | Decision |
|---|---|---|
| Parse the displayed `ReturnValue` or `Parameter[...]` spelling | Creates a second grammar and can drift from tagged model input. | Reject. |
| Use DataFusion joins over typed model, application and binding tables | Keeps candidate identity and unknown status in Arrow rows; no generic join code. | Use. |
| Assign call site as runtime resource identity | Falsely merges executions and aliases. | Reject; identify only the source expression. |

## 10. Verification status

| Claim | Label and command | Result |
|---|---|---|
| `open` source, shadowed withholding, tamper, schema/rules/ledger | Tested 2026-09-25; focused `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema -p cpg-core -E '…' --no-tests=pass --no-fail-fast` | `passed` (6/6) |
| Package lint | Tested 2026-09-25; `cargo clippy --release -p cpg-core -p cpg-schema --all-targets -- -D warnings` | `passed` |
| Integrated Stage 3 and pilot | `just test-all`; `just pilot` | `not_run` |

## 12. Decision

**Accept as candidate-local resource source evidence only.** R01 and R02 bar lifecycle or
whole-operation acquisition claims until the summary layer supplies their proofs.
