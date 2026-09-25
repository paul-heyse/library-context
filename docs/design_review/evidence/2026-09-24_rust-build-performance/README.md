# Rust build development screen — 2026-09-24

This records the single-run screen that informed ADR-0026. The pinned workspace source was
`716bed3` (snapshot SHA-256 `e85d1fd34b15cbb53c89e4ce45700512ebb2e7f5ae6deb6ac839746613ec1295`).
The command was `cargo test --locked --workspace --no-run --timings --message-format=json`
in isolated targets. Every successful cold run rebuilt 752 artifacts. Process-tree RSS was
sampled, so peaks are approximate. Local receipts remain under ignored
`build/perf/rust-build-2026-09-24/results/`.

| Toolchain / configuration | Cold wall time | Sampled peak RSS | Outcome |
|---|---:|---:|---|
| stable 1.98.1, 32 jobs, no sccache | 128.27 s | 19.89 GiB | passed |
| stable 1.98.1, 32 jobs, empty isolated sccache | 141.90 s | 21.23 GiB | passed |
| nightly 2026-08-18, 32 jobs × 1 frontend thread, sccache | 139.92 s | 18.96 GiB | passed |
| nightly 2026-08-18, 16 jobs × 2 frontend threads, sccache | 131.20 s | 16.25 GiB | passed |
| nightly 2026-08-18, 8 jobs × 4 frontend threads, sccache | 104.98 s | 17.53 GiB | passed |
| nightly 2026-08-18, 4 jobs × 8 frontend threads, sccache | 113.65 s | 11.90 GiB | passed |
| nightly 2026-09-13, 32 jobs × 1 frontend thread | — | — | failed: `allocative` 0.3.6 conflicting `Allocative` impl (`E0119`) |

The warmed unchanged stable/no-cache run took 0.28 s; the stable/sccache run took 0.37 s.
The sccache run populated an isolated cache, but target recovery was **not_run**. An exact
stable 16-job versus nightly 16-job, one-thread comparison was **not_run**. The first stable
trial-2 was interrupted at the operator's pivot away from benchmarking, before a result was
recorded. None of the single-run differences is a statistically established speedup.

ADR-0026 makes the operator's selected stable 16-job, one-thread, cached route permanent.
It disables Cargo incremental compilation because sccache cannot cache incremental rustc
invocations. The screen above predates that repository default, so it does not measure the
final configuration's end-to-end speed. A future performance decision needs representative
paired runs, including edit/rebuild and cache recovery, if the actual development loop becomes
slow.
