---
id: ADR-0126
title: Validation trusts acknowledged immutable inputs and verification follows current contracts
status: accepted
date: 2026-10-04
supersedes: [ADR-0110]
superseded-by: null
design: [§1.2, §6.1, §6.2, §8, §15]
evidence: Interface-checked
revisit: A legitimate service writer can mutate an acknowledged frame, or family selection omits a current product guarantee, or agents repeatedly skip `just turn-end`.
---

## Context

The [assurance coordinator](../plans/testing-architecture-pivot-plan_2026-10-04.md) owns F01–F04
from the current testing architecture review. Model validators were repeated under different names;
publication, read grants and selection repeatedly scanned the same acknowledged inputs. Broad test
commands prepared unrelated adapters and recreated identical Catalog fixtures. Some oracle claims
pointed to absent current controls. This project is in the design phase: historical suites and
runtime copies have no retention authority.

[§15](../design/sections/semantic-model.md) owns semantic declarations;
[§6](../design/sections/storage-and-publication.md) owns store effects;
[§8](../design/sections/validation-and-evaluation.md) owns assurance meaning;
[§1.2](../design/DESIGN.md#section-1-2) owns the development loop.
The target is accepted; implementation and qualification remain separately recorded by the coordinator.

## Options

1. Repeat physical hashing and semantic replay at every consumer. This can challenge privileged
   mutation, but duplicates computation even where owned immutability already excludes legal writers.
2. Deduplicate callback/name occurrences or retain one relation attachment. This hides complete
   premises and makes a relation an accidental authority for other consumers.
3. Add a global validation/pass cache. This creates a second evidence owner and an unclear invalidation
   contract for mutable effects and incomplete runs.
4. Declare obligations once, acknowledge exact immutable bindings in existing receipts, and organize
   verification by explicit contract families (selected). Scope remains visible and independently testable.

## Decision

Model-owned definitions carry stable IDs/revisions and complete ordered premises; relation/stage
references resolve explicitly and participate in model identity. Missing premises refuse. Store
receipts acknowledge only complete successful bindings discriminating installation, generation,
model/layout, definition, immutable source/prefix and answer-affecting configuration.

Ordinary output and vocabulary closure drain writers and exclude further legal mutation before
validation and acknowledgement. READ COMMITTED is sufficient only under those enforced unchanged
logical frames. Session preparation and compatible ordered streams are shared within the original
charged lifetime; incompatible order or aggregate state uses bounded separate execution.

Routine reads trust acknowledged store-owned immutability, while still admitting current roles,
leases, capabilities and capacity. They do not promise discovery of arbitrary superuser tampering.
Explicit read-only generation audit recomputes content and semantic obligations. Repair quiesces
readers and rebuilds with new authority; there are no old-format readers or compatibility proofs.
Failures, partial execution, cancellation and uncertain commits create no reusable success.

Verification selects model, analytics, provider, store, serving, oracle and tooling families explicitly.
Compatible read-only cases share immutable seed preparation inside one test process; mutable cases
remain isolated and assertions run fresh. Readiness prepares only declared Python member/group
closure, once before execution; tests use `--no-sync`. Observation generators, provider parity,
independent semantic challenges and transport controls retain their distinct claims.

During implementation run compile checks and focused contract controls. At functional completion
run affected controls and applicable non-functional leaves. This shared-contract pivot requires
one assembled `just qualify` run, including real disposable PG/native/MCP and full keep-going
Clippy. The launcher collects independent failures and blocks only families with failed prerequisites.
Empty required selections fail. Superseded broad gate aliases are removed with their consumers.

There are no hooks: end-of-turn hooks were tried and removed, since a report-only background job
needed a script, a lock and operator plumbing to run a few recipes agents can run themselves. The
root agent runs `just turn-end` (ADR index, build-features, fmt) at the end of a turn that changed
files, and `just ready` (skills-sync, images, tools) after an environment change. Agents fix
failures in their selected scope and rerun failed checks; they never format or run generators
mid-work.

## Consequences

Repeated acknowledged questions avoid replay and rehash. Explicit audit owns privileged corruption
challenges. Proof correctness now depends on enforcing immutable frames and exact dependencies,
which require revealing writer, prefix, failure and admission controls before qualification.

Current Q0/Q1 and Phase 5 fixture guarantees move to required replacement families, staying open
until actual acceptance passes. Real-library activation, live vectors, product comparisons and
heldout remain stopped. No speed or RSS improvement is claimed without comparable measurements.
Optional measurements do not block functional completion. The coordinator records implementation,
qualification and remaining limits; accepted decision status alone closes no finding.

## Amendments

- 2026-10-07 (pointer, [ADR-0134](0134-scoped-preparation-and-maintenance.md); approved by the operator as plan OD1): ADR-0134 now governs two clauses of this record:
  - "Readiness prepares only declared Python member/group closure, once before execution." Readiness observes instead, and preparation happens through the explicit, scoped `just sync` routes.
  - The whole-tree `just turn-end` duty. Maintenance may instead be scoped to a turn's paths when whole-tree maintenance would rewrite another agent's uncommitted work.

  Every other decision here remains in force.
