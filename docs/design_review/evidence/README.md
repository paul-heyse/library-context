# Evidence

Probes, spikes and investigations that informed a decision, kept so they are not recreated and so
a later reader can see what a decision rested on. Evidence, never authority: DESIGN.md and the ADRs
decide (binding §4). Folder created by the operator, 2026-09-24.

**Layout.** One folder per investigation, `YYYY-MM-DD_<topic>/`, with:
- `README.md`: the question, how it was run (commands, versions, machine), the result, and which
  review, ADR, plan or deviation cites it;
- the probe's sources (small text, tracked normally);
- `raw/` for raw tool output (Git LFS, with PDFs, archives, Arrow and Parquet; `.gitattributes`).

Never commit virtualenvs, `target/` directories, stores or caches (`.gitignore` here); record how
to rebuild them instead. Keep a probe runnable where that is cheap; otherwise keep its source and
its recorded output.

## Index

| Folder | Question | Cited by |
|---|---|---|
| [`2026-09-24_translator-atom-identity`](2026-09-24_translator-atom-identity/) | Does the ty → condition translation drop feasible paths? (P1 translator, P2 CPython `sys.monitoring`) | `design_review_external-review-assessment_2026-09-24.md` (X1–X3, X6); forward plan Stage 2.9 |
| [`2026-09-24_bdd-biodivine`](2026-09-24_bdd-biodivine/) | Is biodivine-lib-bdd 0.6.3 fit as the condition kernel? (P3) | The assessment (X4, E12); forward plan Stage 3.0; decision D-11 |
| [`2026-09-24_crosshair`](2026-09-24_crosshair/) | Does CrossHair 0.0.110 run on 3.14 and find operator differences? (P4) | The assessment (E8, E22); forward plan Stage 3.1 |
| [`2026-09-24_pysa-spike`](2026-09-24_pysa-spike/) | Does Pysa (pyre-check 0.10.0, Pyrefly) run offline on FastMCP 4.0.5, and at what cost? | Deviation B21; forward plan Stage 3.5 |
| [`2026-09-24_stage3-design-notes`](2026-09-24_stage3-design-notes/) | Working notes on resolving `call_transfer` by summaries | Forward plan Stage 3.3 (superseded by it where they differ) |
| [`2026-09-24_fastmcp-channel-prevalence`](2026-09-24_fastmcp-channel-prevalence/) | How common is each behavior channel in FastMCP 4.0.5? | The pivot plan's §2 baseline; the behavioral-model pivot review |
| [`earlier-probes`](earlier-probes/) | Spikes from 2026-09-22 to 2026-09-24, sources only | See `earlier-probes/README.md` |
| [`research`](research/) | Papers and documentation pages read for the behavioral-model pivot review | `design_review_behavioral-model-pivot_2026-09-24.md` (Appendix C) |
