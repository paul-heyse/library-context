# Stage 3 native primitive refutation — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | FORMAT 7 test-leaf/value-link projection and one-path exact-input refutation |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the internal one-path boundary; public compatibility remains open |

I traced published test leaves and entry-value links through the bundle SQL, Arrow schema,
manifest, Python load and native semantic index. The source value-link rows retain their full
publication validation. The serving projection keeps the fields needed by the shared
`cpg-schema::primitive_theory` kernel, including the effect-rule digest. The focused Python
fixture demonstrates an exact `None` input refuting one conditional `strict` return path; a
satisfiable input remains `unknown`.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, scoped | The projection reads published rows and the manifest records their effect-rule digest. Native admission checks atom, module, condition, place, operation/formal and digest joins. | Include the source-span and proof closure needed by the public tool. |
| G2 Fidelity | Pass, scoped | `refute_value_path` selects an actual summary for the requested public formal, then calls the same bounded primitive kernel as the compiler. Satisfiable or unsupported cases remain unknown. | Aggregate paths only after candidate and boundary coverage is modeled. |
| G3 Validity | Pass, scoped | Manifest hashes, Arrow schemas, load caps and native row checks precede query use. A focused fixture loaded the generated files. | Measure load cardinality on the end pilot. |
| G4 Hidden behaviour | Pass | One immutable native object evaluates in memory and performs no compiler/store read at query time. | — |
| G5 Consistency | Pass | Rust/Python schema digest answers and the generation rebuild matched. | — |
| G6 Transformation | Pass, scoped | The shared Rust kernel uses checked links, not Python reimplementation of source-atom semantics. | Add public work accounting/cursor. |
| G7 Claims | Pass, scoped | The method says only that one cited summary path is refuted under an exact primitive model. It makes no operation-wide negative or positive claim. | Keep the method internal until full served semantics are built. |
| G8 Library leverage | Pass | DataFusion projects the validated rows, Arrow IPC carries them and PyO3 calls the shared bounded BDD theory. | — |
| CI-G1 Fidelity | Pass, scoped | A query formal and summary identity are checked before evaluation; missing entry-value links cannot be inferred from spelling. | — |
| CI-G2 Evidence closure | Deferred for public use | Link ids and operand spans are retained, but no public operation-level proof closure exists. | Validate source and summary evidence closure before serving the claim. |
| CI-G3 Evaluation integrity | Pass | The focused fixture is analyzed source, not the FastMCP gold. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | One refuted summary path does not refute an operation. | CI-05, CI-10, DP-20 · G2 | Other summaries, open target sets, handler paths and unknown boundaries may still admit the behavior. | Aggregate only with complete candidate coverage; otherwise return unknown. |
| F02, deferred | The link projection cites source rows that are not all loaded for native proof explanation. | CI-11 · CI-G2 | The internal method can show the linked operand span but cannot explain the full reaching-fact chain to an agent. | Project and validate same-snapshot proof closure before a public explanation. |
| F03, deferred | The 100,000-row load cap lacks release-scale cardinality evidence. | DP-20 · G7 | A larger valid generation would fail startup explicitly. | Measure and tune at the integrated pilot. |

The other core and code-intelligence principles are met within this internal, one-path scope.
There is no exception to the unknown-is-not-absent rule.

## 12. Decision

**Accept the internal primitive-refutation projection.** On 2026-09-25, the targeted schema
digest checks, byte-identical bundle rebuild, fixture generation, native load/refutation test
and targeted Clippy passed. `just fmt`, `just test-all`, the fresh pilot, structured evaluation
and clean-wheel query remain `not_run` until the full Stage 3 functionality is implemented.
