# Stage 3 value-path inspection — compact review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | Native paged value-path inspection and typed FastMCP result |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept path-local inspection; retain operation-wide compatibility as open |

The native executor pages canonical summary ids and open boundaries for one real public
operation/formal. Exact primitive requests are type checked before crossing the native boundary.
The result of BDD refutation is local to each cited path. Python binds the cursor to snapshot,
generation, canonical operation, formal, exact input and builtin assumption; it shapes the
native result but does not decide semantic compatibility. The focused fixture and a FastMCP
client round trip checked both a refuted `None` path and a `call_transfer` boundary.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass, scoped | One validated native index supplies every path, condition, boundary and exact-input result. | — |
| G2 Fidelity | Pass, scoped | A refuted path is never promoted to a refuted operation; `unknown` and boundaries are explicit. | Build complete candidate coverage before a compatibility filter. |
| G3 Validity | Pass | Typed Pydantic input refuses cross-kind values; the native method rejects unknown paths/formals, bad offsets and limits. | — |
| G4 Hidden behaviour | Pass | The tool reads its lifespan generation and performs no store access or compilation. | — |
| G5 Consistency | Pass | Both direct Python and in-process FastMCP calls returned the same native result. | — |
| G6 Transformation | Pass, scoped | Native ordering and a generation/query-bound cursor make pages deterministic. The response accounts for examined rows. | Add BDD node/pair-work counts for the final compatibility API. |
| G7 Claims | Pass, scoped | The tool description, result note and fields restrict the claim to a cited summary path. A satisfiable remainder is unknown. | Never treat absence as a negative. |
| G8 Library leverage | Pass | Pydantic/FastMCP supply request and structured-result schemas; PyO3 uses the shared Rust BDD kernel. | — |
| CI-G1 Fidelity | Pass, scoped | The operation and formal are resolved before inspecting a summary. | — |
| CI-G2 Evidence closure | Partial | The path carries ordered proof-step ids and the refuting value-link operand span. Full source spans/text for every step are not yet served. | Add step source evidence before offering an operation-wide explained claim. |
| CI-G3 Evaluation integrity | Pass | The fixture was analyzed source, not the gold reference. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, deferred | Refuting every displayed path would still not close the operation. | CI-05, CI-10, DP-20 · G2 | Unexamined paths, open dispatch and missing L2/model coverage remain possible. | Add complete-candidate certification and report unknown otherwise. |
| F02, deferred | Row work is counted, but BDD work is only internally capped. | DP-20 · G6 | A client cannot see the exact node/pair cost of refutation. | Expose kernel work metrics with the final compatibility result. |
| F03, deferred | Proof-step source spans are absent from the served projection. | CI-11 · CI-G2 | The tool gives stable evidence ids and refuting operand span but cannot show the full proof chain as source. | Project validated source evidence for every step kind. |

No exception to the unknown-is-not-absent rule is requested. The finding actions remain within
the Stage 3 serving and behavior scope; this tool is an inspection surface, not its exit gate.

## 12. Decision

**Accept path-local inspection.** On 2026-09-25, focused native load, exact-input, paging,
request-validation and FastMCP structured-result tests passed; targeted native Clippy passed.
Formatting, `just test-all`, the fresh pilot, structured evaluation and clean-wheel query remain
`not_run` until all planned Stage 3 functionality is implemented.
