# Design review: PostgreSQL foundations PG8–PG11

**2026-09-27 · Change / conformance · Final decision: Accept scoped after correction.** This is a
fresh bounded review of the
implementation under [ADR-0068](../../adr/0068-postgresql-serving-and-standard-embeddings.md),
at `de2343c` plus the uncommitted PG9–PG11 tree. It retains the initial findings and records their
correction recheck below; current disposition belongs in the
[forward plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).
The [PostgreSQL plan](../../plans/postgresql-integration-plan_2026-09-27.md) owns execution.

## 1. Scope, owners and change scenarios

**Standard:** repository core 3.0, code-intelligence profile 1.1, and the library-context binding
declared by `design_principles/standard.toml`. The `design-review` and
`design-review-code-intelligence` skills govern this review. The binding's stale summary of §B13
does not override the actual §B13 owner and accepted ADR-0068.

**Supported scope:** format-2 1024 embedding identity; pure serving manifests and relation
receipts; SQLx effect ownership; typed SQL/COPY codecs; extension, role and pool foundations;
shared support validation; file/native consumers; standalone native-wheel identity. The review
examines the changed boundary and adjacent consumers, not the entire behavioral compiler.
Production PostgreSQL import/promotion, query cutover, ANN/fusion, and qualified federation remain
PG12–PG17. Neither those capabilities nor Stage 3 completion are certified here.

**Method and strength:** source inspection is **Implemented / Interface-checked** on 2026-09-27.
The focused malformed-manifest probe below is **Tested** on that date. Other execution claims are
attributed to the [implementation evidence](../evidence/2026-09-27_postgresql-expansion/implementation.md),
which was read; this reviewer did not rerun its live/GPU/database workloads. Broad gates were
**not_run** by this reviewer. The user-authorized functional scope had completed before this
review began. Concurrent Stage 3 review and supplied external review were left untouched.

| Component | Responsibility and consumer boundary | Dependency / change direction |
|---|---|---|
| `cpg-schema::embedding_spec` | Pure canonical spec, admission and vector checks | Compiler, Python spec parser and cache consume one identity; no database setup |
| `cpg-schema::serving_projection`, `serving_support` | Declared Arrow relations, manifests, logical receipts, keys and evidence closure | File publication, native binding and SQL adapter depend on this meaning |
| `cpg-core::bundle` | Published Delta snapshot → immutable file generation | Orchestrates schema-owned validation; snapshot remains canonical |
| `lctx-postgres` | SQLx migrations, cache/operations, role pools and physical codecs | Depends on schema; serving wheel avoids compiler/DataFusion/Delta dependencies |
| `lctx_storage` | Explicit asynchronous repository open/check/close | One configured Tokio runtime, lifespan-owned pool; pure semantic wheel remains separate |
| `cpg-core::postgres_read` | Typed configuration and separate bounded provider pool | Planned read consumer only; PG15 owns SQL/type/pushdown admission |
| Python generation loader | File reference consumer and native/lexical state construction | Uses shared schemas/closure; existing relational maps await PG13 replacement |

**Fact and fidelity boundary.** This slice projects existing attributed facts; it does not add
an analyzer or reinterpret behavioral verdicts.

| Family | Provider / fidelity | Coverage, identity and consumer |
|---|---|---|
| Operations, paths, facets and behaviors | Existing compiler/catalog; derived | Domain IDs and five verdicts remain in declared Arrow fields; exact file selectors remain current |
| Assertions, findings, witnesses and attribute incidences | Existing programmatic synthesis/analytics | Shared validation retains complete support and ordered handoff roles; IDs remain generation scoped |
| Conditions, summaries and source/model identities | Existing native IPC contract / bounded model | Native files move to a shared inventory; semantic executor retains its own admission and budgets |
| Vector views/chunks | Pinned fake or Qwen spec; heuristic retrieval input | Float32 bytes and logical multiplicity are retained; no vector route acquires exhaustive semantics |

The projection answers “which immutable relation/artifact content belongs to this generation?”
Its universe is all 51 declared relations and the complete native/lexical artifact inventory.
Logical multiset hashes ignore row order and batch boundaries while retaining duplicates, nulls
and Float32 bits. Explicit budgets refuse work; they never certify a truncated projection.

### Traced scenarios

1. **Add a Stage 4 serving relation using already supported Arrow types.** The semantic owner
   adds the declaration, keys/references and genuinely new domain rules; file queries provide
   its rows. SQL physical types and field codecs follow the schema, and Python's deleted schema
   inventory does not need a second edit. The immutable migration must be appended/generated,
   version/definition identity must change as appropriate, and an independent malformed relation
   exercises the boundary. This is a credible extension path; a new semantic family legitimately
   needs a new shared validator. No provider framework or ORM is needed.
2. **Load the same relation receipts under a different declared embedding/provenance identity.**
   The shared validator should reject inconsistent identities before returning an admitted
   generation. Currently each identity can be valid in isolation while disagreeing with another
   carrier: F01. Checks spread across callers make the future importer depend on hidden ordering.
3. **Use a canonical snapshot compiled without embeddings.** The manifest already admits
   dimensions 0 and `bundle::files(0)` declares zero-width empty vector schemas. The physical
   codec instead fixes dimensions to 1024: F02. This is an existing supported variant, not a
   request to introduce a second production embedding standard.
4. **Cancel a blocked repository query / freeze while an importer transaction writes.**
   QueryLease discards unfinished SQLx connections. A generation row's shared writer lock
   conflicts with the freeze update; writes admitted after freezing fail. The implementation
   evidence records real blocked-query cancellation, heartbeat/reuse, concurrent writer/freeze,
   readiness visibility and immutable-ready controls. No promotion capability is exposed to
   importer credentials yet; PG12 must retain that boundary.

## 6. Correctness and fidelity gates

These are the initial assessment's verdicts. The correction inspection at the end records the
subsequent implementation without erasing the defects and evidence that prompted it.

| Gate | Verdict | Evidence and boundary |
|---|---|---|
| G1 Authority | **fail** | F01: independently admitted manifest/envelope/spec assertions can disagree without reconciliation |
| G2 Semantic fidelity | **fail** | F02: dimension-0 declared empty vector schemas cannot round-trip through the fixed-1024 codec |
| G3 Validity | **fail** | F01: contradictory embedding identity reaches successful pure projection admission |
| G4 Hidden behavior | **pass** | Schema validation/native evaluation remain pure; database/file effects have explicit owners; no startup migration |
| G5 Consistency and recovery | **fail** | Manifest provenance consistency is incomplete (F01). SQL readiness foundation and cancellation evidence otherwise support the bounded claim; production promotion/recovery remain excluded |
| G6 Transformation and reuse | **fail** | F01/F02 affect identity/substitution. Logical receipt framing, multiplicity and Float32 preservation otherwise have independent controls |
| G7 Truthful capability claims | **pass scoped** | Implementation evidence distinguishes selected foundations from PG12–PG17 activation, transport growth from a tenfold semantic corpus, and root/wheel identity from full qualification |
| G8 Library leverage | **pass** | SQLx, pgpq, pgvector, Arrow and PyO3 async own generic mechanisms; domain closure and receipt framing have concrete contracts |
| CI-G1 Source fidelity | **pass scoped** | Existing verdict, coverage and relation kinds are carried without reinterpretation; semantic compiler completeness is not assessed |
| CI-G2 Evidence closure | **fail** | F01 permits contradictory snapshot identities across the admitted projection and the file/native envelope; support-row closure itself remains shared and checked |
| CI-G3 Evaluation integrity | **n.a.** | No evaluation tuning, acquisition path or gold input changes in the reviewed slice |

## 7. Findings and applicability

<a id="F01"></a>

### F01 — Projection admission does not reconcile its identity carriers

**Priority:** P1. **Owner:** `cpg-schema::serving_projection`, with file/native envelope adapters.
**Principles:** FP-02/04/05, DP-01/03/04/09/24, CI-11/13; A2; G1/G3/G5/G6 and CI-G2.

**Implemented evidence:** `Manifest::validate` checks the shape of `spec_hash`, catalog and
provenance digests. `validate_relations` separately checks the parsed `embedding_spec` relation
against its own spec/hash/dimensions. Neither binds the manifest's hash to that relation.
`validate_projection_ipc` compares relation/artifact receipts and the definition digest but does
not reconcile those identities. `cpg-core::bundle::verify` and Python `generation.load` validate
the nested projection key without checking that its snapshot, content/compiler, spec and
entry-effect identities agree with the outer file manifest. The native executor receives the
outer snapshot/entry-effect identity, while the projection advertises its own.

**Tested, 2026-09-27:** a focused `uv run --no-sync python` probe loaded `build/py-fixture`'s
existing manifest and all Arrow bytes, then independently replaced each field below with a
valid-width all-`ff` value before calling `lctx_semantics.validate_projection_ipc`. All four calls
returned successful projection keys:

| Changed field | Result |
|---|---|
| `spec_hash` | admitted, key prefix `62b19a024f8382f7` |
| `catalog_digest` | admitted, key prefix `f70e18329df1ac58` |
| `snapshot_id` | admitted, key prefix `4aa4140065648b4d` |
| `entry_value_effect_digest` | admitted, key prefix `3af868222d2dd59c` |

The spec case is directly contradicted by unchanged relation content. Snapshot identity cannot
be inferred from snapshot-stripped serving rows, so its authority must be supplied by and bound
to the envelope rather than guessed. The catalog must identify the admitted model catalog or
an explicitly supported compatibility policy.

**Correction:** expose a complete shared manifest/content validation boundary that internally
checks definition, receipts, aggregate per-relation budgets, parsed spec identity and supported
catalog identity. Bind outer envelope fields to that validated manifest in both readers through
one declared consistency contract. Keep snapshot authority with the published-snapshot/file
envelope owner. Today callers invoke `receipt` before `validate_relations`; the latter checks
limits per batch, so the combined boundary should also internalize aggregate per-relation limits
instead of making PG12 callers remember this precondition. No new ADR is needed to enforce the
accepted identity contract.

**Closure:** malformed spec/catalog controls must be rejected after valid outer receipt framing;
contradictory nested/outer snapshot, content/compiler, entry-effect and spec identities must be
rejected by both readers after recomputing content-derived keys. Valid compiler output and the
standalone native wheel must continue to agree. Current disposition belongs in
[forward-plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).

<a id="F02"></a>

### F02 — The physical codec loses the no-embedding schema contract

**Priority:** P2. **Owner:** `lctx-postgres::projection`.
**Principles:** FP-02/03/05, DP-02/08/15/24, CI-13; A3; G2/G6.

**Interface-checked, 2026-09-27:** `Manifest::validate` accepts dimensions 0 or 1024. The
`bundle::files` contract explicitly uses 0 without a spec, producing `FixedSizeList(Float32, 0)`
for the two empty vector relations. `copy_bytes` calls `validate_batch(name, 1024, batch)`;
`decode_rows` selects `bundle::files(1024)` even for an empty result. Therefore an empty vector
relation from a supported no-embedding snapshot is rejected on encoding, and decoding invents a
different schema/digest. Existing empty-round-trip coverage only uses `lexical_text`, whose
schema does not depend on dimensions.

**Correction:** carry validated generation dimensions through COPY/decode or a small bound
codec value. For dimension 0, preserve the declared empty schema and reject nonempty vector
rows; PostgreSQL's physical `vector(1024)` parent need not change. Avoid inferring semantic
dimensions from catalog types or silently substituting a 1024 schema.

**Closure:** focused empty `vectors` and `operation_vectors` encode/decode controls at dimensions
0 preserve exact declared schema/receipts, while nonempty dimension-0 inputs and mixed-width
inputs fail. Retain the existing real 1024 pgvector round trip. Current disposition belongs in
[forward-plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings).

**Initial foundations assessment:** FP-01 and FP-06 are satisfied for the bounded owners and isolated tests. FP-04
and FP-05 are violated by the unreconciled identity carriers (F01). FP-02 and FP-03 are violated
at the dimension-dependent adapter substitution boundary (F02). These are narrow corrections
inside the accepted architecture, not reasons to reverse the SQLx/Rust ownership decision.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-27.** Pinned implementations and their uses were inspected; no new
package selection is made. This code review did not require library-documentation research.

| Capability | Fit, alternative and decision |
|---|---|
| Transactions/migrations/pooling | SQLx remains one application effect family; native PostgreSQL constraints/RLS/row locks express lifecycle. Adding an ORM or Python pool would duplicate current ownership |
| Arrow binary ingestion / vector codec | pgpq owns COPY encoding, SQLx transports bytes, pgvector owns vector adaptation; real[] staging respects the RLS restriction. A bespoke generic binary encoder is unnecessary |
| Python awaitables | PyO3 async runtimes supplies event-loop bridging/cancellation; the narrow repository surface has a real PG13 consumer and avoids compiler dependencies |
| Logical receipts / support closure | Specialized schema-owned rules are justified. IPC byte hashes alone would conflate batching with content; database catalog inference would lose domain metadata. Independent Python framing is a useful oracle |
| Provider pool | The owned fork adds bounded typed construction instead of rebuilding the provider. A separate pool has explicit cost and a shared budget; production schema/pushdown/read-view qualification remains PG15 |

The explicit ordered definition serializer closes the earlier serde_json feature-unification
identity drift. The implementation packet attributes matching root/native-wheel digest
`01f5b50e67bf65f16e9a9df89c2416c73c61c0d6663b4b903a089375978d95da` and real file-generation loads.
That is evidence for deterministic encoding, not evidence that all identity fields are mutually
consistent; F01 addresses the latter.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario and required action |
|---|---|---|
| A1 Localize change | **satisfied** | Shared Arrow meaning, isolated SQL effects and independent semantic wheel localize schema/provider/lifecycle work; corrections remain within those owners |
| A2 Encode meaning structurally | **satisfied after correction** | F01's identity relationships now have one schema-owned admission contract, used by both readers |
| A3 Extend through composition | **satisfied after correction** | F02's explicit dimensions preserve the existing empty/no-embedding contract through the physical adapter |

**Final bounded decision: Accept scoped.** F01/F02 are corrected within ADR-0068 at the
source-inspected and focused-tested strength below. The selected stack and ownership split
remain appropriate. This review does not authorize or
certify PG12–PG17 activation. The enclosing PostgreSQL serving architecture remains unresolved
until production import/readiness, exact hydration, ranking and provider qualification are
implemented and assessed at their declared boundaries. Code gates and release qualification
remain distinct from this architectural judgment.

### Correction inspection and focused recheck, 2026-09-27

**Implemented / Interface-checked:** the author corrected both boundaries during this review.
`Manifest::validate` now admits the current definition/catalog; `Manifest::validate_relations`
owns receipt/domain checks and binds the spec hash to content. Aggregate per-relation budgets
are checked by the free validator as well. `Manifest::validate_envelope` declares provenance,
format and spec consistency, and both core verification and Python file loading call it. The
native wrapper's optional envelope permits database readers to validate already pinned content
without inventing a file envelope; it does not assert provenance beyond the supplied authority.

The physical COPY/decode functions now take generation dimensions and both reject unsupported
dimension values before schema construction. Empty width-zero vector schema controls were added
alongside the real PostgreSQL test. Core `a_changed_generation_is_refused` now changes the nested
snapshot identity, recomputes its projection key, and requires the envelope-specific rejection;
the shared check is also visibly called by `bundle::verify`. That additional core regression is
**Implemented**, with its broader gate run still pending at this checkpoint.

| Correction evidence | Outcome and scope |
|---|---|
| `uv run pytest python/lctx_mcp/tests/test_projection.py -q`; inspected `build/pg11-python-projection.log` | **passed:** 9 tests, including independent receipt controls, missing support/codes and all six spec/catalog/provenance mismatches |
| `CARGO_TARGET_DIR=/home/paul/library-context/target cargo test --release -p cpg-schema --test serving_projection -p lctx-postgres --test serving`; inspected `build/pg11-review-regressions.log` | **passed:** 5 schema/receipt tests; ordinary PG target output in this log correctly leaves its 2 database tests ignored |
| `target/release/deps/serving-faa9590caab9df5d --include-ignored --nocapture`; inspected `build/pg11-review-pg.log` | **passed:** all 3 tests, including both empty dimension-0 vector schemas, existing 1024 COPY/pgvector round trip and role/RLS/freeze controls |
| Reviewer `uv run --no-sync python` correction probe against the rebuilt standalone wheel | **passed:** valid fixture returns its declared generation; all six inconsistent identities are rejected with the envelope, and the false spec hash is also rejected without an envelope |

The reviewer reran the final probe directly; the other rows are inspected receipts from the
author's focused commands, not new reviewer runs. Nonempty dimension-0 vectors cannot satisfy
the existing finite unit-vector check; real returned 1024 vectors cannot match a width-zero
decoder. The unsupported-dimension guard was separately source-inspected. No empty-schema
fallback or reinterpretation was introduced.

**Rechecked gates:** G1/G2/G3/G5/G6 and CI-G2 now pass for the bounded supported foundation
contracts. G4/G7/G8 and CI-G1 retain their earlier scoped passes; CI-G3 remains not applicable.
FP-01–FP-06 are satisfied for the traced scenarios after correction. This recheck does not extend
the scope to production promotion, query cutover, ANN, federation or whole-product release
qualification. The forward plan, rather than this dated evidence, owns F01/F02's current closure
status. The implementation owner records subsequent code-gate continuation and pilot outcomes
in the [implementation evidence](../evidence/2026-09-27_postgresql-expansion/implementation.md);
those receipts do not extend this review to PG17.
