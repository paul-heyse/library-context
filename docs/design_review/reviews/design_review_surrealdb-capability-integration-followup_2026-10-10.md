# F01 selected-relation correction: bounded follow-up

**Design / Target follow-up · 2026-10-10 · Decision: Accept, bounded source design**

The correction resolves the execution structure identified by F01 in the [integrated review](design_review_surrealdb-capability-integration_2026-10-10.md). Relation nomination now begins with exact selected-view membership and the model-declared source relations that can produce the requested alias endpoints. Retaining unrelated same-relation payloads no longer widens this nomination universe. The existing exact membership, nominal-conflict and complete global ordering stages remain before the returned prefix.

This accepts the inspected architectural correction. Its maturity is **Implemented**, not **Tested**: new-source compile and functional controls were **not_run** at this follow-up boundary. The original dated **Revise** review remains intact; coordinator §8 owns F01's current disposition and §9.1 owns execution receipts. This addendum does not promote the pending assembled qualification or any operator deployment claim.

## Scope and baseline

This is a bounded follow-up to F01, using core/template 3.3, heuristics 1.0, code-intelligence 1.5 and the repository binding. It examines the relation nomination correction, its shared alias-closure extraction and affected controls. It also examines the small executable-local schema-identity cache. It does not restart SA0–SA9 review, certify provider extraction, or reassess unmodified publication, migration, search and lifecycle protocols.

The baseline is dirty shared `main`, HEAD `3d41aec11f379d2fc6edcfcb4d06e2cef645ecbb`, with these inspected source identities:

| File | SHA256 |
|---|---|
| `crates/lctx-model/src/domain/graph.rs` | `bba6e98e1b19fe25903d976e2564807425999b7a1e6726f37aef17eb92c6187d` |
| `crates/lctx-surrealdb/src/reader.rs` | `2c8495dd2311022f72f1274277d804ffe8d1d1524653670690e06383c9880766` |
| `crates/lctx-surrealdb/src/selection.rs` | `7cdae303b7fa499d26928ba054d0a869bade0aa1bf7fa3cb0305c98b032de4d1` |
| `crates/lctx-surrealdb/tests/native.rs` | `14217788dd10c306e298a91c32e0e80a4ae7d21586d833a272112b4aee07075d` |
| `crates/lctx-surrealdb/src/compiler.rs` | `8291ad96693f06b1db539f49190a91c93690db96e41843f6c11cec1c66f0b95a` |

Method: independent static inspection of the changed production paths, their decisive shared helpers, deterministic schema inputs and the affected controls. The reviewer ran no Cargo command, native probe or service operation and made no repository changes.

## Why this addresses F01

`reader.rs:238` validates cancellation/admission, caller scratch authority, exact view scope and relation schema before choosing the route. Limit zero or an empty selected-view set returns an owned empty stream without submitting row queries. Other requests enter `SelectedPayloads::sparse_relation_rows` rather than the prior physical-table-wide semantic-type query.

`selection.rs:449` nominates direct members with:

```sql
SELECT node AS id FROM compiler_view_member
WITH INDEX view_key
WHERE view=$view AND relation=$relation
```

The actual index is `view_key(view, relation, semantic_key UNIQUE)`. Each query binds one exact selected view and relation. The operation therefore consumes that selected relation's membership, rather than asking which retained physical payloads belong to the relation across all releases.

Aliases remain part of the supported answer. The model declaration in `graph.rs:688` now generates both `Entity::canonical_place_endpoint` and `Entity::canonical_place_endpoint_relations`. The latter identifies Place→EntityRef from the same declaration that constructs that endpoint. The reader does not duplicate this semantic classification. For an EntityRef request, nomination also reads selected Place membership, follows indexed `alias_source` edges from those sources and filters the exact reached physical IDs to the requested endpoint relation. Intermediate nodes remain available for traversal; they are not mistaken for requested endpoints.

The extracted `expand_nodes` helper at `selection.rs:175` keeps the existing disk-backed merge and unseen-frontier algorithm. Each round admits only previously unseen targets to the next frontier, so cycles terminate. Ordinary complete selection still requests retained aliases; selective relation nomination uses the same traversal without building an unused alias output. No complete unrelated view is prepared for the selective route.

Nominated identities then enter the existing sparse candidate driver at `selection.rs:509`. Reverse ancestry and indexed exact membership establish eligibility. Nominal membership probes still detect another selected physical revision of a witnessed key. Payload hydration remains finite and checks correspondence. The sort is exhausted once for validation and rewound before any result is delivered; the outer reader applies the limit afterward. The correction does not introduce a per-view early quota, weaken union deduplication, or allow a late selected conflict to be hidden by an apparently valid prefix.

For F01's revealing growth scenario, hold the selected publication and requested relation fixed while adding unrelated retained same-relation payloads. None of the new nomination queries enumerates those payloads. Work can still grow with the requested selected relation, its relevant alias closure, the number of selected views and the witnessed nominal keys. Those are legitimate inputs to this exact relation contract. Complete validation before a small prefix also remains intentional; this follow-up makes no claim that all limited answers require work proportional only to their output size.

## Change locality, composition and preserved guarantees

Adding another supported canonical endpoint kind belongs in the model's construction/mapping declaration and its lowering/admission controls. Native nomination consumes that mapping, while the native module owns indexes, finite windows, spilling and transport completion. This is a useful semantic boundary without a new registry, persistent representation or provider framework. The current supported mapping is Place→EntityRef; this review does not certify arbitrary future alias policies or newly admitted source/target families without their own revealing controls.

Using the existing membership prefix and forward alias index is simpler than a new selected-relation projection or another schema generation. It preserves immutable exact views and reuses qualified sparse membership machinery. Restoring a global semantic-type scan or imposing early limits would reintroduce the original growth defect or weaken exact union answers.

Cancellation is still checked before submission through the cancellable source. Nested nomination and hydration streams retain their drainage paths, and the returned relation stream retains scope/client ownership. The extracted closure continues to use charged disk-backed rows; endpoint windows have explicit charges. These are source preservation judgments, not new runtime cancellation or cleanup results. No source change in this correction narrows publication pins, backup snapshots, history guards or maintenance ownership.

## Schema-identity cache

`compiler.rs:660` wraps the existing `base_schema_identity` computation in an executable-local `std::sync::OnceLock<ContentHash>`. Its initializer hashes the same ordered concatenation of canonical schema, compiler schema, control schema and `VIEW_SCHEMA` bytes. Inspection of those inputs shows fixed declarations or deterministic model-derived schema generation, rather than a selected database, configuration or mutable catalog.

Caching that value once per executable is appropriate: it avoids repeated DDL construction/hash work without turning actual installation inspection into a cached certificate. The cache does not change the hash inputs, introduce a new schema generation or refresh an installed executable. Changing the function later to include configuration-dependent or live inputs would require revisiting this lifetime. No material defect was found in this bounded cache change.

## Controls and remaining acceptance obligations

The new control source challenges the correction at useful boundaries:

- `graph.rs:14321`, `canonical_place_endpoint_discovery_matches_construction`, independently asserts the supported Place→EntityRef pair, concrete endpoint construction and absence for an ordinary Package.
- `selection.rs:658`, `relation_nominations_follow_selected_membership_independently_of_foreign_payload_growth`, holds selected membership fixed across 0 and 4096 foreign same-relation payloads. It covers absent/present membership and direct/alias answers, includes a cyclic alias chain, rejects global payload enumeration, compares query/node work and checks charge release. This is a fixture seam; it does not establish native planner behavior.
- `reader.rs:1098`, `empty_relation_scope_preserves_admission_and_budget_checks_without_native_queries`, adds a nonempty view with limit zero using an unconnected client, while retaining schema, cancellation and budget refusal checks.
- `tests/native.rs:581`, `indexed_sparse_reads_preserve_selected_absence_aliases_conflicts_and_budget_lifetimes`, exercises actual native membership and payload behavior. Its compatible union crosses finite windows, checks exact payloads and global limited ordering, deduplicates shared rows, follows aliases from Place, rejects conflicting selected revisions before a prefix and checks request-charge release. The added native `EXPLAIN` query at line 698 checks the exact selected view/relation prefix route for `view_key`.

**Outcome on this source: not_run.** This reviewer executed none of these controls. The coordinator's earlier 804 model/analytics passes belong to an older source and cannot qualify these changes. The active pre-patch assembled run also cannot establish acceptance of this patch. Provider timeouts are a separate reported failure scope and do not supply either positive or negative evidence for these relation controls.

Before recording functional closure, the coordinator should compile the touched model/native crates and run the focused new/changed controls above under the existing verification ownership. Suitable focused invocations, not executed here, include:

```sh
cargo check --release -p lctx-model -p lctx-surrealdb --tests
cargo nextest run --release -p lctx-model -E 'test(canonical_place_endpoint_discovery_matches_construction)'
cargo nextest run --release -p lctx-surrealdb --lib -E 'test(relation_nominations_follow_selected_membership_independently_of_foreign_payload_growth) | test(empty_relation_scope_preserves_admission_and_budget_checks_without_native_queries) | test(relevant_nominal_conflict_is_rejected_before_any_prefix_is_exposed)'
just fixture -- cargo nextest run --release -p lctx-surrealdb --test native -E 'test(indexed_sparse_reads_preserve_selected_absence_aliases_conflicts_and_budget_lifetimes)'
```

Retain applicable cancellation/terminal and charge-release acceptance in the coordinator's affected-control selection, particularly because complete selection shares the extracted closure helper. This calls for focused regression evidence, not another architectural restart or timing campaign. Applicable functional/non-functional and assembled obligations remain with the existing plan. A failed focused control must be resolved or recorded candidly before claiming Tested closure.

## Judgments and bounded decision

| Judgment | Verdict for this follow-up |
|---|---|
| A1 — Localize change | **Satisfied:** model endpoint meaning and native access mechanics have separate, local owners |
| A2 — Encode domain meaning explicitly | **Satisfied:** construction and relation discovery share the model declaration; exact selected membership still governs eligibility |
| A3 — Extend through composition | **Satisfied:** membership prefix, alias traversal, sparse checks and global sort compose without a new persistent subsystem |
| A4 — Fit execution to workload | **Satisfied for F01's inspected scenario:** unrelated retained same-relation content is outside nomination; relevant selected relation/alias validation remains explicit |

G1–G4, G7 and G8 **pass at the inspected source boundary**. G5 and G6 **pass for the preserved inspected protocols**, with runtime and enclosing assembled qualification still **unresolved**. CI-G1 and CI-G2 **pass for the inspected exact-view/alias distinctions and evidence boundaries**; provider completeness and the assembled journey are not certified. CI-G3 remains **not applicable**. No positive source judgment offsets a failed or unrun execution result.

The F01 mismatch against FP-07 and DP-10/14/20 is corrected in this source, with H5–H10 informing the selected-universe and preparation assessment. The source provides no new material violation of the responsibilities and fidelity obligations examined in the integrated review. This is not a new verdict on its excluded scenarios.

**Finding disposition:** preserve stable ID **F01**. Its architectural correction is **Implemented and source-accepted**; functional closure awaits the new-source controls. No new material finding is raised. The [persisted coordinator §8](../../plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) remains the sole current disposition owner.

**Authority/rule impacts:** none. No ADR, design-standard rule, semantic authority, schema identity or operator-adoption rule must change to adopt the correction.

**Decision:** accept this bounded source design and proceed to focused functional acceptance. The original integrated review's F01 is resolved architecturally at the identities above. Pending assembled acceptance, installed-maintenance executable refresh, generic collection, SM8, BC3, real-library qualification and operator adoption retain their original boundaries. This addendum neither establishes their completion nor creates new authorization for them.

**Intended publication path:** `docs/design_review/reviews/design_review_surrealdb-capability-integration-followup_2026-10-10.md`.
