# Stage 3 finalizer proof steps — compact review

## 1. Scope and coverage

**Tested (2026-09-25), change/conformance.** This review covers one append-only
`summary_flow_step_kind`, the direct and modeled finite proof producers, their shared
validation and focused source fixtures. I inspected the source `return_exit_statuses` witness,
the summary-ID recipe and the FORMAT 7 step projection. Multi-frame exits, `with` suppression,
callbacks, resources and full source-span rendering remain outside this slice.

## 2–5. Authority, contracts and journeys

`return_exit_statuses` is the shared source-derived authority for whether one literal
`finally: pass` frame can be discharged. Its cited `pass_fact_id` now enters the finite
summary's ordered proof as `finalizer_pass` (code 10). A direct identity has raw-value then
pass steps. A modeled call, assignment return or acyclic local wrapper inserts the pass before
its `return_exit` step. The canonical `summary_id` hashes the resulting sequence; the
publication validator re-derives it. The FORMAT 7 projection carries the step kind, fact id
and condition id through its existing structural index.

The ordinary direct return has no invented pass step. One pass frame yields a positive path
with a cited pass. An effectful `finally`, two pending pass frames and `with` stay without a
positive summary. Removing the pass step from a registered relation triggers the shared
summary-step source-equality violation. This proves structural evidence closure for that
step; source text/span presentation is still open.

## 6. Gates

| Gate | Verdict | Basis or remaining action |
|---|---|---|
| G1 authority | Pass | One status relation supplies the fact; no second finalizer classifier. |
| G2 fidelity | Pass, scoped | Only the already admitted sole-pass frame gains a proof step. |
| G3 validity | Pass | Re-derived step sequence rejects a removed pass. |
| G4 hidden behavior | Pass | Pure derivation; analyzed fixture code is not executed by the compiler. |
| G5 consistency | Pass, scoped | Compiler version 69 and append-only codebook distinguish new output. |
| G6 transformation | Pass | Ordered proof and canonical ID change together; FORMAT 7 keeps the typed step. |
| G7 claims | Pass | No claim about general `finally` completion or operation-wide behavior. |
| G8 library leverage | Pass | DataFusion selects source witnesses and the existing typed recipe carries identity. |
| CI-G1 fidelity | Pass, scoped | Uncontrolled frames remain unknown. |
| CI-G2 evidence closure | Partial | Pass fact id is cited; full step source spans are not yet served. |
| CI-G3 evaluation integrity | Pass | Source fixture and CPython oracle are independent of gold. |

## 7–10. Findings, alternatives and verification

**F01, deferred:** an ID alone does not give a client the pass source span. The consumer can
join the same-snapshot `syntax_nodes` fact during a future explained-query projection; defer
that projection until full proof-step span coverage is built. A free-text pass marker would
lose identity and make the source claim uncheckable.

**Passed, 2026-09-25:** focused `cargo test -p cpg-core --test compile` filters for nested
return frames and pinned modeled identity paths, including the missing-step validator control;
targeted `INSTA_UPDATE=no cargo test -p cpg-schema --test contracts rules_snapshot -- --exact`
after inspecting and accepting its one codebook addition; `cargo clippy -p cpg-core --test
compile -- -D warnings`. Formatting, `just test-all`, pilot, structured evaluation and clean
wheel remain **not_run** until all Stage 3 functionality lands.

## 11–12. Decision

**Accept the cited sole-pass proof.** It strengthens path evidence without changing the
permitted control scope or a §B decision.
