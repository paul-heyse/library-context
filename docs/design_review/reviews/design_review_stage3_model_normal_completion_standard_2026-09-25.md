# Stage 3 pinned model normal completion — standard design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | ADR-0033; DESIGN §B5 and §9.9; the authored `normal_return` assertion and its compiled `model_targets` field |
| Standard | Core 2.0; code-intelligence profile 1.0; library-context binding |
| Tier · purpose | Design · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the narrow pinned-target assertion; no source-call summary is certified |

**Outcome and baseline.** A transfer rule was conditional on a callee returning, while exception silence provided no termination proof. The change lets two pinned `typing` helpers assert their own normal completion. I inspected the catalog parser/binder, schema, shared validator, pinned CPython 3.14.7 source, Python typing documentation, focused tests and DESIGN/ADR. I did not inspect other modeled bodies, prove source-call dispatch, run the pilot, or trace a served answer. Those are outside this assertion's supported scope.

## 2. Authority and identity map

| Fact or relation | Authority and fidelity | Revision and identity | Consumer |
|---|---|---|---|
| Normal-completion assertion | Authored `external.toml`; model claim, not provider-observed execution | Catalog bytes in compiler digest; model revision and ID | `Model.normal_return` |
| Pinned target | `context_definitions` from the analyzed context | Context fact and target node IDs; source pin | `Catalog::bind_targets` |
| Compiled assertion | Derived `model_targets.normal_return` with `synthetic_model` origin | Snapshot, model and target IDs | Future source-call composition |

The pinned CPython body and documentation justify authorship for `typing.cast` and `typing.assert_type`; they are not a second writable compiler authority. `false` denotes absence of this positive assertion, not proof of non-return. A model revision or catalog-byte change moves compiler identity; a published table row cannot independently redefine the claim.

## 3. Contracts and invariants

| Contract | Enforcement point | Failure behaviour |
|---|---|---|
| Completion assertion requires complete exception coverage and no authored exception rule | `Catalog::parse`, `models.rs:608` | Reject the catalog |
| Assertion requires a pinned function target | `Catalog::bind_targets`, `models.rs:720` | Reject active non-function target |
| Compiled assertion equals the committed model and cited context | Shared `model-catalog-target-equality` validator, `validate.rs:1013` | Reject publication of missing, extra or changed rows |
| Absent assertion remains unknown | Default-false authored field; DESIGN §9.9 | No positive completion claim |

The Arrow contract gains a non-null Boolean field, and compiler output version 56 marks the migration. The reviewed insta diff adds only that field to `model_targets` (plus insta's assertion-line metadata). No negative normal-completion state is asserted.

## 4. Derivation and execution

| Stage and question | Projection, method and dependencies | Exactness, bounds, lineage and effects |
|---|---|---|
| Parse: is a completion claim well-formed? | Serde/TOML model with explicit coverage, rule collection and default false | Exact catalog validation; no analyzed-code execution |
| Bind: which pinned target receives it? | Existing context-definition match for scope, pin, module and callable | Only cited function facts; dormant models do not invent targets |
| Publish: does the stored assertion equal the catalog? | Existing typed Arrow/DataFusion table and shared source-equality validator | Reject forged row before snapshot append; no new graph projection or iteration |

The source target universe is the pinned context, not a name-only lookup. This stage does not analyze calls, form a call graph, prove argument evaluation, or apply an SCC summary. Its data volume is one Boolean per bound model target; release-scale cost is not measured.

## 5. Journeys

For an ordinary pinned `typing.cast`, `normal_return = true` binds to the context function fact and publishes beside the model ID. A source call still needs exact target selection and endpoint/path proof. A new model without this assertion leaves normal completion unknown. A model that authors an exception rule or only partial exception coverage is rejected; a non-function target is rejected when active. Changing the model bytes changes compiler identity; a forged published flag fails the same validator used at publication. Interruption before snapshot append follows the existing attempt boundary; this slice adds no external effect.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Catalog assertion is the sole editable meaning; table and DESIGN derive from it. | — |
| G2 Fidelity | Pass, scoped | Positive assertion is distinct from transfer and exception channels; false is explicitly absence of assertion. | Preserve the unknown interpretation in consumers. |
| G3 Validity | Pass | Parser/binder and shared publication validator reject invalid combinations and forgery. | — |
| G4 Hidden behaviour | Pass | Compile reads the committed catalog and pinned context; no runtime import of analyzed code. | — |
| G5 Consistency | Pass | The existing attempt validates before one snapshot publication. | — |
| G6 Transformation | Pass | The compiled field retains model and target identity without inferring from silence. | — |
| G7 Claims | Pass, scoped | Claim concerns two pinned targets, not arbitrary calls or total Python-program termination. | Keep L3 call proof separate. |
| G8 Library leverage | Pass | Serde/TOML, typed Arrow/DataFusion and the existing validator do the generic work. | — |
| CI-G1 Fidelity | Pass, scoped | Authored model, pinned target and source-call candidate retain separate status. | Do not promote candidate calls without dispatch and exit proof. |
| CI-G2 Evidence closure | Not applicable | No served answer uses this field yet. | Trace the assertion through FORMAT 7 when serving is built. |
| CI-G3 Evaluation integrity | Pass | The gold reference is not a model input; source and docs are independent of it. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | The assertion is not yet a source-call completion proof. | DP-08, CI-06 · CI-G1 | `model_targets` has no call-site path/dispatch witness; promoting `call_transfer` now would overclaim. | Compose only after exact unique target, endpoint, condition and enclosing-exit evidence is available. |
| F02, deferred | Total normal return for more complex models needs its own qualification. | DP-15, DP-22 · G7 | A future callback, blocking or divergent target could satisfy no-exception coverage yet never return. | Keep the authored flag absent until pinned source and independent oracle support it. |

F01–F02 are boundaries on future composition, not defects in the two asserted pinned helpers. DP-01/02/03/04/05/08/09/11/13/14/15/18/19/21/22/23/24 and CI-01/02/04/06/07/10/12 are satisfied for this scope. Graph structure, heuristics and served projections (DP-07/12, CI-03/05/08/09/11/13) are outside this bounded catalog change. No SHOULD-level exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Built-in or adopted feature | Fit |
|---|---|---|---|
| Model parsing | Domain invariant only | Serde tagged structs, TOML parser | Unknown fields and typed rule variants already enforced. |
| Relational publication | Catalog-to-target binding | Arrow schema, DataFusion and shared validator | Existing target equality checks the new field without a second test-only implementation. |
| Runtime source witness | Authored semantic assertion | CPython 3.14.7 source and typing docs | Supports only the two simple helpers; no generic termination analyzer is justified. |

## 9. Alternatives

| Alternative | Meaning and locality | Risk and decision |
|---|---|---|
| Baseline: transfer plus exception catalog, no completion | One authority but every call completion stays unknown | Sound; insufficient for the first L3 positive model step. |
| Infer completion from definite transfer or no exception row | Minimal schema change | Unsound for divergence; rejected. |
| Analyze arbitrary dependency bodies for termination | Avoids authored claim | Unbounded Python termination problem and stubs may lack bodies; rejected. |
| Explicit pinned assertion through existing model catalog | One additional field and invariant; no new generic engine | Selected for narrowly inspected helpers. |

The library-owned and simplest viable alternatives coincide: existing Serde/Arrow/DataFusion machinery carries a small authored domain claim. No library offers a general proof of Python termination under this contract.

## 10. Verification plan

| Claim or risk | Label | Evidence and conditions | Current result or gap |
|---|---|---|---|
| The two pinned helper bodies directly return `val` | Interface-checked, 2026-09-25 | `uv run --no-project --python 3.14.7` with `inspect.getsource`; Python 3.14 typing docs | Passed for `cast` and `assert_type` only. |
| Invalid catalog/target combinations are rejected | Tested, 2026-09-25 | Focused release Nextest `normal_return_requires_complete_exception_coverage_without_exception_rules` and `binding_requires_the_exact_context_and_a_cited_definition` | Passed; includes partial coverage, exception rule and class target. |
| Published rows retain exact authored flags | Tested, 2026-09-25 | Focused release Nextest `pinned_identity_models_require_and_publish_their_real_formals` with forged false flags | Passed; shared validator rejects forgery. |
| Schema migration and static quality | Tested, 2026-09-25 | Reviewed `contracts__model_targets.snap.new`, `cargo insta accept --workspace`; focused release Nextest, `cargo clippy --release -p cpg-schema -p cpg-core --all-targets -- -D warnings`, `just adr lint` | Passed. Integrated `just test-all` and `just pilot`: not_run by operator instruction. |

## 11. Authority changes and exceptions

ADR-0033 amends DESIGN §B5 and §9.9. No accepted decision is superseded: it adds a positive authored assertion while keeping ADR-0022's model/summary boundary. Binding conflicts K1–K3 do not alter this decision. There is no exception record.

## 12. Decision

**Accept the narrow model-level completion assertion.** The pinned source and docs support its two current instances, focused tests exercise rejection and publication, and no current path uses the field to publish a source-call claim. F01–F02 remain explicit Stage 3 work. The complete integrated gates remain `not_run` until functional Stage 3 is assembled.
