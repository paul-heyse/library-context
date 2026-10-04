# Model contract and execution alignment

**Proposed, 2026-10-04.** Supporting design for the [target-alignment coordinator](target-implementation-alignment-plan_2026-10-04.md). The coordinator owns source dispositions, conditional outcomes and combined qualification. This document owns F1–F6/IF1–IF4 design and acceptance, not a second progress table.

## 1. Foundations and selected target

Current typed declarations, immutable generations, exact consumed views, charged Rows and pure operations are suitable foundations. The [target review §§4–8](../design_review/reviews/design_review_target-implementation-alignment_2026-10-04.md#4-composition-and-execution) corrects earlier assumptions: Summary retains both Facts and Model epochs; model-owned membership joins exist; in-memory relational compute can remain pure. Conservative identity invalidation is deliberate.

**Interface-checked source, 2026-10-04:** the analysis-family macro has a common publication expansion, while catalog model outputs and core declarations independently enumerate it. `DependencyClosure` already owns exact requirements and widened grants. `ValidatedModel` keeps relations sorted. PostgreSQL repeats physical scalar/order conventions across signatures, tables and serving views. `KeySink` v3 already has an independent explicit-byte framing control. The embedding `Spec` already declares ordered whitespace-free JSON and is persisted in the cache.

The target centralizes repeated mechanics without replacing specialized semantics. Semantic records remain Rust model-owned; physical generation/epoch/tag columns remain PostgreSQL-owned. Missing/ambiguous input, cardinality, deterministic order and admitted resource lifetime remain explicit.

## 2. F1 — finite publication and code expansion

Own the common analysis publication record list once beside `analysis_family!`. Derive `publication_relations()`, model stage outputs and core `StageOutput::declare::<R>` expansions from that typed list. Compose stage-specific outputs separately. Keep actual domain computations and awaited per-record pushes explicit: the demonstrated common list is synchronous declaration, so an asynchronous heterogeneous visitor is unnecessary for this first slice.

Migrate catalog core/evidence and all actual consumers of the common family, then remove independent lists. Preserve duplicate-output, undeclared-output, incomplete-publication and empty-read enforcement. An authoritative list is not a substitute for validation that the producer actually published it.

Extend `DomainCode`/`DomainSum` mechanics only for real repeated consumers: explicit numeric codes/wire names, labels, exhaustive inventory and schema arms must consume one finite definition. Keep non-codebook arrays where they are deliberate ordering policies; do not introduce strum/reflection solely to rename a list. Append-only codebooks cannot be renumbered.

**Further port assessment (bounded part of F1):** inventory remaining repeated typed stage/provider inventories after the publication slice. A current repeated loader/writer consumer can justify a finite expansion or derive. Keep decoder field reachability, actual read membership, exact validators and sufficient grants separate. Qualify Catalog exclusions and upstream unions, including evidence/native preparation. Sequential awaited loads are the default; a visitor is adopted only after a small compile-qualified contract demonstrates a simpler current consumer. No mandatory universal port DSL.

Acceptance: one added common publication record propagates to model and writer declarations without another list edit; stage-specific additions stay separate. Exercise both profiles, empty/missing/duplicate output refusal, composed upstream membership and actual PostgreSQL publication. Tests independently name important records/epochs rather than deriving every expected set from the new expansion.

## 3. F2 — authoritative grants and closure

Expose a convenient grant-construction operation on `DependencyClosure` using its existing algorithm. It consumes direct uses, exact validation roots, own outputs, immutable vocabulary boundary, lower-layer policy and `PublicationOrder`; it produces sufficient grants or `ModelError`. Callers retain their decoder inventories and exact consumed views. Replace copied walks in Local/execution/Model/Summary/Structural/catalog/evidence/native preparation and any remaining actual stage consumer found by scoped search.

Preserve exact requirements as `(relation, vocabulary epoch, stream order)`. A wider acknowledged grant satisfies earlier reads but must not collapse the Facts and Model consumed views. Ordinary facts stay explicitly required; the frozen-facts treatment of inferred premises stays unchanged. Local missing predecessor/self-dependency becomes a typed rejection rather than panic.

Use binary lookup over `ValidatedModel`'s sorted relation vector for repeated name lookup; no additional maintained name map is needed. Preserve relation ordering and errors.

**Validator composition selected:** empty-list inheritance and identical nonempty lists compose; differing nonempty lists remain incompatible. Different transport/availability contracts still refuse. Current source supplies no consumer requiring conjunction, so expanding that policy would add unsupported meaning. A future named consumer can reopen it at this owner, specifying input epochs, validator identity/order and digests; do not union lists in a macro.

Acceptance: both profiles across actual stages, grant-policy conflict and Local unfinished/self-dependency refusals; exact ordinary facts and Summary Facts/Model controls; shuffled source determinism; actual PG validation views. Compare previous and new schedule/grant digests where intended equivalent. A corrected policy difference must be identified and deliberately migrated, not hidden in snapshot acceptance.

## 4. F3 — one PostgreSQL physical layout

`lctx-postgres` owns a finite physical-column descriptor derived from each declared `Relation`. It names ordered generation/id columns, immutable vocabulary epoch where present, semantic fields and subtype tags, with PostgreSQL type and nullability. Table lowering, projected column lists, signatures and expected serving view shapes consume it. Centralize scalar-to-PostgreSQL type conventions there.

Keep Arrow/store codecs separate: their semantic fields follow the exact model schema, and internal generation/epoch/tag columns do not become new semantic attributes. Preserve foreign keys, subtype active/inactive payload constraints, list element/null rules, binary widths, finite-float constraints, indexes, quoting and role grants. Share quoting only where a concrete cross-module consumer currently duplicates it.

Prefer an unchanged physical representation. If the descriptor exposes an actual convention defect, version the affected lowering and rebuild; the consolidation alone must not silently reorder columns or relax constraints. PostgreSQL catalog inspection remains independent of descriptor generation.

Acceptance: independent hand-expected scalar/list/nullable/subtype shapes, real PG codec round trips and deliberate wrong type/order/nullability/tag/FK/view/ACL controls. Keep `pg_attribute`/`format_type` and rolled-back shadow comparison. Never test only two consumers of the same mistaken descriptor.

## 5. F4 — explicit identity encodings

Replace `Debug` contributions in model declarations, mapping and consumer/preparation signatures with framed owner-defined encodings. Include owned scalar kind, list/nullability, role, reference target/subtype, numeric codebook entries and sum-arm membership. Relations retain canonical ordering; fields retain declaration order; sum arms use numeric code and declared payload membership. Use explicit capability/preparation names rather than Rust formatting.

Version affected identity namespaces. Preserve the model build fingerprint's raw Rust source, relative paths, manifests and lockfile dependencies: it is an artifact compatibility fingerprint, not a formatting/refactoring-stable semantic ID. Mapping/consumer identities currently have test-only consumers; do not describe their current risk as a demonstrated production cache failure. Encode contract-relevant metadata deliberately and exclude incidental Arrow presentation metadata.

Retain and extend `prepared_hashes_preserve_original_framing_and_order`; it already independently reconstructs KeySink v3 framing. Add hand-authored declaration/mapping byte vectors, order/mutation cases and intended-equivalence controls. Changing field meaning, reference target, sum code, provider/source or manifest dependency must invalidate the appropriate identity; metadata formatting alone must not alter a contract encoding that excludes it.

**Embedding specification:** retain current ordered, whitespace-free struct JSON, field order, integer representations, optional/null distinction and vector order. Parsing is not byte identity; serialize validated typed `Spec` using the specified owner before hashing. Preserve Rust/Python conformance and persisted cache spec checks. Add independent vectors for Unicode escaping, absent versus present admission and vector order as useful. No JCS dependency is selected: this spec has no floating values needing a new numeric-equivalence rule. A serializer upgrade/external consumer requiring another equivalence must explicitly version and rebuild the cache contract.

Selection/wire JSON identities keep their explicitly pinned serialization contracts; inspect actual consumers for incidental unordered components. A cross-version reuse requirement can enable a bounded follow-up, not a blanket rewrite of all hashes.

Acceptance: independent framing/declaration/mapping/spec vectors, payload mutations, shuffled irrelevant metadata controls and no stale cache reuse. Review identity/snapshot changes and rebuild source-matching artifacts. No old-format readers or durable rename-stable IDs without a real consumer.

## 6. F5 — small charged helpers and existing mechanisms

Add required Rows lookup accepting the caller's typed error constructor. Preserve different Local/theory/field/model refusal domains; no shared generic error replaces them. Keep same-key conflicts and deterministic ID order. PostgreSQL `Batch` lookup is a different decoded-store boundary and is not unified with Rows.

Add a secondary index only for a named repeated access pattern with useful current consumers. It retains nominal IDs, owns its reservation, charges before allocation/growth and preserves deterministic iteration. Uniqueness/ambiguity policy belongs to the consumer, not an assumed one-row index.

Reuse `Diagram::support()` for the existing local-semantics support consumer if it preserves the current charged output/identity contract. Keep bounded simultaneous substitution, keyed topological ordering and interval containment where no library supplies the required refusal/lineage semantics. Existing correct stable sorting/byte windows/validator framework need no replacement.

Acceptance: missing and conflicting lookup, distinct typed errors, shuffled input equivalence, admission before retention and reservation release. An index adoption requires a selected workload and construction/retention cost accounting; no universal query abstraction or performance claim without measurements.

## 7. F6 — caps and PostgreSQL effect lifecycle

Make the enriched execution loop's 64-round boundary explicit in its owning outcome: identify the cap and preserve pending/refused evidence. Prefer an established domain-derived finite bound only if the actual lattice/composition proves it; do not infer completion from reaching a numeric stop. Consumers report partial/refused, never absence or complete semantics.

Investigate migration-error advisory-lock lifetime through the pinned SQLx effect owner. Scope: one failing migration, connection reuse/lock observation and retry. If failure retains a session lock in the pool, use a dedicated owned connection for migration and discard/close it on error so lock lifetime cannot leak into unrelated work. Qualify successful migration, deliberate failure, lock release and repeat install. If the premise does not apply to the actual configured path, record retained behavior and a reproducible reopening trigger. Do not change request retry policy to fix migration lifecycle.

Typed SQL construction remains conditional on a substantive variable query consumer. Existing SQLx/sea-query/pgpq effects and bound identifiers/values remain; `plan_to_sql`, binder or another driver are not required for simple scans. Physical layout consolidation and this operational inquiry are separate packages.

## 8. Conditional library investigations

Investigations use one named current/planned operation and the smallest relevant source/control set. First establish the consumer; absence closes the inquiry as deferred with trigger. Adoption requires an explicit producer/consumer migration and retirement of the replaced path. Historical isolated P1–P6 probes are leads, not qualification of this tree.

| Package | Question, decision evidence and outcome boundary |
|---|---|
| IF1 BDD elimination | Compare current bounded exists with fused limited library operations on finite independent truth tables, variable support and deterministic caps. Equivalent accepted values do not establish equivalent refusal; choose preserve or deliberately migrate WorkPreflight/task-count policy. Adopt only if simpler under selected policy; measure separately before speed claims |
| IF2 graph operations | A named evidence-preserving SCC/ranking operation must specify internal self-loops, parallel arc identity, borrowed canonical IDs, distinct-neighbor weighting, charged allocation, work/stop and deterministic outputs. `condensation(false)`/graphops is not a drop-in. Dense-index crates need a real cross-module index consumer. No projection/ranking need yields deferral |
| IF3 concepts | Compare one needed FCA/RCA operation against odis lazy iteration/public preclosure or fcars while preserving iceberg/pseudo-intent and per-step caps. Keep development oracle independence. No operation gap yields retention; do not claim odis wholly batch-only |
| IF4 compute/fixpoint/diagnostics | For a named shared-channel relation consider Ascent BYODS/custom relations or DataFusion in-memory lowering, including pending/refused state, deterministic work, order/multiplicity/nulls and admission before mutation. Earlier Ascent probe is not universal impossibility. Tracing needs a named diagnostic consumer; lack of one, not stale OpenTelemetry claims, is the deferral reason |

Accepted bespoke finite kernels record scoped CI-07/DP-13 reasoning at the compute owner: nominal input, typed refusal, deterministic bounded work and store-free testing reduce total integration cost for those operations. This does not exempt unrelated generic reimplementations or ban library-owned computation.

## 9. Integration and acceptance boundary

F1/F2 contracts can develop independently with coordinated shared declarations; broader port migration consumes both. F3/F4 are separate changes followed by deliberate compatibility refresh. F5 lookup and F6 effect inquiry need neither broad port completion nor real-library activation.

Each selected producer/consumer package uses touched-crate compile checks and focused model/core/actual-PG tests. Q0 in the coordinator owns snapshots, matching artifacts, remaining shared journeys and final `just test-all`/`just hygiene`; no full campaign after every slice. Real-library selection, embeddings and measurements remain stopped. Current documentation source assessment is Interface-checked; production remedies and all future test outcomes remain Proposed/not_run until executed.
