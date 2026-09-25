# Stage 3 Pydantic validation candidate — compact review

## 1. Scope and coverage

**Implemented and Tested for catalog parsing (2026-09-25), change/conformance.** This reviews
one pinned `pydantic==2.13.5` TypeAdapter candidate, the empty-site fixture and the model
contract. I read the pinned local method signature, Context7's Pydantic `TypeAdapter` use
examples, and an older read-only FastMCP snapshot's dependency module and method definition.
Current-compiler binding to that full dependency context, source application and pilot behavior
were not executed in this slice.

## 2–5. Authority, contract, derivation and journeys

The committed TOML model is the assertion authority. The exact dependency pin, module and
qualified method enter target identity; the compiler binds only a matching context definition.
The instance-selected schema and user validators remain unknown. The sole authored rule is a
**potential** `Parameter[object]` to `ReturnValue` transform. Normal return and the other four
channels are not asserted. The empty-site fixture therefore publishes zero Pydantic targets,
as it must. The pinned local method's `object` formal and an older pilot snapshot's matching
module/definition establish interface plausibility, not current binding.

The available `validate(schema)` effect variant requires a named schema. Writing
`pydantic.TypeAdapter` there would name the adapter class rather than the runtime-selected
schema. The model deliberately withholds that effect. A typed dynamic-schema case and a source
witness are prerequisites to adding it.

## 6. Gates

| Gate | Verdict | Basis or remaining action |
|---|---|---|
| G1 authority | Pass, scoped | Committed model bytes and exact pin determine model identity. |
| G2 fidelity | Pass, scoped | Potential transfer stays distinct from validation effect and normal completion. |
| G3 validity | Pass | Tagged TOML parser and formal-path test admit the model; empty dependency context stays dormant. |
| G4 hidden behavior | Pass | Catalog parsing and binding are read-only; no validator is invoked. |
| G5 consistency | Unresolved for real binding | Full pinned dependency context has not been compiled with current code. |
| G6 transformation | Pass, scoped | The existing model compiler owns the typed path; no new parser or renderer. |
| G7 claims | Pass | No validation-effect, source behavior or positive summary is claimed. |
| G8 library leverage | Pass | Pydantic supplies actual validation; the compiler records only bounded evidence about it. |
| CI-G1 fidelity | Pass, scoped | Dormant model is not a behavior claim. |
| CI-G2 evidence closure | Not applicable | No served claim uses this candidate. |
| CI-G3 evaluation integrity | Pass | The reference sources are not compiler inputs. |

## 7–10. Findings, alternatives and verification

**F01, deferred:** the current effect contract cannot express a runtime-selected validation
schema without conflating it with a named one. The consequence is missing validation-effect
coverage, not a false positive. Revisit when L3 or a registered query needs a TypeAdapter
validation effect; add a typed dynamic-schema variant and source binding before claiming it.
A free-text `"dynamic"` schema sentinel would duplicate and weaken the typed contract.

**Passed, 2026-09-25:** focused `cargo test -p cpg-schema --lib
models::tests::committed_catalog_has_typed_identity_path_and_digest` and the empty-site
`cargo test -p cpg-core --test compile
pinned_identity_models_require_and_publish_their_real_formals -- --exact` (the latter after
the final catalog edit). The read-only older snapshot query found
`pydantic.type_adapter`/`TypeAdapter.validate_python`, but is not a test of current binding.
Current FastMCP binding, formatting, integrated `just test-all`, fresh pilot, structured
evaluation and clean-wheel acceptance are **not_run**.

## 11–12. Decision

**Accept as a dormant, potential transfer candidate.** The compiler must keep it dormant
without matching pinned dependency context. Full binding and effect semantics remain open.
