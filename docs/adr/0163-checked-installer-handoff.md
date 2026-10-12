---
id: ADR-0163
title: Hand off maintenance executables against unchanged native installations
status: accepted
date: 2026-10-12
supersedes: []
superseded-by: null
design: [§6.5]
evidence: Interface-checked
revisit: A needed executable changes the installed native schema, an interruption cannot reconcile its exact descriptor successor, or a retained predecessor has no current consumer.
---

## Context

The efficiency campaign repeatedly reached acknowledged daemon join but could not finish
native drainage through its immutable older maintenance executable. The current executable
corrects operation SQL without changing the installed native schema. Separate bounded repairs
preserved original recovery evidence, but repeating those repairs is an unnecessary operating
burden. Same-digest stabilization deliberately cannot replace executable identity.

The service owner in [storage §6.5](../design/sections/storage-and-publication.md#section-6-5)
must migrate this live dependency without rebuilding compatible current content. The operator's
design-phase direction removes legacy runtime paths; completed recovery evidence retains its
original outcome rather than becoming authority to execute an obsolete installer. The
[efficiency coordinator D04](../plans/efficiency-alignment-plan_2026-10-11.md#4-package-sequence-and-sole-execution-checkpoint)
owns implementation and actual acceptance. This complements existing service and recovery
decisions; it does not replace their native-state or exact-owner contracts.

## Options

1. Keep the current binding and repeat separate repairs. This preserves identity but retains a
   known defective live dependency and repeats privileged coordination after otherwise useful tests.
2. Use a checked handoff in the existing service owner (selected). Reuse immutable executable
   preparation, admission locks and native checks; add only the journal needed to reconcile the
   descriptor publication. The semantic store and its consumers remain independent of a CLI repair.
3. Regenerate compatible databases. This discards useful current content without addressing an
   executable-only change. Regeneration remains the route for incompatible design-phase state.
4. Overwrite the immutable executable or repurpose same-digest stabilization. Either destroys the
   identity contract on which existing maintenance and recovery evidence depend.

## Decision

The existing service owner exposes explicit `maintenance --handoff-installer PATH`. It admits and
copies the candidate into its immutable tools store, then owns exclusive host admission, borrower
drainage and the maintenance token. No unresolved borrower or specialized maintenance operation
may be bypassed; any named recovery completes first.

Before candidate effects, compare its declared native schema/version with the actual unmodified
main and validation markers and the selected installation generation. Candidate closure, actual
drainage and readiness checks must succeed on both databases. Require observed closed admission;
a helper that normalizes an observation to closed is not evidence. Use the candidate explicitly
under this operation, because calling a defective predecessor's drain first would defeat the handoff.

Journal exact predecessor/successor descriptor bytes and immutable executable identities privately.
Publish only the `installer` and `installer_generation` descriptor fields, atomically, after both
scopes pass. Restarting the operation accepts only its same candidate and exact predecessor or
successor descriptor. A failure or uncertain acknowledgement keeps admission closed with the
journal available for reconciliation. Success rechecks the published descriptor and normal service
readiness before releasing maintenance.

The service, server, native generation, schema, credentials, selection and semantic content do not
change. Ordinary close/drain/open control effects retain their existing meaning. Original named
recovery plans, joins, receipts and Unknown outcomes remain immutable. Same-digest stabilization
continues to mean same-digest transfer. A different schema refuses this handoff and follows explicit
current-design regeneration instead of reviving historical migration paths.

## Consequences

Maintenance can consume a corrected executable while preserving useful current native content.
The existing service owner bears one bounded publication/recovery contract; no second daemon,
automatic upgrade policy or general migration framework is introduced. Previous executables lose
their live maintenance consumer after handoff; their owner must resolve any actual recovery consumer
before retirement, without inferring deletion authority from a changed digest.

Independent source-led judgment accepted this contract on 2026-10-12. Implementation and focused
host/actual campaign acceptance remain open in the coordinator. Revealing controls cover the second
database's schema and drain refusals, interruption around descriptor publication, exact same-candidate
resume and the two-field-only success. Decision acceptance alone closes no source finding.
