---
id: ADR-0181
title: Retain checked native context through the actual audit pin lifetime
status: accepted
date: 2026-10-16
supersedes: []
superseded-by: null
design: ["§6.1"]
evidence: Interface-checked
revisit: A supported writer can change generation or validity while the exact audit pin remains live, or lifecycle closure cannot reject reuse before remote pin release.
---

## Context

RN03 and approved RC03 require complete protected premises to survive preserving reads under
controlled writes. The [runtime coordinator §6](../plans/catalog-runtime-efficiency-plan_2026-10-16.md#6-sole-disposition-and-current-checkpoint)
owns R:F05 and current acceptance. Fresh native view selection currently repeats generation and
validity-epoch queries even while one actual audit pin protects both values. This decision refines
ADR-0178's controlled-write realization; its effect, receiving, recovery and semantic contracts remain.

## Options

1. Retain every remote context comparison. Simple and detects arbitrary live administrative edits,
   but repeats assurance already protected by the native pin. Live arbitrary edits are excluded by
   the approved controlled-write model.
2. Observe context once after pin acquisition, then reuse it through that exact protected lifetime.
   Selected. Reuses native pin admission, drained maintenance and existing source/session ownership;
   fresh view-specific earning remains independent.
3. Memoize constructor context or infer authority from a local lease, descriptor or `Arc`.
   Rejected. Context capture precedes pin acquisition and can race maintenance; these objects alone
   do not establish native exclusion or an open source.
4. Add a final context CAS to every ordinary selection. A valid alternative where protection is
   unavailable, but it adds writes and composition without improving the held-pin case.

## Decision

`lctx-native` owns the checked audit context. Establish it with a successful native generation and
validity comparison **after** the exact audit pin has been admitted. Constructor capture alone is
insufficient. Do not publish successful context knowledge after a failed, cancelled or Unknown check.

Authorized validity advancement requires the installation admission cut and drained predicate;
an unreleased audit pin excludes it. Pin admission participates in the same cut. Generation/schema
replacement requires authenticated reader/worker drainage and quiescence. These native protocols,
not a host mutex or local operation lease, protect the retained observation.

Reuse requires the same actual pin, client/session, target, resource pool and open source. Check
closing/admission state before reuse: close disables new work before remote pin release acknowledges.
Already admitted work retains its actual owner and must drain. Released, closing, foreign or
uncertain ownership cannot become a reusable premise merely because an allocation remains alive.
Unprotected acquisition retains its necessary native comparison.

Fresh descriptor, contributor, membership, alias, incarnation and semantic admission checks remain
strict. Independent receiving retains its complete ordered CAS inventory and final current-context
guard. Ordinary selection has no invented final context CAS; its reused context rests on pin exclusion.
No cross-source/global context cache or new native retry path is introduced.

## Consequences

Repeated preserving operations avoid remote assurance for unchanged protected context. The first
post-pin check, lifetime bookkeeping and fresh semantic earning remain necessary. Deliberate raw
administrative epoch edits during live reads no longer define ordinary preservation; they require
quiescence/rebuild. Controls must retain post-pin acquisition-race, closing/release, foreign-owner,
native drained-predicate and independent receiving-CAS coverage.

Independent source/design advice on 2026-10-16 accepts this route within RN03/RC03. Implementation
and actual functional acceptance remain pending with the coordinator. No timing or memory
improvement is measured by acceptance of this decision.
