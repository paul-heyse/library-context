# SurrealDB 3.3.0 remediation candidates

2026-10-06. Research baseline: clean `main`, `e52e6312367140a173e8aef338e10a6c69efbeb6`, `/home/paul/library-context`. Coordinator assignment: planning research only; no production changes, test/probe execution, upgrades, skill edits, commits or delegation. This file is the only written artifact. Design candidates below are **Proposed**, grounded in **source observations** and the skill's historical named probes; they are not newly Tested or Measured.

## Scope and evidence

Read shared worker/library-research contracts and AGENTS supplied in context; applied `neo4j-surrealdb`, `context7-mcp`, core FP-07 and the efficient-architecture heuristics. Memory registry search for SurrealDB/graph-native/neo4j-surrealdb returned no relevant entries; no memory-derived fact was used. Cargo.lock:7438 resolves SDK 3.3.0. Skill `surrealdb/reference.md` pins v3.3.0 commit `238bfeb11f5725bebed370167656748df8067595`; documentation capture is main commit `82b7ac1935fcbb400e80cf357dd68f47182bad26`, not release-versioned documentation.

Reviewed the SurrealDB guide/reference, every one of its 18 capability briefs, all 15 catalog files, all four route files and the 49-question topic map. For the 4,454-line SurrealQL catalog, surveyed its complete 165-feature inventory/coverage and examined relevant examples (SELECT, INSERT/relation, GROUP/SPLIT, EXPLAIN, WITH, sets/arrays, transactions/export); no assertion that every unrelated example was independently revalidated. Other catalogs cover values, DDL, functions, GQL, errors, features/releases, server, GraphQL/Postgres/MCP, coverage/interop and upstream language tests. Neo4j was outside scope. No refresh or corpus-wide implementation audit was performed.

Context7 resolve selected official `/surrealdb/docs.surrealdb.com` for bulk/conflicts and SELECT/filter/graph/EXPLAIN concepts. Its current docs support INSERT IGNORE, whole-transaction conflict retries, record-only FOR UPDATE and filtered graph/group expressions. `/surrealdb/surrealdb` returned no match for gRPC export finality; that gap was resolved from pinned SDK and official tag source. Do not transfer Context7's JavaScript `.retry()` example into Rust: the pinned Rust API/probes do not provide that guarantee.

Audit anchors: `docs/design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md` F04:144, F07:197, F11:265. Repository consumer inspection: `crates/lctx-surrealdb/src/cache.rs`, `reconciliation.rs`, `schema.rs`; `crates/lctx-publisher/src/search.rs`, `inspection.rs`, `backup.rs`.

## Recommended F07 candidate: delegate finality to existing gRPC file export

Replace only backup's HTTP export client with a dedicated authenticated gRPC client targeting the pinned database. Preserve the canonical-only table selection, users/accesses/versions exclusions, private staging file, no-clobber publication, fsync and session cleanup. Existing HTTP import can remain where its composed contract is adequate. Do not build a parallel protobuf exporter, dump certificate or mandatory restore-before-backup publication workflow.

The full 3.3.0 path supports this recommendation:

1. SDK `surrealdb/src/engine/remote/grpc.rs:1370–1380` `export_file` awaits `export_surql` then `write_export_to_file`.
2. SDK `:1483–1502` opens server ExportSurql and passes frames through `export_chunks`.
3. SDK `:1566–1643` refuses EOF without a terminal trailer, transport status errors, explicit error frames, unrecognized frames and trailer byte-length mismatch. A matching trailer ends the export successfully. It deliberately does **not** verify optional trailer BLAKE3.
4. SDK `:1765–1783` drains all normalized chunks, propagates each error and flushes the file before returning. Choose the file route: `export_bytes` launches a detached channel pump, so awaiting its setup is insufficient by itself.
5. Server `surrealdb/server/src/rpc/grpc.rs:2043–2066` obtains `export_with_config`'s actual engine future and retains its task outcome beside a bounded chunk channel.
6. Server `:2890–2989` `frame_byte_stream` drains chunks and then **awaits that outcome**. Only `Ok(Ok(()))` emits Trailer; engine failure or task panic emits Error. This is the crucial difference from the HTTP handler that logs late failures and closes its channel as ordinary EOF.

Official source URLs:

- https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/grpc.rs#L1370-L1380
- https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/grpc.rs#L1566-L1643
- https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/grpc.rs#L1765-L1783
- https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/rpc/grpc.rs#L2043-L2066
- https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/rpc/grpc.rs#L2890-L2989

Remaining wrapper gap: error/timeout/invalidation failure must prevent completed destination publication, and cleanup must not replace the original failure. The existing NamedTempFile lifetime can discard a partial dump. A failure after successful final rename/fsync is a separate local publication-result concern; preserve truthful failure semantics. Terminal trailer establishes completed engine export/transport, not semantic graph validity or protection against an adversarial root. Do not claim checksum verification by the SDK.

Later implementation acceptance should cover normal canonical-only backup/fresh restore plus deterministic error frame, missing trailer/short stream and engine-late-failure cases. None were executed here. Existing skill SB118 covers normal embedded export/import, not the late gRPC failure path.

## Recommended F11 cache candidate: typed bulk immutable admission and exact winners

Retain one immutable authority keyed by full specification hash plus input hash. Prevalidate all supplied candidates and exact definition/token/vector bytes before making effects; deduplicate repeated keys in a bounded batch with an explicit duplicate-input rule. Construct typed `Winner` values (`SurrealValue`, `RecordId`, `Bytes`, native float array), bind a row array and submit one `INSERT IGNORE INTO embedding_cache $rows RETURN NONE` per bounded batch. Follow it with exact full-key winner retrieval, ideally by the deterministic RecordIds already constructed rather than a broader scan. Keep reads/inserts as separate auto-committed operations unless a real cross-record invariant requires a transaction: cache records are immutable, so a fresh read sees committed winners and no long client transaction is needed.

Validate returned key coverage, exact canonical definition, admitted token contract, finite dimension-correct vector, exact retained bytes and value digest. A winner may differ from the losing proposal's vector: return the committed winner, not the proposal. Clarify in the plan whether token count must match the deterministic input admission count, rather than only stay below the model limit. Preserve winner's original token/bytes; never overwrite with `ON DUPLICATE KEY UPDATE`, UPSERT or UPDATE. Do not trust returned INSERT rows as the complete set of winners.

**Important additional source observation:** at 3.3.0, INSERT IGNORE can suppress more than duplicate IDs/UNIQUE conflicts. `surrealdb-core/src/doc/insert.rs:46–52` rolls back and ignores any insert_create error when IGNORE is set; `:75–100` explicitly documents broad behavior for leaf errors. The upstream `reproductions/insert_ignore_on_duplicate_key_leaf_error.surql` shows a VALUE(THROW) create failure being skipped. Prevalidation and missing-winner refusal are therefore indispensable. A checked query is not evidence every row was admitted. This extends, rather than contradicts, the brief's narrower unique-violation warning.

Concurrent same-key admissions can fail at statement commit despite IGNORE. Retry owner belongs in this cache operation: bounded attempts/backoff within the caller's deadline, retry only classified transient transaction conflicts, replay the same immutable batch or only missing keys after a fresh read. Never retry all generic Internal errors or re-run embeddings. Typed `QueryError::TransactionConflict` is available in remote paths; skill probes establish it over ws, while backend Internal messages vary. Inspect and exercise actual gRPC conflict conversion at implementation before making a typed-only classifier assumption. Ambiguous connection loss is safely reconciled through fresh exact winner lookup, because insert is idempotent and immutable; it is not proof the original write failed. Missing keys after retry exhaustion must fail.

Evidence: skill briefs `surrealdb.bulk-writes-and-upsert`, `transactions-and-concurrency`, `values-and-types`, `errors`; historical SB016, SB048, SB033/SB053/SB089/SB090, SB006/SB046; source https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/doc/insert.rs (local corpus path `surrealdb/content/corpus/surrealdb-core/src/doc/insert.rs`; verify upstream tree location if following URL).

FOR UPDATE is not a required substitute for unique immutable winner storage. It takes record-ID targets only and forbids GROUP/SPLIT (SB092); phantom/missing-key protection and lock ordering must not be invented. Real multi-record mutable invariants would need a short transaction and whole-transaction retry, rather than retrying the failed final statement alone.

## Recommended F11 publisher candidate: reduce first, stream lowering, retain complete witnesses

The existing 128-unit page is not an operation memory bound: it fetches all fragments in those corpora and expands unit × fragment × member × anchor × available embedding uses, decodes the same vector inside that expansion and retains whole-load expected-ID sets. Merely replacing scalar inserts or chunking completed vectors leaves the costly structure intact.

Adopt three coarse intents under the model's semantic authority:

- Produce shared lexical documents once per distinct family/text digest and shared vectors once per specification/input. Decode/check available vector bytes once at their own grain, before occurrence fanout. Retrieve only fields needed by that pass and use scoped native predicates/traversal to avoid unrelated corpus hydration.
- Produce complete contextual occurrence witnesses as a streaming nested expansion. Page/stream fragment, unit-subject, anchor and embedding-use relations separately; no unit/corpus fanout is assumed small. Emit each completed row into a byte/row bounded batch before constructing another; release decoded inputs after their last use. Preserve default `None` member/anchor witnesses when the model requires them, parallel/context distinctions, current stable keys and lexical/vector semantics.
- Reconcile in bounded ordered streams or attempt-owned spill/sort of minimal expected rows/keys, not a growing BTreeMap of every identity. Shared-row deduplication should happen before repeated hydration/decoding; ordinary database unique IDs can discharge storage deduplication, but conflicting same-ID content still needs exact readback. Total counts alone are insufficient for omitted/extra substitutions.

The pinned SDK already supplies `Query::stream_items()` at `surrealdb/src/method/query.rs:118–229`. Ordinary `.await` yields fully buffered IndexedResults. gRPC implements real `query_stream`; some other transports replay buffered results. `StreamItem::Row` is provisional until successful StatementEnd, and outer errors also matter. For private publication, provisional downstream inserts are acceptable only if any late source failure abandons the entire private realization before seal. For cold read-only audit, retain mismatch/hash state and report success only after every expected statement's terminal success and outer completion. This is a library capability, not LIVE SELECT and not a reason to embed core.

SDK item queue is 256 rows; that bounds count, not application bytes or total live working set. Server gRPC batch framing checks byte width but a single oversized record can fail. A plan must acknowledge variable text/payload size and combine projection/source streaming, bounded live source state and output batches. Splitting a built full Vec is not bounded construction. No universal accounting service is needed.

Specific graph/set/bulk alternatives considered:

- Native `INSERT ... (SELECT ...)`, bound row-array INSERT and `INSERT RELATION` retain sets across a boundary. Rust can still own canonical key construction and vector byte validation where SurrealQL lacks the exact domain codec. Do not invent a universal translator for Rust model rules.
- SurrealQL arrows permit edge-local WHERE/ORDER/LIMIT and grouped edge subqueries. Prefix/range compound indexes and indexed IN predicates can avoid repeated full scans; record links/FETCH hydrate useful related records, but FETCH of unlimited fanout can recreate the memory problem.
- `array::distinct`, grouping/aggregation and set functions can reduce repeated identities before transfer. They must preserve occurrence multiplicity and contextual witnesses; global dedup of endpoint pairs is wrong when parallel semantic relationships matter. Rust BTreeSet binds as an array, not native SurrealQL set.
- `SPLIT` and GROUP cannot appear together in one SELECT (catalog/upstream tests); separate subqueries would be required. GROUP ALL and count indexes can produce compact totals, but do not certify exact derived content.
- Factorized occurrence representation could remove the persisted Cartesian product if all required witness combinations are mechanically reconstructible. It changes physical querying and realization identity; treat it as a deliberate complete alternative with S1/S2 consumer changes, not an incidental chunking fix. Under the bounded remediation brief, keep existing complete occurrence semantics and stream them; do not silently truncate witnesses.

Evidence: skill graph/query/schema/bulk briefs; `surrealql.md` feature inventory; upstream `where_in_compound_index_order_limit.surql`, `compound_index_limit_start.surql`, `graph_target_vertex_fast_path.surql`. These demonstrate capabilities, not that the repository's specific expressions use the expected access path or are performant.

## Recommended F04 candidate: explicit read-only mapping reconciliation

Extend cold audit to reconcile every query-visible lowering against canonical admitted records and the retained consumed embedding specification/bytes. Canonical entity/assertion bytes and effective definitions remain the authority. Do not replay extraction, analyses or external embedding, mutate tables, rebuild search or create a second durable manifest of authoritative meanings during audit.

Inventory and compare:

1. Canonical row body plus **all scope fields and scope_keys**, including absent/null distinctions and whole-ID string lowering. `schema.rs:141–144` uses VALUE (write-time persisted) fields; correct current definition alone does not prove stored values remain correct. Recompute expected fields from canonical body/model mapping, not the row's potentially altered body alone.
2. Shared documents: exact family/table placement, key/text/digest and total set. A valid digest alone must not excuse altered text or document substitution.
3. Shared vectors: full specification/input identity, exact retained bytes/digest and numeric array lowering. Check array vs source bytes with exact f32 representation rules, including signed zero if admitted; JSON/HTTP conversions can lose `-0.0`, while the skill's gRPC value route preserved it. Numeric equivalence does not prove exact winning bytes.
4. Lexical/vector occurrences: ID, graph endpoints, family/unit/fragment/context/member/anchor/input, eligible and deterministic tie/occurrence keys, plus missing/extra rows. Validate required records are actual native relations; stored in/out fields alone on a plain INSERT do not establish graph adjacency (SB117).

Read-only SELECT/graph queries can generate compact candidate mismatches or scoped expected inputs. Exact row merge comparison can stream actual and expected rows in deterministic ID order; minimal expected streams may use scratch sort/spill to accommodate the arbitrary occurrence hash order. Independent reverse inventory/count reconciliation rejects extras, while per-expected-row validation rejects omissions/substitutions. Schema/definition/index identity remains a separate existing check. Use an explicit audit event/lifecycle boundary, not a rehash on each ordinary MCP request.

Share the declaration of the canonical-to-physical mapping with publication where it avoids divergent meanings. Keep independent adverse expectations/mutation controls to detect a shared wrong implementation; replaying the same producer and comparing it to itself does not become an independent oracle. Do not introduce certificates or query adoption qualification just to express this straightforward mapping contract.

## Whole-operation fit and alternatives outside adoption

Recommended units are one bulk cache intent, one private streamed realization, one explicit read-only audit and one checked gRPC backup. They remove item crossings, repeated vector/document preparation and retained whole-load inventories while respecting exact witnesses, immutable authority and final visibility. Short set operations and the SDK's genuine streaming route are a better fit than per-row transactions, extra generic retry frameworks or extra durable registries.

Use SurrealQL SELECT predicates, aggregation and graph traversal where they remove transfer/decoding or exploit declared indexes. Keep Rust execution for exact semantic hashes/codecs and available-vector validation; meaning ownership does not dictate execution placement. Index selection is not guaranteed by having an index. EXPLAIN (and newer planner EXPLAIN ANALYZE in upstream tests) is a focused diagnostic when a consequential access-path assumption remains unclear; do not make a separate per-query adoption gate, benchmark program or formal work model. Prefix/range and residual-filter tests specifically warn that LIMIT must follow residual filtering.

GQL is unnecessary here: it is a supported subset, no Rust SDK gql method, and Cypher MERGE/WITH/UNWIND/CREATE are refused. Materialized table views and COMPUTED fields are alternatives for reusable deterministic lowings; they buy database lifecycle/write work and can shift identity/audit obligations. No evidence here makes wholesale replacement beneficial. Live queries/changefeeds, GraphQL/MCP/Postgres front ends, DEFINE API/files/Surrealism, alternate embedded engines, DISKANN and new vector models offer no needed remedy for these three findings. The exact current embedding spec stays unchanged. No performance/throughput/capacity improvement is claimed.

## Checks and limits

- **passed** read-only baseline/pin inspection: `git rev-parse HEAD`, `git status --short`, Cargo.lock/source/skill reads.
- **passed** Context7 resolve and targeted docs queries for bulk/conflicts and selective queries; **blocked** gRPC finality docs match (no documentation hit), resolved with official pinned source.
- **not_run** all production compile/test controls, new probes, native-store journeys, concurrent same-key campaign, late export failure injection, cold drift controls and performance measurements, as assigned.

Historical skill probes were read as existing scoped evidence, never relabeled as new results. Actual gRPC conflicting admissions and full whole-operation working-set behavior remain implementation acceptance uncertainties. Source establishes the gRPC terminal-finality mechanism; it does not replace a later composed backup failure control. Coordinator decides adoption and owns the tracked remediation plans.
