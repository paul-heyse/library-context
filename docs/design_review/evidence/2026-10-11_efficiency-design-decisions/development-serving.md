# Development and serving decisions

**Interface-checked decisions, 2026-10-11.** Baseline `439f3d2d`; product implementation and finding
closure remain **Planned**. Main-tree writes are the assigned two plans and this evidence subtree.
The owned `efficiency-decisions` worktree carries disposable probe inputs; retained sources/results
below use this dated evidence folder's Git owner and the efficiency coordinator as their consumer.
No shared service, environment synchronization, real-library campaign, generator, or repository
quality gate was run by this lane. The coordinator owns the dedicated native disconnect campaign.

## D02: contract placement and responsible implementation identity

`lctx-embed/Cargo.toml:12` currently links `cpg-core`; its source imports `CoreError`, `EmbedFuture`
and `Embedder` from that compiler. `cpg-core/src/embedding_service.rs:7-28` defines the service trait
but already reexports model-owned `Spec`/`check_vector` and returns the model's tokenizer contract.
Select **the existing `lctx-model::domain::embedding` owner** for `Embedder` and its future, returning
`ModelError`; compiler adapters translate to `CoreError`. Keep fake implementation/compiler policy
outside the pure contract. C01 independently selects `domain::producer_contract` for fact inventory
and immutable acquisition/capture descriptors, with executable provider constructors in `cpg-extract`.
This removes the real embedder-to-analyzer dependency without creating a contracts crate per type.

The publisher's production dependency on `cpg-core` is substantive: `native_publication.rs:2,18-20`
consumes three compiler artifact types; `restore.rs:933-935,1046` constructs `Workspace` and performs
semantic restored admission. Those artifacts own `Arc<Workspace>` (`artifact.rs:222,445,952`).
Select a **lower native compiler-store/admission boundary**, physically separate from analyzer
orchestration, for native store handles, completed-view admission and publication capabilities.
The planned `lctx-compiler-store` package consumes model descriptors plus the native data plane;
`cpg-core` invokes its validated admission and hands its capability to the publisher. The capability
has private fields/validated construction; a manifest, arbitrary native handle or boolean is never
admission. Detached restore uses that same independent semantic admission, rather than importing
the extraction orchestrator. Publisher budget defaults come from the resource/config owner, not
`WorkspaceOptions`. Ordinary compiler/publisher/native-read consumers migrate together.

Physical native responsibilities need an actual split, not a Cargo feature illusion. Keep common
codec, installed-definition/control identity, canonical ingestion, transport lifecycle and selected
semantic admission in the lower native dependency closure; extract compiler-store orchestration
and DataFusion providers from the current omnibus `lctx-surrealdb::compiler`. Serving preparation,
presentation/ranking and reader-only adapters stay above that closure. Shared selected-read or
codec changes that alter admission are **responsible semantic changes**, not reader-only changes.
The extraction identity excludes reader/serving code; compiler identity excludes only physically
separated serving implementation, never shared native admission or N08 derivation implementation.

`scripts/producer_fingerprint.rs:117-195` walks every declared normal/build local dependency,
including optional/cfg declarations, and entire production directories. It also captures local
patches, lock/toolchain/manifests, runtime scripts and model specs. Retain this conservative full-byte
closure until physical migration removes a dependency. Reject feature-only fingerprint shrinking,
an ad hoc filename allowlist and a new cache key that omits ingestion/codec/derivation semantics.
The D09 placeholder experiment is solely Cargo/Hakari **freshness**; it must never replace producer
source/runtime-asset bytes. Future closure independently mutates serving-only, model, provider,
codec, ingestion, runtime asset and N08 derivation inputs, and inspects actual normal/build closures
and focused builds of embed, publisher and serving consumers. No dependency/pin change is made here.

### Parquet: retain the supported public capability

Source search covered production/test Rust under `crates/`, Python and scripts for literal
`read_parquet`, `register_parquet`, `Parquet`, `CREATE EXTERNAL` and dynamic `.sql` calls.
No direct product Parquet operation was found in that scope; this is not proof of no consumer.
`cpg-core::sql::query` (`sql.rs:7-19`) is read-only, but
`CompletedInputs::session` (`workspace.rs:2963`) publicly returns an unrestricted `SessionContext`.
Native CLI snapshot queries instead accept a model relation (`lctx/src/newnative.rs:159-184`), so
they are not arbitrary DataFusion SQL. Registered production sources include native providers,
MemTables and Arrow artifacts (`artifact.rs:1239`); they do not establish a Parquet requirement.

At the exact pin, DataFusion `execution/session_state_defaults.rs:147-158` registers
`ParquetFormatFactory` behind `feature="parquet"`. Thus public sessions expose dynamic file-format
registration/SQL even without literal repository callers. **Retain Parquet** rather than silently
narrow that public surface. A future explicit supported-surface decision plus consumer migration
can remove it; do not invent that narrowing during dependency cleanup.
`cargo tree --locked --offline -e features -i parquet` **passed** in the owned worktree; full output
is [parquet-features.json](development-serving/raw/parquet-features.json). It records DataFusion
55.1.0/Parquet 59.3.0 and the real transitive feature union, including CLI/test consumers. The result
is not proof that removing one declaration removes every feature union. DataFusion's own external
sort spills **Arrow IPC**, not Parquet: `physical-plan/src/spill/mod.rs:42-45,240-283` and
`spill/in_progress_spill_file.rs:85` construct IPC stream writers. Spill therefore supplies no
independent reason to retain Parquet. Retention closes this bounded inquiry, not finding O-F46.

## D03: one O2 dev/test family, release remains distinct

Select current O2 incremental local dev/test, debug=0, debug assertions and overflow checks **on**;
imported packages O3 non-incremental; retain toolchain `nightly-2026-09-29`, linker and available
parallelism. Make assertion/overflow policy explicit in the final dev/test configuration rather
than depending on default inference. Release retains its separate acceptance policy and artifacts.
Do not add another candidate optimization level absent contradictory functional evidence.

[profile_probe.py](development-serving/probes/profile_probe.py) copied the exact current profile
sections into an isolated tiny workspace with an imported path dependency. Actual `cargo build
--offline --profile dev -vv`, `cargo test --offline --profile test -vv`, and `cargo test --offline
--profile release -vv` **passed**, 2026-10-11. Full rustc commands and runtime assertions are in
[profile-results.json](development-serving/raw/profile-results.json). They show local O2 with
incremental output, imported O3 without incremental output, the **same imported dev artifact** reused
by test, test assertions on and caught arithmetic overflow, and release assertions off and wrapping
overflow. This is a small policy/reuse probe, not product functional qualification or elapsed-time
measurement. The first probe attempt accidentally inherited the enclosing workspace; the retained
script now declares isolated workspace ownership and the final receipt is the corrected run.

Future functional boundary selects current pure invariant/assertion-sensitive controls, affected
native contract controls and Python/native consumers, then switches `scripts/verify.py:56`
`DEFAULT_CARGO_PROFILE` to `test`, fixture shortcuts/artifact directories, recorded reruns and profile
docs together. Existing `artifact_directory(test)->debug`, explicit release qualifier guards and
historical-receipt profile recovery are preservation constraints (`tests/scripts/test_verify.py:
1106-1431` contains controls, not results from this task). Native Python production leaves remain
explicit release until their own consumer is migrated. Remove `local-test-candidate` after its exact
current tooling consumers migrate. Do not read/wait for BC3, purge caches, or claim test-target
objects and ordinary library objects are identical simply because they share a profile directory.

## D04 and D05: two distinct evidence lifetimes

**D04 selected service witness.** `surrealdb_fixture::substrate_readiness` checks before a borrow
(`surrealdb_fixture.py:223-231`); `Attachment.create` borrows later (`:454-489`). The borrow owner
(`surrealdb_service.py:804-827`) checks under the admission gate and holds `borrowers.lock`.
`Installation.check`/administrative query identity checks (`:374-464`) repeatedly inspect the
executable and installed marker. Select one `CheckedServiceGeneration` returned only by the
existing borrow owner: acquire protection **before** executable/admin/database checks, bind daemon
process identity + immutable executable generation + installation/service/schema generation + exact
endpoint/namespace/databases, and pass the checked witness through fixture/verify prerequisites and
child setup. Its lifetime is the actual borrow; a replacement, admission closure or changed installed
generation invalidates it. Only administrative work within that protected generation may reuse it.
Readiness outside a borrow remains observational. A stat cache or a previous successful readiness
check cannot grant this authority. Preserve maintenance's exclusive route and unknown attachment
holds. Future controls cover replacement before admission, during protected use, multiple required
databases, readiness failure and borrower release without retirement.

**D05 selected completion observation.** `verify_evidence/capture.py:140-195` currently observes
`context_after` then completion-time compiler-artifact paths, using a stat tuple binary cache and
before/after path stats. `context.py:27-92` independently observes source bytes and expressly denies
source-to-binary provenance. A service descriptor is neither this source context nor proof of the
executed test binary. Select a completion-time descriptor observation: open each executable once at
completion; hash/read and compare `fstat` before/after plus pathname binding, and store the original
capture time, descriptor identity, digest and observation policy in the receipt. Missing/racing or
mutated observations remain unavailable/changed. Retain `executed_bytes=unknown` and
`source_closure=unknown`. An open descriptor pins an inode across rename, **not in-place immutability**.

For an unprotected Cargo output path, rehash at each completion; equal path/stats alone do not grant
reuse. Reuse digest only for a producer-owned immutable executable generation held through observation;
the generation witness supplies that guarantee, not an optimization hidden in the reporting reader.
Exports/report/view consume retained receipt values without opening a later mutable path. Reject
delayed report-time hashing and accidental strengthening to launch-time executed bytes. Future controls
replace a path after completion, replace during capture, mutate the same inode, remove the path, reuse
a sealed generation, and independently alter source context without rewriting binary evidence.

## D09: metadata-only shape capture with conservative fallback

Select a Cargo/Hakari freshness capture that keeps **full path/file/directory shape**, every manifest,
lock/toolchain, applicable ancestor/Cargo-home configuration and Hakari declaration byte, and empty
placeholders for source bodies. Preserve automatic/explicit targets, build scripts, arbitrary source
suffixes, empty directories and glob membership. Let Cargo decide target meaning; do not reconstruct
discovery in Python. Capture membership and declaration bytes before/after the observation; an
addition/removal/config edit makes it stale/inconclusive. Body-only source edits need not invalidate
this metadata operation. This cannot attest producer identity or runnable code.

[cargo_metadata_probe.py](development-serving/probes/cargo_metadata_probe.py) **passed 19 cases**
against actual pinned `cargo metadata --offline --no-deps --format-version 1`, 2026-10-11; complete
normalized metadata/error values survive in
[cargo-metadata-results.json](development-serving/raw/cargo-metadata-results.json). It compares
automatic files/directory main targets, all auto flags disabled, explicit inferred paths/arbitrary
suffixes, custom build paths, valid missing explicit source, ignored empty directory, invalid no-target
package, workspace globs/exclusions and invalid glob member, cfg/optional path dependency, config,
same-name discovery error, and concurrent additions/removals/declaration versus body changes.
Symlink files/directories and escaping declarations are valid to Cargo in these fixtures but the
candidate **refuses** them; their refusal is a successful boundary control, not metadata equivalence.
The initial inherited-workspace fixture defect was corrected before the retained final run.

Unsupported symlink/path/config layouts keep today's conservative body capture, or its honest
`not_run` refusal when even that owner cannot safely relocate them; never emit clean from an
incomplete key. The probe is intentionally a candidate, not an edit to `scripts/freshness.py:102-228`.
The subsequent [actual Hakari capture probe](development-serving/probes/hakari_capture_probe.py)
compared 988 inputs (18,765,919 original bytes) using the existing `metadata_inputs` owner on the
owned checkout. Full-body and `.rs`-placeholder copies produced identical **complete resolved**
`cargo metadata --offline --format-version 1`, `cargo hakari generate --diff` and
`cargo hakari manage-deps --dry-run` output/exit status. It additionally removes a generated `ahash`
dependency and the CLI's hack dependency in the copies: both negative cases returned 1 with identical
drift/edit observations. All five comparisons passed; identical infrastructure errors are never a pass. The final
receipt is [hakari-capture-results.json](development-serving/raw/hakari-capture-results.json);
each variant's complete stdout/stderr/result survives in the adjacent `hakari-*.json.gz` files.
The initial copy under the owned checkout inherited outer and relocated Cargo flag arrays twice;
that run is retained as `*ambient-config-blocked*`, not equivalence. The successful rerun binds the
exact configured Linux flags via `CARGO_ENCODED_RUSTFLAGS` for both copies to isolate that extra
ancestor; it changes no job/thread policy. Final production capture must retain intended effective
ancestor/home configuration and avoid such duplicate ambient inheritance. Remaining implementation
controls cover unsupported configurations, declared exclusions, lock normalization and concurrent
changes. Keep existing clean/stale/heuristic/not_run distinctions and return codes. Source bodies
may be elided only for this metadata operation; declared non-source/config/runtime assets retain
their actual relevant bytes, and producer fingerprinting remains full-byte. Detailed per-root
diagnostics remain explicitly selected; terminal/coarse phase/drain reporting stays ordinary.

## S03: retain the current selected native query owner

Inspecting current source resolves the bounded query question without changing a native query.
`selection.rs:652-886` starts sparse membership at nominated physical IDs, follows reverse aliases
through `alias_target`, checks `view_nodes`, then validates competing nominal variants using `view_key`.
`relation_rows` (`:929-1018`) nominates from selected relation membership; it does not need a global
payload scan or a repeated correlated per-row membership SQL. Windows are bounded and charged.
`derived_search::eligible_context_rows` (`:181-250`) loads current primary binding and companion
support context; a prior positive dependency check is explicitly insufficient for absence/new
binding validity. `reader.rs:78-105` already owns shared selection and retains unknown pins on an
unconfirmed selection lifetime. Retain these queries and extend charged `PreparedCache`
(`lctx-serving/src/preparation.rs:158`) at its immutable pin/epoch boundary with domain/index data.
Current-context eligibility witnesses must include binding absence and companion facts, not just
positive membership. Eviction recomputes preparation; it cannot reconstruct expired ranked results.
No EXPLAIN or native query experiment is needed to choose **retain**, and none was run by this lane.
Future controls still invalidate new binding, changed support/map/anchor/alias, definitions and pins.

## S05 and S06: settled exact kernels and minimal sorter repair

**S05 select selected-first batching.** `search.rs:135-154,413-414` scores selected documents and
rejects nonfinite scores; `:443-504` independently hydrates/rescores full winner values.
Retain exact-vector policy, selected eligibility before quotas, distinct targets/ties and final
winner rescoring; index repeated tier membership and use bounded exact top-k. **Reject optional
cohort-first lazy scoring for this increment.** It adds shared-cohort scans for sparse pins and
cannot skip corrupted losing selected documents while preserving present refusal semantics.
Result equivalence alone is insufficient: a finite winner plus malformed/nonfinite losing candidate
must retain failure. No approximate/pre-eligibility shortlist replaces the selected frontier.
Future finite controls compare both accepted results and failures for sparse/dense, duplicate targets,
ineligible high scorers, ties, corrupt losers and branch ordering. No cohort-first campaign or
latency claim is required for rejecting the optional route.

**S06 select repair of current shared `ordered_rows::Sorter`.** At `:253-269` it eagerly creates a
TempDir; `finish` (`:415`) flushes even a fitting tail. Existing binary carry merge (`:382-411`)
already bounds descriptors and preserves canonical equal-key/equal-bytes deduplication versus
different-bytes conflict (`:362-375`). Select an explicit memory/disk backing enum: keep sorted
deduplicated fitting rows under reservation; lazily create TempDir/first run only when needed;
carry a memory tail into bounded merge; derive admissible run sizing from the owned reservation;
retain poison/conflict checks through final merge and every cursor/point-index consumer. Adapt
`PreparedRows`, `PreparedCandidates` and per-cursor ownership, not a second sorter implementation.

The credible library alternative is **public** DataFusion `SortExec`/DataFrame sort + a streaming
adjacent validator over canonical key and complete encoded value bytes. `ExternalSorter` is private
at 55.1.0 (`physical-plan/src/sorts/sort.rs:212`); `SortExec` is public (`:932`). An alternative would
need globally ordered single output (`SortPreservingMergeExec` where necessary), a ResourceBudget
memory-pool adapter, owned DiskManager temp lifetime, Arrow encoding and async/sync adaptation,
followed by full-value collision validation. Hash-only equality or SQL DISTINCT is insufficient.
Existing row/candidate adapters, replayable point indexes, synchronous backup/restore consumers and
error/cleanup semantics make that conversion broader than the observed defect. Keep repair as the
selected route; revisit a library replacement only when an Arrow-native consumer removes those
costs. Future empty/small/no-artifact, spilled-tail, late conflict, cancellation, I/O and all-consumer
controls establish functional closure, never aggregate host bounds (RC12 remains).

## PD02 / S09 / D10: remote execution must have a finality owner

Exact server revision: `238bfeb11f5725bebed370167656748df8067595`, source under
`~/.local/state/library-context/surrealdb.tools/server-build/3efaf277b46867beb1e75f6cae56eb30dfd40878bbb823ef953213cf1cd84680/source`.
All following paths are relative to its `surrealdb/`. The root's dedicated campaign owns its binary
identity and observations; this lane's source result alone is not a native cancellation pass.

1. Server `server/src/rpc/grpc.rs:1653-1760` constructs the job, sends Begin and returns
   `once(Begin).chain(unfold(QueryFraming,...))` as the tonic response. Deadline watchdog trips
   cancellation/closes the item receiver, but does not own/poll execution.
2. `QueryFraming` (`:2295-2312`) exclusively owns `Option<QueryStreamRun>`. Its `next` (`:2411-2457`)
   polls that future while consuming the bounded item channel. No `Drop for QueryFraming` exists.
3. `ReleaseOnDrop` (`:2222-2288`) owns cancel/session/live-query cleanup only. Its task does **not**
   receive the run. Field drop therefore destroys an unfinished execution future instead of moving
   it to a finality owner, contradicting its comment at `:2219-2221`.
4. Core `core/src/rpc/protocol.rs:2077-2083` wraps the job in another future;
   `core/src/kvs/ds.rs:5133-5164`
   constructs the future invoking `Executor::execute_statements_streaming`. No detached executor
   is introduced there. `core/src/dbs/executor.rs:2377-2393` must reach normal finalization/cancel awaits.
   `datastore/src/tr.rs:30-40` emits the open writable-transaction drop error; no async rollback is
   performed by that Drop. Backend object destruction alone does not establish physical cleanup.
5. The exact WebSocket path (`server/src/rpc/websocket.rs:792-807`) explicitly drains all in-flight
   futures and warns that dropping them abandons open transactions. It provides a useful local
   ownership pattern, not proof for gRPC.
6. Patched SDK `third_party/surrealdb/src/engine/remote/grpc.rs:904-950,1102-1119` forwards the
   remote stream; local receiver disappearance returns and drops tonic stream. Its newer
   `drain_after_end` checks terminal EOF on normal paths, not abandonment. Tonic's body/stream
   lifetime does not secretly retain/poll the server's dropped `QueryFraming` future. Exact tonic
   0.14.6 `src/server/grpc.rs:262-299,429-456` moves the returned stream into `EncodeBody`;
   `src/codec/encode.rs:25,219,257-272` owns that source directly and introduces no execution task.

**Dedicated native evidence, 2026-10-11 (coordinator-owned).** Server SHA-256
`710229df33346fdf9733ef86d46e31874f0cacc2691529327e19e6534477b8ed` ran isolated ns/db
`efficiency/disconnect`. The build command `cargo test --release -p lctx-publisher
--test efficiency_grpc_campaign --no-run` passed. The coordinator’s `native_campaign.py` then
invoked that reported test binary with `--nocapture`, explicit dedicated endpoint/credentials,
SQL and drop-phase environment for each case; see the campaign README for the exact invocation. Normal EOF transaction committed `grpc_probe:drained`. Dropping
after Begin plus 250 ms inside the transaction emitted
`ERROR surrealdb_datastore::tr: A transaction was dropped without being committed or cancelled`
at `2026-10-11T13:11:01.489564Z`. Its uncommitted row was absent. Autocommit disconnect after first
batch retained `grpc_probe:before` and omitted the later row. Subsequent `RETURN 1` succeeded;
SIGTERM/wait confirmed child exit 0. All clients exited 0; these diagnostic client successes
**reproduce the defect**, not a passing remote-drain contract. See
[observations](raw/grpc-01-observations.json), [server log](raw/grpc-01-server.log),
[command/run output](raw/grpc-01-run.log) and per-client logs. Absent uncommitted rows do not negate
the observed missing finality owner. No repaired-server qualification has run.

**Selected target: minimal server owner transfer**, confined to gRPC stream execution. Give the
existing framing owner a Drop path that first trips cancellation, closes item delivery, takes the
remaining run and transfers it to a server-owned drain task **registered before returning**; that
task keeps the request/session/transaction authority until the run settles. Drain/discard items
without blocking the executor, reconcile unregistered live queries from the eventual run result,
then release ephemeral session authority. Normal EOF uses the same terminal bookkeeping once.
Retain tasks through server shutdown and expose a checked terminal outcome to the operation owner;
an untracked fire-and-forget spawn or timeout that aborts the run repeats the defect. Do not install
a main-tree vendor patch in this design-only task. Future patch qualification covers before first
execution, inside transaction, full channel, sort/no-output, committed-before-disconnect, deadline,
shutdown and normal EOF. A backend rollback observed after disconnect is not the owner contract.
The bounded interface is an internal handoff of request identity + run + release state to the existing
gRPC service lifetime, and an awaited terminal result to its drain owner; no new public status RPC or
cross-service scheduler is needed to fix future polling. After connection loss that server result is
not automatically a client acknowledgement: native operation identity/recovery remains separate.

**Selected current behavior: recovery-owned unknown when remote terminal proof is lost.** The
operation/attempt identity and accepted effect guard remain protected; journal/control reconciliation
must prove commit or cancellation before effect hold release. Disconnect is not an acknowledgement.
For read-only work, a lost physical terminal retains the reader/session hold as unknown until its
existing recovery owner establishes quiescence; do not claim drainage from no mutation. A named
client transaction/session can be explicitly cancelled/detached only by its legitimate owner, not
by a broad service reset. Preserve `Completion::RemoteState::Unknown` and existing reader guard
behavior. The campaign confirms the defect and fixes the regression cases, not authority to release
unknown work; neither the interim unknown disposition nor these probes close S09/D10.

`NativeRows` (`reader.rs:1048-1208,1327-1372`) is stronger than raw SDK receiver drop: it retains the
raw stream and client/transaction/scope, drains exact statement + outer transport terminals, and
transfers unfinished scoped reads to a drain task or protects the pin if no runtime remains.
Preserve it. Normal owned drainage can avoid the raw-abandonment defect, but interrupted process
or lost connection still needs the target server owner/recovery semantics.

S09 entrypoint is `lctx/src/main.rs::main` (`:1120`), its Tokio runtime/command dispatch and owned
compiler/provider/native scopes. Add SIGINT/SIGTERM bridging to the existing shared cancellation
flag, fence new admissions and await owned terminal completion before returning; do not wrap/drop
the entire command future on first signal. The actual local interfaces are
`Workspace::cancellation().cancel()` / `Workspace::drain_report()` (`workspace.rs:1907-1934`),
which join provider tasks, provider bridge, native calls and `NativeCompilerStore::drain_report()`;
that native owner closes admission, waits operation leases and returns independent local/remote/
committed facts (`compiler.rs:2673-2722`). Reader-only work uses `NativeRows::drain_transport()` and
its reader/session close owner. Compose those existing reports with `Completion::complete`, keep
typed failures and uncertain attempt identity, and release fixture ownership only where cleanup is
permitted. D10 separately binds a cancellation owner inside the
actual nextest/test process before native work and fixture use; outer `scripts/verify.py` command
signalling and `surrealdb_fixture.py::run_attached` (`:877`) only establish child/process outcomes.
Direct test entrypoints must keep their native guards/read streams and completion reporting alive
through cooperative cancellation. Hard stop/lost runtime leaves recoverable unknown attachments,
not fabricated rollback/cleanup. Future independent CLI/direct-nextest/wrapper controls retain
child exit, local cleanup and remote finality as separate facts and preserve unrelated borrowers.

## Probe reproduction

Run the three Python scripts in `development-serving/probes/` with `--repository
/home/paul/library-context-wt/efficiency-decisions`, `--scratch
/home/paul/library-context-wt/efficiency-decisions/build/development-serving-probes` and
`--output` pointing respectively to `development-serving/raw/profile-results.json`,
`development-serving/raw/cargo-metadata-results.json` or
`development-serving/raw/hakari-capture-results.json` under this evidence folder. Use Python 3.14
for the Hakari probe because it imports the repository freshness owner. Its compressed raw command
outputs are written beside the requested receipt. Recorded outputs and the earlier ambient-config
blocked attempt remain in `development-serving/raw/`; only source scripts belong in `probes/`.

## Evidence limits

Context7 resolve/query discovery was used for DataFusion, SurrealDB and Cargo on 2026-10-11 before
exact local skill/source inspection. Current-branch documentation was a lead only; exact installed
source and pinned command receipts govern conclusions above. The selected architectural remedies
are Proposed/Interface-checked, not Implemented/Tested product changes. Pure probe passes do not
close product findings, native serving, whole-product acceptance or performance. D01/D06/D07/D08
remain settled as written; this investigation adds no new mechanism or obligation to them.
