---
id: ADR-0144
title: Owner obligations govern managed storage retirement while warm caches remain available
status: accepted
date: 2026-10-09
supersedes: []
superseded-by: null
design: ["section-1-2"]
evidence: Implemented
---

## Context

Shared compiler intermediates, isolated task builds and retained compiler captures have different
consumers. Rebuildability and file age do not establish release. A persistent service also has
maintenance executables and recovery dependencies whose lifetimes exceed individual checkouts.
The [storage plan](../plans/storage-lifecycle-management-plan_2026-10-09.md) owns implementation
and acceptance. ADR-0137's build reuse and ADR-0143's native persistence ownership remain in force.

## Options

1. **Age/size-based deletion** is small but cannot distinguish warm inputs, cited captures,
   active descendants or service dependencies. It would sacrifice immediate reuse.
2. **A generic cache service** duplicates Cargo, uv, sccache and native-store owners and still
   needs application-specific consumer semantics. Its additional daemon and coordination are
   unnecessary for one operator.
3. **Owner-backed lifetimes with declarative policy** add only missing consumer, independent
   component and crash-recovery contracts. Native cache internals remain with their existing owners.
   Category adapters make changes local; explicit protection handles unqualified boundaries.

## Decision

Choose option3. Executable category policy lives in `.config/storage.toml`; the storage manager
keeps private descriptors, ancestor admission locks and retirement journals outside disposable
outputs. Named obligations describe immediate use, raw replay, restorable replay, reports and
receipts. Deadlines start at authoritative release/confirmed cleanup, never modification time.
Current warm caches have no storage-manager eviction policy. No pressure-triggered compiler,
parallelism, optimization, instrumentation or cache-setting reduction is introduced.

Producers establish physical lifetimes when exposing new outputs. Exclusive retirement excludes
managed users through ancestor admission and fixed owner locks, reobserves obligations/citations,
then journals a same-filesystem rename and bounded non-following deletion. Parent/child groups
retire independently. Unknown owners, legacy exposure, mount crossings and uncertain cleanup
protect content. Native content, service data, Git/LFS and opaque caches retain external ownership.

Run receipts, logs, profiling raw and reports have separate consumers. Archive substitution needs
explicit capability requirements, complete digested contents and isolated-location replay with
exact readers; raw consumers are never silently weakened. Other repositories and central skill stores are outside this decision's implementation scope. Service executables transfer only
through the service's maintenance boundary into stable service-owned generations.

The command bootstrap uses the existing pinned Python3.14.7 independently of the project venv.
A single user timer and cheap completion queues may activate only after matching-source integrated
qualification and independent assembled review. Missing or changed qualification blocks scheduled
effects. Ordinary builds retain their existing paths and available parallelism.

## Consequences

Storage eligibility becomes explainable and repeatable without using a file-age heuristic or
scanning/hashing large caches on every build. Whole-group ownership and journals add state that
must itself have bounded receipt/tombstone retention. Unknown legacy consumers require explicit
resolution; a conservative protected disposition is coverage, not recovered space.

**Implemented, focused verification in progress, 2026-10-09:** policy, manager, producer bindings,
repository bindings, portability and service executable support exist. This accepted decision does
not establish every producer's release qualification, assembled acceptance or scheduling. The
[plan §9/§10](../plans/storage-lifecycle-management-plan_2026-10-09.md#9-finding-disposition-owner)
retains those obligations. Revisit if multiple operators, remote storage, independent daemon
availability or different native owners make this single-user filesystem coordination insufficient.
