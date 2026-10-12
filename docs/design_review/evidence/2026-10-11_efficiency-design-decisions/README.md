# Efficiency design-decision investigation

**2026-10-11; Interface-checked design selections, with bounded Tested controls below.**
Consumer: the [new efficiency coordinator](../../../plans/efficiency-alignment-plan_2026-10-11.md)
and its five companions. This folder records the approved investigation of their open mechanisms;
it is evidence, not another plan or finding-disposition owner. Product packages remain **Planned**.

## Baseline and authorization

Main and the independent probe checkout began clean at
`439f3d2dd73135cfada43a58e90361a56a96668e`. The original review's source was `88f33bc0`.
The intervening product changes are in artifact-manifest eligibility, native compiler/reader,
selection and derived-search paths and their controls. The investigation uses those current
context/negative-witness and charged-batch foundations rather than restoring the review snapshot.
No new product edits were integrated by the investigation. Probe code ran on branch
`wt/efficiency-decisions`; its surviving source and evidence are integrated in the main checkout.

The operator authorized a dedicated native campaign and bounded implementation probes, integrating
decision evidence and plan updates only. Existing plans were not reviewed or superseded. Original
review/audit documents remain unchanged. No shared service installation, restart, database query,
maintenance, fixture borrowing, environment synchronization, operator adoption, real-library compile,
protected evaluation input, vLLM operation or repository quality campaign was performed.

## Evidence routes

| Material | Question and scope |
|---|---|
| [Native contracts](native.md) | N01–N11: predicates, effect grain, complete logical/physical closure, derivations, premises, migration and release |
| [Compiler and analysis](compiler-analysis.md) | C01–C08 and A02/A04/A06: complete input keys, value/evidence separation, demand, compact outcomes, exact kernels and batching |
| [Development and linked serving](development-serving.md) | D02–D05/D09/D10 and S03/S05/S06/S09: dependency/profile boundaries, protected observations, metadata and remote ownership |
| [Owned engine campaign](native_campaign.py) | Actual patched SurrealDB RocksDB conflict/phantom/rollback primitives; dedicated gRPC disconnect observations |
| [Cold tar/hash control](cold_hash_probe.py) | Exact copied-asset and finalized-archive byte hashing, trailing metadata and fail-closed publication |
| `raw/` | Exact engine/campaign outputs, including the first setup failure; tracked through Git LFS |

Each lane records selected mechanisms, rejected alternatives, migration consumers and the checks
still required on the eventual implementation. Finite design models are counterexamples and
contract checks; they do not qualify nonexistent product mechanisms. Static source can settle a
design choice without proving its eventual implementation. No speed, capacity or contention
improvement is claimed.

## Independent content review

A separate design reviewer assessed `439f3d2d` plus the final six-plan/evidence overlay,
read-only, on 2026-10-11. The reviewer found no remaining material issue in the selected
decisions after reconciliation. The review tightened N03's operation-owner allocation so ordinary
windows do not recreate a global write, C05's relation-local cutoff/configuration groups with current
exact receipts, and C07's complete captured-tree bracket before provider completion. It also checked
that PD02 consistently records a reproduced defect and a required server-owner correction.

This accepts the bounded design content. It does not close product findings or establish implementation,
migration, native qualification or performance. The reviewer ran no builds, tests, services or quality
checks. The coordinator alone owns finding dispositions and subsequent implementation acceptance.

## Native campaign boundary

The campaign verified and copied the immutable executable from generation
`405f6c02ddfe12c004b04deab6bd91fd1458ed1c3ba247c05baa4a5396d5e206`, SHA256
`710229df33346fdf9733ef86d46e31874f0cacc2691529327e19e6534477b8ed`.
It reports `3.3.0+lctx.timeout-stack.1`; upstream commit
`238bfeb11f5725bebed370167656748df8067595`, with the repository's recorded timeout-stack
and DiskANN build patches. The SDK is the repository's patched 3.3.0; protocol is 0.13.1.
Exact source was read under the service-owned source-cache lineage, never edited.
Context7 documentation supplied discovery; pinned source and these controls establish the
applicable backend behavior, not a cross-version documentation transfer.

The child server used a fresh RocksDB path, ephemeral loopback port, fresh in-memory credentials,
and the existing 20s query/10s transaction deadlines. The systemd installer was deliberately not
used: its unit name is fixed even when the state path is redirected. Normal compiler and runtime
parallelism were preserved. Engine cases use one database with fresh case state; no per-test
service or database policy is proposed.

The first run, `20261011T130336.743Z-0acd2e`, **failed** because the new namespace had not been
defined. Its child stopped with exit0 and outer cleanup was confirmed. No engine result is credited
from that run. The corrected run, `20261011T130506.074Z-fe9f43`, exited0 and confirmed cleanup,
including a waited server exit0 and no surviving run members. Seven concurrent cases, late THROW
rollback and unsupported range-FOR-UPDATE refusal were inspected and checked independently with
`validate_engine()` in the campaign source. The timed SLEEP windows establish overlap only; their
durations are not performance evidence.

Write guards on existing and first-created absent rows, named FOR UPDATE reads, and all-stripe
cuts produced the expected competing-transaction conflict. Plain range reads allowed a phantom;
writing the same target guard prevented it. Independent stripes committed both effects. Late
THROW left neither its new effect nor its revision increment. These are **Tested engine premises**,
not a test of the proposed 64-stripe application protocol or a complete retirement migration.

The gRPC campaign `20261011T131054.150Z-4fd098` completed with outer exit0, confirmed cleanup
and waited server exit0. Its raw tonic client is [preserved here](grpc_campaign.rs). It is a
protocol control, not a claim that a mocked provider or the SDK's local consumer-drop route passed.
An explicit transaction drained to EOF committed `grpc_probe:drained`. Dropping after Begin plus
250ms left the uncommitted row absent and produced this actual server error at 13:11:01.489564Z:
“A transaction was dropped without being committed or cancelled”. An autocommit stream dropped
after its first batch retained the earlier committed row and did not write the later row. Subsequent
health query passed. The harness completed successfully; the explicit-transaction **remote finalization
control failed**, confirming PD02's missing execution owner. Row absence and health do not erase that
failure. [Server log](raw/grpc-01-server.log) and [observations](raw/grpc-01-observations.json) retain
the exact distinction. The proposed owner fix is not implemented or qualified in this investigation.

## Commands and outcomes

Commands use repository Python 3.14.7 and pinned nightly-2026-09-29. Small lane probes have their
exact commands and results in the linked notes. Product acceptance and broad checks are **not_run**.
The commands below describe the original investigation, with fresh output destinations shown for
any explicit replay. Original absolute destinations remain in the immutable managed run records;
retaining or replaying the old worktree, executable copies or RocksDB stores is not required.

```sh
uv run --no-sync python scripts/worktree.py create efficiency-decisions --ref 439f3d2d --no-ready
uv run --no-sync python docs/design_review/evidence/2026-10-11_efficiency-design-decisions/cold_hash_probe.py
just run --background --label efficiency-engine -- uv run --no-sync python docs/design_review/evidence/2026-10-11_efficiency-design-decisions/native_campaign.py --binary /home/paul/.local/state/library-context/surrealdb.tools/surreal/405f6c02ddfe12c004b04deab6bd91fd1458ed1c3ba247c05baa4a5396d5e206/surreal --destination /absolute/new-campaign-output/engine-02
# In the probe checkout, with grpc_campaign.rs copied to crates/lctx-publisher/tests/efficiency_grpc_campaign.rs:
cargo test --release -p lctx-publisher --test efficiency_grpc_campaign --no-run
# Use the exact executable path Cargo reports as --grpc-probe:
just run --background --label efficiency-grpc -- uv run --no-sync python docs/design_review/evidence/2026-10-11_efficiency-design-decisions/native_campaign.py --binary /home/paul/.local/state/library-context/surrealdb.tools/surreal/405f6c02ddfe12c004b04deab6bd91fd1458ed1c3ba247c05baa4a5396d5e206/surreal --destination /absolute/new-campaign-output/grpc-01 --grpc-only --grpc-probe /home/paul/.cargo/build/library-context/release/build/lctx-publisher/f38b14262b567ed1/out/efficiency_grpc_campaign-f38b14262b567ed1
```

Worktree creation **passed**, without readiness/sync. Cold tar/hash control **passed** six groups:
normal framing, short writes, write error, finalization error, fsync refusal and short asset read.
Archive hashing includes padding/trailer; a pointer is available only after successful finalization
and the durability boundary. Appending `recovery.json` is compatible with the inspected name-based
restore lookup. The control exercises standard-library I/O with synthetic files; actual private
service backup/restore and source-owner quiescence acceptance remain **not_run**.
Keeping metadata first would require a prehash or another spool; appending it avoids both and
preserves the existing name-based restore contract. Seeking back to rewrite tar metadata would
invalidate an incremental whole-archive hash and is rejected. No independent restore check is
removed. The selected production contract is in the native plan's transition section.

## Storage and preservation

Campaign paths were admitted before creation/use; the corrected campaign enrolled its output
under its actual run owner and named consumer `citation:efficiency-design-decisions`. Run receipt,
server logs and observations retain their producer/cleanup boundary. The failed setup path remains
protected evidence. Credentials were not persisted or printed. No state was collected or deleted
following test success. Exact raw evidence is copied here; compiler artifacts and RocksDB stores
are not committed. On 2026-10-12, the completed engine and gRPC campaign logs and observations
were checked byte-for-byte against these canonical raw copies; the failed setup observations and
server log were also copied unchanged as `raw/engine-01-observations.json` and
`raw/engine-01-server.log`. All three original managed runs retain confirmed child cleanup with no
survivors (`0acd2e`, `fe9f43`, `4fd098`); the failed setup retains its original failure.
The exact `citation:efficiency-design-decisions` obligations on the completed engine and gRPC
campaign outputs were released after those checks. This evidence consumer needs the canonical raw files and probe
source, not the old executable copies or runtime stores. Their filesystem disposal and worktree
removal still belong to the existing storage/worktree owners and must recheck live holders.
