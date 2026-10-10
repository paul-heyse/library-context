# Build storage: census, attribution and reclamation candidates

**Measured filesystem observations / Interface-checked source, 2026-10-09.** This evidence
supports the [independent build-storage review](../../reviews/design_review_build-storage_2026-10-09.md).
It establishes storage and producer facts, not cleanup permission, a performance result or
product qualification. Nothing was deleted, synchronized, rebuilt or stopped by this review.

## Method and limits

The initial [census](raw/inventory.json) ran from **21:36:03 through 21:36:42 UTC** on the
operator's ext4/relatime root filesystem. It observed **254,723 unique regular-file inodes**,
**409.383 GiB** of regular-file allocations and another **0.240 GiB** of directory allocations,
with zero reported scan errors. Values are binary GiB (`bytes / 1024³`), not decimal GB.

The [follow-up census](raw/inventory-followup.json), **21:41:47–21:42:01 UTC**, observed
**407.867 GiB** of regular-file allocations. Active concurrent compilation changed the shared
cache; no cleanup by this review explains the difference. Available space changed from
**26.328 GiB** at the first census's end to **34.074 GiB** at the second's end. These are live
observations, not an atomic filesystem snapshot or a measured growth rate.

[audit.py](audit.py) reads metadata only. It stays on each explicitly selected root's device,
does not follow symlinks, deduplicates `(device,inode)` and records observed hard-link names.
Per-bucket exclusive-file bytes exclude inodes with links outside that bucket. Across the three
roots there were no regular-file inodes shared between buckets; **0.920 GiB** had unobserved
hard-link names elsewhere. Those bytes are mostly outside-link cases in the shared cache and
review probes and must not be counted as independently reclaimable by unlinking those paths alone.

Allocated bytes are `st_blocks * 512`; apparent lengths, file counts and mtime ranges are
separate. Directory blocks are separate from regular-file bytes. The first census lists but
does not count symlink blocks; the follow-up also records **98,304 bytes** of symlink allocation.
The first largest capture's independent `du` check differed by exactly **16,384 bytes**, traced
to four allocated symlinks; regular-file and directory accounting matched. A separate
`du -B1 -sx /home/paul/.cache/library-context-build` matched the first census's regular-file plus
directory total exactly: **140,279,435,264 bytes**, or **130.645 GiB**.

Neither metadata census determines unique filesystem extent ownership, future use, process
quiescence, last use or creation date. Reclaimable estimates remain upper bounds until open
references, aliases and actual post-removal filesystem availability are checked. The original
claim of 440 GB *created since October 1* cannot be reconstructed from current mtimes. Its units,
scan scope, timestamp meaning and baseline were not supplied. In particular, `~/.cargo/build`
also contains PSE-arrow output, which is outside this cleanup subject.

## Ranked content and disposition

The first census supplies the following regular-file allocation figures. Directory allocations
are included separately in the exact isolated-root candidate table below.

| Content | GiB | Established use and disposition |
|---|---:|---|
| Shared Cargo release profile | 149.280 | **Retain while active.** Includes 101.441 GiB of incremental state; current Cargo/rustc processes reference it. Review variants after current work is quiescent. |
| Eight legacy isolated build roots | 130.504 | **Remove candidates.** Original October 5 commands and removed source worktrees are attributable; no current tracked selector found within the examined scope. Exact total including directories: 130.645 GiB. |
| Five large compiler captures | 101.787 | **Retain pending consumer release or validated archival.** All five have retain markers and current plan citations; see table below. |
| Shared local-test-candidate profile | 18.291 | **Retain.** BC3 has not qualified its replacement of the current verifier default; retiring the temporary profile belongs to that cutover. |
| Other managed run content | 3.785 | **Classify individually.** Includes retained IR/reproducer work and verification receipts. Whole unretained runs account for only 0.438 GiB. |
| Shared debug profile | 2.722 | **Retain pending use review.** A smaller valid profile is not obsolete solely because release is the verifier default. |
| Review-probe content | 2.051 | **Unresolved by subdirectory.** Existing snapshot-diff/graph-analytics consumers and hard links require producer-specific review. |
| Remaining checkout build content | 0.963 | **Retain/classify by owner.** Acquisition sources, analyzed environments, docs, native data, backups, source forks, small studies and fixtures are not one generic cache. |

The active shared release incremental directory contained **1,073 unit directories** at the
later inspection, including 16 `lctx_model` and 21 `cpg_core` prefixes. The old isolated native-test
root contained 185 release incremental unit directories and **55.854 GiB** of incremental state
across its profiles. These counts and repeated crate prefixes are variant leads, not proof that
the compiled units are substitutable. Features, source paths, profile, toolchain, flags,
instrumentation and crate/target kind can all justify different units.

### Exact legacy-root candidates

All paths below are children of **`/home/paul/.cache/library-context-build/`**. These are generated
Cargo trees with `CACHEDIR.TAG`, profile outputs and compiler metadata at their roots. The
census found no outside hard-link names in these buckets. Latest regular-file mtimes were
October 5–6, which supports dating the material but does not establish last use.

| Child path | Allocated GiB, including directories | Original producing scope |
|---|---:|---|
| `graph-native-native-tests` | 87.897 | `cpg-core`/`cpg-extract` test checks and Nextest controls |
| `graph-native-root` | 14.699 | `cpg-extract`/`cpg-core` checks |
| `graph-native-model` | 11.536 | `lctx-model` checks and controls |
| `graph-native-compiler-build` | 8.021 | `cpg-core` checks and release controls |
| `graph-native-effects` | 3.469 | `cpg-core`/`lctx-model` effects/wiring checks |
| `graph-native-manifest` | 2.066 | `cpg-core` manifest-focused checks |
| `graph-native-compiler-check` | 1.789 | Further `cpg-core` checks |
| `graph-native-model-check` | 1.168 | `lctx-model --tests` checks |
| **Total, with parent-directory blocks** | **130.645** | **Candidate estimate, not reclaimed space** |

Before a subsequent cleanup, recheck command/config selectors and live handles against these
exact eight roots, confirm no new producer selected them, and exclude all source copies. A
record of past tests does not require retaining their compiler cache; preserve the test receipts
and unique authored material independently. Removal would sacrifice reuse available only through
those old roots; no cold-rebuild time was measured here.

### Current capture consumers

All five runs are children of **`build/runs/`**. Figures below are regular-file allocation.

| Run | GiB | Recorded termination | Current consumer |
|---|---:|---|---|
| `20261008T181813.329Z-ddd158` | 30.157 | canceled | [Compilation-cost plan §12](../../../plans/rust-compilation-costs-plan_2026-10-08.md#12-interrupted-capture-assessment-and-remaining-correction) |
| `20261008T200802.033Z-6761de` | 21.743 | canceled | Compilation-cost plan's correction comparison and report |
| `20261008T224204.565Z-5a9f8e` | 19.195 | interrupted | [Coroutine plan](../../../plans/rust-coroutine-compilation-amplification-plan_2026-10-08.md) diagnostic before filesystem exhaustion |
| `20261008T225117.998Z-e84d9b` | 15.361 | completed | Coroutine plan's bounded final diagnostic and assessment |
| `20261009T003236.093Z-dfb25d` | 15.332 | completed | Coroutine plan's integrated-source diagnostic and assessment |

These references establish current evidence obligations, not a perpetual requirement for every
sidecar. Retirement requires identifying the exact files needed for each surviving claim and
replay operation. Current `report`, `status`, `view` and run-local assessments consume raw
sidecars; an existing JSON or Markdown summary is not equivalent replay evidence. No archival
destination, archive portability or compression ratio was qualified. Copying an uncompressed
archive to the same filesystem would not resolve capacity pressure.

Across all **131** inventoried managed-run directories, allocations were **105.572 GiB**:
**82.821 GiB** of Rust self-profile data, **15.742 GiB** of perf data, **6.130 GiB** of other
files and **0.879 GiB** of incremental outputs. Retained runs account for **105.134 GiB**.
The biggest individual sidecar was an **11.880 GiB** `cpg_core` self-profile. Perf's 45-second
rotation separates closed chunks for readers; it does not limit total capture size.

## Producer attribution and current routing

Current `.cargo/config.toml` selects `{cargo-cache-home}/build/library-context`; its resolved
root is `/home/paul/.cargo/build/library-context`. Checkout `target/` holds final artifacts
separately. `scripts/build_environment.py` normalizes inherited foreign target exports and
honors explicit target selection; worktree `own` mode supplies an ignored `.dev/build-dir`
selection to recipes and `just env --`, while bare Cargo still reads tracked configuration.
The current registered worktrees are main and the protected dirty `pc3-scope` checkout.

The legacy roots were attributed by targeted examination of matching **October 5** command
records, not by scanning all conversation history. The following session IDs and physical
JSONL line numbers are source pointers; full private transcripts were not copied into evidence.

| Bucket | Session UUID / line | UTC timestamp | Explicit selection in the command |
|---|---|---|---|
| root | `01a10d51-8e55-7e23-91f8-31ca287b7fab` / 1025 | 19:30:28 | `build.build-dir` override |
| native-tests | `01a10d9d-ecce-75c2-82d5-7fa9aff03e28` / 323 | 19:58:03 | `build.build-dir` override |
| compiler-build | `01a10d9c-6367-7170-bef2-bad79969a45a` / 136 | 19:51:54 | `build.build-dir`; workdir `graph-native-compiler` |
| compiler-check | same session / 398 | 19:59:38 | `build.build-dir`; workdir `graph-native-compiler` |
| model | `01a10d9c-8476-7043-9844-a01aeff47eb0` / 1935 | 21:03:47 | `build.build-dir` and matching `library-context-target` target |
| model-check | same session / 1773 | 20:59:32 | `build.build-dir` and matching target |
| manifest | `01a10d9c-6367-7170-bef2-bad79969a45a` / 871 | 20:22:35 | `build.build-dir` and matching target |
| effects | `01a10dea-ffea-7380-a9c4-848aa9ae97c3` / 242 | 21:23:09 | `build.build-dir`, matching target and `graph-native-effects` workdir |

The named compiler/model/effects source worktrees no longer exist. Distinct source slices and
explicit paths support an isolation explanation, but the matched calls do not state a durable
owner or an expiry policy. Current-selector searches covered tracked docs, scripts, tests,
agent instructions and STATUS; their negative result does not rule out a future explicit override.

Adjacent observations, excluded from the three-root total:

- `~/.cache/library-context-worktrees/{graph-native-model-build,graph-native-compiler-build}`
  contain about **3.8 GiB** of Cargo output, with no root `Cargo.toml` or registered worktree.
  They are secondary candidates requiring their own exact inventory. The nearby
  `compiler-workspace-check` contains a non-Git source copy and remains **unresolved/protected**.
- Matching `~/.cache/library-context-target` directories total only **60 KiB**. Their historic
  selectors do not represent another large space-recovery opportunity.
- Shared sccache 0.17.0 reports roughly **36 GiB** of cache data with **100 GiB maximum**;
  `du` rounds the physical tree to 37 GiB. Global counters do not attribute reuse to this repo.
  PSE-arrow has separate active output under the global Cargo build parent.

## Live-use observations

[live_references.py](live_references.py) inspects same-user visible `/proc` cwd/exe, descriptors
and mapped paths without reading command arguments, environments or database contents. The
[first survey](raw/live-references.json), **21:37:45 UTC**, found an active managed run
`20261009T213536.960Z-54edcb`, Cargo/rustc references into the shared release cache and a live
disposable native fixture. The [follow-up survey](raw/live-references-followup.json) also covers
the adjacent historical target/worktree roots. Neither survey found a reference to the eight
legacy build roots. Two descriptor directories were inaccessible in the first survey.

The first survey found zero same-filesystem deleted-open bytes within the reviewed roots;
about **49.8 MiB** elsewhere on that filesystem was observed, outside this task. Memory-backed
`memfd`/`/dev/shm` allocations were explicitly excluded; they do not explain this ext4 usage.
These surveys are observation leads, not proof that absence makes a directory safe to remove.

## Reproduction and verification

Run from the repository root; all four observations write only their explicitly named JSON
output and do not change scanned files:

```bash
python3 docs/design_review/evidence/2026-10-09_build-storage/audit.py \
  /home/paul/.cargo/build/library-context \
  /home/paul/.cache/library-context-build \
  /home/paul/library-context/build > /tmp/lctx-build-storage.json
python3 docs/design_review/evidence/2026-10-09_build-storage/live_references.py \
  /home/paul/.cargo/build/library-context \
  /home/paul/.cache/library-context-build \
  /home/paul/library-context/build \
  /home/paul/library-context-wt/pc3-scope \
  /home/paul/.cache/library-context-worktrees \
  /home/paul/.cache/library-context-target > /tmp/lctx-build-storage-live.json
du -B1 -sx /home/paul/.cache/library-context-build
cargo -Z unstable-options config get build --show-origin
```

**passed, 2026-10-09:** both metadata censuses completed without scan errors; the isolated-root
`du` reconciliation was exact. A disposable control invoked `audit.py` against three empty roots
containing an 8-KiB cross-root hard link, an 8-MiB sparse file and a symlink to the real cache;
it confirmed unique-inode accounting, exclusion of shared links from independent reclamation,
separation of sparse apparent size and no symlink traversal. The actual command was an inline
`python3`/`tempfile` control, not a product test or a new standing suite.

**Interface-checked, 2026-10-09:** `cargo --version` reports
`1.101.0-nightly (3d7cf6e93 2026-09-25)`; `rustc --version` reports
`1.101.0-nightly (c1070d693 2026-09-28)`; `sccache --version` reports `0.17.0`.
Context7 resolve/query was used for Cargo and sccache; claims were narrowed to the installed
Cargo revision and sccache tag where relevant. Pinned Cargo's
[clean implementation](https://github.com/rust-lang/cargo/blob/3d7cf6e93/src/ops/cargo_clean.rs)
removes both selected target and intermediate roots for broad clean; its source explicitly
does not acquire a lock for that branch. Its
[global GC controls](https://github.com/rust-lang/cargo/blob/3d7cf6e93/doc/book/src/reference/unstable.md#gc)
cover download/source caches, not a lifecycle policy for these compiler trees.
[sccache 0.17.0 Rust caveats](https://github.com/mozilla/sccache/blob/v0.17.0/README.md#rust)
exclude incremental compilation and linking outputs; its bounded cache cannot replace all
workspace intermediates. No new library/tool was installed.

**not_run:** deletion, archival, configuration/policy implementation, whole-family/product
qualification, cold-rebuild timing, compression measurement and operator database work. The
review owns proposed corrections and their acceptance scenarios; existing plans retain their
scheduled findings and historical verification boundaries.

**Publication/source checks, 2026-10-09:** `just docs` **passed**, 367 canonical pages and zero
link errors. `uv run --no-sync ruff check --no-force-exclude --target-version py312` and the
matching `ruff format --check` over the two evidence scripts **passed**; JSON parsing and
Python 3.12 syntax inspection passed. Full `just docs-check` **failed** before publication on an
already-stale concurrent ADR index. That unrelated generated file was not regenerated or staged
by this review. This failure does not establish a product defect or negate the scoped publication
check.

**2026-10-10 disposition:** the operator authorized obsolete raw retirement for these five captures while retaining reports, assessments, provenance and run receipts. Independent report bundles are preserved under each run’s `compile-profile-reports/`; the sizes above are historical observations. Actual cleanup is recorded by [storage plan §10](../../../plans/storage-lifecycle-management-plan_2026-10-09.md#10-current-checkpoint).
