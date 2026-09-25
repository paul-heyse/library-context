# Evidence

Probes, spikes and investigations that informed a decision, kept so they are not recreated and so
a later reader can see what a decision rested on. Evidence, never authority: the architectural
collection and the ADRs decide (binding §4).

**Layout.** One folder per investigation, `YYYY-MM-DD_<topic>/`, with:
- `README.md`: the question, how it was run (commands, versions, machine), the result, and which
  review, ADR or plan item consumes it;
- the probe's sources (small text, tracked normally);
- `raw/` for raw tool output (Git LFS, with PDFs, archives, Arrow and Parquet; `.gitattributes`).

Never commit virtualenvs, `target/` directories, stores or caches (`.gitignore` here); record how
to rebuild them instead. Keep a probe runnable where that is cheap; otherwise keep its source and
its recorded output.

## Index

A folder stays while a current decision, open finding, test or build consumes it; otherwise it is
removed and recovered from Git (ADR-0042, [historical recovery](../../README.md#historical-recovery)).

| Folder | Question | Current consumer |
|---|---|---|
| [`2026-09-24_bdd-pilot-survey`](2026-09-24_bdd-pilot-survey/README.md) | Can the published pilot's conditions convert to BDDs, and at what size? | ADR-0024 (proposed kernel allowance) |
| [`2026-09-24_test-leaf-proof-joins`](2026-09-24_test-leaf-proof-joins/README.md) | Do test leaves and entry values join to proof rows by source identity? | ADR-0024 |
| [`2026-09-24_entry-value-bridge`](2026-09-24_entry-value-bridge/README.md) | When is an entry formal stable up to a guard use? | ADR-0025 (proposed native executor) |
| [`2026-09-24_rust-build-performance`](2026-09-24_rust-build-performance/README.md) | Which development build configuration reuses cached work? | ADR-0026 |
| [`2026-09-24_typing_cast_model_oracle`](2026-09-24_typing_cast_model_oracle/README.md) | Does the `typing.cast` identity model agree with CPython? | Behavioral analysis §9.9 |
| [`2026-09-25_typing_assert_type_model_oracle`](2026-09-25_typing_assert_type_model_oracle/README.md) | Does the `typing.assert_type` identity model agree with CPython? | Behavioral analysis §9.9 |
| [`2026-09-25_pysa-tito-rule`](2026-09-25_pysa-tito-rule/README.md) | Does a real Pysa source→sink rule give an independent TITO control? | `cpg-core` test `pysa_tito_control_uses_the_same_source_as_finite_summary_fixture`; validation §8.1 |
| [`2026-09-24_z3-5-migration`](2026-09-24_z3-5-migration/README.md) | Can the vLLM service's tilelang build against Z3 5? | `services/vllm` (its tilelang wheel is referenced from here) |
| [`2026-09-25_bdd-decision-apis`](2026-09-25_bdd-decision-apis/README.md) | Which biodivine decision APIs decide what the kernel refuses? | Forward plan W6/W11 (reasoning review F01–F03) |
| [`2026-09-25_library-fit-followup-kernel`](2026-09-25_library-fit-followup-kernel/README.md) | Actual-kernel decision refusal, support history, catalog hydration | Forward plan W6 |
| [`2026-09-25_library-fit-followup-flow`](2026-09-25_library-fit-followup-flow/README.md) | Cycle-head reach and qualified/aliased builtin access | Forward plan W4, W7 |
| [`2026-09-25_library-fit-followup-serving`](2026-09-25_library-fit-followup-serving/README.md) | Facet verdicts and claim citations in served projections | Forward plan W2, W3 |
