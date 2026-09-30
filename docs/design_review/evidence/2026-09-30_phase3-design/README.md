# Phase 3 design investigation

**Interface-checked, 2026-09-30.** Baseline `e4ab3ea` on shared main. Concurrent changes in
AGENTS, agent definitions and Claude settings were preserved. This folder supports the
[Phase 3 detailed plan](../../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md).
It does not qualify normalized compilation, analysis or serving.

## Current execution evidence

| Command | Outcome and boundary |
|---|---|
| `uv run python docs/design_review/evidence/2026-09-30_phase3-design/inspect_dependencies.py` | passed: 24 selected resolved packages and their unified features in [dependencies.json](raw/dependencies.json); offline locked metadata, not a build |
| `python3 scripts/build_environment.py -- cargo test --release -p cpg-core --test model_runtime --test generation_read` | passed: 11 real PostgreSQL reader tests and 2 runtime tests; [raw log](raw/baseline-tests.log); existing behavior only |
| Proposed completed-stage reads, normalized admission, normalized semantics and normalized pilot | not_run: these are proposed implementation, not existing executable paths |
| `just test-all`, facts pilots, served journeys | not_run for this documentation/design scope; historical P0–P2 qualification remains in its [own receipt](../2026-09-30_facts-qualification/README.md) |

The existing tests cover provider readback, admitted pushdown, frontier/digest refusals, leases,
mid-stream cancellation, terminal transport loss and shared memory accounting. They do not test
the open close/early-cancellation defects, source-bound registration or cumulative frontiers.
No new runtime probe was necessary to establish those source-visible gaps. Performance of the
proposed design is unmeasured; its qualification workload is specified before implementation.

## Source findings used in the design

| Evidence | Consequence |
|---|---|
| `domain/stages.rs`: `Schedule::build`, `StageAccess::retain/finish`, `emit` | Every input currently has a schedule writer; every declared downstream reader causes retained batches. P3 needs store reads as a distinct input transport, not another reader count on all facts. |
| `cpg-core/src/model_runtime.rs`: `StageSession::register` | Checks stage permit and schema, but accepts arbitrary `Arc<dyn TableProvider>`; schema equality does not establish input generation or producer. |
| `generations/ddl.rs`, `lifecycle.rs`, `receipts.rs`, `control.sql` | Store frontier dispatch and admission are facts-shaped; stage receipts persist only at whole-generation seal. |
| `generations/locks.rs`, `lease.rs` | Generation leases conflict with transitions; per-stage readers must drain before stage completion or seal. Published-only leases must remain distinct from private attempt reads. |
| `lctx-postgres/src/roles.rs`; provider fork `09cc8a8` `pool.rs` | Importer provider connections are currently forbidden; serving provider connections are capped at two. Fork bound-pool limit is 32. Multi-scan admission must account for repeated scans and partitions. |
| `domain/declarations.rs`, `calls.rs`, `occurrence_owner.rs` | Attributed declaration links, complete argument binder and scalar/indexed occurrence-owner rules already exist and should govern normalization. |
| `cpg-extract/src/symbol_records.rs`; `domain/types.rs` | Undecorated signatures are extracted; no general decorated-callable type observation exists. Arbitrary effective signatures cannot be claimed from these facts. |
| `domain/calls.rs`: `site_facts`, `CallPolicy::admits`; `domain_calls` tests | Uniqueness currently uses provider symbols. Association deliberately retains Potential; the old receipt's blanket exclusion sentence is inaccurate. |
| `cpg-schema/src/{derived,projection}.rs`; `cpg-core/tests/dormant/{compile,graph}.rs` | Recover semantic answers, not old keys/SQL winners. `compile.rs` contains P4 obligations as well as P3 controls. |
| `domain/value.rs`, `transfer.rs`, `composition.rs` | P2 Places retain occurrence/provider coordinates; preserve them as attributed facts and use explicit normalized links. P4 still owns normalized transfer/composition realization. |

Paths above are relative to `crates/`, with domain paths under `lctx-model/src` and generation
paths under `lctx-postgres/src`. The plan links the exact owning files. Independent bounded
semantic and runtime assessments informed the draft; the assembled review remains the final
assessment of the resulting document.

The [independent assembled review](../../reviews/design_review_phase3-plan_2026-09-30.md) is
**Accept scoped**, at Proposed strength, after two binding-admission gaps were corrected and
reinspected. The plan's §11 owns their design closure and unstarted implementation controls.
ADR indexing/lint and documentation publication checks are recorded in the current STATUS receipt;
they do not establish semantic or performance qualification.

## Library evidence and limits

The selected skills were `datafusion`, `sqlx-postgres`, `rust-graphs`, and `serde-arrow`.
Their pins were compared with the resolved metadata. Relevant source/contracts were read;
their historical probes were not rerun or relabeled as current measurements.

| Library and pin | Inspected capability evidence | Design use and limit |
|---|---|---|
| DataFusion 55.1.0 / Arrow 59.3.0 | Skill briefs `df.relations`, `df.pushdown`, `df.consume`, `df.source`, `df.storage-reuse`; linked exact-release contracts | Joins/windows, streaming, explicit providers and shared reservations. `collect` retains outputs; streaming does not bound blocking operator state. A view does not cache results. |
| Provider fork `09cc8a8` | `pool.rs`, `conn.rs`, `bounded.rs`; SQLx skill `provider.read-path` | Keep declared schemas, restricted pushdown and bounded transport. Generic inferred bytea loses fixed-width IDs. Crates.io's same package version is not this dependency graph. Lifecycle extensions require a new pinned fork revision during implementation. |
| SQLx 0.9.0 / pgpq 0.12.0 / SeaQuery 1.0.2 | Existing store transactions/COPY/lowering; skill transaction, COPY and provider contracts | Reuse explicit commit/rollback/COPY abort and generated DDL. No new driver or ORM. Workspace metadata shows broader unified SeaQuery features than the direct manifest declaration alone. |
| petgraph 0.8.3 | Skill connectivity brief, container/trait matrix and Graph/filter/reverse contracts | Directed multigraph, canonical construction and borrowed views; SCC at P4. No persistent graph indices or generic transitive-closure engine. |
| serde_arrow 0.15.1 / marrow 0.3.1 | Skill schema, builder and Arrow-59 contracts; generated domain codec | Explicit schemas and validated nominal records; no inference from samples. `arrow-59` is the resolved profile; default hosted docs may show another major. |
| Pyrefly 1.3.1 fork / Ruff 0.0.11 / ty 0.0.14 / salsa 0.28.2 | Existing typed producer and model interfaces | Consume attributed facts; preserve the separate ty line. No application Salsa database or new parse in P3. |
| biodivine-lib-bdd 0.6.3 / fixedbitset 0.5.7 / leiden-rs 0.8.1 | Resolved pins and current consumers | Keep conditions canonical and projection membership bounded. P3 does not add an inference engine or run P4 heuristic analytics. |

Context7 discovery used `/apache/datafusion`, `/websites/rs_sqlx`, and `/petgraph/petgraph`:

- [Official DataFusion provider guide](https://github.com/apache/datafusion/blob/main/docs/source/library-user-guide/custom-table-providers.md): discover pushdown concepts; its main-branch signatures are not evidence for 55.1.0.
- [SQLx transaction documentation](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html): discover explicit transaction lifecycle; pinned local source governs 0.9 details.
- [petgraph source](https://github.com/petgraph/petgraph): discover borrowed views and graph algorithms; the captured 0.8.3 contracts govern the plan.

The Context7 SCC wording and the pinned skill's ordering evidence are not identical. The plan
therefore requires canonical component/member ordering and independent graph controls, rather
than relying on a latest-documentation iteration promise. No performance superiority is inferred
from library names or existing probe counts.
