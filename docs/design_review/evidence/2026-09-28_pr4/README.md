# PR4 selection and retrieval evidence

**Implemented and bounded Tested, 2026-09-28; live embeddings waived.** The [forward plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md#pr4-execution)
owns execution and disposition; [ADR-0077 (retired; current successor)](../../../adr/0131-evidence-values-and-contextual-delivery.md)
owns decisions. This evidence does not qualify the complete product.

The assembled [design/target review](../../reviews/design_review_pr4-selection-retrieval_2026-09-28.md)
accepts the implementation and named focused controls. PR4/F01–F18 are closed at bounded Tested strength;
the forward plan owns final closure. Canonical unit reconstruction, foreign operand/receipt
refusal, cancellation-safe CPU slots, source context, quantification, exact rank arithmetic and
bounded unit headers have focused controls.

**Operator waiver:** embedding-related PR4 tests are **not_run**, at the operator's explicit
request while a critical GPU benchmark runs. Both real-library profiles and actual PostgreSQL/MCP
qualification use `--embedder none`. Deterministic fixture vectors do not qualify live embeddings
or retrieval quality. The briefly started repository launcher was stopped before qualification;
the benchmark's service was not stopped or changed.

[ADR-0078 (retired; current successor)](../../../adr/0131-evidence-values-and-contextual-delivery.md) requires a complete pivot after validation.
Obsolete runtime generations, compatibility paths, matching runtimes and rollback backups were
removed after the validated current cutover. The fixed six-query lexical development comparison is diagnostic
only; no confirmation/heldout material is read and no Context7 superiority claim is made.

The code gate is **passed after a localized expected-output repair**, with an explicit composite
[receipt](raw/code-gate.json). The final `just test-all` run passed formatting/lints and ran all
474 Rust tests without fail-fast; its sole failure was the output guard changed by canonical
selection-domain ordering. That guard's focused rerun passed, followed by all remaining gate
components. This is not a claim that the unmodified `just test-all` command exited successfully.

| Command, 2026-09-28 | Outcome and scope |
|---|---|
| `just fmt`; `just fmt-check`; `just test-all` lint subgates | **passed:** Rust/Python formatting, Clippy and Ruff |
| Release workspace nextest; localized `all_techniques_guard` rerun | **passed after repair:** all 474 tests accounted for; 20 intentionally skipped in the ordinary run. [Full run](raw/test-all.log), [repair](raw/guard-repair.log) |
| `just py-check rules-scan rules-test lint-agents fixtures-check deps gold test-postgres sqlx-check`; `just adr lint` | **passed:** 191 Python tests, Pyrefly, seven rule suites, 90 fixture parses, dependency/fork/shear/gold/agent/ADR checks, 19 real-PG Rust tests, two real-PG Python tests and SQLx metadata. [Continuation](raw/gate-continuation.log) |
| Catalog nextest with reversed source/catalog inputs | **failed before correction; passed after:** canonical domain bytes and IDs are independent of scan order. [Before](raw/domain-order-red.log), [after](raw/domain-order-green.log) |
| `deployment_check.py ... --out build/pr4-qualified-tasks` | **passed:** real isolated programmatic and CLI stdio tasks returned 5. [Programmatic](raw/task-programmatic.json), [CLI](raw/task-cli.json) |
| Packet-budget follow-up; scoped `cargo clippy --release -p lctx-postgres --all-targets --quiet -- -D warnings` | **passed:** one boundary test, 16 affected Python tests and Clippy. [Boundary](raw/packet-bound.log), [Python](raw/packet-python.log), [composite receipt](raw/code-gate.json) |
| `LCTX_POSTGRES_TEST=1 uv run --no-sync pytest tests/scripts/test_postgres_serving.py`; scoped Ruff; `uv run --no-sync pyrefly check` | **passed:** three real-PG checks of current split provisioning, existing-state refusal, async lifetime and cancellation; lints/types passed. [PG](raw/bootstrap-pg.log), [Ruff](raw/bootstrap-lint.log), [Pyrefly](raw/bootstrap-types.log) |
| `uv run --no-sync python docs/design_review/evidence/2026-09-28_pr4/pilot_probe.py build/pr4-qualified-pilots build/pr4-qualified-tasks` | **passed:** both fresh current profiles compiled, imported and served with embeddings disabled. A later `--reuse-published` run requalified the serving-only packet fix; receipts record both CLI digests. [Pilots](raw/pilots.json), [catalog compile](raw/pilot-catalog-none.log), [behavioral compile](raw/pilot-behavioral-none.log) |
| `uv run --no-sync python docs/design_review/evidence/2026-09-28_pr4/compare_profiles.py build/pr4-qualified-pilots` | **passed:** all eighteen catalog relations match after excluding run-qualified citation IDs and declared profile-only fields, preserving semantic fields and multiplicity. [Parity](raw/profile-parity.json) |
| Current-format backup/restore in `pilot_probe.py` | **passed:** 88 relations, both ready profiles, native capability loading and selected pointer. [Reconstruction](raw/pilot-reconstruction.json) |
| `operator_cutover.py build/pr4-qualified-pilots build/pr4-operator-current`; `cleanup_current.py --apply` | **passed:** operator schemas replaced, both profiles imported/reconciled/served, behavioral selected, current reconstruction passed, obsolete runtime assets removed. [Cutover](raw/operator-cutover.json), [cleanup](raw/cleanup.json) |
| `just docs-check`; `just fmt-check`; `git diff --check` | **passed:** 168 canonical pages, zero link/fragment errors, formatting and whitespace checks. [Publication](raw/docs-check.log), [format](raw/fmt-check.log) |
| `lctx serving status --generation b87c881a3838e21ddf1399089631d2472cc4d476094b343059c44cec5d463630 --verify-artifacts` | **passed:** current schema, ready/available generation and every required artifact available after cleanup. [Status](raw/operator-status.json) |

Each profile contains 5,097 members, 18,010 addressable units and 24,246 fragments. The serving probe
checks declared-parameter strict/discovery and union-type selection, six lexical queries and original
bytes across all four families. Catalog hydrates all 30 requested expanded packets. Behavioral
hydrates 28 and explicitly refuses two above 256 KiB; all winners of each refused member are expanded
through terminal original continuation and checked byte for byte. `fastmcp.cli.cli.run` measures
567,783 bytes (catalog section 30,677), and `fastmcp.server.auth.JWTVerifier` 351,727 bytes (catalog
19,367). [Diagnostic](raw/packet-diagnosis.json). Optional enrichment pagination/omission is a PR5
usability consideration, not a newly imposed PR4 acceptance condition.

**Measured, shared-machine run; not a controlled performance comparison:** catalog compilation
reported 239.4 seconds total and 3,904 MiB peak, behavioral 408.7 seconds and 5,082 MiB. Original
compiles used the same pinned task receipts. The serving-only rerun did not modify canonical
producer sources and revalidated complete imports. No live embedding, hybrid quality, comparative
superiority, general semantic completion or full product usability is established by these results.

The fixed six-query lexical development diagnostic retained the requests and channel. Its top-five
declaration overlap counts are 1, 0, 2, 1, 2, 0 for both current profiles. This measures movement,
not relevance improvement. [Catalog](raw/development-comparison-catalog.json),
[behavioral](raw/development-comparison-behavioral.json). The obsolete baseline runtime, inputs and
one-off reader were deleted once these current conclusions were recorded.

Only the two current canonical profiles and their required serving artifact closure remain active.
Temporary reconstruction dumps were deleted after verification. Current test fixtures retain one
selected generation each; acquisition inputs, build caches and unexecuted evaluation inputs remain
inputs. The obsolete administrator upgrade CLI and duplicate pending credentials were removed;
fresh bootstrap provisions current roles, pgvector and separate protected configs directly.

## NVFP4 deployment adoption, 2026-09-28

**Implemented and bounded Tested (ADR-0080).** The operator selected the published gpu-stack
B3/r2 SM120 wheel and NVFP4-r2 checkpoint. This is basic working validation, not an accuracy or
ranking assessment. The launcher verifies the manifest identity and every checkpoint checksum;
new canonical spec `c7a04584c626a62efda4de0a2a9bf9852ff2730344821996c669acb37e1f4d16`
separates the new vectors from prior BF16 keys. Keep the 1024/float32/MRL/L2 contract.

| Command | Outcome and scope |
|---|---|
| `uv lock --project services/vllm`; `uv sync --project services/vllm --locked`; wheel SHA-256 and installed metadata check | **passed:** optimized r2 wheel, torch 2.14.0+cu132, CUDA runtime 13.4.2; the existing TileLang/Z3 pairing remains installed |
| `uv run --no-sync pytest -q python/lctx_mcp/tests/test_embed_serve.py python/lctx_mcp/tests/test_embedder.py` | **passed:** nine targeted checks, including changed checkpoint/manifest refusal |
| `python scripts/build_environment.py -- cargo build --release -p lctx` | **passed:** CLI includes the new canonical spec |
| `uv run --no-sync python scripts/embed_serve.py` | **passed:** verified local checkpoint, controlled role E startup on loopback port 8000; the first launch compiled CUDA kernels |
| `uv run --no-sync python docs/design_review/evidence/2026-09-28_pr4/embedding_smoke.py` | **passed:** health, served identity, three tokenizer admissions and production-client output validation; [receipt](raw/nvfp4-smoke.json) |
| `just adr index`; `just adr lint` | **passed:** current decision index and record checks |
| Accuracy assessment, live cosine conformance, ranking evaluation, benchmarks and `just test-all` | **not_run:** explicit operator instruction |

The detached job `build/embedding-adoption/reembed.py` rebuilds behavioral then catalog with
`--embedder vllm`, the existing task observations and a temporary pinned CLI hard link. This is a
data update, not a test suite. `status.json`, `behavioral.log`, `catalog.log` and `service.log` in
that directory own live progress. Successful completion reports `rebuilt_pending_current_only_cutover`;
failures report the failed stage. Import/selection and retirement of superseded runtime assets remain
pending completion of both builds. No old generation is retained as a recovery requirement and no
benchmark artifact is removed. No local speedup, retrieval relevance or BF16-equivalence claim is made.
