# Design review: exact literal control of a recursive value path

**2026-09-26 · change/conformance · scoped author review.** Core 3.0, code-intelligence
profile 1.1 and the library-context binding from `standard.toml` apply. The subject is the
two-argument local-call derivation, bounded conditional callee transfer, ordered proof,
shared publication validator and FORMAT 8 native admission. This is one value-path case
within ADR-0053; [plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns its current completion status. The earlier [worklist review](design_review_recursive-value-worklist_2026-09-26.md)
records the unconditional baseline, not this extension.

## Contract and change scenario

The new source shape is `def f(value, stop): if stop: return value; return f(value, True)`.
The `cpg-schema` DataFusion query selects the tracked first positional argument and a
second exact `True` or `False` literal. Pass B must definitely map the literal to a
distinct callee formal, and `flow_test_value_links` must directly connect that formal
to its entry-value guard. The query returns linked fact IDs and atom rather than a
standalone assertion that the recursive call terminates. The `lctx-analytics` worklist
owns the transfer: it restricts the cited callee BDD on that exact atom under a bounded
operation, admits only a `true` result, retains the caller's condition and origin, and
adds the literal evaluation and `callee_condition_link` immediately before the cited
callee summary. `cpg-core` reconstructs the relation and ordered steps before Delta
publication; native FORMAT 8 admits a conditional cited callee only with a direct
link to its guard atom. Its own check cannot rederive the literal's source value from
the compact serving bundle; the shared publication validator owns that stronger check.

The next likely change is a computed or keyword control argument. That requires an
explicit argument-to-formal value relation and condition substitution in the summary
owner, with a corresponding bounded proof and validator rule. It cannot be admitted by
weakening the SQL literal predicate or by treating an arbitrary call edge as proof.
An unchanged recursive control, another guard atom, a false restriction or a residual
BDD remains unknown. A separately raising control expression is outside this shape.

The source relation is **derived** from pinned analyzer facts, the BDD and summary are
**derived under the stated model**, and the served value path is a conditional may-path,
not a universal return guarantee. Source fact, formal, guard leaf, link, caller origin,
condition, callee summary and step IDs retain distinct identities. The query does not
convert missing extraction into negative evidence. This covers CI-01–04, CI-06 and CI-08
for this narrow relation; operation-wide conclusions and other transfer channels are
outside the reviewed scope.

## Gates, library fit and disposition

| Gate | Scoped judgment |
|---|---|
| G1–G3, CI-G1 | Satisfied for the exact literal case: attributed source and formal links remain separate, a false control withholds, and the shared validator rejects a removed link. A same-value conditional self-call still yields no unjustified positive. |
| G4–G6 | Satisfied in the focused round trip: bounded BDD restriction and deterministic SCC worklist produce an origin-specific proof and typed refusal; publication and native read are explicit. Pair-cap Delta/native publication remains untested. |
| G7, CI-G2 | Satisfied for path-local native admission: the conditional callee requires its immediately preceding cited link to an atom in the callee guard. A broader served claim is not established. |
| G8 | Satisfied for this case: pinned `biodivine-lib-bdd` restriction, DataFusion joins, petgraph SCC order and existing typed contracts suffice. Ascent/datafrog add no proven benefit for one value relation and do not supply this proof semantics. |
| CI-G3 | Unaffected: gold capability families are not compiler inputs or worklist parameters. |

FP-01–06 and applicable DP-01/02/03/04/08/11/12/13/16/18/19/21, CI-01–04/06/08
are satisfied for the scoped path: acquisition, semantic composition, publication
validation and serving have distinct owners, while proof identity and unknowns stay
explicit. There is no new architectural finding for this accepted case. General
argument substitution, BDD conjunction across caller and callee scopes and the other
summary channels remain [plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
work, including the earlier review's F01. Recompare a rule engine only when multiple
channels share the same recursive state and proof obligations.

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib summaries::finite --quiet`
passed 13 pure controls including true/false literal restriction;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile
nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries --quiet`
passed a real source and shared publication check;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed the real Delta-to-native conditional proof and removed-link rejection;
`uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py::test_native_index_refuses_missing_proof_steps -q`
passed a missing-link native rejection;
`INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --quiet` and
`INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet` passed
reviewed append-only codebook and generated-rule snapshots;
targeted `cargo clippy -p lctx-analytics --lib --quiet -- -D warnings` and
`cargo clippy -p cpg-schema --lib --quiet -- -D warnings` passed.
`just fmt`, `just test-all`, `just pilot` and W12's full recursive/effect scope are
**not_run** pending functional completion.

**A1 satisfied, A2 satisfied, A3 satisfied for the bounded change. Accept scoped** at
Tested strength. The enclosing Stage 3 architecture, general conditional recursive
transfer and integrated product qualification remain unresolved.
