# Status

_Updated 2026-09-23 by the handoff skill._

## Where we are

- **Increment 1** (DESIGN §1.2): slices 1–3 are done; the **CPG plan (C1–C6)** is done, with
  every review fixed; ADR-0015 stores each module's text and role.
- **H1, the library-leverage hardening slice, is done** (`b4ee5cc`…`a974fd6`, 26 commits). It
  carried out every item of `design_review_library-leverage_2026-09-23.md` (operator,
  2026-09-23), and its standard review (`design_review_h1-library-leverage_2026-09-23.md`,
  Revise narrowly) is fixed.
  - **Correctness:**
    - static branches are Pyrefly's own decisions, applied recursively;
    - corpus selection uses globset and walkdir and refuses links (ADR-0018);
    - typed library definitions, `RECORD` read as CSV, clap, a hermetic `git init`.
  - **Runtime:**
    - jemalloc (ADR-0016);
    - cached, concurrent validation;
    - per-commit reads checked against the commit's snapshot (ADR-0017 supersedes ADR-0009);
    - zstd; a quiet log subscriber.
  - **Pins:** every workspace dependency is exact, with a pins row, checked by `just deps`.
  - **Design:** ADR-0011 amended (our own PageRank, normalized Leiden input, our own FCA, §5's
    adapter recipe).
  - **Fork:** `a07b7bae` on the new branch `lctx/1.3.1-r3` (pushed, 2026-09-23).
- **Unpushed:** 26 commits on `main` since `origin/main` (`2473e6a`). Push only when asked.

## Last verified (2026-09-23)

| Command | Outcome |
|---|---|
| `just test-all` (at `9714510`) | passed: nextest 107/107 (about 85 s, from 225 s), pytest 21/21, rule tests 4/4, adr lint 18, family and exact-pin check, cargo-deny, `cargo shear`, pyrefly-fork (`a07b7bae` = tag + patch), gold |
| `just pilot` (fresh store) | passed: snapshot `1e6de86f…`, content `19c3e8e2…`, 905,648 nodes, 1,449,162 edges, all 496 rules; 29.9–33.4 s (from 45.0), peak 3,640–3,649 MiB (from 7,587), store 235 MiB (from 272); stderr is uv's one line |
| cargo-mutants on `hash.rs`/`table.rs` (scratch clone) | 23 caught, 4 equivalent survivors (H1 review O3), 4 unviable |

## Known gaps

- **`build/`** is local and rebuildable. A contract or reader change can make an old store fail;
  delete it or pass a new `--store`.
- **`pyproject.toml`** (the project environment) still owes its ADR-0010 restructuring.

## Open decisions

- **ADR-0011** (analytics) is still `proposed`, amended by H1. It is accepted at its increment-2
  spike; the leiden-rs determinism probe passed on normalized input.
- **Owed by their slices** (H1 review F9): §9.5's PageRank projection and parameters, and
  §9.4's pair-to-arcs lineage.
- **Deferred rows**, each with its trigger in its review:
  - H1 review O2 (bundled-stub identity by content), O4 (a full-content oracle), O5, O6;
  - the leverage review's §7 items (Glean, streaming derive, `FairSpillPool`,
    `datafusion-tracing`, rustworkx-core, datafrog, …);
  - C2–C6 review rows;
  - older ADR-0013, slice-1 and slice-2 rows.
- **`just adr revisit`** (2026-09-23): ADR-0002's `just deps` passed; the other triggers are
  manual, and none has fired.

## Next

Increment 1, slice 4: the analytics config, the invocation projection (built by §5's adapter
recipe over the published catalogs, with its total arc order) and Pass A.
