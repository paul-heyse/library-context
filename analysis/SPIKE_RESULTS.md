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
