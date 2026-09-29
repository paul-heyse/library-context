# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared `main`._

## Product checkpoint

- **PR5 completed at bounded Tested strength.** Commit `35afc09` adds bounded invocation packets/sections, scoped browse/vocabulary, independent evidence search, ordered comparison and coarse catalog/retrieval rebuilds (ADR-0081). [Product owner](docs/design/sections/api-and-evidence-product.md).
- **Current formats:** compiler112, extractor37, template21, catalog7, bundle16/projection6/wire4, migration012. Superseded public packet encodings are removed.
- Complete cores and optional section continuation now serve the formerly oversized CLI/auth operations. Probe responses reached at most 34,100 bytes within the expanded 256 KiB bound; default admission remains 32 KiB.
- **Rebuild qualification:** eight controls compare every canonical column, including provenance/evidence, across clean/reused runs and input/policy changes. Invalid disposable entries recompute. Pure-stage admission re-derives output; no CPU speedup is claimed.
- **Review accepted at bounded source-inspected strength:** [PR5 review](docs/design_review/reviews/design_review_pr5-agent-journeys-rebuild_2026-09-29.md). [Forward-plan §6.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition) owns PR5/F01–F04 and CLF/F01/F03 closure.
- **Current-only operator pivot passed:** exactly two ready NVFP4 profiles; behavioral selected. Both default paths point to the validated current publication. Superseded runtime/artifact/fixture generations and stale pointers are removed; temporary reconstruction archives are deleted. All 58 protected benchmark files were unchanged.
- **PR0 comparison blocked:** exact-release/material parity and independent confirmation admission remain unresolved. Confirmation stays sealed. PR6 comparative work remains Proposed; general semantic Stage3 remains incomplete and outside this product gate.

## Verification boundary

[PR5 evidence](docs/design_review/evidence/2026-09-28_pr5/README.md) owns commands and receipts.

| Command, 2026-09-29 | Outcome and scope |
|---|---|
| `just fmt`; `just test-all` with localized reruns and remaining gate components | **passed, composite receipt:** 485 ordinary Rust tests accounted for, 204 Python, 19 real-PG Rust, three real-PG Python; types/lints/rules/dependencies/gold/SQLx/ADR checks. Initial failures and focused repairs remain explicit in the receipt |
| `cargo nextest ... --test rebuild` with focused selectors | **passed:** eight controls; both profiles, membership/absence/coordinates/coverage/provider attribution/roots/policy, corrupt cache and retrieval boundary |
| `deployment_check.py ... --out build/pr5-qualified-tasks` | **passed:** isolated pinned programmatic and CLI stdio tasks each returned 5 |
| `pilot_probe.py build/pr5-qualified build/pr5-qualified-tasks --reuse-published` | **passed:** fresh catalog/behavioral compilation, 114 real MCP calls across both routes, and 88-relation reconstruction |
| `operator_cutover.py ...`; `cleanup_current.py --apply` | **passed:** current serving, selection and reconstruction; obsolete runtime removal and benchmark preservation |
| `just docs-check`; `just fmt-check`; `git diff --check` | **passed:** 173 canonical pages, zero documentation errors |
| Accuracy assessment, comparative evaluation, sealed tasks, performance benchmarks | **not_run**, outside authorized scope; benchmark artifacts preserved |

## Environment and next action

- **ADR-0079:** dated nightly `2026-09-29`, workspace feature union, CLI-only Hakari, shared Cargo intermediates/fine-grain locking and local final targets. Existing benchmark data and directories remain intact. [Build evidence](docs/design_review/evidence/2026-09-28_cargo-cache/README.md).
- **ADR-0080:** published NVFP4-r2 checkpoint and gpu-stack B3/r2 SM120 wheel run at `http://127.0.0.1:8000`; full spec binds model/tokenizer/runtime, standard1024 output. Basic live validation passed; current PR5 journeys use this runtime without accuracy claims.
- Current runtime and commands are in the [PostgreSQL runbook](docs/postgresql.md). No prior generation or rollback copy is retained; accuracy/performance assessment was not run.
- Next product work is PR6 under the [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md), subject to the unresolved PR0 comparison admission. The concurrent external graph-review document is preserved and is not part of this implementation.
