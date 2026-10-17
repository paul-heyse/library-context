# Publication library capability inquiry

2026-10-17. Baseline `7f353fa1fffd081eb9f25003b61cbea9f74ded67`.
Static source/document inquiry only. No services, builds, tests, environment sync,
schema changes or performance probes were run. Recommendations below are Proposed.

## Sources and applicability

`Cargo.lock` selects SurrealDB 3.3.0, Surrealkit 1.0.0-beta.1,
DataFusion 55.1.0, Arrow 59.3.0, Tokio 1.53.2 and Tower 0.5.3.
`Cargo.toml:186` patches the SDK/protocol/types, Surrealkit and Tower locally.
The SDK evidence below therefore uses `third_party/`, not an unpatched SDK.
SurrealDB core/runtime/expr and DataFusion evidence uses the locked sources under
`/home/paul/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (abbreviated
`REG/` below). This establishes source behavior, not the identity/configuration of
a running daemon. The existing service owner must establish deployed correspondence.

Selected skills inspected: `neo4j-surrealdb`, `datafusion`, `context7-mcp`.
Context7 resolved `/surrealdb/docs.surrealdb.com` and `/apache/datafusion`;
queried exact-record/locked reads, array/object indexing, and memory execution.
Current docs are discovery corroboration, not a version-pinned engine oracle.
`docs/surrealdb_technical_reference.md` was used only as a discovery directory.

## Complete-operation comparison

| Operation | Available composition | Material boundary |
|---|---|---|
| Observe bounded immutable candidates | One `SELECT * FROM $typed_ids`; associate returned objects by typed identity | Missing/forbidden rows disappear; no positional zip, no inferred absence from a missing statement, no locked-read guarantee |
| Lock commit-sensitive existing/absent identities | Scalar record targets with `FOR UPDATE`; retain present per-record maps, or investigate generated comma-separated scalar parameters | An array-valued parameter is rejected under `FOR UPDATE`; no predicate/range lock |
| Install immutable payload plus relationships and holds | Preflight complete equality; bulk ordinary `INSERT`, bulk `INSERT RELATION`, bulk holds, all inside the existing owner effect transaction | IGNORE/upsert alone cannot establish equal-existing acceptance or changed-existing refusal |
| Index repeated in-memory lookups | Typed scalar candidate index with full native comparison and stable candidates; compound fallback | String object indexes and generic Value hash maps do not preserve all present identity/equality semantics automatically |
| Recover ambiguous apply | Existing durable operation/request/owner/era receipt and absent-effect fence | Transactions and transport retries do not independently prove whether a lost COMMIT applied |
| Local relational preparation | Existing typed Rust preparation; Arrow/DataFusion for already represented finite relational data | Must preserve native NONE/NULL, numeric/compound-key equality, first-match and full-payload semantics; native admission still runs at commit |

### Native exact reads are a real batch capability

`REG/surrealdb-core-3.3.0/src/exec/operators/source_expr.rs:96` resolves
array record IDs through `batch_fetch_in_place`, then removes NONE/NULL results.
`.../operators/fetch.rs:432` applies permissions and computed fields;
`:464-516` overlaps individual record fetches and replaces their original array
positions. This is not evidence of one storage multi-get. Transaction-cache
reuse avoids repeated underlying reads of the same ID.
`.../exec/fan_out.rs:56-88` overlaps read-only work above the configured
`fan_out_row_threshold`, otherwise runs sequentially, retaining input order.
Array results lose missing positions after filtering: reconstruct requested
absence by exact identity and reject foreign/duplicate observations.
The current `control/held.rs:194-260` has these explicit expectation checks.

`.../dbs/statement.rs:189-243` accepts scalar record expressions for FOR UPDATE,
not arrays, ranges, tables or arbitrary field idioms. The planner rechecks
resolved parameters at `.../exec/planner/select/mod.rs:945-956`.
`.../operators/scan/dynamic.rs:267` also rejects a runtime array value.
Consequently an unlocked set read is useful but cannot silently replace locked
maps in `schema.surql:250,258,316`. A comma-separated list of scalar parameters
is a source-supported candidate requiring a focused probe, including missing IDs.

Official documentation: [SELECT](https://surrealdb.com/docs/reference/query-language/statements/select),
[transactions](https://surrealdb.com/docs/learn/querying/concepts-and-guides/transactions).
The documented absence conflict registration does not lock a predicate against
new matching records. Retain an exact shared write/conflict address for each
cut/admission invariant, such as the existing stripe/predicate records.

### Bulk writes already exist; immutable agreement is a separate obligation

`crates/lctx-native/src/control/schema.surql:229-300` already separates
ordinary rows, relation rows, original declarations and holds, performs full
existing-payload checks, and inserts only missing rows in bulk. It also preserves
parent/completed-boundary checks and owner/target existence. The row counts alone
do not describe all publication work.

The selected skill's recorded SB016/SB017/SB027/SB048 controls characterize
one-statement batch rollback, ON DUPLICATE KEY UPDATE, INSERT RELATION and
INSERT IGNORE. These are historical skill evidence, not this checkout's checks.
IGNORE silently accepts unique violations; ON DUPLICATE updates mutable content.
Neither is a substitute for throwing on a changed immutable full value.
Keep all payload, hold, incidence and receipt changes in the same transaction.

Official documentation: [INSERT](https://surrealdb.com/docs/reference/query-language/statements/insert).

### Index and equality semantics are consequential

`REG/surrealdb-runtime-3.3.0/src/array.rs:352-371`: `array::find` invokes
the closure sequentially and returns the first truthy match, or NONE.
`:297-317` filter and `:574-588` map also await closures sequentially.
Replacing a repeated filter with find can reduce allocation/early-stop work;
a nested find is still a sequential nested search, not an indexed join.

`REG/surrealdb-runtime-3.3.0/src/object.rs:22-61`: object::from_entries builds
`BTreeMap<Strand,Value>`, keeps string keys as-is, renders other keys via
`to_sql()`, and overwrites duplicate keys. A string spelling of a printed typed
key can collide; duplicate handling is last-wins rather than first-wins.
It is not a generic exact typed-key index. A tagged encoding must be proved
injective for its admitted domain and must preserve duplicate refusal/order.

`third_party/surrealdb-types/src/value/number.rs:310-345` hashes numeric
variants differently while equality compares across Int/Float/Decimal and
equates signed zero. A generic hash index cannot assume native equality is a
lawful hashing key. Compound keys recursively inherit numeric behavior.
`control/exact_index.rs:1-115` deliberately indexes string/integer/UUID scalar
branches and retains ordered compound candidate scans and final equality.
Use this existing preparation where it fits instead of replacing it with
serialized strings or a new universal key type.

`REG/surrealdb-expr-3.3.0/src/val/mod.rs:375-415` makes NONE equal only NONE,
NULL equal only NULL, and object equality compare the full object. SurrealQL
`!=` is Boolean negation of native equality (`surrealdb-runtime/src/operate.rs:46`).
This differs from SQL NULL three-valued filtering. Comparing only IDs, digests or
selected fields does not preserve the current full-object agreement contract.

Official documentation: [array](https://surrealdb.com/docs/reference/query-language/functions/database-functions/array),
[object](https://surrealdb.com/docs/reference/query-language/functions/database-functions/object).

### Parse, planning and execution placement

`REG/surrealdb-core-3.3.0/src/kvs/ds.rs:5168-5194` parses Text query sources
through the parser; AST and Plan sources are internal alternatives. The patched
remote SDK sends query text (`third_party/surrealdb/src/engine/remote/grpc.rs:1091-1150`).
The inspected remote API does not expose a prepared native plan handle.
This is bounded negative evidence about this path, not a claim that no internal
plan cache or alternate deployment extension could exist.

`.../exec/physical_expr/function/user_defined.rs:60-155` retrieves the stored
function definition, limits auth, checks permissions and arguments, binds a new
context, and explicitly replans its block on every call to see LET values.
A stored function is a maintainability/security owner, not a compiled stored
procedure eliminating per-row planning. Reducing repeated nested UDF calls by
preparing distinct targets and checks once can remove work without changing the
backend. Measure the complete effect; text length alone cannot establish cost.
`.../exec/fan_out.rs:36-48` keeps mutating fan-out serial because savepoints are
stack-based. Do not introduce concurrent writers inside one transaction body.

### SDK transport and recovery

The locally patched gRPC engine already dispatches concurrent calls on cheap
HTTP/2 channel clones (`third_party/surrealdb/src/engine/remote/grpc.rs:1-17`).
It separately orders session establishment/replay. No evidence here supports
adding a session pool as a publication performance prerequisite.

`third_party/surrealdb/src/method/query.rs:119-158` exposes finite batch streams
and explicit admitted framing. `:172-177` marks rows provisional until statement
success; errors retract prior rows. `grpc.rs:92-135` drains after application end
and preserves late wire errors; local refusal does not prove remote finality.
These contracts already answer bounded read ingress; streaming writes/results
do not remove an immutable effect's final commit/reconciliation obligation.

`control/authorization.rs:840-920` records owner/request/era identity and payload
inside one BEGIN/COMMIT, and refuses mismatched replay without rerunning payload.
`:1782-1795` reconciles an absent request by installing a conflicting fence.
`control.rs:79-85` recognizes a typed native transaction conflict plus one exact
backend message fallback. The application retry owner preserves request identity.
Neither generic Tower retry nor a fresh operation allocation can replace that
unknown-outcome protocol. No public Rust query/transaction retry method was found
in the patched SDK method sources; other SDK languages' `.retry()` docs do not
establish Rust support. Surrealkit is used by `tools/lctx-surreal-dev` for schema
embedding/sync/testing; it is not publication's transport or receipt engine.

### Arrow/DataFusion fit

DataFusion 55.1.0 supports hash-join null-equality policy explicitly
(`REG/datafusion-physical-plan-55.1.0/src/joins/hash_join/exec.rs:288-368`), but
default SQL joins are not full native object equality and ordinary joins can
multiply duplicates instead of choosing the original first candidate.
Use it when publication preparation already consists of Arrow columns with a
defined typed-key projection, and retain native residual equality where needed.
For the current small bounded `Value` windows, representation conversion plus
query planning can cost more than the existing Rust index; that is unmeasured.

`REG/datafusion-execution-55.1.0/src/memory_pool/mod.rs:43-65` expressly excludes
all intermediate streaming allocations from pool tracking. A MemoryPool is not
the whole publication working-set budget. New conversion buffers/index scratch
must stay in the existing owner reservation. Context7 describes streaming and
spill opportunities, but no DataFusion throughput or spill claim was transferred
to this publication workload.

Official documentation: [DataFusion architecture](https://datafusion.apache.org/library-user-guide/architecture.html),
[memory pool](https://docs.rs/datafusion-execution/55.1.0/datafusion_execution/memory_pool/index.html).

## Proposed target and revealing probes

Prefer one bounded prepared semantic effect: locally coalesce and index exact
candidate families, derive distinct guard/parent/target sets once, batch native
unlocked observations, validate complete returned inventories, preserve exact
locked commit dependencies, then bulk insert payload/relations/holds alongside
the durable outcome. Keep catalog/model authority and native conflict/receipt
authority in their current owners. This needs no new dependency or backend.
An indexing optimization must not weaken rejection of malformed or duplicate
input merely because production usually emits canonical IDs.

1. Compare existing maps, bound-ID set reads, and generated scalar-target locked
   reads against missing/present/duplicate/foreign typed IDs; include string,
   integer, UUID, array/object keys and two-table lookalikes. Test empty sets and
   permission filtering; verify both concurrent commit orders for absent IDs.
2. Compare any object/candidate index against the original first-match oracle:
   equal and conflicting duplicate full rows; nested numeric representations,
   signed zero; NONE versus NULL; extra/missing object fields. Compound domains
   require either proved equality-compatible partitioning or retained fallback.
3. Compare complete native immutable effects, not isolated array functions:
   missing/equal/conflicting rows, completed parents, changed incidence, holds,
   relations, original chunks, maximum revision, stale owner, and atomic rollback.
   Exercise unknown commit/replay and freeze/close races through existing owners.
4. Profile parse/planning/UDF execution, target-read work, native apply, receipt
   settlement and lifecycle separately, then report total time and owner memory.
   The known 4096-row/73-window 15.714s probe, 12.697s atomic apply and 84 effects
   including lifecycle describe their original run only. No speedup is established.

No consequential claim of a missing universal native publish API is made: search
coverage was current publication consumers, selected skill catalogs, patched SDK
methods and core/runtime execution paths above. Plugins/custom server APIs,
alternate engines and new deployed behavior were not exhaustively excluded.
