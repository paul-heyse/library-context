# Source5 to format6 maintenance foundation

Prepared 2026-10-11 for root source review. This is an OFF-TREE draft, not an applied,
compiled, exercised or independently accepted migration. No service effects were performed.

## Included behavior

- Actual source5 schema `eec4dc767fe4477fbfc3f5a584b54a6c8855345af02c3390ad5f77ac326e0383`,
  immutable commit `ebcf423cc16e6cacea6455500a0cdfc9795d08ac`, explicit version5→6/kind/format3.
  Source-control declaration bytes come directly from that Git commit. The old source4
  history-page protocol is removed from this draft's runtime upgrade entrypoint; it is
  preserved verbatim in `base/` as review evidence, never relabeled as source5.
- Original source marker, executable SHA256, original-installer drain receipt SHA256,
  host borrower-drain receipt SHA256 and retained-obligation inventory are immutable
  contract inputs. A mandatory live `Source5MaintenanceLease` rechecks them; serialized
  fields do not confer authority. Target executable SHA256 and aggregate activation
  implementation identity are equally bound.
- Exact ordered handlers for Control, TypedModel, Originals, CompilerBindings,
  CompletedAdmission, Publications, RetainedObligations, ExecutableDefinitions. Every
  handler implements Preflight/Declarations/Transform/Verify; missing handlers or an
  empty proof/body refuse. Every Preflight finishes before any conversion declaration;
  every Transform finishes before independent Verify. Only all8 Verify completions
  can reach Ready. No default success converter or verifier is supplied.
- Separate `native_upgrade_v6` immutable original journal and one current
  `native_upgrade_progress_v2` CAS checkpoint. Presence/absence observations, bounded
  source-range checks, conversion bodies and progress commit together. Lost write
  acknowledgement observes the exact current slot; primary write errors survive a
  missing/mismatched or failed observation. Existing unchanged-slot acquisition requires
  a successful transaction check; failed responses reconcile only exact new bootstrap or
  ownership transfer plus independent current source-fence/original-journal readback.
  Compatible verifier correction rewinds all
  verification; published execution stays immutable and cannot claim a changed verifier.
- Bootstrap writes source5 control_revision once. Subsequent checkpoints/ownership
  transfer write it once atomically with progress. Expected source marker is initial
  marker with control_revision = original + progress_revision + 1. Any unexplained
  source write refuses resume. Initial source bytes stay unchanged in the journal;
  old request eras/epochs/outcomes, job counters/cutoffs/incarnations are not rewritten.
- ControlComponent authenticates original source-control declarations, audits EVERY
  retirement job and actual reciprocal predecessor/successor rows, and proves
  recovery_source ancestry one edge per resumable checkpoint. Strict era increase
  excludes cycles without an ancestry/depth cap. Nominal `lineage` is never invented
  as a rich-row dependency. Job/item/outcome bytes remain original.
- RetiringClaimAudit is composed before every guard Preflight, Translate and Verify.
  Exact current authenticated source-marker, guard, item and owning job bytes are
  compared in the same checkpoint. The proof fanout fits one existing native window.
  Old retiring+retired=true refuses pending ORIGINAL installer completion.
- Old source5 terminal frozen is not target6 active freeze. It requires the exact
  retained original close operation, exact immutable finish_attempt_close SQL digest,
  original terminal_attempt owner/epoch/era, committed+resolved receipt, and no incoming
  attempt-owned holds. The transform atomically stores full original attempt+effect
  bytes in fixed `native_upgrade_frozen_v6` proof and maps state to closed, revision+1.
  Verify rechecks that exact transformation. Missing original receipt refuses.
- Activation requires root's nonempty registered aggregate finalizer. In ONE transaction
  it repeats source/observation/drain/progress checks, final target declarations,
  N03 all64 stripe+backup-gate/era cut, target marker/version, original journal phase and
  sealed progress. No standalone N03 publication exists.
- Target installation retains fixed `source5_upgrade` journal pointer. Private
  `retained_source5_installation_outcome` authenticates that exact published journal,
  source version/schema/generation/closed original era/admission horizon, committed
  installation owner NONE outcomes only. It neither scans journals nor grants any
  pending/new old-format request execution.

## Root integration obligations and pending seams

1. Do not apply this alone as a runnable upgrade. Implement every seven non-Control
   converter/validator and `AggregateActivation`. Register exact handler contracts and
   activation identity; no shim returning an empty proof. TypedModel owns full old
   six-family decode/identities; CompilerBindings owns exact physical contributor
   mappings; Originals may consume root's existing source5_originals decoder but must
   earn target blob full readback and ownership; CompletedAdmission must independently
   earn destination semantics; Publications owns exact handle consumer mapping;
   RetainedObligations preserves all original effect/outcome/history/job/cleanup scopes;
   ExecutableDefinitions owns source/target definitions and index readiness.
2. Implement the host adapter under the real existing exclusive maintenance lease. It
   must inspect actual immutable original CLI bytes, retained source drain receipt,
   exact daemon/namespace/database/generation/closed era, every named borrower/cleanup,
   original native request/owner drainage and authenticated retained-obligation input
   inventory. A JSON-only adapter returning Ok, process-exit evidence, or running target
   check_installation against source5 is invalid. Bind actual target executable too.
3. Replace scripts/surrealdb_service.py format2/source4 contract validator and CLI
   UpgradeContract/Upgrade/UpgradeQualify producers/consumers with real format3 inputs,
   registry and host capability. Existing `upgrade_with_contract` signature changes;
   old qualification API is intentionally absent, not relabeled. Existing target-only
   maintenance_client remains available. Retire the old `control::migration` module
   registration/file when integrating the new State; it references old State.after and
   remains source4-specific. Do not compile it against target6 progress.
4. Preserve root's `pub(crate) mod source5_originals;` and `pub(crate) mod source5_rows;`
   declarations and files. Draft includes both; this package does not edit those decoders. Preserve other concurrent work.
5. Declare source5_upgrade in the target base schema/identity and preserve current
   journal/frozen-proof rows as named installation-owned evidence. The overlay supplies
   temporary migration declaration; it is not a substitute for updating target ownership.
6. Root must invoke retained_source5_installation_outcome only from the already committed,
   original digest/operation/owner_kind authenticated replay path. Current authorization
   still refuses NONE. Do not call this helper to authorize execution or refresh an era.
7. Supply Source5FrozenReceipts from the authenticated immutable retained-obligation
   manifest, keyed by exact attempt. No operation lookup scan or guessed absent outcome.
   If original close receipt has been compacted/unretained, use the actual original
   installer/owner route to establish an independently reviewed original receipt or
   refuse; this packet cannot synthesize it. Old generic source_preflight still refuses
   frozen without this proof. The aggregate driver uses the new audited Control route.
8. Preserve source marker fields until final activation; target declarations can coexist
   in staging but cannot remove source admission/backup revisions or change source
   functions/era prematurely. Aggregate activation body must not mutate original marker
   before N03 activation's exact marker comparison. All newly converted pre-activation
   guards created AFTER Control Transform must initialize tombstone_revision consistently
   with revision, because Control Verify audits all physical guards. No target-runtime
   writer attaches before marker publication. Final DRAINED predicate excludes active6
   frozen too; destination conversion owners must actually finish before publication.
9. This foundation preserves the SAME actual service/store generation across the atomic
   adjacent native-format transition, matching the accepted N03 activation component.
   A distinct target service generation needs an explicit reviewed generation mapping;
   do not silently reuse this contract for it.
10. Run compile checks and pure controls only after root accepts packet and capacity/Cargo
    ownership permits. Then actual selected fixture interruption/native migration controls
    require root authorization and authentic source5 original installer. No current test
    result qualifies the draft. Focused controls live in contract::tests, progress::tests,
    migration6::{retiring_claim_controls,component_controls}; all are NOT_RUN.

## Patch application

`patches/all-foundation.patch` changes only upgrade.rs, upgrade/{contract.rs,progress.rs,
progress.surql,source5-control.surql} and control/migration6.rs. Review bytes under tree/;
base/ holds the exact captured main counterpart (including the root module declaration).
There are no main edits, commits, plans/STATUS updates, formatting or Cargo/service calls.
