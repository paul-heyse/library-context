# Writing the audit account

Use Markdown to explain the problem at the scale it deserves. Consistent content makes findings
comparable; identical headings, tables or paragraph counts are unnecessary. The binding locates the
living collection and the audit skill owns baseline and lifecycle behavior.

A useful finding conveys its stable ID and specific title, relevant principle IDs, diagnosis and
consequence, source evidence and known footprint, material uncertainty or counterevidence, and
links to related findings or subsequent work. A narrow finding may fit in a few paragraphs; a broad
finding may need examples, subsidiary discussion or a diagram. Show why manifestations belong
together without flattening distinct obligations.

Choose a primary home by structural cause. Component names describe the footprint, and principle
IDs express the criteria; neither requires a duplicate finding. Keep references stable when moving
text, and preserve predecessor identities when splitting or combining findings. Short links can
connect independently useful explanations without creating a finding graph or second register.

The following are illustrative shapes, not findings about the current repository.

## A targeted observation

> DA-001 — A presentation consumer independently interprets completion outcomes
>
> FP-02/04, DP-01/08. At the inspected baseline, the operation and its presentation consumer
> independently decide whether a partial result counts as complete. Cite both definitions and
> a legitimate outcome on which their meanings can diverge. Explain the consequence for another
> presentation consumer, even if the current happy path agrees. Identify what has been inspected
> and any unresolved reason the two interpretations might intentionally differ.

This explanation can establish a diagnosis while leaving the correction open. It need not propose
a registry, a new service or a particular library.

## A broad observation

> DA-002 — Execution boundaries recreate preparation for each item
>
> FP-03/07, DP-10/16. Explain the shared lifecycle choice, representative executing sites and
> the repeated preparation it causes across the inspected operations. Identify the contexts
> where independent preparation has a real purpose. Separate demonstrated manifestations from
> other consumers that may share the problem. Discuss their relevant growth and failure behavior
> without presenting an unmeasured speedup as a result.

A common cause can justify one broad explanation. Distinct causes or preservation obligations may
justify separate findings with a relationship explained in prose. The reviewer later determines
whether one correction serves them all.

## Uncertainty and follow-through

A suspected gap can be useful if it names the unsettled premise and why resolving it matters.
Do not describe absence of inspection as a violation. Existing design strengths and counterexamples
help bound a finding and prevent a later remedy from discarding useful behavior.

Solution leads remain proposals. When a review consumes the account, cite the audit's publication
revision and finding anchor so its argument survives future updates. Preserve audit IDs in subsequent
reasoning; additional review findings can retain their own identities without renaming an existing
diagnosis. Scheduled disposition and closure remain with the implementation plan.
