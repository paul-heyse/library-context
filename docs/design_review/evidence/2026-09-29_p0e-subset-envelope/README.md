# P0-E stage-bound subset envelope (fastmcp 4.0.5)

**Label:** Measured, 2026-09-29. **Consumer:** plan §4.1.1 E1 and the input-validation F02
residual (resource review F02/F04/F08). This is not a facts pilot: the subset schedule
(capture and syntax only) cannot be admitted to the facts frontier.

## What ran

```bash
LCTX_P0E_INPUT=$PWD/build/envs/fastmcp/lib/python3.14/site-packages/fastmcp \
  python3 scripts/build_environment.py -- cargo test --release -p cpg-extract \
  --test typed_conformance -- --ignored typed_subset_envelope --nocapture
```

- **Input:** the `.py`/`.pyi` files of the pinned `fastmcp` 4.0.5 package in its uv
  environment: 257 files, 2,975,089 bytes.
- **Path:** capture, then `typed_syntax::extract`, then the `capture` and `syntax` stages through
  `StageOutput`, into a stage-bound `MemoryGeneration` validated with every model invariant.
- **Setup:**
  - pinned Pyrefly fork with Ruff 0.0.11, `Require::Everything`, one inline thread;
  - default `SyntaxLimits`;
  - a 16 GiB fixed budget;
  - release profile on an AMD Ryzen 9 9950X3D (32 threads), Linux 7.0.0-34;
  - the E1 slice's working tree on top of `c1b9344`.

## Results

| Quantity | Value |
|---|---|
| Occurrences / identifier observations | 243,840 / 93,582 |
| Syntax coverage | 257 Partial (identifier subset), 0 Unavailable |
| Peak budget reservation (`ResourceBudget::peak`) | 209,021,869 bytes |
| Peak RSS (`VmHWM`) | 534,628–535,000 kB over two runs |
| Capture | 6–34 ms |
| Extraction (Pyrefly transaction and emission) | 9.6–9.7 s |
| Both stages into memory | about 0.03 s |
| In-memory validation | about 18.4 s |
| Content digest | identical across both runs |
| Budget at half the measured peak (104,510,934 bytes) | refused in extraction: `syntax_supports memory reservation refused: requested 5619712 bytes with 101723892/104510934 reserved`; reserved after the refusal: 0 |

## Reading

- **Uncounted memory.** Peak RSS exceeds the budget peak by about 326 MB. This is the named
  uncounted term: Pyrefly's internal parse and solver state under `Require::Everything` for all
  257 modules at once, allocator retention, and the test process itself. The emitter charges only
  the rows it holds. The pre-parse admission of each source (`admit` before handles) bounds what
  Pyrefly is given, not what it retains (resource review F04). Charged-map undercount (F08) is
  within this term and is not separated here.
- **Exhaustion is clean.** A budget below the peak refuses with a typed `Resource` error during
  extraction and returns every reservation (review focus: budget exhaustion on a large input).
- **Time.** In-memory validation (18 s) dominates the stage path. Performance is not a design
  gate; P1/P2 measurements at phase exit decide any tuning.

## Not measured here

- PostgreSQL COPY and store validation for this input: the PG18 path is exercised on the fixture
  by `the_subset_publishes_a_conformance_generation_equal_to_memory`.
- The behavioral profile and the other P2 providers.
- SQLx buffer retention (resource review F06).
