---
id: ADR-0189
title: Qualify original layouts by model while sharing content-addressed bytes
status: accepted
date: 2026-10-17
supersedes: []
superseded-by: null
design: [section-6-1, section-6-2]
evidence: Interface-checked
---

## Context

The original layout was addressed by nominal Source EntityId while its immutable `artifact`
field named a model-qualified entity payload. Read-only validation inspection established two
payloads with identical canonical source, body, content and anchor but different physical IDs.
The existing original layout named the earlier payload. A current acquire therefore requested a
different full layout at the same address and correctly hit the immutable collision guard.
The SourceArtifact batch phase includes original ingress after its graph write; identifying the
batch did not establish that the graph rows themselves differed.

ADRs0145/0187 preserve nominal semantics separately from model-qualified immutable physical
content. This corrects original-layout realization within that contract. No user-facing Source ID,
original manifest identity, source fact or byte-content meaning changes.

## Options

1. **Qualify physical original layouts by model/codec and nominal source.** Callers that already
   hold a logical Source EntityId derive its exact current physical layout without hydrating source
   content. Immutable artifact reference, independent source validation and current ownership stay
   explicit. Each model has small layout/hold metadata; identical blob/chunk bytes remain shared.
2. **Link a stable layout to a nominal anchor.** This would change its exact payload authority and
   physical field contract, requiring new source resolution and schema rules. It does not provide
   a necessary advantage for the current immutable model-qualified realization.
3. **Overwrite the old layout, weaken equality or retain nominal fallback.** Reject: these hide a
   real identity conflict and mix different physical model promises. Regenerating data alone would
   defer the same structural defect until the next model change.

## Decision

`lctx-native::originals::layout_id` owns one model/codec-qualified physical address from the
nominal Source EntityId. Use it for creation, chunk metadata, pending ingress, byte reads,
completion, installed audits, reconciliation, selected backup and restore. Keep public Source IDs,
manifest originals and export filenames nominal. Cold restore derives that nominal identity from
its independently decoded source payload, never by reversing the physical layout hash.

The layout continues to reference the exact model-qualified Source payload. Blob/chunk identities
remain content-based. Full layout, source, content, length, holds and immutable collision checks
remain strict. No old-address reader, compatibility copy, data rewrite or schema change is added.
Existing old objects with unresolved native owners remain protected for those actual consumers.
New current-model compilation creates current layouts through its ordinary owner.

Ingress source resolution considers indexed anchor candidates' physical ID and full-content digest
before canonical decoding. Recompute the expected current-model payload address and exclude
foreign-model candidates first; then retain exact type/source/content/length checks and refusal for
multiple conflicting current candidates. Neither first/latest choice nor a stale-layout fallback
can establish a source's authority.

## Consequences

Changing a model no longer asks one immutable layout address to name two physical payloads.
Small model-specific metadata coexists while unchanged original bytes remain reusable. This is
an address cutover: new code resolves only qualified layouts; retained old obligations are not
settled by the cutover. Existing publication/import admission still requires compatible model and
exact payload authority.

Independent source advice supports this correction at Interface-checked strength. Implementation,
finite identity/candidate controls, actual current-owner reuse/original ingress and genuine Summary
verification remain separately recorded in upper-execution coordinator §6. No measured speedup,
historical timeout diagnosis, full Catalog qualification or operator adoption follows. Reopen for
independently selected older-model payload authority or a need to share layout metadata across
models; either requires a complete declared model/codec source context, not an unqualified alias.
