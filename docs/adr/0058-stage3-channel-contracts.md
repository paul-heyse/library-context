---
id: ADR-0058
title: Represent Stage 3 channel subjects and proof obligations explicitly
status: accepted
date: 2026-09-26
supersedes: []
superseded-by: null
design: [§B5, §B6, §9.6, §9.9, §11.3]
evidence: Proposed
revisit: A supported channel cannot express its subject, exhaustive empty coverage or cited proof without an invented value-flow identity.
---

## Context

ADR-0057 accepts shared Stage 3 composition. The value-only coverage contract requires a
parameter and raw value-flow origin, and requires a positive witness for completeness. Effects
before an exception, parameterless operations and exhaustive empty channels cannot honestly use
that shape. Native admission still interprets proof adjacency independently. The operator approved
the detailed remaining-stage execution plan on 2026-09-26, including the triggered transfer and
current RCA fidelity repairs, while retaining the Stage 5 execution boundary.

## Options

1. **Extend value-shaped rows and existing seed/native classifiers.** Smallest immediate edit,
   but forces fabricated value identifiers or repeated rules for other channels and makes empty
   coverage indistinguishable from missing analysis.
2. **Typed subjects, structural proof obligations and independent coverage (chosen).** Existing
   modules retain their responsibilities; mechanical projections map to schema-owned meaning.
   Pure consumers can be challenged without a store or server.
3. **A universal interpreter, proof engine or provider framework.** Adds representation and
   lifecycle machinery without supplying the missing Python semantic evidence.

## Decision

`cpg-schema` owns typed channel subjects and origins, phase applicability, proof obligations and
coverage invariants. Coverage concerns a declared subject, condition, channel and invocation
phase. Semantic completeness, positive witnesses, unresolved alternatives and omitted witnesses
are separate. Complete empty domains require a coverage certificate; a positive witness alone
never certifies completeness. Each subject variant has checked references to its actual source
relations, rather than borrowed parameter/return identifiers.

Analytics owns pure evaluation, completion, binding and composition; core acquires and publishes
relations; the native executor consumes the same proof admission. A new semantic category may
change the contract deliberately. An ordinary model using existing categories adds a declaration
and focused controls, not another source/native interpreter.

Models declare supported invocation phases independently of provider call observations and channel
coverage. Dynamic validation schemas have distinct statically resolved, runtime-identified and
unresolved representations; a class name is not an instance schema. Effects can precede a failed
return and therefore use reached-prefix evidence rather than requiring a value-return summary.

Published transfer projections preserve kind/condition alternatives. Current RCA attributes retain
typed relation, modality, target and evidence through analysis; labels are derived presentation.
These repairs apply to existing consumers, not new Stage 4 registry or behavioral FCA features.

FORMAT 9 carries the expanded structural support. Rust owns bounded semantic query interpretation;
Python owns protocol adaptation. Negative exclusion requires the complete claim-specific domain,
including open and unexamined alternatives. Path inspection remains path-local. Rebuild current
stores/generations under ADR-0048; no historical-format reader is required.

## Consequences

The target is **Proposed implementation**, not completed by accepting this decision. The
[forward plan §3.0](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns execution and §6 owns finding disposition. Schema migrations, real source/publication/native
controls and independent semantic challenges precede integrated qualification. Existing narrow
admission paths are deleted as consumers migrate. Stage 5 deferred execution, general alias
analysis and protocol interpretation remain outside this scope. The multi-channel engine
comparison uses the actual semantic/refusal contract; ADR-0053 remains the scheduler default.
