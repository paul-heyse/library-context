---
id: ADR-0142
title: Agent execution binds readiness to ownership and separates cleanup and display from child outcomes
status: accepted
date: 2026-10-09
supersedes: []
superseded-by: null
design: [§1.2]
evidence: Proposed
revisit: A managed consumer cannot hold publication ownership through use, or a native observer cannot detach without affecting command supervision.
---

## Context

The [agent-effectiveness follow-up](../plans/agent-effectiveness-followup-plan_2026-10-09.md)
records operator-confirmed RC01–RC04 and source-inspected F01–F05. Existing managed ownership,
durable attempts and disposable fixtures are useful, but readiness can precede ownership,
terminal output can block supervision, and process termination does not establish cleanup.
These supported failure cases need corrected contracts, not another scheduler or instruction gate.
The operator accepted the scope and contract changes on 2026-10-09. This decision complements
ADR-0134's explicit preparation and scoped maintenance; it does not replace those decisions.

## Options

1. **Instructions and native runtime handles alone.** Suitable for ephemeral commands, but
   cannot correct exception leaks, stale admission or blocked supervisors and cannot replace
   identified cross-session recovery.
2. **Correct the current owners and compose native capabilities.** Selected. One run owner,
   requirement-scoped locks, direct file capture and small outcome observations keep changes
   local. A detachable observer and explicit recovery have real lifecycle consumers.
3. **A workflow engine, global lock or permanent diagnostic bridge.** More state and coupling
   without a demonstrated need; would constrain independent native work unnecessarily.

## Decision

- Managed discovery and execution observe readiness after acquiring effective shared resource
  ownership, held through consumption. Exclusive preparation rechecks after acquisition and
  validates before release. Readiness never synchronizes; pure Rust work borrows no Python state.
- The run owner acquires cleanup responsibility immediately after spawn. Actual child exit,
  launch/supervisor failure and cleanup confirmed/failed/unknown are independent observations.
  Unknown or failed cleanup remains recoverable and ineligible for pruning/removal.
- Schema-2 writers and one current reader preserve historical run IDs, fields, exits and
  captures. Observation does not rewrite old records. Explicit selected recovery holds the
  owner lock, revalidates identity and may atomically upgrade a released legacy record.
- Full file logs are authoritative. Live display is independently owned, detachable and best
  effort; it cannot block supervision, capture or cancellation. Small tails use suffix access.
- Fixture diagnostics reuse actual owned content/configuration and native envelopes. Direct
  HTTP MCP uses invocation-local Codex configuration and non-sensitive disposable content;
  its native errors are not claimed sanitized. No global registration or second datastore.
- Codex is the primary personal-workstation runtime; useful host/user customization is eligible.
  Native commands, explicit overrides and available parallelism remain first-class.

## Consequences

The expected benefits follow concrete mechanisms: failure cannot escape spawned-child cleanup,
a stopped reader cannot hold supervision, readiness describes the protected publication being
used, and an actual child127 cannot masquerade as a launch failure. These are Proposed until
implemented and exercised; no effectiveness or speed measurement is implied.

The extra observer, lock-protected selected recovery and fixture client have explicit lifetimes
and focused negative cases. Retained profiling evidence remains usable. Unmanaged commands
remain available outside managed guarantees. No new workload, thread or healthy-duration cap,
mandatory startup audit, selector store or semantic index follows.

[Plan §8](../plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner)
owns implementation and closure. Decision acceptance alone closes no finding. The operator's
execution-specific acceptance scope is recorded in that plan and does not change ADR-0126's
general assurance policy.
