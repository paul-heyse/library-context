---
id: ADR-0115
title: Capture declared embedded runtime scripts while retaining conservative semantic source roots
status: accepted
date: 2026-10-02
supersedes: []
superseded-by: null
design: [§4.0]
evidence: Implemented
---

## Context

The producer build fingerprint captures production source membership and bytes. Traversing the
whole scripts directory also invalidates a producer after unrelated administration edits.
The extraction adapter embeds the deployment-check runner; its bytes affect accepted evidence.
The [incremental alignment plan](../plans/semantic-model-incremental-alignment-plan_2026-10-02.md)
schedules O4/T1 to remove this incidental input without weakening semantic source capture.

## Options

1. **Keep the complete scripts subtree.** Conservative but couples producer identity to
   administration that does not participate in extraction.
2. **Maintain a separate fingerprint allowlist.** Smaller capture, but embedding and capture
   can drift independently.
3. **Share an explicit runtime-script declaration.** Both embedding and fingerprint capture
   expand the same paths; source membership outside scripts stays conservative. Selected.

## Decision

The extraction adapter owns one finite runtime-script declaration. Embedded script constants
and producer fingerprint traversal consume it mechanically. The declaration and fingerprint
helper are themselves captured. The deployment runner is currently the only embedded script;
its imports are standard-library modules and do not add sibling runtime files.

Retain all existing production crate, manifest, toolchain, specification and third-party roots.
This change does not narrow ordinary semantic source capture or infer dependencies by parsing
Rust source. A new embedded runner must enter the shared declaration; runtime sibling dependencies
must be added there after inspection.

## Consequences

Unrelated administration script edits no longer rotate producer identity. Runtime script bytes,
path membership, declaration edits and helper edits still do. Relocation and existing cache/test
exclusions remain unchanged. This is **Implemented** on 2026-10-02; focused mutation controls and
combined final-tree acceptance are recorded by the plan's sole disposition owner. Acceptance of
this decision alone does not close O4. Reopen if runtime acquisition no longer uses this finite
embedding boundary.
