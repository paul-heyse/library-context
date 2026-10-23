# Catalog compile performance: design review and diagnostics plan

18 Oct 2026 · library-context at `b0dd321` (main)

## Bottom line

The catalog compile is slow because of how stages hand data to each other, not because of analysis work. In the only complete Catalog compile trace retained on your machine, about 95% of stage wall time is spent inside native-store calls or the workspace completion gate; analyzers and kernels account for roughly 20 s of 307 s.

The trace is run `20261018T044128.645Z-1d2be6` (the genuine-catalog fixture setup, 18 Oct 2026, Catalog profile and frontier). It wrote only 5,986 logical rows across 289 relations, so its 307 s is almost entirely fixed and control-plane cost:

- **306.7 s compile for 5,986 rows**, about 51 ms per logical row. Time does not track data volume.
- **2,481 guarded effects plus 2,481 recovery settlements**, through 1,785 immutable windows whose median size is one row.
- **117.5 s of transaction time inside optimistic-conflict retry loops.** The worst stall lasted 34 s (98 retries) and froze three producers at once.
- **80.5 s of waits on the completion gate**, a single workspace mutex.
- **150 s of optional product retention**, growing with stage depth from 5.5 s to 37.9 s and sitting between dependent stages.
- **Mean concurrency 1.34** for native writes while any were in flight, on a many-core machine.

**Question 1, design changes.** The order-of-magnitude change is to stop using SurrealDB as the compiler's working memory: keep stage-to-stage data in process as Arrow, verify by construction, and persist once per compile as a coarse, bulk publication. Quick wins (retention off the critical path, conflict-free control rows, no global gate) plausibly take the fixture compile to roughly 120–170 s, and protocol coarsening (packet-level effects, verify once) to 50–80 s; about 30 s for fastmcp needs the pivot.

**Question 2, diagnostics.** Replace begin/end log lines with a span trace that carries a typed wait taxonomy, SurrealDB statement correlation and per-stage counters, plus an analyzer that computes the stage critical path and what-if savings. Most raw material already exists in `lctx_phase` events, retry events and SurrealDB's OTLP support.

One gap: I found no retained log of a real fastmcp compile. The cost structure above carries over; fastmcp adds data-proportional cost on top, about 3 ms per written row on the existing 4,096-row write probe.

## What the existing evidence shows

The retained trace is a compile that mostly waits: 296 s of the 313 s summed stage wall time sits inside native-store calls or the completion gate, and the upper stages run as a near-serial chain.

**Stage timeline** (seconds from the first logged event; source: `build/runs/20261018T044128.645Z-1d2be6` `native.log`, Catalog compile of the genuine-catalog fixture, 18 Oct 2026):

| Stage | Start (s) | End (s) | Duration (s) | Optional product retention after it (s) |
| --- | ---: | ---: | ---: | --- |
| Fact providers (5) | 0.2 | 26.0 | 25.8 | |
| Facts freeze + input setup | 26.0 | 47.5 | 21.5 | |
| Native inventory | 47.5 | 83.8 | 36.3 | |
| 7 unrequested analyses | 48.2 | 75.2 | 27.0 | |
| Normalize entities | 48.4 | 56.5 | 8.1 | 56.5–66.0 (9.5) |
| Analytic text | 59.9 | 65.2 | 5.3 | 65.2–83.8 (18.6) |
| Normalize relations | 64.9 | 86.5 | 21.6 | |
| Normalize 7 more (callables → coverage) | 86.5 | 103.3 | 16.8 | |
| Catalog core | 110.2 | 117.1 | 6.9 | 117.1–122.7 (5.6) |
| Structural | 126.7 | 139.2 | 12.5 | 139.2–149.0 (9.8) |
| Catalog evidence | 132.0 | 151.6 | 19.6 | 151.6–159.2 (7.6) |
| Analytic | 159.9 | 169.4 | 9.5 | |
| Catalog selection | 174.0 | 187.6 | 13.6 | 187.6–200.2 (12.6) |
| Synthesis | 209.4 | 226.6 | 17.2 | 226.6–254.0 (27.4) |
| Retrieval | 259.2 | 275.8 | 16.6 | 275.8–313.8 (38.0) |
| Catalog frontier | 287.2 | 306.7 | 19.5 | |

Conflict stall: 49.1–83.4 s (34 s), three transactions retrying optimistic conflicts (221 retries in total). The retention intervals listed above total 129 s.

Read the table top to bottom as a timeline. Facts finish at 26 s, but upper consumers start only at 47.5 s. From 110 s onward each upper stage waits for the previous one, and each gap is filled by that stage's optional product retention. The 49.1–83.4 s stall is a single livelock in which three producers retried conflicting transactions until they stopped overlapping.

| Mechanism | What the trace shows | Cumulative time (overlapping) |
| --- | --- | --- |
| Optimistic transaction conflicts | 91 retried transactions; the worst took 34.1 s and 98 retries, released together with 18.5 s / 65 and 17.3 s / 58 | 117.5 s |
| Optional product retention | 15 retentions; 5.5 s after catalog core rising to 37.9 s after retrieval | 149.9 s |
| Completion gate | 47 waits on one workspace mutex, median 1.6 s | 80.5 s |
| Effect lifecycle | 2,481 effect applies (median 9.6 ms), each followed by a recovery settlement (median 15.7 ms) | 111.1 s + 52.8 s |
| Input acquisition and checking | 804 completed-input provider spans; 19 reference validations | 43.7 s + 14.7 s |
| Fact provider completion | 5 completions, largely one after another; Pyrefly's took 15.1 s for 309 rows | 26.3 s |
| Fact provider execution | The actual Ruff/ty/Pyrefly work; Pyrefly 3.8 s | 5.5 s |

Phase totals overlap, so they do not add up to wall time; the timeline is what ties them together. Outside native calls and gate waits, all stages together spent 16.9 s.

Writes are small and many. The trace has 1,785 immutable windows with a median of one row, 1,376 of them carrying four rows or fewer, and 244 of 289 relations hold ten rows or fewer. Every window and control write is a guarded effect: an apply transaction plus a recovery-settlement transaction, often after an expectation read.

The per-row cost fastmcp adds shows in the existing write probe (`native_summary_pair_outcome_batch_has_committed_window_receipts`, 17 Oct). Its 4,096 rows became 4,096 payload nodes, 4,096 memberships and 73 immutable effects, with 12.7 s of atomic apply across 84 effects: about 3.1 ms per row with full 128-row windows.

Method: a read-only parse of the retained `native.log` (`lctx_phase` begin/end events, span context and retry events) for nested invocation `b653540a…`; nothing was executed against the pipeline or the service.

## Where the time goes structurally

One design choice drives most of the cost: SurrealDB is the compiler's working memory, and every write into it goes through a fine-grained, journaled, fenced protocol. Eight mechanisms follow from that, and they multiply rather than add.

1. **Persistence is the inter-stage transport.** Producers write through `ProducerOutput` → `NativeIngress` → `WindowWriter`; consumers read back through `scan_rows` → per-row `Value` → `ProjectedBuilder` → Arrow → `Batch::read`. A row is re-encoded about six times per hop: typed record, canonical JSON (`serde_json::to_vec` per row), SurrealDB `Value` trees, plan bindings serialized again for sizing (`plan_bytes`) and digesting (`payload_digest`), gRPC, and the reverse on read.
2. **A transaction protocol around every immutable write.** `fn::native_immutable_install` does a point `SELECT` per row, admission checks per target, predicate and guard revision bumps under `FOR UPDATE`, and an `array::find` over deferred guards inside a `map` (quadratic per window). It runs inside an operation page (era guard, stripe participation, page row) and is followed by a settlement transaction. Each logical row becomes a payload node, a guard and a membership locator, and windows cap at 128 rows (`NATIVE_WINDOW_ROWS`) and split whenever the SQL plan shape changes. The code itself reserves 32× the logical bytes for physical lowering.
3. **Serial ingress per producer.** Each contribution has one ingress driver with a one-slot channel; operation pages apply under a mutex in ordinal order, and provider threads block on the bridge until the previous batch commits. Pyrefly, which writes most of the roughly 246 fact relations, therefore drains through one serial stream, and the five fact-provider completions ran largely one after another.
4. **Hot control rows under optimistic concurrency.** Attempt stripes, `native_predicate` revisions, guards and completed-state parents are shared by concurrent writers. Retries are unbounded, with jittered backoff of up to 250 ms, so overlapping large transactions retry each other: the 34 s, 98-retry stall. Adding parallelism adds conflicts, which is why native-write concurrency sits at 1.34.
5. **Global serialization in the coordinator.** One `completion_gate` mutex covers input binding, prefix freezing and completion handoff; bindings and view compositions run one selection at a time under it, and producer completion prepares one view per output relation sequentially. The coordinator awaits each launch's binding and input checks before launching the next ready stage, and native calls share an 8-permit semaphore.
6. **Verification by re-reading.** Completion re-streams every membership row of a contribution to compute digests it could have computed while writing, and re-checks originals in 32-row pages. Consumers re-validate references and invariants over their inputs; the seven unrequested analysis owners stream inputs to prove exact counts; admission then re-executes all invariants and references over all relations before publication rebuilds references and search. Each pass uses mechanism 1's read path.
7. **Optional retention on the critical path.** After every stage `retain_child` traverses the stage's full transitive dependency closure and installs compound views, so its cost grows with depth (5.5 s to 37.9 s). It shares native-call admission and the store with the next stage's input installation; in the trace every dependent stage starts after its predecessor's retention ends. The exact blocking edge is not established statically: shared root installation gates, the call semaphore and DB contention are all candidates.
8. **Fixed costs times relation count.** The model declares 314 relations (about 246 fact families), and the compile touches about 300 relations across about 40 stages. Every relation pays writer, view declaration, installation, freeze binding, composition, completion inventory and provider setup, even though 244 of 289 written relations held ten rows or fewer.

| Mechanism | Grows with | Weight in the fixture trace | Expected weight for fastmcp |
| --- | --- | --- | --- |
| 1 Transport and re-encoding | rows × hops × readers | moderate | dominant data-proportional cost |
| 2 Write protocol | windows and rows | high (2,481 effects) | high: about 3 ms per row |
| 3 Serial ingress | rows of the largest producer | low (309 Pyrefly rows) | high: Pyrefly writes most facts |
| 4 Conflict livelock | overlap of large transactions | 117.5 s retried | higher: larger windows overlap longer |
| 5 Coordinator serialization | stages × relations | 80.5 s gate waits | similar |
| 6 Re-read verification | rows × verification passes | moderate | high |
| 7 Retention on critical path | stage depth × closure size | 129–150 s | higher: closures carry more rows |
| 8 Per-relation fixed cost | relations × stages | dominant | similar |

## Design changes ranked by expected impact

The largest gain comes from moving persistence off the compile's critical path; the quick wins remove retention and conflict stalls that cost tens of seconds each today. Savings below are estimates against the fixture trace unless marked otherwise; none is measured yet.

| Rank | Change | Mechanisms removed | Expected effect | Effort and risk |
| --- | --- | --- | --- | --- |
| 1 | **In-process stage handoff.** Completed views become immutable Arrow tables owned by the workspace, spillable to local Arrow IPC under the existing budget; consumers read them directly | 1, 3, 4, 5 (DB part), 6 (read-back), 7 | Compile wall becomes analysis compute plus in-memory kernels; the order-of-magnitude change | High: revisits the persisted-execution decision; intra-compile crash recovery becomes rerun |
| 2 | **Bulk, coarse publication.** Persist the admitted graph once: per-relation columnar blocks (extend `compiler_block`) plus only the index records serving queries, staged under the attempt, then one fenced manifest commit | 2, 8 | Write cost from about 3 ms to tens of µs per row; one fencing transaction per compile | Medium-high; contained in the native layer and publisher |
| 3 | **Retention off the critical path.** Skip retention on cold compiles, or retain once from the admitted manifest after publication | 7 | 60–130 s of wall in the trace | Low; a flag-off run is itself the confirming experiment |
| 4 | **Conflict-free write path.** No shared revision counters on immutable writes (pinned manifests already protect readers), per-contribution stripe and predicate rows, bounded jittered retries, `array::find` replaced by an object lookup, per-row `SELECT` replaced by one `SELECT * FROM $ids` | 2, 4 | Removes the 34 s stall class (117.5 s retried) and makes concurrency pay | Medium |
| 5 | **One effect per packet.** Fold the settlement record into the apply transaction (idempotent replay instead of a second write), size windows by bytes rather than 128 rows, pack a producer's small relations together | 2, 8 | 2,481 effects and about 5,000 transactions down to a few hundred | Medium |
| 6 | **Verify by construction, once.** Digest contributions while writing, run each invariant once on the producing stage's in-memory rows and record evidence; admission checks evidence; keep full re-execution only for cold imports and independent receiving | 6 | Removes completion rescans, consumer re-checks and admission's full re-read | Medium; preserves independent receiving |
| 7 | **No global gate in the hot path.** Per-boundary locks, binding and composition outside the gate with compare-and-set, launch preparation spawned rather than awaited by the coordinator | 5 | Up to 80.5 s of gate waits; lets independent stages overlap | Medium |
| 8 | **Pipelined ingress.** Several in-flight windows per producer, provider threads not blocked on commit, Pyrefly output split by family into parallel writers. Only after rank 4, or it adds conflicts | 3 | Large for fastmcp's fact volume; small in the fixture | Low-medium |
| 9 | **Fact-stage fixes.** Complete providers concurrently and derive facts availability from completed descriptors | 3, 6 | About 15–35 s of the 47.5 s before the first upper consumer | Low-medium |

Three areas are not worth early effort. Analyzer execution is 5.5 s, blake3 hashing and budget atomics are cheap, and DataFusion planning never appears among the slow phases. The Python fact-collection review's adapter fixes still matter for fastmcp's fact volume, but they sit behind the persistence costs.

Ranks 3, 4 and 7 can land without changing any ADR-level decision and should be first. Rank 1 is the decision to make deliberately: it trades intra-compile durability for speed, on the argument that a compile is a deterministic function of pinned inputs and is cheaper to rerun than to recover.

## Target architecture and sequencing

About 30 s for fastmcp is reachable only when stage hops stay in memory and the store is written once. The incremental steps get roughly halfway, are cheap, and keep the system shippable, so they come first.

**Stage-to-stage data path, today vs target:**

| Step | Today, for every stage-to-stage hop | Target |
| --- | --- | --- |
| 1 | **Stage A writes typed rows** — `ProducerOutput`, one serial ingress per producer | **Stage A emits Arrow batches** — content digests computed while streaming |
| 2 | **Lowering, about six re-encodings** — canonical JSON, `Value` trees, plan sizing, digest | **Completed view in memory** — immutable Arrow, spilled to local IPC if large |
| 3 | **Guarded native writes** — 128-row windows: apply + settlement transactions; guards, holds, predicate bumps; conflict retries | **Stage B reads the same batches** — no store round trip, no re-encoding |
| 4 | **Completion under the global gate** — rescan members, compose views, freeze bindings | *(after the last stage)* **Admission once** — invariants evaluated once; evidence digests |
| 5 | **Optional retention** — installs the full upstream closure again | **Bulk publication** — columnar blocks plus serving indexes; one fenced manifest commit |
| 6 | **Stage B reads it back** — install and pin view, scan `Value` rows to Arrow; re-validate references and invariants | |
| | Repeated across about 40 stages and 300 relations | Persistence leaves the critical path |

On the left, every hop pays lowering, guarded writes, completion, retention and read-back. On the right, hops are in-memory reads, and the store sees one admitted graph after the last stage.

| Step (fastmcp, estimate) | Target wall | Basis |
| --- | --- | --- |
| Acquisition and capture | 1–3 s | hashing and inventory, unchanged |
| Ruff, ty and Pyrefly facts | 5–10 s | Pyrefly checks a library this size in about a second; fact construction and the adapter fixes from the 17 Oct review dominate |
| Normalization and upper stages in memory | 5–10 s | the fixture trace spent 16.9 s outside native calls across all stages |
| Admission once | 2–5 s | invariants over in-memory Arrow, evaluated once |
| Bulk publication | 5–10 s, partly overlapped | columnar blocks; facts can stream out while upper stages run |
| **Total** | **about 20–35 s** | |

1. **Instrument and baseline** (see the diagnostics section). Trace a real fastmcp compile, cold, once with retention on and once off. Gate: a critical-path report naming the five largest blocking edges.
2. **Quick wins** (ranks 3, 4, 7, 9). Expected: the fixture compile from 307 s to roughly 120–170 s. Gate: retried-transaction time under 5% of wall, and no inter-stage gap over 2 s attributed to retention.
3. **Protocol coarsening** (ranks 5, 6, 8, plus empty relations as metadata only). Expected: roughly 50–80 s. Gate: effects per stage in single digits, one transaction per effect, and at most one store read per produced row outside admission.
4. **The pivot** (ranks 1 and 2). Expected: 20–35 s for fastmcp. Gate: the critical path is analyzer and kernel time, and an interrupted compile is simply rerun.

Invest in the per-window protocol only as far as removing hot rows and bounding retries. Incremental digests, columnar blocks and evaluate-once admission carry straight into the target; deeper tuning of 128-row windows does not.

Rank 1 supersedes an accepted position, that Arrow/DataFusion is bounded compute and transfer rather than completed-input authority, and it narrows the scope of the persisted-execution pivot. It deserves its own ADR: durability, fencing, recovery and independent receiving stay at the publication boundary, where a second reader actually exists.

## The diagnostics and measurement system

Build one trace per compile that answers, for every moment of wall time, what was on the critical path and what it was waiting for. Today's `lctx_phase` events already supplied enough to write the evidence and mechanism sections above, but they cannot show causality, server-side time or volumes, and overlapping phases cannot be summed.

**What exists.** `Phase` begin/finish events with `elapsed_ms`, `phase::wait` on selected pending waits, span context flattened into text lines, retry events (`native transaction conflict` / `terminal`), invocation correlation and `runtime_adapter.py`. **What is missing:** span ids and parentage, who-blocked-whom edges, row/byte/transaction counters, SurrealDB execution time, a CPU-versus-wait split, and an analyzer. The text form also costs: the fixture compile wrote 69,632 `lctx_phase` lines.

| Component | What it records | Question it answers |
| --- | --- | --- |
| Trace recorder | A `tracing` layer enabled by `LCTX_TRACE=<file>`: span open/close with monotonic ns, parent, thread, tokio task, busy time from enter/exit; direct `O_APPEND` writes (the 15 Oct probe showed the non-blocking appender loses lines on SIGTERM); Perfetto/Chrome export | What ran when, nested and on which thread |
| Span taxonomy | Fixed names and fields: compile › stage (producer, kind) › compute, read, write, complete, retain › native op (effect id, query digest, transaction stage) › DB statement | Where a stage's time went, comparably across runs |
| Wait spans with blockers | A closed set: completion gate, root installation gate, native-call permits, ingress channel, bridge acknowledgement, DB in flight, retry backoff, upstream dependency. Instrumented mutexes record the holder's span id | Who blocked whom; the edges of the critical path |
| Schedule events | The static schedule at start; each stage's ready, launched, started and finished times | Scheduler latency (ready to launched) apart from execution |
| Counters | Per stage and relation: rows and bytes in/out, physical records by family (nodes, guards, members, holds, predicates), effects, transactions, statements, retries, gRPC bytes, rows scanned back; a run summary JSON | Amplification: physical writes per logical row, store reads per produced row, transactions per stage |
| Server correlation | Patch the vendored SDK to send W3C `traceparent` gRPC metadata and to surface `QueryStats.execution_duration` (it defaults to zero today); SurrealDB OTLP traces to a local collector; slow log at 50 ms; `/metrics` scraped each second | How each DB span splits into client prep, queue, server execution and commit; conflict and key-write rates |
| Resource sampler | 1 Hz CPU, RSS, IO and context switches for the `lctx` and `surreal` processes, plus disk utilization | Whether the run is CPU-bound, IO-bound or idle-waiting |
| CPU profiles | A `profiling` Cargo profile (release, line-tables debug info, frame pointers); `samply` or `perf --call-graph fp` on both processes; off-CPU sampling when waits dominate; tokio runtime metrics if scheduler latency appears | Which code burns the CPU that is spent |

The analyzer (`just trace analyze RUN`) turns the trace into decisions:

1. **Timeline** in Perfetto with a lane per stage and per DB connection.
2. **Critical path:** walk back from compile end over span nesting, recorded wait edges and schedule dependencies, labelling each segment compute, DB execution, DB queue, retry backoff, gate, retention or dependency. Target: at least 95% of wall time attributed to a named category.
3. **What-if:** recompute the critical path with a category scaled (retries to zero, retention removed, DB execution ×0.1) to rank interventions before building them.
4. **Wall decomposition** per stage, an amplification table per relation, top-N and p99 spans, and a diff between two runs.

The harness makes the numbers repeatable. `just bench compile fastmcp --trace` runs three cold compiles on a fresh logical scope with retention on and off. Micro-benchmarks measure write rows/s by window size and concurrency, the empty-effect latency floor, read rows/s and conflict rate by concurrency. Speed-of-light baselines bound the target: Pyrefly's own check of the same inputs, the same pipeline on in-memory Arrow, and a raw SurrealDB bulk insert of the same bytes.

Build order, each step useful on its own: Perfetto export of today's spans plus per-stage counters and the run summary; then wait spans with blockers; then SDK trace propagation and `QueryStats`; then the critical-path and what-if analyzer; then the harness. Keep the cost honest: zero when `LCTX_TRACE` is unset, and measured overhead under 3% when set.

## Open questions and confidence

The structural diagnosis is high-confidence: source at `b0dd321` and the retained trace agree on every mechanism above. The savings estimates in the design-change and target sections are Proposed until a traced run measures them.

- **Where the 300 s fastmcp figure comes from.** No retained real-fastmcp compile log was found under `build/runs`; if one exists elsewhere, its phase lines can be decomposed the same way. The first traced fastmcp run should report rows per relation, which decides how much ranks 1–2 matter relative to 3–7.
- **The exact edge that holds dependent stages behind retention.** Candidates are the root installation gates (the registry is named `product_retention_gates` and is also taken by input view installation), the 8-permit native-call semaphore, and DB contention. A retention-off run settles whether it matters; wait spans name the edge.
- **Which rows the 34 s livelock fought over.** Retry events carry a query digest but not the conflicting key. Stripes, predicate revisions and guards are the suspects; log the server's conflict detail on each retry.
- **The per-transaction floor.** Effect applies have a 9.6 ms median even for one-row windows. The split between parse, function execution, index work and RocksDB commit is unknown until `QueryStats` or server OTLP is captured, and commit durability settings were not examined.
- **Trace conditions.** The trace is one nextest control on a campaign target under maintenance, with a single test running. Expect similar structure, not identical numbers, from a CLI compile against the stable service.

Nothing was changed on your machine or in the repository. The only actions there were read-only listings and Python parses of existing logs under `build/runs`.

## Appendix: evidence and code references

Code references are at `b0dd321` (main, 18 Oct 2026); paths are relative to the repository root.

| Topic | Location |
| --- | --- |
| Immutable install function (per-row `SELECT`, `array::find` inside `map`, predicate and guard bumps) | `crates/lctx-native/src/control/schema.surql:238` |
| Window construction and the 32× lowering reservation | `crates/lctx-native/src/control/immutable_window.rs:14` |
| Window install: expectation read, then operation-page apply | `crates/lctx-native/src/control/immutable_window.rs:614`, `:632` |
| Operation pages applied under a mutex in ordinal order | `crates/lctx-native/src/control/operation_pages.rs:83` |
| Unbounded conflict retry, jittered backoff up to 250 ms | `crates/lctx-native/src/control/retry.rs:74`, `:152` |
| 128-row window cap | `crates/lctx-native/src/loader.rs:1147` |
| Per-row canonical JSON and physical lowering | `crates/lctx-compiler-store/src/compiler.rs:5800`, `:5887` |
| One ingress driver per contribution, one-slot channel | `crates/lctx-compiler-store/src/compiler/ingress.rs:142` |
| Completion rescan of every membership; originals in 32-row pages | `crates/lctx-compiler-store/src/compiler.rs:6532`, `:6382` |
| Root installation gates shared with input view installation | `crates/lctx-compiler-store/src/compiler.rs:2425`, `:2463` |
| Retention over the transitive dependency closure | `crates/lctx-compiler-store/src/compiler/production.rs:303`; spawned at `crates/cpg-core/src/workspace.rs:273` |
| Read path: per-row `Value` into Arrow builders | `crates/lctx-compiler-store/src/compiler_provider.rs:814`, `:896` |
| 8-permit native-call semaphore | `crates/lctx-compiler-store/src/native_calls.rs:56` |
| Completion gate, scheduled binding and prefix freezing | `crates/cpg-core/src/workspace.rs:137`, `:1626`, `:1644` |
| Producer ingress shared by all of a producer's relations | `crates/cpg-core/src/workspace.rs:2790` |
| Coordinator loop | `crates/cpg-core/src/compilation.rs:1221` |
| Unrequested owners stream inputs to prove exact counts | `crates/cpg-core/src/compilation/uniform.rs:126` |
| Full admission: invariants, frontier, graph, manifest | `crates/lctx-compiler-store/src/artifact.rs:1745`; `crates/lctx-compiler-store/src/admission.rs:1412` |

Trace evidence on your machine:

- Run `build/runs/20261018T044128.645Z-1d2be6`, nested invocation `b653540a2b744e58a4c8cc85df3ce91a`: `verify-evidence/…/native.log` (36.7 MB, 69,632 `lctx_phase` lines), test `prepare_publication_expectations_artifact`, timed out at 360.8 s after a 306.7 s compile.
- The 4,096-row write probe and the 900 s recursive Summary control are summarized in `docs/design_review/reviews/design_review_catalog-publication-performance-target_2026-10-17.md`, section "What the retained executions establish".
- SurrealDB's tracing, `QueryStats` and metrics capabilities are recorded in `docs/design_review/evidence/2026-10-15_operation-diagnostics-surrealdb/README.md`; tracing-appender loss on SIGTERM in `…/2026-10-15_operation-diagnostics-instrumentation/README.md`.
