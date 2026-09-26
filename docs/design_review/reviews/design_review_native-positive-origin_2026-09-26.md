# Design review: positive source identity in native value paths

**2026-09-26 · change/conformance · scoped author review.** Core 3.0,
code-intelligence profile 1.1 and the library-context binding declared by
`standard.toml` apply. The subject is FORMAT 8's existing `summary_flows`
source columns, the native projection, and `inspect_value_paths`'s response.
[Plan W5](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns the remaining boundary-cause work; this review is dated evidence.

## Contract and scenario

The realistic input is one raw returned use with a proved parameter origin and
an unresolved sibling origin. The compiler already stores distinct
`source_flow_fact_id` and `source_origin_id` on `summary_flows` and
`summary_boundaries`; the shared validator checks the source-to-summary
relation before publication. FORMAT 8 exports both columns. Its checked IPC
decoder now forwards the positive columns to the immutable native executor,
which retains them with each admitted path. The Python adapter names both
fields in the typed `ValuePath` response, matching `OpenBoundary`. A user can
therefore distinguish the two source contributions in one paged result without
an additional Delta or generation-table lookup. Summary ID remains the proof
identity, not a replacement for the source-origin identity.

Adding another path with the same raw fact changes producer rows, not the
serving interpretation or response shape. An API consumer can join a path to
its canonical source fact/origin; the native reader cannot independently
reconstruct the compiler's full source derivation from its compact bundle, so
the shared publication validator remains the stronger authority. Python only
transports the native IDs and never infers a positive from a missing boundary.
These are attributed derived may-paths under the declared model; a path-local
refutation remains path-local.

## Gates and decision

| Gate | Scoped judgment |
|---|---|
| G1–G3, CI-G1 | Satisfied in the production route: schema-owned columns are decoded by name, checked publication retains the source link, and the two-origin fixture keeps both distinct. The synthetic constructor accepts IDs for focused native tests; it is not a publication proof. |
| G4–G6 | Satisfied: no new read-time data source or identity recipe; the existing generation supplies the two IDs and pagination remains bounded. |
| G7, CI-G2 | Satisfied for one inspected formal: the response exposes both positive and open source identities but makes no operation-wide claim. Full claim-support closure remains outside this slice. |
| G8 | Satisfied: the existing Arrow IPC projection, PyO3 executor and Pydantic response suffice. Another lookup layer or new table would duplicate the already published identity. |
| CI-G3 | Unaffected: gold capability families are not generation inputs. |

FP-01–06 and applicable DP-01/02/03/04/08/11/16/18/19/21 and
CI-01/03/04/06/11 are satisfied for this serving extension. There is no new
architectural finding. W5's work/node-cap publication and the enclosing
operation-wide semantic response remain [plan-owned](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).

**Tested 2026-09-26:** `cargo check -p lctx-semantics --quiet` passed;
`uv sync --locked --reinstall-package lctx-semantics` rebuilt the editable
extension; `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -q`
passed 12 native controls; `uv run --no-sync pytest
python/lctx_mcp/tests/test_value_paths.py -q` passed three typed/MCP controls;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle
finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed the same-fact Delta/native trace;
`cargo clippy -p lctx-semantics --lib --quiet -- -D warnings` and targeted
`uv run --no-sync ruff check` passed. `just fmt`, `just test-all`, `just pilot`
and clean-wheel query are **not_run** pending functional completion.

**A1 satisfied, A2 satisfied, A3 satisfied. Accept scoped** at Tested
strength. The enclosing Stage 3 architecture and integrated product
qualification remain unresolved.
