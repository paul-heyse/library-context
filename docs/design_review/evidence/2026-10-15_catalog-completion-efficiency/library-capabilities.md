# Catalog completion: native library realization inquiry

Examined 2026-10-15 at HEAD `24b0d08b6f1ed39a43b677ebc684806e07563aaf`.
Static review and documentation research only; no database queries, services, builds or probes.
These are source-established work shapes and proposed alternatives, not measured speedups or
assembled acceptance. Coordinator owns finding disposition and complete alternatives.

## Exact evidence boundary

Product SDK/RPC: vendored 3.3.0, including physical EOF, finite batch shape and external
transaction/session ownership extensions (`third_party/surrealdb/README.lctx.md`,
`third_party/surrealdb-rpc/README.lctx.md`). Server: upstream commit
`238bfeb11f5725bebed370167656748df8067595`, default+CJK Community build, with the exact
timeout-stack repair and dependency correction in `third_party/surrealdb-server/*provenance.json`.
SurrealKit: exact beta.1, manifest-only profile patch, separate development component.

Below, `S/` means the inspected exact server build source:
`/home/paul/.local/state/library-context/surrealdb.tools/server-build/3efaf277b46867beb1e75f6cae56eb30dfd40878bbb823ef953213cf1cd84680/source/surrealdb/`.
Its executor patch is identified by the repository provenance. Source citations under S are
local static observations, not assertions that every source branch has run in this review.

The shared `neo4j-surrealdb` skill supplies pinned probes and API routes, but excludes SurrealKit
and performance beyond one small-scale comparison. Current Context7 documentation resolved to
`/surrealdb/docs.surrealdb.com`, without a version-specific result. Current docs are discovery
and contract leads; the source above decides availability at this pin.

## LC-L01: disjoint retirement pages write a common global gate

**Observed.** `crates/lctx-native/src/control/schema.surql:176–181` defines
`fn::native_backup_clear`: read `native_backup_gate:current FOR UPDATE`, reject nonzero active
count, then increment its revision. Every claim, outgoing-hold and final-deletion page invokes
it (`control/retirement.rs:519,771,838`). Even jobs with disjoint objects therefore share a
written key. Transaction conflicts repeat the original SQL and bindings through
`control/retry.rs:145–210`; jitter cannot remove this underlying contention domain.

**Proposed, high-value narrow alternative.** Retain the checked `FOR UPDATE` read and remove
the retirement-side gate update. Backup acquisition/release still write count/revision
(`control.rs:716–739`). Pinned FOR UPDATE is optimistic snapshot-relative commit validation:
`S/kvs-rocksdb/src/lib.rs:1965–1988` implements `getu` using
`get_for_update_opt(..., &self.read_options)`; the read execution path is
`S/core/src/exec/operators/scan/record_id.rs:432`. Concurrent retiring readers need not write
the same gate, while a backup writer can still invalidate retirement's observation.
This is ordinary pinned 3.3 capability, not a new-version proposal or blocking-lock assumption.

**Stronger simplification to assess.** A source search for `acquire_backup_hold(` across `crates`
found only its definition. The actual selected backup takes a publication reader pin, then a
dedicated external snapshot transaction (`lctx-publisher/src/backup.rs:44–118`), and releases
it only after checked cancellation/session invalidation (`:180–200`). Determine whether the
global hold/gate still has a supported public consumer before carrying its entire lifecycle
forward. Search coverage does not establish absence of external callers of this public API.

**Countercases/qualification.** Gate revision exhaustion and observer contracts must be
reconciled; no runtime result is claimed. The revealing control is two disjoint retirement
transactions plus acquisition/release races in both commit orders on the exact RocksDB server.
The change must preserve active-backup refusal and interrupted-operation reconciliation.
It does not justify replacing per-object guards with predicate scans.

## LC-L02: immutable-write RPC batches repeat parent/incidence work per row

**Observed.** `control.rs:339–420` already groups rows in one effect; the issue is inside that
effect. `immutable_row_plan` loops each row, invokes `native_incidence_insert`, point-reads its
existing value and creates/inserts individually. For every compiler membership,
`schema.surql:167` touches the contribution, then `:186–200` calls
`native_contribution_incidence`, which touches it again and rereads the same parent FOR UPDATE.
Repeated members of one contribution repeat identical parent eligibility checks and predicate
updates. The generic operation wrapper also performs durable intent, payload, and (unless retained)
outcome-reference release transactions (`authorization.rs:740–807`).

**Proposed.** Borrow the existing retirement lowering pattern (`retirement.rs:769–811`):
deduplicate exact affected records, validate each shared parent once, acquire/check exact
object guards, compare full existing values, then group new ordinary rows and relation rows
into native array INSERTs. Preserve admission fencing, original-blob completion checks,
full-value collision refusal, declared table families and complete batch rollback.
Pinned INSERT/INSERT RELATION are available; no dependency change is required.

**Countercases.** Do not replace exact collision checking with INSERT IGNORE or unconditional
ON DUPLICATE updates. `S/core/src/doc/insert.rs:35–108` shows per-row savepoint/update/ignore
behavior; bulk syntax still has real per-record engine/index work. Deduplicated touch is not
a blanket rewrite: `control/completed.rs:300` and `control/live_view.rs:268` rely on an exact
observed predicate revision plus one, so each operation's invalidation meaning must be preserved.
Durable intent is a recovery feature: collapsing its transaction needs a complete account of
discoverable in-flight effects and uncertain commit, not a round-trip count alone.

**Qualification.** Static evidence establishes repeated work, not its runtime share. A focused
control should combine repeated parents, pre-existing equal rows, a late different row, mixed
relations and interruption; only then compare realistic whole-batch latency and retry behavior.

## LC-L03: progressive finality does not establish request-budgeted transport ingress

**Observed.** Native authentication uses `Surreal::new::<Grpc>` with no caller Config
(`reader.rs:3542–3557`). SDK response decoding adopts the server-advertised message limit
unless a caller supplies `GrpcConfig.max_message_size` (`grpc.rs:190–227,2239`).
`query_stream` sends `max_batch_records=0` (`:1142–1158`). Server chooses up to 256 records
and splits by its wire byte budget (`S/server/src/rpc/grpc.rs:105–120,3042`), not this request's
Rust resource budget. `stream_batches` has a two-batch raw queue (`method/query.rs:122–138`;
`surrealdb-rpc/src/stream.rs:32`). Values are already decoded before delivery and budget charging;
the repository says so explicitly (`reader.rs:540–546`). Row-stream admission similarly occurs
after SDK queue allocation (`reader.rs:3242–3250`).

**Proposed.** Make the ingress contract explicit at the transport owner: a suitable caller decode
bound plus query-specific batch/byte negotiation or accounted decode workspace, selected with
the operation's row widths and SINGLE shape. A lower decode bound alone can reject a valid
server batch; it does not ask the server to emit smaller frames. The pinned protocol has
`max_batch_records`, but this SDK entry point does not expose it. Server byte framing and
client object expansion/queue coexistence must be considered together.

**Countercases.** This is a known accounting boundary, not proof of OOM or a server RSS claim.
Do not impose worker/thread/job caps. Keep throughput-oriented adaptive batching and legitimate
wide values; do not replace stream ownership with buffered query results. Quantitative memory
claims require measurement; source is sufficient to reject a claimed request-budget bound over
pre-delivery allocations.

## Supported mechanisms worth retaining, with their actual limits

**Set receiving is already present.** `control/live_view.rs:156–233` deduplicates and pages exact
contributor CAS plus holds. `installed_view/receiving.rs:77–127` then rereads current installation,
contributor full bytes/incarnations and source holds. Assess the additional uncertainty covered by
that postflight with the compiler mapper before removing it. The source audit is operation-local,
pinned, charged and shared (`installed_view/audit.rs:33–150`); portable metadata cannot substitute
for an independently earned receiver. There may be further reuse within the same protected
operation, but blanket claims that every receiver replays every semantic validator are unsupported.

**Planner-aware keyset is deliberate.** `control.rs:94–126` uses `type::record(range)` to select
DynamicScan and push LIMIT into range lookup. `S/core/src/exec/operators/scan/record_id.rs:238–255`
passes no limit to a helper returning Vec; `scan/dynamic.rs:306–337` forwards limit/start.
Do not simplify the cast away merely because literal range syntax looks cleaner.
The repository already has native keyset controls (`control.rs:1050–1147`). EXPLAIN shape alone
is not a measured end-to-end speedup or proof that all generated predicates remain indexed.

**Scalar locked maps are pin-shaped.** The retirement page maps deduplicated IDs to ONLY FOR UPDATE
reads before set comparisons/mutations. `S/core/src/exec/operators/scan/dynamic.rs:267–273` rejects
a FOR UPDATE source unless it is one RecordId. Replacing the map with one bound ID array loses
the admitted locked-read path at this pin. Ordinary SELECT arrays remain useful for non-locking
preflight; future multi-get locking would need a qualified adapter/engine path.

**Physical finality is substantive.** SDK `grpc.rs:71–88,935–985` drains query transport after End
and composes late errors. `method/query.rs:392–418` joins delivery with execution. Native
`AcceptedValue` (`reader.rs:470–535`) checks ordered statement inventory and SINGLE/Rows shape
and refuses subscriptions; cancellation drains rather than proving remote finality from Drop.
Replacing it with `stream_items`, a transport clone or ordinary buffered queries changes these
contracts. `Surreal::clone` creates another session (`lib.rs:337–350`), while Arc clone borrows
the same one. Backup uses `retain_on_error`, `cancel_ref`, `invalidate_session` to preserve exact
finalization handles; stock 3.3.0 evidence does not establish these extensions.

**Arrow enum is not an implemented fast path.** The pinned server explicitly rejects an
Arrow-only accepted encoding (`S/server/src/rpc/grpc.rs:1342–1351`: server does not serve columnar
query results). SDK sends an empty accepted-encodings list and rejects Arrow payloads
(`grpc.rs:806,1022`). A native Arrow alternative is unavailable in the inspected implementation;
another version/adapter requires separate research. An enum or architecture aspiration is insufficient.

## Graph, references, schema and development substitution limits

Native relation tables already use ENFORCED and explicit incoming/outgoing indexes
(`schema.rs:181`). Native graph traversal/reference storage may improve particular selections,
but must preserve selected-view membership, attributed direction, multiplicity, isolates and
terminal ownership; table-wide traversal is not equivalent by itself.

REFERENCE writes reference keys and optional inline caches (`S/core/src/doc/field.rs:873–946`),
and deletion scans incoming references and applies REJECT/CASCADE/UNSET (`doc/purge.rs:429–560`).
This is useful database-local integrity, but does not establish the current incarnation/admission/
recoverable-outcome protocol. Inline cache source (`S/idx/src/inline_cache/mod.rs:12–39,270–309`)
names coherent fixed-cap snapshots, common-key writer conflicts, sticky spill, and an unresolved
cap-change/backfill race. No full concurrent-reference replacement claim is justified from these
source paths. A proposed reference replacement needs actual add/delete/recreation races on the
selected backend; no absence or engine-defect conclusion follows from not running them here.

SurrealKit beta.1 offers real embedded Sync and declarative tests, but its generic tester is not
native lifecycle authority. `tester/runner.rs:121–145` aborts suite tasks on setup/join failure;
`:212–217` logs cleanup failure while returning a report; `cleanup_suite_db :913–931` discards
statement-level `resp.check()` failures. The existing development adapter deliberately uses
`fail_fast=false,keep_db=true` (`tools/lctx-surreal-dev/src/worker.rs:292–300`) and whole-target
disposal, with generated authority and installed definition postflight. This is a justified
containment countercase, not evidence that beta cleanup can replace production finality.

## Documentation and diagnostics evidence

Current official transaction docs retrieved through Context7 support record-ID-only optimistic
FOR UPDATE, not blocking or predicate locking:
https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/learn/querying/concepts-and-guides/transactions.mdx

Current observability docs mix Community metrics with Enterprise DsMetrics/audit/slow-query
observers:
https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/manage/observability/enterprise-observability.mdx
The concurrent operation-diagnostics evidence folder now contains a README with exact patched
Community-binary observations. Its memory-engine runtime, source-only SDK metadata inspection
and unmeasured overhead remain its limits. The coordinator owns its reconciliation and diagnostic
recommendations; this inquiry neither reran nor adopted its conclusions.

Current SurrealKit testing docs describe suite-created databases and automatic filesystem sync:
https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/manage/schema-migration/testing/index.mdx
Exact beta source and the contained adapter above qualify any transfer.

## Check outcome

Read-only source searches and documentation reads completed. Runtime checks, builds, performance
measurements and whole-product qualification: **not_run** (assignment excludes execution).
No production, shared skill, concurrent evidence, configuration or other documentation was edited.
