# Workspace storage

Workspace storage management keeps current inputs, immediate-use caches and unresolved content
available. It retires an independently owned output only when its authoritative owner has
confirmed cleanup, every consumer obligation permits retirement, the configured grace has
expired, and fresh reader, identity and reference checks pass. File modification time, directory
size, a changed pin and a completed child process do not establish release.

The executable policy is [`.config/storage.toml`](../.config/storage.toml). The
[storage lifecycle plan](plans/storage-lifecycle-management-plan_2026-10-09.md) owns implementation
and qualification status; this guide describes the command and ownership contract. An available
command does not establish that every content category has a qualified retirement route.

## Inspect and act

Run commands from this checkout. `just storage` uses the installed **Python 3.14.7** directly,
offline and without synchronizing the project environment. Its equivalent launcher is:

```sh
uv run --no-project --offline --no-python-downloads --python 3.14.7 python scripts/storage.py status
```

Do not invoke these scripts with the system `python3` (3.12).

| Command | Effect |
|---|---|
| `just storage inventory` | Shallow inventory, policy categories, participant coverage and protected/unclassified roots |
| `just storage inventory --deep` | Explicit recursive allocated/apparent accounting; does not authorize retirement |
| `just storage status [ID…]` | Current dispositions, capacity, last sweep and automation status |
| `just storage plan [ID…]` | Fresh eligibility and reasons, without deleting content |
| `just storage retain ID --consumer NAME --requires immediate-use` | Add or renew a named local-availability obligation |
| `just storage release ID --consumer NAME --reason 'consumer completed'` | Release that obligation; owner cleanup and other holds still apply |
| `just storage apply ID…` | Recheck and attempt retirement of these objects |
| `just storage sweep [ID…]` | Recheck eligible objects, descendants first; omitted IDs select the managed inventory |
| `just storage adopt PATH --owner KIND --owner-path OWNER --category CATEGORY` | Register legacy content as protected, with unresolved exposure |

Read a plan's reasons before applying it. `apply` and `sweep` reobserve authoritative owners and
tracked references under nonblocking exclusive admission; a busy owner is skipped. They journal
the exact physical lifetime before quarantine and deletion so a retry can finish an interrupted
effect. JSON output distinguishes actions from dispositions. Exit 75 means `blocked`; exit 1
means `failed`. A successfully executed inventory can still report unresolved or retained objects.

`adopt` is observation, not a declaration that old content was safely managed. It invents no
historical cleanup time or deadline and does not make arbitrary legacy paths disposable. A
qualified owner must establish writer/reader coverage and release; absent that route the content
remains protected. Do not edit descriptors to bypass this boundary.

## Ownership and retention

Each managed object has a stable ID, category, authoritative owner, physical lifetime and named
consumer obligations. Owners retain their existing records and locks; the storage descriptor does
not copy mutable process state or replace native publication/retirement authority. Nested outputs
are independent objects: a report can outlive raw capture, and a receipt cannot disappear while
an unretired child still needs it.

Temporary obligations start only after confirmed owner cleanup. New run receipts retain 90 days,
run logs 30 days, profile raw 14 days and profile reports 30 days. An explicit additional consumer
hold wins over those deadlines. Cancellation with unresolved descendants does not start expiry.
The seven-day grace for released task builds, local attachments, documentation candidates and
managed acquisition generations starts from authoritative release. Configuring a category's
grace does not create a release route for its owner.

| Content | Current management boundary |
|---|---|
| Ordinary shared Cargo intermediates, checkout `target/`, sccache, uv, toolchains and model/kernel caches | Remain warm under their native owners; no generic cache purge or internal Cargo eviction |
| Worktree `--build-dir own` | Unique task lifetime; creation/use is admitted. Explicit worktree removal releases the build root, which remains through grace |
| Run receipts, logs and profiling components | Owner cleanup and named obligations govern expiry; existing retention markers remain protective |
| Checkout native/vLLM environments and extension resources | Selected resources stay warm; actual managed holders exclude sync and source removal. Superseded environment retirement is not qualified merely by a new selection |
| Acquired library sources/environments | The CLI owns unique pinned generations, native uv/git writers and original-input reader leases through frozen capture. Successful selector publication releases old managed versions after reader acknowledgement; current versions stay warm. Legacy paths remain protected |
| Local validation attachments | Confirmed local release governs attachment records. This never deletes their native database content or the persistent service |
| Documentation candidates and previous site generations | Publisher process cleanup releases admitted candidates; current published site and live preview readers remain protected |
| Externally owned inputs and tools | Protected here; this repository's lifecycle work does not manage central skill stores or other repositories |
| Native persistent content, service state, recovery archives, Git/LFS evidence and unknown content | Protected here; use the owning native/operator or evidence workflow |

`just runs prune` keeps its familiar count/age selectors, but they select candidates only;
lifecycle obligations authorize effects. A retained terminal run, profile evidence or unresolved
cleanup also protects its containing worktree. `just worktree-remove --force` cannot override
live environment/build admissions, service dependencies or evidence holds.

There are no new CPU/thread/worker limits, healthy-command timeouts, cache-size eviction limits,
or routine clean commands. The diagnostic 64/100 GiB policy values are advisory observations.
Do not use `cargo clean`, broad `rm`, mtime-based deletion or native cache purges as a substitute
for owner release.

## Host coverage

The host configuration is
`${XDG_CONFIG_HOME:-~/.config}/library-context-storage/config.toml`; `LCTX_STORAGE_CONFIG`
selects an explicit alternative. Private lifecycle state lives under
`${XDG_STATE_HOME:-~/.local/state}/library-context-storage/`, or `LCTX_STORAGE_STATE`.
Keep that state outside disposable output roots. Repository policy remains in the checkout.

```toml
schema = 1
repositories = ["/absolute/path/to/library-context"]
# Optional explicit locations; selection does not qualify archive replay:
# archive_destination = "/absolute/path/to/archive-volume"
# diagnostic_root = "/absolute/path/to/diagnostics"
```

Use `just storage register-repo /absolute/repository` to register an available Git repository and
its discovered worktrees explicitly. This task covers library-context and its worktrees only.
Inventory never silently registers or deregisters a participant. An unavailable checkout or
unregistered worktree leaves coverage unresolved. `unregister-repo` refuses a repository that
still owns live lifecycle records.
Do not infer release from absence or remove a participant simply to make a sweep eligible.

`lctx acquire`, compilation's Stage A capture and deployment identity all use the acquisition
owner. `--reinstall` creates a fresh environment generation; no admitted reader's files are
mutated. The exact interpreter, frozen lock and uv copy isolation remain unchanged. An explicit
owner release of an unused managed override is available as:

```sh
uv run --no-project --offline --no-python-downloads --python 3.14.7 python scripts/storage_acquisition.py release ID
```

This refuses active or unacknowledged readers, changes the owner's selection and starts grace;
it never deletes the generation directly. A helper/parent crash leaves an unresolved reader
receipt. EOF, pin edits and unavailable checkouts do not stand in for an explicit acknowledgement.

## Profiling, evidence and archives

Before creating a diagnostic capture, give it a run/profile owner and a named consumer. Keep
replayable raw data separate from small derived reports and retained evidence. A consumer that
needs local replay uses `raw-replay` or `immediate-use`; a cited report uses `cited-report`.
Use `just storage retain --help` for the exact capability contract and repeatable replay
`--operation` declarations. Tracked references protect their cited physical objects and
containing scopes; releasing one descriptor consumer does not erase those references.

For raw data that can leave local storage, an explicit `restorable-replay` obligation must name
the operations it needs. `just storage archive ID --destination PATH` requires a profile owner,
confirmed cleanup, no immediate/raw-local hold, an available destination and qualified readers.
It validates archive integrity and required replay behavior before publishing the archive
receipt; successful archive creation does not itself delete local raw. Missing destination,
reader or replay capability is `blocked`, and the original stays available.

`just storage restore ID --destination NEW_PATH` restores to a new destination through the
profile/archive owner. A verified digest alone is insufficient evidence of replay capability.
Archives remain protected recovery objects. Keep Git/LFS evidence in its existing evidence owner;
do not commit venvs, Cargo outputs or raw runtime stores to make them persistent.

## Persistent service dependencies

The stable SurrealDB service owns its executable dependencies independently of database payloads.
If its maintenance installer still comes from a checkout or task build, that supplying scope
cannot retire. Inspect `just service status` and use
`just service maintenance --stabilize-installer` to transfer the exact executable digest to
the service-owned sibling tools store. This uses the existing maintenance boundary and checks
the transferred executable; it is not an upgrade or operator database adoption. An incomplete
transfer keeps both dependencies protected. Native content retirement continues through
`lctx snapshot retire` and native pin/quiescence rules, never the generic filesystem sweep.

## Producer integration and automation

New producers acquire shared `storage_lifecycle.admission(paths)` before creating or using
managed output and keep it through child/descendant cleanup. Acquire it before native owner
locks. Publish the actual fixed owner and named consumer while admitted; mark `managed=True`
only when creation and all use paths are covered. Complete temporary obligations after closing
the owner's live lock and confirming cleanup. Enrollment failure protects content and must not
turn a healthy command into a failure. Unintegrated bare overrides remain unresolved.

Use owner adapters for authoritative observations and exclusive owner guards for effects;
do not add generic PID guesses or deletion callbacks to configuration. Preserve available
parallelism. A new content class needs an actual producer, reader, release event and focused
failure/concurrency controls before it joins an automated sweep.

`just storage automation status` observes installation. `automation install` and scheduled sweeps
require the matching-source SM8 acceptance receipt, accepted independent review and explicit host
configuration. The daily user timer is therefore blocked until that boundary is qualified;
adding a command or a policy row does not activate it. `automation disable` stops it explicitly.
