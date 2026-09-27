---
id: ADR-0060
title: Activate authored actions only at an independently proved callee trigger
status: accepted
date: 2026-09-27
supersedes: []
superseded-by: null
design: [§9.9]
evidence: Proposed
revisit: A supported action requires a trigger that cannot be expressed as invocation or an independently proved outcome of the exact callee.
---

## Context

ADR-0058 requires effects before failed returns to consume reached-prefix evidence. The current
call-execution contract proves invocation independently of normal completion. Callback/resource
rules already name an exit; effects do not. Activating every candidate at invocation would lose
normal-outcome promises such as returned-resource acquisition and callback registration. Requiring
normal return for every effect would lose partial I/O before failure. The same rule must govern
analytics composition, publication and native admission under [§9.9](../design/sections/behavioral-analysis.md#section-9-9).

## Options

1. **Normal return for every action.** Minimal reuse, but cannot express partial effects before
   failure and conflates an action with successful completion of the whole call.
2. **Invocation for every action.** Uses the new reached-prefix contract, but falsely promotes
   normal-exit promises and returned resources before they exist.
3. **One authored trigger with independent proof (chosen).** Extend the existing exit selector
   to all action channels and append Invocation without renumbering. Pure composition and one
   schema-owned admission contract retain rule meaning and modality. No runtime interpreter,
   model-name switch or separate serving policy is introduced.

## Decision

Each effect, callback and resource rule authors a trigger. Invocation means successful callee
and argument evaluation followed by entry to the exact bound callee. Normal and Exceptional
select that callee's independently proved outcome. Finally means either proved callee exit;
it never means invocation, an enclosing finalizer, or the caller's pending outcome. Missing or
unsupported trigger evidence yields an explicitly unresolved candidate. Models never weaken a
Normal promise to Invocation merely to activate it.

An established invocation activates a Potential rule only as a model-qualified potential action.
It does not establish that an effect actually occurred. Rule definitions distinguish attempted or
partial action from completed results. Authored subjectless effects require no invented subject; an unresolved required subject stays unknown.
Argument subjects may be available at Invocation. A
returned resource requires Normal and separate returned-value identity; a call-expression id is
never a runtime resource identity. Stored, Registered, Forwarded and Invoked callbacks stay distinct.

Schema owns selector meanings and shared structural admission. Analytics composes candidate,
phase, condition, exact source binding, invocation and outcome evidence; core acquires, publishes
and reconstructs those inputs. Native consumers use the same contract. Initial normal triggers
reuse existing whole-expression normal evidence; no new Boolean callee-outcome assumptions or
general precondition interpreter are added. Potential exceptions alone cannot supply an
Exceptional or Finally witness. Coverage remains independent of positive action witnesses.

## Consequences

This elaborates ADR-0058 without superseding it. Implementation and integrated qualification are
**Proposed**. Catalog and schema migrations, positive/withholding source controls, mutation checks
and independent partial-I/O challenges precede closure. Unproved registration/acquisition remain
candidates even when invocation is proved. [Plan §3.0 S1–S3/S6](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns execution; §6 owns finding disposition. Broader resource identity, all-channel propagation
and serving remain part of the accepted Stage 3 scope.
