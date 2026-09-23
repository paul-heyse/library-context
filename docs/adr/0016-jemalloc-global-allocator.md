---
id: ADR-0016
title: The binaries use jemalloc as their global allocator
status: accepted
date: 2026-09-23
supersedes: []
superseded-by: null
design: [§4.3]
evidence: Measured
revisit: tikv-jemallocator leaves Pyrefly's dependency graph or moves to a version Pyrefly does not use (it would add a C build); the pilot's peak under jemalloc exceeds extraction's working set by more than 1 GB; or a target outside Linux/macOS matters.
---

## Context

The C6 deep review (F3) found that the pilot's peak memory (6.7–8.0 GB across identical runs,
DESIGN §4.3) was 40–45% glibc arena retention. Under `MALLOC_ARENA_MAX=2` the peak fell to
4.2 GB, but the compile took 61 s instead of 45 s. The memory triggers of the deferred items read
that noisy, allocator-driven number. The library-leverage review (P1) measured the alternatives
on the pilot.

## Options

1. **glibc malloc (the default).** Peak 6.1–7.3 GB; 44.8–48.9 s. Most of the rise after
   extraction is retention, not working set.
2. **glibc with `MALLOC_ARENA_MAX=2`.** Peak 3.77 GB, but 55.8–61.2 s: validation and derivation
   slow down under arena contention. It is also an environment setting, not a property of the
   binary.
3. **mimalloc 0.1.52 (v3.3.2).** Peak 3.76–4.13 GB, a 370 MiB spread; 39.6–44.3 s. It compiles
   new C code: it is in `Cargo.lock` only as a Windows dependency.
4. **tikv-jemallocator 0.7.0 (jemalloc 5.3.1).** Peak 3.64 GB, within 7 MiB across 4 runs, flat
   from extraction on; 40.3–42.7 s with the lowest user CPU. **It is already compiled**: Pyrefly
   depends on it on Linux and macOS, and Pyrefly's and Ruff's own CLIs use it.

## Decision

`lctx` and `lctx-extract` set `#[global_allocator] static GLOBAL: tikv_jemallocator::Jemalloc`,
under `cfg(any(target_os = "linux", target_os = "macos"))` (Pyrefly's own guard).
`tikv-jemallocator = "=0.7.0"` is a workspace pin, and the lock gains no package. The symbols are
prefixed, so C libraries that call `malloc` directly (zstd) stay on glibc; tuning goes through
`_RJEM_MALLOC_CONF`, and `MALLOC_CONF` is ignored.

## Consequences

- **The peak is the working set.** DESIGN §4.3's memory triggers read the plain `VmHWM` peak;
  the `MALLOC_ARENA_MAX=2` reading is retired, since that variable only affects glibc.
- **Per-rule `VmHWM` deltas lose meaning.** The validation stage's "raised the peak most" line is
  replaced by plan metrics (the library-leverage review's P5).
- **Measured** (the allocator spike, 2026-09-23, 16 runs, one content digest `9e33e57b…` across
  every allocator), then re-measured by H1's pilots (DESIGN §4.3).
- **No `cargo deny` or `check_family.py` change:** the licenses (MIT/Apache-2.0; the C source is
  BSD-2-Clause, which `deny.toml` allows) and the version were already in the graph.
- **Windows builds keep the system allocator**, because of the `cfg` guard.
