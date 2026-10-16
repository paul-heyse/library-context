---
id: ADR-0172
title: Managed runs compose delegated command and capture ownership
status: superseded
date: 2026-10-14
supersedes: [ADR-0142]
superseded-by: ADR-0175
design: [§1.2]
evidence: Tested
revisit: A complete native containment route simplifies authenticated delegated cleanup while preserving shared daemons, observer finalization, capture writers and independent native finality.
---

## Context

The [friction review F09/F10/O01/RC01/RC02](../design_review/reviews/design_review_implementation-friction-target_2026-10-14.md#F09)
found that a directly supervised group could finish while an independently supervised fixture
compiler still wrote the same capture. Direct cleanup was truthful within its scope; it was not
enclosing host cleanup. Full-history detailed discovery also repeatedly hydrated unrelated runs.
The operator accepted composed delegated ownership and bounded discovery on 2026-10-14. This
replaces ADR-0142 self-containedly, retaining its readiness, display, recovery and native contracts.

## Options

1. **Compose existing authenticated owners (baseline).** Prospective registration and a short
   admission/cancellation boundary supply the missing enclosing property without copying native
   authority, creating a permanent supervisor or flattening profiler/shared-daemon lifetimes.
2. **Single transient systemd group (rejected for this integration).** The owned installed255 probe
   contained nested sessions and retained descendants after leader exit, but exact unit stop also
   killed its newly started reusable-daemon witness and interrupted an observer's independent
   finalization. A separately owned outside sentinel survived. Splitting those lifetimes into more
   units/transfer protocols retains explicit owner obligations and adds machinery. This rejects
   the tested disposable-group candidate, not every cgroup design. A cgroup never proves native
   request finality.
3. **Longer grace, status wording or another all-purpose scheduler.** Rejected: these do not
   establish prospective ownership or protect a log writer outside the direct group.

## Decision

Managed discovery/execution observe readiness after acquiring effective shared resource ownership,
held through actual consumption. Explicit exclusive preparation rechecks and validates before
release. Readiness never syncs; pure Rust controls need no Python/native state. ADR-0134 and
ADR-0144 remain the environment/storage authorities.

A run accepts prospective delegated owner/launch registrations before commands become usable.
Closing admission and observing the accepted child set share one short synchronization boundary.
Cancellation stops accepting work and requests registered owners to settle. Every child retains
its receipt's writer and authenticated identity; the enclosing run composes references and cannot
complete host/capture release while a registered writer is unresolved. Owner death retains the
references needed for exact selected recovery. No historical delegation is invented retrospectively.

Direct child/group exit, launch/supervisor failure, delegated-host cleanup, capture health and native
finality remain independent observations. Unknown/failed cleanup protects pruning, storage expiry
and worktree removal. Native Unknown is not settled by an empty process group/cgroup. Persistent
services, shared cache daemons and profiler observers retain their separate actual owners; detached
best-effort display is a reader whose failure cannot change command/capture supervision.

Current run writers/readers preserve historical IDs, bytes, exits and captures without bulk rewrite.
Selected recovery holds the owner lock, revalidates exact identity and can publish its recovery
observation without discarding original results. Full logs are authoritative; direct file capture
avoids display backpressure and suffix access handles small tails.

Default run discovery is a bounded compact recent page with explicit full history, filters and
continuation/coverage. Select cheap metadata before live hydration where possible; a request reuses
one process observation. Live filters are observations, not a simultaneous historical snapshot.
Complete safety consumers explicitly request complete coverage and fail closed on corrupt or
unresolved records; internal complete inventory remains available. No persistent run index is added.

Opt-in exact-command native capture extends existing run/fixture/verification launch seams and
ADR-0147's native-original authority. A short single registration operation attaches invocation
writers to the existing manifest; each writer owns its UUID receipt/raw artifacts and actual cwd.
Original/effective arguments, fixture context, command exit, capture health, postprocessing and
cleanup remain separate. Unsupported capture runs the original command. Bare tools remain
first-class. No shell parsing, synthetic test families, inferred source-to-binary closure or generic
telemetry store is introduced. Exact-case rerun planning still requires declared prerequisites.

Fixture diagnostics reuse actual content/configuration and native envelopes. Invocation-local
direct HTTP MCP remains limited to explicitly authorized non-sensitive disposable contexts;
native errors are not presumed sanitized. No global client registration or second datastore.
Codex remains the primary personal runtime, with explicit overrides and available parallelism.

## Consequences

Prospective references and aggregate release add a real obligation with an existing consumer:
captured bytes and their storage cannot expire while delegated commands still write them. Their
failure behavior must be exercised, including registration races, leader loss, receipt failures,
repeated cancellation and native Unknown. I03 selected explicit delegated-owner composition on
2026-10-14 before dependent integration. The probe's two-second stop policy and four-second inert
observer establish lifetime coupling under that policy, not a production latency benchmark.

The [Catalog coordinator §6](../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint)
owns the current decision result, implementation, focused acceptance and unresolved limits. The
earlier agent-effectiveness plan retains its exact dated AF receipts. No measured benefit, mandatory
runner, healthy-command timeout, resource cap, cache purge or wider product qualification follows
from accepting this decision. Product/cold/Catalog/pilot acceptance stays with its distinct scope.
