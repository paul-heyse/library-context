---
id: ADR-0035
title: Bind pinned object-method formals with cited implicit receivers
status: superseded
date: 2026-09-25
supersedes: []
superseded-by: [ADR-0045]
design: [§9.9]
evidence: Tested
revisit: A bound-method fixture maps the wrong positional formal, a class receiver needs the same contract, or the pinned Pysa receiver code no longer distinguishes object receivers.
---

## Context

The first model formal binder (§9.9) withheld every implicit receiver. That was safe but left
`logging.Logger.warning(msg)` unable to cite its explicit message even when Pysa selected the
pinned method and said the object receiver was implicit. The Stage 3 logging effect needs the
source argument, without relabeling the receiver as `msg` or treating a dynamic logger call as
the pinned target (CI-02, CI-05, CI-07).

## Options

1. **Keep all implicit receivers unknown.** This preserves the existing safe boundary but
   discards a common, source-attributed method formal despite enough evidence to account for
   the receiver's positional slot.
2. **Shift every method's positional arguments by one.** This is compact but confuses object,
   class and static method calls, and can silently bind a different formal.
3. **Shift only a cited object-bound direct attribute call.** Chosen. Pysa owns receiver kind;
   Ruff syntax owns the direct callee shape; the pinned signature owns formal ordinals.

## Decision

For an applied pinned model, `model_argument_bindings` subtracts one from a positional
signature formal's ordinal only when the Pysa target row says
`true_with_object_receiver` **and** the call's Ruff callee is an attribute expression.
Keywords continue to bind by exact formal name. Every pinned signature must select the
same explicit argument and no `*` or `**` unpacking may occur. Class receivers, unsupported
callee shapes and ambiguous signatures retain `unknown` with a boundary. The candidate
model's target and modality remain separate; this argument bridge does not establish a log
emission or a completed call.

DataFusion composes a distinct set of Ruff attribute callees into the existing binding
relation. The shared publication validator reconstructs the relation. A focused analyzed
fixture verifies the Pysa object-receiver code and bound `msg`, and withholds an untyped
logger target and an unpacked message argument. This is **Tested** on 2026-09-25;
integrated Stage 3 acceptance remains `not_run`.

## Consequences

Pinned method models can now attribute explicit parameter effects without inventing a
receiver value. Future classmethod and descriptor behavior still needs a separate proof;
the object-receiver rule must not be generalized from spelling or from a method-shaped
syntax node alone. A later Pysa or pinned-signature change requires the focused receiver
fixture and publication validator to be rerun before transferring the decision.
