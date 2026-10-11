---
id: ADR-0150
title: Use exact eligible vector ranking while preserving contextual evidence and value contracts
status: accepted
date: 2026-10-11
supersedes: [ADR-0131]
superseded-by: null
design: [§B12, §B13, §B14, §6.4, §6.5, §11, §11.1, §11.2, §11.4, §14.3, §14.4, §14.7, §14.8, §14.9, §14.10, §14.13, §15]
evidence: Interface-checked
revisit: A supported product task requires approximate retrieval with an explicitly accepted recall policy and complete consumer migration.
---

## Context

The reconciled efficiency review and audit identify a mismatch between the approved exact frozen
ranking policy, current exact serving implementation and older HNSW policy/index/reference consumers.
The operator approved RC10 and S01 in the [efficiency coordinator](../plans/efficiency-alignment-plan_2026-10-11.md).
This record changes that ranking clause of ADR-0131 and carries its remaining semantic, embedding,
contextual delivery, deployment and cutover obligations below. It does not adopt older plans or
claim their qualification. Current native/code inspection establishes the interface boundary;
functional exact-reference, consumer migration and index retirement acceptance remain in S01/N08.

## Options

1. Keep exact serving and maintain unused HNSW indexes and ANN references. This preserves a separate
   approximation experiment but leaves physical write/rebuild work and conflicting product policy
   without a current product consumer.
2. Switch production back to HNSW. This reduces some large-cohort scoring work but changes candidate
   completeness and failure behavior, requires a recall policy, and is not the approved product.
3. Use exact eligible selection consistently (selected). Work follows the protected selected scope;
   deterministic identity ties and independent finite references remain revealing. Large scopes may
   still require substantial scoring, which is accepted without an unmeasured latency claim. Maintain
   physical scalar/context indexes and retire HNSW only after all required consumers migrate.

This follows the design principles and efficient-architecture guidance on promised behavior,
selective scope, independent assurance and removing unused physical maintenance. No benchmark,
new score ledger, scheduler or second vector service is introduced.

## Decision

Rust typed source/catalog/association owners retain facts and qualification. Retrieval construction
owns roots, primary/context parts, original maps, semantic windows, exact/candidate bindings and
interpretation dependencies. Defining source differs from public access provenance; context-only
mentions/setup cannot nominate targets as exact primary evidence. Source/non-callable/ambiguous,
option, document and release material remain independently available.

Use local matching tokenizer assets for complete-input partition/admission. Prefer1024 tokens,
hard normal2048 including context/headers/special tokens, without truncation. Oversized indivisible
information stays explicitly lexical-only/partial-vector and expandable. No larger admission follows
from vector dimension. Mechanical declarations/codecs preserve registered meanings; publisher
lowers bindings rather than inferring applicability from parent proximity.

The model embedding owner separates encoder/input, rendering, query, projection and consumer
policy dependencies. Store normalized full4096 F32 winning bytes once per actual encoder/input.
Derive initial1024 prefix projections with F64 norm accumulation, one F32 rounding and shared
validation. Both exact initial vector selection and E1 analytics consume that explicit projection; full4096 rescoring
and offline references use full values. E1 methods/results carry representation policy and exact
consumption. Query-only changes do not re-encode unchanged documents. Projection changes require
no inference; encoder/input changes do. Endpoint-reduced parity is not assumed bitwise.

The selected custom SM120 gpu-stack vLLM build and NVFP4-r2 Qwen checkpoint remain the initial
encoder, acquired by existing locks. The checkpoint/tokenizer published manifest and all listed
files are verified before loading; local paths are deployment details. No service/dependency/model
upgrade is authorized by this record. Replace launcher/pooler admission, both clients, receipts,
cache/schema/transport and all E1/native consumers before enabling full output. Existing1024 values
cannot supply missing components. No BF16-equivalence, quality or throughput claim is made.

Native SurrealDB remains the host. Use exact identifiers, code-aware lexical match eligibility
including zero-BM25 matches, exact eligible vector scoring and useful scoped native expansion.
Contextual best family/channel contributions and compatible bundles replace indiscriminate member
collapse; initial RRF K60/equal-family weights and canonical ties remain explicit. Full rescore
cannot recover missing candidates. Eligibility precedes all target/context caps. Exact selected
document scoring uses the declared normalized1024 representation, deterministic score/identity
ordering, contextual witness selection and full4096 rescoring of the resulting union. Shared
cohorts cannot supply unselected candidates. Supported predicates choose indexed scalar/bitmap
and bounded exact routes without adding a work-accounting layer or second vector database.
Optional scorers are non-generative and subordinate to evidence validity.

Production evidence demand does not contain private expectations. Final packets preserve mandatory
identity/signatures/qualification and describe actual delivered fields/spans and bounded expansion.
Opaque IDs do not establish readable information. References and continuations pin one complete
realization and answer-affecting policy/order; ranked-result retention has a bounded current-query
consumer, not a permanent query history.

**Semantic guarantees carried from ADR-0077/0078.** Selection is model-owned and retains discovery
versus strict modes, separate supported/unresolved/conflicting groups, comparable-context conflicts,
non-vacuous complete-empty contradiction and incomplete absence as unknown. Contradictions cannot
be hidden by search. Public exposure, declaration, variant, operation and instance identities remain
distinct; ambiguous lookups return choices. Authored/setup/expected-failure evidence is not executed
success. Optional capabilities keep NotRequested/empty/unavailable/failure distinctions; selected
compiler failures cannot be ignored and brief grounding remains strict. Canonical source bytes,
consumer values, scope/coverage and complete nominal closure participate in identity.

**Current-only lifecycle carried from ADR-0078.** Catalog remains default; behavioral enrichment is
explicit and does not restrict mandatory public/evidence roots. Reconstruct current formats from
pinned inputs. Validate the fresh replacement, quiesce affected project readers/writers, replace
state, then remove obsolete runtime/code/generations/rollback assets. No old-format reader, ID bridge,
dual write or historical-runtime packaging. Unrelated workspace/source inputs remain preserved.
Current comparison/receipt artifacts survive only while a named consumer needs them; Git holds history.
Backup/restore tests exercise the current format, re-admit canonical content and rebuild derivatives.

ADR-0128 owns store-free compilation and native publication/selection, SDK, roles, reconciliation,
atomic visibility and viewer pinning. Query-time compilation or source/library execution stays
excluded. Effects use the shared process runtimes, bounded admission/deadlines/drain and explicit
availability; cache winners are exact insert-only values consumed once per attempt, never mutable
reconstruction inputs. Rust-generated wire/schema contracts keep Python thin; read-only annotations,
masked safe errors, actual final envelopes and clean stdio remain. Changed interfaces migrate all
callers without compatibility adapters. Native federation/operational projections retain their
named contracts; no new ADBC/SQL-wire/network fallback is selected. Editable native development
needs no matching wheel gate. Gold/frozen confirmation remain protected under ADR-0130.


**Exact reference and physical cutover.** The private numeric worker independently checks stored
full4096 F32 values, F64 scalar arithmetic, one-rounding normalized1024 projections and exhaustive
eligible cohorts of at most256. Native exact observations cannot supply their own expected order.
Compare actual serving candidates separately against finite independently computed eligibility,
scores and ties. Preserve missing/corrupt-value refusal and exact context/alias/binding/support
meaning. Migrate CLI/evaluator/report/script consumers from `native_ann` to `native_exact` before
removing HNSW DDL. Advance native derivation/index identities and rebuild through the coherent
schema transition; no marker relabel, retained incompatible shortcut or old-format reader.
Model/provider exact analytical neighbor APIs keep their own method contracts.

## Consequences

The accepted decision is in force; it does not close a source finding. S01/N08 and the coordinator
own exact reference, actual served output, index/restore and integrated acceptance. Current policy
inspection is Interface-checked. No recall, speed, quality, endpoint parity, real-library or operator
adoption claim follows. All current context/evidence/value and lifecycle guarantees carried above
remain required; only approximate HNSW initial retrieval is rejected.
