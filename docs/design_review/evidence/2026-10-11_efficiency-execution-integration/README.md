# Efficiency execution integration, 2026-10-11

Question: which exact source packets and unresolved consumer contracts must survive the
capacity-blocked integration of the reconciled efficiency track?

The current consumer is [coordinator §4](../../../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint),
especially C04/C05/C06 and N03/N04/N05/N06/N08/N09/N10/N11. This folder preserves reviewable
source and investigation output; it is not another execution plan or a completion ledger.
The original reviews and their evidence are unchanged.

## Baseline and method

Execution baseline is `ebcf423cc16e6cacea6455500a0cdfc9795d08ac`; the source packets were
prepared against main `45e816a37b276420647a6538f9b8652a33c3bf2e` plus their explicitly
captured dirty dependencies. Authentic source5 remains schema
`eec4dc767fe4477fbfc3f5a584b54a6c8855345af02c3390ad5f77ac326e0383`, model
`b6161054e824aac56f85bd596117e9f740b8416f35ebc3fe072ef81b34471da6`.
The dedicated installation and retained cold archives remain in their existing lifecycle
owners; no database, build cache, environment or executable is copied into this folder.

Investigation used Git source recovery, bounded typed metadata inspection and independent
source checks on the Linux development host. Exact source-recovery commands and pins are in
the packet handoffs. Root used `git apply --check` before integrating accepted hunks.
These are source checks only. Cargo compilation and functional execution are **blocked**
after the earlier LLVM/filesystem ENOSPC; the current tree has no compile or native acceptance.
No service effects, quality campaigns or dependency version moves follow from these packets.

## Preserved packets

| Files | State and limits |
|---|---|
| `migration-foundation.patch`, `migration-foundation.md` | **Unapplied draft.** Exact source5→6 contract, original journal, resumable progress, source control/retiring/frozen audits, component ordering and aggregate activation interface. Seven non-Control handlers, real maintenance authority, finalizer and CLI/host consumers remain required. It must not be applied alone as a runnable converter. |
| `migration-replay-pointer.patch`, `migration-replay-pointer.md`, `migration-host-interface.md` | **Unapplied dependent draft.** Authenticated original committed installation outcomes and required live host lease wiring. Serialized evidence is insufficient; pending/new requests cannot enter an old-era route. |
| `historical-assurance-draft.patch`, `historical-declarations.md`, `raw/historical-packet.tar` | **Unapplied historical declaration draft.** Original identity recipes, six provider declarations, nine normalization stages in both profiles, explicit source-recovery scripts and selected original metadata. The archive preserves all 16 small source/evidence files with their ledger. No semantic constructor, current producer substitution or legacy reader is supplied; 21 upper recipes remain unsupported. Rust controls are **not_run**. Raw archive uses Git LFS. |
| `c04-provider-demand.patch`, `c04-held-members-dependent.patch`, `c04-integration.md`, `live-installed-view.patch` | Applied to main as uncompiled source, with root corrections retaining compiler operation ownership and exact native-session identity. Live/published held-view earning, bounded compact member lookup and direct consumers are integrated; audit injects its exact published capability. Ten written controls are **not_run**. Optimizer-elided empty scans require authority at provider/input creation; scan-level validation alone cannot close that obligation. Cold dependency attachment, paged mappings and native acceptance remain open. |
| `cleanup-nomination.patch`, `cleanup-nomination.md` | Applied to main, **not compiled or tested**. Exact bounded outgoing hold page and retirement nomination commit before hold removal in one cleanup-owned transaction. No collection, product release or binding compaction is established. |
| `native-query-lifetime.md`, `raw/native-query-callers.txt` | Source investigation only. Buffered `query_native` has no retained accepted-query terminal owner; current SDK streaming also loses result shape and empty-SINGLE metadata. No approximate collector or expanded dependency patch was installed. Root separately repaired private/owned `NativeRows::Drop` drainage and wrote one revealing control, **not_run**; that does not close buffered-query ownership/accounting. |
| `admission-interface.md` | Source comparison found that model-only premises omit actual profile/frontier applicability and facts/producer/reference/graph/manifest gates. Both admission transitions refuse before SQL until the full evidence owner and ordinary/restore consumers exist. No model-only semantic grant is installed. |

[SHA256SUMS.md](SHA256SUMS.md) identifies the captured packet bytes, not the eventual
integrated source or an accepted executable. Consult current source before replaying any patch;
later integration may intentionally differ. Off-tree absolute paths in handoffs are provenance,
not durable runtime dependencies. The coordinator owns remaining applicability, admission,
migration, preservation and acceptance obligations.
