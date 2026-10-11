# Minimal live maintenance interface request

Source inspected 2026-10-11: scripts/surrealdb_service.py and crates/lctx/src/main.rs.
This is a source-led integration request, not an implemented host adapter.

## Exact existing seams that must change

- service.py UPGRADE_SOURCES at 1882-1887 supports target4/source3 and target5/source4 only;
  upgrade_installer admission at 2289 likewise accepts only installed3|4. The new route must
  be explicit target6/source5 with immutable ebcf423cc16e6cacea6455500a0cdfc9795d08ac schema
  eec4dc767fe4477fbfc3f5a584b54a6c8855345af02c3390ad5f77ac326e0383. No old plan relabeling.
- _validate_upgrade_contract at 2104 currently accepts only formats1|2 and a flat set of
  hash arrays. Format3 must require EXACT new fields target_version, original, components,
  target_executable_sha256, activation in addition to existing fields. Original has exact
  commit, typed full source marker, three SHA256 labels and retained_obligation_inventory.
  Components are the exact ordered eight names with four ContentHash identities each.
  Compare target executable SHA256 to the actual selected candidate descriptor, original
  executable SHA256 to retained previous_installer_sha256, and actual per-scope evidence
  identities to privately retained original receipts/inventory. Validate marker full typed
  correspondence, not just generation/schema. No permissive legacy format3 fallback.
- _upgrade_execution_contracts (2129) calls metadata-only CLI, and upgrade_installer (2326)
  creates contracts BEFORE acquiring exclusive borrowers.lock and before old native
  close-admission/drain (2460). Original source evidence cannot exist at that point. Keep
  the initial immutable intent plan for crash recovery, acquire/drain under its exact owner,
  then create-once per-scope source-evidence+execution artifacts, and bind their exact hashes
  in the maintenance operation's durable execution descriptor. Never overwrite the initial
  plan or synthesize pre-drain original marker/receipts. Resume authenticates both layers.
- _run_installer (606) runs original close/drain but returns None and retains no output or
  scope-bound original receipt. The source5 original CLI's authentic StoreCommand::Drain
  prints only ready:false, namespace, database after the actual drain call; it prints no
  request IDs or attempt inventory. Host capture must retain exact executable SHA, argv,
  source scope/generation/marker, child identity, stdout/stderr/exit and confirmed local
  cleanup, plus independent exact source drained predicates/native outcome inventory.
  Absence of output is not a receipt; no invented per-request identity or retrospective
  completion. Retained obligation manifest provides exact old frozen close NativeRequests.
- _upgrade_cli (1909-1930) uses subprocess.run with no descriptor handoff. Existing
  maintenance_owned (976) accepts an environment token + lock-held probe + live process;
  that is not authenticated live authority for a new Rust process. It cannot implement
  Source5MaintenanceLease by rereading JSON and returning Ok.
- _upgrade_replacement_preflight (2141) and _upgrade_predecessor_observations (2204)
  observe old native_upgrade phase enum and source4 progress. For format3 read the fixed
  native_upgrade_v6 pointer/journal plus exact native_upgrade_progress_v2 contract/state/
  revision/sealed and current source fence. Initial journal bytes remain immutable;
  current source control_revision is initial + checkpoint_revision + 1. Sealed target
  requires exact target marker/pointer/published journal/sealed contract, not phase alone.
- main.rs UpgradeContract (216,958) takes only schema/operation/execution and is labelled
  metadata-only. Upgrade (1005) passes a definition blueprint instead of a component
  registry/live host lease; UpgradeQualify (994) calls the removed source4 qualifier.
  Final definition installation after activation (1019) cannot establish the new atomic
  ExecutableDefinitions barrier: final required definitions belong to aggregate activation.
  No old progress qualification receipt is proof that source5 typed conversion is valid.

## Requested narrow capability and arguments

Retain --expected-schema, --operation, --execution, --execution-contract and validation-only
--stop-after-pages. Add --source-evidence PATH to upgrade-contract and --maintenance-fd FD to
upgrade-contract, upgrade and any replacement format3 qualification control. Source-evidence
is the immutable typed original evidence/obligation manifest, not arbitrary SQL and not
execution authority. The actual binary constructs the exact registry/activation identities;
Python does not invent component code hashes. Existing config supplies namespace/database,
authentication and generation. Metadata-only `store schema` stays independent.

Use one inherited private Unix socket endpoint owned by the existing Python maintenance
owner, passed only to its exact Popen child via pass_fds. The parent retains the existing
exclusive admission.lock AND borrowers.lock and storage admission throughout child cleanup;
no new lock domain, capacity cap or background daemon. Replace only this subprocess.run
site with a managed Popen + request/reply servicing loop, while preserving existing output/
error/child-drain ownership. Parent death/closed channel or failed checks fail closed and
leave maintenance marker/unknown operation evidence retained; never silently recreate a
broker from serialized JSON.

The Rust Source5MaintenanceLease adapter must authenticate the connected parent as the
exact maintenance ProcessIdentity (PID/start identity, UID), operation/token/installation
and immutable contract digest; independently validate its inherited endpoint and actual
selected executable identity. Each recheck sends a fresh challenge with operation, execution,
contract digest and exact scope/generation. The owner validates this is its exact current
child (PID/start + actual executable SHA), checks it STILL owns both exact lock-file inodes,
authenticates maintenance marker/initial plan/execution descriptor/evidence bytes, rejects
all new or unresolved borrower records, checks the exact current daemon/config scope, and
checks immutable original executable/drain/obligation evidence. Reply binds challenge,
contract digest, owner+child identities and scope; any mismatch/EOF refuses. A self-declared
JSON token or one startup acknowledgement is insufficient. The native driver independently
checks actual source/target marker, drained predicates, current checkpoint fence and immutable
journal at its transaction boundaries. Preserve original-source evidence on published resume;
do not rerun an old installer on format6 or rewrite original closed era to current era.

The exact socket credential/FD validation belongs to the host+CLI adapter implementation;
root must implement and test authenticated live ownership and parent death, wrong inode,
changed receipt/candidate, borrower appearance and mismatched scope before effects. A generic
callback returning Ok is not the requested interface. If this cannot be authenticated with
the actual process APIs, refuse rather than accept JSON-only authority.

Existing _upgrade_restart is not automatically remote finality evidence: required original
remote drainage must come from the actual reviewed original installer/server join route and
retained native observations, not merely a stopped process or a passed child body. Host
original-source receipts and the immutable terminal operation map remain the authority inputs.

## Root integration boundary

Root owns CLI/service adapter, actual registry implementations, target binding descriptors,
module registration, journal evidence retention/selected backup, and all actual service effects.
The pointer/replay patch and foundation are still source-only; no assembled qualification is
claimed. Compile/source snapshots and minimal revealing native controls remain NOT_RUN.
