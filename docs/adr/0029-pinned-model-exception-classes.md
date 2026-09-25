---
id: ADR-0029
title: Resolve modeled exception classes in the pinned context before source application
status: superseded
date: 2026-09-25
supersedes: []
superseded-by: [ADR-0045]
design: [§B5, §3.2, §4.0, §9.9]
evidence: Tested
revisit: An applicable exception model names a class in a context module the extractor does not describe, or a Stage 3 handler match needs a class relationship absent from the pinned context facts.
---

## Context

Stage 3 can apply a model exception rule to a source call, but `model_exceptions.class` was
only a dotted display string. `handler_types` already cites a pinned context class identity.
Joining those channels by text would turn spelling into authority (DESIGN §B5, §9.9; DP-02,
CI-02). The context extractor normally retains definitions referenced by the analyzed source or
its exports; `builtins.OSError`, authored by the `builtins.open` model, was absent from a focused
`model_shapes` extraction even though `open` itself was pinned. Merely changing the model compiler
to require the class therefore failed before publication. Context capture, compiler validation
and producer identity must change together (§3.2, §4.0).

## Options

1. **Keep dotted class names until summary time.** No extraction change, but two independently
   sourced names could be equated without proof, and an unresolved name could masquerade as a
   caught exception. This cannot support the L2 handler contract.
2. **Resolve names ad hoc in each L2 consumer.** A caller could inspect source or the context
   module on demand, but this creates repeated resolution semantics and no shared publication
   check.
3. **Retain model-named classes as pinned context definitions and bind them at model compilation.**
   Chosen. Only a unique context class with the exact module and qualified name is admissible;
   an applicable rule with no such definition fails before publication. A model with no bound
   target stays dormant. Source application carries the class node and fact ids along with the
   display name. Candidate openness and modality stay separate from exception occurrence.

## Decision

The extractor retains class definitions named by committed exception rules in context modules
it already describes. It treats the committed catalog digest as part of its producer build
identity, so changed catalog bytes cannot reuse a run id whose context facts meant something
else. The model compiler binds each authored source and conversion class to a unique pinned
`context_definitions` class row before writing `model_exceptions`. It refuses a referenced,
unresolved or ambiguous class; it does not guess by name. `modeled_exception_sites` attaches that
identity to each resolved source call candidate and keeps the target/model modality and unresolved
remainder. The shared publication validator reconstructs both model and site rows and checks
the class references. Neither row says that the exception occurred, escaped or was caught.

## Consequences

Handler matching can compare exact class identities with cited source and model facts. A model
class in a different, unvisited context module is now a visible compile failure; the next step
would resolve and describe that module through Pyrefly's pinned import context, not downgrade to
a text match. Adding model classes to the extraction inputs increases context rows and makes
catalog edits move producer ids. The extractor output version and Arrow schema move explicitly;
the Stage 3 pilot must measure the resulting context and candidate-site counts. A pinned class
identity alone still does not prove subtype catch, conversion, normal completion or a whole
operation's exception fate.
