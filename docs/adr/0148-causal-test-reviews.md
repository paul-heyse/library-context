---
id: ADR-0148
title: Test reviews investigate causes across code, tests and execution conditions
status: accepted
date: 2026-10-11
supersedes: []
superseded-by: null
design: ["§2"]
evidence: Implemented
revisit: Repeated use reveals that the test-review route duplicates design-review judgment or imposes investigation/reporting ceremony without a useful consumer.
---

## Context

Native test-evidence access is implemented under ADR-0147. The operator requested companion
skills that turn test observations into a causal, structural review aligned with the design
principles. Investigation includes code review, probes, experiments and research; existing test
outputs do not delimit it. The operator explicitly chose flexible reporting and supporting
material according to reviewer judgment, without an evidence packet or prescribed inventory.

[DESIGN §2](../design/DESIGN.md#section-2) owns review policy; its binding owns report routes
and finding disposition. Existing design-review and plan-creation workflows remain available.

## Options

1. **Use design-review unchanged.** Reuses judgment and lifecycle, but its general architectural
   assessment/report contract does not directly guide causal investigation of stale tests,
   environment failures and incomplete execution. Routine test review need not claim architectural
   acceptance of an enclosing subsystem.
2. **Companion process skills and flexible report guidance (selected).** Reuse the principles,
   claim vocabulary and finding lifecycle, with reasoning targeted at test-triggered investigation.
   One planning entrypoint and one execution entrypoint match existing workflow pairs; the report
   guide belongs to the execution skill. This adds a small maintained surface without another
   test catalog, evidence store, disposition register or agent role.
3. **A new governed schema, evidence packet and failure registry.** Makes reporting more uniform
   but adds repeated transcription and lifecycle ownership without a requested consumer. The
   existing native readers and source-review/plan relationship already supply the needed routes.

## Decision

`plan-test-review` prepares an adaptable conversational inquiry. `execute-test-review` performs
useful source/contract review, probes, focused execution, comparisons and research, then writes
one integrated assessment with structural recommendations. Remediation remains separate work.
The execution skill owns flexible report guidance; the binding places reports in the existing
review collection with the `test_review_{slug}_{YYYY-MM-DD}.md` filename.

Test, production, environment and harness explanations are assessed independently and may coexist.
An accepted target does not establish that a replacement is implemented; a passing rerun does
not erase an earlier failure. A source-supported structural defect can warrant correction without
being the measured bottleneck. Recommendations use applicable design principles and library fit.

The reviewer chooses document structure and supporting material. Existing truthful claims and
calibration still apply; no evidence inventory, packet, fixed sequence or exhaustive failure table
is required. Stable findings and rule impacts feed the existing plan-creation/disposition route.
Formal design-review cadence and architectural acceptance obligations remain unchanged. A combined
report can serve both purposes when it satisfies the applicable obligations.

## Consequences

Agents have explicit reasoning support for causal test review across code, tests and execution
conditions, with direct use of the existing selective evidence tools and downstream planning.
The two skills and their report guidance are implemented, with routes in the skill index, binding
and agent instructions. Their behavioral quality needs judgment in use; mechanical validation
alone does not establish it. No quantitative effectiveness improvement is claimed.

This decision complements ADR-0040/0093/0130 and ADR-0147; it replaces no architectural criterion,
execution contract or supervision owner. Creating the skills does not resume paused product
testing or review its failure inventory. Scope-specific validation and current handoff belong
in STATUS; future review findings retain their own source and single disposition owner.
