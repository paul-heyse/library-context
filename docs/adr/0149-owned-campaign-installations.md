---
id: ADR-0149
title: Isolate native qualification through checked campaign installations
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: [§8]
evidence: Interface-checked
revisit: A new validation campaign requires lifecycle semantics the existing installation owner cannot provide.
---

## Context

The efficiency-alignment track changes native formats, transport drainage and service recovery while
another track's shared validation installation is paused with retained obligations. The operator
selected one dedicated campaign installation, with affected functional acceptance. Shared service
maintenance is outside this campaign's authority. [The active plan](../plans/efficiency-alignment-plan_2026-10-11.md)
owns progress and findings; acceptance of this decision does not establish package closure.

## Options

1. **Reuse the shared installation.** It requires coordinating disruptive transitions with paused
   qualification and couples independent recovery obligations. This does not meet the selected isolation.
2. **Own a process-backed substitute.** It provides isolation but adds another process, cross-command
   restart, attachment, backup and recovery owner alongside established service machinery.
3. **Select a checked campaign installation through the existing owner.** It adds one explicit install
   mode and derives the unit from installation identity. All later operations retain the same admission,
   borrower, maintenance, recovery and immutable executable contracts.

## Decision

Use option 3. `just service install --campaign` requires an explicit separate
`LCTX_SURREAL_SERVICE_CONFIG`. The descriptor distinguishes campaign from stable installation and
owns a unique systemd unit derived from its UUID; arbitrary unit overrides are rejected. Stable
installation remains the default, with its fixed unit. A mode mismatch cannot convert an existing
installation. Daemon checks, fixture receipts, status, backup assets, restart and native/server
handoffs all consume the descriptor's checked unit.

One campaign service contains the existing main/validation scopes. Tests use logical attempts under
real borrower/attachment ownership; no service or database per case. Ordinary verification observes
and borrows only: install, migration and maintenance remain explicit. Storage admission covers
installation creation and owned executable transfer; campaign content retains its named evidence
consumer. Shared and operator installations are never adopted or modified by campaign selection.

## Consequences

Independent native migrations and disconnect/restart controls can reuse production ownership without
reimplementing service management. The campaign still needs a real daemon, immutable producer/recovery
inputs and actual functional controls. Script controls establish descriptor routing only; they do not
qualify native drainage, persistence or migration. No host resource caps, shared-service promotion,
automatic retirement or historical compatibility readers follow from this decision.
