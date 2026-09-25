# Stage 3 decoding and compression model data — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Three pinned model targets and their source applications |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the candidate model rows; retain execution and complete-channel claims as unknown |

The authored signatures were checked against the pinned CPython 3.14.7 runtime and against the
[JSON](https://docs.python.org/3/library/json.html) and
[gzip](https://docs.python.org/3/library/gzip.html) documentation through Context7. The model
compiler checks each referenced formal against pinned context signatures. The analyzed fixture
confirms `json.loads(s)`, `gzip.compress(data)` and `gzip.decompress(data)` produce attributed
potential transfer sites; a local parameter named `gzip` produces none. `gzip.compress` also
produces one subject-bound potential compression effect. The existing shared validator rejects
catalog/target/application row tampering. No model asserts normal completion or complete
effects, callback, resource or exception coverage.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, scoped | Catalog bytes, exact target pins and pinned signatures own model identity. | — |
| G2 Fidelity | Pass, scoped | Potential transforms describe data dependence only; decoder hooks and failed calls remain open. | Prove completion and hook fates before a positive summary. |
| G3 Validity | Pass | Typed paths compile and bind to exact `s`/`data` formals in the analyzed fixture. | — |
| G4 Hidden behaviour | Pass | Models are committed static data and cannot execute analyzed code. | — |
| G5 Consistency | Pass | The same model compiler feeds publication and the shared validator. | — |
| G6 Transformation | Pass, scoped | Stable catalog digest and typed path ids identify added rows. | Reassess when richer effect/fate summaries consume them. |
| G7 Claims | Pass, scoped | No `normal_return` or complete-channel assertion was added. | Keep candidate rows distinct from completed paths. |
| G8 Library leverage | Pass | Serde tagged variants and pinned source signatures avoid a new expression parser. | — |
| CI-G1 Fidelity | Pass, scoped | The shadowed module has no modeled transfer site. | — |
| CI-G2 Evidence closure | Partial | Target, rule and source call identities are present; concrete output and exit evidence are absent. | Derive L2 exits and L3 summaries. |
| CI-G3 Evaluation integrity | Pass | The fixture is analyzed source, not gold compiler input. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | JSON hooks may invoke user code and alter results. | CI-05, CI-07 · G2 | `json.loads` accepts decoder classes and parse/object hooks; current catalog only claims potential data dependence. | Add callback and exceptional fates when exact bindings and exits can be cited. |
| F02, deferred | A potential transform is not an executable equivalence model. | DP-19, CI-11 · CI-G2 | CrossHair cannot certify a loose dependence rule as the function's output semantics. | Use an exact restricted pure model if a registered question needs that claim. |

## 12. Decision

**Accept the candidate rows.** On 2026-09-25, targeted `models::tests` (7/7), the analyzed
model fixture (1/1, including shared publication validation) and targeted library Clippy
passed. Formatting, `just test-all`, the fresh pilot, structured evaluation and clean-wheel
query remain `not_run` until the full planned Stage 3 functionality is implemented.
