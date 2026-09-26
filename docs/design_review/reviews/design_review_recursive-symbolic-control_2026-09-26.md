# Design review: guard-fixed symbolic control in recursive value paths

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding in
`standard.toml` apply. The subject is the exact direct-formal extension of
ADR-0053's two-argument local-call relation and its source-to-Delta-to-native
proof. [Plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns the remaining general composition work.

## Ownership, contract and scenario

The change scenario is mutual recursion where `f(value, stop)` calls
`g(value, stop)` only on the `stop` branch and `g` has a finite return of
`value` on its own `stop` branch. The callee's evaluated atom has a different
source identity from the caller's. Plain string equality or an unqualified
call edge would not prove a transfer.

The `cpg-schema` DataFusion relation requires the second positional argument
to be a directly read caller formal: Pass B gives one definite mapping to a
distinct callee formal, its source parameter matches the lexical name binding,
ty's exact reaching row proves normal name evaluation,
and both formal tests have direct entry-value links. The `lctx-analytics`
worklist checks that the caller's *existing* path BDD implies its linked
truthy atom or its negation. It then restricts the callee BDD by that fixed
value under the kernel's deterministic bounds and admits only a true result.
The output keeps the caller condition and source origin. The ordered proof
cites the evaluated argument, `caller_condition_link` (append-only code 15),
`callee_condition_link` (code 14) and exact callee summary. The shared
validator reconstructs the derivation before publication. Native admission
requires the caller link to be direct, on the caller operation, fixed by its
condition and immediately before the callee link; the compact bundle cannot
independently reconstruct the full source mapping.

An opposing guard cannot reuse the finite base. A caller condition that leaves
the control value undetermined, an indirect or possibly deleted argument, a
residual callee BDD or a kernel cap stays unknown. This case does not publish
a new cross-scope BDD root; general substitution and conjunction remain
planned work. The path is a conditional may-path under the stated source and
model abstraction, not a guarantee of normal return for all inputs.

## Gates, library fit and decision

| Gate | Scoped judgment |
|---|---|
| G1–G3, CI-G1 | Satisfied for the supported case: the two source-specific links, ty reaching row, caller condition, callee summary and origin remain distinct. Shared publication validation recomputes the result; the opposing branch yields no recursive positive. |
| G4–G6 | Satisfied: the relation, worklist and caps have explicit inputs, deterministic order and typed unknowns. No ambient search or unbounded condition operation was added. |
| G7, CI-G2 | Satisfied for path-local native admission; an invalid caller link is refused. Operation-wide transfer/absence claims remain outside scope. |
| G8 | Satisfied: pinned DataFusion joins, biodivine BDD implication/restriction and the existing petgraph SCC schedule cover this narrow relation. A new substitution framework or rule engine would add machinery before multiple channels share rules. |
| CI-G3 | Unaffected: gold references are not compiler inputs. |

FP-01–06 and applicable DP-01/02/03/04/08/11/12/13/16/18/19/21 and
CI-01–04/06/08 are satisfied for this bounded extension. There is no new
architectural finding. General argument expressions, cross-scope BDD
conjunction, other summary channels and real cap publication remain
[plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
work; the earlier literal-control review is historical evidence for that
separate case.

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib
summaries::finite --quiet` passed 14 pure controls, including true/opposite
caller guards; `RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core
--test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries
--quiet` passed the real mutual-recursion positive and withholding case;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed a real Delta/native proof and open boundary; `uv run --no-sync pytest
python/lctx_mcp/tests/test_native_semantics.py -q` passed 12 controls,
including a forged caller-link rejection. Reviewed append-only codebook and
generated-rule snapshots passed `INSTA_UPDATE=no cargo test -p cpg-schema
--test codebooks --quiet` and `--test contracts --quiet`; targeted Clippy
for `lctx-analytics`, `cpg-schema` and `lctx-semantics` passed.
`just fmt`, `just test-all`, `just pilot` and full Stage 3 evaluation are
**not_run** pending functional completion.

**A1 satisfied, A2 satisfied, A3 satisfied. Accept scoped** at Tested
strength. The enclosing Stage 3 architecture and integrated product
qualification remain unresolved.
