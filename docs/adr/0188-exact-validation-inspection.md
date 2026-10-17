---
id: ADR-0188
title: Inspect exact validation records under a read-only service lease
status: accepted
date: 2026-10-17
supersedes: []
superseded-by: null
design: [section-6-5]
evidence: Interface-checked
---

## Context

The upper-execution focused Summary control fails during native SourceArtifact installation
with an immutable-address collision. Its protected unknown native attempts must survive diagnosis.
Exclusive SQL maintenance closes admission and requires every borrower to drain; unresolved
borrowers therefore prevent the observation needed to understand the failure. Entering that path
would close service admission without obtaining the requested evidence.

The service already owns generation-checked shared leases, loopback transport and installed
validation VIEWER credentials. A bounded record observation has no schema, retirement or native
admission effect. The operator authorized this investigation and its corrective followup on
2026-10-17; it grants no settlement of existing unknown owners or inspection of operator data.

## Options

1. **Exclusive arbitrary SQL maintenance.** Keep this for effectful SQL and native MCP. Its drained
   exclusion protects arbitrary effects, but requiring remote drainage for exact reads couples
   diagnosis to the recovery it is meant to inform.
2. **A typed exact-record observation under the ordinary service lease.** Reuse existing validation
   VIEWER authority, actual generation checks and transport. Restrict intent before any request;
   no arbitrary query interpreter, alternate service or new lifecycle registry is needed.
3. **Direct writer/root SQL or inferred borrower release.** Reject: either would bypass the owning
   authority or manufacture drainage from host exit. Copying target data into another service also
   adds work and does not establish the live mismatch.

## Decision

The existing service owner supplies `just service inspect-records FILE`. A finite manifest names
exact allowed physical table/string-key pairs. The command validates the entire bounded input,
constructs only point reads internally, and fixes the target to installed validation. It uses only
installed VIEWER credentials, with no writer/root fallback. Arbitrary SQL, functions, traversal,
MCP, target overrides and child commands are not accepted.

Acquire the ordinary generation-checked shared service lease and require its actual non-null
proof. Existing maintenance admission or inherited maintenance authority refuses; there is no
closed-marker exception. Retain the lease through bounded response reading, envelope/status and
returned-identity checks, then recheck the exact generation before publishing observations.
Bound request size, key count and response bytes; retain the existing HTTP socket timeout. These bounds limit diagnostic
input/output, not compiler/test concurrency. The socket timeout is not a hard total wall-clock deadline.

Unresolved native borrowers may coexist with observations. Do not modify their records, native
admission, maintenance markers, pins, schemas or retirement state. Results carry installation,
generation and observation time; absent records differ from refused or malformed responses.
Concurrent reads are observations, not a coherent completed snapshot or proof of native finality.
Existing managed run/storage ownership retains diagnostic output.

## Consequences

Exact-state diagnosis no longer requires destructive recovery or an exclusive whole-service
barrier. The small allowlisted interface deliberately cannot answer arbitrary SQL questions;
those retain explicit exclusive maintenance. New physical tables need a concrete diagnostic
consumer before extending the allowlist. Transport failure proves neither absence nor drainage.

Independent design advice accepts this boundary at Interface-checked strength. Implementation,
focused negative/lease controls and actual VIEWER point-read verification remain distinct from
that decision. The upper-execution coordinator §6 owns their actual outcomes and the unresolved
Summary gate. Reopen this design for a need to observe a closed maintenance target, mutate state,
inspect operator databases or claim a coherent multi-record snapshot; none is authorized here.
