# Stage 3 primitive work accounting — compact review

## 1. Scope and coverage

**Tested (2026-09-25), change/conformance.** This reviews the exact-input assessment counters in
`cpg-schema`, their native PyO3 transport, and the typed FastMCP page. It follows the earlier
value-path inspection review's F02. I inspected the bounded apply and the two language
boundaries, exercised a refuted path, an unlinked unknown path, an assignment-cap refusal, and
the generated fixture through native and MCP calls. I did not inspect operation-wide closure,
effect/role filters or clean-wheel packaging; those remain Stage 3 exit work.

## 2–5. Authority, contract, derivation and representative journeys

The immutable generation's condition and value links remain the input authority. One shared
`TheoryWork` records links examined, assignments completed, the sum of input BDD node products
preflighted, and peak BDD nodes. The Rust result retains that work when a budget refuses an
answer. The native tuple preserves each field; the Python response types them and computes
page-local sums and a peak, without deciding a semantic verdict. A single refuted path reports
one completed assignment and a nonzero preflight bound. A path with no links stays unknown with
zero assignment work. A 33-atom path reaches the 32-assignment cap and returns a named boundary,
not false. Cursor continuation is another page; no work from an unexamined page is claimed.

The pair figure is deliberately **not** the number of internal BDD pairs visited. It is the
preflight product used to bound each attempted conjunction, including one refused by the
aggregate budget. That distinction is part of the served contract. The page's `examined_rows`
counts source rows separately.

## 6. Gates

| Gate | Verdict | Basis or remaining action |
|---|---|---|
| G1 authority | Pass, scoped | Counters come from the shared native assessment; Python only aggregates. |
| G2 fidelity | Pass, scoped | Preflight bound and completed operations have distinct names; refusal is unknown. |
| G3 validity | Pass | The shared cap refuses the 33rd assignment in the focused test. |
| G4 hidden behavior | Pass | Read-only query of a pinned generation. |
| G5 consistency | Pass, scoped | Native and MCP fixture results agree; clean wheel remains unrun. |
| G6 transformation | Pass, scoped | Typed transport preserves per-path counts; page sum/peak have explicit scope. |
| G7 claims | Pass | No internal pair-visit or operation-wide claim is made. |
| G8 library leverage | Pass | biodivine owns BDD operations, PyO3 transport and Pydantic typed results. |
| CI-G1 fidelity | Pass, scoped | Unknown remains separate from refuted and compatible. |
| CI-G2 evidence closure | Partial | Full proof-step source spans remain the earlier review's F03. |
| CI-G3 evaluation integrity | Pass | Focused analyzed fixtures, no gold input. |

## 7–10. Findings, alternatives and verification

**F02 closed within path-local scope.** The earlier review's missing BDD work counters now have a
consumer. This deliberately adds no new persisted relation or bundle version: query-time work
depends on the exact request, so storing it with the immutable generation would be the wrong
authority. A Python reimplementation of BDD work counting would duplicate the Rust semantics.
No exception is needed. CI-G2 remains partial for full proof-step source spans; the trigger is
the planned explained compatibility result. Operation-wide negative closure and total work over
all cursor pages remain deferred until the Stage 3 compatibility filter exists.

**Passed, 2026-09-25:** targeted `cargo test -p cpg-schema --lib` filters for the exact-string and
assignment-cap cases; `cargo check -p lctx-semantics`; `cargo clippy -p cpg-schema -p
lctx-semantics --lib -- -D warnings`; `uv run pytest
python/lctx_mcp/tests/test_native_semantics.py python/lctx_mcp/tests/test_value_paths.py -q`
(9 tests); and targeted `uv run ruff check` on the changed Python files. Formatting, full
`just test-all`, pilot, structured evaluation and clean-wheel checks are **not_run** by the
operator's end-of-functional-scope instruction.

## 11–12. Decision

**Accept the path-local accounting change.** Keep the preflight wording and unknown-on-cap
contract when the future operation-wide API reuses these counters. No §B decision changed.
