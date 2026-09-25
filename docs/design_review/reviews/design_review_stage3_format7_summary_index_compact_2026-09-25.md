# Stage 3 FORMAT 7 summary index — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | FORMAT 7 projection of analysis conditions, operation formals, finite value summaries and proofs; native operation/formal lookup |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept this internal positive-path inspection boundary; Stage 3.6 serving remains open |

I traced the published summary and condition tables through the DataFusion bundle query, Arrow
schema/digest, Python generation preflight and PyO3 index. I checked a regenerated fixture and
an operation/formal lookup with a real proof, and rejected missing steps and missing cited callee
summaries. I did not examine pilot cardinality, primitive-origin compatibility, effect/role
filters, source-span hydration, cursors or a clean wheel.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, scoped | `summary_flows` and `summary_flow_steps` remain the published authority; `bundle.rs:203–235` only projects them. Rust/Python schemas share known digest answers. | — |
| G2 Fidelity | Pass, scoped | `lib.rs:176–252` rejects absent/false positive conditions, missing or gapped proof steps and non-finite/unordered callee citations. The result is a may-flow, not universal execution. | Add typed compatibility separately. |
| G3 Validity | Pass, scoped | `generation.py:369–417` checks size, duplicate condition identity and native construction after manifest hashes and schemas. The shared publication validator is still the source-equality gate. | Include remaining evidence/source-span rows before serving proof explanations. |
| G4 Hidden behaviour | Pass | The serving index reads one immutable generation and has no compiler, store or network access. | — |
| G5 Consistency | Pass | FORMAT 7 rejects older manifests; one native object holds both condition catalogs and proof index. | — |
| G6 Transformation | Pass, scoped | Ordered proof identity survives the projection; `value_paths` sorts canonical summary ids and reports its scan count/truncation. | Add a generation/query-bound cursor before a public paged tool. |
| G7 Claims | Pass, scoped | The lookup is internal and returns positive paths and cited open boundaries only. DESIGN labels compatibility and effect/role filtering as still open. | Do not expose a negative or complete verdict from this index alone. |
| G8 Library leverage | Pass | DataFusion performs projection, Arrow IPC carries checked columns, and PyO3 holds the immutable index over the existing BDD kernel. | — |
| CI-G1 Fidelity | Pass, scoped | A public path and formal are resolved to ids before selecting summaries; absent proofs do not become negative. | — |
| CI-G2 Evidence closure | Not applicable to this internal lookup | The step has kind, evidence id and condition, but this slice serves no claim or source-span explanation to an agent. | Add same-snapshot evidence closure before a public explanation. |
| CI-G3 Evaluation integrity | Pass | The fixture test uses analyzed source, not the FastMCP gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | A returned proof step id is not yet a source-span explanation. | CI-11, DP-21 · CI-G2 | `bundle.rs:221–226` projects typed step ids but not the source fact's span; an MCP explanation would be ungrounded if it claimed to show source. | Carry the relevant same-snapshot fact/span rows and validate closure before a public explanation. |
| F02, deferred | An internal truncated value-path lookup has no resumable cursor. | DP-20, CI-08 · G6 | `lib.rs:290–323` reports work and truncation, but a public paged tool could not continue deterministically. | Bind a cursor to generation key, query and last canonical id before exposing the method. |
| F03, deferred | The native index is bounded at 100,000 summary/proof rows without pilot cardinality evidence. | DP-20, DP-22 · G7 | `generation.py:373–378` may refuse a valid large generation; this is an explicit startup failure, not silent truncation. | Measure the fresh pilot at integrated exit and tune the declared load budget if needed. |

DP-01/02/03/04/05/08/09/10/11/12/13/14/15/18/19/20/21/22/23/24 and CI-01/02/04/06/07/08/10/12/13 are satisfied within the internal projection scope. CI-11 requires new work before a future public explanation. Heuristic graph and gold-to-compiler concerns are outside this slice. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke scope | Built-in feature | Fit |
|---|---|---|---|
| Sorted, typed serving projection | SQL selection and field declaration | DataFusion and Arrow IPC | Reuses one pinned published session; no second store reader. |
| Structural condition admission | Identity and source-to-summary links | Existing biodivine BDD kernel through PyO3 | The native index shares the compiler's Boolean mechanism. |
| Public operation lookup | Path/formal indexing only | Rust `HashMap` and PyO3 class | No independent semantic resolver in Python. |

## 12. Decision

**Accept the internal FORMAT 7 value-path inspection slice.** The shared schema digest test,
focused byte-identical bundle rebuild, regenerated Python fixture, native lookup/tamper tests,
targeted Clippy and one FastMCP operation round trip passed on 2026-09-25. The default-stack
debug bundle test initially overflowed; the same exact test passed with `RUST_MIN_STACK=33554432`,
then reached its declared file-list check. `just fmt`, `just test-all`, `just pilot`, integrated
evaluation and clean-wheel acceptance remain `not_run` until the entire Stage 3 functional scope
is implemented.
