# Status

_Updated 2026-10-06 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Graph-native remediation and improvement plan authored; production corrections remain pending.**
The [coordinator §9](docs/plans/graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution)
is the current executable continuation from the
[implementation audit](docs/design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md).
Its four supporting plans develop semantic admission/partitioned compiler preparation, native
bulk and lifecycle correction, scoped/cursor/failure consumers and projection input/stream lifetime.
[Coordinator §7](docs/plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities)
remains the sole finding-disposition owner. All twelve audit findings remain open; authoring a
correction does not close it. Plan targets are Proposed, not repaired/Tested product behavior.

Authoring started from clean `e52e6312`; production remains the audit's unchanged `b90cb611`.
The original graph-native target and SurrealDB-capabilities reviews remain the architectural
basis; the implementation audit supplies current defects/evidence. No other review was made an
additional architecture or qualification prerequisite. Native querying stays selected by ADR-0128.
This remains a hard design-phase pivot with no legacy reader, dual store or historical runtime
artifact retention requirement. Exact existing embedding specifications/values remain authoritative.

The [capability investigation](docs/design_review/evidence/2026-10-06_graph-native-remediation-capabilities/README.md)
records comprehensive SurrealDB skill coverage, targeted Context7/official GitHub/web follow-up,
matched DataFusion foundation advice and exact installed FastMCP/MCP source feasibility. Root
integrated bounded research and independent focused advice, then inspected decisive source. Final
independent plan assessment found no material blocker to the compiler-first start; cache ownership,
backup post-commit uncertainty and selection revalidation clarifications are incorporated.
No new capability probes, dependency upgrades, tooling installation or shared-skill edits occurred.

Material proposed choices: necessary semantic admission separate from producer replay; detached
semantic re-admission; ordered dependency-closed compiler preparation and immutable normalized/
Enriched reuse; complete executable source membership; streamed witness construction/reconciliation;
real cache batch insertion with exact committed winners; gRPC terminally checked backup; one
atomic selection authority; scoped vocabulary, parent-addressed nested cursors and typed safe
MCP failures. SurrealKit/GQL/generic database frontends retain actual consumer triggers rather than
adding a second schema/product authority. Performance reasoning is qualitative, without detailed
accounting, proofs, per-query plan capture or a query-adoption gate.

**Authoring checks, 2026-10-06:** initial `UV_NO_SYNC=1 just docs-check` **failed** solely on two
new subsection fragments. Explicit package anchors repaired those links; `UV_NO_SYNC=1 just docs-check`
then **passed** (322 canonical pages, zero link errors). `git diff --check` **passed**. Root
`UV_NO_SYNC=1 just turn-end` is the final scope-end bookkeeping step.
These are documentation checks, not production remediation or functional qualification.

**Preserved audit evidence, 2026-10-06:** the normalized-environment publisher/serving release
build passed. The
[owned diagnostic](docs/design_review/evidence/2026-10-06_graph-native-pivot-audit/README.md)
reproduced failed selection publishing a different serving pin and cold audit accepting seven
changed search texts. Those intended product guarantees failed. The audit removed its disposable
persistent fixture/credentials/scratch and inspected no operator state. No authoring result repairs
or reclassifies those failures.

**Prior implementation receipts remain historical:**
[Coordinator §8](docs/plans/graph-native-pivot-plan_2026-10-05.md#8-current-checkpoint) preserves
actual native/projection/publication/search/ten-tool Catalog and in-process/stdio MCP results with
exercised revisions and limits. The final wheel build/install/import on production `599010a0`
does not demonstrate every optional behavioral branch or final-source runtime journey. The prior
cache case was sequential winner reuse, not an executed simultaneous same-key race.

**Compiler stage remains complete / user-accepted, 2026-10-05.** Its stopped-test boundary is
preserved. Planned focused remediation counterexamples do not reopen the historical compiler
suite, legacy CLI snapshot/parity or old receipt/grant tests. Current audit findings still prevent
claiming that the full graph-native target is realized.

Production compile/tests, broad `just qualify`, real-library/live-Qwen/operator adoption and
performance measurement: **not_run** in this documentation scope. Heldout/gold inputs, operator
services/client registrations, unrelated worktrees and shared Cargo caches remain untouched.

**Next:** execute R-C0's necessary-versus-diagnostic validation split and invalid-support control,
then coherent R-C1/R-C2 normalization/upper-consumer chains. R-I1 executable capture and R-P3 gRPC
backup can proceed independently on existing inputs. Follow §9's actual prerequisites for the
native/consumer lanes and R-Q0 targeted integrated acceptance. Q1 activation remains separate.
