# Versioned SurrealDB and SurrealKit contracts

**Interface-checked source research, 2026-10-12.** Supporting evidence for the
[principal review](../../reviews/design_review_surrealdb-lifecycle-target_2026-10-12.md), not an
independent architectural decision or runtime qualification. Context7 supplied initial official
documentation routes; the researcher inspected tagged source and primary documentation, and the
coordinator reread consequential SDK, server reference/event/GQL/invalidation paths.

The user selected **SurrealKit 1.0 beta as the target**, including its Rust library integration.
The concrete inspected tool version is `1.0.0-beta.1`; 0.7 is comparison context, not an open
stable-versus-beta selection. This is distinct from the project's SurrealDB SDK/server 3.3.0.
No dependency was installed or changed.

## Source identities

Repository baseline: `ad1c1020e6ce39fd14c7ff5e506d0197deff894a`.
Vendored SDK provenance: [README.lctx.md](../../../../third_party/surrealdb/README.lctx.md),
published 3.3.0 at upstream `238bfeb11f5725bebed370167656748df8067595`, archive SHA256
`f04b42991e6a01e2d01f1ea6183714cf364d3c40fa2197bd8993ec7834e0b4c4`.
The repository patches both the SDK and RPC framing; version 3.3.0 alone does not identify those
local contracts.

Server source inspection used the retained source checkout at:

```text
/home/paul/.local/state/library-context/surrealdb.tools/server-build/3efaf277b46867beb1e75f6cae56eb30dfd40878bbb823ef953213cf1cd84680/source/
```

`git rev-parse HEAD` identified the same upstream commit. This is source inspection, not a
connection to an existing installation. `SERVER/` below means that checkout. The timeout repair
is separate from the additional committed
[server execution-drain patch](../../../../third_party/surrealdb-server/grpc-execution-drain.patch)
and [its provenance](../../../../third_party/surrealdb-server/grpc-drain-provenance.json).
Patch presence and a build recipe do not establish adoption by any running server.

## SDK and remote finality

gRPC is a real 3.3.0 remote engine behind `protocol-grpc`; current documentation distinguishes
its incremental delivery from buffered WebSocket/HTTP results. Message ceilings apply to each
direction and indivisible records as well as batched messages; wasm32 lacks this engine.
[Connection documentation](https://surrealdb.com/docs/reference/rust/methods/connect)

The local adaptations are wider than a single EOF check:

| Contract | Inspected implementation | Qualification |
|---|---|---|
| Application End versus physical EOF | `third_party/surrealdb/src/engine/remote/grpc.rs:44-90,942-969` | Success drains the physical stream; late transport/protobuf errors and invalid terminals remain errors. Abandonment is different from checked completion. |
| Connection versus session | `third_party/surrealdb/src/lib.rs:338-357`; gRPC session task at `grpc.rs:1089` | Clone shares `Inner` but creates a fresh session UUID. Drop submits asynchronous detach; it does not return a cleanup acknowledgement. |
| Exact external transaction cancellation | `third_party/surrealdb/src/method/transaction.rs:40-50` | `cancel_ref` retains the owning session and transaction; invalidation does not acknowledge failed rollback. |
| Begin failure ownership | `third_party/surrealdb/src/method/begin.rs:21` | `retain_on_error` returns the consumed original client for explicit finalization. |
| Finite batch framing | `third_party/surrealdb/src/method/query.rs:395`; `third_party/surrealdb-rpc/README.lctx.md` | Execution future and delivery channel are polled together. Rows, Value, EmptyValue and Finished stay distinguishable; channel EOF alone is insufficient. |
| Server shutdown/finalizer ownership | `third_party/surrealdb-server/grpc-execution-drain.patch:59,154,779` | The local patch adds tracked finalizers, admission closure and joined shutdown; source existence is not composed runtime acceptance. |

**Invalidation is not transaction finality.** `SERVER/surrealdb/core/src/rpc/protocol.rs:1102-1112`
clears authentication and live queries, without cancelling transactions. gRPC cleanup attempts
transaction cancellation and logs failures (`SERVER/surrealdb/server/src/rpc/grpc.rs:649-681`).
Neither dropping a clone nor invalidating another session substitutes for retaining and settling
the actual transaction owner.

SDK configuration exposes timeouts, capabilities and gRPC controls, but no general
transaction-conflict retry loop. `FOR UPDATE` provides selected-record commit validation; it is
not a database-wide blocking lock. Snapshot reads alone do not exclude write skew.
[SELECT documentation](https://surrealdb.com/docs/reference/query-language/statements/select)

## Native references, events and disposal

Record `REFERENCE` supports IGNORE, UNSET, CASCADE, REJECT and custom deletion handling, with
reverse lookup. Plain record links alone do not establish target existence or retention.
[Reference documentation](https://surrealdb.com/docs/reference/query-language/language-primitives/record-references)

The pinned add-reference path writes a backlink key and optional inline-cache state
(`SERVER/surrealdb/core/src/doc/field.rs:877-944`); target deletion scans its backlink range
(`doc/purge.rs:455-478`, REJECT at 526). The inspected insertion path supplies no mandatory
shared target-record lock. Optional inline-cache coordination is not universal phantom exclusion.
**Concurrent reference admission versus deletion remains unresolved**, rather than demonstrated
safe or demonstrated broken. A native concurrency control or source-established common locking
protocol would settle that question before replacing application retention safeguards.

Synchronous events execute with the source mutation. Async enqueue is transactional
(`doc/event.rs:146-172`), but execution and external effects have separate failure boundaries:

- Pinned 3.3.0 increments attempts, retries within its configured count without backoff and
  deletes exhausted events after warning (`doc/event.rs:489-503`). Certain final errors also
  delete the queued record (`:507-526`).
- Current documentation's conflict-retry exemption starts in **3.3.1**, and cannot be credited
  to 3.3.0. External effects can repeat even when database effects roll back.
- At-least-once delivery does not supply exactly-once external work, an indefinitely retained
  failure obligation or guaranteed eventual completion.

[Event documentation](https://surrealdb.com/docs/reference/query-language/statements/define/event)

General elapsed-time record TTL and scheduling are not engine facilities described by the
current reactive-pattern contract. Indexed expiry enables candidate selection; writes trigger
events; a periodic controller must own time-driven work.
[Reactive patterns](https://surrealdb.com/docs/learn/schema-management/events-and-triggers/reactive-patterns)

`REMOVE DATABASE` immediately removes catalog reachability and enqueues the prefix for
asynchronous physical reclamation (`SERVER/surrealdb/core/src/legacy/expr/statements/remove/database.rs:48-67`).
Acknowledged logical disposal therefore differs from reclaimed disk space and disposal of
external artifacts. It also does not establish that an abandoned client previously had a
successful or failed native outcome.

Changefeeds retain opted-in mutations for a configured window; their consumer owns cursor
progress, replay and reconciliation after a retention gap. Live queries are subscriptions,
not durable replay. Versioned SELECT requires a version-enabled supported backend; version
history does not replace application immutable manifests or consumer obligations.
[Changefeeds](https://surrealdb.com/docs/learn/querying/real-time/changefeeds),
[live queries](https://surrealdb.com/docs/learn/querying/real-time/live-queries),
[versioned SELECT](https://surrealdb.com/docs/reference/query-language/statements/select)

## SurrealKit 1.0 beta target

The official pages describe `1.0.0-beta.1`; installation without an explicit prerelease selects
stable 0.7. Use an exact tool version and known lock/binary when qualifying integration.
The beta workspace's compatible SDK 3.2.4 requirement does not establish an exact patched 3.3.0
runtime. [Overview](https://surrealdb.com/docs/manage/schema-migration),
[beta manifest](https://raw.githubusercontent.com/surrealdb/surrealkit/v1.0.0-beta.1/Cargo.toml)

| Capability | Target use and boundary |
|---|---|
| Sync and watch | Fast schema iteration on owned disposable targets. Application runtime admission/installation must remain explicit. |
| Modules and targets | Partition schema-management ownership and order dependencies. Module names are lifecycle identities: renaming can make old objects stale. No transaction spans the database matrix. |
| Rollouts | Useful where a real retained consumer requires staged change. Early-design regeneration should not acquire historical compatibility obligations merely because the tool supports rollbacks. |
| Rust library / `embed_schema!` | Reuse schema application machinery over generated authoritative SurrealQL. Bundling schema does not decide semantic ownership or authorize applying it. |
| Declarative tests | SQL expectations, permissions, schema metadata/behavior and HTTP API cases. Useful for schema/effect boundaries, with exact backend, credentials and target ownership. |
| Type generation | Introspects a live database to emit JSON and optional TypeScript interfaces. It does not become the authoritative Rust semantic model. |
| JSON reports and `--keep-db` | Useful inspection/retention interfaces; their meanings must be integrated with the existing run and evidence owners. |

Beta module/target selection is fully honored by `sync`, while `test`, `seed`, `typegen` and
most rollout verbs have documented gaps. Avoid assuming one matrix config governs every command.
[Modules and targets](https://surrealdb.com/docs/manage/schema-migration/modules-and-targets),
[sync and watch](https://surrealdb.com/docs/manage/schema-migration/sync),
[testing](https://surrealdb.com/docs/manage/schema-migration/testing),
[type generation](https://surrealdb.com/docs/manage/schema-migration/typegen)

**Cleanup requires composition.** The inspected beta tester warns on cleanup error and still
returns its suite report (`tester/runner.rs:198-202`). Its cleanup helper discards statement
checks for database/namespace removal (`:863-880`). A reported test pass or successful helper
return therefore does not establish acknowledged disposal. Setup failure and aborted parallel
suites also require owner recovery. This is an integration requirement, not a reason to reject
the user-selected beta.
[Beta tester source](https://raw.githubusercontent.com/surrealdb/surrealkit/v1.0.0-beta.1/crates/surrealkit/src/tester/runner.rs)

Beta improves assertions: missing paths fail rather than silently passing as they could in
0.7, and row-permission probing was reworked. Historical stable results cannot inherit beta
assertion strength. Prebuilt beta CLI embeds memory only; persistent engines require matching
build features and exclusive datastore access. Memory controls cannot qualify persistent
storage, remote framing, authentication or patched server drainage.

Testcontainers can provide remote-server fidelity only with an explicitly matching image/binary,
engine, flags and patches. The inspected Rust module defaults to older `v2.2`, memory and
permissive capabilities; defaults are not project fidelity.
[Testcontainers module source](https://docs.rs/testcontainers-modules/latest/src/testcontainers_modules/surrealdb/mod.rs.html)

## Graph, retrieval and diagnostic capabilities

The pinned server has an active, default-enabled `POST /gql` endpoint, not merely an unused
parser (`SERVER/surrealdb/server/src/ntw/gql.rs:1-5,36,64`; server Cargo defaults `:23-35`).
It executes through session, capability and timeout plumbing. Its supported subset is not
general Cypher compatibility. [GQL overview](https://surrealdb.com/docs/learn/querying/gql/overview)

Lightweight relations and inline adjacency are real facilities, but property-free edges cannot
represent independently attributed relationship facts without loss. Inline caches can spill;
their source explicitly limits cap-change DDL coherence because a first-write backfill can race
an invalidation scan (`SERVER/surrealdb/idx/src/inline_cache/mod.rs:24-28`). Fixed-schema and
online-reconfiguration claims require separate judgments.
[Table documentation](https://surrealdb.com/docs/reference/query-language/statements/define/table)

BM25, HNSW, DiskANN and fusion functions exist at the pin (`core/src/fnc/search.rs:144,335`).
HNSW's graph needs RAM beyond vector-cache settings; DiskANN lacks wasm32 support. Current
documentation includes index/array fixes beginning in 3.3.1 that cannot be transferred to 3.3.0.
Native ranking must still preserve the selected universe, semantic relation meaning, evidence
and exact/heuristic distinctions. No performance gain was measured.
[Indexes](https://surrealdb.com/docs/reference/query-language/statements/define/indexes)

`EXPLAIN ANALYZE FORMAT JSON` can reveal operator work, but its output is explicitly unstable:
retain diagnostic evidence, not an unversioned stable product protocol.
[EXPLAIN](https://surrealdb.com/docs/reference/query-language/statements/explain)

CLI `validate` parses syntax without executing or proving schema/admission/permission behavior
(`SERVER/surrealdb/server/src/cli/validate.rs:30,76`). Studio complements interactive inspection.
Native MCP stdio exposes owner-authority database operations; authenticated HTTP remains a
general database surface. Neither replaces the product's publication-scoped domain API.
[MCP](https://surrealdb.com/docs/build/ai-agents/mcp/embedded)

Community provides metrics, traces/logs, Prometheus and OTLP. Enterprise adds durable audit and
slow-query pipelines and distributed-storage signals; `surrealctl` is Cloud management.
Edition/deployment availability and actual product need remain explicit.
[Observability](https://surrealdb.com/docs/manage/observability)

Logical export/import is not equivalent to the exact publication backup or cold service archive.
Official documentation identifies omitted record history and pre-3.3.1 ACCESS signing-key and
reference-key batching limitations. A quiescent physical copy, a portable dump and an
application-verified recovery closure have different contracts.
[Backup/recovery](https://surrealdb.com/docs/manage/self-hosted/backups-and-recovery)

## Database-hosted APIs, executable modules and file buckets

Follow-up checked 2026-10-12 with Context7, current official documentation and the same pinned
server source. These are SurrealDB 3.3.0 capabilities, separate from the selected SurrealKit beta.

`DEFINE API` supplies database-hosted HTTP handlers with endpoint permissions and middleware.
The pinned implementation checks endpoint permission, then disables table/field permissions for
the handler body (`SERVER/surrealdb/core/src/api/invocation.rs:117-151,183-185`). Omitted endpoint
permissions default to FULL, including anonymous callers. At the pin, HTTP response bodies must
be NONE, bytes or string (`server/src/ntw/api.rs:132`); object-returning `api::invoke` examples do
not establish equivalent HTTP response behavior. A bounded retrieval endpoint is a candidate,
but its authorization and deployment boundary must preserve Rust-owned operation/admission
contracts. [DEFINE API](https://surrealdb.com/docs/reference/query-language/statements/define/api)

Surrealism is built by default at the pin, with runtime experimental enablement separate
(`server/Cargo.toml:33`; `core/Cargo.toml:53`; `syn/src/parser/stmt/define.rs:226`). The 3.3 parser
requires `FROM` and `UNSIGNED` (`:319`); the latter anticipates signature verification rather than
establishing it. Modules load bucket `.surli` artifacts or Silo packages; cached instances retain
namespace/database-scoped state. Definition attempts runtime loading but logs failure
(`core/src/legacy/expr/statements/define/module.rs:100-125`), so successful definition alone does
not prove successful invocation. Relocating selected Rust computation would add an executable
artifact, provenance and lifecycle; provider extraction or semantic-authority substitution has
not been justified. [DEFINE MODULE](https://surrealdb.com/docs/reference/query-language/statements/define/module)

Buckets require experimental `files` and support memory, allowlisted filesystem and object-store
backends. Memory is nonpersistent. File put/get/delete delegate to a `BucketController`
(`core/src/fnc/file.rs:13-62`) with operation permissions (`core/src/buc/controller.rs:88,134,154`).
Delegation to `Arc<dyn ObjectStore>` does not establish transactional rollback of object writes;
atomic record-plus-object publication remains unresolved.

Removing a bucket removes its definition without deleting objects
(`core/src/legacy/expr/statements/remove/bucket.rs:43-50`). Export includes API, bucket and module
definitions without enumerating bucket bytes (`core/src/kvs/export.rs:141-155`). Restored pointers
are not restored content. Bulk-byte adoption therefore requires an independent immutable object
inventory, digest/provenance, borrower protection, retention and selected backup/restore contract.
Module executables stored there share those dependencies.
[DEFINE BUCKET](https://surrealdb.com/docs/reference/query-language/statements/define/bucket)

## Evidence limits and settling questions

These results are source/interface evidence, not fresh tested behavior. No concurrent-reference,
SurrealKit cleanup, recovery, load or end-to-end probe ran. The search covered the named pinned
server/SDK paths, committed patches, the selected capability skill, tagged SurrealKit sources
and linked official documents. No empty search was treated as proof of absence.

Before adopting a changed primitive, settle the exact consumer contract it would replace:
reference-create/delete exclusion; beta cleanup acknowledgement and crash disposal; SDK/tool
dependency coherence; server patch finalizer behavior; replay after retention gaps; and any
query/schema transformation's provenance, multiplicity, universe and complete-result semantics.
These are bounded follow-up questions, not a mandatory new qualification campaign.
