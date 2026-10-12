# Efficiency execution integration, 2026-10-11

Question: which source packets and unresolved consumer contracts must survive integration
of the reconciled efficiency track?

The current consumer is [coordinator §4](../../../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint),
especially C04/N04, N05/N09/N11 and S09/D10. This folder preserves reviewable source and
investigation output; it is not another execution plan or completion ledger. The original
reviews and their evidence are unchanged.

## Baseline and current boundary

The packets were prepared against main `45e816a37b276420647a6538f9b8652a33c3bf2e` plus
their explicitly captured dirty dependencies, with execution baseline
`ebcf423cc16e6cacea6455500a0cdfc9795d08ac`. Their source checks describe those captured
bytes, not the current implementation or an accepted executable. Integration has since
revised the source; the coordinator owns current receipts and remaining acceptance.

On 2026-10-12, [ADR-0161](../../../adr/0161-regenerate-unmigratable-design-views.md)
replaced historical translation and opaque preservation with current-format regeneration
from pinned inputs. Consumed historical migration, replay and assurance drafts, their
handoffs and the frozen source archive have been removed from this folder. Git retains
their history. No historical artifact or fixture is a current acceptance prerequisite.
This evidence cleanup does not delete runtime inputs, worktrees, executable artifacts or
service state. The dedicated campaign's interrupted outcome and one-off cutover disposition
remain with their existing runtime and lifecycle owners.

## Surviving packets

| Files | Consumer and limits |
|---|---|
| `c04-provider-demand.patch`, `c04-held-members-dependent.patch`, `c04-integration.md`, `live-installed-view.patch` | C04/N04 exact selected-view authority and bounded member demand. These captured drafts preceded subsequent ownership and reader-lifetime corrections. Keep the direct-consumer handoff while current attachment, mapping, reader protection and native acceptance remain under integration. Earlier focused passes do not qualify later changes. |
| `cleanup-nomination.patch`, `cleanup-nomination.md` | N05/N09 exact outgoing-hold nomination before removal. Preserve the unresolved lifecycle handoff; the draft does not establish collection, product release, binding compaction or current native acceptance. |
| `native-query-lifetime.md`, `raw/native-query-callers.txt` | S09/D10 source investigation of accepted-query ownership, result shape and physical EOF. The later dedicated wire plus eight pure controls passed9 (`20261012T033523.004Z-40c84c`) with native finality and released attachment. That receipt has its original scope; current reader lifetime, accounting and signal acceptance remain with the coordinator. |
| `admission-interface.md` | N11 comparison showing why model-only premises omit actual profile/frontier, facts, producer, reference, graph and manifest gates. Preserve the handoff for the current private artifact-admission owner and its ordinary/restored consumers. The historical migration interface described in the capture is retired; current native admission acceptance remains separate. |

[SHA256SUMS.md](SHA256SUMS.md) identifies the surviving captured packet bytes. Consult
current source before using a patch; later integration intentionally differs. Absolute
off-tree paths in captured handoffs are provenance, not durable runtime dependencies.
The coordinator remains the sole owner of applicability, verification and acceptance.
