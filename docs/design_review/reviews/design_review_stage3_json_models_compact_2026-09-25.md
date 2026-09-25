# Stage 3 JSON model family — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Pinned `json.dumps` and `json.dump` model rules and candidate source application |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept as potential transform/effect candidates; keep custom behavior open |

I read the CPython JSON documentation through Context7, inspected the typed catalog and source-binding SQL, and ran the focused `model_shapes` release Nextest case (1/1 passed on 2026-09-25). It verifies the candidate transform and exact `fp` subject. I did not establish whole-call completion, the behavior of custom encoders, or a served effect. Integrated gates remain deferred.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | The two authored rulesets live in the catalog; target/formal identity comes from pinned context definitions. | — |
| G2 Fidelity | Pass | `obj` and `fp` are distinct typed subjects; stream I/O is attached only to `fp`. | — |
| G3 Validity | Pass | Existing typed parser, formal validation and shared source-equality validator apply; focused compile published and validated. | — |
| G4 Hidden behavior | Pass | Catalog bytes enter compiler identity; no ambient JSON module import is used to decide a model. | — |
| G5 Recovery | Pass | Failed target/formal binding aborts before publication. | — |
| G6 Transformation | Pass, scoped | `json.dumps` is a potential transformed value source, not an identity or guaranteed result. | Compose only with exact call-path proof. |
| G7 Claims | Pass, scoped | The catalog uses potential modality and partial/unspecified coverage. | Keep incomplete channels out of negative claims. |
| G8 Library leverage | Pass | Typed Serde model parsing and DataFusion source joins are reused. | — |
| CI-G1 Fidelity | Pass | Synthetic model actions stay distinct from observed or completed effects. | — |
| CI-G2 Evidence closure | N.a. | No served summary consumes these actions yet. | Check in FORMAT 7. |
| CI-G3 Evaluation integrity | Pass | The source fixture and CPython docs are not the FastMCP gold family reference. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | Custom `cls` or `default` can invoke user code whose effects this model does not enumerate. | CI-04, CI-06 · CI-G1 | The catalog marks effect coverage partial and callback coverage unspecified. Treating absent rules as absence of an effect would be unsound. | Preserve open coverage in L3 and serving; add a qualified callback rule only if exit timing can be stated. |

Within the stated candidate-model scope, DP-01/02/03/04/05/07/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/10/12 are satisfied. Recursive summary, graph, heuristic and served-projection obligations remain outside this slice. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| Catalog extension | Two typed TOML declarations | Existing Serde parser, pinned context binding and DataFusion joins | The focused test binds JSON formals to source arguments. | Keep this as data, without a JSON-specific source recognizer. |
| Serialization and I/O | No runtime implementation | CPython's `json` module is the behavior being described | The model records only potential actions; it does not execute serialization. | Leave custom encoder effects open. |

## 12. Decision

**Accept the bounded JSON candidate models.** The focused release Nextest case passed 1/1 on 2026-09-25. A stream-write candidate cites `fp`, while serialization cites `obj`; no completed effect or negative behavior follows. Integrated `just test-all`, pilot, structured evaluation and formatting remain `not_run` by the operator's end-of-scope policy.
