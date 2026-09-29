# Status

_Updated 2026-09-28 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared `main`._

## Product checkpoint

- **PR4 completed at bounded Tested strength, with the operator's embedding waiver.** Contextual typed selection, four retrieval families, addressable winning units and shared Rust classification/rank validation are implemented. [Product owner](docs/design/sections/api-and-evidence-product.md), ADR-0077/0078.
- **Current formats:** compiler111, extractor37, template21, catalog7, bundle16/projection6/wire3, migration012. Standard1024 remains the embedding contract; pins are unchanged.
- **Operator pivot completed:** exactly two current ready profiles; behavioral selected. Default store/generation paths point to the validated publication. Obsolete generations, stores, runtime copies, rollback dumps, old readers/ANN code and backup-dependent upgrade tooling are removed. Delta remains canonical; PostgreSQL is rebuildable.
- **Review accepted:** [PR4 review](docs/design_review/reviews/design_review_pr4-selection-retrieval_2026-09-28.md). [Forward plan §6.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition) owns bounded closure of AP/F04/F05 and PR4/F01–F18.
- **Packet limit:** two behavioral packets explicitly exceed 256 KiB; every winning original remains accessible and verified. PR5 owns optional enrichment pagination/omission usability work.
- **PR0 comparison blocked:** exact-release/material parity and independent confirmation admission remain unresolved. Confirmation stays sealed; no comparative superiority claim.
- General semantic Stage3 remains incomplete and outside this product gate. PR5–PR6 remain Proposed.

## Verification boundary

[PR4 evidence](docs/design_review/evidence/2026-09-28_pr4/README.md) owns commands and current receipts.

| Command, 2026-09-28 | Outcome and scope |
|---|---|
| `just fmt`; `just test-all` plus localized guard repair and remaining gate components | **passed after repair, composite receipt:** 474 ordinary Rust tests accounted for, 191 Python, 19 real-PG Rust, two real-PG Python, lints/types/rules/dependencies/gold/SQLx/ADR checks. The original full invocation had one expected-output guard failure; its focused rerun passed |
| Packet-budget follow-up and current bootstrap checks | **passed:** one Rust boundary test, 16 affected Python tests, scoped Clippy; three real-PG Python checks, Ruff and Pyrefly |
| `deployment_check.py ... --out build/pr4-qualified-tasks` | **passed:** isolated programmatic and CLI stdio tasks both returned 5 |
| `pilot_probe.py ...`; `compare_profiles.py ...`; operator cutover/cleanup | **passed:** both fresh profiles through real PG/MCP, 18-relation catalog parity, 88-table reconstruction, current operator serving and obsolete-state deletion |
| `just docs-check`; `just fmt-check`; `git diff --check` | **passed:** 168 canonical pages, zero documentation errors; formatting and whitespace checks passed |
| Live embedding/hybrid testing | **not_run:** explicit operator waiver for the critical GPU benchmark; benchmark service unchanged |

- **Build environment implemented (ADR-0079):** dated nightly `2026-09-29`, workspace feature union, CLI-only Hakari, shared Cargo intermediates with fine-grain locking and local final targets. Default/foreign target exports are normalized; benchmark trials own both directories and retain their artifacts. An exact upstream allocative backport makes the pinned graph compile; no dependency version moved.
- **Build validation, 2026-09-28:** release CLI and both native crates, forced editable reinstall/import (nightly + mold identified in the binaries), 13 focused Python tests, shared two-checkout/concurrent-warm probe, dependency checks and docs (168 pages) **passed**. The full `just test-all` **passed**: 475 ordinary Rust tests, 199 Python tests, 19 real-PG Rust tests, three real-PG Python tests, lints/types/rules and SQLx metadata. [Build evidence](docs/design_review/evidence/2026-09-28_cargo-cache/README.md) owns the bounded claims. Performance benchmarks and live embeddings **not_run**; existing benchmark data and service remain intact.

## Next

PR5 agent journeys, bounded packet usability and coarse rebuild/reuse qualification. PR0 independent parity/admission precedes comparative scoring. The [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) is the single execution/adoption owner; the [PostgreSQL runbook](docs/postgresql.md) owns current commands. Completed PR1–PR3 records and the completed PostgreSQL plan are retired; surviving findings/triggers remain in the forward plan.
