# PR5 bounded agent journeys and coarse rebuild qualification

**Completed at bounded Tested strength, 2026-09-29.**
[ADR-0081](../../../adr/0081-bounded-agent-journeys-and-coarse-rebuilds.md) and
[forward-plan PR5](../../../plans/behavioral-model-forward-plan_2026-09-24.md#pr5-detailed-execution-adr-0081)
own the scope. The [assembled review](../../reviews/design_review_pr5-agent-journeys-rebuild_2026-09-29.md)
accepts source structure; its four findings have repairs. Current disposition belongs to forward-plan §6.2.

## Qualification commands

| Command | Outcome / boundary |
|---|---|
| `just fmt`; `just test-all` | Passed through localized repair and continuation: 485 ordinary Rust tests accounted for, 204 Python tests, 19 real-PG Rust and three real-PG Python tests, SQLx metadata, types, lints, rules, dependencies, gold and ADR/agent checks. The initial Rust run had three failures; the initial Python run had two stale tool-list failures. |
| `cargo nextest run --release -p cpg-core --test rebuild` (focused selectors through `scripts/build_environment.py`) | Eight rebuild controls passed: clean/reused complete-row equality, malformed and semantically wrong cache recovery, member addition/deletion, missing documentation, moved source coordinates, coverage and provider-attribution inputs. Behavioral clean/reused provenance equality and selected-policy invalidation also passed. |
| `uv run --no-sync pytest` and focused tool-list rerun | 204 passed, three intentionally skipped, after two stale tool-list expectations were repaired. No heldout task executed. |
| `uv run --no-sync pyrefly check --summary=none` | Passed after narrowing SDK content and documenting dynamic validated packet access. |
| `uv run --no-sync python scripts/deployment_check.py --source build/sources/fastmcp/004bf15a2ba99f077160993c00a404e1a9da83ea --out build/pr5-qualified-tasks` | Passed: pinned disposable programmatic and CLI stdio tasks each returned 5. |
| `uv run --no-sync python docs/design_review/evidence/2026-09-28_pr5/pilot_probe.py build/pr5-qualified build/pr5-qualified-tasks --reuse-published` | Passed: both fresh profiles compiled; 114 real MCP calls covered lexical-only and live NVFP4 routes. Joint reconstruction passed for all 88 relations; both ready profiles and selection were preserved. |
| `just docs-check` | Passed: 173 canonical pages, zero documentation errors. |
| `operator_cutover.py build/pr5-qualified build/pr5-operator-current`; `cleanup_current.py --apply` | Passed: exactly two current ready profiles, behavioral selected; current reconstruction passed over 88 relations; nine obsolete runtime paths, ten obsolete artifact files and eight fixture paths removed. All 58 protected benchmark files unchanged. |
| Accuracy assessment, comparative evaluation, sealed tasks, performance benchmarking | **not_run**, explicitly outside the authorized scope. Benchmark artifacts are preserved. |

The full gate is a composite receipt after localized repairs, not a claim that the first
`just test-all` invocation passed. The remaining real-PG, SQLx, rule and policy components passed in the continuation. Fake-vector tests qualify deterministic contracts only;
the separately named live NVFP4 journeys qualify runtime integration, not retrieval accuracy.

## Rebuild and transport boundaries

The rebuild oracle compares every canonical raw, derived and analysis column after neutralizing
only the enclosing snapshot ID. Source facts, producer/run identity, coverage, evidence coordinates
and attribution are retained. Mutations seed the old cache immediately before rebuilding the new
captured input, then compare with a clean publication. Provider-attribution and coverage mutations
are controlled fixture inputs; they do not claim another provider was actually executed.

Disposable normalization and association cache admission recomputes the existing pure owner and
compares canonical Arrow batches. This prevents validly encoded wrong cache bytes from becoming
authority. It does **not** establish a pure-stage CPU speedup. Shared semantic dependencies
invalidate conservatively. Fine-grained performance work remains conditional on a real consumer.

`packet_probe.py` uses actual stdio MCP listing/calls and measures the SDK's final serialized
`CallToolResult`. It checks browse/vocabulary, typed selection, independent original expansion,
release-only deployment discovery, comparison, optional-section continuation, and declared versus
installed versus observed deployment facts. Both formerly oversized CLI/auth operations must
expose complete expanded cores and bounded behavior pages. Class invocation and singleton-owned
field evidence remain distinct; no first-instance field inference is made.

`pilot_probe.py` compiles pinned inputs with current task observations and the complete NVFP4
embedding specification, imports into disposable PG18, and tests both retrieval routes per profile.
Its explicit `--reuse-published` option is only for a serving-only repair with unchanged canonical
compiler sources and exact task/profile inputs; canonical and serving CLI hashes remain separate.
Temporary reconstruction archives are deleted on exit. `operator_cutover.py` requires successful
both-profile journey and reconstruction receipts, then checks project database quiescence before
replacement. Current-only cleanup follows validated deployment; it never targets benchmarks.

Current runtime inventories and bounds are in [journeys](raw/journeys.json), [operator qualification](raw/operator.json), [cleanup](raw/cleanup.json) and the [composite gate receipt](raw/gate.json). These are bounded current receipts, not retained runtime generations.
