---
id: ADR-0174
title: Prepare exact catalog inputs privately and index shared lexical postings
status: accepted
date: 2026-10-15
supersedes: []
superseded-by: null
design: ["§6.2", "§11.2", "§15.11"]
evidence: Proposed
revisit: Residual flat selector emission becomes material, an actual immutable capture owner exists, a substantive early consumer can earn complete authority, or postings cannot qualify both indexed dimensions.
---

## Context

The [efficiency review I01–I04/I06–I07](../design_review/reviews/design_review_catalog-completion-efficiency-target_2026-10-15.md)
asks whether current private preparation follows actual consumers without changing the catalog's
meaning. The [efficiency coordinator §6](../plans/catalog-efficiency-plan_2026-10-15.md#6-sole-disposition-and-current-checkpoint)
owns the approved CN06/CN07/CC00–CC03 execution and acceptance. This refines ADR-0166/0168/0173;
codec1, semantic v4, selected-port semantics and scoring policy remain in force.

## Options

1. Keep flat selectors, gate-held eager composition, repeated original reference scans and full
   document term arrays. This is viable but repeated copying/scanning and unrelated gate exclusion
   follow physical containers rather than the data each consumer needs.
2. Share private protected handles and invocation preparation, prepare exact unions outside the
   visibility gate, and index immutable document/term postings. Selected. Existing owner/resource,
   Arrow IPC/spill, immutable ingress and compound-index facilities provide the complete route.
3. Persist selector fragments, use lazy visible unions, introduce immutable filesystem/VFS capture,
   cache rich Catalog correspondence across windows, or use global native BM25. These respectively
   add receiving/GC, change exact readiness, require a new complete capture owner, extend backing and
   invalidation scope, or change selected-corpus scoring. None has a justified simpler complete route
   for this delivery. Their useful benefits are retained as specific revisit triggers below.

## Decision

**Accepted target, 2026-10-15.** Keep flat root-qualified published mappings. Private async consumers
share the actual charged live mapping owner or protected installed descriptor; synchronous consumers
borrow slices. Materialize the public vector once at its real boundary. Metadata never supplies pins,
originals, incarnations, current ownership or receiving success. Persisted fragments remain unselected
unless residual flat emission/reconstruction warrants their additional construction/receiving/retirement.

Keep eager exact completed visibility. Prepare each affected relation's union, exact count/full-payload
conflict result and host carrier privately over captured predecessors. Under the existing gate,
recheck cancellation/writability, duplicate owners and exact affected predecessors. Reprepare only
changed relations outside the gate, retaining unaffected preparation. Bind all final outputs before
coherent host visibility. Failed/Unknown submitted work is terminal and drained, never replayed as a
stale miss. Full-payload conflicts remain monotone under additive contribution completion. Old selected
ports remain immutable; final freeze and admission keep their separate authority.

Retain Facts/Normalized admission barriers. Six input-free configuration producers can legally compute
earlier, but current admission includes completed output inventory: moving them also requires a new
admission frontier and overlapping lifecycle. Their small private work supplies no substantive early
consumer benefit. Reopen for a material consumer with a complete earlier input/admission slice; do not
infer that no early computation is legal.

Retain full captured-tree opening and post-provider membership/content verification. The whole revision
includes Python dependencies, support, documentation, examples and configuration. Same-process writes
remain possible; readonly modes and consumed-file subsets do not establish the promised immutable
closure. Successful provider work closes only after this verification, before final flush/outcome.
Acquisition/Assembly keep their explicit existing exclusions. A digest states a check's scope/time,
not protection between checks. Reopen when an actual immutable owner covers full bytes, membership
and lifetime, or a genuinely narrower provider contract is adopted.

Carry one admitted complete-key optional Catalog request through lookup, computation and retention.
Disabled caching constructs none; no-reuse eligibility can still retain a fresh result. If optional
ownership prevents a pure preparation's resource admission, release it, disable retention for that
window and retry only that failing pure step once. Keep already-computed rows and preflight output
headroom before effects. Preserve current window-local borrowed correspondence and full keys; neither
whole-release rich caching nor signature-only reuse supplies the same context/witness contract.

Independent reference admission lazily captures each needed original input once into invocation-owned
schema-checked Arrow IPC, sharing local scans and exact target-family ID/subtype projections. Full-row
conflicts use the existing null-sensitive typed pairwise SQL against local IPC: grouping all supported
nested/binary/null types has not earned an equivalent contract. Original owners remain live, current
checks remain mandatory even for empty/memo cases, complete EOF precedes success, and memo transfer is
failure-atomic. IPC is bounded spillable preparation, never completed authority.

Store one immutable posting per analyzed document/term, shared across selected scopes and source
occurrences. Store document length separately. Bounded candidate windows intersect document IDs and
distinct analyzed query terms through the `(in, term)` compound index; aggregate matched postings once
and map scores back to selected source documents. Selected-view/family N/df/length, analyzer, BM25,
zero-IDF matches, eligibility before caps, ties/fusion and winning witnesses retain their owners.
Actual EXPLAIN must establish both compound dimensions; an index name alone does not qualify access.

## Consequences

This removes duplicate private copies/key derivations/original scans and avoids inspecting unrelated
stored terms during scoring. It adds posting rows and index writes; frequencies already computed at
construction produce those rows, shared independently of corpus membership. Exact native and cold
reconciliation includes all five lexical families; generic immutable endpoint holds protect posting
parents and retirement. Canonical export/backup omits derived rows and independently reconstructs them.
Native document realization v2 and compiled definition/derivation identities change together. This is
a physical schema migration: regenerate owned targets, preserve no old-format reader or compatibility
runtime data. Operator/shared adoption remains CE80; fresh owned functional targets do not activate it.

Acceptance of this decision is not functional evidence. The coordinator owns source integration,
actual compound access/scoring/reconstruction/retirement controls, independent review and complete
compile/strict keep-going Clippy. CE90, old-plan reconciliation, embeddings, submitted MCP runtime and
real-library qualification remain separate. No timing or measured speed claim is made.
