---
id: ADR-0089
title: Assemble attributed facts through declared stage contributions over a captured input closure
status: accepted
date: 2026-09-29
supersedes: []
superseded-by: null
design: [§15.3, §15.4, §15.11]
evidence: Proposed
---

## Context

The operator accepted the detailed phase 0–2 execution on 2026-09-29
([cutover plan §4.1.1](../plans/semantic-model-cutover-plan_2026-09-29.md#411-detailed-remaining-execution-order),
decisions T1, T3, T6 and T7). Producer design for P2 exposed four contract gaps in the typed model
of ADR-0085 and the generation store of ADR-0086:

- Several providers emit identical shared vocabulary (evidence, qualifications, conditions, places),
  but §15.11 allows one writer per relation, and PostgreSQL keys refuse a repeated row at COPY.
- Reaching flow requires a use and all its reaching definitions to share one place; a place rooted
  at one binding occurrence cannot express a local variable with several definitions.
  `ProviderSymbol.module` and `TypeVariable.module` were display strings, so an acquired module,
  a provider-bundled stub and an unresolved import of the same spelling were indistinguishable.
- Acquisition read an environment it did not fully capture and wrote derived Markdown code blocks
  into the source tree it had just hashed.
- Provider observations attach to Pyrefly/Ruff occurrences by span; nothing defined what a
  non-exact match produces.

## Options

1. **One terminal assembler writes every facts relation.** Simple ownership, but it holds all
   provider output in memory until the end and loses per-provider stage receipts and outcomes.
2. **Declared stage contributions (chosen).** A stage declares relations it contributes to; the
   relation's single writer is scheduled after its contributors and deduplicates across batches.
3. Keep string modules and occurrence-rooted locals, resolving them in normalization. That leaves
   L0 facts unable to state which module or variable a provider meant, and moves an identity
   question into a consumer.
4. Capture only first-party sources and trust the installed environment. Smaller stores, but the
   analyzer input is no longer the captured evidence (CI-10).

## Decision

- **Contributions.** `Stage.contributes` declares handoffs into another stage's output. The schedule
  refuses self-contribution, orders the writer after every contributor and includes contributions in
  its digest. Contributed rows travel as attempt-owned, budget-reserved batches readable only through
  read permits; the writer emits each identity once and refuses conflicting payloads.
- **Provider modules and local places.** `ProviderModule` is a tagged sum: `Acquired` names the typed
  module over captured bytes, `Bundled` a provider-scoped stub, `Unresolved` the provider's spelling in
  one context. Symbols and type variables reference it; a stored invariant binds bundled and unresolved
  modules to their provider (and context). `PlaceRoot::Local { scope, name }` roots a local variable at
  its scope-opening occurrence; expression values keep `PlaceRoot::Occurrence`.
- **Captured closure.** Acquisition captures the complete analyzer-readable input: site-packages
  sources, stubs and `.pth`/`py.typed` files, dist-info metadata and records, and the selected corpus
  files. Derived artifacts (Markdown Python blocks, task receipts) are written only into a reserved
  `_lctx/` namespace of the frozen capture, with typed provenance. The source tree is never written;
  a stale `_lctx_blocks/` from the retired compiler is refused.
- **Exact attachment.** A provider observation attaches to an occurrence only on an exact source, span,
  syntax kind and role match. Every other outcome records a subject boundary with its candidates and
  Partial coverage (`AttachmentAmbiguous` or `AttachmentUnmatched`); facts depending on an unattached
  event are counted, never guessed.

## Consequences

Providers stream independently while shared vocabulary keeps one owner. Local-variable and module
identity become explicit L0 facts, so normalization no longer interprets spellings. Captured stores
grow with the dependency closure. Attachment misses surface as coverage rather than silent gaps.

Implemented and focused-tested on 2026-09-29: `ProviderModule`, its owner invariant and
`PlaceRoot::Local` (K1). Stage contributions (D0), capture (A1–A2) and attachment (A3, B1–B2) remain
open in the cutover plan, which owns their verification. Revisit if measured capture size or
contribution buffering requires a different physical mechanism; any replacement keeps one writer per
relation and the captured-input boundary.

## Amendments

- 2026-09-29 (plan A2): a distribution's `RECORD` is verification evidence, not analyzer input. Its
  console-script lines carry the environment's location, so capturing its bytes would make the
  input depend on where the environment sits. Acquisition verifies every captured byte a `RECORD`
  lists against the frozen copy and records the `RECORD`'s in-site entries as the verification's
  digest; `METADATA` and `entry_points.txt` are captured. The decision is unchanged.
