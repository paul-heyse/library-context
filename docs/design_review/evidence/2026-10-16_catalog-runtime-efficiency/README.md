# Catalog runtime efficiency evidence

**Inspected and derived 2026-10-16; current source baseline 2193bbfc.** Consumer:
[catalog runtime efficiency target review](../../reviews/design_review_catalog-runtime-efficiency-target_2026-10-16.md).
This folder contains small derived indexes over retained historical receipts, not new product runs.
The principal review owns the combined assessment and finding IDs. Existing plans remain unchanged.

## Method and limits

Read the retained CE90 acceptance/repair captures from `20261016T154844` through
`20261016T193502` with the `efficiency-` run label. This covers 34 managed runs and 47 Nextest
invocations, including invocations that never executed tests. Prefer terminal JUnit cases when
available; otherwise use complete libtest JSON event lines. Normalize library target names using
the producer's suite metadata, preserving original locators. Same-invocation native Nextest
`TIMEOUT` summaries supply termination kind when libtest omits it. Compilation, ignored cases,
unstarted tests and enclosing invocation duration do not enter the slow-case count.

The derived inventory contains 3,669 terminal records, 606 over 60 seconds: 449 timed out, 130 failed
and 27 passed. These are historical occurrences, not distinct workloads or current defects.
There are 392 distinct captured case keys. Their latest captured terminal outcomes are 260 timed
out, 114 failed and 18 passed; 10 latest results are now at or below 60 seconds. A later pass or
shorter duration is not a controlled speed comparison and does not resolve enclosing cleanup.
Repeated executions and old binaries remain separate. No source-to-binary closure is inferred
from invocation-scoped source observations.

`slow-inventory.json` retains each occurrence's run/invocation, source locator, duration, outcome,
and review group, plus latest terminal evidence for every ever-slow key. Original files remain
in their managed run owners; no raw captures are copied here. Named citation holds protect their
existing run-log and run-receipt objects. Some interrupted captures lack JUnit; JSON coverage is
only its readable complete-line prefix. Unstarted cases are not classified as passed or slow.

## Complete slow-case routing

Every captured key has the corresponding `review_group` field. These groups locate a production
path and a causal question; they are not 392 independently established diagnoses. The principal
review investigates shared mechanisms deeply and preserves case-specific uncertainties.

| Group | Keys | Paths and retained question |
|---|---:|---|
| T1 | 202 | `cpg-core::fixture_corpus`: fixture capture, actual Facts compilation, validation and drainage; paired layouts repeat two required configurations. Which provider/native/preparation/finality work dominates each retained case? Latest paired-layout evidence stops at Pyrefly completion, not Base/Summary. |
| T2 | 96 | `cpg-extract` acquisition/extraction targets: pinned environment acquisition, independent providers, native ingress/completion, selected observation and drainage. Which prerequisite accounts for each delay? The joined ty oracle's child and later actual provider comparison are distinct operations. |
| T3 | 92 | Other `cpg-core` library/compiler/artifact/admission targets: common compilation, normalization, scope preparation, analyses, admission/export and finality. Named algorithm assertions generally follow these prerequisites. Current source defects do not identify a historical case's dominant cost without phase evidence. |
| T4 | 1 | Native exact key/atomic-field selection: construction, completed-view earning, indexed demand, independent frozen membership and owner drainage. A 66-second pass alone does not show that selection lookup itself is slow. |
| T5 | 1 | Publication selected backup: Catalog construction/admission, publication, selected inventory/export, independent receiving and cleanup. Latest timeout remains in initial Catalog compilation; backup-source findings are independent source judgments. |

## Latest reconstruction phase evidence

`phase-summary.json` derives attributed observations from seven JUnit cases in
`20261016T190337.280Z-029373`, invocation `49584dfdb8244ff7b0d3bb9ec5b81d9f`.
It retains producer context counts, phase begins/terminals, maximum observed terminal elapsed
time, incomplete observations and observed client transaction terminals. It intentionally does
not add overlapping phase durations or infer live server work from an unmatched begin.
Ready operations can emit no wait phase. Flat span fields do not prove complete parentage;
JUnit normalization and unknown capture truncation retain their original limits.

Decisive reachable intervals are Pyrefly provider completion for paired layout; `evaluate_base`
membership/payload capture for both Summary cases; `analysis_native_inventory` output for the
SourceCall/Base controls; and `assess_catalog_frontier` in the backup control. These identify
work reached, not exclusive initiating causes. Late cancellation/finalization events can occur
at or after the timeout boundary. Neither a test name nor its last event establishes a deadlock,
SCC failure, leak or dominant time attribution. Frozen-view BM25 passed 39.958 seconds.

## Reproduce

From the repository root, without any database, build or test invocation:

```sh
uv run --no-sync python docs/design_review/evidence/2026-10-16_catalog-runtime-efficiency/slow_inventory.py
uv run --no-sync python docs/design_review/evidence/2026-10-16_catalog-runtime-efficiency/phase_summary.py
```

The [library evidence](library-evidence.md) records exact source/version qualifications, the
default installation versus disposed campaign correction, and limits on memory attribution.

The scripts write only the two derived JSON files in this folder and require the cited retained
run captures. No service/fixture operation, environment synchronization, regeneration, product
test, heap measurement or performance benchmark was run for this review.

## Publication checks

On 2026-10-16, `just docs-check` passed (467 canonical pages; no link errors).
Scoped `uv run --no-sync ruff check --no-force-exclude` and
`uv run --no-sync pyrefly check --summary=none` passed for both evidence scripts.
Reproduction of both derived indexes passed with byte-identical outputs.
Product compile checks, tests and service operations were **not_run**, as agreed for this review.
