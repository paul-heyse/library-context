# Catalog completion efficiency review evidence

**Examined 2026-10-15.** Supporting material for the
[principal target review](../../reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md).
The review owns the integrated judgment. This folder contains bounded evidence, not another
finding-disposition or implementation ledger.

## Baseline, method and ownership

Production source matched `24b0d08b6f1ed39a43b677ebc684806e07563aaf`;
concurrent changes comprised the evidence index, operation-diagnostics research and the supplied
`docs/surrealdb_technical_reference.md`. Those changes were preserved. This review changes no
production code, dependency, schema, timeout, resource budget, service or operator selection.

The coordinator inspected current owners, source and historical captures. A read-only mapper
investigated compiler/provider/analysis preparation; a library researcher inspected native
realization and exact library source; a fresh design reviewer independently inspected decisive
source and reconciled the combined architecture. Core/template 3.3, efficiency heuristics 1.0 and
code-intelligence 1.5 govern the judgment. Current rules and earlier reviews are design facts and
leads, not acceptance criteria.

Evidence mode: source and existing results first, with selective revealing probes. The only new
behavioral probe substitutes process liveness and signalling to exercise the actual delegated-stop
branch. It does not launch a process group, query a database or establish real host/native drainage.
No new native campaign or real-library journey was needed to establish the structural findings.

| Supporting file | Boundary |
|---|---|
| [Library capability inquiry](library-capabilities.md) | Exact native, SDK/server and SurrealKit source, primary documentation, suitable alternatives and contrary evidence; no new runtime qualification |
| [Delegated-stop probe](delegated-stop-probe.py) | Actual stop-loop selection with temporary invocation-owned receipts and substituted liveness/signals; no real signalling |
| [Phase observations](phase-observations.py) | Streaming extraction from two retained historical captures, with content digests and line references; no performance attribution or new product execution |

## Source inquiry coverage

The scope follows Catalog-completion responsibilities and adjacent consumers, including previously
implemented packages. It is not a claim of line-by-line examination of every dependency or of
whole-product qualification.

| Responsibility | Decisive routes and qualifications |
|---|---|
| Providers, capture and scheduling | `cpg-core/{facts,compilation}.rs`, `cpg-extract/{bundle,capture,pyrefly_stage,ty_flow}.rs`, `cpg-flow/src/lib.rs`: exact ready scheduling and within-phase overlap; full capture checks protect writable input; phase barriers need their epoch/admission premises assessed before removal |
| Normalization, graph and analysis | `cpg-core/{semantic_execution,base_scope,semantic_summaries,catalog_core,catalog_evidence,analysis_graphs}.rs`, model `execution/{evaluation,summary_production}.rs`, `catalog_scope_program.rs`, analytics kernels: root/cohort preparation and correspondence; preserve contextual graph meaning and separate attempted-pair/proof obligations |
| Compiler composition and reuse | `workspace.rs`, compiler-store `compiler.rs`, `compiler/production.rs`, `product_cache.rs`: eager union and private handoff; binary ingredient reuse, scoped exact aliases and backing ownership qualify broad claims of repeated native reconstruction |
| Admission, transport and publication | compiler-store `admission.rs`, native `installed_view/{audit,receiving}.rs`, `completed_state.rs`, `completed_transport.rs`, publisher `native_publication.rs` and `backup.rs`: independent receiver, exact physical copies, original bytes and complete terminals remain substantive |
| Native control and recovery | native `control/{authorization,coordination,retry,retirement,live_view,completed}.rs`, `schema.surql`, `reader.rs`, vendored SDK/RPC and exact server: transaction-local work, conflict domains, finite ingress and recovery admission |
| Development loop | `scripts/{runs,run_listing,run_delegation,harness,surrealdb_fixture,catalog_completion_validate}.py`, `verify_evidence/{capture,context,nextest}.py`, development adapter and build owners: known-ID versus history demand, stop-state composition, capture cost and beta containment |

The fresh reviewer settles which candidates are findings and which remain investigations. Examples
not promoted without further premises include broad phase fusion, persisted selector factorization,
all-pairs analytic replacement, immutable capture ownership and Catalog cross-window reuse.

## Historical capture observations

These are observations from earlier commands, not fresh tests or a controlled comparison. The
captured source-to-executable closure remains unknown; post-command executable hashes do not
establish which complete source closure executed. Current-source findings are independently based
on the pinned source above.

| Run / native invocation | Captured bytes / SHA256 |
|---|---|
| `20261015T181356.562Z-8a3347` / `80231803896d4ef6a3fe53fc0bc76a37` | 56,395,382 / `ac529f913484cf68e2b8da56c3505963b0bc7128d2b72c8a8c4a5258a7d42a8c` |
| `20261015T181515.206Z-7c15e3` / `d6b4959571144b13870c492fda4d1477` | 25,911,722 / `5efa077ec0a18192e265ffb88b190ba0fabf1e043d2c75400d30bb449494b2b0` |

Each native log is at `build/runs/RUN/verify-evidence/INVOCATION/native.log`. Run receipts,
invocation records, JUnit and native recordings retain separate command/capture/cleanup facts.
The extraction found no literal truncation notice in these two logs. This is not a general
full-fidelity guarantee: the adapter reports per-output truncation unknown and a 256MiB producer
maximum; abrupt termination and missing observations remain possible. The earlier finite replay
`bd6eb5` explicitly truncated the Base interval and cannot supply its missing duration.

**Recursive run.** `evaluate_base` reports passed at line 62802, elapsed 322,938.343629ms; the
capture contains 148 `base_selected_grain` terminal events,15,188 `base_selected_input_stream`
terminals,4,055 `completed_input_provider` terminals and 677 nominal-pair batch terminals.
These are phase events, **not native RPC counts**. Empty-selection shortcuts and shared preparation
remain relevant. `analyze_summaries` reports failed at line 71803, elapsed 83,786.007915ms;
its final native acknowledgement intervals include 30,755.138443ms and 25,861.756405ms.
The panic reports `native transaction retry cancelled`. Neither these observations nor the timeout
establish SCC iteration as the dominant cause. Nested/concurrent intervals must not be added.

**Document/lexical run.** Both genuine Catalog compilations report passed, at lines 21357 and 42765,
elapsed 259,748.678710ms and 264,876.670754ms. Their subsequent admission intervals fail after
100,146.486953ms and 95,052.973607ms, before mutation assertions. There are 8,678 captured provider
terminal events across both executions. The fixture is the genuine four-line annotated
`discover` function in `publication_expectations.rs:117`, with optional analytics disabled.
It has no optional payload cache; producer-wide retention is a different operation. These are not
FastMCP or Behavioral-runtime measurements and do not identify a scalar-cache cause.

**Native conflicts.** The recursive capture contains 33 intent,104 payload and 1 recovery-reference
release conflict-log observations; maximum observed retry counts are 4/8/1. The document/lexical
capture contains 156/591/47 such observations, with maxima 4/64/2. Logging is at powers of two:
these are not total retry counts or unique operations. They establish that conflicts occurred,
including one payload reaching at least 64 retries; they do not identify the exact conflicting key,
measure a causal share, or establish indefinite recovery in either run. Shared conflict addresses
and unbounded recovery re-admission are separately source-established.

**Cancellation receipts.** Recursive cancellation was requested 18:26:36UTC; the enclosing run ended
18:29:10UTC, exit 100, with cancellation and a Nextest 900s timeout both recorded. Direct/delegated
host cleanup ultimately confirmed; native finality remained Unknown. The sorted delegation entries
include completed setup registration `54b792d02bb04c5fb967ebe1be538a7f` before active writer/command
registrations `9f123c3ee42a4df883ed6388576f3fc6`, `a 52fff1cdb2a4a309f52f6d8256b8efa` and
`a 6742adefe534722b7b9b88a34159297`, all owned by the same exact process identity (PID3454987,
start_ticks24953310, boot and namespace retained in the original receipts). Recovery requests for
these entries were persisted only at 18:29:10. This matches the branch defect below; it does not
attribute all native execution or host scheduling delay to it.

## New branch control

Command, **passed 2026-10-15, exit 0**:

```bash
uv run --no-sync python docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/delegated-stop-probe.py
```

The actual `run_delegation.request_stop` sees one completed and one active registration for one
owner, under three polls at monotonic 100/110/120 with grace 5. With completed-first ordering it
requests no signal and persists no live request. With live-first ordering it requests SIGTERM,
then SIGKILL twice, and persists the original request. Signals and liveness are substituted;
the branch outcome is Tested, real process drainage **not_run**. Temporary receipts are deleted
by their invocation-owned `TemporaryDirectory`. No production fix was made.

Historical extraction command, **passed 2026-10-15, exit 0**:

```bash
uv run --no-sync python docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/phase-observations.py
```

It emits the hashes, terminal counts, top observed intervals and original line numbers. It does not
change retained evidence or translate intervals into causal percentages.

## Durable citations and release

Both reports were inspected and retained on2026-10-15, exit 0:

```bash
just verify report 20261015T181356.562Z-8a3347 --json --consumer citation:catalog-completion-efficiency
just verify report 20261015T181515.206Z-7c15e3 --json --consumer citation:catalog-completion-efficiency
```

The new citation consumer holds raw evidence and receipt components:

| Run | Storage component IDs |
|---|---|
| Recursive | `f53ddfc7-1328-4d10-a 449-e764325905cb`, `509927ae-c2ec-47de-85d5-e54020a49ce1` |
| Document/lexical | `7a3e669b-e161-4d67-bc57-a 038d46bb37c`, `4c818e09-84b7-4e87-8d84-9f32749ffcd0` |

Keep this consumer while the principal review or a successor needs the cited evidence. After its
last consumer has transferred or ended, release each exact ID through
`just storage release ID --consumer citation:catalog-completion-efficiency --reason TEXT`.
Release is not deletion authority and does not alter original failed/Unknown outcomes or other
consumers. No campaign was created or disposed by this review.

The concurrent operation-diagnostics folders have their own owners. Their exact patched-binary
memory-engine observations may inform diagnostic choices; they do not establish RocksDB behavior,
SDK runtime contracts or measured instrumentation overhead here.

The concurrent principal operation-diagnostics review appeared during this inquiry. The fresh
reviewer independently inspected the actual runtime adapter, projection, reporting and Phase source.
Principal I08 records the narrower coverage questions and disagreements: flat attribution exists;
supplemental duplicate observations are disclosed; captured dispatcher ownership is real. Known
truncation must be qualified to the consumed channel. No automatic adoption or rewriting of the
other review occurred, and no extra blanket gate failures were inferred from its opinions.

## Publication checks

**Passed 2026-10-15:** both evidence scripts above, a local Markdown destination-path check of
the principal and this folder's two Markdown files (no missing paths), and scoped Ruff:

```bash
uv run --no-sync ruff check --no-force-exclude docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/delegated-stop-probe.py docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/phase-observations.py
```

The first Ruff check identified standalone-import ordering and long lines; the probe documents
its necessary path setup, the lines were split, and the final check passed. Explicit
`ruff format --no-force-exclude` reported both files unchanged; documentation paths are otherwise
excluded by the repository's Ruff configuration.

**Failed 2026-10-15, exit 1:** `just docs-check` stops at
`missing title/H1: docs/surrealdb_technical_reference.md`, a supplied concurrent document outside
this review's ownership. That file was preserved. Its ADR and agent lint stages passed; complete
publication/link validation did not finish. The local destination check is narrower and does not
replace it. Product builds, native tests and assembled qualification remain **not_run** for this review.
