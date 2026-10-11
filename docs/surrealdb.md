# SurrealDB operator runbook

**Accepted target / implementation in progress, 2026-10-09 (ADR-0143).** The
[unified plan](plans/unified-persistent-surrealdb-plan_2026-10-09.md) replaces private compiler
databases, sealing and disposable fixtures with one durable service and exact published views.
The [persisted coordinator](plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07)
owns current receipts; [STATUS](../STATUS.md) distinguishes implementation from qualification.
Existing operator stores, client registrations, real-library pilots and selection remain held.
The newly owned library-context service is a separate explicitly installed target.

## Server and configuration

`just service install --installer target/release/lctx` explicitly installs the pinned native
SurrealDB3.3.0 RocksDB service and schemas. It verifies binary digest/version, creates the owned
`library-context-surrealdb.service`, and records installation identity, storage and generation at
`${XDG_STATE_HOME:-~/.local/state}/library-context/surrealdb/installation.json`. Set
`LCTX_SURREAL_SERVICE_CONFIG` to that manifest when using another selected location. State and
coordination belong outside checkouts. Neither ordinary commands nor worktrees start a substitute.

The stable namespace is `library_context`, with `main` and `validation` databases. Main contains
canonical payloads, exact memberships, manifests, attempts, effects, pins and cache tables.
Validation contains synthetic test data under the same schema. Tests never inspect/reset main.
No database per attempt or test, scratch fallback, job/thread cap or fixed memory cap is selected.
Default engine durability and background maintenance remain enabled. Existing query and transaction
deadlines remain20s/10s. The128MiB gRPC ceiling accommodates admitted indivisible rows; it does
not remove bounded native ingress or increase deadlines.

Private `main-runtime.json` and `validation-runtime.json` use database-scoped writer credentials;
`*-installer.json` files carry Root credentials only for explicit maintenance. All are mode0600.
Runtime configuration identifies endpoint, namespace/database, service generation, authentication
scope, credentials, same-database cache tables and selection path. Generation is a full32-byte
identity, separate from client builds and semantic content. Ordinary readiness verifies the
installed model/physical schema and generation and never performs DDL.

CLI defaults resolve the installed main runtime; `LCTX_COMPILER_RUNTIME_CONFIG` selects a
validation runtime for managed controls. Explicit `--runtime-config` takes precedence. Missing or
incompatible state refuses before acquisition and names the repair. VIEWER credentials alone
are not a capability for one publication in a shared database. Product readers use the trusted
Rust boundary, exact publication views and durable pins; credentials never enter handles/logs.
The private serving configuration carries the database principal needed to create and release
pins. The Rust/PyO3 domain session exposes read operations and never exposes those credentials
or arbitrary native SQL to product callers.

```sh
just service status
just service check
just service maintenance --check
# Explicit disruptive operation, after borrower drainage:
just service maintenance --restart
# Revalidate a failed maintenance operation; unresolved ownership stays closed:
just service maintenance --recover
```

Maintenance closes host and native admission, drains identified borrowers and native effects,
checks exact service/storage identity, then performs the selected operation. Reopening requires
native reconciliation and compatibility. Local child cleanup, a missing heartbeat or an empty
process group cannot establish remote terminality. Unresolved ownership keeps admission closed
with its recovery route. Never synchronize a native extension while guarded clients are live.
Disruptive native controls use `just service maintenance --native-clients -- COMMAND`: the
owner admits only that child, closes and drains it afterward, then revalidates before reopening.

A failed client may leave durable pins requiring explicit recovery. After exact predecessor
processes and their cleanup receipts establish drainage, use `just service maintenance --recover
--reconcile-database validation --reconcile-pin <64-hex-id>` (repeat for each named pin; named
backup holds use `--reconcile-backup-hold`). The service owns the exclusive host lease and checks
released-client cleanup evidence. Native reconciliation requires installer authority, closed
admission and an explicit readers-stopped assertion; it fences unresolved effects before guarded
release of only the supplied identities. It never discovers and releases all pins automatically.
The underlying trusted operator command is `lctx store reconcile --runtime-config ROOT.json
--pin <64-hex-id> --readers-stopped`. Missing identities or unresolved ownership preserve closed
admission; pin age, empty views and a lost heartbeat are not drainage proof.

Patched gRPC3.3.0 remains the supported product transport. Progressive rows are provisional until
checked statement/application completion and physical EOF; late errors and cancellation retain
their disposition. WS/HTTP alternatives require equivalent streaming, cancellation, terminal-error
and backup contracts. No transport switch or dependency bump is implied.

The owned server build route applies the pinned timeout-stack repair separately from the SDK
backport. It retains the shipped default+CJK features and existing runtime deadlines. Source,
lockfile, patches, compiler/build recipe and executable identity belong to its immutable descriptor:

```sh
uv run --no-sync python scripts/surrealdb_server.py --service-directory /absolute/private/service build
just service maintenance --server-generation /absolute/private/service.tools/surreal/IDENTITY/record.json
```

Select the matching installation through `LCTX_SURREAL_SERVICE_CONFIG` when its location differs.
Building publishes an owned executable generation without installing it. Explicit maintenance
drains and stops the daemon, durably switches the unit and installation pointer, then verifies
the exact executable/HTTP identity and both database scopes. A separate immutable handoff journal
retains predecessor/successor generations and supports exact retry after interruption. During an
interrupted native schema upgrade, handoff preserves its native operation, credentials and scope
checkpoints and leaves admission closed; installer resume remains a separate action. Cold service
archives carry descriptor and executable bytes and require neither source caches nor a rebuild.
These mechanisms are source-accepted with mocked controls; the persisted coordinator records
actual server build/adoption and native qualification separately.

## Compilation products and publication

Compilation stages immutable model/codec-qualified payloads under a durable attempt. Full nominal
anchors and exact view membership resolve graph targets without global nominal-key ambiguity.
Equal canonical content shares storage; unequal revisions may coexist in different views but
conflict within one selected view. Original bytes and full generated backing remain authoritative.

A completed contribution is not automatically admitted. After private core admission, compatible
retained products can attach compact membership pointers to current ownership without canonical
row replay. Current producer specification, exact dependencies, provenance and semantic premises
must agree. Optional detached portable products still undergo typed and semantic checks before
ingress. Optional `reuse` storage uses the configured database with its own tables, quota and
absolute shared lease directory; it does not create another database. Hash equality grants no
semantic authority. Mutable caches cannot own published consumed vector values.

Ordinary publication commits an immutable manifest over admitted views and a named executable
definition epoch. It does not seal the database or re-import its own export. Handles bind publication,
semantic/realization identity, exact view inventory and service generation. Definitions cannot change
beneath a pin. New publication does not select it; changing selection affects new sessions only.
Failure may retain compatible completed work. Unknown effects reconcile by durable operation ID;
no arbitrary pending task resumes and process exit cannot authorize shared-data deletion.

```sh
lctx store check
lctx compile fastmcp --through catalog --runtime-config /absolute/main-runtime.json
lctx compile fastmcp --through catalog --artifact-only --output build/admitted
lctx publish-artifact build/admitted
lctx snapshot select build/native/handle.json
lctx snapshot show
# Model-declared relation, scoped to the selected publication; arbitrary SQL is not exposed:
lctx snapshot query packages --limit 5
```

These examples document interfaces, not authorization for real-library qualification or selection.
Build the current native Python extension before clients run. Running clients retain their exact
handle, pins and charged preparation; close drains owners and releases pins before invalidation.
`lctx snapshot list` lists manifests in the configured database. `audit HANDLE` independently
reconciles the selected content/state and trusted definition epoch; it does not certify credentials,
live subscriptions, unrelated publications or database-wide semantic completeness.

## Embeddings and native search

Use the selected full4096 Qwen specification and the existing custom inference service. The
encoder identity includes the actual checkpoint/tokenizer/runtime/output semantics; document and
query recipes are independently identified. Complete document inputs are admitted with the acquired
local tokenizer at2048 tokens. Query admission uses its separate8192-token recipe. Compilation
opens cache/inference only when requested; exact normalized full winners and their declared
normalized1024 prefix projections enter admitted content and replay without the mutable cache.
Search and E1 analytics share that explicit1024 representation; bounded search rescoring uses the
full winners. MCP's optional
`--embedding-url` names the existing service. Omission disables vector search; service failure
reports degradation while native lexical search remains available. No service is started or
upgraded implicitly. BM25 and shared-vector HNSW candidates are restricted to eligible contextual
occurrences before distinct target/context channel caps. Exact name/path/option matches have an
independent prioritized lane, including literal matches with zero BM25. Session-retained ranked
continuations reuse the initial candidate pool and query vector under the original snapshot,
request and policy. Analytical exact-neighbor contracts remain separate.

## Export, restore and retirement

Analytical exports preserve named universe, roles, semantic IDs, isolates, parallel arcs, lineage,
coverage and declared losses. They are distinct from recovery backups.

```sh
lctx snapshot export --projection <name> --input <32-hex-id> --context <32-hex-id> --output graph.json
lctx snapshot backup --handle build/native/handle.json --output main.surql
lctx snapshot restore main.surql --publication <64-hex-publication-digest>
lctx snapshot retire build/native/handle.json --readers-stopped
```

Logical backup retains the requested publication pin and selects its recoverable closure through
bounded indexed reads in one external database transaction. The manifest, exact completed views,
originals and pinned definition comparison metadata come from that same snapshot. Application
terminals, physical EOF and explicit transaction cancellation must all succeed before staged bytes
are durably published. RocksDB snapshot reads preserve concurrently retired rows. Unrelated
retained publications are not exported. Actual selected role/search rows remain inert source
claims: restore independently regenerates their target rows and compares the complete claims
before admission. Executable definitions are generated from the pinned native blueprint, never
executed from dump text. Credentials and live runtime authority never enter the logical dump.
The protected whole-service archive remains a separate recovery checkpoint.

`just service backup /absolute/private/service.tar` closes admission, drains borrowers/effects,
stops only the owned daemon, captures its cold data, pinned binaries, private configuration,
credentials, selections and canonical coordination, and restarts/revalidates it. The archive is
mode0600 and must lie outside owned service state. `just service restore ARCHIVE` validates and
stages without changing live state; `--apply` explicitly restores only the matching owned
installation/generation under maintenance. Recovery validates asset digests and preserves the
predecessor until readiness succeeds. Failure keeps admission closed and retains recovery assets.

An installed control-format change uses the explicit journaled upgrade route:

```sh
just service maintenance --upgrade-installer /absolute/path/to/current/lctx
```

The owner closes and drains both databases, journals the exact old/new executable and schema
identities privately, rotates credentials and restarts the owned daemon before native migration.
The candidate verifies the exact source marker, adds checked declarations, translates legacy
state with named outcome references and publishes the new marker last. The executable pointer
moves only after both database checks succeed. An interrupted upgrade remains closed; repeat
with the same candidate to reconcile its journal. Ordinary attachment and `store init` refuse
an incompatible installed format. This transition is a schema migration.

If the candidate itself is defective, the explicit recovery route is:

```sh
just service maintenance --replace-upgrade-installer /absolute/path/to/corrected/lctx
```

This creates a distinct immutable host successor and preserves the failed executable, private
journal, native intent and rotated credentials. After drainage and owned-daemon restart, each
unpublished scope must retain its exact closed source3 marker and absent/intent-only native journal.
Partial declarations are reconciled; potentially translated unpublished scope refuses replacement.
If a scope already published, replacement requires an identical complete target declaration plus
its exact closed target4/current-generation marker, published native journal and durable completed
checkpoint. That scope is preserved and skipped. Remaining scopes resume the original native
migration operation; the new host operation owns the immutable candidate bundle and diagnostics.
A different target is allowed only before any scope publishes. Ordinary resume still requires the
same candidate. The persisted coordinator owns actual qualification; this route does not imply a
successful migration. Installation uses concurrent index construction and waits for every desired
index to report ready, including catalog-present indexes on a retry; source identity is unchanged.

**Accepted recoverable transition (ADR-0146); runtime qualification open.** A separately checked
advanced-state recovery uses `just service maintenance --reconcile-upgrade-installer PATH`.
It preserves the original native migration/credential operations and exact completed main scope,
then classifies actual validation state before adopting only recognized partial forms. The original
replacement route remains absent/intent-only. `--upgrade-step-pages N` on reconciliation stops at
a durable validation checkpoint with admission still closed; it does not limit normal compiler or
test parallelism. Continue only the exact staged successor through `--upgrade-installer PATH`.

The host stages each database's versioned execution contract privately and passes its exact path
to `store upgrade --execution-contract PATH`. Contract metadata is separate from `store schema`.
Preflight and all native passes carry atomic page progress, including nested cleanup cursors.
Unknown acknowledgement reconciles that revision; normal advancement retains one native client.
Native sealing, executable-definition completion and host scope completion are distinct. A sealed
scope with a missing host checkpoint finishes definition checks rather than replaying translation.
Migration overlays follow cold-service recovery, are excluded from selected portable content and
remain protected until their named terminal recovery consumers release them.

Native keyset scans retain native record IDs and excluded cursor bounds. On pinned
SurrealDB3.3, the shared helper wraps the range in the identity `type::record(...)`
so `DynamicScan` receives the page limit. The literal `RecordIdScan` path eagerly
reads the remaining suffix before its outer limit. Regression controls require
the pushed limit and exact complete page bodies/order; a small returned page alone
does not demonstrate bounded examined work. This physical refinement preserves
the migration protocol, schema, acknowledged prefix and runtime limits.

The format3→4 recovery keeps its complete target unchanged. The subsequent explicit format4→5
transition supplies bounded last-page history outcomes and expected-revision replay. Ordinary
`store init` cannot install that changed runtime meaning; the explicit transition owns readback
and publication. Existing era/horizon/reference protections continue to govern collection.

For compatible executable-epoch installation, run the current build's `lctx store
--runtime-config INSTALLER_CONFIG init --keep-closed` through `just service maintenance -- COMMAND`.
It preserves existing era/fencing watermarks and prior pinned functions. The owner checks and
drains both databases before reopening. Ordinary compilation/publication verifies definitions
and never installs them.

Native history maintenance also requires closed admission, actual host borrower drainage and
Root installer authority. `lctx store --runtime-config INSTALLER_CONFIG history qualify
--evidence HASH` records the reviewed consumer qualification before `history cut` permanently
closes the old issuance era. `history compact --limit 128` creates a bounded persisted collector;
`history resume --identity HASH --expected-revision REVISION --limit 128` continues that exact collector. Named outcome
references, unresolved work, provenance and permanent object/authorization fences survive.
This is an explicit owner operation, never an automatic age-based deletion policy.
`store recover --cleanup HASH --retirement HASH --limit 128` claims distinct current-era
successors for exact interrupted obligations; it prints successor identities before continuation.
Retrying a predecessor returns its existing successor and never refreshes the old request's era.
Run these commands only through the service maintenance owner; a schema-ready marker alone
is not evidence of drainage or permission to collect history.

Restore parses closed literal data locally and lowers it through typed staging, independent
semantic admission and trusted executable generation. Dump SQL is never sent as Root-authorized
commands. Imported control records cannot recreate grants, live attempts, fences, pins or visibility.
Malformed/dynamic data, administrative commands and control-ID injection refuse. Concurrent imports
receive fresh logical ownership; failure does not remove unrelated shared content.

Retirement refuses selected or pinned publications. The native owner serializes reference changes
and retirement through exact guard records, rechecks reachability, and protects shared surviving
content. The `--readers-stopped` declaration is not evidence sufficient to bypass native pins or
unknown effects. Storage-manager policy delegates to this owner. Worktree removal never deletes
service storage, canonical payloads or shared caches.

## Focused controls

`just fixture -- COMMAND` borrows validation on the installed stable service and supplies
`LCTX_SURREAL_TEST_CONFIG`, `LCTX_COMPILER_RUNTIME_CONFIG` and a logical attempt identity.
Command exit releases that borrower; it never stops the daemon or tears down a database.
`--list` observes ownership; `--recover ID` resolves identified local cleanup without assuming
remote drainage. Retained serving outputs carry exact handles and content identity.

Use `just verify --print --select FAMILY:BOUNDARY` to inspect commands and focused selectors
for implementation. Native Rust/Python/CLI controls share the installed service with independent
logical ownership and normal available parallelism. Pure model/finite kernels need no database.
Restart-bearing controls drain the producer, perform explicit exclusive maintenance, then attach
a fresh consumer. Fresh/cold negative controls execute their intended validation; retained content
cannot substitute cached success. Infrastructure/readiness blocking is exit75; unexplained process
endings remain failed with unknown cause.

## Maintenance diagnostics

Diagnostics use synthetic validation content on the stable service. SQL and invocation-scoped
native MCP require explicit maintenance admission:

```sh
printf 'RETURN 1;\n' | surreal validate --stdin
just service maintenance -- just fixture --sql query.surql
just service maintenance -- just fixture --mcp --non-sensitive
```

Native MCP uses the existing server's `/mcp`, never `surreal mcp` with another datastore. The
launcher selects validation and passes only database-scoped credentials through child-only routing.
Main/operator configurations are refused. Native results can contain sensitive data; raw MCP is
limited to explicitly synthetic, non-sensitive controls. Check every statement status, including
`has_errors`, and preserve truncation and terminality boundaries. Sanitized harness errors omit SQL,
bind and credential values. Diagnostic controls establish tooling capability, not product acceptance
or measured effectiveness. Prior AF receipts retain their original disposable-fixture scope.
