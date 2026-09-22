# Spike: Pyrefly 1.3.1 (patched) + Ruff 0.0.11 in-process (ADR-0012 evidence)

Run 2026-09-22 on branch `spike/pyrefly-inproc`, toolchain 1.98.1 stable, x86_64 Linux.
Subject: FastMCP 4.0.3 (257 modules from the `fastmcp-slim` wheel, installed with uv into a
spike-only venv; `mcp` 2.2.0 and `mcp-types` 2.2.0 in site-packages), and the fixture
`analysis/fixture/lcfix`, which has a BOM, CRLF line endings, non-ASCII identifiers and
strings, `__all__` re-exports, `@overload`, `@property`, `__new__`/`__init__`, a
higher-order call and f-strings.

Pyrefly source: tag `1.3.1` (`3e3177d0`) plus one commit `b9f28575`. That commit is
`third_party/pyrefly-1.3.1.patch`: 6 files, +22/−11. The changes are visibility, a
`write_files` switch whose default keeps upstream behaviour, and a `pysa_reporter()`
borrow accessor. It changes no logic.

| # | Outcome | Command | Observation |
|---|---|---|---|
| S0 | passed (over the estimate) | `git -C vendor/pyrefly show --stat HEAD` | 33 changed lines against the ~15 estimated. Two additions were found while probing: `get_exports_data` must be `pub`, and `Transaction::pysa_reporter()` is needed because modules solved lazily during extraction still need the reporter installed |
| S1 | passed | `cargo build -p spike-pyrefly`; `uv run python scripts/check_family.py Cargo.lock`; `cargo deny check bans sources licenses` | Builds on stable 1.98.1. Each family crate resolves to one version, including ruff 0.0.11, pyrefly 1.3.1 and blake3 1.8.6. The allow-git list needs `yangdanny97/lsp-types`. The license list needs additions. From pyrefly: 0BSD, ISC, Unicode-DFS-2016, BSL-1.0. From delta-rs as a normal dependency, which is not part of this pivot: BSD-3-Clause, bzip2-1.0.6, MPL-2.0, CDLA-Permissive-2.0, OpenSSL. Clean dev build ~2 min wall; target 3.8 GB |
| S2 | passed | `strace -f -e trace=execve …`; runs a/d | The only execve is the binary itself, so there is no interpreter probe. Output is byte-identical with `PATH=/nonexistent`, a bogus `VIRTUAL_ENV`, `PYTHONPATH`, `CONDA_PREFIX` and `cwd=/tmp`. The run exits 3 when `PYREFLY_STACK_SIZE` or `PYSA_DUMP_*` is set. The context digest (serialized `ConfigFile` plus resolved search path, site path and sys info) is stable |
| S3 | passed | runs a, b, c (`--shuffle`), e (`--threads 4`) | The five tables (`declarations` 2,712, `call_syntax` 13,292, `pysa_calls` 34,204, `parameter_semantics` 10,129, `public_names` 1,196 rows) are byte-identical Arrow IPC in every run. The 1-thread contract stays, because upstream documents order-dependent types in import cycles |
| S4 | passed | `python3 scripts/parity.py tree venv/… runs/fastmcp-a runs/fastmcp-cli` | Call graphs: 257/257 project modules byte-identical to the CLI's `--report-pysa-format json`, with `module_id` stripped. Definitions: 257/257 equal as sets; 33 differ only in the order of two set-valued lists (`captured_variables`, and union `class_names`), so the extractor sorts them. Public names: the pairs flatten exactly to `compute_public_fqns`, asserted in every run, and all 2,790 CLI `coverage report --public-only` symbols are explained |
| S5 | passed | summary `s5` | 29,279 Pysa locations convert back to byte ranges with 0 mismatches (`LineIndex::offset`, UTF-8). The join key is the **full call-expression range**: 13,104/13,292 calls match on it and 0 match on the callee range. The 188 unmatched calls are all inside annotations (`Annotated[…, cyclopts.Parameter(…)]`), which Pysa's call model does not cover, so they become boundary rows |
| S6 | passed, with one correction | `cargo test -p spike-pyrefly --test s6_delta -- --nocapture` | `CreateBuilder` **rejects** `delta.constraints.*` keys unless unknown keys are allowed, so constraints go through `add_constraint()` after create. CHECK is enforced on `write(batches)` and on `write(vec![]).with_input_plan(plan)`. `appendOnly` rejects delete. `commitInfo` metadata can be read back through `history()`. **DataFusion `INSERT INTO` committed a CHECK-violating row** (bypass confirmed). Ids read back as `BinaryView`; a direct cast to FSB(16) is unsupported, and the two-step cast works |
| S7 | Measured | release build, run a | FastMCP 4.0.3 at `Everything`, 1 thread: run 2.5 s + extract 2.8 s = 5.4 s cold, 4.2 s warm, 3.1 s at 4 threads. Peak RSS ~918 MB |

**Addendum (review F8), 2026-09-22.** `--threads 0` selects `ThreadCount::Inline`, with everything
on the driver's own 512 MiB thread. Runs f (plain) and g (`--shuffle`, cwd=/tmp) produced tables
and a context digest byte-identical to run a. They took 4.05 s with peak RSS ~765 MB. FastMCP
cannot discriminate whether one thread is needed; an import-cycle fixture must.

**Addendum (fork published), 2026-09-22.** `github.com/paul-heyse/pyrefly` has branch `lctx/1.3.1`
at `b9f28575`, and the fork also carries tag `1.3.1` (`3e3177d0`). A fresh clone's
`git format-patch -1` matches the committed patch's sha256 (`b7b82828…b767`). The spike now depends
on the fork by `git`/`rev`:
- `check_family.py` and `cargo deny` pass, with the fork added to `allow-git`;
- the release build took 37 s with dependencies cached;
- run h (`Inline`) is byte-identical to run a.

## ADR-0009 probe, remaining parts (2026-09-22)

`cargo test -p spike-pyrefly --test p_publication -- --nocapture`: passed, 4/4.

| # | Outcome | Observation |
|---|---|---|
| P1 | passed (with a finding) | Delta `add.stats` carry min/max for `fact_key` and `label` but **not for the Binary `snapshot_id`**, so Delta skips no files: all 3 files are opened. The Parquet footers do carry binary statistics. `DataSourceExec` pruned row groups 3 → 1 (`files_ranges_pruned_statistics 3→3`, `row_groups_pruned_statistics 3→1`, 57 bytes scanned). Filtering on `snapshot_id` therefore costs one footer read per file, not a full data scan |
| P2 | passed | Attempt A wrote rows with a duplicate key; the `GROUP BY … HAVING` validator caught it and there was no `snapshots` append. Attempt B published. B's reader (row set → pinned version → `snapshot_id` filter) sees 3 rows, all B's, although A's rows are physically in the version |
| P3 | passed | An append that committed but whose result was discarded is classified `published` by re-reading `snapshots`. An attempt that never appended is classified `unpublished` |
| P4 | passed | A bundle built from the published snapshot at its recorded versions (sorted, concatenated, cast back to the declared schema, Arrow IPC file) is byte-identical when rebuilt after later appends and another publication (1,386 bytes, blake3 `1f353367…`) |

## ADR-0010 spikes (2026-09-22)

The model is **Qwen/Qwen3-Embedding-8B** (operator decision, replacing 4B), revision
`1d8ad4ca9b3dd8059ad90a75d4983776a23d44af`. It has 4096 dimensions, bf16 weights (15 GB),
last-token pooling, and a sentence-transformers `2_Normalize` step.

| # | Outcome | Command | Observation |
|---|---|---|---|
| E1 | passed | `HF_HUB_OFFLINE=1 .venv/bin/vllm serve Qwen/Qwen3-Embedding-8B --revision 1d8ad4ca… --runner pooling --served-model-name Qwen/Qwen3-Embedding-8B --port 8011 --max-model-len 8192 --gpu-memory-utilization 0.80` | vLLM 0.30.0 on the RTX 5090. The resolved pooling config is `LAST` with `use_activation=True`, taken from sentence-transformers. Weights plus non-torch memory is 15.5 GiB, KV cache 8.2 GiB, ~26.9 GB of the GPU in total. Engine start takes 85 s (17 s compile). Every returned vector has norm 1 ± 1e-7, **so vLLM normalizes**. This contradicts Context7's "not L2-normalized by default" for this model and version |
| E2 | passed, with a tolerance | `analysis/embed/py_client.py` (httpx) and `target/release/embed_client` (reqwest 0.12.28, already locked); `compare.py` | Both clients produce **byte-identical request texts** (the spec's query template) and apply the same response checks. **vLLM is not bitwise deterministic across requests**, even for identical inputs. Max component difference: 3.8e-3 with prefix caching; 1.6e-3 for the same batch sent twice without it; 3.1e-3 for a single input vs the full batch (cosine 0.999877). Rust vs Python agree to cosine ≥ 0.99991, or exactly when the requests happen to match. The conformance oracle is therefore: identical texts, plus cosine ≥ 0.9995 per vector |
| E3 | passed | `uv run --with pyarrow --with pytest python -m pytest analysis/mcp` (8 passed; pyarrow 25.0.1, FastMCP 4.0.5) | `Client(mcp, mode="auto")` negotiated `2026-07-28` and `mode="legacy"` `2025-11-25`. Both round trips return `structured_content`, with object output schemas and the read-only/idempotent/closed-world annotations. Exact-symbol promotion and the degraded lexical-only mode are reported. An unknown library or capability raises `ToolError`. A **schema-digest mismatch** and an **embedding-spec mismatch** both fail at `Client` connect (the lifespan raises) |
| E4 | Measured (sanity only) | `compare.py` | Nearest document by cosine: "register a python function as an MCP tool" → the `FastMCP.tool` brief (0.90); "run code around every request" → the middleware brief (0.67) |
