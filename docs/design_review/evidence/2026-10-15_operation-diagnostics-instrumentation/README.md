# Operation diagnostics: client and harness instrumentation (2026-10-15)

**Question.** Which tracing, appender, DataFusion, tokio, nextest and stack-capture facilities can give
live and post-run diagnostic insight into long native Rust test controls and CLI compiles, and what
does each fail to establish (for example after SIGTERM/SIGKILL)? Assignment B of the
operation-diagnostics design review. Assignment A covers SurrealDB in
[`2026-10-15_operation-diagnostics-surrealdb`](../2026-10-15_operation-diagnostics-surrealdb/README.md).

**Consumer.** Operation-diagnostics design review (2026-10-15). This folder holds evidence only. It
makes no recommendation.

## Setup

- Machine: x86_64 Linux 7.0.0-38, `kernel.yama.ptrace_scope=0`, `perf_event_paranoid=2`; gdb at
  `/usr/bin/gdb`, perf from linux-tools 7.0.0-38; `eu-stack`/`pstack` absent.
- Toolchain `nightly-2026-09-29` (the workspace pin, via `probe/rust-toolchain.toml`).
- Probe crate `probe/`: `tracing =0.1.44` with `release_max_level_debug`, `tracing-subscriber =0.3.23`
  (`env-filter`, `json`, `registry`, `fmt`), `tracing-appender =0.2.5`, `libc =0.2.190`. The release
  profile is `debug = 0`, as in the workspace. The release test binary has `.symtab` and `.eh_frame`
  but no `.debug_*` sections. The workspace's release test binaries show the same sections.
- cargo-nextest 0.9.146 (commit 8af696ddc). Its source at tag `cargo-nextest-0.9.146` was read
  from the same commit.
- All runs went through `just run --label opdiag-instrumentation-probe`. The probe was built and
  run outside the repository, with `NEXTEST_USER_CONFIG_FILE=none` so that the user record store
  was not touched.

Rebuild: `cargo build --offline --release --bins --tests --manifest-path probe/Cargo.toml --target-dir <scratch>`.

## Results

### 1. `tracing-appender` non-blocking writer versus a direct append `File` (`probe/src/bin/appender.rs`)

The burst events go only to the non-blocking layer (`-nb`). In the `-slow` cases the sink sleeps
20 µs per line, which makes the worker slower than the producer. The figures are lines retained in
the file.

| Mode | Fast sink, 1,000,000 events | Slow sink, 50,000 events |
|---|---|---|
| normal exit (`WorkerGuard` dropped) | 1,000,000 | **14,397** |
| `panic!` (guard dropped by unwinding) | 1,000,000 | **14,363** |
| `raise(SIGTERM)`, exit 143 | 1,000,000 | **445** |
| `abort()`, exit 134 | 1,000,000 | **443** |

- `dropped_lines()` reported 0 throughout. The channel never filled: 50,000 is less than the
  default limit of 128,000 lines.
- The losses come from the guard's bounded shutdown wait, not from lossy mode. Source:
  `tracing-appender-0.2.5/src/non_blocking.rs:283-301` waits 100 ms, then 1000 ms, and then the
  process exits.
- A kill or abort keeps only the lines the worker had already written.
- The direct `Arc<File>` layer (opened `O_APPEND`, one `write_all` per event; source
  `tracing-subscriber-0.3.23/src/fmt/fmt_layer.rs:1018-1050`) retained all 200,000 events in every
  mode, including SIGTERM and abort.

### 2. Per-layer filtering and static level elision

- One `Registry` carried three layers with different filters:
  - a `Targets` filter sent `probe_phase` events to both files;
  - a stderr layer used `LevelFilter::WARN`.
- Results:
  - The INFO event appeared in the files and not on stderr; the WARN event appeared in both.
  - All 10 `trace!` events were absent from every output. With `release_max_level_debug` and
    `debug_assertions` off, TRACE is compiled out regardless of runtime filters.

### 3. `FmtSpan::CLOSE` timings (`raw/fmtspan-close-events.jsonl`)

- The parent did 20 ms of its own work, then entered a child that worked for 50 ms, then waited
  30 ms outside any span.
- The parent's close event recorded `"time.busy":"70.1ms","time.idle":"30.1ms"`, and the child's
  recorded `"time.busy":"50.1ms"`.
- Busy time is therefore **inclusive** of entered children; the source agrees
  (`fmt_layer.rs:932-1010`). The values are formatted strings with units, not numbers.

### 4. nextest timeout, live views and side channel (`probe/tests/live.rs`, `probe/.config/nextest.toml`)

Settings: `slow-timeout = { period = "1s", terminate-after = 4, grace-period = "3s" }`, run with
`--message-format libtest-json-plus` (`raw/nextest-libtest-json-plus.jsonl`).

- **Environment and side channel.** Each test process received:
  - `NEXTEST_RUN_ID`, `NEXTEST_TEST_NAME`, `NEXTEST_ATTEMPT_ID` (`<run-id>:<binary-id>$<test>`),
    `NEXTEST_BINARY_ID`, `NEXTEST_PROFILE`, `NEXTEST_EXECUTION_MODE=process-per-test`,
    `NEXTEST_TEST_GLOBAL_SLOT` and `NEXTEST_ATTEMPT`;
  - its own process group (`pgid == pid`).

  A file appended by the test could be read live from outside while the test ran
  (`raw/side-channel-handles_term.txt`).
- **libtest-json-plus.** The stream emits only `started` and terminal events:
  - each event is flushed as it occurs;
  - there is no slow or terminating event;
  - a timed-out test ends `"event":"failed", ... "reason":"time limit exceeded"` **with no output
    field**.
- **SIGUSR1.** Sending SIGUSR1 to the nextest process printed, on nextest's stderr, each running
  test's PID, elapsed time, slow state and **all output captured so far**
  (`raw/nextest-stderr-with-sigusr1.txt`).
- **Timeout.** On timeout nextest sent SIGTERM to the process group (`TERMINATING` at 4 s).
  - The test with the default disposition ended at 4.005 s.
  - The test with a libc SIGTERM handler wrote `signal=SIGTERM handled in-process` to its side
    file, kept running, and was killed by SIGKILL at the end of the grace period (7.006 s).
  - Source: `nextest-runner/src/runner/unix.rs:77-187`.

### 5. Stack and state capture of a self-started release test process (`probe/stack_capture.sh`)

- **`/proc`.**
  - Per-task `comm`, `stat` state and `wchan` were readable (`futex_do_wait`, `hrtimer_nanosleep`).
  - `/proc/<pid>/syscall` was readable.
  - `/proc/<pid>/task/<tid>/stack` returned `Permission denied`.
- **gdb.** `gdb -batch -p PID -ex 'thread apply all bt'` took 0.08 s for 2 threads and the process
  stayed alive. The output (`raw/gdb-thread-apply-all-bt.txt`) has demangled Rust function names
  from `.symtab` and correct unwinding through `.eh_frame`. It has no file:line, inlined frames or
  locals.
- **perf.** `perf record -F 99 --call-graph dwarf -p PID` worked at paranoid level 2 (user-space
  samples).
  - `perf report --stdio` without `--no-inline` hung during symbolization and was killed.
  - With `--no-inline` it finished at once, with symbol names.
  - perf samples on-CPU time only, so a thread blocked in a futex or sleep contributes nothing.

## Limits

- One machine and small synthetic workloads. The slow-sink loss figures depend on the timing.
- The probes did not measure workspace binaries, DataFusion, tokio taskdump/metrics or
  OpenTelemetry. Those claims are source- or documentation-based and are reported to the
  coordinator.
- No process was attached that the probe did not start.
