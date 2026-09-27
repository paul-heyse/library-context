---
id: ADR-0063
title: Require a cleanup premise before promoting body return to callee completion
status: accepted
date: 2026-09-27
supersedes: []
superseded-by: null
design: [§B5, §9.9, §11.3]
evidence: Proposed
revisit: A supported body creates additional release roots or a consumer needs normal completion after user code can invalidate a caller retainer.
---

## Context

Stage 3 must compose source-callee completion independently of value transfer. A body return
does not yet establish caller continuation: CPython clears the frame first, potentially calling
finalizers for discarded arguments and locals. The existing unqualified `normal_return` model
assertions for `typing.cast` and `typing.assert_type` only justify their direct-return bodies.
The [bounded review F17](../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md)
and [independent generated controls](../design_review/evidence/2026-09-27_frame-exit/README.md)
establish the missing release premise. The controls observe delayed continuation, not a
reproduced false positive in the current narrow compiler input grammar.

## Options

1. **Withhold all modeled normal completion.** Sound, but discards independently provable
   literal and retained-argument cases and leaves the same missing premise for source functions.
2. **Two flags for normal body and no user code.** Small representation, but underspecifies
   implicit releases, internal locals and which returned value keeps its provenance.
3. **Typed body contract plus a shared release proof (chosen).** A direct-return-parameter
   contract describes the entire small body domain; expression provenance discharges each bound
   input separately. It composes with future source body certificates without a second interpreter.

## Decision

Replace unqualified `normal_return` with optional `normal_body =
{ kind = "direct_return_parameter", name = "val" }`. This authored pinned contract asserts an
eager function body that directly returns that bound parameter, creates no additional value
roots, runs no user code and cannot invalidate a caller retainer. It is independent of the
ordinary identity-if-return transfer rule and does not itself promise callee completion.

The pure expression owner distinguishes closed values, caller-retained values and unknown
release provenance. Closed primitives and recursively closed tuples are safe to release;
retained values are safe only over the exact invocation window whose body and later argument
evaluation cannot invalidate those retainers. Identity returns and selected expression branches
preserve provenance. A normal call requires binding, ordered completed inputs, the typed body
contract and an independently counted complete release-proof group before its frame-exit marker.
Omitted defaults require their own release premise even if invocation availability is established.
Missing release evidence retains a specific unknown; it does not erase the reached invocation
or a correctly outcome-qualified postcondition.

Schema owns typed meaning and shared group admission; analytics produces source-qualified
evidence; publication reconstructs it; native serving preserves the same obligations. Changed
bindings, reordered or missing members and deletion of the entire cleanup group must refuse.
Source bodies use the existing completion kernel plus independently closed release domains;
definition-header completion is never body completion. A first zero-formal, synchronous,
undecorated closed-expression domain can support literal branches and first local initialization.
General finalization, arbitrary user-code effects and closure lifetime analysis remain unknown.

## Consequences

The bounded target review accepted this contract on 2026-09-27; implementation remains open.
This corrects a model premise before widening its consumers. Catalog and compiler schemas
migrate; existing stores are rebuilt under ADR-0048. Original proof bounds remain unchanged.
Implementation and focused verification are **Proposed**; acceptance is not finding closure.
The [forward plan §3.0 and §6](../plans/behavioral-model-forward-plan_2026-09-24.md) owns remaining
source outcomes, model composition, native evidence closure and integrated Stage 3 acceptance.
