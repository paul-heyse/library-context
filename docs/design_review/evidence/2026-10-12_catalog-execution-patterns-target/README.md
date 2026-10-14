# Catalog execution and recurring patterns: review evidence

**Recorded 2026-10-12 local / 2026-10-13 UTC.** This folder supports the
[principal design/target review](../../reviews/design_review_catalog-execution-patterns-target_2026-10-12.md).
It is evidence, not another disposition or progress register. The
[Catalog execution coordinator §6](../../../plans/catalog-creation-plan_2026-10-14.md#6-sole-disposition-and-execution-checkpoint)
owns scheduled findings.

## Baseline and method

HEAD was `5090aeec2ce60168a41559c6cd38badb6dde5d0f`, with the preceding session's uncommitted
native/compiler/control/test corrections. [review-snapshot.json](review-snapshot.json) records
SHA256 hashes of those source files and decisive unchanged primitives, exact retained run
commands, receipt/log object IDs, hashes and focused phase summaries. References in the review
use this identified tree, not an assumed clean HEAD. The prior completion review and dirty plan,
architecture and STATUS changes were present; they were not this review's implementation.

The coordinator inspected causal boundaries and reconciled three read-only assignments:
a fresh independent design reviewer of completion and physical effects; a library researcher of
existing local constructs and pinned capabilities; and a second design reviewer of analogous
patterns. All inspected decisive source. No worker changed files, created a worktree, launched
a service or ran a product control. The coordinator alone wrote the principal and handoff.

Source/receipt inspection settled the structural questions. No new probe, build, full Catalog
journey, embedding execution, MCP query, benchmark or repository qualification was needed or run.
The existing operational observations below are historical evidence from earlier in this session,
not new review test passes. Shared/operator targets and old paused campaigns were untouched.

Exact inspected library pins: DataFusion55.1.0, Arrow59.3.0, BLAKE3 1.8.6; native SDK/RPC
are patched local SurrealDB3.3.0 packages. SurrealKit1.0.0-beta.1 is development tooling,
not the product SDK. The selected offline DataFusion and neo4j-surrealdb skills were discovery
routes; the latter's upstream surface does not certify the local patch. Context7 resolved official
DataFusion and SurrealDB documentation; applicability was checked against locked/local source.

## Existing observations and their limits

`20261013T004735.484Z-76f1cf` ran the exact
`current_catalog_analysis_links_public_native_operations_and_original_evidence` case under
`just fixture -- cargo nextest run --release --locked`, with immediate captured output and
normal concurrency. The exact command is in the snapshot. It failed at the existing360s
harness allowance; Nextest reported368.325s including cooperative ending. Root command exit100;
local child cleanup confirmed/no survivors. Native uncertainty was separately retained in
attachment `a69d4875-d845-402b-82c5-3a4132dbb74f` and later settled only through checked
whole-campaign disposal. Disposal does not change the test's failed outcome.

The trace contains403 successful `live_view_logical_roots` spans, and403 each for closure,
membership and alias inspection. Late logical-root spans were about1.78–2.22s in this invocation.
Provider execution and provider completion are different spans: the full providers span was
243.828s; normalization of entities passed after73.118s; normalization of relations failed after
42.501s. These spans nest and overlap other work; their durations must not be summed into a
pipeline attribution. The403 logical-root spans sum to about59.211s; this does not account for
the complete elapsed time or establish logical hashing as the dominant cause.

The final installed batch reported seven contributor observations, one blob observation, seven
logical contributor preparations, one alias preparation and40 requested view references. These
counters measure preparation misses inside that batch. They do not count subsequent root-level
replay, sorting, encoding, hashing, member materialization or effects. Requested references can
deduplicate; a cancelled batch need not finish every reference.403 spans are not a unique-view
inventory.

There are48 **logged conflict events**, not48 total retries: retry logging is selective.
Logged conflicts occur at intent, payload and outcome-reference release, with successful later
commits. The final failed requests say `cancelled_before_issue`, retries0; pin-release failure
then adds a finalization consequence. Neither observation proves an indefinite native deadlock,
a server defect or a single failed transaction consuming the entire allowance.

Acquisition, deployment, Pyrefly, documents and assembly completions passed; normalization began.
Analysis, semantic admission, publication and linked native query assertions were **not_run** in
this failed invocation. Static inspection of those downstream paths is not runtime qualification.
The concurrent facts control below prevents treating this run as an isolated performance trial.

| Retained run | Outcome and exact boundary |
|---|---|
| `20261013T004857.554Z-e28d61` | Facts availability control failed at360s before semantic availability assertions; exit100/local cleanup confirmed; independent native Unknown preserved before campaign disposal. |
| `20261013T004135.077Z-383cbb` | Selected late-membership and unchanged64MiB receiver bodies passed; cold compiler negative control failed; enclosing command failed/Unknown. No whole-run pass. |
| `20261013T004845.788Z-a53c24` | Corrected cold compiler exact-case control passed, exit0 and enclosing fixture cleanup confirmed. |
| `20261013T005654.382Z-0148c9` | Functional campaign checked-disposal passed; failed logical drain remains a separate outcome. |
| `20261013T005805.779Z-df05d6` | Scalar postcheck passed exact private retirement, preserved original native records and unchanged qualified server. |

The prior16MiB facts fixture failure remains failed. Its retirement followed a static incompatibility
with two retained170-view profiles and independent expected-record reservation floors; the replacement
full-compilation owner does not establish lower-memory capacity or improve the timeout evidence.

## Review-owned cleanup

The unused, newly installed libtest campaign was not used for a review probe or functional rerun.
Existing authenticated disposal was invoked only for installation
`84dc2704-be4e-457e-a9e2-4ecafedd7b16`, generation
`6fb86e4c0c4696baae0378a04cce033eb0fabc068febf2222faa72fb97c7573a`:

```text
just run --background --label catalog-review-owned-campaign-disposal -- env \
  LCTX_SURREAL_SERVICE_CONFIG=/home/paul/.local/state/library-context/surrealdb-lifecycle-libtest-2026-10-12/installation.json \
  just service dispose-campaign 84dc2704-be4e-457e-a9e2-4ecafedd7b16
```

Run `20261013T011359.080Z-c6fed4` **passed**, exit0/local cleanup confirmed. Close-admission,
drain, exact executor shutdown, datastore/installer/unit/credential removal and storage release
each passed. No workers were registered. The owned scalar postcheck command is preserved in
`20261013T011453.931Z-d5e7e5`; it **passed**, confirming disposed phase/private absence and
preservation of original native records and the qualified server's SHA256
`710229df33346fdf9733ef86d46e31874f0cacc2691529327e19e6534477b8ed`.
These are cleanup checks, not product acceptance. No auxiliary worktrees existed or were created.

## Retention and reproduction

Every snapshot receipt/log object has the existing storage owner's
`citation:catalog-execution-patterns-target` consumer with `cited-report` capability. Full outputs
remain in `build/runs/<ID>/output.log`; exact commands and storage IDs are in the snapshot.
`just runs status ID` and `just runs logs ID` inspect them without replaying their commands.
No raw log was copied into Git or a second evidence store.

The phase summary was derived read-only from log lines matching
`phase="..." ... status="passed|failed" ... elapsed_ms=...`, grouped by phase, retaining outcome
counts and the last observed line/duration. Batch counters were parsed from
`installed completion batch observations`; conflict events counted the emitted
`native transaction conflict;` lines. Source and receipt/log hashes use SHA256 of file bytes.
The summary records observations only; it does not synthesize missing stages or total query counts.

## Publication checks

**2026-10-12 local:** `just docs-check` **failed** before its publication/link phase, reporting
existing ADR/architecture owner metadata mismatches in ADR-0152/0155–0158/0160/0161 and the
ADR-0151 association. No affected ADR or architectural owner was changed by this review, and
those repository quality items were not repaired. This does not change the source design judgment.
An inline read-only `python3 -` local-target/Markdown-anchor and SHA256 inspection **passed**:
210 local links across the review, evidence index/README, STATUS and lifecycle plans; ten frozen
source hashes; STATUS120 lines. This focused inspection is not a substitute claim that
`just docs-check` passed.

Independent draft checks accepted the combined judgment and preserved each supporting finding.
Their concrete corrections were incorporated: exact native key/JSON-byte merge semantics, row
payload versus spool framing, remaining decode work, contributor-local alias checks versus composed
reachability, optional discovery absence, the actual `Dump::rows` owner and existing fixpoint
termination. No new runtime evidence was inferred from these checks.
